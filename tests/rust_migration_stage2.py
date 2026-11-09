"""Installed-r9 versus Rust pure-policy comparison; no service or worker calls.

Run using the preserved r9 private interpreter with -I. Inputs are saved test
fixtures or in-memory fault variants. Evidence output is ignored and append-only.
"""
import argparse
import copy
import json
from pathlib import Path
import subprocess

from agent_workflow import findings, context
from agent_workflow.storage import canonical, digest


def compare(binary, action, value):
    proc = subprocess.run([str(binary), action], input=json.dumps(value), text=True,
                          capture_output=True, check=True)
    return json.loads(proc.stdout)


def baseline(action, value):
    try:
        if action == "classify":
            output = findings.classify(value["stage"], value["prior"], value["packet"],
                                       value["result"], value.get("interpretation"))
        elif action == "apply":
            output = findings.apply(value["prior"], value["classified"],
                                    {"id": value["ref_id"]}, value["stage"], value["reviewed"])
        elif action == "questions":
            context.validate_questions(value["result"], value["packet"])
            output = {"valid": True}
        elif action == "evidence_ids":
            output = context.evidence_ids(value["packet"])
        elif action == "effective_decisions":
            output = context.effective_decisions(value["state"])
        else:
            raise ValueError(action)
        return {"ok": True, "value": output}
    except Exception as exc:
        return {"ok": False, "error": str(exc)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    root = Path(__file__).parent / "fixtures/finding-protocol"
    saved = json.loads((root / "critique-2-result.json").read_bytes())
    source = json.loads((root / "critique-2-context.json").read_bytes())
    before = {p.name: digest(p.read_bytes()) for p in root.iterdir() if p.is_file()}
    prior = source["prior_findings"]
    packet1 = {"operation_id": source["operation_id"], "schema_version": 2}
    interpretation = {"schema_version": 1, "entries": [
        {"source_index": i, "finding_ref": item["id"], "assessment": "open",
         "source_item_sha256": digest(canonical(item)), "quote": "Keep open"}
        for i, item in enumerate(saved["findings"])], "rationale": "saved references"}
    packet2 = {"operation_id": "fixture-revision-2", "run_id": "fixture",
               "schema_version": 2, "finding_contract_version": 2,
               "decisions": [], "context_snapshot": {"entries": []},
               "finding_dispositions": prior}
    result2 = {"schema_version": 2, "operation_id": packet2["operation_id"],
               "packet_sha256": digest(canonical(packet2)), "content": "Candidate",
               "new_findings": [{"severity": "minor", "evidence": "new behavior",
                                 "requested_change": "Add a check"}],
               "assessments": [{"finding_ref": "C1-2", "assessment": "proposed-addressed",
                                "evidence": "section 3", "rationale": "author claim"}],
               "questions": [], "needs_revision": False}
    cases = []
    def add(name, action, value):
        py = baseline(action, value)
        rs = compare(args.binary, action, value)
        if py != rs:
            raise AssertionError(f"{name}: Python={py!r}; Rust={rs!r}")
        cases.append({"name": name, "action": action, "accepted": py["ok"],
                      "output_sha256": digest(canonical(py))})
        return py
    first = add("v2-new-and-assessment", "classify",
                {"stage": "revision-2", "prior": prior, "packet": packet2, "result": result2})
    assert first["ok"]
    add("v2-apply-preserves-original", "apply", {"stage": "revision-2", "prior": prior,
        "classified": first["value"], "ref_id": "revision-2", "reviewed": {"id":"revision-1"}})
    unknown = copy.deepcopy(result2)
    unknown["assessments"][0]["finding_ref"] = "not-selected"
    add("v2-unknown-ref", "classify", {"stage":"critique-2","prior":prior,"packet":packet2,"result":unknown})
    conflict = copy.deepcopy(result2)
    conflict["assessments"].append(copy.deepcopy(conflict["assessments"][0]))
    add("v2-conflict", "classify", {"stage":"critique-2","prior":prior,"packet":packet2,"result":conflict})
    add("legacy-ambiguous", "classify", {"stage":"critique-2","prior":prior,"packet":packet1,"result":saved})
    add("legacy-interpreted", "classify", {"stage":"critique-2","prior":prior,"packet":packet1,
        "result":saved,"interpretation":interpretation})
    bad = copy.deepcopy(interpretation)
    bad["entries"][0]["source_item_sha256"] = "0" * 64
    add("legacy-tampered-mapping", "classify", {"stage":"critique-2","prior":prior,
        "packet":packet1,"result":saved,"interpretation":bad})
    add("revision-missing-coverage", "classify", {"stage":"revision-1","prior":prior,
        "packet":packet2,"result":{**result2,"assessments":[]}})
    question = {"id":"Q1","question":"Which choice?","options":["A","B"],
                "recommendation":"A","consequences":"A is simpler","topic_key":"choice",
                "rationale":"Brief leaves it open","evidence_refs":["brief"],
                "related_decision_refs":[],"reconsideration_reason":""}
    add("question-valid", "questions", {"packet":packet2,"result":{"questions":[question]}})
    invalid = copy.deepcopy(question)
    invalid["evidence_refs"]=["other-project/private"]
    add("question-unknown-evidence", "questions", {"packet":packet2,"result":{"questions":[invalid]}})
    add("question-duplicate-topic", "questions", {"packet":packet2,"result":{"questions":[question,question]}})
    add("evidence-scoped", "evidence_ids", {"packet":packet2})
    state = {"inherited_decisions":[{"qualified_id":"old/D","answer":"old"}],
             "decisions":[{"request_id":"D2","answer":"new","supersedes":["old/D"]}]}
    add("explicit-supersession", "effective_decisions", {"state":state})
    assert before == {p.name: digest(p.read_bytes()) for p in root.iterdir() if p.is_file()}
    args.evidence.mkdir(parents=True, exist_ok=False)
    (args.evidence / "policy-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline": __import__("agent_workflow").__file__, "cases":cases},indent=2)+"\n")
    print(f"Stage 2 pure-policy parity: {len(cases)} cases passed; source fixtures unchanged")

if __name__ == "__main__": main()
