"""Native admission checks without CLI authentication, Herdr writes, or models."""
import json
import io
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from agent_workflow import artifacts, cli, records, workflow
from agent_workflow.adapters import real
from agent_workflow.native import authorize
from agent_workflow.runtime import bind, load, register, unbind
from agent_workflow.storage import OperationalError, canonical, digest, read


class NativeAdmission(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ww-native-")
        self.addCleanup(self.temp.cleanup)
        self.project = Path(self.temp.name) / "project"
        self.project.mkdir()
        self.paths = load(self.project, registry_dir=Path(self.temp.name) / "registry", initialize=True)
        register(self.paths)
        token = bind(self.paths)
        self.addCleanup(lambda: unbind(token))
        (self.project / "brief.md").write_text("Specify a local decision log; ask about editing existing entries.")
        (self.project / "requirements.md").write_text("Browser-only storage; no backend.")
        (self.project / "selection.toml").write_text('''schema_version = 1
project_id = "decision-log"
scope = "initial-specification"
[[entries]]
kind = "document"
scope = "initial-specification"
reason = "Product constraints"
path = "requirements.md"
''')
        (self.project / "native.toml").write_text('''execution_mode = "native"
workflow_version = 2
prompt_contract = "product-spec-v1"
review_limit = 2
max_invocations = 5
[roles.author]
provider = "claude"
binary = "/fixture/claude"
model = "sonnet"
reasoning = "medium"
[roles.critic]
provider = "codex"
binary = "/fixture/codex"
model = "gpt-6.1-sol"
reasoning = "high"
''')
        self.args = SimpleNamespace(run="decision-log-01", brief="brief.md", profile="native.toml",
            context="selection.toml", workspace="w-fixture", socket="/fixture/socket", authorize_native=None)
        self.patches = [patch.object(real, "verify_role", return_value="supported"),
                        patch("agent_workflow.native.check_auth"),
                        patch.object(cli.Herdr, "call", return_value={"workspace": {"workspace_id": "w-fixture"}})]
        for item in self.patches:
            item.start()
            self.addCleanup(item.stop)

    def test_preflight_is_read_only_and_freezes_complete_plan(self):
        selected = cli._create(self.args, preview=True)
        self.assertEqual(selected["roles"]["author"]["reasoning"], "medium")
        self.assertEqual(selected["roles"]["critic"]["model"], "gpt-6.1-sol")
        self.assertEqual(selected["max_invocations"], 5)
        self.assertFalse((self.paths.state / "state.sqlite").exists())
        self.assertFalse((self.paths.state / "runs").exists())

    def test_digest_authorization_and_frozen_packet(self):
        selected = cli._create(self.args, preview=True)
        with patch("agent_workflow.native.sys.stdin.isatty", return_value=False):
            with self.assertRaisesRegex(OperationalError, "interactive confirmation"):
                cli.create(self.args)
        self.args.authorize_native = "0" * 64
        with self.assertRaisesRegex(OperationalError, "digest does not match"):
            cli.create(self.args)
        self.assertFalse((self.paths.state / "runs/decision-log-01").exists())
        self.args.authorize_native = digest(canonical(selected))
        self.assertEqual(cli.create(self.args), "decision-log-01")
        envelope = read(records.run_dir("decision-log-01") / "inputs/envelope.json")
        self.assertEqual(envelope["native_authorization"]["plan_sha256"], self.args.authorize_native)
        packet = artifacts.packet(envelope, workflow.initial("decision-log-01", envelope), "draft")
        self.assertEqual(packet["max_invocations"], 5)
        self.assertEqual(packet["prompt_contract"], "product-spec-v1")
        self.assertEqual(packet["execution"]["reasoning"], "medium")
        self.assertIn("1,200 words", real.prompt(packet))

    def test_input_change_invalidates_preflight_digest(self):
        selected = cli._create(self.args, preview=True)
        self.args.authorize_native = digest(canonical(selected))
        (self.project / "requirements.md").write_text("Changed after preflight")
        with self.assertRaisesRegex(OperationalError, "digest does not match"):
            cli.create(self.args)
        self.assertFalse((self.paths.state / "runs/decision-log-01").exists())

    def test_interactive_confirmation_displays_scope_without_hash_copying(self):
        output = io.StringIO()
        with patch("agent_workflow.native.sys.stdin.isatty", return_value=True), patch(
                "builtins.input", return_value="authorize decision-log-01") as typed, patch(
                "sys.stdout", output):
            cli.create(self.args)
        self.assertIn(str(self.project), output.getvalue())
        self.assertIn("sonnet effort=medium", output.getvalue())
        self.assertIn("gpt-6.1-sol effort=high", output.getvalue())
        self.assertIn("5 native launches, 2 critiques", output.getvalue())
        self.assertIn("authorize decision-log-01", typed.call_args.args[0])
        envelope = read(records.run_dir("decision-log-01") / "inputs/envelope.json")
        self.assertEqual(envelope["native_authorization"]["mode"], "interactive")


if __name__ == "__main__":
    unittest.main()
