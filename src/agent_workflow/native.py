"""Explicit admission for installed, text-only native workflow runs."""
from __future__ import annotations

import getpass
import json
import subprocess
import sys
from importlib.metadata import version

from .storage import OperationalError, canonical, digest


def application_version():
    return version("wayward-wrangler")


def plan(envelope):
    config = envelope["config"]
    return {
        "schema_version": 1,
        "project_key": envelope["project_key"],
        "project": envelope["project"],
        "run_id": envelope["run_id"],
        "brief_path": envelope["brief_path"],
        "brief_sha256": envelope["brief_sha256"],
        "config_path": envelope["config_path"],
        "config_sha256": envelope["config_sha256"],
        "rules_sha256": envelope["rules_sha256"],
        "context_path": envelope["context_snapshot"]["selection_path"],
        "context_digest": envelope["context_digest"],
        "application_version": envelope["application_version"],
        "protocol_version": envelope["protocol_version"],
        "prompt_contract": config["prompt_contract"],
        "review_limit": config["review_limit"],
        "max_invocations": config["max_invocations"],
        "roles": config["roles"],
        "target": envelope["target"],
    }


def check_auth(config):
    """Status only. Never emit the CLI's account data or authentication files."""
    author = config["roles"]["author"]["binary"]
    critic = config["roles"]["critic"]["binary"]
    claude = subprocess.run([author, "auth", "status", "--json"], capture_output=True,
                            text=True, timeout=15)
    try:
        logged_in = claude.returncode == 0 and json.loads(claude.stdout).get("loggedIn") is True
    except ValueError:
        logged_in = False
    if not logged_in:
        raise OperationalError("Claude authentication status is unavailable or logged out; no model call")
    codex = subprocess.run([critic, "login", "status"], capture_output=True,
                           text=True, timeout=15)
    if codex.returncode != 0 or "logged in" not in (codex.stdout + codex.stderr).lower():
        raise OperationalError("Codex authentication status is unavailable or logged out; no model call")


def authorize(args, envelope):
    selected = plan(envelope)
    fingerprint = digest(canonical(selected))
    supplied = getattr(args, "authorize_native", None)
    if supplied:
        if supplied != fingerprint:
            raise OperationalError("Native authorization digest does not match current project/inputs/settings")
        mode = "explicit-digest"
    else:
        if not sys.stdin.isatty():
            raise OperationalError("Native run requires interactive confirmation or --authorize-native PLAN_SHA256")
        print("\nNative run authorization", flush=True)
        print(f"Project: {selected['project']} ({selected['project_key']})")
        print(f"Run: {selected['run_id']}")
        print(f"Brief: {selected['brief_path']} sha256={selected['brief_sha256']}")
        print(f"Config: {selected['config_path']} sha256={selected['config_sha256']}")
        print(f"Context: {selected['context_path']} digest={selected['context_digest']}")
        for name, role in selected["roles"].items():
            print(f"{name}: {role['provider']} {role['model']} effort={role['reasoning']} binary={role['binary']}")
        print(f"Limits: {selected['max_invocations']} native launches, {selected['review_limit']} critiques")
        expected = f"authorize {selected['run_id']}"
        if input(f"Type '{expected}' to start: ").strip() != expected:
            raise OperationalError("Native run was not authorized; no worker dispatched")
        mode = "interactive"
    return {"schema_version": 1, "plan_sha256": fingerprint, "mode": mode,
            "recorded_by": getpass.getuser()}
