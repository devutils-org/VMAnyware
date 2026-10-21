#!/usr/bin/env python3
"""Run two-window permission acceptance with an owned sandboxed X display."""
import argparse
import os
from pathlib import Path
import secrets
import signal
import struct
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--prefix', required=True, type=Path)
args = parser.parse_args()
prefix = args.prefix.resolve(strict=True)
repo = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix='vmanyware-command-acl-') as temporary:
    root = Path(temporary)
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'GDK_BACKEND': 'x11',
           'LD_LIBRARY_PATH': str(prefix / 'sysroot/usr/lib'), 'LIBGL_ALWAYS_SOFTWARE': '1',
           '__EGL_VENDOR_LIBRARY_FILENAMES': '/usr/share/glvnd/egl_vendor.d/50_mesa.json',
           '__GLX_VENDOR_LIBRARY_NAME': 'mesa'}
    for key in ('XDG_CONFIG_HOME', 'XDG_CACHE_HOME', 'XDG_DATA_HOME', 'XDG_RUNTIME_DIR'):
        directory = root / key
        directory.mkdir(mode=0o700)
        env[key] = str(directory)
    display = str(1000 + secrets.randbelow(20000))
    def field(value):
        return struct.pack('!H', len(value)) + value
    authority = root / 'Xauthority'
    authority.write_bytes(struct.pack('!H', 65535) + field(b'') + field(display.encode()) + field(b'MIT-MAGIC-COOKIE-1') + field(secrets.token_bytes(16)))
    authority.chmod(0o600)
    env.update(DISPLAY=':' + display, XAUTHORITY=str(authority))
    (root / 'x11').mkdir(mode=0o700)
    overlay = ['bwrap', '--die-with-parent', '--ro-bind', '/', '/', '--tmpfs', '/home',
               '--ro-bind', str(repo), str(repo), '--tmpfs', '/tmp',
               '--ro-bind', str(prefix), str(prefix), '--bind', str(root), str(root),
               '--bind', str(root / 'x11'), '/tmp/.X11-unix',
               '--overlay-src', '/usr', '--overlay-src', str(prefix / 'sysroot/usr'), '--ro-overlay', '/usr',
               '--proc', '/proc', '--dev', '/dev']
    with (root / 'xvfb.log').open('w+') as log:
        xvfb = subprocess.Popen(overlay + ['/usr/bin/Xvfb', ':' + display, '-screen', '0', '1100x900x24', '-nolisten', 'tcp', '-auth', str(authority)],
                                env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 5
            while not (root / 'x11' / ('X' + display)).exists():
                assert xvfb.poll() is None and time.monotonic() < deadline, 'Owned display failed'
                time.sleep(.02)
            process = subprocess.Popen(overlay + ['dbus-run-session', '--', str(repo / 'target/debug/examples/command_acl_smoke')],
                                       env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                       text=True, start_new_session=True)
            try:
                _, output = process.communicate(timeout=40)
                print(output[-10000:], end='')
                assert process.returncode == 0
                assert 'Command ACL main allow / other window deny: passed' in output
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
        finally:
            os.killpg(xvfb.pid, signal.SIGTERM)
            try:
                xvfb.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(xvfb.pid, signal.SIGKILL)
                xvfb.wait()
