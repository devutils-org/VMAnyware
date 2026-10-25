"""Run the real TUI in a PTY against a disposable localhost HTTPS fixture.

Requires Unix, Python 3 and openssl. No Proxmox, credentials or user config.
"""
import argparse
import codecs
import errno
import fcntl
import hashlib
import http.server
import json
import os
from pathlib import Path
import pty
import re
import secrets
import select
import signal
import ssl
import struct
import subprocess
import tempfile
import termios
import threading
import time
import unicodedata


class Screen:
    """Decode the cursor moves, erases and SGR used by the Crossterm backend.

    Assertions inspect rendered cells, not substrings in a differential ANSI
    stream (an unchanged letter may never be transmitted a second time).
    """
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.cells = [[' '] * width for _ in range(height)]
        self.x = self.y = 0
        self.pending = ''
        self.decoder = codecs.getincrementaldecoder('utf-8')('replace')

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            char = self.pending[0]
            if char == '\x1b':
                if len(self.pending) < 2:
                    return
                if self.pending[1] == '[':
                    match = re.match(r'\x1b\[([0-?]*)([ -/]*)([@-~])', self.pending)
                    if not match:
                        return
                    params, _, command = match.groups()
                    self.pending = self.pending[match.end():]
                    if params.startswith('?'):
                        continue
                    values = [int(v) if v else 0 for v in params.split(';')]
                    count = values[0] or 1
                    if command in ('H', 'f'):
                        self.y = count - 1
                        self.x = (values[1] or 1) - 1 if len(values) > 1 else 0
                    elif command == 'G':
                        self.x = count - 1
                    elif command == 'd':
                        self.y = count - 1
                    elif command in 'ABCD':
                        if command == 'A': self.y = max(0, self.y - count)
                        if command == 'B': self.y += count
                        if command == 'C': self.x += count
                        if command == 'D': self.x = max(0, self.x - count)
                    elif command == 'J' and values[0] in (2, 3):
                        self.cells = [[' '] * self.width for _ in range(self.height)]
                    elif command == 'K' and self.y < self.height:
                        start = 0 if values[0] in (1, 2) else self.x
                        end = self.width if values[0] in (0, 2) else self.x + 1
                        self.cells[self.y][start:end] = [' '] * (end - start)
                    continue
                self.pending = self.pending[2:]
                continue
            self.pending = self.pending[1:]
            if char == '\r': self.x = 0
            elif char == '\n': self.y += 1
            elif not char.isprintable() or unicodedata.combining(char): continue
            else:
                width = 2 if unicodedata.east_asian_width(char) in ('W', 'F') else 1
                if self.y < self.height and self.x < self.width:
                    self.cells[self.y][self.x] = char
                    if width == 2 and self.x + 1 < self.width:
                        self.cells[self.y][self.x + 1] = ''
                self.x += width

    def text(self):
        return '\n'.join(''.join(row) for row in self.cells)


def check(binary):
    token = secrets.token_urlsafe(32)
    instance_id, operation_id = 'e' * 32, 'd' * 32
    yaml = 'apiVersion: vmanyware.dev/v1\nkind: Environment\n'
    definition_hash = hashlib.sha256(yaml.encode()).hexdigest()
    environment = {
        'id': instance_id, 'definitionName': 'terminal-lab',
        'definitionHash': definition_hash, 'definitionRevision': 1,
        'providerReference': {'provider': 'proxmox', 'scopeId': 'c' * 64,
                              'node': 'fixture-node', 'vmid': 202},
        'status': 'registered', 'lastGuest': None,
        'createdAt': '2026-10-25T12:00:00Z',
    }
    operation = {
        'id': operation_id, 'kind': 'vm_provision', 'instanceId': instance_id,
        'status': 'running', 'failure': None,
        'createdAt': '2026-10-25T12:00:00Z', 'updatedAt': '2026-10-25T12:00:01Z',
    }
    requests = []
    hold_power = threading.Event()
    power_started = threading.Event()

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            assert self.headers.get('Authorization') == f'Bearer {token}'
            requests.append(self.path)
            if self.path == '/api/v1/health':
                body = {'status': 'ok'}
            elif self.path == '/api/v1/environments':
                body = [environment]
            elif self.path == f'/api/v1/environments/{instance_id}':
                body = dict(environment, definitionYaml=yaml)
            elif self.path == f'/api/v1/environments/{instance_id}/status':
                if hold_power.is_set():
                    power_started.set()
                    # Keep this request in flight while the user quits.
                    time.sleep(3)
                body = {'instanceId': instance_id, 'providerScopeId': 'c' * 64,
                        'observation': {'node': 'fixture-node', 'vmid': 202,
                                        'status': 'running', 'qmpStatus': 'running'}}
            elif self.path == '/api/v1/operations':
                body = [operation]
            elif self.path == f'/api/v1/operations/{operation_id}/provisioning':
                kinds = ['full_clone', 'configure_resources', 'configure_cloud_init',
                         'start_vm', 'wait_for_guest_readiness']
                body = {'operationId': operation_id, 'instanceId': instance_id,
                        'providerScopeId': 'c' * 64, 'definitionHash': definition_hash,
                        'planHash': 'f' * 64, 'node': 'fixture-node', 'vmid': 202,
                        'status': operation['status'], 'failure': None,
                        'stages': [{'position': i, 'kind': kind,
                                    'status': 'succeeded' if operation['status'] == 'succeeded' or i == 0 else 'running' if i == 1 else 'queued',
                                    'submissionStarted': i <= 1 or operation['status'] == 'succeeded'}
                                   for i, kind in enumerate(kinds)]}
            else:
                raise AssertionError(f'Unexpected read path: {self.path}')
            payload = json.dumps(body).encode()
            self.send_response(200)
            self.send_header('X-VMAnyware-Authenticated', 'bearer')
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            try:
                self.wfile.write(payload)
            except (BrokenPipeError, ConnectionResetError, ssl.SSLError):
                pass

    with tempfile.TemporaryDirectory(prefix='vmanyware-tui-') as tmp:
        ca, key = Path(tmp) / 'public-ca.pem', Path(tmp) / 'fixture.key'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
                        '-keyout', str(key), '-out', str(ca), '-days', '1',
                        '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost',
                        '-addext', 'basicConstraints=critical,CA:FALSE'],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(ca, key)
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        master, slave = pty.openpty()
        original = termios.tcgetattr(slave)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 35, 120, 0, 0))
        env = dict(os.environ, TERM='xterm-256color', HOME=tmp,
                   VMANYWARE_ACCESS_TOKEN='', VMANYWARE_ENDPOINT='', VMANYWARE_CA='')
        process = subprocess.Popen([str(binary), 'tui', '--endpoint',
                                    f'https://localhost:{server.server_port}', '--ca', str(ca)],
                                   stdin=slave, stdout=slave, stderr=slave, env=env,
                                   start_new_session=True)
        output = bytearray()
        screen = Screen(120, 35)

        def drain(timeout=0.1):
            if select.select([master], [], [], timeout)[0]:
                try:
                    data = os.read(master, 65536)
                    output.extend(data)
                    screen.feed(data)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise

        def wait_for(predicate, label, timeout=10):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                drain()
                if predicate():
                    return
                if process.poll() is not None:
                    raise AssertionError(f'TUI exited while waiting for {label}')
            safe_output = screen.text().replace(token, '[REDACTED]')
            raise AssertionError(f'Timed out waiting for {label}; paths={requests}; screen=\n{safe_output}')

        def shown(text):
            return text in screen.text()

        def send(keys):
            os.write(master, keys)

        def click(label):
            wait_for(lambda: shown(label), f'click target {label}')
            for row, cells in enumerate(screen.cells):
                column = ''.join(cells).find(label)
                if column >= 0:
                    send(f'\x1b[<0;{column + 1};{row + 1}M\x1b[<0;{column + 1};{row + 1}m'.encode())
                    return
            raise AssertionError(f'No visible click target: {label}')

        try:
            wait_for(lambda: shown('Access token'), 'connection screen')
            click('Access token')
            send(b'\x1b[200~' + token.encode() + b'\x1b[201~')
            wait_for(lambda: shown('••••••••'), 'masked pasted token')
            assert token.encode() not in output, 'Token was rendered'
            click('[Connect]')
            wait_for(lambda: shown('Observed power'), 'machine overview and live power')
            click('[Definition]')
            wait_for(lambda: shown('apiVersion: vmanyware.dev/v1'), 'saved YAML')
            click('[Activity]')
            wait_for(lambda: f'/api/v1/operations/{operation_id}/provisioning' in requests,
                     'recorded provisioning GET')
            wait_for(lambda: shown('configure resources'), 'provisioning stages')
            click('vm provision')
            operation['status'] = 'succeeded'
            wait_for(lambda: shown('Provisioning: succeeded'), 'automatic operation refresh')
            count = requests.count(f'/api/v1/operations/{operation_id}/provisioning')
            click('[Refresh]')
            wait_for(lambda: requests.count(f'/api/v1/operations/{operation_id}/provisioning') > count,
                     'manual operation refresh')
            click('[?]')
            wait_for(lambda: shown('Keyboard help'), 'keyboard help')
            click('[Close]')
            wait_for(lambda: not shown('Keyboard help'), 'closed help')
            click('[Machines]')
            wait_for(lambda: shown('Machine details'), 'machines navigation')
            click('[Search]')
            send(b'terminal\r')
            wait_for(lambda: shown('Filter:'), 'search')
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 55, 0, 0))
            screen = Screen(55, 24)
            process.send_signal(signal.SIGWINCH)
            send(b'1\r')
            wait_for(lambda: shown('Overview') and shown('Observed power'), 'narrow focused detail after resize')
            hold_power.set()
            send(b'r')
            wait_for(power_started.is_set, 'in-flight power read')
            start = time.monotonic()
            click('[Quit]')
            while process.poll() is None and time.monotonic() - start < 2:
                drain()
            assert process.poll() == 0, 'Quit blocked behind a network read'
            assert termios.tcgetattr(slave) == original, 'Terminal mode was not restored'
            drain()
            assert b'\x1b[?1006h' in output and b'\x1b[?1006l' in output, 'Mouse capture was not enabled and restored'
            assert token.encode() not in output, 'Token leaked into terminal output'
            assert operation['status'] == 'succeeded', 'Client exit changed server work'
            print('TUI PTY check passed: mouse connection/tabs/activity/refresh/search/help/quit, HTTPS/auth, machines/power, YAML, stages, resize, responsive quit and terminal/mouse restoration.')
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)
            server.shutdown()
            server.server_close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/debug/vmanyware'))
    args = parser.parse_args()
    check(args.binary.resolve(strict=True))
