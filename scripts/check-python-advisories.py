#!/usr/bin/env python3
"""Check locked Python package identities with OSV; no source is transmitted."""
import json
import pathlib
import sys
import urllib.request

root = pathlib.Path(__file__).resolve().parents[1]
packages = [line.split("==", 1) for line in (root / "requirements.lock.txt").read_text().splitlines() if "==" in line]
queries = [{"package": {"name": name, "ecosystem": "PyPI"}, "version": version} for name, version in packages]
request = urllib.request.Request("https://api.osv.dev/v1/querybatch", data=json.dumps({"queries": queries}).encode(), headers={"Content-Type": "application/json"})
with urllib.request.urlopen(request, timeout=60) as response:
    results = json.load(response)["results"]
if len(results) != len(packages):
    raise RuntimeError("Incomplete advisory response")
findings = 0
for (name, version), result in zip(packages, results):
    ids = sorted({item["id"] for item in result.get("vulns", [])})
    if ids:
        findings += 1
        print(f"{name} {version}: {', '.join(ids)}")
print(f"OSV Python check: {len(packages)} locked packages; {findings} affected packages")
sys.exit(bool(findings))
