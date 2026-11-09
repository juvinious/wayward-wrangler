from __future__ import annotations
import contextlib
import getpass
import json
import re
import sqlite3
from .storage import LOCAL, OperationalError, atomic, canonical, now, read

DB = LOCAL / "state.sqlite"

def state_dir():
    from .runtime import current
    bound = current()
    return bound.state if bound else LOCAL

def db_path():
    from .runtime import current
    bound = current()
    return bound.state / "state.sqlite" if bound else DB
TERMINAL = {"approved", "rejected", "failed", "cancelled", "output-invalid"}
STAGES = ("draft", "critique-1", "revision-1", "critique-2", "revision-2")

def name(value):
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}", value):
        raise OperationalError("Use a short alphanumeric ID with hyphens/underscores")
    return value

def run_dir(run_id):
    return state_dir() / "runs" / name(run_id)

@contextlib.contextmanager
def connect():
    state_dir().mkdir(parents=True, exist_ok=True)
    db = sqlite3.connect(db_path(), timeout=10)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA journal_mode=WAL")
    db.execute("PRAGMA synchronous=FULL")
    db.executescript("""
    CREATE TABLE IF NOT EXISTS app_runs (
      run_id TEXT PRIMARY KEY, envelope TEXT NOT NULL,
      status TEXT NOT NULL, active INTEGER NOT NULL, detail TEXT);
    CREATE UNIQUE INDEX IF NOT EXISTS app_single_active ON app_runs(active) WHERE active=1;
    CREATE TABLE IF NOT EXISTS app_operations (
      run_id TEXT NOT NULL, stage TEXT NOT NULL, operation_id TEXT NOT NULL UNIQUE,
      packet_hash TEXT NOT NULL, PRIMARY KEY(run_id, stage));
    CREATE TABLE IF NOT EXISTS app_requests (
      run_id TEXT NOT NULL, request_id TEXT NOT NULL, payload TEXT NOT NULL,
      PRIMARY KEY(run_id, request_id));
    CREATE TABLE IF NOT EXISTS app_rebindings (
      id INTEGER PRIMARY KEY AUTOINCREMENT, run_id TEXT NOT NULL, payload TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS app_responses (
      run_id TEXT NOT NULL, request_id TEXT NOT NULL, payload TEXT NOT NULL,
      PRIMARY KEY(run_id, request_id));
    """)
    try:
        yield db
        db.commit()
    finally:
        db.close()

def get_run(run_id):
    with connect() as db:
        row = db.execute("SELECT * FROM app_runs WHERE run_id=?", (name(run_id),)).fetchone()
    if row is None:
        raise OperationalError(f"Unknown run {run_id}")
    result = dict(row)
    result["envelope"] = json.loads(result["envelope"])
    from .runtime import current
    bound = current()
    key = result["envelope"].get("project_key")
    if bound and key is not None and key != bound.key:
        raise OperationalError("Run belongs to another project")
    return result

def active_run():
    with connect() as db:
        row = db.execute("SELECT run_id FROM app_runs WHERE active=1").fetchone()
    return row["run_id"] if row else None

def create_run(envelope):
    with connect() as db:
        try:
            db.execute("INSERT INTO app_runs VALUES (?,?,?,1,NULL)", (envelope["run_id"], canonical(envelope), "running"))
        except sqlite3.IntegrityError as error:
            raise OperationalError("Run already exists or another run is active; inspect status first") from error

def set_status(run_id, status, detail=None):
    with connect() as db:
        db.execute("UPDATE app_runs SET status=?, active=?, detail=? WHERE run_id=?",
                   (status, 0 if status in TERMINAL and status != "output-invalid" else 1, detail, run_id))

def reserve(run_id, stage, op_id, packet_hash):
    if stage not in STAGES:
        raise OperationalError("Unrecognized stage or third critique refused")
    with connect() as db:
        old = db.execute("SELECT * FROM app_operations WHERE run_id=? AND stage=?", (run_id, stage)).fetchone()
        if old:
            if old["operation_id"] != op_id or old["packet_hash"] != packet_hash:
                raise OperationalError("Stage already reserved with different input; no redispatch")
            return
        if stage.startswith("critique") and db.execute(
            "SELECT count(*) FROM app_operations WHERE run_id=? AND stage LIKE 'critique-%'", (run_id,)).fetchone()[0] >= 2:
            raise OperationalError("Two-critique limit reached")
        db.execute("INSERT INTO app_operations VALUES (?,?,?,?)", (run_id, stage, op_id, packet_hash))

def request(run_id, payload):
    with connect() as db:
        prior = db.execute("SELECT payload FROM app_requests WHERE run_id=? AND request_id=?",
                           (run_id, payload["request_id"])).fetchone()
        text = canonical(payload)
        if prior and prior["payload"] != text:
            raise OperationalError("Request changed under an existing ID")
        db.execute("INSERT OR IGNORE INTO app_requests VALUES (?,?,?)", (run_id, payload["request_id"], text))
    return payload

def response(run_id, request_id):
    with connect() as db:
        row = db.execute("SELECT payload FROM app_responses WHERE run_id=? AND request_id=?", (run_id, request_id)).fetchone()
    return json.loads(row["payload"]) if row else None

def save_response(run_id, payload):
    old = response(run_id, payload["request_id"])
    if old:
        if any(old.get(k) != payload.get(k) for k in ("answer", "outcome", "artifact_sha256", "decision_revision", "context_digest", "supersedes")):
            raise OperationalError("Conflicting response; accepted decisions are immutable")
        export_response(run_id, old)
        return old
    record = {**payload, "recorded_by": getpass.getuser(), "time": now(), "schema_version": 1}
    with connect() as db:
        db.execute("INSERT INTO app_responses VALUES (?,?,?)", (run_id, payload["request_id"], canonical(record)))
    export_response(run_id, record)
    return record

def export_response(run_id, record):
    base = run_dir(run_id) / "artifacts" / ("decision-" + name(record["request_id"]))
    atomic(base.with_suffix(".json"), record, immutable=True)
    atomic(base.with_suffix(".md"), f"# {record['request_id']}\n\nOutcome: {record['outcome']}\n\n{record['answer']}\n\nReviewed SHA-256: {record['artifact_sha256']}\n", immutable=True)

def operations(run_id):
    with connect() as db:
        return [dict(row) for row in db.execute("SELECT * FROM app_operations WHERE run_id=? ORDER BY rowid", (run_id,))]


def latest_rebinding(run_id):
    with connect() as db:
        row = db.execute("SELECT id,payload FROM app_rebindings WHERE run_id=? ORDER BY id DESC LIMIT 1",
                         (name(run_id),)).fetchone()
    return {"id": row["id"], **json.loads(row["payload"])} if row else None

def effective_target(run_id):
    latest = latest_rebinding(run_id)
    return latest["to"] if latest else get_run(run_id)["envelope"]["target"]

def append_rebinding(run_id, payload):
    with connect() as db:
        cursor = db.execute("INSERT INTO app_rebindings(run_id,payload) VALUES (?,?)", (run_id, canonical(payload)))
        revision = cursor.lastrowid
    record = {"id": revision, **payload}
    export_rebinding(run_id, record)
    return record

def export_rebinding(run_id, record):
    atomic(run_dir(run_id) / "routing" / f"rebind-{record['id']:04d}.json", record, immutable=True)
