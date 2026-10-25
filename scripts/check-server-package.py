#!/usr/bin/env python3
"""Check an owned archive/staged process without installing or starting host units."""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import secrets
import shlex
import shutil
import signal
import socket
import sqlite3
import stat
import subprocess
import tarfile
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('archive', type=Path)
mode = parser.add_mutually_exclusive_group()
mode.add_argument('--previous-schema', action='store_true', help='Start from a populated schema-14 SQL fixture; no older-binary claim')
mode.add_argument('--future-schema-only', action='store_true', help='Only check rejection/preservation of a schema-16 fixture after package/staging checks')
args = parser.parse_args()
archive = args.archive.resolve(strict=True)
expected = archive.with_suffix(archive.suffix + '.sha256').read_text().split()[0]
assert hashlib.sha256(archive.read_bytes()).hexdigest() == expected, 'Archive checksum mismatch'
env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8'}
with tempfile.TemporaryDirectory(prefix='vmanyware-package-check-') as temporary:
    directory = Path(temporary)
    with tarfile.open(archive) as source:
        assert all(member.isfile() or member.isdir() for member in source.getmembers())
        source.extractall(directory / 'unpacked', filter='data')
    packages = list((directory / 'unpacked').iterdir())
    assert len(packages) == 1
    package = packages[0]
    for line in (package / 'SHA256SUMS').read_text().splitlines():
        digest, name = line.split('  ', 1)
        path = package / name
        assert path.resolve().is_relative_to(package.resolve())
        assert hashlib.sha256(path.read_bytes()).hexdigest() == digest, name
    stage = directory / 'root'
    stage.mkdir()
    subprocess.run([str(package / 'install.sh'), '--root', str(stage)], env=env, check=True, stdout=subprocess.DEVNULL)
    binary = stage / 'usr/local/lib/vmanyware/vmanyware-server'
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == hashlib.sha256((package / 'bin/vmanyware-server').read_bytes()).hexdigest()
    config = stage / 'etc/vmanyware/server.env'
    config.write_text('owned configuration sentinel\n')
    config.chmod(0o600)
    state = stage / 'var/lib/vmanyware'
    state.mkdir(parents=True)
    (state / 'owned-state').write_text('retained state\n')
    subprocess.run([str(package / 'install.sh'), '--root', str(stage)], env=env, check=True, stdout=subprocess.DEVNULL)
    assert config.read_text() == 'owned configuration sentinel\n'
    assert (state / 'owned-state').read_text() == 'retained state\n'
    assert stat.S_IMODE(config.stat().st_mode) == 0o600
    assert stat.S_IMODE((stage / 'etc/vmanyware').stat().st_mode) == 0o755
    assert stat.S_IMODE((stage / 'etc/vmanyware/server.env.example').stat().st_mode) == 0o600
    unit = (stage / 'etc/systemd/system/vmanyware.service').read_text()
    # Parse/verify the unit with only ExecStart adjusted to our staged binary.
    staged_unit = directory / 'vmanyware-package-check.service'
    staged_unit.write_text(unit.replace('ExecStart=/usr/local/lib/vmanyware/vmanyware-server', f'ExecStart={binary}'))
    subprocess.run(['systemd-analyze', 'verify', '--man=no', str(staged_unit)], env=env, check=True, capture_output=True, text=True, timeout=15)
    prestart = next(line.split('=', 1)[1] for line in unit.splitlines() if line.startswith('ExecStartPre='))
    command = shlex.split(prestart.replace('$$', '$'))
    assert subprocess.run(command, env=env, capture_output=True).returncode != 0
    token = secrets.token_urlsafe(48)
    assert subprocess.run(command, env=dict(env, VMANYWARE_ACCESS_TOKEN=token), capture_output=True).returncode == 0
    runtime = directory / 'runtime'
    runtime.mkdir(mode=0o700)
    if args.previous_schema:
        database = runtime / 'server.sqlite'
        legacy_yaml = (ROOT / 'examples/kernel-lab.yaml').read_text().replace('name: kernel-lab', 'name: legacy-lab')
        legacy_hash = hashlib.sha256(legacy_yaml.encode()).hexdigest()
        with sqlite3.connect(database) as connection:
            for migration in sorted((ROOT / 'migrations').glob('*.sql')):
                if int(migration.name[:4]) <= 14:
                    connection.executescript(migration.read_text())
            connection.execute('PRAGMA user_version=14')
            connection.execute('INSERT INTO definitions(name,definition_yaml,definition_hash,revision) VALUES (?,?,?,?)',
                               ('legacy-lab', legacy_yaml, legacy_hash, 3))
            connection.execute("INSERT INTO operations(id,request_key,kind,status,failure_code) VALUES (?,?, 'inventory_refresh','failed','provider_not_configured')",
                               ('e' * 32, 'legacy-read-key'))
            connection.commit()
            legacy_tables = [row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
            legacy_rows = {table: connection.execute('SELECT * FROM "' + table + '"').fetchall() for table in legacy_tables}
            assert connection.execute('PRAGMA foreign_key_check').fetchall() == []
        previous_backup = directory / 'schema-14.sqlite'
        shutil.copy2(database, previous_backup)
        previous_digest = hashlib.sha256(previous_backup.read_bytes()).hexdigest()
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    server_env = dict(env)
    for line in (package / 'server.env.example').read_text().splitlines():
        if line and not line.startswith('#'):
            name, value = line.split('=', 1)
            server_env[name] = value
    server_env.update(VMANYWARE_DATABASE=str(runtime / 'server.sqlite'), VMANYWARE_PORT=str(port), VMANYWARE_ACCESS_TOKEN=token)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    base = f'http://127.0.0.1:{port}'
    def request(path, data=None, authenticated=True, key=None):
        headers = {'Authorization': 'Bearer ' + token} if authenticated else {}
        if key is not None:
            headers['Idempotency-Key'] = key
        if data is not None:
            headers['Content-Type'] = 'application/yaml'
        return opener.open(urllib.request.Request(base + path, data=data, headers=headers), timeout=2)
    if args.future_schema_only:
        future = runtime / 'future.sqlite'
        with sqlite3.connect(future) as connection:
            connection.executescript("CREATE TABLE future_state(id INTEGER PRIMARY KEY, label TEXT NOT NULL, payload BLOB NOT NULL); PRAGMA user_version=17; PRAGMA application_id=424242;")
            connection.execute('INSERT INTO future_state VALUES (?,?,?)', (1, 'owned-future-fixture', bytes(range(256))))
            connection.commit()
        before = future.read_bytes()
        future_env = dict(server_env, VMANYWARE_DATABASE=str(future))
        for _ in range(2):
            rejected = subprocess.run([str(binary)], env=future_env, capture_output=True, text=True, timeout=10)
            assert rejected.returncode == 1, 'Future-schema process did not fail closed'
            events = [json.loads(line) for line in rejected.stderr.splitlines()]
            assert events == [{'event': 'server_failed', 'reason': 'database_unavailable'}]
            assert token not in rejected.stderr and rejected.stdout == ''
            assert future.read_bytes() == before, 'Future database bytes changed on refusal'
            with sqlite3.connect(future.as_uri() + '?mode=ro', uri=True) as connection:
                assert connection.execute('PRAGMA user_version').fetchone() == (17,)
                assert connection.execute('PRAGMA application_id').fetchone() == (424242,)
                assert connection.execute('PRAGMA integrity_check').fetchall() == [('ok',)]
                assert connection.execute('SELECT * FROM future_state').fetchall() == [(1, 'owned-future-fixture', bytes(range(256)))]
                assert connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall() == [('future_state',)]
            assert not future.with_name(future.name + '-journal').exists()
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', port))
        print('Future-schema package check passed: two schema-16 startup refusals, exact database bytes/version/rows/schema unchanged, safe error only, released lock and unused listener. Package/staging checks pass; normal process/backup cases not selected. No host service or provider accessed.')
        raise SystemExit(0)
    backup = directory / 'stopped-backup'
    for iteration in range(3):
        log_path = runtime / f'run-{iteration}.log'
        with log_path.open('w+') as log:
            process = subprocess.Popen([str(binary)], env=server_env, stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
            try:
                deadline = time.monotonic() + 10
                while True:
                    assert process.poll() is None, 'Packaged server exited before health'
                    try:
                        with request('/api/v1/health') as response:
                            assert response.status == 200 and response.headers['X-VMAnyware-Authenticated'] == 'bearer'
                        break
                    except (urllib.error.URLError, TimeoutError, http.client.HTTPException):
                        assert time.monotonic() < deadline, 'Packaged server health deadline'
                        time.sleep(0.05)
                try:
                    request('/api/v1/health', authenticated=False)
                    raise AssertionError('Packaged service accepted unauthenticated access')
                except urllib.error.HTTPError as error:
                    assert error.code == 401
                if args.previous_schema:
                    with request('/api/v1/definitions/legacy-lab') as response:
                        legacy = json.load(response)
                        assert (legacy['name'], legacy['yaml'], legacy['hash'], legacy['revision']) == ('legacy-lab', legacy_yaml, legacy_hash, 3)
                    with request('/api/v1/operations/' + 'e' * 32) as response:
                        legacy_operation = json.load(response)
                        assert legacy_operation['status'] == 'failed' and legacy_operation['failure'] == 'provider_not_configured'
                    with request('/api/v1/provider/inventory/refresh', b'', key='legacy-read-key') as response:
                        assert json.load(response) == legacy_operation
                if iteration == 0:
                    # Ensure this exact archive includes the new scoped recovery handlers.
                    for suffix in ('', '/' + 'e' * 32):
                        try:
                            request('/api/v1/environments/' + 'f' * 32 + '/lifecycle-operations' + suffix)
                            raise AssertionError('Missing lifecycle scope unexpectedly resolved')
                        except urllib.error.HTTPError as error:
                            with error:
                                assert error.code == 404
                                assert error.headers['X-VMAnyware-Authenticated'] == 'bearer'
                                assert json.load(error)['error']['code'] == 'lifecycle_operation_not_found'
                if iteration == 0:
                    with request('/api/v1/definitions', (ROOT / 'examples/kernel-lab.yaml').read_bytes()) as response:
                        saved = json.load(response)
                with request('/api/v1/definitions/kernel-lab') as response:
                    assert json.load(response) == saved
                if iteration == 0:
                    # A real read-only job yields durable failure without any configured provider.
                    with request('/api/v1/provider/inventory/refresh', b'', key='package-backup-read') as response:
                        operation = json.load(response)
                    deadline = time.monotonic() + 5
                    while True:
                        with request('/api/v1/operations/' + operation['id']) as response:
                            operation = json.load(response)
                        if operation['status'] == 'failed':
                            break
                        assert time.monotonic() < deadline, 'Read-only fixture operation deadline'
                        time.sleep(0.05)
                    assert operation['failure'] == 'provider_not_configured'
                    with request('/api/v1/operations') as response:
                        history = json.load(response)
                else:
                    with request('/api/v1/operations/' + operation['id']) as response:
                        assert json.load(response) == operation
                    with request('/api/v1/operations') as response:
                        assert json.load(response) == history
                if iteration == 1:
                    late_yaml = (ROOT / 'examples/kernel-lab.yaml').read_bytes().replace(b'name: kernel-lab', b'name: post-backup')
                    with request('/api/v1/definitions', late_yaml) as response:
                        assert json.load(response)['name'] == 'post-backup'
                if iteration == 2:
                    try:
                        request('/api/v1/definitions/post-backup')
                        raise AssertionError('Restore retained a post-backup definition')
                    except urllib.error.HTTPError as error:
                        assert error.code == 404
                    with request('/api/v1/provider/inventory/refresh', b'', key='package-backup-read') as response:
                        assert json.load(response) == operation
                    with request('/api/v1/operations') as response:
                        assert json.load(response) == history
                if iteration == 1:
                    # Exercise admission in the exact staged release process, not a router double.
                    limited = False
                    for _ in range(180):
                        try:
                            with request('/api/v1/health', authenticated=False):
                                raise AssertionError('Unauthenticated burst reached a handler')
                        except urllib.error.HTTPError as error:
                            with error:
                                if error.code == 429:
                                    assert error.headers['Retry-After'] == '1'
                                    assert json.load(error)['error']['code'] == 'request_limit_exceeded'
                                    limited = True
                                    break
                                assert error.code == 401
                    assert limited, 'Packaged server did not limit an authentication burst'
                    time.sleep(1.1)
                    with request('/api/v1/definitions/kernel-lab') as response:
                        assert json.load(response) == saved
                process.send_signal(signal.SIGTERM)
                assert process.wait(timeout=10) == 0
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
            log.seek(0)
            assert token not in log.read()
        if args.previous_schema:
            with sqlite3.connect(runtime / 'server.sqlite') as connection:
                assert connection.execute('PRAGMA user_version').fetchone() == (16,)
                assert connection.execute('PRAGMA foreign_key_check').fetchall() == []
                assert connection.execute('PRAGMA integrity_check').fetchall() == [('ok',)]
                for table in legacy_tables:
                    rows = connection.execute('SELECT * FROM "' + table + '"').fetchall()
                    assert all(row in rows for row in legacy_rows[table]), 'Legacy row changed in ' + table
                assert connection.execute('SELECT * FROM recreation_sources').fetchall() == []
                triggers = {row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='trigger'")}
                assert {'validate_recreation_source', 'retain_recreation_source', 'retain_recreation_source_history'} <= triggers
                try:
                    connection.execute('INSERT INTO recreation_sources VALUES (?,?)', ('e' * 32, 'f' * 32))
                    raise AssertionError('Migration did not enforce recreation lineage')
                except sqlite3.IntegrityError as error:
                    assert 'invalid_recreation_source' in str(error)
                connection.rollback()
            assert hashlib.sha256(previous_backup.read_bytes()).hexdigest() == previous_digest
        # Copy/replace only after the owned process has exited successfully.
        if iteration == 0:
            (runtime / 'retained-sidecar').write_text('whole-directory backup sentinel\n')
            shutil.copytree(runtime, backup)
            assert stat.S_IMODE(backup.stat().st_mode) == 0o700
        elif iteration == 1:
            (runtime / 'post-backup-file').write_text('must not survive restore\n')
            shutil.rmtree(runtime)
            shutil.copytree(backup, runtime)
            assert stat.S_IMODE(runtime.stat().st_mode) == 0o700
            assert (runtime / 'retained-sidecar').read_text() == 'whole-directory backup sentinel\n'
            assert not (runtime / 'post-backup-file').exists()
            for file in backup.iterdir():
                assert file.is_file() and (runtime / file.name).read_bytes() == file.read_bytes()
    invalid_env = dict(server_env, VMANYWARE_DATABASE=str(runtime / 'invalid.sqlite'), VMANYWARE_ACCESS_TOKEN='short')
    invalid = subprocess.run([str(binary)], env=invalid_env, capture_output=True, text=True, timeout=10)
    assert invalid.returncode != 0 and not (runtime / 'invalid.sqlite').exists()
print('Server package passed: checksums, staging/reinstall preservation, unit syntax, auth preflight, actual packaged process/auth/restart/persistence/admission/refill/stopped-backup-restore/history, scoped lifecycle recovery route binding, invalid-token refusal. No host service was installed or started.')

if args.previous_schema:
    print('Previous-schema package check passed: SQL schema 14 -> packaged startup 15, legacy rows/API/history/key unchanged, lineage trigger enforcement, restart and restore, preserved pre-upgrade backup. No older binary executed.')
