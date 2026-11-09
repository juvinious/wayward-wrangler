"""Checkpoint-4 installed, fake-only acceptance. Never addresses operator units.

Each project uses an isolated HOME/runtime and an exact project key. The test
retains project records under --evidence-root and stops only units named in its
own immutable host intents. Python is the harness, not the installed runtime.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import uuid


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def wait(probe, seconds=70):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = probe()
        if result:
            return result
        time.sleep(.1)
    raise AssertionError("Timed out; retained installed fixture evidence needs inspection")


def call(endpoint, key, release, kind, body):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(12)
        peer.connect(str(endpoint))
        reader = peer.makefile("rb")
        def send(name, payload):
            peer.sendall((json.dumps({"v": 1, "kind": name,
                                      "request_id": uuid.uuid4().hex,
                                      "body": payload}) + "\n").encode())
            return json.loads(reader.readline())
        hello = send("hello", {"protocol": {"major": 1, "minor": 1},
                               "project_key": key, "release_id": release,
                               "channel": "command"})
        assert hello["body"]["accepted"] is True, hello
        return send(kind, body)["body"]


def cli(exe, project, args, env, root, label, stdin=None, allowed=(0,)):
    result = subprocess.run([str(exe), "--project", str(project), *map(str, args)],
                            cwd=project, env=env, input=stdin, text=True,
                            capture_output=True, timeout=60)
    (root / f"{label}.stdout").write_text(result.stdout)
    (root / f"{label}.stderr").write_text(result.stderr)
    save(root / f"{label}.command.json", {"exit_code": result.returncode,
         "argv": [str(exe), "--project", str(project), *map(str, args)]})
    assert result.returncode in allowed, (label, result.returncode, result.stderr[-2000:])
    return result


def main(root, final, previous):
    assert not root.exists(), "Use a fresh ignored evidence directory"
    root.mkdir(parents=True)
    runtime = Path(tempfile.mkdtemp(prefix="ww-s4-"))
    for name in ("home", "config", "runtime", "state"):
        (runtime / name).mkdir(parents=True)
    save(root / "runtime-location.json", {"path": str(runtime),
         "role": "test-owned short Unix-socket directory"})
    env = {**os.environ, "HOME": str(runtime / "home"),
           "XDG_CONFIG_HOME": str(runtime / "config"),
           "XDG_RUNTIME_DIR": str(runtime / "runtime"),
           "XDG_STATE_HOME": str(runtime / "state"),
           "HERDR_ENV": "1", "HERDR_SOCKET_PATH": str(runtime / "missing-herdr.sock"),
           "PATH": "/usr/bin:/bin", "TERM": "xterm-256color"}
    env.pop("PYTHONPATH", None)
    projects = {}
    summary = {"release_final": final.name, "release_previous": previous.name,
               "checks": [], "herdr_server_started": False}
    try:
        for label, release in (("a", final), ("b", previous), ("upgrade", previous)):
            project = root / f"project-{label}"; project.mkdir()
            exe = release / "bin/waywr"
            cli(exe, project, ["init", "."], env, root, f"init-{label}")
            config = project / ".wayward-wrangler"
            key = next(line.split('"')[1] for line in (config / "project.toml").read_text().splitlines()
                       if line.startswith("project_key"))
            state = config / "state"
            endpoint = runtime / "runtime/waywr" / f"{key}.sock"
            projects[label] = {"project": project, "key": key, "state": state,
                               "endpoint": endpoint, "release": release, "exe": exe}
            assert not (config / "herdr.json").exists()
            assert not (runtime / "missing-herdr.sock").exists()
            project.joinpath("brief.md").write_text("Specify a small local decision log.\n")
            profile = config / "workflow.toml"
            profile.write_text(profile.read_text().replace("seconds = 3.0", "seconds = 0.2"))
        assert len({item["key"] for item in projects.values()}) == 3
        summary["checks"].append("Three local initializations created no Herdr endpoint or server")

        def info(label): return projects[label]
        def ask(label, kind, body):
            item = info(label)
            return call(item["endpoint"], item["key"], item["release"].name, kind, body)
        def task(label):
            rows = ask(label, "snapshot", {})["tasks"]
            return rows[0] if rows else None
        def detail(label): return ask(label, "task.get", {"task_id": task(label)["task_id"]})
        def gate(label, kind):
            row = detail(label)
            return row if isinstance(row.get("pending"), dict) and row["pending"]["kind"] == kind else None

        a = info("a"); b = info("b")
        for label in ("a", "b"):
            item = info(label)
            cli(item["exe"], item["project"], ["start", "--run", "shared-01",
                "--brief", "brief.md"], env, root, f"start-{label}", stdin="y\n")
        first = wait(lambda: gate("a", "choice"))
        second = wait(lambda: gate("b", "choice"))
        assert first["task"]["run_id"] == second["task"]["run_id"] == "shared-01"
        assert a["key"] != b["key"]
        packets = {}
        for label in ("a", "b"):
            item = info(label)
            packets[label] = "\n".join(path.read_text() for path in item["state"].rglob("packet.json"))
        assert str(b["project"]) not in packets["a"] and b["key"] not in packets["a"]
        assert str(a["project"]) not in packets["b"] and a["key"] not in packets["b"]
        summary["checks"].append("Same run ID reached independent human gates with no cross-project packet content")

        # Identical command replay cannot create a second task or coordinator call.
        frozen = json.loads((a["state"] / "session/cli-start/shared-01.json").read_text())
        replay = ask("a", "command", frozen["command"])
        assert replay["task_id"] == first["task"]["task_id"], replay
        assert len(list((a["state"] / "session/coordinator" / replay["task_id"]).glob("*/attempts/*/invocations.json"))) == 1
        summary["checks"].append("Replayed accepted task command reused the same coordinator invocation")

        # B is cancelled independently while A remains at its first question.
        cancelled = ask("b", "command", {"command_id": "fixture-cancel-b",
            "action": "cancel_task", "expected_task_version": second["task"]["version"],
            "expected_state_revision": second["task"]["state_revision"],
            "payload": {"task_id": second["task"]["task_id"]}})
        assert cancelled["status"] == "accepted", cancelled
        assert gate("a", "choice") is not None
        routed = cli(a["exe"], b["project"], ["status"], env, root, "route-b-status")
        assert json.loads(routed.stdout)["project_key"] == b["key"]
        loop_probe = cli(a["exe"], b["project"], ["status"],
                         {**env, "WAYWR_ROUTE_DEPTH": "1"}, root, "route-b-loop-guard", allowed=(2,))
        assert "Project release routing loop" in loop_probe.stderr
        summary["checks"].append("B cancellation left A pending; final launcher routed B to its compatible prior release")

        # Stop and resume A around its durable question; no worker is retried.
        cli(a["exe"], a["project"], ["session", "stop"], env, root, "stop-a")
        wait(lambda: subprocess.run(["systemctl", "--user", "is-active", f"waywr-{a['key']}.service"],
                                   capture_output=True, text=True).stdout.strip() != "active")
        paused = json.loads(cli(a["exe"], a["project"], ["status"], env, root, "paused-a").stdout)
        assert paused["session"]["phase"] != "running" and paused["tasks"][0]["status"] == "awaiting-input"
        cli(a["exe"], a["project"], ["workers", "shared-01"], env, root, "restart-a")
        resumed = ask("a", "snapshot", {})
        answer = ask("a", "command", {"command_id": "fixture-answer-a",
            "action": "answer_task", "expected_task_version": first["task"]["version"],
            "expected_state_revision": first["task"]["state_revision"],
            "payload": {"task_id": first["task"]["task_id"],
                        "request_id": first["pending"]["request_id"],
                        "outcome": "answered",
                        "answer": first["pending"]["question"]["options"][0],
                        "artifact_sha256": first["pending"]["artifact_sha256"]}})
        assert answer["status"] == "accepted", answer
        after_answer = ask("a", "snapshot", {})
        continuation = ask("a", "command", {"command_id": "fixture-resume-a",
            "action": "resume_session", "expected_state_revision": after_answer["state_revision"],
            "payload": {}})
        assert continuation["status"] == "accepted", continuation
        final_gate = wait(lambda: gate("a", "final-review"))
        before = final_gate["pending"]["artifact_sha256"]
        wrong = ask("a", "command", {"command_id": "fixture-wrong-approval",
            "action": "answer_task", "expected_task_version": final_gate["task"]["version"],
            "expected_state_revision": final_gate["task"]["state_revision"],
            "payload": {"task_id": final_gate["task"]["task_id"],
                        "request_id": final_gate["pending"]["request_id"],
                        "outcome": "approved", "answer": "Fixture approval",
                        "artifact_sha256": "0" * 64}})
        assert wrong["status"] == "rejected", wrong
        approved = ask("a", "command", {"command_id": "fixture-exact-approval",
            "action": "answer_task", "expected_task_version": final_gate["task"]["version"],
            "expected_state_revision": final_gate["task"]["state_revision"],
            "payload": {"task_id": final_gate["task"]["task_id"],
                        "request_id": final_gate["pending"]["request_id"],
                        "outcome": "approved", "answer": "Fixture approval",
                        "artifact_sha256": before}})
        assert approved["status"] == "accepted", approved
        assert wait(lambda: (row if (row := detail("a"))["task"]["status"] == "approved" else None))
        save(root / "approval-a.json", {"task": first["task"]["task_id"], "artifact_sha256": before,
             "approved_response": approved, "session_after_restart": resumed["session"]})
        counts = {stage: json.loads(next((a["state"] / "runs/shared-01/operations" / stage / "attempts").glob("*/invocations.json")).read_text())["count"]
                  for stage in ("draft", "critique-1", "revision-1", "critique-2", "revision-2")}
        assert counts == {stage: 1 for stage in counts}, counts
        summary["checks"].append("Stop/restart preserved decision; five fake calls, stale approval refused, exact approval bound")

        # Upgrade and rollback only a quiescent disposable host project.
        u = info("upgrade")
        upgraded = json.loads(cli(a["exe"], u["project"], ["session", "upgrade",
            "--release", final.name], env, root, "upgrade-u").stdout)
        assert upgraded["binding_generation"] == 2
        restored = json.loads(cli(a["exe"], u["project"], ["session", "rollback",
            "--release", previous.name], env, root, "rollback-u").stdout)
        assert restored["binding_generation"] == 3
        cli(a["exe"], u["project"], ["session", "upgrade", "--release", final.name],
            env, root, "reupgrade-u")
        u["release"] = final
        u["exe"] = a["exe"]
        # Accept and then cancel a task, so quiescence passes but the full
        # state fingerprint has changed since the last binding boundary.
        cli(a["exe"], u["project"], ["start", "--run", "rollback-check",
            "--brief", "brief.md"], env, root, "accepted-u", stdin="n\n", allowed=(2,))
        proposed = wait(lambda: row if (row := task("upgrade"))["status"] == "proposed" else None)
        cancelled_u = ask("upgrade", "command", {"command_id": "fixture-cancel-u",
            "action": "cancel_task", "expected_task_version": proposed["version"],
            "expected_state_revision": proposed["state_revision"],
            "payload": {"task_id": proposed["task_id"]}})
        assert cancelled_u["status"] == "accepted", cancelled_u
        cli(a["exe"], u["project"], ["session", "stop"], env, root, "stop-u")
        wait(lambda: subprocess.run(["systemctl", "--user", "is-active", f"waywr-{u['key']}.service"],
                                   capture_output=True, text=True).stdout.strip() != "active")
        refused = cli(a["exe"], u["project"], ["session", "rollback",
            "--release", previous.name], env, root, "refused-rollback-u", allowed=(2,))
        assert "Rollback would cross new project work" in refused.stderr, refused.stderr
        assert json.loads((u["project"] / ".wayward-wrangler/release.json").read_text())["release_id"] == final.name
        summary["checks"].append("Quiescent fixture upgrade/rollback succeeded; rollback after accepted-and-cancelled work refused without restoring state")

        # No service or optional-view state belongs to an unregistered Herdr
        # server; all A/B operations were hosted by project-owned systemd.
        assert not (runtime / "missing-herdr.sock").exists()
        assert not any((item["state"] / "views").exists() for item in projects.values())
        save(root / "summary.json", summary)
        print(json.dumps({"passed": True, "evidence": str(root), "checks": len(summary["checks"])}))
    finally:
        cleanup = {}
        for label, item in projects.items():
            units = {f"waywr-{item['key']}.service"}
            for path in item["state"].rglob("intent.json"):
                try:
                    intent = json.loads(path.read_text())
                except (OSError, ValueError):
                    continue
                unit = intent.get("unit", "")
                if intent.get("project_key") == item["key"] and unit.startswith(f"waywr-op-{item['key']}-"):
                    units.add(unit)
            for unit in sorted(units):
                subprocess.run(["systemctl", "--user", "stop", unit], capture_output=True, text=True)
                subprocess.run(["systemctl", "--user", "reset-failed", unit], capture_output=True, text=True)
            cleanup[label] = {unit: subprocess.run(["systemctl", "--user", "is-active", unit],
                capture_output=True, text=True).stdout.strip() for unit in sorted(units)}
        save(root / "cleanup.json", cleanup)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--final-release", type=Path, required=True)
    parser.add_argument("--previous-release", type=Path, required=True)
    options = parser.parse_args()
    main(options.evidence_root.resolve(), options.final_release.resolve(),
         options.previous_release.resolve())
