use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

static STARTED: Mutex<bool> = Mutex::new(false);

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

pub fn spawn_once(guard_exe: &Path, parent_pid: u32, marker: &Path) -> std::io::Result<()> {
    let mut started = STARTED
        .lock()
        .map_err(|_| std::io::Error::other("lock poisoned"))?;
    if *started {
        return Ok(());
    }
    let base = DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW;
    let embedded = !guard_exe.is_file();
    let executable = if embedded {
        std::env::current_exe()?
    } else {
        guard_exe.to_owned()
    };
    let run = |flags: u32| {
        let mut command = Command::new(&executable);
        if embedded {
            command.arg("--taskbar-guard");
        }
        command
            .arg(parent_pid.to_string())
            .arg(marker)
            .creation_flags(flags)
            .spawn()
    };
    run(base | CREATE_BREAKAWAY_FROM_JOB).or_else(|_| run(base))?;
    *started = true;
    Ok(())
}
