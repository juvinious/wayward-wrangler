#!/usr/bin/env python3
"""Collect upstream Rust notices from the locked Cargo sources, without host paths."""
import argparse
import json
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true")
args = parser.parse_args()
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=root))
tree = subprocess.check_output(["cargo", "tree", "--locked", "--offline", "--target", "x86_64-unknown-linux-gnu", "--edges", "normal,build", "--prefix", "none", "--format", "{p}"], cwd=root).decode()
selected = set(re.findall(r"^([\w-]+) v([^\s]+)", tree, re.M))
packages = sorted((p for p in metadata["packages"] if p["source"] and (p["name"], p["version"]) in selected), key=lambda p: (p["name"], p["version"]))
parts = ["# Third-party notices\n\nGenerated from the workspace Cargo.lock by `scripts/generate-notices.py`.\nCovers normal/build dependencies for the supported Linux x86_64 GNU bundle.\nDevelopment-only and other-platform packages are not shipped by that bundle.\nUpstream attributions are retained as required by their licenses. The project\nlicense does not replace these licenses.\n"]
missing = []
for package in packages:
    source = pathlib.Path(package["manifest_path"]).parent
    candidates = set()
    if package.get("license_file"):
        candidates.add(source / package["license_file"])
    for path in source.iterdir():
        if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE", "COPYRIGHT")):
            candidates.add(path)
    for folder in ("licenses", "LICENSES"):
        if (source / folder).is_dir():
            candidates.update(p for p in (source / folder).rglob("*") if p.is_file())
    if not candidates:
        missing.append(package["name"] + " " + package["version"])
        continue
    parts.append(f"\n## {package['name']} {package['version']}\n\nLicense expression: `{package.get('license') or 'see upstream notice'}`.\n")
    for path in sorted(candidates):
        relative = path.relative_to(source).as_posix()
        text = path.read_text(encoding="utf-8").strip()
        parts.append(f"\n### {relative}\n\n```text\n{text}\n```\n")
if missing:
    raise RuntimeError("Missing upstream license text: " + ", ".join(missing))
result = "".join(parts)
destination = root / "THIRD_PARTY_NOTICES.md"
if args.check:
    if not destination.exists() or destination.read_text() != result:
        sys.exit("Third-party notices differ; regenerate from locked sources")
else:
    destination.write_text(result)
print(f"Third-party notices: {len(packages)} locked Rust packages")
