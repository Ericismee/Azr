use azr_platform::launch;
use azr_platform::process;
use azr_platform::tray::TrayApp;
use azr_platform::wins::{enumerate, focus};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    IsWindowVisible, IsZoomed, PostMessageW, ShowWindow, SW_RESTORE, SW_SHOWMAXIMIZED, WM_CLOSE,
};

static STATES: Mutex<Option<HashMap<isize, bool>>> = Mutex::new(None);

fn states() -> std::sync::MutexGuard<'static, Option<HashMap<isize, bool>>> {
    STATES.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn list() -> Vec<TrayApp> {
    let apps = azr_platform::tray::tray_apps(std::process::id());
    let mut states = states();
    let map = states.get_or_insert_with(HashMap::new);
    for app in &apps {
        for &pid in &app.pids {
            let Some(h) = enumerate::main_window_of(pid) else {
                continue;
            };
            let hwnd = HWND(h as *mut _);
            if unsafe { IsWindowVisible(hwnd).as_bool() } && !focus::is_minimized(h) {
                map.insert(h, unsafe { IsZoomed(hwnd).as_bool() });
            }
        }
    }
    apps
}

/// Brings the app's real window back, or launches its executable if no window
/// was found.
pub fn open(app: &TrayApp) -> bool {
    for &pid in &app.pids {
        if let Some(h) = enumerate::main_window_of(pid) {
            let maximized = states()
                .as_ref()
                .and_then(|m| m.get(&h).copied())
                .unwrap_or(false)
                || unsafe { IsZoomed(HWND(h as *mut _)).as_bool() };
            unsafe {
                let _ = ShowWindow(
                    HWND(h as *mut _),
                    if maximized {
                        SW_SHOWMAXIMIZED
                    } else {
                        SW_RESTORE
                    },
                );
            }
            states()
                .get_or_insert_with(HashMap::new)
                .insert(h, maximized);
            if focus::force_foreground(h) {
                return true;
            }
        }
    }
    if !app.path.is_empty() {
        launch::open(&app.path);
        return true;
    }
    false
}

/// Asks the app to close, then terminates it if it is still alive. Most tray
/// apps answer WM_CLOSE by hiding again, so the fallback is the only reliable
/// way to quit them without their notification-area menu.
pub fn quit(app: &TrayApp) {
    if app.pids.is_empty() {
        return;
    }
    for &pid in &app.pids {
        for h in enumerate::windows_of(pid) {
            unsafe {
                let _ = PostMessageW(Some(HWND(h as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
    let pids = app.pids.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        for pid in pids {
            if process::is_alive(pid) {
                let has_visible_window = enumerate::windows_of(pid)
                    .into_iter()
                    .any(|h| unsafe { IsWindowVisible(HWND(h as *mut _)).as_bool() });
                if !has_visible_window {
                    process::terminate(pid);
                }
            }
        }
    });
}
