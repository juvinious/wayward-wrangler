"""Explicit project-to-project reads. No registry scan or source-store mutation."""
from __future__ import annotations

import json
from contextlib import closing
from pathlib import Path
import re
import sqlite3
import tomllib

from .storage import OperationalError, atomic, canonical, digest, read
from .records import name

MAX_FOREIGN_BYTES = 16 * 1024


def source_binding(project, state=None):
    project = Path(project).expanduser().resolve(strict=True)
    if not project.is_dir():
        raise OperationalError("Explicit source project is not a directory")
    config = tomllib.loads((project / ".wayward-wrangler/project.toml").read_text())
    key = config.get("project_key")
    if config.get("schema_version") != 1 or not isinstance(key, str) or not re.fullmatch(r"[0-9a-f]{32}", key):
        raise OperationalError("Source project identity is invalid")
    state = Path(state).expanduser().resolve(strict=True) if state else project / ".wayward-wrangler/state"
    marker = read(state / "project-identity.json")
    if marker != {"schema_version": 1, "project_key": key}:
        raise OperationalError("Source project/state identity mismatch")
    if not (state / "state.sqlite").is_file():
        raise OperationalError("Source state database is unavailable")
    return {"project": str(project), "state": str(state), "project_key": key}


def _database(binding):
    uri = (Path(binding["state"]) / "state.sqlite").as_uri() + "?mode=ro"
    db = sqlite3.connect(uri, uri=True, timeout=2)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA query_only=ON")
    return db


def _record(db, table, run_id, request_id):
    row = db.execute(f"SELECT payload FROM {table} WHERE run_id=? AND request_id=?",
                     (run_id, request_id)).fetchone()
    if not row:
        raise OperationalError(f"Selected source record is missing: {run_id}/{request_id}")
    return json.loads(row["payload"])


def select(binding, spec):
    """Read one explicit source item with a read-only SQLite connection."""
    kind = spec.get("kind")
    run_id = name(spec.get("run"))
    item_id = name(spec.get("id"))
    if kind not in ("artifact", "decision", "finding"):
        raise OperationalError("Cross-project import supports one selected artifact, decision or finding")
    with closing(_database(binding)) as db:
        row = db.execute("SELECT envelope,status FROM app_runs WHERE run_id=?", (run_id,)).fetchone()
        if not row or row["status"] != "approved":
            raise OperationalError("Selected source run is not approved")
        envelope = json.loads(row["envelope"])
        if envelope.get("project_key") != binding["project_key"]:
            raise OperationalError("Source run belongs to another project identity")
        final = _record(db, "app_requests", run_id, "A1")
        approval = _record(db, "app_responses", run_id, "A1")
        if (approval.get("outcome") != "approved" or
            approval.get("artifact_sha256") != final.get("artifact_sha256") or
            approval.get("decision_revision") != final.get("decision_revision")):
            raise OperationalError("Source final approval binding is invalid")
        source_root = Path(binding["state"]) / "runs" / run_id
        if kind == "artifact":
            ref = final["artifact"]
            if ref["id"] != item_id:
                raise OperationalError("Only the exact approved source artifact may be imported")
            manifest = read(source_root / "artifacts" / (item_id + ".json"))["manifest"]
            if manifest != ref:
                raise OperationalError("Source artifact manifest disagrees with approval")
            path = (source_root / ref["path"]).resolve(strict=True)
            if not path.is_relative_to(source_root.resolve()) or digest(path.read_bytes()) != ref["sha256"]:
                raise OperationalError("Source artifact hash/path changed")
            content = path.read_text()
            record_hash = digest(canonical(ref))
            version_hash = ref["sha256"]
        elif kind == "decision":
            request = _record(db, "app_requests", run_id, item_id)
            response = _record(db, "app_responses", run_id, item_id)
            if (request.get("kind") != "choice" or response.get("outcome") != "answered" or
                not any(d["request_id"] == item_id and d["answer"] == response["answer"]
                        for d in final.get("decisions", [])) or
                response.get("artifact_sha256") != request.get("artifact_sha256") or
                response.get("decision_revision") != request.get("decision_revision")):
                raise OperationalError("Selected source decision is not accepted in approved context")
            content = {"question": request["question"], "response": response}
            record_hash = digest(canonical({"request": request, "response": response}))
            version_hash = record_hash
        else:
            finding = final.get("findings", {}).get(item_id)
            if not finding or finding.get("status") != "open":
                raise OperationalError("Selected source finding is not open")
            stage = name(finding["origin"])
            origin = read(source_root / "artifacts" / (stage + ".json"))
            ledger_path=source_root / "quality" / (stage + "-finding-ledger.json")
            if ledger_path.exists():
                ledger = read(ledger_path)
                supported = (ledger.get("result_sha256") == digest(canonical(origin["result"])) and
                             any(x.get("id") == item_id and x.get("evidence") == finding["evidence"]
                                 for x in ledger.get("new_findings", [])))
            else:
                supported = any(x.get("id") == item_id and x.get("evidence") == finding["evidence"]
                                for x in origin["result"].get("findings", []))
            if not supported:
                raise OperationalError("Selected finding origin evidence is invalid")
            content = {**finding, "id": f"{binding['project_key']}/{run_id}/{item_id}"}
            record_hash = digest(canonical(finding))
            version_hash = record_hash
    item = {"schema_version": 1, "source_project_key": binding["project_key"],
            "source_run": run_id, "kind": kind, "record_id": item_id,
            "qualified_id": f"{binding['project_key']}/{run_id}/{item_id}",
            "source_record_sha256": record_hash, "version_sha256": version_hash,
            "approval_sha256": final["artifact_sha256"], "content": content}
    if len(canonical(item).encode()) > MAX_FOREIGN_BYTES:
        raise OperationalError("Selected foreign item exceeds 16 KiB; narrow the request")
    return item


def grant_root(paths):
    return paths.state / "session/context-access"


def grant_path(paths, grant_id):
    return grant_root(paths) / "grants" / (name(grant_id) + ".json")


def revoked(paths, grant_id):
    return (grant_root(paths) / "revocations" / (name(grant_id) + ".json")).exists()


def active_for_task(paths, task_id):
    result = []
    for path in sorted((grant_root(paths) / "grants").glob("*.json")):
        grant = read(path)
        if grant.get("schema_version") != 1 or grant.get("recipient_project_key") != paths.key:
            raise OperationalError("Unknown or foreign context grant record")
        if grant["task_id"] == task_id and not revoked(paths, grant["grant_id"]):
            result.append(grant)
    if sum(len(canonical(grant["item"]).encode()) for grant in result) > MAX_FOREIGN_BYTES:
        raise OperationalError("Selected foreign context exceeds 16 KiB across grants")
    return result


def check_frozen(paths, imports):
    for item in imports:
        grant = read(grant_path(paths, item["grant_id"]))
        if (grant.get("recipient_project_key") != paths.key or revoked(paths, item["grant_id"]) or
            digest(canonical(grant["item"])) != item["item_sha256"] or
            digest(canonical(item["item"])) != item["item_sha256"]):
            raise OperationalError("Cross-project grant was revoked or changed; dependent dispatch stopped")
        if grant["dependency_mode"] == "must_match_source":
            binding = source_binding(grant["source_project"], grant["source_state"])
            current = select(binding, grant["selector"])
            if digest(canonical(current)) != item["item_sha256"]:
                raise OperationalError("Selected source item changed; dependent dispatch stopped")
