#!/usr/bin/env python3
"""Replace a real paired install, remove the old build, and reopen saved state.

Uses real binaries, bundle installation, IPC and TUI. Services are started directly
inside the fixture: this checks project reopening, not systemd bootstrap or model
execution. No operator installation, credentials, network or project is used.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shutil
import struct
import subprocess
import tempfile
import termios
import time


def wait_for(check, message, seconds=15):
    deadline = time.monotonic() + seconds
    while not check():
        if time.monotonic() >= deadline:
            raise AssertionError(message)
        time.sleep(.05)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, default=Path('target/release'))
    parser.add_argument('--systemd-handover', action='store_true',
                        help='Use real uniquely keyed user units; replace a still-running idle service')
    args = parser.parse_args()
    binaries = args.binaries.resolve()
    source = Path(__file__).resolve().parent.parent
    for name in ['waywr', 'waywr-service', 'waywr-runner']:
        if not (binaries / name).is_file():
            parser.error('Build the paired release binaries before this check')
    with tempfile.TemporaryDirectory(prefix='waywr-reopen-') as temporary:
        root = Path(temporary)
        for name in ['home', 'project', 'config', 'data', 'state', 'cache', 'run']:
            (root / name).mkdir(mode=0o700)
        (root / 'bin').mkdir()
        node=shutil.which('node'); assert node, 'Node runtime prerequisite missing'
        (root / 'bin/node').symlink_to(node)
        project = root / 'project'
        env = {'HOME': str(root / 'home'), 'PATH': str(root / 'bin') + ':/usr/bin:/bin',
               'TERM': 'xterm-256color', 'LANG': 'C.UTF-8',
               'DBUS_SESSION_BUS_ADDRESS': 'unix:path=' + str(root / 'no-user-bus')}
        if args.systemd_handover:
            env['DBUS_SESSION_BUS_ADDRESS'] = os.environ.get(
                'DBUS_SESSION_BUS_ADDRESS', f'unix:path=/run/user/{os.getuid()}/bus')
        for key, name in [('CONFIG', 'config'), ('DATA', 'data'), ('STATE', 'state'),
                          ('CACHE', 'cache'), ('RUNTIME', 'run')]:
            env['XDG_' + key + ('_HOME' if key != 'RUNTIME' else '_DIR')] = str(root / name)
        if args.systemd_handover:
            # systemctl uses the real user manager's runtime directory. Keep
            # Waywr's IPC isolated through its explicit --ipc-dir option.
            env['XDG_RUNTIME_DIR'] = os.environ.get('XDG_RUNTIME_DIR', f'/run/user/{os.getuid()}')

        def run(argv):
            if str(argv[0]) == str(launcher):
                argv = [*argv, '--ipc-dir', root / 'run']
            result = subprocess.run(list(map(str, argv)), cwd=project, env=env,
                                    capture_output=True, text=True, timeout=30)
            assert result.returncode == 0, result.stderr
            return result.stdout

        prefix = root / 'home/.local/share/wayward-wrangler'
        launcher = root / 'home/.local/bin/waywr'
        releases = ['reopen-before', 'reopen-after']
        for release in releases:
            bundle = root / release
            run([binaries / 'waywr', 'bundle-build', '--release-id', release,
                 '--output', bundle, '--source-root', source,
                 '--frontend', binaries / 'waywr', '--service', binaries / 'waywr-service',
                 '--runner', binaries / 'waywr-runner'])
        run([binaries / 'waywr', 'bundle-install', '--bundle', root / releases[0],
             '--prefix', prefix, '--select-default'])
        initialized = json.loads(run([launcher, 'init', project]))
        state = Path(initialized['state'])
        key = initialized['project_key']
        unit = f'waywr-{key}.service'
        ipc = root / 'run/waywr' / (key + '.sock')
        ipc.parent.mkdir(exist_ok=True)
        kept = [project / '.waywr/project.toml', project / '.waywr/coordinator.json',
                project / '.waywr/release.json']
        before = {path: path.read_bytes() for path in kept}
        draft = 'Continue this work after updating Waywr'
        service = None

        def refuses_invalid_metadata():
            # Test the public setup entry point without relying on a missing bus
            # or process to cause refusal. Each fixture is restored byte-for-byte.
            contract = state / 'state-contract.json'
            for path, field, invalid in [(contract, 'state_schema', 999),
                                         (project / '.waywr/release.json', 'project_key', '0' * 32)]:
                original = path.read_bytes()
                try:
                    changed = json.loads(original)
                    changed[field] = invalid
                    path.write_text(json.dumps(changed))
                    result = subprocess.run([str(launcher), 'init', str(project)], env=env,
                                            cwd=project, text=True, capture_output=True, timeout=10)
                    assert result.returncode != 0, 'Setup accepted incompatible/foreign project metadata'
                    assert any(word in result.stderr.lower() for word in ['schema', 'contract', 'identity', 'release binding']), result.stderr
                    assert json.loads(path.read_text()) == changed, 'Refusal changed project metadata'
                finally:
                    path.write_bytes(original)

        def saved_draft():
            path = state / 'session/draft.json'
            if not path.exists():
                return None
            value = json.loads(path.read_text())
            return value.get('draft', value).get('text')

        def start_service(release):
            nonlocal service
            installed = prefix / 'releases' / release
            log = open(root / (release + '.log'), 'wb')
            try:
                service = subprocess.Popen(list(map(str, [installed / 'bin/waywr-service',
                    '--project', project, '--state-dir', state, '--project-key', key,
                    '--release-id', release, '--ipc', ipc,
                    '--runner', installed / 'bin/waywr-runner',
                    '--frontend', installed / 'bin/waywr'])), cwd=project, env=env,
                    stdout=log, stderr=log)
            finally:
                log.close()
            wait_for(lambda: ipc.exists() or service.poll() is not None,
                     'Fresh isolated service did not start')
            assert service.poll() is None, (root / (release + '.log')).read_text()

        def stop_service():
            nonlocal service
            if service is not None and service.poll() is None:
                service.terminate()
                service.wait(timeout=10)
            service = None

        def unit_property(name):
            return run(['systemctl', '--user', 'show', '--property=' + name, '--value', unit]).strip()

        def replace_install():
            run([binaries / 'waywr', 'bundle-install', '--bundle', root / releases[1],
                 '--prefix', prefix, '--select-default'])
            shutil.rmtree(prefix / 'releases' / releases[0])
            shutil.rmtree(root / releases[0])

        def require_handover_refusal():
            pid = unit_property('MainPID')
            result = subprocess.run([str(launcher), '--ipc-dir', str(root / 'run')], cwd=project, env=env,
                                    capture_output=True, text=True, timeout=20)
            assert result.returncode != 0, 'Replacement interrupted busy service'
            assert 'update waits for this project to be idle' in result.stderr, result.stderr
            assert unit_property('MainPID') == pid, 'Busy service owner changed'

        def open_and_detach(expected_draft=None, while_open=None):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 120, 0, 0))
            ui = subprocess.Popen([str(launcher), '--ipc-dir', str(root / 'run')], cwd=project, env=env,
                                  stdin=slave, stdout=slave, stderr=slave,
                                  start_new_session=True)
            os.close(slave)
            screen = bytearray()
            try:
                # A restored nonempty composer displays the draft instead of
                # the empty-input placeholder used on the first launch.
                ready_text = expected_draft.encode() if expected_draft is not None else b'Message the coordinator'
                deadline = time.monotonic() + 15
                while ready_text not in screen and time.monotonic() < deadline:
                    if select.select([master], [], [], .1)[0]:
                        try:
                            screen.extend(os.read(master, 65536))
                        except OSError:
                            break
                    if ui.poll() is not None:
                        break
                assert ready_text in screen, (
                    'Normal launch failed before the composer opened: ' +
                    screen.decode(errors='replace')[-1200:])
                if expected_draft is None:
                    os.write(master, draft.encode())
                    wait_for(lambda: saved_draft() == draft,
                             'TUI did not save the composed draft')
                else:
                    assert expected_draft.encode() in screen, 'Saved draft was not restored in the composer'
                if while_open:
                    while_open()
                os.write(master, b'\x11')  # Ctrl-Q opens Detach without submitting the draft.
                time.sleep(.2)
                os.write(master, b'\x1b[C\r')
                assert ui.wait(timeout=10) == 0
            finally:
                if ui.poll() is None:
                    ui.terminate()
                    ui.wait(timeout=10)
                os.close(master)

        try:
            refuses_invalid_metadata()
            if args.systemd_handover:
                # The ordinary launcher owns real systemd bootstrap and handover.
                # Removing A while its TUI is attached must not interrupt it.
                def replace_while_attached():
                    process_env = Path('/proc') / unit_property('MainPID') / 'environ'
                    inherited = dict(item.split(b'=', 1) for item in process_env.read_bytes().split(b'\0') if b'=' in item)
                    for name in ['HOME', 'PATH']:
                        assert inherited.get(name.encode()) == env[name].encode(), 'Service must preserve the calling terminal environment: ' + name
                    replace_install()
                    require_handover_refusal()
                open_and_detach(while_open=replace_while_attached)
                old_pid = unit_property('MainPID')
                old_owner = json.loads((state / 'supervision/owner.json').read_text())
                pending = state / 'session/outbox/fixture-pending.json'
                pending.parent.mkdir(exist_ok=True)
                pending.write_text('{}')
                try:
                    require_handover_refusal()
                finally:
                    pending.unlink()
                uncertain = state / 'session/coordinator/fixture-call/attempt/launch.json'
                uncertain.parent.mkdir(parents=True)
                uncertain.write_text('{}')
                try:
                    require_handover_refusal()
                    assert uncertain.read_text() == '{}', 'Uncertain launch evidence changed'
                finally:
                    shutil.rmtree(state / 'session/coordinator/fixture-call')
                open_and_detach(draft)
                assert unit_property('MainPID') != old_pid, 'Old service was not handed over'
                owner = json.loads((state / 'supervision/owner.json').read_text())
                assert owner['generation'] > old_owner['generation']
                assert owner['release_id'] == releases[1]
                assert all(path.read_bytes() == value for path, value in before.items())
                assert saved_draft() == draft
                assert not list(state.rglob('spawn-committed.json'))
                print(json.dumps({'real_systemd_bootstrap_and_idle_handover': True,
                                  'live_ui_pending_delivery_and_uncertain_launch_preserved': True,
                                  'old_build_removed': True, 'model_calls': 0}))
                return
            start_service(releases[0])
            open_and_detach()
            run([launcher, 'session', 'stop'])
            wait_for(lambda: json.loads(run([launcher, 'session', 'status']))['session']['phase'] == 'paused',
                     'Project did not reach its saved pause')
            stop_service()
            assert saved_draft() == draft
            # Replace through the real installer; never edit the project binding.
            replace_install()
            assert launcher.resolve() == prefix / 'releases' / releases[1] / 'bin/waywr'
            refuses_invalid_metadata()
            start_service(releases[1])
            open_and_detach(draft)
            status = json.loads(run([launcher, 'session', 'status']))
            assert status['session']['phase'] == 'paused', 'Opening resumed execution'
            run([launcher, 'init', project])  # Repeated setup must preserve saved state too.
            assert all(path.read_bytes() == contents for path, contents in before.items()), 'Project settings/legacy metadata changed'
            assert not list(state.rglob('spawn-committed.json'))
            assert not list(state.rglob('ollama-send-intent.json'))
            print(json.dumps({'reopened_with_old_build_removed': True,
                              'draft_settings_and_pause_preserved': True,
                              'manual_rebinds': 0, 'model_calls': 0,
                              'service_bootstrap': 'direct isolated process, not systemd'}))
        finally:
            stop_service()
            if args.systemd_handover:
                subprocess.run(['systemctl', '--user', 'stop', unit], env=env,
                               capture_output=True, timeout=15)


if __name__ == '__main__':
    main()
