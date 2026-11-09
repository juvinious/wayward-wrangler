"""Explicit live Herdr acceptance test, run inside a prototype controller tab."""
import json
import subprocess
import time
from agent_workflow.herdr_backend import Herdr, receipt
from agent_workflow.storage import ROOT, LOCAL, atomic, read, alive

def cli(*args):
    cmd=[str(ROOT/".venv/bin/awp"),*args]
    print("\nCONTROLLER:", " ".join(cmd), flush=True)
    result=subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout, flush=True)
    assert result.returncode==0, result.stdout
    return result.stdout

def main():
    h=Herdr()
    evidence=[]
    p=LOCAL/"probes/restart-live"
    cli("probe-start","restart-live","--seconds","6","--detach")
    original=read(p/"binding.json")
    deadline=time.monotonic()+3
    while not (p/"invocations.json").exists() and time.monotonic()<deadline:time.sleep(.05)
    assert not (p/"receipt.json").exists()
    snapshot=h.call("pane.read",{"pane_id":original["pane_id"],"source":"recent","lines":30})["read"]["text"]
    assert "simulated work 1/5" in snapshot
    first_runner=read(p/"claim.json")["runner"]
    assert alive(first_runner)
    cli("probe-resume","restart-live","--detach")
    assert read(p/"binding.json")==original
    assert read(p/"invocations.json")["count"]==1
    evidence.append({"test":"live output + controller process exit/reconnect", "binding":original, "snapshot":snapshot,"counter":1})
    cli("probe-resume","restart-live")
    assert receipt(p)["status"]=="completed"
    assert "COMPLETED" in h.call("pane.read",{"pane_id":original["pane_id"],"source":"recent","lines":40})["read"]["text"]
    # A retained previously completed worker is also checked for cancellation isolation.
    sentinel=read(LOCAL/"probes/stream-success/claim.json")["runner"]
    assert alive(sentinel)
    cli("probe-start","retained-failure","--mode","failure","--seconds","1")
    f=LOCAL/"probes/retained-failure"
    assert receipt(f)["status"]=="failed"
    fb=read(f/"binding.json")
    assert "FAILED" in h.call("pane.read",{"pane_id":fb["pane_id"],"source":"recent","lines":30})["read"]["text"]
    evidence.append({"test":"retained failure","binding":fb,"receipt":receipt(f)})
    cli("probe-start","cancel-target","--seconds","30","--detach")
    c=LOCAL/"probes/cancel-target"
    while not (c/"child.json").exists():time.sleep(.05)
    owned=read(c/"child.json")["identity"]
    assert alive(owned)
    cli("probe-cancel","cancel-target")
    cli("probe-resume","cancel-target")
    assert receipt(c)["status"]=="cancelled" and not alive(owned)
    assert alive(sentinel) and alive(first_runner)
    assert h.call("pane.get",{"pane_id":original["pane_id"]})["pane"]["pane_id"]==original["pane_id"]
    evidence.append({"test":"targeted cancellation","binding":read(c/"binding.json"),"child_gone":True,
                     "retained_success_unaffected":True,"first_runner_unaffected":True,"controller_survived":True})
    atomic(LOCAL/"evidence/foundation.json",{"passed":True,"checks":evidence})
    print("\nFOUNDATION PASSED. All completed/failed/cancelled panes are retained.\nNo agent model or credential was used.",flush=True)

if __name__=="__main__":
    try:
        main()
    except BaseException as error:
        atomic(LOCAL/"evidence/foundation-error.json",{"error":repr(error)})
        import traceback
        traceback.print_exc()
    print("Controller test output retained. Close this tab when finished.",flush=True)
    while True:time.sleep(2)
