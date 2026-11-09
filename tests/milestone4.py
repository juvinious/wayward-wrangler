"""Opt-in installed-wheel + isolated Herdr acceptance. Simulated workers only.

Run with the pipx-installed Python and --cli pointing at the installed executable.
All target projects, state, HOME, server and worker processes are test-owned.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import tempfile
import time

from agent_workflow.herdr_backend import Herdr
from agent_workflow.storage import alive, atomic, identity, read


def eventually(action, timeout=30):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        try:
            value = action()
        except (FileNotFoundError, ConnectionRefusedError):
            value = None
        if value:
            return value
        time.sleep(.05)
    raise AssertionError("Timed out awaiting isolated evidence")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--skip-review-open", action="store_true", help="Container has no Vim; host run covers the click")
    args = parser.parse_args()
    root = args.evidence_root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    for name in ("home", "config/herdr", "runtime", "registry", "a", "b", "other"):
        (root / name).mkdir(parents=True, exist_ok=True)
    (root / "config/herdr/config.toml").write_text('onboarding = false\nshell = "/bin/sh"\n')
    socket = str(root / "runtime/herdr.sock")
    env = {**os.environ, "HOME": str(root / "home"), "XDG_CONFIG_HOME": str(root / "config"),
           "XDG_DATA_HOME": str(root / "data"), "XDG_RUNTIME_DIR": str(root / "runtime"),
           "HERDR_SOCKET_PATH": socket, "SHELL": "/bin/sh", "TERM": "xterm-256color"}
    server_log = (root / "herdr-server.log").open("wb")
    server = subprocess.Popen(["herdr", "server"], env=env, stdout=server_log, stderr=subprocess.STDOUT)
    server_log.close()
    owned = identity(server.pid)
    h = Herdr(socket)
    checks = []

    def cli(project, *arguments, expected=0, cwd=None):
        command = [str(args.cli), "--project", str(project), "--registry-dir", str(root / "registry"), *arguments]
        result = subprocess.run(command, env=env, cwd=cwd or root / "other", capture_output=True, text=True, timeout=100)
        assert result.returncode == expected, (command, result.returncode, result.stdout, result.stderr)
        return result.stdout

    def phase(project, run="demo"):
        return project / ".wayward-wrangler/state/runs" / run

    def status(project):
        return json.loads(cli(project, "status", "demo", "--json"))

    def complete(project, answer):
        run = phase(project)
        eventually(lambda: status(project)["status"] == "awaiting-input")
        choice = status(project)["pending"]
        assert choice["kind"] == "choice"
        (project / "answer.txt").write_text(answer)
        cli(project, "decide", "demo", "--request", choice["request_id"],
            "--answer-file", "answer.txt", "--record-only")
        assert read(run / "artifacts" / ("decision-" + choice["request_id"] + ".json"))["answer"] == answer
        cli(project, "resume", "demo", expected=2)
        final = status(project)["pending"]
        assert final["kind"] == "final-review"
        cli(project, "approve", "demo", "--request", final["request_id"],
            "--sha256", final["artifact_sha256"], "--record-only")
        cli(project, "resume", "demo")
        assert status(project)["status"] == "approved"
        assert read(run / "operations/draft/invocations.json")["count"] == 1
        return final

    try:
        eventually(lambda: h.server_identity())
        workspace = h.call("workspace.create", {"label": "Wayward Wrangler M4 isolated",
                    "cwd": str(root), "focus": False})["workspace"]["workspace_id"]
        plugin = root / "data/wayward-wrangler/herdr"
        installed = subprocess.run([str(args.cli), "--registry-dir", str(root / "registry"),
            "integration", "herdr", "install", "--destination", str(plugin), "--socket", socket],
            env=env, cwd=root / "other", capture_output=True, text=True)
        assert installed.returncode == 0, (installed.stdout, installed.stderr)
        assert (plugin / "binding.json").is_file() and (plugin / "open_review.py").is_file()
        linked = subprocess.run(["herdr", "plugin", "list", "--json"], env=env, capture_output=True, text=True)
        assert linked.returncode == 0 and "local.agent-workflow-review" in linked.stdout
        checks.append({"case": "copied-installed-plugin", "passed": True, "destination": str(plugin)})
        for project in (root / "a", root / "b"):
            cli(project, "init")
            (project / "brief.md").write_text("# Fixture\n\nThis automated acceptance uses synthetic decisions only.\n")
        key_a = read(root / "a/.wayward-wrangler/state/project-identity.json")["project_key"]
        key_b = read(root / "b/.wayward-wrangler/state/project-identity.json")["project_key"]
        assert key_a != key_b
        # Controller A and its actual fake worker are separate Herdr panes.
        cli(root / "a", "open-demo", "--run", "demo", "--brief", "brief.md", "--workspace", workspace)
        operation = phase(root / "a") / "operations/draft"
        binding = eventually(lambda: read(operation / "binding.json") if (operation / "binding.json").exists() else None)
        controller = read(phase(root / "a") / "controller-pane.json")
        assert binding["pane_id"] != controller["root"]["pane_id"]
        eventually(lambda: (operation / "child.json").exists())
        worker = read(operation / "child.json")["identity"]
        assert alive(worker)
        live = eventually(lambda: (lambda text: text if "simulated work" in text or "RUNNING child" in text else None)(
            h.call("pane.read", {"pane_id": binding["pane_id"], "source": "recent", "lines": 35})["read"]["text"]))
        h.call("tab.close", {"tab_id": controller["tab_id"]})
        eventually(lambda: (operation / "receipt.json").exists())
        assert read(operation / "receipt.json")["status"] == "completed"
        cli(root / "a", "resume", "demo", expected=2)
        assert read(operation / "binding.json") == binding
        assert read(operation / "invocations.json")["count"] == 1
        retained = h.call("pane.read", {"pane_id": binding["pane_id"], "source": "recent", "lines": 35})["read"]["text"]
        assert "completed" in retained.lower()
        final_a = complete(root / "a", "7 days")
        checks.append({"case": "controller-exit-worker-survival-reconnect", "passed": True,
                       "project_key": key_a, "pane": binding["pane_id"], "draft_invocations": 1})
        # The same run ID is independent in B, including its link and decisions.
        cli(root / "b", "open-demo", "--run", "demo", "--brief", "brief.md", "--workspace", workspace)
        eventually(lambda: status(root / "b")["status"] == "awaiting-input")
        final_b = complete(root / "b", "30 days")
        assert final_a["artifact_sha256"] != "" and final_b["artifact_sha256"] != ""
        assert read(phase(root / "a") / "artifacts/decision-D-retention.json")["answer"] == "7 days"
        assert read(phase(root / "b") / "artifacts/decision-D-retention.json")["answer"] == "30 days"
        from agent_workflow.runtime import load, bind
        from agent_workflow.review import url, parse_url
        bind(load(root / "a", registry_dir=root / "registry")); link_a = url("demo", final_a["artifact"])
        bind(load(root / "b", registry_dir=root / "registry")); link_b = url("demo", final_b["artifact"])
        assert link_a != link_b and parse_url(link_a)[0] == key_a and parse_url(link_b)[0] == key_b
        if not args.skip_review_open:
            clicked_env = {**env, "HERDR_ENV": "1", "HERDR_PLUGIN_CLICKED_URL": link_b}
            opened = subprocess.run(["python3", str(plugin / "open_review.py")], env=clicked_env,
                                    cwd=root / "other", capture_output=True, text=True, timeout=15)
            assert opened.returncode == 0, (opened.stdout, opened.stderr)
            layout = next(x for x in h.call("tab.list")["tabs"] if "review" in x["label"].lower())
            node = h.call("layout.export", {"tab_id": layout["tab_id"]})["layout"]["root"]
            assert str(root / "b/.wayward-wrangler/state/runs/demo") in " ".join(node["command"])
            bad_env = {**clicked_env, "HERDR_PLUGIN_CLICKED_URL": link_b[:-64] + "0" * 64}
            wrong = subprocess.run(["python3", str(plugin / "open_review.py")], env=bad_env,
                                   cwd=root / "other", capture_output=True, text=True, timeout=15)
            assert wrong.returncode != 0 and "hash" in wrong.stderr.lower()
            old = subprocess.run(["python3", str(plugin / "open_review.py")],
                env={**clicked_env, "HERDR_PLUGIN_CLICKED_URL":
                     "https://awp.invalid/review/demo/revision-2/" + final_b["artifact_sha256"]},
                cwd=root / "other", capture_output=True, text=True, timeout=15)
            assert old.returncode != 0 and "ambiguous" in old.stderr.lower()
        checks.append({"case": "same-run-id-project-isolation", "passed": True,
                       "keys": [key_a, key_b], "links": [link_a, link_b]})
        # A quiescent copied project can move without changing run evidence or
        # requiring the former absolute location for newly resolved context.
        h.call("server.stop")
        server.wait(timeout=10)
        project_a = root / "a"
        (project_a / "docs").mkdir()
        (project_a / "docs/rules.md").write_text("# Project rule\nKeep review decisions explicit.\n")
        (project_a / "selection.toml").write_text('''schema_version = 1
project_id = "fixture-a"
scope = "report"
[[entries]]
kind = "document"
scope = "report"
reason = "Project rule"
path = "docs/rules.md"
[[entries]]
kind = "decision"
scope = "report"
reason = "Approved retention decision"
run = "demo"
id = "D-retention"
topic_key = "retention"
''')
        from agent_workflow.context import resolve, check
        from agent_workflow.storage import canonical, digest, OperationalError
        bind(load(project_a, registry_dir=root / "registry"))
        snapshot = resolve(project_a / "selection.toml")
        assert snapshot["schema_version"] == 2 and not Path(snapshot["selection_path"]).is_absolute()
        preserved = (phase(project_a) / "artifacts/decision-A1.json").read_bytes()
        relocated = root / "a-relocated"
        shutil.copytree(project_a, relocated)
        project_a.rename(root / "a-original-unavailable")
        cli(relocated, "location-rebind")
        assert (phase(relocated) / "artifacts/decision-A1.json").read_bytes() == preserved
        bind(load(relocated, registry_dir=root / "registry"))
        envelope = {"workflow_version": 2, "run_id": "next", "context_snapshot": snapshot,
                    "context_digest": digest(canonical(snapshot))}
        check(envelope)
        (relocated / "docs/rules.md").write_text("# changed\n")
        try:
            check(envelope)
        except OperationalError as error:
            assert "Stale context" in str(error)
        else:
            raise AssertionError("Changed project context was accepted")
        checks.append({"case": "explicit-relocation-relative-context", "passed": True,
                       "project_key": key_a, "approval_sha256": digest(preserved)})
        atomic(root / "result.json", {"passed": True, "checks": checks, "installed_cli": str(args.cli)})
        print("MILESTONE 4 INSTALLED HERDR ACCEPTANCE PASSED", root, flush=True)
    finally:
        # Only this fresh server is stopped. It owns every pane created above.
        if server.poll() is None and alive(owned):
            try:
                h.call("server.stop")
                server.wait(timeout=10)
            except Exception:
                os.kill(server.pid, signal.SIGTERM)
                server.wait(timeout=10)


if __name__ == "__main__":
    main()
