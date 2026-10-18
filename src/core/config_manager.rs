use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Home,
    QuickMenu,
    TaskManager,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keybinds {
    pub buttons: BTreeMap<u16, Action>,
}
impl Default for Keybinds {
    fn default() -> Self {
        Self {
            buttons: [
                (316, Action::Home),
                (314, Action::TaskManager),
                (315, Action::QuickMenu),
            ]
            .into(),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub wallpaper: Option<PathBuf>,
}
pub fn directory() -> Result<PathBuf> {
    Ok(std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .context("HOME or XDG_CONFIG_HOME must be set")?
        .join("tuxconsole"))
}
pub fn load<T: serde::de::DeserializeOwned + Default>(name: &str) -> Result<T> {
    let path = directory()?.join(name);
    match fs::read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)
            .with_context(|| format!("Invalid {}", path.display()))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}
pub fn save<T: Serialize>(name: &str, value: &T) -> Result<()> {
    let dir = directory()?;
    fs::create_dir_all(&dir)?;
    // A unique exclusive temporary file prevents symlink clobbering and concurrent writers.
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| -> Result<()> {
        file.write_all(&serde_json::to_vec_pretty(value)?)?;
        file.sync_all()?;
        fs::rename(&temp, dir.join(name))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
impl Keybinds {
    pub fn assign(&mut self, code: u16, action: Action) {
        self.buttons.retain(|_, a| *a != action);
        self.buttons.insert(code, action);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remapping_removes_old_and_replaces_collision() {
        let mut k = Keybinds::default();
        k.assign(315, Action::Home);
        assert!(!k.buttons.contains_key(&316));
        assert_eq!(k.buttons.get(&315), Some(&Action::Home));
        assert!(!k.buttons.values().any(|a| *a == Action::QuickMenu));
    }
    #[test]
    fn config_round_trip() {
        let k = Keybinds::default();
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(
            serde_json::from_str::<Keybinds>(&json).unwrap().buttons,
            k.buttons
        );
    }
}
