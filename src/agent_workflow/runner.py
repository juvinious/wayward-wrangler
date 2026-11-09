"""Herdr-owned foreground supervisor. Never detached; never advances the graph."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
from .storage import ROOT, OperationalError, alive, atomic, canonical, digest, identity, lock, now, operation_path, read
from .herdr_backend import Herdr, validate_result
from . import terminal as ui
from .runtime import bind, load, current


def reserve_native_attempt(directory, op):
    """Count an attempt before process launch; refuse both sixth and duplicate calls."""
    directory = Path(directory)
    with lock(directory.parent.parent / "invocation-budget.lock"):
        counters = list(directory.parent.glob("*/invocations.json"))
        limit = op.get("max_invocations", 5)
        if type(limit) is not int or not 1 <= limit <= 5 or (directory / "invocations.json").exists() or sum(
                read(path)["count"] for path in counters) >= limit:
            raise OperationalError("Duplicate dispatch or frozen native invocation limit")
        atomic(directory / "invocations.json", {"operation_id": op["operation_id"], "count": 1,
               "meaning": "reserved native process launch attempt"}, immutable=True)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--operation-dir", required=True)
    parser.add_argument("--launch-token", required=True)
    parser.add_argument("--state-dir", help="Explicit controller-owned state root; defaults to project .local")
    parser.add_argument("--project")
    parser.add_argument("--registry-dir")
    args = parser.parse_args()
    if args.project:
        selected = load(args.project, args.state_dir, args.registry_dir)
        bind(selected)
    directory = operation_path(args.operation_dir, local=args.state_dir)
    op = read(directory / "operation.json")
    if current() and op.get("project_key") not in (None, current().key):
        raise OperationalError("Runner project identity mismatch")
    if op.get("protocol_version", 1) != 1:
        raise OperationalError("Runner protocol mismatch; no execution")
    if op.get("authorization_plan_sha256"):
        from .native import application_version
        if op.get("application_version") != application_version():
            raise OperationalError("Installed runner version differs from frozen native operation; no execution")
    if args.launch_token != op["launch_token"]:
        raise OperationalError("Launch token mismatch")
    with lock(directory / "runner.lock"):
        if (directory / "claim.json").exists():
            print("Duplicate runner refused: operation already claimed.", flush=True)
            return 1
        me = identity()
        herdr = {key: os.environ.get(env) for key, env in (
            ("pane_id", "HERDR_PANE_ID"), ("tab_id", "HERDR_TAB_ID"), ("workspace_id", "HERDR_WORKSPACE_ID"))}
        if not all(herdr.values()):
            raise OperationalError("Runner must execute inside an actual Herdr pane")
        atomic(directory / "claim.json", {"runner": me, "herdr": herdr, "time": now()}, immutable=True)
        launched_target = read(directory / "launch.json").get("target", op["target"])
        h = Herdr(launched_target["socket"])
        key = (current().key[:8] + " ") if current() else ""
        label = f"Wayward Wrangler {key}{op['run_id']} {op['role']} {op['provider']}/{op['model']} {op['stage']}"
        def report(state, text):
            try:
                h.call("pane.report_agent", {"pane_id": herdr["pane_id"], "source": "awp-runner",
                    "agent": "awp-" + op["provider"], "state": state, "message": text})
                h.call("pane.report_metadata", {"pane_id": herdr["pane_id"], "source": "awp-runner",
                    "title": label + " " + text, "display_agent": f"{op['role']} {op['provider']}/{op['model']}"})
            except Exception as error:
                print(f"[Herdr presentation unavailable: {error}]", flush=True)
        print(ui.paint(label, op["role"]), flush=True)
        print(f"Operation: {op['operation_id']}\nPacket: {op['packet_sha256']}\nPane: {herdr['pane_id']}\nWaiting for persisted dispatch permit.", flush=True)
        report("blocked", "awaiting-dispatch")
        cancelled = False
        def on_interrupt(_signum, _frame):
            nonlocal cancelled
            cancelled = True
        signal.signal(signal.SIGINT, on_interrupt)
        while not (directory / "permit.json").exists():
            if cancelled or (directory / "cancel.json").exists():
                break
            time.sleep(.1)
        child = None
        result = None
        status = "cancelled"
        exit_code = None
        error_text = None
        native_events = []
        native_buffer = b""
        native = op["provider"] != "fake"
        try:
            if not cancelled and not (directory / "cancel.json").exists():
                permit = read(directory / "permit.json")
                if permit["runner"] != me or permit["launch_token"] != op["launch_token"]:
                    raise OperationalError("Permit does not authorize this runner")
                packet = read(directory / "packet.json")
                if digest(canonical(packet)) != op["packet_sha256"]:
                    raise OperationalError("Packet was modified")
                input_file = None
                if native:
                    from .adapters import real
                    invocation = real.build(directory, packet)
                    atomic(directory / "execution.json", invocation["record"], immutable=True)
                    argv, env = invocation["argv"], invocation["env"]
                    input_file = invocation["stdin"].open("rb")
                    # Reserve before Popen. A lost spawn is never a blind retry.
                    reserve_native_attempt(directory, op)
                else:
                    env = {"PATH": "/usr/bin:/bin", "PYTHONUNBUFFERED": "1", "LANG": "C.UTF-8",
                           "HOME": str(directory)}
                    # The fake child uses this installation, never the source checkout.
                    module = [sys.executable, "-m", "agent_workflow.adapters.fake"]
                    argv = module + [
                            "--packet", str(directory / "packet.json"), "--counter", str(directory / "invocations.json")]
                atomic(directory / "dispatch.json", {"runner": me, "time": now()}, immutable=True)
                try:
                    child = subprocess.Popen(argv, stdin=input_file if native else subprocess.DEVNULL,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, cwd=directory, process_group=0)
                finally:
                    if input_file: input_file.close()
                child_identity = identity(child.pid)
                atomic(directory / "child.json", {"identity": child_identity, "pgid": child.pid}, immutable=True)
                report("working", "running")
                print(ui.paint(f"RUNNING child PID {child.pid}; Ctrl-C cancels this worker only."), flush=True)
                selector = selectors.DefaultSelector()
                buffers = {}
                for name, stream in (("stdout", child.stdout), ("stderr", child.stderr)):
                    selector.register(stream, selectors.EVENT_READ, name)
                    buffers[name] = b""
                terminate_at = None
                deadline = time.monotonic() + 600 if native else None
                with (directory / "transcript.log").open("ab", buffering=0) as transcript:
                    while selector.get_map():
                        if deadline is not None and time.monotonic() > deadline:
                            raise OperationalError("Native worker exceeded ten-minute limit; no retry")
                        if cancelled and not (directory / "cancel.json").exists():
                            atomic(directory / "cancel.json", {"source": "worker-pane-SIGINT", "runner": me, "time": now()})
                        requested = cancelled or (directory / "cancel.json").exists()
                        if requested and terminate_at is None and child.poll() is None:
                            if not alive(child_identity) or os.getpgid(child.pid) != child.pid:
                                raise OperationalError("Child identity changed during cancellation")
                            os.killpg(child.pid, signal.SIGTERM)
                            terminate_at = time.monotonic()
                            cancelled = True
                        if terminate_at is not None and child.poll() is None and time.monotonic() - terminate_at > 3:
                            if alive(child_identity) and os.getpgid(child.pid) == child.pid:
                                os.killpg(child.pid, signal.SIGKILL)
                        for key, _ in selector.select(.1):
                            chunk = os.read(key.fileobj.fileno(), 65536)
                            if not chunk:
                                selector.unregister(key.fileobj)
                                key.fileobj.close()
                                continue
                            name = key.data
                            transcript.write(chunk)
                            # Live bytes from the actual child, not a tailed log.
                            sys.stdout.write(ui.paint(chunk.decode("utf-8", errors="replace"), "error" if name == "stderr" else op["role"]))
                            sys.stdout.flush()
                            buffers[name] += chunk
                            if native and name == "stdout":
                                with (directory/"native-events.jsonl").open("ab") as native_log:
                                    native_log.write(chunk)
                                native_buffer += chunk
                                while b"\n" in native_buffer:
                                    line, native_buffer = native_buffer.split(b"\n",1)
                                    if line.strip():
                                        event = json.loads(line)
                                        native_events.append(event)
                                        real.inspect_event(op["provider"],event)
                    os.fsync(transcript.fileno())
                exit_code = child.wait()
                if cancelled or (directory / "cancel.json").exists():
                    status = "cancelled"
                elif exit_code != 0:
                    status = "failed"
                    error_text = f"Worker exited {exit_code}"
                else:
                    if native:
                        if native_buffer.strip(): native_events.append(json.loads(native_buffer))
                        result = real.finish(op["provider"],native_events,directory)
                    else:
                        events = [json.loads(line) for line in buffers["stdout"].splitlines() if line.strip()]
                        results = [event["result"] for event in events if event.get("type") == "result"]
                        if len(results) != 1:
                            raise OperationalError("Expected one structured worker result")
                        result = results[0]
                    try:
                        validate_result(result, packet)
                    except OperationalError as error:
                        raise OperationalError(f"Output invalid: {error}") from error
                    atomic(directory / "result.json", result, immutable=True)
                    status = "completed"
        except Exception as error:
            if child is not None and child.poll() is None:
                # Do not certify completion while a child still runs.
                if native and (directory / "child.json").exists():
                    recorded = read(directory / "child.json")["identity"]
                    if alive(recorded) and os.getpgid(child.pid) == child.pid:
                        os.killpg(child.pid, signal.SIGTERM)
                print(f"UNCERTAIN: runner error; stop requested for identified worker: {error}", flush=True)
                atomic(directory / "uncertain.json", {"error": str(error), "time": now()})
                report("blocked", "uncertain")
                while True:
                    time.sleep(2)
            status = "failed"
            error_text = str(error)
        outcome = {key: op[key] for key in ("operation_id", "packet_sha256", "launch_token")}
        outcome.update({"schema_version": 1, "runner": me, "status": status, "exit_code": exit_code,
                        "error": error_text, "completed": now(),
                        "result_sha256": digest(canonical(result)) if result is not None and status == "completed" else None})
        atomic(directory / "receipt.json", outcome, immutable=True)
        report("idle", status)
        try:
            h.call("pane.rename", {"pane_id": herdr["pane_id"], "label": label + " " + status.upper()})
        except Exception as error:
            print(f"Pane label update failed; durable outcome is {status}: {error}", flush=True)
        print("\n" + ui.status(status), flush=True)
        print(f"Retained output, no active agent.\nReceipt: {directory / 'receipt.json'}\nTranscript: {directory / 'transcript.log'}\nClose this completed tab when finished inspecting. Terminal text is never a workflow decision.", flush=True)
        # Retain the actual execution pane; no further dispatch is possible.
        while True:
            time.sleep(2)

if __name__ == "__main__":
    raise SystemExit(main())
