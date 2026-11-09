"""Development-only r9/Rust legacy-inspection comparison on disposable copies.

Run with the preserved r9 private Python, which imports its installed backend.
No operator project, service, worker or model is selected.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess

from agent_workflow import artifacts, records
from agent_workflow.runtime import bind, load, unbind
from agent_workflow.storage import canonical


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def project_files(project: Path) -> dict[str, str]:
    return {str(p.relative_to(project)): digest(p.read_bytes())
            for p in project.rglob("*") if p.is_file()}


def baseline(project: Path, run_id: str) -> dict:
    state = project / ".wayward-wrangler/state"
    registry = project.parent / "test-registry"
    registry.mkdir(exist_ok=True)
    paths = load(project, state_dir=state, registry_dir=registry)
    token = bind(paths)
    try:
        run = records.get_run(run_id)
        out = {"run_id": run_id, "status": run["status"],
               "envelope_sha256": digest(run["envelope_raw"].encode()) if "envelope_raw" in run else None,
               "requests": [], "responses": [], "operations": [],
               "artifacts": [], "finding_ledgers": []}
        db = sqlite3.connect(state / "state.sqlite")
        out["envelope_sha256"] = digest(db.execute(
            "SELECT envelope FROM app_runs WHERE run_id=?", (run_id,)).fetchone()[0].encode())
        for table, target in (("app_requests", "requests"), ("app_responses", "responses")):
            for key, raw in db.execute(
                f"SELECT request_id,payload FROM {table} WHERE run_id=? ORDER BY rowid", (run_id,)):
                value = json.loads(raw)
                assert value["request_id"] == key
                if table == "app_responses":
                    assert records.response(run_id, key) == value
                    assert (state / "runs" / run_id / "artifacts" / f"decision-{key}.json").read_bytes() == raw.encode()
                out[target].append({"id": key, "sha256": digest(raw.encode())})
        out["operations"] = [
            {"stage": item["stage"], "operation_id": item["operation_id"],
             "packet_hash": item["packet_hash"]} for item in records.operations(run_id)]
        directory = state / "runs" / run_id
        for path in sorted((directory / "artifacts").glob("*.json")):
            if path.name.startswith("decision-"):
                continue
            manifest = json.loads(path.read_bytes())["manifest"]
            artifacts.content(run_id, manifest)
            out["artifacts"].append({"id": manifest["id"], "sha256": manifest["sha256"],
                                     "path": manifest["path"], "parents": manifest["parents"],
                                     "manifest_raw_sha256": digest(path.read_bytes())})
        out["artifacts"].sort(key=lambda item: item["id"])
        for path in sorted((directory / "quality").glob("*-finding-ledger.json")):
            value = json.loads(path.read_bytes())
            out["finding_ledgers"].append({"stage": value["stage"],
                                           "raw_sha256": digest(path.read_bytes())})
        db.close()
        return out
    finally:
        unbind(token)


def rust(binary: Path, project: Path, run_id: str) -> tuple[bool, dict | str]:
    call = subprocess.run([str(binary), str(project / ".wayward-wrangler/state"), run_id],
                          capture_output=True, text=True, check=False)
    return (True, json.loads(call.stdout)) if call.returncode == 0 else (False, call.stderr.strip())


def projection(value: dict) -> dict:
    return {"run_id": value["run_id"], "status": value["status"],
            "envelope_sha256": value["envelope"]["raw_sha256"],
            "requests": [{"id": r["id"], "sha256": r["raw_sha256"]} for r in value["requests"]],
            "responses": [{"id": r["id"], "sha256": r["raw_sha256"]} for r in value["responses"]],
            "operations": value["operations"],
            "artifacts": value["artifacts"],
            "finding_ledgers": [{"stage": item["stage"], "raw_sha256": item["raw_sha256"]}
                                for item in value["finding_ledgers"]]}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("binary", type=Path)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("run_id")
    args = parser.parse_args()
    source = args.source.resolve()
    binary = args.binary.resolve()
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    source_hashes = project_files(source)
    results = []
    for name in ("base", "changed-artifact", "path-escape", "decision-conflict",
                 "stale-approval", "unknown-schema", "unknown-state-schema"):
        target = evidence / name
        if target.exists():
            raise RuntimeError(f"refusing to overwrite evidence: {target}")
        shutil.copytree(source, target)
        artifacts_dir = target / ".wayward-wrangler/state/runs" / args.run_id / "artifacts"
        if name == "changed-artifact":
            with (artifacts_dir / "draft.md").open("ab") as out:
                out.write(b"\nTampered fixture\n")
        elif name == "path-escape":
            file = artifacts_dir / "draft.json"
            value = json.loads(file.read_text())
            value["manifest"]["path"] = "../../../../outside.md"
            file.write_text(json.dumps(value, indent=2) + "\n")
        elif name == "decision-conflict":
            with (artifacts_dir / "decision-A1.json").open("ab") as out:
                out.write(b"\n")
        elif name == "unknown-schema":
            file = artifacts_dir / "draft.json"
            value = json.loads(file.read_text())
            value["manifest"]["schema_version"] = 2
            file.write_text(json.dumps(value, indent=2) + "\n")
        elif name == "stale-approval":
            database = target / ".wayward-wrangler/state/state.sqlite"
            db = sqlite3.connect(database)
            raw = db.execute("SELECT payload FROM app_responses WHERE run_id=? AND request_id='A1'",
                             (args.run_id,)).fetchone()[0]
            value = json.loads(raw)
            value["artifact_sha256"] = json.loads((artifacts_dir / "draft.json").read_text())["manifest"]["sha256"]
            changed = canonical(value)
            db.execute("UPDATE app_responses SET payload=? WHERE run_id=? AND request_id='A1'",
                       (changed, args.run_id))
            db.commit()
            db.close()
            (artifacts_dir / "decision-A1.json").write_text(changed)
        elif name == "unknown-state-schema":
            database = target / ".wayward-wrangler/state/state.sqlite"
            db = sqlite3.connect(database)
            db.execute("CREATE TABLE ww_schema_migrations(version INTEGER)")
            db.commit()
            db.close()
        base_ok, base_value = True, None
        try:
            base_value = baseline(target, args.run_id)
        except Exception as error:  # expected for tampered copies
            base_ok, base_value = False, f"{type(error).__name__}: {error}"
        rust_ok, rust_value = rust(binary, target, args.run_id)
        if name == "base":
            assert base_ok and rust_ok, (base_value, rust_value)
            assert base_value == projection(rust_value), "meaningful baseline/Rust mismatch"
            assert len(rust_value["responses"]) == 2
            assert rust_value["status"] == "approved"
            assert rust_value["graph_continuation"].startswith("opaque:")
            (evidence / "python-baseline.json").write_text(json.dumps(base_value, indent=2) + "\n")
            (evidence / "rust-inspection.json").write_text(json.dumps(rust_value, indent=2) + "\n")
        else:
            assert not rust_ok, f"Rust accepted {name}"
            if name in ("changed-artifact", "path-escape", "decision-conflict"):
                assert not base_ok, f"Python accepted {name}"
            else:
                assert base_ok, "unexpected Python schema refusal; review variance"
        results.append({"case": name, "python_accepted": base_ok, "rust_accepted": rust_ok,
                        "python_detail": "accepted" if base_ok else base_value,
                        "rust_detail": "accepted" if rust_ok else rust_value})
    assert project_files(source) == source_hashes, "source evidence changed"
    (evidence / "comparison.json").write_text(json.dumps({
        "schema_version": 1, "r9_python": __import__("agent_workflow").__file__,
        "source": str(source), "source_file_count": len(source_hashes),
        "cases": results}, indent=2) + "\n")
    print("Stage 1: base parity and six integrity/schema refusals passed; source unchanged")


if __name__ == "__main__":
    main()
