use crate::prefs;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LMENU, VK_MENU, VK_RMENU, VK_SHIFT, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_CYCLE: u32 = WM_APP + 125;
const WM_OVERVIEW: u32 = WM_APP + 126;
const WM_TASK_MANAGER: u32 = WM_APP + 127;
const WM_OVERVIEW_ACCEPT: u32 = WM_APP + 128;
const T_TASK_MANAGER: usize = 1;

static TARGET: AtomicIsize = AtomicIsize::new(0);
static HOOK: AtomicIsize = AtomicIsize::new(0);
static OVERVIEW_KEY: AtomicBool = AtomicBool::new(false);
static CYCLE_KEY: AtomicBool = AtomicBool::new(false);
static ALT_SWITCHING: AtomicBool = AtomicBool::new(false);
static TAB_HELD: AtomicBool = AtomicBool::new(false);
static CYCLE_HELD: AtomicBool = AtomicBool::new(false);

pub fn init() -> windows_core::Result<()> {
    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class = w!("AzrSwitcherCtl");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(ctl_proc),
            hInstance: instance,
            lpszClassName: class,
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("Azr-switcher"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )?;
        prefs::listen(hwnd);
        TARGET.store(hwnd.0 as isize, Ordering::Relaxed);
        sync();
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), Some(instance), 0)?;
        HOOK.store(hook.0 as isize, Ordering::Relaxed);
    }
    Ok(())
}

fn sync() {
    let settings = prefs::get();
    OVERVIEW_KEY.store(settings.overview_alt_tab, Ordering::Relaxed);
    CYCLE_KEY.store(settings.mac_shortcuts, Ordering::Relaxed);
}

pub fn shutdown() {
    let hook = HOOK.swap(0, Ordering::Relaxed);
    if hook != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(hook as *mut _));
        }
    }
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let key = &*(lp.0 as *const KBDLLHOOKSTRUCT);
        let alt = key.flags.contains(LLKHF_ALTDOWN);
        let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let vk = key.vkCode as u16;
        let target = HWND(TARGET.load(Ordering::Relaxed) as *mut _);
        let ctrl = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
        let shift = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
        if crate::shortcuts::capture_key(vk, down, alt, ctrl, shift) {
            return LRESULT(1);
        }

        if vk == VK_ESCAPE.0 && ctrl && shift && down {
            // Windows handles the original shortcut. Recover an existing hidden window afterwards.
            let _ = PostMessageW(Some(target), WM_TASK_MANAGER, WPARAM(0), LPARAM(0));
        }

        if !down
            && [VK_MENU.0, VK_LMENU.0, VK_RMENU.0].contains(&vk)
            && ALT_SWITCHING.swap(false, Ordering::Relaxed)
        {
            let _ = PostMessageW(Some(target), WM_OVERVIEW_ACCEPT, WPARAM(0), LPARAM(0));
        }

        let binding = if vk == VK_TAB.0 {
            Some((&OVERVIEW_KEY, &TAB_HELD, WM_OVERVIEW))
        } else if key.scanCode == 0x29 || vk == 0xC0 {
            Some((&CYCLE_KEY, &CYCLE_HELD, WM_CYCLE))
        } else {
            None
        };
        if let Some((enabled, held, msg)) = binding {
            if down
                && alt
                && !ctrl
                && enabled.load(Ordering::Relaxed)
                && (msg != WM_OVERVIEW || !crate::fullscreen::active())
            {
                if !held.swap(true, Ordering::Relaxed) {
                    if msg == WM_OVERVIEW {
                        ALT_SWITCHING.store(true, Ordering::Relaxed);
                    }
                    let _ = PostMessageW(Some(target), msg, WPARAM(shift as usize), LPARAM(0));
                }
                return LRESULT(1);
            }
            if !down && held.swap(false, Ordering::Relaxed) {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

unsafe extern "system" fn ctl_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        prefs::WM_PREFS => sync(),
        WM_CYCLE => crate::macshortcuts::cycle(wp.0 != 0),
        WM_OVERVIEW => crate::mission::step_switch(wp.0 != 0),
        WM_OVERVIEW_ACCEPT => crate::mission::accept_alt_tab(),
        WM_TASK_MANAGER => {
            SetTimer(Some(hwnd), T_TASK_MANAGER, 350, None);
        }
        WM_TIMER if wp.0 == T_TASK_MANAGER => {
            let _ = KillTimer(Some(hwnd), T_TASK_MANAGER);
            azr_platform::launch::reveal_task_manager();
        }
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    LRESULT(0)
}
