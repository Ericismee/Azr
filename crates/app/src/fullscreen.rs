use azr_platform::wins::{focus, overlap, tracker};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_FULLSCREEN: u32 = WM_APP + 243;
static ACTIVE: AtomicBool = AtomicBool::new(false);

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
    ACTIVE.load(Ordering::Relaxed)
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
        state.stabilize(active(), detected, Instant::now())
    });
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
