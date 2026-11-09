"""Isolated Herdr foreground-client return-to-shell proof. No shared socket."""
import argparse
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time
import fcntl

from agent_workflow.herdr_backend import Herdr
from agent_workflow.storage import identity, alive


def wait_for(check, seconds=8):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        try:
            value = check()
        except (FileNotFoundError, ConnectionRefusedError):
            value = None
        if value:
            return value
        time.sleep(.05)
    raise AssertionError("isolated Herdr proof timed out")


def signal_owned(process_id, original, sig):
    if not alive(original):
        return
    observed = identity(process_id)
    assert all(observed[key] == original[key] for key in ("pid", "start", "boot"))
    fd = os.pidfd_open(process_id)
    try:
        signal.pidfd_send_signal(fd, sig)
    finally:
        os.close(fd)


def children(parent):
    raw = Path(f"/proc/{parent}/task/{parent}/children").read_text().strip()
    return [int(item) for item in raw.split()] if raw else []


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    for relative in ("home", "config/herdr", "runtime"):
        (root / relative).mkdir(parents=True)
    (root / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket = str(root / "runtime/herdr.sock")
    env = {**os.environ, "HOME": str(root / "home"),
           "XDG_CONFIG_HOME": str(root / "config"),
           "XDG_RUNTIME_DIR": str(root / "runtime"),
           "HERDR_SOCKET_PATH": socket, "SHELL": "/bin/sh",
           "TERM": "xterm-256color"}
    env.pop("HERDR_ENV", None)
    server_log = (root / "server.log").open("wb")
    server = subprocess.Popen(["herdr", "server"], env=env, stdout=server_log,
                              stderr=subprocess.STDOUT)
    server_log.close()
    server_identity = identity(server.pid)
    h = Herdr(socket)
    shell = None
    master = slave = None
    result = {"schema_version": 1, "socket": socket,
              "server": server_identity, "signals": []}
    try:
        peer = wait_for(lambda: h.server_identity() if server.poll() is None else False)
        assert peer["pid"] == server.pid
        for sig in (signal.SIGTERM, signal.SIGINT):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 28, 100, 0, 0))
            shell = subprocess.Popen(["/bin/sh", "-c", "herdr; printf '\\nWAYWR_SHELL_RETURNED\\n'"],
                                     env=env, stdin=slave, stdout=slave, stderr=slave,
                                     start_new_session=True)
            client_pid = wait_for(lambda: children(shell.pid)[0] if children(shell.pid) else None)
            client_identity = identity(client_pid)
            assert b"herdr" in Path(f"/proc/{client_pid}/cmdline").read_bytes()
            time.sleep(.7)
            assert shell.poll() is None and server.poll() is None
            signal_owned(client_pid, client_identity, sig)
            output = bytearray()
            end = time.monotonic() + 4
            while time.monotonic() < end and b"WAYWR_SHELL_RETURNED" not in output:
                if select.select([master], [], [], .1)[0]:
                    try:
                        output.extend(os.read(master, 65536))
                    except OSError:
                        break
            returned = b"WAYWR_SHELL_RETURNED" in output
            (root / f"client-{sig.name.lower()}.pty").write_bytes(output)
            result["signals"].append({"signal": sig.name, "client": client_identity,
                                      "returned_to_shell": returned,
                                      "server_unchanged": h.server_identity()["pid"] == server.pid,
                                      "captured_bytes": len(output)})
            if not returned:
                signal_owned(client_pid, client_identity, signal.SIGKILL)
            shell.wait(timeout=5)
            os.close(master)
            os.close(slave)
            master = slave = None
            shell = None
        result["proved"] = any(item["returned_to_shell"] and item["server_unchanged"]
                               for item in result["signals"])
    finally:
        if shell and shell.poll() is None:
            for pid in children(shell.pid):
                signal_owned(pid, identity(pid), signal.SIGKILL)
            shell.wait(timeout=5)
        for fd in (master, slave):
            if fd is not None:
                os.close(fd)
        if server.poll() is None:
            try:
                peer = h.server_identity()
            except (FileNotFoundError, ConnectionRefusedError):
                peer = None
            if peer is not None:
                assert peer["pid"] == server_identity["pid"]
                h.call("server.stop")
            else:
                signal_owned(server.pid, server_identity, signal.SIGTERM)
            server.wait(timeout=10)
        result["server_stopped"] = server.poll() is not None
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    if not result.get("proved"):
        raise SystemExit(2)


if __name__ == "__main__":
    main()
