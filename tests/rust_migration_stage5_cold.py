"""Ordinary-terminal cold start with private Herdr and project systemd unit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid

from rust_migration_stage5 import BIN,call_herdr,save,wait

def main(root):
    assert not root.exists(),"Use fresh evidence"
    root.mkdir(parents=True)
    private=Path(tempfile.mkdtemp(prefix="ww-rust-m5-cold-"))
    (private/"config/herdr").mkdir(parents=True)
    (private/"config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    (private/"runtime").mkdir()
    (private/"home").mkdir()
    sock=str(private/"runtime/herdr.sock")
    key=uuid.uuid4().hex;unit=f"waywr-{key}.service"
    project=root/"project";config=project/".wayward-wrangler";config.mkdir(parents=True)
    (config/"project.toml").write_text(f'schema_version = 1\nproject_key = "{key}"\n')
    (config/"workflow.toml").write_text('review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "deterministic-author-v1"\n[roles.critic]\nprovider = "fake"\nmodel = "deterministic-critic-v1"\n[fake]\nseconds = 0.1\n')
    release=root/"releases/rust-stage5-test";binary=release/"bin";binary.mkdir(parents=True)
    for name in ("waywr","waywr-service","waywr-runner"):shutil.copy2(BIN/name,binary/name)
    save(release/"manifest.json",{"schema_version":1,"release_id":"rust-stage5-test","frontend":"0.4.0",
        "backend":"0.1.0","backend_kind":"rust","protocol_major":1,"state_schema_min":2,"state_schema_max":2})
    digest=hashlib.sha256((release/"manifest.json").read_bytes()).hexdigest()
    save(config/"release.json",{"schema_version":1,"project_key":key,"release_id":"rust-stage5-test",
        "manifest_sha256":digest,"binding_generation":1})
    env={**os.environ,"HOME":str(private/"home"),"XDG_CONFIG_HOME":str(private/"config"),
        "HERDR_SOCKET_PATH":sock,"TERM":"xterm-256color"}
    env.pop("HERDR_ENV",None)
    command=[str(binary/"waywr"),"--project",str(project),"--herdr-socket",sock,
        "--ipc-dir",str(private/"runtime")]
    # `script` allocates a real PTY and preserves the launcher's foreground
    # terminal contract. No shared Herdr client or socket is involved.
    import shlex
    log=(root/"ordinary-terminal.log").open("wb")
    client=subprocess.Popen(["script","-qfc","stty rows 40 cols 120; exec "+shlex.join(command),"/dev/null"],cwd=project,
        env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    log.close()
    cleanup={"unit":unit,"unit_stopped":False,"private_herdr_stopped":False,"terminal_client_stopped":False}
    try:
        state=config/"state"
        wait(lambda:Path(sock).is_socket() and (state/"session/ui-current.json").is_file(),30)
        binding=json.loads((state/"session/ui-current.json").read_text())
        assert binding["project_key"]==key
        assert binding["pane"]["workspace_id"]
        assert subprocess.run(["systemctl","--user","is-active",unit],capture_output=True,text=True).stdout.strip()=="active"
        save(root/"binding.json",binding)
        save(root/"summary.json",{"status":"passed","checks":["Cold ordinary-terminal waywr launched supported Herdr client",
            "Private Herdr server and Rust project service started","Owned UI pane registered"]})
        print(root/"summary.json")
    finally:
        subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
        cleanup["unit_stopped"]=subprocess.run(["systemctl","--user","is-active",unit],
            capture_output=True,text=True).stdout.strip()!="active"
        if Path(sock).is_socket():
            try:call_herdr(sock,"server.stop")
            except (OSError,RuntimeError):pass
        try:wait(lambda:not Path(sock).exists(),8)
        except AssertionError:pass
        cleanup["private_herdr_stopped"]=not Path(sock).is_socket()
        try:client.wait(timeout=8)
        except subprocess.TimeoutExpired:
            os.killpg(client.pid,15);client.wait(timeout=8)
        cleanup["terminal_client_stopped"]=client.poll() is not None
        save(root/"cleanup.json",cleanup)

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--evidence-root",type=Path,required=True)
    main(parser.parse_args().evidence_root.resolve())
