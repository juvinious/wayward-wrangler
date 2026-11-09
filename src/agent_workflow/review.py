"""Read-only artifact navigation, isolated from workflow dispatch and answers."""
import os
import re
import shutil
from . import artifacts, records
from .herdr_backend import Herdr
from .storage import ROOT, LOCAL, OperationalError, lock, read
from .runtime import current

LINK_PATTERN = r"https://awp\.invalid/review/([a-zA-Z0-9][a-zA-Z0-9_-]{0,63})/(draft|critique-1|critique-2|revision-1|revision-2)/([0-9a-f]{64})"
LINK_V2 = r"https://awp\.invalid/v2/review/([0-9a-f]{32})/([a-zA-Z0-9][a-zA-Z0-9_-]{0,63})/(draft|critique-1|critique-2|revision-1|revision-2)/([0-9a-f]{64})"

def url(run_id, ref):
    selected = current()
    if selected and not selected.legacy:
        result = f"https://awp.invalid/v2/review/{selected.key}/{run_id}/{ref['id']}/{ref['sha256']}"
        pattern = LINK_V2
    else:
        result = f"https://awp.invalid/review/{run_id}/{ref['id']}/{ref['sha256']}"
        pattern = LINK_PATTERN
    if not re.fullmatch(pattern, result):
        raise OperationalError("Invalid review link identity")
    return result

def parse_url(value):
    match = re.fullmatch(LINK_V2, value)
    if match:
        return match.groups()
    match = re.fullmatch(LINK_PATTERN, value)
    if not match:
        raise OperationalError("Not a valid prototype review link")
    return match.groups()

def resolve(run_id, artifact="latest", expected_hash=None):
    from .workflow import state
    if artifact == "latest":
        ref = state(run_id).values.get("current")
    elif artifact in records.STAGES:
        ref = read(records.run_dir(run_id) / "artifacts" / (artifact + ".json"))["manifest"]
        if ref["id"] != artifact:
            raise OperationalError("Artifact identity mismatch")
    else:
        raise OperationalError("Choose latest or a published stage name")
    if not ref:
        raise OperationalError("No published artifact yet")
    if expected_hash is not None and ref["sha256"] != expected_hash:
        raise OperationalError("Review link hash no longer matches the artifact")
    artifacts.content(run_id, ref)  # Checks path containment and bytes, including historical revisions.
    return ref, (records.run_dir(run_id) / ref["path"]).resolve()

def open_review(run_id, artifact="latest", expected_hash=None):
    if os.environ.get("HERDR_ENV") != "1":
        raise OperationalError("Run open inside Herdr, or click through the Herdr review plugin")
    selected = current()
    with lock((selected.state if selected else LOCAL) / "review-open.lock"):
        ref, path = resolve(run_id, artifact, expected_hash)
        target = records.effective_target(run_id)
        if os.environ.get("HERDR_SOCKET_PATH") != target["socket"]:
            raise OperationalError("Review request belongs to another Herdr server; use the run's session")
        editor = shutil.which("vim")
        if not editor:
            raise OperationalError("Vim is not available on PATH")
        # No shell, modelines, startup scripts, swap or viminfo; review cannot edit the artifact.
        argv = [editor, "-N", "-u", "NONE", "-i", "NONE", "-n", "-M",
                "-c", "set nomodeline", "-c", "syntax on", "--", str(path)]
        h = Herdr(target["socket"])
        h.call("workspace.get", {"workspace_id": target["workspace"]})
        layout = None
        for tab in h.call("tab.list")["tabs"]:
            if tab["workspace_id"] != target["workspace"]:
                continue
            candidate = h.call("layout.export", {"tab_id": tab["tab_id"]})["layout"]
            node = candidate["root"]
            if node["type"] == "pane" and node.get("command") == argv:
                layout = candidate
                break
        if layout is None:
            # No blind retry on a lost response. A subsequent request first searches the live layout.
            key = selected.key[:8] if selected else "legacy"
            layout = h.create(target["workspace"], f"Wayward Wrangler review {key} {run_id} {ref['id']} {ref['sha256'][:8]}", argv, selected.project if selected else ROOT)
        h.call("tab.focus", {"tab_id": layout["tab_id"]})
        return layout
