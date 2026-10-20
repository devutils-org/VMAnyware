#!/usr/bin/env python3
"""Owned Linux Xvfb/WebKit/Tauri -> actual Axum -> generated HTTPS lifecycle smoke."""
import argparse
import hashlib
import http.client
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
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--snapshots', action='store_true', help='Read-only remote snapshot UI instead of lifecycle mutations')
    modes.add_argument('--snapshot-create', action='store_true', help='Confirmed disk-only snapshot UI against generated infrastructure')
    modes.add_argument('--snapshot-recovery', action='store_true', help='Create once, restart app/server, then recover by GET only')
    modes.add_argument('--snapshot-restore', action='store_true', help='Destructive restore UI, generated provider and lost HTTP response retry')
    modes.add_argument('--restore-recovery', action='store_true', help='Restore once, restart app/server and recover through GET only')
    args = parser.parse_args()
    args.snapshot_create = args.snapshot_create or args.snapshot_recovery
    args.snapshot_restore = args.snapshot_restore or args.restore_recovery
    remote_snapshot = args.snapshots or args.snapshot_create or args.snapshot_restore
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
        state = {'running': False, 'marker': '', 'writes': [], 'errors': [], 'snapshot_reads': 0, 'snapshot': None, 'task_reads': 0, 'provider_reads': 0, 'recovery_methods': [], 'power_reads': 0, 'saved_reads': 0, 'restore_submissions': []}

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
                state['provider_reads'] += 1
                assert self.headers.get('Authorization') == 'PVEAPIToken=reader@pve!fixture=fixture-only-token'
                if args.snapshot_restore and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot':
                    state['snapshot_reads'] += 1
                    self.reply([state['snapshot'], {'name': 'current'}])
                elif args.snapshot_restore and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot/native-before-work/config':
                    state['saved_reads'] += 1
                    self.reply(state['saved_config'])
                elif args.snapshot_restore and self.path == '/api2/json/nodes/pve-test/qemu/202/feature?feature=snapshot&snapname=native-before-work':
                    self.reply({'hasFeature': 1})
                elif args.snapshot_create and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot':
                    state['snapshot_reads'] += 1
                    self.reply(([state['snapshot']] if state['snapshot'] else []) + [{'name': 'current'}])
                elif args.snapshot_create and self.path == '/api2/json/nodes/pve-test/qemu/202/feature?feature=snapshot':
                    self.reply({'hasFeature': 1})
                elif args.snapshots and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot':
                    state['snapshot_reads'] += 1
                    if state['snapshot_reads'] == 1:
                        self.reply([{'name': 'current', 'description': 'Live VM'}])
                    elif state['snapshot_reads'] == 2:
                        self.reply([{'name': 'before-work', 'description': '<b>literal description</b>',
                                     'snaptime': 0, 'parent': 'baseline', 'snapstate': 'prepare'},
                                    {'name': 'current'}])
                    else:
                        self.send_error(403, 'fixture-only-private-provider-detail')
                elif self.path == '/api2/json/nodes/pve-test/qemu/202/config?current=1':
                    self.reply({'digest': ('c' if args.snapshot_restore and state['writes'] else 'b') * 40, 'tags': state['marker'], 'template': 0})
                elif self.path == '/api2/json/nodes/pve-test/qemu/202/pending':
                    self.reply([])
                elif self.path == '/api2/json/nodes/pve-test/qemu/202/status/current':
                    # First read-only review simulates a running target; subsequent
                    # reviews observe it stopped. No power write is supported here.
                    running = state['power_reads'] < 2 if args.snapshot_restore else state['running']
                    state['power_reads'] += 1
                    self.reply({'vmid': 202, 'status': 'running' if running else 'stopped',
                                'qmpstatus': 'running' if running else None})
                elif self.path.startswith('/api2/json/nodes/pve-test/tasks/') and self.path.endswith('/status'):
                    state['task_reads'] += 1
                    upid = urllib.parse.unquote(self.path.split('/tasks/')[1][:-len('/status')])
                    assert upid == state['upid']
                    self.reply({'node': 'pve-test', 'upid': upid, 'status': 'stopped', 'exitstatus': 'OK'})
                else:
                    raise AssertionError('Unexpected provider GET path')

            def do_POST(self):
                assert self.headers.get('Authorization') == 'PVEAPIToken=reader@pve!fixture=fixture-only-token'
                assert not args.snapshots, 'Snapshot-list scenario attempted a provider mutation'
                assert self.headers.get('Content-Type') == 'application/x-www-form-urlencoded'
                body = self.rfile.read(int(self.headers.get('Content-Length', '0'))).decode()
                if args.snapshot_restore:
                    assert self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot/native-before-work/rollback'
                    assert not state['writes'], 'Rollback POST was repeated'
                    assert body == 'start=0', 'Rollback tried to change power options'
                    state['writes'].append('restore')
                    state['upid'] = 'UPID:pve-test:00000001:00000002:00000003:qmrollback:202:fixture@pve:'
                    self.reply(state['upid'])
                    return
                if args.snapshot_create:
                    assert self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot'
                    assert not state['writes'], 'Snapshot POST was repeated'
                    fields = urllib.parse.parse_qs(body, keep_blank_values=True)
                    assert fields == {'snapname': ['native-before-work'], 'description': ['Disk & café + work\nOnly'], 'vmstate': ['0']}, fields
                    state['writes'].append('snapshot')
                    state['snapshot'] = {'name': fields['snapname'][0], 'description': fields['description'][0], 'snaptime': 1700000000, 'vmstate': 0}
                    state['upid'] = 'UPID:pve-test:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:'
                    self.reply(state['upid'])
                    return
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
        token = secrets.token_urlsafe(32)
        if remote_snapshot:
            server_env['VMANYWARE_ACCESS_TOKEN'] = token
            server_env.pop('VMANYWARE_LIFECYCLE_ENABLED', None)
        if args.snapshot_create:
            server_env['VMANYWARE_SNAPSHOTS_ENABLED'] = 'true'
        if args.snapshot_restore:
            server_env['VMANYWARE_RESTORES_ENABLED'] = 'true'
        proxy = None
        try:
            server = subprocess.Popen([str(REPO / 'target/debug/vmanyware-server')], env=server_env,
                                      stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, start_new_session=True)
            children.append(server)
            lines, ready = [], queue.Queue()

            def log_reader():
                for line in server.stderr:
                    lines.append(line)
                    ready.put(line)
            log_thread = threading.Thread(target=log_reader, daemon=True)
            log_thread.start()
            assert json.loads(ready.get(timeout=10)) == {'event': 'server_listening', 'port': port}
            # Fixture setup only; subsequent actions come from the actual native React UI.
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            if remote_snapshot:
                opener.addheaders = [('Authorization', 'Bearer ' + token)]
            root = f'http://127.0.0.1:{port}'
            request = urllib.request.Request(root + '/api/v1/definitions', data=(REPO / 'examples/kernel-lab.yaml').read_bytes(), headers={'Content-Type': 'application/yaml'})
            definition = json.load(opener.open(request, timeout=5))
            request = urllib.request.Request(root + '/api/v1/environments', data=json.dumps({
                'definitionName': 'kernel-lab', 'definitionHash': definition['hash'], 'node': 'pve-test', 'vmid': 202,
            }).encode(), headers={'Content-Type': 'application/json'})
            instance = json.load(opener.open(request, timeout=5))
            state['marker'] = 'vmanyware-' + instance['id']
            if args.snapshot_restore:
                state['snapshot'] = {'name': 'native-before-work', 'description': 'Disk & café + work\nOnly', 'snaptime': 1700000000, 'vmstate': 0}
                state['saved_config'] = {'tags': state['marker'], 'template': 0, 'scsi0': 'fixture:vm-202-disk-0,size=8G',
                                         'snaptime': 1700000000, 'description': state['snapshot']['description']}
                canonical = json.dumps(state['saved_config'], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
                state['snapshot_hash'] = hashlib.sha256(b'vmanyware-restore-snapshot-config-v1\0native-before-work\0' + canonical).hexdigest()
            if remote_snapshot:
                class Proxy(http.server.BaseHTTPRequestHandler):
                    def log_message(self, *_):
                        pass
                    def do_POST(self):
                        assert args.snapshot_create or args.snapshot_restore, 'Read-only proxy received POST'
                        self.forward()
                    def do_GET(self):
                        self.forward()
                    def forward(self):
                        if state.get('recovery_phase'):
                            state['recovery_methods'].append(self.command)
                        upstream = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
                        try:
                            headers = {'Authorization': self.headers.get('Authorization', '')}
                            payload = None
                            if self.command == 'POST':
                                payload = self.rfile.read(int(self.headers.get('Content-Length', '0')))
                                headers['Content-Type'] = self.headers.get('Content-Type', '')
                                if self.headers.get('Idempotency-Key'):
                                    headers['Idempotency-Key'] = self.headers['Idempotency-Key']
                            upstream.request(self.command, self.path, body=payload, headers=headers)
                            reply = upstream.getresponse()
                            body = reply.read()
                            if args.snapshot_restore and self.command == 'POST' and self.path.endswith('/restore'):
                                state['restore_submissions'].append((self.path, headers.get('Idempotency-Key'), payload))
                                assert reply.status == 202
                                if len(state['restore_submissions']) == 1:
                                    # The real API already stored this approval. Lose
                                    # only its response; native must retain key/body.
                                    self.close_connection = True
                                    return
                            self.send_response(reply.status)
                            self.send_header('Content-Length', str(len(body)))
                            self.send_header('Content-Type', 'application/json')
                            marker = reply.getheader('X-VMAnyware-Authenticated')
                            if marker:
                                self.send_header('X-VMAnyware-Authenticated', marker)
                            self.end_headers()
                            self.wfile.write(body)
                        finally:
                            upstream.close()
                proxy = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Proxy)
                proxy.socket = tls.wrap_socket(proxy.socket, server_side=True)
                threading.Thread(target=proxy.serve_forever, daemon=True).start()
                env['VMANYWARE_SNAPSHOT_FIXTURE'] = json.dumps({'endpoint': f'https://localhost:{proxy.server_port}',
                    'token': token, 'ca': certificate.read_text(), 'creation': args.snapshot_create, 'restore': args.snapshot_restore})

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
                if args.snapshot_recovery or args.restore_recovery:
                    history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                    operations = [entry for entry in history if entry['kind'] == ('vm_snapshot_restore' if args.restore_recovery else 'vm_snapshot_create')]
                    assert len(operations) == 1 and operations[0]['status'] == 'succeeded'
                    saved_operation = json.load(opener.open(root + '/api/v1/operations/' + operations[0]['id'], timeout=5))
                    provider_reads_before = state['provider_reads']
                    os.killpg(server.pid, signal.SIGTERM)
                    assert server.wait(timeout=10) == 0
                    log_thread.join(timeout=2)
                    assert not log_thread.is_alive()
                    port = free_port()
                    root = f'http://127.0.0.1:{port}'
                    server_env['VMANYWARE_PORT'] = str(port)
                    server_env['VMANYWARE_SNAPSHOTS_ENABLED'] = 'false'
                    server_env['VMANYWARE_RESTORES_ENABLED'] = 'false'
                    server_env['VMANYWARE_LIFECYCLE_ENABLED'] = 'false'
                    server_env['VMANYWARE_PROVISIONING_ENABLED'] = 'false'
                    for name in list(server_env):
                        if name.startswith('VMANYWARE_PROXMOX_'):
                            del server_env[name]
                    ready = queue.Queue()
                    server = subprocess.Popen([str(REPO / 'target/debug/vmanyware-server')], env=server_env,
                        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, start_new_session=True)
                    children.append(server)
                    log_thread = threading.Thread(target=log_reader, daemon=True)
                    log_thread.start()
                    assert json.loads(ready.get(timeout=10)) == {'event': 'server_listening', 'port': port}
                    state['recovery_phase'] = True
                    env['VMANYWARE_FIXTURE_PORT'] = str(port)
                    configuration = json.loads(env['VMANYWARE_SNAPSHOT_FIXTURE'])
                    configuration.update({'recovery': not args.restore_recovery, 'restoreRecovery': args.restore_recovery, 'operationId': operations[0]['id']})
                    env['VMANYWARE_SNAPSHOT_FIXTURE'] = json.dumps(configuration)
                    restarted = subprocess.Popen(overlay + ['dbus-run-session', '--', str(REPO / 'target/debug/examples/lifecycle_smoke')],
                        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, start_new_session=True)
                    children.append(restarted)
                    assert restarted.pid != result.pid
                    _, recovery_output = restarted.communicate(timeout=55)
                    print(recovery_output[-10000:], end='')
                    output += recovery_output
                    assert restarted.returncode == 0 and 'Native lifecycle smoke: passed' in recovery_output
                    assert state['recovery_methods'] and set(state['recovery_methods']) == {'GET'}, state['recovery_methods']
                    assert state['provider_reads'] == provider_reads_before, 'Recovery contacted the provider'
                    assert state['writes'] == (['restore'] if args.restore_recovery else ['snapshot']), 'Recovery resubmitted provider work'
                    after = json.load(opener.open(root + '/api/v1/operations/' + operations[0]['id'], timeout=5))
                    assert after == saved_operation, 'Recovery changed the stored operation'

            assert state['writes'] == (['restore'] if args.snapshot_restore else ['snapshot'] if args.snapshot_create else [] if args.snapshots else ['start', 'shutdown']), state['writes']
            if args.snapshot_restore:
                assert state['snapshot_reads'] == 12, state['snapshot_reads']
                assert state['saved_reads'] == 10, state['saved_reads']
                assert state['power_reads'] == 10, state['power_reads']
                assert state['task_reads'] >= 1
                assert len(state['restore_submissions']) == 2
                assert state['restore_submissions'][0] == state['restore_submissions'][1], 'HTTP retry changed approval/key/path'
                history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                assert len(history) == 1 and history[0]['kind'] == 'vm_snapshot_restore'
                operation = json.load(opener.open(root + '/api/v1/operations/' + history[0]['id'], timeout=5))
                assert operation['status'] == 'succeeded' and operation['providerTaskKnown'] is True
                evidence = operation['result']
                assert evidence['taskStatus'] == 'succeeded'
                assert evidence['snapshotName'] == 'native-before-work'
                assert evidence['snapshotConfigHash'] == state['snapshot_hash']
                assert evidence['instanceId'] == instance['id'] and evidence['providerScopeId'] == instance['providerReference']['scopeId']
                assert evidence['node'] == 'pve-test' and evidence['vmid'] == 202
                assert evidence['observation'] == {'node': 'pve-test', 'vmid': 202, 'status': 'stopped', 'qmpStatus': None}
            if args.snapshot_create:
                assert state['snapshot_reads'] == 5, state['snapshot_reads']
                assert state['task_reads'] >= 1, 'No independent task poll'
            if args.snapshots:
                assert state['snapshot_reads'] == 3, state['snapshot_reads']
            assert not state['errors'], state['errors']
            assert not state['running']
            retained = json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5))
            assert retained == instance
            if args.snapshot_create:
                history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                operations = [operation for operation in history if operation['kind'] == 'vm_snapshot_create']
                assert len(operations) == 1 and operations[0]['status'] == 'succeeded'
                operation = json.load(opener.open(root + '/api/v1/operations/' + operations[0]['id'], timeout=5))
                assert operation['providerTaskKnown'] is True
                assert operation['result']['taskStatus'] == 'succeeded'
                assert operation['result']['snapshot']['name'] == 'native-before-work'
                assert operation['result']['snapshot']['includesRam'] is False
                assert operation['result']['snapshot']['providerState'] is None

            os.killpg(server.pid, signal.SIGTERM)
            assert server.wait(timeout=10) == 0
            assert all('fixture-only-token' not in line and 'UPID:' not in line for line in lines)
            if remote_snapshot:
                assert token not in output and all(token not in line for line in lines)
            print('Real native React/WebKit -> Tauri IPC -> Axum -> generated HTTPS ' + ('snapshot restore/app+server restart/read-only recovery' if args.restore_recovery else 'snapshot restore/same-key retry after lost HTTP reply' if args.snapshot_restore else 'snapshot create/app+server restart/read-only recovery' if args.snapshot_recovery else 'snapshot creation' if args.snapshot_create else 'snapshot listing' if args.snapshots else 'start/shutdown') + ': passed')
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
            if proxy:
                proxy.shutdown()
                proxy.server_close()
            peer.shutdown()
            peer.server_close()


if __name__ == '__main__':
    main()
