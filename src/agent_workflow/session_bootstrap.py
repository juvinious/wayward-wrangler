"""Project setup and service lifecycle behind the Rust launcher."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time
import uuid

from .runtime import bind, load, register
from .shell import selection
from .storage import OperationalError, atomic, lock, read


def _ipc(key, override=None):
    root = Path(override or os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")) / "waywr"
    return root / f"{key}.sock"


def _request(ipc, key, release, kind, payload):
    from .session_service import encode, parse_frame
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(4)
        peer.connect(str(ipc))
        _pid, uid, _gid = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if uid != os.geteuid():
            raise OperationalError("Project service UID mismatch")
        peer.sendall(encode("hello", "bootstrap-hello", {"protocol": {"major": 1, "minor": 0},
                     "release_id": release, "project_key": key, "client_id": "bootstrap", "channel": "command"}))
        hello = parse_frame(peer.makefile("rb").readline(1024 * 1024 + 1))
        if hello["kind"] != "hello_result" or not hello["body"].get("accepted"):
            raise OperationalError("Project service protocol/release/project mismatch")
        peer.sendall(encode(kind, "bootstrap-action", payload))
        response = parse_frame(peer.makefile("rb").readline(1024 * 1024 + 1))
        if response["kind"] == "error":
            raise OperationalError(f"Project service {response['body']}")
        return response["body"]


def _unit(key):
    return f"waywr-{key}.service"


def prepare(args):
    project = Path(args.project).expanduser().resolve()
    config = project / ".wayward-wrangler/project.toml"
    if not config.exists() and not args.setup:
        return {"status": "setup_required", "project": str(project),
                "configuration": str(config), "state": str(project / ".wayward-wrangler/state")}
    fresh = not config.exists()
    paths = load(project, args.state_dir, args.registry_dir, initialize=fresh)
    bind(paths)
    register(paths)
    from .release_maintenance import pending_binding
    if pending_binding(paths)[0]:
        raise OperationalError("Project has an incomplete release binding; reconcile with its exact session bind/upgrade/rollback action")
    herdr = selection(paths, args.herdr_socket)
    descriptor = project / ".wayward-wrangler/release.json"
    manifest_path = Path(args.frontend).resolve().parent.parent / "manifest.json"
    manifest = read(manifest_path)
    if manifest.get("release_id") != args.release_id or manifest.get("protocol_major") != 1:
        raise OperationalError("Paired release manifest identity/protocol mismatch")
    manifest_sha = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    if descriptor.exists():
        binding = read(descriptor)
        if (binding.get("schema_version") != 1 or binding.get("project_key") != paths.key or
            binding.get("release_id") != args.release_id or
            binding.get("manifest_sha256",manifest_sha) != manifest_sha):
            raise OperationalError("Project is bound to another release; route to its compatible executable")
    else:
        if not fresh:
            raise OperationalError("Existing project has no release binding; use explicit 'waywr --project DIR session bind --release ID' before attach")
        atomic(descriptor, {"schema_version": 1, "project_key": paths.key, "release_id": args.release_id,
                            "manifest_sha256":manifest_sha,"binding_generation": 1}, immutable=True)
    ipc = _ipc(paths.key, args.ipc_dir)
    unit = _unit(paths.key)
    root = paths.state / "session"
    root.mkdir(parents=True, exist_ok=True)
    with lock(root / "start.lock"):
        try:
            _request(ipc, paths.key, args.release_id, "snapshot", {})
        except (OSError, OperationalError, ValueError):
            # A paused service can still be completing its cooperative stop
            # after the durable phase has changed. Starting it while systemd
            # reports active/deactivating is a no-op and loses the resume race.
            settling = time.monotonic() + 10
            while True:
                state = subprocess.run(["systemctl", "--user", "show", "--property=ActiveState",
                                        "--value", unit], capture_output=True, text=True)
                if state.returncode:
                    raise OperationalError(f"Could not inspect selected project unit {unit}: {state.stderr.strip()}")
                if state.stdout.strip() in ("inactive", "failed"):
                    break
                if state.stdout.strip() == "active":
                    try:
                        _request(ipc, paths.key, args.release_id, "snapshot", {})
                        break
                    except (OSError, OperationalError, ValueError):
                        pass
                if time.monotonic() >= settling:
                    raise OperationalError(f"Selected project unit {unit} did not settle for safe restart; inspect its journal")
                time.sleep(.1)
            if state.stdout.strip() == "active":
                started = subprocess.CompletedProcess([], 0, "", "")
            else:
                service_command = [sys.executable, "-I", "-m", "agent_workflow.session_service",
                       "--project", str(paths.project), "--state-dir", str(paths.state),
                       "--registry-dir", str(paths.registry), "--ipc", str(ipc),
                       "--herdr-socket", herdr, "--release-id", args.release_id,
                       "--frontend", str(Path(args.frontend).resolve())]
                loaded = subprocess.run(["systemctl", "--user", "show", "--property=LoadState",
                                     "--value", unit], capture_output=True, text=True)
                if loaded.returncode == 0 and loaded.stdout.strip() == "loaded":
                    existing = subprocess.run(["systemctl", "--user", "show", "--property=ExecStart",
                                           "--value", unit], capture_output=True, text=True)
                    rendered = existing.stdout
                    if existing.returncode or not all(part in rendered for part in service_command):
                        raise OperationalError("Loaded project unit command differs from this bound release/project; start refused")
                    started = subprocess.run(["systemctl", "--user", "start", unit], capture_output=True, text=True)
                elif loaded.returncode == 0 and loaded.stdout.strip() == "not-found":
                    command = ["systemd-run", "--user", "--collect", f"--unit={unit}",
                           "--property=Restart=on-failure", "--property=RestartSec=1s",
                           "--property=StartLimitBurst=3", "--property=StartLimitIntervalSec=30",
                               *service_command]
                    started = subprocess.run(command, capture_output=True, text=True)
                else:
                    raise OperationalError(f"Could not inspect selected project unit {unit}: {loaded.stderr.strip()}")
            if started.returncode:
                raise OperationalError(f"Could not start project unit {unit}: {started.stderr.strip()}")
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                try:
                    _request(ipc, paths.key, args.release_id, "snapshot", {})
                    break
                except (OSError, OperationalError, ValueError):
                    time.sleep(.1)
            else:
                raise OperationalError(f"Project unit {unit} did not become ready; inspect its journal")
    return {"status": "ready", "project": str(paths.project), "project_key": paths.key,
            "state": str(paths.state), "registry": str(paths.registry), "ipc": str(ipc),
            "unit": unit, "herdr_socket": herdr, "release_id": args.release_id}


def attach(args):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            return _request(args.ipc, args.project_key, args.release_id, "ui.attach", {})
        except (FileNotFoundError, ConnectionRefusedError, TimeoutError, OperationalError) as error:
            if isinstance(error, OperationalError) and not any(marker in str(error) for marker in ("No such file", "Connection refused")):
                raise
            time.sleep(.1)
    raise OperationalError("Herdr attachment did not complete within 30 seconds")


def lifecycle(args):
    """Durable CLI outbox for a lost response; the service owns all transitions."""
    ready = prepare(args)
    if ready["status"] != "ready":
        return ready
    paths = load(ready["project"], args.state_dir, args.registry_dir)
    pending = paths.state / "session/cli-lifecycle-pending.json"
    with lock(paths.state / "session/cli-lifecycle.lock"):
        if pending.exists():
            body = read(pending)
            if body.get("action") != args.action:
                raise OperationalError("Another lifecycle command has an unresolved reply; reconcile it first")
        else:
            snapshot = _request(ready["ipc"], ready["project_key"], args.release_id, "snapshot", {})
            body = {"command_id": "lifecycle-" + uuid.uuid4().hex, "action": args.action,
                    "expected_state_revision": snapshot["state_revision"], "payload": {}}
            atomic(pending, body)
        try:
            result = _request(ready["ipc"], ready["project_key"], args.release_id,
                              "command_get", {"command_id": body["command_id"]})
            if result.get("status") == "not_found":
                result = _request(ready["ipc"], ready["project_key"], args.release_id, "command", body)
        except (OSError, TimeoutError):
            return {"status": "delivery_uncertain", "command_id": body["command_id"],
                    "action": args.action, "operator_action": "Run the same command again to reconcile its saved ID"}
        if result.get("code") == "service_stopping":
            return {"status": "delivery_uncertain", "command_id": body["command_id"],
                    "action": args.action, "operator_action": "Run the same command again after the selected project service restarts"}
        if result.get("status") == "accepted" or result.get("status") == "rejected":
            pending.unlink()
        return {**result, "project": ready["project"], "project_key": ready["project_key"],
                "state": ready["state"], "unit": ready["unit"]}


def coordinator_recovery(args):
    """Explicit CLI command with a durable ID; no replacement without proof."""
    ready=prepare(args)
    if ready["status"]!="ready":return ready
    paths=load(ready["project"],args.state_dir,args.registry_dir)
    pending=paths.state/"session/cli-coordinator-recovery"/(args.task_id+".json")
    with lock(paths.state/"session/cli-coordinator-recovery.lock"):
        if pending.exists():
            body=read(pending)
            if body.get("payload",{}).get("task_id")!=args.task_id:
                raise OperationalError("Saved coordinator recovery target differs")
        else:
            snapshot=_request(ready["ipc"],ready["project_key"],args.release_id,"snapshot",{})
            detail=_request(ready["ipc"],ready["project_key"],args.release_id,
                            "task.get",{"task_id":args.task_id})
            body={"command_id":"coordinator-recovery-"+uuid.uuid4().hex,
                  "action":"recover_coordinator","expected_state_revision":snapshot["state_revision"],
                  "expected_task_version":detail["task"]["version"],"payload":{"task_id":args.task_id}}
            atomic(pending,body,immutable=True)
        try:
            result=_request(ready["ipc"],ready["project_key"],args.release_id,
                            "command_get",{"command_id":body["command_id"]})
            if result.get("status")=="not_found":
                result=_request(ready["ipc"],ready["project_key"],args.release_id,"command",body)
        except (OSError,TimeoutError):
            return {"status":"delivery_uncertain","task_id":args.task_id,
                    "operator_action":"Repeat the same command to reconcile its saved ID"}
        if result.get("status") in ("accepted","rejected"):
            pending.unlink()
        return {**result,"project":ready["project"],"task_id":args.task_id}


def main(argv=None):
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    start = sub.add_parser("prepare")
    start.add_argument("--project", required=True)
    start.add_argument("--state-dir")
    start.add_argument("--registry-dir")
    start.add_argument("--herdr-socket")
    start.add_argument("--ipc-dir")
    start.add_argument("--release-id", required=True)
    start.add_argument("--frontend", required=True)
    start.add_argument("--setup", action="store_true")
    move = sub.add_parser("attach")
    move.add_argument("--ipc", required=True)
    move.add_argument("--project-key", required=True)
    move.add_argument("--release-id", required=True)
    lifecycle_command = sub.add_parser("lifecycle")
    lifecycle_command.add_argument("--action", required=True, choices=("stop_session", "resume_session"))
    lifecycle_command.add_argument("--project", required=True)
    lifecycle_command.add_argument("--state-dir")
    lifecycle_command.add_argument("--registry-dir")
    lifecycle_command.add_argument("--herdr-socket")
    lifecycle_command.add_argument("--ipc-dir")
    lifecycle_command.add_argument("--release-id", required=True)
    lifecycle_command.add_argument("--frontend", required=True)
    recovery = sub.add_parser("coordinator-recovery")
    recovery.add_argument("--task-id",required=True)
    for flag in ("--project","--release-id","--frontend"):
        recovery.add_argument(flag,required=True)
    for flag in ("--state-dir","--registry-dir","--herdr-socket","--ipc-dir"):
        recovery.add_argument(flag)
    maintenance = sub.add_parser("maintenance")
    maintenance.add_argument("--action", required=True, choices=("status","stop","bind","upgrade","rollback"))
    maintenance.add_argument("--project", required=True)
    maintenance.add_argument("--state-dir")
    maintenance.add_argument("--registry-dir")
    maintenance.add_argument("--catalog", required=True)
    maintenance.add_argument("--release-id")
    options = parser.parse_args(argv)
    try:
        if options.command == "prepare":result = prepare(options)
        elif options.command == "attach":result = attach(options)
        elif options.command == "lifecycle":result = lifecycle(options)
        elif options.command == "coordinator-recovery":result = coordinator_recovery(options)
        else:
            from .release_maintenance import manage
            result=manage(options.action,options.project,options.state_dir,options.registry_dir,
                          options.catalog,options.release_id)
    except (OperationalError, OSError, ValueError) as error:
        print(json.dumps({"status": "error", "message": str(error)}))
        return 2
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
