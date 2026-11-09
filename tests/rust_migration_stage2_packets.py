"""Frozen five-stage fake packet parity from approved r9 evidence; no dispatch."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from agent_workflow import artifacts
from agent_workflow.runtime import RuntimePaths,bind,unbind
from agent_workflow.storage import canonical,digest

RUN="r-t-806033fa323fee0597848fe2"
STAGES=["draft","critique-1","revision-1","critique-2","revision-2"]

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("project",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    source=args.project.resolve()
    source_files={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in source.rglob("*") if p.is_file()}
    args.evidence.mkdir(parents=True,exist_ok=False)
    project=args.evidence.resolve()/"project"
    shutil.copytree(source,project)
    state=project/".wayward-wrangler/state"
    key=json.loads((state/"project-identity.json").read_text())["project_key"]
    paths=RuntimePaths(project,state,args.evidence/"registry",key)
    token=bind(paths)
    try:
        import sqlite3
        db=sqlite3.connect(state/"state.sqlite")
        envelope=json.loads(db.execute("SELECT envelope FROM app_runs WHERE run_id=?",(RUN,)).fetchone()[0])
        db.close()
        root=state/"runs"/RUN
        manifests={p.stem:json.loads(p.read_text())["manifest"] for p in (root/"artifacts").glob("*.json")
                   if not p.name.startswith("decision-")}
        results=[]
        for stage in STAGES:
            frozen_path=root/"operations"/stage/"packet.json"
            frozen=frozen_path.read_bytes()
            packet=json.loads(frozen)
            previous=packet.get("previous_reviewed","")
            last_reviewed=next((ref for ref in manifests.values()
                if previous and artifacts.content(RUN,ref)==previous),None)
            state_value={"run_id":RUN,"current":packet["current_ref"],"last_reviewed":last_reviewed,
                "decisions":packet["decisions"],"findings":packet["finding_dispositions"],
                "last_critique":packet["critique"],"completed":STAGES[:STAGES.index(stage)]}
            python_packet=artifacts.packet(envelope,state_value,stage)
            if python_packet!=packet or canonical(python_packet).encode()!=frozen:
                raise AssertionError(f"{stage}: reconstructed Python packet differs from original evidence")
            body={"envelope":envelope,"state":state_value,"stage":stage,
                  "current_text":packet["current_draft"],"previous_text":previous}
            rust=json.loads(subprocess.check_output([str(args.binary.resolve()),"packet"],
                input=json.dumps(body).encode()))
            if not rust["ok"] or rust["value"]!=packet or canonical(rust["value"]).encode()!=frozen:
                raise AssertionError(f"{stage}: Rust packet differs from frozen original: {rust}")
            results.append({"stage":stage,"packet_sha256":digest(frozen),
                            "delta_sha256":digest(packet["delta"]),"bytes":len(frozen)})
    finally:unbind(token)
    # The installed Python reader may create transient SQLite WAL/SHM files
    # while bound; check source evidence after its connection lifecycle ends.
    assert source_files=={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in source.rglob("*") if p.is_file()}
    assert source_files=={str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in project.rglob("*") if p.is_file()}
    (args.evidence/"packet-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"run_id":RUN,
        "source_file_count":len(source_files),"stages":results},indent=2)+"\n")
    print("Stage 2 packets: five frozen stage bytes/digests match Python and Rust; source unchanged")

if __name__=="__main__":main()
