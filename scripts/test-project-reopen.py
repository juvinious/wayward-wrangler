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
import re
import select
import shutil
import sqlite3
import socket
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


def terminal_text(data, rows, columns):
    """Reconstruct Ratatui's positioned text instead of concatenating diff writes."""
    grid = [[' '] * columns for _ in range(rows)]
    x = y = 0
    for match in re.finditer(r'\x1b\[([0-?]*)([ -/]*)([@-~])|([^\x1b])', data.decode(errors='replace'), re.S):
        params, _, command, char = match.groups()
        if command:
            if command not in 'HfABCDJK' or re.fullmatch(r'[0-9;]*', params) is None:
                continue
            values = [int(p) if p else 0 for p in params.split(';')]
            count = values[0] or 1
            if command in ('H', 'f'):
                y = min(rows - 1, max(0, count - 1))
                x = min(columns - 1, max(0, (values[1] if len(values) > 1 else 1) - 1))
            elif command == 'A': y = max(0, y - count)
            elif command == 'B': y = min(rows - 1, y + count)
            elif command == 'C': x = min(columns - 1, x + count)
            elif command == 'D': x = max(0, x - count)
            elif command == 'J' and values[0] in (2, 3): grid = [[' '] * columns for _ in range(rows)]
            elif command == 'K':
                start, end = (0, columns) if values[0] == 2 else (0, x + 1) if values[0] == 1 else (x, columns)
                grid[y][start:end] = [' '] * (end - start)
        elif char == '\r': x = 0
        elif char == '\n': y = min(rows - 1, y + 1)
        elif char and char.isprintable():
            grid[y][x] = char
            x = min(columns - 1, x + 1)
    return '\n'.join(''.join(row) for row in grid)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, default=Path('target/release'))
    parser.add_argument('--retired-profiles', action='store_true',
                        help='Open schema-2 native settings for F3 replacement without executing or rewriting them')
    parser.add_argument('--provider-navigation', action='store_true',
                        help='Use real F3 terminal input to select a model at the end of the catalog and save it in the fixture')
    parser.add_argument('--activate-profile', action='store_true',
                        help='Explicitly activate the saved model through its F3 review while the fixture session is paused')
    parser.add_argument('--coordinator-usage', action='store_true',
                        help='Display inert retained coordinator telemetry through real IPC/TUI, including after reopening')
    parser.add_argument('--idle-cpu', action='store_true',
                        help='Measure real idle TUI/service CPU with the full catalog and seeded retained history')
    parser.add_argument('--blocked-send', action='store_true',
                        help='Use Send to activate the saved model from a retired binding and exercise both input-clear keys')
    parser.add_argument('--systemd-handover', action='store_true',
                        help='Use real uniquely keyed user units; replace a still-running idle service')
    parser.add_argument('--portable', action='store_true',
                        help='Use a temporary symlink directly to paired bundles, without bundle-install')
    parser.add_argument('--populated-history', action='store_true',
                        help='Seed retained display records so real snapshot delivery exceeds 1 MiB')
    parser.add_argument('--previous-binaries', type=Path,
                        help='Use an older paired bin directory for the initial service in the populated systemd upgrade check')
    args = parser.parse_args()
    if args.portable and not args.systemd_handover:
        parser.error('--portable requires --systemd-handover to exercise ordinary service replacement')
    if args.previous_binaries and not (args.systemd_handover and args.populated_history):
        parser.error('--previous-binaries requires --systemd-handover and --populated-history')
    if args.activate_profile and (not args.provider_navigation or args.systemd_handover):
        parser.error('--activate-profile requires --provider-navigation with the direct isolated service')
    if args.coordinator_usage and args.systemd_handover:
        parser.error('--coordinator-usage uses the direct isolated service')
    if args.blocked_send and not (args.activate_profile and args.retired_profiles):
        parser.error('--blocked-send requires --activate-profile and --retired-profiles')
    if args.idle_cpu and (args.systemd_handover or not args.populated_history):
        parser.error('--idle-cpu requires --populated-history with the direct isolated service')
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
        if args.portable:
            launcher = root / 'local-bin/waywr'
        def release_root(release):
            return root / release if args.portable else prefix / 'releases' / release
        for release in releases:
            bundle = root / release
            pair = args.previous_binaries.resolve() if args.previous_binaries and release == releases[0] else binaries
            run([binaries / 'waywr', 'bundle-build', '--release-id', release,
                 '--output', bundle, '--source-root', source,
                 '--frontend', pair / 'waywr', '--service', pair / 'waywr-service',
                 '--runner', pair / 'waywr-runner'])
        if args.portable:
            launcher.parent.mkdir()
            launcher.symlink_to(release_root(releases[0]) / 'bin/waywr')
        else:
            run([binaries / 'waywr', 'bundle-install', '--bundle', root / releases[0],
                 '--prefix', prefix, '--select-default'])
        initialized = json.loads(run([launcher, 'init', project]))
        state = Path(initialized['state'])
        if args.retired_profiles:
            path = project / '.waywr/coordinator.json'
            prefs = json.loads(path.read_text())
            prefs['schema_version'] = 2
            for profile in prefs['profiles'].values():
                profile.pop('options', None)
                profile.pop('transport', None)
                profile['binary'] = '/retired/native-client-must-not-run'
            path.write_text(json.dumps(prefs) + '\n')
        key = initialized['project_key']
        unit = f'waywr-{key}.service'
        ipc = root / 'run/waywr' / (key + '.sock')
        ipc.parent.mkdir(exist_ok=True)
        kept = [project / '.waywr/project.toml', project / '.waywr/coordinator.json',
                project / '.waywr/release.json']
        before = {path: path.read_bytes() for path in kept}
        catalog = run([launcher, 'models', 'list']).splitlines() if args.provider_navigation else []
        if args.provider_navigation:
            assert len(catalog) > 16, 'SDK catalog must exercise navigation beyond the former preview'
            selected_provider, selected_model = catalog[-2].split(' · context ', 1)[0].split('/', 1)
            metadata = subprocess.run([node, str(release_root(releases[0]) / 'resources/model-runtime/bridge.mjs'), 'describe'],
                                      input='{}', capture_output=True, text=True, env=env, timeout=15, check=True)
            model = next(m for m in json.loads(metadata.stdout)['models'] if m['provider'] == selected_provider and m['model'] == selected_model)
            selected_thinking = model['thinking_levels'][-1]
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
            installed = release_root(release)
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

        def seed_history():
            with sqlite3.connect(state / 'state.sqlite') as database:
                # Inert retained notifications exercise projection size without
                # seeding a request, authority, worker or model call.
                for index in range(8):
                    record = {'id': f'fixture-history-{index}', 'status': 'retained',
                              'summary': 'Retained fixture history. ' * 4096}
                    database.execute('INSERT INTO development_records(kind,id,record_json) VALUES(?,?,?)',
                                     ('notification', record['id'], json.dumps(record)))
                if args.idle_cpu:
                    # Match a populated project's retained event volume, not an
                    # empty new-project idle screen. These events admit no work.
                    for index in range(200):
                        database.execute("INSERT INTO session_events(schema_version,type,task_id,body) VALUES(1,'fixture.retained',NULL,?)",
                                         (json.dumps({'summary':'Retained inert event. '*2000}),))

        def seed_coordinator_usage():
            # Inert completed-call fixture, not execution/settlement evidence.
            turn = 'fixture-usage-call'
            attempt = 'd' * 32
            directory = state / 'session/coordinator' / turn / 'call-v1'
            evidence = directory / 'attempts' / attempt
            evidence.mkdir(parents=True)
            records = {
                directory / 'operation.json': {'schema_version': 2, 'project_key': key,
                    'operation_id': turn, 'packet_sha256': 'e' * 64,
                    'provider': 'fixture-provider', 'model': 'fixture-model'},
                directory / 'active-host-attempt.json': {'attempt_id': attempt, 'unit': 'fixture-no-process.service'},
                evidence / 'intent.json': {'attempt_id': attempt, 'unit': 'fixture-no-process.service',
                    'project_key': key, 'operation_id': turn, 'packet_sha256': 'e' * 64},
                evidence / 'runtime-result.json': {'model_identity': 'SDK route fixture',
                    'requested_thinking_level': 'high', 'private': 'PRIVATE FIXTURE FIELD',
                    'usage': {'input': 1234, 'output': 56, 'cacheRead': 200, 'cacheWrite': 0,
                              'cost': 0.0123, 'context_tokens': 1490, 'context_window': 4096}},
            }
            for path, record in records.items():
                path.write_text(json.dumps(record))
            with sqlite3.connect(state / 'state.sqlite') as database:
                database.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status) VALUES(?,1,?,?,1,'completed')",
                                 (turn, key, 'fixture-usage-message'))
                database.execute("INSERT INTO dialogue_inputs(turn_id,subject_id) VALUES(?,'discussion')", (turn,))

        def inspect_coordinator_usage():
            with socket.socket(socket.AF_UNIX) as peer:
                peer.settimeout(5)
                peer.connect(str(ipc))
                stream = peer.makefile('rwb', buffering=0)
                def send(kind, body):
                    stream.write(json.dumps({'v': 1, 'kind': kind, 'request_id': kind, 'body': body}).encode() + b'\n')
                    return json.loads(stream.readline())
                owner = json.loads((state / 'supervision/owner.json').read_text())
                assert send('hello', {'protocol': {'major': 1, 'minor': 10}, 'release_id': owner['release_id'],
                                     'project_key': key, 'client_id': 'fixture-reader', 'channel': 'command'})['body']['accepted']
                result = send('conversation.activity', {'turn_id': 'fixture-usage-call'})
                assert result['kind'] == 'conversation.activity_result', result
                assert result['body']['runtime']['usage']['input'] == 1234
                assert 'PRIVATE' not in json.dumps(result)
                assert send('conversation.activity', {'turn_id': 'foreign-turn'})['kind'] == 'error'

        def prove_old_snapshot_closes():
            with socket.socket(socket.AF_UNIX) as peer:
                peer.settimeout(5)
                peer.connect(str(ipc))
                stream = peer.makefile('rwb', buffering=0)
                def send(kind, body):
                    stream.write(json.dumps({'v': 1, 'kind': kind, 'request_id': kind, 'body': body}).encode() + b'\n')
                    return stream.readline()
                hello = send('hello', {'protocol': {'major': 1, 'minor': 10}, 'release_id': releases[0],
                                      'project_key': key, 'client_id': 'fixture-reader', 'channel': 'events'})
                assert json.loads(hello)['body']['accepted']
                assert json.loads(send('session.status', {}))['body']['project_key'] == key
                assert send('snapshot', {}) == b'', 'Previous binary did not reproduce its oversized-snapshot closure'

        def inspect_populated_snapshot(release):
            with socket.socket(socket.AF_UNIX) as peer:
                peer.settimeout(5)
                peer.connect(str(ipc))
                stream = peer.makefile('rwb', buffering=0)
                def send(kind, body):
                    stream.write(json.dumps({'v': 1, 'kind': kind, 'request_id': kind, 'body': body}).encode() + b'\n')
                    return stream.readline(9 * 1024 * 1024)
                hello = send('hello', {'protocol': {'major': 1, 'minor': 10}, 'release_id': release,
                                      'project_key': key, 'client_id': 'fixture-reader', 'channel': 'events'})
                assert json.loads(hello)['body']['accepted']
                raw = send('snapshot', {})
                assert len(raw) > 1024 * 1024, 'Populated snapshot was truncated instead of delivered'
                result = json.loads(raw)
                assert result['kind'] == 'snapshot'
                development = result['body']['conversation']['development']
                retained = [r for r in development['notifications'] if r.get('id', '').startswith('fixture-history-')]
                assert len(retained) == 8
                assert all(r['summary'] == 'Retained fixture history. ' * 4096 for r in retained)
                if catalog:
                    assert len(development['providers']['discovery']['models']) == len(catalog), 'Catalog entries were lost'
                if args.blocked_send:
                    chat = result['body']['conversation']['chat']
                    assert chat['status'] in ('available','recoverable'), 'Retired binding still blocks the reopened conversation'
                    assert chat['profile']['model'] == selected_model

        def unit_property(name):
            return run(['systemctl', '--user', 'show', '--property=' + name, '--value', unit]).strip()

        def replace_install():
            if args.portable:
                next_link = launcher.with_name('waywr.next')
                next_link.symlink_to(release_root(releases[1]) / 'bin/waywr')
                os.replace(next_link, launcher)
                shutil.rmtree(root / releases[0])
                return
            run([binaries / 'waywr', 'bundle-install', '--bundle', root / releases[1],
                 '--prefix', prefix, '--select-default'])
            shutil.rmtree(prefix / 'releases' / releases[0])
            shutil.rmtree(root / releases[0])

        def require_attached_ui_preserved():
            pid = unit_property('MainPID')
            result = subprocess.run([str(launcher), '--ipc-dir', str(root / 'run')], cwd=project, env=env,
                                    capture_output=True, text=True, timeout=20)
            assert result.returncode != 0, 'Replacement interrupted busy service'
            assert 'already attached' in result.stderr, result.stderr
            assert unit_property('MainPID') == pid, 'Busy service owner changed'

        def open_and_detach(expected_draft=None, while_open=None):
            master, slave = pty.openpty()
            rows, columns = (24, 80) if args.provider_navigation else (32, 120)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', rows, columns, 0, 0))
            # Retired in-memory bindings remain inspectable by an explicit IPC
            # client. Normal startup now renews obsolete sessions automatically;
            # test-session-renewal.py owns that ordinary entry-point behavior.
            ui_args = [str(launcher), '--ipc-dir', str(root / 'run')]
            if args.retired_profiles and not args.systemd_handover:
                owner = json.loads((state / 'supervision/owner.json').read_text())
                ui_args = [str(launcher), '--socket', str(ipc), '--project-key', key,
                           '--release-id', owner['release_id'], '--outbox-dir', str(state / 'session/outbox')]
            ui = subprocess.Popen(ui_args, cwd=project, env=env,
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
                if args.coordinator_usage:
                    deadline = time.monotonic() + 10
                    while '$0.0123' not in terminal_text(screen, rows, columns) and time.monotonic() < deadline:
                        if select.select([master], [], [], .1)[0]:
                            screen.extend(os.read(master, 65536))
                    visible = terminal_text(screen, rows, columns)
                    assert '$0.0123' in visible and '1234' in visible and '2606' in visible, visible
                    assert 'fixture-provider/fixture-model' in visible
                    inspect_coordinator_usage()
                if args.idle_cpu and expected_draft is None:
                    def cpu_seconds(pid):
                        fields = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')',1)[1].split()
                        return (int(fields[11]) + int(fields[12])) / os.sysconf('SC_CLK_TCK')
                    assert service is not None, '--idle-cpu uses the direct isolated service'
                    cpu_before = [cpu_seconds(ui.pid),cpu_seconds(service.pid)]
                    started = time.monotonic()
                    while time.monotonic() - started < 3:
                        if select.select([master], [], [], .1)[0]:
                            screen.extend(os.read(master,65536))
                    elapsed = time.monotonic() - started
                    cpu = [(cpu_seconds(pid)-before)/elapsed for pid,before in zip([ui.pid,service.pid],cpu_before)]
                    print(json.dumps({'idle_ui_cpu_percent':round(cpu[0]*100,1),'idle_service_cpu_percent':round(cpu[1]*100,1)}),flush=True)
                    assert all(value < .20 for value in cpu), 'Idle application consumes more than 20% of a CPU'
                if args.retired_profiles:
                    os.write(master, b'\x1b[13~')  # F3: setup must be reachable.
                    def setup_visible():
                        # Ratatui may position each word separately in a diff frame.
                        text = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b' ', screen)
                        return re.search(rb'Provider\s+setup\s+required', text) is not None
                    deadline = time.monotonic() + 10
                    while not setup_visible() and time.monotonic() < deadline and not (args.activate_profile and expected_draft is not None):
                        if select.select([master], [], [], .1)[0]:
                            screen.extend(os.read(master, 65536))
                    if not (args.activate_profile and expected_draft is not None):
                        assert setup_visible(), ('F3 setup was not reachable: ' + screen.decode(errors='replace')[-1500:])
                if args.provider_navigation and expected_draft is None:
                    if not args.retired_profiles:
                        os.write(master, b'\x1b[13~')
                    # End/Up must move the result cursor, not scroll a truncated preview.
                    os.write(master, b'f\x1b[F\x1b[A')
                    deadline = time.monotonic() + 10
                    while selected_model.encode() not in screen and time.monotonic() < deadline:
                        if select.select([master], [], [], .1)[0]:
                            screen.extend(os.read(master, 65536))
                    assert selected_model.encode() in screen, 'Catalog tail model was not visible in the picker'
                    # Accept model, move to Thinking, choose its last advertised level, save.
                    os.write(master, b'\r\x1b[B\r\x1b[F\rs')
                    profile_path = project / '.waywr/coordinator.json'
                    def profile_saved():
                        profile = json.loads(profile_path.read_text())['profiles']['default']
                        return (profile['provider'] == selected_provider and profile['model'] == selected_model
                                and profile.get('options', {}).get('reasoning') == selected_thinking)
                    wait_for(profile_saved, 'The explicit picker selection/save did not reach project preferences')
                    before[profile_path] = profile_path.read_bytes()
                    if args.activate_profile:
                        def wait_for_text(pattern, message):
                            deadline = time.monotonic() + 10
                            while time.monotonic() < deadline:
                                text = terminal_text(screen, rows, columns).encode()
                                if re.search(pattern, text):
                                    return
                                if select.select([master], [], [], .1)[0]:
                                    screen.extend(os.read(master, 65536))
                            raise AssertionError(message + ': ' + screen.decode(errors='replace')[-1200:])
                        wait_for_text(rb'Preferences\s+saved', 'The form did not acknowledge its durable save')
                        if args.blocked_send:
                            os.write(master, b'\x1b')
                            time.sleep(.2)
                            os.write(master, b'Review this saved model\r')
                        else:
                            os.write(master, b'a')
                        wait_for_text(rb'Activate\s+saved\s+model', 'Profiles did not open its activation review')
                        os.write(master, b'\x1b[C\r')
                        def activation_applied():
                            with sqlite3.connect(state / 'state.sqlite') as database:
                                row = database.execute("SELECT value FROM session_metadata WHERE name='provider_coordinator'").fetchone()
                                profile = json.loads(row[0])['profile']
                                return (profile['provider'] == selected_provider and profile['model'] == selected_model
                                        and profile.get('options', {}).get('reasoning') == selected_thinking)
                        wait_for(activation_applied, 'The explicit activation review did not change the coordinator binding')
                        assert json.loads(run([launcher, 'session', 'status']))['session']['phase'] == 'paused', 'Activation resumed a paused session'
                        if args.blocked_send:
                            wait_for(lambda: saved_draft() == 'Review this saved model', 'Blocked Send lost or submitted the draft')
                            os.write(master,b'\x03')  # Ctrl-C clears, not process interruption.
                            wait_for(lambda: saved_draft() == '', 'Ctrl-C did not clear the durable draft')
                            os.write(master,b'Clear with Escape')
                            wait_for(lambda: saved_draft() == 'Clear with Escape', 'Clear-key fixture input was not saved')
                            os.write(master,b'\x1b')
                            wait_for(lambda: saved_draft() == '', 'Escape did not clear the durable draft')
                if args.provider_navigation and expected_draft is not None:
                    profile = json.loads((project / '.waywr/coordinator.json').read_text())['profiles']['default']
                    assert profile['options']['reasoning'] == selected_thinking, 'Thinking choice did not survive reopening'
                if (args.retired_profiles or (args.provider_navigation and expected_draft is None)) and not (args.blocked_send and expected_draft is None):
                    os.write(master, b'\x1b')
                    time.sleep(.2)
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
                    require_attached_ui_preserved()
                open_and_detach(while_open=replace_while_attached)
                old_pid = unit_property('MainPID')
                old_owner = json.loads((state / 'supervision/owner.json').read_text())
                if args.populated_history:
                    seed_history()
                    if args.previous_binaries:
                        prove_old_snapshot_closes()
                pending = state / 'session/outbox/fixture-pending.json'
                pending.parent.mkdir(exist_ok=True)
                pending.write_text('{}')
                try:
                    open_and_detach(draft)
                    assert unit_property('MainPID') == old_pid, 'Pending delivery owner changed'
                finally:
                    pending.unlink(missing_ok=True)  # Reattached UI may reconcile its outbox.
                uncertain = state / 'session/coordinator/fixture-call/attempt/launch.json'
                uncertain.parent.mkdir(parents=True)
                uncertain.write_text('{}')
                try:
                    open_and_detach(draft)
                    assert unit_property('MainPID') == old_pid, 'Unsettled owner changed'
                    assert uncertain.read_text() == '{}', 'Uncertain launch evidence changed'
                finally:
                    shutil.rmtree(state / 'session/coordinator/fixture-call')
                open_and_detach(draft)
                if args.populated_history:
                    inspect_populated_snapshot(releases[1])
                assert unit_property('MainPID') != old_pid, 'Old service was not handed over'
                owner = json.loads((state / 'supervision/owner.json').read_text())
                assert owner['generation'] > old_owner['generation']
                assert owner['release_id'] == releases[1]
                assert all(path.read_bytes() == value for path, value in before.items())
                assert saved_draft() == draft
                assert not list(state.rglob('spawn-committed.json'))
                print(json.dumps({'real_systemd_bootstrap_and_idle_handover': True,
                                  'portable_bundle_symlink': args.portable,
                                  'populated_history_snapshot_delivered': args.populated_history,
                                  'upgraded_from_snapshot_closing_service': bool(args.previous_binaries),
                                  'catalog_navigation_saved_and_reopened': args.provider_navigation,
                                  'thinking_level_saved_and_reopened': args.provider_navigation,
                                  'existing_owner_attached_then_idle_handover': True,
                                  'old_build_removed': True, 'model_calls': 0}))
                return
            start_service(releases[0])
            if args.populated_history or args.coordinator_usage:
                stop_service()
                if args.populated_history:
                    seed_history()
                if args.coordinator_usage:
                    seed_coordinator_usage()
                start_service(releases[0])
            if args.activate_profile:
                run([launcher, 'session', 'stop'])
                wait_for(lambda: json.loads(run([launcher, 'session', 'status']))['session']['phase'] == 'paused',
                         'Activation fixture did not reach its explicit pause')
                # Exit drains/stops this fixture service. Bring the same saved
                # pause back with the direct host before testing UI activation.
                stop_service()
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
            if args.activate_profile:
                with sqlite3.connect(state / 'state.sqlite') as database:
                    row = database.execute("SELECT value FROM session_metadata WHERE name='provider_coordinator'").fetchone()
                    profile = json.loads(row[0])['profile']
                    assert (profile['provider'], profile['model'], profile['options']['reasoning']) == (
                        selected_provider, selected_model, selected_thinking), 'Activated binding did not survive reopening'
            if args.populated_history:
                inspect_populated_snapshot(releases[1])
            status = json.loads(run([launcher, 'session', 'status']))
            assert status['session']['phase'] == 'paused', 'Opening resumed execution'
            run([launcher, 'init', project])  # Repeated setup must preserve saved state too.
            assert all(path.read_bytes() == contents for path, contents in before.items()), 'Project settings/legacy metadata changed'
            assert not list(state.rglob('spawn-committed.json'))
            assert not list(state.rglob('ollama-send-intent.json'))
            print(json.dumps({'reopened_with_old_build_removed': True,
                              'catalog_navigation_saved_and_reopened': args.provider_navigation,
                              'thinking_level_saved_and_reopened': args.provider_navigation,
                              'model_activated_without_resuming': args.activate_profile,
                              'coordinator_usage_visible_and_reopened': args.coordinator_usage,
                              'blocked_send_setup_and_clear_controls': args.blocked_send,
                              'populated_history_snapshot_delivered': args.populated_history,
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
