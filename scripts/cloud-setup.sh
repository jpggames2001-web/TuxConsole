#!/bin/bash
# Non-root Debian cloud development toolchain, confined to /workspace/tooling.
set -euo pipefail
cd /workspace/TuxConsole
base=/workspace/tooling
mkdir -p "$base/apt/"{lists/partial,cache/archives/partial,empty} "$base/sysroot"
export CARGO_HOME="$base/cargo" RUSTUP_HOME="$base/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
if ! command -v cargo >/dev/null; then
  curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o "$base/rustup-init.sh"
  sh "$base/rustup-init.sh" -y --profile minimal --default-toolchain 1.99.1 --no-modify-path
fi
rustup component add rustfmt clippy
cat > "$base/apt/sources.list" <<'SOURCES'
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie main
SOURCES
apt_options=(-o "Dir::Etc::parts=$base/apt/empty" -o "Dir::Etc::sourcelist=$base/apt/sources.list" -o "Dir::Etc::sourceparts=$base/apt/empty" -o "Dir::State::lists=$base/apt/lists" -o "Dir::Cache=$base/apt/cache" -o Debug::NoLocking=true)
/usr/bin/apt-get "${apt_options[@]}" update
/usr/bin/apt-get "${apt_options[@]}" --download-only -y install libgtk-4-dev libgtk4-layer-shell-dev libwayland-dev libxkbcommon-dev labwc wlr-randr swaybg wlrctl greetd grim wireplumber pipewire-bin
for deb in "$base"/apt/cache/archives/*.deb; do dpkg-deb -x "$deb" "$base/sysroot"; done
python3 - <<'PY'
from pathlib import Path
root = Path('/workspace/tooling/sysroot')
for p in root.rglob('*.pc'):
    # Each extraction restores original package files before prefix rewriting.
    s = p.read_text()
    p.write_text(s.replace('/usr', str(root) + '/usr'))
for p in root.rglob('*'):
    if p.is_symlink() and not p.exists():
        target = p.readlink()
        candidate = target if target.is_absolute() else Path('/' + str(p.parent.relative_to(root))) / target
        if candidate.exists():
            p.unlink()
            p.symlink_to(candidate.resolve())
PY
cat > "$base/activate" <<'ACTIVATE'
export CARGO_HOME=/workspace/tooling/cargo
export RUSTUP_HOME=/workspace/tooling/rustup
export PATH=/workspace/tooling/cargo/bin:/workspace/tooling/sysroot/usr/bin:/workspace/tooling/sysroot/usr/sbin:$PATH
export PKG_CONFIG_PATH=/workspace/tooling/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig:/workspace/tooling/sysroot/usr/share/pkgconfig
export LD_LIBRARY_PATH=/workspace/tooling/sysroot/usr/lib/x86_64-linux-gnu
export XDG_DATA_DIRS=/workspace/tooling/sysroot/usr/share:/usr/local/share:/usr/share
ACTIVATE
. "$base/activate"
cargo build --locked --features gui --examples
cargo build --locked --features gui
cargo test --locked --features gui
python3 scripts/smoke-wayland.py
