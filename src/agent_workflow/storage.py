from __future__ import annotations
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
LOCAL = ROOT / ".local"

class OperationalError(RuntimeError):
    pass

class Uncertain(OperationalError):
    pass

class OutputInvalid(OperationalError):
    pass

def canonical(value):
    return json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n"

def digest(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()

def read(path):
    return json.loads(Path(path).read_text())

def atomic(path, value, *, immutable=False):
    path = Path(path)
    data = canonical(value).encode() if not isinstance(value, (str, bytes)) else value
    if isinstance(data, str):
        data = data.encode()
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".publish-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as out:
            out.write(data)
            out.flush()
            os.fsync(out.fileno())
        if immutable:
            try:
                os.link(tmp, path)
            except FileExistsError:
                if path.read_bytes() != data:
                    raise OperationalError(f"Immutable content conflict: {path}")
        else:
            os.replace(tmp, path)
        directory = os.open(path.parent, os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        Path(tmp).unlink(missing_ok=True)

@contextlib.contextmanager
def lock(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise OperationalError(f"Already owned: {path}")
        try:
            yield
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)

def identity(pid=None):
    pid = pid or os.getpid()
    stat = Path(f"/proc/{pid}/stat").read_text()
    fields = stat[stat.rindex(")") + 2:].split()
    return {"pid": pid, "start": fields[19],
            "boot": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "state": fields[0]}

def alive(expected):
    try:
        actual = identity(expected["pid"])
        return all(actual[k] == expected[k] for k in ("pid", "start", "boot")) and actual["state"] != "Z"
    except (OSError, KeyError, ProcessLookupError):
        return False

def operation_path(path, *, local=None):
    path = Path(path).resolve()
    if local is None:
        from .runtime import current
        selected = current()
        local = selected.state if selected else LOCAL
    if not path.is_relative_to(Path(local).resolve()):
        raise OperationalError("Operation path must be inside the selected state directory")
    return path

def now():
    return time.time()
