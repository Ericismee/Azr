use std::sync::mpsc::{channel, Receiver, Sender};
use windows::Devices::Radios::{Radio, RadioKind, RadioState};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
    pub ssid: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Wifi,
    Bluetooth,
}

pub enum Cmd {
    Refresh,
    Set(Kind, bool),
}

pub struct Radios {
    pub commands: Sender<Cmd>,
    pub snapshots: Receiver<Snapshot>,
}

fn read_all() -> Vec<Radio> {
    let Some(list) = Radio::GetRadiosAsync().ok().and_then(|o| o.join().ok()) else {
        return vec![];
    };
    (0..list.Size().unwrap_or(0))
        .filter_map(|i| list.GetAt(i).ok())
        .collect()
}

fn snapshot(radios: &[Radio], include_ssid: bool) -> Snapshot {
    let state = |k: RadioKind| {
        radios
            .iter()
            .find(|r| r.Kind().ok() == Some(k))
            .map(|r| r.State().ok() == Some(RadioState::On))
    };
    Snapshot {
        wifi: state(RadioKind::WiFi),
        bluetooth: state(RadioKind::Bluetooth),
        ssid: if include_ssid {
            crate::wifi::ssid()
        } else {
            None
        },
    }
}

pub fn spawn(notify: impl Fn() + Send + 'static) -> Radios {
    spawn_worker(false, notify)
}

pub fn spawn_with_ssid(notify: impl Fn() + Send + 'static) -> Radios {
    spawn_worker(true, notify)
}

fn spawn_worker(include_ssid: bool, notify: impl Fn() + Send + 'static) -> Radios {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (snap_tx, snap_rx) = channel::<Snapshot>();
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let _ = Radio::RequestAccessAsync().ok().and_then(|o| o.join().ok());
        let _ = snap_tx.send(snapshot(&read_all(), include_ssid));
        notify();
        while let Ok(cmd) = cmd_rx.recv() {
            if let Cmd::Set(kind, on) = cmd {
                let want = match kind {
                    Kind::Wifi => RadioKind::WiFi,
                    Kind::Bluetooth => RadioKind::Bluetooth,
                };
                for r in read_all() {
                    if r.Kind().ok() == Some(want) {
                        let st = if on { RadioState::On } else { RadioState::Off };
                        let res = r.SetStateAsync(st).ok().and_then(|o| o.join().ok());
                        crate::log::line("radio", &format!("{kind:?} -> {on}: {res:?}"));
                    }
                }
            }
            let _ = snap_tx.send(snapshot(&read_all(), include_ssid));
            notify();
        }
    });
    Radios {
        commands: cmd_tx,
        snapshots: snap_rx,
    }
}
