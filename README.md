<div align="center">
<img src="assets/brand/azr-liquid-glass.png" width="144" alt="Azr glass ribbon logo">
<h1>Azr</h1>
<p>Desktop customization for Windows 11.</p>

[English](README.md) · [Tiếng Việt](README.vi.md)

[![Build](https://github.com/Ericismee/Azr/actions/workflows/ci.yml/badge.svg)](https://github.com/Ericismee/Azr/actions/workflows/ci.yml)
[![Stars](https://img.shields.io/github/stars/Ericismee/Azr?style=flat)](https://github.com/Ericismee/Azr/stargazers)
[![Forks](https://img.shields.io/github/forks/Ericismee/Azr?style=flat)](https://github.com/Ericismee/Azr/forks)
![Windows 11](https://img.shields.io/badge/Windows-11-0078D4)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial-64748b)](LICENSE.md)
</div>

Azr is a desktop customization app for **Windows 11**, inspired by macOS. It adds a Dock, Dynamic Island, menu bar, quick controls, and desktop widgets, with appearance settings you can adjust to suit your workflow. Available in English and Vietnamese.

**[Download the latest release](https://github.com/Ericismee/Azr/releases/latest)** · [Screenshots](#screenshots) · [Tiếng Việt](README.vi.md)

![Azr on Windows 11](docs/assets/desktop-user-en.png)

## Main features

- **Dock and Launchpad:** Access frequently used apps, browse your applications, and open Downloads or Recycle Bin. Customize icon appearance and enable magnification on hover.
- **Dynamic Island:** View the clock and access music controls, a timer, and a file shelf. Expand the panel to interact with it, then return it to a compact view.
- **Menu bar and quick controls:** View system status and access sound, display, and connectivity controls. Arrange the controls around the settings you use most.
- **Desktop widgets:** Keep a clock, calendar, battery status, and system usage visible on your desktop.
- **Search and window tools:** Find applications, switch between windows, and assign actions to screen corners.
- **Appearance customization:** Choose light or dark themes, glass surfaces, transparency, colors, and icon styles. Adjust the Dock, icon corners, and menu logo using a built-in symbol or your own image.

## Screenshots

<details>
<summary>Explore the desktop, glass controls, and widgets</summary>

![English desktop](docs/assets/desktop-user-en.png)
![Glass control center](docs/assets/control-center-user.png)
![Desktop widgets](docs/assets/widgets-user.png)

</details>

Captured in Windows Sandbox, showing several appearance configurations. Glass effects may vary with wallpaper, Windows settings, hardware, and remote-session support.

## Resource usage

![Azr processes in Task Manager](docs/assets/process-memory.png)

In this Task Manager snapshot, Azr shows **6.6 MB** of memory and **0.1% CPU**, while its recovery companion shows **0.6 MB** and **0% CPU**—about **7.2 MB combined at that moment**. This is a single observation, not a benchmark or a fixed requirement. Usage changes with your setup and activity, and the displayed figures do not include every graphics or system memory cost. Download size is separate from runtime RAM.

## Get started

1. [Download the latest Windows x64 portable release](https://github.com/Ericismee/Azr/releases/latest).
2. Extract the ZIP and keep its files together.
3. Open `azr.exe` and use Settings to adjust the appearance, language, and features.

Requires **Windows 11 x64**. Keep `azr-guard.exe`, `fonts`, and `brand` beside the app. There is no installer, service, or driver to add. The EXEs are unsigned; release checksums are supplied in `SHA256SUMS.txt`. Prefer to build it yourself? See the [build guide](docs/BUILD.md).

## Configuration and privacy

Autostart, taskbar takeover, clipboard history, online lyrics, and local file sharing are **off by default**. Your settings live in `%APPDATA%\Azr`. The companion process helps restore the taskbar when takeover is enabled and Azr stops unexpectedly.

Online lyrics use an external service. Local file sharing uses unencrypted HTTP and should only be enabled on a trusted network. For a first trial, Windows Sandbox is a convenient option. To remove Azr, disable autostart, exit the app, and delete the extracted folder; remove the profile separately if you also want to erase settings.

## Feedback and contributions

Report bugs or suggest features through [Issues](https://github.com/Ericismee/Azr/issues). The star and fork badges and the chart below reflect the project’s activity on GitHub.

[![Star history](https://api.star-history.com/svg?repos=Ericismee/Azr&type=Date)](https://www.star-history.com/#Ericismee/Azr&Date)

## License

[PolyForm Noncommercial 1.0.0](LICENSE.md) · [Third-party components](legal/THIRD-PARTY-LICENSES.md)

Azr is an independent Windows project, not affiliated with Apple or Microsoft. Its glass effects are implemented for Windows.
