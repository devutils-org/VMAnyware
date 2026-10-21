#!/usr/bin/env python3
"""Opt-in Linux Secret Service tests on a private bus and disposable keyring only."""
import argparse
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--prefix', type=Path, required=True)
parser.add_argument('--test-binary', type=Path, required=True)
parser.add_argument('--inside', action='store_true', help=argparse.SUPPRESS)
parser.add_argument('--case', help=argparse.SUPPRESS)
args = parser.parse_args()
prefix = args.prefix.resolve(strict=True)
binary = args.test_binary.resolve(strict=True)
daemon = prefix / 'sysroot/usr/bin/gnome-keyring-daemon'
assert daemon.is_file() and os.access(binary, os.X_OK)
cases = [
    'ssh::agent::app_tests::owned_vault_ssh_selection_checks_host_owner_and_actual_authentication',
    'ssh::identity::vault::tests::owned_vault_roundtrip_refuses_other_identity_duplicates_wrong_secret_and_locked_store',
]
if args.inside:
    def stop(_signal, _frame):
        raise SystemExit('Owned vault runner interrupted')
    signal.signal(signal.SIGTERM, stop)
    assert args.case in cases
    root = Path(os.environ['VMANYWARE_OWNED_VAULT_TEST'])
    assert root.joinpath('owned-test-bus').is_file()
    assert os.environ['DBUS_SESSION_BUS_ADDRESS'].startswith('unix:path=')
    with root.joinpath('daemon.log').open('w+') as log:
        service = subprocess.Popen([str(daemon), '--foreground', '--unlock', '--components=secrets',
                                    '--control-directory', str(root / 'control')],
                                   stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            service.stdin.write(b'disposable vault daemon phrase 42')
            service.stdin.close()
            deadline = time.monotonic() + 10
            while True:
                ready = subprocess.run(['gdbus', 'call', '--session', '--dest', 'org.freedesktop.secrets',
                                        '--object-path', '/org/freedesktop/secrets', '--method', 'org.freedesktop.DBus.Peer.Ping'],
                                       capture_output=True, timeout=2)
                if ready.returncode == 0:
                    break
                if service.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError('Owned Secret Service did not start')
                time.sleep(0.05)
            result = subprocess.run([str(binary), args.case, '--exact', '--ignored', '--nocapture'], capture_output=True, text=True, timeout=90)
            print(result.stdout, end='')
            if result.returncode != 0:
                print(result.stderr, end='', file=sys.stderr)
            assert result.returncode == 0 and 'running 1 test' in result.stdout and '1 passed' in result.stdout, 'Owned vault test did not pass exactly one case'
        finally:
            if service.poll() is None:
                os.killpg(service.pid, signal.SIGTERM)
                try:
                    service.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(service.pid, signal.SIGKILL)
                    service.wait()
else:
    for case in cases:
        with tempfile.TemporaryDirectory(prefix='vmanyware-vault-') as temporary:
            root = Path(temporary)
            env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8',
                   'LD_LIBRARY_PATH': str(prefix / 'sysroot/usr/lib'),
                   'VMANYWARE_OWNED_VAULT_TEST': str(root)}
            for key, name in [('XDG_CONFIG_HOME', 'config'), ('XDG_CACHE_HOME', 'cache'),
                              ('XDG_DATA_HOME', 'data'), ('XDG_RUNTIME_DIR', 'runtime')]:
                root.joinpath(name).mkdir(mode=0o700)
                env[key] = str(root / name)
            root.joinpath('control').mkdir(mode=0o700)
            root.joinpath('owned-test-bus').write_text('Private runner fixture; no personal vault access.\n')
            child = subprocess.Popen(['dbus-run-session', '--', sys.executable, str(Path(__file__).resolve()),
                                      '--prefix', str(prefix), '--test-binary', str(binary), '--inside', '--case', case],
                                     env=env, start_new_session=True)
            try:
                assert child.wait(timeout=110) == 0, 'Owned vault case failed'
            finally:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGTERM)
                    try:
                        child.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
    print('Owned Linux Secret Service: scoped save/reopen/forget, duplicate/corrupt/locked refusal and real SSH authentication passed. No personal bus/keyring or provider accessed.')
