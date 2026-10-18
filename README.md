# TuxConsole

A Rust / GTK4 desktop-shell prototype for a gamepad-first Arch Linux living-room PC.
The session uses **labwc**, not a new compositor. GTK layer-shell keeps the Home,
Tasks and Quick views above fullscreen clients. A separate evdev process handles
gamepads globally; a separate window-policy process reapplies fullscreen.

This is an initial implementation, **not a production-ready DE or a completed
implementation of every requirement**. See the remaining work below.

## Develop

Requires current stable Rust, GTK4, gtk4-layer-shell, pkg-config and a C toolchain.

```sh
cargo build --locked --features gui
cargo test --locked --features gui
cargo clippy --locked --all-targets --features gui -- -D warnings
cargo build --locked --features gui --examples
python3 scripts/smoke-wayland.py
```

The smoke test also needs labwc, Xwayland, wlrctl and dbus-run-session. It creates
an isolated headless compositor and temporary HOME, sends gamepad IPC messages,
launches a `.desktop` fixture, checks singleton protection, checks fullscreen
re-enforcement against a real GTK client, and uses the virtual keyboard against
a mock greetd server. It cleans up its own processes.
It does not exercise physical evdev, real PAM, a physical display or audio.
The greeter UI smoke assumes a local UID 1000–65533 account, as in this cloud image.

For this Debian cloud workspace, `bash scripts/cloud-setup.sh` installs verified
Debian packages into `/workspace/tooling/sysroot` without root, installs Rust
under `/workspace/tooling`, builds and runs the checks. Use
`. /workspace/tooling/activate` in subsequent shells. This helper targets Debian
13 amd64; `scripts/install-arch.sh` targets the actual Arch machine.
Each cloud task is already isolated: use the existing checkout, without creating
an additional worktree.

Without GTK dependencies, the core is also usable:

```sh
cargo test --locked
cargo run --locked -- scan
cargo run --locked -- doctor
```

The default Cargo feature set intentionally omits GTK to support core development
on machines without native GTK libraries. The desktop requires `--features gui`.

## Arch session installation

Read the install script before executing it on an Arch machine. Install `wlrctl`
from an available package source first (review its PKGBUILD if using the AUR).
Then run `./scripts/install-arch.sh` from this checkout. It builds the GUI and
installs the binary, session entry, compositor config, and narrowly scoped udev
rule. It does **not** enable a display manager or overwrite your greetd config.
Dependencies include GTK4, gtk4-layer-shell, labwc, greetd, PipeWire,
WirePlumber (`wpctl`), wlr-randr, wlrctl, swaybg, D-Bus and Xwayland for X11 games.
The script installs the packages available through pacman; configure PipeWire
and WirePlumber's user services for the target machine as normal on Arch.

Select **TuxConsole** from an existing Wayland-capable login manager to test the
user session. `tuxconsole-session` starts labwc and a supervised component group.
Closing the shell ends that session and cleans up its two daemons.
For direct boot, merge `packaging/greetd/config.toml` into `/etc/greetd/config.toml`
and enable `greetd.service` after configuring it as the machine's sole display
manager. Keep a working recovery TTY during deployment. No desktop process is
run as root. Never use shell auto-login to bypass password authentication.

### Gamepad login permissions

Ordinary users get gamepad input via logind's active-seat access and
`packaging/70-tuxconsole-gamepad.rules`. The greeter runs before the user session
and may not have that access. To enable gamepad login, create a dedicated system
group `tuxconsole-gamepad`, add **only the greeter account** to it, and install
`packaging/71-tuxconsole-greeter-gamepad.rules` under `/etc/udev/rules.d/`.
Reload udev rules and reconnect the controller. Do not add the greeter to the
broad `input` group or make `/dev/input` world-readable. The kernel must mark the
controller with `ID_INPUT_JOYSTICK=1`.

`greetd` supplies `GREETD_SOCK`; TuxConsole then displays the user selector and
virtual keyboard. It implements greetd's framed JSON authentication protocol;
greetd performs PAM authentication, account checks and session creation. The
session wrapper removes `GREETD_SOCK` before starting the authenticated shell.
Local users with UIDs 1000–65533 and login shells are listed. A user's `.face`
image is displayed when readable. Passwords are masked and never logged or saved.

## Controls and behavior

| Input | Action |
| --- | --- |
| D-pad (hat axes or BTN_DPAD) | Spatial focus navigation |
| A / BTN_SOUTH | Activate focused button |
| B / BTN_EAST | Hide shell, return to current application |
| Guide / BTN_MODE (316) | Home |
| Select / BTN_SELECT (314) | Tasks |
| Start / BTN_START (315) | Quick menu |

Arrow keys, Enter and Escape provide development fallbacks. Global bindings
are configurable in Settings: choose Change, then press the desired gamepad
button. The new mapping is saved atomically and loaded on the next input event.
Bindings replace collisions; the status label reports the newly assigned code.
Buttons use Linux evdev codes, not device-specific Xbox/PlayStation labels.
The daemon does **not** grab input devices, so games still receive their normal
controller input (including global shortcut buttons).

- **Games:** scan the XDG desktop directory, falling back to `~/Desktop`.
- **Apps:** scan XDG application directories with user entries taking precedence.
  Hidden, NoDisplay, non-Application and terminal entries are excluded.
  `Exec` arguments are parsed without a shell; common standalone field codes are
  supported. Invalid or unsupported embedded codes produce a launch error.
- **Tasks:** refresh to obtain real foreign-toplevel titles, focus or gracefully
  close matching windows. Force stop is available for live direct child processes
  launched by this shell; it never guesses a PID from an application title.
- **Quick:** change volume through wpctl, refresh the monitor's advertised modes
  and select one with the D-pad. No refresh rate is invented or hard-coded.
- **Wallpaper:** select images in `~/Pictures/Wallpapers`, apply to the dashboard
  and to the compositor background via swaybg, and restore the saved selection.

Configuration lives in `$XDG_CONFIG_HOME/tuxconsole` or `~/.config/tuxconsole`:
`keybinds.json` maps numeric button codes to `home`, `quick_menu`, `task_manager`;
`settings.json` stores `wallpaper`. Malformed files are reported rather than
silently overwritten. Files are written with mode 0600 using atomic replacement.
The input socket is private to `$XDG_RUNTIME_DIR`; a file lock rejects duplicate
shells and permits safe recovery from stale sockets after a crash.

## Architecture and current limits

`src/core` contains configuration, launcher scanning, process ownership, wallpaper
and IPC. `src/daemon` contains evdev discovery/readers and the fullscreen policy.
`src/ui` contains GTK pages, including the greetd greeter. `packaging` contains
Arch deployment examples; none were applied to the cloud host's boot configuration.

The fullscreen policy requests fullscreen for unfullscreen regular toplevels at
250 ms intervals, in addition to labwc's initial window rule. Popups and layer
surfaces are not ordinary toplevels. This has been checked with labwc headless;
compatibility with games, multiple monitors and the target GPU still needs testing.

Outstanding product work:

- Live per-window thumbnails need compositor capture integration; the current
  switcher uses titles, not fabricated screenshots.
- `wlrctl` matches windows by app_id. Multiple windows with the same app_id are
  ambiguous. Exact per-window identity requires a direct foreign-toplevel client.
- Force termination covers owned direct child processes, not detached Steam,
  Flatpak or D-Bus-activated processes. Arbitrary-window force kill is not implemented.
- The desktop parser supports common launcher entries, not the full Freedesktop
  specification (e.g. D-Bus activation, desktop actions or every escaping rule).
- Gamepad repeat/hold gestures, device profiles and controller disconnect UI are
  not implemented; global shortcuts currently trigger on button presses.
- Hot-plug gamepads are discovered, but physical controller permissions, Bluetooth,
  actual PAM, Arch boot, audio hardware and TV refresh switching remain unverified.
- The greeter currently enumerates local passwd users and handles synchronous
  PAM conversations with bounded socket timeouts; remote users and production
  authentication hardening need further work. The OSK currently uses ASCII keys.

No user credentials are needed to build or run the headless tests.
