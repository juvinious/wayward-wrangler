"""Saved output envelope/authority parity and one explicit fake-v2 defect fix."""
import argparse
import copy
import json
from pathlib import Path
import subprocess

from agent_workflow.herdr_backend import validate_result
from agent_workflow.storage import canonical,digest

RUN="r-t-806033fa323fee0597848fe2"

def check_python(result,packet):
    try:validate_result(result,packet);return {"ok":True,"value":{"valid":True}}
    except Exception as exc:return {"ok":False,"error":str(exc)}

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("project",type=Path)
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    binary=args.binary.resolve();evidence=args.evidence.resolve()
    evidence.mkdir(parents=True,exist_ok=False)
    root=args.project.resolve()/".wayward-wrangler/state/runs"/RUN/"operations"
    cases=[]
    def compare(name,packet,result,same_error=True):
        py=check_python(result,packet)
        rs=json.loads(subprocess.check_output([str(binary),"validate_output"],
            input=json.dumps({"packet":packet,"result":result}).encode()))
        if py["ok"]!=rs["ok"] or (py["ok"] and py["value"]!=rs["value"]) or \
           (same_error and py.get("error")!=rs.get("error")):
            raise AssertionError(f"{name}: Python={py}; Rust={rs}")
        cases.append({"case":name,"accepted":py["ok"],"detail":py.get("error","valid")})
    packets={p.parent.name:json.loads(p.read_text()) for p in root.glob("*/packet.json")}
    results={p.parent.name:json.loads(p.read_text()) for p in root.glob("*/result.json")}
    for stage in sorted(packets):compare("saved-"+stage,packets[stage],results[stage])
    packet=packets["draft"];result=results["draft"]
    for name,mutation in [
        ("forged-approval",lambda x:x.update(approval="approved")),
        ("wrong-input",lambda x:x.update(packet_sha256="0"*64)),
        ("empty-content",lambda x:x.update(content="  ")),
        ("invalid-needs-revision",lambda x:x.update(needs_revision="no")),
    ]:
        changed=copy.deepcopy(result);mutation(changed)
        compare(name,packet,changed)
    native_packet={**packet,"provider":"claude"}
    native_result={**result,"packet_sha256":digest(canonical(native_packet)),
        "questions":[{"id":"q","question":"Choose?","options":["A","B"],
                      "recommendation":"A","consequences":"tradeoff"}]}
    compare("legacy-native-question-refused",native_packet,native_result)
    v2_packet={"schema_version":2,"finding_contract_version":2,"operation_id":"op",
        "stage":"draft","provider":"fake","run_id":"run","decisions":[],
        "context_snapshot":{"entries":[]},"finding_dispositions":{}}
    rust_fake=json.loads(subprocess.check_output([str(binary),"fake_result"],
        input=json.dumps({"packet":v2_packet}).encode()))
    assert rust_fake["ok"]
    compare("v2-fake-valid",v2_packet,rust_fake["value"])
    invalid=copy.deepcopy(rust_fake["value"])
    invalid["questions"][0]["evidence_refs"]=["another-project/secret"]
    compare("v2-unknown-evidence",v2_packet,invalid)
    invalid=copy.deepcopy(rust_fake["value"])
    invalid["new_findings"]=[{"severity":"minor","evidence":"issue"}]
    compare("v2-invalid-finding",v2_packet,invalid)
    coordinator={"operation_kind":"coordinator","operation_id":"coordinator-op",
        "task_id":"task","task_version":1,"message_sha256":"message"}
    proposal={"schema_version":1,"operation_id":"coordinator-op","packet_sha256":digest(canonical(coordinator)),
        "proposal":{"schema_version":1,"action":"propose_task","task_id":"task",
                    "task_version":1,"message_sha256":"message","brief":"Do this"}}
    compare("coordinator-valid",coordinator,proposal)
    stale=copy.deepcopy(proposal);stale["proposal"]["task_version"]=2
    compare("coordinator-stale-task",coordinator,stale)
    # Installed r9 fake worker still emits v1 finding fields for a v2 finding
    # packet. Keep the mismatch visible; the Rust fake follows the v2 contract.
    old_shape=copy.deepcopy(rust_fake["value"])
    old_shape["findings"]=[];old_shape["dispositions"]=[]
    old_shape.pop("new_findings");old_shape.pop("assessments")
    assert not check_python(old_shape,v2_packet)["ok"]
    (evidence/"output-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"cases":cases,
        "intentional_fake_v2_fix":{"baseline_old_shape_refused":True,
            "rust_version2_shape_accepted":True,
            "impact":"simulated v2 finding protocol can exercise the approved contract"}},indent=2)+"\n")
    print(f"Stage 2 output: {len(cases)} saved/fault cases matched; fake-v2 defect isolated")

if __name__=="__main__":main()
