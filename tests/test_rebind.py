"""Routing tests use temporary SQLite and mocked Herdr; no real workflow execution."""
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch
from agent_workflow import records, routing, workflow
from agent_workflow.storage import OperationalError, atomic

class Rebind(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.local=Path(self.tmp.name)
        self.patches=[
            patch.object(records,"LOCAL",self.local),
            patch.object(records,"DB",self.local/"state.sqlite"),
            patch.object(workflow,"state"),
            patch.object(routing,"Herdr"),
            patch.object(routing.artifacts,"content",return_value="candidate")]
        self.mocks=[p.start() for p in self.patches]
        self.envelope={"run_id":"test","target":{"socket":"/fixture.sock","workspace":"old"}}
        records.create_run(self.envelope)
        records.set_status("test","awaiting-input")
        self.snapshot=SimpleNamespace(values={"pending":{"request_id":"D1"},"completed":[],"current":{"sha256":"hash"}},
                                      interrupts=(object(),),config={"configurable":{"checkpoint_id":"fixture"}})
        self.mocks[2].return_value=self.snapshot
        h=self.mocks[3].return_value
        h.call.side_effect=lambda method,params=None: {"workspace":{"label":"project"}} if method=="workspace.get" else {"tabs":[]}
        h.server_identity.return_value={"pid":1,"start":"1","boot":"fixture"}

    def tearDown(self):
        for p in reversed(self.patches):p.stop()
        self.tmp.cleanup()

    def test_rebind_is_append_only_idempotent_and_changes_only_future_target(self):
        before=records.get_run("test")
        result=routing.rebind("test","new")
        self.assertEqual(result["status"],"rebound")
        self.assertEqual(records.get_run("test"),before)
        self.assertEqual(records.effective_target("test")["workspace"],"new")
        self.assertEqual(routing.rebind("test","new")["status"],"already-bound")
        with records.connect() as db:
            self.assertEqual(db.execute("SELECT count(*) FROM app_rebindings").fetchone()[0],1)
        self.assertIsNone(records.response("test","D1"))
        with patch.object(workflow.artifacts,"packet",return_value={"operation_id":"test-draft"}), \
             patch.object(workflow,"prepare",side_effect=RuntimeError("stop-before-dispatch")) as prepare:
            with self.assertRaisesRegex(RuntimeError,"stop-before-dispatch"):
                workflow.worker({"run_id":"test","next_stage":"draft","completed":[]})
            self.assertEqual(prepare.call_args.args[2]["workspace"],"new")

    def test_answered_gate_is_not_consumed_or_rebound(self):
        records.save_response("test",{"request_id":"D1","answer":"yes","outcome":"answered","artifact_sha256":"hash","decision_revision":0})
        with self.assertRaisesRegex(OperationalError,"already answered"):
            routing.rebind("test","new")
        self.assertIsNone(records.latest_rebinding("test"))

    def test_incomplete_reserved_operation_is_refused(self):
        records.reserve("test","draft","op","hash")
        with self.assertRaisesRegex(OperationalError,"incomplete"):
            routing.rebind("test","new")
        self.assertEqual(records.effective_target("test")["workspace"],"old")

    def test_existing_controller_is_preserved(self):
        with patch.object(routing,"controller_panes",return_value=["old:p1"]):
            with self.assertRaisesRegex(OperationalError,"Exit the existing"):
                routing.rebind("test","new")
        self.assertIsNone(records.latest_rebinding("test"))

    def test_live_worker_is_refused_even_with_receipt(self):
        records.reserve("test","draft","op","hash")
        self.snapshot.values["completed"]=["draft"]
        directory=records.run_dir("test")/"operations/draft"
        atomic(directory/"claim.json",{"runner":{"pid":123}})
        with patch.object(routing,"receipt",return_value={"status":"completed"}), \
             patch.object(routing,"alive",return_value=True):
            with self.assertRaisesRegex(OperationalError,"Live execution"):
                routing.rebind("test","new")
        self.assertIsNone(records.latest_rebinding("test"))

if __name__=="__main__":
    unittest.main()
