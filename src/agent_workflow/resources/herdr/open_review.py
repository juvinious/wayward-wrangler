"""Herdr supplies a clicked URL; installed CLI checks project and artifact identity."""
import json
import os
from pathlib import Path
import re

binding = json.loads((Path(__file__).parent / "binding.json").read_text())
launcher = binding.get("launcher")
if launcher:
    url = os.environ.get("HERDR_PLUGIN_CLICKED_URL", "")
    match = re.fullmatch(r"https://awp\.invalid/v2/review/([0-9a-f]{32})/[A-Za-z0-9][A-Za-z0-9_-]{0,63}/(?:draft|critique-1|critique-2|revision-1|revision-2)/[0-9a-f]{64}", url)
    if not match:
        raise SystemExit("Review link has no unambiguous project identity")
    index = Path(binding["registry"]) / "locations.json"
    projects = json.loads(index.read_text()).get("projects", {})
    entry = projects.get(match.group(1))
    if not entry or not Path(entry["project"]).is_dir():
        raise SystemExit("Review project location is missing; rebind it explicitly")
    executable = Path(launcher)
    if not executable.is_file():
        raise SystemExit("Wayward Wrangler launcher is missing; reinstall the paired release")
    os.execve(str(executable), [str(executable), "--project", entry["project"],
                                "--registry-dir", binding["registry"], "open-link"], os.environ.copy())
python = Path(binding["python"])
if not python.is_file():
    raise SystemExit("Wayward Wrangler installation is missing; reinstall Herdr integration")
os.execve(str(python), [str(python), "-m", "agent_workflow.cli",
                          "--registry-dir", binding["registry"], "open-link"], os.environ.copy())
