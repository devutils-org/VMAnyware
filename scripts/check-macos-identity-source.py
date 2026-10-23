#!/usr/bin/env python3
"""Typecheck the real identity/Keychain modules for macOS, without Apple SDK or execution."""
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

root = Path(__file__).resolve().parents[1]
target = 'aarch64-apple-darwin'
installed = subprocess.check_output(['rustup', 'target', 'list', '--installed'], text=True).splitlines()
if target not in installed:
    raise SystemExit('Install the pinned toolchain target explicitly: rustup target add aarch64-apple-darwin')
lock = tomllib.loads((root / 'Cargo.lock').read_text())
packages = lock['package']
def version(name, prefix):
    matches = [package['version'] for package in packages if package['name'] == name and package['version'].startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit('Ambiguous locked dependency: ' + name)
    return '=' + matches[0]

output = root / '.lab/macos-identity-source'
output.mkdir(parents=True, exist_ok=True)
manifest = '''[package]
name = "vmanyware-macos-identity-source-check"
version = "0.0.0"
edition = "2021"
publish = false
[workspace]
[lib]
path = "lib.rs"
[dependencies]
'''
for name, prefix in [('sha2', '0.10.'), ('rand', '0.10.'), ('zeroize', '1.'), ('tempfile', '3.'), ('libc', '0.2.')]:
    manifest += json.dumps(name) + ' = ' + json.dumps(version(name, prefix)) + '\n'
manifest += '"security-framework" = { version = ' + json.dumps(version('security-framework', '3.7.')) + ', default-features = false }\n'
manifest += '"serde" = { version = ' + json.dumps(version('serde', '1.')) + ', features = ["derive"] }\n'
manifest += '"ssh-key" = { version = ' + json.dumps(version('ssh-key', '0.7.0-rc.11')) + ', default-features = false, features = ["std", "ed25519", "encryption"] }\n'
(output / 'Cargo.toml').write_text(manifest)
identity = root / 'apps/client/src-tauri/src/ssh/identity.rs'
(output / 'lib.rs').write_text('''// Probe-only: the full application supplies the other identity callers.
#![allow(dead_code)]
pub mod ssh {
    #[path = ''' + json.dumps(str(identity)) + ''']
    pub mod identity;
}
''')
# Seed resolution from the committed lock. The isolated package may prune unused
# entries but must not silently validate a newer dependency than production.
shutil.copyfile(root / 'Cargo.lock', output / 'Cargo.lock')
subprocess.run(['cargo', 'check', '--manifest-path', str(output / 'Cargo.toml'), '--target', target, '--lib'], check=True, cwd=root)
checked = tomllib.loads((output / 'Cargo.lock').read_text())['package']
production = {(package['name'], package['version'], package.get('checksum')) for package in packages}
for package in checked:
    if package['name'] == 'vmanyware-macos-identity-source-check':
        continue
    if (package['name'], package['version'], package.get('checksum')) not in production:
        raise SystemExit('Probe used an unreviewed dependency: ' + package['name'])
print('Real IdentityStore and macOS Keychain backend typecheck for aarch64-apple-darwin with production-locked dependency versions.')
print('Metadata only: no Apple SDK/framework link, full Tauri app build, macOS launch, Keychain access or runtime acceptance.')
