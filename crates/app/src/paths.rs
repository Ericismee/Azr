use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("AZR_CONFIG_DIR") {
        return PathBuf::from(d);
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Azr")
}

pub fn marker() -> PathBuf {
    config_dir().join("taskbar_hidden.flag")
}

pub fn log() -> PathBuf {
    config_dir().join("Azr.log")
}

pub fn guard_exe() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("azr-guard.exe")))
        .unwrap_or_else(|| config_dir().join("missing-guard.exe"))
}
