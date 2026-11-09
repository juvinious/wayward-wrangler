"""Project-local dispatch fence; no release registry or model state is consulted."""
from __future__ import annotations

from contextlib import contextmanager
import json
import sqlite3

from .runtime import current
from .storage import OperationalError, lock


def phase(paths):
    database = paths.state / "state.sqlite"
    if not database.exists():
        return "running"  # historical runs without a project session
    with sqlite3.connect(database, timeout=3) as db:
        present = db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='session_metadata'").fetchone()
        if not present:
            return "running"
        row = db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone()
    if row is None:
        return "running"
    value = json.loads(row[0])
    if value.get("schema_version") != 1 or value.get("phase") not in ("running", "exit-requested", "draining", "paused"):
        raise OperationalError("Unknown project session lifecycle record; dispatch refused")
    return value["phase"]


@contextmanager
def dispatch_guard():
    selected = current()
    if selected is None:
        yield  # historical standalone run
        return
    with lock(selected.state / "session/dispatch.lock"):
        if phase(selected) != "running":
            from .herdr_backend import ControllerDetached
            raise ControllerDetached("Project session exit pause; existing execution remains supervised")
        yield
