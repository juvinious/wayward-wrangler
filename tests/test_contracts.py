"""Offline contract tests. No Herdr writes and no worker/provider execution."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from agent_workflow import artifacts, records
from agent_workflow.herdr_backend import validate_result
from agent_workflow.storage import OperationalError, atomic, canonical, digest

class Contracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
    def tearDown(self):
        self.temp.cleanup()

    def test_immutable_publication_replay_and_conflict(self):
        p=self.root/"artifact.md"
        atomic(p,"same",immutable=True)
        atomic(p,"same",immutable=True)
        with self.assertRaises(OperationalError):
            atomic(p,"different",immutable=True)
        self.assertEqual(p.read_text(),"same")

    def test_artifact_detects_mutation_and_path_escape(self):
        p=self.root/"draft.md"
        p.write_text("original")
        ref={"path":"draft.md","sha256":digest("original")}
        with patch.object(artifacts,"run_dir",return_value=self.root):
            self.assertEqual(artifacts.content("run",ref),"original")
            p.write_text("changed")
            with self.assertRaises(OperationalError):
                artifacts.content("run",ref)
            with self.assertRaises(OperationalError):
                artifacts.content("run",{"path":"../escape","sha256":digest("")})

    def test_worker_cannot_approve_or_change_input_identity(self):
        packet={"operation_id":"operation"}
        good={"schema_version":1,"operation_id":"operation","packet_sha256":digest(canonical(packet)),
              "content":"text","findings":[],"dispositions":[],"questions":[],"needs_revision":False}
        validate_result(good,packet)
        for extra in ({"approval":"yes"},{"operation_id":"different"},{"packet_sha256":"wrong"}):
            with self.assertRaises(OperationalError):
                validate_result({**good,**extra},packet)

    def test_worker_cannot_accept_substantive_risk(self):
        packet={"operation_id":"operation"}
        bad={"schema_version":1,"operation_id":"operation","packet_sha256":digest(canonical(packet)),
             "content":"text","findings":[],"dispositions":[{"id":"F1","status":"accepted-risk","rationale":"agent chose"}],
             "questions":[],"needs_revision":False}
        with self.assertRaises(OperationalError):
            validate_result(bad,packet)

    def test_operation_ids_cannot_escape_local_paths(self):
        for value in ("../other","a/b","", "name with space"):
            with self.assertRaises(OperationalError):
                records.name(value)

if __name__=="__main__":
    unittest.main()
