"""Deterministic, single-run Herdr infrastructure reconciliation."""
from __future__ import annotations

import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import time
import uuid
from . import records
from .herdr_backend import Herdr, receipt
from .ownership import current as owner_current, next_owner
from .routing import controller_panes, recover_missing_workspace, rebind
from .storage import OperationalError, atomic, canonical, digest, identity, alive, lock, now, read
from .runtime import current


def root():
    return records.state_dir() / "supervision"


def event(run_id, cause, action, outcome, detail=""):
    entry = {"schema_version": 1, "run_id": run_id, "at": now(),
             "cause": cause, "action": action, "outcome": outcome, "detail": detail}
    directory = root() / "events"
    directory.mkdir(parents=True, exist_ok=True)
    atomic(directory / f"{time.time_ns()}-{uuid.uuid4().hex[:8]}.json", entry, immutable=True)
    return entry


def action_count(run_id, action):
    return sum(1 for file in (root() / "events").glob("*.json")
               if (record := read(file)).get("run_id") == run_id and record.get("action") == action)


def notice(run_id, status, detail, *, herdr=None, workspace=None):
    action = ("Inspect the saved output and finding IDs. No model retry or later stage is authorized from this result."
              if status == "output-invalid" else
              "Inspect status and supervision events; do not retry an ambiguous worker.")
    entry = {"schema_version": 1, "run_id": run_id, "status": status,
             "detail": detail, "operator_action": action,
             "at": now()}
    atomic(root() / "notices" / f"{time.time_ns()}-{uuid.uuid4().hex[:8]}.json", entry, immutable=True)
    if herdr:
        try:
            # Herdr's notification API is presentation only; the file above is authoritative.
            herdr.call("notification.show", {"title": f"Wayward Wrangler: {status}",
                       "body": f"{run_id}: {detail[:350]}", "sound": "request"})
        except (OSError, OperationalError, ValueError):
            pass
    return entry


def _pane_nodes(node):
    if node["type"] == "pane":
        yield node
    else:
        yield from _pane_nodes(node["first"])
        yield from _pane_nodes(node["second"])


def _same_process(left, right):
    return all(left.get(key) == right.get(key) for key in ("pid", "start", "boot"))


def controller_matches(h, token):
    found = []
    for tab in h.call("tab.list")["tabs"]:
        layout = h.call("layout.export", {"tab_id": tab["tab_id"]})["layout"]
        for node in _pane_nodes(layout["root"]):
            argv = node.get("command") or []
            if "--owner-token" in argv and argv.index("--owner-token") + 1 < len(argv) and argv[argv.index("--owner-token") + 1] == token:
                found.append({"pane_id": node["pane_id"], "tab_id": layout["tab_id"],
                              "workspace_id": layout["workspace_id"]})
    return found


def _workspace(h, run_id, target):
    try:
        return h.call("workspace.get", {"workspace_id": target["workspace"]})["workspace"], False
    except OperationalError as error:
        if "workspace_not_found" not in str(error):
            raise
    if action_count(run_id, "create-workspace") >= 3:
        raise OperationalError("Workspace recovery limit exhausted")
    created = h.call("workspace.create", {"label": f"Wayward Wrangler {current().project.name} {run_id}",
                   "cwd": str(current().project), "focus": False})["workspace"]
    return created, True


def _controller_claim(generation):
    path = root() / "controllers" / f"{generation:06d}" / "claim.json"
    return read(path) if path.exists() else None


def _assert_old_controller_gone(h, owner):
    matches = controller_matches(h, owner["token"])
    if len(matches) > 1:
        raise OperationalError("Duplicate supervised controller panes; manual identity review required")
    if matches:
        if matches[0]["workspace_id"] != owner["workspace"]:
            raise OperationalError("Controller moved to an unexpected workspace")
        return matches[0]
    claim = _controller_claim(owner["generation"])
    if claim and alive(claim["process"]):
        raise OperationalError("Controller process survives without its Herdr pane; refusing replacement")
    return None


def _launch_controller(h, run_id, workspace, server):
    selected = current()
    if selected is None or selected.legacy:
        raise OperationalError("Supervision requires an initialized target project")
    if action_count(run_id, "create-controller") >= 3:
        raise OperationalError("Controller replacement limit exhausted")
    token = uuid.uuid4().hex
    owner = next_owner(run_id=run_id, token=token, server=server, workspace=workspace)
    from sys import executable
    argv = [executable, "-m", "agent_workflow.cli", "--project", str(selected.project),
            "--state-dir", str(selected.state), "--registry-dir", str(selected.registry),
            "demo", "--run", run_id, "--workspace", workspace, "--socket", h.path,
            "--owner-generation", str(owner["generation"]), "--owner-token", token]
    intent = {"schema_version": 1, "owner": owner, "argv": argv, "at": now(),
              "status": "may-have-been-sent"}
    atomic(root() / "controllers" / f"{owner['generation']:06d}" / "intent.json", intent, immutable=True)
    layout = h.create(workspace, f"Wayward Wrangler {run_id} controller", argv, selected.project)
    event(run_id, "controller-missing", "create-controller", "created", layout["tab_id"])
    return layout


def reconcile_once(run_id, socket_path):
    """One bounded pass; never retries a worker whose launch may have been sent."""
    selected = current()
    if selected is None or selected.legacy:
        raise OperationalError("Supervision requires explicit initialized project and state")
    run = records.get_run(run_id)
    h = Herdr(socket_path)
    server = h.server_identity()  # validates peer uid, PID, start and boot
    target = records.effective_target(run_id)
    if target["socket"] != socket_path:
        raise OperationalError("Configured Herdr socket differs from frozen run socket")
    if run["status"] in records.TERMINAL:
        if run["status"] in ("failed", "output-invalid") and not any(
            read(path).get("run_id") == run_id and read(path).get("status") == run["status"]
            for path in (root() / "notices").glob("*.json")):
            notice(run_id, run["status"], run["detail"] or "A worker output could not be accepted", herdr=h)
        if run["status"] == "output-invalid":
            owner = owner_current()
            if owner and owner["run_id"] == run_id and _same_process(owner["server"], server):
                if _assert_old_controller_gone(h, owner):
                    return "quality-hold"
                # The task is stopped. A replacement controller only presents
                # status/review commands; drive() treats this state as terminal.
                h.call("workspace.get", {"workspace_id": target["workspace"]})
                _launch_controller(h, run_id, target["workspace"], server)
                return "quality-hold"
        return "terminal"
    owner = owner_current()
    existing = None
    if owner is None and controller_panes(h, run_id):
        raise OperationalError("Existing unmanaged controller pane; exit it before supervisor takeover")
    if owner:
        if owner["run_id"] != run_id:
            raise OperationalError("Another run owns the supervisor")
        if _same_process(owner["server"], server):
            existing = _assert_old_controller_gone(h, owner)
        else:
            claim = _controller_claim(owner["generation"])
            if claim and alive(claim["process"]):
                raise OperationalError("Old controller survives server replacement")
            raise OperationalError("Herdr server identity changed; saved workspace IDs cannot be reused automatically")
    if existing:
        if run["status"] == "uncertain":
            raise OperationalError(f"Controller exists but run is uncertain: {run['detail']}")
        recovery = records.run_dir(run_id) / "operations" / "critique-2" / "recovery/finding-interpretation.json"
        if run["status"] == "recovering" and recovery.exists() and snapshot_stage(run_id) == "critique-2":
            claim = _controller_claim(owner["generation"])
            if not claim or not alive(claim["process"]) or claim.get("pane_id") != existing["pane_id"]:
                raise OperationalError("Quality-held controller identity cannot be verified for replacement")
            if action_count(run_id, "retire-quality-controller") == 0:
                os.kill(claim["process"]["pid"], signal.SIGTERM)
                event(run_id, "finding-interpretation", "retire-quality-controller", "sent", existing["pane_id"])
            return "controller-retiring"
        return "controller-present"
    # Do not create infrastructure for ambiguous worker execution.
    from .workflow import state
    snapshot = state(run_id)
    for op in records.operations(run_id):
        directory = records.run_dir(run_id) / "operations" / op["stage"]
        if op["stage"] in snapshot.values.get("completed", []):
            if not receipt(directory):
                raise OperationalError("Completed stage receipt missing")
            continue
        if (directory / "launch.json").exists():
            # The graph driver, after replacement, will reconcile this *same*
            # launch only if the saved server and process evidence remain valid.
            launch = read(directory / "launch.json")
            if not _same_process(launch["server"], server):
                raise OperationalError("Launched worker belongs to another Herdr server; uncertain")
    workspace, created = _workspace(h, run_id, target)
    workspace_id = workspace["workspace_id"]
    if created:
        event(run_id, "workspace-not-found", "create-workspace", "created", workspace_id)
        with lock(records.state_dir() / "controller.lock"):
            if run["status"] == "uncertain":
                recovered = recover_missing_workspace(run_id, workspace_id,
                                                      generation=(owner["generation"] + 1 if owner else 1))
            elif snapshot.interrupts and snapshot.values.get("pending"):
                recovered = rebind(run_id, workspace_id, allow_answered=True)
            else:
                raise OperationalError("Missing workspace outside supported quiescent recovery boundary")
            records.set_status(run_id, "recovering", run["detail"])
            event(run_id, "workspace-not-found", "append-routing", "verified", canonical(recovered))
    if run["status"] == "uncertain" and not created:
        raise OperationalError("Uncertain run requires explicit verified recovery cause: " + str(run["detail"]))
    _launch_controller(h, run_id, workspace_id, server)
    return "controller-created"


def snapshot_stage(run_id):
    from .workflow import state
    return state(run_id).values.get("next_stage")


def serve(run_id, socket_path, *, interval=2.0, max_attempts=3, once=False):
    if not Path(socket_path).is_absolute() or max_attempts < 1 or not 0.2 <= interval <= 30:
        raise OperationalError("Supervisor requires absolute Herdr socket and bounded retry settings")
    with lock(root() / "supervisor.lock"):
        failures = 0
        last_error = ""
        while True:
            try:
                result = reconcile_once(run_id, socket_path)
                failures = 0
                last_error = ""
                if once:
                    return result
                if result == "terminal" and records.get_run(run_id)["status"] != "output-invalid":
                    return result
            except (OperationalError, OSError, ValueError, KeyError) as error:
                detail = str(error)
                event(run_id, "reconcile", "verify-or-recover", "refused", detail)
                failures = failures + 1 if detail == last_error else 1
                last_error = detail
                if failures >= max_attempts:
                    if records.get_run(run_id)["status"] != "uncertain":
                        records.set_status(run_id, "uncertain", f"Supervisor exhausted {failures} attempts: {detail}")
                    notice(run_id, "uncertain", detail, herdr=Herdr(socket_path))
                    return "uncertain"
                if once:
                    raise
            time.sleep(interval)


def install_service(run_id, socket_path):
    """One explicit, installed-package user service; no checkout symlink."""
    selected = current()
    if selected is None or selected.legacy:
        raise OperationalError("Select an initialized project")
    if not Path(socket_path).is_absolute():
        raise OperationalError("Herdr socket must be absolute")
    records.get_run(run_id)
    name = f"wayward-wrangler-{selected.key[:8]}-{records.name(run_id)}.service"
    # systemd's command parser treats spaces specially. Reject them rather
    # than emitting an ambiguous service definition.
    args = [sys.executable, "-m", "agent_workflow.cli", "--project", str(selected.project),
            "--state-dir", str(selected.state), "--registry-dir", str(selected.registry),
            "supervise", run_id, "--socket", socket_path]
    if any(" " in part or "\n" in part for part in args):
        raise OperationalError("Service paths with whitespace need explicit escaping")
    unit = ("[Unit]\nDescription=Wayward Wrangler supervised run " + run_id + "\n"
            "After=default.target\nStartLimitIntervalSec=60\nStartLimitBurst=4\n\n"
            "[Service]\nType=simple\nWorkingDirectory=" + str(selected.project) +
            "\nExecStart=" + " ".join(args) + "\nRestart=on-failure\nRestartSec=2\n"
            "SuccessExitStatus=3\n\n"
            "[Install]\nWantedBy=default.target\n")
    path = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "systemd/user" / name
    atomic(path, unit, immutable=True)
    for command in (["systemctl", "--user", "daemon-reload"],
                    ["systemctl", "--user", "enable", "--now", name]):
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode:
            raise OperationalError(f"User service setup failed ({command[-1]}): {result.stderr.strip()}")
    return {"unit": name, "path": str(path), "run_id": run_id, "socket": socket_path}
