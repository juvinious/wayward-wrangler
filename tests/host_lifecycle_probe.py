#!/usr/bin/env python3
"""Isolated transient-service lifecycle proof; no Waywr service or model calls.

Exercise only uniquely named test units and evidence below --evidence. Python is a
fixture interpreter, not an application runtime dependency.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import secrets
import signal
import subprocess
import sys
import threading
import time


def publish(path: Path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp." + secrets.token_hex(8))
    with tmp.open("x") as out:
        json.dump(value, out, sort_keys=True)
        out.flush()
        os.fsync(out.fileno())
    os.replace(tmp, path)
    directory = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def read(path: Path):
    return json.loads(path.read_text())


def proc_start(pid):
    try:
        return int(Path(f"/proc/{pid}/stat").read_text().split(") ", 1)[1].split()[19])
    except (OSError, IndexError, ValueError):
        return None


def runner(folder: Path, token: str):
    claim = {"pid": os.getpid(), "start": proc_start(os.getpid()),
             "invocation": os.getenv("INVOCATION_ID"), "token": token,
             "cgroup": Path("/proc/self/cgroup").read_text().strip()}
    publish(folder / "claim.json", claim)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if (folder / "retired.json").exists():
            publish(folder / "refused.json", {"reason": "retired", "token": token})
            return
        permit_file = folder / "permit.json"
        if permit_file.exists():
            permit = read(permit_file)
            owner = read(folder / "owner.json")
            if permit != {"token": token, "generation": owner["generation"]}:
                publish(folder / "refused.json", {"reason": "stale_permit", "token": token})
                return
            # This fake fence models the application gate. Stage 1 will replace it
            # with the real project/operation fence and durable budget reservation.
            with (folder / "gate.lock").open("a+b") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                if (folder / "retired.json").exists() or (folder / "committed.json").exists():
                    publish(folder / "refused.json", {"reason": "retired_or_committed", "token": token})
                    return
                owner = read(folder / "owner.json")
                if permit != {"token": token, "generation": owner["generation"]}:
                    publish(folder / "refused.json", {"reason": "stale_owner", "token": token})
                    return
                publish(folder / "committed.json", {"token": token, "generation": owner["generation"]})
            child = subprocess.Popen([sys.executable, __file__, "child", str(folder)],
                                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL)
            publish(folder / "child.json", {"pid": child.pid, "start": proc_start(child.pid)})
            code = child.wait()
            publish(folder / "receipt.json", {"token": token, "exit": code})
            return
        time.sleep(.025)
    publish(folder / "refused.json", {"reason": "permit_timeout", "token": token})


def child(folder: Path):
    with (folder / "counter.lock").open("a+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        path = folder / "counter.json"
        value = (read(path)["count"] if path.exists() else 0) + 1
        publish(path, {"count": value})
    while True:
        if (folder / "finish.json").exists():
            return
        publish(folder / "heartbeat.json", {"pid": os.getpid(), "start": proc_start(os.getpid()), "at": time.time()})
        time.sleep(.1)


def cmd(*argv, check=True):
    result = subprocess.run(argv, capture_output=True, text=True, timeout=15)
    if check and result.returncode:
        raise RuntimeError(f"{argv[0]} failed ({result.returncode}): {result.stderr.strip()}")
    return {"rc": result.returncode, "stdout": result.stdout.strip(), "stderr": result.stderr.strip()}


def unit_properties(name):
    result = cmd("systemctl", "--user", "show", name,
                 "--property=Id,LoadState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,Result,Type,ExitType,Restart,RemainAfterExit,CollectMode,KillMode,TimeoutStopUSec,TimeoutStartUSec,OOMPolicy,ExecStart",
                 check=False)
    values = {}
    for line in result["stdout"].splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value
    return values


def wait_for(predicate, seconds=10):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.05)
    raise RuntimeError("Timed out waiting for expected fixture evidence")


def started(name, folder, token, label):
    publish(folder / "owner.json", {"generation": 1})
    result = cmd("systemd-run", "--user", "--no-ask-password", f"--unit={name}",
                 "--service-type=exec", f"--working-directory={Path(__file__).resolve().parent.parent}",
                 "--property=ExitType=cgroup", "--property=RemainAfterExit=yes",
                 "--property=CollectMode=inactive", "--property=Restart=no",
                 "--property=KillMode=control-group", "--property=TimeoutStopSec=3s",
                 "--property=SendSIGKILL=yes", "--property=TimeoutStartSec=10s",
                 "--property=OOMPolicy=continue", "--property=StandardOutput=null",
                 "--property=StandardError=null", sys.executable, str(Path(__file__).resolve()),
                 "runner", str(folder), token)
    publish(folder / "start-client.json", result)
    claim = wait_for(lambda: read(folder / "claim.json") if (folder / "claim.json").exists() else None)
    properties = unit_properties(name)
    publish(folder / "host-observed.json", properties)
    assert properties["Type"] == "exec" and properties["ExitType"] == "cgroup", properties
    assert properties["Restart"] == "no" and properties["RemainAfterExit"] == "yes", properties
    assert properties["CollectMode"] == "inactive" and properties["KillMode"] == "control-group", properties
    assert properties["InvocationID"] == claim["invocation"] and claim["invocation"], (properties, claim)
    if properties["MainPID"] == "0" and properties["SubState"] == "exited":
        # The runner can refuse and exit before the status query. The retained
        # ExecStart PID and InvocationID still corroborate the immutable claim.
        assert f"pid={claim['pid']} ;" in properties["ExecStart"], (properties, claim)
        assert properties["Result"] == "success", properties
    else:
        assert properties["ControlGroup"] in claim["cgroup"], (properties, claim)
        assert properties["MainPID"] == str(claim["pid"]), (properties, claim)
    return claim


def exercise(base: Path):
    base = base.resolve()
    base.mkdir(parents=True, exist_ok=False)
    suffix = secrets.token_hex(7)
    names = {key: f"waywr-proof-{suffix}-{key}.service" for key in ("restart", "survive", "retire", "delayed", "linked", "complete", "race", "control")}
    publish(base / "ownership.json", {"units": names, "pid": os.getpid(), "project": str(base),
                                      "script": str(Path(__file__).resolve())})
    summary = {}
    try:
        # A sender can lose the success reply; reconcile the one named host.
        folder = base / "restart"; folder.mkdir()
        token = secrets.token_hex(16)
        claim = started(names["restart"], folder, token, "lost-reply")
        publish(folder / "controller-restarted.json", {"old_generation": 1, "new_generation": 2,
                                                        "discarded_client_reply": True})
        publish(folder / "owner.json", {"generation": 2})
        publish(folder / "permit.json", {"token": token, "generation": 2})
        wait_for(lambda: (folder / "counter.json").exists())
        assert read(folder / "counter.json")["count"] == 1
        assert read(folder / "committed.json")["generation"] == 2
        assert unit_properties(names["restart"])["MainPID"] == str(claim["pid"])
        summary["lost_reply_supervisor_restart"] = "one verified host and one child"

        folder = base / "survive"; folder.mkdir()
        token = secrets.token_hex(16)
        claim = started(names["survive"], folder, token, "survive")
        publish(folder / "permit.json", {"token": token, "generation": 1})
        child = wait_for(lambda: read(folder / "child.json") if (folder / "child.json").exists() else None)
        wait_for(lambda: (folder / "heartbeat.json").exists())
        os.kill(claim["pid"], signal.SIGKILL)
        wait_for(lambda: unit_properties(names["survive"])["MainPID"] == "0")
        assert proc_start(child["pid"]) == child["start"]
        assert unit_properties(names["survive"])["ActiveState"] == "active"
        summary["runner_death_child_survives"] = "verified same child alive in active unit; no receipt"
        publish(folder / "survival-observed.json", {"host": unit_properties(names["survive"]), "child": child,
                                                    "heartbeat": read(folder / "heartbeat.json")})

        folder = base / "retire"; folder.mkdir()
        token = secrets.token_hex(16)
        started(names["retire"], folder, token, "retire")
        publish(folder / "retired.json", {"token": token, "generation": 1})
        publish(folder / "permit.json", {"token": token, "generation": 1})
        wait_for(lambda: (folder / "refused.json").exists())
        assert not (folder / "counter.json").exists()
        summary["retired_stale_permit"] = "no child"
        wait_for(lambda: unit_properties(names["retire"])["SubState"] == "exited")
        summary["unit_retention"] = "exited runner remains active and inspectable"
        publish(folder / "terminal-host.json", unit_properties(names["retire"]))

        folder = base / "delayed"; folder.mkdir()
        token = secrets.token_hex(16)
        publish(folder / "retired.json", {"token": token, "generation": 1})
        started(names["delayed"], folder, token, "delayed")
        wait_for(lambda: (folder / "refused.json").exists())
        assert not (folder / "counter.json").exists()
        summary["delayed_start_after_retirement"] = "started runner refused without child"

        folder = base / "linked"; folder.mkdir()
        token = secrets.token_hex(16)
        started(names["linked"], folder, token, "linked")
        publish(folder / "permit.json", {"token": token, "generation": 1})
        wait_for(lambda: (folder / "counter.json").exists())
        assert read(folder / "counter.json")["count"] == 1
        summary["linked_replacement"] = "fresh host executed once after proven old refusal"

        folder = base / "complete"; folder.mkdir()
        token = secrets.token_hex(16)
        started(names["complete"], folder, token, "complete")
        publish(folder / "permit.json", {"token": token, "generation": 1})
        wait_for(lambda: (folder / "counter.json").exists())
        publish(folder / "finish.json", {"reason": "normal completion"})
        receipt = wait_for(lambda: read(folder / "receipt.json") if (folder / "receipt.json").exists() else None)
        assert receipt == {"token": token, "exit": 0}
        wait_for(lambda: unit_properties(names["complete"])["SubState"] == "exited")
        summary["completed_unit_retention"] = "receipt written before empty active/exited unit"

        folder = base / "race"; folder.mkdir()
        token = secrets.token_hex(16)
        started(names["race"], folder, token, "race")
        publish(folder / "permit.json", {"token": token, "generation": 1})
        wait_for(lambda: (folder / "counter.json").exists())
        result = {}
        go = threading.Event()
        def finish():
            go.wait()
            publish(folder / "finish.json", {"reason": "completion race"})
        def cancel():
            go.wait()
            result["stop"] = cmd("systemctl", "--user", "stop", names["race"], check=False)
        threads = [threading.Thread(target=finish), threading.Thread(target=cancel)]
        for worker in threads: worker.start()
        go.set()
        for worker in threads: worker.join(15)
        assert all(not worker.is_alive() for worker in threads), "race fixture stalled"
        race_receipt = read(folder / "receipt.json") if (folder / "receipt.json").exists() else None
        assert race_receipt is None or race_receipt == {"token": token, "exit": 0}
        assert read(folder / "counter.json")["count"] == 1
        publish(folder / "race-observed.json", {"receipt": race_receipt, "host": unit_properties(names["race"]), "stop": result})
        summary["cancel_completion_race"] = "one child; actual receipt or stopped without receipt, never manufactured"

        # Control host must not be touched by a targeted cancellation.
        folder = base / "control"; folder.mkdir()
        token = secrets.token_hex(16)
        started(names["control"], folder, token, "control")
        cmd("systemctl", "--user", "stop", names["survive"])
        wait_for(lambda: proc_start(child["pid"]) != child["start"])
        assert unit_properties(names["control"])["ActiveState"] == "active"
        summary["targeted_cleanup"] = "surviving child stopped; unrelated control unit remained active"
        publish(base / "summary.json", {"status": "pass", "cases": summary, "units": names})
    except BaseException as exc:
        publish(base / "summary.json", {"status": "failed", "cases": summary, "error": repr(exc), "units": names})
        raise
    finally:
        clean = {}
        for key, name in names.items():
            prior = unit_properties(name)
            if prior.get("LoadState") == "loaded":
                stopped = cmd("systemctl", "--user", "stop", name, check=False)
                reset = cmd("systemctl", "--user", "reset-failed", name, check=False)
                clean[key] = {"before": prior, "stop": stopped, "reset": reset, "after": unit_properties(name)}
        publish(base / "cleanup.json", clean)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("runner", "child", "exercise"))
    parser.add_argument("path", type=Path)
    parser.add_argument("token", nargs="?")
    args = parser.parse_args()
    if args.mode == "exercise":
        exercise(args.path)
    elif args.mode == "runner":
        runner(args.path, args.token)
    else:
        child(args.path)
