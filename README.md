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

Your desktop is where everything starts. Make it a place you enjoy coming back to.

**Azr brings a macOS-inspired feel to Windows 11** with a glass Dock, an adaptive Island, a menu bar, and controls that feel at home together. Keep your familiar apps. Give the space around them a little more character.

**[Download Azr](https://github.com/Ericismee/Azr/releases/latest)** · [See it in action](#a-closer-look) · [Tiếng Việt](README.vi.md)

![Azr desktop](docs/assets/desktop-user-en.png)

## Less searching. More doing.

### Your everyday apps, within reach

A Dock for the things you open most. Move between apps, browse Launchpad, reach your downloads, and keep the desktop flowing. Magnification and rounded icons bring a familiar rhythm to everyday clicks.

### A small Island with more to show

Time, music, a timer, and a place for files—close at hand without filling your screen. The Island opens when you need more room and settles back into a compact presence when you are done.

### The right controls, together

A menu bar above, quick controls a click away. Bring sound, display, and connectivity controls into one place, then arrange the panel around the things you use.

### A desktop that keeps you informed

Give your clock, calendar, battery, and system information a place of their own. Desktop widgets keep the useful details visible while you get on with something else.

### Room to find your focus

Find apps quickly, move between windows, and put screen corners to work. Search, window tools, and hot corners offer small shortcuts that add up throughout the day.

### Make it unmistakably yours

Light or dark. Frosted or clear. A quieter palette or a little more color. Shape the Dock, restyle icons, and choose the Azr logo, a simple ring, or your own image. English and Vietnamese are built in.

Start with one change. See where it takes your desktop.

## A closer look

<details>
<summary>Explore the desktop, glass controls, and widgets</summary>

![English desktop](docs/assets/desktop-user-en.png)
![Vietnamese desktop](docs/assets/desktop-user-vi.png)
![Glass control center](docs/assets/control-center-user.png)
![Desktop widgets](docs/assets/widgets-user.png)

</details>

Screenshots were captured in Windows Sandbox. Glass effects vary with your wallpaper, Windows settings, hardware, and remote-session support. Some captures show an earlier interface; current releases include updated Vietnamese weekday labels.

## A small footprint in the moment

![Azr processes in Task Manager](docs/assets/process-memory.png)

In this Task Manager snapshot, Azr shows **6.6 MB** of memory and **0.1% CPU**, while its recovery companion shows **0.6 MB** and **0% CPU**—about **7.2 MB combined at that moment**. This is a single observation, not a benchmark or a fixed requirement. Usage changes with your setup and activity, and the displayed figures do not include every graphics or system memory cost. Download size is separate from runtime RAM.

## Get started

1. [Download the latest Windows x64 portable release](https://github.com/Ericismee/Azr/releases/latest).
2. Extract the ZIP and keep its files together.
3. Open `azr.exe`, then explore Settings to make it yours.

Requires **Windows 11 x64**. Keep `azr-guard.exe`, `fonts`, and `brand` beside the app. There is no installer, service, or driver to add. The EXEs are unsigned; release checksums are supplied in `SHA256SUMS.txt`. Prefer to build it yourself? See the [build guide](docs/BUILD.md).

## Your desktop, your choices

Autostart, taskbar takeover, clipboard history, online lyrics, and local file sharing are **off by default**. Your settings live in `%APPDATA%\Azr`. The companion process helps restore the taskbar when takeover is enabled and Azr stops unexpectedly.

Online lyrics use an external service. Local file sharing uses unencrypted HTTP and should only be enabled on a trusted network. For a first trial, Windows Sandbox is a convenient option. To remove Azr, disable autostart, exit the app, and delete the extracted folder; remove the profile separately if you also want to erase settings.

## Follow Azr

Found a place for Azr on your desktop? A star helps other people discover it. Ideas and feedback are welcome in [Issues](https://github.com/Ericismee/Azr/issues).

[![Star history](https://api.star-history.com/svg?repos=Ericismee/Azr&type=Date)](https://www.star-history.com/#Ericismee/Azr&Date)

## License

[PolyForm Noncommercial 1.0.0](LICENSE.md) · [Third-party components](legal/THIRD-PARTY-LICENSES.md)

Azr is an independent Windows project, not affiliated with Apple or Microsoft. Its glass effects are implemented for Windows.
