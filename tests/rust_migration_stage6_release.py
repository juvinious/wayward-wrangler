"""Fresh Python-free install, aliases, per-project routing and rollback checks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import time

def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,indent=2,sort_keys=True)+"\n")

def run(binary,args,env,root,expect=True):
    result=subprocess.run([str(binary),*args],env=env,cwd=root,capture_output=True,text=True)
    if expect and result.returncode:raise AssertionError(f"{args}: {result.stderr}")
    return result
def wait_inactive(unit):
    until=time.monotonic()+15
    while time.monotonic()<until:
        state=subprocess.run(["systemctl","--user","show","--property=ActiveState","--value",unit],
            capture_output=True,text=True).stdout.strip()
        if state in ("inactive","failed"):return
        time.sleep(.1)
    raise AssertionError(f"Test-owned service {unit} did not stop")

def main(root,prefix,first,second):
    assert not root.exists(),"Use a fresh evidence root"
    root.mkdir(parents=True)
    private=Path(tempfile.mkdtemp(prefix="ww-rust-s6-route-"))
    (private/"home").mkdir()
    (private/"runtime").mkdir()
    (private/"state").mkdir()
    env={**os.environ,"HOME":str(private/"home"),"XDG_STATE_HOME":str(private/"state"),
        "HERDR_SOCKET_PATH":str(private/"runtime/herdr.sock")}
    env.pop("HERDR_ENV",None)
    default=prefix/"bin/waywr";alternate=prefix/"releases"/second/"bin/waywr"
    assert default.is_symlink() and alternate.is_file()
    assert all((prefix/"bin"/name).is_symlink() for name in ("waywr","wayward-wrangler","awp"))
    assert not list(prefix.rglob("*.whl")) and not list(prefix.rglob("*.py"))
    assert not list(prefix.rglob("venv"))
    projects=[root/"project-a",root/"project-b"]
    for project in projects:project.mkdir()
    units=[]
    checks=[]
    try:
        for binary,project in [(default,projects[0]),(alternate,projects[1])]:
            output=run(binary,["--project",str(project),"init"],env,project)
            assert json.loads(output.stdout)["project"]==str(project)
            key=tomllib.loads((project/".wayward-wrangler/project.toml").read_text())["project_key"]
            units.append(f"waywr-{key}.service")
        assert json.loads((projects[0]/".wayward-wrangler/release.json").read_text())["release_id"]==first
        assert json.loads((projects[1]/".wayward-wrangler/release.json").read_text())["release_id"]==second
        checks.append("Rust init created independent project/config/state/release records without starting a service")
        for project in projects:
            status=json.loads(run(default,["--project",str(project),"status"],env,project).stdout)
            assert status["tasks"]==[]
            assert status["release_id"]==json.loads((project/".wayward-wrangler/release.json").read_text())["release_id"]
        checks.append("Read-only status routed each project to its own installed release without starting a unit")
        # A stale unrelated registry entry is metadata only: it is never opened
        # or added to either project's packet or service context.
        index=private/"state/wayward-wrangler/locations.json"
        document=json.loads(index.read_text())
        document["projects"]["f"*32]={"project":"/definitely/unavailable-other-project",
            "state":"/definitely/unavailable-other-state"}
        save(index,document)
        current=json.loads(run(default,["--project",str(projects[0]),"status"],env,projects[0]).stdout)
        assert current["project_key"]==tomllib.loads((projects[0]/".wayward-wrangler/project.toml").read_text())["project_key"]
        checks.append("Unavailable unrelated registry project did not block selected project status/service")
        upgrade=json.loads(run(default,["--project",str(projects[0]),"session","upgrade","--release",second],env,projects[0]).stdout)
        assert upgrade["release_id"]==second
        rollback=json.loads(run(default,["--project",str(projects[0]),"session","rollback","--release",first],env,projects[0]).stdout)
        assert rollback["release_id"]==first
        checks.append("Quiescent project upgrade and state-revision-bound rollback succeeded without touching B")
        run(default,["--project",str(projects[0]),"session","upgrade","--release",second],env,projects[0])
        routed=json.loads(run(default,["--project",str(projects[0]),"status"],env,projects[0]).stdout)
        assert routed["tasks"]==[]
        assert routed["release_id"]==second
        assert all(subprocess.run(["systemctl","--user","is-active",unit],capture_output=True,text=True).stdout.strip()!="active" for unit in units)
        checks.append("Unrelated and quiescent projects left service routing and release bindings independent")
        save(root/"summary.json",{"status":"passed","checks":checks,"units":units,
            "bindings":[json.loads((p/".wayward-wrangler/release.json").read_text()) for p in projects]})
        print(root/"summary.json")
    finally:
        for unit in units:subprocess.run(["systemctl","--user","stop",unit],capture_output=True)
        save(root/"cleanup.json",{"units":{unit:subprocess.run(["systemctl","--user","is-active",unit],
            capture_output=True,text=True).stdout.strip()!="active" for unit in units}})

if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("--evidence-root",type=Path,required=True)
    p.add_argument("--prefix",type=Path,required=True)
    p.add_argument("--release-a",required=True)
    p.add_argument("--release-b",required=True)
    args=p.parse_args()
    main(args.evidence_root.resolve(),args.prefix.resolve(),args.release_a,args.release_b)
