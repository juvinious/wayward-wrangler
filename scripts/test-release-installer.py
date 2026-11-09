#!/usr/bin/env python3
"""Offline archive-validation/delegation fixtures. Never installs or starts Waywr."""
import hashlib, io, os, pathlib, subprocess, tarfile, tempfile, unittest
INSTALLER = pathlib.Path(__file__).with_name('install.sh').resolve()

class Installer(unittest.TestCase):
    def test_repository_selection_is_required_before_download(self):
        env = dict(os.environ)
        env.pop('WAYWR_REPOSITORY', None)
        result = subprocess.run(['sh', str(INSTALLER), '--version', 'v0.4.0'], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('Select the release repository', result.stderr)

    def attempt(self, extra=None, corrupt=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); assets = root / 'assets'; assets.mkdir()
            name = 'waywr-v0.4.0-linux-x86_64-gnu'
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
            checksum = ('0' * 64 if corrupt else hashlib.sha256(archive.read_bytes()).hexdigest())
            sums = assets / 'SHA256SUMS'; sums.write_text(checksum + '  ' + archive.name + '\n')
            bins = root / 'bin'; bins.mkdir()
            gh = bins / 'gh'; gh.write_text('''#!/bin/sh
if [ "$1" = api ]; then
 case "$*" in *tag_name*) echo v0.4.0;; *) printf 'sha256:'; sha256sum "$TEST_ASSETS/SHA256SUMS" | cut -d' ' -f1;; esac
else
 while [ "$#" -gt 0 ]; do if [ "$1" = --dir ]; then shift; cp "$TEST_ASSETS/"* "$1/"; exit; fi; shift; done
 exit 2
fi
'''); gh.chmod(0o755)
            result = root / 'delegation'; env = dict(os.environ, PATH=str(bins) + ':' + os.environ['PATH'], TEST_ASSETS=str(assets), TEST_RESULT=str(result))
            p = subprocess.run(['sh', str(INSTALLER), '--version', 'v0.4.0', '--repo', 'example/wayward-wrangler', '--prefix', str(root / 'unwritten-install')], env=env, text=True, capture_output=True)
            return p.returncode, result.read_text() if result.exists() else '', p.stderr
    def test_exact_archive_delegates_to_existing_bundle_installer(self):
        status, args, error = self.attempt(); self.assertEqual(status, 0, error)
        self.assertIn('bundle-install\n--bundle\n', args); self.assertIn('--select-default', args)
    def test_checksum_refuses_before_execution(self):
        status, args, _ = self.attempt(corrupt=True); self.assertNotEqual(status, 0); self.assertEqual(args, '')
    def test_traversal_and_symlink_refuse_before_execution(self):
        for name in ['waywr-v0.4.0-linux-x86_64-gnu/../escape', 'waywr-v0.4.0-linux-x86_64-gnu/link']:
            status, args, _ = self.attempt(extra=name); self.assertNotEqual(status, 0); self.assertEqual(args, '')

if __name__ == '__main__': unittest.main()
