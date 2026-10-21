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
import sqlite3
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
    modes.add_argument('--manual-ssh', action='store_true', help='Existing-row native application-key SSH terminal against owned peer; no provider read')
    modes.add_argument('--lifecycle-recovery', action='store_true', help='Remote start/shutdown, restart app/server and recover both operations by GET only')
    modes.add_argument('--remote-lifecycle', action='store_true', help='Remote HTTPS native start/shutdown, lost replies and identical retries')
    modes.add_argument('--snapshots', action='store_true', help='Read-only remote snapshot UI instead of lifecycle mutations')
    modes.add_argument('--snapshot-create', action='store_true', help='Confirmed disk-only snapshot UI against generated infrastructure')
    modes.add_argument('--snapshot-recovery', action='store_true', help='Create once, restart app/server, then recover by GET only')
    modes.add_argument('--snapshot-restore', action='store_true', help='Destructive restore UI, generated provider and lost HTTP response retry')
    modes.add_argument('--restore-recovery', action='store_true', help='Restore once, restart app/server and recover through GET only')
    modes.add_argument('--deletion-recovery', action='store_true', help='Delete once, restart app/server and recover through GET only')
    modes.add_argument('--declared-create', action='store_true', help='Native create with declared packages/final snapshot and lost API reply')
    modes.add_argument('--declared-recreation', action='store_true', help='Native recreation with declared packages/final snapshot and lost API reply')
    modes.add_argument('--recreation', action='store_true', help='Actual native retained-source recreation against generated HTTPS infrastructure')
    modes.add_argument('--recreation-recovery', action='store_true', help='Recreate once, restart app/server and recover by GET only without provider/catalog/writes')
    args = parser.parse_args()
    args.remote_lifecycle = args.remote_lifecycle or args.lifecycle_recovery
    args.declared = args.declared_create or args.declared_recreation
    args.recreation = args.recreation or args.recreation_recovery or args.declared
    args.snapshot_create = args.snapshot_create or args.snapshot_recovery
    args.snapshot_restore = args.snapshot_restore or args.restore_recovery
    remote_snapshot = args.manual_ssh or args.remote_lifecycle or args.snapshots or args.snapshot_create or args.snapshot_restore or args.deletion_recovery or args.recreation
    prefix = args.prefix.resolve()
    assert (prefix / 'sysroot/usr/bin/Xvfb').is_file()
    children = []
    with tempfile.TemporaryDirectory(prefix='vmanyware-lifecycle-native-') as temporary:
        directory = Path(temporary)
        env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'GDK_BACKEND': 'x11',
               'LD_LIBRARY_PATH': str(prefix / 'sysroot/usr/lib'), 'LIBGL_ALWAYS_SOFTWARE': '1',
               '__EGL_VENDOR_LIBRARY_FILENAMES': '/usr/share/glvnd/egl_vendor.d/50_mesa.json',
               '__GLX_VENDOR_LIBRARY_NAME': 'mesa'}
        if args.manual_ssh:
            env['VMANYWARE_MANUAL_SSH_FIXTURE'] = '1'
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
                if args.deletion_recovery and self.path in ('/api2/json/nodes/pve-test/qemu/202/config', '/api2/json/nodes/pve-test/qemu/202/config?current=1'):
                    assert not state.get('deleted'), 'Deleted target was reread'
                    self.reply({'digest': 'b' * 40, 'tags': state['marker'], 'template': 0, 'scsi0': 'fixture:vm-202-disk-0,size=8G'})
                elif args.deletion_recovery and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot':
                    self.reply([{'name': 'current'}])
                elif args.deletion_recovery and self.path.startswith('/api2/json/access/permissions?'):
                    path = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)['path'][0]
                    assert path in ('/vms/202', '/storage/fixture')
                    self.reply({path: {'VM.Audit' if path == '/vms/202' else 'Datastore.Allocate': 0}})
                elif args.deletion_recovery and self.path in ('/api2/json/cluster/ha/resources', '/api2/json/cluster/replication'):
                    self.reply([])
                elif args.deletion_recovery and self.path.startswith('/api2/json/nodes/pve-test/storage/fixture/content?'):
                    query = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
                    assert query in ({'content': ['images'], 'vmid': ['202']}, {'content': ['images']})
                    self.reply([] if state.get('deleted') else [{'volid': 'fixture:vm-202-disk-0', 'vmid': 202, 'format': 'raw'}])
                elif args.deletion_recovery and self.path == '/api2/json/cluster/nextid?vmid=202':
                    assert state.get('deleted'), 'Absence requested before DELETE'
                    self.reply(202)
                elif args.snapshot_restore and self.path == '/api2/json/nodes/pve-test/qemu/202/snapshot':
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
                    running = state['power_reads'] < 2 if args.snapshot_restore or args.deletion_recovery else state['running']
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

            def do_DELETE(self):
                assert args.deletion_recovery
                assert self.headers.get('Authorization') == 'PVEAPIToken=reader@pve!fixture=fixture-only-token'
                assert self.path == '/api2/json/nodes/pve-test/qemu/202?purge=0&destroy-unreferenced-disks=0'
                assert not state['writes'], 'DELETE repeated'
                assert not self.rfile.read(int(self.headers.get('Content-Length', '0')))
                state['writes'].append('delete')
                state['deleted'] = True
                state['upid'] = 'UPID:pve-test:00000001:00000002:00000003:qmdestroy:202:fixture@pve:'
                self.reply(state['upid'])

            def do_POST(self):
                assert not args.deletion_recovery, 'Deletion sent provider POST'
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
            if not args.remote_lifecycle:
                server_env.pop('VMANYWARE_LIFECYCLE_ENABLED', None)
        if args.snapshot_create:
            server_env['VMANYWARE_SNAPSHOTS_ENABLED'] = 'true'
        if args.snapshot_restore:
            server_env['VMANYWARE_RESTORES_ENABLED'] = 'true'
        if args.deletion_recovery:
            server_env['VMANYWARE_DELETIONS_ENABLED'] = 'true'
        proxy = None
        try:
            if args.recreation:
                # Reuse the Android journey's generated provider protocol fixture.
                demo_key = next(line.strip()[2:] for line in (REPO / 'examples/kernel-lab.yaml').read_text().splitlines() if line.strip().startswith('- ssh-ed25519 '))
                configuration = directory / 'recreation-provider.json'
                configuration.write_text(json.dumps({'certificate': str(certificate), 'key': str(key), 'publicKey': ' '.join(demo_key.split()[:2]), 'declared': args.declared}))
                generated_provider = subprocess.Popen(['node', str(REPO / 'experiments/lifecycle-native/recreation-provider.mjs'), str(configuration)],
                    env={'PATH': '/usr/bin:/bin'}, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
                children.append(generated_provider)
                provider_lines = queue.Queue()
                def provider_reader():
                    for line in generated_provider.stdout:
                        provider_lines.put(line)
                threading.Thread(target=provider_reader, daemon=True).start()
                provider_port = json.loads(provider_lines.get(timeout=10))['port']
                catalog = directory / 'templates.yaml'
                catalog.write_text((REPO / 'examples/templates.yaml').read_text() + '    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}\n')
                server_env.update({'VMANYWARE_PROXMOX_URL': f'https://localhost:{provider_port}', 'VMANYWARE_TEMPLATES': str(catalog), 'VMANYWARE_PROVISIONING_ENABLED': 'true'})
            if args.declared:
                server_env['VMANYWARE_SNAPSHOTS_ENABLED'] = 'true'
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
            yaml = (REPO / 'examples/kernel-lab.yaml').read_text()
            if args.recreation:
                yaml = yaml.replace('kernel-lab', 'mobile-lab').replace('cpu: 4', 'cpu: 2').replace('memoryMiB: 8192', 'memoryMiB: 2048').replace('diskGiB: 40', 'diskGiB: 8').replace('packages:\n      - build-essential\n      - git', 'packages: []')
            if args.declared:
                yaml = yaml.replace('packages: []', 'packages:\n      - build-essential\n      - git=1:2.0-1').replace('snapshotAfterProvisioning: false', 'snapshotAfterProvisioning: true')
            if args.declared_create:
                yaml = json.dumps({'apiVersion': 'vmanyware.dev/v1', 'kind': 'Environment', 'metadata': {'name': 'mobile-lab'},
                    'spec': {'image': 'debian-13', 'resources': {'cpu': 2, 'memoryMiB': 2048, 'diskGiB': 8},
                    'provisioning': {'cloudInit': True, 'sshAuthorizedKeys': [demo_key], 'packages': ['build-essential', 'git=1:2.0-1']},
                    'lifecycle': {'snapshotAfterProvisioning': True}}})
            definition_name = 'mobile-lab' if args.recreation else 'kernel-lab'
            request = urllib.request.Request(root + '/api/v1/definitions', data=yaml.encode(), headers={'Content-Type': 'application/yaml'})
            definition = json.load(opener.open(request, timeout=5))
            request = urllib.request.Request(root + '/api/v1/environments', data=json.dumps({
                'definitionName': definition_name, 'definitionHash': definition['hash'], 'node': 'pve-test', 'vmid': 202,
            }).encode(), headers={'Content-Type': 'application/json'})
            instance = json.load(opener.open(request, timeout=5))
            state['marker'] = 'vmanyware-' + instance['id']
            if args.recreation:
                # Retired source fixture only; this mode does not execute deletion.
                with sqlite3.connect(directory / 'fixture.sqlite') as database:
                    database.execute("UPDATE instances SET status='deleted' WHERE id=?", (instance['id'],))
                instance = json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5))
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
                        assert args.remote_lifecycle or args.snapshot_create or args.snapshot_restore or args.deletion_recovery or args.recreation, 'Read-only proxy received POST'
                        self.forward()
                    def do_GET(self):
                        self.forward()
                    def forward(self):
                        if args.remote_lifecycle:
                            state.setdefault('lifecycle_methods', []).append((self.command, self.path))
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
                            if args.remote_lifecycle and self.command == 'GET' and self.path == '/api/v1/environments/' + instance['id'] + '/status':
                                assert reply.status == 200
                                observed = json.loads(body)
                                assert observed['instanceId'] == instance['id'] and observed['providerScopeId'] == instance['providerReference']['scopeId']
                                assert observed['observation']['node'] == 'pve-test' and observed['observation']['vmid'] == 202
                                state.setdefault('explicit_power_reads', []).append(observed['observation']['status'])
                            if self.command == 'POST' and ((args.snapshot_restore and self.path.endswith('/restore')) or (args.deletion_recovery and self.path.endswith('/delete')) or (args.recreation and self.path.endswith('/recreate')) or (args.declared_create and self.path == '/api/v1/provisioning/create') or (args.remote_lifecycle and self.path.endswith(('/start', '/shutdown')))):
                                state['restore_submissions'].append((self.path, headers.get('Idempotency-Key'), payload))
                                assert reply.status == 202
                                first_reply = sum(entry[0] == self.path for entry in state['restore_submissions']) == 1 if args.remote_lifecycle else len(state['restore_submissions']) == 1
                                if first_reply:
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
                    'token': token, 'ca': certificate.read_text(), 'remoteLifecycle': args.remote_lifecycle, 'manualSsh': args.manual_ssh, 'creation': args.snapshot_create, 'restore': args.snapshot_restore, 'deletion': args.deletion_recovery, 'recreation': args.recreation, 'declared': args.declared, 'declaredCreate': args.declared_create, 'publicKey': demo_key if args.recreation else None})

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
                    if args.recreation:
                        with sqlite3.connect(directory / 'fixture.sqlite') as database:
                            print('Owned operation states:', database.execute('SELECT kind,status,failure_code FROM operations').fetchall())
                            print('Owned provisioning stages:', database.execute('SELECT kind,status,dispatch_started FROM provisioning_stages ORDER BY position').fetchall())
                        generated_provider.stdin.write('report\n')
                        generated_provider.stdin.flush()
                        diagnostics = json.loads(provider_lines.get(timeout=5))
                        print('Owned provider diagnostics:', {name: diagnostics.get(name) for name in ('failure', 'errors')})
                assert result.returncode == 0 and 'Native lifecycle smoke: passed' in output
                if args.lifecycle_recovery or args.snapshot_recovery or args.restore_recovery or args.deletion_recovery or args.recreation_recovery:
                    history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                    operations = [entry for entry in history if (entry['kind'] in ('vm_start', 'vm_shutdown') if args.lifecycle_recovery else entry['kind'] == ('vm_provision' if args.recreation_recovery else 'vm_delete' if args.deletion_recovery else 'vm_snapshot_restore' if args.restore_recovery else 'vm_snapshot_create'))]
                    assert len(operations) == (2 if args.lifecycle_recovery else 1) and all(entry['status'] == 'succeeded' for entry in operations)
                    if args.lifecycle_recovery:
                        saved_lifecycle = {entry['id']: json.load(opener.open(root + '/api/v1/operations/' + entry['id'], timeout=5)) for entry in operations}
                        lifecycle_provider_before = json.loads(json.dumps({name: state[name] for name in ('provider_reads', 'power_reads', 'task_reads', 'writes', 'running', 'upid')}))
                    saved_operation = json.load(opener.open(root + '/api/v1/operations/' + operations[0]['id'], timeout=5))
                    provider_reads_before = state['provider_reads']
                    if args.recreation_recovery:
                        generated_provider.stdin.write('snapshot\n')
                        generated_provider.stdin.flush()
                        provider_snapshot = json.loads(provider_lines.get(timeout=10))
                        assert 'failure' not in provider_snapshot, provider_snapshot
                        source_before = json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5))
                        target_before = json.load(opener.open(root + '/api/v1/environments/' + saved_operation['instanceId'], timeout=5))
                    os.killpg(server.pid, signal.SIGTERM)
                    assert server.wait(timeout=10) == 0
                    log_thread.join(timeout=2)
                    assert not log_thread.is_alive()
                    port = free_port()
                    root = f'http://127.0.0.1:{port}'
                    server_env['VMANYWARE_PORT'] = str(port)
                    server_env['VMANYWARE_DELETIONS_ENABLED'] = 'false'
                    server_env['VMANYWARE_SNAPSHOTS_ENABLED'] = 'false'
                    server_env['VMANYWARE_RESTORES_ENABLED'] = 'false'
                    server_env['VMANYWARE_LIFECYCLE_ENABLED'] = 'false'
                    server_env['VMANYWARE_PROVISIONING_ENABLED'] = 'false'
                    for name in list(server_env):
                        if name.startswith('VMANYWARE_PROXMOX_'):
                            del server_env[name]
                    if args.recreation_recovery or args.lifecycle_recovery:
                        server_env.pop('VMANYWARE_TEMPLATES', None)
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
                    configuration['detailInstance'] = {name: instance[name] for name in ('id', 'definitionName', 'definitionHash', 'definitionRevision', 'definitionYaml', 'status')}
                    if args.lifecycle_recovery:
                        configuration.update({'lifecycleRecovery': True, 'remoteLifecycle': False, 'operationIds': {entry['kind']: entry['id'] for entry in operations}})
                    configuration.update({'recreationRecovery': args.recreation_recovery, 'deletionRecovery': args.deletion_recovery, 'recovery': not (args.lifecycle_recovery or args.restore_recovery or args.deletion_recovery or args.recreation_recovery), 'restoreRecovery': args.restore_recovery, 'operationId': operations[0]['id']})
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
                    assert state['writes'] == (['start', 'shutdown'] if args.lifecycle_recovery else [] if args.recreation_recovery else ['delete'] if args.deletion_recovery else ['restore'] if args.restore_recovery else ['snapshot']), 'Recovery resubmitted provider work'
                    after = json.load(opener.open(root + '/api/v1/operations/' + operations[0]['id'], timeout=5))
                    assert after == saved_operation, 'Recovery changed the stored operation'
                    if args.lifecycle_recovery:
                        for operation_id, before in saved_lifecycle.items():
                            assert json.load(opener.open(root + '/api/v1/operations/' + operation_id, timeout=5)) == before
                        assert {name: state[name] for name in lifecycle_provider_before} == lifecycle_provider_before, 'Lifecycle recovery contacted or changed provider'
                        assert json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5)) == instance
                        assert json.load(opener.open(root + '/api/v1/definitions/kernel-lab', timeout=5)) == definition
                    if args.recreation_recovery:
                        assert json.load(opener.open(root + '/api/v1/environments/' + instance['id'], timeout=5)) == source_before
                        assert json.load(opener.open(root + '/api/v1/environments/' + saved_operation['instanceId'], timeout=5)) == target_before

            assert state['writes'] == ([] if args.recreation else ['delete'] if args.deletion_recovery else ['restore'] if args.snapshot_restore else ['snapshot'] if args.snapshot_create else [] if args.snapshots or args.manual_ssh else ['start', 'shutdown']), state['writes']
            if args.manual_ssh:
                assert state['provider_reads'] == 0 and state['task_reads'] == 0, 'Manual SSH preparation contacted provider'
            if args.remote_lifecycle:
                submissions = state['restore_submissions']
                assert len(submissions) == 4
                assert submissions[0] == submissions[1] and submissions[2] == submissions[3], 'Lifecycle retry changed key/body/path'
                assert submissions[0][0].endswith('/start') and submissions[2][0].endswith('/shutdown')
                assert submissions[0][1] != submissions[2][1], 'Distinct actions reused one request key'
                methods = state['lifecycle_methods']
                assert len([entry for entry in methods if entry[0] == 'POST']) == 4
                assert state['explicit_power_reads'] == ['stopped', 'running', 'stopped'], 'Explicit native status reads missing or changed'
                assert len([entry for entry in methods if entry[0] == 'GET' and entry[1].startswith('/api/v1/operations/')]) >= 2
                history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                assert len(history) == 2 and {entry['kind'] for entry in history} == {'vm_start', 'vm_shutdown'}
                for entry in history:
                    saved = json.load(opener.open(root + '/api/v1/operations/' + entry['id'], timeout=5))
                    assert saved['status'] == 'succeeded' and saved['providerTaskKnown'] is True
                    evidence = saved['result']
                    assert evidence['instanceId'] == instance['id'] and evidence['providerScopeId'] == instance['providerReference']['scopeId']
                    assert evidence['taskStatus'] == 'succeeded'
                    assert evidence['observation']['node'] == 'pve-test' and evidence['observation']['vmid'] == 202
                    assert evidence['observation']['status'] == ('running' if saved['kind'] == 'vm_start' else 'stopped')
                assert state['task_reads'] >= 2
            if args.recreation:
                generated_provider.stdin.write('report\n')
                generated_provider.stdin.flush()
                generated_provider.stdin.close()
                provider_report = json.loads(provider_lines.get(timeout=10))
                assert 'failure' not in provider_report, provider_report
                if args.recreation_recovery:
                    assert provider_report == provider_snapshot, 'Saved recovery contacted or changed the provider'
                assert generated_provider.wait(timeout=5) == 0
                assert len(state['restore_submissions']) == 2 and state['restore_submissions'][0] == state['restore_submissions'][1], 'Recreation retry changed approval/key/path'
                history = json.load(opener.open(root + '/api/v1/operations', timeout=5))
                assert len(history) == 1 and history[0]['kind'] == 'vm_provision'
                operation = json.load(opener.open(root + '/api/v1/operations/' + history[0]['id'], timeout=5))
                assert operation['status'] == 'succeeded' and operation['instanceId'] != instance['id']
                assert operation['result']['guestReady'] is True and operation['result']['guest']['ipv4'] == '192.0.2.42'
                target = json.load(opener.open(root + '/api/v1/environments/' + operation['instanceId'], timeout=5))
                assert target['status'] == 'registered' and target['definitionYaml'] == instance['definitionYaml'] and target['definitionHash'] == instance['definitionHash']
                assert json.load(opener.open(root + '/api/v1/definitions/mobile-lab', timeout=5)) == definition
                assert len(json.load(opener.open(root + '/api/v1/environments', timeout=5))) == 2
                with sqlite3.connect(directory / 'fixture.sqlite') as database:
                    assert database.execute('SELECT source_instance_id FROM recreation_sources WHERE operation_id=?', (operation['id'],)).fetchone() == (None if args.declared_create else (instance['id'],))
                    if args.declared:
                        package = json.loads(database.execute("SELECT result_json FROM provisioning_stages WHERE operation_id=? AND kind='install_declared_packages'", (operation['id'],)).fetchone()[0])
                        snapshot = json.loads(database.execute("SELECT result_json FROM provisioning_stages WHERE operation_id=? AND kind='snapshot_after_provisioning'", (operation['id'],)).fetchone()[0])
                        assert package['installed'] is True and package['packageCount'] == 2 and package['instanceId'] == operation['instanceId']
                        assert package['planHash'] == operation['result']['planHash']
                        assert snapshot['snapshot']['name'] == 'ready_' + operation['id'] and snapshot['snapshot']['includesRam'] is False
                        assert snapshot['instanceId'] == operation['instanceId'] and snapshot['taskStatus'] == 'succeeded'
                    assert database.execute("SELECT count(*) FROM provisioning_stages WHERE operation_id=? AND status!='succeeded'", (operation['id'],)).fetchone() == (0,)
                assert len([request for request in provider_report['requests'] if request['method'] != 'GET']) == (8 if args.declared else 6)
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
            if args.deletion_recovery:
                expected = dict(instance, status='deleted')
                assert retained == expected, retained
                assert len(state['restore_submissions']) == 2
                assert state['restore_submissions'][0] == state['restore_submissions'][1], 'Deletion retry changed approval/key/path'
                evidence = saved_operation['result']
                assert all(evidence[field] is True for field in ('vmAbsenceVerified', 'volumesAbsenceVerified', 'definitionRetained', 'associationRetired'))
                assert evidence['instanceId'] == instance['id'] and evidence['taskStatus'] == 'succeeded'
                definition_after = json.load(opener.open(root + '/api/v1/definitions/kernel-lab', timeout=5))
                assert definition_after == definition, 'Deletion changed the retained definition'
            else:
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
            print('Real native React/WebKit -> Tauri IPC -> Axum -> generated HTTPS ' + ('declared create/packages/final snapshot/same-key retry' if args.declared_create else 'declared recreation/packages/final snapshot/same-key retry' if args.declared_recreation else 'manual SSH application-key/interactive terminal' if args.manual_ssh else 'remote lifecycle/app+server restart/GET-only recovery' if args.lifecycle_recovery else 'remote lifecycle/start-shutdown/identical retry' if args.remote_lifecycle else 'recreation/app+server restart/read-only recovery' if args.recreation_recovery else 'retained-definition recreation/same-key retry/new guest' if args.recreation else 'deletion/app+server restart/read-only recovery' if args.deletion_recovery else 'snapshot restore/app+server restart/read-only recovery' if args.restore_recovery else 'snapshot restore/same-key retry after lost HTTP reply' if args.snapshot_restore else 'snapshot create/app+server restart/read-only recovery' if args.snapshot_recovery else 'snapshot creation' if args.snapshot_create else 'snapshot listing' if args.snapshots else 'start/shutdown') + ': passed')
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
