//! A regular client that deliberately leaves fullscreen, for compositor smoke tests.
use gtk::{glib, prelude::*};
fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("org.tuxconsole.WindowProbe")
        .build();
    app.connect_activate(|app| {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Window policy probe")
            .default_width(320)
            .default_height(240)
            .build();
        window.present();
        glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            window.unfullscreen()
        });
    });
    app.run()
}
