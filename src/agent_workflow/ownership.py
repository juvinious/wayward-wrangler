"""Fencing for a Herdr controller created by the project supervisor."""
from __future__ import annotations

from contextvars import ContextVar
from contextlib import contextmanager
from pathlib import Path
from .storage import OperationalError, atomic, read, lock

_generation = ContextVar("controller_generation", default=None)
_service_generation = ContextVar("project_service_generation", default=None)


def path() -> Path:
    from .records import state_dir
    return state_dir() / "supervision" / "owner.json"


def bind(generation: int):
    return _generation.set(generation)


def unbind(token):
    _generation.reset(token)


def bind_service(generation: int):
    return _service_generation.set(generation)


def unbind_service(token):
    _service_generation.reset(token)


def current():
    location = path()
    return read(location) if location.exists() else None


def next_owner(*, run_id: str, token: str, server: dict, workspace: str):
    with lock(path().parent / "fence.lock"):
        previous = current()
        generation = (previous["generation"] if previous else 0) + 1
        value = {"schema_version": 1, "run_id": run_id, "generation": generation,
                 "token": token, "server": server, "workspace": workspace}
        atomic(path(), value)
        return value


def check():
    service_generation = _service_generation.get()
    if service_generation is not None:
        from .records import state_dir
        service_file = state_dir() / "session/service-generation.json"
        if not service_file.exists() or read(service_file)["generation"] != service_generation:
            raise OperationalError("Stale project service generation; dispatch refused")
    owner = current()
    if owner is None:
        return
    generation = _generation.get()
    if generation != owner["generation"]:
        raise OperationalError("Stale or unowned controller; dispatch refused")


@contextmanager
def fenced():
    with lock(path().parent / "fence.lock"):
        check()
        yield
