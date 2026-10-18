use anyhow::{bail, Result};
use std::process::Command;
pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let result = Command::new(program).args(args).output()?;
    if !result.status.success() {
        bail!("{program}: {}", String::from_utf8_lossy(&result.stderr));
    }
    Ok(String::from_utf8_lossy(&result.stdout).into())
}
pub fn volume(delta: &str) -> Result<String> {
    if !["5%+", "5%-"].contains(&delta) {
        bail!("Invalid volume step");
    }
    run(
        "wpctl",
        &["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", delta],
    )?;
    run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])
}
pub fn socket_path() -> Result<std::path::PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| anyhow::anyhow!("XDG_RUNTIME_DIR is required"))?;
    let dir = std::path::PathBuf::from(dir);
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(&dir)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        bail!("Runtime directory must be private and owned by current user");
    }
    Ok(dir.join("tuxconsole-input.sock"))
}
