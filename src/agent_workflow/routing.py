"""Append-only workspace relocation for future operations; never rewrite past identity."""
from . import artifacts, records
from .herdr_backend import Herdr, receipt
from .storage import OperationalError, alive, canonical, digest, now, read
from .runtime import current

def controller_panes(h, run_id):
    found = []
    for tab in h.call("tab.list")["tabs"]:
        layout = h.call("layout.export", {"tab_id": tab["tab_id"]})["layout"]
        def visit(node):
            if node["type"] != "pane":
                visit(node["first"])
                visit(node["second"])
                return
            argv = node.get("command") or []
            # Only our explicit controller invocation, not a pane's display label.
            is_controller = any(a.endswith(("/awp", "/wayward-wrangler")) or a in ("awp", "wayward-wrangler", "agent_workflow.cli") for a in argv)
            if is_controller and "demo" in argv and "--run" in argv:
                pos = argv.index("--run")
                if pos + 1 < len(argv) and argv[pos + 1] == run_id:
                    selected = current()
                    if selected and "--project" in argv:
                        project = argv[argv.index("--project") + 1]
                        if project == str(selected.project):
                            found.append(node["pane_id"])
                    elif selected is None or selected.legacy:
                        found.append(node["pane_id"])
        visit(layout["root"])
    return found

def rebind(run_id, workspace, *, allow_answered=False):
    """Caller must hold controller.lock. Does not invoke or update the graph."""
    from .workflow import state
    run = records.get_run(run_id)
    snapshot = state(run_id)
    pending = snapshot.values.get("pending")
    if run["status"] != "awaiting-input" or not snapshot.interrupts or not pending:
        raise OperationalError("Rebind requires an unanswered human gate")
    if records.response(run_id, pending["request_id"]) is not None and not allow_answered:
        raise OperationalError("Gate already answered; rebind will not consume or alter its response")
    original = records.effective_target(run_id)
    target = {"socket": original["socket"], "workspace": workspace}
    h = Herdr(original["socket"])
    verified = h.call("workspace.get", {"workspace_id": workspace})["workspace"]
    server = h.server_identity()
    if controller_panes(h, run_id):
        raise OperationalError("Exit the existing demo controller before rebinding; its panes will not be moved")
    ops = records.operations(run_id)
    completed = snapshot.values.get("completed", [])
    if {o["stage"] for o in ops} != set(completed):
        raise OperationalError("Reserved/incomplete execution exists; refusing workspace rebind")
    evidence = []
    for op in ops:
        directory = records.run_dir(run_id) / "operations" / op["stage"]
        done = receipt(directory)
        if not done or done["status"] != "completed":
            raise OperationalError("Every recorded operation must have a valid completed receipt")
        for file, field in (("claim.json", "runner"), ("child.json", "identity")):
            path = directory / file
            if path.exists() and alive(read(path)[field]):
                raise OperationalError("Live execution/retention process exists; preserve it before rebinding")
        artifact = read(records.run_dir(run_id) / "artifacts" / (op["stage"] + ".json"))["manifest"]
        artifacts.content(run_id, artifact)
        evidence.append({"operation_id": op["operation_id"], "stage": op["stage"],
                         "receipt_sha256": digest((directory / "receipt.json").read_bytes())})
    artifacts.content(run_id, snapshot.values["current"])
    latest = records.latest_rebinding(run_id)
    if original == target:
        if latest:
            records.export_rebinding(run_id, latest)
        return {"status": "already-bound", "target": target, "pending": pending["request_id"]}
    payload = {"schema_version": 1, "run_id": run_id, "from": original, "to": target,
               "workspace_label": verified["label"], "server": server, "recorded_at": now(),
               "pending_request": pending["request_id"], "checkpoint": snapshot.config,
               "graph_state_sha256": digest(canonical(snapshot.values)),
               "completed_evidence": evidence,
               "scope": "future operations only; original envelope, bindings, receipts and decisions unchanged"}
    return {"status": "rebound", "record": records.append_rebinding(run_id, payload)}


def recover_missing_workspace(run_id, workspace, *, generation):
    """Retarget only a provably unlaunched prepared operation.

    Caller holds controller.lock. Immutable operation inputs and old bindings
    stay in place; future execution resolves the appended run routing record.
    """
    from pathlib import Path
    from .workflow import state
    run = records.get_run(run_id)
    if run["status"] not in ("uncertain", "recovering") or "workspace_not_found" not in (run["detail"] or ""):
        raise OperationalError("Recovery requires the recorded missing-workspace failure")
    snapshot = state(run_id)
    if snapshot.interrupts or snapshot.values.get("pending"):
        raise OperationalError("Recovery cannot skip a human gate")
    original = records.effective_target(run_id)
    h = Herdr(original["socket"])
    server = h.server_identity()
    verified = h.call("workspace.get", {"workspace_id": workspace})["workspace"]
    target = {"socket": original["socket"], "workspace": workspace}
    if original == target:
        return {"status": "already-bound", "target": target}
    # The old workspace must really be gone. A transient lookup failure is not
    # grounds to move a possibly active worker.
    try:
        h.call("workspace.get", {"workspace_id": original["workspace"]})
    except OperationalError as error:
        if "workspace_not_found" not in str(error):
            raise
    else:
        raise OperationalError("Original workspace still exists; relocation refused")
    completed = set(snapshot.values.get("completed", []))
    incomplete = []
    evidence = []
    for op in records.operations(run_id):
        directory = records.run_dir(run_id) / "operations" / op["stage"]
        operation = read(directory / "operation.json")
        if operation["packet_sha256"] != op["packet_hash"]:
            raise OperationalError("Reserved packet hash differs from operation")
        packet = read(directory / "packet.json")
        if digest(canonical(packet)) != operation["packet_sha256"]:
            raise OperationalError("Frozen packet integrity failure")
        done = receipt(directory)
        if op["stage"] in completed:
            if not done or done["status"] != "completed":
                raise OperationalError("Completed stage lacks a validated receipt")
            evidence.append({"stage": op["stage"], "receipt_sha256": digest((directory / "receipt.json").read_bytes())})
            continue
        if done or any((directory / name).exists() for name in
                       ("launch.json", "claim.json", "dispatch.json", "child.json", "permit.json",
                        "execution.json", "invocations.json", "result.json", "runtime-result.json",
                        "uncertain.json", "binding.json")):
            raise OperationalError("Unresolved operation has execution evidence; automatic relocation refused")
        if h.find(operation["launch_token"]):
            raise OperationalError("Herdr still lists a pane for the prepared operation")
        incomplete.append(op["stage"])
    if len(incomplete) != 1:
        raise OperationalError("Expected exactly one prepared, unlaunched stage")
    if not snapshot.values.get("current"):
        raise OperationalError("Missing current artifact")
    artifacts.content(run_id, snapshot.values["current"])
    if len(controller_panes(h, run_id)):
        raise OperationalError("Existing controller pane found; relocation refused")
    payload = {"schema_version": 1, "run_id": run_id, "from": original, "to": target,
               "workspace_label": verified["label"], "server": server, "recorded_at": now(),
               "generation": generation, "cause": "workspace_not_found-before-launch",
               "prepared_stage": incomplete[0], "completed_evidence": evidence,
               "graph_state_sha256": digest(canonical(snapshot.values)),
               "scope": "future execution only; immutable operation and decision provenance retained"}
    return {"status": "recovered", "record": records.append_rebinding(run_id, payload)}
