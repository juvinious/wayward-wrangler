#!/usr/bin/env python3
"""Exercise the actual workflow bootstrap commands with an isolated apt stub.

No package manager, network, container or system configuration is modified.
"""
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def bootstrap(path):
    # The bootstrap precedes checkout, so it intentionally stays inline in YAML.
    text = path.read_text().split('      - uses: actions/checkout@', 1)[0]
    line = re.search(r'^      - (?:name:.*\n        )?run: (.*)$', text, re.M)
    assert line, path
    if line[1] != '|':
        return line[1]
    lines = []
    for item in text[line.end():].splitlines():
        if not item.strip():
            continue
        if not item.startswith('          '):
            break
        lines.append(item[10:])
    return '\n'.join(lines)


def main():
    for name in ['ci.yml', 'release.yml']:
        command = bootstrap(ROOT / '.github/workflows' / name)
        for permanent in [False, True]:
            with tempfile.TemporaryDirectory(prefix='waywr-apt-fixture-') as directory:
                root = Path(directory)
                apt = root / 'apt-get'
                apt.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> "$APT_FIXTURE_LOG"
case "$*" in
 *update*)
  if [ ! -e "$APT_FIXTURE_MARKER" ]; then
   : > "$APT_FIXTURE_MARKER"
   exit 100
  fi
  if [ "$APT_FIXTURE_PERMANENT" = 1 ]; then exit 100; fi;;
esac
''')
                apt.chmod(0o700)
                (root / 'sleep').write_text('#!/bin/sh\nexit 0\n')
                (root / 'sleep').chmod(0o700)
                env = {'PATH': str(root) + ':' + os.environ['PATH'],
                       'APT_FIXTURE_LOG': str(root / 'calls'), 'APT_FIXTURE_MARKER': str(root / 'failed'),
                       'APT_FIXTURE_PERMANENT': str(int(permanent))}
                result = subprocess.run(['sh', '-e', '-c', command], env=env, capture_output=True, text=True, timeout=5)
                calls = (root / 'calls').read_text().splitlines()
                if permanent:
                    assert result.returncode != 0, 'Permanent mirror failure was hidden'
                    assert not any('install' in call for call in calls), 'Installed with failed/stale package indexes'
                    assert len(calls) <= 4, 'Bootstrap retries are unbounded'
                else:
                    assert result.returncode == 0, f'{name}: temporary DNS failure was not retried'
                    assert sum('update' in call for call in calls) == 2
                    assert any('Error-Mode=any' in call for call in calls)
                    assert sum('install' in call for call in calls) == 1
    print('Builder prerequisite retries: temporary recovery and bounded permanent failure OK')


if __name__ == '__main__':
    main()
