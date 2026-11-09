"""No Herdr server needed: verify only identity-safe UI launch transitions."""
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from agent_workflow import shell
from agent_workflow.runtime import RuntimePaths
from agent_workflow.storage import OperationalError, atomic, identity, read


class FakeHerdr:
    def __init__(self, *, lose_workspace=False, lose_layout=False):
        self.workspaces = []
        self.layouts = []
        self.lose_workspace = lose_workspace
        self.lose_layout = lose_layout
        self.creates = 0
        self.applies = 0

    def server_identity(self):
        return {"pid": 1, "start": "1", "boot": "test", "state": "R"}

    def call(self, method, params=None):
        params = params or {}
        if method == "ping": return {"protocol": 22}
        if method == "workspace.list": return {"workspaces": self.workspaces}
        if method == "workspace.create":
            self.creates += 1
            value = {"workspace_id": f"w{self.creates}", "label": params["label"]}
            self.workspaces.append(value)
            if self.lose_workspace:
                self.lose_workspace = False
                raise TimeoutError("reply lost after creation")
            return {"workspace": value}
        if method == "tab.list":
            return {"tabs": [{"tab_id": item["tab_id"]} for item in self.layouts]}
        if method == "layout.export":
            return {"layout": next(item for item in self.layouts if item["tab_id"] == params["tab_id"])}
        if method == "layout.apply":
            self.applies += 1
            layout = {"workspace_id": params["workspace_id"], "tab_id": f"t{self.applies}",
                      "root": {"type": "pane", "pane_id": f"p{self.applies}",
                               "command": params["root"]["command"]}}
            if not self.lose_layout: self.layouts.append(layout)
            else:
                self.lose_layout = False
                raise TimeoutError("layout result unknown")
            return {"layout": layout}
        if method in ("workspace.focus", "tab.focus"):
            return {}
        raise AssertionError(method)


class SessionAttachRecoveryTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name)
        self.paths = RuntimePaths(root, root / "state", root / "registry", "a" * 32)
        self.socket = str(root / "herdr.sock")

    def tearDown(self): self.temp.cleanup()

    def test_lost_workspace_reply_uses_existing_workspace(self):
        fake = FakeHerdr(lose_workspace=True)
        with patch.object(shell, "Herdr", return_value=fake):
            with self.assertRaises(TimeoutError): shell.attach(self.paths, self.socket, ["/installed/waywr"])
            recovered = shell.attach(self.paths, self.socket, ["/installed/waywr"])
        self.assertEqual(recovered["status"], "created")
        self.assertEqual(fake.creates, 1)
        self.assertEqual(fake.applies, 1)

    def test_unknown_layout_does_not_dispatch_again(self):
        fake = FakeHerdr(lose_layout=True)
        with patch.object(shell, "Herdr", return_value=fake):
            with self.assertRaisesRegex(OperationalError, "uncertain"):
                shell.attach(self.paths, self.socket, ["/installed/waywr"])
            with self.assertRaisesRegex(OperationalError, "uncertain"):
                shell.attach(self.paths, self.socket, ["/installed/waywr"])
        self.assertEqual(fake.applies, 1)

    def test_changed_server_with_live_ui_refuses_replacement(self):
        fake = FakeHerdr()
        pointer = self.paths.state / "session/ui-current.json"
        old = {"schema_version": 1, "project_key": self.paths.key, "token": "old",
               "server": {"pid": 2, "start": "2", "boot": "test"},
               "pane": {"workspace_id": "w1", "tab_id": "t1", "pane_id": "p1"}}
        atomic(pointer, old)
        atomic(pointer.parent / "ui-claims/old.json", {"process": identity(), "pane_id": "p1"})
        with patch.object(shell, "Herdr", return_value=fake):
            with self.assertRaisesRegex(OperationalError, "survives a Herdr server change"):
                shell.attach(self.paths, self.socket, ["/installed/waywr"])
        self.assertEqual(fake.creates, 0)


if __name__ == "__main__": unittest.main()
