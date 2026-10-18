use anyhow::Result;
use std::{process::Command, thread, time::Duration};
/// Enforce fullscreen for regular toplevels, including apps that later leave fullscreen.
/// Layer-shell surfaces (dashboard/overlays) and popup surfaces are excluded by the protocol.
pub fn run() -> Result<()> {
    loop {
        let result = Command::new("wlrctl")
            .args(["toplevel", "fullscreen", "state:unfullscreen"])
            .output()?;
        if !result.status.success() && !result.stderr.is_empty() {
            anyhow::bail!(
                "Fullscreen policy: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        thread::sleep(Duration::from_millis(250));
    }
}
