"""Offline quality and adoption boundaries; no Herdr or provider calls."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from agent_workflow import artifacts, quality, quality_recovery, workflow
from agent_workflow.storage import OperationalError, atomic, digest


class RevisionQuality(unittest.TestCase):
    def setUp(self):
        self.findings = {
            key: {"id": key, "status": "open", "origin": "critique-1", "evidence": "original " + key,
                  "requested_change": "Keep provisional", "reviewed": {"id": "draft"}, "severity": "substantive"}
            for key in ("C1-1", "C1-2", "C1-3")}
        self.result = {"findings": [], "dispositions": [{"id": "C1-1", "status": "addressed", "rationale": "done"}],
                       "questions": [], "needs_revision": False}

    def test_missing_dispositions_are_provisional_and_keep_original_evidence(self):
        gaps = quality.revision_coverage("revision-1", self.findings, self.result)
        self.assertEqual([f["id"] for f in gaps], ["C1-2", "C1-3"])
        self.assertEqual(gaps[0]["evidence"], "original C1-2")
        self.assertEqual([f["status"] for f in gaps], ["open", "open"])
        state = workflow.initial("fixture")
        state.update(next_stage="critique-2", current={"id":"revision-1", "sha256":"hash"},
                     last_reviewed={"id":"draft", "sha256":"old"}, findings=self.findings,
                     coverage_gaps={"revision-1": gaps})
        env = {"run_id":"fixture", "brief":"brief", "instructions":[], "skills":{"critique":"critique"},
               "config":{"roles":{"critic":{"provider":"fake", "model":"fixture"}}},
               "rules_sha256":"r", "config_sha256":"c"}
        with patch.object(artifacts, "content", side_effect=["new", "old"]):
            packet = artifacts.packet(env, state, "critique-2")
        self.assertEqual(packet["revision_coverage"]["revision-1"], gaps)
        self.assertEqual([f["id"] for f in packet["open_findings"]], ["C1-1", "C1-2", "C1-3"])
        self.assertIn("omitted dispositions", packet["authority"])

    def test_unknown_or_duplicate_disposition_is_invalid(self):
        for ids in (["unknown"], ["C1-1", "C1-1"]):
            changed = {**self.result, "dispositions": [{"id": i, "status": "addressed", "rationale": "x"} for i in ids]}
            with self.assertRaises(OperationalError):
                quality.revision_coverage("revision-1", self.findings, changed)

    def test_critic_reusing_an_existing_id_is_output_invalid(self):
        changed = {**self.result, "findings": [{"id":"C1-2", "severity":"substantive",
                        "evidence":"new text", "requested_change":"new request"}]}
        with self.assertRaisesRegex(OperationalError, "Duplicate finding ID"):
            quality.revision_coverage("critique-2", self.findings, changed)

    def test_second_critique_cannot_skip_needed_revision(self):
        state = workflow.initial("fixture")
        state.update(workflow_version=2, next_stage="critique-2", completed=["draft", "critique-1", "revision-1"],
                     inherited_decisions=[], questions_asked=1, revision_required=False,
                     coverage_gaps={"revision-1": quality.revision_coverage("revision-1", self.findings, self.result)})
        result = {**self.result, "operation_id":"fixture-critique-2", "content":"candidate"}
        change = workflow.context_change(state, result, {"id":"critique-2"}, "packet", self.findings)
        self.assertEqual(change["next_stage"], "revision-2")
        self.assertEqual(change["completed"][-1], "critique-2")
        self.assertNotIn("critique-3", change["completed"])


class QualityAdoption(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.directory = Path(self.tmp.name) / "operations/revision-1"
        self.gaps = [{"id":"C1-2", "status":"open", "evidence":"original"},
                     {"id":"C1-3", "status":"open", "evidence":"original"}]
        atomic(self.directory / "packet.json", {"stage":"revision-1"})
        self.evidence = {"packet.json":digest((self.directory / "packet.json").read_bytes())}
        self.done = {"status":"completed", "result_sha256":"saved-hash"}
        self.patches = [patch.object(quality_recovery, "inspect", return_value=(self.directory,self.gaps,self.evidence,self.done)),
                        patch.object(quality_recovery.records,"get_run",return_value={"status":"uncertain"}),
                        patch.object(quality_recovery.records,"run_dir",return_value=Path(self.tmp.name)),
                        patch.object(quality_recovery.records,"set_status"),
                        patch.object(quality_recovery,"EVIDENCE",("packet.json",))]
        for p in self.patches: p.start()

    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.tmp.cleanup()

    def test_append_only_restart_and_repeated_adoption(self):
        original = quality_recovery.adopt("decision-log-01")
        path = self.directory / "recovery/quality-adoption.json"
        before = path.read_bytes()
        with patch.object(quality_recovery, "verify_adoption", return_value=original), \
             patch.object(workflow, "state", return_value=type("S",(),{"values":{}})()), \
             patch("agent_workflow.herdr_backend.receipt",return_value=self.done):
            self.assertEqual(quality_recovery.adopt("decision-log-01"), original)
        self.assertEqual(path.read_bytes(), before)

    def test_mismatched_evidence_refused(self):
        quality_recovery.adopt("decision-log-01")
        with patch.object(quality_recovery,"inspect",return_value=(self.directory,self.gaps,{"result.json":"changed"},self.done)):
            with self.assertRaisesRegex(OperationalError,"evidence"):
                quality_recovery.verify_adoption("decision-log-01", "revision-1", {}, {}, self.gaps)

    def test_restart_between_record_and_status_recovers_without_dispatch(self):
        with patch.object(quality_recovery.records, "set_status", side_effect=RuntimeError("crash")):
            with self.assertRaisesRegex(RuntimeError, "crash"):
                quality_recovery.adopt("decision-log-01")
        with patch.object(quality_recovery,"verify_adoption",return_value=True), \
             patch.object(workflow,"state",return_value=type("S",(),{"values":{}})()), \
             patch("agent_workflow.herdr_backend.receipt",return_value=self.done):
            quality_recovery.adopt("decision-log-01")
        quality_recovery.records.set_status.assert_called_with("decision-log-01", "recovering", unittest.mock.ANY)

    def test_budget_exhaustion_refuses_sixth_dispatch(self):
        from agent_workflow.runner import reserve_native_attempt
        root = Path(self.tmp.name) / "budget"
        for i in range(5):
            directory = root / "operations" / f"stage-{i}"
            atomic(directory / "invocations.json", {"operation_id": f"op-{i}", "count": 1})
        sixth = root / "operations/stage-5"
        with self.assertRaisesRegex(OperationalError, "invocation limit"):
            reserve_native_attempt(sixth, {"operation_id": "op-5", "max_invocations": 5})
        self.assertFalse((sixth / "invocations.json").exists())
        with self.assertRaisesRegex(OperationalError, "Duplicate dispatch"):
            reserve_native_attempt(root / "operations/stage-4", {"operation_id": "op-4", "max_invocations": 5})


if __name__ == "__main__":
    unittest.main()
