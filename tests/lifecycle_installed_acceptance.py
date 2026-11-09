"""One isolated installed, simulated session exit/resume proof. Never uses native agents."""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import socket as network_socket
import subprocess
import sys
import time

import agent_workflow
from agent_workflow.herdr_backend import Herdr
from agent_workflow.session_bootstrap import _request
from agent_workflow.session_service import encode, parse_frame
from agent_workflow.storage import atomic, identity, read


def eventually(check, seconds=45):
    until=time.monotonic()+seconds
    while time.monotonic()<until:
        try:
            value=check()
            if value:return value
        except (FileNotFoundError,ConnectionRefusedError):pass
        time.sleep(.1)
    raise AssertionError("isolated installed lifecycle check timed out")


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--root",type=Path,required=True)
    parser.add_argument("--frontend",type=Path,required=True)
    parser.add_argument("--release-id",required=True)
    args=parser.parse_args()
    args.frontend=args.frontend.resolve(strict=True)
    root=args.root.resolve();root.mkdir(parents=True,exist_ok=False)
    assert not Path(agent_workflow.__file__).resolve().is_relative_to(Path(__file__).resolve().parents[1] / "src")
    for part in ("home","config/herdr","runtime","registry","a","b","c"):
        (root/part).mkdir(parents=True,exist_ok=True)
    (root/"config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket=str(root/"runtime/herdr.sock")
    env={**os.environ,"HOME":str(root/"home"),"XDG_CONFIG_HOME":str(root/"config"),
         "HERDR_SOCKET_PATH":socket,"SHELL":"/bin/sh","TERM":"xterm-256color"}
    env.pop("HERDR_ENV",None)
    log=(root/"herdr-server.log").open("wb")
    server=subprocess.Popen(["herdr","server"],env=env,stdout=log,stderr=subprocess.STDOUT)
    log.close();owned=identity(server.pid)
    h=Herdr(socket)
    units=[];checks=[]

    def bootstrap(project,command,extra=()):
        argv=[sys.executable,"-I","-m","agent_workflow.session_bootstrap",command,
              "--project",str(root/project),"--state-dir",str(root/project/".wayward-wrangler/state"),
              "--registry-dir",str(root/"registry"),"--herdr-socket",socket,
              "--ipc-dir",str(root/"runtime"),"--release-id",args.release_id,
              "--frontend",str(args.frontend),*extra]
        result=subprocess.run(argv,env=env,cwd=root,capture_output=True,text=True,timeout=30)
        if result.returncode:
            raise AssertionError((argv,result.stdout,result.stderr))
        return json.loads(result.stdout)

    def call(ready,kind,body):
        return _request(ready["ipc"],ready["project_key"],args.release_id,kind,body)

    def snapshot(ready):return call(ready,"snapshot",{})
    def durable_phase(ready):
        with sqlite3.connect(Path(ready["state"])/"state.sqlite") as db:
            return json.loads(db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone()[0])
    def command(ready,action,ident,payload=None):
        return call(ready,"command",{"command_id":ident,"action":action,
                    "expected_state_revision":snapshot(ready)["state_revision"],
                    "payload":payload or {}})

    def send_lost_reply(ready,body):
        with network_socket.socket(network_socket.AF_UNIX) as peer:
            peer.settimeout(5)
            peer.connect(ready["ipc"])
            peer.sendall(encode("hello","lost-hello",{"protocol":{"major":1},
                "release_id":args.release_id,"project_key":ready["project_key"]}))
            assert parse_frame(peer.makefile("rb").readline())["body"]["accepted"]
            peer.sendall(encode("command","lost-command",body))
            # Deliberately drop the reply after sending the exact saved command.

    def start_project(name):
        ready=bootstrap(name,"prepare",("--setup",))
        assert ready["status"]=="ready",ready
        units.append(ready["unit"])
        profile=root/name/".wayward-wrangler/workflow.toml"
        profile.write_text(profile.read_text().replace("seconds = 3.0","seconds = 5.0"))
        pane=call(ready,"ui.attach",{})
        assert pane["status"] in ("created","reconnected"),pane
        assert eventually(lambda: (root/name/".wayward-wrangler/state/session/ui-claims").is_dir())
        return ready,pane

    def authorize(ready,pane,ident):
        queued=command(ready,"submit_message",ident,{"text":"Draft a short local decision-log specification."})
        task_id=queued["task_id"]
        eventually(lambda: next((row for row in snapshot(ready)["tasks"] if row["task_id"]==task_id and row["status"]=="proposed"),None),20)
        plan=call(ready,"task.plan",{"task_id":task_id,"workspace":pane["workspace_id"],"run_id":"same-run"})
        assert plan["plan"]["config"]["roles"]["author"]["provider"]=="fake"
        task=next(row for row in snapshot(ready)["tasks"] if row["task_id"]==task_id)
        result=call(ready,"command",{"command_id":"admit-same","action":"admit_task",
              "expected_task_version":task["version"],"expected_state_revision":task["state_revision"],
              "payload":{"task_id":task_id,"workspace":pane["workspace_id"],
                         "run_id":"same-run","profile":plan["plan"]["profile"],
                         "plan_digest":plan["plan_digest"]}})
        assert result["status"]=="accepted",result
        return task_id

    try:
        eventually(lambda:h.server_identity() if server.poll() is None else None)
        assert h.server_identity()["pid"]==owned["pid"]
        a,pane_a=start_project("a")
        task_a=authorize(a,pane_a,"submit-same")
        stage=root/"a/.wayward-wrangler/state/runs/same-run/operations/draft"
        eventually(lambda:(stage/"permit.json").exists() and (stage/"child.json").exists(),20)
        exit_body={"command_id":"exit-a","action":"stop_session",
                   "expected_state_revision":snapshot(a)["state_revision"],"payload":{}}
        send_lost_reply(a,exit_body)
        stopped=eventually(lambda:(lambda r:r if r["status"]=="accepted" else None)(
            call(a,"command_get",{"command_id":"exit-a"})),5)
        assert stopped["status"]=="accepted",stopped
        checks.append({"case":"active-work-exit","phase":stopped["session"]["phase"],
                       "draft_permitted":True})
        paused=eventually(lambda:(lambda s:s if s["phase"]=="paused" else None)(durable_phase(a)),25)
        assert (stage/"receipt.json").exists() and not (stage.parent/"critique-1/permit.json").exists()
        assert read(stage/"invocations.json")["count"]==1
        checks.append({"case":"drain-then-pause","phase":paused["phase"],
                       "draft_invocations":1,"new_stage_permitted":False})
        # The original service may already have exited. Bootstrap starts only A's bound unit.
        resumed=bootstrap("a","lifecycle",("--action","resume_session"))
        assert resumed["status"]=="accepted",resumed
        eventually(lambda:next((row for row in snapshot(a)["tasks"] if row["task_id"]==task_a and row["status"]=="awaiting-input"),None),35)
        assert read(stage/"invocations.json")["count"]==1
        checks.append({"case":"resume-no-duplicate-draft","draft_invocations":1,
                       "critique_invocations":read(stage.parent/"critique-1/invocations.json")["count"]})
        generation_before=read(root/"a/.wayward-wrangler/state/session/service-generation.json")["generation"]
        killed=subprocess.run(["systemctl","--user","kill","--kill-whom=main","--signal=KILL",a["unit"]],capture_output=True,text=True)
        assert killed.returncode==0,killed.stderr
        eventually(lambda:read(root/"a/.wayward-wrangler/state/session/service-generation.json")["generation"]>generation_before,15)
        eventually(lambda:snapshot(a),15)
        assert read(stage/"invocations.json")["count"]==1
        checks.append({"case":"selected-service-restart","generation_advanced":True,"duplicate_draft":False})
        b,pane_b=start_project("b")
        task_b=authorize(b,pane_b,"submit-same")
        eventually(lambda:(root/"b/.wayward-wrangler/state/runs/same-run/operations/draft/permit.json").exists(),20)
        assert snapshot(a)["session"]["phase"]=="running"
        assert task_a==task_b and a["project_key"]!=b["project_key"]
        checks.append({"case":"two-project-same-IDs","a_awaiting_input":True,
                       "b_worker_permitted":True,"separate_project_keys":True})
        h.call("tab.close",{"tab_id":pane_b["tab_id"]})
        eventually(lambda:next((row for row in snapshot(b)["tasks"] if row["task_id"]==task_b and row["status"]=="awaiting-input"),None),35)
        assert read(root/"b/.wayward-wrangler/state/runs/same-run/operations/draft/invocations.json")["count"]==1
        checks.append({"case":"detach-b-ui-continues-work","b_reached_question":True,"a_unchanged":True})
        c,pane_c=start_project("c")
        stop_c=command(c,"stop_session","exit-c-race")
        assert stop_c["status"]=="accepted"
        resume_c=command(c,"resume_session","resume-c-race")
        assert resume_c["status"]=="accepted" and resume_c["session"]["generation"]==stop_c["session"]["generation"]+1
        time.sleep(1.5)
        assert snapshot(c)["session"]["phase"]=="running"
        checks.append({"case":"resume-wins-before-shutdown","old_exit_cannot_stop_resumed_service":True,
                       "generation":resume_c["session"]["generation"]})
        # Automated fake-only fixture consent; never evidence of operator approval.
        detail=call(a,"task.get",{"task_id":task_a})
        question=detail["pending"]
        assert question["kind"]=="choice" and "7 days" in question["question"]["options"]
        decision=call(a,"command",{"command_id":"fixture-answer-a","action":"answer_task",
            "expected_task_version":detail["task"]["version"],
            "expected_state_revision":detail["task"]["state_revision"],
            "payload":{"task_id":task_a,"request_id":question["request_id"],"outcome":"answered",
                       "answer":"7 days","artifact_sha256":question["artifact_sha256"]}})
        assert decision["status"]=="accepted",decision
        final=eventually(lambda:(lambda d:d if d["task"]["status"]=="awaiting-input" and d.get("pending") and d["pending"].get("kind")=="final-review" else None)(
            call(a,"task.get",{"task_id":task_a})),45)
        sha=final["pending"]["artifact_sha256"]
        stale=call(a,"command",{"command_id":"fixture-stale-approval","action":"answer_task",
            "expected_task_version":final["task"]["version"],
            "expected_state_revision":final["task"]["state_revision"],
            "payload":{"task_id":task_a,"request_id":final["pending"]["request_id"],
                       "outcome":"approved","answer":"Fake fixture approval",
                       "artifact_sha256":"0"*64}})
        assert stale["status"]=="rejected",stale
        approved=call(a,"command",{"command_id":"fixture-approve-a","action":"answer_task",
            "expected_task_version":final["task"]["version"],
            "expected_state_revision":final["task"]["state_revision"],
            "payload":{"task_id":task_a,"request_id":final["pending"]["request_id"],
                       "outcome":"approved","answer":"Fake fixture approval",
                       "artifact_sha256":sha}})
        assert approved["status"]=="accepted",approved
        eventually(lambda:next((row for row in snapshot(a)["tasks"] if row["task_id"]==task_a and row["status"]=="approved"),None),10)
        with sqlite3.connect(Path(a["state"])/"state.sqlite") as db:
            saved=json.loads(db.execute("SELECT payload FROM app_responses WHERE run_id=? AND request_id=?",
                                        ("same-run",final["pending"]["request_id"])).fetchone()[0])
        assert saved["artifact_sha256"]==sha
        generation_before=read(root/"a/.wayward-wrangler/state/session/service-generation.json")["generation"]
        killed=subprocess.run(["systemctl","--user","kill","--kill-whom=main","--signal=KILL",a["unit"]],capture_output=True,text=True)
        assert killed.returncode==0,killed.stderr
        eventually(lambda:read(root/"a/.wayward-wrangler/state/session/service-generation.json")["generation"]>generation_before,15)
        eventually(lambda:snapshot(a),15)
        with sqlite3.connect(Path(a["state"])/"state.sqlite") as db:
            after=json.loads(db.execute("SELECT payload FROM app_responses WHERE run_id=? AND request_id=?",
                                        ("same-run",final["pending"]["request_id"])).fetchone()[0])
        assert after==saved and next(row for row in snapshot(a)["tasks"] if row["task_id"]==task_a)["status"]=="approved"
        checks.append({"case":"fixture-approval-binding-after-restart","artifact_sha256":sha,
                       "stale_refused":True,"approval_unchanged":True,"fixture_only":True})
        atomic(root/"result.json",{"schema_version":1,"passed":True,"checks":checks,
                                    "installed_package":agent_workflow.__file__,"release_id":args.release_id,
                                    "server":owned},immutable=True)
    finally:
        for unit in units:
            subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
            report=subprocess.run(["journalctl","--user","-u",unit,"-n","80","--no-pager"],capture_output=True,text=True)
            (root/f"{unit}.journal.log").write_text(report.stdout)
        if server.poll() is None:
            try:
                peer=h.server_identity()
                if all(peer[k]==owned[k] for k in ("pid","start","boot")):
                    h.call("server.stop")
            except (OSError,ValueError):pass
            try:server.wait(timeout=10)
            except subprocess.TimeoutExpired:server.terminate();server.wait(timeout=5)
    print(json.dumps(read(root/"result.json"),indent=2))


if __name__=="__main__":main()
