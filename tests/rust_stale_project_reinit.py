"""Reproduce and verify reinit after a project's local files disappear.

Uses isolated project/registry paths and never starts a service or Herdr.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def invoke(exe, project, env):
    result = subprocess.run(
        [str(exe), "--registry-dir", str(env["WW_TEST_REGISTRY"]),
         "--ipc-dir", str(env["XDG_RUNTIME_DIR"]), "init", "."],
        cwd=project, env=env, text=True, capture_output=True, timeout=20,
    )
    return {"code": result.returncode, "stdout": result.stdout.strip(),
            "stderr": result.stderr.strip()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--old", type=Path, required=True)
    parser.add_argument("--new", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    root = args.evidence.resolve()
    assert not root.exists(), "Evidence path must be fresh"
    root.mkdir(parents=True)
    project = root / "project"; project.mkdir()
    home = root / "home"; home.mkdir()
    runtime = root / "runtime"; runtime.mkdir()
    registry = root / "registry"
    env = {**os.environ, "HOME": str(home), "XDG_STATE_HOME": str(root / "state"),
           "XDG_RUNTIME_DIR": str(runtime), "WW_TEST_REGISTRY": str(registry)}
    first = invoke(args.old, project, env)
    assert first["code"] == 0, first
    old_key = json.loads(first["stdout"])["project_key"]
    archive = root / "archived-original-project-files"
    shutil.move(str(project / ".wayward-wrangler"), archive)
    original_marker_hash = digest(archive / "state/project-identity.json")

    prior = invoke(args.old, project, env)
    assert prior["code"] != 0 and "registered to another project" in prior["stderr"], prior
    old_index_hash = digest(registry / "locations.json")
    current = invoke(args.new, project, env)
    assert current["code"] == 0, current
    new_key = json.loads(current["stdout"])["project_key"]
    assert old_key != new_key
    assert digest(registry / "locations.json") == old_index_hash, "init changed the external registry"
    location = json.loads((registry / "locations.json").read_text())["projects"]
    assert old_key in location and new_key not in location
    assert json.loads((project / ".wayward-wrangler/state/project-identity.json").read_text())["project_key"] == new_key
    assert digest(archive / "state/project-identity.json") == original_marker_hash

    config = project / ".wayward-wrangler"
    before = {path.name: digest(path) for path in config.iterdir() if path.is_file()}
    repeated = invoke(args.new, project, env)
    assert repeated["code"] == 0, repeated
    after = {path.name: digest(path) for path in config.iterdir() if path.is_file()}
    assert before == after
    assert digest(registry / "locations.json") == old_index_hash

    fresh = root / "fresh-project"; fresh.mkdir()
    fresh_registry = root / "fresh-registry"
    fresh_env = {**env, "WW_TEST_REGISTRY": str(fresh_registry)}
    fresh_init = invoke(args.new, fresh, fresh_env)
    assert fresh_init["code"] == 0, fresh_init
    assert not fresh_registry.exists(), "Fresh init created a user-local registry"
    assert (fresh / ".wayward-wrangler/state/project-identity.json").is_file()
    assert not any(runtime.rglob("*.sock"))
    summary = {"old_failure": prior, "new_success": current, "repeated_success": repeated,
               "fresh_success": fresh_init,
               "old_key": old_key, "new_key": new_key,
               "original_state_marker_sha256": original_marker_hash,
               "old_state_retained": str(archive), "registry_unchanged_by_init": True,
               "fresh_init_created_registry": False, "service_sockets": 0}
    (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"result": "passed", "evidence": str(root / "summary.json")}))


if __name__ == "__main__":
    main()
