"""Select existing CI jobs from the complete Git diff; uncertainty runs both."""
import os
import re
import subprocess
from pathlib import Path


def select(paths):
    server = frontend = False
    for path in paths:
        if path.endswith('.md') or path.startswith('docs/'):
            continue
        if path.startswith(('server/', 'migrations/')):
            server = True
        elif path.startswith('apps/client/src-tauri/'):
            # Native builds have their own path-filtered workflows.
            continue
        elif path.startswith('apps/client/'):
            frontend = True
        elif path.startswith('apps/tui/') or path in ('scripts/check-tui.py', '.github/workflows/tui.yml'):
            # Standalone TUI checks have their own path-filtered workflow.
            continue
        else:
            # Shared locks, toolchains, Makefile, CI and unknown inputs.
            server = frontend = True
    return server, frontend


def main():
    base = os.environ.get('DIFF_BASE', '')
    head = os.environ.get('DIFF_HEAD', '')
    result = (True, True)
    if os.environ.get('DIFF_EVENT') != 'workflow_dispatch':
        if all(re.fullmatch(r'[0-9a-f]{40}', ref) for ref in (base, head)) and base != '0' * 40:
            try:
                paths = subprocess.check_output([
                    'git', 'diff', '--name-only', '--no-renames', '-z', base, head, '--'
                ]).decode('utf-8').rstrip('\0').split('\0')
                result = select([path for path in paths if path])
            except (subprocess.CalledProcessError, UnicodeError):
                pass
    output = '\n'.join(f'{job}={str(enabled).lower()}' for job, enabled in zip(
        ('server', 'frontend'), result
    )) + '\n'
    print(output, end='')
    if destination := os.environ.get('GITHUB_OUTPUT'):
        with Path(destination).open('a') as stream:
            stream.write(output)


if __name__ == '__main__':
    main()
