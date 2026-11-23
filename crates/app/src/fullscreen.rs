use azr_platform::wins::{focus, overlap, tracker};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_FULLSCREEN: u32 = WM_APP + 243;
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn poll() {
    let fg = focus::foreground();
    let candidate = if crate::topmost::is_own_window(fg) {
        tracker::external_foreground()
    } else {
        fg
    };
    let fullscreen = overlap::foreground_fullscreen(candidate)
        || unsafe { SHQueryUserNotificationState() }
            .is_ok_and(|state| state == QUNS_RUNNING_D3D_FULL_SCREEN);
    if ACTIVE.swap(fullscreen, Ordering::Relaxed) == fullscreen {
        return;
    }
    if fullscreen {
        crate::spotlight::close();
        crate::mission::close();
        crate::controlcenter::close();
    }
    azr_platform::log::line("fullscreen", if fullscreen { "entered" } else { "left" });
    crate::prefs::broadcast_message(WM_FULLSCREEN);
}
