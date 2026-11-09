"""Installed fake-only gate navigation through the real TUI in an isolated PTY.

All mutations are keyboard actions. IPC is read only for waiting/assertions.
This fixture never touches an operator project or a shared Herdr endpoint.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import socket
import struct
import subprocess
import tempfile
import termios
import time
import uuid


def read_only(endpoint, key, release, kind, body):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(3)
        peer.connect(str(endpoint))
        stream = peer.makefile("rb")
        def request(name, payload):
            peer.sendall((json.dumps({"v": 1, "kind": name,
                                      "request_id": uuid.uuid4().hex, "body": payload}) + "\n").encode())
            return json.loads(stream.readline())
        assert request("hello", {"protocol": {"major": 1, "minor": 1},
                                  "project_key": key, "release_id": release,
                                  "channel": "command"})["body"]["accepted"] is True
        return request(kind, body)["body"]


def main(root, release):
    assert not root.exists(), "Use a fresh ignored evidence directory"
    root.mkdir(parents=True)
    private = Path(tempfile.mkdtemp(prefix="ww-discoverability-"))
    for name in ("home", "runtime", "state"):
        (private / name).mkdir()
    env = {**os.environ, "HOME": str(private / "home"),
           "XDG_RUNTIME_DIR": str(private / "runtime"),
           "XDG_STATE_HOME": str(private / "state"),
           "HERDR_ENV": "1", "HERDR_SOCKET_PATH": str(private / "missing-herdr.sock"),
           "PATH": "/usr/bin:/bin", "TERM": "xterm-256color"}
    env.pop("PYTHONPATH", None)
    project = root / "project"
    project.mkdir()
    exe = release / "bin/waywr"
    init = subprocess.run([str(exe), "init", "."], cwd=project, env=env,
                          capture_output=True, text=True, timeout=15)
    assert init.returncode == 0, init.stderr
    (root / "init.stdout").write_text(init.stdout)
    config = project / ".wayward-wrangler"
    profile = config / "workflow.toml"
    profile.write_text(profile.read_text().replace("seconds = 3.0", "seconds = 0.2"))
    assert 'provider = "fake"' in profile.read_text()
    key = next(line.split('"')[1] for line in (config / "project.toml").read_text().splitlines()
               if line.startswith("project_key"))
    endpoint = private / "runtime/waywr" / f"{key}.sock"
    unit = f"waywr-{key}.service"
    pid, master = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 120, 0, 0))
        os.chdir(project)
        os.execve(str(exe), [str(exe)], env)
    output = bytearray()
    child_reaped = False
    def drain():
        if select.select([master], [], [], .05)[0]:
            try: output.extend(os.read(master, 65536))
            except OSError: pass
    def wait(label, probe, seconds=35):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            drain()
            found = probe()
            if found: return found
            time.sleep(.05)
        raise AssertionError(f"Timed out at {label}; retained PTY and project evidence")
    def press(keys):
        os.write(master, keys)
        time.sleep(.12)
        drain()
    def status():
        try:
            rows = read_only(endpoint, key, release.name, "snapshot", {})["tasks"]
            return rows[0] if rows else None
        except (OSError, KeyError, AssertionError): return None
    def gate(kind):
        row = status()
        if not row: return None
        detail = read_only(endpoint, key, release.name, "task.get", {"task_id": row["task_id"]})
        return detail if isinstance(detail.get("pending"), dict) and detail["pending"]["kind"] == kind else None
    steps = []
    try:
        wait("title", lambda: b"Wayward Wrangler" in output, 20)
        press(b"Draft a small local decision log specification.\r")
        wait("scope action", lambda: b"Review scope" in output and status() and status()["status"] == "proposed")
        press(b"\r")  # empty composer opens the visible scope action
        wait("scope panel", lambda: b"Review scope \xc2\xb7 simulated workers" in output)
        assert status()["status"] == "proposed", "opening scope authorized execution"
        press(b"\t\r")  # explicit Authorize
        steps.append("visible scope action opened without authorization; separate Authorize accepted")
        wait("question action", lambda: gate("choice"), 60)
        time.sleep(2)  # bounded event/snapshot/detail refresh before pressing the visible action
        press(b"\r")
        wait("question panel", lambda: b"Your answer needed \xc2\xb7 simulated workflow" in output)
        assert gate("choice"), "opening question answered it"
        press(b"1\t\r")  # choose first listed option, then explicit Save answer
        steps.append("visible question action opened without answering; explicit choice saved")
        pending = wait("review action", lambda: gate("final-review"), 60)
        time.sleep(2)
        candidate_hash = pending["pending"]["artifact_sha256"]
        press(b"\r")
        wait("review panel", lambda: b"Final review \xc2\xb7 simulated workflow" in output)
        assert gate("final-review"), "opening review approved it"
        press(b"\r")  # Read is focused on opening the review panel
        time.sleep(1)  # the fake candidate is one page; an early return fails the approval gate
        drain()
        press(b"\x1b")  # back to review; separate Approve is now focused
        assert gate("final-review"), "reading candidate approved it"
        press(b"\r")
        wait("approved result", lambda: status() and status()["status"] == "approved", 45)
        time.sleep(.4)
        press(b"\r")
        time.sleep(.5)
        drain()
        steps.append("exact candidate read preceded explicit approval; completed task offered View result")
        press(b"\x1b\x11")  # close result, then open detach confirmation
        time.sleep(.5)
        drain()
        press(b"\t\r")
        def exited():
            result = os.waitpid(pid, os.WNOHANG)
            return result if result[0] else None
        finished = wait("TUI return", exited, 15)[1]
        child_reaped = True
        assert os.waitstatus_to_exitcode(finished) == 0
        assert b"\x1b[?1049l" in output, "terminal was not restored"
        assert not (private / "missing-herdr.sock").exists()
        state = config / "state"
        invocations = list(state.rglob("invocations.json"))
        summary = {"passed": True, "release": release.name, "project_key": key,
                   "candidate_sha256": candidate_hash, "task_id": status()["task_id"],
                   "steps": steps, "invocation_records": len(invocations),
                   "no_herdr_contact": True, "terminal_restored": True,
                   "unit": unit, "private_runtime": str(private)}
        (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    finally:
        (root / "interaction.pty").write_bytes(output)
        if not child_reaped:
            try: os.kill(pid, signal.SIGTERM)
            except ProcessLookupError: pass
            try: os.waitpid(pid, 0)
            except ChildProcessError: pass
        os.close(master)
        stopped = subprocess.run(["systemctl", "--user", "stop", unit], capture_output=True, text=True)
        subprocess.run(["systemctl", "--user", "reset-failed", unit], capture_output=True, text=True)
        active = subprocess.run(["systemctl", "--user", "is-active", unit], capture_output=True, text=True)
        (root / "cleanup.json").write_text(json.dumps({"unit": unit,
            "stop_code": stopped.returncode, "active_after_cleanup": active.stdout.strip()}, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--release", type=Path, required=True)
    opts = parser.parse_args()
    main(opts.evidence_root.resolve(), opts.release.resolve())
