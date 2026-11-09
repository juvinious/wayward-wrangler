"""Read-only explicit foreign access parity on copied r9 evidence only."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
from types import SimpleNamespace

from agent_workflow import context_access
from agent_workflow.storage import atomic, canonical, digest

RUN="r-t-806033fa323fee0597848fe2"
RECIPIENT="a"*32

def files(path):
    return {str(p.relative_to(path)):hashlib.sha256(p.read_bytes()).hexdigest()
            for p in path.rglob("*") if p.is_file()}

def rust(binary,action,body):
    call=subprocess.run([str(binary),action],input=json.dumps(body),text=True,
                        capture_output=True,check=True)
    return json.loads(call.stdout)

def python(action,body):
    try:
        if action=="foreign_select":
            binding=context_access.source_binding(body["source_project"],body["source_state"])
            value=context_access.select(binding,body["selector"])
        else:
            paths=SimpleNamespace(state=Path(body["recipient_state"]),key=body["recipient_key"])
            if action=="active_grants":
                value=context_access.active_for_task(paths,body["task_id"])
            elif action=="check_frozen":
                context_access.check_frozen(paths,body["imports"])
                value={"valid":True}
            else:raise ValueError(action)
        return {"ok":True,"value":value}
    except Exception as exc:return {"ok":False,"error":str(exc)}

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("source",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    before=files(args.source)
    args.evidence.mkdir(parents=True,exist_ok=False)
    project=args.evidence/"source-project"
    shutil.copytree(args.source,project)
    state=project/".wayward-wrangler/state"
    recipient=args.evidence/"recipient-state"
    recipient.mkdir()
    body={"source_project":str(project),"source_state":str(state)}
    cases=[]
    def compare(name,action,details,same_error=False):
        value={**body,**details}
        py=python(action,value);rs=rust(args.binary,action,value)
        if py["ok"]!=rs["ok"] or (py["ok"] and py["value"]!=rs["value"]) or \
           (same_error and py.get("error")!=rs.get("error")):
            raise AssertionError(f"{name}: Python={py!r}; Rust={rs!r}")
        cases.append({"case":name,"action":action,"accepted":py["ok"],
                      "python_detail":py.get("error","matched output"),
                      "rust_detail":rs.get("error","matched output")})
        return py
    artifact={"run":RUN,"kind":"artifact","id":"revision-2"}
    decision={"run":RUN,"kind":"decision","id":"D-retention"}
    selected=compare("approved-artifact","foreign_select",{"selector":artifact},True)["value"]
    compare("accepted-decision","foreign_select",{"selector":decision},True)
    compare("unknown-source-run","foreign_select",{"selector":{**artifact,"run":"other"}},True)
    compare("no-implicit-import","active_grants",{"recipient_state":str(recipient),
        "recipient_key":RECIPIENT,"task_id":"task"})
    grant={"schema_version":1,"grant_id":"grant1","recipient_project_key":RECIPIENT,
           "task_id":"task","source_project_key":selected["source_project_key"],
           "source_project":str(project),"source_state":str(state),"selector":artifact,
           "dependency_mode":"pinned","purpose":"selected artifact","item":selected}
    grant_path=recipient/"session/context-access/grants/grant1.json"
    atomic(grant_path,grant)
    access={"recipient_state":str(recipient),"recipient_key":RECIPIENT,"task_id":"task"}
    compare("active-explicit-grant","active_grants",access)
    frozen={"grant_id":"grant1","item_sha256":digest(canonical(selected)),
            "purpose":"selected artifact","item":selected}
    compare("pinned-frozen","check_frozen",{**access,"imports":[frozen]})
    # Unrelated state must not invalidate an explicit pinned item.
    (state/"unrelated.json").write_text('{"new":"unrelated"}\n')
    compare("unrelated-source-change","check_frozen",{**access,"imports":[frozen]})
    (recipient/"session/context-access/revocations").mkdir()
    atomic(recipient/"session/context-access/revocations/grant1.json",{"grant_id":"grant1"})
    compare("revoked-frozen","check_frozen",{**access,"imports":[frozen]},True)
    compare("revoked-future-import","active_grants",access)
    (recipient/"session/context-access/revocations/grant1.json").unlink()
    grant["dependency_mode"]="must_match_source"
    atomic(grant_path,grant)
    compare("source-matched","check_frozen",{**access,"imports":[frozen]})
    # The preserved approval marked F1 addressed. A disposable copy can model
    # an approved source with one still-open issue without changing original
    # bytes or inventing a finding that lacks origin evidence.
    database=state/"state.sqlite"
    with sqlite3.connect(database) as db:
        raw=db.execute("SELECT payload FROM app_requests WHERE run_id=? AND request_id='A1'",(RUN,)).fetchone()[0]
        final=json.loads(raw)
        final["findings"]["F1"]["status"]="open"
        db.execute("UPDATE app_requests SET payload=? WHERE run_id=? AND request_id='A1'",
                   (canonical(final),RUN))
    finding={"run":RUN,"kind":"finding","id":"F1"}
    compare("selected-open-finding","foreign_select",{"selector":finding})
    ledger=state/"runs"/RUN/"quality/critique-1-finding-ledger.json"
    original_ledger=ledger.read_bytes()
    damaged=json.loads(original_ledger)
    damaged["new_findings"][0]["evidence"]="changed origin"
    ledger.write_text(canonical(damaged))
    compare("tampered-finding-ledger","foreign_select",{"selector":finding})
    ledger.write_bytes(original_ledger)
    compare("restored-finding-ledger","foreign_select",{"selector":finding})
    (state/"runs"/RUN/"artifacts/revision-2.md").write_text("tampered copy\n")
    compare("changed-selected-source","check_frozen",{**access,"imports":[frozen]})
    compare("changed-artifact-refusal","foreign_select",{"selector":artifact})
    assert files(args.source)==before,"preserved r9 fixture changed"
    (args.evidence/"access-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"source_file_count":len(before),
        "cases":cases},indent=2)+"\n")
    print(f"Stage 2 explicit access: {len(cases)} cases passed; source unchanged")

if __name__=="__main__":main()
