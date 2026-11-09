"""Opt-in, isolated Rust project-service and Herdr UI proof; fake workers only.

The harness owns its private Herdr server and service. It never targets the
operator's Herdr socket, project state or systemd units.
"""
import argparse
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
BIN = ROOT / "target/debug"
HERDR = Path.home() / ".local/bin/herdr"
ACTIVE_RELEASE = "rust-stage5-test"


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def wait(test, seconds=20):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = test()
        if value:
            return value
        time.sleep(.05)
    raise AssertionError("Timed out awaiting isolated Rust service evidence")


def call_herdr(endpoint, method, params=None):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(8)
        peer.connect(endpoint)
        peer.sendall((json.dumps({"id": uuid.uuid4().hex, "method": method,
                                  "params": params or {}}) + "\n").encode())
        response = json.loads(peer.makefile("rb").readline())
    if "error" in response:
        raise RuntimeError(f"Herdr {method}: {response['error']}")
    return response["result"]


def call_service(endpoint, key, kind, body):
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(10)
        peer.connect(endpoint)
        stream = peer.makefile("rb")
        def request(name, payload):
            peer.sendall((json.dumps({"v": 1, "kind": name, "request_id": uuid.uuid4().hex,
                                      "body": payload}) + "\n").encode())
            return json.loads(stream.readline())
        hello = request("hello", {"protocol": {"major": 1, "minor": 0},
                                  "project_key": key, "release_id": ACTIVE_RELEASE,
                                  "channel": "command"})
        assert hello["body"]["accepted"] is True, hello
        return request(kind, body)


def main(root, installed=False, two_projects=False, missing_workspace=False, exit_active=False, release_root=None):
    global ACTIVE_RELEASE
    assert not root.exists(), "Use a fresh evidence directory"
    root.mkdir(parents=True)
    private = Path(tempfile.mkdtemp(prefix="ww-rust-m5-"))
    for name in ("home", "config/herdr", "runtime"):
        (private / name).mkdir(parents=True)
    (private / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    herdr_socket = str(private / "runtime/herdr.sock")
    env = {**os.environ, "HOME": str(private / "home"),
           "XDG_CONFIG_HOME": str(private / "config"), "XDG_RUNTIME_DIR": str(private / "runtime"),
           "HERDR_SOCKET_PATH": herdr_socket, "SHELL": "/bin/sh", "TERM": "xterm-256color"}
    env.pop("HERDR_ENV", None)
    env.pop("NO_COLOR", None)
    project = root / "project"
    (project / ".wayward-wrangler").mkdir(parents=True)
    key = uuid.uuid4().hex if release_root is not None else "a" * 32
    (project / ".wayward-wrangler/project.toml").write_text(f'schema_version = 1\nproject_key = "{key}"\n')
    (project / ".wayward-wrangler/workflow.toml").write_text(
        'review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "deterministic-author-v1"\n'
        '[roles.critic]\nprovider = "fake"\nmodel = "deterministic-critic-v1"\n[fake]\nseconds = '+('3.0' if exit_active else '0.1')+'\n')
    state = project / ".wayward-wrangler/state"
    ipc = private / "runtime/waywr" / f"{key}.sock"
    binaries = BIN
    if release_root is not None:
        release=release_root.resolve()
        manifest=json.loads((release/"manifest.json").read_text())
        ACTIVE_RELEASE=manifest["release_id"]
        binaries=release/"bin"
        installed=True
    elif installed:
        release = root / "releases/rust-stage5-test"
        binaries = release / "bin"
        binaries.mkdir(parents=True)
        for name in ("waywr", "waywr-service", "waywr-runner"):
            shutil.copy2(BIN / name, binaries / name)
        manifest = {"schema_version": 1, "release_id": "rust-stage5-test", "frontend": "0.4.0",
                    "backend": "0.1.0", "backend_kind": "rust", "protocol_major": 1,
                    "state_schema_min": 2, "state_schema_max": 2}
        save(release / "manifest.json", manifest)
        import hashlib
        digest = hashlib.sha256((release / "manifest.json").read_bytes()).hexdigest()
        save(project / ".wayward-wrangler/release.json", {"schema_version": 1,
             "project_key": key, "release_id": ACTIVE_RELEASE, "manifest_sha256": digest,
             "binding_generation": 1})
    if release_root is not None:
        import hashlib
        digest=hashlib.sha256((release/"manifest.json").read_bytes()).hexdigest()
        save(project/".wayward-wrangler/release.json",{"schema_version":1,
            "project_key":key,"release_id":ACTIVE_RELEASE,"manifest_sha256":digest,
            "binding_generation":1})
    server_log = (root / "herdr-server.log").open("wb")
    server = subprocess.Popen([str(HERDR), "server"], env=env, stdout=server_log,
                              stderr=subprocess.STDOUT)
    server_log.close()
    service = None
    service_b = None
    checks = []
    try:
        wait(lambda: server.poll() is None and Path(herdr_socket).is_socket())
        service_log = (root / "service.log").open("wb")
        service = subprocess.Popen([str(binaries / "waywr-service"), "--project", str(project),
                                    "--state-dir", str(state), "--project-key", key,
                                    "--release-id", ACTIVE_RELEASE, "--ipc", str(ipc),
                                    "--herdr-socket", herdr_socket, "--runner", str(binaries / "waywr-runner"),
                                    "--frontend", str(binaries / "waywr")], env=env,
                                   stdout=service_log, stderr=subprocess.STDOUT)
        service_log.close()
        wait(lambda: service.poll() is None and ipc.is_socket())
        if installed:
            launch_workspace = call_herdr(herdr_socket, "workspace.create", {"label":"Installed Rust launcher proof",
                "cwd":str(project),"focus":False})["workspace"]["workspace_id"]
            call_herdr(herdr_socket, "layout.apply", {"workspace_id":launch_workspace,
                "tab_label":"Installed waywr launch", "focus":False,
                "root":{"type":"pane","label":"Installed waywr launch","cwd":str(project),
                    "command":[str(binaries / "waywr"),"--project",str(project),
                        "--herdr-socket",herdr_socket,"--ipc-dir",str(private / "runtime")]}})
            binding = wait(lambda: json.loads((state / "session/ui-current.json").read_text())
                           if (state / "session/ui-current.json").exists() else None)
            pane = binding["pane"]["pane_id"]
            checks.append("Installed waywr attached from a managed Herdr pane without Python")
        else:
            attached = call_service(str(ipc), key, "ui.attach", {})
            assert attached["kind"] == "ui.attach_result", attached
            pane = attached["body"]["pane_id"]
        try:
            wait(lambda: (state / "session/ui-claims").exists()
                 and list((state / "session/ui-claims").glob("*.json")))
        except AssertionError:
            # Keep the terminal's own failure evidence before the private server stops.
            try:
                save(root / "ui-pane-on-failure.json", call_herdr(herdr_socket,
                    "pane.read", {"pane_id": pane, "source": "recent", "lines": 80}))
            except RuntimeError as error:
                save(root / "ui-pane-on-failure.json", {"error": str(error)})
            raise
        checks.append("UI pane created and registered through supported Herdr IPC")
        first = call_service(str(ipc), key, "command", {"command_id": "first", "action": "submit_message",
            "expected_state_revision": 0, "payload": {"text": "Specify a local project decision log."}})
        assert first["body"]["status"] == "accepted", first
        task_id = first["body"]["task_id"]
        def proposed():
            result = call_service(str(ipc), key, "task.get", {"task_id": task_id})
            return result if result["body"]["task"]["status"] == "proposed" else None
        detail = wait(proposed, 35)
        checks.append("Fake coordinator completed in a separately visible Herdr worker pane")
        plan = call_service(str(ipc), key, "task.plan", {"task_id": task_id})
        assert plan["kind"] == "task.plan_result", plan
        save(root / "proposal.json", detail["body"])
        save(root / "plan.json", plan["body"])
        admission = call_service(str(ipc), key, "command", {
            "command_id": "authorize", "action": "admit_task", "expected_task_version": 1,
            "expected_state_revision": plan["body"]["plan"]["task_state_revision"],
            "payload": {"task_id": task_id, "run_id": plan["body"]["plan"]["run_id"],
                        "workspace": plan["body"]["plan"]["workspace"],
                        "profile": plan["body"]["plan"]["profile"],
                        "context": None, "plan_digest": plan["body"]["plan_digest"]}})
        assert admission["body"]["status"] == "accepted", admission
        if exit_active:
            draft=state/"runs"/("r-"+task_id)/"operations/draft"
            wait(lambda:(draft/"permit.json").exists(),35)
            started=time.monotonic()
            before_stop=call_service(str(ipc),key,"snapshot",{})["body"]
            assert time.monotonic()-started<1.0,"Status blocked behind active worker"
            stopping=call_service(str(ipc),key,"command",{"command_id":"active-exit",
                "action":"stop_session","expected_state_revision":before_stop["state_revision"],"payload":{}})
            assert stopping["body"]["status"]=="accepted",stopping
            service.wait(timeout=20)
            assert (draft/"receipt.json").is_file(),"Active worker receipt must survive foreground drain"
            service_log=(root/"service-after-active-exit.log").open("wb")
            service=subprocess.Popen([str(binaries/"waywr-service"),"--project",str(project),
                "--state-dir",str(state),"--project-key",key,"--release-id",ACTIVE_RELEASE,
                "--ipc",str(ipc),"--herdr-socket",herdr_socket,
                "--runner",str(binaries/"waywr-runner"),"--frontend",str(binaries/"waywr")],
                env=env,stdout=service_log,stderr=subprocess.STDOUT)
            service_log.close()
            wait(lambda:service.poll() is None and ipc.is_socket())
            paused=call_service(str(ipc),key,"snapshot",{})["body"]
            resumed=call_service(str(ipc),key,"command",{"command_id":"active-resume",
                "action":"resume_session","expected_state_revision":paused["state_revision"],"payload":{}})
            assert resumed["body"]["status"]=="accepted",resumed
            assert len(list(draft.glob("receipt.json")))==1
            checks.append("Exit drained an active worker and restart reused its one receipt")
        def gate(kind):
            result = call_service(str(ipc), key, "task.get", {"task_id": task_id})
            status = result["body"]["task"]["status"]
            if status in ("uncertain", "blocked", "failed", "output-invalid"):
                raise AssertionError(f"Workflow stopped at {status}: {result}")
            return result if status == "awaiting-input" and (result["body"].get("pending") or {}).get("kind") == kind else None
        first_gate = wait(lambda: gate("choice"), 60)
        if missing_workspace:
            binding = json.loads((state / "session/ui-current.json").read_text())
            old_workspace = binding["pane"]["workspace_id"]
            run_id = "r-" + task_id
            envelope_file = state / "runs" / run_id / "inputs/envelope.json"
            original_envelope = envelope_file.read_bytes()
            call_herdr(herdr_socket, "workspace.close", {"workspace_id": old_workspace})
            renewed = call_service(str(ipc), key, "ui.attach", {})
            assert renewed["kind"] == "ui.attach_result", renewed
            assert renewed["body"]["workspace_id"] != old_workspace
            assert renewed["body"].get("routing_notice") is None, renewed
            assert envelope_file.read_bytes() == original_envelope
            routing = list((state / "runs" / run_id / "routing/bindings").glob("*.json"))
            assert len(routing) == 1
            route = json.loads(routing[0].read_text())
            assert route["target"]["workspace"] == renewed["body"]["workspace_id"]
            save(root / "routing.json", route)
            checks.append("Missing owned workspace reconnected and appended a route without rewriting frozen run evidence")
        if two_projects:
            other = root / "project-b"
            (other / ".wayward-wrangler").mkdir(parents=True)
            other_key = uuid.uuid4().hex if release_root is not None else "b" * 32
            (other / ".wayward-wrangler/project.toml").write_text(
                f'schema_version = 1\nproject_key = "{other_key}"\n')
            shutil.copy2(project / ".wayward-wrangler/workflow.toml",
                         other / ".wayward-wrangler/workflow.toml")
            other_state = other / ".wayward-wrangler/state"
            other_ipc = private / "runtime/waywr" / f"{other_key}.sock"
            other_log = (root / "service-b.log").open("wb")
            service_b = subprocess.Popen([str(binaries / "waywr-service"), "--project", str(other),
                    "--state-dir", str(other_state), "--project-key", other_key,
                    "--release-id", ACTIVE_RELEASE, "--ipc", str(other_ipc),
                    "--herdr-socket", herdr_socket, "--runner", str(binaries / "waywr-runner"),
                    "--frontend", str(binaries / "waywr")], env=env,
                    stdout=other_log, stderr=subprocess.STDOUT)
            other_log.close()
            wait(lambda: service_b.poll() is None and other_ipc.is_socket())
            attached_b = call_service(str(other_ipc), other_key, "ui.attach", {})
            assert attached_b["body"]["workspace_id"] != attached["body"]["workspace_id"] if not installed else True
            submitted_b = call_service(str(other_ipc), other_key, "command", {
                "command_id": "first", "action": "submit_message", "expected_state_revision": 0,
                "payload": {"text": "Specify a local project decision log."}})
            assert submitted_b["body"]["task_id"] == task_id
            detail_b = wait(lambda: (lambda x: x if x["body"]["task"]["status"] == "proposed" else None)(
                call_service(str(other_ipc), other_key, "task.get", {"task_id": task_id})), 35)
            assert gate("choice")["body"]["task"]["status"] == "awaiting-input"
            coordinator_a = next((state / "session/coordinator" / task_id).glob("*/packet.json")).read_text()
            coordinator_b = next((other_state / "session/coordinator" / task_id).glob("*/packet.json")).read_text()
            assert other_key not in coordinator_a and key not in coordinator_b
            cancelled_b = call_service(str(other_ipc), other_key, "command", {
                "command_id": "cancel-b", "action": "cancel_task",
                "expected_task_version": detail_b["body"]["task"]["version"],
                "expected_state_revision": detail_b["body"]["task"]["state_revision"],
                "payload": {"task_id": task_id}})
            assert cancelled_b["body"]["status"] == "accepted", cancelled_b
            assert gate("choice")["body"]["task"]["status"] == "awaiting-input"
            snapshot_b = call_service(str(other_ipc), other_key, "snapshot", {})["body"]
            stop_b = call_service(str(other_ipc), other_key, "command", {
                "command_id": "stop-b", "action": "stop_session",
                "expected_state_revision": snapshot_b["state_revision"], "payload": {}})
            assert stop_b["body"]["status"] == "accepted", stop_b
            service_b.wait(timeout=15)
            checks.append("Project B proposed and cancelled its identical task ID while A awaited input; packets stayed separate")
        saved = first_gate["body"]
        pending = saved["pending"]
        answer = call_service(str(ipc), key, "command", {
            "command_id": "test-choice", "action": "answer_task",
            "expected_task_version": saved["task"]["version"],
            "expected_state_revision": saved["task"]["state_revision"],
            "payload": {"task_id": task_id, "request_id": pending["request_id"],
                        "outcome": "answered", "answer": pending["question"]["options"][0],
                        "artifact_sha256": pending["artifact_sha256"]}})
        assert answer["body"]["status"] == "accepted", answer
        final_gate = wait(lambda: gate("final-review"), 90)
        checks.append("Five-slot fake workflow reached final review with one durable human choice")
        save(root / "final-gate.json", final_gate["body"])
        snapshot = call_service(str(ipc), key, "snapshot", {})
        save(root / "snapshot.json", snapshot["body"])
        launch = list((state / "session/coordinator" / task_id).glob("*/launch.json"))
        receipt = list((state / "session/coordinator" / task_id).glob("*/receipt.json"))
        assert len(launch) == len(receipt) == 1
        save(root / "coordinator-receipt.json", json.loads(receipt[0].read_text()))
        stop = call_service(str(ipc), key, "command", {"command_id": "exit", "action": "stop_session",
            "expected_state_revision": snapshot["body"]["state_revision"], "payload": {}})
        assert stop["body"]["status"] == "accepted", stop
        service.wait(timeout=15)
        checks.append("Service exit preserved proposal, receipts, answer and pending final gate")
        service_log = (root / "service-restarted.log").open("wb")
        service = subprocess.Popen([str(binaries / "waywr-service"), "--project", str(project),
                                    "--state-dir", str(state), "--project-key", key,
                                    "--release-id", ACTIVE_RELEASE, "--ipc", str(ipc),
                                    "--herdr-socket", herdr_socket, "--runner", str(binaries / "waywr-runner"),
                                    "--frontend", str(binaries / "waywr")], env=env,
                                   stdout=service_log, stderr=subprocess.STDOUT)
        service_log.close()
        wait(lambda: service.poll() is None and ipc.is_socket()
             and call_service(str(ipc), key, "snapshot", {}))
        before = call_service(str(ipc), key, "task.get", {"task_id": task_id})["body"]
        assert before["pending"]["kind"] == "final-review"
        snapshot = call_service(str(ipc), key, "snapshot", {})["body"]
        resumed = call_service(str(ipc), key, "command", {"command_id": "resume", "action": "resume_session",
            "expected_state_revision": snapshot["state_revision"], "payload": {}})
        assert resumed["body"]["status"] == "accepted", resumed
        after = call_service(str(ipc), key, "task.get", {"task_id": task_id})["body"]
        assert after["pending"] == before["pending"]
        bad = call_service(str(ipc), key, "command", {"command_id": "stale-approval", "action": "answer_task",
            "expected_task_version": after["task"]["version"],
            "expected_state_revision": after["task"]["state_revision"],
            "payload": {"task_id": task_id, "request_id": after["pending"]["request_id"],
                        "outcome": "approved", "answer": "Fixture approval",
                        "artifact_sha256": "0" * 64}})
        assert bad["body"]["status"] == "rejected", bad
        approved = call_service(str(ipc), key, "command", {"command_id": "fixture-approval",
            "action": "answer_task", "expected_task_version": after["task"]["version"],
            "expected_state_revision": after["task"]["state_revision"],
            "payload": {"task_id": task_id, "request_id": after["pending"]["request_id"],
                        "outcome": "approved", "answer": "Fixture approval",
                        "artifact_sha256": after["pending"]["artifact_sha256"]}})
        assert approved["body"]["status"] == "accepted", approved
        wait(lambda: (lambda x: x if x["body"]["task"]["status"] == "approved" else None)(
            call_service(str(ipc), key, "task.get", {"task_id": task_id})))
        checks.append("Fresh service consumed the saved answer and exact fixture approval once")
        snapshot = call_service(str(ipc), key, "snapshot", {})["body"]
        stop_again = call_service(str(ipc), key, "command", {"command_id": "exit-after-test",
            "action": "stop_session", "expected_state_revision": snapshot["state_revision"], "payload": {}})
        assert stop_again["body"]["status"] == "accepted", stop_again
        service.wait(timeout=15)
        save(root / "summary.json", {"checks": checks, "ui_pane": pane, "task_id": task_id,
             "service_exit": service.returncode, "private_herdr_socket": herdr_socket,
             "project_key":key,"release_id":ACTIVE_RELEASE,
             "artifact_sha256": after["pending"]["artifact_sha256"],"installed_launch":installed})
        print(root / "summary.json")
    finally:
        if service is not None and service.poll() is None:
            service.terminate(); service.wait(timeout=8)
        if service_b is not None and service_b.poll() is None:
            service_b.terminate(); service_b.wait(timeout=8)
        if server.poll() is None:
            try: call_herdr(herdr_socket, "server.stop")
            except (OSError, RuntimeError): server.terminate()
            server.wait(timeout=8)
        save(root / "cleanup.json", {"service_stopped": service is None or service.poll() is not None,
             "service_b_stopped": service_b is None or service_b.poll() is not None,
             "server_stopped": server.poll() is not None})


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--installed-launch", action="store_true")
    parser.add_argument("--two-projects", action="store_true")
    parser.add_argument("--missing-workspace", action="store_true")
    parser.add_argument("--exit-active", action="store_true")
    parser.add_argument("--release-root",type=Path)
    args=parser.parse_args()
    main(args.evidence_root.resolve(), args.installed_launch, args.two_projects, args.missing_workspace,args.exit_active,args.release_root)
