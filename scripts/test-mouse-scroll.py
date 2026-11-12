#!/usr/bin/env python3
"""Check actual SGR wheel input/capture/restoration in the isolated scripted TUI.

No service, model, account, installation or operator project is used.
"""
import argparse
import errno
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time
import fcntl


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/debug/waywr-ui-prototype'))
    args = parser.parse_args()
    binary = args.binary.resolve()
    with tempfile.TemporaryDirectory(prefix='waywr-wheel-') as directory:
        root = Path(directory)
        state = root / 'scripted-journal.json'

        def read_state():
            return json.loads(state.read_text()) if state.exists() else {}

        def open_ui(action=None):
            master, slave = pty.openpty()
            before = termios.tcgetattr(slave)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
            child = subprocess.Popen([str(binary), '--scenario', 'decision-log', '--data-dir', str(root)],
                env={'PATH': os.environ['PATH'], 'HOME': str(root), 'TERM': 'xterm-256color', 'LANG': 'C.UTF-8'},
                stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            output = bytearray()

            def wait_for(check, label):
                until = time.monotonic() + 5
                while not check() and time.monotonic() < until:
                    assert child.poll() is None, label + ': UI exited early'
                    if select.select([master], [], [], .05)[0]:
                        output.extend(os.read(master, 65536))
                assert check(), label

            try:
                wait_for(lambda: b'[Send]' in output, 'TUI did not open')
                assert b'\x1b[?1006h' in output, 'SGR mouse capture was not enabled'
                if action:
                    action(master, wait_for)
                os.write(master, b'\x11')
                assert child.wait(timeout=2) == 0
                while select.select([master], [], [], .05)[0]:
                    try:
                        chunk = os.read(master, 65536)
                        if not chunk:
                            break
                        output.extend(chunk)
                    except OSError as error:
                        if error.errno == errno.EIO:
                            break
                        raise
                assert b'\x1b[?1006l' in output, 'Mouse capture was not disabled on leave'
                assert termios.tcgetattr(master) == before, 'Terminal modes were not restored'
            finally:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)
                os.close(master)

        open_ui()
        fixture = read_state()
        fixture['messages'] = [{'seq': n + 1, 'speaker': 'Coordinator', 'text': f'Retained wheel fixture row {n:02}'} for n in range(50)]
        fixture['next_seq'] = 51
        fixture['effects'] = 'off'
        fixture['drafts']['coordinator'] = {'text': 'Unsent wheel fixture', 'row': 0, 'col': 6, 'multiline': False, 'literal_paste': False}
        state.write_text(json.dumps(fixture))

        def scroll_up(master, wait_for):
            os.write(master, b'\x1b[<64;3;5M')
            wait_for(lambda: read_state().get('scroll_back') == 3, 'Actual wheel input did not scroll three lines')

        open_ui(scroll_up)
        assert read_state()['drafts']['coordinator']['text'] == 'Unsent wheel fixture'

        def scroll_down(master, wait_for):
            os.write(master, b'\x1b[<65;3;5M')
            wait_for(lambda: read_state().get('scroll_back') == 0, 'Wheel down did not return to latest output')

        open_ui(scroll_down)
        assert read_state()['drafts']['coordinator']['text'] == 'Unsent wheel fixture'
        print(json.dumps({'actual_sgr_wheel_scroll': True, 'draft_preserved': True,
                          'terminal_capture_restored': True, 'model_calls': 0}))


if __name__ == '__main__':
    main()
