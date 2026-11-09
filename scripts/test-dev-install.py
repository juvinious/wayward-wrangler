#!/usr/bin/env python3
"""Real local rebuild/install lifecycle, using a temporary explicit prefix.

Rust uses the normal permanent toolchain/HOME. No operator installation, shell
configuration, project, service or model is changed. Only compiler failure is a
fixture; successful builds/installations use the real tools and application.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parent.parent
SCRIPT = SOURCE / 'scripts/dev-install.sh'
BINARY = SOURCE / 'target/release/waywr'


class LocalInstall(unittest.TestCase):
    def test_version_works_without_a_project_or_installed_manifest(self):
        with tempfile.TemporaryDirectory(prefix='waywr-version-') as tmp:
            result = subprocess.run([str(BINARY), '--version'], cwd=tmp,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertRegex(result.stdout, r'^waywr \d+\.\d+\.\d+')
            self.assertEqual(list(Path(tmp).iterdir()), [])

    def test_one_command_replaces_install_and_preserves_work_after_a_failed_build(self):
        with tempfile.TemporaryDirectory(prefix='waywr-dev-acceptance-') as tmp:
            root = Path(tmp)
            prefix = root / 'installation with spaces'
            project = root / 'project'; project.mkdir()
            def run(argv, env=None, ok=True):
                result = subprocess.run(list(map(str, argv)), cwd=project,
                    env=env, capture_output=True, text=True, timeout=240)
                self.assertEqual(result.returncode == 0, ok, result.stdout[-1500:] + result.stderr[-1500:])
                return result
            run(['sh', SCRIPT, '--prefix', prefix])
            first = (prefix / 'current').resolve()
            first_manifest = json.loads((first / 'manifest.json').read_text())
            launcher = prefix / 'bin/waywr'
            version = run([launcher, '--version']).stdout
            self.assertIn(first_manifest['release_id'], version)
            run([launcher, 'init', project])
            saved = {path: path.read_bytes() for path in (project / '.waywr').rglob('*') if path.is_file()}
            run(['sh', SCRIPT, '--prefix', prefix])
            second = (prefix / 'current').resolve()
            self.assertNotEqual(first, second, 'A rebuild must select a fresh immutable artifact')
            manifest = json.loads((second / 'manifest.json').read_text())
            self.assertIn(manifest['release_id'], run([launcher, '--version']).stdout)
            for name in ['waywr', 'waywr-service', 'waywr-runner']:
                self.assertTrue((second / 'bin' / name).is_file())
            shutil.rmtree(first)
            run([launcher, 'init', project])
            self.assertTrue(all(path.read_bytes() == value for path, value in saved.items()))
            wrapper = root / 'refuse-compiler'
            wrapper.write_text('#!/bin/sh\necho "Fixture compiler failure" >&2\nexit 23\n')
            wrapper.chmod(0o755)
            failed_env = dict(os.environ, RUSTC_WRAPPER=str(wrapper))
            failed = run(['sh', SCRIPT, '--prefix', prefix], env=failed_env, ok=False)
            self.assertIn('Fixture compiler failure', failed.stderr)
            self.assertEqual((prefix / 'current').resolve(), second)
            self.assertIn(manifest['release_id'], run([launcher, '--version']).stdout)
            self.assertFalse(list((prefix / 'releases').glob('.install-*')))
            self.assertTrue(all(path.read_bytes() == value for path, value in saved.items()))


if __name__ == '__main__':
    unittest.main()
