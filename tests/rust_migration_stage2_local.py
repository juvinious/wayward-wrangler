"""Project-local context snapshot and stale refusal parity on archived fake data."""
import argparse
import hashlib
import json
import re
from pathlib import Path
import shutil
import sqlite3
import subprocess

from agent_workflow import context
from agent_workflow.runtime import RuntimePaths,bind,unbind
from agent_workflow.storage import canonical,digest


def files(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob("*") if p.is_file()}

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("source",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    ap.add_argument("--run", required=True, help="run identity in the selected private fake fixture")
    ap.add_argument("--project-key", required=True, help="project identity in that fixture")
    args=ap.parse_args()
    run, key = args.run, args.project_key
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}", run) or not re.fullmatch(r"[a-f0-9]{32}", key):
        ap.error("Invalid fixture run or project identity")
    args.source=args.source.resolve()
    args.binary=args.binary.resolve()
    args.evidence=args.evidence.resolve()
    source_files=files(args.source)
    args.evidence.mkdir(parents=True,exist_ok=False)
    project=args.evidence/"project"
    shutil.copytree(args.source,project)
    state=project/".wayward-wrangler/state"
    (project/"AGENTS.md").write_text("# Selected project rule\nNo external sharing.\n")
    (project/"CLAUDE.md").write_text("Read [AGENTS.md](AGENTS.md).\n")
    selection=project/"context.toml"
    selection.write_text(f'''schema_version=1
project_id="decision-log"
scope="follow-up"
[[entries]]
kind="decision"
run="{run}"
id="D-retention"
topic_key="transcript.retention"
scope="follow-up"
reason="accepted retention policy"
[[entries]]
kind="artifact"
run="{run}"
id="revision-2"
scope="follow-up"
reason="approved source candidate"
[[entries]]
kind="findings"
run="{run}"
ids=["open"]
scope="follow-up"
reason="unresolved findings if any"
[[entries]]
kind="document"
path="CLAUDE.md"
scope="follow-up"
reason="selected project rules"
''')
    paths=RuntimePaths(project,state,args.evidence/"registry",key)
    token=bind(paths)
    try:
        base={"project":str(project),"state":str(state),"project_key":key,
              "selection":str(selection),"exclude_run":"new-run"}
        cases=[]
        def compare(name,action,more=None):
            body={**base,**(more or {})}
            try:
                if action=="local_resolve":value=context.resolve(selection,body.get("exclude_run"))
                else:
                    context.check({"run_id":body.get("exclude_run"),"workflow_version":2,
                        "context_snapshot":body["snapshot"],"context_digest":body["context_digest"]})
                    value={"valid":True}
                py={"ok":True,"value":value}
            except Exception as exc:py={"ok":False,"error":str(exc)}
            rs=json.loads(subprocess.check_output([str(args.binary),action],
                input=json.dumps(body).encode()))
            if py["ok"]!=rs["ok"] or (py["ok"] and py["value"]!=rs["value"]):
                raise AssertionError(f"{name}: Python={py!r}; Rust={rs!r}")
            cases.append({"case":name,"action":action,"accepted":py["ok"],
                          "python_detail":py.get("error","matched snapshot"),
                          "rust_detail":rs.get("error","matched snapshot")})
            return py
        snapshot=compare("explicit-selected-context","local_resolve")["value"]
        assert snapshot["schema_version"]==2 and len(snapshot["entries"])==3
        assert [e["kind"] for e in snapshot["entries"]]==["decision","artifact","document"]
        assert "CLAUDE.md" in snapshot["dependencies"] and "AGENTS.md" in snapshot["dependencies"]
        frozen={"snapshot":snapshot,"context_digest":digest(canonical(snapshot))}
        compare("unchanged-frozen","local_check",frozen)
        selected_digest=digest(canonical(snapshot))
        for name,path,replacement in [
            ("changed-document",project/"AGENTS.md",None),
            ("changed-selection",selection,None),
            ("changed-artifact",state/"runs"/run/"artifacts/revision-2.md",None),
            ("changed-decision-export",state/"runs"/run/"artifacts/decision-D-retention.json",None),
        ]:
            original=path.read_bytes()
            path.write_bytes(original+b"\nmodified fixture\n")
            compare(name,"local_check",frozen)
            path.write_bytes(original)
            compare(name+"-restored","local_check",frozen)
        with sqlite3.connect(state/"state.sqlite") as db:
            raw=db.execute("SELECT payload FROM app_requests WHERE run_id=? AND request_id='A1'",(run,)).fetchone()[0]
            final=json.loads(raw);final["findings"]["F1"]["status"]="open"
            db.execute("UPDATE app_requests SET payload=? WHERE run_id=? AND request_id='A1'",
                       (canonical(final),run))
        compare("open-finding-stales-frozen","local_check",frozen)
        revised=compare("selected-open-finding","local_resolve")["value"]
        assert any(e["kind"]=="findings" and e["qualified_id"].endswith("/F1") for e in revised["entries"])
        ledger=state/"runs"/run/"quality/critique-1-finding-ledger.json"
        original_ledger=ledger.read_bytes()
        damaged=json.loads(original_ledger)
        damaged["new_findings"][0]["evidence"]="tampered origin"
        ledger.write_text(canonical(damaged))
        compare("tampered-open-finding-ledger","local_resolve")
        ledger.write_bytes(original_ledger)
        with sqlite3.connect(state/"state.sqlite") as db:
            raw=db.execute("SELECT envelope FROM app_runs WHERE run_id=?",(run,)).fetchone()[0]
            changed=json.loads(raw);changed["project_key"]="b"*32
            db.execute("UPDATE app_runs SET envelope=? WHERE run_id=?",(canonical(changed),run))
        # records.get_run also checks source identity in the installed baseline.
        foreign=compare("foreign-labelled-run","local_resolve")
        assert not foreign["ok"]
        with sqlite3.connect(state/"state.sqlite") as db:
            db.execute("UPDATE app_runs SET envelope=? WHERE run_id=?",(raw,run))
        assert digest(canonical(snapshot))==selected_digest
        assert files(args.source)==source_files,"preserved source fixture changed"
        (args.evidence/"local-comparison.json").write_text(json.dumps({"schema_version":1,
            "baseline":__import__("agent_workflow").__file__,"source_file_count":len(source_files),
            "snapshot_digest":selected_digest,"cases":cases,
            "identity_refusal":{"both_refused":True,"baseline_reason":foreign["error"]}},indent=2)+"\n")
        print(f"Stage 2 local context: {len(cases)} cases passed; source unchanged")
    finally:unbind(token)

if __name__=="__main__":main()
