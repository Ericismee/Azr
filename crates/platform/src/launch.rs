use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_ESCAPE,
};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, SC_TASKLIST,
    SW_SHOWNORMAL, WM_SYSCOMMAND,
};

fn off_thread(job: impl FnOnce() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("Azr-launch".into())
        .spawn(move || {
            use windows::Win32::System::Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            };
            let com =
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
                    .is_ok();
            job();
            if com {
                unsafe { CoUninitialize() };
            }
        });
    if let Err(e) = spawned {
        crate::log::line("launch", &format!("thread gagal: {e}"));
    }
}

fn shell_options(file: &HSTRING, params: Option<&HSTRING>) -> SHELLEXECUTEINFOW {
    SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("open"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: params.map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    }
}

fn dispatch(file: String, params: Option<String>) {
    off_thread(move || {
        let target = HSTRING::from(file.as_str());
        let parameters = params.map(HSTRING::from);
        let mut exec = shell_options(&target, parameters.as_ref());
        match unsafe { ShellExecuteExW(&mut exec) } {
            Ok(()) => crate::log::line("launch", "Windows accepted the launch request"),
            Err(error) => crate::log::line(
                "launch",
                &format!("Windows rejected the launch request: {error}"),
            ),
        }
    });
}

pub fn open(path: &str) {
    dispatch(path.to_owned(), None);
}

pub fn is_start_open() -> bool {
    let mut pid = 0u32;
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        GetWindowThreadProcessId(fg, Some(&mut pid));
    }
    let Some(path) = crate::wins::enumerate::exe_path_of_pid(pid) else {
        return false;
    };
    let file = path.rsplit('\\').next().unwrap_or("").to_lowercase();
    file == "startmenuexperiencehost.exe" || file == "searchhost.exe"
}

fn tap(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) {
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let seq = [key(Default::default()), key(KEYEVENTF_KEYUP)];
    unsafe {
        SendInput(&seq, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn toggle_start(was_open_at_press: bool) {
    if is_start_open() {
        tap(VK_ESCAPE);
    } else if !was_open_at_press {
        unsafe {
            if let Ok(tray) = FindWindowW(w!("Shell_TrayWnd"), None) {
                let _ = PostMessageW(
                    Some(tray),
                    WM_SYSCOMMAND,
                    WPARAM(SC_TASKLIST as usize),
                    LPARAM(0),
                );
            }
        }
    }
}

pub fn reveal_task_manager() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindowAsync, SW_RESTORE, SW_SHOW,
    };
    unsafe {
        let Ok(window) = FindWindowW(w!("TaskManagerWindow"), None) else {
            return false;
        };
        if IsIconic(window).as_bool() {
            let _ = ShowWindowAsync(window, SW_RESTORE);
        } else if !IsWindowVisible(window).as_bool() {
            let _ = ShowWindowAsync(window, SW_SHOW);
        }
        SetForegroundWindow(window).as_bool()
    }
}

pub fn is_task_manager_path(path: &str) -> bool {
    path.rsplit(['\\', '/'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("taskmgr.exe"))
}

pub fn task_manager() {
    if reveal_task_manager() {
        crate::log::line("launch", "Task Manager window activated");
    } else {
        crate::log::line("launch", "Task Manager requested from Windows");
        run("taskmgr.exe", "");
    }
}

pub fn run(file: &str, params: &str) {
    dispatch(
        system_tool(file),
        (!params.is_empty()).then(|| params.to_owned()),
    );
}

fn system_tool(file: &str) -> String {
    if file.contains(['\\', '/', ':']) {
        return file.to_string();
    }
    if let Some(root) = std::env::var_os("SystemRoot") {
        let p = std::path::Path::new(&root).join("System32").join(file);
        if p.is_file() {
            return p.to_string_lossy().to_string();
        }
    }
    file.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_dispatch_keeps_query_strings_and_requests_a_visible_window() {
        let url = HSTRING::from("https://example.com/?from=azr");
        let command = shell_options(&url, None);
        assert_eq!(command.nShow, SW_SHOWNORMAL.0);
        assert_eq!(
            unsafe { command.lpFile.to_string() }.unwrap(),
            "https://example.com/?from=azr"
        );
        let parameters = HSTRING::from(r#"/select,"C:\Example.txt""#);
        let command = shell_options(&url, Some(&parameters));
        assert_eq!(
            unsafe { command.lpParameters.to_string() }.unwrap(),
            parameters.to_string_lossy()
        );
    }

    #[test]
    fn bare_tools_resolve_to_system32() {
        let p = system_tool("rundll32.exe");
        assert!(p.to_lowercase().ends_with("system32\\rundll32.exe"), "{p}");
        assert_eq!(system_tool("C:\\x\\y.exe"), "C:\\x\\y.exe");
        assert_eq!(system_tool("tidak-ada-xyz.exe"), "tidak-ada-xyz.exe");
    }
}
