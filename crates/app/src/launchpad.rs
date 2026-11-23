use azr_core::spotlight::Mode;
use std::cell::Cell;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::Composition::Compositor;

const KEY_MASK_TAG: usize = 0x415A_5257;

pub const WM_WINDOWS_TAP: u32 = WM_APP + 167;

#[derive(Clone, Copy, Default)]
struct WinTap {
    held: u8,
    used: bool,
}

impl WinTap {
    fn event(&mut self, vk: u32, up: bool) -> bool {
        let bit = match vk {
            0x5B => 1,
            0x5C => 2,
            _ => {
                self.used |= self.held != 0;
                return false;
            }
        };
        if !up {
            self.used |= self.held != 0 && self.held & bit == 0;
            self.held |= bit;
            return false;
        }
        let tap = self.held == bit && !self.used;
        self.held &= !bit;
        if self.held == 0 {
            self.used = false;
        }
        tap
    }
}

thread_local! {
    static HOOK: Cell<Option<HHOOK>> = const { Cell::new(None) };
    static TAP: Cell<WinTap> = Cell::new(WinTap::default());
    static TARGET: Cell<HWND> = Cell::new(HWND::default());
}

pub fn init(_comp: &Compositor) -> windows_core::Result<()> {
    Ok(())
}

pub fn bind_windows_key(hwnd: HWND, enabled: bool) {
    if !enabled {
        shutdown();
        return;
    }
    TARGET.set(hwnd);
    HOOK.with(|slot| {
        if slot.get().is_none() {
            unsafe {
                let module = GetModuleHandleW(None).ok().map(Into::into);
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), module, 0) {
                    Ok(hook) => slot.set(Some(hook)),
                    Err(e) => {
                        azr_platform::log::line("launchpad", &format!("Windows key hook: {e}"))
                    }
                }
            }
        }
    });
}

pub fn shutdown() {
    if let Some(hook) = HOOK.take() {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }
    TAP.set(WinTap::default());
}

pub fn shortcut_label() -> &'static str {
    match crate::prefs::get().launchpad_key {
        1 => crate::prefs::tr("Windows key"),
        2 => "Ctrl+Alt+L",
        _ => "Ctrl+Shift+Space",
    }
}

pub fn toggle() {
    crate::spotlight::toggle_mode(Mode::Apps);
}

pub fn icon(px: u32, mode: u8, background: u8) -> Vec<u8> {
    let side = px as usize;
    if mode == 3 {
        if let Some(bitmap) = crate::menulogo::launcher_path()
            .and_then(|path| azr_platform::wic::decode_logo_file_fit(&path.to_string_lossy(), px))
        {
            let mut pixels = vec![0; side * side * 4];
            let (w, h) = (bitmap.w as usize, bitmap.h as usize);
            let (x, y) = ((side - w) / 2, (side - h) / 2);
            for row in 0..h {
                let start = ((row + y) * side + x) * 4;
                pixels[start..start + w * 4]
                    .copy_from_slice(&bitmap.bgra[row * w * 4..(row + 1) * w * 4]);
            }
            return pixels;
        }
    }
    if !matches!(mode, 1 | 2) {
        return azr_core::iconplate::launchpad_custom(px, background);
    }
    let mut pixels = vec![0; side * side * 4];
    for (i, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        let mut channels = [0u16; 4];
        for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let x = ((i % side) as f32 + dx) / px as f32;
            let y = ((i / side) as f32 + dy) / px as f32;
            for (total, value) in channels.iter_mut().zip(launcher_pixel(mode, x, y)) {
                *total += value as u16;
            }
        }
        for (channel, total) in pixel.iter_mut().zip(channels) {
            *channel = (total / 4) as u8;
        }
    }
    pixels
}

fn launcher_pixel(mode: u8, x: f32, y: f32) -> [u8; 4] {
    let shine = (1.0 - y).powi(2);
    if mode == 1 {
        if (x - 0.5).abs() <= 0.025
            || (y - 0.5).abs() <= 0.025
            || !(0.13..0.87).contains(&x)
            || !(0.13..0.87).contains(&y)
        {
            return [0; 4];
        }
        return [
            250,
            (119.0 + shine * 71.0) as u8,
            (15.0 + shine * 62.0) as u8,
            255,
        ];
    }
    let left = x < 0.5 + (0.46 - y) * 0.17;
    let eye =
        (x - 0.29).hypot((y - 0.35) * 0.63) < 0.026 || (x - 0.70).hypot((y - 0.34) * 0.63) < 0.026;
    let smile = (0.26..0.75).contains(&x) && (y - (0.67 - 0.79 * (x - 0.5).powi(2))).abs() < 0.019;
    if eye || smile {
        [108, 53, 25, 255]
    } else if left {
        [
            252,
            (145.0 + 66.0 * shine) as u8,
            (62.0 + 116.0 * shine) as u8,
            255,
        ]
    } else {
        [
            255,
            (229.0 + 25.0 * shine) as u8,
            (192.0 + 62.0 * shine) as u8,
            255,
        ]
    }
}

fn input(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                dwFlags: flags,
                dwExtraInfo: KEY_MASK_TAG,
                ..Default::default()
            },
        },
    }
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let key = &*(lp.0 as *const KBDLLHOOKSTRUCT);
        if key.dwExtraInfo != KEY_MASK_TAG {
            let up = matches!(wp.0 as u32, WM_KEYUP | WM_SYSKEYUP);
            let mut tap = TAP.get();
            if !up && matches!(key.vkCode, 0x5B | 0x5C) && tap.held == 0 {
                tap.used = [0x01, 0x02, 0x10, 0x11, 0x12]
                    .iter()
                    .any(|vk| GetAsyncKeyState(*vk) < 0);
            }
            let alone = tap.event(key.vkCode, up);
            TAP.set(tap);
            if alone {
                // Mark the key as used before releasing it so Explorer leaves Start closed.
                let events = [
                    input(0xE8, KEYBD_EVENT_FLAGS(0)),
                    input(0xE8, KEYEVENTF_KEYUP),
                    input(key.vkCode as u16, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP),
                ];
                if SendInput(&events, size_of::<INPUT>() as i32) == events.len() as u32 {
                    let _ = PostMessageW(Some(TARGET.get()), WM_WINDOWS_TAP, WPARAM(0), LPARAM(0));
                    return LRESULT(1);
                }
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unaccompanied_windows_key_releases_open_launchpad() {
        let mut tap = WinTap::default();
        assert!(!tap.event(0x5B, false));
        assert!(!tap.event(0x5B, false));
        assert!(tap.event(0x5B, true));
        assert!(!tap.event(0x5C, false));
        assert!(!tap.event(b'D' as u32, false));
        assert!(!tap.event(0x5C, true));
        assert!(!tap.event(0x5B, false));
        assert!(!tap.event(0x5C, false));
        assert!(!tap.event(0x5B, true));
        assert!(!tap.event(0x5C, true));
    }
}
