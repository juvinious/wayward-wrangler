"""Explicit project and state binding for an installed controller or runner."""
from __future__ import annotations

from contextvars import ContextVar
import contextlib
from dataclasses import dataclass
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import uuid
import sqlite3

from .storage import OperationalError, atomic, lock, read

_current: ContextVar[RuntimePaths | None] = ContextVar("wayward_runtime", default=None)

@contextlib.contextmanager
def registry_lock(registry):
    """Short metadata transactions may wait for another status/registration call."""
    path = Path(registry) / "registry.lock"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)


@dataclass(frozen=True)
class RuntimePaths:
    project: Path
    state: Path
    registry: Path
    key: str
    legacy: bool = False
    legacy_origin: Path | None = None


def current() -> RuntimePaths | None:
    return _current.get()


def bind(paths: RuntimePaths):
    return _current.set(paths)


def unbind(token):
    _current.reset(token)


def project_file(project):
    path = Path(project).expanduser().resolve()
    if not path.is_dir():
        raise OperationalError(f"Project directory does not exist: {path}")
    return path


def selected_path(value, project):
    path = Path(value).expanduser()
    return (path if path.is_absolute() else project / path).resolve()


def registry_default():
    base = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state"))
    return base / "wayward-wrangler"


def load(project, state_dir=None, registry_dir=None, *, initialize=False, legacy_origin=None):
    project = project_file(project)
    config = project / ".wayward-wrangler/project.toml"
    if initialize:
        if config.exists():
            raise OperationalError(f"Project already initialized: {config}")
        config.parent.mkdir(parents=True, exist_ok=True)
        key = uuid.uuid4().hex
        atomic(config, f'schema_version = 1\nproject_key = "{key}"\n', immutable=True)
        from importlib.resources import files
        workflow = config.parent / "workflow.toml"
        atomic(workflow, files("agent_workflow.resources").joinpath("fake-workflow.toml").read_text(), immutable=True)
        ignore = project / ".gitignore"
        rule = ".wayward-wrangler/state/"
        prior = ignore.read_text() if ignore.exists() else ""
        if rule not in prior.splitlines():
            atomic(ignore, prior + ("\n" if prior and not prior.endswith("\n") else "") + rule + "\n")
        if shutil.which("git"):
            target_ignored = subprocess.run(["git", "-C", str(project), "check-ignore", "--no-index", "-q",
                                             str(project)], capture_output=True).returncode == 0
            if not target_ignored:
                for versionable in (config, workflow, config.parent / "context/selection.toml"):
                    checked = subprocess.run(["git", "-C", str(project), "check-ignore", "--no-index", "-q",
                                              str(versionable)], capture_output=True)
                    if checked.returncode == 0:
                        raise OperationalError(f"Project ignore rules hide versionable configuration/context: {versionable}")
    if not config.is_file():
        # Existing checkout state is selected explicitly; never migrate its files.
        if state_dir is None or selected_path(state_dir, project) != project / ".local":
            raise OperationalError(f"Missing project configuration: {config}; run init first")
        key = "legacy-" + uuid.uuid5(uuid.NAMESPACE_URL, str(project)).hex
        legacy = True
    else:
        import tomllib
        value = tomllib.loads(config.read_text())
        key = value.get("project_key")
        if value.get("schema_version") != 1 or not isinstance(key, str) or not re.fullmatch(r"[0-9a-f]{32}", key):
            raise OperationalError(f"Invalid project identity: {config}")
        legacy = False
    state = selected_path(state_dir, project) if state_dir else project / ".wayward-wrangler/state"
    registry = Path(registry_dir).expanduser().resolve() if registry_dir else registry_default().resolve()
    if registry_dir and not Path(registry_dir).expanduser().is_absolute():
        raise OperationalError("--registry-dir must be absolute")
    origin = Path(legacy_origin).expanduser().resolve() if legacy_origin else None
    if origin and not legacy:
        raise OperationalError("--legacy-origin applies only to an explicit legacy state root")
    return RuntimePaths(project, state, registry, key, legacy, origin)


def register(paths: RuntimePaths):
    """Refuse implicit takeover of a project key or state root."""
    marker = paths.state / "project-identity.json"
    with registry_lock(paths.registry):
        index_path = paths.registry / "locations.json"
        index = read(index_path) if index_path.exists() else {"schema_version": 1, "projects": {}}
        for key, entry in index["projects"].items():
            if key != paths.key and entry["state"] == str(paths.state):
                raise OperationalError("State directory is registered to another project")
        old = index["projects"].get(paths.key)
        expected = {"project": str(paths.project), "state": str(paths.state)}
        if old and old != expected:
            raise OperationalError("Project key has a different registered location; explicit rebind required")
        if marker.exists() and read(marker) != {"schema_version": 1, "project_key": paths.key}:
            raise OperationalError("State directory identity does not match selected project")
        if not marker.exists() and paths.state.exists() and any(paths.state.iterdir()):
            raise OperationalError("Nonempty unbound state directory; explicit legacy binding required")
        atomic(marker, {"schema_version": 1, "project_key": paths.key}, immutable=True)
        index["projects"][paths.key] = expected
        atomic(index_path, index)


def lookup(registry, key):
    index_path = Path(registry) / "locations.json"
    if not index_path.exists():
        raise OperationalError("Review location index missing; install/rebind the Herdr integration")
    projects = read(index_path).get("projects", {})
    if key not in projects:
        raise OperationalError("Review project location missing; rebind it explicitly")
    entry = projects[key]
    paths = load(entry["project"], entry["state"], registry)
    marker = paths.state / "project-identity.json"
    try:
        valid_marker = marker.is_file() and read(marker) == {"schema_version": 1, "project_key": key}
    except (OSError, ValueError):
        valid_marker = False
    if paths.key != key or not valid_marker:
        raise OperationalError("Review project location is stale; rebind it explicitly")
    return paths


def rebind_location(paths: RuntimePaths):
    """Explicit quiescent relocation; append sidecar, never alter run evidence."""
    from .storage import alive, now
    from .herdr_backend import receipt
    marker = paths.state / "project-identity.json"
    if not marker.is_file() or read(marker) != {"schema_version": 1, "project_key": paths.key}:
        raise OperationalError("Relocated state lacks the matching project identity marker")
    def quiescent(state):
        database = state / "state.sqlite"
        if database.exists():
            with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as db:
                if db.execute("SELECT count(*) FROM app_runs WHERE active=1").fetchone()[0]:
                    raise OperationalError("Active or unresolved run; location rebind refused")
        for operation in (state / "runs").glob("*/operations/*"):
            if not operation.is_dir():
                continue
            for name, field in (("claim.json", "runner"), ("child.json", "identity")):
                file = operation / name
                if file.exists() and alive(read(file)[field]):
                    raise OperationalError("Live worker exists; location rebind refused")
            if (operation / "operation.json").exists():
                if not (operation / "receipt.json").exists():
                    raise OperationalError("Unresolved operation; location rebind refused")
                receipt(operation)
    quiescent(paths.state)
    with registry_lock(paths.registry):
        index_path = paths.registry / "locations.json"
        index = read(index_path)
        old = index["projects"].get(paths.key)
        if old is None:
            raise OperationalError("Project key was not previously registered")
        new = {"project": str(paths.project), "state": str(paths.state)}
        if old == new:
            return {"status": "already-bound", "project_key": paths.key, "location": new}
        previous = Path(old["state"])
        if previous.exists():
            quiescent(previous)
        history = paths.registry / "rebindings" / paths.key
        number = len(list(history.glob("*.json"))) + 1
        record = {"schema_version": 1, "project_key": paths.key, "from": old, "to": new,
                  "recorded_at": now(), "scope": "location metadata only; run evidence unchanged"}
        atomic(history / f"{number:04d}.json", record, immutable=True)
        index["projects"][paths.key] = new
        atomic(index_path, index)
        return {"status": "rebound", "record": record}
