"""Opt-in isolated Rust runner comparison. No default Herdr socket or project is used.

Usage: python3 tests/rust_migration_stage3.py --evidence-root .local/rust-migration/stage3/live-N
The harness starts and stops only its own Herdr server and verifies its peer PID.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
PROBE = ROOT / "target/debug/operation-probe"
R9 = ROOT / ".local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9"
PYTHON = R9 / "venv/bin/python"
HERDR = Path.home() / ".local/bin/herdr"


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def load(path):
    return json.loads(path.read_text())


def eventual(fn, seconds=18):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = fn()
        if value:
            return value
        time.sleep(.05)
    raise AssertionError("Timed out awaiting owned test evidence")


def identity(pid):
    stat = Path(f"/proc/{pid}/stat").read_text()
    fields = stat[stat.rfind(")") + 2:].split()
    return {"pid": pid, "start": fields[19],
            "boot": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}


def live(expected):
    try:
        found = identity(expected["pid"])
        return all(found[k] == expected[k] for k in ("pid", "start", "boot"))
    except (FileNotFoundError, ProcessLookupError):
        return False


def signal_owned(expected, sig):
    if not live(expected):
        return
    fd = os.pidfd_open(expected["pid"])
    try:
        assert live(expected)
        signal.pidfd_send_signal(fd, sig)
    finally:
        os.close(fd)


class Harness:
    def __init__(self, root):
        assert not root.exists(), "Use a fresh evidence root"
        self.root = root
        root.mkdir(parents=True)
        self.private = Path(tempfile.mkdtemp(prefix="ww-rust-m3-"))
        for name in ("home", "config/herdr", "runtime"):
            (self.private / name).mkdir(parents=True)
        (self.private / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
        self.socket = str(self.private / "runtime/herdr.sock")
        self.env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "HOME": str(self.private / "home"),
                    "XDG_CONFIG_HOME": str(self.private / "config"),
                    "XDG_RUNTIME_DIR": str(self.private / "runtime"), "HERDR_SOCKET_PATH": self.socket,
                    "SHELL": "/bin/sh", "TERM": "xterm-256color", "LANG": "C.UTF-8"}
        self.server = None
        self.server_id = None
        self.workspace = None
        self.workers = []
        self.cases = []
        save(root / "expected-before-test.json", {"no_duplicate_dispatch": True,
             "claim_before_permit": True, "validated_receipt_replay_without_server": True,
             "dead_runner_live_child": "uncertain; never auto retry",
             "changed_server": "uncertain; never auto retry",
             "cancellation": "only verified owned child; receipt can win late request"})
        save(root / "isolation.json", {"private": str(self.private), "socket": self.socket,
             "r9_manifest_sha256": hashlib.sha256((R9 / "manifest.json").read_bytes()).hexdigest(),
             "test_harness": identity(os.getpid())})

    def call(self, method, params=None):
        import socket
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(8)
            client.connect(self.socket)
            client.sendall((json.dumps({"id":uuid.uuid4().hex,"method":method,"params":params or {}})+"\n").encode())
            value = json.loads(client.makefile("rb").readline())
        if "error" in value:
            raise RuntimeError(f"Herdr {method}: {value['error']}")
        return value["result"]

    def start(self):
        n = 1 if self.server_id is None else 2
        logfile = (self.root / f"server-{n}.log").open("wb")
        self.server = subprocess.Popen([str(HERDR), "server"], env=self.env,
                                       stdout=logfile, stderr=subprocess.STDOUT)
        logfile.close()
        self.server_id = identity(self.server.pid)
        eventual(lambda: self.server.poll() is None and self.try_connect())
        self.workspace = self.call("workspace.create", {"label":"Rust migration isolated test",
                              "cwd":str(self.root), "focus":False})["workspace"]["workspace_id"]
        save(self.root / f"server-{n}.json", self.server_id)

    def try_connect(self):
        try:
            self.call("workspace.list")
            return True
        except (OSError, RuntimeError):
            return False

    def stop(self):
        if self.server is not None and self.server.poll() is None:
            assert live(self.server_id), "Owned server identity changed; refusing stop"
            self.call("server.stop")
            self.server.wait(timeout=10)
        self.server = None

    def case(self, label, seconds=.15):
        state = self.root / label
        state.mkdir()
        save(state / "rust-migration-test-state.json", {"case":label})
        run = "rust-" + label
        directory = state / "probes" / run
        packet = {"schema_version":1,"operation_id":run,"run_id":run,"stage":"draft",
                  "role":"author","provider":"fake","model":"deterministic-m3",
                  "brief":"Isolated Rust migration comparison", "seconds":seconds}
        target = {"socket":self.socket,"workspace":self.workspace}
        save(state / "packet-input.json",packet)
        save(state / "target-input.json",target)
        return state, directory

    def probe(self, action, state, directory, extra=(), wait=True):
        command = [str(PROBE),action,str(state),str(directory),*map(str,extra)]
        if wait:
            return subprocess.run(command,capture_output=True,text=True,timeout=20)
        logfile = state / f"{action}-{len(self.workers)}.log"
        output = logfile.open("wb")
        child = subprocess.Popen(command,stdout=output,stderr=subprocess.STDOUT)
        output.close()
        self.workers.append((child,identity(child.pid),logfile))
        return child, logfile

    def prepare(self,state,directory):
        response=self.probe("prepare",state,directory,(state/"packet-input.json",state/"target-input.json"))
        assert response.returncode==0,response.stderr

    def pass_case(self,name,**detail):
        self.cases.append({"case":name,"passed":True,**detail})
        save(self.root/"progress.json",self.cases)
        print("PASS",name,flush=True)

    def normal(self):
        state,directory=self.case("normal")
        self.prepare(state,directory)
        initial=self.probe("ensure",state,directory)
        assert initial.returncode==0,initial.stderr
        eventual(lambda:(directory/"receipt.json").exists())
        receipt=load(directory/"receipt.json")
        assert receipt["status"]=="completed"
        assert load(directory/"invocations.json")["count"]==1
        before=(directory/"receipt.json").read_bytes()
        replay=self.probe("ensure",state,directory)
        assert replay.returncode==0,replay.stderr
        assert (directory/"receipt.json").read_bytes()==before
        late=self.probe("cancel",state,directory)
        assert late.returncode==0 and json.loads(late.stdout)["status"]=="completed"
        assert (directory/"receipt.json").read_bytes()==before
        pane=self.call("pane.read",{"pane_id":load(directory/"binding.json")["pane_id"],
                     "source":"recent","lines":80})
        save(state/"retained-pane.json",pane)
        self.pass_case("normal-and-replay",status=receipt["status"],count=1,
                       pane=load(directory/"binding.json"))
        return state,directory

    def cancel(self):
        state,directory=self.case("cancel",seconds=8)
        self.prepare(state,directory)
        controller,log=self.probe("ensure",state,directory,wait=False)
        eventual(lambda:(directory/"child.json").exists())
        assert controller.wait(timeout=8)==0,log.read_text()
        before=load(directory/"child.json")["identity"]
        other_runner=load(self.root/"normal/probes/rust-normal/claim.json")["runner"]
        assert live(other_runner)
        response=self.probe("cancel",state,directory)
        assert response.returncode==0,response.stderr
        eventual(lambda:(directory/"receipt.json").exists())
        receipt=load(directory/"receipt.json")
        assert receipt["status"]=="cancelled" and load(directory/"invocations.json")["count"]==1
        assert not live(before) and live(other_runner)
        self.pass_case("targeted-cancellation",status=receipt["status"],count=1,
                       unrelated_test_runner_alive=True)

    def baseline(self):
        state,directory=self.case("r9-baseline")
        script="""import json,sys
from pathlib import Path
from agent_workflow import storage
storage.LOCAL=Path(sys.argv[1]);storage.ROOT=storage.LOCAL
from agent_workflow.herdr_backend import prepare,wait
directory=Path(sys.argv[2]);state=Path(sys.argv[1])
prepare(directory,json.loads((state/'packet-input.json').read_text()),json.loads((state/'target-input.json').read_text()))
print(json.dumps(wait(directory)))
"""
        result=subprocess.run([str(PYTHON),"-I","-c",script,str(state),str(directory)],
                              env={**self.env,"PYTHONUNBUFFERED":"1"},capture_output=True,text=True,timeout=20)
        (state/"controller.log").write_text(result.stdout+result.stderr)
        assert result.returncode==0,result.stderr
        receipt=load(directory/"receipt.json")
        assert receipt["status"]=="completed" and load(directory/"invocations.json")["count"]==1
        rust=load(self.root/"normal/probes/rust-normal/receipt.json")
        assert (receipt["status"],load(directory/"invocations.json")["count"])==(rust["status"],1)
        old_result=load(directory/"result.json")
        new_result=load(self.root/"normal/probes/rust-normal/result.json")
        for value in (old_result,new_result):
            value.pop("operation_id");value.pop("packet_sha256")
        assert old_result==new_result,"Meaningful fake-result difference from installed r9"
        self.pass_case("installed-r9-baseline",status=receipt["status"],count=1,
                       rust_result=rust["result_sha256"],r9_result=receipt["result_sha256"],
                       normalized_result_equal=True)

    def missing_workspace(self):
        state,directory=self.case("missing-workspace")
        target=load(state/"target-input.json");target["workspace"]="w-does-not-exist"
        save(state/"target-input.json",target)
        self.prepare(state,directory)
        result=self.probe("ensure",state,directory)
        assert result.returncode==3 and "workspace unavailable before launch" in result.stderr.lower(),result.stderr
        assert not (directory/"launch.json").exists() and not (directory/"claim.json").exists()
        self.pass_case("missing-workspace-before-launch",status="operational",count=0)

    def orphan(self):
        state,directory=self.case("orphan",seconds=20)
        self.prepare(state,directory)
        controller,log=self.probe("ensure",state,directory,wait=False)
        eventual(lambda:(directory/"child.json").exists())
        assert controller.wait(timeout=8)==0,log.read_text()
        runner=load(directory/"claim.json")["runner"]
        child=load(directory/"child.json")["identity"]
        assert live(runner) and live(child)
        signal_owned(runner,signal.SIGKILL)
        eventual(lambda:not live(runner))
        assert live(child),"Fault requires child to survive runner"
        retry=self.probe("ensure",state,directory)
        assert retry.returncode==3 and "runner disappeared" in retry.stderr.lower(),retry.stderr
        assert load(directory/"invocations.json")["count"]==1
        status=self.probe("status",state,directory)
        assert json.loads(status.stdout)["status"]=="uncertain"
        save(state/"diagnostic.json",json.loads(status.stdout))
        signal_owned(child,signal.SIGTERM)
        eventual(lambda:not live(child))
        self.pass_case("runner-death-surviving-child",status="uncertain",count=1,
                       cleanup="pidfd verified test child only")

    def server_missing_and_changed(self):
        state,directory=self.case("server-change",seconds=20)
        self.prepare(state,directory)
        controller,log=self.probe("ensure",state,directory,wait=False)
        eventual(lambda:(directory/"child.json").exists())
        assert controller.wait(timeout=8)==0,log.read_text()
        previous=load(directory/"launch.json")["server"]
        self.stop()
        missing=self.probe("ensure",state,directory)
        assert missing.returncode==3 and "server unavailable" in missing.stderr.lower(),missing.stderr
        assert load(directory/"invocations.json")["count"]==1
        self.start()
        changed=self.probe("ensure",state,directory)
        assert changed.returncode==3 and "server changed" in changed.stderr.lower(),changed.stderr
        assert load(directory/"invocations.json")["count"]==1
        self.pass_case("missing-and-changed-server",status="uncertain",count=1,
                       original=previous,replacement=self.server_id)
        done_state=self.root/"normal";done_dir=done_state/"probes/rust-normal"
        self.stop()
        replay=self.probe("ensure",done_state,done_dir)
        assert replay.returncode==0,replay.stderr
        assert json.loads(replay.stdout)["status"]=="completed"
        self.pass_case("completed-receipt-without-server",status="completed",count=1)

    def cleanup(self):
        for path in self.root.glob("*/probes/*/child.json"):
            expected=load(path)["identity"]
            if live(expected):signal_owned(expected,signal.SIGTERM)
        for path in self.root.glob("*/probes/*/claim.json"):
            expected=load(path)["runner"]
            if live(expected):signal_owned(expected,signal.SIGTERM)
        for child,expected,_ in self.workers:
            if child.poll() is None:signal_owned(expected,signal.SIGTERM)
        self.stop()
        save(self.root/"cleanup.json",{"owned_server_stopped":True,
             "live_owned_runners":[str(path) for path in self.root.glob("*/probes/*/claim.json")
               if live(load(path)["runner"])],
             "live_owned_children":[str(path) for path in self.root.glob("*/probes/*/child.json")
               if live(load(path)["identity"])]})


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--evidence-root",type=Path,required=True)
    args=parser.parse_args()
    assert PROBE.is_file() and (PROBE.parent/"waywr-runner").is_file()
    suite=Harness(args.evidence_root.resolve())
    try:
        suite.start()
        suite.normal()
        suite.cancel()
        suite.baseline()
        suite.missing_workspace()
        suite.orphan()
        suite.server_missing_and_changed()
        save(suite.root/"summary.json",{"passed":suite.cases,"r9_manifest":
             hashlib.sha256((R9/"manifest.json").read_bytes()).hexdigest()})
    finally:
        suite.cleanup()


if __name__=="__main__":main()
