#!/bin/sh
set -eu
# Run from the repository root as a regular user. Does not enable a display manager.
command -v pacman >/dev/null || { echo 'This installer targets Arch Linux.' >&2; exit 1; }
sudo pacman -S --needed rust gtk4 gtk4-layer-shell labwc greetd wireplumber pipewire wlr-randr swaybg dbus pkgconf xorg-xwayland xdg-user-dirs
# wlrctl availability depends on the enabled repositories; review an AUR PKGBUILD if needed.
command -v wlrctl >/dev/null || { echo "Install wlrctl before deploying the fullscreen/window policy." >&2; exit 1; }
cargo build --release --locked --features gui
sudo install -Dm755 target/release/tuxconsole /usr/local/bin/tuxconsole
sudo install -Dm755 packaging/tuxconsole-session /usr/local/bin/tuxconsole-session
sudo install -Dm755 packaging/tuxconsole-components /usr/local/libexec/tuxconsole-components
sudo install -Dm644 packaging/labwc/rc.xml /etc/tuxconsole/labwc/rc.xml
sudo install -Dm644 packaging/tuxconsole.desktop /usr/share/wayland-sessions/tuxconsole.desktop
sudo install -Dm644 packaging/70-tuxconsole-gamepad.rules /etc/udev/rules.d/70-tuxconsole-gamepad.rules
printf '%s\n' 'Installed. Review packaging/greetd/config.toml and README before enabling greetd.'
