import difflib
from .storage import ROOT, OperationalError, atomic, canonical, digest, read
from .records import run_dir

def content(run_id, ref):
    if ref is None:
        return ""
    root = run_dir(run_id).resolve()
    path = (root / ref["path"]).resolve()
    if not path.is_relative_to(root) or not path.is_file() or digest(path.read_bytes()) != ref["sha256"]:
        raise OperationalError("Artifact path/hash validation failed")
    return path.read_text()

def publish(run_id, stage, result, packet_hash, parents):
    directory = run_dir(run_id) / "artifacts"
    path = directory / (stage + ".md")
    atomic(path, result["content"], immutable=True)
    ref = {"schema_version": 1, "id": stage, "kind": stage.split("-")[0],
           "path": str(path.relative_to(run_dir(run_id))), "sha256": digest(result["content"]),
           "packet_sha256": packet_hash, "operation_id": result["operation_id"],
           "parents": [r["id"] for r in parents if r]}
    atomic(directory / (stage + ".json"), {"manifest": ref, "result": result}, immutable=True)
    return ref

def packet(envelope, state, stage):
    config = envelope["config"]
    role = "critic" if stage.startswith("critique") else "author"
    role_config = config["roles"][role]
    current = content(state["run_id"], state.get("current"))
    previous = content(state["run_id"], state.get("last_reviewed"))
    delta = ""
    if stage == "critique-2":
        delta = "".join(difflib.unified_diff(previous.splitlines(True), current.splitlines(True),
                                            fromfile="previous-reviewed", tofile="current-revision"))
    task = "critique" if stage.startswith("critique") else ("revise" if stage.startswith("revision") else "draft")
    result = {"schema_version": 1, "run_id": state["run_id"], "operation_id": state["run_id"] + "-" + stage,
            "stage": stage, "role": role, "provider": role_config["provider"], "model": role_config["model"],
            "seconds": config.get("fake", {}).get("seconds", 0), "mode": "success",
            "fake_question_count": config.get("fake", {}).get("questions", 1),
            "brief": envelope["brief"], "instructions": envelope["instructions"], "skill": envelope["skills"][task],
            "rules_sha256": envelope["rules_sha256"], "config_sha256": envelope["config_sha256"],
            "current_draft": current, "current_ref": state.get("current"),
            "previous_reviewed": previous if stage == "critique-2" else "",
            "delta": delta, "decisions": state.get("decisions", []),
            "open_findings": [f for f in state.get("findings", {}).values() if f["status"] == "open"],
            "finding_dispositions": state.get("findings", {}),
            "critique": state.get("last_critique"),
            "authority": "Return text and findings only. Never approve, execute tools, or reopen accepted decisions."}

    if role_config["provider"] != "fake":
        result.update(execution=role_config, planned_decision=config.get("decision"),
                      remaining_stage_budget=(config["max_invocations"] if envelope.get("native_authorization") else 5)-len(state.get("completed",[])))
        if envelope.get("native_authorization"):
            result.update(max_invocations=config["max_invocations"],
                          authorization_plan_sha256=envelope["native_authorization"]["plan_sha256"],
                          prompt_contract=config["prompt_contract"])
    if envelope.get("workflow_version",1)==2:
        from .context import effective_decisions, evidence_ids
        result.update(schema_version=2, context_snapshot=envelope["context_snapshot"],
                      context_digest=envelope["context_digest"], decisions=effective_decisions(state),
                      decision_revision=state["decision_revision"],
                      new_question_budget=max(0,config.get("max_questions",1)-state.get("questions_asked",0)),
                      authority="Return provisional text/findings/questions only. Human decisions remain controller-owned. Reconsider inherited policy only with referenced new evidence.")
        result.pop("planned_decision",None)
        result["evidence_ids"]=evidence_ids(result)
    if stage in ("critique-2", "revision-2") and state.get("coverage_gaps", {}).get("revision-1"):
        result["revision_coverage"] = {"revision-1": state["coverage_gaps"]["revision-1"]}
        result["authority"] += " Revision-1 omitted dispositions for the listed revision_coverage findings. Their original IDs and evidence remain open; inspect the provisional text and explicitly assess each."
    if state.get("finding_contract_version", 1) == 2:
        result["finding_contract_version"] = 2
        result["finding_assessments"] = state.get("finding_assessments", [])
        result["authority"] += (" New findings have no worker ID; the application assigns durable IDs. "
                                "Assess existing findings only through explicit finding_ref entries. "
                                "Original evidence is immutable; assessments never authorize a human decision.")
    if envelope.get("cross_imports"):
        result["cross_project_imports"]=[{"source_project_key":entry["item"]["source_project_key"],
                                         "grant_id":entry["grant_id"],"purpose":entry["purpose"],
                                         "item":entry["item"]} for entry in envelope["cross_imports"]]
    return result
