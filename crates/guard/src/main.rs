#![windows_subsystem = "windows"]

use azr_platform::process::{wait_for_exit, WaitOutcome};
use azr_platform::shell::taskbar;
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let (Some(pid), Some(marker)) = (args.next(), args.next()) else {
        return;
    };
    let Ok(pid) = pid.to_string_lossy().parse::<u32>() else {
        return;
    };
    let marker = PathBuf::from(marker);

    azr_platform::log::init(marker.with_file_name("Azr-guard.log"));
    azr_platform::log::line("guard", &format!("menunggu pid {pid}"));

    if wait_for_exit(pid) == WaitOutcome::Unknown {
        azr_platform::log::line(
            "guard",
            "tidak bisa memastikan proses induk; keluar tanpa tindakan",
        );
        return;
    }

    let parked = marker.with_file_name("parked.txt");
    if parked.exists() {
        let n = azr_platform::wins::park::restore_file(&parked);
        azr_platform::log::line("guard", &format!("{n} jendela terparkir dikembalikan"));
    }

    let cur_flag = marker.with_file_name("cursors_replaced.flag");
    if azr_platform::cursors::flagged(&cur_flag) {
        azr_platform::cursors::restore(&cur_flag);
        azr_platform::log::line("guard", "kursor sistem dipulihkan");
    }

    for _ in 0..120 {
        match taskbar::read_marker(&marker) {
            Some(m) if m.pid != pid => return,
            None if !marker.exists() => return,
            _ => {}
        }
        if taskbar::recover_leftover(&marker) && !marker.exists() {
            azr_platform::log::line("guard", "taskbar dipulihkan");
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    azr_platform::log::line("guard", "menyerah setelah 30 s");
}
