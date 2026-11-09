from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import sys
import signal
import time
import tomllib
from importlib.resources import files
from .storage import ROOT, LOCAL, OperationalError, Uncertain, atomic, canonical, digest, lock, now, read
from .herdr_backend import DEFAULT_SOCKET, Herdr, prepare, wait, cancel, observe, receipt
from . import records, terminal as ui
from .runtime import current, load, bind, register, selected_path

def state_dir():
    selected = current()
    return selected.state if selected else LOCAL

def project_dir():
    selected = current()
    return selected.project if selected else ROOT

def no_other_execution(probe=None):
    active = records.active_run()
    if active:
        raise OperationalError(f"Run {active} is active; resume or resolve it before starting another")
    for directory in (state_dir() / "probes").glob("*"):
        if directory != probe and (directory / "operation.json").exists() and not (directory / "receipt.json").exists():
            raise OperationalError(f"Foundation operation remains unresolved: {directory}")

def target(args):
    socket = getattr(args, "socket", None) or os.environ.get("HERDR_SOCKET_PATH", DEFAULT_SOCKET)
    workspace = getattr(args, "workspace", None) or os.environ.get("HERDR_WORKSPACE_ID")
    if not workspace:
        raise OperationalError("Run inside Herdr or provide --workspace explicitly")
    Herdr(socket).call("workspace.get", {"workspace_id": workspace})
    return {"socket": socket, "workspace": workspace}

def create(args):
    selected = current()
    if selected:
        with lock(selected.state / "admission.lock"):
            return _create(args)
    return _create(args)

def _create(args, *, preview=False):
    if current() and current().legacy:
        raise OperationalError("Legacy state is for explicit existing-run access; initialize a new project for new runs")
    if not preview:
        no_other_execution()
    run_id = records.name(args.run or time.strftime("demo-%Y%m%d-%H%M%S"))
    profile = selected_path(args.profile, project_dir())
    if current() and not profile.is_relative_to(project_dir()):
        raise OperationalError("Configuration file escapes the selected project")
    config = tomllib.loads(profile.read_text())
    if config.get("review_limit") != 2 or set(config.get("roles", {})) != {"author", "critic"}:
        raise OperationalError("Prototype requires exactly two roles and review_limit=2")
    if config.get("workflow_version", 1) == 2:
        question_limit = config.get("max_questions", 1)
        if type(question_limit) is not int or not 1 <= question_limit <= 4:
            raise OperationalError("Version-2 question limit must be an integer from 1 to 4")
    real_trial = any(r.get("provider") != "fake" for r in config["roles"].values())
    if real_trial:
        config.setdefault("finding_contract_version", 2)
        if config["finding_contract_version"] != 2:
            raise OperationalError("New native runs require finding contract version 2")
        if not current() or config.get("execution_mode") != "native":
            raise OperationalError("Native execution requires an initialized project and execution_mode=native")
        if run_id in ("m2-cli-01", "context-01") or config.get("trial_id"):
            raise OperationalError("Historical native trial identity cannot authorize a new project run")
        if config.get("workflow_version") != 2 or config.get("decision") or config.get("prompt_contract") != "product-spec-v1":
            raise OperationalError("Native project runs require version-2 agent questions and product-spec-v1")
        limit = config.get("max_invocations")
        if type(limit) is not int or not 1 <= limit <= 5:
            raise OperationalError("Native invocation limit must be an explicit integer from 1 to 5")
        from .adapters.real import verify_role
        if [config["roles"][r]["provider"] for r in ("author","critic")] != ["claude","codex"]:
            raise OperationalError("This trial requires Claude author and Codex critic")
        for role in config["roles"].values(): verify_role(role)
        from .native import check_auth
        check_auth(config)
    else:
        if config.get("execution_mode", "fake") != "fake":
            raise OperationalError("Fake roles require execution_mode=fake")
        for role in config["roles"].values():
            if not isinstance(role.get("model"), str): raise OperationalError("Missing fake model")
        seconds = config.get("fake", {}).get("seconds")
        if not isinstance(seconds, (int, float)) or not 0.2 <= seconds <= 120:
            raise OperationalError("Fake duration must be 0.2–120 seconds")
        count = config.get("fake", {}).get("questions", 1)
        if type(count) is not int or not 0 <= count <= 2:
            raise OperationalError("Fake question count must be 0–2")
    instructions = []
    for entry in config.get("rules", []):
        path = (profile.parent / entry).resolve()
        if not path.is_relative_to(project_dir()):
            raise OperationalError("Rule file escapes the selected project")
        data = path.read_text()
        source = str(path.relative_to(project_dir())) if current() and not current().legacy else str(path)
        instructions.append({"source": source, "sha256": digest(data), "text": data})
    inline_brief = getattr(args, "brief_text", None)
    if inline_brief is None:
        brief_path = selected_path(args.brief, project_dir())
        if not brief_path.is_relative_to(project_dir()):
            raise OperationalError("Brief escapes the selected project")
        brief = brief_path.read_text()
    else:
        if real_trial or not isinstance(inline_brief, str) or not inline_brief.strip():
            raise OperationalError("Session inline briefs are limited to nonempty simulated work")
        brief_path = None
        brief = inline_brief
    envelope = {"schema_version": 1, "workflow_version": 1, "run_id": run_id, "thread_id": run_id,
                "created": now(), "brief": brief, "brief_sha256": digest(brief),
                "instructions": instructions, "rules_sha256": digest(canonical(instructions)),
                "config": config, "config_sha256": digest(canonical(config)),
                "skills": {k: files("agent_workflow.resources.skills").joinpath(k + ".md").read_text()
                           for k in ("draft", "critique", "revise")},
                "target": target(args)}
    if current() and not current().legacy:
        from .native import application_version
        envelope.update(location_format=2, project_key=current().key,
                        project=str(current().project), application_version=application_version(), protocol_version=1,
                        brief_path=str(brief_path.relative_to(project_dir())) if brief_path else None,
                        config_path=str(profile.relative_to(project_dir())))
    imports=getattr(args,"cross_imports",[])
    if imports:
        if not current() or current().legacy:
            raise OperationalError("Cross-project imports need an initialized project")
        from .context_access import check_frozen
        check_frozen(current(),imports)
        envelope.update(cross_import_contract_version=1,cross_imports=imports)
    if config.get("workflow_version",1)==2:
        from .context import resolve
        if config.get("decision") or not getattr(args,"context",None):
            raise OperationalError("Version 2 requires explicit context and forbids injected trial questions")
        snapshot=resolve(selected_path(args.context, project_dir()))
        envelope.update(workflow_version=2,context_snapshot=snapshot,context_digest=digest(canonical(snapshot)))
    if real_trial:
        from .native import plan, authorize
        selected_plan = plan(envelope)
        if preview:
            return selected_plan
        envelope["native_authorization"] = authorize(args, envelope)
    elif preview:
        raise OperationalError("Preflight is for explicit native profiles only")
    # Reject ID reuse before writing frozen input files.
    if records.run_dir(run_id).exists():
        raise OperationalError("Run directory already exists; use resume or choose a new run ID")
    atomic(records.run_dir(run_id) / "inputs/envelope.json", envelope, immutable=True)
    records.create_run(envelope)
    print(f"Run: {run_id}\nFrozen inputs: {records.run_dir(run_id) / 'inputs'}", flush=True)
    return run_id

def summary(run_id):
    from .workflow import state
    info = records.get_run(run_id)
    if current() and info["envelope"].get("project_key") not in (None, current().key):
        raise OperationalError("Run belongs to another project")
    snapshot = state(run_id)
    result = {"run_id": run_id, "status": info["status"], "detail": info["detail"],
              "completed": snapshot.values.get("completed", []), "pending": snapshot.values.get("pending"),
              "current": snapshot.values.get("current"), "execution_target": records.effective_target(run_id), "workers": [],
              "revision_coverage": snapshot.values.get("coverage_gaps", {}),
              "findings": snapshot.values.get("findings", {}),
              "finding_assessments": snapshot.values.get("finding_assessments", [])}
    notices = sorted((state_dir() / "supervision" / "notices").glob("*.json"))
    if notices and info["status"] in ("failed", "uncertain", "output-invalid"):
        latest = read(notices[-1])
        if latest.get("run_id") == run_id:
            result["supervisor_notice"] = latest
    if current():
        result.update(project_key=current().key, project=str(current().project),
                      state_dir=str(current().state), legacy=current().legacy)
    if info["envelope"].get("workflow_version")==2:
        result["selected_context"]=info["envelope"]["context_snapshot"]
        result["context_digest"]=info["envelope"]["context_digest"]
        result["saved_pending_answer"]=records.response(run_id,result["pending"]["request_id"]) if result["pending"] else None
    live_panes = {}
    for op in records.operations(run_id):
        directory = records.run_dir(run_id) / "operations" / op["stage"]
        original = read(directory / "operation.json") if (directory / "operation.json").exists() else {}
        binding = read(directory / "binding.json") if (directory / "binding.json").exists() else {}
        from .herdr_backend import inspect_operation
        diagnostic = inspect_operation(directory) if original else {
            "status": "prepared", "detail": "Stage reserved; operation not yet prepared", "operator_action": None}
        status = diagnostic["status"]
        socket_path = original.get("target", {}).get("socket")
        if socket_path and socket_path not in live_panes:
            try:
                live_panes[socket_path] = {p["pane_id"]: p for p in Herdr(socket_path).call("pane.list")["panes"]}
            except (OSError, OperationalError, ValueError):
                live_panes[socket_path] = None
        observed = live_panes.get(socket_path)
        current_pane = observed.get(binding.get("pane_id")) if observed is not None else None
        presence = "unverified" if observed is None else (
            "present" if current_pane and current_pane["terminal_id"] == binding.get("terminal_id") else "closed")
        result["workers"].append({"operation": op["stage"], "role": original.get("role"),
            "provider": original.get("provider"), "model": original.get("model"), "status": status,
            "execution_detail": diagnostic["detail"], "operator_action": diagnostic["operator_action"],
            "runner_alive": diagnostic.get("runner_alive"), "child_alive": diagnostic.get("child_alive"),
            **binding, "pane_state": presence, "transcript": str(directory / "transcript.log")})
    return result

def print_summary(run_id, as_json=False):
    info = summary(run_id)
    if as_json:
        print(json.dumps(info, indent=2))
        return info
    print("\n" + ui.paint(run_id) + ": " + ui.status(info["status"]))
    print(ui.paint("Execution workspace:", "muted"), info["execution_target"]["workspace"])
    if info["detail"]:
        print(ui.paint(info["detail"], "warning"))
    for stage, gaps in info.get("revision_coverage", {}).items():
        unresolved = [f["id"] for f in gaps if info["findings"].get(f["id"], {}).get("status") == "open"]
        if unresolved:
            print(ui.paint(f"{stage}: provisional coverage; findings still open: " + ", ".join(unresolved), "warning"))
    if info.get("supervisor_notice"):
        notice = info["supervisor_notice"]
        print(ui.paint("Supervisor notice:", "warning"), notice["detail"])
        print("Action:", notice["operator_action"])
    for worker in info["workers"]:
        print("  " + ui.paint(worker["operation"], worker["role"] or "heading") + ": " +
              f"{worker['role']} {worker['provider']}/{worker['model']} " + ui.status(worker["status"]))
        print(ui.paint(f"    pane={worker.get('pane_id', 'unbound')} ({worker['pane_state']}) "
                       f"tab={worker.get('tab_id', 'unbound')}", "muted"))
        if worker.get("operator_action"):
            print(ui.paint("    " + worker["execution_detail"], "warning"))
            print("    Action: " + worker["operator_action"])
    if info["current"]:
        from .review import url
        ref = info["current"]
        print("\n" + ui.paint("Review artifact:") + " " + str(records.run_dir(run_id) / ref["path"]))
        print(ui.paint("Version:", "muted"), ref["id"])
        print(ui.link("Open in Vim [Ctrl-click; Herdr plugin]", url(run_id, ref)))
        print("Or run: " + ui.paint(f"wayward-wrangler --project {project_dir()} open {run_id} --artifact {ref['id']}", "action"))
    if info.get("selected_context"):
        print(ui.paint("Inherited context:"),info["selected_context"]["project_id"],"—",info["selected_context"]["scope"])
        for e in info["selected_context"]["entries"]:
            value=e["content"]["response"]["answer"] if e["kind"]=="decision" else e["kind"]
            print("  "+e["qualified_id"]+" — "+value)
        print("Explicit selection; resolution and delivery are automatic.")
    request = info["pending"]
    if request:
        print("\n" + ui.paint(f"YOUR INPUT — {request['request_id']} ({request['kind']})", "warning"))
        if request["kind"] == "choice":
            q = request["question"]
            print(q["question"])
            print("Options: " + " | ".join(ui.paint(f"{i}. {option}", "action") for i,option in enumerate(q["options"],1)))
            if q.get("rationale"):
                print("Why your decision is needed:",q["rationale"])
                print("Evidence:",", ".join(q["evidence_refs"]))
                if q["related_decision_refs"]:
                    print("Explicit reconsideration of:",", ".join(q["related_decision_refs"]),"—",q["reconsideration_reason"])
            if info.get("saved_pending_answer"):
                print("Answer already saved; restart/resume to consume it.")
            print(f"Recommendation: {q['recommendation']}\nConsequences: {q['consequences']}")
        else:
            print(ui.paint(request["notice"], "warning"))
            for decision in request.get("decisions", []):
                print(ui.paint("Accepted decision:", "success"), decision["request_id"], "—", decision["answer"])
            for finding_id, finding in request.get("findings", {}).items():
                print("Finding:", finding_id, ui.status(finding["status"]), "—", finding["requested_change"])
            for assessment in request.get("finding_assessments", []):
                print("Assessment:", assessment["finding_ref"], assessment["assessment"],
                      "from", assessment["stage"], "—", assessment["rationale"])
            print("Approve only after reviewing this exact artifact and the recorded decisions.")
            print("When ready, say " + ui.paint("approve this revision", "action") + ". I will record the exact version shown above.")
    return info

def run_controller(run_id):
    from .workflow import drive
    drive(run_id)
    info = print_summary(run_id)
    return 2 if info["status"] == "awaiting-input" else (3 if info["status"] in ("uncertain","blocked","failed","cancelled") else 0)

def controller_demo(args):
    """Actual interactive controller in a Herdr pane, not an observer/tail pane."""
    if not os.environ.get("HERDR_PANE_ID"):
        raise OperationalError("Interactive controller must run inside Herdr")
    from . import ownership
    if getattr(args, "owner_generation", None) is not None:
        owner = ownership.current()
        if not owner or owner["generation"] != args.owner_generation or owner["token"] != args.owner_token or owner["run_id"] != args.run:
            raise OperationalError("Controller ownership changed before startup")
        from .storage import identity
        atomic(state_dir() / "supervision" / "controllers" / f"{args.owner_generation:06d}" / "claim.json",
               {"process": identity(), "pane_id": os.environ["HERDR_PANE_ID"],
                "workspace_id": os.environ.get("HERDR_WORKSPACE_ID"), "token": args.owner_token}, immutable=True)
        ownership.bind(args.owner_generation)
    with lock(state_dir() / "controller.lock"):
        run_id = args.run
        try:
            records.get_run(run_id)
        except OperationalError:
            run_id = create(args)
        if args.workspace and args.workspace != records.effective_target(run_id)["workspace"]:
            raise OperationalError("Existing run targets another workspace; use rebind first")
        try:
            run_controller(run_id)
        except OperationalError as error:
            # A task-level stop is inspectable; the supervisor and Herdr
            # controller remain available without retrying the worker.
            print(ui.paint(f"Task stopped: {error}", "error"), flush=True)
            print_summary(run_id)
    if getattr(args,"exit_at_choice",False):
        pending=summary(run_id).get("pending")
        return 2 if pending and pending["kind"]=="choice" else 3
    while True:
        info = print_summary(run_id)
        if info["status"] in records.TERMINAL and info["status"] != "output-invalid":
            print("\n" + ui.status(info["status"]) + " — controller output retained. No active worker.")
            print("Close this tab when finished inspecting.", flush=True)
            while True:
                time.sleep(2)
        request = info.get("pending")
        if not request:
            try:
                text = ui.read_command("\nTask stopped. 'status', 'open [stage]', or 'exit': ")
            except (EOFError, OperationalError):
                return 2
            if text == "exit":
                return 2
            if text == "status":
                continue
            if review_input(run_id, text):
                continue
            print("No task action taken; inspect status and recorded evidence.", flush=True)
            continue
        prompt = ("\nChoose an option number or label, 'open [stage]', or 'exit': " if request["kind"] == "choice" else
                  "\nReview with 'open', say 'approve this revision', 'reject REASON', or 'exit': ")
        try:
            text = ui.read_command(prompt)
        except OperationalError as error:
            print(ui.paint(str(error), "warning"))
            print_summary(run_id)
            continue
        except EOFError:
            return 2
        if review_input(run_id, text):
            continue
        if text == "exit":
            return 2
        if request["kind"] == "choice":
            outcome, sha = "answered", None
            if text.isdigit() and 1<=int(text)<=len(request["question"]["options"]):
                text=request["question"]["options"][int(text)-1]
        else:
            try:
                outcome, sha, text = final_review_input(text, request)
            except OperationalError as error:
                print(ui.paint(str(error), "warning"))
                continue
        try:
            with lock(state_dir() / "controller.lock"):
                answer_displayed(run_id, request, outcome, text, sha)
                if request["kind"]=="choice" and getattr(args,"record_choice_and_exit",False):
                    print("Your answer is saved. This controller exits; a fresh controller will consume it.",flush=True)
                    return 2
                run_controller(run_id)
        except OperationalError as error:
            print(ui.paint(f"Refused: {error}", "error"))
            print_summary(run_id)

def final_review_input(text, displayed):
    # Explicit phrases, not a model guessing whether discussion constitutes consent.
    phrase = text.casefold().rstrip(".")
    if phrase in ("approve", "approve this revision", "i approve this revision"):
        return "approved", displayed["artifact_sha256"], "Approved through controller"
    if text.startswith("approve "):
        # Keep explicit hashes available for compatibility, with strict validation below.
        return "approved", text.split(" ", 1)[1], "Approved through controller"
    if text.startswith("reject ") and text[7:].strip():
        return "rejected", None, text[7:].strip()
    raise OperationalError("I haven't recorded a decision. Say 'approve this revision', 'reject REASON', 'open', or 'exit'.")


def answer_displayed(run_id, displayed, outcome, text, sha):
    """Called under controller.lock: never bind consent to an unseen newer gate."""
    from .workflow import answer, state
    current = state(run_id).values.get("pending")
    if current != displayed:
        raise OperationalError("The review or its decisions changed. Review the updated version before answering.")
    answer(run_id, displayed["request_id"], outcome, text, sha)


def review_input(run_id, text):
    if text != "open" and not text.startswith("open "):
        return False
    from .review import open_review
    try:
        layout = open_review(run_id, text[5:].strip() if text.startswith("open ") else "latest")
        print(ui.paint(f"Review tab: {layout['tab_id']} — Vim is read-only; :q closes it.", "action"))
        print("Return to this controller for decisions. Opening a file does not approve it.")
    except (OperationalError, OSError, ValueError) as error:
        print(ui.paint(f"Could not open review: {error}", "error"))
    return True

def parser():
    p = argparse.ArgumentParser(description="Wayward Wrangler: persistent workflows for coding agents; Herdr-visible, bounded execution")
    p.add_argument("--color", choices=("auto", "always", "never"), default="auto")
    p.add_argument("--project", help="Explicit target project directory")
    p.add_argument("--state-dir", help="Project state override (relative to project)")
    p.add_argument("--registry-dir", help="Absolute user-local location index override")
    p.add_argument("--legacy-origin", help="Frozen original root for verification of copied legacy state")
    commands = p.add_subparsers(dest="command", required=True)
    commands.add_parser("init", help="Create project identity, fake workflow and state ignore rule")
    commands.add_parser("location-rebind", help="Explicitly bind a relocated quiescent project/state")
    sub = commands.add_parser("preflight", help="Validate native inputs and CLI/auth status without a model call")
    sub.add_argument("--run", required=True)
    sub.add_argument("--brief", required=True)
    sub.add_argument("--profile", "--config", default=".wayward-wrangler/workflow.toml")
    sub.add_argument("--context", required=True)
    sub.add_argument("--workspace")
    sub.add_argument("--socket")
    integration = commands.add_parser("integration", help="Install an explicit display integration")
    integration.add_argument("integration", choices=("herdr",))
    integration.add_argument("action", choices=("install",))
    integration.add_argument("--destination", help="Absolute plugin asset destination")
    integration.add_argument("--socket", help="Herdr server socket")
    sub = commands.add_parser("adopt-saved-draft", help="Explicit authorized context-01 saved-output recovery; never a native retry")
    sub.add_argument("run", choices=("context-01",))
    sub = commands.add_parser("adopt-provisional-revision", help="Adopt the exact completed decision-log-01 revision as provisional")
    sub.add_argument("run", choices=("decision-log-01",))
    sub = commands.add_parser("adopt-reviewed-critique", help="Adopt a validated saved critique with explicit finding interpretation")
    sub.add_argument("run")
    sub.add_argument("--interpretation", required=True, help="Reviewed JSON mapping of saved references")
    sub = commands.add_parser("open", help="Open a hash-checked artifact in a read-only Vim Herdr tab")
    sub.add_argument("run")
    sub.add_argument("--artifact", default="latest", choices=("latest", *records.STAGES))
    sub.add_argument("--sha256")
    commands.add_parser("open-link", help=argparse.SUPPRESS)
    sub = commands.add_parser("rebind", help="Change future worker workspace at an unanswered, quiescent human gate")
    sub.add_argument("run")
    sub.add_argument("--workspace", required=True)
    for name in ("start", "demo", "open-demo"):
        sub = commands.add_parser(name)
        sub.add_argument("--run", required=name != "start")
        sub.add_argument("--brief", required=name != "demo")
        sub.add_argument("--profile", "--config", default=".wayward-wrangler/workflow.toml")
        sub.add_argument("--workspace")
        sub.add_argument("--socket")
        sub.add_argument("--context", help="Explicit project-local context selection file")
        sub.add_argument("--authorize-native", metavar="PLAN_SHA256", help="Scripted native authorization; interactive runs confirm on screen")
        if name=="demo":
            sub.add_argument("--exit-at-choice",action="store_true")
            sub.add_argument("--record-choice-and-exit",action="store_true")
            sub.add_argument("--owner-generation", type=int, help=argparse.SUPPRESS)
            sub.add_argument("--owner-token", help=argparse.SUPPRESS)
    sub = commands.add_parser("supervise", help="Run the bounded project supervisor (service entry point)")
    sub.add_argument("run")
    sub.add_argument("--socket", required=True, help="Explicit Herdr Unix socket")
    sub.add_argument("--interval", type=float, default=2.0)
    sub.add_argument("--max-attempts", type=int, default=3)
    sub.add_argument("--once", action="store_true", help=argparse.SUPPRESS)
    sub = commands.add_parser("supervisor-install", help="Install and start a restartable user service for this run")
    sub.add_argument("run")
    sub.add_argument("--socket", required=True)
    sub = commands.add_parser("notices", help="Show durable supervisor failure notices")
    sub.add_argument("run")
    for name in ("resume", "status", "workers", "show", "logs", "observe", "cancel", "decide", "approve", "reject"):
        sub = commands.add_parser(name)
        sub.add_argument("run")
        if name == "status":
            sub.add_argument("--watch", action="store_true")
            sub.add_argument("--json", action="store_true")
        if name in ("logs", "observe", "cancel"):
            sub.add_argument("--operation", required=True, choices=records.STAGES)
        if name == "show":
            sub.add_argument("--artifact", default="latest")
        if name in ("decide", "approve", "reject"):
            sub.add_argument("--request", required=True)
            sub.add_argument("--record-only", action="store_true", help="Persist response, leave graph continuation for resume")
        if name == "decide":
            sub.add_argument("--answer-file", required=True)
        if name == "approve":
            sub.add_argument("--sha256", required=True)
        if name == "reject":
            sub.add_argument("--reason-file", required=True)
    for name in ("probe-start", "probe-resume", "probe-cancel", "probe-observe"):
        sub = commands.add_parser(name, help="Simulated backend foundation diagnostic")
        sub.add_argument("name")
        sub.add_argument("--workspace")
        sub.add_argument("--socket")
        sub.add_argument("--seconds", type=float, default=4)
        sub.add_argument("--mode", choices=["success", "failure"], default="success")
        sub.add_argument("--detach", action="store_true")
    return p

def main():
    from .herdr_backend import STOP_OBSERVING
    STOP_OBSERVING.clear()
    def stop_controller(_signal, _frame):
        STOP_OBSERVING.set()
        raise KeyboardInterrupt
    signal.signal(signal.SIGINT, stop_controller)
    args = parser().parse_args()
    ui.MODE = args.color
    try:
        action = args.command
        if action == "integration":
            from .integration import install_herdr
            print(json.dumps(install_herdr(args), indent=2))
            return 0
        if action == "open-link":
            from .review import parse_url
            from .runtime import lookup, registry_default
            link = parse_url(os.environ.get("HERDR_PLUGIN_CLICKED_URL", ""))
            if len(link) != 4:
                raise OperationalError("Legacy review link is ambiguous; open with explicit --project")
            key = link[0]
            paths = lookup(args.registry_dir or registry_default(), key)
            bind(paths)
        else:
            if not args.project:
                raise OperationalError("Select a project explicitly with --project DIR")
            paths = load(args.project, args.state_dir, args.registry_dir, initialize=action == "init",
                         legacy_origin=args.legacy_origin)
            bind(paths)
            if action == "init":
                register(paths)
                print(f"Initialized {paths.project}\nProject key: {paths.key}\nState: {paths.state}")
                return 0
            if action == "location-rebind":
                from .runtime import rebind_location
                print(json.dumps(rebind_location(paths), indent=2))
                return 0
            if not paths.legacy and action != "preflight":
                register(paths)
        if action == "preflight":
            from .native import plan
            selected_plan = _create(args, preview=True)
            print(json.dumps({"plan": selected_plan, "plan_sha256": digest(canonical(selected_plan)),
                              "native_clis_authenticated": True}, indent=2))
            return 0
        if action == "supervise":
            from .supervision import serve
            result = serve(args.run, args.socket, interval=args.interval, max_attempts=args.max_attempts,
                           once=args.once)
            print(result)
            return 3 if result == "uncertain" else 0
        if action == "supervisor-install":
            from .supervision import install_service
            print(json.dumps(install_service(args.run, args.socket), indent=2))
            return 0
        if action == "notices":
            from .supervision import root
            for file in sorted((root() / "notices").glob("*.json")):
                value = read(file)
                if value["run_id"] == args.run:
                    print(json.dumps(value, indent=2))
            return 0
        if action == "adopt-saved-draft":
            from .recovery import adopt
            with lock(state_dir() / "controller.lock"):
                result=adopt(args.run)
                print("Saved output adopted; original failure retained. No new draft call.")
                print_summary(args.run)
            return 0
        if action == "adopt-provisional-revision":
            from .quality_recovery import adopt
            with lock(state_dir() / "controller.lock"):
                result = adopt(args.run)
            print("Saved revision accepted provisionally; omitted findings remain open. No worker dispatched by adoption.")
            print("Output SHA-256:", result["result_sha256"])
            print_summary(args.run)
            return 0
        if action == "adopt-reviewed-critique":
            from .finding_recovery import adopt
            with lock(state_dir() / "controller.lock"):
                result = adopt(args.run, read(Path(args.interpretation)))
            print("Saved critique interpreted; original output, receipt and rejection retained. No critic call dispatched.")
            print("Output SHA-256:", result["result_sha256"])
            print_summary(args.run)
            return 0
        if action in ("open", "open-link"):
            from .review import open_review, parse_url
            values = parse_url(os.environ.get("HERDR_PLUGIN_CLICKED_URL", "")) if action == "open-link" else (args.run, args.artifact, args.sha256)
            layout = open_review(*(values[1:] if action == "open-link" else values))
            print(ui.paint(f"Review tab: {layout['tab_id']} — Vim is read-only; :q closes it.", "action"))
            return 0
        if action == "rebind":
            from .routing import rebind
            with lock(state_dir() / "controller.lock"):
                print(json.dumps(rebind(args.run, args.workspace), indent=2))
            return 0
        if action == "demo":
            return controller_demo(args)
        if action == "open-demo":
            with lock(state_dir() / "controller.lock"):
                no_other_execution()
                selected = target(args)
                # Reserve/freeze first. Repeating open-demo cannot create duplicate controller tabs.
                run_id = create(args)
                argv = [sys.executable, "-m", "agent_workflow.cli", "--project", str(project_dir()),
                        "--state-dir", str(state_dir()), "--registry-dir", str(current().registry), "demo", "--run", run_id,
                        "--workspace", selected["workspace"], "--socket", selected["socket"]]
                path = records.run_dir(run_id) / "controller-launch.json"
                atomic(path, {"status": "may-have-been-sent", "argv": argv}, immutable=True)
                layout = Herdr(selected["socket"]).create(selected["workspace"], f"Wayward Wrangler {current().key[:8]} controller {run_id}", argv, project_dir())
                atomic(records.run_dir(run_id) / "controller-pane.json", layout, immutable=True)
                print(json.dumps(layout, indent=2))
            return 0
        if action == "start":
            with lock(state_dir() / "controller.lock"):
                return run_controller(create(args))
        if action == "resume":
            with lock(state_dir() / "controller.lock"):
                return run_controller(args.run)
        if action in ("decide", "approve", "reject"):
            from .workflow import answer
            text = selected_path(args.answer_file, project_dir()).read_text().strip() if action == "decide" else (
                selected_path(args.reason_file, project_dir()).read_text().strip() if action == "reject" else "Approved through controller")
            with lock(state_dir() / "controller.lock"):
                answer(args.run, args.request, {"decide": "answered", "approve": "approved", "reject": "rejected"}[action],
                       text, getattr(args, "sha256", None))
                if args.record_only:
                    print("Response persisted; resume will consume it exactly once.")
                    return 0
                return run_controller(args.run)
        if action in ("status", "workers"):
            while True:
                print_summary(args.run, getattr(args, "json", False))
                if not getattr(args, "watch", False):
                    return 0
                time.sleep(2)
        if action == "show":
            from .artifacts import content
            from .workflow import state
            values = state(args.run).values
            ref = values["current"] if args.artifact == "latest" else read(
                records.run_dir(args.run) / "artifacts" / (records.name(args.artifact) + ".json"))["manifest"]
            print_summary(args.run)
            print("\n" + content(args.run, ref))
            return 0
        if action in ("logs", "observe", "cancel"):
            directory = records.run_dir(args.run) / "operations" / args.operation
            if action == "logs":
                print("Captured transcript (historical; use observe for the actual pane):")
                print((directory / "transcript.log").read_text())
            elif action == "observe":
                observe(directory)
            else:
                with lock(state_dir() / "cancellation.lock"):
                    print(json.dumps(cancel(directory), indent=2))
                    deadline = time.monotonic() + 10
                    while receipt(directory) is None and time.monotonic() < deadline:
                        time.sleep(.1)
                    result = receipt(directory)
                    if result is None:
                        records.set_status(args.run, "uncertain", "Cancellation outcome unresolved")
                        raise Uncertain("Cancellation outcome unresolved; no retry")
                    if result["status"] == "cancelled":
                        records.set_status(args.run, "cancelled", "Operator cancelled worker")
                    elif result["status"] == "completed":
                        # Do not reopen a completed/approved run by cancelling a historical worker.
                        print("Worker already completed; cancellation did not alter it.")
                    print(result["status"])
            return 0
        if action.startswith("probe"):
            directory = state_dir() / "probes" / records.name(args.name)
            if action == "probe-cancel":
                print(json.dumps(cancel(directory), indent=2))
                return 0
            if action == "probe-observe":
                observe(directory)
                return 0
            with lock(state_dir() / "controller.lock"):
                if action == "probe-start":
                    no_other_execution(directory)
                    prepare(directory, {"schema_version": 1, "operation_id": args.name, "run_id": "foundation",
                            "stage": args.name, "role": "author", "model": "deterministic-v1", "mode": args.mode,
                            "seconds": args.seconds}, target(args))
                print(json.dumps(wait(directory, detach=args.detach), indent=2))
            return 0
    except KeyboardInterrupt:
        print("\nController exited; existing workers remain in Herdr. Use resume.")
        return 130
    except (OperationalError, OSError, ValueError) as error:
        print(ui.paint(f"Stopped: {error}", "error", sys.stderr), file=sys.stderr)
        return 3
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
