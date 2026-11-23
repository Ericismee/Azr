use super::taskbar;
use crate::process::{wait_for_exit, WaitOutcome};
use std::{path::Path, time::Duration};

pub fn wait_and_restore(pid: u32, marker: &Path) {
    if pid == 0 || !taskbar::read_marker(marker).is_some_and(|entry| entry.pid == pid) {
        return;
    }
    crate::log::init(marker.with_file_name("Azr-guard.log"));
    crate::log::line("guard", &format!("menunggu pid {pid}"));

    if wait_for_exit(pid) == WaitOutcome::Unknown {
        crate::log::line(
            "guard",
            "tidak bisa memastikan proses induk; keluar tanpa tindakan",
        );
        return;
    }

    let parked = marker.with_file_name("parked.txt");
    if parked.exists() {
        let n = crate::wins::park::restore_file(&parked);
        crate::log::line("guard", &format!("{n} jendela terparkir dikembalikan"));
    }

    let cur_flag = marker.with_file_name("cursors_replaced.flag");
    if crate::cursors::flagged(&cur_flag) {
        crate::cursors::restore(&cur_flag);
        crate::log::line("guard", "kursor sistem dipulihkan");
    }

    for _ in 0..120 {
        match taskbar::read_marker(marker) {
            Some(m) if m.pid != pid => return,
            None if !marker.exists() => return,
            _ => {}
        }
        if taskbar::recover_leftover(marker) && !marker.exists() {
            crate::log::line("guard", "taskbar dipulihkan");
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    crate::log::line("guard", "menyerah setelah 30 s");
}
