"""Standalone entry point and Herdr presentation attachment.

The socket API is used by deterministic application code. Model output never
contains a Herdr method or command. No focused-pane fallback is permitted.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import sys
import uuid

from .herdr_backend import DEFAULT_SOCKET, Herdr
from .storage import OperationalError, alive, atomic, read


def selection(paths, override=None):
    """An endpoint is explicit after first use, including for future reconnects."""
    file = paths.project / ".wayward-wrangler/herdr.json"
    if file.exists():
        value = read(file)
        if value.get("schema_version") != 1 or value.get("project_key") != paths.key:
            raise OperationalError("Herdr endpoint configuration belongs to another project")
        socket = value.get("socket")
        if override and str(Path(override).expanduser().resolve()) != socket:
            raise OperationalError("Requested Herdr socket differs from saved project endpoint")
    else:
        socket = str(Path(override or os.environ.get("HERDR_SOCKET_PATH", DEFAULT_SOCKET)).expanduser().resolve())
        atomic(file, {"schema_version": 1, "project_key": paths.key, "socket": socket}, immutable=True)
    if not Path(socket).is_absolute():
        raise OperationalError("Herdr socket must be absolute")
    return socket


def _pane_nodes(node):
    if node["type"] == "pane":
        yield node
    else:
        yield from _pane_nodes(node["first"])
        yield from _pane_nodes(node["second"])


def _matches(h, token):
    result = []
    for tab in h.call("tab.list")["tabs"]:
        layout = h.call("layout.export", {"tab_id": tab["tab_id"]})["layout"]
        for pane in _pane_nodes(layout["root"]):
            argv = pane.get("command") or []
            if "--ui-token" in argv and argv.index("--ui-token") + 1 < len(argv) and argv[argv.index("--ui-token") + 1] == token:
                result.append({"workspace_id": layout["workspace_id"], "tab_id": layout["tab_id"], "pane_id": pane["pane_id"]})
    return result


def _same_server(left, right):
    return all(left[field] == right[field] for field in ("pid", "start", "boot"))


def _finish_intent(paths, h, root, intent, ui_command):
    """Reconcile a durable launch intent through Herdr's explicit identities."""
    token = intent["token"]
    matches = _matches(h, token)
    if len(matches) > 1:
        raise OperationalError("Duplicate UI panes match one launch token; inspect before recovery")
    if matches:
        pane = matches[0]
    else:
        label = intent["workspace_label"]
        preferred=intent.get("preferred_workspace_id")
        workspace=None
        if preferred:
            try:
                workspace=h.call("workspace.get",{"workspace_id":preferred})["workspace"]
            except OperationalError as error:
                if "workspace_not_found" not in str(error):raise
        workspaces = [] if workspace else [item for item in h.call("workspace.list")["workspaces"] if item["label"] == label]
        if len(workspaces) > 1:
            raise OperationalError("Duplicate workspaces match one UI intent; inspect before recovery")
        if workspaces and workspace is None:
            workspace = workspaces[0]
        if workspace is None:
            workspace = h.call("workspace.create", {"label": label,
                "cwd": str(paths.project), "focus": True})["workspace"]
        # A sent layout with no matching pane cannot be safely sent again.
        marker = root / "ui-layout-intents" / (token + ".json")
        if marker.exists():
            raise OperationalError("UI layout outcome is uncertain; inspect the owned workspace before replacement")
        argv = ([*ui_command, "--ui-token", token] if ui_command else
                [sys.executable, "-m", "agent_workflow.shell", "--project", str(paths.project),
                 "--state-dir", str(paths.state), "--registry-dir", str(paths.registry),
                 "tui", "--ui-token", token, "--socket", intent["socket"]])
        atomic(marker, {"schema_version": 1, "token": token, "workspace_id": workspace["workspace_id"],
                        "argv": argv, "status": "may-have-been-sent"}, immutable=True)
        try:
            layout = h.call("layout.apply", {"workspace_id": workspace["workspace_id"],
                "tab_label": "Waywr coordinator", "focus": True,
                "root": {"type": "pane", "label": "Waywr coordinator",
                         "cwd": str(paths.project), "command": argv}})["layout"]
            pane = {"workspace_id": layout["workspace_id"], "tab_id": layout["tab_id"],
                    "pane_id": layout["root"]["pane_id"]}
        except Exception:
            matches = _matches(h, token)
            if len(matches) != 1:
                raise OperationalError("UI layout outcome is uncertain; saved intent retained")
            pane = matches[0]
    binding = {**intent, "pane": pane, "status": "bound"}
    atomic(root / "ui-bindings" / (token + ".json"), binding, immutable=True)
    atomic(root / "ui-current.json", binding)
    return {"status": "created", **pane}


def attach(paths, socket, ui_command=None):
    """Create or focus only an identity-verified, application-owned UI pane."""
    h = Herdr(socket)
    server = h.server_identity()
    ping = h.call("ping")
    if ping.get("protocol") != 22:
        raise OperationalError("Unsupported Herdr API protocol; no workspace mutation")
    root = paths.state / "session"
    pointer = root / "ui-current.json"
    prior = read(pointer) if pointer.exists() else None
    if prior:
        if not _same_server(prior["server"], server):
            claim = root / "ui-claims" / (prior["token"] + ".json")
            if claim.exists() and alive(read(claim)["process"]):
                raise OperationalError("Old UI process survives a Herdr server change; inspect before replacement")
        else:
            matches = _matches(h, prior["token"])
            if len(matches) > 1:
                raise OperationalError("UI launch identity is ambiguous; inspect the saved binding")
            claim = root / "ui-claims" / (prior["token"] + ".json")
            if claim.exists() and alive(read(claim)["process"]):
                if len(matches) != 1 or matches[0] != prior["pane"]:
                    raise OperationalError("Live UI process has no verified matching pane; replacement refused")
                h.call("workspace.focus", {"workspace_id": matches[0]["workspace_id"]})
                h.call("tab.focus", {"tab_id": matches[0]["tab_id"]})
                return {"status": "reconnected", **matches[0]}
            # Verified old process exited. Keep its tab/output; create a new UI.
    for file in sorted((root / "ui-intents").glob("*.json")):
        intent = read(file)
        if _same_server(intent["server"], server) and not (root / "ui-bindings" / file.name).exists():
            return _finish_intent(paths, h, root, intent, ui_command)
    token = uuid.uuid4().hex
    intent = {"schema_version": 1, "project_key": paths.key, "token": token,
              "server": server, "socket": socket, "status": "may-have-been-sent",
              "workspace_label": f"Wayward Wrangler {paths.project.name} [{token[:8]}]"}
    if prior and _same_server(prior["server"],server):
        intent["preferred_workspace_id"]=prior["pane"]["workspace_id"]
    atomic(root / "ui-intents" / (token + ".json"), intent, immutable=True)
    return _finish_intent(paths, h, root, intent, ui_command)


def _legacy(argv):
    from .cli import main as old_main
    sys.argv = [sys.argv[0], *argv]
    return old_main()


def main():
    args = sys.argv[1:]
    paired = Path(sys.prefix).parent / "bin/waywr"
    if paired.is_file():
        os.execv(str(paired), [str(paired), *args])
    if args and any(arg in {"init","location-rebind","preflight","integration","open","open-link",
                            "status","workers","show","logs","observe","cancel","decide","approve",
                            "reject","resume","start","demo","open-demo","supervise","supervisor-install",
                            "notices","rebind","adopt-saved-draft","adopt-provisional-revision",
                            "adopt-reviewed-critique","probe-start","probe-resume","probe-cancel","probe-observe"}
                    for arg in args):
        return _legacy(args)
    print("Wayward Wrangler interactive mode requires the paired Rust installation. "
          "Existing diagnostic commands remain available with --project.", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
