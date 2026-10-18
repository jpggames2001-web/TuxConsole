pub mod dashboard;
pub mod login_window;
pub mod quick_menu;
pub mod settings;
pub mod task_switcher;
use crate::core::config_manager::{self, Action, Keybinds};
use anyhow::Result;
use gtk::{glib, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{cell::RefCell, rc::Rc, time::Duration};
pub type Buttons = Rc<RefCell<Vec<gtk::Button>>>;
pub fn button(label: &str, buttons: &Buttons, f: impl Fn() + 'static) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.set_size_request(160, 64);
    b.connect_clicked(move |_| f());
    buttons.borrow_mut().push(b.clone());
    b
}
pub fn status(label: &gtk::Label, result: anyhow::Result<String>) {
    label.set_text(&match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    });
}
pub fn run() -> Result<()> {
    let endpoint = crate::core::ipc::Endpoint::bind()?;
    let socket = endpoint.socket.try_clone()?;
    let app = gtk::Application::builder()
        .application_id("org.tuxconsole.Shell")
        .build();
    let socket = Rc::new(socket);
    app.connect_activate(move |app| {
        let win = gtk::ApplicationWindow::builder()
            .application(app)
            .title("TuxConsole")
            .default_width(1280)
            .default_height(720)
            .build();
        let buttons: Buttons = Rc::default();
        let listening: Rc<RefCell<Option<Action>>> = Rc::default();
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        let note = gtk::Label::new(None);
        note.set_wrap(true);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
        root.set_margin_top(32);
        root.set_margin_bottom(32);
        root.set_margin_start(48);
        root.set_margin_end(48);
        let title = gtk::Label::new(Some("TUXCONSOLE"));
        title.add_css_class("title-1");
        root.append(&title);
        let nav = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        for name in ["Home", "Tasks", "Quick", "Settings"] {
            let s = stack.clone();
            let n = name.to_string();
            nav.append(&button(name, &buttons, move || {
                s.set_visible_child_name(&n)
            }));
        }
        root.append(&nav);
        root.append(&stack);
        root.append(&note);
        win.set_child(Some(&root));
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::prelude::WidgetExt::display(&win),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        if let Some(s) = gtk::Settings::default() {
            s.set_gtk_application_prefer_dark_theme(true);
        }
        let login = std::env::var_os("GREETD_SOCK").is_some();
        if login {
            root.remove(&nav);
            stack.add_named(&login_window::build(&buttons, &note), Some("Login"));
        } else {
            win.init_layer_shell();
            win.set_layer(Layer::Overlay);
            win.set_namespace(Some("tuxconsole"));
            win.set_keyboard_mode(KeyboardMode::Exclusive);
            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                win.set_anchor(edge, true);
            }
            match config_manager::load::<config_manager::Settings>("settings.json") {
                Ok(settings) => {
                    if let Some(path) = settings.wallpaper {
                        if let Err(e) = settings::wallpaper::apply(&path, &root)
                            .and_then(|_| crate::core::wallpaper::apply(&path).map(|_| ()))
                        {
                            note.set_text(&e.to_string());
                        }
                    }
                }
                Err(e) => note.set_text(&e.to_string()),
            }
            stack.add_named(&dashboard::build(&buttons, &note, &win), Some("Home"));
            stack.add_named(&task_switcher::build(&buttons, &note, &win), Some("Tasks"));
            stack.add_named(&quick_menu::build(&buttons, &note), Some("Quick"));
            stack.add_named(
                &settings::main_settings::build(&buttons, &note, &listening, &root),
                Some("Settings"),
            );
        }
        let dispatch: Rc<dyn Fn(u16)> = Rc::new({
            let win = win.clone();
            let buttons = buttons.clone();
            let stack = stack.clone();
            let note = note.clone();
            let listening = listening.clone();
            move |code| {
                if let Some(action) = listening.borrow_mut().take() {
                    let result = (|| {
                        let mut k: Keybinds = config_manager::load("keybinds.json")?;
                        k.assign(code, action);
                        config_manager::save("keybinds.json", &k)?;
                        Ok(format!("Bound button {code}"))
                    })();
                    status(&note, result);
                    return;
                }
                if !login {
                    match config_manager::load::<Keybinds>("keybinds.json") {
                        Ok(k) => {
                            if let Some(action) = k.buttons.get(&code) {
                                stack.set_visible_child_name(match action {
                                    Action::Home => "Home",
                                    Action::TaskManager => "Tasks",
                                    Action::QuickMenu => "Quick",
                                });
                                win.present();
                                return;
                            }
                        }
                        Err(e) => {
                            note.set_text(&e.to_string());
                            return;
                        }
                    }
                }
                if !win.is_visible() {
                    return;
                }
                let visible: Vec<_> = buttons
                    .borrow()
                    .iter()
                    .filter(|b| b.is_mapped() && b.is_sensitive())
                    .cloned()
                    .collect();
                if visible.is_empty() {
                    return;
                }
                let index = visible.iter().position(|b| b.is_focus()).unwrap_or(0);
                match code {
                    304 => visible[index].emit_clicked(),
                    305 if !login => win.hide(),
                    544..=547 => {
                        if let Some(origin) = visible[index].compute_bounds(&win) {
                            let ox = origin.x() + origin.width() / 2.;
                            let oy = origin.y() + origin.height() / 2.;
                            let best = visible
                                .iter()
                                .filter_map(|b| {
                                    let r = b.compute_bounds(&win)?;
                                    let dx = r.x() + r.width() / 2. - ox;
                                    let dy = r.y() + r.height() / 2. - oy;
                                    let (forward, side) = match code {
                                        544 => (-dy, dx),
                                        545 => (dy, dx),
                                        546 => (-dx, dy),
                                        _ => (dx, dy),
                                    };
                                    (forward > 1.).then_some((forward + side.abs() * 3., b))
                                })
                                .min_by(|a, b| a.0.total_cmp(&b.0));
                            if let Some((_, b)) = best {
                                b.grab_focus();
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        let keyboard = gtk::EventControllerKey::new();
        let d = dispatch.clone();
        keyboard.connect_key_pressed(move |_, key, _, _| {
            let code = match key {
                gtk::gdk::Key::Left => 546,
                gtk::gdk::Key::Right => 547,
                gtk::gdk::Key::Up => 544,
                gtk::gdk::Key::Down => 545,
                gtk::gdk::Key::Return => 304,
                gtk::gdk::Key::Escape => 305,
                _ => return glib::Propagation::Proceed,
            };
            d(code);
            glib::Propagation::Stop
        });
        win.add_controller(keyboard);
        let socket = socket.clone();
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut buffer = [0u8; 32];
            for _ in 0..64 {
                match socket.recv(&mut buffer) {
                    Ok(n) => {
                        if let Ok(s) = std::str::from_utf8(&buffer[..n]) {
                            if let Ok(code) = s.parse() {
                                dispatch(code);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            glib::ControlFlow::Continue
        });
        glib::timeout_add_seconds_local(1, || {
            let _ = crate::core::processes::list();
            glib::ControlFlow::Continue
        });
        win.present();
        if let Some(b) = buttons.borrow().iter().find(|b| b.is_mapped()) {
            b.grab_focus();
        };
    });
    let exit = app.run_with_args::<&str>(&[]);
    drop(endpoint);
    if exit != glib::ExitCode::SUCCESS {
        anyhow::bail!("GTK application exited unsuccessfully");
    }
    Ok(())
}
