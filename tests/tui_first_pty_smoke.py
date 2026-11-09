"""Installed fake-only Waywr TUI attach/detach smoke under an isolated PTY."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time


def pty_session(exe, project, env, args, output):
    pid, master = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 120, 0, 0))
        os.chdir(project)
        os.execve(str(exe), [str(exe), *args], env)
    data = bytearray()
    sent = False
    confirmed = False
    deadline = time.monotonic() + 15
    status = None
    try:
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], .2)
            if ready:
                try: data.extend(os.read(master, 65536))
                except OSError: pass
            if b"Wayward Wrangler" in data and not sent:
                time.sleep(.3)
                os.write(master, b"\x11")  # documented Ctrl-Q detach
                sent = True
            if sent and b"Detach interface?" in data and not confirmed:
                os.write(master, b"\t\r")  # select explicit [Detach], then activate
                confirmed = True
            finished, status = os.waitpid(pid, os.WNOHANG)
            if finished: break
        else:
            os.kill(pid, signal.SIGTERM)
            _, status = os.waitpid(pid, 0)
            raise AssertionError("Installed TUI did not detach within 15 seconds")
    finally:
        output.write_bytes(data)
        os.close(master)
    assert sent and confirmed and os.waitstatus_to_exitcode(status) == 0
    assert b"\x1b[?1049h" in data and b"\x1b[?1049l" in data, "Alternate screen was not restored"


def main(root, release):
    assert not root.exists()
    root.mkdir(parents=True)
    runtime = Path(tempfile.mkdtemp(prefix="ww-s4-pty-"))
    for name in ("home", "runtime", "state"): (runtime / name).mkdir()
    env = {**os.environ, "HOME": str(runtime / "home"),
           "XDG_RUNTIME_DIR": str(runtime / "runtime"),
           "XDG_STATE_HOME": str(runtime / "state"),
           "HERDR_ENV": "1", "HERDR_SOCKET_PATH": str(runtime / "stale.sock"),
           "PATH": "/usr/bin:/bin", "TERM": "xterm-256color"}
    env.pop("PYTHONPATH", None)
    exe = release / "bin/waywr"
    project = root / "project"; project.mkdir()
    subprocess.run([str(exe), "init", "."], cwd=project, env=env, check=True,
                   stdout=(root / "init.stdout").open("wb"))
    key = next(line.split('"')[1] for line in
               (project / ".wayward-wrangler/project.toml").read_text().splitlines()
               if line.startswith("project_key"))
    unit = f"waywr-{key}.service"
    try:
        pty_session(exe, project, env, [], root / "launch.pty")
        pty_session(exe, project, env, ["resume"], root / "resume.pty")
        assert not (runtime / "stale.sock").exists()
        assert not (project / ".wayward-wrangler/herdr.json").exists()
        assert not (project / ".wayward-wrangler/state/views").exists()
        (root / "summary.json").write_text(json.dumps({"passed": True,
            "launch_and_resume": "Ctrl-Q detach restored alternate screen",
            "herdr_contact": False, "unit": unit,
            "runtime": str(runtime)}, indent=2) + "\n")
    finally:
        subprocess.run(["systemctl", "--user", "stop", unit], capture_output=True, text=True)
        subprocess.run(["systemctl", "--user", "reset-failed", unit], capture_output=True, text=True)
        active = subprocess.run(["systemctl", "--user", "is-active", unit],
                                capture_output=True, text=True).stdout.strip()
        (root / "cleanup.json").write_text(json.dumps({"unit": unit,
            "active_after_cleanup": active}, indent=2) + "\n")
        assert active != "active"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--release", type=Path, required=True)
    options = parser.parse_args()
    main(options.evidence_root.resolve(), options.release.resolve())
