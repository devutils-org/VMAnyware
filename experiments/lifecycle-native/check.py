#!/usr/bin/env python3
"""Owned Linux Xvfb/WebKit/Tauri -> actual Axum -> generated HTTPS lifecycle smoke."""
import argparse
import http.server
import json
import os
from pathlib import Path
import queue
import secrets
import signal
import socket
import ssl
import struct
import subprocess
import tempfile
import threading
import time
import urllib.parse
import urllib.request

REPO = Path(__file__).resolve().parents[2]


def free_port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--prefix', type=Path, required=True, help='Verified package prefix containing sysroot/usr')
    args = parser.parse_args()
    prefix = args.prefix.resolve()
    assert (prefix / 'sysroot/usr/bin/Xvfb').is_file()
    children = []
    with tempfile.TemporaryDirectory(prefix='vmanyware-lifecycle-native-') as temporary:
        directory = Path(temporary)
        env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'GDK_BACKEND': 'x11',
               'LD_LIBRARY_PATH': str(prefix / 'sysroot/usr/lib'), 'LIBGL_ALWAYS_SOFTWARE': '1',
               '__EGL_VENDOR_LIBRARY_FILENAMES': '/usr/share/glvnd/egl_vendor.d/50_mesa.json',
               '__GLX_VENDOR_LIBRARY_NAME': 'mesa'}
        for key, name in [('XDG_CONFIG_HOME', 'config'), ('XDG_CACHE_HOME', 'cache'),
                          ('XDG_DATA_HOME', 'data'), ('XDG_RUNTIME_DIR', 'runtime')]:
            (directory / name).mkdir(mode=0o700)
            env[key] = str(directory / name)
        certificate, key = directory / 'ca.pem', directory / 'generated-key.pem'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
                        '-nodes', '-days', '1', '-addext', 'basicConstraints=critical,CA:FALSE',
                        '-addext', 'extendedKeyUsage=serverAuth', '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost',
                        '-keyout', str(key), '-out', str(certificate)], check=True,
                       env={'PATH': '/usr/bin:/bin'}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        state = {'running': False, 'marker': '', 'writes': [], 'errors': []}

        class Peer(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def reply(self, data):
                body = json.dumps({'data': data}).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def handle_one_request(self):
                try:
                    super().handle_one_request()
                except Exception as error:
                    state['errors'].append(type(error).__name__)
                    raise

            def do_GET(self):
                assert self.headers.get('Authorization') == 'PVEAPIToken=reader@pve!fixture=fixture-only-token'
                if self.path == '/api2/json/nodes/pve-test/qemu/202/config?current=1':
                    self.reply({'digest': 'b' * 40, 'tags': state['marker'], 'template': 0})
                elif self.path == '/api2/json/nodes/pve-test/qemu/202/pending':
                    self.reply([])
                elif self.path == '/api2/json/nodes/pve-test/qemu/202/status/current':
                    self.reply({'vmid': 202, 'status': 'running' if state['running'] else 'stopped',
                                'qmpstatus': 'running' if state['running'] else None})
                elif self.path.startswith('/api2/json/nodes/pve-test/tasks/') and self.path.endswith('/status'):
                    upid = urllib.parse.unquote(self.path.split('/tasks/')[1][:-len('/status')])
                    assert upid == state['upid']
                    self.reply({'node': 'pve-test', 'upid': upid, 'status': 'stopped', 'exitstatus': 'OK'})
                else:
                    raise AssertionError('Unexpected provider GET path')

            def do_POST(self):
                assert self.headers.get('Authorization') == 'PVEAPIToken=reader@pve!fixture=fixture-only-token'
                assert self.headers.get('Content-Type') == 'application/x-www-form-urlencoded'
                body = self.rfile.read(int(self.headers.get('Content-Length', '0'))).decode()
                action = self.path.rsplit('/', 1)[-1]
                assert self.path == '/api2/json/nodes/pve-test/qemu/202/status/' + action
                assert action in ('start', 'shutdown')
                assert body == ('' if action == 'start' else 'timeout=60&forceStop=0')
                state['writes'].append(action)
                state['running'] = action == 'start'
                state['upid'] = f'UPID:pve-test:00000001:00000002:00000003:qm{action}:202:fixture@pve:'
                self.reply(state['upid'])

        peer = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Peer)
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.minimum_version = ssl.TLSVersion.TLSv1_2
        tls.load_cert_chain(certificate, key)
        peer.socket = tls.wrap_socket(peer.socket, server_side=True)
        threading.Thread(target=peer.serve_forever, daemon=True).start()
        port = free_port()
        server_env = {'PATH': '/usr/bin:/bin', 'VMANYWARE_PORT': str(port),
                      'VMANYWARE_DATABASE': str(directory / 'fixture.sqlite'), 'VMANYWARE_LIFECYCLE_ENABLED': 'true',
                      'VMANYWARE_PROXMOX_URL': f'https://localhost:{peer.server_port}',
                      'VMANYWARE_PROXMOX_CA': str(certificate), 'VMANYWARE_PROXMOX_TOKEN_ID': 'reader@pve!fixture',
                      'VMANYWARE_PROXMOX_TOKEN_SECRET': 'fixture-only-token'}
        try:
            server = subprocess.Popen([str(REPO / 'target/debug/vmanyware-server')], env=server_env,
                                      stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, start_new_session=True)
            children.append(server)
            lines, ready = [], queue.Queue()

            def log_reader():
                for line in server.stderr:
                    lines.append(line)
                    ready.put(line)
            threading.Thread(target=log_reader, daemon=True).start()
            assert json.loads(ready.get(timeout=10)) == {'event': 'server_listening', 'port': port}
            # Fixture setup only; subsequent actions come from the actual native React UI.
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            root = f'http://127.0.0.1:{port}'
            request = urllib.request.Request(root + '/api/v1/definitions', data=(REPO / 'examples/kernel-lab.yaml').read_bytes(), headers={'Content-Type': 'application/yaml'})
            definition = json.load(opener.open(request, timeout=5))
            request = urllib.request.Request(root + '/api/v1/environments', data=json.dumps({
                'definitionName': 'kernel-lab', 'definitionHash': definition['hash'], 'node': 'pve-test', 'vmid': 202,
            }).encode(), headers={'Content-Type': 'application/json'})
            instance = json.load(opener.open(request, timeout=5))
            state['marker'] = 'vmanyware-' + instance['id']
            display = str(1000 + secrets.randbelow(20000))
            authority = directory / 'Xauthority'
            # Generated cookie for this owned display; no personal Xauthority is read.
            def field(value):
                return struct.pack('!H', len(value)) + value
            authority.write_bytes(struct.pack('!H', 65535) + field(b'') + field(display.encode()) + field(b'MIT-MAGIC-COOKIE-1') + field(secrets.token_bytes(16)))
            authority.chmod(0o600)
            env.update({'DISPLAY': ':' + display, 'XAUTHORITY': str(authority), 'VMANYWARE_FIXTURE_PORT': str(port)})
            (directory / 'x11').mkdir(mode=0o700)
            overlay = ['bwrap', '--die-with-parent', '--ro-bind', '/', '/', '--tmpfs', '/home',
                       '--ro-bind', str(REPO), str(REPO), '--tmpfs', '/tmp',
                       '--ro-bind', str(prefix), str(prefix), '--bind', str(directory), str(directory),
                       '--bind', str(directory / 'x11'), '/tmp/.X11-unix',
                       '--overlay-src', '/usr', '--overlay-src', str(prefix / 'sysroot/usr'), '--ro-overlay', '/usr',
                       '--proc', '/proc', '--dev', '/dev']
            with (directory / 'xvfb.log').open('w+') as display_log:
                xvfb = subprocess.Popen(overlay + ['/usr/bin/Xvfb', ':' + display, '-screen', '0', '1100x900x24',
                                                  '-nolisten', 'tcp', '-auth', str(authority)], env=env,
                                        stdout=display_log, stderr=subprocess.STDOUT, start_new_session=True)
                children.append(xvfb)
                deadline = time.monotonic() + 5
                while not (directory / 'x11' / ('X' + display)).exists():
                    if xvfb.poll() is not None or time.monotonic() > deadline:
                        display_log.seek(0)
                        raise RuntimeError('Xvfb failed: ' + display_log.read())
                    time.sleep(0.02)
                time.sleep(0.2)
                result = subprocess.Popen(overlay + ['dbus-run-session', '--', str(REPO / 'target/debug/examples/lifecycle_smoke')],
                                          env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, start_new_session=True)
                children.append(result)
                _, output = result.communicate(timeout=55)
                print(output[-10000:], end='')
                if result.returncode != 0:
                    display_log.seek(0)
                    print('Owned Xvfb log:', display_log.read()[-5000:])
                    print('Owned server events:', ''.join(lines[-12:]))
                assert result.returncode == 0 and 'Native lifecycle smoke: passed' in output
            assert state['writes'] == ['start', 'shutdown'], state['writes']
            assert not state['errors'], state['errors']
            assert not state['running']
            retained = json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5))
            assert retained == instance
            os.killpg(server.pid, signal.SIGTERM)
            assert server.wait(timeout=10) == 0
            assert all('fixture-only-token' not in line and 'UPID:' not in line for line in lines)
            print('Real native React/WebKit -> Tauri IPC -> Axum -> generated HTTPS start/shutdown: passed')
        finally:
            for child in reversed(children):
                # Reap remaining owned group members even if their leader exited.
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
            peer.shutdown()
            peer.server_close()


if __name__ == '__main__':
    main()
