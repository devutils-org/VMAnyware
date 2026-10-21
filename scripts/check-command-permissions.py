#!/usr/bin/env python3
"""Check the reviewed production handler, manifest and main-window grant agree."""
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1] / 'apps/client/src-tauri'
source = (root / 'src/lib.rs').read_text()
assert '.commands(app_commands::COMMANDS)' in (root / 'build.rs').read_text(), 'Command ACL manifest not enrolled'
handler = source.split('tauri::generate_handler![', 1)[1].split('])', 1)[0]
commands = [item.strip().split('::')[-1] for item in handler.split(',') if item.strip()]
manifest = re.findall(r'"([a-z_]+)"', (root / 'app_commands.rs').read_text())
assert len(commands) == len(set(commands)), 'Duplicate production command'
assert set(commands) == set(manifest), 'Production handler/permission manifest drift'
capability = json.loads((root / 'capabilities/default.json').read_text())
assert capability['identifier'] == 'default'
assert capability['windows'] == ['main'] and capability['local'] is True
assert 'remote' not in capability and 'webviews' not in capability
expected = {'core:default'} | {'allow-' + name.replace('_', '-') for name in commands}
assert set(capability['permissions']) == expected, 'Unexpected or missing production grants'
assert len(capability['permissions']) == len(expected)
config = json.loads((root / 'tauri.conf.json').read_text())
assert config['app']['security']['capabilities'] == ['default'], 'Unexpected enabled capability'
assert 'owned-fixture-report' not in source, 'Fixture permission granted in production'
print(f'Production IPC permissions: {len(commands)} commands, local main window only; fixture reporting ungranted.')
