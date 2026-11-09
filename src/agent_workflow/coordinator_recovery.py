"""One explicit fake-coordinator replacement after a proven pre-permit runner exit.

The failed operation remains immutable. A missing receipt alone is never proof.
"""
from __future__ import annotations

from pathlib import Path

from .storage import OperationalError, alive, canonical, digest, read


ABSENT = ("permit.json", "dispatch.json", "child.json", "execution.json",
          "invocations.json", "result.json", "receipt.json", "transcript.log",
          "uncertain.json", "cancel.json")


def verify(paths, task, call):
    """Return the original attempt only when its frozen evidence proves no child call."""
    if (task["status"] != "blocked" or task["run_id"] is not None or
            call["status"] != "uncertain" or call["task_id"] != task["task_id"]):
        raise OperationalError("Coordinator recovery requires a blocked pre-workflow call")
    directory = paths.state / "session/coordinator" / task["task_id"] / f"call-v{task['version']}"
    try:
        packet = read(directory / "packet.json")
        op = read(directory / "operation.json")
        launch = read(directory / "launch.json")
        claim = read(directory / "claim.json")
    except (OSError, ValueError, KeyError) as error:
        raise OperationalError("Pre-permit recovery evidence is incomplete") from error
    if any((directory / name).exists() for name in ABSENT):
        raise OperationalError("Execution or terminal-outcome evidence exists; recovery refused")
    if ((paths.state / "session/coordinator" / task["task_id"] / f"call-v{task['version']}-r1" / "launch.json").exists()):
        raise OperationalError("Replacement attempt already launched; inspect it")
    if (packet.get("operation_kind") != "coordinator" or packet.get("provider") != "fake" or
            packet.get("task_id") != task["task_id"] or packet.get("task_version") != task["version"] or
            packet.get("message") != task["body"] or packet.get("project_key") != paths.key or
            op.get("packet_sha256") != digest(canonical(packet)) or call["packet_sha256"] != op["packet_sha256"] or
            op.get("operation_id") != packet.get("operation_id") or op.get("project_key") != paths.key or
            op.get("launch_token") is None or launch.get("status") != "may-have-been-sent" or
            launch.get("target") != op.get("target") or claim.get("herdr", {}).get("workspace_id") != op["target"]["workspace"]):
        raise OperationalError("Frozen coordinator identity or packet does not match the task")
    if alive(claim["runner"]):
        raise OperationalError("Original coordinator runner is still alive")
    return directory, op, claim
