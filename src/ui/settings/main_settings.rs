use super::super::Buttons;
use crate::core::config_manager::Action;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
pub fn build(
    b: &Buttons,
    n: &gtk::Label,
    listen: &Rc<RefCell<Option<Action>>>,
    background: &gtk::Box,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.append(&super::keybinds::build(b, n, listen));
    root.append(&super::wallpaper::build(b, n, background));
    root
}
