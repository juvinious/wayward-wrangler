#!/usr/bin/env python3
"""Release metadata/publisher checks; gh is a local fixture, never a remote write."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def publish_command():
    text = (ROOT / '.github/workflows/release.yml').read_text()
    text = text.split('      - name: Publish immutable version assets\n', 1)[1]
    match = re.search(r'^        run: (.*)$', text, re.M)
    assert match
    if match[1] != '|':
        return match[1]
    lines = []
    for line in text[match.end():].splitlines():
        if not line.strip():
            continue
        if not line.startswith('          '):
            break
        lines.append(line[10:])
    return '\n'.join(lines)


class Publisher(unittest.TestCase):
    def attempt(self, tag, moved=False):
        with tempfile.TemporaryDirectory(prefix='waywr-release-publisher-') as tmp:
            root = Path(tmp)
            (root / 'release').mkdir()
            (root / 'release/archive.tar.gz').write_text('Publisher transport fixture, not an application archive')
            (root / 'release/SHA256SUMS').write_text('Publisher transport fixture')
            (root / 'release-notes').mkdir()
            notes = '- A concise user-visible change.\n'
            (root / 'release-notes/notes.md').write_text(notes)
            (root / 'bin').mkdir()
            gh = root / 'bin/gh'
            gh.write_text(f'#!{sys.executable}\n' + '''import json, os, sys
from pathlib import Path
args = sys.argv[1:]
if args[0] == 'api':
    assert args[1] == 'repos/' + os.environ['GH_REPO'] + '/commits/tags/' + os.environ['RELEASE_TAG']
    print(os.environ['FIXTURE_TAG_SHA'])
    sys.exit(0)
notes = Path(args[args.index('--notes-file') + 1]).read_text() if '--notes-file' in args else None
Path(os.environ['PUBLISH_FIXTURE_RESULT']).write_text(json.dumps({'args': args, 'notes': notes}))
''')
            gh.chmod(0o700)
            env = {'PATH': str(root / 'bin') + ':' + os.environ['PATH'], 'RELEASE_TAG': tag,
                   'PUBLISH_FIXTURE_RESULT': str(root / 'result'), 'GH_REPO': 'example/wayward-wrangler',
                   'RELEASE_SHA': 'a' * 40, 'FIXTURE_TAG_SHA': ('b' if moved else 'a') * 40}
            result = subprocess.run(['sh', '-eu', '-c', publish_command()], cwd=root, env=env,
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode == 0, not moved, result.stderr)
            return json.loads((root / 'result').read_text()) if (root / 'result').exists() else None, notes

    def test_stable_release_uses_reviewed_changelog_notes(self):
        result, notes = self.attempt('v0.4.0')
        self.assertEqual(result['notes'], notes, 'Published release bypassed the reviewed changelog')
        self.assertIn('--verify-tag', result['args'])
        self.assertNotIn('--generate-notes', result['args'])
        self.assertNotIn('--prerelease', result['args'])

    def test_moved_tag_cannot_publish_the_previously_built_commit(self):
        result, _ = self.attempt('v0.4.0', moved=True)
        self.assertIsNone(result)

    def test_preview_is_a_prerelease_and_never_latest(self):
        result, notes = self.attempt('v0.4.0-rc.1')
        self.assertIn('--prerelease', result['args'], 'Preview tag was published as a stable release')
        self.assertIn('--latest=false', result['args'])
        self.assertEqual(result['notes'], notes)


class ReleaseMetadata(unittest.TestCase):
    def attempt(self, tag='v0.4.0', version='0.4.0', released='0.4.0', date='2026-11-11', extra=''):
        with tempfile.TemporaryDirectory(prefix='waywr-release-notes-') as tmp:
            root = Path(tmp); (root / 'frontend').mkdir()
            (root / 'frontend/Cargo.toml').write_text(f'[package]\nversion = "{version}"\n')
            (root / 'CHANGELOG.md').write_text(f'# Changelog\n\n## [Unreleased]\n\n- Future change.\n\n## [{released}] - {date}\n\n- Reviewed change.\n' + extra)
            notes = root / 'notes.md'
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/release-notes.py'),
                '--source-root', str(root), '--tag', tag, '--output', str(notes)], capture_output=True, text=True)
            return result, notes.read_text() if notes.exists() else None

    def test_matching_version_gets_only_its_release_notes(self):
        result, notes = self.attempt()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(notes, '- Reviewed change.\n')
        self.assertFalse(json.loads(result.stdout)['prerelease'])

    def test_preview_requires_matching_application_and_entry(self):
        result, notes = self.attempt(tag='v0.4.0-rc.1', version='0.4.0-rc.1', released='0.4.0-rc.1')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['prerelease'])
        self.assertEqual(notes, '- Reviewed change.\n')

    def test_bad_identity_or_changelog_never_produces_release_notes(self):
        cases = [dict(tag='v0.5.0'), dict(released='0.3.0'), dict(date='2026-99-99'),
                 dict(tag='0.4.0'), dict(extra='\n## [0.4.0] - 2026-11-11\n\n- Duplicate.\n'),
                 dict(tag='v0.4.0-rc.01', version='0.4.0-rc.01', released='0.4.0-rc.01')]
        for case in cases:
            with self.subTest(case=case):
                result, notes = self.attempt(**case)
                self.assertNotEqual(result.returncode, 0)
                self.assertIsNone(notes)


class MainAncestry(unittest.TestCase):
    def attempt(self, unreviewed):
        with tempfile.TemporaryDirectory(prefix='waywr-release-main-') as tmp:
            root = Path(tmp); (root / 'frontend').mkdir(); (root / 'scripts').mkdir()
            shutil.copyfile(ROOT / 'scripts/release-notes.py', root / 'scripts/release-notes.py')
            (root / 'frontend/Cargo.toml').write_text('[package]\nversion = "0.4.0"\n')
            (root / 'CHANGELOG.md').write_text('# Changelog\n\n## [Unreleased]\n\n## [0.4.0] - 2026-11-11\n\n- Reviewed.\n')
            env = {'PATH': os.environ['PATH'], 'HOME': str(root), 'GIT_CONFIG_NOSYSTEM': '1',
                   'GIT_CONFIG_GLOBAL': '/dev/null', 'RELEASE_TAG': 'v0.4.0', 'GITHUB_OUTPUT': str(root / 'github-output')}
            def git(*args):
                subprocess.run(['git', *args], cwd=root, env=env, check=True, capture_output=True)
            git('init', '--initial-branch=main', '--template=')
            git('add', '.')
            git('-c', 'user.name=Release fixture', '-c', 'user.email=release@example.invalid', 'commit', '-m', 'Fixture main')
            git('update-ref', 'refs/remotes/origin/main', 'HEAD')
            if unreviewed:
                (root / 'unreviewed.txt').write_text('Off-main fixture')
                git('add', '.')
                git('-c', 'user.name=Release fixture', '-c', 'user.email=release@example.invalid', 'commit', '-m', 'Off-main fixture')
            workflow = (ROOT / '.github/workflows/release.yml').read_text()
            step = workflow.split('      - name: Validate release identity and notes\n', 1)[1]
            command = re.search(r'^        run: \|\n((?:          .*\n)+)', step, re.M)[1]
            command = '\n'.join(line[10:] for line in command.splitlines())
            result = subprocess.run(['sh', '-eu', '-c', command], cwd=root, env=env, capture_output=True, text=True)
            return result, (root / 'release-notes/notes.md').exists()

    def test_reviewed_main_commit_can_prepare_release(self):
        result, notes = self.attempt(False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(notes)

    def test_unmerged_commit_cannot_prepare_release(self):
        result, notes = self.attempt(True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(notes)


if __name__ == '__main__':
    unittest.main()
