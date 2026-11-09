"""Isolated project-unit bootstrap and restart through a real Herdr pane.

Only the randomly keyed test unit and private Herdr server are stopped.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid

from rust_migration_stage5 import BIN, HERDR, call_herdr, call_service, save, wait

def main(root):
    assert not root.exists(), "Use fresh evidence"
    root.mkdir(parents=True)
    private=Path(tempfile.mkdtemp(prefix="ww-rust-m5-unit-"))
    (private/"config/herdr").mkdir(parents=True)
    (private/"config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    (private/"runtime").mkdir()
    sock=str(private/"runtime/herdr.sock")
    key=uuid.uuid4().hex
    unit=f"waywr-{key}.service"
    project=root/"project"; config=project/".wayward-wrangler";config.mkdir(parents=True)
    (config/"project.toml").write_text(f'schema_version = 1\nproject_key = "{key}"\n')
    (config/"workflow.toml").write_text('review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "deterministic-author-v1"\n[roles.critic]\nprovider = "fake"\nmodel = "deterministic-critic-v1"\n[fake]\nseconds = 0.1\n')
    release=root/"releases/rust-stage5-test";binary=release/"bin";binary.mkdir(parents=True)
    for name in ("waywr","waywr-service","waywr-runner"):
        shutil.copy2(BIN/name,binary/name)
    save(release/"manifest.json",{"schema_version":1,"release_id":"rust-stage5-test",
        "frontend":"0.4.0","backend":"0.1.0","backend_kind":"rust",
        "protocol_major":1,"state_schema_min":2,"state_schema_max":2})
    digest=hashlib.sha256((release/"manifest.json").read_bytes()).hexdigest()
    save(config/"release.json",{"schema_version":1,"project_key":key,"release_id":"rust-stage5-test",
        "manifest_sha256":digest,"binding_generation":1})
    endpoint=private/"runtime"/"waywr"/f"{key}.sock"
    env={**os.environ,"HOME":str(private/"home"),"XDG_CONFIG_HOME":str(private/"config"),
        "HERDR_SOCKET_PATH":sock,"TERM":"xterm-256color"}
    (private/"home").mkdir()
    env.pop("HERDR_ENV",None)
    log=(root/"herdr-server.log").open("wb")
    server=subprocess.Popen([str(HERDR),"server"],env=env,stdout=log,stderr=subprocess.STDOUT)
    log.close()
    cleanup={"unit":unit,"server_stopped":False,"unit_stopped":False}
    try:
        wait(lambda: server.poll() is None and Path(sock).is_socket())
        workspace=call_herdr(sock,"workspace.create",{"label":"Rust unit proof","cwd":str(project),"focus":False})["workspace"]["workspace_id"]
        call_herdr(sock,"layout.apply",{"workspace_id":workspace,"tab_label":"waywr unit start","focus":False,
            "root":{"type":"pane","label":"waywr unit start","cwd":str(project),
                "command":[str(binary/"waywr"),"--project",str(project),"--herdr-socket",sock,
                    "--ipc-dir",str(private/"runtime")]}})
        state=config/"state"
        wait(lambda: endpoint.is_socket() and (state/"session/ui-current.json").exists(),30)
        active=subprocess.run(["systemctl","--user","show","--property=ActiveState","--value",unit],capture_output=True,text=True,check=True)
        assert active.stdout.strip()=="active",active.stdout
        first=call_service(str(endpoint),key,"command",{"command_id":"first","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"Specify a decision log."}})
        assert first["body"]["status"]=="accepted",first
        task=first["body"]["task_id"]
        wait(lambda:(lambda x:x if x["body"]["task"]["status"]=="proposed" else None)(
            call_service(str(endpoint),key,"task.get",{"task_id":task})),35)
        receipts=list((state/"session/coordinator"/task).glob("*/receipt.json"))
        assert len(receipts)==1
        subprocess.run(["systemctl","--user","kill","--signal=KILL",unit],check=True)
        def restored():
            if not endpoint.is_socket():return False
            try:return call_service(str(endpoint),key,"snapshot",{})["body"]["tasks"][0]["status"]=="proposed"
            except (OSError,KeyError,IndexError):return False
        wait(restored,30)
        assert len(list((state/"session/coordinator"/task).glob("*/receipt.json")))==1
        snapshot=call_service(str(endpoint),key,"snapshot",{})["body"]
        stop=call_service(str(endpoint),key,"command",{"command_id":"exit","action":"stop_session",
            "expected_state_revision":snapshot["state_revision"],"payload":{}})
        assert stop["body"]["status"]=="accepted",stop
        wait(lambda:subprocess.run(["systemctl","--user","show","--property=ActiveState","--value",unit],
            capture_output=True,text=True).stdout.strip() in ("inactive","failed"),20)
        save(root/"summary.json",{"status":"passed","unit":unit,"task_id":task,
            "checks":["systemd-run started project-scoped Rust service via managed Herdr pane",
                "KILL restart preserved one completed coordinator receipt and proposal",
                "stop drained and closed only this test project service"]})
        print(root/"summary.json")
    finally:
        subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
        cleanup["unit_stopped"]=subprocess.run(["systemctl","--user","is-active",unit],
            capture_output=True,text=True).stdout.strip()!="active"
        if server.poll() is None:
            try:call_herdr(sock,"server.stop")
            except (OSError,RuntimeError):server.terminate()
            server.wait(timeout=8)
        cleanup["server_stopped"]=server.poll() is not None
        save(root/"cleanup.json",cleanup)

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--evidence-root",type=Path,required=True)
    main(parser.parse_args().evidence_root.resolve())
