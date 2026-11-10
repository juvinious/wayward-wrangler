#!/usr/bin/env python3
"""Real isolated TUI/service/SDK sign-in with a dummy key; no model/account calls.

Exercises explicit method selection, masked private entry, cancellation, retained
composer input and absence of credentials in project state/history.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import runpy
import select
import sqlite3
import struct
import subprocess
import tempfile
import termios
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, default=Path('target/release'))
    args = parser.parse_args()
    binaries = args.binaries.resolve()
    source = Path(__file__).resolve().parents[1]
    terminal_text = runpy.run_path(str(source / 'scripts/test-project-reopen.py'))['terminal_text']
    with tempfile.TemporaryDirectory(prefix='waywr-login-tui-') as directory:
        root = Path(directory)
        project = root / 'project'
        project.mkdir()
        env = {'PATH': os.environ['PATH'], 'TERM': 'xterm-256color', 'LANG': 'C.UTF-8'}
        for key, name in [('HOME', 'home'), ('XDG_CONFIG_HOME', 'config'),
                          ('XDG_DATA_HOME', 'data'), ('XDG_STATE_HOME', 'registry'),
                          ('XDG_CACHE_HOME', 'cache'), ('XDG_RUNTIME_DIR', 'run')]:
            (root / name).mkdir()
            env[key] = str(root / name)

        def run(argv):
            return subprocess.run(list(map(str, argv)), cwd=project, env=env,
                                  capture_output=True, text=True, timeout=30, check=True).stdout

        bundle = root / 'bundle'
        release = 'local-login-fixture'
        run([binaries / 'waywr', 'bundle-build', '--release-id', release, '--output', bundle,
             '--source-root', source, '--frontend', binaries / 'waywr',
             '--service', binaries / 'waywr-service', '--runner', binaries / 'waywr-runner'])
        app = bundle / 'bin/waywr'
        initialized = json.loads(run([app, 'init', project]))
        state, key = Path(initialized['state']), initialized['project_key']
        prefs_path = project / '.waywr/coordinator.json'
        prefs = json.loads(prefs_path.read_text())
        profile = prefs['profiles'][prefs['coordinator']]
        profile['provider'] = 'anthropic'
        profile['model'] = 'fixture-no-model-request'
        profile['transport']['destination'] = {'provider': 'anthropic', 'kind': 'runtime_builtin'}
        profile['options'] = {}
        prefs_path.write_text(json.dumps(prefs) + '\n')
        ipc = root / 'run/service.sock'
        log = open(root / 'service.log', 'wb')
        service = subprocess.Popen(list(map(str, [bundle / 'bin/waywr-service',
            '--project', project, '--state-dir', state, '--project-key', key,
            '--release-id', release, '--ipc', ipc, '--runner', bundle / 'bin/waywr-runner',
            '--frontend', app])), cwd=project, env=env, stdout=log, stderr=log)
        try:
            until = time.monotonic() + 15
            while not ipc.exists() and service.poll() is None and time.monotonic() < until:
                time.sleep(.05)
            assert ipc.exists() and service.poll() is None, (root / 'service.log').read_text()
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 34, 140, 0, 0))
            ui = subprocess.Popen(list(map(str, [app, '--socket', ipc, '--project-key', key,
                '--release-id', release, '--outbox-dir', state / 'session/outbox'])),
                cwd=project, env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            screen = bytearray()

            def wait_for(check, label):
                until = time.monotonic() + 10
                while not check() and time.monotonic() < until:
                    assert ui.poll() is None, 'UI exited: ' + label
                    if select.select([master], [], [], .05)[0]:
                        screen.extend(os.read(master, 65536))
                assert check(), label + ': ' + terminal_text(screen, 34, 140)[-1600:]

            def visible(text):
                return text in terminal_text(screen, 34, 140)

            def draft():
                path = state / 'session/draft.json'
                if not path.exists():
                    return None
                value = json.loads(path.read_text())
                return value.get('draft', value).get('text')

            try:
                wait_for(lambda: visible('Describe work') or visible('Message the coordinator'), 'Composer missing')
                os.write(master, b'Keep this unsent draft')
                wait_for(lambda: draft() == 'Keep this unsent draft', 'Draft was not saved')
                os.write(master, b'\x1bOR')  # F3
                wait_for(lambda: visible('Project providers'), 'Settings missing')
                os.write(master, b'g')
                wait_for(lambda: visible('Choose provider'), 'Sign-in provider picker missing')
                os.write(master, b'\r')
                wait_for(lambda: visible('choose the SDK sign-in method'), 'Method picker missing')
                # The SDK advertises apiKey first for Anthropic; confirm that route.
                wait_for(lambda: visible('apiKey'), 'API-key method absent')
                os.write(master, b'\r')
                wait_for(lambda: visible('Enter Anthropic API key'), 'SDK secret prompt missing')
                os.write(master, b'\x1b[200~fixture-local-api-key\x1b[201~')
                wait_for(lambda: visible('••••'), 'Secret entry not displayed as masked')
                assert b'fixture-local-api-key' not in screen, 'Key leaked into terminal output'
                os.write(master, b'\r')
                wait_for(lambda: visible('Signed in. Credentials saved privately'), 'Sign-in did not return success')
                credentials = root / 'config/waywr/auth.json'
                assert json.loads(credentials.read_text())['anthropic']['key'] == 'fixture-local-api-key'
                assert credentials.stat().st_mode & 0o777 == 0o600
                os.write(master, b'\r')  # Back to settings, retained draft underneath.
                wait_for(lambda: visible('Project providers'), 'Sign-in did not return to settings')
                # Cancel a second local secret prompt; existing credentials remain.
                os.write(master, b'g')
                wait_for(lambda: visible('Choose provider'), 'Second sign-in missing')
                os.write(master, b'\r')
                wait_for(lambda: visible('choose the SDK sign-in method'), 'Second method picker missing')
                os.write(master, b'\r')
                wait_for(lambda: visible('Enter Anthropic API key'), 'Second SDK prompt missing')
                os.write(master, b'\x1b[200~discarded-private-value\x1b[201~\x1b')
                wait_for(lambda: visible('Project providers'), 'Cancel trapped the interface')
                assert json.loads(credentials.read_text())['anthropic']['key'] == 'fixture-local-api-key'
                assert b'discarded-private-value' not in screen
                os.write(master, b'\x1b')
                wait_for(lambda: visible('Keep this unsent draft'), 'Sign-in lost the draft')
                # Leave while the SDK is still waiting, rather than after settlement.
                os.write(master, b'\x1bOR')
                wait_for(lambda: visible('Project providers'), 'Settings missing before active leave')
                os.write(master, b'g')
                wait_for(lambda: visible('Choose provider'), 'Sign-in missing before active leave')
                os.write(master, b'\r')
                wait_for(lambda: visible('choose the SDK sign-in method'), 'Method missing before active leave')
                os.write(master, b'\r')
                wait_for(lambda: visible('Enter Anthropic API key'), 'SDK not waiting before active leave')
                children = Path(f'/proc/{ui.pid}/task/{ui.pid}/children').read_text().split()
                login_children = [pid for pid in children if b'login-ui' in Path(f'/proc/{pid}/cmdline').read_bytes()]
                assert login_children, 'Expected one owned sign-in child'
                started = time.monotonic()
                os.write(master, b'\x11')
                assert ui.wait(timeout=2) == 0 and time.monotonic() - started < 1
                assert all(not Path(f'/proc/{pid}').exists() for pid in login_children), 'Leaving retained its sign-in child'
                assert service.poll() is None, 'Leaving stopped the project service'
                assert draft() == 'Keep this unsent draft'
                for path in project.rglob('*'):
                    if path.is_file():
                        data = path.read_bytes()
                        assert b'fixture-local-api-key' not in data and b'discarded-private-value' not in data, 'Private input leaked into project data'
                with sqlite3.connect(state / 'state.sqlite') as db:
                    assert db.execute('SELECT count(*) FROM conversation_turns').fetchone()[0] == 0
            finally:
                if ui.poll() is None:
                    ui.terminate()
                    ui.wait(timeout=5)
                os.close(master)
        finally:
            service.terminate()
            service.wait(timeout=10)
            log.close()
        print(json.dumps({'in_app_sdk_login': True, 'masked_private_entry': True,
                          'cancel_and_draft_retention': True, 'model_calls': 0, 'account_calls': 0}))


if __name__ == '__main__':
    main()
