"""Private Herdr plugin and relocated completed-artifact review, Rust only."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import uuid

from rust_migration_stage5 import HERDR,call_herdr,save,wait

def main(root,release,source):
    assert not root.exists(),"Use fresh evidence"
    root.mkdir(parents=True)
    report=json.loads((source/"summary.json").read_text())
    project=source/"project";key=report["project_key"]
    sock=report["private_herdr_socket"]
    private=Path(sock).parent.parent
    state=project/".wayward-wrangler/state"
    old_server=json.loads((state/"session/ui-current.json").read_text())["server"]
    unit=f"waywr-{key}.service"
    active=subprocess.run(["systemctl","--user","is-active",unit],capture_output=True,text=True)
    assert active.stdout.strip()!="active","Refusing to touch an already-active service"
    env={**os.environ,"HOME":str(private/"home"),"XDG_CONFIG_HOME":str(private/"config"),
        "HERDR_SOCKET_PATH":sock,"TERM":"xterm-256color"}
    env.pop("HERDR_ENV",None)
    # Retain the real user systemd bus while using this fixture's explicit
    # private Herdr socket/configuration.
    log=(root/"herdr-server.log").open("wb")
    server=subprocess.Popen([str(HERDR),"server"],env=env,stdout=log,stderr=subprocess.STDOUT)
    log.close()
    cleanup={"unit":unit,"unit_stopped":False,"private_server_stopped":False}
    try:
        wait(lambda:server.poll() is None and Path(sock).is_socket())
        workspace=call_herdr(sock,"workspace.create",{"label":"Rust review reconnect","cwd":str(project),"focus":False})["workspace"]["workspace_id"]
        call_herdr(sock,"layout.apply",{"workspace_id":workspace,"tab_label":"Waywr reconnect","focus":False,
            "root":{"type":"pane","label":"Waywr reconnect","cwd":str(project),
                "command":[str(release/"bin/waywr"),"--project",str(project),"--herdr-socket",sock,
                    "--ipc-dir",str(private/"runtime")]}})
        binding=wait(lambda:json.loads((state/"session/ui-current.json").read_text())
            if (state/"session/ui-current.json").is_file() and
                json.loads((state/"session/ui-current.json").read_text())["server"]!=old_server
            else None,20)
        review_workspace=binding["pane"]["workspace_id"]
        # The source path is the same retained state; check the binding's
        # process claim and fresh server identity instead of rewriting history.
        wait(lambda:(state/"session/ui-claims"/f'{binding["token"]}.json').exists(),10)
        registry=private/"home/.local/state/wayward-wrangler"
        destination=private/"data/rust-review-plugin"
        installed=subprocess.run([str(release/"bin/waywr"),"--registry-dir",str(registry),
            "integration","install-herdr","--destination",str(destination)],env=env,cwd=project,
            capture_output=True,text=True)
        (root/"plugin-install-stdout.log").write_text(installed.stdout)
        (root/"plugin-install-stderr.log").write_text(installed.stderr)
        assert installed.returncode==0,installed.stderr
        manifest=(destination/"herdr-plugin.toml").read_text()
        assert "python" not in manifest.lower() and str(release/"bin/waywr") in manifest
        shutil.copy2(destination/"herdr-plugin.toml",root/"herdr-plugin.toml")
        shutil.copy2(destination/"binding.json",root/"plugin-binding.json")
        listed=subprocess.run([str(HERDR),"plugin","list","--json"],env=env,capture_output=True,text=True,check=True)
        save(root/"plugin-list.json",json.loads(listed.stdout))
        assert any(p["plugin_id"]=="local.agent-workflow-review" for p in json.loads(listed.stdout)["result"]["plugins"])
        run="r-"+report["task_id"]
        url=f'https://awp.invalid/v2/review/{key}/{run}/revision-2/{report["artifact_sha256"]}'
        for index in (1,2):
            call_herdr(sock,"layout.apply",{"workspace_id":workspace,"tab_label":f"Test review action {index}","focus":False,
                "root":{"type":"pane","label":f"Test review action {index}","cwd":str(project),
                    "command":["env",f"HERDR_PLUGIN_CLICKED_URL={url}",str(release/"bin/waywr"),
                        "--registry-dir",str(registry),"open-link"]}})
            def reviews():
                tabs=call_herdr(sock,"tab.list")["tabs"]
                return [tab for tab in tabs if tab["workspace_id"]==review_workspace and
                    tab["label"].startswith("Wayward Wrangler review")]
            wait(lambda:len(reviews())==1,15)
        assert len(reviews())==1,"Review click created duplicate Vim tabs"
        assert (state/"runs"/run/"artifacts/revision-2.md").is_file()
        save(root/"summary.json",{"status":"passed","review_tabs":len(reviews()),
            "project_key":key,"artifact_sha256":report["artifact_sha256"],
            "checks":["Plugin copied/linked only in private Herdr configuration without Python",
                "Completed artifact opened in verified current UI workspace after old server loss",
                "Repeated review action focused one read-only Vim tab without duplicate"]})
        print(root/"summary.json")
    finally:
        subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
        cleanup["unit_stopped"]=subprocess.run(["systemctl","--user","is-active",unit],
            capture_output=True,text=True).stdout.strip()!="active"
        if server.poll() is None:
            try:call_herdr(sock,"server.stop")
            except (OSError,RuntimeError):server.terminate()
            server.wait(timeout=8)
        cleanup["private_server_stopped"]=server.poll() is not None
        logs=subprocess.run(["journalctl","--user-unit",unit,"-n","80","--no-pager"],
            capture_output=True,text=True)
        (root/"unit.log").write_text(logs.stdout)
        save(root/"cleanup.json",cleanup)

if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("--evidence-root",type=Path,required=True)
    p.add_argument("--release-root",type=Path,required=True)
    p.add_argument("--approved-fixture",type=Path,required=True)
    a=p.parse_args();main(a.evidence_root.resolve(),a.release_root.resolve(),a.approved_fixture.resolve())
