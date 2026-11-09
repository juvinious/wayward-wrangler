import tempfile
from types import SimpleNamespace
from pathlib import Path
import unittest
from unittest.mock import patch

from agent_workflow import cli, records
from agent_workflow.context import project_file
from agent_workflow.runtime import bind, load, lookup, register, unbind
from agent_workflow.storage import OperationalError


class Portability(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ww-portability-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.registry = self.root / "registry"
        for name in ("a", "b", "c"):
            (self.root / name).mkdir()
        self.a = load(self.root / "a", registry_dir=self.registry, initialize=True)
        self.b = load(self.root / "b", registry_dir=self.registry, initialize=True)
        register(self.a); register(self.b)
        self.token = bind(self.a)
        self.addCleanup(lambda: unbind(self.token))

    def test_init_ignores_only_state_and_tracks_config_context(self):
        project = self.root / "a"
        self.assertEqual((project / ".gitignore").read_text(), ".wayward-wrangler/state/\n")
        self.assertTrue((project / ".wayward-wrangler/project.toml").is_file())
        self.assertTrue((project / ".wayward-wrangler/workflow.toml").is_file())
        self.assertNotEqual(self.a.key, self.b.key)

    def test_init_allows_disposable_target_ignored_by_parent(self):
        target = self.root / "c"
        with patch("agent_workflow.runtime.shutil.which", return_value="git"), patch(
                "agent_workflow.runtime.subprocess.run", return_value=SimpleNamespace(returncode=0)) as run:
            selected = load(target, registry_dir=self.registry, initialize=True)
        self.assertTrue((target / ".wayward-wrangler/project.toml").is_file())
        self.assertEqual(run.call_count, 1)
        self.assertEqual(run.call_args.args[0][-1], str(target))
        self.assertEqual(selected.project, target)

    def test_wrong_state_and_duplicate_key_refused(self):
        with self.assertRaisesRegex(OperationalError, "another project"):
            register(load(self.root / "b", state_dir=self.a.state, registry_dir=self.registry))
        (self.root / "c/.wayward-wrangler").mkdir()
        (self.root / "c/.wayward-wrangler/project.toml").write_bytes(
            (self.root / "a/.wayward-wrangler/project.toml").read_bytes())
        with self.assertRaisesRegex(OperationalError, "different registered location"):
            register(load(self.root / "c", registry_dir=self.registry))

    def test_context_symlink_escape_and_missing_link_refused(self):
        outside = self.root / "outside.md"
        outside.write_text("secret")
        (self.root / "a/link.md").symlink_to(outside)
        with self.assertRaisesRegex(OperationalError, "project-local"):
            project_file(self.root / "a/link.md")
        with self.assertRaisesRegex(OperationalError, "missing"):
            lookup(self.registry, "0" * 32)

    def test_review_lookup_rejects_missing_or_changed_state_marker(self):
        marker = self.a.state / "project-identity.json"
        original = marker.read_bytes()
        marker.unlink()
        with self.assertRaisesRegex(OperationalError, "stale"):
            lookup(self.registry, self.a.key)
        marker.write_text('{"schema_version": 1, "project_key": "' + self.b.key + '"}')
        with self.assertRaisesRegex(OperationalError, "stale"):
            lookup(self.registry, self.a.key)
        marker.write_text("{broken")
        with self.assertRaisesRegex(OperationalError, "stale"):
            lookup(self.registry, self.a.key)
        marker.write_bytes(original)
        self.assertEqual(lookup(self.registry, self.a.key), self.a)

    def test_other_active_project_does_not_block_selected_project(self):
        records.create_run({"run_id": "demo", "project_key": self.a.key})
        other = bind(self.b)
        try:
            cli.no_other_execution()
        finally:
            unbind(other)
        with self.assertRaisesRegex(OperationalError, "Run demo is active"):
            cli.no_other_execution()
        records.set_status("demo", "approved")
        other = bind(self.b)
        try:
            cli.no_other_execution()
        finally:
            unbind(other)

    def test_historical_native_trial_id_cannot_authorize_new_project(self):
        profile = self.root / "a/native.toml"
        profile.write_text('''execution_mode = "native"
review_limit = 2
trial_id = "context-01"
workflow_version = 2
max_invocations = 5
[roles.author]
provider = "claude"
model = "opus"
[roles.critic]
provider = "codex"
model = "sol"
''')
        (self.root / "a/brief.md").write_text("No native call is authorized")
        args = SimpleNamespace(run="context-01", profile="native.toml", brief="brief.md")
        with self.assertRaisesRegex(OperationalError, "Historical native trial identity"):
            cli.create(args)


if __name__ == "__main__":
    unittest.main()
