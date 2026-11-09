"""Interpret completed text outputs without confusing coverage with execution."""
from .storage import OutputInvalid, atomic, canonical, digest
from . import records


def revision_coverage(stage, prior, result):
    """Return omitted, still-open original findings; never infer a disposition."""
    known = set(prior)
    seen = set()
    for item in result["dispositions"]:
        key = item["id"]
        if key not in known or key in seen:
            raise OutputInvalid(f"Invalid or duplicate finding disposition {key} in {stage}")
        seen.add(key)
    for item in result["findings"]:
        if item["id"] in known or item["id"] in seen:
            raise OutputInvalid(f"Duplicate finding ID {item['id']} in {stage}; saved output cannot be adopted")
        seen.add(item["id"])
    if not stage.startswith("revision"):
        return []
    return [dict(value) for key, value in prior.items()
            if value["status"] == "open" and key not in {d["id"] for d in result["dispositions"]}]


def record_gap(run_id, stage, packet_hash, result, missing):
    path = records.run_dir(run_id) / "quality" / (stage + ".json")
    atomic(path, {"schema_version": 1, "condition": "revision-incomplete", "stage": stage,
                  "packet_sha256": packet_hash, "result_sha256": digest(canonical(result)),
                  "omitted_findings": missing}, immutable=True)


def record_invalid_output(run_id, stage, packet_hash, result, reason):
    """Retain a durable semantic rejection without changing the worker receipt."""
    directory = records.run_dir(run_id) / "operations" / stage
    path = records.run_dir(run_id) / "quality" / (stage + "-invalid.json")
    atomic(path, {"schema_version": 1, "condition": "output-invalid", "stage": stage,
                  "reason": reason, "packet_sha256": packet_hash,
                  "result_sha256": digest(canonical(result)),
                  "receipt_file_sha256": digest((directory / "receipt.json").read_bytes())}, immutable=True)
    return path
