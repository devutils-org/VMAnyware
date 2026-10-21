#!/usr/bin/env python3
"""Verify/copy current debug APKs. Never installs on a device or reads signing keys."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--sdk', type=Path, required=True)
parser.add_argument('--output', type=Path, default=ROOT / '.lab/packages/android')
args = parser.parse_args()
build_tools = args.sdk.resolve() / 'build-tools/36.0.0'
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
output = args.output.resolve()
output.mkdir(parents=True, exist_ok=True)

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

entries = []
for variant, abi, target in [('arm64', 'arm64-v8a', 'aarch64-linux-android'), ('x86_64', 'x86_64', 'x86_64-linux-android')]:
    apk = ROOT / f'apps/client/src-tauri/gen/android/app/build/outputs/apk/{variant}/debug/app-{variant}-debug.apk'
    library = ROOT / f'target/{target}/debug/libvmanyware_client_lib.so'
    signature = subprocess.run([str(build_tools / 'apksigner'), 'verify', '--verbose', '--print-certs', str(apk)], check=True, capture_output=True, text=True).stdout
    certificate = re.search(r'Signer #1 certificate SHA-256 digest: ([0-9a-f]+)', signature)
    assert certificate, 'APK signer certificate missing'
    badging = subprocess.run([str(build_tools / 'aapt2'), 'dump', 'badging', str(apk)], check=True, capture_output=True, text=True).stdout
    assert "name='dev.vmanyware.client'" in badging and 'application-debuggable' in badging
    assert f"versionName='{version}'" in badging, "APK version differs from workspace version"
    assert re.search(r"^(?:minSdkVersion|sdkVersion):'24'$", badging, re.MULTILINE) and "targetSdkVersion:'36'" in badging
    assert 'testOnly' not in badging, 'Do not distribute a test-only APK as a normal manual-install artifact'
    manifest = subprocess.run([str(build_tools / 'aapt2'), 'dump', 'xmltree', str(apk), '--file', 'AndroidManifest.xml'], check=True, capture_output=True, text=True).stdout
    assert 'androidx.core.content.FileProvider' not in manifest, 'Unused file-sharing provider must not be packaged'
    for attribute in ('allowBackup', 'fullBackupContent'):
        assert re.search(r':' + attribute + r'\(0x[0-9a-f]+\)=false$', manifest, re.MULTILINE), 'Backup policy missing: ' + attribute
    assert re.search(r':dataExtractionRules\(0x[0-9a-f]+\)=@0x[0-9a-f]+$', manifest, re.MULTILINE), 'Extraction policy missing'
    with zipfile.ZipFile(apk) as archive:
        assert 'res/xml/file_paths.xml' not in archive.namelist(), 'Broad sharing-path resource remains in APK'
        dex_files = [name for name in archive.namelist() if re.fullmatch(r'classes[0-9]*\.dex', name)]
        assert any(b'Ldev/vmanyware/client/IdentityVault;' in archive.read(name) for name in dex_files), 'Android identity vault helper missing from APK'
        names = [entry for entry in archive.namelist() if entry.startswith('lib/') and entry.endswith('/libvmanyware_client_lib.so')]
        assert names == [f'lib/{abi}/libvmanyware_client_lib.so'], names
        with archive.open(names[0]) as native:
            packaged_hash = hashlib.file_digest(native, 'sha256').hexdigest()
        assert packaged_hash == digest(library), 'APK native library differs from the current target build'
    destination = output / f'vmanyware-{version}-android-{abi}-debug.apk'
    shutil.copyfile(apk, destination)
    entries.append({'file': destination.name, 'abi': abi, 'package': 'dev.vmanyware.client', 'debuggable': True, 'minSdk': 24, 'targetSdk': 36, 'sha256': digest(destination), 'nativeLibrarySha256': packaged_hash, 'signerCertificateSha256': certificate.group(1), 'bytes': destination.stat().st_size})
assert len({entry['signerCertificateSha256'] for entry in entries}) == 1, 'Split APK signatures differ'
(output / 'artifacts.json').write_text(json.dumps({'version': version, 'releaseCandidate': False, 'runtimeValidatedForTheseApks': False, 'artifacts': entries}, indent=2) + '\n')
(output / 'SHA256SUMS').write_text(''.join(f"{entry['sha256']}  {entry['file']}\n" for entry in entries))
shutil.copyfile(ROOT / 'docs/install-android.md', output / 'INSTALL.md')
print(json.dumps({'directory': str(output), 'artifacts': [{'file': entry['file'], 'bytes': entry['bytes']} for entry in entries]}, indent=2))
