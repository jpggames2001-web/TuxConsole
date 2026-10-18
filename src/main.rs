use anyhow::Result;
fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("window-daemon") => tuxconsole::daemon::window_manager::run(),
        Some("input-daemon") => tuxconsole::daemon::input_listener::run(),
        Some("scan") => {
            for app in tuxconsole::core::app_scanner::scan()? {
                println!(
                    "{}\t{}\t{}",
                    if app.game { "game" } else { "app" },
                    app.name,
                    app.exec
                );
            }
            Ok(())
        }
        Some("doctor") => {
            let mut missing = false;
            for tool in ["labwc", "wpctl", "wlr-randr", "wlrctl", "greetd"] {
                let found = std::env::var_os("PATH")
                    .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()));
                println!("{tool}: {}", if found { "found" } else { "missing" });
                missing |= !found;
            }
            if missing {
                anyhow::bail!("Install missing session tools (see README)");
            }
            Ok(())
        }
        Some("--help") => {
            println!("tuxconsole [input-daemon|window-daemon|scan|doctor]\nRun without arguments for the GTK shell (requires --features gui).");
            Ok(())
        }
        _ => {
            #[cfg(feature = "gui")]
            {
                tuxconsole::ui::run()
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!("Build the desktop with cargo build --features gui");
            }
        }
    }
}
