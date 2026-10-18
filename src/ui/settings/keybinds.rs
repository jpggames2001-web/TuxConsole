use super::super::{button, Buttons};
use crate::core::config_manager::Action;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
pub fn build(
    buttons: &Buttons,
    note: &gtk::Label,
    listen: &Rc<RefCell<Option<Action>>>,
) -> gtk::Box {
    let current =
        crate::core::config_manager::load::<crate::core::config_manager::Keybinds>("keybinds.json")
            .unwrap_or_default();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for (label, action) in [
        ("Change Home", Action::Home),
        ("Change Quick Menu", Action::QuickMenu),
        ("Change Task Manager", Action::TaskManager),
    ] {
        let l = listen.clone();
        let n = note.clone();
        let code = current
            .buttons
            .iter()
            .find(|(_, a)| **a == action)
            .map(|(c, _)| c.to_string())
            .unwrap_or_else(|| "unassigned".into());
        root.append(&button(&format!("{label} [{code}]"), buttons, move || {
            *l.borrow_mut() = Some(action.clone());
            n.set_text("Press the new gamepad button. Existing assignment will be replaced.");
        }));
    }
    root
}
