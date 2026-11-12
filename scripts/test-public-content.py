#!/usr/bin/env python3
"""Exercise the public-content gate with synthetic private binary paths."""
import pathlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("check-public-content.py")


class PublicContent(unittest.TestCase):
    def test_binary_home_paths_are_refused_without_echoing_values(self):
        for prefix in ("home", "Users"):
            private = ("/" + prefix + "/synthetic-builder").encode()
            with tempfile.TemporaryDirectory() as tmp:
                artifact = pathlib.Path(tmp) / "candidate.bin"
                artifact.write_bytes(b"\x7fELF\x00" + private + b"\x00")
                selected = artifact if prefix == "home" else pathlib.Path(tmp)
                run = subprocess.run(["python3", str(SCRIPT), "--artifact", str(selected)], capture_output=True)
                self.assertNotEqual(run.returncode, 0)
                self.assertIn(b"absolute personal-home path", run.stderr)
                self.assertNotIn(private, run.stderr + run.stdout)

    def test_private_deny_values_are_never_echoed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            identity = b"synthetic-" + b"private-identity"
            deny = root / "deny"; deny.write_bytes(identity + b"\n")
            artifact = root / "candidate.bin"; artifact.write_bytes(b"\x00" + identity + b"\x00")
            run = subprocess.run(["python3", str(SCRIPT), "--artifact", str(artifact), "--deny-file", str(deny)], capture_output=True)
            self.assertNotEqual(run.returncode, 0)
            self.assertNotIn(identity, run.stdout + run.stderr)

    def test_removed_contact_in_intermediate_commit_still_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); (root / "scripts").mkdir()
            shutil.copy2(SCRIPT, root / "scripts" / SCRIPT.name)
            env = dict(os.environ, GIT_AUTHOR_NAME="Fixture", GIT_COMMITTER_NAME="Fixture", GIT_AUTHOR_EMAIL="source@example.invalid", GIT_COMMITTER_EMAIL="source@example.invalid")
            def git(*args):
                return subprocess.check_output(["git", "-C", str(root), "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args], env=env, stderr=subprocess.PIPE)
            git("init", "--initial-branch=main", "--template="); git("add", "."); git("commit", "-m", "Fixture baseline")
            baseline = git("rev-parse", "HEAD").decode().strip()
            contact = b"synthetic-person" + b"@" + b"corporate" + b".com"
            (root / "removed.txt").write_bytes(contact)
            git("add", "."); git("commit", "-m", "Introduce fixture")
            git("rm", "removed.txt"); git("commit", "-m", "Remove fixture")
            run = subprocess.run(["python3", str(root / "scripts" / SCRIPT.name), "--revisions", baseline + "..HEAD"], capture_output=True, env=env)
            self.assertNotEqual(run.returncode, 0)
            self.assertIn(b"non-example contact address", run.stderr)
            self.assertNotIn(contact, run.stdout + run.stderr)

    def test_repository_owner_from_event_is_not_echoed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); (root / "scripts").mkdir()
            shutil.copy2(SCRIPT, root / "scripts" / SCRIPT.name)
            subprocess.run(["git", "init", "--template=", str(root)], check=True, capture_output=True)
            identity = "synthetic-" + "account-owner"
            (root / "identifying.txt").write_text(identity.upper())
            event = root.parent / (root.name + "-event.json")
            try:
                event.write_text(json.dumps({"repository": {"owner": {"type": "User", "login": identity}}}))
                run = subprocess.run(["python3", str(root / "scripts" / SCRIPT.name)], env=dict(os.environ, GITHUB_EVENT_PATH=str(event)), capture_output=True)
                self.assertNotEqual(run.returncode, 0)
                self.assertIn(b"private identifier", run.stderr)
                self.assertNotIn(identity.encode(), run.stdout + run.stderr)
            finally:
                event.unlink(missing_ok=True)

    def test_public_readme_badges_allow_only_canonical_workflow_metadata(self):
        identity = "synthetic-" + "account-owner"
        repository = identity + "/example-repo"
        canonical = "https://github.com/" + repository + "/actions/workflows/ci.yml"
        badge = "[![Build](" + canonical + "/badge.svg?branch=main)](" + canonical + ")\n"
        cases = [("README.md", badge, True), ("src/lib.rs", badge, False),
                 ("docs/README.md", badge, False),
                 ("README.md", badge + identity, False),
                 ("README.md", badge.replace("example-repo", "private-repo"), False),
                 ("README.md", badge.replace("ci.yml", "unlisted.yml"), False),
                 ("README.md", badge.replace("?branch=main", "?branch=main&token=secret"), False)]
        cases = [(name, content, allowed, False) for name, content, allowed in cases]
        cases += [("README.md", badge.replace("ci.yml", workflow), True, False) for workflow in ["security.yml", "codeql.yml"]]
        cases.append(("README.md", badge, False, True))
        for name, content, allowed, private in cases:
            with self.subTest(name=name, allowed=allowed), tempfile.TemporaryDirectory() as tmp:
                root = pathlib.Path(tmp); (root / "repo/scripts").mkdir(parents=True)
                repo = root / "repo"
                shutil.copy2(SCRIPT, repo / "scripts" / SCRIPT.name)
                subprocess.run(["git", "init", "--template=", str(repo)], check=True, capture_output=True)
                path = repo / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(content)
                event = root / "event.json"
                event.write_text(json.dumps({"repository": {"owner": {"type": "User", "login": identity},
                    "full_name": repository, "private": private}}))
                env = dict(os.environ, GITHUB_EVENT_PATH=str(event))
                result = subprocess.run(["python3", str(repo / "scripts" / SCRIPT.name)], env=env, capture_output=True)
                self.assertEqual(result.returncode == 0, allowed, result.stderr.decode())
                if allowed:
                    artifact = root / "candidate.bin"; artifact.write_text(canonical)
                    refused = subprocess.run(["python3", str(repo / "scripts" / SCRIPT.name), "--artifact", str(artifact)], env=env, capture_output=True)
                    self.assertNotEqual(refused.returncode, 0)
                else:
                    self.assertNotIn(identity.encode(), result.stdout + result.stderr)

    def test_local_public_context_keeps_deny_file_and_history_checks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); repo = root / "repo"; (repo / "scripts").mkdir(parents=True)
            shutil.copy2(SCRIPT, repo / "scripts" / SCRIPT.name)
            identity = "synthetic-" + "account-owner"; repository = identity + "/example-repo"
            canonical = "https://github.com/" + repository + "/actions/workflows/ci.yml"
            badge = "[![Build](" + canonical + "/badge.svg?branch=main)](" + canonical + ")\n"
            deny = root / "deny"; deny.write_text(identity + "\n")
            env = dict(os.environ, GIT_AUTHOR_NAME="Fixture", GIT_COMMITTER_NAME="Fixture",
                       GIT_AUTHOR_EMAIL="source@example.invalid", GIT_COMMITTER_EMAIL="source@example.invalid")
            env.pop("GITHUB_EVENT_PATH", None)
            def git(*args):
                return subprocess.check_output(["git", "-C", str(repo), "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args], env=env, stderr=subprocess.PIPE)
            def scan(*args):
                return subprocess.run(["python3", str(repo / "scripts" / SCRIPT.name), "--deny-file", str(deny), *args], env=env, capture_output=True)
            git("init", "--initial-branch=main", "--template=")
            readme = repo / "README.md"; readme.write_text(badge)
            git("add", "."); git("commit", "-m", "Fixture badge")
            allowed = ["--public-repository", repository]
            self.assertNotEqual(scan().returncode, 0)
            self.assertEqual(scan(*allowed, "--revisions", "HEAD").returncode, 0)
            for name, content in [("README.md", badge + identity), ("src.rs", canonical),
                                  ("README.md", badge.replace("ci.yml", "unlisted.yml")),
                                  ("README.md", badge.replace("?branch=main", "?branch=main&token=secret"))]:
                with self.subTest(name=name, content=content):
                    path = repo / name; path.write_text(content)
                    refused = scan(*allowed)
                    self.assertNotEqual(refused.returncode, 0)
                    self.assertNotIn(identity.encode(), refused.stdout + refused.stderr)
                    if path == readme: readme.write_text(badge)
                    else: path.unlink()
            artifact = root / "candidate.bin"; artifact.write_text(canonical)
            self.assertNotEqual(scan(*allowed, "--artifact", str(artifact)).returncode, 0)
            readme.write_text(badge + identity); git("add", "."); git("commit", "-m", "Fixture disclosure")
            readme.write_text(badge); git("add", "."); git("commit", "-m", "Remove disclosure")
            refused = scan(*allowed, "--revisions", "HEAD~2..HEAD")
            self.assertNotEqual(refused.returncode, 0)
            self.assertNotIn(identity.encode(), refused.stdout + refused.stderr)
            event = root / "event.json"; env["GITHUB_EVENT_PATH"] = str(event)
            for full_name, private, ok in [(repository, False, True), (repository, True, False),
                                          (identity + "/other-repo", False, False)]:
                event.write_text(json.dumps({"repository": {"full_name": full_name, "private": private}}))
                result = scan(*allowed)
                self.assertEqual(result.returncode == 0, ok, result.stderr.decode())
                self.assertNotIn(identity.encode(), result.stdout + result.stderr)

    def test_credential_filename_refuses_even_without_recognized_secret(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); (root / "scripts").mkdir()
            shutil.copy2(SCRIPT, root / "scripts" / SCRIPT.name)
            subprocess.run(["git", "init", "--template=", str(root)], check=True, capture_output=True)
            (root / ".env").write_text("EXAMPLE=value\n")
            run = subprocess.run(["python3", str(root / "scripts" / SCRIPT.name)], capture_output=True)
            self.assertNotEqual(run.returncode, 0)
            self.assertIn(b"credential or private runtime file path", run.stderr)
            (root / ".env").rename(root / ".env.example")
            allowed = subprocess.run(["python3", str(root / "scripts" / SCRIPT.name)], capture_output=True)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)

    def test_merge_only_private_content_is_checked_after_removal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp); (root / "scripts").mkdir()
            shutil.copy2(SCRIPT, root / "scripts" / SCRIPT.name)
            env = dict(os.environ, GIT_AUTHOR_NAME="Fixture", GIT_COMMITTER_NAME="Fixture", GIT_AUTHOR_EMAIL="source@example.invalid", GIT_COMMITTER_EMAIL="source@example.invalid")
            def git(*args):
                return subprocess.check_output(["git", "-C", str(root), "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args], env=env, stderr=subprocess.PIPE)
            git("init", "--initial-branch=main", "--template="); git("add", "."); git("commit", "-m", "Fixture baseline")
            baseline = git("rev-parse", "HEAD").decode().strip()
            git("checkout", "-b", "feature"); (root / "feature.txt").write_text("Feature\n")
            git("add", "."); git("commit", "-m", "Feature")
            git("checkout", "main"); (root / "main.txt").write_text("Main\n")
            git("add", "."); git("commit", "-m", "Main")
            git("merge", "--no-ff", "--no-commit", "feature")
            contact = b"synthetic-person" + b"@" + b"corporate" + b".com"
            (root / "removed.txt").write_bytes(contact)
            git("add", "."); git("commit", "-m", "Merge fixture")
            git("rm", "removed.txt"); git("commit", "-m", "Remove fixture")
            run = subprocess.run(["python3", str(root / "scripts" / SCRIPT.name), "--revisions", baseline + "..HEAD"], capture_output=True, env=env)
            self.assertNotEqual(run.returncode, 0)
            self.assertIn(b"non-example contact address", run.stderr)
            self.assertNotIn(contact, run.stdout + run.stderr)


if __name__ == "__main__":
    unittest.main()
