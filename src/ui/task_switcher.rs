use super::{button, status, Buttons};
use gtk::prelude::*;
pub fn build(buttons: &Buttons, note: &gtk::Label, win: &gtk::ApplicationWindow) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let cards = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let list = cards.clone();
    let b = buttons.clone();
    let n = note.clone();
    let window = win.clone();
    root.append(&button("Refresh open windows", buttons, move || {
        while let Some(child) = list.first_child() { list.remove(&child); }
        for (pid, name) in crate::core::processes::list() {
            let n = n.clone(); list.append(&button(&format!("Force stop: {name}"), &b, move || status(&n, crate::core::processes::kill(pid))));
        }
        match crate::core::system::run("wlrctl", &["toplevel", "list"]) {
            Err(e) => n.set_text(&e.to_string()),
            Ok(text) => {
                for line in text.lines() {
                    let Some((id, title)) = line.split_once(':') else { continue };
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8); row.append(&gtk::Label::new(Some(title.trim())));
                    for (label, action) in [("Focus", "focus"), ("Close", "close")] {
                        let matcher = format!("app_id:{}", id.trim()); let n = n.clone(); let w = window.clone();
                        row.append(&button(label, &b, move || {
                            let result = crate::core::system::run("wlrctl", &["toplevel", action, &matcher]);
                            if result.is_ok() && action == "focus" { w.hide(); } status(&n, result);
                        }));
                    } list.append(&row);
                }
                n.set_text("Close requests a graceful exit. Windows sharing an app_id may be matched together.");
            }
        }
    }));
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(350)
        .child(&cards)
        .build();
    root.append(&scroll);
    root
}
