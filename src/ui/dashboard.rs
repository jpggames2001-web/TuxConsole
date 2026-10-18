use super::{button, Buttons};
use gtk::prelude::*;
pub fn build(buttons: &Buttons, note: &gtk::Label, win: &gtk::ApplicationWindow) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    match crate::core::app_scanner::scan() {
        Err(e) => note.set_text(&e.to_string()),
        Ok(apps) => {
            for game in [true, false] {
                root.append(&gtk::Label::new(Some(if game {
                    "Games"
                } else {
                    "Apps / Software"
                })));
                let grid = gtk::FlowBox::new();
                grid.set_selection_mode(gtk::SelectionMode::None);
                grid.set_max_children_per_line(5);
                for app in apps.iter().filter(|a| a.game == game) {
                    let app = app.clone();
                    let n = note.clone();
                    let w = win.clone();
                    grid.insert(
                        &button(&app.name.clone(), buttons, move || match app.launch() {
                            Ok(child) => {
                                crate::core::processes::register(app.name.clone(), child);
                                w.hide();
                            }
                            Err(e) => n.set_text(&format!("Launch failed: {e}")),
                        }),
                        -1,
                    );
                }
                root.append(&grid);
            }
        }
    }
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(400)
        .child(&root)
        .build();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.append(&scroll);
    outer
}
