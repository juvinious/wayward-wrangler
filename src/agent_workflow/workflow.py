from __future__ import annotations
from . import terminal as ui
# Explicitly disable external tracing for this local-only prototype.
import os
os.environ["LANGSMITH_TRACING"] = "false"
os.environ["LANGCHAIN_TRACING_V2"] = "false"
import sqlite3
from typing import TypedDict
from langgraph.graph import END, START, StateGraph
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command, interrupt
from . import artifacts, records
from .storage import OperationalError, Uncertain, OutputInvalid, canonical, digest, read
from .herdr_backend import ControllerDetached, check_stop, prepare, wait

class State(TypedDict, total=False):
    run_id: str
    next_stage: str
    completed: list
    current: dict | None
    last_reviewed: dict | None
    last_critique: dict | None
    findings: dict
    decisions: list
    decision_revision: int
    questions: list
    pending: dict | None
    outcome: str
    workflow_version: int
    inherited_decisions: list
    context_digest: str
    questions_asked: int
    max_questions: int
    revision_required: bool
    blocked_reason: str
    question_origin: dict
    coverage_gaps: dict
    finding_assessments: list
    finding_contract_version: int

class StageStopped(OperationalError):
    def __init__(self, status):
        self.status = status
        super().__init__(f"Worker {status}; no automatic retry or continuation")

def worker(state: State):
    check_stop()
    run_id = state["run_id"]
    envelope = records.get_run(run_id)["envelope"]
    from .context import check
    check(envelope)
    if envelope.get("cross_imports"):
        from .runtime import current
        from .context_access import check_frozen
        check_frozen(current(),envelope["cross_imports"])
    stage = state["next_stage"]
    if stage not in records.STAGES or stage in state["completed"]:
        raise OperationalError("Completed or invalid stage cannot dispatch")
    packet = artifacts.packet(envelope, state, stage)
    packet_hash = digest(canonical(packet))
    from .session_lifecycle import dispatch_guard
    with dispatch_guard():
        records.reserve(run_id, stage, packet["operation_id"], packet_hash)
        directory = records.run_dir(run_id) / "operations" / stage
        prepare(directory, packet, records.effective_target(run_id))
    print(ui.paint(f"\n{run_id}: {stage} — {packet['role']} {packet['provider']}/{packet['model']}", packet["role"]), flush=True)
    done = wait(directory)
    if done["status"] != "completed":
        raise StageStopped("output-invalid" if done["status"] == "failed" and
                           str(done.get("error", "")).startswith("Output invalid:") else done["status"])
    result = read(directory / "result.json")
    from . import findings as finding_protocol
    interpretation = None
    interpretation_path = directory / "recovery/finding-interpretation.json"
    if interpretation_path.exists():
        from .finding_recovery import verify_interpretation
        interpretation = verify_interpretation(run_id, state, packet)
    try:
        classified = finding_protocol.classify(stage, state.get("findings", {}), packet, result, interpretation)
    except OutputInvalid as error:
        from .quality import record_invalid_output
        record_invalid_output(run_id, stage, packet_hash, result, str(error))
        raise
    missing = classified["missing"]
    if stage.startswith("revision") and (directory / "recovery/quality-adoption.json").exists():
        from .quality_recovery import verify_adoption
        verify_adoption(run_id, stage, state, packet, missing)
    ref = artifacts.publish(run_id, stage, result, packet_hash, [state.get("current"), state.get("last_reviewed")])
    findings = finding_protocol.apply(state.get("findings", {}), classified, ref, stage, state.get("current"))
    finding_protocol.record(run_id, stage, classified)
    assessment_history = state.get("finding_assessments", []) + classified["assessments"]
    gaps = dict(state.get("coverage_gaps", {}))
    if stage.startswith("revision"):
        gaps[stage] = missing
        if missing:
            from .quality import record_gap
            record_gap(run_id, stage, packet_hash, result, missing)
    if state.get("workflow_version")==2:
        return {**context_change(state, result, ref, packet_hash, findings), "coverage_gaps": gaps,
                "finding_assessments": assessment_history,
                "finding_contract_version": 2 if interpretation else state.get("finding_contract_version", 1)}
    known = {r["question"]["id"]: r for r in state["decisions"]}
    questions = []
    for q in result["questions"]:
        if q["id"] in known:
            if q != known[q["id"]]["question"]:
                raise OperationalError("Worker conflicts with an accepted decision")
        else:
            questions.append(q)
    next_stage = {"draft": "critique-1", "critique-1": "revision-1",
                  "revision-1": "critique-2", "critique-2": "revision-2", "revision-2": "final"}[stage]
    real_trial = envelope["config"].get("trial_id") is not None
    if real_trial and stage == "critique-1":
        questions = [envelope["config"]["decision"]]
    if stage.startswith("critique") and not result["needs_revision"] and not (real_trial and stage == "critique-1"):
        next_stage = "final"
    change = {"completed": state["completed"] + [stage], "findings": findings, "coverage_gaps": gaps,
              "finding_assessments": assessment_history,
              "questions": questions, "next_stage": next_stage}
    if stage.startswith("critique"):
        change.update(last_reviewed=state.get("current"), last_critique={"artifact": ref, "result": result})
    else:
        change["current"] = ref
    print(ui.paint(f"Published {ref['path']} sha256={ref['sha256']}", "success"), flush=True)
    return change

def route(state):
    if state.get("blocked_reason"):
        return "blocked_gate"
    if state["questions"]:
        return "prepare_choice"
    if state["next_stage"] == "final":
        return "prepare_final"
    return "worker"

def prepare_choice(state):
    question = state["questions"][0]
    request = {"request_id": "D-" + records.name(question["id"]), "kind": "choice", "question": question,
               "artifact_sha256": state["current"]["sha256"], "decision_revision": state["decision_revision"],
               "dependent_stage": state["next_stage"]}
    if state.get("workflow_version")==2:
        request.update(request_id="D-"+digest(canonical({"origin":state["question_origin"],"id":question["id"]}))[:24],
                       context_digest=state["context_digest"], source=state["question_origin"],
                       inherited_decisions=state["inherited_decisions"])
    return {"pending": records.request(state["run_id"], request)}

def choice_gate(state):
    returned = interrupt(state["pending"])
    saved = records.response(state["run_id"], state["pending"]["request_id"])
    if saved is None or returned != saved:
        raise OperationalError("Human response must first be recorded through the controller")
    records.export_response(state["run_id"], saved)
    change = {"decisions": state["decisions"] + [{**saved, "question": state["questions"][0]}],
            "decision_revision": state["decision_revision"] + 1, "questions": state["questions"][1:], "pending": None}
    if state.get("workflow_version")==2:
        if saved.get("context_digest")!=state["context_digest"]:
            raise OperationalError("Answer context binding changed")
        change["revision_required"]=True
        if state["next_stage"]=="final":
            change["blocked_reason"]="Answer saved, but no author revision slot remains; separately authorized follow-up required"
    return change

def prepare_final(state):
    if state.get("workflow_version")==2 and state.get("revision_required"):
        raise OperationalError("Current artifact predates the accepted answer")
    artifacts.content(state["run_id"], state["current"])
    request = {"request_id": "A1", "kind": "final-review", "artifact_sha256": state["current"]["sha256"],
               "artifact": state["current"], "decision_revision": state["decision_revision"],
               "last_reviewed": state.get("last_reviewed"), "findings": state["findings"],
               "decisions": state["decisions"],
               "notice": "Final revisions may not have been independently re-reviewed; inspect last_reviewed. Open findings remain unresolved unless explicitly addressed; review provisional sections before approval.",
               "coverage_gaps": state.get("coverage_gaps", {})}
    request["finding_assessments"] = state.get("finding_assessments", [])
    if state.get("workflow_version")==2:
        from .context import effective_decisions
        request.update(context_digest=state["context_digest"], decisions=effective_decisions(state))
    unresolved = [key for key, value in state["findings"].items() if value["status"] == "open"]
    if unresolved:
        request["notice"] += " Provisional specification: unresolved findings " + ", ".join(unresolved) + "."
    return {"pending": records.request(state["run_id"], request)}

def final_gate(state):
    returned = interrupt(state["pending"])
    saved = records.response(state["run_id"], state["pending"]["request_id"])
    if saved is None or returned != saved:
        raise OperationalError("Approval must be recorded through controller")
    artifacts.content(state["run_id"], state["current"])
    if saved["artifact_sha256"] != state["current"]["sha256"] or saved["decision_revision"] != state["decision_revision"]:
        raise OperationalError("Approval no longer matches artifact/decisions")
    if state.get("workflow_version")==2 and saved.get("context_digest")!=state["context_digest"]:
        raise OperationalError("Approval context mismatch")
    records.export_response(state["run_id"], saved)
    return {"outcome": saved["outcome"], "pending": None}

def blocked_gate(state):
    records.set_status(state["run_id"], "blocked", state["blocked_reason"])
    return {"outcome":"blocked", "pending":None}


def build(saver):
    graph = StateGraph(State)
    for function in (worker, prepare_choice, choice_gate, prepare_final, final_gate, blocked_gate):
        graph.add_node(function.__name__, function)
    graph.add_edge(START, "worker")
    graph.add_conditional_edges("worker", route)
    graph.add_edge("prepare_choice", "choice_gate")
    graph.add_conditional_edges("choice_gate", route)
    graph.add_edge("prepare_final", "final_gate")
    graph.add_edge("final_gate", END)
    graph.add_edge("blocked_gate", END)
    return graph.compile(checkpointer=saver)

def initial(run_id, envelope=None):
    result = {"run_id": run_id, "next_stage": "draft", "completed": [], "current": None,
            "last_reviewed": None, "last_critique": None, "findings": {}, "decisions": [],
            "decision_revision": 0, "questions": [], "pending": None, "outcome": "", "coverage_gaps": {},
            "finding_assessments": [], "finding_contract_version": envelope.get("config", {}).get("finding_contract_version", 1) if envelope else 1}
    if envelope and envelope.get("workflow_version")==2:
        from .context import inherited
        snapshot=envelope["context_snapshot"]
        result.update(workflow_version=2, inherited_decisions=inherited(snapshot),
                      context_digest=envelope["context_digest"], questions_asked=0,
                      max_questions=envelope["config"].get("max_questions",1),
                      revision_required=False, blocked_reason="", question_origin={})
        result["findings"]={e["qualified_id"]:e["content"] for e in snapshot["entries"] if e["kind"]=="findings"}
    return result


def state(run_id):
    with sqlite3.connect(records.db_path(), check_same_thread=False) as conn:
        graph = build(SqliteSaver(conn))
        return graph.get_state({"configurable": {"thread_id": run_id}})

def drive(run_id):
    from .ownership import check as check_owner
    check_owner()
    record = records.get_run(run_id)
    if record["envelope"].get("protocol_version", 1) != 1:
        raise OperationalError("Run protocol is incompatible with this installation; no dispatch")
    if record["envelope"].get("native_authorization"):
        from .native import application_version
        if record["envelope"].get("application_version") != application_version():
            raise OperationalError("Installed controller version differs from frozen native run; no dispatch")
    if record["status"] in records.TERMINAL:
        print(f"{run_id}: {record['status']} (terminal; no workers launched)", flush=True)
        return state(run_id)
    if record["status"] == "uncertain":
        raise Uncertain("Execution or application state is uncertain; use verified recovery before continuation")
    if record["status"]=="blocked":
        return state(run_id)
    from .context import check
    check(record["envelope"])
    config = {"configurable": {"thread_id": run_id}, "recursion_limit": 60}
    with sqlite3.connect(records.db_path(), check_same_thread=False, timeout=10) as conn:
        conn.execute("PRAGMA synchronous=FULL")
        graph = build(SqliteSaver(conn))
        current = graph.get_state(config)
        argument = None if current.values else initial(run_id,record["envelope"])
        if current.interrupts:
            pending = current.values["pending"]
            saved = records.response(run_id, pending["request_id"])
            if saved is None:
                records.set_status(run_id, "awaiting-input")
                return current
            argument = Command(resume=saved)
        records.set_status(run_id, "running")
        try:
            graph.invoke(argument, config, durability="sync")
        except StageStopped as error:
            records.set_status(run_id, error.status, str(error))
            raise
        except (KeyboardInterrupt, ControllerDetached):
            records.set_status(run_id, "paused", "Controller exited; reconcile worker on resume")
            raise
        except Exception as error:
            records.set_status(run_id, "output-invalid" if isinstance(error, OutputInvalid) else "uncertain", str(error))
            raise
        current = graph.get_state(config)
        if current.interrupts:
            pending = current.values.get("pending", {})
            open_ids = [key for key, value in current.values.get("findings", {}).items() if value["status"] == "open"]
            detail = ("Revision incomplete; provisional artifact has unresolved findings: " + ", ".join(open_ids)) if pending.get("kind") == "final-review" and open_ids else None
            records.set_status(run_id, "awaiting-input", detail)
        elif current.values.get("outcome"):
            records.set_status(run_id, current.values["outcome"],current.values.get("blocked_reason") or None)
        return current

def answer(run_id, request_id, outcome, text, sha256=None):
    current = state(run_id)
    old = records.response(run_id, request_id)
    v2=current.values.get("workflow_version")==2
    if v2:
        from .context import check
        check(records.get_run(run_id)["envelope"])
    if old:
        if outcome == "approved":
            artifacts.content(run_id, current.values["current"])
        payload = {"request_id": request_id, "answer": text, "outcome": outcome,
                   "artifact_sha256": sha256 or old["artifact_sha256"], "decision_revision": old["decision_revision"]}
        if v2:
            payload.update(context_digest=current.values["context_digest"],supersedes=old.get("supersedes",[]))
        return records.save_response(run_id, payload)
    if not current.interrupts or current.values.get("pending", {}).get("request_id") != request_id:
        raise OperationalError("Request is not the current pending human gate")
    pending = current.values["pending"]
    artifacts.content(run_id, current.values["current"])
    if pending["decision_revision"] != current.values["decision_revision"]:
        raise OperationalError("Stale decision revision")
    if pending["kind"] == "choice":
        if outcome != "answered" or text not in pending["question"]["options"]:
            raise OperationalError("Choose one of the documented substantive options")
    elif outcome not in ("approved", "rejected"):
        raise OperationalError("Final review requires approval or rejection")
    elif outcome == "approved" and sha256 != pending["artifact_sha256"]:
        raise OperationalError("Stale or missing artifact SHA-256")
    payload = {"request_id": request_id, "answer": text, "outcome": outcome,
               "artifact_sha256": pending["artifact_sha256"], "decision_revision": pending["decision_revision"]}
    if v2:
        if pending.get("context_digest")!=current.values["context_digest"]:
            raise OperationalError("Stale approval/answer context")
        payload.update(context_digest=current.values["context_digest"],
                       supersedes=pending.get("question",{}).get("related_decision_refs",[]))
    return records.save_response(run_id, payload)


def context_change(state, result, ref, packet_hash, findings):
    """Version-2 routing only; completed result is checkpointed before any gate."""
    from .context import effective_decisions
    stage=state["next_stage"]
    known={d["question"].get("topic_key",d["question"]["id"]):d for d in effective_decisions(state)}
    questions=[]; blocked=""
    for q in result["questions"]:
        prior=known.get(q["topic_key"])
        if prior and not q["reconsideration_reason"]:
            old=prior["question"]
            if q["question"]==old["question"] and q["options"]==old["options"]:
                continue
            blocked="Question conflicts with an accepted topic without evidenced reconsideration; inspect retained result"
        elif prior and prior.get("qualified_id",state["run_id"]+"/"+prior["request_id"]) not in q["related_decision_refs"]:
            blocked="Reconsideration does not reference the accepted topic's decision"
        questions.append(q)
    allowed=state.get("max_questions",1)
    if len(questions)+state["questions_asked"]>allowed:
        blocked=f"Frozen {allowed}-question limit reached; result retained, no automatic retry"
    next_stage={"draft":"critique-1","critique-1":"revision-1","revision-1":"critique-2",
                "critique-2":"revision-2","revision-2":"final"}[stage]
    needs_answer=state.get("revision_required",False)
    if stage.startswith("revision") and not questions: needs_answer=False
    if stage.startswith("critique") and not result["needs_revision"] and not questions and not needs_answer and not (
        stage == "critique-2" and state.get("coverage_gaps", {}).get("revision-1")):
        next_stage="final"
    change={"completed":state["completed"]+[stage],"findings":findings,"questions":questions,
            "questions_asked":state["questions_asked"]+len(questions),"next_stage":next_stage,
            "revision_required":needs_answer,"blocked_reason":blocked,
            "question_origin":{"operation_id":result["operation_id"],"artifact":ref,
                               "result_sha256":digest(canonical(result)),"packet_sha256":packet_hash}}
    if stage.startswith("critique"):
        change.update(last_reviewed=state.get("current"),last_critique={"artifact":ref,"result":result})
    else: change["current"]=ref
    return change
