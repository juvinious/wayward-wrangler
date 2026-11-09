"""Private installed direct-terminal startup proof; fake-only, no shared Herdr."""
import argparse
import json
import os
from pathlib import Path
import pty
import select
import signal
import subprocess
import tempfile
import time
import uuid
import socket
import hashlib

from rust_migration_stage5 import call_herdr, save, wait


def tab_counts(socket):
    workspaces = call_herdr(socket, "workspace.list")["workspaces"]
    tabs = call_herdr(socket, "tab.list")["tabs"]
    return {"workspaces": len(workspaces), "tabs": len(tabs)}


def service_call(endpoint, key, release, kind, body):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(10); peer.connect(str(endpoint))
        stream = peer.makefile("rb")
        def request(name, payload):
            peer.sendall((json.dumps({"v": 1, "kind": name, "request_id": uuid.uuid4().hex,
                                      "body": payload}) + "\n").encode())
            return json.loads(stream.readline())
        hello = request("hello", {"protocol": {"major": 1, "minor": 0}, "project_key": key,
                                  "release_id": release, "channel": "command"})
        assert hello["body"]["accepted"] is True, hello
        return request(kind, body)


def drain(fd, seconds=.1):
    data = bytearray()
    end = time.monotonic() + seconds
    while time.monotonic() < end and select.select([fd], [], [], .02)[0]:
        try: data.extend(os.read(fd, 65536))
        except OSError: break
    return data


def terminal_command(command, project, env):
    master, slave = pty.openpty()
    process = subprocess.Popen(["/bin/sh", "-c", command + "; echo WAYWR_RETURNED"],
                               cwd=project, stdin=slave, stdout=slave, stderr=slave,
                               env=env, start_new_session=True)
    return process, master, slave


def detach(process, master):
    os.write(master, b"\x11\t\r")
    output = drain(master, 2)
    wait(lambda: process.poll() is not None, 8)
    output.extend(drain(master, .2))
    assert b"WAYWR_RETURNED" in output, "Waywr did not restore invoking terminal"
    return output


def main(root, release, herdr_shell="/bin/sh"):
    assert not root.exists(), "Use fresh evidence"
    root.mkdir(parents=True)
    release = release.resolve()
    exe = release / "bin/waywr"
    private = Path(tempfile.mkdtemp(prefix="waywr-direct-ui-"))
    for part in ("home", "config/herdr", "runtime"):
        (private / part).mkdir(parents=True)
    (private / "config/herdr/config.toml").write_text(
        'onboarding = false\nshell = ' + json.dumps(herdr_shell) + '\n')
    endpoint = str(private / "runtime/herdr.sock")
    env = {**os.environ, "HOME": str(private / "home"), "XDG_CONFIG_HOME": str(private / "config"),
           "XDG_STATE_HOME": str(private / "state"), "XDG_RUNTIME_DIR": str(private / "runtime"),
           "HERDR_SOCKET_PATH": endpoint, "SHELL": "/bin/sh", "TERM": "xterm-256color"}
    for name in ("HERDR_ENV", "HERDR_WORKSPACE_ID", "HERDR_TAB_ID", "HERDR_PANE_ID"):
        env.pop(name, None)
    project = root / "project"; project.mkdir()
    sibling = root / "named"
    nested = root / "nested/deeper"
    init = []
    original_files = None
    for args in (["init", "."], ["init", "."], ["init", str(sibling)],
                 ["init", str(sibling)], ["init", str(nested)]):
        result = subprocess.run([str(exe), *args], cwd=project, env=env, capture_output=True, text=True)
        init.append({"args": args, "code": result.returncode, "output": result.stdout or result.stderr})
        assert result.returncode == 0, init[-1]
        selected = project if args[-1] == "." else (nested if args[-1] == str(nested) else sibling)
        config_files = [selected / ".wayward-wrangler" / name for name in
                        ("project.toml", "workflow.toml", "herdr.json", "release.json")]
        digests = [hashlib.sha256(path.read_bytes()).hexdigest() for path in config_files]
        if args[-1] == ".":
            if original_files is None: original_files = digests
            else: assert digests == original_files, "Repeated init changed project configuration"
        elif selected == sibling:
            if "named_files" not in locals(): named_files = digests
            else: assert digests == named_files, "Repeated named init changed project configuration"
        else:
            assert all(path.is_file() for path in config_files)
    config = project / ".wayward-wrangler"
    key = json.loads((config / "release.json").read_text())["project_key"]
    unit = f"waywr-{key}.service"
    assert not Path(endpoint).exists()
    assert not (config / "state/session/ui-current.json").exists()
    assert subprocess.run(["systemctl", "--user", "is-active", unit], capture_output=True,
                          text=True).stdout.strip() != "active"
    cleanup = {"unit": unit}
    master = slave = None
    launcher = None
    try:
        launcher, master, slave = terminal_command(f"'{exe}' --project '{project}'", project, env)
        state = config / "state"
        binding = wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                       if (state / "session/ui-current.json").is_file() else None, 30)
        assert binding["mode"] == "direct-terminal" and binding["pane"] is None
        assert binding["process"]["pid"] > 0
        counts = tab_counts(endpoint)
        assert counts == {"workspaces": 0, "tabs": 0}, counts
        repeated_while_running = subprocess.run([str(exe), "init", "."], cwd=project, env=env,
                                                 capture_output=True, text=True)
        assert repeated_while_running.returncode == 0, repeated_while_running.stderr
        assert tab_counts(endpoint) == counts, "init changed a running Herdr topology"
        ipc = private / "runtime/waywr" / f"{key}.sock"
        old_attach = service_call(ipc, key, release.name, "ui.attach", {})
        assert old_attach["kind"] == "error" and tab_counts(endpoint) == counts
        admitted = service_call(ipc, key, release.name, "command", {"command_id": "direct-ui-task-1",
            "action": "submit_message", "expected_state_revision": 0,
            "payload": {"text": "Draft a short project decision log specification."}})
        assert admitted["body"]["status"] == "accepted", admitted
        task_id = admitted["body"]["task_id"]
        worker_binding = wait(lambda: json.loads((state / "session/workspace-current.json").read_text())
                              if (state / "session/workspace-current.json").is_file() else None, 15)
        worker_workspace = worker_binding["workspace_id"]
        assert worker_workspace != binding["pane"]
        wait(lambda: any(t["tab_id"] != worker_binding["initial_tab_id"]
                         for t in call_herdr(endpoint, "tab.list")["tabs"]), 15)
        worker_counts = tab_counts(endpoint)
        assert worker_counts == {"workspaces": 1, "tabs": 1}, worker_counts
        proposal = wait(lambda: (detail if detail["body"]["task"]["status"] in ("proposed", "blocked") else None)
                        if (detail := service_call(ipc, key, release.name, "task.get", {"task_id": task_id})) else None, 20)
        assert proposal["body"]["task"]["status"] == "proposed", proposal
        # Ctrl-Q opens a confirmation; Tab selects Detach, Enter confirms it.
        output = detach(launcher, master)
        (root / "outside-terminal.pty").write_bytes(output)
        after_detach = tab_counts(endpoint)
        assert after_detach == worker_counts

        os.close(master); os.close(slave)
        launcher, master, slave = terminal_command(f"'{exe}' --project '{project}' resume", project, env)
        resumed = wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                       if json.loads((state / "session/ui-current.json").read_text())["token"] != binding["token"] else None, 15)
        assert resumed["pane"] is None and tab_counts(endpoint) == worker_counts
        detach(launcher, master)
        os.close(master); os.close(slave); master = slave = None

        failure, fail_master, fail_slave = terminal_command(
            f"'{exe}' --project '{sibling}' --herdr-socket '{private / 'wrong.sock'}'", sibling, env)
        try:
            wait(lambda: failure.poll() is not None, 8)
            failed_output = drain(fail_master, .3)
            assert b"WAYWR_RETURNED" in failed_output
            assert b"differs" in failed_output
            (root / "startup-failure.pty").write_bytes(failed_output)
            assert tab_counts(endpoint) == worker_counts
        finally:
            os.close(fail_master); os.close(fail_slave)

        # Start a real managed shell in this private server. Herdr supplies
        # HERDR_* itself; the harness does not forge a managed environment.
        workspace = call_herdr(endpoint, "workspace.create", {"label": "Direct startup private pane",
            "cwd": str(project), "focus": False})
        pane = workspace["root_pane"]["pane_id"]
        tab = workspace["tab"]["tab_id"]
        before_inside = tab_counts(endpoint)
        command = f"'{exe}' --project '{project}'"
        call_herdr(endpoint, "pane.send_input", {"pane_id": pane, "text": command, "keys": ["enter"]})
        direct = wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                      if json.loads((state / "session/ui-current.json").read_text())["token"] != resumed["token"] else None, 15)
        assert direct["pane"]["pane_id"] == pane and direct["pane"]["tab_id"] == tab
        assert direct["pane"]["workspace_id"] != worker_workspace
        assert json.loads((state / "session/workspace-current.json").read_text())["workspace_id"] == worker_workspace
        assert tab_counts(endpoint) == before_inside
        duplicate, dup_master, dup_slave = terminal_command(f"'{exe}' --project '{project}'", project, env)
        try:
            wait(lambda: duplicate.poll() is not None, 8)
            duplicate_output = drain(dup_master, .3)
            assert b"already attached" in duplicate_output and b"WAYWR_RETURNED" in duplicate_output
            assert tab_counts(endpoint) == before_inside
        finally:
            os.close(dup_master); os.close(dup_slave)
        call_herdr(endpoint, "pane.send_keys", {"pane_id": pane, "keys": ["ctrl+q", "tab", "enter"]})
        wait(lambda: not Path("/proc").joinpath(str(direct["process"]["pid"])).exists(), 8)
        shell_after = call_herdr(endpoint, "pane.process_info", {"pane_id": pane})["process_info"]
        assert shell_after["foreground_processes"] == [shell_after["foreground_processes"][0]]
        assert shell_after["foreground_processes"][0]["pid"] == shell_after["shell_pid"]
        call_herdr(endpoint, "pane.send_input", {"pane_id": pane, "text": command, "keys": ["enter"]})
        inside_resumed = wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                              if json.loads((state / "session/ui-current.json").read_text())["token"] != direct["token"] else None, 15)
        assert inside_resumed["pane"]["pane_id"] == pane and tab_counts(endpoint) == before_inside
        call_herdr(endpoint, "pane.send_keys", {"pane_id": pane, "keys": ["ctrl+q", "tab", "enter"]})
        wait(lambda: not Path("/proc").joinpath(str(inside_resumed["process"]["pid"])).exists(), 8)
        after_inside = tab_counts(endpoint)
        assert after_inside == before_inside

        launcher, master, slave = terminal_command(f"'{exe}' --project '{project}'", project, env)
        wait(lambda: json.loads((state / "session/ui-current.json").read_text())
             if json.loads((state / "session/ui-current.json").read_text())["token"] != inside_resumed["token"] else None, 15)
        os.write(master, b"/exit\r")
        time.sleep(.15)
        os.write(master, b"\t\r")
        exit_output = drain(master, 3)
        wait(lambda: launcher.poll() is not None, 8)
        exit_output.extend(drain(master, .2))
        assert b"WAYWR_RETURNED" in exit_output, "Exit did not restore invoking terminal"
        (root / "outside-exit.pty").write_bytes(exit_output)
        assert tab_counts(endpoint) == before_inside
        os.close(master); os.close(slave); master = slave = None

        latest = json.loads((state / "session/ui-current.json").read_text())["token"]
        launcher, master, slave = terminal_command(f"'{exe}' --project '{project}' resume", project, env)
        try:
            wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                 if json.loads((state / "session/ui-current.json").read_text())["token"] != latest else None, 15)
        except AssertionError:
            (root / "resume-after-exit-failure.pty").write_bytes(drain(master, .5))
            raise
        after_exit_resume = tab_counts(endpoint)
        assert after_exit_resume == before_inside
        detach(launcher, master)
        os.close(master); os.close(slave); master = slave = None
        save(root / "summary.json", {"status": "passed", "init": init,
            "outside_start": counts, "outside_detach": after_detach,
            "worker_workspace": worker_workspace, "worker_counts": worker_counts,
            "coordinator_status": proposal["body"]["coordinator"]["status"],
            "inside_before": before_inside, "inside_after": after_inside,
            "exit_returned_to_shell": True, "after_exit_resume": after_exit_resume,
            "resume_outside_token": resumed["token"], "resume_inside_token": inside_resumed["token"],
            "binding": direct, "release": str(release)})
    finally:
        if launcher and launcher.poll() is None:
            os.killpg(launcher.pid, signal.SIGTERM)
            launcher.wait(timeout=5)
        for fd in (master, slave):
            if fd is not None: os.close(fd)
        subprocess.run(["systemctl", "--user", "stop", unit], capture_output=True)
        cleanup["unit_stopped"] = subprocess.run(["systemctl", "--user", "is-active", unit],
            capture_output=True, text=True).stdout.strip() != "active"
        if Path(endpoint).is_socket():
            try: call_herdr(endpoint, "server.stop")
            except (OSError, RuntimeError): pass
        server_pid = binding["server"]["pid"] if "binding" in locals() else None
        if server_pid:
            for _ in range(50):
                if not Path(f"/proc/{server_pid}").exists(): break
                time.sleep(.1)
        cleanup["private_herdr_stopped"] = server_pid is None or not Path(f"/proc/{server_pid}").exists()
        save(root / "cleanup.json", cleanup)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--herdr-shell", default="/bin/sh")
    args = parser.parse_args()
    main(args.evidence_root.resolve(), args.release, args.herdr_shell)
