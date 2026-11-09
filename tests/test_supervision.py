"""Offline infrastructure recovery; no model or shared Herdr server."""
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from agent_workflow import records, routing, ownership, supervision, workflow, herdr_backend
from agent_workflow.runtime import RuntimePaths, bind, unbind
from agent_workflow.storage import OperationalError, atomic, canonical, digest, read


class Supervision(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.project = base / "project"
        self.project.mkdir()
        self.state = base / "state"
        self.bound = bind(RuntimePaths(self.project, self.state, base / "registry", "a" * 32))
        self.run = "fixture"
        records.create_run({"run_id": self.run, "target": {"socket": str(base / "herdr.sock"), "workspace": "old"}})
        self.server = {"pid": 10, "start": "2", "boot": "b"}
        self.snapshot = SimpleNamespace(values={"pending": None, "completed": ["draft"],
                                         "current": {"id": "draft", "sha256": "hash"}}, interrupts=(), config={})
        self.packet = {"schema_version": 1, "run_id": self.run, "operation_id": "fixture-draft",
                       "stage": "draft", "role": "author", "model": "fake", "provider": "fake"}
        self._operation("draft", self.packet)
        atomic(records.run_dir(self.run) / "operations/draft/receipt.json", {"status": "completed"}, immutable=True)
        # A completed draft's receipt is mocked only because this suite tests
        # routing and ownership, not the receipt validator itself.
        self.packet2 = {**self.packet, "operation_id": "fixture-critique-1", "stage": "critique-1",
                        "role": "critic"}
        self._operation("critique-1", self.packet2)
        records.set_status(self.run, "uncertain", "Herdr workspace.get: {'code': 'workspace_not_found'}")

    def _operation(self, stage, packet):
        records.reserve(self.run, stage, packet["operation_id"], digest(canonical(packet)))
        directory = records.run_dir(self.run) / "operations" / stage
        atomic(directory / "packet.json", packet, immutable=True)
        atomic(directory / "operation.json", {"operation_id": packet["operation_id"],
               "run_id": self.run, "stage": stage, "packet_sha256": digest(canonical(packet)),
               "target": records.effective_target(self.run), "launch_token": stage}, immutable=True)

    def tearDown(self):
        unbind(self.bound)
        self.tmp.cleanup()

    def _herdr(self):
        class Fake:
            path = "fixture"
            def server_identity(_self): return self.server
            def find(_self, token): return []
            def call(_self, method, params=None):
                if method == "workspace.get":
                    if params["workspace_id"] == "old":
                        raise OperationalError("Herdr workspace.get: {'code': 'workspace_not_found'}")
                    return {"workspace": {"workspace_id": "new", "label": "replacement"}}
                if method == "tab.list": return {"tabs": []}
                raise AssertionError(method)
        return Fake()

    def test_missing_workspace_rebinds_only_prepared_stage(self):
        with patch.object(workflow, "state", return_value=self.snapshot), \
             patch.object(routing, "Herdr", return_value=self._herdr()), \
             patch.object(routing, "receipt", side_effect=lambda p: {"status": "completed"} if p.name == "draft" else None), \
             patch.object(routing.artifacts, "content", return_value="draft"), \
             patch.object(routing, "controller_panes", return_value=[]):
            result = routing.recover_missing_workspace(self.run, "new", generation=1)
        self.assertEqual(result["status"], "recovered")
        self.assertEqual(records.effective_target(self.run)["workspace"], "new")
        self.assertEqual(read(records.run_dir(self.run) / "operations/critique-1/operation.json")["target"]["workspace"], "old")
        self.assertFalse((records.run_dir(self.run) / "operations/critique-1/launch.json").exists())

    def test_execution_evidence_refuses_rebind(self):
        atomic(records.run_dir(self.run) / "operations/critique-1/launch.json", {"status": "may-have-been-sent"})
        with patch.object(workflow, "state", return_value=self.snapshot), \
             patch.object(routing, "Herdr", return_value=self._herdr()), \
             patch.object(routing, "receipt", side_effect=lambda p: {"status": "completed"} if p.name == "draft" else None):
            with self.assertRaisesRegex(OperationalError, "execution evidence"):
                routing.recover_missing_workspace(self.run, "new", generation=1)
        self.assertEqual(records.effective_target(self.run)["workspace"], "old")

    def test_stale_controller_generation_refuses_dispatch(self):
        ownership.next_owner(run_id=self.run, token="first", server=self.server, workspace="old")
        token = ownership.bind(1)
        ownership.check()
        ownership.next_owner(run_id=self.run, token="second", server=self.server, workspace="new")
        with self.assertRaisesRegex(OperationalError, "Stale"):
            ownership.check()
        ownership.unbind(token)

    def test_supervisor_restart_reuses_live_controller(self):
        ownership.next_owner(run_id=self.run, token="existing", server=self.server, workspace="old")
        records.set_status(self.run, "awaiting-input")
        pane = {"pane_id": "old:p1", "tab_id": "old:t1", "workspace_id": "old"}
        class Fake:
            def server_identity(_self): return self.server
        with patch.object(supervision, "Herdr", return_value=Fake()), \
             patch.object(supervision, "controller_matches", return_value=[pane]), \
             patch.object(supervision, "_workspace") as workspace, \
             patch.object(supervision, "_launch_controller") as launch:
            self.assertEqual(supervision.reconcile_once(self.run, records.effective_target(self.run)["socket"]),
                             "controller-present")
        workspace.assert_not_called()
        launch.assert_not_called()
        self.assertEqual(ownership.current()["generation"], 1)

    def test_unmanaged_controller_prevents_duplicate_takeover(self):
        records.set_status(self.run, "awaiting-input")
        class Fake:
            def server_identity(_self): return self.server
        with patch.object(supervision, "Herdr", return_value=Fake()), \
             patch.object(supervision, "controller_panes", return_value=["old:p1"]), \
             patch.object(supervision, "_launch_controller") as launch:
            with self.assertRaisesRegex(OperationalError, "unmanaged controller"):
                supervision.reconcile_once(self.run, records.effective_target(self.run)["socket"])
        launch.assert_not_called()

    def test_controller_loss_creates_one_new_generation(self):
        ownership.next_owner(run_id=self.run, token="lost", server=self.server, workspace="old")
        records.set_status(self.run, "awaiting-input")
        class Fake:
            path = "fixture"
            def server_identity(_self): return self.server
            def create(_self, workspace, label, command, cwd):
                return {"tab_id": "old:t2", "pane_id": "old:p2", "workspace_id": workspace}
        with patch.object(supervision, "Herdr", return_value=Fake()), \
             patch.object(supervision, "controller_matches", return_value=[]), \
             patch.object(supervision, "_workspace", return_value=({"workspace_id": "old"}, False)):
            self.assertEqual(supervision.reconcile_once(self.run, records.effective_target(self.run)["socket"]),
                             "controller-created")
        self.assertEqual(ownership.current()["generation"], 2)
        self.assertEqual(len(list((self.state / "supervision/controllers").glob("*/intent.json"))), 1)

    def test_changed_server_identity_refuses_workspace_id_reuse(self):
        ownership.next_owner(run_id=self.run, token="old", server=self.server, workspace="old")
        records.set_status(self.run, "awaiting-input")
        class Fake:
            def server_identity(_self): return {**self.server, "start": "different"}
        with patch.object(supervision, "Herdr", return_value=Fake()), \
             patch.object(supervision, "_workspace") as workspace, \
             patch.object(supervision, "_launch_controller") as launch:
            with self.assertRaisesRegex(OperationalError, "server identity changed"):
                supervision.reconcile_once(self.run, records.effective_target(self.run)["socket"])
        workspace.assert_not_called()
        launch.assert_not_called()

    def test_exhausted_reconciliation_is_durable_and_bounded(self):
        with patch.object(supervision, "reconcile_once", side_effect=OperationalError("worker report missing")) as attempt, \
             patch.object(supervision.time, "sleep"):
            result = supervision.serve(self.run, str(self.project / "herdr.sock"), interval=.2, max_attempts=3)
        self.assertEqual(result, "uncertain")
        self.assertEqual(attempt.call_count, 3)
        self.assertEqual(records.get_run(self.run)["status"], "uncertain")
        self.assertEqual(len(list((self.state / "supervision/notices").glob("*.json"))), 1)
        self.assertEqual(len(list((self.state / "supervision/events").glob("*.json"))), 3)

    def test_repeated_controller_replacement_has_persistent_limit(self):
        for index in range(3):
            supervision.event(self.run, "controller-missing", "create-controller", "created", str(index))
        class Fake:
            pass
        with self.assertRaisesRegex(OperationalError, "replacement limit exhausted"):
            supervision._launch_controller(Fake(), self.run, "new", self.server)

    def test_output_invalid_keeps_supervisor_and_replaces_only_controller(self):
        ownership.next_owner(run_id=self.run, token="quality", server=self.server, workspace="old")
        records.set_status(self.run, "output-invalid", "duplicate finding identity")
        class Fake:
            def server_identity(_self): return self.server
            def call(_self, method, params=None):
                if method == "workspace.get": return {"workspace":{"workspace_id":"old"}}
                if method == "notification.show": return {}
                raise AssertionError(method)
        with patch.object(supervision, "Herdr", return_value=Fake()), \
             patch.object(supervision, "controller_matches", return_value=[]), \
             patch.object(supervision, "_launch_controller") as launch:
            self.assertEqual(supervision.reconcile_once(self.run, records.effective_target(self.run)["socket"]),
                             "quality-hold")
        launch.assert_called_once()
        self.assertEqual(records.get_run(self.run)["status"], "output-invalid")
        self.assertEqual(records.active_run(), self.run)

    def test_interpreted_quality_hold_retires_only_owned_controller_once(self):
        ownership.next_owner(run_id=self.run, token="quality", server=self.server, workspace="old")
        records.set_status(self.run, "recovering", "saved critique interpretation")
        atomic(records.run_dir(self.run) / "operations/critique-2/recovery/finding-interpretation.json", {})
        pane = {"pane_id":"old:p1", "tab_id":"old:t1", "workspace_id":"old"}
        class Fake:
            def server_identity(_self): return self.server
        claim = {"process":{"pid":12345,"start":"old","boot":"test"},"pane_id":"old:p1"}
        with patch.object(supervision,"Herdr",return_value=Fake()), \
             patch.object(supervision,"controller_matches",return_value=[pane]), \
             patch.object(supervision,"_controller_claim",return_value=claim), \
             patch.object(supervision,"alive",return_value=True), \
             patch.object(supervision,"snapshot_stage",return_value="critique-2"), \
             patch.object(supervision.os,"kill") as kill:
            self.assertEqual(supervision.reconcile_once(self.run,records.effective_target(self.run)["socket"]),
                             "controller-retiring")
            self.assertEqual(supervision.reconcile_once(self.run,records.effective_target(self.run)["socket"]),
                             "controller-retiring")
        kill.assert_called_once_with(12345,supervision.signal.SIGTERM)


if __name__ == "__main__":
    unittest.main()
