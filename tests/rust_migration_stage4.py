"""Differential fake workflow gate on private Herdr and copied application state."""
import argparse
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import sys
import time
from rust_migration_stage3 import Harness, PYTHON, ROOT, eventual, identity, live, load, save, signal_owned

FLOW=ROOT/"target/debug/workflow-probe"
OP=ROOT/"target/debug/operation-probe"

def digest(value):
    return hashlib.sha256((json.dumps(value,sort_keys=True,indent=2,ensure_ascii=False)+"\n").encode()).hexdigest()

def envelope(run,target,version=1,questions=1):
    config={"roles":{role:{"provider":"fake","model":"deterministic-stage4"} for role in ("author","critic")},
            "fake":{"seconds":.04,"questions":questions},"review_limit":2,"max_questions":questions,
            "finding_contract_version":1}
    value={"schema_version":1,"workflow_version":version,"run_id":run,"thread_id":run,
        "brief":"Simulated recovery fixture; responses are automated test data, not human approval.",
        "instructions":[],"skills":{"draft":"deterministic test","critique":"deterministic test","revise":"deterministic test"},
        "rules_sha256":hashlib.sha256(b"[]").hexdigest(),"config_sha256":digest(config),"target":target,"config":config}
    if version==2:
        value["context_snapshot"]={"schema_version":1,"scope":"test-project","entries":[]}
        value["context_digest"]=digest(value["context_snapshot"])
    return value

class Suite(Harness):
    def command(self,action,state,run,*more,ok=True):
        result=subprocess.run([str(FLOW),action,str(state),run,*map(str,more)],capture_output=True,text=True,timeout=20)
        if ok:assert result.returncode==0,result.stderr
        return json.loads(result.stdout) if result.returncode==0 else result

    def new(self,name,version=1,questions=1,findings_version=1):
        state=self.root/name
        state.mkdir()
        save(state/"rust-migration-test-state.json",{"case":name})
        run="flow-"+name
        input_value=envelope(run,{"socket":self.socket,"workspace":self.workspace},version,questions)
        input_value["config"]["finding_contract_version"]=findings_version
        input_value["config_sha256"]=digest(input_value["config"])
        save(state/"envelope-input.json",input_value)
        self.command("create",state,run,state/"envelope-input.json")
        return state,run

    def stage(self,state,run,expected):
        reservation=self.command("reserve",state,run)
        assert reservation["stage"]==expected
        directory=Path(reservation["directory"])
        dispatched=subprocess.run([str(OP),"ensure",str(state),str(directory)],capture_output=True,text=True,timeout=20)
        assert dispatched.returncode==0,dispatched.stderr
        eventual(lambda:(directory/"receipt.json").exists())
        assert load(directory/"receipt.json")["status"]=="completed"
        snapshot=self.command("apply",state,run,expected)
        replay=self.command("apply",state,run,expected)
        assert replay==snapshot,"Repeated result changed state or applied effect twice"
        assert load(directory/"invocations.json")["count"]==1
        return snapshot

    def full_v1(self):
        state,run=self.new("rust-v1")
        for stage in ("draft","critique-1"):
            self.stage(state,run,stage)
        gate=self.command("gate",state,run)
        assert gate["request_id"]=="D-retention" and gate["kind"]=="choice"
        answered=self.command("answer",state,run,gate["request_id"],"answered","7 days")
        assert answered["answer"]=="7 days"
        # Separate CLI process consumes the durable answer after the first exits.
        consumed=self.command("consume",state,run)
        assert consumed["state"]["decisions"][0]["answer"]=="7 days"
        assert self.command("answer",state,run,gate["request_id"],"answered","7 days")==answered
        for stage in ("revision-1","critique-2","revision-2"):
            self.stage(state,run,stage)
        final=self.command("gate",state,run)
        assert final["kind"]=="final-review" and final["artifact_sha256"]==self.command("status",state,run)["state"]["current"]["sha256"]
        stale=self.command("answer",state,run,"A1","approved","Automated fixture approval","0"*64,ok=False)
        assert "Stale or missing artifact" in stale.stderr
        self.command("answer",state,run,"A1","approved","Automated fixture approval",final["artifact_sha256"])
        ended=self.command("consume",state,run)
        assert ended["status"]=="approved"
        assert self.command("status",state,run)==ended
        assert len(list((state/"runs"/run/"operations").glob("*/invocations.json")))==5
        save(state/"final-summary.json",{"status":ended["status"],"completed":ended["state"]["completed"],
             "artifact":ended["state"]["current"],"decision_revision":ended["state"]["decision_revision"],
             "finding_statuses":{k:v["status"] for k,v in ended["state"]["findings"].items()}})
        self.pass_case("full-v1-record-answer-restart-exact-approval",stages=5,invocations=5,
                       decision_revision=1,artifact_sha256=final["artifact_sha256"],fixture_approval=True)
        return state,run

    def baseline_v1(self,rust):
        state=self.root/"installed-r9-flow";state.mkdir()
        run="flow-installed-r9"
        save(state/"envelope-input.json",envelope(run,{"socket":self.socket,"workspace":self.workspace}))
        script="""import json,sys
from pathlib import Path
from agent_workflow import storage
state=Path(sys.argv[1]);run=sys.argv[2];action=sys.argv[3]
storage.LOCAL=state
from agent_workflow import records,workflow
from agent_workflow.storage import atomic
if action=='create':
    envelope=json.loads((state/'envelope-input.json').read_text())
    records.create_run(envelope)
    atomic(records.run_dir(run)/'inputs/envelope.json',envelope,immutable=True)
elif action=='drive':workflow.drive(run)
elif action=='answer':
    pending=workflow.state(run).values['pending']
    workflow.answer(run,pending['request_id'],'answered','7 days')
elif action=='approve':
    pending=workflow.state(run).values['pending']
    workflow.answer(run,pending['request_id'],'approved','Automated fixture approval',pending['artifact_sha256'])
if action!='create':
    state_value=workflow.state(run).values
    print('CHECKPOINT:'+json.dumps({'status':records.get_run(run)['status'],'completed':state_value.get('completed',[]),
        'current':state_value.get('current'),'decision_revision':state_value.get('decision_revision',0),
        'findings':state_value.get('findings',{})}))
"""
        values=[]
        for action in ("create","drive","answer","drive","approve","drive"):
            result=subprocess.run([str(PYTHON),"-I","-c",script,str(state),run,action],
                                  env={**self.env,"PYTHONUNBUFFERED":"1"},capture_output=True,text=True,timeout=35)
            with (state/"controller.log").open("a") as log:log.write(f"\n== {action} ==\n"+result.stdout+result.stderr)
            assert result.returncode==0,result.stderr
            for line in result.stdout.splitlines():
                if line.startswith("CHECKPOINT:"):values.append(json.loads(line[len("CHECKPOINT:"):]))
        final=values[-1]
        old=load(rust[0]/"final-summary.json")
        assert final["status"]==old["status"]=="approved"
        assert len(final["completed"])==len(old["completed"])==5
        assert final["decision_revision"]==old["decision_revision"]==1
        assert final["current"]["sha256"]==old["artifact"]["sha256"]
        assert {k:v["status"] for k,v in final["findings"].items()}==old["finding_statuses"]
        save(state/"final-summary.json",final)
        self.pass_case("installed-r9-full-flow-parity",status="approved",stages=5,
                       artifact_sha256=final["current"]["sha256"])

    def multiple_questions_v2(self):
        state,run=self.new("rust-v2-two-questions",version=2,questions=2,findings_version=2)
        self.stage(state,run,"draft")
        first=self.command("gate",state,run)
        assert first["question"]["id"]=="sharing"
        self.command("answer",state,run,first["request_id"],"answered","Project only")
        self.command("consume",state,run)
        second=self.command("gate",state,run)
        assert second["question"]["id"]=="retention" and second["request_id"]!=first["request_id"]
        self.command("answer",state,run,second["request_id"],"answered","7 days")
        consumed=self.command("consume",state,run)
        assert consumed["state"]["decision_revision"]==2
        for stage in ("critique-1","revision-1","critique-2","revision-2"):
            self.stage(state,run,stage)
        final=self.command("gate",state,run)
        assert final["kind"]=="final-review" and len(final["decisions"])==2
        assert len(final["findings"])==2 and all(value["status"]=="open" for value in final["findings"].values())
        self.pass_case("two-durable-questions-and-fresh-controller-consumption",
                       questions=2,stages=5,final_candidate=final["artifact_sha256"],
                       open_findings=2,fixture_approval=False)

    def cleanup(self):
        for path in self.root.glob("*/runs/*/operations/*/child.json"):
            who=load(path)["identity"]
            if live(who):signal_owned(who,signal.SIGTERM)
        for path in self.root.glob("*/runs/*/operations/*/claim.json"):
            who=load(path)["runner"]
            if live(who):signal_owned(who,signal.SIGTERM)
        self.stop()
        save(self.root/"cleanup.json",{"owned_server_stopped":True,
             "live_owned_runners":[str(p) for p in self.root.glob("*/runs/*/operations/*/claim.json") if live(load(p)["runner"])],
             "live_owned_children":[str(p) for p in self.root.glob("*/runs/*/operations/*/child.json") if live(load(p)["identity"])]})

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--evidence-root",type=Path,required=True)
    args=parser.parse_args()
    suite=Suite(args.evidence_root.resolve())
    try:
        suite.start()
        rust=suite.full_v1()
        suite.baseline_v1(rust)
        suite.multiple_questions_v2()
        save(suite.root/"summary.json",{"passed":suite.cases})
    finally:suite.cleanup()

if __name__=="__main__":main()
