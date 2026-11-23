use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, IsZoomed, GWL_EXSTYLE, GWL_STYLE, WS_CAPTION, WS_EX_TOOLWINDOW,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overlap {
    pub overlapped: bool,
    pub fullscreen: bool,
}

fn class_of(h: HWND) -> String {
    let mut b = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut b) };
    String::from_utf16_lossy(&b[..n.max(0) as usize])
}

fn is_shell_class(c: &str) -> bool {
    matches!(
        c,
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Windows.UI.Core.CoreWindow"
            | "SimpleWindow"
    )
}

fn frame_rect(h: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe {
        if DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_ok()
        {
            return Some(r);
        }
        GetWindowRect(h, &mut r).ok()?;
    }
    Some(r)
}

fn covers_monitor(frame: RECT, monitor: RECT, maximized: bool, captioned: bool) -> bool {
    const BORDER_TOLERANCE: i32 = 8;
    !(maximized && captioned)
        && frame.left <= monitor.left + BORDER_TOLERANCE
        && frame.top <= monitor.top + BORDER_TOLERANCE
        && frame.right >= monitor.right - BORDER_TOLERANCE
        && frame.bottom >= monitor.bottom - BORDER_TOLERANCE
}

struct Foreground {
    frame: RECT,
    maximized: bool,
    fullscreen: bool,
}

fn on_monitor(fg: isize, monitor: HMONITOR) -> Option<Foreground> {
    let h = HWND(fg as *mut _);
    unsafe {
        if fg == 0
            || monitor.is_invalid()
            || !IsWindowVisible(h).as_bool()
            || IsIconic(h).as_bool()
            || MonitorFromWindow(h, MONITOR_DEFAULTTONULL) != monitor
        {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == std::process::id() || is_shell_class(&class_of(h)) {
            return None;
        }
        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, 4)
            .is_ok()
            && cloaked != 0
        {
            return None;
        }
        let frame = frame_rect(h)?;
        let maximized = IsZoomed(h).as_bool();
        let captioned = (GetWindowLongW(h, GWL_STYLE) as u32 & WS_CAPTION.0) == WS_CAPTION.0;
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let fullscreen = GetMonitorInfoW(monitor, &mut info).as_bool()
            && covers_monitor(frame, info.rcMonitor, maximized, captioned);
        if !fullscreen && GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
            return None;
        }
        Some(Foreground {
            frame,
            maximized,
            fullscreen,
        })
    }
}

pub fn foreground_fullscreen(fg: isize) -> bool {
    let monitor = unsafe { MonitorFromWindow(HWND(fg as *mut _), MONITOR_DEFAULTTONULL) };
    on_monitor(fg, monitor).is_some_and(|window| window.fullscreen)
}

pub fn primary_fullscreen(fg: isize) -> bool {
    let monitor = unsafe { MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY) };
    on_monitor(fg, monitor).is_some_and(|window| window.fullscreen)
}

pub fn fullscreen_on(fg: isize, surface: HWND) -> bool {
    let monitor = unsafe { MonitorFromWindow(surface, MONITOR_DEFAULTTONULL) };
    on_monitor(fg, monitor).is_some_and(|window| window.fullscreen)
}

pub fn evaluate(fg: isize, surface: HWND, area: (i32, i32, i32, i32)) -> Overlap {
    let monitor = unsafe { MonitorFromWindow(surface, MONITOR_DEFAULTTONULL) };
    let Some(window) = on_monitor(fg, monitor) else {
        return Overlap::default();
    };
    let r = window.frame;
    let hit = r.bottom > area.1 && r.top < area.3 && r.right > area.0 && r.left < area.2;
    Overlap {
        overlapped: window.fullscreen || window.maximized || hit,
        fullscreen: window.fullscreen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn accepts_small_borderless_gaps_but_not_a_window_leaving_the_taskbar_space() {
        let monitor = rect(-1920, 0, 0, 1080);
        assert!(covers_monitor(
            rect(-1918, 1, -3, 1077),
            monitor,
            false,
            false
        ));
        assert!(!covers_monitor(
            rect(-1920, 0, 0, 1040),
            monitor,
            false,
            false
        ));
        assert!(!covers_monitor(
            rect(-1918, 1, -3, 1077),
            monitor,
            true,
            true
        ));
    }

    #[test]
    fn detects_borderless_fullscreen_on_negative_origin_monitor() {
        let monitor = rect(-1920, -200, 0, 880);
        assert!(covers_monitor(monitor, monitor, false, false));
        assert!(covers_monitor(
            rect(-1928, -208, 8, 888),
            monitor,
            true,
            false
        ));
    }

    #[test]
    fn rejects_maximized_captioned_windows_and_partial_coverage() {
        let monitor = rect(0, 0, 1920, 1080);
        assert!(!covers_monitor(monitor, monitor, true, true));
        assert!(!covers_monitor(
            rect(0, 0, 1920, 1040),
            monitor,
            false,
            false
        ));
    }
}
