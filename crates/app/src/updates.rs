use azr_platform::{log, updates::Update};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_STATUS: u32 = WM_APP + 77;
pub const RELEASES: &str = "https://github.com/Ericismee/Azr/releases/latest";

#[derive(Clone)]
pub enum Status {
    Idle,
    Checking,
    Current,
    Available(Update),
    Installing,
    CheckFailed,
    InstallFailed(Update),
}

static STATUS: Mutex<Status> = Mutex::new(Status::Idle);

pub fn status() -> Status {
    STATUS.lock().unwrap().clone()
}

fn publish(status: Status) {
    *STATUS.lock().unwrap() = status;
    crate::prefs::broadcast_message(WM_STATUS);
}

pub fn schedule_check() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(8));
        if matches!(status(), Status::Idle) {
            check();
        }
    });
}

pub fn check() {
    {
        let mut status = STATUS.lock().unwrap();
        if matches!(*status, Status::Checking | Status::Installing) {
            return;
        }
        *status = Status::Checking;
    }
    crate::prefs::broadcast_message(WM_STATUS);
    std::thread::spawn(|| {
        publish(
            match azr_platform::updates::check(env!("CARGO_PKG_VERSION")) {
                Ok(Some(update)) => {
                    log::line("update", &format!("Available: {}", update.tag));
                    let dir = crate::paths::config_dir();
                    let notice = dir.join("update-notified.txt");
                    let already_notified =
                        std::fs::read_to_string(&notice).as_deref().ok() == Some(&update.tag);
                    if !already_notified && crate::mainwin::notify_update(&update.tag) {
                        let _ = std::fs::create_dir_all(dir);
                        let _ = std::fs::write(notice, &update.tag);
                    }
                    Status::Available(update)
                }
                Ok(None) => {
                    log::line("update", "Up to date");
                    Status::Current
                }
                Err(error) => {
                    log::line("update", &error);
                    Status::CheckFailed
                }
            },
        );
    });
}

pub fn install_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_owned();
    (dir.join("unins000.exe").is_file() && dir.join("unins000.dat").is_file()).then_some(dir)
}

pub fn install() {
    let Some(dir) = install_dir() else {
        azr_platform::launch::open(RELEASES);
        return;
    };
    let update = {
        let mut status = STATUS.lock().unwrap();
        let update = match &*status {
            Status::Available(update) | Status::InstallFailed(update) => update.clone(),
            _ => return,
        };
        *status = Status::Installing;
        update
    };
    crate::prefs::broadcast_message(WM_STATUS);
    std::thread::spawn(move || match update.start_setup(&dir) {
        Ok(()) => crate::mainwin::request_quit(),
        Err(error) => {
            log::line("update", &error);
            publish(Status::InstallFailed(update));
        }
    });
}
