#!/usr/bin/env python3
"""Project lifecycle and input controls through real isolated systemd/IPC/TUI.

No operator project, installation, credentials or model account is used. Held
operations use sleep or local HTTP/SDK fixtures; only fixture units are stopped.
"""
import argparse
import fcntl
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import pty
import re
import runpy
import select
import shutil
import signal
import sqlite3
import struct
import subprocess
import tempfile
import termios
import time
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, default=Path('target/release'))
    parser.add_argument('--busy', action='store_true', help='Reconnect to a healthy active owner after an update')
    parser.add_argument('--controls', action='store_true', help='Interrupt a real held SDK stream, leave while stopping, reopen and leave disconnected')
    parser.add_argument('--steering', action='store_true', help='Steer the active SDK, edit a queued follow-up and verify separate results through the real TUI/service')
    args = parser.parse_args()
    binaries = args.binaries.resolve()
    source = Path(__file__).resolve().parents[1]
    terminal_text = runpy.run_path(str(source / 'scripts/test-project-reopen.py'))['terminal_text']
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
        def open_ui(launcher, while_open=None, expected_draft=None):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 120, 0, 0))
            ui = subprocess.Popen(list(map(str, [launcher, *cli])), cwd=project, env=env,
                                  stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            try:
                screen = bytearray()
                deadline = time.monotonic() + 15
                def composer_open():
                    if expected_draft is not None:
                        return expected_draft.encode() in screen
                    return any(text in screen for text in [b'Message the coordinator', b'Describe work'])
                while not composer_open() and time.monotonic() < deadline and ui.poll() is None:
                    if select.select([master], [], [], .1)[0]:
                        try: screen.extend(os.read(master, 65536))
                        except OSError: break
                assert composer_open(), re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', screen).decode(errors='replace')[:2500]
                def wait_for(check, label, seconds=15):
                    until = time.monotonic() + seconds
                    while not check() and time.monotonic() < until:
                        assert ui.poll() is None, 'UI closed while ' + label
                        if select.select([master], [], [], .05)[0]:
                            try: screen.extend(os.read(master, 65536))
                            except OSError: break
                    assert check(), label + ': ' + re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', screen).decode(errors='replace')[-1600:]
                if while_open:
                    while_open(master, wait_for, screen)
                started = time.monotonic()
                os.write(master, b'\x11')
                assert ui.wait(timeout=2) == 0
                assert time.monotonic() - started < 1, 'Leaving waited for service/agent settlement'
            finally:
                if ui.poll() is None:
                    ui.terminate()
                    ui.wait(timeout=10)
                os.close(master)

        server = None
        release_stream = threading.Event()
        held_runner = None
        try:
            if args.controls or args.steering:
                started = threading.Event()
                calls = []
                class HeldStream(BaseHTTPRequestHandler):
                    def log_message(self, *_): pass
                    def do_POST(self):
                        calls.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                        self.send_response(200)
                        self.send_header('Content-Type', 'text/event-stream')
                        self.end_headers()
                        chunk = {'id': 'held', 'object': 'chat.completion.chunk', 'created': 1,
                                 'model': 'model-outside-core', 'choices': [{'index': 0, 'delta': {'role': 'assistant', 'content': 'Held local fixture stream'}, 'finish_reason': None}]}
                        self.wfile.write(('data: ' + json.dumps(chunk) + '\n\n').encode())
                        self.wfile.flush()
                        started.set()
                        release_stream.wait(45)
                        if args.steering:
                            number=len(calls)
                            reply={"schema_version":3,"assistant_text":"Steered fixture reply" if number==2 else "Follow-up fixture reply" if number>=3 else "Provisional fixture reply","evidence_refs":[],"action":None}
                            chunk['choices']=[{'index':0,'delta':{'tool_calls':[{'index':0,'id':'submit-'+str(number),'type':'function','function':{'name':'submit_result','arguments':json.dumps(reply)}}]},'finish_reason':None}]
                            self.wfile.write(('data: '+json.dumps(chunk)+'\n\n').encode())
                            chunk['choices']=[{'index':0,'delta':{},'finish_reason':'tool_calls'}]
                            self.wfile.write(('data: '+json.dumps(chunk)+'\n\ndata: [DONE]\n\n').encode())
                            self.wfile.flush()
                server = ThreadingHTTPServer(('127.0.0.1', 0), HeldStream)
                server.daemon_threads = True
                threading.Thread(target=server.serve_forever, daemon=True).start()
                endpoint = f'http://127.0.0.1:{server.server_port}/v1'
                config = root / 'models.json'
                config.write_text(json.dumps({'providers': [{'id': 'held-fixture', 'api': 'openai-completions',
                    'base_url': endpoint, 'models': [{'id': 'model-outside-core', 'context_window': 32768}]}]}))
                path = project / '.waywr/coordinator.json'
                prefs = json.loads(path.read_text())
                profile = prefs['profiles']['default']
                profile.update(provider='held-fixture', model='model-outside-core', options={})
                profile['transport'] = {'entry': 'bundled', 'config': str(config),
                    'destination': {'provider': 'held-fixture', 'api': 'openai-completions', 'base_url': endpoint, 'auth_env': None}}
                path.write_text(json.dumps(prefs))
                if args.steering:
                    guidance='Use the revised\nfixture constraint'
                    follow_up='Explain it afterwards'
                    updated='Updated follow-up message'
                    def scalar(sql, values=()):
                        with sqlite3.connect(state / 'state.sqlite') as db:
                            return db.execute(sql, values).fetchone()[0]
                    def steer_and_queue(master, wait, screen):
                        os.write(master,b'Initial local fixture request\r')
                        wait(started.is_set,'SDK did not reach the held fixture')
                        wait(lambda:'Interrupt coordinator' in terminal_text(screen,32,120),'Active coordinator was not visible')
                        os.write(master,b'\x1b[200~'+guidance.encode()+b'\x1b[201~\x1b[A\x1b[B\r')
                        wait(lambda:scalar("SELECT count(*) FROM conversation_live_inputs WHERE status='pending'")==1,'Enter did not persist live guidance')
                        wait(lambda:bool(list(state.glob('session/coordinator/*/call-v*/attempts/*/live-input/pending/*.json'))),'Guidance was not published to the active attempt')
                        os.write(master,follow_up.encode()+b'\x1b\r')
                        wait(lambda:scalar("SELECT count(*) FROM conversation_turns WHERE status='queued'")==1,'Alt+Enter did not queue a follow-up')
                        # Running turn first, queued follow-up second; restore it
                        # to the editor and replace it before either can finish.
                        os.write(master,b'\x1b[18~')
                        wait(lambda:follow_up in terminal_text(screen,32,120),'Queued follow-up was not visible in F7')
                        os.write(master,b'\x1b[Be')
                        wait(lambda:scalar("SELECT count(*) FROM conversation_turns WHERE status='withdrawn'")==1,'Queue E did not withdraw for editing')
                        wait(lambda:json.loads((state/'session/draft.json').read_text()).get('draft',{}).get('text')==follow_up,'Queued text was not restored to the composer')
                        os.write(master,b'\x01'+updated.encode()+b'\x1b\r')
                        wait(lambda:scalar("SELECT count(*) FROM conversation_turns WHERE status='queued'")==1,'Edited follow-up was not requeued')
                        release_stream.set()
                        wait(lambda:scalar("SELECT count(*) FROM conversation_turns WHERE status='completed'")==2,'Steered turn and follow-up did not complete',seconds=25)
                        wait(lambda:scalar("SELECT count(*) FROM conversation_live_inputs WHERE status='applied'")==1,'Guidance was not marked applied')
                        assert len(calls)==3, 'Steering or queued editing caused duplicated model requests'
                        assert guidance in json.dumps(calls[1],ensure_ascii=False).replace('\\n','\n')
                        assert updated not in json.dumps(calls[1]), 'Follow-up entered the unfinished call'
                        assert updated in json.dumps(calls[2]), 'Updated follow-up never reached its own call'
                        assert 'Steered fixture reply' in json.dumps(calls[2]), 'Follow-up lost the earlier result'
                        assert follow_up not in json.dumps(calls[2]), 'Withdrawn follow-up leaked into the new context'
                    open_ui(old,steer_and_queue)
                    open_ui(new)
                    assert len(calls)==3,'Reopening replayed completed input'
                    print(json.dumps({'live_sdk_steering_through_real_tui_service_runner':True,'follow_up_waited_and_was_edited':True,
                                      'separate_completed_turns':2,'http_fixture_requests':3,'model_calls':0}))
                    return
                draft = 'Keep this unsent draft while stopping'
                def interrupt(master, wait, screen):
                    nonlocal held_runner
                    os.write(master, b'A bounded local fixture conversation\r')
                    wait(started.is_set, 'SDK did not reach the held HTTP endpoint')
                    wait(lambda: 'Interrupt coordinator' in terminal_text(screen, 32, 120), 'Interrupt action missing')
                    claims = list(state.glob('session/coordinator/*/call-v*/attempts/*/claim.json'))
                    assert len(claims) == 1
                    claim = json.loads(claims[0].read_text())
                    held_runner = claim['runner']['pid']
                    os.kill(held_runner, signal.SIGSTOP)  # Settlement cannot finish until this fixture resumes it.
                    os.write(master,b'Keep this guidance pending\rLater follow-up\x1b\r')
                    def input_queued():
                        with sqlite3.connect(state/'state.sqlite') as db:
                            return db.execute("SELECT count(*) FROM conversation_live_inputs WHERE status='pending'").fetchone()[0]==1 and db.execute("SELECT count(*) FROM conversation_turns WHERE status='queued'").fetchone()[0]==1
                    wait(input_queued,'Steering/follow-up inputs were not queued before interruption')
                    os.write(master, b'\x03' + draft.encode())
                    def interrupt_recorded():
                        with sqlite3.connect(state / 'state.sqlite') as db:
                            return db.execute('SELECT COUNT(*) FROM dialogue_inputs WHERE interrupt_command_id IS NOT NULL').fetchone()[0] == 1
                    wait(interrupt_recorded, 'Empty-input Ctrl-C did not interrupt through IPC')
                    assert not list(state.rglob('receipt.json')), 'Fixture settled before prompt-leave check'
                open_ui(old, interrupt)
                assert json.loads((state / 'session/draft.json').read_text()).get('draft', {}).get('text') == draft
                # Reattach while the same owner is still waiting for settlement.
                open_ui(new, expected_draft=draft)
                os.kill(held_runner, signal.SIGCONT)
                held_runner = None
                until = time.monotonic() + 15
                while not list(state.rglob('receipt.json')) and time.monotonic() < until:
                    time.sleep(.05)
                assert list(state.rglob('receipt.json')), 'Interrupted runner did not settle'
                with sqlite3.connect(state/'state.sqlite') as db:
                    until=time.monotonic()+5
                    while time.monotonic()<until and db.execute("SELECT count(*) FROM conversation_turns WHERE status='held'").fetchone()[0]!=1:
                        time.sleep(.05)
                    assert db.execute("SELECT count(*) FROM conversation_turns WHERE status='held'").fetchone()[0]==1
                    assert db.execute("SELECT count(*) FROM conversation_live_inputs WHERE status='held'").fetchone()[0]==1
                    assert db.execute("SELECT count(*) FROM conversation_messages WHERE text='Keep this guidance pending'").fetchone()[0]==0,'Unapplied guidance entered conversation context'
                def disconnect(master, wait, screen):
                    os.write(master, b'\x03/pause\r')  # Clear draft, explicitly pause; do not leave.
                    def paused():
                        with sqlite3.connect(state / 'state.sqlite') as db:
                            phase = json.loads(db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone()[0])
                            return phase['phase'] in ['exit-requested', 'draining', 'paused']
                    wait(paused, 'Pause did not stop new dispatch while keeping the UI open')
                    os.write(master, draft.encode())
                    run(['systemctl', '--user', 'stop', unit])
                    wait(lambda: any(text in terminal_text(screen, 32, 120).lower() for text in ['reconnecting', 'disconnected']), 'UI did not expose disconnection')
                open_ui(new, disconnect, expected_draft=draft)
                assert len(calls) == 1, 'Interrupt/reopen replayed the model request'
                print(json.dumps({'real_sdk_stream_interrupted': True, 'prompt_leave_while_stopping_and_disconnected': True,
                                  'draft_retained_and_busy_owner_reattached': True, 'pause_is_separate_from_leave': True,
                                  'unapplied_steering_and_follow_up_held_without_replay':True,
                                  'http_fixture_requests': 1, 'model_calls': 0}))
                return
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
            if held_runner is not None:
                os.kill(held_runner, signal.SIGCONT)
            release_stream.set()
            if server is not None:
                server.shutdown()
                server.server_close()
            subprocess.run(['systemctl', '--user', 'stop', unit], env=env, capture_output=True, timeout=20)
            units = subprocess.run(['systemctl', '--user', 'list-units', '--all', '--plain', '--no-legend', '--full',
                                    f'waywr-op-{key}-*.service'], env=env, capture_output=True, text=True, timeout=5)
            owned = [line.split()[0] for line in units.stdout.splitlines()
                     if re.fullmatch(f'waywr-op-{key}-[0-9a-f]{{16}}-[0-9a-f]{{32}}.service', line.split()[0])]
            subprocess.run(['systemctl', '--user', 'stop', operation_unit, *owned], env=env, capture_output=True, timeout=20)


if __name__ == '__main__':
    main()
