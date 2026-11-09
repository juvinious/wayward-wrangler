"""Compare isolated project-command traces against the preserved installed r9 Store.

This is development-only migration verification. Neither side starts a service,
Herdr, model CLI, or operator project.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / ".local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/venv/bin/python"
PYTHON_TRACE = r'''
import json,sys,types
from pathlib import Path
from agent_workflow.session_service import Store
p=Path(sys.argv[1]);s=p/'.wayward-wrangler/state';p.mkdir(parents=True,exist_ok=True)
a=Store(types.SimpleNamespace(state=s,key='a'*32))
first={'command_id':'first','action':'submit_message','expected_state_revision':0,
 'payload':{'text':'Specify a local project decision log.'}}
accepted=a.command(first);replay=a.command(first)
conflict=a.command({**first,'payload':{'text':'changed'}})
stale=a.command({'command_id':'stale','action':'submit_message','expected_state_revision':0,
 'payload':{'text':'Second task'}})
task=a.task(accepted['task_id']);id=accepted['task_id']
steer={'command_id':'steer','action':'steer_queued','expected_task_version':1,
 'expected_state_revision':task['state_revision'],'payload':{'task_id':id,'text':'Revised brief'}}
steered=a.modify_queued(steer);steer_replay=a.modify_queued(steer)
cancel={'command_id':'cancel','action':'cancel_task','expected_task_version':2,
 'expected_state_revision':a.task(id)['state_revision'],'payload':{'task_id':id}}
cancelled=a.cancel_undispatched(cancel)
stop={'command_id':'exit','action':'stop_session','expected_state_revision':a.snapshot()['state_revision'],'payload':{}}
exiting=a.lifecycle_command(stop);stop_replay=a.lifecycle_command(stop)
resume={'command_id':'resume','action':'resume_session','expected_state_revision':a.snapshot()['state_revision'],'payload':{}}
resumed=a.lifecycle_command(resume);stale_exit=a.update_exit('exit',1,'paused')
print(json.dumps({'accepted':accepted,'replay':replay,'conflict':conflict,'stale':stale,
 'steered':steered,'steer_replay':steer_replay,'cancelled':cancelled,'exiting':exiting,
 'stop_replay':stop_replay,'resumed':resumed,'stale_exit':stale_exit,'snapshot':a.snapshot()}))
'''

def normalized(value):
    if isinstance(value, dict):
        return {key: normalized(item) for key, item in value.items()
                if key not in {"requested_at", "deadline", "service_attention"}}
    if isinstance(value, list):
        return [normalized(item) for item in value]
    return value

def main(evidence):
    assert not evidence.exists(), "Use a fresh evidence directory"
    evidence.mkdir(parents=True)
    assert BASELINE.is_file(), "Preserved r9 interpreter unavailable"
    old_project=evidence/"r9-project";old_project.mkdir()
    rust_project=evidence/"rust-project";rust_project.mkdir()
    baseline=subprocess.run([str(BASELINE),"-I","-c",PYTHON_TRACE,str(old_project)],
        capture_output=True,text=True)
    (evidence/"r9-stderr.log").write_text(baseline.stderr)
    baseline.check_returncode()
    rust=subprocess.run([str(ROOT/"target/debug/session-probe"),str(rust_project),
        str(rust_project/".wayward-wrangler/state")],capture_output=True,text=True)
    (evidence/"rust-stderr.log").write_text(rust.stderr)
    rust.check_returncode()
    old=json.loads(baseline.stdout);new=json.loads(rust.stdout)
    (evidence/"r9.json").write_text(json.dumps(old,indent=2,sort_keys=True)+"\n")
    (evidence/"rust.json").write_text(json.dumps(new,indent=2,sort_keys=True)+"\n")
    assert normalized(old)==normalized(new), "Stage-5 command trace differs; inspect saved reports"
    (evidence/"summary.json").write_text(json.dumps({"status":"passed",
        "checked":"submit/replay/conflict/stale/steer/cancel/stop/resume/stale-exit/events",
        "normalized_only":["requested_at","deadline","service_attention"]},indent=2)+"\n")
    print(evidence/"summary.json")

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--evidence-root",type=Path,required=True)
    main(parser.parse_args().evidence_root.resolve())
