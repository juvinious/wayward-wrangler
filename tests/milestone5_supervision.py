"""Isolated Herdr and deterministic fake workers. No shared session/model calls."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from agent_workflow.herdr_backend import Herdr
from agent_workflow.storage import alive, atomic, identity, read


def eventually(check, timeout=60):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        try:
            value = check()
            if value:
                return value
        except (FileNotFoundError, ConnectionRefusedError):
            pass
        time.sleep(.1)
    raise AssertionError("Timed out waiting for isolated evidence")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    executable = args.cli.resolve()
    root.mkdir(parents=True, exist_ok=False)
    for part in ("home", "config/herdr", "runtime", "registry", "project", "other"):
        (root / part).mkdir(parents=True, exist_ok=True)
    (root / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket = str(root / "runtime/herdr.sock")
    env = {**os.environ, "HOME": str(root / "home"), "XDG_CONFIG_HOME": str(root / "config"),
           "XDG_RUNTIME_DIR": str(root / "runtime"), "HERDR_SOCKET_PATH": socket,
           "SHELL": "/bin/sh", "TERM": "xterm-256color"}
    log = (root / "herdr-server.log").open("wb")
    server = subprocess.Popen(["herdr", "server"], env=env, stdout=log, stderr=subprocess.STDOUT)
    log.close()
    owned = identity(server.pid)
    h = Herdr(socket)
    project = root / "project"
    run_id = "workspace-loss-fake"
    checks = []

    def cli(*cmd, expected=0):
        result = subprocess.run([str(executable), "--project", str(project),
            "--registry-dir", str(root / "registry"), *cmd], env=env,
            cwd=root / "other", capture_output=True, text=True, timeout=120)
        if result.returncode != expected:
            raise AssertionError((cmd, result.returncode, result.stdout[-3000:], result.stderr[-3000:]))
        return result.stdout

    def status():
        return json.loads(cli("status", run_id, "--json"))

    try:
        eventually(lambda: h.server_identity() if server.poll() is None else False)
        assert h.server_identity()["pid"] == owned["pid"]
        h.call("notification.show", {"title": "Wayward Wrangler isolated notice",
               "body": "Test-owned server only", "sound": "none"})
        checks.append({"case": "supported-herdr-notification", "accepted": True})
        workspace = h.call("workspace.create", {"label": "Wayward Wrangler isolated recovery",
                    "cwd": str(project), "focus": False})["workspace"]["workspace_id"]
        cli("init")
        profile = project / ".wayward-wrangler/workflow.toml"
        profile.write_text(profile.read_text().replace("seconds = 3.0", "seconds = 0.2"))
        (project / "brief.md").write_text("Draft a short local policy.\n")
        cli("start", "--run", run_id, "--brief", "brief.md", "--workspace", workspace,
            "--socket", socket, expected=2)
        info = status()
        assert info["status"] == "awaiting-input" and info["pending"]["kind"] == "choice"
        assert info["completed"] == ["draft", "critique-1"]
        (project / "answer.txt").write_text("7 days\n")
        cli("decide", run_id, "--request", info["pending"]["request_id"],
            "--answer-file", "answer.txt", "--record-only")
        source = project / ".wayward-wrangler/state/runs" / run_id
        old_decision = (source / f"artifacts/decision-{info['pending']['request_id']}.json").read_bytes()
        h.call("workspace.close", {"workspace_id": workspace})
        cli("resume", run_id, expected=3)
        broken = status()
        assert broken["status"] == "uncertain" and broken["completed"] == ["draft", "critique-1"]
        prepared = source / "operations/revision-1"
        assert (prepared / "operation.json").exists() and not (prepared / "launch.json").exists()
        checks.append({"case": "missing-workspace-before-launch", "status": "uncertain",
                       "completed": broken["completed"], "worker_count_before_recovery": 2})
        cli("supervise", run_id, "--socket", socket, "--once")
        recovered = eventually(lambda: status() if status()["status"] == "awaiting-input" else None)
        assert recovered["pending"]["kind"] == "final-review"
        assert (source / f"artifacts/decision-{info['pending']['request_id']}.json").read_bytes() == old_decision
        for stage in ("draft", "critique-1", "revision-1", "critique-2", "revision-2"):
            assert read(source / f"operations/{stage}/invocations.json")["count"] == 1
        assert recovered["execution_target"]["workspace"] != workspace
        checks.append({"case": "automatic-workspace-and-controller-recovery", "status": recovered["status"],
                       "new_workspace": recovered["execution_target"]["workspace"], "all_counts": 1})
        owner_path = project / ".wayward-wrangler/state/supervision/owner.json"
        first_owner = read(owner_path)
        cli("supervise", run_id, "--socket", socket, "--once")
        assert read(owner_path) == first_owner
        checks.append({"case": "supervisor-process-restart-reuses-controller", "generation": first_owner["generation"]})
        # Close only the test-owned controller, then reconcile a replacement.
        from agent_workflow.supervision import controller_matches
        pane = controller_matches(h, first_owner["token"])
        assert len(pane) == 1
        h.call("tab.close", {"tab_id": pane[0]["tab_id"]})
        eventually(lambda: not controller_matches(h, first_owner["token"]))
        cli("supervise", run_id, "--socket", socket, "--once")
        second_owner = read(owner_path)
        assert second_owner["generation"] == first_owner["generation"] + 1
        assert len(controller_matches(h, second_owner["token"])) == 1
        for stage in ("draft", "critique-1", "revision-1", "critique-2", "revision-2"):
            assert read(source / f"operations/{stage}/invocations.json")["count"] == 1
        checks.append({"case": "controller-loss-replacement", "generation": second_owner["generation"],
                       "duplicate_calls": False})
        final_sha = recovered["pending"]["artifact_sha256"]
        h.call("tab.close", {"tab_id": controller_matches(h, second_owner["token"])[0]["tab_id"]})
        eventually(lambda: not controller_matches(h, second_owner["token"]))
        refused = subprocess.run([str(executable), "--project", str(project),
            "--registry-dir", str(root / "registry"), "approve", run_id, "--request", "A1",
            "--sha256", "0" * 64, "--record-only"], env=env, cwd=root / "other",
            capture_output=True, text=True, timeout=30)
        assert refused.returncode != 0 and status()["pending"]["artifact_sha256"] == final_sha
        cli("approve", run_id, "--request", "A1", "--sha256", final_sha, "--record-only")
        assert status()["status"] == "awaiting-input"
        cli("supervise", run_id, "--socket", socket, "--once")
        approved = eventually(lambda: status() if status()["status"] == "approved" else None)
        assert approved["status"] == "approved" and approved["pending"] is None
        from agent_workflow import records
        from agent_workflow.runtime import RuntimePaths, bind, unbind
        binding = bind(RuntimePaths(project, project / ".wayward-wrangler/state", root / "registry",
                                    approved["project_key"]))
        try:
            response = records.response(run_id, "A1")
        finally:
            unbind(binding)
        assert response["artifact_sha256"] == final_sha and response["outcome"] == "approved"
        cli("supervise", run_id, "--socket", socket, "--once")
        assert status()["status"] == "approved"
        for stage in ("draft", "critique-1", "revision-1", "critique-2", "revision-2"):
            assert read(source / f"operations/{stage}/invocations.json")["count"] == 1
        checks.append({"case": "artifact-bound-approval-after-controller-restart",
                       "sha256": final_sha, "duplicate_calls": False, "stale_approval_refused": True})
        atomic(root / "result.json", {"passed": True, "checks": checks,
               "decision_sha256": __import__("hashlib").sha256(old_decision).hexdigest()}, immutable=True)
    finally:
        if server.poll() is None:
            try:
                peer = h.server_identity()
                if all(peer[k] == owned[k] for k in ("pid", "start", "boot")):
                    h.call("server.stop")
            finally:
                server.wait(timeout=10)


if __name__ == "__main__":
    main()
