import socket
import tempfile
import threading
import unittest
from pathlib import Path

from tests.session_fixture import Fixture, read_frame, send


class SessionFixtureTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.fixture = Fixture(Path(self.temp.name) / "session.sock")
        self.thread = threading.Thread(target=self.fixture.serve, daemon=True)
        self.thread.start()
        self.sock = socket.socket(socket.AF_UNIX)
        self.sock.connect(str(self.fixture.path))
        send(self.sock, "hello", "h", {"protocol": {"major": 1}, "release_id": "proof-v1", "project_key": "fixture"})
        self.assertTrue(read_frame(self.sock)["body"]["accepted"])

    def tearDown(self):
        self.sock.close()
        self.fixture.close()
        self.thread.join(timeout=1)
        self.temp.cleanup()

    def request(self, kind, ident, body):
        send(self.sock, kind, ident, body)
        return read_frame(self.sock)

    def test_idempotency_stale_and_replay(self):
        command = {"command_id": "c1", "action": "submit_message", "expected_state_revision": 0,
                   "payload": {"text": "first"}}
        self.assertEqual(self.request("command", "1", command)["body"]["status"], "accepted")
        self.assertEqual(self.request("command", "2", command)["body"]["status"], "accepted")
        self.assertEqual(self.request("command", "3", {**command, "payload": {"text": "other"}})["body"]["code"], "idempotency_conflict")
        self.assertEqual(self.request("command", "4", {**command, "command_id": "c2"})["body"]["code"], "stale_state")
        self.assertEqual(self.request("events_read", "5", {"after_seq": 0, "limit": 64})["body"]["events"][0]["text"], "first")
        self.assertEqual(self.request("command_get", "6", {"command_id": "c1"})["body"]["status"], "accepted")

    def test_foreign_project_refused(self):
        other = socket.socket(socket.AF_UNIX)
        other.connect(str(self.fixture.path))
        send(other, "hello", "h2", {"protocol": {"major": 1}, "release_id": "proof-v1", "project_key": "other"})
        self.assertFalse(read_frame(other)["body"]["accepted"])
        other.close()


if __name__ == "__main__":
    unittest.main()
