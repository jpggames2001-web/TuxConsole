use anyhow::Result;
use std::{
    path::Path,
    process::{Child, Command},
    sync::{Mutex, OnceLock},
};
pub fn apply(path: &Path) -> Result<String> {
    static CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
    let mut slot = CHILD.get_or_init(|| Mutex::new(None)).lock().unwrap();
    let child = Command::new("swaybg")
        .arg("-i")
        .arg(path)
        .args(["-m", "fill"])
        .spawn()?;
    if let Some(mut old) = slot.take() {
        if old.try_wait()?.is_none() {
            let _ = old.kill();
        }
        let _ = old.wait();
    }
    *slot = Some(child);
    Ok("Wallpaper applied".into())
}
