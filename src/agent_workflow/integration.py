"""Explicit Herdr plugin installation; no shared registration on wheel install."""
from importlib.resources import files
import json
import os
from pathlib import Path
import subprocess
import sys
from .runtime import registry_default
from .storage import OperationalError, atomic


def install_herdr(args):
    base = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "wayward-wrangler/herdr"
    destination = Path(args.destination).expanduser() if args.destination else base
    if not destination.is_absolute():
        raise OperationalError("Plugin destination must be absolute")
    destination = destination.resolve()
    registry = Path(args.registry_dir).expanduser().resolve() if args.registry_dir else registry_default().resolve()
    # Keep the venv interpreter path. Resolving its symlink to /usr/bin/python
    # would discard the installed package environment during a Herdr click.
    python = Path(sys.executable).absolute()
    proposed = {"schema_version": 1, "python": str(python), "registry": str(registry)}
    release = python.parent.parent
    launcher = release.parent.parent / "bin/waywr"
    if (release / "manifest.json").is_file() and launcher.is_file():
        proposed["launcher"] = str(launcher)
    old = destination / "binding.json"
    if old.exists() and json.loads(old.read_text()) != proposed:
        raise OperationalError("Plugin destination belongs to another installation; choose a new destination")
    environment = {**os.environ, **({"HERDR_SOCKET_PATH": args.socket} if args.socket else {})}
    listing = subprocess.run(["herdr", "plugin", "list", "--json"], capture_output=True, text=True, env=environment)
    if listing.returncode:
        raise OperationalError("Could not inspect Herdr plugin registrations: " + listing.stderr.strip())
    registrations = json.loads(listing.stdout).get("result", {}).get("plugins", [])
    matches = [item for item in registrations if item.get("plugin_id") == "local.agent-workflow-review"]
    if len(matches) > 1 or (matches and Path(matches[0]["plugin_root"]).resolve() != destination):
        raise OperationalError("Existing Herdr review plugin is registered elsewhere; unlink it explicitly before installing")
    destination.mkdir(parents=True, exist_ok=True)
    source = files("agent_workflow.resources.herdr")
    for name in ("herdr-plugin.toml", "open_review.py"):
        atomic(destination / name, source.joinpath(name).read_bytes())
    atomic(old, proposed, immutable=True)
    if matches:
        registration = "already linked at the same destination"
    else:
        linked = subprocess.run(["herdr", "plugin", "link", str(destination)], capture_output=True, text=True, env=environment)
        if linked.returncode:
            raise OperationalError("Herdr plugin link failed: " + linked.stderr.strip())
        registration = linked.stdout.strip()
    return {"destination": str(destination), "registry": str(registry), "python": str(python),
            "registration": registration}
