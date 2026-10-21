#!/usr/bin/env python3
"""Create an allowlisted local Linux server archive; never installs or starts it."""
import argparse
import hashlib
import json
import platform
from pathlib import Path
import shutil
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/vmanyware-server')
parser.add_argument('--output', type=Path, default=ROOT / '.lab/packages')
args = parser.parse_args()
if platform.system() != 'Linux':
    parser.error('Build this package on Linux for the actual deployment architecture.')
binary = args.binary.resolve(strict=True)
with binary.open('rb') as stream:
    header = stream.read(20)
architectures = {62: 'x86_64', 183: 'aarch64'}
if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') not in architectures:
    parser.error('Expected a 64-bit little-endian x86_64/aarch64 Linux ELF binary.')
architecture = architectures[int.from_bytes(header[18:20], 'little')]
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
name = f'vmanyware-server-{version}-linux-{architecture}'
args.output.mkdir(parents=True, exist_ok=True)
archive = args.output / f'{name}.tar.gz'
with tempfile.TemporaryDirectory(prefix='vmanyware-package-') as temporary:
    stage = Path(temporary) / name
    (stage / 'bin').mkdir(parents=True)
    shutil.copyfile(binary, stage / 'bin/vmanyware-server')
    (stage / 'bin/vmanyware-server').chmod(0o755)
    for entry in ['vmanyware.service', 'vmanyware.sysusers', 'server.env.example', 'install.sh']:
        shutil.copyfile(ROOT / 'packaging/server' / entry, stage / entry)
        (stage / entry).chmod(0o755 if entry == 'install.sh' else 0o644)
    shutil.copyfile(ROOT / 'examples/templates.yaml', stage / 'templates.yaml.example')
    for source, target in [('docs/install-server.md', 'INSTALL.md'), ('LICENSE-MIT', 'LICENSE-MIT'), ('LICENSE-APACHE', 'LICENSE-APACHE')]:
        shutil.copyfile(ROOT / source, stage / target)
    (stage / 'BUILD.json').write_text(json.dumps({'version': version, 'architecture': architecture, 'os': 'linux', 'libcBuildHost': platform.libc_ver(), 'releaseCandidate': False}, indent=2) + '\n')
    files = sorted(path for path in stage.rglob('*') if path.is_file())
    (stage / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(stage)}\n' for path in files))
    with tarfile.open(archive, 'w:gz') as output:
        output.add(stage, arcname=name)
checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_suffix(archive.suffix + '.sha256').write_text(f'{checksum}  {archive.name}\n')
print(archive)
