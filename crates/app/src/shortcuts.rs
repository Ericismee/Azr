use crate::prefs;
use azr_core::settings::Settings;
use azr_core::shortcuts::{Chord, Command, ALT, CTRL, SHIFT};
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, AtomicU16, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, PostMessageW, WM_APP};

const BASE: i32 = 0xAB01;
const SPARE: i32 = 16;
pub const WM_CAPTURED: u32 = WM_APP + 177;

#[derive(Clone, Copy)]
struct Binding {
    id: i32,
    chord: Chord,
}

struct Controller {
    hwnd: HWND,
    bindings: [Option<Binding>; 3],
    windows_key: bool,
    last_requested: [Option<Chord>; 3],
}

thread_local! {
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}

static CAPTURE: AtomicIsize = AtomicIsize::new(0);
static CONSUMED_KEY: AtomicU16 = AtomicU16::new(0);

fn effective(settings: &Settings, command: Command) -> Chord {
    settings.shortcuts[command.index()]
        .unwrap_or_else(|| command.default_chord(settings.launchpad_key))
}

impl Controller {
    fn bind(&mut self, command: Command, chord: Chord) -> bool {
        let index = command.index();
        let old = self.bindings[index];
        if old.is_some_and(|b| b.chord == chord) {
            return true;
        }
        let primary = BASE + index as i32;
        let id = if old.is_some_and(|b| b.id == primary) {
            primary + SPARE
        } else {
            primary
        };
        unsafe {
            let modifiers = HOT_KEY_MODIFIERS(u32::from(chord.modifiers)) | MOD_NOREPEAT;
            if RegisterHotKey(Some(self.hwnd), id, modifiers, u32::from(chord.key)).is_err() {
                return false;
            }
            if let Some(old) = old {
                let _ = UnregisterHotKey(Some(self.hwnd), old.id);
            }
        }
        self.bindings[index] = Some(Binding { id, chord });
        true
    }

    fn sync_windows(&mut self, settings: &Settings) {
        let enabled =
            settings.launchpad_key == 1 && settings.shortcuts[Command::Launchpad.index()].is_none();
        if self.windows_key != enabled {
            crate::launchpad::bind_windows_key(self.hwnd, enabled);
            self.windows_key = enabled;
        }
    }
}

pub fn init(hwnd: HWND) {
    CONTROLLER.with(|slot| {
        *slot.borrow_mut() = Some(Controller {
            hwnd,
            bindings: [None; 3],
            windows_key: false,
            last_requested: [None; 3],
        });
    });
    sync();
}

pub fn sync() {
    let settings = prefs::get();
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow_mut().as_mut() {
            for command in Command::ALL {
                let chord = effective(&settings, command);
                let previous = &mut controller.last_requested[command.index()];
                if previous.replace(chord) != Some(chord) {
                    let _ = controller.bind(command, chord);
                }
            }
            controller.sync_windows(&settings);
        }
    });
}

pub fn shutdown() {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow_mut().take() {
            for binding in controller.bindings.into_iter().flatten() {
                unsafe {
                    let _ = UnregisterHotKey(Some(controller.hwnd), binding.id);
                }
            }
        }
    });
}

fn apply(next: Settings, command: Command) -> bool {
    let chord = effective(&next, command);
    if Command::ALL
        .into_iter()
        .any(|other| other != command && effective(&next, other) == chord)
    {
        return false;
    }
    let bound = CONTROLLER.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .is_some_and(|controller| controller.bind(command, chord))
    });
    if bound {
        prefs::update(|settings| {
            settings.shortcuts = next.shortcuts;
            settings.launchpad_key = next.launchpad_key;
        });
        sync();
    }
    bound
}

pub fn set(command: Command, chord: Option<Chord>) -> bool {
    let mut next = prefs::get();
    next.shortcuts[command.index()] = chord;
    apply(next, command)
}

pub fn set_launchpad_preset(key: u8) -> bool {
    let mut next = prefs::get();
    next.launchpad_key = key;
    next.shortcuts[Command::Launchpad.index()] = None;
    apply(next, Command::Launchpad)
}

pub fn invoke(id: i32) {
    let command = CONTROLLER.with(|slot| {
        let slot = slot.borrow();
        let controller = slot.as_ref()?;
        Command::ALL
            .into_iter()
            .find(|c| controller.bindings[c.index()].is_some_and(|binding| binding.id == id))
    });
    match command {
        Some(Command::Search) => crate::spotlight::toggle(),
        Some(Command::Launchpad) => crate::launchpad::toggle(),
        Some(Command::Overview) => crate::mission::toggle(),
        None => {}
    }
}

pub fn configured_label(command: Command) -> String {
    let settings = prefs::get();
    if command == Command::Launchpad
        && settings.launchpad_key == 1
        && settings.shortcuts[command.index()].is_none()
    {
        return prefs::tr("Windows key").to_owned();
    }
    effective(&settings, command).label()
}

pub fn active_label(command: Command) -> Option<String> {
    let settings = prefs::get();
    let active = CONTROLLER.with(|slot| {
        slot.borrow().as_ref().is_some_and(|c| {
            (command == Command::Launchpad && c.windows_key)
                || c.bindings[command.index()]
                    .is_some_and(|binding| binding.chord == effective(&settings, command))
        })
    });
    active.then(|| configured_label(command))
}

pub fn begin_capture(hwnd: HWND) {
    CAPTURE.store(hwnd.0 as isize, Ordering::Relaxed);
}

pub fn end_capture(hwnd: HWND) {
    let _ = CAPTURE.compare_exchange(hwnd.0 as isize, 0, Ordering::Relaxed, Ordering::Relaxed);
}

pub fn capture_key(vk: u16, down: bool, alt: bool, ctrl: bool, shift: bool) -> bool {
    if !down
        && CONSUMED_KEY
            .compare_exchange(vk, 0, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        return true;
    }
    if down && CONSUMED_KEY.load(Ordering::Relaxed) == vk {
        return true;
    }
    let window = CAPTURE.load(Ordering::Relaxed);
    if window == 0 || window != (unsafe { GetForegroundWindow() }.0 as isize) {
        return false;
    }
    if matches!(vk, 0x10..=0x12 | 0xA0..=0xA5) {
        return false;
    }
    if !down {
        return matches!(vk, 0x5B | 0x5C);
    }
    CONSUMED_KEY.store(vk, Ordering::Relaxed);
    let modifiers = (if alt { ALT } else { 0 })
        | (if ctrl { CTRL } else { 0 })
        | (if shift { SHIFT } else { 0 });
    unsafe {
        let _ = PostMessageW(
            Some(HWND(window as *mut _)),
            WM_CAPTURED,
            WPARAM(vk as usize),
            LPARAM(modifiers as isize),
        );
    }
    true
}
