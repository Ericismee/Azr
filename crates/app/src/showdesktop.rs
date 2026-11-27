use azr_platform::wins::{enumerate, focus};
use std::sync::Mutex;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE, SW_SHOWNOACTIVATE};

struct State {
    shown: bool,
    hidden: Vec<isize>,
}

static STATE: Mutex<State> = Mutex::new(State {
    shown: false,
    hidden: Vec::new(),
});

/// Toggles "Show Desktop". The first call minimizes every app window and
/// remembers it; the next call restores exactly those windows.
///
/// Windows owned by Azr (dock, menu bar, widgets) are excluded because
/// `enumerate::list` skips our own process, so the desktop stays intact.
pub fn toggle() {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !st.shown {
        st.hidden.clear();
        for w in enumerate::list(std::process::id()) {
            if focus::is_minimized(w.hwnd) {
                continue;
            }
            unsafe {
                let _ = ShowWindow(HWND(w.hwnd as *mut _), SW_MINIMIZE);
            }
            st.hidden.push(w.hwnd);
        }
        st.shown = true;
    } else {
        let mut first = 0;
        for h in st.hidden.drain(..) {
            if focus::is_alive(h) && focus::is_minimized(h) {
                unsafe {
                    let _ = ShowWindow(HWND(h as *mut _), SW_SHOWNOACTIVATE);
                }
                if first == 0 {
                    first = h;
                }
            }
        }
        st.shown = false;
        if first != 0 {
            focus::force_foreground(first);
        }
    }
}
