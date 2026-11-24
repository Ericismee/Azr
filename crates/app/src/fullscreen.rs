use azr_platform::wins::{focus, overlap, tracker};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONULL};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_FULLSCREEN: u32 = WM_APP + 243;
static ACTIVE_MONITOR: AtomicIsize = AtomicIsize::new(0);

#[derive(Default)]
struct FullscreenDetection {
    exclusive_owner: Option<u32>,
    inactive_since: Option<Instant>,
}

impl FullscreenDetection {
    fn stabilize(&mut self, active: bool, detected: bool, now: Instant) -> bool {
        if detected || !active {
            self.inactive_since = None;
            return detected;
        }
        let since = self.inactive_since.get_or_insert(now);
        now.saturating_duration_since(*since) < Duration::from_millis(180)
    }

    fn observe(&mut self, foreground_pid: u32, geometry: bool, exclusive: bool) -> bool {
        if !exclusive {
            self.exclusive_owner = None;
            return geometry;
        }
        if foreground_pid == 0 {
            return geometry;
        }
        if geometry {
            self.exclusive_owner = Some(foreground_pid);
        }
        geometry || self.exclusive_owner == Some(foreground_pid)
    }
}

thread_local! {
    static DETECTION: std::cell::RefCell<FullscreenDetection> =
        std::cell::RefCell::new(FullscreenDetection::default());
}

pub fn active() -> bool {
    ACTIVE_MONITOR.load(Ordering::Relaxed) != 0
}

pub fn on_window(hwnd: HWND) -> bool {
    let monitor = ACTIVE_MONITOR.load(Ordering::Relaxed);
    monitor != 0 && unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL).0 as isize == monitor }
}

pub fn on_point(point: POINT) -> bool {
    let monitor = ACTIVE_MONITOR.load(Ordering::Relaxed);
    monitor != 0 && unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONULL).0 as isize == monitor }
}

pub fn on_selected() -> bool {
    if !active() {
        return false;
    }
    let (x, y) = crate::display::selected().origin();
    on_point(POINT { x, y })
}

pub fn on_pointer() -> bool {
    if !active() {
        return false;
    }
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).is_ok() && on_point(point) }
}

pub fn foreground_for_overlap() -> isize {
    let foreground = focus::foreground();
    if crate::topmost::is_own_window(foreground) {
        tracker::external_foreground()
    } else {
        foreground
    }
}

pub fn poll() {
    let candidate = focus::foreground();
    let geometry = overlap::foreground_fullscreen(candidate);
    let exclusive = unsafe { SHQueryUserNotificationState() }
        .is_ok_and(|state| state == QUNS_RUNNING_D3D_FULL_SCREEN);
    let mut pid = 0u32;
    if candidate != 0 {
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                windows::Win32::Foundation::HWND(candidate as *mut _),
                Some(&mut pid),
            );
        }
    }
    let fullscreen = DETECTION.with(|state| {
        let mut state = state.borrow_mut();
        let detected = state.observe(pid, geometry, exclusive);
        if detected {
            state.inactive_since = None;
            unsafe {
                MonitorFromWindow(HWND(candidate as *mut _), MONITOR_DEFAULTTONULL).0 as isize
            }
        } else if state.stabilize(active(), false, Instant::now()) {
            ACTIVE_MONITOR.load(Ordering::Relaxed)
        } else {
            0
        }
    });
    if ACTIVE_MONITOR.swap(fullscreen, Ordering::Relaxed) == fullscreen {
        return;
    }
    if fullscreen != 0 {
        crate::spotlight::close_for_fullscreen();
        crate::mission::close_for_fullscreen();
        crate::controlcenter::close_for_fullscreen();
    }
    azr_platform::log::line(
        "fullscreen",
        if fullscreen != 0 { "entered" } else { "left" },
    );
    crate::prefs::broadcast_message(WM_FULLSCREEN);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brief_focus_loss_preserves_fullscreen_but_sustained_loss_restores() {
        let mut state = FullscreenDetection::default();
        let now = Instant::now();
        assert!(state.stabilize(true, false, now));
        assert!(state.stabilize(true, true, now + Duration::from_millis(170)));
        assert!(state.stabilize(true, false, now + Duration::from_millis(200)));
        assert!(!state.stabilize(true, false, now + Duration::from_millis(380)));
    }

    #[test]
    fn shell_exclusive_hint_cannot_claim_an_unconfirmed_foreground_window() {
        let mut state = FullscreenDetection::default();
        assert!(!state.observe(13, false, true));
        assert!(state.observe(7, true, true));
        assert!(!state.observe(13, false, true));
    }

    #[test]
    fn exclusive_signal_does_not_follow_alt_tab_to_another_app() {
        let mut state = FullscreenDetection::default();
        assert!(state.observe(7, true, true));
        assert!(!state.observe(13, false, true));
        assert!(!state.observe(13, false, true));
        assert!(state.observe(7, false, true));
        assert!(!state.observe(13, false, false));
    }
}
