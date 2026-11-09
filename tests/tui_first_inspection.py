"""Installed fake-only output and optional-view gate; uses a private Herdr server.

The exact project service and private server are stopped in finally. Evidence
under the caller's ignored root remains intact after assertions or failures.
"""
import argparse
import atexit
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import uuid


def saved(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def wait(predicate, seconds=40):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        value = predicate()
        if value:
            return value
        time.sleep(.1)
    raise AssertionError("Fixture timed out; retained evidence is in the test root")


def herdr(endpoint, method, params=None):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(10)
        peer.connect(str(endpoint))
        peer.sendall((json.dumps({"id": uuid.uuid4().hex, "method": method,
                                  "params": params or {}}) + "\n").encode())
        response = json.loads(peer.makefile("rb").readline())
    if "error" in response:
        raise RuntimeError(f"Private Herdr {method}: {response['error']}")
    return response["result"]


def call(endpoint, key, release, kind, body, channel="command", view=None):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(10)
        peer.connect(str(endpoint))
        stream = peer.makefile("rb")
        def send(name, payload):
            peer.sendall((json.dumps({"v": 1, "kind": name, "request_id": uuid.uuid4().hex,
                                      "body": payload}) + "\n").encode())
            return json.loads(stream.readline())
        hello = {"protocol": {"major": 1, "minor": 1}, "project_key": key,
                 "release_id": release, "channel": channel}
        if view:
            hello.update({"view_id": view["view_id"], "view_token": view["token"]})
        answer = send("hello", hello)
        assert answer["body"]["accepted"] is True, answer
        return send(kind, body)


def command(exe, args, env, root, label, stdin=None, allowed=(0,)):
    outcome = subprocess.run([str(exe), *map(str, args)], cwd=root / "project", env=env, input=stdin,
                             text=True, capture_output=True, timeout=45)
    (root / f"{label}.stdout").write_text(outcome.stdout)
    (root / f"{label}.stderr").write_text(outcome.stderr)
    saved(root / f"{label}.json", {"exit_code": outcome.returncode,
                                    "argv": [str(exe), *map(str, args)]})
    assert outcome.returncode in allowed, (label, outcome.returncode, outcome.stderr[-1200:])
    return outcome


def main(root, release):
    assert not root.exists(), "Use a fresh ignored evidence directory"
    root.mkdir(parents=True)
    private = Path(tempfile.mkdtemp(prefix="ww-s3-"))
    saved(root / "private-runtime-location.json", {"path": str(private),
        "reason": "short Unix socket path; test-owned runtime only"})
    for name in ("home", "config/herdr", "runtime", "state"):
        (private / name).mkdir(parents=True)
    (private / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    env = {**os.environ, "HOME": str(private / "home"), "XDG_CONFIG_HOME": str(private / "config"),
           "XDG_RUNTIME_DIR": str(private / "runtime"), "XDG_STATE_HOME": str(private / "state"),
           "HERDR_SOCKET_PATH": str(private / "stale-herdr.sock"), "SHELL": "/bin/sh",
           "TERM": "xterm-256color"}
    env.pop("HERDR_ENV", None)
    project = root / "project"; project.mkdir()
    exe = release / "bin/waywr"
    common = ["--project", project, "--ipc-dir", private / "runtime",
              "--registry-dir", private / "registry"]
    command(exe, [*common, "init", "."], env, root, "init")
    config = project / ".wayward-wrangler"
    owned_key = next(line.split('"')[1] for line in (config / "project.toml").read_text().splitlines()
                     if line.startswith("project_key"))
    owned_unit = f"waywr-{owned_key}.service"
    def stop_owned_service():
        subprocess.run(["systemctl", "--user", "stop", owned_unit], check=False,
                       capture_output=True, text=True)
        active = subprocess.run(["systemctl", "--user", "is-active", owned_unit],
                                capture_output=True, text=True).stdout.strip() == "active"
        saved(root / "service-cleanup.json", {"unit": owned_unit, "active_after_cleanup": active})
    atexit.register(stop_owned_service)
    profile = config / "workflow.toml"
    profile.write_text(profile.read_text().replace("seconds = 3.0", "seconds = 0.6"))
    (project / "brief.md").write_text("Specify a local project decision log and ask about edit policy.\n")
    start = command(exe, [*common, "start", "--run", "inspect-01", "--brief", "brief.md"],
                    env, root, "start", stdin="y\n")
    assert "admit_task" in start.stdout or '"status": "accepted"' in start.stdout
    key = json.loads((config / "release.json").read_text())["project_key"]
    endpoint = private / "runtime/waywr" / f"{key}.sock"
    release_id = release.name
    state = config / "state"
    task = wait(lambda: (rows[0] if rows else None) if
                (rows := call(endpoint, key, release_id, "snapshot", {})["body"]["tasks"]) else None)
    task_id = task["task_id"]
    operation = wait(lambda: (items[0] if items else None) if
                     (items := call(endpoint, key, release_id, "operation.list",
                                    {"task_id": task_id})["body"]["operations"]) else None)
    assert operation["stage"] == "coordinator"
    first = wait(lambda: (page if page["high_water"] else None) if
                 (page := call(endpoint, key, release_id, "operation.output.read",
                               {"task_id": task_id, "stage": "coordinator", "limit": 5})["body"]["page"])
                 else None)
    replay = call(endpoint, key, release_id, "operation.output.read",
                  {"task_id": task_id, "stage": "coordinator", "cursor": first["cursor"],
                   "limit": 64})["body"]["page"]
    assert first["format"] == "indexed-v1" and first["cursor"] != replay["cursor"]
    assert replay["segments"] and replay["segments"][0]["stream"] in ("stdout", "stderr")
    saved(root / "cursor-pages.json", {"first": first, "next": replay})
    assert not (private / "stale-herdr.sock").exists()
    assert not (state / "views").exists(), "Ordinary workflow created view state"
    wait(lambda: any(item["stage"] == "draft" and item["output_available"] for item in
         call(endpoint, key, release_id, "operation.list", {"task_id": task_id})["body"]["operations"]))
    command(exe, [*common, "workers", "inspect-01"], env, root, "workers")
    command(exe, [*common, "logs", "inspect-01", "--operation", "draft"],
            env, root, "logs")
    draft_attempt = wait(lambda: next((path for path in (state / "runs/inspect-01/operations/draft/attempts")
                                      .glob("*/invocations.json")), None))
    assert json.loads(draft_attempt.read_text())["count"] == 1
    draft_artifact = wait(lambda: state / "runs/inspect-01/artifacts/draft.md" if
                          (state / "runs/inspect-01/artifacts/draft.md").exists() else None)
    shown = command(exe, [*common, "show", "inspect-01", "--artifact", "draft"],
                    env, root, "show-draft")
    assert shown.stdout == draft_artifact.read_text()
    assert not (state / "views").exists()
    server_env = {**env, "HERDR_SOCKET_PATH": str(private / "runtime/herdr.sock")}
    server_log = (root / "herdr-server.log").open("wb")
    server = subprocess.Popen([str(Path.home() / ".local/bin/herdr"), "server"],
                              env=server_env, stdout=server_log, stderr=subprocess.STDOUT)
    server_log.close()
    view_endpoint = private / "runtime/herdr.sock"
    cleanup = {"private_server_stopped": False, "project_unit_stopped": False}
    try:
        wait(lambda: server.poll() is None and view_endpoint.is_socket())
        before = {"workspaces": len(herdr(view_endpoint, "workspace.list")["workspaces"]),
                  "tabs": len(herdr(view_endpoint, "tab.list")["tabs"])}
        assert before == {"workspaces": 0, "tabs": 0}, before
        view_args = [*common, "observe", "inspect-01", "--operation", "draft",
                     "--viewer", "herdr", "--socket", view_endpoint]
        first_view = command(exe, view_args, env, root, "view-first")
        after = {"workspaces": len(herdr(view_endpoint, "workspace.list")["workspaces"]),
                 "tabs": len(herdr(view_endpoint, "tab.list")["tabs"])}
        assert after == {"workspaces": 1, "tabs": 2}, after  # one bootstrap shell + one mirror
        repeated_view = command(exe, view_args, env, root, "view-repeat")
        repeated = {"workspaces": len(herdr(view_endpoint, "workspace.list")["workspaces"]),
                    "tabs": len(herdr(view_endpoint, "tab.list")["tabs"])}
        assert repeated == after, repeated
        assert repeated_view.stdout == first_view.stdout, "Repeat open did not reuse exact pane"
        assert len(list((state / "views/intents").glob("*.json"))) == 1
        intent = next((state / "views/intents").glob("*.json"))
        grant = json.loads(intent.read_text())
        wrong = call(endpoint, key, release_id, "command", {"action": "cancel_task"},
                     channel="view", view=grant)
        assert wrong["kind"] == "error" and wrong["body"]["code"] == "read_only_scope"
        wrong_project = call(endpoint, key, release_id, "operation.output.read",
                             {"task_id": "unrelated", "stage": "draft", "attempt_id": grant["attempt_id"]},
                             channel="view", view=grant)
        assert wrong_project["kind"] == "error" and wrong_project["body"]["code"] == "read_only_scope"
        binding = json.loads((state / "views/bindings" / f"{grant['view_id']}.json").read_text())
        view_tab = binding["tab_id"]
        herdr(view_endpoint, "tab.close", {"tab_id": view_tab})
        command(exe, [*common, "logs", "inspect-01", "--operation", "draft"],
                env, root, "logs-after-view-close")
        assert json.loads(draft_attempt.read_text())["count"] == 1
        herdr(view_endpoint, "server.stop")
        wait(lambda: server.poll() is not None, 8)
        command(exe, [*common, "logs", "inspect-01", "--operation", "draft"],
                env, root, "logs-after-server-loss")
        assert json.loads(draft_attempt.read_text())["count"] == 1
        saved(root / "view-summary.json", {"before": before, "opened": after,
                                            "repeat": repeated, "grant": {"view_id": grant["view_id"],
                                            "stage": grant["stage"], "attempt_id": grant["attempt_id"]},
                                            "closed_tab": view_tab})
    finally:
        if server.poll() is None:
            try: herdr(view_endpoint, "server.stop")
            except Exception: pass
            try: server.wait(timeout=8)
            except subprocess.TimeoutExpired: pass
        cleanup["private_server_stopped"] = server.poll() is not None
        unit = f"waywr-{key}.service"
        subprocess.run(["systemctl", "--user", "stop", unit], check=False,
                       capture_output=True, text=True)
        subprocess.run(["systemctl", "--user", "reset-failed", unit], check=False,
                       capture_output=True, text=True)
        cleanup["project_unit_stopped"] = subprocess.run(["systemctl", "--user", "is-active", unit],
                                                        capture_output=True, text=True).stdout.strip() != "active"
        saved(root / "cleanup.json", cleanup)
    assert cleanup["private_server_stopped"] and cleanup["project_unit_stopped"]
    print(json.dumps({"passed": True, "evidence": str(root), "task_id": task_id}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--release", type=Path, required=True)
    options = parser.parse_args()
    main(options.evidence_root.resolve(), options.release.resolve())
