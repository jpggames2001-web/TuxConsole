use anyhow::{bail, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
};
#[derive(Clone, Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub path: Option<PathBuf>,
    pub game: bool,
    pub source: PathBuf,
}
impl App {
    pub fn parse(source: &Path, text: &str, desktop: bool) -> Option<Self> {
        let mut fields = BTreeMap::new();
        let mut section = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line == "[Desktop Entry]";
                continue;
            }
            if section && !line.starts_with('#') {
                if let Some((k, v)) = line.split_once('=') {
                    fields.insert(k, v);
                }
            }
        }
        if fields.get("Type") != Some(&"Application")
            || fields.get("Hidden") == Some(&"true")
            || fields.get("NoDisplay") == Some(&"true")
            || fields.get("Terminal") == Some(&"true")
        {
            return None;
        }
        let exec = fields.get("Exec")?.to_string();
        let game = desktop
            || fields
                .get("Categories")
                .is_some_and(|s| s.split(';').any(|c| c == "Game"));
        Some(Self {
            id: source.file_name()?.to_string_lossy().into(),
            name: fields.get("Name")?.to_string(),
            exec,
            icon: fields.get("Icon").map(|s| s.to_string()),
            path: fields
                .get("Path")
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
            game,
            source: source.into(),
        })
    }
    pub fn argv(&self) -> Result<Vec<String>> {
        let words = shlex::split(&self.exec).context("Malformed desktop Exec quoting")?;
        let mut out = Vec::new();
        for word in words {
            match word.as_str() {
                "%f" | "%F" | "%u" | "%U" => {}
                "%i" => {
                    if let Some(icon) = &self.icon {
                        out.extend(["--icon".into(), icon.clone()]);
                    }
                }
                "%c" => out.push(self.name.clone()),
                "%k" => out.push(self.source.to_string_lossy().into()),
                _ => {
                    let escaped = word.replace("%%", "\0");
                    if escaped.contains('%') {
                        bail!("Unsupported embedded desktop field code");
                    }
                    out.push(escaped.replace('\0', "%"));
                }
            }
        }
        if out.is_empty() {
            bail!("Empty desktop Exec");
        }
        Ok(out)
    }
    pub fn launch(&self) -> Result<Child> {
        let args = self.argv()?;
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        if let Some(path) = &self.path {
            cmd.current_dir(path);
        }
        Ok(cmd.spawn()?)
    }
}
pub fn scan() -> Result<Vec<App>> {
    let home = PathBuf::from(std::env::var_os("HOME").context("HOME missing")?);
    let desktop = Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .filter(|p| p.is_absolute() && *p != home)
        .unwrap_or_else(|| home.join("Desktop"));
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let mut roots = vec![(desktop, true), (data_home.join("applications"), false)];
    for p in std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
        .split(':')
        .filter(|p| !p.is_empty())
    {
        roots.push((PathBuf::from(p).join("applications"), false));
    }
    let mut seen = BTreeSet::new();
    let mut apps = Vec::new();
    for (dir, game) in roots {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_none_or(|s| s != "desktop") {
                continue;
            }
            let id = path.file_name().unwrap().to_string_lossy().to_string();
            if !seen.insert(id) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&path) {
                if let Some(app) = App::parse(&path, &text, game) {
                    apps.push(app);
                }
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app(exec: &str) -> App {
        App::parse(
            Path::new("test.desktop"),
            &format!("[Desktop Entry]\nType=Application\nName=Game\nExec={exec}\n"),
            true,
        )
        .unwrap()
    }
    #[test]
    fn launch_is_not_a_shell() {
        assert_eq!(
            app("game 'a b' ';' %U %%").argv().unwrap(),
            vec!["game", "a b", ";", "%"]
        );
    }
    #[test]
    fn rejects_invalid_field() {
        assert!(app("game --url=%u").argv().is_err());
    }
    #[test]
    fn hidden_entries_are_excluded() {
        assert!(App::parse(
            Path::new("x"),
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nHidden=true",
            false
        )
        .is_none());
    }
    #[test]
    fn ignores_other_sections() {
        assert_eq!(App::parse(Path::new("x"), "[Desktop Entry]\nType=Application\nName=Good\nExec=good\n[Desktop Action X]\nName=Bad\nExec=bad", false).unwrap().name, "Good");
    }
}
