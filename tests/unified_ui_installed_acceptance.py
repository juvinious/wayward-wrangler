"""Private Herdr proof that installed waywr uses the accepted view and real project IPC.

All agents are deterministic fake workers. Any answer/approval is a test fixture,
never operator consent or evidence about a native provider.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shlex
import sqlite3
import struct
import subprocess
import termios
import time

from agent_workflow.herdr_backend import Herdr
from agent_workflow.storage import identity, read
import agent_workflow


def eventually(check, seconds=35):
    limit=time.monotonic()+seconds
    while time.monotonic()<limit:
        try:
            result=check()
            if result:return result
        except (FileNotFoundError,ConnectionRefusedError,sqlite3.OperationalError):pass
        time.sleep(.1)
    raise AssertionError("isolated installed UI condition not reached")


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--root",type=Path,required=True)
    parser.add_argument("--binary",type=Path,required=True)
    args=parser.parse_args()
    root=args.root.resolve();root.mkdir(parents=True,exist_ok=False)
    binary=args.binary.resolve(strict=True)
    assert "/site-packages/agent_workflow/" in str(Path(agent_workflow.__file__).resolve()),agent_workflow.__file__
    for item in ("home","config/herdr","runtime","registry","project"):
        (root/item).mkdir(parents=True,exist_ok=True)
    (root/"config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket=str(root/"runtime/herdr.sock")
    env={**os.environ,"HOME":str(root/"home"),"XDG_CONFIG_HOME":str(root/"config"),
         "HERDR_SOCKET_PATH":socket,"SHELL":"/bin/sh","TERM":"xterm-256color"}
    env.pop("HERDR_ENV",None)
    # Exercise the color-capable pane used in normal Ghostty/Herdr operation.
    # Inheriting NO_COLOR hid a coordinator runner startup failure before permit.
    env.pop("NO_COLOR",None)
    log=(root/"herdr-server.log").open("wb")
    server=subprocess.Popen(["herdr","server"],env=env,stdout=log,stderr=subprocess.STDOUT)
    log.close();owned=identity(server.pid);h=Herdr(socket)
    master=slave=shell=unit=None
    state=root/"project/.wayward-wrangler/state"
    checks=[]

    def pane_text(pane):
        return h.call("pane.read",{"pane_id":pane,"source":"recent","lines":90})["read"]["text"]

    def task():
        with sqlite3.connect(state/"state.sqlite") as db:
            db.row_factory=sqlite3.Row
            item=db.execute("SELECT * FROM session_tasks ORDER BY rowid LIMIT 1").fetchone()
            return dict(item) if item else None

    def send(pane,value):
        h.call("pane.send_text",{"pane_id":pane,"text":value})
        # Herdr delivers text and key events asynchronously to the pane PTY.
        # Let the slash menu consume the text before Enter selects its action.
        time.sleep(.15)
        h.call("pane.send_keys",{"pane_id":pane,"keys":["enter"]})

    try:
        eventually(lambda:h.server_identity() if server.poll() is None else None)
        master,slave=pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",36,120,0,0))
        argv=[str(binary),"--project",str(root/"project"),"--registry-dir",str(root/"registry"),
              "--herdr-socket",socket,"--ipc-dir",str(root/"runtime"),"--yes"]
        command=" ".join(shlex.quote(item) for item in argv)+"; printf '\\nWAYWR_UNIFIED_RETURNED\\n'"
        shell=subprocess.Popen(["/bin/sh","-c",command],env=env,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
        binding=eventually(lambda:read(state/"session/ui-current.json") if (state/"session/ui-current.json").exists() else None,25)
        unit="waywr-"+binding["project_key"]+".service"
        pane=binding["pane"]["pane_id"]
        screen=eventually(lambda:(lambda text:text if "SIMULATED SERVICE" in text and "New task request" in text else None)(pane_text(pane)))
        (root/"initial-screen.txt").write_text(screen)
        assert "Wayward Wrangler" in screen and "•" not in screen and "›" in screen
        assert "Ctrl-S submit" not in screen and "Project Decision Log" not in screen
        checks.append("accepted-conversation-layout-in-installed-pane")

        send(pane,"Draft a short local decision-log specification.")
        proposed=eventually(lambda:(lambda row:row if row and row["status"]=="proposed" else None)(task()),25)
        assert proposed["body"]=="Draft a short local decision-log specification."
        eventually(lambda:(lambda text:text if "Task proposal ready" in text else None)(pane_text(pane)),10)
        checks.append("enter-submitted-durable-task")
        send(pane,"/plan")
        scope=eventually(lambda:(lambda text:text if "Review scope" in text and "Maximum worker calls" in text else None)(pane_text(pane)),15)
        (root/"scope-screen.txt").write_text(scope)
        assert task()["status"]=="proposed"
        h.call("pane.send_keys",{"pane_id":pane,"keys":["tab","enter"]})
        eventually(lambda:(lambda row:row if row and row["run_id"] else None)(task()),20)
        checks.append("scope-authorized-only-by-panel-action")

        awaiting=eventually(lambda:(lambda row:row if row and row["status"]=="awaiting-input" else None)(task()),40)
        eventually(lambda:(lambda text:text if "awaiting-input" in text else None)(pane_text(pane)),10)
        send(pane,"/answer")
        try:
            question=eventually(lambda:(lambda text:text if "Your answer needed" in text and "7 days" in text else None)(pane_text(pane)),15)
        except AssertionError:
            (root/"question-failure-screen.txt").write_text(pane_text(pane))
            raise
        (root/"question-screen.txt").write_text(question)
        h.call("pane.send_keys",{"pane_id":pane,"keys":["1","tab","enter"]})
        with sqlite3.connect(state/"state.sqlite") as db:
            answered=eventually(lambda:db.execute("SELECT payload FROM app_responses WHERE run_id=?",(awaiting["run_id"],)).fetchone(),15)
        response=json.loads(answered[0]);assert response["answer"]=="7 days",response
        checks.append("fixture-answer-saved-through-visible-question")

        def final_gate():
            with sqlite3.connect(state/"state.sqlite") as db:
                count=db.execute("SELECT count(*) FROM session_events WHERE type='task.state_changed' AND body LIKE '%awaiting-input%'").fetchone()[0]
            row=task()
            return row if count>=2 and row and row["status"]=="awaiting-input" else None
        eventually(final_gate,40)
        (root/"before-review-screen.txt").write_text(pane_text(pane))
        time.sleep(1.5)  # allow the UI's next bounded event/snapshot update
        # The second gate is a final review. This isolated fake-only fixture
        # deliberately checks UI approval binding; it is not operator consent.
        send(pane,"/approve")
        time.sleep(.25)
        if "/ commands" in pane_text(pane) and "/approve" in pane_text(pane):
            h.call("pane.send_keys",{"pane_id":pane,"keys":["enter"]})
        try:
            review=eventually(lambda:(lambda text:text if "Final review" in text and "Read candidate" in text else None)(pane_text(pane)),15)
        except AssertionError:
            (root/"review-failure-screen.txt").write_text(pane_text(pane))
            raise
        (root/"review-screen.txt").write_text(review)
        assert task()["status"]=="awaiting-input"
        checks.append("final-review-visible-before-fixture-approval")

        h.call("pane.send_keys",{"pane_id":pane,"keys":["tab","enter"]})
        artifact=eventually(lambda:(lambda text:text if "Candidate · exact project artifact" in text and "SHA-256" in text else None)(pane_text(pane)),10)
        (root/"artifact-screen.txt").write_text(artifact)
        h.call("pane.send_keys",{"pane_id":pane,"keys":["enter"]})
        time.sleep(.2)
        h.call("pane.send_keys",{"pane_id":pane,"keys":["tab","enter"]})
        approved=eventually(lambda:(lambda row:row if row and row["status"]=="approved" else None)(task()),15)
        with sqlite3.connect(state/"state.sqlite") as db:
            payloads=[json.loads(item[0]) for item in db.execute("SELECT payload FROM app_responses WHERE run_id=?",(approved["run_id"],))]
        final=[item for item in payloads if item["outcome"]=="approved"]
        assert len(final)==1 and len(final[0]["artifact_sha256"])==64,final
        checks.append("fixture-approval-bound-to-opened-artifact")

        h.call("pane.send_keys",{"pane_id":pane,"keys":["esc"]})
        send(pane,"/detach")
        h.call("pane.send_keys",{"pane_id":pane,"keys":["tab","enter"]})
        output=bytearray();limit=time.monotonic()+15
        while time.monotonic()<limit and b"WAYWR_UNIFIED_RETURNED" not in output:
            if select.select([master],[],[],.1)[0]:
                try:output.extend(os.read(master,65536))
                except OSError:break
        (root/"foreground.pty").write_bytes(output)
        assert b"WAYWR_UNIFIED_RETURNED" in output
        assert h.server_identity()["pid"]==owned["pid"]
        checks.append("detach-returned-shell-service-preserved")
        (root/"result.json").write_text(json.dumps({"passed":True,"fixture_only":True,"checks":checks,
            "task_id":proposed["task_id"],"run_id":awaiting["run_id"],"server":owned},indent=2)+"\n")
    finally:
        if unit:
            subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
            journal=subprocess.run(["journalctl","--user","-u",unit,"-n","80","--no-pager"],capture_output=True,text=True)
            (root/"service.journal.log").write_text(journal.stdout)
        if shell and shell.poll() is None:
            shell.terminate()
            try:shell.wait(timeout=3)
            except subprocess.TimeoutExpired:shell.kill();shell.wait(timeout=3)
        for fd in (master,slave):
            if fd is not None:os.close(fd)
        if server.poll() is None:
            try:
                peer=h.server_identity()
                if all(peer[k]==owned[k] for k in ("pid","start","boot")):
                    h.call("server.stop")
            except (OSError,ValueError):pass
            try:server.wait(timeout=10)
            except subprocess.TimeoutExpired:server.terminate();server.wait(timeout=5)
    print((root/"result.json").read_text())


if __name__=="__main__":main()
