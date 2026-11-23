#![windows_subsystem = "windows"]

fn main() {
    let mut args = std::env::args_os().skip(1);
    let (Some(pid), Some(marker)) = (args.next(), args.next()) else {
        return;
    };
    if let Ok(pid) = pid.to_string_lossy().parse() {
        azr_platform::shell::recovery::wait_and_restore(pid, std::path::Path::new(&marker));
    }
}
