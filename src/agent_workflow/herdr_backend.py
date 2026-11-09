from __future__ import annotations
import json
import os
from pathlib import Path
import socket
import struct
import sys
import time
import threading
import uuid
from .storage import ROOT, LOCAL, OperationalError, Uncertain, OutputInvalid, alive, atomic, canonical, digest, identity, lock, now, operation_path, read
from .runtime import current

STOP_OBSERVING = threading.Event()

class ControllerDetached(OperationalError):
    pass

def check_stop():
    from .ownership import check
    check()
    if STOP_OBSERVING.is_set():
        raise ControllerDetached("Controller stopped observing; worker remains in Herdr")

DEFAULT_SOCKET = str(Path.home() / ".config/herdr/herdr.sock")

class Herdr:
    def __init__(self, path=None):
        self.path = path or os.environ.get("HERDR_SOCKET_PATH", DEFAULT_SOCKET)

    def call(self, method, params=None):
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(8)
            client.connect(self.path)
            client.sendall((json.dumps({"id": uuid.uuid4().hex, "method": method, "params": params or {}}) + "\n").encode())
            line = client.makefile("rb").readline(4 * 1024 * 1024)
        response = json.loads(line)
        if "error" in response:
            raise OperationalError(f"Herdr {method}: {response['error']}")
        return response["result"]

    def server_identity(self):
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(5)
            client.connect(self.path)
            pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            if uid != os.getuid():
                raise OperationalError("Herdr socket is not owned by this user")
            return identity(pid)

    def create(self, workspace, label, command, cwd):
        return self.call("layout.apply", {"workspace_id": workspace, "tab_label": label,
            "focus": False, "root": {"type": "pane", "label": label,
            "cwd": str(cwd), "command": command}})["layout"]

    def find(self, token):
        matches = []
        for tab in self.call("tab.list")["tabs"]:
            layout = self.call("layout.export", {"tab_id": tab["tab_id"]})["layout"]
            def visit(node):
                if node["type"] == "pane":
                    argv = node.get("command") or []
                    if token in argv and "agent_workflow.runner" in argv:
                        matches.append({"pane_id": node["pane_id"], "tab_id": layout["tab_id"],
                            "workspace_id": layout["workspace_id"]})
                else:
                    visit(node["first"])
                    visit(node["second"])
            visit(layout["root"])
        return matches

def prepare(directory, packet, target):
    directory = operation_path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    atomic(directory / "packet.json", packet, immutable=True)
    op = {"schema_version": 1, "operation_id": packet["operation_id"],
          "run_id": packet["run_id"], "stage": packet["stage"], "role": packet["role"],
          "provider": packet.get("provider", "fake"), "model": packet["model"], "packet_sha256": digest(canonical(packet)),
          "launch_token": uuid.uuid4().hex, "target": target, "created": now()}
    selected = current()
    if selected:
        from .native import application_version
        op.update(project_key=selected.key, application_version=application_version(), protocol_version=1)
    if "max_invocations" in packet:
        op.update(max_invocations=packet["max_invocations"],
                  authorization_plan_sha256=packet["authorization_plan_sha256"])
    if (directory / "operation.json").exists():
        old = read(directory / "operation.json")
        if old["packet_sha256"] != op["packet_sha256"]:
            raise OperationalError("Existing stage has a different packet")
        return old
    atomic(directory / "operation.json", op, immutable=True)
    return op

def receipt(directory):
    directory = Path(directory)
    path = directory / "receipt.json"
    if not path.exists():
        return None
    op = read(directory / "operation.json")
    result = read(path)
    if any(result.get(k) != op[k] for k in ("operation_id", "packet_sha256", "launch_token")):
        raise Uncertain("Receipt identity mismatch")
    claim = read(directory / "claim.json")
    if result.get("runner") != claim["runner"]:
        raise Uncertain("Receipt runner mismatch")
    if result["status"] == "completed":
        raw = (directory / "result.json").read_bytes()
        if digest(raw) != result["result_sha256"]:
            raise Uncertain("Result hash mismatch")
        try:
            validate_result(json.loads(raw), read(directory / "packet.json"))
        except (OperationalError, ValueError, KeyError, TypeError) as error:
            raise OutputInvalid(f"Saved output invalid: {error}") from error
    elif result["status"] not in ("failed", "cancelled"):
        raise Uncertain("Unknown receipt outcome")
    if (directory / "recovery/adoption.json").exists():
        from .recovery import effective_receipt
        return effective_receipt(directory,result)
    return result

def validate_result(result, packet):
    if packet.get("operation_kind") == "coordinator":
        if set(result) != {"schema_version", "operation_id", "packet_sha256", "proposal"} or result["schema_version"] != 1:
            raise OperationalError("Invalid coordinator output envelope")
        if result["operation_id"] != packet["operation_id"] or result["packet_sha256"] != digest(canonical(packet)):
            raise OperationalError("Coordinator output refers to different frozen input")
        proposal = result["proposal"]
        if not isinstance(proposal, dict) or set(proposal) != {"schema_version", "action", "task_id", "task_version", "message_sha256", "brief"}:
            raise OperationalError("Invalid coordinator proposal contract")
        if proposal["schema_version"] != 1 or proposal["action"] != "propose_task" or any(proposal[key] != packet[key] for key in ("task_id", "task_version", "message_sha256")):
            raise OperationalError("Coordinator proposal references different task input")
        if not isinstance(proposal["brief"], str) or not proposal["brief"].strip() or len(proposal["brief"].encode()) > 32768:
            raise OperationalError("Coordinator brief is missing or oversized")
        return
    expected = {"schema_version", "operation_id", "packet_sha256", "content",
                "questions", "needs_revision"}
    expected |= ({"new_findings", "assessments"} if packet.get("finding_contract_version") == 2
                 else {"findings", "dispositions"})
    if set(result) != expected or result["schema_version"] != packet.get("schema_version",1):
        raise OperationalError("Invalid output schema; worker control/approval fields are forbidden")
    if result["operation_id"] != packet["operation_id"] or result["packet_sha256"] != digest(canonical(packet)):
        raise OperationalError("Result refers to different input")
    if not isinstance(result["content"], str) or not result["content"].strip():
        raise OperationalError("Empty result")
    if not isinstance(result["needs_revision"], bool):
        raise OperationalError("Invalid needs_revision")
    for key in (("new_findings", "assessments", "questions") if packet.get("finding_contract_version") == 2
                else ("findings", "dispositions", "questions")):
        if not isinstance(result[key], list):
            raise OperationalError(f"Invalid {key}")
    if packet.get("schema_version",1)==1 and packet.get("provider", "fake") != "fake" and result["questions"]:
        raise OperationalError("Unexpected worker question: this trial uses its controller-owned substantive gate")
    if packet.get("schema_version")==2:
        from .context import validate_questions
        validate_questions(result,packet)
    for q in result["questions"] if packet.get("schema_version",1)==1 else []:
        if set(q) != {"id", "question", "options", "recommendation", "consequences"}:
            raise OperationalError("Invalid question")
        if not isinstance(q["options"], list) or len(q["options"]) < 2:
            raise OperationalError("Question requires substantive options")
    if packet.get("finding_contract_version") == 2:
        for f in result["new_findings"]:
            if set(f) != {"severity", "evidence", "requested_change"} or f["severity"] not in ("minor", "substantive") or not all(
                    isinstance(f[key], str) and f[key].strip() for key in ("evidence", "requested_change")):
                raise OperationalError("Invalid new finding")
        for a in result["assessments"]:
            if set(a) != {"finding_ref", "assessment", "evidence", "rationale"} or a["assessment"] not in ("open", "proposed-addressed") or not all(
                    isinstance(a[key], str) and a[key].strip() for key in ("finding_ref", "evidence", "rationale")):
                raise OperationalError("Invalid finding assessment")
    else:
        for f in result["findings"]:
            if set(f) != {"id", "severity", "evidence", "requested_change"}:
                raise OperationalError("Invalid finding")
        for d in result["dispositions"]:
            if set(d) != {"id", "status", "rationale"} or d["status"] != "addressed":
                raise OperationalError("Workers may only propose addressed findings")

RECOVERY_ACTION = ("Inspect status, launch/claim/child records and retained output. "
                   "Do not delete evidence or retry dispatch; reconcile the original server/worker "
                   "or request explicit recovery after execution is resolved.")


def server_matches(h, expected=None):
    try:
        current = h.server_identity()
    except (OSError, ValueError) as error:
        raise Uncertain("Herdr server unavailable; no dispatch/permit. " + RECOVERY_ACTION) from error
    if expected and any(current[k] != expected[k] for k in ("pid", "start", "boot")):
        raise Uncertain("Herdr server changed; no automatic relaunch. " + RECOVERY_ACTION)
    return current


def inspect_operation(directory):
    """Read-only diagnostic; never issues permits, cancels or starts execution."""
    directory = Path(directory)
    try:
        done = receipt(directory)
        if done:
            return {"status": done["status"], "detail": "Validated immutable completion receipt", "operator_action": None}
        op = read(directory / "operation.json")
        claim = read(directory / "claim.json") if (directory / "claim.json").exists() else None
        child = read(directory / "child.json") if (directory / "child.json").exists() else None
        lives = {"runner_alive": bool(claim and alive(claim["runner"])),
                 "child_alive": bool(child and alive(child["identity"]))}
        if (directory / "uncertain.json").exists():
            raise Uncertain(read(directory / "uncertain.json")["error"])
        if claim and not lives["runner_alive"]:
            return {**lives, "status": "uncertain", "detail": "Runner disappeared without receipt" +
                    ("; recorded child is still alive" if lives["child_alive"] else ""),
                    "operator_action": RECOVERY_ACTION}
        launch = read(directory / "launch.json") if (directory / "launch.json").exists() else None
        if launch:
            server_matches(Herdr(op["target"]["socket"]), launch["server"])
            if not claim:
                raise Uncertain("Launch may have been sent; no runner claim")
        elif claim or any((directory / name).exists() for name in ("dispatch.json", "child.json", "permit.json")):
            raise Uncertain("Execution evidence exists without launch intent")
        return {**lives, "status": "active" if (directory / "permit.json").exists() else "awaiting-permit" if claim else "prepared",
                "detail": "No terminal receipt; resume reconciles existing execution", "operator_action": None}
    except (OperationalError, OSError, ValueError, KeyError) as error:
        return {"status": "uncertain", "detail": str(error), "operator_action": RECOVERY_ACTION}


def ensure(directory):
    """Caller serializes controller activity. A possibly sent creation is never retried."""
    check_stop()
    directory = operation_path(directory)
    done = receipt(directory)
    if done:
        return done
    if (directory / "uncertain.json").exists():
        raise Uncertain(read(directory / "uncertain.json")["error"])
    op = read(directory / "operation.json")
    launch_path = directory / "launch.json"
    # A prepared operation has immutable provenance but has not yet selected
    # its execution location. An appended run binding may relocate it.
    if launch_path.exists():
        target = read(launch_path).get("target", op["target"])
    else:
        from .records import effective_target
        try:
            target = effective_target(op["run_id"])
        except OperationalError as error:
            if not str(error).startswith("Unknown run "):
                raise
            # Foundation probes have operations but no workflow run record.
            target = op["target"]
    h = Herdr(target["socket"])
    server = server_matches(h)
    if launch_path.exists():
        launch = read(launch_path)
        if any(launch["server"][k] != server[k] for k in ("pid", "start", "boot")):
            raise Uncertain("Herdr server changed; no automatic relaunch. " + RECOVERY_ACTION)
    else:
        if any((directory / name).exists() for name in ("claim.json", "dispatch.json", "child.json", "permit.json")):
            raise Uncertain("Execution evidence exists without launch intent. " + RECOVERY_ACTION)
        h.call("workspace.get", {"workspace_id": target["workspace"]})
        from .ownership import fenced
        with fenced():
            check_stop()
            from .session_lifecycle import dispatch_guard
            with dispatch_guard():
                launch = {"status": "may-have-been-sent", "server": server, "target": target, "created": now()}
                atomic(launch_path, launch, immutable=True)
                selected = current()
                key = (selected.key[:8] + " ") if selected else ""
                label = f"Wayward Wrangler {key}{op['run_id']} {op['role']} {op['provider']}/{op['model']} {op['stage']}"
                argv = [sys.executable, "-m", "agent_workflow.runner",
                        "--operation-dir", str(directory), "--launch-token", op["launch_token"],
                        "--state-dir", str(selected.state if selected else LOCAL)]
                if selected:
                    argv += ["--project", str(selected.project), "--registry-dir", str(selected.registry)]
                h.create(target["workspace"], label, argv, directory)
    # Allow a newly started runner time to register, without retrying creation.
    deadline = time.monotonic() + 8
    while not (directory / "claim.json").exists() and time.monotonic() < deadline:
        check_stop()
        time.sleep(.1)
    if not (directory / "claim.json").exists():
        raise Uncertain("Creation may have succeeded, but no runner claim; no retry. " + RECOVERY_ACTION)
    claim = read(directory / "claim.json")
    if not alive(claim["runner"]):
        raise Uncertain("Runner disappeared without completion evidence. " + RECOVERY_ACTION)
    matches = h.find(op["launch_token"])
    if len(matches) != 1:
        raise Uncertain(f"Expected one matching worker pane, found {len(matches)}")
    binding = matches[0]
    if any(claim["herdr"].get(k) != binding[k] for k in binding):
        raise Uncertain("Runner and Herdr disagree about pane identity")
    if binding["workspace_id"] != target["workspace"]:
        raise Uncertain("Worker moved to an unexpected workspace")
    pane = h.call("pane.get", {"pane_id": binding["pane_id"]})["pane"]
    binding["terminal_id"] = pane["terminal_id"]
    saved = directory / "binding.json"
    if saved.exists() and read(saved) != binding:
        raise Uncertain("Herdr terminal binding changed")
    atomic(saved, binding, immutable=True)
    from .ownership import fenced
    with fenced():
        check_stop()
        server_matches(h, launch["server"])
        existing_permit = directory / "permit.json"
        if existing_permit.exists():
            permit = read(existing_permit)
            if permit.get("runner") != claim["runner"] or permit.get("launch_token") != op["launch_token"] or permit.get("binding") != binding:
                raise Uncertain("Existing dispatch permit differs from verified worker identity")
            return {"status": "active", "binding": binding, "runner": claim["runner"]}
        from .session_lifecycle import dispatch_guard
        with dispatch_guard():
            _permit_new(directory, op, claim, binding)
    return {"status": "active", "binding": binding, "runner": claim["runner"]}

def _permit_new(directory, op, claim, binding):
        packet=read(directory / "packet.json")
        from . import records
        frozen_input=records.run_dir(op["run_id"]) / "inputs/envelope.json"
        envelope_imports=read(frozen_input).get("cross_imports",[]) if frozen_input.exists() else []
        if envelope_imports:
            from .context_access import check_frozen
            selected=current()
            if not selected:
                raise OperationalError("Cross-project permit has no selected project")
            from .storage import lock
            with lock(selected.state / "session/admission.lock"):
                expected=[{"source_project_key":entry["item"]["source_project_key"],
                           "grant_id":entry["grant_id"],"purpose":entry["purpose"],
                           "item":entry["item"]} for entry in envelope_imports]
                if packet.get("cross_project_imports")!=expected:
                    raise OperationalError("Execution packet differs from frozen imported context")
                check_frozen(selected,envelope_imports)
                atomic(directory / "permit.json", {"runner": claim["runner"], "launch_token": op["launch_token"],
                                                  "binding": binding}, immutable=True)
        else:
            if packet.get("cross_project_imports"):
                raise OperationalError("Execution packet adds an unauthorized foreign import")
            atomic(directory / "permit.json", {"runner": claim["runner"], "launch_token": op["launch_token"],
                                              "binding": binding}, immutable=True)

def wait(directory, *, detach=False):
    status = ensure(directory)
    if detach or status["status"] != "active":
        return status
    print(f"Observing existing worker {status['binding']['pane_id']}", flush=True)
    while True:
        check_stop()
        done = receipt(directory)
        if done:
            return done
        if (Path(directory) / "uncertain.json").exists():
            raise Uncertain(read(Path(directory) / "uncertain.json")["error"])
        claim = read(Path(directory) / "claim.json")
        if not alive(claim["runner"]):
            raise Uncertain("Runner exited without receipt. " + RECOVERY_ACTION)
        time.sleep(.2)

def cancel(directory, source="controller"):
    directory = operation_path(directory)
    done = receipt(directory)
    if done:
        return done
    claim = read(directory / "claim.json")
    if not alive(claim["runner"]):
        raise Uncertain("Cannot cancel: runner ownership unresolved")
    atomic(directory / "cancel.json", {"source": source, "runner": claim["runner"], "time": now()})
    return {"status": "cancellation-requested"}

def observe(directory):
    op = read(Path(directory) / "operation.json")
    h = Herdr(op["target"]["socket"])
    matches = h.find(op["launch_token"])
    if len(matches) != 1:
        raise Uncertain("Existing worker pane cannot be uniquely located")
    return h.call("pane.focus", {"pane_id": matches[0]["pane_id"]})
