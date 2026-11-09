"""Project-scoped session service. Protocol projections never expose registry contents."""
from __future__ import annotations

import argparse
import asyncio
import fcntl
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import struct
import sys
import threading
import time
import tomllib

from .runtime import bind, load, register, selected_path
from .storage import OperationalError, atomic, canonical, digest, identity, lock, read

FRAME_MAX = 1024 * 1024
COMMAND_MAX = 64 * 1024


def _unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key")
        result[key] = value
    return result


def parse_frame(raw):
    if len(raw) > FRAME_MAX or not raw.endswith(b"\n"):
        raise ValueError("oversize or unterminated frame")
    value = json.loads(raw, object_pairs_hook=_unique,
                       parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
    if set(value) != {"v", "kind", "request_id", "body"} or value["v"] != 1 or not isinstance(value["body"], dict):
        raise ValueError("invalid protocol envelope")
    if not isinstance(value["request_id"], str) or not 1 <= len(value["request_id"]) <= 128:
        raise ValueError("invalid request ID")
    return value


def encode(kind, ident, body):
    return (json.dumps({"v": 1, "kind": kind, "request_id": ident, "body": body},
                       ensure_ascii=False, separators=(",", ":"), allow_nan=False) + "\n").encode()


class Store:
    def __init__(self, paths):
        self.paths = paths
        self.file = paths.state / "state.sqlite"
        self.file.parent.mkdir(parents=True, exist_ok=True)
        with self.open() as db:
            db.executescript("""
            CREATE TABLE IF NOT EXISTS session_metadata (name TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_commands (command_id TEXT PRIMARY KEY, body TEXT NOT NULL,
              outcome TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_tasks (task_id TEXT PRIMARY KEY, version INTEGER NOT NULL,
              state_revision INTEGER NOT NULL, status TEXT NOT NULL, title TEXT NOT NULL,
              body TEXT NOT NULL, run_id TEXT);
            DROP INDEX IF EXISTS session_one_workflow;
            CREATE UNIQUE INDEX session_one_workflow
              ON session_tasks((1)) WHERE run_id IS NOT NULL
              AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused');
            CREATE TABLE IF NOT EXISTS session_events (seq INTEGER PRIMARY KEY AUTOINCREMENT,
              schema_version INTEGER NOT NULL, type TEXT NOT NULL, task_id TEXT, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_coordinator_calls (
              call_id TEXT PRIMARY KEY, task_id TEXT NOT NULL UNIQUE,
              packet_sha256 TEXT NOT NULL, status TEXT NOT NULL, result_sha256 TEXT);
            CREATE TABLE IF NOT EXISTS session_proposals (
              task_id TEXT PRIMARY KEY, call_id TEXT NOT NULL, result_sha256 TEXT NOT NULL,
              body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_admissions (
              task_id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE,
              plan_sha256 TEXT NOT NULL, plan_json TEXT NOT NULL);
            """)
            db.execute("INSERT OR IGNORE INTO session_metadata VALUES ('state_revision','0')")
            db.execute("INSERT OR IGNORE INTO session_metadata VALUES ('lifecycle_v1',?)",
                       (canonical({"schema_version": 1, "phase": "running", "generation": 0,
                                   "exit_id": None, "deadline": None, "notice": None}),))

    def open(self):
        db = sqlite3.connect(self.file, timeout=3)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA synchronous=FULL")
        return db

    def snapshot(self):
        with self.open() as db:
            db.execute("BEGIN")
            revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0])
            cursor = db.execute("SELECT coalesce(max(seq),0) FROM session_events").fetchone()[0]
            pending = [dict(row) for row in db.execute(
                "SELECT * FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled') ORDER BY rowid LIMIT 100")]
            recent_done = [dict(row) for row in db.execute(
                "SELECT * FROM session_tasks WHERE status IN ('approved','rejected','failed','cancelled') ORDER BY rowid DESC LIMIT 50")]
            tasks = pending + list(reversed(recent_done))
            recent = [dict(row) for row in db.execute(
                "SELECT seq,type,task_id,body FROM session_events ORDER BY seq DESC LIMIT 200")]
            return {"state_revision": revision, "cursor": cursor, "tasks": tasks,
                    "recent_events": [{**row, "body": json.loads(row["body"])} for row in reversed(recent)],
                    "project_key": self.paths.key, "session": self._phase(db)}

    @staticmethod
    def _phase(db):
        value = json.loads(db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone()[0])
        if value.get("schema_version") != 1 or value.get("phase") not in ("running", "exit-requested", "draining", "paused"):
            raise OperationalError("Unsupported project session lifecycle record; no dispatch")
        return value

    def phase(self):
        with self.open() as db:
            return self._phase(db)

    def lifecycle_replay(self, body):
        command_id=body.get("command_id")
        if not isinstance(command_id,str):
            return None
        with self.open() as db:
            old=db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",(command_id,)).fetchone()
        if old:
            return json.loads(old["outcome"]) if old["body"]==canonical(body) else {
                "status":"rejected","code":"idempotency_conflict"}
        return None

    def lifecycle_command(self, body):
        command_id = body.get("command_id")
        if not isinstance(command_id, str) or not 1 <= len(command_id) <= 128 or body.get("action") not in ("stop_session", "resume_session"):
            return {"status": "rejected", "code": "invalid_request"}
        frozen = canonical(body)
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?", (command_id,)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"] == frozen else {"status": "rejected", "code": "idempotency_conflict"}
            revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0])
            if body.get("expected_state_revision") != revision:
                return {"status": "rejected", "code": "stale_state", "state_revision": revision}
            previous = self._phase(db)
            if body["action"] == "stop_session":
                if previous["phase"] != "running":
                    return {"status": "rejected", "code": "already_paused", "session": previous}
                phase = {"schema_version": 1, "phase": "exit-requested",
                         "generation": previous["generation"] + 1, "exit_id": command_id,
                         "requested_at": time.time(), "deadline": time.time() + 30, "notice": None}
            else:
                if previous["phase"] == "running":
                    return {"status": "rejected", "code": "already_running", "session": previous}
                phase = {"schema_version": 1, "phase": "running",
                         "generation": previous["generation"] + 1, "exit_id": None,
                         "requested_at": None, "deadline": None, "notice": previous.get("notice")}
            db.execute("UPDATE session_metadata SET value=? WHERE name='lifecycle_v1'", (canonical(phase),))
            cursor, next_revision = self._event(db, "session.state_changed", None,
                                                {"phase": phase["phase"], "generation": phase["generation"],
                                                 "exit_id": phase["exit_id"], "supersedes": previous.get("exit_id")})
            outcome = {"status": "accepted", "session": phase, "state_revision": next_revision,
                       "event_seq": cursor}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)", (command_id, frozen, canonical(outcome)))
            return outcome

    def update_exit(self, exit_id, generation, phase_name, notice=None):
        """Compare-and-swap: a resumed generation makes the old drain inert."""
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            previous = self._phase(db)
            if previous["exit_id"] != exit_id or previous["generation"] != generation or previous["phase"] not in ("exit-requested", "draining"):
                return None
            if previous["phase"] == phase_name and previous.get("notice") == notice:
                return previous
            current = {**previous, "phase": phase_name, "notice": notice}
            db.execute("UPDATE session_metadata SET value=? WHERE name='lifecycle_v1'", (canonical(current),))
            self._event(db, "session.state_changed", None,
                        {"phase": phase_name, "generation": generation, "exit_id": exit_id, "notice": notice})
            return current

    def dispatch_allowed(self):
        if self.phase()["phase"] != "running":
            raise OperationalError("Project session exit pause fences new execution; resume explicitly")

    def events(self, after, limit):
        with self.open() as db:
            rows = db.execute("SELECT * FROM session_events WHERE seq>? ORDER BY seq LIMIT ?", (after, limit)).fetchall()
            revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0])
            return {"events": [{**dict(row), "body": json.loads(row["body"])} for row in rows],
                    "next_seq": rows[-1]["seq"] if rows else after, "state_revision": revision}

    def command_get(self, command_id):
        with self.open() as db:
            row = db.execute("SELECT outcome FROM session_commands WHERE command_id=?", (command_id,)).fetchone()
            return json.loads(row[0]) if row else {"status": "not_found"}

    def external_result(self, body):
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            row = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",
                             (body["command_id"],)).fetchone()
            if row:
                return json.loads(row["outcome"]) if row["body"] == frozen else {
                    "status": "rejected", "code": "idempotency_conflict"}
        return None

    def record_external(self, body, outcome):
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",
                             (body["command_id"],)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"] == frozen else {
                    "status": "rejected", "code": "idempotency_conflict"}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)",
                       (body["command_id"], frozen, json.dumps(outcome)))
            return outcome

    def task(self, task_id):
        with self.open() as db:
            row = db.execute("SELECT * FROM session_tasks WHERE task_id=?", (task_id,)).fetchone()
            return dict(row) if row else None

    def active(self):
        with self.open() as db:
            row = db.execute("SELECT * FROM session_tasks WHERE run_id IS NOT NULL AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused') LIMIT 1").fetchone()
            return dict(row) if row else None

    def next_ready(self):
        with self.open() as db:
            row=db.execute("SELECT t.*,a.command_id,a.plan_sha256,a.plan_json FROM session_tasks t JOIN session_admissions a ON a.task_id=t.task_id "
                           "WHERE t.status='ready' ORDER BY t.rowid LIMIT 1").fetchone()
            return dict(row) if row else None

    def run_claimed(self, run_id, task_id):
        with self.open() as db:
            return db.execute("SELECT 1 FROM session_tasks WHERE run_id=? AND task_id<>?",
                              (run_id, task_id)).fetchone() is not None

    def next_coordinator_task(self):
        with self.open() as db:
            row = db.execute("SELECT t.* FROM session_tasks t LEFT JOIN session_coordinator_calls c ON c.task_id=t.task_id "
                             "WHERE t.status='queued' AND c.task_id IS NULL ORDER BY t.rowid LIMIT 1").fetchone()
            return dict(row) if row else None

    def pending_coordinator_call(self):
        with self.open() as db:
            row = db.execute("SELECT * FROM session_coordinator_calls WHERE status='reserved' ORDER BY rowid LIMIT 1").fetchone()
            return dict(row) if row else None

    def reserve_coordinator(self, task, packet_hash):
        call_id = f"c-{task['task_id']}-v{task['version']}"
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            latest = db.execute("SELECT status,version,body FROM session_tasks WHERE task_id=?",
                                (task["task_id"],)).fetchone()
            if not latest or latest["status"] != "queued" or latest["version"] != task["version"] or latest["body"] != task["body"]:
                raise OperationalError("Coordinator proposal input became stale before reservation")
            old = db.execute("SELECT * FROM session_coordinator_calls WHERE task_id=?",(task["task_id"],)).fetchone()
            if old:
                if old["packet_sha256"] != packet_hash:
                    raise OperationalError("Coordinator input changed after reservation")
                return dict(old)
            if self._phase(db)["phase"] != "running":
                raise OperationalError("Project session exit pause fences coordinator reservation")
            # This proof admits one coordinator call per task. There is no
            # installation-wide or lifetime call limit shared by projects.
            db.execute("INSERT INTO session_coordinator_calls VALUES (?,?,?,'reserved',NULL)",(call_id,task["task_id"],packet_hash))
            return {"call_id":call_id,"task_id":task["task_id"],"packet_sha256":packet_hash,"status":"reserved"}

    def proposal(self, task_id):
        with self.open() as db:
            row=db.execute("SELECT body,result_sha256 FROM session_proposals WHERE task_id=?",(task_id,)).fetchone()
            return {"body":json.loads(row["body"]),"result_sha256":row["result_sha256"]} if row else None

    def publish_proposal(self, task_id, call_id, result, result_hash):
        proposal=result["proposal"]
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            prior=db.execute("SELECT result_sha256 FROM session_proposals WHERE task_id=?",(task_id,)).fetchone()
            if prior:
                if prior["result_sha256"] != result_hash:
                    raise OperationalError("Coordinator proposal changed after publication")
                return
            task=db.execute("SELECT * FROM session_tasks WHERE task_id=?",(task_id,)).fetchone()
            if not task or task["version"] != proposal["task_version"] or digest(task["body"]) != proposal["message_sha256"]:
                raise OperationalError("Coordinator proposal became stale before publication")
            db.execute("INSERT INTO session_proposals VALUES (?,?,?,?)",(task_id,call_id,result_hash,json.dumps(proposal)))
            db.execute("UPDATE session_coordinator_calls SET status='completed',result_sha256=? WHERE call_id=?",(result_hash,call_id))
            db.execute("UPDATE session_tasks SET status='proposed',state_revision=state_revision+1 WHERE task_id=?",(task_id,))
            self._event(db,"proposal.ready",task_id,{"task_id":task_id,"call_id":call_id,"result_sha256":result_hash,"status":"proposed"})

    def stop_coordinator(self, task_id, call_id, status, detail):
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            db.execute("UPDATE session_coordinator_calls SET status=? WHERE call_id=?",(status,call_id))
            db.execute("UPDATE session_tasks SET status='blocked',state_revision=state_revision+1 WHERE task_id=?",(task_id,))
            self._event(db,"task.state_changed",task_id,{"status":"blocked","detail":detail,"coordinator_execution":status})

    def reopen_prepermit_coordinator(self, body, task_id):
        """Record one explicit, evidence-verified replacement of a fake pre-permit runner."""
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            prior = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",
                               (body["command_id"],)).fetchone()
            if prior:
                return json.loads(prior["outcome"]) if prior["body"] == frozen else {"status":"rejected","code":"idempotency_conflict"}
            task = db.execute("SELECT * FROM session_tasks WHERE task_id=?", (task_id,)).fetchone()
            call = db.execute("SELECT * FROM session_coordinator_calls WHERE task_id=?", (task_id,)).fetchone()
            revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0])
            if (not task or not call or task["status"] != "blocked" or call["status"] != "uncertain" or
                body.get("expected_task_version") != task["version"] or body.get("expected_state_revision") != revision):
                return {"status":"rejected","code":"stale_state","state_revision":revision}
            if self._phase(db)["phase"] != "running":
                return {"status":"rejected","code":"session_paused"}
            db.execute("UPDATE session_coordinator_calls SET status='reserved' WHERE task_id=?",(task_id,))
            db.execute("UPDATE session_tasks SET status='queued',state_revision=state_revision+1 WHERE task_id=?",(task_id,))
            cursor, next_revision=self._event(db,"task.recovery_started",task_id,
                {"status":"queued","reason":"verified pre-permit fake coordinator exit",
                 "original_failure_preserved":True})
            outcome={"status":"accepted","task_id":task_id,"state_revision":next_revision,"event_seq":cursor}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)",(body["command_id"],frozen,canonical(outcome)))
            return outcome

    def _event(self, db, event_type, task_id, content):
        cursor = db.execute("INSERT INTO session_events(schema_version,type,task_id,body) VALUES (1,?,?,?)",
                            (event_type, task_id, json.dumps(content))).lastrowid
        revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0]) + 1
        db.execute("UPDATE session_metadata SET value=? WHERE name='state_revision'", (str(revision),))
        return cursor, revision

    def command(self, body):
        command_id = body.get("command_id")
        if not isinstance(command_id, str) or not 1 <= len(command_id) <= 128:
            return {"status": "rejected", "code": "invalid_request"}
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?", (command_id,)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"] == frozen else {"status": "rejected", "code": "idempotency_conflict"}
            revision = int(db.execute("SELECT value FROM session_metadata WHERE name='state_revision'").fetchone()[0])
            if body.get("expected_state_revision") != revision:
                return {"status": "rejected", "code": "stale_state", "state_revision": revision}
            if body.get("action") != "submit_message" or not isinstance(body.get("payload"), dict):
                return {"status": "rejected", "code": "invalid_request"}
            if db.execute("SELECT count(*) FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled')").fetchone()[0] >= 100:
                return {"status":"rejected","code":"queue_full","message":"This project already has 100 unresolved tasks"}
            text = body["payload"].get("text")
            if not isinstance(text, str) or not text.strip() or len(text.encode()) > 32 * 1024:
                return {"status": "rejected", "code": "invalid_request"}
            task_id = "t-" + hashlib.sha256(command_id.encode()).hexdigest()[:24]
            db.execute("INSERT INTO session_tasks VALUES (?,?,1,?,?,?,NULL)", (task_id, 1, "queued", text.splitlines()[0][:100], text))
            event = {"task_id": task_id, "version": 1, "status": "queued", "text": text}
            cursor, next_revision = self._event(db, "task.queued", task_id, event)
            outcome = {"status": "accepted", "task_id": task_id, "event_seq": cursor, "state_revision": next_revision}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)", (command_id, frozen, json.dumps(outcome)))
            return outcome

    def admit(self, body, task_id, plan):
        command_id = body["command_id"]
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?", (command_id,)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"] == frozen else {"status": "rejected", "code": "idempotency_conflict"}
            row = db.execute("SELECT * FROM session_tasks WHERE task_id=?", (task_id,)).fetchone()
            if not row:
                return {"status": "rejected", "code": "not_found"}
            if row["status"] != "proposed" or row["version"] != body.get("expected_task_version"):
                return {"status": "rejected", "code": "stale_state", "task_version": row["version"], "task_status": row["status"]}
            if row["state_revision"] != body.get("expected_state_revision"):
                return {"status": "rejected", "code": "stale_state", "task_state_revision": row["state_revision"]}
            frozen_plan=canonical(plan)
            plan_hash=digest(frozen_plan)
            db.execute("INSERT INTO session_admissions VALUES (?,?,?,?)",(task_id,command_id,plan_hash,frozen_plan))
            db.execute("UPDATE session_tasks SET status='ready',state_revision=state_revision+1 WHERE task_id=?",(task_id,))
            cursor,next_revision=self._event(db,"task.authorized",task_id,{"task_id":task_id,"run_id":plan["run_id"],"status":"ready","plan_sha256":plan_hash})
            outcome={"status":"accepted","task_id":task_id,"run_id":plan["run_id"],"task_status":"ready","event_seq":cursor,"state_revision":next_revision}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)",(command_id,frozen,json.dumps(outcome)))
            return outcome

    def activate(self, task_id, run_id):
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            if self._phase(db)["phase"] != "running":
                raise OperationalError("Project session exit pause fences task activation")
            row=db.execute("SELECT status,run_id FROM session_tasks WHERE task_id=?",(task_id,)).fetchone()
            if not row or row["status"] != "ready" or row["run_id"] is not None:
                raise OperationalError("Authorized task is no longer ready")
            if db.execute("SELECT 1 FROM session_tasks WHERE run_id IS NOT NULL AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused')").fetchone():
                raise OperationalError("Another workflow in this project is unresolved")
            db.execute("UPDATE session_tasks SET status='active',run_id=?,state_revision=state_revision+1 WHERE task_id=?",(run_id,task_id))
            self._event(db,"task.active",task_id,{"task_id":task_id,"run_id":run_id,"status":"active"})

    def transition(self, task_id, run_id, status, detail=None):
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            row=db.execute("SELECT status,run_id FROM session_tasks WHERE task_id=?",(task_id,)).fetchone()
            if not row or row["run_id"] != run_id:
                raise OperationalError("Driver task/run binding changed")
            if row["status"] == status:
                return
            db.execute("UPDATE session_tasks SET status=?,state_revision=state_revision+1 WHERE task_id=?",(status,task_id))
            self._event(db,"task.state_changed",task_id,{"task_id":task_id,"run_id":run_id,"status":status,"detail":detail})

    def modify_queued(self, body):
        command_id = body.get("command_id")
        payload = body.get("payload")
        if not isinstance(command_id, str) or not isinstance(payload, dict):
            return {"status":"rejected","code":"invalid_request"}
        replacement = payload.get("text")
        if not isinstance(replacement, str) or not replacement.strip() or len(replacement.encode()) > 32*1024:
            return {"status":"rejected","code":"invalid_request"}
        frozen = json.dumps(body, sort_keys=True, separators=(",", ":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old = db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",(command_id,)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"] == frozen else {"status":"rejected","code":"idempotency_conflict"}
            row = db.execute("SELECT * FROM session_tasks WHERE task_id=?",(payload.get("task_id"),)).fetchone()
            if (not row or row["status"] != "queued" or row["version"] != body.get("expected_task_version")
                or row["state_revision"] != body.get("expected_state_revision")
                or db.execute("SELECT 1 FROM session_coordinator_calls WHERE task_id=?",(payload.get("task_id"),)).fetchone()):
                return {"status":"rejected","code":"stale_or_started","message":"Steering requires a queued task before coordinator reservation; frozen work needs a new authorized task"}
            version=row["version"]+1
            db.execute("UPDATE session_tasks SET version=?,state_revision=state_revision+1,title=?,body=? WHERE task_id=?",
                       (version,replacement.splitlines()[0][:100],replacement,row["task_id"]))
            seq,revision=self._event(db,"task.steered",row["task_id"],{"task_id":row["task_id"],"version":version,"text":replacement})
            outcome={"status":"accepted","task_id":row["task_id"],"task_version":version,"state_revision":revision,"event_seq":seq}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)",(command_id,frozen,json.dumps(outcome)))
            return outcome

    def cancel_undispatched(self, body):
        command_id=body.get("command_id")
        payload=body.get("payload")
        if not isinstance(command_id,str) or not isinstance(payload,dict):
            return {"status":"rejected","code":"invalid_request"}
        frozen=json.dumps(body,sort_keys=True,separators=(",",":"))
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            old=db.execute("SELECT body,outcome FROM session_commands WHERE command_id=?",(command_id,)).fetchone()
            if old:
                return json.loads(old["outcome"]) if old["body"]==frozen else {"status":"rejected","code":"idempotency_conflict"}
            row=db.execute("SELECT * FROM session_tasks WHERE task_id=?",(payload.get("task_id"),)).fetchone()
            if (not row or row["status"] not in ("queued","proposed","ready") or row["run_id"] is not None
                or row["version"]!=body.get("expected_task_version")
                or row["state_revision"]!=body.get("expected_state_revision")):
                return {"status":"rejected","code":"stale_or_started"}
            call=db.execute("SELECT status FROM session_coordinator_calls WHERE task_id=?",(row["task_id"],)).fetchone()
            if call and call["status"]=="reserved":
                return {"status":"rejected","code":"coordinator_active","message":"Wait for the visible coordinator call to resolve before cancelling"}
            db.execute("UPDATE session_tasks SET status='cancelled',state_revision=state_revision+1 WHERE task_id=?",(row["task_id"],))
            seq,revision=self._event(db,"task.state_changed",row["task_id"],{"task_id":row["task_id"],"status":"cancelled","detail":"Cancelled before workflow dispatch"})
            outcome={"status":"accepted","task_id":row["task_id"],"task_status":"cancelled","event_seq":seq,"state_revision":revision}
            db.execute("INSERT INTO session_commands VALUES (?,?,?)",(command_id,frozen,json.dumps(outcome)))
            return outcome

    def block_unbound(self, task_id, run_id, detail):
        with self.open() as db:
            db.execute("BEGIN IMMEDIATE")
            row = db.execute("SELECT status,run_id FROM session_tasks WHERE task_id=?", (task_id,)).fetchone()
            if row and row["status"] in ("queued", "proposed", "ready") and row["run_id"] is None:
                db.execute("UPDATE session_tasks SET status='blocked',run_id=?,state_revision=state_revision+1 WHERE task_id=?",(run_id,task_id))
                self._event(db,"task.state_changed",task_id,{"task_id":task_id,"run_id":run_id,"status":"blocked","detail":detail})


class Service:
    def __init__(self, paths, ipc, herdr_socket, release_id, frontend, generation=1):
        self.paths = paths
        self.ipc = Path(ipc)
        self.herdr_socket = herdr_socket
        self.release_id = release_id
        self.frontend = frontend
        self.generation = generation
        self.store = Store(paths)
        self.connections = asyncio.Semaphore(8)
        self.driver_lock = threading.Lock()
        self.driver_thread = None
        self.driver_task = None
        self.driver_pending = None
        self.coordinator_lock = threading.Lock()
        self.coordinator_thread = None
        self.promotion_lock = threading.Lock()
        self.promotion_thread = None
        self.promotion_again = False
        self.drain_thread = None
        self.drain_lock = threading.Lock()
        self.shutdown_event = None
        self.event_loop = None
        self.shutting_down = False
        self.active_writers = set()

    def checkpoint(self):
        """Rebuildable, bounded navigation projection; authoritative ledgers stay in SQLite."""
        snapshot = self.store.snapshot()
        entries = []
        pending_questions = []
        with self.store.open() as db:
            if db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_responses'").fetchone():
                rows = db.execute("SELECT run_id,request_id,payload FROM app_responses ORDER BY rowid DESC LIMIT 100").fetchall()
                for row in rows:
                    saved = json.loads(row["payload"])
                    entries.append({"run_id": row["run_id"], "request_id": row["request_id"],
                                    "outcome": saved.get("outcome"), "answer": saved.get("answer") if len(str(saved.get("answer", ""))) <= 512 else None,
                                    "artifact_sha256": saved.get("artifact_sha256"),
                                    "record_sha256": digest(row["payload"]),
                                    "record": f"runs/{row['run_id']}/artifacts/decision-{row['request_id']}.json"})
            if (db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_requests'").fetchone()
                and db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_responses'").fetchone()):
                rows = db.execute("SELECT r.run_id,r.request_id,r.payload FROM app_requests r "
                                  "LEFT JOIN app_responses a ON a.run_id=r.run_id AND a.request_id=r.request_id "
                                  "WHERE a.request_id IS NULL ORDER BY r.rowid DESC LIMIT 100").fetchall()
                pending_questions = [{"run_id": row["run_id"], "request_id": row["request_id"],
                                      "request_sha256": digest(row["payload"])} for row in rows]
        tasks = [{"task_id": row["task_id"], "status": row["status"], "run_id": row["run_id"],
                  "title": row["title"][:120], "version": row["version"]} for row in snapshot["tasks"]]
        context_refs = []
        budget_refs = []
        with self.store.open() as db:
            for row in db.execute("SELECT task_id,plan_json,plan_sha256 FROM session_admissions ORDER BY rowid DESC LIMIT 100"):
                plan=json.loads(row["plan_json"])
                context_refs.append({"task_id":row["task_id"],"plan_sha256":row["plan_sha256"],
                                     "selection":plan.get("context"),
                                     "selection_sha256":plan.get("context_snapshot_sha256"),
                                     "grants":[{"grant_id":item["grant_id"],"item_sha256":item["item_sha256"]}
                                               for item in plan.get("cross_imports",[])]})
                budget_refs.append({"task_id":row["task_id"],"run_id":plan.get("run_id"),
                                    "max_worker_invocations":plan.get("max_worker_invocations"),
                                    "review_limit":plan.get("review_limit")})
        open_findings = []
        from .runtime import unbind
        token=bind(self.paths)
        try:
            from .workflow import state as graph_state
            for run_id in {row["run_id"] for row in snapshot["tasks"] if row["run_id"]}:
                try:
                    values=graph_state(run_id).values
                    for finding_id, finding in values.get("findings",{}).items():
                        if finding.get("status")=="open":
                            open_findings.append({"run_id":run_id,"finding_id":finding_id,
                                                  "evidence_sha256":digest(canonical(finding))})
                except (OperationalError,OSError,ValueError,KeyError,sqlite3.Error):
                    pass  # canonical graph/records remain authoritative
        finally:
            unbind(token)
        artifact_refs = []
        for run_id in {row["run_id"] for row in snapshot["tasks"] if row["run_id"]}:
            for path in sorted((self.paths.state / "runs" / run_id / "artifacts").glob("*.md")):
                if path.name.startswith(("draft", "critique", "revision")):
                    artifact_refs.append({"run_id": run_id, "path": str(path.relative_to(self.paths.state)),
                                          "sha256": digest(path.read_bytes())})
        projection = {"schema_version": 1, "project_key": self.paths.key, "release_id": self.release_id,
                      "state_revision": snapshot["state_revision"], "event_cursor": snapshot["cursor"],
                      "session": snapshot["session"], "tasks": tasks, "decision_refs": entries,
                      "pending_questions": pending_questions, "artifact_refs": artifact_refs,
                      "open_findings": open_findings, "context_refs": context_refs,
                      "budget_refs": budget_refs,
                      "finding_state_ref": "selected project's LangGraph checkpoint and versioned finding records",
                      "next_actions": ["Inspect pending task questions and reviews", "Resume explicitly if exited"]}
        if len(canonical(projection).encode()) > 32768:
            projection["decision_refs"] = [{key: item[key] for key in ("run_id", "request_id", "record_sha256", "record")}
                                           for item in entries]
            projection["tasks"] = [{key: item[key] for key in ("task_id", "status", "run_id")} for item in tasks]
            projection["artifact_refs"] = []
            projection["artifact_query"] = "runs/<run-id>/artifacts in selected project state"
            projection["open_findings"] = []
            projection["context_refs"] = []
            projection["budget_refs"] = []
            projection["context_query"] = "session_admissions and selected project's run envelopes"
        if len(canonical(projection).encode()) > 32768:
            projection["tasks"] = []
            projection["task_query"] = "session_tasks in selected project's state.sqlite"
            projection["decision_refs"] = []
            projection["decision_query"] = "app_responses in selected project's state.sqlite"
        content = canonical(projection)
        reference = f"{snapshot['state_revision']}-{digest(content)[:16]}.json"
        root = self.paths.state / "session/checkpoints"
        atomic(root / reference, projection, immutable=True)
        atomic(root / "latest.json", {"schema_version": 1, "project_key": self.paths.key,
                                      "state_revision": snapshot["state_revision"], "file": reference,
                                      "sha256": digest(content)})
        return {"path": str(root / reference), "sha256": digest(content), "state_revision": snapshot["state_revision"]}

    def lifecycle_command(self, body):
        with lock(self.paths.state / "session/dispatch.lock"):
            prior=self.store.lifecycle_replay(body)
            if prior is not None:
                return prior
            if self.shutting_down:
                return {"status": "rejected", "code": "service_stopping",
                        "message": "Project service is closing; reconnect and replay this command ID"}
            if body.get("action") == "resume_session":
                issue = self.reconcile_before_resume()
                if issue:
                    return {"status": "rejected", "code": "operation_uncertain", "message": issue,
                            "operator_action": "Inspect this project's retained operation records and resolve execution before resuming"}
            outcome = self.store.lifecycle_command(body)
        if outcome.get("status") == "accepted":
            self.checkpoint()
            if body["action"] == "stop_session" and self.store.phase()["exit_id"] == body["command_id"]:
                self.start_drain()
            elif body["action"] == "resume_session" and self.store.phase()["phase"] == "running":
                self.resume_active()
                self.start_promotion()
                self.start_coordinator()
        return outcome

    def reconcile_before_resume(self):
        from .herdr_backend import inspect_operation, receipt
        from .storage import Uncertain, OutputInvalid
        roots = [self.paths.state / "runs", self.paths.state / "session/coordinator"]
        candidates = list(roots[0].glob("*/operations/*")) + list(roots[1].glob("*/call-v*"))
        for directory in candidates:
            if not (directory / "operation.json").exists():
                continue
            if (directory / "receipt.json").exists():
                try:
                    receipt(directory)
                except (Uncertain, OutputInvalid, OperationalError, OSError, ValueError) as error:
                    return f"{directory}: saved completion evidence invalid: {error}"
                continue
            if any((directory / name).exists() for name in ("launch.json", "claim.json", "permit.json")):
                report = inspect_operation(directory)
                if report["status"] == "uncertain":
                    return f"{directory}: {report['detail']}"
        return None

    def start_drain(self):
        with self.drain_lock:
            if self.drain_thread and self.drain_thread.is_alive():
                return
            self.drain_thread = threading.Thread(target=self._drain_loop, daemon=True)
            self.drain_thread.start()

    def _inflight(self):
        from .herdr_backend import inspect_operation
        operations = []
        selected = self.store.active()
        if selected and selected["run_id"]:
            for directory in sorted((self.paths.state / "runs" / selected["run_id"] / "operations").glob("*")):
                if (directory / "permit.json").exists() and not (directory / "receipt.json").exists():
                    operations.append({"operation": str(directory), **inspect_operation(directory)})
        for directory in (self.paths.state / "session/coordinator").glob("*/call-v*"):
            if (directory / "permit.json").exists() and not (directory / "receipt.json").exists():
                operations.append({"operation": str(directory), **inspect_operation(directory)})
        return operations

    def _drain_loop(self):
        while True:
            phase = self.store.phase()
            if phase["phase"] not in ("exit-requested", "draining"):
                return
            exit_id, generation = phase["exit_id"], phase["generation"]
            operations = self._inflight()
            ambiguous = [item for item in operations if item["status"] == "uncertain"]
            if ambiguous:
                notice = {"kind": "execution-uncertain", "reason": "An existing operation cannot be verified",
                          "operations": ambiguous, "operator_action":
                          "Inspect saved claim, permit, child, receipt and Herdr identity; resolve this operation explicitly before resume. No retry is authorized."}
                if phase.get("notice") != notice or phase["phase"] != "draining":
                    self.store.update_exit(exit_id, generation, "draining", notice)
                    self.checkpoint()
                return  # keep service responsive; restart reevaluates the same evidence
            threads_busy = any(thread and thread.is_alive() for thread in
                               (self.driver_thread, self.coordinator_thread, self.promotion_thread))
            if operations or threads_busy:
                notice = None
                if time.time() >= phase["deadline"]:
                    notice = {"kind": "attention-required", "reason": "Foreground drain deadline passed; verified call remains running or transition is stalled",
                              "operations": operations,
                              "driver_alive": bool(self.driver_thread and self.driver_thread.is_alive()),
                              "coordinator_alive": bool(self.coordinator_thread and self.coordinator_thread.is_alive()),
                              "operator_action":
                              "Inspect these project-owned operation records and panes; do not retry uncertain work. Resume may continue only after reconciliation."}
                if phase["phase"] != "draining" or notice != phase.get("notice"):
                    self.store.update_exit(exit_id, generation, "draining", notice)
                    self.checkpoint()
                time.sleep(.5)
                continue
            if time.time() < phase.get("requested_at", 0) + 1:
                time.sleep(.1)  # let the accepted command reply reach its foreground UI
                continue
            with lock(self.paths.state / "session/dispatch.lock"):
                if self.store.phase().get("generation") != generation:
                    return
                done = self.store.update_exit(exit_id, generation, "paused", phase.get("notice"))
                if done is None:
                    return
                self.checkpoint()
                # While this flag is set a concurrent client gets a retryable
                # service-stopping response; it cannot commit resume to a dying owner.
                self.shutting_down = True
                atomic(self.paths.state / "session/shutdown-intents" / f"{generation}-{exit_id}.json",
                       {"schema_version": 1, "exit_id": exit_id, "generation": generation,
                        "service_generation": self.generation,
                        "loop_ready": self.event_loop is not None,
                        "event_ready": self.shutdown_event is not None}, immutable=True)
                if self.event_loop is not None and self.shutdown_event is not None:
                    self.event_loop.call_soon_threadsafe(self.shutdown_event.set)
                return

    def plan(self, payload):
        task = self.store.task(payload.get("task_id"))
        if not task or task["status"] != "proposed":
            raise OperationalError("Task has no validated coordinator proposal")
        proposal = self.store.proposal(task["task_id"])
        if not proposal or proposal["body"]["task_version"] != task["version"] or proposal["body"]["message_sha256"] != digest(task["body"]):
            raise OperationalError("Coordinator proposal is missing or stale")
        proposed_brief = proposal["body"]["brief"]
        profile = selected_path(payload.get("profile", ".wayward-wrangler/workflow.toml"), self.paths.project)
        if not profile.is_relative_to(self.paths.project) or not profile.is_file():
            raise OperationalError("Workflow profile must be a project-local file")
        config = tomllib.loads(profile.read_text())
        if config.get("review_limit") != 2 or any(role.get("provider") != "fake" for role in config.get("roles", {}).values()):
            raise OperationalError("Session proof permits only the two-role simulated profile")
        from .herdr_backend import Herdr
        workspace = payload.get("workspace")
        if workspace is None:
            pointer = self.paths.state / "session/ui-current.json"
            if pointer.exists():
                workspace = read(pointer)["pane"]["workspace_id"]
        if not isinstance(workspace, str):
            raise OperationalError("An explicit owned workspace is required")
        pointer = self.paths.state / "session/ui-current.json"
        if not pointer.exists() or read(pointer)["pane"]["workspace_id"] != workspace:
            raise OperationalError("Worker workspace is not bound to this project UI")
        Herdr(self.herdr_socket).call("workspace.get", {"workspace_id": workspace})
        run_id = payload.get("run_id") or "r-" + task["task_id"]
        from .records import name
        name(run_id)
        plan = {"schema_version": 1, "project_key": self.paths.key,
                "task_id": task["task_id"], "task_version": task["version"],
                "task_state_revision": task["state_revision"], "run_id": run_id,
                "brief": proposed_brief, "brief_sha256": digest(proposed_brief),
                "source_message_sha256": digest(task["body"]), "profile": str(profile.relative_to(self.paths.project)),
                "profile_sha256": digest(profile.read_bytes()), "workspace": workspace,
                "herdr_socket": self.herdr_socket, "config": config,
                "proposal_result_sha256": proposal["result_sha256"],
                "max_worker_invocations": 5, "review_limit": 2}
        if config.get("workflow_version",1)==2:
            from .context import resolve
            selection=selected_path(payload.get("context", ".wayward-wrangler/context/selection.toml"), self.paths.project)
            if not selection.is_relative_to(self.paths.project) or not selection.is_file():
                raise OperationalError("Version-2 context selection must be an existing project-local file")
            snapshot=resolve(selection)
            plan.update(context=str(selection.relative_to(self.paths.project)),
                        context_snapshot_sha256=digest(canonical(snapshot)),
                        context_preview=[{"kind":item["kind"],"qualified_id":item["qualified_id"],
                                          "source_record_digest":item["source_record_digest"],
                                          "artifact_sha256":item.get("artifact_sha256")}
                                         for item in snapshot["entries"]])
        from .context_access import active_for_task
        grants=active_for_task(self.paths,task["task_id"])
        if grants:
            plan["cross_imports"]=[{"grant_id":grant["grant_id"],
                                    "item_sha256":digest(canonical(grant["item"])),
                                    "purpose":grant["purpose"],"item":grant["item"]}
                                   for grant in grants]
            if len(canonical(plan["cross_imports"]).encode()) + len(canonical(plan.get("context_preview",[])).encode()) > 48*1024:
                raise OperationalError("Combined selected context exceeds 48 KiB")
        return plan

    def task_detail(self, task_id):
        task = self.store.task(task_id)
        if not task:
            raise OperationalError("Task not found in this project")
        detail = {"task": task, "proposal": self.store.proposal(task_id)}
        with self.store.open() as db:
            call = db.execute("SELECT * FROM session_coordinator_calls WHERE task_id=?", (task_id,)).fetchone()
        if call:
            from .herdr_backend import inspect_operation
            directory = self.paths.state / "session/coordinator" / task_id / f"call-v{task['version']}"
            detail["coordinator"] = {"call": dict(call), "operation_dir": str(directory),
                                      "evidence": inspect_operation(directory) if (directory / "operation.json").exists() else None,
                                      "launch_recorded": (directory / "launch.json").exists(),
                                      "claim_recorded": (directory / "claim.json").exists(),
                                      "permit_recorded": (directory / "permit.json").exists(),
                                      "child_recorded": (directory / "child.json").exists(),
                                      "receipt_recorded": (directory / "receipt.json").exists()}
            recovery = directory / "recovery.json"
            if recovery.exists(): detail["coordinator"]["recovery"] = read(recovery)
            elif task["status"] == "blocked":
                from .coordinator_recovery import verify
                try:
                    verify(self.paths, task, dict(call))
                    detail["coordinator"]["supported_action"] = (
                        f"waywr --project {self.paths.project} session recover-coordinator {task_id}")
                except (OperationalError, OSError, KeyError, ValueError, TypeError):
                    detail["coordinator"]["supported_action"] = "No automatic retry; inspect the original execution evidence"
        if task["run_id"]:
            from . import records, workflow
            record = records.get_run(task["run_id"])
            detail["run"] = {"run_id": task["run_id"], "status": record["status"], "detail": record["detail"]}
            try:
                state = workflow.state(task["run_id"])
                pending = state.values.get("pending")
                detail["pending"] = pending
                detail["current"] = state.values.get("current")
                if detail["current"]:
                    from .artifacts import content
                    content(task["run_id"], detail["current"])
                    detail["artifact_path"] = str(records.run_dir(task["run_id"]) / detail["current"]["path"])
                detail["completed"] = state.values.get("completed", [])
                detail["findings"] = state.values.get("findings", {})
            except (OperationalError, ValueError):
                detail["pending"] = None
        return detail

    def answer_task(self, body):
        from . import records, workflow
        payload = body.get("payload")
        command_id = body.get("command_id")
        if not isinstance(command_id, str) or not 1 <= len(command_id) <= 128 or not isinstance(payload, dict):
            return {"status": "rejected", "code": "invalid_request"}
        prior = self.store.external_result(body)
        if prior is not None:
            return prior
        task = self.store.task(payload.get("task_id"))
        if not task or not task["run_id"] or body.get("expected_task_version") != task["version"]:
            return {"status": "rejected", "code": "stale_state"}
        request_id = payload.get("request_id")
        if not isinstance(request_id, str):
            return {"status": "rejected", "code": "invalid_request"}
        old = records.response(task["run_id"], request_id)
        if old is None and (task["status"] != "awaiting-input" or
                            body.get("expected_state_revision") != task["state_revision"]):
            return {"status": "rejected", "code": "stale_state"}
        try:
            response = workflow.answer(task["run_id"], request_id, payload.get("outcome"),
                                       payload.get("answer"), payload.get("artifact_sha256"))
            outcome = {"status": "accepted", "task_id": task["task_id"], "run_id": task["run_id"],
                       "request_id": request_id, "response": response}
            saved = self.store.record_external(body, outcome)
            self.resume_answered_gate(task["task_id"], task["run_id"], request_id)
            return saved
        except (OperationalError, OSError, ValueError, TypeError) as error:
            return {"status": "rejected", "code": "invalid_request", "message": str(error)}

    def cancel_task(self, body):
        from . import records
        from .herdr_backend import cancel, receipt
        payload=body.get("payload")
        if not isinstance(body.get("command_id"),str) or not isinstance(payload,dict):
            return {"status":"rejected","code":"invalid_request"}
        prior=self.store.external_result(body)
        if prior is not None:return prior
        task=self.store.task(payload.get("task_id"))
        if not task or task["version"]!=body.get("expected_task_version") or task["state_revision"]!=body.get("expected_state_revision"):
            return {"status":"rejected","code":"stale_state"}
        if task["status"] in ("queued","proposed","ready"):
            with lock(self.paths.state / "session/admission.lock"):
                return self.store.cancel_undispatched(body)
        if task["status"] == "awaiting-input":
            with lock(self.paths.state / "session/admission.lock"):
                if self.store.task(task["task_id"])["status"] != "awaiting-input":
                    return {"status":"rejected","code":"stale_state"}
                records.set_status(task["run_id"],"cancelled","Operator abandoned pending task without answering its gate")
                self.store.transition(task["task_id"],task["run_id"],"cancelled","Abandoned pending human gate")
                result=self.store.record_external(body,{"status":"accepted","task_id":task["task_id"],"task_status":"cancelled"})
            self.start_promotion()
            return result
        if task["status"] == "active":
            operations=records.operations(task["run_id"])
            if not operations:
                return {"status":"rejected","code":"not_cancellable_now","message":"No worker claim exists yet; retry after stage dispatch or wait for the next gate"}
            operation=operations[-1]
            directory=records.run_dir(task["run_id"]) / "operations" / operation["stage"]
            try:
                if receipt(directory):
                    return {"status":"rejected","code":"completed_race","message":"Worker already completed; inspect task status before another action"}
                result=cancel(directory,source=f"project-session:{self.paths.key}:{task['task_id']}")
            except (OperationalError,OSError,ValueError) as error:
                return {"status":"rejected","code":"uncertain","message":str(error)}
            return self.store.record_external(body,{"status":"accepted","task_id":task["task_id"],"worker":result})
        return {"status":"rejected","code":"not_cancellable_now","message":"This task is already stopped or terminal"}

    def grant_context(self, body):
        with lock(self.paths.state / "session/admission.lock"):
            return self._grant_context_locked(body)

    def _grant_context_locked(self, body):
        from .context_access import active_for_task, grant_path, select, source_binding
        payload=body.get("payload")
        command_id=body.get("command_id")
        if not isinstance(command_id,str) or not 1<=len(command_id)<=128 or not isinstance(payload,dict):
            return {"status":"rejected","code":"invalid_request"}
        prior=self.store.external_result(body)
        if prior is not None:return prior
        grant_id="g-"+digest(command_id)[:24]
        existing=grant_path(self.paths,grant_id)
        if existing.exists():
            grant=read(existing)
            if (grant.get("authorization_command_id")!=command_id or
                grant.get("command_sha256")!=digest(canonical(body))):
                return {"status":"rejected","code":"idempotency_conflict"}
            item=grant["item"]
            return self.store.record_external(body,{"status":"accepted","grant_id":grant_id,
                "task_id":grant["task_id"],"source_project_key":grant["source_project_key"],
                "qualified_id":item["qualified_id"],"item_sha256":digest(canonical(item))})
        task=self.store.task(payload.get("task_id"))
        if (not task or task["status"]!="proposed" or task["version"]!=body.get("expected_task_version")
            or task["state_revision"]!=body.get("expected_state_revision")):
            return {"status":"rejected","code":"stale_state"}
        purpose=payload.get("purpose")
        if not isinstance(purpose,str) or not purpose.strip() or len(purpose)>500:
            return {"status":"rejected","code":"invalid_request","message":"A concise explicit purpose is required"}
        mode=payload.get("dependency_mode","pinned")
        if mode not in ("pinned","must_match_source"):
            return {"status":"rejected","code":"invalid_request","message":"Unknown dependency mode"}
        try:
            if len(active_for_task(self.paths,task["task_id"]))>=8:
                raise OperationalError("At most eight explicit foreign items per task")
            if not isinstance(payload.get("source_project"),str) or not Path(payload["source_project"]).is_absolute():
                raise OperationalError("Cross-project source must be an explicit absolute project path")
            binding=source_binding(payload.get("source_project"),payload.get("source_state"))
            if binding["project_key"]==self.paths.key:
                raise OperationalError("Use ordinary same-project context selection for this project")
            source_release=Path(binding["project"])/".wayward-wrangler/release.json"
            if source_release.exists() and read(source_release).get("release_id")!=self.release_id:
                raise OperationalError("Source project is bound to another release; a compatible read-only reader is required")
            selector={"run":payload.get("run"),"kind":payload.get("kind"),"id":payload.get("id")}
            item=select(binding,selector)
            grant={"schema_version":1,"grant_id":grant_id,"revision":1,"recipient_project_key":self.paths.key,
                   "task_id":task["task_id"],"direction":"source-to-recipient-read-only",
                   "source_project_key":binding["project_key"],"source_project":binding["project"],
                   "source_state":binding["state"],"purpose":purpose,"scope":"task",
                   "selector":selector,"dependency_mode":mode,"item":item,
                   "authorization_command_id":command_id,"command_sha256":digest(canonical(body))}
            atomic(grant_path(self.paths,grant_id),grant,immutable=True)
            return self.store.record_external(body,{"status":"accepted","grant_id":grant_id,
                                                    "task_id":task["task_id"],"source_project_key":binding["project_key"],
                                                    "qualified_id":item["qualified_id"],"item_sha256":digest(canonical(item))})
        except (OperationalError,OSError,ValueError,TypeError,KeyError,sqlite3.Error) as error:
            return {"status":"rejected","code":"invalid_request","message":str(error)}

    def revoke_context(self, body):
        with lock(self.paths.state / "session/admission.lock"):
            return self._revoke_context_locked(body)

    def _revoke_context_locked(self, body):
        from .context_access import grant_path, grant_root
        payload=body.get("payload")
        command_id=body.get("command_id")
        if not isinstance(command_id,str) or not isinstance(payload,dict):
            return {"status":"rejected","code":"invalid_request"}
        prior=self.store.external_result(body)
        if prior is not None:return prior
        try:
            grant_id=payload["grant_id"]
            grant=read(grant_path(self.paths,grant_id))
            if grant["recipient_project_key"]!=self.paths.key:
                raise OperationalError("Grant belongs to another project")
            path=grant_root(self.paths)/"revocations"/(grant_id+".json")
            if path.exists():
                prior_revocation=read(path)
                if prior_revocation.get("command_sha256")!=digest(canonical(body)):
                    return {"status":"rejected","code":"idempotency_conflict"}
                return self.store.record_external(body,{"status":"accepted","grant_id":grant_id,"revoked":True})
            atomic(path,{"schema_version":1,"grant_id":grant_id,"recipient_project_key":self.paths.key,
                         "revocation_command_id":command_id,"command_sha256":digest(canonical(body)),
                         "reason":payload.get("reason","Operator revoked access")},immutable=True)
            return self.store.record_external(body,{"status":"accepted","grant_id":grant_id,"revoked":True})
        except (OperationalError,OSError,ValueError,TypeError,KeyError) as error:
            return {"status":"rejected","code":"invalid_request","message":str(error)}

    def context_access_list(self, task_id):
        from .context_access import grant_root, revoked
        result=[]
        for path in sorted((grant_root(self.paths)/"grants").glob("*.json")):
            grant=read(path)
            if grant.get("recipient_project_key")!=self.paths.key:
                raise OperationalError("Foreign grant record in selected project state")
            if task_id is None or grant["task_id"]==task_id:
                result.append({"grant_id":grant["grant_id"],"task_id":grant["task_id"],
                               "source_project_key":grant["source_project_key"],"purpose":grant["purpose"],
                               "selector":grant["selector"],"dependency_mode":grant["dependency_mode"],
                               "active":not revoked(self.paths,grant["grant_id"])})
        return {"grants":result}

    def dispatch_command(self, payload):
        with lock(self.paths.state / "session/maintenance.lock"):
            action=payload.get("action")
            if action in ("stop_session", "resume_session"):
                return self.lifecycle_command(payload)
            if self.shutting_down:
                return {"status": "rejected", "code": "service_stopping"}
            if action == "recover_coordinator":
                return self.recover_coordinator(payload)
            if action=="admit_task":return self.admit_task(payload)
            if action=="answer_task":return self.answer_task(payload)
            if action=="steer_queued":return self.store.modify_queued(payload)
            if action=="cancel_task":return self.cancel_task(payload)
            if action=="grant_context_access":return self.grant_context(payload)
            if action=="revoke_context_access":return self.revoke_context(payload)
            return self.store.command(payload)

    def recover_coordinator(self, body):
        from .coordinator_recovery import verify
        payload=body.get("payload")
        if (not isinstance(body.get("command_id"),str) or not isinstance(payload,dict) or
            not isinstance(payload.get("task_id"),str)):
            return {"status":"rejected","code":"invalid_request"}
        old=self.store.external_result(body)
        if old is not None:return old
        task=self.store.task(payload["task_id"])
        if not task:return {"status":"rejected","code":"task_not_found"}
        with self.store.open() as db:
            row=db.execute("SELECT * FROM session_coordinator_calls WHERE task_id=?",(task["task_id"],)).fetchone()
        if not row:return {"status":"rejected","code":"no_coordinator_call"}
        try:
            directory,op,claim=verify(self.paths,task,dict(row))
            if (directory / "recovery.json").exists():
                prior=read(directory / "recovery.json")
                if prior.get("command_id") != body["command_id"]:
                    raise OperationalError("Existing recovery belongs to another command")
            else:
                replacement=f"call-v{task['version']}-r1"
                pointer=self.paths.state/"session/ui-current.json"
                if not pointer.exists():raise OperationalError("Attach the corrected project UI before coordinator recovery")
                binding=read(pointer)
                from .herdr_backend import Herdr, server_matches
                target={"socket":binding["socket"],"workspace":binding["pane"]["workspace_id"]}
                herdr=Herdr(target["socket"])
                observed=server_matches(herdr)
                if binding.get("project_key")!=self.paths.key or any(
                    observed[key]!=binding["server"][key] for key in ("pid","start","boot")):
                    raise OperationalError("Current Herdr UI binding is not verified for this project")
                herdr.call("workspace.get",{"workspace_id":target["workspace"]})
                atomic(directory / "recovery.json",{"schema_version":1,"command_id":body["command_id"],
                    "original_launch_token":op["launch_token"],"original_runner":claim["runner"],
                    "operation_sha256":digest((directory / "operation.json").read_bytes()),
                    "launch_sha256":digest((directory / "launch.json").read_bytes()),
                    "claim_sha256":digest((directory / "claim.json").read_bytes()),
                    "packet_sha256":op["packet_sha256"],"replacement":replacement,"target":target,
                    "reason":"fake coordinator exited before dispatch permit"},immutable=True)
            outcome=self.store.reopen_prepermit_coordinator(body,task["task_id"])
            if outcome["status"] == "accepted":self.start_coordinator()
            return outcome
        except (OperationalError,OSError,KeyError,ValueError,TypeError) as error:
            return {"status":"rejected","code":"recovery_refused","message":str(error)}

    def admit_task(self, body):
        with lock(self.paths.state / "session/admission.lock"):
            return self._admit_task_locked(body)

    def _admit_task_locked(self, body):
        payload = body.get("payload")
        if not isinstance(payload, dict) or not isinstance(body.get("command_id"), str):
            return {"status": "rejected", "code": "invalid_request"}
        prior = self.store.external_result(body)
        if prior is not None:
            return prior
        task = self.store.task(payload.get("task_id"))
        if not task or task["status"] != "proposed":
            return {"status": "rejected", "code": "stale_state"}
        try:
            plan = self.plan(payload)
            if digest(canonical(plan)) != payload.get("plan_digest"):
                raise OperationalError("Frozen task plan digest changed")
            if body.get("expected_task_version") != plan["task_version"] or body.get("expected_state_revision") != plan["task_state_revision"]:
                return {"status": "rejected", "code": "stale_state"}
            if self.store.run_claimed(plan["run_id"], plan["task_id"]):
                raise OperationalError("Run ID already belongs to another task in this project")
            profile_path = selected_path(plan["profile"], self.paths.project)
            if digest(profile_path.read_bytes()) != plan["profile_sha256"]:
                raise OperationalError("Profile changed before authorization")
            outcome = self.store.admit(body, plan["task_id"], plan)
            if outcome["status"] == "accepted":
                self.start_promotion()
            return outcome
        except (OperationalError, OSError, ValueError, TypeError, KeyError) as error:
            return {"status": "rejected", "code": "invalid_request", "message": str(error)}

    def reconcile_admissions(self):
        # Authorization and its frozen plan commit in one SQLite transaction.
        # A ready row is the restart-safe work item; no model call has been dispatched.
        self.start_promotion()

    def start_promotion(self):
        with self.promotion_lock:
            if self.promotion_thread and self.promotion_thread.is_alive():
                self.promotion_again = True
                return False
            self.promotion_again = True
            self.promotion_thread = threading.Thread(target=self._promotion_loop, daemon=True)
            self.promotion_thread.start()
            return True

    def _promotion_loop(self):
        while True:
            with self.promotion_lock:
                if not self.promotion_again:
                    self.promotion_thread = None
                    return
                self.promotion_again = False
            self._promote_ready()

    def _promote_ready(self):
        from . import records
        token = bind(self.paths)
        try:
            with lock(self.paths.state / "session/admission.lock"):
                if self.store.phase()["phase"] != "running":
                    return
                if self.store.active():
                    return
                task = self.store.next_ready()
                if not task:
                    return
                plan = json.loads(task["plan_json"])
                run_id = plan["run_id"]
                if digest(task["plan_json"]) != task["plan_sha256"] or plan["project_key"] != self.paths.key:
                    self.store.block_unbound(task["task_id"], None, "Authorized plan integrity mismatch")
                    return
                profile = selected_path(plan["profile"], self.paths.project)
                if digest(profile.read_bytes()) != plan["profile_sha256"] or self.store.run_claimed(run_id, task["task_id"]):
                    self.store.block_unbound(task["task_id"], None, "Frozen profile or run identity changed")
                    return
                if "context" in plan:
                    from .context import resolve
                    selection=selected_path(plan["context"],self.paths.project)
                    if digest(canonical(resolve(selection))) != plan["context_snapshot_sha256"]:
                        self.store.block_unbound(task["task_id"],None,"Selected context changed before dispatch; request a new plan")
                        return
                if plan.get("cross_imports"):
                    from .context_access import check_frozen
                    check_frozen(self.paths, plan["cross_imports"])
                # The workspace may not exist yet when the service starts. The next UI
                # attach will retry promotion without consuming an invocation.
                pointer = self.paths.state / "session/ui-current.json"
                if not pointer.exists() or read(pointer)["pane"]["workspace_id"] != plan["workspace"]:
                    return
                from .herdr_backend import Herdr
                try:
                    Herdr(self.herdr_socket).call("workspace.get", {"workspace_id": plan["workspace"]})
                except (OperationalError, OSError):
                    return
                intent = self.paths.state / "session/admission-intents" / (task["command_id"] + ".json")
                atomic(intent, {"schema_version": 1, "command_id": task["command_id"], "plan": plan,
                                "plan_sha256": task["plan_sha256"], "status": "may-have-created"}, immutable=True)
                try:
                    existing = records.get_run(run_id)
                except OperationalError:
                    existing = None
                if existing is None:
                    from .cli import create
                    create(argparse.Namespace(run=run_id, profile=plan["profile"], brief=None,
                                              brief_text=plan["brief"], socket=self.herdr_socket,
                                              workspace=plan["workspace"], context=plan.get("context"),
                                              cross_imports=plan.get("cross_imports",[])))
                frozen = records.get_run(run_id)["envelope"]
                if (frozen.get("brief_sha256") != plan["brief_sha256"] or
                    frozen.get("config_sha256") != digest(canonical(plan["config"])) or
                    frozen.get("project_key") != self.paths.key or
                    ("context_snapshot_sha256" in plan and frozen.get("context_digest") != plan["context_snapshot_sha256"]) or
                    frozen.get("cross_imports",[]) != plan.get("cross_imports",[])):
                    self.store.block_unbound(task["task_id"], run_id, "Reserved run differs from authorized plan; inspect")
                    return
                self.store.activate(task["task_id"], run_id)
                self.start_driver(task["task_id"], run_id)
        except (OperationalError, OSError, ValueError, KeyError) as error:
            if 'task' in locals() and task:
                self.store.block_unbound(task["task_id"], None, f"Admission stopped: {error}")
        finally:
            from .runtime import unbind
            unbind(token)

    def start_driver(self, task_id, run_id):
        with self.driver_lock:
            if self.driver_thread and self.driver_thread.is_alive():
                if self.driver_task == (task_id, run_id):
                    return False
                if self.driver_pending and self.driver_pending != (task_id, run_id):
                    raise OperationalError("More than one next workflow was scheduled")
                if not self.driver_pending:
                    self.driver_pending = (task_id, run_id)
                    threading.Thread(target=self._after_driver, args=(self.driver_thread,), daemon=True).start()
                return False
            self.driver_task = (task_id, run_id)
            self.driver_thread = threading.Thread(target=self._drive, args=(task_id, run_id), daemon=True)
            self.driver_thread.start()
            return True

    def _after_driver(self, previous):
        previous.join()
        with self.driver_lock:
            pending = self.driver_pending
            self.driver_pending = None
        if pending:
            task = self.store.task(pending[0])
            if task and task["run_id"] == pending[1] and task["status"] == "active":
                self.start_driver(*pending)

    def resume_answered_gate(self, task_id, run_id, request_id):
        # The graph driver may still be exiting its interrupt when the operator
        # answers. Keep its duplicate-dispatch guard; resume only after it exits.
        with self.driver_lock:
            previous = self.driver_thread if self.driver_thread and self.driver_thread.is_alive() else None
        threading.Thread(target=self._answered_gate_after_driver,
                         args=(previous, task_id, run_id, request_id), daemon=True).start()

    def _answered_gate_after_driver(self, previous, task_id, run_id, request_id):
        from . import records, workflow
        token = bind(self.paths)
        try:
            if previous:
                previous.join()
            task = self.store.task(task_id)
            response = records.response(run_id, request_id)
            graph = workflow.state(run_id)
            pending = graph.values.get("pending")
            if (not task or task["run_id"] != run_id or task["status"] not in ("awaiting-input", "active")
                or not response or not graph.interrupts or not pending or pending.get("request_id") != request_id):
                return
            if self.store.phase()["phase"] != "running":
                return  # answer is durable; explicit resume restarts the dependent stage
            self.store.transition(task_id, run_id, "active", "Human response recorded; continuing")
            self.start_driver(task_id, run_id)
        except (OperationalError, OSError, ValueError) as error:
            task = self.store.task(task_id)
            if task and task["run_id"] == run_id:
                self.store.transition(task_id, run_id, "uncertain", f"Answered gate could not safely resume: {error}")
        finally:
            from .runtime import unbind
            unbind(token)

    def _drive(self, task_id, run_id):
        from . import ownership, records, workflow
        token = bind(self.paths)
        owner = ownership.bind_service(self.generation)
        try:
            with lock(self.paths.state / "controller.lock"):
                workflow.drive(run_id)
        except BaseException as error:
            from .storage import Uncertain
            # The graph/runner records retain the exact failure; no retry here.
            try:
                record = records.get_run(run_id)
                if record["status"] == "running":
                    records.set_status(run_id, "uncertain", f"Driver stopped: {error}")
            except Exception:
                pass
        finally:
            try:
                record = records.get_run(run_id)
                self.store.transition(task_id, run_id, record["status"], record["detail"])
            except Exception:
                record = None
            ownership.unbind_service(owner)
            from .runtime import unbind
            unbind(token)
            if record and record["status"] in ("approved", "rejected", "failed", "cancelled"):
                self.start_promotion()

    def resume_active(self):
        from . import records, workflow
        from .herdr_backend import Herdr
        selected = self.store.active()
        if not selected or not selected["run_id"] or selected["status"] in ("uncertain", "blocked"):
            return
        if self.store.phase()["phase"] != "running" and selected["status"] == "paused":
            return
        record = records.get_run(selected["run_id"])
        target=records.effective_target(selected["run_id"])
        try:
            Herdr(target["socket"]).call("workspace.get",{"workspace_id":target["workspace"]})
        except (OperationalError,OSError) as error:
            self.store.transition(selected["task_id"],selected["run_id"],"recovering",
                                  f"Worker workspace unavailable; awaiting verified UI reconnect: {error}")
            return
        if record["status"] == "awaiting-input":
            snapshot = workflow.state(selected["run_id"])
            pending = snapshot.values.get("pending")
            if not pending or records.response(selected["run_id"], pending["request_id"]) is None:
                self.store.transition(selected["task_id"], selected["run_id"], "awaiting-input", record["detail"])
                return
        if record["status"] in ("uncertain", "blocked", "output-invalid"):
            self.store.transition(selected["task_id"], selected["run_id"], record["status"], record["detail"])
            return
        self.start_driver(selected["task_id"], selected["run_id"])

    def reconcile_active_workspace(self, workspace):
        from . import records
        from .routing import rebind, recover_missing_workspace
        selected=self.store.active()
        if not selected or not selected["run_id"]:
            return
        run_id=selected["run_id"]
        target=records.effective_target(run_id)
        if target["workspace"]==workspace:
            self.resume_active()
            return
        run=records.get_run(run_id)
        try:
            with lock(self.paths.state/"controller.lock"):
                if run["status"]=="awaiting-input":
                    rebind(run_id,workspace,allow_answered=True)
                elif run["status"] in ("uncertain","recovering") and "workspace_not_found" in (run["detail"] or ""):
                    recover_missing_workspace(run_id,workspace,generation=self.generation)
                    records.set_status(run_id,"recovering","Verified prelaunch workspace recovery")
                else:
                    raise OperationalError("Workspace changed outside a verified quiescent recovery boundary")
            self.store.transition(selected["task_id"],run_id,run["status"],"Verified workspace routing appended")
            self.resume_active()
        except (OperationalError,OSError,ValueError,KeyError) as error:
            detail=f"Workspace recovery refused: {error}; inspect retained operation evidence"
            records.set_status(run_id,"uncertain",detail)
            self.store.transition(selected["task_id"],run_id,"uncertain",detail)

    def start_coordinator(self):
        if self.store.phase()["phase"] != "running":
            pending = self.store.pending_coordinator_call()
            task = self.store.task(pending["task_id"]) if pending else None
            if not pending or not (self.paths.state / "session/coordinator" / pending["task_id"] /
                                   f"call-v{task['version']}" / "permit.json").exists():
                return False
        with self.coordinator_lock:
            if self.coordinator_thread and self.coordinator_thread.is_alive():
                return False
            self.coordinator_thread = threading.Thread(target=self._coordinate_loop, daemon=True)
            self.coordinator_thread.start()
            return True

    def _coordinate_loop(self):
        from . import ownership
        from .herdr_backend import prepare, wait
        from .storage import Uncertain, OutputInvalid
        from .runtime import unbind
        token = bind(self.paths)
        owner = ownership.bind_service(self.generation)
        try:
            while True:
                call = None
                reserved = self.store.pending_coordinator_call()
                if self.store.phase()["phase"] != "running" and not reserved:
                    return
                task = self.store.task(reserved["task_id"]) if reserved else self.store.next_coordinator_task()
                if not task:
                    return
                original = self.paths.state / "session/coordinator" / task["task_id"] / f"call-v{task['version']}"
                recovery = original / "recovery.json"
                directory = original.parent / read(recovery)["replacement"] if recovery.exists() else original
                packet_file = directory / "packet.json"
                target_file = directory / "target.json"
                if recovery.exists() and not packet_file.exists():
                    atomic(packet_file, read(original / "packet.json"), immutable=True)
                    atomic(target_file, read(recovery).get("target",read(original / "target.json")), immutable=True)
                if packet_file.exists() and target_file.exists():
                    packet, target = read(packet_file), read(target_file)
                else:
                    pointer = self.paths.state / "session/ui-current.json"
                    if not pointer.exists():
                        return  # No visible owned workspace; next attach resumes this queue.
                    workspace = read(pointer)["pane"]["workspace_id"]
                    packet = {"schema_version": 1, "operation_kind": "coordinator",
                              "operation_id": "coordinator-" + task["task_id"],
                              "run_id": task["task_id"], "stage": "coordinator-1", "role": "coordinator",
                              "provider": "fake", "model": "deterministic-coordinator-v1",
                              "task_id": task["task_id"], "task_version": task["version"],
                              "project_key": self.paths.key, "message": task["body"],
                              "message_sha256": digest(task["body"]), "seconds": 1.0}
                    target = {"socket": self.herdr_socket, "workspace": workspace}
                    atomic(packet_file, packet, immutable=True)
                    atomic(target_file, target, immutable=True)
                try:
                    if self.store.phase()["phase"] != "running" and not (directory / "permit.json").exists():
                        return
                    call = self.store.reserve_coordinator(task, digest(canonical(packet)))
                    if call is None:
                        continue
                    prepare(directory, packet, target)
                    done = wait(directory)
                    if done["status"] != "completed":
                        self.store.stop_coordinator(task["task_id"],call["call_id"],done["status"],
                                                    "Coordinator worker stopped; inspect its receipt, no retry")
                        continue
                    result = read(directory / "result.json")
                    self.store.publish_proposal(task["task_id"],call["call_id"],result,done["result_sha256"])
                except (OperationalError, Uncertain, OutputInvalid, OSError, ValueError, KeyError) as error:
                    if str(error) == "Coordinator proposal input became stale before reservation":
                        continue
                    if reserved or self.store.pending_coordinator_call():
                        pending = call or reserved or self.store.pending_coordinator_call()
                        self.store.stop_coordinator(task["task_id"],pending["call_id"],"uncertain",str(error))
                    else:
                        self.store.block_unbound(task["task_id"],None,str(error))
        finally:
            ownership.unbind_service(owner)
            unbind(token)

    async def handle(self, reader, writer):
        async with self.connections:
            self.active_writers.add(writer)
            try:
                peer = writer.get_extra_info("socket")
                raw = peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)
                pid, uid, _gid = struct.unpack("3i", raw)
                if uid != os.geteuid():
                    return
                hello = parse_frame(await reader.readline())
                body = hello["body"]
                allowed = (hello["kind"] == "hello" and body.get("protocol", {}).get("major") == 1
                           and body.get("release_id") == self.release_id and body.get("project_key") == self.paths.key)
                writer.write(encode("hello_result" if allowed else "error", hello["request_id"],
                                    {"accepted": allowed, "owner_generation": 1, "state_schema": 1,
                                     "code": None if allowed else "protocol_mismatch"}))
                await writer.drain()
                if not allowed: return
                while True:
                    raw = await reader.readline()
                    if not raw: break
                    request = parse_frame(raw)
                    kind, ident, payload = request["kind"], request["request_id"], request["body"]
                    if kind == "snapshot":
                        response = "snapshot", await asyncio.to_thread(self.store.snapshot)
                    elif kind == "session.status":
                        response = "session.status_result", {"session": await asyncio.to_thread(self.store.phase),
                                                               "checkpoint": await asyncio.to_thread(self.checkpoint),
                                                               "project_key": self.paths.key}
                    elif kind == "task.plan":
                        try:
                            plan = await asyncio.to_thread(self.plan, payload)
                            response = "task.plan_result", {"plan": plan, "plan_digest": digest(canonical(plan))}
                        except (OperationalError, OSError, ValueError, TypeError) as error:
                            response = "error", {"code": "invalid_request", "message": str(error)}
                    elif kind == "task.get":
                        try: response = "task.result", await asyncio.to_thread(self.task_detail, payload.get("task_id"))
                        except (OperationalError, OSError, ValueError, TypeError) as error:
                            response = "error", {"code": "invalid_request", "message": str(error)}
                    elif kind == "context.access.list":
                        try: response = "context.access.result", await asyncio.to_thread(self.context_access_list,payload.get("task_id"))
                        except (OperationalError,OSError,ValueError) as error:
                            response = "error", {"code":"invalid_request","message":str(error)}
                    elif kind == "events_read":
                        after, limit = payload.get("after_seq"), payload.get("limit")
                        if type(after) is not int or after < 0 or type(limit) is not int or not 1 <= limit <= 64:
                            response = "error", {"code": "invalid_request", "message": "invalid event cursor/limit"}
                        else:
                            result = await asyncio.to_thread(self.store.events, after, limit)
                            if not result["events"]:
                                await asyncio.sleep(min(max(payload.get("wait_ms", 0), 0), 1000) / 1000)
                                result = await asyncio.to_thread(self.store.events, after, limit)
                            response = "events_result", result
                    elif kind == "command_get":
                        response = "command_result", await asyncio.to_thread(self.store.command_get, payload.get("command_id"))
                    elif kind == "command" and len(raw) <= COMMAND_MAX:
                        result = await asyncio.to_thread(self.dispatch_command,payload)
                        response = "command_result", result
                        if payload.get("action") in ("submit_message","steer_queued") and result.get("status") == "accepted":
                            self.start_coordinator()
                    elif kind == "ui.attach":
                        try: response = "ui.attach_result", await asyncio.to_thread(self.attach)
                        except (OperationalError, OSError, ValueError) as error:
                            response = "error", {"code": "busy", "message": str(error),
                                                   "execution_state": "not_applicable"}
                    elif kind == "ui.register":
                        try: response = "ui.register_result", await asyncio.to_thread(self.register_ui, pid, payload)
                        except (OperationalError, OSError, ValueError) as error:
                            atomic(self.paths.state / "session/ui-errors" / f"{time.time_ns()}.json",
                                   {"schema_version": 1, "at": time.time(), "action": "ui.register",
                                    "reason": str(error), "pid": pid}, immutable=True)
                            response = "error", {"code": "unauthorized", "message": str(error),
                                                   "execution_state": "not_applicable"}
                    else:
                        response = "error", {"code": "invalid_request", "message": "unknown request"}
                    writer.write(encode(response[0], ident, response[1]))
                    await writer.drain()
            except (OSError, ValueError, KeyError, TypeError, asyncio.IncompleteReadError):
                pass
            finally:
                self.active_writers.discard(writer)
                writer.close()
                try: await writer.wait_closed()
                except OSError: pass

    def attach(self):
        from .shell import attach
        deadline = time.monotonic() + 3
        while True:
            try:
                with lock(self.paths.state / "session/ui-attach.lock"):
                    return attach(self.paths, self.herdr_socket, ui_command=[self.frontend,
                        "--socket", str(self.ipc), "--project-key", self.paths.key,
                        "--release-id", self.release_id, "--title", self.paths.project.name,
                        "--outbox-dir", str(self.paths.state / "session" / "outbox")])
            except OperationalError as error:
                if not str(error).startswith("Already owned:") or time.monotonic() >= deadline:
                    raise
                time.sleep(.05)

    def register_ui(self, pid, payload):
        from .herdr_backend import Herdr
        from .shell import _matches
        token = payload.get("ui_token")
        pane_id = payload.get("pane_id")
        if not isinstance(token, str) or not isinstance(pane_id, str):
            raise OperationalError("Missing UI token or Herdr pane")
        pointer = self.paths.state / "session/ui-current.json"
        deadline = time.monotonic() + 5
        binding = None
        while time.monotonic() < deadline:
            if pointer.exists():
                candidate = read(pointer)
                if candidate.get("token") == token:
                    binding = candidate
                    break
            time.sleep(.05)
        if binding is None:
            raise OperationalError("Matching UI binding not published within five seconds")
        if binding["project_key"] != self.paths.key:
            raise OperationalError("UI token belongs to another project")
        h = Herdr(self.herdr_socket)
        observed = h.server_identity()
        if any(observed[field] != binding["server"][field] for field in ("pid", "start", "boot")):
            raise OperationalError("Herdr server identity changed")
        matches = _matches(h, token)
        if len(matches) != 1 or matches[0] != binding["pane"] or matches[0]["pane_id"] != pane_id:
            raise OperationalError("UI pane identity does not match binding")
        command = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        if token.encode() not in command or str(self.frontend).encode() not in command:
            raise OperationalError("UI process command does not match release binding")
        claim = {"schema_version": 1, "project_key": self.paths.key, "token": token,
                 "process": identity(pid), "pane_id": pane_id}
        atomic(self.paths.state / "session/ui-claims" / (token + ".json"), claim, immutable=True)
        self.reconcile_active_workspace(matches[0]["workspace_id"])
        self.start_promotion()
        return {"status": "registered", "pane": matches[0]}

    async def run(self):
        self.ipc.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        os.chmod(self.ipc.parent, 0o700)
        if self.ipc.exists():
            self.ipc.unlink()
        server = await asyncio.start_unix_server(self.handle, path=str(self.ipc), limit=FRAME_MAX + 1)
        os.chmod(self.ipc, 0o600)
        self.event_loop = asyncio.get_running_loop()
        self.shutdown_event = asyncio.Event()
        if self.store.phase()["phase"] in ("exit-requested", "draining"):
            self.start_drain()
        async with server:
            await self.shutdown_event.wait()
            server.close()
            for writer in tuple(self.active_writers):
                writer.close()
            await asyncio.wait_for(server.wait_closed(), timeout=3)
        completed=self.store.phase()
        atomic(self.paths.state / "session/shutdown-completions" /
               f"{completed['generation']}-{completed['exit_id']}.json",
               {"schema_version": 1, "service_generation": self.generation,
                "exit_id": completed.get("exit_id")}, immutable=True)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", required=True)
    parser.add_argument("--state-dir", required=True)
    parser.add_argument("--registry-dir", required=True)
    parser.add_argument("--ipc", required=True)
    parser.add_argument("--herdr-socket", required=True)
    parser.add_argument("--release-id", required=True)
    parser.add_argument("--frontend", required=True)
    args = parser.parse_args(argv)
    paths = load(args.project, args.state_dir, args.registry_dir)
    bind(paths)
    register(paths)
    root = paths.state / "session"
    root.mkdir(parents=True, exist_ok=True)
    fence = (root / "supervisor.lock").open("a+")
    try:
        fcntl.flock(fence, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        raise SystemExit("Project service already has an owner")
    generation_file = root / "service-generation.json"
    previous = read(generation_file)["generation"] if generation_file.exists() else 0
    atomic(generation_file, {"schema_version": 1, "project_key": paths.key,
                             "generation": previous + 1, "process": identity(os.getpid())})
    service = Service(paths, args.ipc, args.herdr_socket, args.release_id, args.frontend, previous + 1)
    service.reconcile_admissions()
    service.resume_active()
    service.start_coordinator()
    asyncio.run(service.run())


if __name__ == "__main__":
    main()
