# Azr desktop stability and feature plan

## MVP

- Issue [#2](https://github.com/Ericismee/Azr/issues/2) reports broken text across Settings, widgets and Control Center. Check the chosen font once for Latin and Vietnamese glyphs. Initialize the GDI bitmap and flush queued drawing before reading pixels. The reporter has not yet confirmed the fix on the affected machine.
- Open Search with Alt+Space. Launchpad shares the Search window code. Place it on the monitor under the pointer, using its work area and DPI. Reopen after display topology changes to recalculate placement.
- Move Search and Launchpad shell icon loading and Control Center Wi-Fi queries off the UI thread. Remove the Control Center's fixed 60 ms introductory wait and shorten its animation.
- Route “Force Quit Azr” through Azr's shutdown path so it restores the taskbar and clears its tray icon before exit.

## References and next steps

- [ABRA changes](https://github.com/abdeealfarizi/abra/commits): **513e874** validates installed font glyphs and moves global hook work off-thread. **f722629** adds monitor selection, per-monitor DPI and unplug recovery. **d49f44e** adds wider GPU/display compatibility and menu refinements. **7786fb8**, **4cf394d** and **13bd2c7** are useful references for the Island header, tasks, Clipboard Stack and layout. Implement only relevant behavior in Azr and review any upstream license requirements before reusing code.
- [Nusa](https://rama2555.github.io/nusa/) and the supplied screenshots inform a compact Island with an explicit Shelf / Clipboard switch, an empty drop area, clipboard cards and a clear action. Azr already has a file Shelf and opt-in clipboard history in Search. Share the existing clipboard service, keep collection off by default, honor Windows exclusions, and add a local To-do tab after the UI is stable.
- [MacDock](https://github.com/GodrezJr2/MacDock) and [Muelle](https://github.com/crafter-station/Muelle): compare cursor-driven dock magnification and Genie transitions. Keep the compositor-driven approach, respect disabled animations, and profile capture and upload before extending effects.
- [ArcDockWin](https://github.com/claudsondouglas/ArcDockWin): consider per-monitor menu bars, window cycling from the Dock and on-demand system readings. Prioritize mixed DPI, negative coordinates, hot-plug, rotation and work-area changes first.
- [Wallplayer](https://github.com/lehuynhtan17/Wallplayer): plan an optional local video wallpaper for separate monitors. Pause when occluded, on battery or in full screen. Keep network media and desktop icon changes out of the default path.

## Release gate

Measure cold and warm time to first visible frame for Search, Launchpad, and Control Center. Record median and p95 from repeated runs, with 60 Hz and 120 Hz, at 100% and 150% DPI, and with mixed DPI displays. A functional Sandbox run does not establish frame pacing on physical monitors. Verify taskbar restoration, no remaining Azr processes, keyboard shortcut conflicts, and review issue #2 on the affected configuration before closing it.
