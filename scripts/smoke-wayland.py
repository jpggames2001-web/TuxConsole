#!/usr/bin/env python3
"""Real GTK/Wayland smoke test. Requires labwc and a GUI-enabled debug build.
Injects gamepad IPC (not physical evdev); never starts greetd or changes the host.
"""
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import struct
import json
import threading
import time

repo = Path(__file__).resolve().parents[1]
binary = repo / 'target/debug/tuxconsole'
processes = []
with tempfile.TemporaryDirectory(prefix='tuxconsole-smoke-') as tmp:
    base = Path(tmp)
    runtime = base / 'runtime'
    runtime.mkdir(mode=0o700)
    home = base / 'home'
    desktop = home / 'Desktop'
    desktop.mkdir(parents=True)
    config = base / 'labwc'
    config.mkdir()
    (config / 'rc.xml').write_text((repo / 'packaging/labwc/rc.xml').read_text())
    marker = base / 'launched'
    (desktop / 'smoke.desktop').write_text(
        '[Desktop Entry]\nType=Application\nName=TuxConsole smoke game\n'
        f'Exec=/usr/bin/touch {marker}\n')
    env = dict(os.environ, HOME=str(home), XDG_RUNTIME_DIR=str(runtime),
               XDG_CONFIG_HOME=str(home / '.config'), XDG_DATA_HOME=str(home / '.local/share'),
               WLR_BACKENDS='headless', WLR_RENDERER='pixman',
               WLR_LIBINPUT_NO_DEVICES='1', GDK_BACKEND='wayland', GSK_RENDERER='cairo')
    env.pop('GREETD_SOCK', None)
    if Path('/workspace/tooling/sysroot/usr/bin/Xwayland').exists():
        env['WLR_XWAYLAND'] = '/workspace/tooling/sysroot/usr/bin/Xwayland'
    def wait_for(check, message):
        for _ in range(100):
            if check():
                return
            if any(p.poll() is not None for p in processes):
                raise RuntimeError('Session exited before ' + message)
            time.sleep(.1)
        raise RuntimeError('Timed out: ' + message)
    with (base / 'session.log').open('w+') as log:
        try:
            compositor = subprocess.Popen(['labwc', '-C', str(config)], env=env,
                                          stdout=log, stderr=log, start_new_session=True)
            processes.append(compositor)
            wait_for(lambda: any(runtime.glob('wayland-*')) , 'Wayland socket')
            displays = [p for p in runtime.glob('wayland-*') if not p.name.endswith('.lock')]
            env['WAYLAND_DISPLAY'] = displays[0].name
            shell = subprocess.Popen(['dbus-run-session', '--', str(binary)], env=env,
                                     stdout=log, stderr=log, start_new_session=True)
            processes.append(shell)
            endpoint = runtime / 'tuxconsole-input.sock'
            wait_for(endpoint.exists, 'shell input endpoint')
            time.sleep(.5)  # allow the first GTK frame to map widgets
            with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as sender:
                for code in [316, 545, 304]:  # Home, down to game, A
                    sender.sendto(str(code).encode(), str(endpoint))
                    time.sleep(.2)
            wait_for(marker.exists, 'gamepad navigation to launch desktop entry')
            duplicate = subprocess.run([str(binary)], env=env, capture_output=True, timeout=5)
            assert duplicate.returncode != 0 and b'already running' in duplicate.stderr
            policy = subprocess.Popen([str(binary), 'window-daemon'], env=env, stdout=log, stderr=log, start_new_session=True)
            processes.append(policy)
            probe = subprocess.Popen(['dbus-run-session', '--', str(repo / 'target/debug/examples/window_probe')], env=env, stdout=log, stderr=log, start_new_session=True)
            processes.append(probe)
            time.sleep(2)  # probe deliberately exits fullscreen after 500 ms
            result = subprocess.run(['wlrctl', 'toplevel', 'find', 'app_id:org.tuxconsole.WindowProbe', 'state:fullscreen'], env=env)
            assert result.returncode == 0, 'Fullscreen enforcement failed'
            # Exercise the greeter UI against a test-only greetd server; never real PAM.
            for running in [probe, shell]:
                os.killpg(running.pid, signal.SIGTERM)
                running.wait(timeout=5)
                processes.remove(running)
            endpoint.unlink(missing_ok=True)
            greet_socket = base / 'greetd.sock'
            server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            server.bind(str(greet_socket))
            server.listen()
            received = []
            def greeter_server():
                with server.accept()[0] as client:
                    def read_request():
                        size = struct.unpack('=I', client.recv(4))[0]
                        data = b''
                        while len(data) < size:
                            data += client.recv(size-len(data))
                        return json.loads(data)
                    def respond(value):
                        data = json.dumps(value).encode()
                        client.sendall(struct.pack('=I', len(data)) + data)
                    received.append(read_request())
                    respond({'type':'auth_message', 'auth_message_type':'secret', 'auth_message':'Password:'})
                    received.append(read_request())
                    respond({'type':'error', 'error_type':'auth_error'})
            threading.Thread(target=greeter_server, daemon=True).start()
            env['GREETD_SOCK'] = str(greet_socket)
            greeter = subprocess.Popen(['dbus-run-session', '--', str(binary)], env=env, stdout=log, stderr=log, start_new_session=True)
            processes.append(greeter)
            wait_for(endpoint.exists, 'greeter endpoint')
            time.sleep(.5)
            with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as sender:
                for code in [304, 545, 545, 545, 304, 544, 304]:
                    sender.sendto(str(code).encode(), str(endpoint))
                    time.sleep(.2)
            wait_for(lambda: len(received) == 2, 'virtual keyboard authentication submission')
            assert received[0]['type'] == 'create_session'
            assert received[1] == {'type':'post_auth_message_response', 'response':'h'}
            server.close()
            print('PASS: GTK/Wayland; gamepad IPC; launch; singleton; fullscreen; greeter virtual keyboard with mock greetd')
        except Exception:
            log.flush()
            print((base / 'session.log').read_text())
            raise
        finally:
            for process in reversed(processes):
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
