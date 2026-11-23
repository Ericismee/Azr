use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

#[derive(Clone, Copy)]
pub struct Display {
    pub id: u8,
    pub bounds: RECT,
    pub primary: bool,
    monitor: HMONITOR,
}

impl Display {
    pub fn size(self) -> (i32, i32) {
        (
            self.bounds.right - self.bounds.left,
            self.bounds.bottom - self.bounds.top,
        )
    }

    pub fn origin(self) -> (i32, i32) {
        (self.bounds.left, self.bounds.top)
    }

    pub fn edges(self) -> (i32, i32, i32, i32) {
        (
            self.bounds.left,
            self.bounds.top,
            self.bounds.right,
            self.bounds.bottom,
        )
    }

    pub fn dpi(self) -> f32 {
        let (mut x, mut y) = (96, 96);
        let _ = unsafe { GetDpiForMonitor(self.monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) };
        x.max(96) as f32 / 96.0
    }
}

unsafe extern "system" fn collect(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    context: LPARAM,
) -> BOOL {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(monitor, &mut info.monitorInfo).as_bool() {
        let name = String::from_utf16_lossy(&info.szDevice);
        if let Some(id) = name
            .trim_end_matches('\0')
            .rsplit_once("DISPLAY")
            .and_then(|(_, n)| n.parse().ok())
        {
            let list = &mut *(context.0 as *mut Vec<Display>);
            list.push(Display {
                id,
                bounds: info.monitorInfo.rcMonitor,
                primary: info.monitorInfo.dwFlags & 1 != 0,
                monitor,
            });
        }
    }
    BOOL(1)
}

pub fn list() -> Vec<Display> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors.sort_unstable_by_key(|m: &Display| m.id);
    monitors
}

pub fn selected() -> Display {
    let id = crate::prefs::get().main_display;
    let monitors = list();
    monitors
        .iter()
        .find(|m| m.id == id)
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
        .copied()
        .unwrap_or_else(|| {
            let (right, bottom) = azr_platform::sys::screen_size();
            Display {
                id: 0,
                bounds: RECT {
                    left: 0,
                    top: 0,
                    right,
                    bottom,
                },
                primary: true,
                monitor: HMONITOR::default(),
            }
        })
}
