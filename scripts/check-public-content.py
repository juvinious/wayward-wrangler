#!/usr/bin/env python3
"""Reject machine-home paths and private identifiers without echoing their values."""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys

HOME_PATH = re.compile(rb"/(?:home|Users)/[A-Za-z0-9_.-]+(?:/|\b)")
PRIVATE_ID = re.compile(rb"<(?:private-id|private-record)>")
EMAIL = re.compile(rb"[A-Za-z0-9._%+-]+@([A-Za-z0-9.-]+\.[A-Za-z]{2,})")


def private_path(name):
    parts = pathlib.PurePosixPath(name).parts
    if any(part in {".ssh", ".aws", ".codex", ".claude", ".waywr", ".wayward-wrangler", ".local", "target", "node_modules", ".venv"} for part in parts):
        return True
    leaf = parts[-1] if parts else ""
    return (leaf == ".env" or (leaf.startswith(".env.") and leaf != ".env.example")
            or leaf in {"auth.json", "credentials.json"}
            or leaf.startswith(("credentials.", "secrets."))
            or leaf.endswith((".pem", ".key", ".p12", ".pfx", ".db", ".sqlite", ".sqlite3", ".transcript")))


def problems(data, forbidden=(), artifact=False, allow_contacts=False):
    found = []
    if HOME_PATH.search(data):
        found.append("absolute personal-home path")
    if artifact and PRIVATE_ID.search(data):
        found.append("private-record placeholder in distributed content")
    if any(value and value.lower() in data.lower() for value in forbidden):
        found.append("private identifier or builder path")
    if not artifact and not allow_contacts:
        for match in EMAIL.finditer(data):
            domain = match[1].lower()
            if match[0].lower() == b"git@github.com" or domain.endswith((b".invalid", b".test", b".example")) or domain in (b"example.com", b"example.org", b"example.net"):
                continue
            found.append("non-example contact address")
            break
    return found


def public_workflow_metadata(name, data, repository):
    # Requested public README badges identify GitHub ownership, not a builder.
    # Only canonical workflow links for this public repository are exempt. Do
    # not apply this to application source, artifacts, profile links or queries.
    full_name = repository.get("full_name", "")
    if name != "README.md" or repository.get("private") is not False or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", full_name):
        return data
    base = re.escape(("https://github.com/" + full_name + "/actions/workflows/").encode())
    pattern = base + rb"(?:ci|security|codeql)\.yml(?:/badge\.svg\?branch=main)?(?=[)\s])"
    return re.sub(pattern, b"", data, flags=re.IGNORECASE)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--deny-file", type=pathlib.Path, help="private newline-separated values; never printed")
    parser.add_argument("--revisions", help="also inspect changed blobs in every commit of this Git revision range")
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    forbidden = args.deny_file.read_bytes().splitlines() if args.deny_file else []
    repository = {}
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if event_path:
        repository = json.loads(pathlib.Path(event_path).read_text()).get("repository", {})
        owner = repository.get("owner", {})
        login = owner.get("login") or owner.get("name")
        if owner.get("type", "User") == "User" and login:
            forbidden.append(login.encode())
    def inspect(name, data):
        label = "[withheld path]" if any(value and value.lower() in name.encode().lower() for value in forbidden) else name
        issues = problems(name.encode() + b"\0" + public_workflow_metadata(name, data, repository), forbidden, allow_contacts=name == "THIRD_PARTY_NOTICES.md")
        if private_path(name):
            issues.append("credential or private runtime file path")
        return [(label, issue) for issue in issues]
    paths = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root
    ).decode().split("\0")
    errors, checked = [], 0
    for name in sorted(set(paths) - {""}):
        path = root / name
        if not path.is_symlink() and not path.is_file():
            continue
        checked += 1
        data = path.readlink().as_posix().encode() if path.is_symlink() else path.read_bytes()
        errors.extend(inspect(name, data))
    commits = []
    if args.revisions:
        commits = subprocess.check_output(["git", "rev-list", "--end-of-options", args.revisions], cwd=root).decode().splitlines()
        for commit in commits:
            changed = subprocess.check_output(["git", "diff-tree", "--root", "--no-commit-id", "--no-renames", "--diff-filter=AMT", "--name-only", "-r", "-m", "-z", commit], cwd=root).decode().split("\0")
            for name in set(changed) - {""}:
                data = subprocess.check_output(["git", "show", f"{commit}:{name}"], cwd=root)
                errors.extend((f"{commit[:12]}:{label}", issue) for label, issue in inspect(name, data))
            message = subprocess.check_output(["git", "show", "--no-patch", "--format=%B", commit], cwd=root)
            if HOME_PATH.search(message):
                errors.append((commit[:12], "personal-home path in commit message"))
    artifact_forbidden = forbidden + [str(root).encode(), str(pathlib.Path.home()).encode()]
    artifacts = []
    for path in args.artifact:
        artifacts.extend(sorted(p for p in path.rglob("*") if p.is_file() or p.is_symlink()) if path.is_dir() else [path])
    for path in artifacts:
        data = path.readlink().as_posix().encode() if path.is_symlink() else path.read_bytes()
        for issue in problems(data, artifact_forbidden, artifact=True):
            errors.append((path.name, issue))
    for name, issue in errors:
        print(f"{name}: {issue}", file=sys.stderr)
    print(f"Public-content check: {checked} source files, {len(commits)} commits, {len(artifacts)} artifacts, {len(errors)} issues")
    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())
