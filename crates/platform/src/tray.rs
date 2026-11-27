use std::collections::HashMap;
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    REG_BINARY, REG_SZ,
};

use crate::wins::{enumerate, names};

/// Notification-area owners that belong to Windows itself and should not show
/// up as apps the user can quit.
const SYSTEM_TRAY: &[&str] = &[
    "azr.exe",
    "explorer.exe",
    "monotificationux.exe",
    "securityhealthsystray.exe",
    "textinputhost.exe",
    "applicationframehost.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "conhost.exe",
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "wscript.exe",
    "cscript.exe",
    "mshta.exe",
    "windowsterminal.exe",
    "wt.exe",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayApp {
    pub name: String,
    pub exe: String,
    pub pids: Vec<u32>,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .to_lowercase()
}

fn read_value(key: HKEY, name: &str) -> Option<String> {
    let n = wide(name);
    let mut ty = REG_SZ;
    let mut size = 0u32;
    unsafe {
        let r = RegQueryValueExW(
            key,
            PCWSTR(n.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut size),
        );
        if r != ERROR_SUCCESS || ty != REG_SZ || size == 0 {
            return None;
        }
    }
    let mut buf = vec![0u8; size as usize];
    unsafe {
        let r = RegQueryValueExW(
            key,
            PCWSTR(n.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut size),
        );
        if r != ERROR_SUCCESS {
            return None;
        }
    }
    let units: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Some(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string(),
    )
}

/// Executables that currently own a notification-area icon, from the shell's
/// per-icon settings, paired with the icon tooltip when the shell knows one.
fn registry_names() -> Vec<(String, String)> {
    let mut out = Vec::new();
    unsafe {
        let mut root = HKEY(std::ptr::null_mut());
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Control Panel\\NotifyIconSettings"),
            Some(0),
            KEY_READ,
            &mut root,
        ) != ERROR_SUCCESS
        {
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut buf = [0u16; 256];
            let mut len = buf.len() as u32;
            let r = RegEnumKeyExW(
                root,
                index,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            );
            if r != ERROR_SUCCESS {
                break;
            }
            let sub = wide(&String::from_utf16_lossy(&buf[..len as usize]));
            let mut key = HKEY(std::ptr::null_mut());
            if RegOpenKeyExW(root, PCWSTR(sub.as_ptr()), Some(0), KEY_READ, &mut key)
                == ERROR_SUCCESS
            {
                if read_value(key, "Publisher").as_deref() != Some("Microsoft Windows") {
                    if let Some(path) = read_value(key, "ExecutablePath") {
                        let file = file_name(&path);
                        if !file.is_empty() && !SYSTEM_TRAY.contains(&file.as_str()) {
                            let tip = read_value(key, "InitialTooltip").unwrap_or_default();
                            out.push((file, tip.trim().to_string()));
                        }
                    }
                }
                let _ = RegCloseKey(key);
            }
            index += 1;
        }
        let _ = RegCloseKey(root);
    }
    out
}

fn read_binary(key: HKEY, name: &str) -> Option<Vec<u8>> {
    let n = wide(name);
    let mut ty = REG_BINARY;
    let mut size = 0u32;
    unsafe {
        let r = RegQueryValueExW(
            key,
            PCWSTR(n.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut size),
        );
        if r != ERROR_SUCCESS || ty != REG_BINARY || size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let r = RegQueryValueExW(
            key,
            PCWSTR(n.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut size),
        );
        if r != ERROR_SUCCESS {
            return None;
        }
        buf.truncate(size as usize);
        Some(buf)
    }
}

fn rot13(c: char) -> char {
    match c {
        'a'..='m' => char::from(c as u8 + 13),
        'n'..='z' => char::from(c as u8 - 13),
        'A'..='M' => char::from(c as u8 + 13),
        'N'..='Z' => char::from(c as u8 - 13),
        _ => c,
    }
}

/// Drive-letter paths ending in `.exe` inside a decoded blob.
fn exe_paths(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < chars.len() {
        if chars[i].is_ascii_alphabetic() && chars[i + 1] == ':' && chars[i + 2] == '\\' {
            let mut j = i + 3;
            while j < chars.len() {
                let c = chars[j];
                if c.is_control() || matches!(c, '"' | '<' | '>' | '|' | '?' | '*') {
                    break;
                }
                j += 1;
            }
            let candidate: String = chars[i..j].iter().collect();
            if let Some(pos) = candidate.to_lowercase().rfind(".exe") {
                out.push(candidate[..pos + 4].to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Executables from the shell's tray history. The blob is UTF-16 text with
/// letters rotated by 13; it exists on Windows versions that lack
/// `NotifyIconSettings`, and it also keeps icons the newer key forgets.
fn stream_names() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let mut key = HKEY(std::ptr::null_mut());
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\TrayNotify"),
            Some(0),
            KEY_READ,
            &mut key,
        ) != ERROR_SUCCESS
        {
            return out;
        }
        if let Some(buf) = read_binary(key, "IconStreams") {
            let units: Vec<u16> = buf
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let decoded: String = String::from_utf16_lossy(&units)
                .chars()
                .map(rot13)
                .collect();
            for path in exe_paths(&decoded) {
                let file = file_name(&path);
                if !file.is_empty() && !SYSTEM_TRAY.contains(&file.as_str()) {
                    out.push(file);
                }
            }
        }
        let _ = RegCloseKey(key);
    }
    out
}

fn processes() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let n = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let file = String::from_utf16_lossy(&entry.szExeFile[..n]).to_lowercase();
                if !file.is_empty() {
                    out.push((entry.th32ProcessID, file));
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

fn pretty(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    let mut upper = true;
    for c in stem.chars() {
        if c == '.' || c == '_' || c == '-' {
            out.push(' ');
            upper = true;
        } else {
            if upper {
                out.extend(c.to_uppercase());
                upper = false;
            } else {
                out.push(c);
            }
        }
    }
    out
}

fn display_name(exe: &str, tooltip: &str, path: Option<&str>) -> String {
    if !tooltip.is_empty() {
        return tooltip.to_string();
    }
    let mut base = path
        .map(names::friendly)
        .unwrap_or_default()
        .trim()
        .to_string();
    if base.is_empty() {
        let mut stem = exe.trim_end_matches(".exe");
        if stem.to_lowercase().ends_with(".root") {
            stem = &stem[..stem.len() - 5];
        }
        base = pretty(stem);
    } else if base.to_lowercase().ends_with(".root") {
        base.truncate(base.len() - 5);
    }
    let base = base.trim();
    if base.is_empty() {
        pretty(exe.trim_end_matches(".exe"))
    } else {
        base.to_string()
    }
}

/// Running apps that own a notification-area icon. Every process sharing the
/// executable is grouped, so quitting terminates the whole app.
pub fn tray_apps(own_pid: u32) -> Vec<TrayApp> {
    let mut tips: HashMap<String, String> = HashMap::new();
    for (file, tip) in registry_names() {
        let slot = tips.entry(file).or_default();
        if slot.is_empty() {
            *slot = tip;
        }
    }
    for file in stream_names() {
        tips.entry(file).or_default();
    }
    if tips.is_empty() {
        return Vec::new();
    }
    let mut groups: HashMap<String, Vec<u32>> = HashMap::new();
    for (pid, file) in processes() {
        if pid == 0 || pid == own_pid || !tips.contains_key(&file) {
            continue;
        }
        groups.entry(file).or_default().push(pid);
    }
    let mut out: Vec<TrayApp> = groups
        .into_iter()
        .map(|(exe, mut pids)| {
            pids.sort_unstable();
            pids.dedup();
            let path = enumerate::exe_path_of_pid(pids[0]);
            let name = display_name(
                &exe,
                tips.get(&exe).map(String::as_str).unwrap_or(""),
                path.as_deref(),
            );
            TrayApp { name, exe, pids }
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rot13_round_trips_paths() {
        let plain = "C:\\Users\\Caya\\AppData\\Local\\Discord\\Discord.exe";
        let coded: String = plain.chars().map(rot13).collect();
        assert_eq!(
            coded,
            "P:\\Hfref\\Pnln\\NccQngn\\Ybpny\\Qvfpbeq\\Qvfpbeq.rkr"
        );
        let back: String = coded.chars().map(rot13).collect();
        assert_eq!(back, plain);
    }

    #[test]
    fn finds_exe_paths_inside_blob() {
        let blob = "\u{14}\0P:\\Hfref\\Pnln\\NccQngn\\Ybpny\\Qvfpbeq\\Qvfpbeq.rkr\0\u{14}";
        let decoded: String = blob.chars().map(rot13).collect();
        assert_eq!(
            exe_paths(&decoded),
            vec!["C:\\Users\\Caya\\AppData\\Local\\Discord\\Discord.exe"]
        );
    }

    #[test]
    fn ignores_paths_without_exe() {
        assert!(exe_paths("C:\\Windows\\notepad.txt").is_empty());
    }

    #[test]
    fn display_name_falls_back_through_sources() {
        assert_eq!(display_name("x.exe", "Nice Name", None), "Nice Name");
        assert_eq!(display_name("whatsapp.root.exe", "", None), "Whatsapp");
    }
}
