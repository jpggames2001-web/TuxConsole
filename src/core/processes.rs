//! Only kill processes launched by this shell and still held as unreaped Child handles.
//! Never infer a PID from a title, app_id or another user's window.
use anyhow::{bail, Result};
use std::{
    process::Child,
    sync::{Mutex, OnceLock},
};
struct Managed {
    name: String,
    child: Child,
}
fn registry() -> &'static Mutex<Vec<Managed>> {
    static REGISTRY: OnceLock<Mutex<Vec<Managed>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}
pub fn register(name: String, child: Child) {
    registry().lock().unwrap().push(Managed { name, child });
}
pub fn list() -> Vec<(u32, String)> {
    let mut items = registry().lock().unwrap();
    items.retain_mut(|p| matches!(p.child.try_wait(), Ok(None)));
    items
        .iter()
        .map(|p| (p.child.id(), p.name.clone()))
        .collect()
}
pub fn kill(pid: u32) -> Result<String> {
    let mut items = registry().lock().unwrap();
    let Some(i) = items.iter().position(|p| p.child.id() == pid) else {
        bail!("Process is no longer managed");
    };
    let p = &mut items[i];
    if p.child.try_wait()?.is_some() {
        items.remove(i);
        bail!("Application already exited");
    }
    p.child.kill()?;
    p.child.wait()?;
    items.remove(i);
    Ok("Application terminated".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kill_owned_child_and_reject_unknown_pid() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        register("test".into(), child);
        assert!(list().iter().any(|p| p.0 == pid));
        assert!(kill(pid).is_ok());
        assert!(kill(pid).is_err());
    }
}
