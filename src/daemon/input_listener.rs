use anyhow::Result;
use evdev::{Device, EventType, KeyCode};
use std::{
    collections::HashSet,
    os::unix::net::UnixDatagram,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

// Never grab devices: games must retain their ordinary gamepad input.
// Only game controllers (BTN_GAMEPAD + directional axes) are accepted, not keyboards.
pub fn run() -> Result<()> {
    let socket = UnixDatagram::unbound()?;
    let target = crate::core::system::socket_path()?;
    let (tx, rx) = mpsc::channel();
    let mut attached = HashSet::new();
    let mut last_scan = Instant::now() - Duration::from_secs(2);
    loop {
        if last_scan.elapsed() >= Duration::from_secs(1) {
            last_scan = Instant::now();
            for (path, device) in evdev::enumerate() {
                if attached.contains(&path)
                    || !device
                        .supported_keys()
                        .is_some_and(|k| k.contains(KeyCode::BTN_SOUTH))
                {
                    continue;
                }
                attached.insert(path.clone());
                let tx = tx.clone();
                thread::spawn(move || {
                    let result = listen(device, &tx);
                    if let Err(e) = result {
                        eprintln!("Gamepad disconnected: {e}");
                    }
                    let _ = tx.send((path, None));
                });
            }
        }
        while let Ok((path, code)) = rx.try_recv() {
            if let Some(code) = code {
                let _ = socket.send_to(code.to_string().as_bytes(), &target);
            } else {
                attached.remove(&path);
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
}
fn listen(mut device: Device, tx: &mpsc::Sender<(std::path::PathBuf, Option<u16>)>) -> Result<()> {
    let path = std::path::PathBuf::new();
    loop {
        for event in device.fetch_events()? {
            if event.event_type() == EventType::KEY && event.value() == 1 {
                tx.send((path.clone(), Some(event.code())))?;
            }
            if event.event_type() == EventType::ABSOLUTE && event.value() != 0 {
                let code = match (event.code(), event.value().signum()) {
                    (16, -1) => 546,
                    (16, 1) => 547,
                    (17, -1) => 544,
                    (17, 1) => 545,
                    _ => continue,
                };
                tx.send((path.clone(), Some(code)))?;
            }
        }
    }
}
