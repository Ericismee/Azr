use crate::notch::draw::{Anchor, Painter};
use crate::popup::{work_area_at, Popup};
use crate::prefs::{self, Role};
use azr_core::symbols::Sym;
use azr_ui::gfx::Gfx;
use std::cell::RefCell;
use windows::core::{w, Error, Result, HRESULT, HSTRING};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, ERROR_NOT_SUPPORTED, ERROR_SUCCESS, HANDLE, HWND,
    LPARAM, LRESULT, LUID, POINT, WPARAM,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::{
    Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
    Power::{GetPwrCapabilities, SetSuspendState, SYSTEM_POWER_CAPABILITIES},
    Shutdown::{ExitWindowsEx, EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, SHTDN_REASON_FLAG_PLANNED},
    Threading::{GetCurrentProcess, OpenProcessToken},
};
use windows::Win32::UI::Shell::{IShellDispatch, Shell};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::{Color, Composition::Compositor};

const WIDTH: f32 = 416.0;
const HEIGHT: f32 = 228.0;
const BUTTON_Y: f32 = 164.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_WIDTH: f32 = 176.0;
const BUTTON_LEFT: [f32; 2] = [26.0, 214.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Sleep,
    Restart,
    ShutDown,
    SignOut,
}

impl Action {
    fn needs_confirmation(self, quick: bool) -> bool {
        match self {
            Self::Sleep => false,
            Self::Restart | Self::ShutDown => !quick,
            Self::SignOut => true,
        }
    }

    fn labels(self) -> (&'static str, &'static str, Sym) {
        match self {
            Self::Restart => ("Restart the computer now?", "Restart", Sym::Restart),
            Self::ShutDown => ("Shut down the computer now?", "Shut Down", Sym::Power),
            Self::SignOut => ("Sign out now?", "Sign Out", Sym::Power),
            Self::Sleep => ("Sleep now?", "Sleep", Sym::Power),
        }
    }

    fn execute(self) -> Result<()> {
        if self != Self::Sleep {
            return self.execute_native();
        }
        let mut capabilities = SYSTEM_POWER_CAPABILITIES::default();
        let modern = unsafe { GetPwrCapabilities(&mut capabilities) } && capabilities.AoAc;
        if use_windows_sleep_dialog(modern, None) {
            return show_windows_sleep_dialog();
        }
        match self.execute_native() {
            Err(error) if use_windows_sleep_dialog(false, Some(error.code())) => {
                show_windows_sleep_dialog()
            }
            result => result,
        }
    }

    fn execute_native(self) -> Result<()> {
        unsafe {
            if self == Self::SignOut {
                return ExitWindowsEx(EWX_LOGOFF, SHTDN_REASON_FLAG_PLANNED);
            }
            let mut luid = LUID::default();
            LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut luid)?;
            let mut token = HANDLE::default();
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )?;
            let result = self.execute_with_token(token, luid);
            let _ = CloseHandle(token);
            result
        }
    }

    unsafe fn execute_with_token(self, token: HANDLE, luid: LUID) -> Result<()> {
        let requested = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let mut previous = TOKEN_PRIVILEGES::default();
        let mut len = 0;
        SetLastError(ERROR_SUCCESS);
        AdjustTokenPrivileges(
            token,
            false,
            Some(&requested),
            size_of::<TOKEN_PRIVILEGES>() as u32,
            Some(&mut previous),
            Some(&mut len),
        )?;
        if GetLastError() != ERROR_SUCCESS {
            return Err(Error::from_thread());
        }
        let result = match self {
            Self::Sleep => {
                if SetSuspendState(false, false, false) {
                    Ok(())
                } else {
                    Err(Error::from_thread())
                }
            }
            Self::Restart => ExitWindowsEx(EWX_REBOOT, SHTDN_REASON_FLAG_PLANNED),
            Self::ShutDown => ExitWindowsEx(EWX_POWEROFF, SHTDN_REASON_FLAG_PLANNED),
            Self::SignOut => unreachable!(),
        };
        let _ = AdjustTokenPrivileges(token, false, Some(&previous), 0, None, None);
        result
    }
}

fn use_windows_sleep_dialog(modern_standby: bool, error: Option<HRESULT>) -> bool {
    modern_standby || error == Some(HRESULT::from_win32(ERROR_NOT_SUPPORTED.0))
}

fn show_windows_sleep_dialog() -> Result<()> {
    azr_platform::log::line("power", "Sleep: opening the Windows power dialog");
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let result = CoCreateInstance::<_, IShellDispatch>(&Shell, None, CLSCTX_ALL)
            .and_then(|shell| shell.ShutdownWindows());
        CoUninitialize();
        result
    }
}

struct Dialog {
    popup: Popup,
    paint: Painter,
    action: Action,
    selected: usize,
    pressed: Option<usize>,
}

impl Dialog {
    fn paint(&mut self) {
        let c = &self.popup.content;
        if let Ok(children) = c.Children() {
            let _ = children.RemoveAll();
        }
        let look = prefs::look(Role::Themed);
        let fg = look.fg;
        let d = self.paint.dpi;
        let (title, confirm, icon) = self.action.labels();
        let color = |rgb: [u8; 3], a| Color {
            A: a,
            R: rgb[0],
            G: rgb[1],
            B: rgb[2],
        };
        self.paint.rect(
            c,
            184.0 * d,
            22.0 * d,
            48.0 * d,
            48.0 * d,
            15.0 * d,
            prefs::accent_c(55),
        );
        self.paint
            .sym(c, icon, WIDTH / 2.0 * d, 46.0 * d, 25.0, fg, 1.0);
        self.paint.text(
            c,
            prefs::tr(title),
            WIDTH / 2.0 * d,
            84.0 * d,
            18.0,
            700,
            fg,
            1.0,
            Anchor::Center,
            (WIDTH - 44.0) * d,
        );
        self.paint.text(
            c,
            prefs::tr("Save any open work before continuing."),
            WIDTH / 2.0 * d,
            116.0 * d,
            13.0,
            400,
            fg,
            0.82,
            Anchor::Center,
            (WIDTH - 44.0) * d,
        );
        for (id, label) in ["Cancel", confirm].into_iter().enumerate() {
            let left = BUTTON_LEFT[id] * d;
            if id == self.selected {
                self.paint.rect(
                    c,
                    left - 2.0 * d,
                    (BUTTON_Y - 2.0) * d,
                    (BUTTON_WIDTH + 4.0) * d,
                    (BUTTON_HEIGHT + 4.0) * d,
                    14.0 * d,
                    prefs::accent_c(225),
                );
            }
            let bg = if id == 0 {
                color(
                    if look.dark {
                        [52, 55, 64]
                    } else {
                        [235, 238, 245]
                    },
                    255,
                )
            } else {
                color([24, 81, 174], 255)
            };
            let text = if id == 0 { fg } else { [255; 3] };
            self.paint.rect(
                c,
                left,
                BUTTON_Y * d,
                BUTTON_WIDTH * d,
                BUTTON_HEIGHT * d,
                12.0 * d,
                bg,
            );
            self.paint.text(
                c,
                prefs::tr(label),
                left + BUTTON_WIDTH * d / 2.0,
                (BUTTON_Y + 10.0) * d,
                13.0,
                600,
                text,
                1.0,
                Anchor::Center,
                (BUTTON_WIDTH - 16.0) * d,
            );
        }
    }

    fn button_at(&self, l: LPARAM) -> Option<usize> {
        let x = f32::from(l.0 as i16) / self.paint.dpi;
        let y = f32::from((l.0 >> 16) as i16) / self.paint.dpi;
        if !(BUTTON_Y..BUTTON_Y + BUTTON_HEIGHT).contains(&y) {
            return None;
        }
        BUTTON_LEFT
            .iter()
            .position(|left| (*left..*left + BUTTON_WIDTH).contains(&x))
    }
}

impl Drop for Dialog {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.popup.hwnd);
        }
    }
}

thread_local! {
    static DIALOG: RefCell<Option<Dialog>> = const { RefCell::new(None) };
}

fn with_dialog<T>(hwnd: HWND, f: impl FnOnce(&mut Dialog) -> T) -> Option<T> {
    DIALOG.with(|cell| {
        cell.try_borrow_mut()
            .ok()?
            .as_mut()
            .filter(|d| d.popup.hwnd == hwnd)
            .map(f)
    })
}

fn finish(hwnd: HWND, accepted: bool) {
    let dialog = DIALOG.with(|cell| {
        let mut dialog = cell.borrow_mut();
        if dialog
            .as_ref()
            .is_some_and(|dialog| dialog.popup.hwnd == hwnd)
        {
            dialog.take()
        } else {
            None
        }
    });
    if let Some(dialog) = dialog {
        let action = dialog.action;
        drop(dialog);
        if accepted {
            run(action);
        }
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => finish(hwnd, false),
        WM_KEYDOWN => match wparam.0 {
            0x1B => finish(hwnd, false),
            0x09 | 0x25 | 0x27 => {
                with_dialog(hwnd, |dialog| {
                    dialog.selected = 1 - dialog.selected;
                    dialog.paint();
                });
            }
            0x0D | 0x20 => {
                let yes = with_dialog(hwnd, |dialog| dialog.selected == 1).unwrap_or(false);
                finish(hwnd, yes);
            }
            _ => {}
        },
        WM_LBUTTONDOWN => {
            with_dialog(hwnd, |dialog| dialog.pressed = dialog.button_at(lparam));
        }
        WM_LBUTTONUP => {
            let clicked = with_dialog(hwnd, |dialog| {
                dialog
                    .pressed
                    .take()
                    .filter(|id| Some(*id) == dialog.button_at(lparam))
            })
            .flatten();
            if let Some(id) = clicked {
                finish(hwnd, id == 1);
            }
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }
    LRESULT(0)
}

fn show(comp: &Compositor, action: Action) -> Result<()> {
    let old = DIALOG.with(|dialog| dialog.borrow_mut().take());
    drop(old);
    let display = crate::display::selected();
    let (left, top, right, bottom) = display.edges();
    let area = work_area_at(POINT {
        x: (left + right) / 2,
        y: (top + bottom) / 2,
    });
    let dpi = display
        .dpi()
        .min((area.right - area.left) as f32 / (WIDTH + 16.0))
        .min((area.bottom - area.top) as f32 / (HEIGHT + 16.0));
    let (w, h) = ((WIDTH * dpi).round() as i32, (HEIGHT * dpi).round() as i32);
    let paint = Painter::new(comp.clone(), Gfx::shared(comp)?, dpi);
    let popup = Popup::new_with(
        comp,
        w!("AzrPowerDialog"),
        Some(proc),
        WINDOW_EX_STYLE(0),
        prefs::glass((w as f32, h as f32), 25.0 * dpi, Role::Themed),
        true,
    )?;
    let mut dialog = Dialog {
        popup,
        action,
        paint,
        selected: 0,
        pressed: None,
    };
    dialog.popup.set_rect(
        area.left + (area.right - area.left - w) / 2,
        area.top + (area.bottom - area.top - h) / 2,
        w,
        h,
    )?;
    dialog.paint();
    let hwnd = dialog.popup.hwnd;
    DIALOG.with(|cell| *cell.borrow_mut() = Some(dialog));
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

pub fn request(comp: &Compositor, action: Action) {
    if action.needs_confirmation(prefs::get().quick_power_actions) {
        if let Err(err) = show(comp, action) {
            azr_platform::log::line("power", &format!("Could not open confirmation: {err}"));
            let (title, _, _) = action.labels();
            let title = HSTRING::from(prefs::tr(title));
            std::thread::spawn(move || unsafe {
                if MessageBoxW(
                    None,
                    &title,
                    w!("Azr"),
                    MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION | MB_TOPMOST,
                ) == IDYES
                {
                    run(action);
                }
            });
        }
    } else {
        run(action);
    }
}

fn run(action: Action) {
    let message = HSTRING::from(prefs::tr("Windows could not complete this power action."));
    std::thread::spawn(move || {
        if let Err(err) = action.execute() {
            azr_platform::log::line("power", &format!("{action:?}: {err}"));
            unsafe {
                MessageBoxW(None, &message, w!("Azr"), MB_ICONERROR | MB_TOPMOST);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_standby_and_unsupported_sleep_use_windows_power_dialog() {
        assert!(use_windows_sleep_dialog(true, None));
        assert!(use_windows_sleep_dialog(
            false,
            Some(HRESULT::from_win32(ERROR_NOT_SUPPORTED.0))
        ));
        assert!(!use_windows_sleep_dialog(false, None));
        assert!(!use_windows_sleep_dialog(
            false,
            Some(HRESULT::from_win32(5))
        ));
    }

    #[test]
    fn quick_power_only_skips_shutdown_and_restart_confirmation() {
        for action in [Action::Restart, Action::ShutDown] {
            assert!(action.needs_confirmation(false));
            assert!(!action.needs_confirmation(true));
        }
        assert!(!Action::Sleep.needs_confirmation(false));
        assert!(Action::SignOut.needs_confirmation(true));
    }
}
