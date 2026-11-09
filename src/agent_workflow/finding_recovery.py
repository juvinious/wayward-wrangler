"""Explicit interpretation of one saved, completed legacy critique."""
import getpass
from pathlib import Path
from . import artifacts, findings, records
from .storage import OperationalError, atomic, canonical, digest, now, read

STAGE = "critique-2"
CORRECTION = "finding-protocol-v2: saved legacy entries explicitly refer to existing findings"
EVIDENCE = ("operation.json", "packet.json", "result.json", "receipt.json", "claim.json",
            "permit.json", "dispatch.json", "child.json", "binding.json", "invocations.json",
            "native-events.jsonl", "output-schema.json", "prompt.txt", "execution.json")


def require(condition, reason):
    if not condition:
        raise OperationalError("Saved critique interpretation refused: " + reason)


def inspect(run_id, state):
    from .herdr_backend import receipt, validate_result
    from .adapters import real
    from .context import check
    run = records.get_run(run_id)
    directory = records.run_dir(run_id) / "operations" / STAGE
    require(state.get("next_stage") == STAGE and state.get("completed") == ["draft", "critique-1", "revision-1"],
            "graph is not at saved critique boundary")
    require(not any(x["stage"] == "revision-2" for x in records.operations(run_id)), "later worker already reserved")
    require(read(records.run_dir(run_id) / "inputs/envelope.json") == run["envelope"], "frozen run input differs")
    check(run["envelope"])
    packet = read(directory / "packet.json")
    require(packet == artifacts.packet(run["envelope"], state, STAGE), "graph replay packet differs")
    require(packet.get("finding_contract_version", 1) == 1, "saved output is not legacy finding format")
    op = read(directory / "operation.json")
    require(op["run_id"] == run_id and op["stage"] == STAGE and op["operation_id"] == packet["operation_id"]
            and op["packet_sha256"] == digest(canonical(packet)), "operation identity differs")
    require(any(x["stage"] == STAGE and x["packet_hash"] == op["packet_sha256"] for x in records.operations(run_id)),
            "reserved operation differs")
    done = receipt(directory)
    require(done and done["status"] == "completed", "native completion is not validated")
    require(read(directory / "invocations.json")["count"] == 1, "native invocation count differs")
    require(not (directory / "uncertain.json").exists() and not (directory / "cancel.json").exists(),
            "uncertain or cancelled execution cannot be interpreted")
    require(read(directory / "output-schema.json") == real.schema(packet), "frozen schema differs")
    prompt = (directory / "prompt.txt").read_text()
    require(prompt == real.prompt(packet) and read(directory / "execution.json")["prompt_sha256"] == digest(prompt),
            "frozen prompt differs")
    result = read(directory / "result.json")
    validate_result(result, packet)
    require(digest(canonical(result)) == done["result_sha256"], "saved result differs from receipt")
    import json
    events = [json.loads(line) for line in (directory / "native-events.jsonl").read_text().splitlines() if line.strip()]
    for event in events:
        real.inspect_event(packet["provider"], event)
    require(sum(e.get("type") == "turn.completed" for e in events) == 1,
            "native completion event differs")
    rejected = records.run_dir(run_id) / "quality" / (STAGE + "-invalid.json")
    require(rejected.exists() and read(rejected)["condition"] == "output-invalid" and
            read(rejected)["result_sha256"] == done["result_sha256"], "original semantic rejection missing")
    hashes = {name: digest((directory / name).read_bytes()) for name in EVIDENCE}
    hashes["original_rejection"] = digest(rejected.read_bytes())
    return directory, packet, result, done, hashes


def validate_interpretation(prior, packet, result, proposed):
    require(set(proposed) == {"schema_version", "entries", "rationale"} and proposed["schema_version"] == 1,
            "interpretation schema differs")
    require(isinstance(proposed["rationale"], str) and proposed["rationale"].strip(), "interpretation needs rationale")
    entries = proposed["entries"]
    require(isinstance(entries, list), "interpretation entries missing")
    require(all(isinstance(e, dict) and type(e.get("source_index")) is int for e in entries),
            "interpretation indices must be integers")
    collisions = {i for i, item in enumerate(result["findings"]) if item["id"] in prior}
    require({e.get("source_index") for e in entries} == collisions and len(entries) == len(collisions),
            "each collision needs one explicit interpretation")
    for entry in entries:
        require(set(entry) == {"source_index", "finding_ref", "assessment", "source_item_sha256", "quote"},
                "interpretation entry fields differ")
        index = entry["source_index"]
        item = result["findings"][index]
        require(entry["finding_ref"] == item["id"] and entry["finding_ref"] in prior,
                "reference does not match existing finding")
        require(entry["assessment"] == "open", "saved critique may not resolve a finding")
        require(entry["source_item_sha256"] == digest(canonical(item)), "saved item changed")
        require(entry["quote"] and entry["quote"] in item["evidence"] + " " + item["requested_change"],
                "interpretation quote is not in saved text")
    classified = findings.classify(STAGE, prior, packet, result, proposed)
    require(len(classified["assessments"]) == len(entries) + len(result["dispositions"]),
            "some references were not classified")
    return classified


def verify_interpretation(run_id, state, packet):
    directory, frozen, result, done, hashes = inspect(run_id, state)
    require(packet == frozen, "worker packet differs from saved packet")
    path = directory / "recovery/finding-interpretation.json"
    record = read(path)
    require(record["evidence_hashes"] == hashes and record["result_sha256"] == done["result_sha256"]
            and record["correction"] == CORRECTION and record["result"] == "interpreted-assessments",
            "recovery provenance changed")
    validate_interpretation(state["findings"], frozen, result, record["interpretation"])
    return record["interpretation"]


def adopt(run_id, interpretation):
    from .workflow import state as graph_state
    snapshot = graph_state(run_id)
    run = records.get_run(run_id)
    directory = records.run_dir(run_id) / "operations" / STAGE
    path = directory / "recovery/finding-interpretation.json"
    if path.exists() and STAGE in snapshot.values.get("completed", []):
        from .herdr_backend import receipt
        record = read(path)
        hashes = {name: digest((directory / name).read_bytes()) for name in EVIDENCE}
        rejected = records.run_dir(run_id) / "quality" / (STAGE + "-invalid.json")
        hashes["original_rejection"] = digest(rejected.read_bytes())
        done = receipt(directory)
        require(done and done["status"] == "completed" and record["evidence_hashes"] == hashes
                and record["result_sha256"] == done["result_sha256"] and record["correction"] == CORRECTION,
                "saved recovery evidence changed after adoption")
        return record
    require(snapshot.next == ("worker",), "graph is not waiting at saved critique worker")
    directory, packet, result, done, hashes = inspect(run_id, snapshot.values)
    if path.exists():
        record = read(path)
        require(record["interpretation"] == interpretation, "repeated interpretation differs")
        verify_interpretation(run_id, snapshot.values, packet)
    else:
        require(run["status"] == "output-invalid", "run is not stopped at output-invalid")
        classified = validate_interpretation(snapshot.values["findings"], packet, result, interpretation)
        require(not classified["new"], "saved critique contains a new finding; inspect separately")
        record = {"schema_version": 1, "operation_id": packet["operation_id"],
                  "evidence_hashes": hashes, "result_sha256": done["result_sha256"],
                  "interpretation": interpretation, "correction": CORRECTION,
                  "result": "interpreted-assessments", "next_contract_version": 2,
                  "recorded_by": getpass.getuser(), "time": now(),
                  "authorization": "Operator authorized recovery of this saved critique without another critic call"}
        atomic(path, record, immutable=True)
    if records.get_run(run_id)["status"] == "output-invalid":
        records.set_status(run_id, "recovering", "Saved critique interpreted as existing-finding assessments; no critic retry")
    return record
