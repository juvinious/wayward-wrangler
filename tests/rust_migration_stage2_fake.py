"""Deterministic fake coordinator/worker event parity; no Herdr or model calls."""
import argparse
import json
from pathlib import Path
import subprocess

from agent_workflow.storage import canonical,digest

RUN="r-t-806033fa323fee0597848fe2"
STAGES=["draft","critique-1","revision-1","critique-2","revision-2"]

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("source",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    source=args.source.resolve();binary=args.binary.resolve();evidence=args.evidence.resolve()
    evidence.mkdir(parents=True,exist_ok=False)
    python=Path(__import__("sys").executable)
    packets=[]
    for stage in STAGES:
        packet=source/".wayward-wrangler/state/runs"/RUN/"operations"/stage/"packet.json"
        packets.append((stage,json.loads(packet.read_bytes())))
    packets.append(("coordinator",{"operation_kind":"coordinator","operation_id":"fixture-coordinator",
        "task_id":"task","task_version":1,"message_sha256":"message","message":"Create a bounded proposal",
        "stage":"coordinate","seconds":0}))
    cases=[]
    for stage,packet in packets:
        frozen=evidence/(stage+"-packet.json")
        frozen.write_text(canonical(packet))
        counter=evidence/(stage+"-counter.json")
        baseline=subprocess.run([str(python),"-I","-m","agent_workflow.adapters.fake",
            "--packet",str(frozen),"--counter",str(counter)],capture_output=True,text=True,check=True)
        py=[json.loads(line) for line in baseline.stdout.splitlines()]
        rust=json.loads(subprocess.check_output([str(binary),"fake_events"],
            input=json.dumps({"packet":packet}).encode()))
        assert rust=={"ok":True,"value":py},(stage,py,rust)
        assert json.loads(counter.read_text())["count"]==1
        cases.append({"stage":stage,"packet_sha256":digest(canonical(packet)),
            "event_count":len(py),"result_sha256":digest(canonical(py[-1]["result"])),
            "baseline_invocations":1})
    (evidence/"fake-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"cases":cases},indent=2)+"\n")
    print("Stage 2 fake coordinator and five workers: events matched with one baseline invocation each")

if __name__=="__main__":main()
