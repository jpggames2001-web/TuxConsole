use super::super::{button, status, Buttons};
use crate::core::config_manager::{self, Settings};
use gtk::prelude::*;
pub fn build(buttons: &Buttons, note: &gtk::Label, background: &gtk::Box) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    if let Ok(entries) = std::fs::read_dir(home.join("Pictures/Wallpapers")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file()
                || !path.extension().is_some_and(|e| {
                    ["png", "jpg", "jpeg", "webp"]
                        .iter()
                        .any(|x| e.eq_ignore_ascii_case(x))
                })
            {
                continue;
            }
            let n = note.clone();
            let bg = background.clone();
            root.append(&button(
                &entry.file_name().to_string_lossy(),
                buttons,
                move || {
                    let result = (|| {
                        apply(&path, &bg)?;
                        let message = crate::core::wallpaper::apply(&path)?;
                        config_manager::save(
                            "settings.json",
                            &Settings {
                                wallpaper: Some(path.clone()),
                            },
                        )?;
                        Ok(message)
                    })();
                    status(&n, result);
                },
            ));
        }
    }
    root
}
pub fn apply(path: &std::path::Path, root: &gtk::Box) -> anyhow::Result<()> {
    if !path.is_file() {
        anyhow::bail!("Wallpaper does not exist");
    }
    let uri = gtk::gio::File::for_path(path)
        .uri()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    root.set_widget_name("tuxconsole-background");
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&format!(
        "#tuxconsole-background {{ background-image: url(\"{uri}\"); background-size: cover; }}"
    ));
    root.style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    Ok(())
}
