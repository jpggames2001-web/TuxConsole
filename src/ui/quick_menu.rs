use super::{button, status, Buttons};
use gtk::prelude::*;
use serde_json::Value;
pub fn build(buttons: &Buttons, note: &gtk::Label) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for (label, delta) in [("Volume −", "5%-"), ("Volume +", "5%+")] {
        let n = note.clone();
        root.append(&button(label, buttons, move || {
            status(&n, crate::core::system::volume(delta))
        }));
    }
    let modes = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let list = modes.clone();
    let b = buttons.clone();
    let n = note.clone();
    root.append(&button("Refresh display modes", buttons, move || {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let result = (|| -> anyhow::Result<String> {
            let json: Value =
                serde_json::from_str(&crate::core::system::run("wlr-randr", &["--json"])?)?;
            for output in json
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Invalid display response"))?
            {
                let Some(name) = output["name"].as_str() else {
                    continue;
                };
                for mode in output["modes"].as_array().into_iter().flatten() {
                    let (Some(w), Some(h), Some(hz)) = (
                        mode["width"].as_u64(),
                        mode["height"].as_u64(),
                        mode["refresh"].as_f64(),
                    ) else {
                        continue;
                    };
                    if hz <= 0. {
                        continue;
                    }
                    let spec = format!("{w}x{h}@{:.3}Hz", hz / 1000.);
                    let output = name.to_string();
                    let n = n.clone();
                    list.append(&button(&format!("{output} · {spec}"), &b, move || {
                        status(
                            &n,
                            crate::core::system::run(
                                "wlr-randr",
                                &["--output", &output, "--mode", &spec],
                            ),
                        )
                    }));
                }
            }
            Ok("Choose an advertised display mode".into())
        })();
        status(&n, result);
    }));
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(300)
        .child(&modes)
        .build();
    root.append(&scroll);
    root
}
