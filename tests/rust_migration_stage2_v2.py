"""Version-2 native packet/plan parity using fake project state, no native calls."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from agent_workflow import artifacts, context, native
from agent_workflow.runtime import RuntimePaths,bind,unbind
from agent_workflow.storage import canonical,digest

def rust(binary,action,body):
    return json.loads(subprocess.check_output([str(binary),action],input=json.dumps(body).encode()))

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("source",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    source=args.source.resolve();binary=args.binary.resolve();evidence=args.evidence.resolve()
    before={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in source.rglob("*") if p.is_file()}
    evidence.mkdir(parents=True,exist_ok=False)
    project=evidence/"project";shutil.copytree(source,project)
    state_root=project/".wayward-wrangler/state"
    key=json.loads((state_root/"project-identity.json").read_text())["project_key"]
    token=bind(RuntimePaths(project,state_root,evidence/"registry",key))
    cases=[]
    try:
        snapshot=context.resolve(project/"context.toml",exclude_run="new-run")
        import sqlite3
        db=sqlite3.connect(state_root/"state.sqlite")
        source_run=db.execute("select run_id,envelope from app_runs where status='approved' limit 1").fetchone()
        db.close()
        envelope=json.loads(source_run[1])
        envelope.update(run_id="new-run",workflow_version=2,context_snapshot=snapshot,
                        context_digest=digest(canonical(snapshot)),
                        native_authorization={"plan_sha256":"frozen-plan-digest"})
        config=envelope["config"]
        config.update(max_invocations=5,review_limit=2,max_questions=3,prompt_contract="product-spec-v1")
        config["roles"]={"author":{"provider":"claude","model":"sonnet","reasoning":"medium","binary":"/fixture/claude"},
                         "critic":{"provider":"codex","model":"gpt-6.1-sol","reasoning":"high","binary":"/fixture/codex"}}
        py_plan=native.plan(envelope)
        rs=rust(binary,"native_plan",{"envelope":envelope})
        assert rs=={"ok":True,"value":py_plan},(py_plan,rs)
        cases.append({"case":"frozen-native-plan","sha256":digest(canonical(py_plan))})
        assert rust(binary,"native_digest",{"envelope":envelope})=={"ok":True,"value":digest(canonical(py_plan))}
        inherited=context.inherited(snapshot)
        state={"run_id":"new-run","current":None,"last_reviewed":None,
               "decisions":[],"inherited_decisions":inherited,"decision_revision":0,
               "questions_asked":0,"completed":[],"findings":{},"last_critique":None,
               "finding_contract_version":2,"finding_assessments":[]}
        for stage in ("draft","critique-2"):
            if stage=="critique-2":
                draft=artifacts.publish("new-run","draft",{"operation_id":"new-run-draft","content":"# Draft\n\nold\n",
                    "findings":[]},"packet",[])
                revised=artifacts.publish("new-run","revision-1",{"operation_id":"new-run-revision-1",
                    "content":"# Draft\n\nchanged\n","findings":[]},"packet",[draft])
                state.update(current=revised,last_reviewed=draft,completed=["draft","critique-1","revision-1"],
                    decisions=[{"request_id":"D-new","answer":"local","supersedes":[]}],decision_revision=1,
                    findings={"Z-issue":{"id":"Z-issue","status":"open","evidence":"first issue"},
                              "A-issue":{"id":"A-issue","status":"open","evidence":"second issue"}},
                    finding_order=["Z-issue","A-issue"],
                    coverage_gaps={"revision-1":[{"id":"C1-2","evidence":"original issue","status":"open"}]},
                    finding_assessments=[{"finding_ref":"C1-2","assessment":"open"}],
                    last_critique={"artifact":{"id":"critique-1"}})
                envelope["cross_imports"]=[{"grant_id":"G1","purpose":"selected comparison",
                    "item":{"source_project_key":"b"*32,"qualified_id":"b/item","content":"selected only"}}]
            py=artifacts.packet(envelope,state,stage)
            current=artifacts.content("new-run",state.get("current"))
            previous=artifacts.content("new-run",state.get("last_reviewed"))
            rs=rust(binary,"packet",{"envelope":envelope,"state":state,"stage":stage,
                "current_text":current,"previous_text":previous})
            assert rs=={"ok":True,"value":py},(stage,rs,py)
            assert digest(canonical(rs["value"]))==digest(canonical(py))
            cases.append({"case":stage,"packet_sha256":digest(canonical(py)),
                "decisions":len(py["decisions"]),"imports":len(py.get("cross_project_imports",[])),
                "delta_sha256":digest(py["delta"])})
        assert cases[1]["decisions"]==1 and cases[1]["imports"]==0
        assert cases[2]["imports"]==1
        assert [f["id"] for f in py["open_findings"]]==["Z-issue","A-issue"]
    finally:unbind(token)
    assert before=={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in source.rglob("*") if p.is_file()}
    (evidence/"v2-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"cases":cases},indent=2)+"\n")
    print("Stage 2 version-2 packets and frozen native plan match installed r9; no native call")

if __name__=="__main__":main()
