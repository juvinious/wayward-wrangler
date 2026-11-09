"""Explicit, append-only adoption of a completed but previously rejected revision."""
import json
import getpass
from . import artifacts, records
from .storage import OperationalError, atomic, canonical, digest, now, read
from .quality import revision_coverage

RUN = "decision-log-01"
STAGE = "revision-1"
FAILURE = "Revision omitted an open finding"
CORRECTION = "coverage-policy-v1: retain omitted findings as open and continue provisionally"
EVIDENCE = ("operation.json", "packet.json", "result.json", "receipt.json", "claim.json",
            "permit.json", "dispatch.json", "child.json", "binding.json", "invocations.json",
            "native-events.jsonl", "output-schema.json", "prompt.txt", "execution.json")


def require(condition, reason):
    if not condition:
        raise OperationalError("Revision recovery refused: " + reason)


def inspect(run_id):
    from . import workflow
    from .herdr_backend import receipt, validate_result
    from .adapters import real
    from .storage import alive
    require(run_id == RUN, "only the authorized decision-log-01 revision is eligible")
    run = records.get_run(run_id)
    directory = records.run_dir(run_id) / "operations" / STAGE
    snapshot = workflow.state(run_id)
    require(snapshot.values.get("next_stage") == STAGE and snapshot.values.get("completed") == ["draft", "critique-1"]
            and snapshot.next == ("worker",), "graph is not at the original rejected revision boundary")
    require(not any(x["stage"] in ("critique-2", "revision-2") for x in records.operations(run_id)),
            "later stage already reserved")
    require(run["envelope"].get("native_authorization") and run["envelope"]["config"]["max_invocations"] == 5,
            "frozen native authorization differs")
    require(read(records.run_dir(run_id) / "inputs/envelope.json") == run["envelope"], "frozen envelope mismatch")
    from .context import check
    check(run["envelope"])
    expected = artifacts.packet(run["envelope"], snapshot.values, STAGE)
    packet = read(directory / "packet.json")
    require(packet == expected, "frozen packet differs from graph replay")
    op = read(directory / "operation.json")
    require(op["run_id"] == run_id and op["stage"] == STAGE and op["operation_id"] == run_id + "-" + STAGE
            and op["packet_sha256"] == digest(canonical(packet)), "operation identity mismatch")
    require(any(x["stage"] == STAGE and x["operation_id"] == op["operation_id"] and x["packet_hash"] == op["packet_sha256"]
                for x in records.operations(run_id)), "stage reservation mismatch")
    done = receipt(directory)
    require(done and done["status"] == "completed", "validated native completion missing")
    require(read(directory / "invocations.json")["count"] == 1, "revision invocation count differs")
    require(not alive(read(directory / "child.json")["identity"]), "original native child is still active")
    require(not (directory / "uncertain.json").exists() and not (directory / "cancel.json").exists(),
            "uncertain or cancelled execution cannot be adopted")
    result = read(directory / "result.json")
    validate_result(result, packet)
    missing = revision_coverage(STAGE, snapshot.values["findings"], result)
    require([f["id"] for f in missing] == ["C1-2", "C1-3"], "omitted finding identities differ")
    require(all(f["status"] == "open" for f in missing), "omitted finding was resolved")
    require(digest(canonical(result)) == done["result_sha256"], "receipt does not bind result")
    require(read(directory / "output-schema.json") == real.schema(packet), "frozen output schema differs")
    prompt = (directory / "prompt.txt").read_text()
    require(prompt == real.prompt(packet) and read(directory / "execution.json")["prompt_sha256"] == digest(prompt),
            "frozen native input differs")
    require(packet["provider"] == "claude" and packet["execution"]["reasoning"] == "medium"
            and packet["model"] == "sonnet", "frozen provider settings differ")
    # The saved stream must contain the same single successful structured result.
    events = [json.loads(line) for line in (directory / "native-events.jsonl").read_text().splitlines() if line.strip()]
    for event in events:
        real.inspect_event("claude", event)
    finals = [e for e in events if e.get("type") == "result"]
    require(len(finals) == 1 and finals[0].get("subtype") == "success" and not finals[0].get("is_error")
            and finals[0].get("structured_output") == result, "native final event differs from saved output")
    require(json.loads(finals[0]["result"]) == result, "native final representations differ")
    require(run["detail"] == FAILURE or (directory / "recovery/quality-adoption.json").exists(),
            "original rejection is not recorded")
    hashes = {name: digest((directory / name).read_bytes()) for name in EVIDENCE}
    return directory, missing, hashes, done


def verify_adoption(run_id, stage, state, packet, missing):
    require(run_id == RUN and stage == STAGE, "adoption identity differs")
    directory, verified, hashes, done = inspect(run_id)
    record = read(directory / "recovery/quality-adoption.json")
    require(record["evidence_hashes"] == hashes and record["result_sha256"] == done["result_sha256"]
            and record["omitted_findings"] == verified == missing and record["correction"] == CORRECTION
            and record["original_rejection"] == FAILURE, "adoption record differs from evidence")
    require(packet == read(directory / "packet.json"), "replayed packet differs")
    return record


def adopt(run_id):
    require(run_id == RUN, "only the authorized decision-log-01 revision is eligible")
    directory = records.run_dir(run_id) / "operations" / STAGE
    path = directory / "recovery/quality-adoption.json"
    if path.exists():
        from . import workflow
        from .herdr_backend import receipt
        record = read(path)
        hashes = {name: digest((directory / name).read_bytes()) for name in EVIDENCE}
        done = receipt(directory)
        require(record["evidence_hashes"] == hashes and done and done["status"] == "completed"
                and record["result_sha256"] == done["result_sha256"] and record["correction"] == CORRECTION,
                "saved adoption evidence changed")
        if STAGE in workflow.state(run_id).values.get("completed", []):
            return record
        _, missing, _, _ = inspect(run_id)
        verify_adoption(run_id, STAGE, workflow.state(run_id).values, read(directory / "packet.json"), missing)
        if records.get_run(run_id)["status"] == "uncertain":
            records.set_status(run_id, "recovering", "Revision incomplete: C1-2 and C1-3 remain open; adopting exact completed output")
        return record
    directory, missing, hashes, done = inspect(run_id)
    require(records.get_run(run_id)["status"] == "uncertain", "run has a different stop status")
    record = {"schema_version": 1, "operation_id": run_id + "-" + STAGE,
              "result_sha256": done["result_sha256"], "evidence_hashes": hashes,
              "omitted_findings": missing, "original_rejection": FAILURE,
              "correction": CORRECTION, "result": "adopted-provisionally",
              "recorded_by": getpass.getuser(), "time": now(),
              "authorization": "Operator authorized this exact saved revision and at most two remaining native stages"}
    atomic(path, record, immutable=True)
    records.set_status(run_id, "recovering", "Revision incomplete: C1-2 and C1-3 remain open; adopting exact completed output")
    return record
