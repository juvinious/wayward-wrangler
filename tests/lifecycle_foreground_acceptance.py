"""Installed ordinary-terminal /exit path using only an isolated Herdr server."""
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


def eventually(check,seconds=15):
    until=time.monotonic()+seconds
    while time.monotonic()<until:
        try:
            result=check()
            if result:return result
        except (FileNotFoundError,ConnectionRefusedError):pass
        time.sleep(.1)
    raise AssertionError("foreground UI did not reach expected isolated state")


def main():
    p=argparse.ArgumentParser()
    p.add_argument("--root",type=Path,required=True)
    p.add_argument("--binary",type=Path,required=True)
    args=p.parse_args()
    root=args.root.resolve();root.mkdir(parents=True,exist_ok=False)
    binary=args.binary.resolve(strict=True)
    for part in ("home","config/herdr","runtime","registry","project"):
        (root/part).mkdir(parents=True,exist_ok=True)
    (root/"config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket=str(root/"runtime/herdr.sock")
    env={**os.environ,"HOME":str(root/"home"),"XDG_CONFIG_HOME":str(root/"config"),
         "HERDR_SOCKET_PATH":socket,"SHELL":"/bin/sh","TERM":"xterm-256color"}
    env.pop("HERDR_ENV",None)
    log=(root/"server.log").open("wb")
    server=subprocess.Popen(["herdr","server"],env=env,stdout=log,stderr=subprocess.STDOUT)
    log.close();owned=identity(server.pid)
    h=Herdr(socket)
    master=slave=None;shell=None;unit=None;output=bytearray()
    try:
        eventually(lambda:h.server_identity() if server.poll() is None else None)
        master,slave=pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",30,110,0,0))
        argv=[str(binary),"--project",str(root/"project"),"--registry-dir",str(root/"registry"),
              "--herdr-socket",socket,"--ipc-dir",str(root/"runtime"),"--yes"]
        command=" ".join(shlex.quote(arg) for arg in argv)+"; printf '\\nWAYWR_RETURNED_TO_SHELL\\n'"
        shell=subprocess.Popen(["/bin/sh","-c",command],env=env,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
        state=root/"project/.wayward-wrangler/state"
        binding=eventually(lambda:read(state/"session/ui-current.json") if (state/"session/ui-current.json").exists() else None,25)
        unit="waywr-"+binding["project_key"]+".service"
        claim=state/"session/ui-claims"/(binding["token"]+".json")
        eventually(lambda:claim.exists(),10)
        pane=binding["pane"]["pane_id"]
        screen=h.call("pane.read",{"pane_id":pane,"source":"recent","lines":60})["read"]["text"]
        (root/"installed-screen.txt").write_text(screen)
        assert "Wayward Wrangler" in screen and "SIMULATED SERVICE" in screen,screen
        assert "New task request" in screen and "›" in screen,screen
        assert "Ctrl-S submit" not in screen and "Enter send" in screen,screen
        h.call("pane.send_text",{"pane_id":pane,"text":"/exit"})
        h.call("pane.send_keys",{"pane_id":pane,"keys":["enter"]})
        time.sleep(.25)
        h.call("pane.send_keys",{"pane_id":pane,"keys":["tab","enter"]})
        end=time.monotonic()+15
        while time.monotonic()<end and b"WAYWR_RETURNED_TO_SHELL" not in output:
            if select.select([master],[],[],.1)[0]:
                try:output.extend(os.read(master,65536))
                except OSError:break
        (root/"foreground.pty").write_bytes(output)
        phase=read(state/"session/checkpoints/latest.json")
        result={"schema_version":1,"returned_to_shell":b"WAYWR_RETURNED_TO_SHELL" in output,
                "server_unchanged":h.server_identity()["pid"]==owned["pid"],
                "checkpoint":phase,"pane":binding["pane"]}
        assert result["returned_to_shell"] and result["server_unchanged"],result
        shell.wait(timeout=3)
        os.close(master);os.close(slave);master=slave=None;shell=None
        master,slave=pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",30,110,0,0))
        resume_argv=[*argv[:-1],"resume"]
        resume_command=" ".join(shlex.quote(arg) for arg in resume_argv)+"; printf '\\nWAYWR_RESUME_DETACHED\\n'"
        shell=subprocess.Popen(["/bin/sh","-c",resume_command],env=env,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
        new_binding=eventually(lambda:(lambda value:value if value["token"]!=binding["token"] else None)(
            read(state/"session/ui-current.json")),25)
        eventually(lambda:(state/"session/ui-claims"/(new_binding["token"]+".json")).exists(),10)
        new_pane=new_binding["pane"]["pane_id"]
        h.call("pane.send_text",{"pane_id":new_pane,"text":"/detach"})
        h.call("pane.send_keys",{"pane_id":new_pane,"keys":["enter"]})
        time.sleep(.25)
        h.call("pane.send_keys",{"pane_id":new_pane,"keys":["tab","enter"]})
        second=bytearray();end=time.monotonic()+15
        while time.monotonic()<end and b"WAYWR_RESUME_DETACHED" not in second:
            if select.select([master],[],[],.1)[0]:
                try:second.extend(os.read(master,65536))
                except OSError:break
        (root/"resume-detach.pty").write_bytes(second)
        with sqlite3.connect(state/"state.sqlite") as db:
            session=json.loads(db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone()[0])
        result.update(resume_detach_returned=b"WAYWR_RESUME_DETACHED" in second,
                      resume_phase=session["phase"],new_pane=new_binding["pane"])
        (root/"result.json").write_text(json.dumps(result,indent=2)+"\n")
        assert result["resume_detach_returned"] and result["resume_phase"]=="running",result
    finally:
        if unit:
            subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
            journal=subprocess.run(["journalctl","--user","-u",unit,"-n","60","--no-pager"],capture_output=True,text=True)
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
                if all(peer[key]==owned[key] for key in ("pid","start","boot")):
                    h.call("server.stop")
            except (OSError,ValueError):pass
            try:server.wait(timeout=10)
            except subprocess.TimeoutExpired:server.terminate();server.wait(timeout=5)
    print((root/"result.json").read_text())


if __name__=="__main__":main()
