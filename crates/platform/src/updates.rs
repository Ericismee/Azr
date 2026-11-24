#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS_11: &str = "Azr-Setup-Windows11-x64.exe";

    fn metadata(tag: &str, name: &str, digest: &str) -> Vec<u8> {
        format!(
            r#"{{"tag_name":"{tag}","draft":false,"prerelease":false,"assets":[{{"name":"{name}","size":4,"digest":"{digest}"}}]}}"#
        )
        .into_bytes()
    }

    #[test]
    fn offers_only_newer_stable_releases_with_the_correct_installer_and_digest() {
        let digest = "sha256:".to_owned() + &"a".repeat(64);
        let json = metadata("v0.1.10", WINDOWS_11, &digest);
        let update = newer(&json, "0.1.8", WINDOWS_11).unwrap().unwrap();
        assert_eq!(update.tag, "v0.1.10");
        assert!(update.url.ends_with("/v0.1.10/Azr-Setup-Windows11-x64.exe"));
        assert!(newer(&json, "0.1.10", WINDOWS_11).unwrap().is_none());
        assert!(newer(&json, "0.1.11", WINDOWS_11).unwrap().is_none());
        assert!(newer(&json, "0.1.8", "Azr-Setup-Windows10-x64.exe").is_err());
        assert!(newer(
            &metadata("v0.1.10-rc.1", WINDOWS_11, &digest),
            "0.1.8",
            WINDOWS_11
        )
        .is_err());
        assert!(newer(
            &metadata("v0.1.10", WINDOWS_11, "sha256:bad"),
            "0.1.8",
            WINDOWS_11
        )
        .is_err());
    }

    #[test]
    fn installer_integrity_checks_both_size_and_sha256() {
        let digest = "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        let update = newer(&metadata("v0.1.9", WINDOWS_11, digest), "0.1.8", WINDOWS_11)
            .unwrap()
            .unwrap();
        assert!(update.verify(b"test").is_ok());
        assert!(update.verify(b"junk").is_err());
        assert!(update.verify(b"").is_err());
    }
}
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{self, Write};
use std::os::windows::{ffi::OsStringExt, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::core::w;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

const LATEST: &str = "https://api.github.com/repos/Ericismee/Azr/releases/latest";
const DOWNLOADS: &str = "https://github.com/Ericismee/Azr/releases/download";
const MAX_INSTALLER: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Update {
    pub tag: String,
    url: String,
    size: usize,
    digest: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: usize,
    digest: Option<String>,
}

fn stable_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split('.');
    let mut number = || {
        let value = parts.next()?;
        if value.len() > 1 && value.starts_with('0') || !value.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        value.parse::<u32>().ok()
    };
    let version = (number()?, number()?, number()?);
    parts.next().is_none().then_some(version)
}

fn newer(json: &[u8], current: &str, installer: &str) -> Result<Option<Update>, String> {
    let release: Release = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let current = stable_version(current).ok_or("Invalid installed version")?;
    let tag = release.tag_name;
    let remote = tag
        .strip_prefix('v')
        .and_then(stable_version)
        .ok_or("Invalid release tag")?;
    if remote <= current {
        return Ok(None);
    }
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == installer)
        .ok_or("No installer for this Windows version")?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| d.len() == 64 && d.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or("Release installer has no valid SHA-256")?
        .to_ascii_lowercase();
    if !(1..=MAX_INSTALLER).contains(&asset.size) {
        return Err("Invalid release installer size".into());
    }
    Ok(Some(Update {
        url: format!("{DOWNLOADS}/{tag}/{installer}"),
        tag,
        size: asset.size,
        digest,
    }))
}

pub fn check(current: &str) -> Result<Option<Update>, String> {
    let mut build = [0u16; 32];
    let mut bytes = std::mem::size_of_val(&build) as u32;
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(build.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .ok()
        .map_err(|e| e.to_string())?;
    }
    let end = build.iter().position(|&c| c == 0).unwrap_or(build.len());
    let build = String::from_utf16_lossy(&build[..end])
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let installer = match build {
        22000.. => "Azr-Setup-Windows11-x64.exe",
        19045.. => "Azr-Setup-Windows10-x64.exe",
        _ => return Err("Unsupported Windows version".into()),
    };
    newer(&fetch(LATEST, 1024 * 1024, "15")?, current, installer)
}

fn fetch(url: &str, limit: usize, seconds: &str) -> Result<Vec<u8>, String> {
    let mut system = [0u16; 260];
    let len = unsafe { GetSystemDirectoryW(Some(&mut system)) } as usize;
    if len == 0 || len >= system.len() {
        return Err("Windows system directory unavailable".into());
    }
    let curl = PathBuf::from(OsString::from_wide(&system[..len])).join("curl.exe");
    let output = Command::new(curl)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "--max-redirs",
            "5",
            "--connect-timeout",
            "8",
            "--max-time",
            seconds,
            "--max-filesize",
            &limit.to_string(),
            "--user-agent",
            "Azr-Updater",
            url,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() || output.stdout.len() > limit {
        return Err(format!("GitHub request failed ({})", output.status));
    }
    Ok(output.stdout)
}

impl Update {
    fn verify(&self, content: &[u8]) -> Result<(), String> {
        if content.len() != self.size || format!("{:x}", Sha256::digest(content)) != self.digest {
            return Err("Installer checksum mismatch".into());
        }
        Ok(())
    }

    pub fn start_setup(&self, install_dir: &Path) -> Result<(), String> {
        let bytes = fetch(&self.url, self.size, "150")?;
        self.verify(&bytes)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let folder =
            std::env::temp_dir().join(format!("Azr-update-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&folder).map_err(|e| e.to_string())?;
        let setup = folder.join("Azr-Setup.exe");
        let mut destination = OsString::from("/DIR=");
        destination.push(install_dir);
        let started = (|| -> io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&setup)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            Command::new(&setup)
                .args([
                    "/AZRUPDATE",
                    "/SP-",
                    "/SILENT",
                    "/NORESTART",
                    "/NORESTARTAPPLICATIONS",
                ])
                .arg(destination)
                .spawn()?;
            Ok(())
        })();
        if let Err(error) = started {
            let _ = std::fs::remove_file(&setup);
            let _ = std::fs::remove_dir(&folder);
            return Err(error.to_string());
        }
        Ok(())
    }
}
