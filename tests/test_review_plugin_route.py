import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from importlib.resources import files


class ReviewPluginRouteTest(unittest.TestCase):
    def test_project_link_selects_bound_launcher_and_rejects_ambiguous_url(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plugin = root / "plugin"
            plugin.mkdir()
            shutil.copyfile(files("agent_workflow.resources.herdr").joinpath("open_review.py"),
                            plugin / "open_review.py")
            key = "a" * 32
            project = root / "project"
            project.mkdir()
            registry = root / "registry"
            registry.mkdir()
            (registry / "locations.json").write_text(json.dumps({"projects": {key: {
                "project": str(project), "state": str(project / "state")}}}))
            (plugin / "binding.json").write_text(json.dumps({"schema_version": 1,
                "python": "/missing/python", "launcher": "/usr/bin/echo", "registry": str(registry)}))
            url = f"https://awp.invalid/v2/review/{key}/run-1/draft/" + "b" * 64
            env = {**os.environ, "HERDR_PLUGIN_CLICKED_URL": url}
            command = ["python3", "-I", str(plugin / "open_review.py")]
            accepted = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            self.assertIn(f"--project {project} --registry-dir {registry} open-link", accepted.stdout)
            env["HERDR_PLUGIN_CLICKED_URL"] = "https://awp.invalid/review/run-1"
            refused = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertNotEqual(refused.returncode, 0)
            self.assertIn("unambiguous project identity", refused.stderr)


if __name__ == "__main__":
    unittest.main()
