"""Application-owned finding identities and append-only assessment semantics."""
from .storage import OutputInvalid, canonical, digest, atomic
from . import records


def durable_id(operation_id, index):
    return "F-" + digest(f"{operation_id}:{index}")[:16]


def classify(stage, prior, packet, result, interpretation=None):
    """Separate new findings from references without changing prior evidence."""
    version = packet.get("finding_contract_version", 1)
    new, assessments, seen_new, seen_refs = [], [], set(), set()
    if version == 2:
        if interpretation is not None:
            raise OutputInvalid("Finding interpretation applies only to a saved legacy result")
        for index, item in enumerate(result["new_findings"]):
            key = durable_id(packet["operation_id"], index)
            if key in prior or key in seen_new:
                raise OutputInvalid(f"Application finding identity collision {key}")
            seen_new.add(key)
            new.append({**item, "id": key, "source_index": index})
        for index, item in enumerate(result["assessments"]):
            ref = item["finding_ref"]
            if ref not in prior:
                raise OutputInvalid(f"Assessment refers to unknown finding {ref}")
            if ref in seen_refs:
                raise OutputInvalid(f"Conflicting assessments for finding {ref}")
            seen_refs.add(ref)
            assessments.append({"finding_ref": ref, "assessment": item["assessment"],
                                "evidence": item["evidence"], "rationale": item["rationale"],
                                "source_field": "assessments", "source_index": index})
    elif version == 1:
        mappings = {} if interpretation is None else {item["source_index"]: item for item in interpretation["entries"]}
        for index, item in enumerate(result["findings"]):
            key = item["id"]
            if key in prior:
                mapped = mappings.get(index)
                if not mapped or mapped.get("finding_ref") != key or mapped.get("assessment") not in ("open", "reverified-addressed"):
                    raise OutputInvalid(f"Ambiguous reused finding ID {key} in {stage}")
                if digest(canonical(item)) != mapped.get("source_item_sha256"):
                    raise OutputInvalid(f"Saved assessment text changed for {key}")
                if key in seen_refs:
                    raise OutputInvalid(f"Conflicting assessments for finding {key}")
                seen_refs.add(key)
                assessments.append({"finding_ref": key, "assessment": mapped["assessment"],
                                    "evidence": item["evidence"], "rationale": item["requested_change"],
                                    "source_field": "findings", "source_index": index})
            else:
                if key in seen_new or index in mappings:
                    raise OutputInvalid(f"Ambiguous new finding identity {key}")
                seen_new.add(key)
                new.append({**item, "source_index": index})
        if set(mappings) != {a["source_index"] for a in assessments if a["source_field"] == "findings"}:
            raise OutputInvalid("Interpretation contains unmatched finding entries")
        for index, item in enumerate(result["dispositions"]):
            ref = item["id"]
            if ref not in prior and ref not in seen_new:
                raise OutputInvalid(f"Unknown finding disposition {ref}")
            if ref in seen_refs:
                raise OutputInvalid(f"Conflicting assessments for finding {ref}")
            seen_refs.add(ref)
            assessments.append({"finding_ref": ref, "assessment": "addressed",
                                "evidence": item["rationale"], "rationale": item["rationale"],
                                "source_field": "dispositions", "source_index": index})
    else:
        raise OutputInvalid("Unsupported finding contract version")
    # One source item cannot claim two meanings; duplicate references in the
    # legacy result are allowed only when they are separate, explicit assessments.
    output_hash = digest(canonical(result))
    for item in assessments:
        ref = item["finding_ref"]
        original = prior.get(ref)
        item.update(stage=stage, operation_id=packet["operation_id"],
                    packet_sha256=digest(canonical(packet)), result_sha256=output_hash,
                    original_evidence_sha256=digest(original["evidence"]) if original else None)
    prior_open = {key for key, value in prior.items() if value["status"] == "open"}
    covered = {item["finding_ref"] for item in assessments}
    missing = [dict(prior[key]) for key in prior if key in prior_open - covered] if stage.startswith("revision") else []
    return {"new": new, "assessments": assessments, "missing": missing,
            "protocol_version": version, "result_sha256": output_hash}


def apply(prior, classified, ref, stage, reviewed):
    findings = {key: dict(value) for key, value in prior.items()}
    for item in classified["new"]:
        value = {key: val for key, val in item.items() if key != "source_index"}
        findings[value["id"]] = {**value, "status": "open", "origin": ref["id"],
                                 "reviewed": reviewed}
    for item in classified["assessments"]:
        key = item["finding_ref"]
        # Legacy author dispositions retain their historical semantics. A
        # critic's reassessment and all v2 claims never silently resolve work.
        if classified["protocol_version"] == 1 and item["source_field"] == "dispositions" and stage.startswith("revision"):
            findings[key]["status"] = "addressed"
            findings[key]["rationale"] = item["rationale"]
    return findings


def record(run_id, stage, classified):
    path = records.run_dir(run_id) / "quality" / (stage + "-finding-ledger.json")
    atomic(path, {"schema_version": 1, "stage": stage,
                  "result_sha256": classified["result_sha256"],
                  "new_findings": classified["new"],
                  "assessments": classified["assessments"],
                  "missing_assessments": classified["missing"]}, immutable=True)
    return path
