"""Opt-in r9/Rust ordinary CLI comparison on separate fake-only projects.

All Herdr/service processes are private to this fixture. No authenticated CLI
is invoked. Run with a fresh --evidence-root and an installed Rust release.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
R9 = ROOT / ".local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9"
HERDR = Path.home() / ".local/bin/herdr"


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def wait(predicate, seconds=70):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        result = predicate()
        if result:
            return result
        time.sleep(.1)
    raise AssertionError("Fixture timed out; inspect retained service/operation evidence")


def herdr(endpoint, method, params=None):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(8)
        peer.connect(endpoint)
        peer.sendall((json.dumps({"id": uuid.uuid4().hex, "method": method,
                                  "params": params or {}}) + "\n").encode())
        value = json.loads(peer.makefile("rb").readline())
    if "error" in value:
        raise RuntimeError(f"Private Herdr {method}: {value['error']}")
    return value["result"]


def service_call(endpoint, key, release, kind, body):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(10)
        peer.connect(endpoint)
        reader = peer.makefile("rb")
        def send(name, content):
            peer.sendall((json.dumps({"v": 1, "kind": name, "request_id": uuid.uuid4().hex,
                                      "body": content}) + "\n").encode())
            return json.loads(reader.readline())
        hello = send("hello", {"protocol": {"major": 1, "minor": 0},
                               "project_key": key, "release_id": release,
                               "channel": "command"})
        assert hello["body"]["accepted"] is True, hello
        return send(kind, body)["body"]


def run_cli(binary, env, root, label, args, input_text=None, allowed=(0,)):
    result = subprocess.run([str(binary), *map(str, args)], env=env, input=input_text,
                            text=True, capture_output=True, timeout=90)
    (root / f"{label}.stdout").write_text(result.stdout)
    (root / f"{label}.stderr").write_text(result.stderr)
    save(root / f"{label}.result.json", {"exit_code": result.returncode,
                                         "argv": [str(binary), *map(str, args)]})
    assert result.returncode in allowed, (label, result.returncode, result.stderr[-2000:])
    return result


def main(root, release):
    assert not root.exists(), "Use a fresh evidence root"
    root.mkdir(parents=True)
    private = Path(tempfile.mkdtemp(prefix="ww-corrective-cli-"))
    for name in ("home", "config/herdr", "runtime"):
        (private / name).mkdir(parents=True)
    (private / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    endpoint = private / "runtime/herdr.sock"
    env = {**os.environ, "HOME": str(private / "home"), "XDG_CONFIG_HOME": str(private / "config"),
           "XDG_RUNTIME_DIR": str(private / "runtime"), "HERDR_SOCKET_PATH": str(endpoint),
           "SHELL": "/bin/sh", "TERM": "xterm-256color"}
    env.pop("HERDR_ENV", None)
    env.pop("NO_COLOR", None)
    server_log = (root / "herdr-server.log").open("wb")
    server = subprocess.Popen([str(HERDR), "server"], env=env,
                              stdout=server_log, stderr=subprocess.STDOUT)
    server_log.close()
    service = None
    summary = {"rust_release": release.name, "r9_manifest_sha256":
               hashlib.sha256((R9 / "manifest.json").read_bytes()).hexdigest(),
               "private_herdr_socket": str(endpoint), "checks": []}
    try:
        wait(lambda: server.poll() is None and endpoint.is_socket(), 10)
        workspace = herdr(str(endpoint), "workspace.create", {"label": "CLI comparison",
            "cwd": str(root), "focus": False})["workspace"]["workspace_id"]
        manifest = json.loads((release / "manifest.json").read_text())
        rust_bin = release / "bin/waywr"
        rust = root / "rust-project"
        (rust / ".wayward-wrangler").mkdir(parents=True)
        key = uuid.uuid4().hex
        (rust / ".wayward-wrangler/project.toml").write_text(f'schema_version = 1\nproject_key = "{key}"\n')
        (rust / ".wayward-wrangler/workflow.toml").write_text(
            'review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "deterministic-author-v1"\n'
            '[roles.critic]\nprovider = "fake"\nmodel = "deterministic-critic-v1"\n[fake]\nseconds = 0.2\n')
        (rust / "brief.md").write_text("Specify a small local project decision log with a meaningful retention choice.\n")
        (rust / "context.toml").write_text("schema_version = 1\n")
        rust_state = rust / ".wayward-wrangler/state"
        digest = hashlib.sha256((release / "manifest.json").read_bytes()).hexdigest()
        save(rust / ".wayward-wrangler/release.json", {"schema_version": 1,
            "project_key": key, "release_id": manifest["release_id"],
            "manifest_sha256": digest, "binding_generation": 1})
        ipc = private / "runtime/waywr" / f"{key}.sock"
        service_log = (root / "rust-service.log").open("wb")
        service = subprocess.Popen([str(release / "bin/waywr-service"), "--project", str(rust),
            "--state-dir", str(rust_state), "--project-key", key, "--release-id", manifest["release_id"],
            "--ipc", str(ipc), "--herdr-socket", str(endpoint), "--runner", str(release / "bin/waywr-runner"),
            "--frontend", str(rust_bin)], env=env, stdout=service_log, stderr=subprocess.STDOUT)
        service_log.close()
        wait(lambda: service.poll() is None and ipc.is_socket(), 10)
        attachment = service_call(str(ipc), key, manifest["release_id"], "ui.attach", {})
        owned_workspace = attachment["workspace_id"]
        common = ["--project", rust, "--state-dir", rust_state, "--registry-dir", private / "registry",
                  "--herdr-socket", endpoint, "--ipc-dir", private / "runtime"]
        run_cli(rust_bin, env, root, "rust-integration", [*common, "integration", "herdr", "install",
            "--destination", private / "review-plugin", "--socket", endpoint])
        preflight = run_cli(rust_bin, env, root, "rust-preflight", [*common, "preflight", "--run", "rust-run",
            "--brief", "brief.md", "--context", "context.toml", "--workspace", owned_workspace,
            "--socket", endpoint])
        assert json.loads(preflight.stdout)["admission"] == "simulated-ready"
        assert json.loads(preflight.stdout)["workspace"] == owned_workspace
        malformed = run_cli(rust_bin, env, root, "rust-malformed", [*common, "start", "--run", "bad",
            "--brief", "brief.md", "--artifcat", "draft"], allowed=(2,))
        assert "unexpected argument" in malformed.stderr
        assert not (rust_state / "runs/bad").exists()
        run_cli(rust_bin, env, root, "rust-start", [*common, "start", "--run", "rust-run",
            "--brief", "brief.md", "--workspace", owned_workspace, "--socket", endpoint],
            input_text="yes\n")
        def rust_gate(kind):
            snapshot = service_call(str(ipc), key, manifest["release_id"], "snapshot", {})
            task = next((v for v in snapshot["tasks"] if v["run_id"] == "rust-run"), None)
            if not task:
                return None
            detail = service_call(str(ipc), key, manifest["release_id"], "task.get", {"task_id": task["task_id"]})
            if task["status"] in ("blocked", "failed", "uncertain", "output-invalid"):
                raise AssertionError(f"Rust task stopped: {detail}")
            pending = detail.get("pending") or {}
            return detail if task["status"] == "awaiting-input" and pending.get("kind") == kind else None
        choice = wait(lambda: rust_gate("choice"))
        run_cli(rust_bin, env, root, "rust-status", [*common, "status", "rust-run", "--json"])
        run_cli(rust_bin, env, root, "rust-workers", [*common, "workers", "rust-run"])
        run_cli(rust_bin, env, root, "rust-observe", [*common, "observe", "rust-run", "--operation", "draft"])
        run_cli(rust_bin, env, root, "rust-notices", [*common, "notices", "rust-run"])
        run_cli(rust_bin, env, root, "rust-supervisor-install", [*common, "supervisor-install", "rust-run", "--socket", endpoint])
        run_cli(rust_bin, env, root, "rust-supervise-once", [*common, "supervise", "rust-run", "--socket", endpoint,
            "--once", "--interval", "2.0", "--max-attempts", "3"])
        run_cli(rust_bin, env, root, "rust-rebind", [*common, "rebind", "rust-run", "--workspace", owned_workspace])
        shown = run_cli(rust_bin, env, root, "rust-show", [*common, "show", "rust-run", "--artifact", "draft"])
        assert shown.stdout.encode() == (rust_state / "runs/rust-run/artifacts/draft.md").read_bytes()
        transcript = run_cli(rust_bin, env, root, "rust-logs", [*common, "logs", "rust-run", "--operation", "draft"])
        assert transcript.stdout.encode() == (rust_state / "runs/rust-run/operations/draft/transcript.log").read_bytes()
        answer = choice["pending"]["question"]["options"][0]
        (rust / "answer.txt").write_text(answer + "\n")
        run_cli(rust_bin, env, root, "rust-decide", [*common, "decide", "rust-run", "--request",
            choice["pending"]["request_id"], "--answer-file", "answer.txt", "--record-only"])
        time.sleep(.5)
        assert rust_gate("choice"), "record-only response advanced without explicit resume"
        run_cli(rust_bin, env, root, "rust-release-answer", [*common, "resume", "rust-run"])
        review = wait(lambda: rust_gate("final-review"), 90)
        sha = review["pending"]["artifact_sha256"]
        run_cli(rust_bin, env, root, "rust-approve", [*common, "approve", "rust-run", "--request",
            review["pending"]["request_id"], "--sha256", sha, "--record-only"])
        time.sleep(.5)
        assert rust_gate("final-review"), "record-only approval advanced without explicit resume"
        run_cli(rust_bin, env, root, "rust-release-approval", [*common, "resume", "rust-run"])
        wait(lambda: service_call(str(ipc), key, manifest["release_id"], "snapshot", {})["tasks"][-1]["status"] == "approved")
        counts = lambda: sum(json.loads(path.read_text())["count"] for path in
            (rust_state / "runs/rust-run/operations").glob("*/invocations.json"))
        before_replay = counts()
        start_id = json.loads((rust_state / "session/cli-start/rust-run.json").read_text())["command"]["command_id"]
        command_status = run_cli(rust_bin, env, root, "rust-command-status", [*common, "session",
            "command-status", start_id])
        assert json.loads(command_status.stdout)["result"]["status"] == "accepted"
        run_cli(rust_bin, env, root, "rust-command-replay", [*common, "session",
            "command-replay", start_id])
        run_cli(rust_bin, env, root, "rust-start-replay", [*common, "start", "--run", "rust-run",
            "--brief", "brief.md", "--workspace", owned_workspace, "--socket", endpoint])
        (rust / "other-context.toml").write_text("schema_version = 1\nchanged = true\n")
        changed = run_cli(rust_bin, env, root, "rust-start-changed-context", [*common, "start",
            "--run", "rust-run", "--brief", "brief.md", "--context", "other-context.toml",
            "--workspace", owned_workspace, "--socket", endpoint], allowed=(2,))
        assert "Frozen CLI start inputs differ" in changed.stderr
        assert counts() == before_replay == 5
        (rust / ".wayward-wrangler/workflow.toml").write_text(
            (rust / ".wayward-wrangler/workflow.toml").read_text().replace("seconds = 0.2", "seconds = 2.0"))
        run_cli(rust_bin, env, root, "rust-cancel-start", [*common, "start", "--run", "cancel-run",
            "--brief", "brief.md", "--workspace", owned_workspace, "--socket", endpoint], input_text="yes\n")
        draft = rust_state / "runs/cancel-run/operations/draft"
        wait(lambda: (draft / "permit.json").exists(), 30)
        run_cli(rust_bin, env, root, "rust-cancel", [*common, "cancel", "cancel-run", "--operation", "draft"])
        wait(lambda: (draft / "receipt.json").exists(), 15)
        assert json.loads((draft / "receipt.json").read_text())["status"] == "cancelled"
        count = json.loads((draft / "invocations.json").read_text())["count"] if (draft / "invocations.json").exists() else 0
        assert count <= 1, "Cancellation must not dispatch a duplicate worker"
        session_status = run_cli(rust_bin, env, root, "rust-session-status", [*common, "session", "status"])
        assert json.loads(session_status.stdout)["project_key"] == key
        run_cli(rust_bin, env, root, "rust-session-stop", [*common, "session", "stop"])
        service.wait(timeout=10)
        herdr(str(endpoint), "workspace.close", {"workspace_id": owned_workspace})
        wait(lambda: all(not Path(f"/proc/{json.loads(path.read_text())['process']['pid']}").exists()
                         for path in (rust_state / "session/ui-claims").glob("*.json")), 10)
        run_cli(rust_bin, env, root, "rust-location-rebind", [*common, "location-rebind"])
        relocated = root / "rust-project-relocated"
        shutil.copytree(rust, relocated)
        moved_state = relocated / ".wayward-wrangler/state"
        moved = ["--project", relocated, "--state-dir", moved_state, "--registry-dir", private / "registry",
                 "--herdr-socket", endpoint, "--ipc-dir", private / "runtime"]
        run_cli(rust_bin, env, root, "rust-location-relocated", [*moved, "location-rebind"])
        moved_status = json.loads(run_cli(rust_bin, env, root, "rust-relocated-status",
            [*moved, "status", "rust-run", "--json"]).stdout)
        assert moved_status["task"]["status"] == "approved"
        assert hashlib.sha256((moved_state / "runs/rust-run" / review["current"]["path"]).read_bytes()).hexdigest() == sha
        summary["checks"].extend(["Rust preflight and strict malformed-flag refusal",
            "Installed CLI start respected explicit run/workspace and did not duplicate on replay",
            "Artifact and transcript CLI bytes matched verified saved evidence",
            "Record-only decision and exact-hash approval held through resume and applied once",
            "Private integration installation, worker focus, service controls and quiescent relocation with unchanged approval bytes",
            "Targeted fake cancellation retained one receipt and no duplicate invocation"])

        baseline = root / "r9-project"
        baseline.mkdir()
        old_common = ["--color", "never", "--project", baseline, "--registry-dir", private / "r9-registry"]
        old_bin = R9 / "bin/waywr"
        run_cli(old_bin, env, root, "r9-init", [*old_common, "init"])
        (baseline / ".wayward-wrangler/workflow.toml").write_text(
            'review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "deterministic-author-v1"\n'
            '[roles.critic]\nprovider = "fake"\nmodel = "deterministic-critic-v1"\n[fake]\nseconds = 0.2\n')
        (baseline / "brief.md").write_text((rust / "brief.md").read_text())
        run_cli(old_bin, env, root, "r9-start", [*old_common, "start", "--run", "r9-run",
            "--brief", "brief.md", "--workspace", workspace, "--socket", endpoint], allowed=(2,))
        old_state = baseline / ".wayward-wrangler/state"
        old_choice = json.loads(run_cli(old_bin, env, root, "r9-status", [*old_common, "status", "r9-run", "--json"]).stdout)
        assert old_choice["status"] == "awaiting-input" and old_choice["pending"]["kind"] == "choice"
        run_cli(old_bin, env, root, "r9-workers", [*old_common, "workers", "r9-run"])
        old_show = run_cli(old_bin, env, root, "r9-show", [*old_common, "show", "r9-run", "--artifact", "draft"])
        assert (old_state / "runs/r9-run/artifacts/draft.md").read_text() in old_show.stdout
        old_logs = run_cli(old_bin, env, root, "r9-logs", [*old_common, "logs", "r9-run", "--operation", "draft"])
        assert (old_state / "runs/r9-run/operations/draft/transcript.log").read_text() in old_logs.stdout
        (baseline / "answer.txt").write_text(old_choice["pending"]["question"]["options"][0] + "\n")
        run_cli(old_bin, env, root, "r9-decide", [*old_common, "decide", "r9-run", "--request",
            old_choice["pending"]["request_id"], "--answer-file", "answer.txt", "--record-only"])
        held = json.loads(run_cli(old_bin, env, root, "r9-held", [*old_common, "status", "r9-run", "--json"]).stdout)
        assert held["status"] == "awaiting-input"
        run_cli(old_bin, env, root, "r9-resume", [*old_common, "resume", "r9-run"], allowed=(2,))
        old_review = json.loads(run_cli(old_bin, env, root, "r9-review", [*old_common, "status", "r9-run", "--json"]).stdout)
        assert old_review["pending"]["kind"] == "final-review"
        run_cli(old_bin, env, root, "r9-approve", [*old_common, "approve", "r9-run", "--request",
            old_review["pending"]["request_id"], "--sha256", old_review["pending"]["artifact_sha256"],
            "--record-only"])
        run_cli(old_bin, env, root, "r9-final-resume", [*old_common, "resume", "r9-run"])
        old_final = json.loads(run_cli(old_bin, env, root, "r9-final-status", [*old_common, "status", "r9-run", "--json"]).stdout)
        assert old_final["status"] == "approved"
        summary["checks"].append("Separate installed r9 fake fixture matched run targeting, artifact/log inspection, record-only gates and exact approval outcome")
        summary["observed_difference"] = "r9 fake preflight is native-only; Rust preflight accepts simulated profiles. Rust project service continues asynchronously while r9 resume drives a run synchronously."
        summary["rust_run"] = "rust-run"
        summary["r9_run"] = "r9-run"
        summary["status"] = "passed"
        save(root / "summary.json", summary)
    finally:
        if service is not None and service.poll() is None:
            service.terminate()
            try: service.wait(timeout=8)
            except subprocess.TimeoutExpired:
                service.kill(); service.wait(timeout=8)
        if server.poll() is None:
            server.terminate()
            try: server.wait(timeout=8)
            except subprocess.TimeoutExpired:
                server.kill(); server.wait(timeout=8)
        save(root / "cleanup.json", {"service_alive": service is not None and service.poll() is None,
                                     "private_server_alive": server.poll() is None,
                                     "private_runtime": str(private)})


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--release-root", type=Path, required=True)
    arguments = parser.parse_args()
    main(arguments.evidence_root.resolve(), arguments.release_root.resolve())
