#!/usr/bin/env python3
"""Isolated PTY smoke check for the scripted Rust UI. Never contacts services."""
import argparse
import fcntl
import json
import os
import pty
import select
import signal
import struct
import subprocess
import termios
import time
from pathlib import Path


def window(fd, width, height):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))


def drain(fd, duration, output):
    end = time.monotonic() + duration
    while time.monotonic() < end:
        readable, _, _ = select.select([fd], [], [], min(0.04, max(0, end - time.monotonic())))
        if readable:
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            output.extend(data)


def session(binary, data_dir, capture, first=False, compose=None, signal_stop=False, size=(80, 24)):
    binary = binary.resolve()
    master, slave = pty.openpty()
    window(slave, *size)
    prior = termios.tcgetattr(slave)
    proc = subprocess.Popen(
        [str(binary), "--scenario", "decision-log", "--data-dir", str(data_dir)],
        stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
        close_fds=True, cwd="/",
    )
    output = bytearray()
    try:
        drain(master, 0.3, output)
        idle_before = len(output)
        drain(master, 0.7, output)  # Cross the 550 ms blink interval plus the 250 ms input poll.
        idle_bytes = len(output) - idle_before
        if first:
            paste = b"Clarify Decision Log alternatives\nKeep approved free in-place editing."
            os.write(master, b"\x1b[200~" + paste + b"\x1b[201~")
            drain(master, 0.1, output)
            os.write(master, b"\r")
            drain(master, 0.45, output)
            window(slave, 120, 36)
            proc.send_signal(signal.SIGWINCH)
            drain(master, 0.2, output)
        if compose:
            for chunk in compose:
                os.write(master, chunk)
                drain(master, 0.08, output)
            drain(master, 0.45, output)
        if signal_stop:
            proc.send_signal(signal.SIGTERM)
        else:
            os.write(master, b"\x11")  # Ctrl-Q: open detach confirmation.
            drain(master, 0.1, output)
            os.write(master, b"\t\r")  # Focus Detach, then activate it.
            drain(master, 0.2, output)
        try:
            code = proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.terminate()  # Only this test-owned UI process.
            code = proc.wait(timeout=3)
            raise AssertionError("prototype did not restore/exit after the requested test stop")
        drain(master, 0.1, output)
        assert code == 0, f"prototype exited {code}"
        assert termios.tcgetattr(slave) == prior, "PTY raw mode was not restored"
        assert b"SIMULATED" in output, "persistent simulation label was not rendered"
        assert b"Project Decision Log" in output, "project header was not rendered"
        assert b"\x1b[>1u" in output, "app did not request modified-key reporting"
        assert b"\x1b[<1u" in output, "app did not restore prior keyboard protocol"
        capture.write_bytes(output)
        return {"exit": code, "bytes": len(output), "terminal_restored": True,
                "idle_bytes_after_initial_frame": idle_bytes}
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=3)
        os.close(slave)
        os.close(master)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    args = parser.parse_args()
    args.evidence_dir = args.evidence_dir.resolve()
    args.evidence_dir.mkdir(parents=True, exist_ok=True)
    data = args.evidence_dir / "evaluation"
    first = session(args.binary, data, args.evidence_dir / "first.pty", first=True)
    saved = json.loads((data / "scripted-journal.json").read_text())
    assert saved["first_request"] == (
        "Clarify Decision Log alternatives\nKeep approved free in-place editing."
    ), "bracketed paste was changed or submitted early"
    assert saved["coordinator_calls"] == 1, "first controller action duplicated"
    second = session(args.binary, data, args.evidence_dir / "reopen.pty")
    reopened = json.loads((data / "scripted-journal.json").read_text())
    assert reopened["coordinator_calls"] == 1, "reopen repeated the scripted call"
    assert reopened["first_request"] == saved["first_request"], "reopen lost the request"
    assert 0 < first["idle_bytes_after_initial_frame"] < 4096, f"cursor blink must redraw briefly without a continuous repaint: {first['idle_bytes_after_initial_frame']} bytes"
    fallback_dir = args.evidence_dir / "multiline-fallback"
    fallback = session(args.binary, fallback_dir, args.evidence_dir / "multiline-fallback.pty",
        compose=[b"line one", b"\x1bOQ", b"\r", b"line two", b"\t", b"\r"])
    fallback_state = json.loads((fallback_dir / "scripted-journal.json").read_text())
    assert fallback_state["first_request"] == "line one\nline two", "F2/Tab/Send multiline fallback failed"
    shifted_dir = args.evidence_dir / "shift-enter"
    shifted = session(args.binary, shifted_dir, args.evidence_dir / "shift-enter.pty",
        compose=[b"line one", b"\x1b[13;2u", b"line two", b"\r"])
    shifted_state = json.loads((shifted_dir / "scripted-journal.json").read_text())
    assert shifted_state["first_request"] == "line one\nline two", "CSI-u Shift+Enter did not insert a newline"
    literal_dir = args.evidence_dir / "literal-command"
    literal = session(args.binary, literal_dir, args.evidence_dir / "literal-command.pty",
        compose=[b"\x1b[200~/approve\x1b[201~", b"\r"])
    literal_state = json.loads((literal_dir / "scripted-journal.json").read_text())
    assert literal_state["first_request"] == "/approve", "pasted slash command was executed or altered"
    slash_settings = session(args.binary, args.evidence_dir / "slash-settings",
        args.evidence_dir / "slash-settings.pty", compose=[b"/settings", b"\r"])
    assert b"fallback" in (args.evidence_dir / "slash-settings.pty").read_bytes(), "slash menu Enter did not open settings"
    slash_tasks = session(args.binary, args.evidence_dir / "slash-tasks",
        args.evidence_dir / "slash-tasks.pty", compose=[b"/tasks", b"\r"])
    assert b"Workers" in (args.evidence_dir / "slash-tasks.pty").read_bytes(), "slash menu Enter did not open tasks"
    draft_dir = args.evidence_dir / "draft-reopen"
    draft = session(args.binary, draft_dir, args.evidence_dir / "draft-first.pty",
        compose=[b"unsent line", b"\x1b[13;2u", b"still drafting"])
    draft_saved = json.loads((draft_dir / "scripted-journal.json").read_text())
    assert draft_saved["drafts"]["coordinator"]["text"] == "unsent line\nstill drafting"
    draft_reopen = session(args.binary, draft_dir, args.evidence_dir / "draft-reopen.pty")
    narrow = session(args.binary, args.evidence_dir / "narrow",
        args.evidence_dir / "narrow.pty", size=(60, 18))
    wide = session(args.binary, args.evidence_dir / "wide",
        args.evidence_dir / "wide.pty", size=(160, 48))
    signalled = session(args.binary, args.evidence_dir / "signalled",
        args.evidence_dir / "signalled.pty", signal_stop=True)
    summary = {
        "mode": "SIMULATED",
        "pty_first": first,
        "pty_reopen": second,
        "multiline_fallback": fallback,
        "shift_enter_csi_u": shifted,
        "literal_pasted_command": literal,
        "slash_settings": slash_settings,
        "slash_tasks": slash_tasks,
        "multiline_draft_saved": draft,
        "multiline_draft_reopen": draft_reopen,
        "narrow_60x18": narrow,
        "wide_160x48": wide,
        "sigterm_cleanup": signalled,
        "paste_literal": True,
        "reopen_without_duplicate": True,
        "workers_or_services_launched": 0,
    }
    (args.evidence_dir / "pty-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
