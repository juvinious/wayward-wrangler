#!/usr/bin/env python3
"""Go forward from a broken session using real isolated systemd/IPC/TUI.

No operator project, installation, credentials or model call is used. The held
operation is sleep, not a model; only uniquely keyed fixture units are stopped.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import struct
import subprocess
import tempfile
import termios
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, default=Path('target/release'))
    parser.add_argument('--busy', action='store_true', help='Reconnect to a healthy active owner after an update')
    args = parser.parse_args()
    binaries = args.binaries.resolve()
    source = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix='waywr-renewal-') as temporary:
        root = Path(temporary)
        project = root / 'project'
        project.mkdir()
        env = {'PATH': os.environ['PATH'], 'TERM': 'xterm-256color', 'LANG': 'C.UTF-8',
               'DBUS_SESSION_BUS_ADDRESS': os.environ.get('DBUS_SESSION_BUS_ADDRESS', f'unix:path=/run/user/{os.getuid()}/bus')}
        for key, name in [('HOME', 'home'), ('XDG_CONFIG_HOME', 'config'),
                          ('XDG_STATE_HOME', 'registry'), ('XDG_DATA_HOME', 'data'),
                          ('XDG_CACHE_HOME', 'cache')]:
            (root / name).mkdir()
            env[key] = str(root / name)
        env['XDG_RUNTIME_DIR'] = os.environ.get('XDG_RUNTIME_DIR', f'/run/user/{os.getuid()}')

        def run(argv):
            result = subprocess.run(list(map(str, argv)), cwd=project, env=env,
                                    capture_output=True, text=True, timeout=40)
            assert result.returncode == 0, result.stderr
            return result.stdout

        for name in ['before', 'after']:
            run([binaries / 'waywr', 'bundle-build', '--release-id', 'renewal-' + name,
                 '--output', root / name, '--source-root', source,
                 '--frontend', binaries / 'waywr', '--service', binaries / 'waywr-service',
                 '--runner', binaries / 'waywr-runner'])
        old = root / 'before/bin/waywr'
        new = root / 'after/bin/waywr'
        initialized = json.loads(run([old, 'init', project]))
        key = initialized['project_key']
        state = Path(initialized['state'])
        unit = f'waywr-{key}.service'
        operation_unit = f'waywr-op-{key}-' + 'b' * 16 + '-' + 'c' * 32 + '.service'
        cli = ['--project', project, '--ipc-dir', root / 'run']
        def open_ui(launcher):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 120, 0, 0))
            ui = subprocess.Popen(list(map(str, [launcher, *cli])), cwd=project, env=env,
                                  stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            try:
                screen = bytearray()
                deadline = time.monotonic() + 15
                def composer_open():
                    return any(text in screen for text in [b'Message the coordinator', b'Describe work'])
                while not composer_open() and time.monotonic() < deadline and ui.poll() is None:
                    if select.select([master], [], [], .1)[0]:
                        try: screen.extend(os.read(master, 65536))
                        except OSError: break
                assert composer_open(), re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', screen).decode(errors='replace')[:2500]
                os.write(master, b'\x11')
                time.sleep(.2)
                os.write(master, b'\x1b[C\r')  # Current confirmation; direct detach has its own slice.
                assert ui.wait(timeout=10) == 0
            finally:
                if ui.poll() is None:
                    ui.terminate()
                    ui.wait(timeout=10)
                os.close(master)

        try:
            open_ui(old)
            preferences = (project / '.waywr/coordinator.json').read_bytes()
            workflow = project / '.waywr/workflow.json'
            settings = json.loads(workflow.read_text())
            if not args.busy:
                settings['target'] = 'obsolete-external-target'
                workflow.write_text(json.dumps(settings))
            retained = project / '.waywr/worktrees/partial.txt'
            retained.parent.mkdir(parents=True)
            retained.write_text('Uncommitted project work must remain.\n')
            (project / 'source.txt').write_text('Project source must remain.\n')
            attempt = state / 'session/coordinator/old/call-v1/attempts' / ('c' * 32)
            attempt.mkdir(parents=True)
            intent = {'schema_version': 1, 'project_key': key, 'attempt_id': 'c' * 32,
                      'unit': operation_unit}
            (attempt / 'intent.json').write_text(json.dumps(intent))
            uncertain = b'{"reason":"Old dispatch outcome unknown"}\n'
            if not args.busy:
                (attempt / 'uncertain.json').write_bytes(uncertain)
            (attempt / 'spawn-committed.json').write_text('{}')
            run(['systemd-run', '--user', '--collect', '--unit=' + operation_unit,
                 '--property=WorkingDirectory=' + str(attempt), '--property=KillMode=control-group',
                 '/usr/bin/sleep', '300'])
            owner = (state / 'supervision/owner.json').read_bytes()
            open_ui(new)
            if args.busy:
                assert (state / 'supervision/owner.json').read_bytes() == owner, 'Busy service was replaced'
                assert not (project / '.waywr/retired-sessions').exists(), 'Healthy session was discarded'
                assert (attempt / 'spawn-committed.json').exists()
                run(['systemctl', '--user', 'stop', operation_unit])
                shutil.rmtree(state / 'session/coordinator/old')
                open_ui(new)
                assert json.loads((state / 'supervision/owner.json').read_bytes())['release_id'] == 'renewal-after'
                print(json.dumps({'busy_owner_reconnected': True, 'idle_owner_replaced': True,
                                  'real_systemd_ipc_tui': True, 'model_calls': 0}))
                return
            retired = list((project / '.waywr/retired-sessions').glob('*/state'))
            assert len(retired) == 1, 'Broken session was not retired'
            assert (retired[0] / attempt.relative_to(state) / 'uncertain.json').read_bytes() == uncertain
            assert not list(state.rglob('uncertain.json')), 'Old attempt remains live'
            assert not list(state.rglob('spawn-committed.json')), 'Old attempt was replayed'
            assert (project / '.waywr/coordinator.json').read_bytes() == preferences
            assert json.loads(workflow.read_text())['target'] != 'obsolete-external-target', 'Fresh session retained its obsolete target'
            assert retained.read_text() == 'Uncommitted project work must remain.\n'
            assert (project / 'source.txt').read_text() == 'Project source must remain.\n'
            assert run(['systemctl', '--user', 'show', '--property=MainPID', '--value', operation_unit]).strip() == '0'
            open_ui(new)
            assert len(list((project / '.waywr/retired-sessions').glob('*/state'))) == 1, 'Reopen reset a healthy session'
            # Obsolete provider settings are replaced at startup, without an
            # F3 save/switch/context-grant ceremony for a discarded session.
            legacy = json.loads(preferences)
            legacy['schema_version'] = 2
            for profile in legacy['profiles'].values():
                profile.pop('transport', None)
                profile.pop('options', None)
                profile['binary'] = '/retired/client-must-not-run'
            (project / '.waywr/coordinator.json').write_text(json.dumps(legacy))
            open_ui(new)
            renewed = json.loads((project / '.waywr/coordinator.json').read_text())
            assert renewed['schema_version'] == 3
            assert renewed['profiles'][renewed['coordinator']]['transport']['entry'] == 'bundled'
            assert len(list((project / '.waywr/retired-sessions').glob('*/state'))) == 2
            print(json.dumps({'fresh_session_opened': True, 'old_units_stopped': True,
                              'model_settings_and_source_retained': True, 'old_attempt_not_replayed': True,
                              'retired_profiles_replaced_at_startup': True,
                              'real_systemd_ipc_tui': True, 'model_calls': 0}))
        finally:
            subprocess.run(['systemctl', '--user', 'stop', unit, operation_unit], env=env,
                           capture_output=True, timeout=20)


if __name__ == '__main__':
    main()
