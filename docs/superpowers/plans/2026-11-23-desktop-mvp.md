# Azr desktop stability and feature plan

## MVP

- Issue [#2](https://github.com/Ericismee/Azr/issues/2) reports broken text across Settings, widgets and Control Center. Check the chosen font once for Latin and Vietnamese glyphs. Initialize the GDI bitmap and flush queued drawing before reading pixels. The reporter has not yet confirmed the fix on the affected machine.
- Open Search with Alt+Space. Launchpad shares the Search window code. Place it on the monitor under the pointer, using its work area and DPI. Reopen after display topology changes to recalculate placement.
- Move Search and Launchpad shell icon loading and Control Center Wi-Fi queries off the UI thread. Remove the Control Center's fixed 60 ms introductory wait and shorten its animation.
- Route “Force Quit Azr” through Azr's shutdown path so it restores the taskbar and clears its tray icon before exit.

## Monitor placement follow-up

Control Center uses the pointer monitor's work area and DPI when opened. The panel and its editing gallery fit that work area. Its size menu and the network flyouts use absolute monitor coordinates, including negative origins. The main panel closes on a display change so reopening picks up the new configuration. Unit tests cover a tall layout at high DPI and gallery placement on a secondary portrait display.

Physical validation is still needed for mixed DPI, unplugging while a popup is open, and taskbars placed on different edges. Test Dock, menu bar, Dynamic Island and Settings separately because they use their own window placement.

## References and next steps

- [ABRA changes](https://github.com/abdeealfarizi/abra/commits): **513e874** validates installed font glyphs and moves global hook work off-thread. **f722629** adds monitor selection, per-monitor DPI and unplug recovery. **d49f44e** adds wider GPU/display compatibility and menu refinements. **7786fb8**, **4cf394d** and **13bd2c7** are useful references for the Island header, tasks, Clipboard Stack and layout. Implement only relevant behavior in Azr and review any upstream license requirements before reusing code.
- [Nusa](https://rama2555.github.io/nusa/) and the supplied screenshots inform a compact Island with an explicit Shelf / Clipboard switch, an empty drop area, clipboard cards and a clear action. Azr already has a file Shelf and opt-in clipboard history in Search. Share the existing clipboard service, keep collection off by default, honor Windows exclusions, and add a local To-do tab after the UI is stable.
- [MacDock](https://github.com/GodrezJr2/MacDock) and [Muelle](https://github.com/crafter-station/Muelle): compare cursor-driven dock magnification and Genie transitions. Keep the compositor-driven approach, respect disabled animations, and profile capture and upload before extending effects.
- [ArcDockWin](https://github.com/claudsondouglas/ArcDockWin): consider per-monitor menu bars, window cycling from the Dock and on-demand system readings. Prioritize mixed DPI, negative coordinates, hot-plug, rotation and work-area changes first.
- [Wallplayer](https://github.com/lehuynhtan17/Wallplayer): plan an optional local video wallpaper for separate monitors. Pause when occluded, on battery or in full screen. Keep network media and desktop icon changes out of the default path.

## Release gate

Measure cold and warm time to first visible frame for Search, Launchpad, and Control Center. Record median and p95 from repeated runs, with 60 Hz and 120 Hz, at 100% and 150% DPI, and with mixed DPI displays. A functional Sandbox run does not establish frame pacing on physical monitors. Verify taskbar restoration, no remaining Azr processes, keyboard shortcut conflicts, and review issue #2 on the affected configuration before closing it.

## Follow-up backlog (2026-11-24)

### Fix first

- **First click outside a popup.** Dismiss the popup without swallowing a click intended for another app or Azr control. Verify the first click both closes the popup and reaches its target.
- **Dock menu and display changes.** Close the menu and clear its interaction state when DPI or the display changes. In Smart mode, verify that the Dock still hides after the configured delay afterward.
- **Fullscreen on multiple displays.** Track which display contains the fullscreen app. Verify that the game display clears its overlays, the other display stays usable, and Alt+Tab restores the correct surfaces.
- **Quick Panel gallery scrolling.** Clip the scrolling cards and their hit targets to the viewport. Keep the Done button visible and clickable at 100%, 125%, and 150% display scaling.
- **Month widget weekday labels.** Show distinct labels in Vietnamese and English, aligned with each language's current week layout.
- **Reported text rendering.** Keep [issue #2](https://github.com/Ericismee/Azr/issues/2) open until the affected configuration is reproduced or the reporter confirms a fix. Record the Windows build, display scale, language, and monitor arrangement.

### First stability pass

- Fixed the swallowed outside click. A Windows smoke test confirmed that one click closes Quick Panel and reaches the target button.
- Clear Dock menu state and release menu and tooltip windows on reconfiguration. Validate with live DPI or monitor changes while a Dock menu is open.
- Clip Quick Panel gallery cards and their clickable bounds above the footer. The viewport regression test passed.
- Use localized weekday abbreviations in the Month widget and translate the Quick Panel hotspot label.
- Keep Smart Dock visibility checks on pointer entry, exit, and timers. Pointer movement no longer repeats the check for every magnification update. Frame pacing still needs physical measurement.

### Next improvements

1. **Measure responsiveness before tuning.** Compare repeated Dock hover and Quick Search, Launchpad, and Quick Panel open/close runs. Check time to first visible frame, slow frames, and idle CPU on the same machine before and after each change. Use [Windows Performance Recorder](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder) when system-level tracing is needed.
2. **Polish everyday controls.** Respect Windows animation preferences across Azr, expose shortcut conflicts with an alternate binding, and make keyboard focus visible in Settings and popups.
3. **Save desktop setups.** Add local export/import of visual settings and layout profiles. A profile can restore Dock, widgets, and display preferences. Clipboard history and file shelf contents stay outside the export.

### Startup, persistence, and release checks

- [ ] **Startup and uninstall.** Prefer the installed Azr path for startup when both installed and portable builds are present. Verify that running a temporary build does not change startup unexpectedly, and that uninstall removes its own startup entry. The current machine's startup entry points to the local Dock test build even though an installed copy exists.
- [ ] **Open Azr while it is running.** A second invocation currently wakes the existing process, which only logs the event. Forward an explicit request to open Settings and bring that window to the foreground. Verify with an installed Start menu shortcut and a second portable launch.
- [ ] **Visible application launch errors.** Azr suppresses the Windows error dialog and logs rejected launch requests. Show a short message when a Dock item or Quick Search result cannot open. Test with a missing shortcut and a valid app.
- [ ] **Settings persistence.** Save through an atomic replacement, keep the last good configuration, and surface write errors. Verify that a failed save leaves the previous preferences readable on restart.
- [ ] **Reproducible release packaging.** Build the complete portable ZIP and the installer from the same payload and generate SHA-256 checksums. Test clean install, upgrade with Azr running, and uninstall in Windows Sandbox.

### Deferred

Dynamic Island development stays in a separate phase after the desktop stability work. Video wallpaper remains a later feature.

### Overview and desktop performance pass

- Overview uses a lighter translucent backdrop, persistent window labels, and fallback cards for unavailable previews. Alt+Tab and Shift+Alt+Tab cycle through recent windows and releasing Alt confirms the keyboard selection. A Windows test verified that the previous test window actually received focus.
- Smart Dock keeps its 14 DIP activation band. The dwell was reduced from 150 ms to 110 ms. The existing preview is reused when its source windows and size are unchanged.
- Quick Panel queries microphone/camera activity on a worker and skips duplicate radio snapshots. Launchpad measures each visible app label once. Search skips redraws when the app index has not changed.
- Local smoke measurements from command dispatch through visible HWND and one DWM flush, in milliseconds. First opens before and after: Search 72.9 to 76.7, Launchpad 120.5 to 79.3, and Quick Panel 110.5 to 102.0. Warm medians from five opens: Search 20.7 to 31.8, Launchpad 33.1 to 33.3, and Quick Panel 32.8 to 34.5. These measurements are a presentation proxy and show that warm Search still merits investigation. A short five-second hidden-Dock sample measured whole-process CPU at 1.25% and 0.62% of one core, respectively.
- A borderless Windows test window preserved the primary UI while fullscreen on a secondary display. Moving it to the primary display hid the local UI and switching focus restored it. A synthetic display-change message closed the stale Overview layout. Physical monitor unplugging and exclusive fullscreen games still need verification.
- Appearance settings were visually checked in Vietnamese and English at 100%, 125%, and 150% **Azr UI scale** on one monitor. No overlapping labels were observed on that page. The original text rendering report remains open. Native DPI and Azr UI scale are separate checks.

### Custom shortcuts and Windows regression checks

- Settings → General offers shortcut recording for Quick Search, Launchpad and Overview. Click a field and enter a combination. Escape cancels and Backspace restores the default or preset. Windows testing exercised all three shortcuts, an external registration conflict, persistence across restart and restoration of Alt+Space.
- The Rust workspace passed 272 tests, with two ignored. Direct Windows checks covered Dock edge activation, auto-hide, the path from a Dock icon into its preview, Quick Panel layout and click-through on dismissal, application switching and fullscreen transitions on two displays. One Dock hide-timing run failed and a repeat passed. Timing consistency remains an open check.
- Time from dispatch through a visible HWND and DWM flush was 73/81/110 ms for the first Search/Launchpad/Quick Panel open and 32/36/36 ms for the median warm open. A five-second hidden-Dock sample measured 1.24% of a logical CPU for the whole Azr process.
- The Azr menu command opened a Task Manager window. The injected Windows Ctrl+Shift+Esc probe did not open one, so physical keyboard behavior remains to be checked.
- Remaining direct Windows checks include exclusive fullscreen, physical monitor disconnect, native mixed DPI, system power operations, radio toggles, and the install/upgrade/uninstall cycle.
