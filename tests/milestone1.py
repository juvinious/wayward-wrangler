"""Live, simulated-only milestone acceptance. Creates only AWP-owned Herdr tabs."""
import json
import subprocess
import time
from pathlib import Path
from agent_workflow import records
from agent_workflow.herdr_backend import Herdr, ensure, receipt, validate_result
from agent_workflow.storage import ROOT, LOCAL, OperationalError, atomic, lock, read
from agent_workflow.workflow import state

RUN = "acceptance-m1"
checks=[]

def cli(*args, expected=0):
    cmd=[str(ROOT/".venv/bin/awp"), *args]
    print("\nCONTROLLER:", " ".join(cmd), flush=True)
    completed=subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(completed.stdout, flush=True)
    assert completed.returncode==expected, (completed.returncode, expected, completed.stdout)
    return completed

def count_snapshot():
    return {p.parent.name:read(p)["count"] for p in (records.run_dir(RUN)/"operations").glob("*/invocations.json")}

def check(label):
    checks.append(label)
    print("PASS:",label,flush=True)

def main():
    atomic(LOCAL/"test-answer.md", "7 days\n")
    atomic(LOCAL/"test-conflict.md", "30 days\n")
    cli("start","--run",RUN,"--workspace","w5",expected=2)
    first=state(RUN).values
    assert first["pending"]["request_id"]=="D-retention"
    assert first["completed"]==["draft","critique-1"]
    assert count_snapshot()=={"draft":1,"critique-1":1}
    cli("resume",RUN,expected=2)
    assert state(RUN).values["pending"]==first["pending"]
    cli("start","--run","must-not-start","--workspace","w5",expected=3)
    assert not records.run_dir("must-not-start").exists()
    check("single active run; stable human choice across controller exits; completed calls not repeated")
    with lock(LOCAL/"controller.lock"):
        cli("resume",RUN,expected=3)
    directory=records.run_dir(RUN)/"operations/draft"
    old=read(directory/"binding.json")
    assert ensure(directory)["status"]=="completed"
    op=read(directory/"operation.json")
    duplicate=subprocess.run([str(ROOT/".venv/bin/python"),"-m","agent_workflow.runner",
        "--operation-dir",str(directory),"--launch-token",op["launch_token"]],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    assert duplicate.returncode!=0 and count_snapshot()["draft"]==1
    assert read(directory/"binding.json")==old
    check("duplicate controller and delayed duplicate runner refused; receipt reused")
    cli("decide",RUN,"--request","D-retention","--answer-file",str(LOCAL/"test-answer.md"),"--record-only")
    assert records.response(RUN,"D-retention")["answer"]=="7 days"
    assert state(RUN).values["decision_revision"]==0
    cli("resume",RUN,expected=2)
    final=state(RUN).values
    assert final["pending"]["request_id"]=="A1" and final["decision_revision"]==1
    assert final["completed"]==list(records.STAGES)
    assert count_snapshot()=={stage:1 for stage in records.STAGES}
    revision=read(records.run_dir(RUN)/"operations/revision-1/packet.json")
    critique=read(records.run_dir(RUN)/"operations/critique-2/packet.json")
    assert revision["decisions"][0]["answer"]=="7 days" and revision["critique"]["artifact"]["id"]=="critique-1"
    assert critique["delta"] and critique["previous_reviewed"] and critique["current_draft"]
    assert critique["decisions"][0]["answer"]=="7 days"
    assert all(f["status"]=="addressed" for f in final["findings"].values())
    check("persisted answer consumed once; automatic draft/critique/revision packets include decisions, findings and deltas")
    cli("decide",RUN,"--request","D-retention","--answer-file",str(LOCAL/"test-conflict.md"),expected=3)
    try:
        records.reserve(RUN,"critique-3","forbidden","bad")
        raise AssertionError("third critique accepted")
    except OperationalError:
        pass
    packet=read(directory/"packet.json")
    result=read(directory/"result.json")
    try:
        validate_result({**result,"approval":"approved"},packet)
        raise AssertionError("worker approval accepted")
    except OperationalError:
        pass
    cli("approve",RUN,"--request","A1","--sha256","0"*64,expected=3)
    assert records.response(RUN,"A1") is None
    check("conflicting choice, third review, worker-authored approval and stale hash refused")
    actual=final["current"]["sha256"]
    cli("approve",RUN,"--request","A1","--sha256",actual,"--record-only")
    assert state(RUN).values["outcome"]==""
    cli("resume",RUN)
    assert state(RUN).values["outcome"]=="approved"
    assert records.get_run(RUN)["status"]=="approved"
    cli("resume",RUN)
    cli("approve",RUN,"--request","A1","--sha256",actual)
    assert count_snapshot()=={stage:1 for stage in records.STAGES}
    check("approval saved before checkpoint consumption survives fresh processes; approved run does not dispatch")
    evidence={"passed":True,"run_id":RUN,"checks":checks,"calls":count_snapshot(),
              "approval":records.response(RUN,"A1"),"note":"Automated simulated human-response fixture; not operator approval of a real draft."}
    atomic(LOCAL/"evidence/milestone1.json",evidence)
    print("\nMILESTONE 1 WORKFLOW TESTS PASSED — simulated approval fixture only.",flush=True)

if __name__=="__main__":
    try:
        main()
    except BaseException as error:
        atomic(LOCAL/"evidence/milestone1-error.json",{"error":repr(error),"passed_checks":checks})
        import traceback
        traceback.print_exc()
    print("Controller acceptance output retained in this pane.",flush=True)
    while True:time.sleep(2)
