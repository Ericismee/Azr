<div align="center">
<img src="assets/brand/azr-liquid-glass.png" width="144" alt="Azr glass ribbon logo">
<h1>Azr</h1>
<p>A personal desktop, shaped around you.</p>

[English](README.md) · [Tiếng Việt](README.vi.md)

[![Build](https://github.com/Ericismee/Azr/actions/workflows/ci.yml/badge.svg)](https://github.com/Ericismee/Azr/actions/workflows/ci.yml)
[![Stars](https://img.shields.io/github/stars/Ericismee/Azr?style=flat)](https://github.com/Ericismee/Azr/stargazers)
[![Forks](https://img.shields.io/github/forks/Ericismee/Azr?style=flat)](https://github.com/Ericismee/Azr/forks)
![Windows 11](https://img.shields.io/badge/Windows-11-0078D4)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial-64748b)](LICENSE.md)
</div>

Azr is a native Windows 11 desktop customization app written in Rust. Bring together a magnifying Dock, a menu bar, floating controls, and continuously rounded icons in a glass-inspired interface. Available in English and Vietnamese.

![Customized Azr desktop](docs/assets/desktop.png)

## Make it yours

- **Dock:** app shortcuts, magnification, Launchpad, Downloads, and Recycle Bin.
- **Surfaces:** glass, frosted, transparent, or solid; separate opacity and blur controls.
- **Icons:** optional backgrounds, adjustable continuous corners, and color styles.
- **Workspace:** menu bar, quick controls, hot corners, window tools, and desktop widgets.
- **Identity:** choose the Azr symbol, a circular ring, or import your own menu logo.
- **Language and scale:** English and Vietnamese, with wrapped labels and adaptable settings layouts.

The glass and corner effects are Azr's Windows implementation. Rendering depends on Windows transparency settings, hardware, power mode, and remote-session support.

<p align="center"><img src="docs/assets/appearance-en.png" width="720" alt="Azr appearance settings in English"></p>

## Download and screenshots

[Download Azr 0.1.1 for Windows x64](https://github.com/Ericismee/Azr/releases/tag/v0.1.1). Extract `Azr-0.1.1-windows-x64-portable.zip` and run `azr.exe`. This is a portable package, not an installer. Checksums are in `SHA256SUMS.txt`. EXEs are not Authenticode-signed.

<details>
<summary>English / Vietnamese desktop, glass controls, widgets</summary>

![English desktop](docs/assets/desktop-user-en.png)
![Vietnamese desktop](docs/assets/desktop-user-vi.png)
![Glass controls](docs/assets/control-center-user.png)
![Widgets](docs/assets/widgets-user.png)

User-supplied Windows Sandbox captures. The widget image predates the 0.1.1 correction of Vietnamese weekday names. Appearance varies with wallpaper, settings, and rendering support.
</details>

![Task Manager](docs/assets/process-memory.png)

## Resource use

The supplied Task Manager snapshot shows `azr` using **6.6 MB** in the Memory column and **0.1% CPU**, and `azr-guard` using **0.6 MB** and **0% CPU**: approximately **7.2 MB combined at that moment**. Disk activity is 0.1 MB/s and 0 MB/s respectively; both network readings are 0 Mbps.

This is one observation, not a benchmark, maximum, or fixed RAM requirement. The cropped image does not establish the exact memory counter, sampling duration, or VM allocation. It does not represent total committed memory or GPU/DWM memory. Usage varies with animation, artwork, widgets, caches, Windows, hardware, and enabled features. ZIP size, extracted file size, and runtime RAM are different measurements.

`azr.exe` manages and draws the UI. `azr-guard.exe` is the taskbar recovery companion used with taskbar takeover. Normal exit restores the taskbar; the guard supports recovery after an unexpected stop.

## Files and Windows integration

- Portable folder: `Azr-0.1.1`; binaries: `azr.exe`, `azr-guard.exe`. Keep `fonts` and `brand` beside them.
- Configuration: `%APPDATA%\Azr`, or `AZR_CONFIG_DIR`; descriptive files include `settings.cfg`, `pins.txt`, `Azr.log`, and `Azr-guard.log`.
- Optional startup: the `Azr` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, disabled by default. Runtime synchronization names start with `Local\Azr`.
- No system-wide installer, Windows service, or driver is added. To remove: disable autostart, exit Azr, and delete the extracted folder. Delete `%APPDATA%\Azr` separately only to erase settings. Third-party fonts and dependencies retain their own identities and licenses.

## Build and run

Requirements: Windows 11 x64, stable Rust, Visual Studio C++ x64 Build Tools, and the Windows SDK.

```powershell
git clone https://github.com/Ericismee/Azr.git
cd Azr
.\build.ps1
```

The script runs the locked workspace tests, builds release binaries, and creates a portable ZIP with SHA-256 hashes in `dist`. It does not launch or install the app. Extract the package and open `azr.exe`; keep `azr-guard.exe`, `fonts`, and `brand` alongside it. Normal use does not require administrator privileges. Exit through the app menu or notification area.

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
```

## Customize

Open **Settings → Dock** for surface material, opacity, blur, icon backgrounds, and corner shape. **Appearance** controls themes and icon colors. **Menu bar** includes the logo selector and image picker. Logo imports are limited to 8 MiB and 16 million decoded pixels; an invalid image leaves the previous logo intact.

<p align="center"><img src="docs/assets/menu-logo.png" width="720" alt="Azr menu logo options"></p>

## Privacy and safety

Autostart, taskbar takeover, clipboard history, online lyrics, and local-network file sharing are disabled by default. Configuration stays in `%APPDATA%\Azr`; set `AZR_CONFIG_DIR` to use another profile directory.

Enabling online lyrics connects to LRCLIB over HTTPS. Enabling local file sharing uses unencrypted HTTP with random, time-limited links: use it only on a trusted network. The companion guard helps restore the Windows taskbar if takeover is enabled and the app exits unexpectedly. Tests and antivirus scans cannot guarantee the absence of every security issue. For a first trial, use Windows Sandbox.

## Project map

- `crates/app`: application windows, settings, and desktop integration.
- `crates/core`: configuration, layout, rendering logic, and unit tests.
- `crates/platform`: Windows APIs and services.
- `crates/ui`: graphics and composition.
- `crates/guard`: taskbar recovery companion.
- `assets/brand`: original Azr artwork and symbol.

## Community interest

Stars and forks above are live GitHub counters. This is a new repository; history may be empty until real activity appears. The chart reflects recorded stars, without projected growth.

[![Star history](https://api.star-history.com/svg?repos=Ericismee/Azr&type=Date)](https://www.star-history.com/#Ericismee/Azr&Date)

## License

Distributed under [PolyForm Noncommercial 1.0.0](LICENSE.md). Commercial use is not granted by that license. Font and dependency information is in [Third-party components](legal/THIRD-PARTY-LICENSES.md). Azr is an independent Windows project and is not affiliated with Apple or Microsoft.
