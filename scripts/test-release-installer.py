#!/usr/bin/env python3
"""Offline transport/archive checks. Delegation uses a clearly labelled shell fixture."""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from installer_fixture import release_transport

INSTALLER = Path(__file__).with_name('install.sh').resolve()
VERSION = 'v0.4.0'


class Installer(unittest.TestCase):
    def test_repository_selection_is_required_before_download(self):
        env = {'HOME': '/tmp', 'PATH': os.environ['PATH']}
        result = subprocess.run(['sh', str(INSTALLER), '--version', VERSION], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('Select the release repository', result.stderr)

    def attempt(self, extra=None, corrupt=False, latest=False, change_metadata=None, failed_node=False, private=False, no_select=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); assets = root / 'assets'; assets.mkdir()
            name = f'waywr-{VERSION}-linux-x86_64-gnu'
            archive = assets / (name + '.tar.gz')
            with tarfile.open(archive, 'w:gz') as out:
                body = b'#!/bin/sh\nprintf "%s\\n" "$@" > "$TEST_RESULT"\n'
                item = tarfile.TarInfo(name + '/bin/waywr'); item.mode = 0o755; item.size = len(body)
                out.addfile(item, io.BytesIO(body))
                if extra:
                    item = tarfile.TarInfo(extra)
                    if extra.endswith('link'):
                        item.type = tarfile.SYMTYPE; item.linkname = '/etc/passwd'
                    out.addfile(item)
            checksum = '0' * 64 if corrupt else hashlib.sha256(archive.read_bytes()).hexdigest()
            (assets / 'SHA256SUMS').write_text(checksum + '  ' + archive.name + '\n')
            bins = root / 'bin'
            fixture = release_transport(bins, assets, VERSION)
            if change_metadata:
                path = assets / 'release.json'; metadata = json.loads(path.read_text())
                change_metadata(metadata); path.write_text(json.dumps(metadata))
            if failed_node:
                node = bins / 'node'; node.write_text('#!/bin/sh\nexit 1\n'); node.chmod(0o755)
            if private:
                gh = bins / 'gh'
                gh.write_text('''#!/bin/sh
set -eu
if [ "$1" = api ]; then cat "$FIXTURE_ASSETS/release.json"; exit; fi
while [ "$#" -gt 0 ]; do
 if [ "$1" = --dir ]; then shift; cp "$FIXTURE_ASSETS/"* "$1/"; exit; fi
 shift
done
exit 2
''')
                gh.chmod(0o755)
            result = root / 'delegation'
            env = dict(fixture, HOME=str(root), PATH=str(bins) + ':' + os.environ['PATH'], TEST_RESULT=str(result))
            argv = ['sh', str(INSTALLER), '--repo', 'example/wayward-wrangler', '--prefix', str(root / "installation with spaces and 'quote'")]
            if not latest: argv += ['--version', VERSION]
            if private: argv += ['--private']
            if no_select: argv += ['--no-select-default']
            p = subprocess.run(argv, env=env, text=True, capture_output=True, timeout=15)
            requests = (bins / 'requests').read_text() if (bins / 'requests').exists() else ''
            return p.returncode, result.read_text() if result.exists() else '', p.stderr, p.stdout, requests

    def test_public_exact_version_needs_no_github_login(self):
        status, args, error, output, requests = self.attempt()
        self.assertEqual(status, 0, error)
        self.assertIn('bundle-install\n--bundle\n', args); self.assertIn('--select-default', args)
        self.assertIn('/releases/tags/' + VERSION, requests)
        self.assertNotIn('session upgrade', output)
        self.assertIn('export PATH=', output)
        hint = next(line.strip() for line in output.splitlines() if 'export PATH=' in line)
        check = subprocess.run(['bash', '-c', hint + '\nprintf "%s" "$PATH"'], env={'PATH': '/usr/bin:/bin'}, capture_output=True, text=True)
        self.assertEqual(check.returncode, 0, check.stderr)
        prefix = args.split('--prefix\n', 1)[1].splitlines()[0]
        self.assertEqual(check.stdout, prefix + '/bin:/usr/bin:/bin')

    def test_public_latest_stable_needs_no_version_or_github_login(self):
        status, args, error, _, requests = self.attempt(latest=True)
        self.assertEqual(status, 0, error)
        self.assertIn('bundle-install', args)
        self.assertIn('/releases/latest', requests)
        self.assertIn('/releases/download/' + VERSION + '/', requests)

    def test_private_download_requires_explicit_opt_in(self):
        status, args, error, _, requests = self.attempt(private=True)
        self.assertEqual(status, 0, error)
        self.assertIn('bundle-install', args); self.assertEqual(requests, '')

    def test_install_without_selection_leaves_default_alone(self):
        status, args, error, output, _ = self.attempt(no_select=True)
        self.assertEqual(status, 0, error)
        self.assertNotIn('--select-default', args)
        self.assertIn('Default application selection was not changed', output)
        self.assertNotIn('export PATH=', output)

    def test_checksum_refuses_before_execution(self):
        status, args, *_ = self.attempt(corrupt=True)
        self.assertNotEqual(status, 0); self.assertEqual(args, '')

    def test_traversal_and_symlink_refuse_before_execution(self):
        for name in [f'waywr-{VERSION}-linux-x86_64-gnu/../escape', f'waywr-{VERSION}-linux-x86_64-gnu/link']:
            status, args, *_ = self.attempt(extra=name)
            self.assertNotEqual(status, 0); self.assertEqual(args, '')

    def test_untrusted_or_incomplete_release_metadata_refuses_before_execution(self):
        changes = [lambda m: m.update(tag_name='v9.9.9'), lambda m: m.update(draft=True),
                   lambda m: m['assets'].pop(), lambda m: m['assets'][0].update(digest=None),
                   lambda m: m['assets'][0].update(digest='sha256:' + '0' * 64)]
        for change in changes:
            status, args, *_ = self.attempt(change_metadata=change)
            self.assertNotEqual(status, 0); self.assertEqual(args, '')

    def test_latest_does_not_select_a_prerelease(self):
        status, args, *_ = self.attempt(latest=True, change_metadata=lambda m: m.update(prerelease=True))
        self.assertNotEqual(status, 0); self.assertEqual(args, '')

    def test_unusable_node_fails_before_download_with_an_actionable_message(self):
        status, args, error, _, requests = self.attempt(failed_node=True)
        self.assertNotEqual(status, 0); self.assertEqual(args, '')
        self.assertIn('Node >=22.19', error); self.assertEqual(requests, '')


if __name__ == '__main__': unittest.main()
