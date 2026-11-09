"""Installed production UI, read-only offline IPC replay. No provider/service host.

Normal installed startup is covered separately. This peer only supplies saved
and synthetic presentation data; all mutations refuse and no runner exists.
"""
import argparse
import copy
import codecs
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import socket
import struct
import termios
import threading
import time

from session_fixture import Fixture, read_frame, send


class Screen:
    """Bounded test-only decoder for Crossterm's cursor/erase/SGR updates.

    Removing ANSI alone loses unchanged spaces in Ratatui's sparse redraws.
    This covers the output emitted here, not general terminal emulation.
    """
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.cells = [[" "] * width for _ in range(height)]
        self.row = self.col = 0
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.pending = ""

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        i = 0
        while i < len(self.pending):
            ch = self.pending[i]
            if ch == "\x1b":
                match = re.match(r"\x1b\[([0-?]*)([ -/]*)([@-~])", self.pending[i:])
                if not match:
                    if i + 1 == len(self.pending) or self.pending[i + 1] == "[":
                        break
                    i += 2
                    continue
                params, _, cmd = match.groups()
                values = [int(p) if p.isdigit() else 0 for p in params.split(";")]
                n = values[0] or 1
                if cmd in "Hf":
                    self.row = min(self.height - 1, n - 1)
                    self.col = min(self.width - 1, (values[1] or 1) - 1 if len(values) > 1 else 0)
                elif cmd == "A": self.row = max(0, self.row - n)
                elif cmd == "B": self.row = min(self.height - 1, self.row + n)
                elif cmd == "C": self.col = min(self.width - 1, self.col + n)
                elif cmd == "D": self.col = max(0, self.col - n)
                elif cmd == "G": self.col = min(self.width - 1, n - 1)
                elif cmd == "J" and values[0] in (2, 3): self.cells = [[" "] * self.width for _ in range(self.height)]
                elif cmd == "K":
                    start = 0 if values[0] in (1, 2) else self.col
                    end = self.col + 1 if values[0] == 1 else self.width
                    self.cells[self.row][start:end] = [" "] * (end - start)
                i += match.end()
                continue
            if ch == "\r": self.col = 0
            elif ch == "\n": self.row = min(self.height - 1, self.row + 1)
            elif ch.isprintable():
                self.cells[self.row][min(self.col, self.width - 1)] = ch
                self.col = min(self.width - 1, self.col + 1)
            i += 1
        self.pending = self.pending[i:]

    def text(self):
        return "\n".join("".join(row).rstrip() for row in self.cells).encode()


class Replay(Fixture):
    def __init__(self, path, release):
        super().__init__(path, key="offline-rendering", release=release)
        saved = (Path(__file__).parent / "fixtures/claude-conversation/saved-plan.md").read_text()
        self.body = {"cursor": 0, "state_revision": 0, "session": {"phase": "running"},
                     "tasks": [], "conversation": {
                         "messages": [{"seq": 1, "role": "user", "text": "Ordinary **literal** user text."},
                                      {"seq": 2, "role": "assistant", "text": saved}],
                         "turns": [], "questions": [], "actions": [],
                         "binding": {"provider": "claude", "model": "claude-sonnet-5-5", "generation": 1},
                         "budgets": {"conversation": {"limit": 3, "charged": 3, "remaining": 0},
                                     "consultation": {"limit": 0, "charged": 0, "remaining": 0}},
                         "chat": {"status": "exhausted", "authorized": True,
                                  "profile": {"mode": "mock-cli", "model": "claude-sonnet-5-5", "effort": "medium"}}}}
        self.refused = []

    def stream(self):
        with self.lock:
            self.body["conversation"]["provisional"] = {
                "turn_id": "offline-provisional", "provisional": True,
                "text": "**Streaming response** with *emphasis* and `**literal code**`."}
            self.events.append({"seq": len(self.events) + 1, "type": "conversation.delta", "body": {}})
            self.body["cursor"] = self.body["state_revision"] = len(self.events)

    def handle(self, peer):
        with peer:
            try:
                hello = read_frame(peer)
                valid = (hello["kind"] == "hello" and hello["body"]["project_key"] == self.key
                         and hello["body"]["release_id"] == self.release
                         and hello["body"]["protocol"] == {"major": 1, "minor": 3})
                send(peer, "hello_result", hello["request_id"], {"accepted": valid})
                if not valid:
                    return
                while True:
                    q = read_frame(peer)
                    if q["kind"] == "snapshot":
                        with self.lock:
                            value = copy.deepcopy(self.body)
                        send(peer, "snapshot", q["request_id"], value)
                    elif q["kind"] == "events_read":
                        with self.lock:
                            events = self.events[q["body"]["after_seq"]:][:64]
                        if not events:
                            time.sleep(.05)
                        send(peer, "events_result", q["request_id"], {
                            "events": events, "next_seq": q["body"]["after_seq"] + len(events),
                            "state_revision": self.body["state_revision"]})
                    else:
                        self.refused.append(q["kind"])
                        send(peer, "error", q["request_id"], {"code": "offline_read_only"})
            except (OSError, EOFError, ValueError, KeyError):
                return


def main(root, release):
    assert not root.exists()
    root.mkdir(parents=True)
    results = []
    for width, height in [(80, 24), (120, 60)]:
        directory = root / str(width)
        directory.mkdir()
        # Linux Unix socket paths are bounded; keep this transient path short.
        endpoint = Path("/tmp") / f"ww-render-{os.getpid()}-{width}.sock"
        assert not endpoint.exists()
        peer = Replay(endpoint, release.name)
        server = threading.Thread(target=peer.serve)
        server.start()
        pid = fd = None
        captures = []
        screen = None

        def launch():
            nonlocal pid, fd, screen
            screen = Screen(width, height)
            pid, fd = pty.fork()
            if pid == 0:
                fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
                env = {"HOME": str(directory), "TERM": "xterm-256color", "PATH": "/usr/bin:/bin"}
                exe = release / "bin/waywr"
                os.execve(exe, [str(exe), "--socket", str(endpoint), "--project-key", peer.key,
                               "--release-id", release.name, "--outbox-dir", str(directory / "session/outbox"),
                               "--title", "OFFLINE saved-response replay"], env)
            captures.append(bytearray())

        def drain(seconds=.1):
            end = time.monotonic() + seconds
            while time.monotonic() < end:
                if select.select([fd], [], [], .02)[0]:
                    try:
                        data = os.read(fd, 65536)
                        captures[-1].extend(data)
                        screen.feed(data)
                    except OSError:
                        break

        def plain():
            return screen.text()

        def wait(word):
            end = time.monotonic() + 8
            while time.monotonic() < end:
                drain()
                if word in plain():
                    return
            raise AssertionError(f"{width}: absent {word!r}")

        def detach():
            nonlocal pid, fd
            os.write(fd, b"\x11"); drain(.2)
            os.write(fd, b"\t\r")
            end = time.monotonic() + 8
            while time.monotonic() < end:
                drain()
                found, status = os.waitpid(pid, os.WNOHANG)
                if found:
                    assert os.waitstatus_to_exitcode(status) == 0
                    assert b"\x1b[?1049h" in captures[-1] and b"\x1b[?1049l" in captures[-1]
                    assert termios.tcgetattr(fd)[3] & termios.ICANON
                    os.close(fd); pid = fd = None
                    return
            raise AssertionError("Detach/terminal restoration failed")

        try:
            launch(); wait(b"MOCK CLI STREAM")
            os.write(fd, b"\x1b[5~" * 12); drain(.4)  # saved history remains inspectable
            wait(b"Goal:")
            assert b"**Goal:" not in plain() and b"**Check the toolchain" not in plain()
            peer.stream(); drain(.7)
            os.write(fd, b"\x1b[6~" * 12); wait(b"Streaming response")
            assert b"**Streaming response**" not in plain()
            assert b"**literal code**" in plain(), "Code markers must stay literal"
            draft = "draft **keep**\n/inspect literal"
            os.write(fd, b"\x1b[200~" + draft.encode() + b"\x1b[201~"); drain(.2)
            detach()
            stored = json.loads((directory / "session/draft.json").read_text())["draft"]
            assert stored["text"] == draft and stored["literal_paste"]
            launch(); wait(b"draft **keep**"); wait(b"/inspect literal")
            os.write(fd, b"\x1b[5~" * 12); wait(b"Goal:")
            detach()
            assert not peer.refused, "Replay must not emit mutation requests"
            results.append({"columns": width, "saved_response": True, "bold_markers_hidden": True,
                            "provisional_stream": True, "code_literal": True, "draft_restored": True,
                            "terminal_restored": True, "provider_calls": 0, "mutations": 0})
        finally:
            if pid:
                os.kill(pid, signal.SIGTERM); os.waitpid(pid, 0); os.close(fd)
            peer.close(); server.join(timeout=2)
            assert not server.is_alive()
            for index, data in enumerate(captures):
                (directory / f"session-{index}.pty").write_bytes(data)
            (directory / "last-screen.txt").write_bytes(screen.text())
            (root / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--root", type=Path, required=True)
    p.add_argument("--release", type=Path, required=True)
    args = p.parse_args()
    main(args.root.resolve(), args.release.resolve())
