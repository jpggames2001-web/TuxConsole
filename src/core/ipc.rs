use anyhow::{bail, Context, Result};
use std::{
    fs::{File, OpenOptions},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
            net::UnixDatagram,
        },
    },
    path::PathBuf,
};
pub struct Endpoint {
    pub socket: UnixDatagram,
    path: PathBuf,
    _lock: File,
}
impl Endpoint {
    pub fn bind() -> Result<Self> {
        let path = super::system::socket_path()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path.with_extension("lock"))?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            bail!("Another TuxConsole shell is already running");
        }
        match std::fs::symlink_metadata(&path) {
            Ok(m) => {
                if !m.file_type().is_socket() || m.uid() != unsafe { libc::geteuid() } {
                    bail!("Refusing to remove unexpected input endpoint");
                }
                std::fs::remove_file(&path).context("Removing stale shell endpoint")?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let socket = UnixDatagram::bind(&path)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            path,
            _lock: lock,
        })
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
