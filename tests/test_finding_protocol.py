"""Saved critique-2 regression and versioned finding contract, offline only."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from agent_workflow import findings, finding_recovery, records
from agent_workflow.adapters import real
from agent_workflow.herdr_backend import validate_result
from agent_workflow.storage import OperationalError, OutputInvalid, canonical, digest, read
from agent_workflow import workflow, supervision
from agent_workflow.runtime import RuntimePaths, bind, unbind

ROOT = Path(__file__).parent / "fixtures/finding-protocol"


class FindingProtocol(unittest.TestCase):
    def setUp(self):
        self.context = read(ROOT / "critique-2-context.json")
        self.saved = read(ROOT / "critique-2-result.json")
        self.packet = {"operation_id": self.context["operation_id"], "schema_version": 2}
        self.prior = copy.deepcopy(self.context["prior_findings"])
        self.interpretation = {
            "schema_version": 1, "entries": [
                {"source_index": i, "finding_ref": item["id"], "assessment": "open",
                 "source_item_sha256": digest(canonical(item)),
                 "quote": ("Keep this finding open." if i < 2 else
                           "Leave open alongside C1-2." if i == 2 else
                           "Leave open alongside C1-3." if i == 3 else
                           "Leave open and explicitly mark cross-tab behavior provisional.")}
                for i, item in enumerate(self.saved["findings"])],
            "rationale": "Saved entries explicitly retain the original issues."}

    def test_saved_output_is_exact_fixture_and_all_entries_are_reassessments(self):
        self.assertEqual(digest((ROOT / "critique-2-result.json").read_bytes()),
                         self.context["source_sha256"]["result.json"])
        classified = finding_recovery.validate_interpretation(self.prior,self.packet,self.saved,self.interpretation)
        self.assertEqual(classified["new"], [])
        self.assertEqual([a["finding_ref"] for a in classified["assessments"]],
                         ["C1-2", "C1-3", "C2-1", "C2-2", "C2-3", "C1-1", "C1-4"])
        self.assertEqual([a["assessment"] for a in classified["assessments"][:5]], ["open"] * 5)

    def test_original_evidence_survives_assessment_and_replay(self):
        before = copy.deepcopy(self.prior)
        first = finding_recovery.validate_interpretation(self.prior,self.packet,self.saved,self.interpretation)
        second = finding_recovery.validate_interpretation(self.prior,self.packet,self.saved,self.interpretation)
        self.assertEqual(first, second)
        ref = {"id":"critique-2"}
        updated = findings.apply(self.prior, first, ref, "critique-2", {"id":"revision-1"})
        self.assertEqual(updated, before)
        self.assertEqual(self.prior, before)
        with tempfile.TemporaryDirectory() as temp, patch.object(records,"run_dir",return_value=Path(temp)):
            path = findings.record("fixture","critique-2",first)
            original = path.read_bytes()
            findings.record("fixture","critique-2",second)
            self.assertEqual(path.read_bytes(),original)

    def test_conflicting_or_unexplained_legacy_reference_refused(self):
        for mutation in ("wrong_ref", "wrong_hash", "wrong_quote", "missing", "new_issue"):
            interpretation = copy.deepcopy(self.interpretation)
            if mutation == "wrong_ref": interpretation["entries"][0]["finding_ref"] = "C1-3"
            if mutation == "wrong_hash": interpretation["entries"][0]["source_item_sha256"] = "0" * 64
            if mutation == "wrong_quote": interpretation["entries"][0]["quote"] = "unrelated behavior"
            if mutation == "missing": interpretation["entries"].pop()
            if mutation == "new_issue": interpretation["entries"][0]["assessment"] = "new"
            with self.subTest(mutation=mutation), self.assertRaises(OperationalError):
                finding_recovery.validate_interpretation(self.prior,self.packet,self.saved,interpretation)
        with self.assertRaisesRegex(OutputInvalid,"Ambiguous reused"):
            findings.classify("critique-2",self.prior,self.packet,self.saved)

    def test_v2_new_identity_and_explicit_assessment(self):
        packet = {"operation_id":"fixture-revision-2","run_id":"fixture","schema_version":2,
                  "finding_contract_version":2,"decisions":[],"context_snapshot":{"entries":[]}}
        result = {"schema_version":2,"operation_id":packet["operation_id"],
                  "packet_sha256":digest(canonical(packet)),"content":"Provisional specification",
                  "new_findings":[{"severity":"minor","evidence":"new browser behavior","requested_change":"Add check"}],
                  "assessments":[{"finding_ref":"C1-2","assessment":"proposed-addressed",
                                  "evidence":"candidate section 3","rationale":"author claims coverage"}],
                  "questions":[],"needs_revision":False}
        self.assertIn("new_findings",real.schema(packet)["properties"])
        self.assertNotIn("findings",real.schema(packet)["properties"])
        validate_result(result,packet)
        one = findings.classify("revision-2",self.prior,packet,result)
        two = findings.classify("revision-2",self.prior,packet,result)
        self.assertEqual(one,two)
        self.assertEqual(one["new"][0]["id"],findings.durable_id(packet["operation_id"],0))
        updated = findings.apply(self.prior,one,{"id":"revision-2"},"revision-2",{"id":"revision-1"})
        self.assertEqual(updated["C1-2"]["status"],"open")
        self.assertEqual(updated["C1-2"]["evidence"],self.prior["C1-2"]["evidence"])
        self.assertIn(one["new"][0]["id"],updated)

    def test_v2_unknown_assessment_reference_refused(self):
        packet = {"operation_id":"fixture-critique","finding_contract_version":2}
        result = {"new_findings":[],"assessments":[{"finding_ref":"does-not-exist",
                 "assessment":"open","evidence":"x","rationale":"y"}]}
        with self.assertRaisesRegex(OutputInvalid,"unknown finding"):
            findings.classify("critique-1",self.prior,packet,result)
        result["assessments"] = [{"finding_ref":"C1-2","assessment":"open","evidence":"x","rationale":"y"},
                                 {"finding_ref":"C1-2","assessment":"proposed-addressed","evidence":"z","rationale":"w"}]
        with self.assertRaisesRegex(OutputInvalid,"Conflicting assessments"):
            findings.classify("critique-1",self.prior,packet,result)

    def test_ambiguous_output_stops_durably_with_actionable_notice(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            project = base / "project"
            project.mkdir()
            state = base / "state"
            token = bind(RuntimePaths(project, state, base / "registry", "a" * 32))
            try:
                records.create_run({"run_id":"ambiguous-fixture", "target":{"socket":"fixture.sock",
                                    "workspace":"fixture-workspace"}})
                packet = {"operation_id":"fixture-critic", "finding_contract_version":2}
                result = {"new_findings":[], "assessments":[
                    {"finding_ref":"C1-2", "assessment":"open", "evidence":"first", "rationale":"first"},
                    {"finding_ref":"C1-2", "assessment":"proposed-addressed", "evidence":"second", "rationale":"second"}]}
                class Graph:
                    def get_state(self, config):
                        return SimpleNamespace(values={"next_stage":"critique-2"}, interrupts=())
                    def invoke(self, argument, config, durability):
                        findings.classify("critique-2", self_prior, packet, result)
                self_prior = self.prior
                with patch("agent_workflow.ownership.check"), patch("agent_workflow.context.check"), \
                     patch.object(workflow, "build", return_value=Graph()):
                    with self.assertRaisesRegex(OutputInvalid, "Conflicting assessments"):
                        workflow.drive("ambiguous-fixture")
                stopped = records.get_run("ambiguous-fixture")
                self.assertEqual(stopped["status"], "output-invalid")
                self.assertIn("C1-2", stopped["detail"])
                class Herdr:
                    def server_identity(self): return {"pid":1,"start":"1","boot":"fixture"}
                    def call(self, method, params=None):
                        if method == "notification.show": return {}
                        raise AssertionError(method)
                with patch.object(supervision, "Herdr", return_value=Herdr()):
                    self.assertEqual(supervision.reconcile_once("ambiguous-fixture", "fixture.sock"), "terminal")
                notices = list((state / "supervision/notices").glob("*.json"))
                self.assertEqual(len(notices), 1)
                self.assertIn("Inspect the saved output", read(notices[0])["operator_action"])
            finally:
                unbind(token)

    def test_saved_adoption_record_replays_idempotently_without_dispatch(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);directory=root/"operations/critique-2"
            snapshot=SimpleNamespace(values={"next_stage":"critique-2","completed":["draft","critique-1","revision-1"],
                                             "findings":self.prior},next=("worker",))
            done={"status":"completed","result_sha256":digest(canonical(self.saved))}
            hashes={"result.json":done["result_sha256"]}
            with patch.object(records,"run_dir",return_value=root), \
                 patch("agent_workflow.finding_recovery.inspect",return_value=(directory,self.packet,self.saved,done,hashes)), \
                 patch("agent_workflow.workflow.state",return_value=snapshot), \
                 patch.object(records,"get_run",return_value={"status":"output-invalid"}), \
                 patch.object(records,"set_status") as status:
                first=finding_recovery.adopt("fixture",self.interpretation)
                path=directory/"recovery/finding-interpretation.json"
                before=path.read_bytes()
                second=finding_recovery.adopt("fixture",self.interpretation)
            self.assertEqual(first,second)
            self.assertEqual(path.read_bytes(),before)
            self.assertEqual(first["result_sha256"],done["result_sha256"])
            self.assertEqual(status.call_count,2)  # safe repeated status write, no graph drive
            self.assertFalse((root/"operations/revision-2").exists())

    def test_recovery_record_tampering_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);directory=root/"operations/critique-2"
            done={"status":"completed","result_sha256":digest(canonical(self.saved))}
            hashes={"result.json":"original"}
            with patch.object(records,"run_dir",return_value=root), \
                 patch("agent_workflow.finding_recovery.inspect",return_value=(directory,self.packet,self.saved,done,hashes)), \
                 patch("agent_workflow.workflow.state",return_value=SimpleNamespace(values={"next_stage":"critique-2",
                            "completed":["draft","critique-1","revision-1"],"findings":self.prior},next=("worker",))), \
                 patch.object(records,"get_run",return_value={"status":"output-invalid"}), \
                 patch.object(records,"set_status"):
                finding_recovery.adopt("fixture",self.interpretation)
                changed=copy.deepcopy(self.interpretation)
                changed["entries"][0]["quote"]="different"
                with self.assertRaisesRegex(OperationalError,"differs"):
                    finding_recovery.adopt("fixture",changed)
                with patch("agent_workflow.finding_recovery.inspect",return_value=(directory,self.packet,self.saved,done,{"result.json":"tampered"})):
                    with self.assertRaisesRegex(OperationalError,"provenance"):
                        finding_recovery.verify_interpretation("fixture",{"findings":self.prior},self.packet)


if __name__ == "__main__":
    unittest.main()
