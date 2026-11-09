"""Herdr supplies clicked URL and caller context; never execute URL contents."""
import os
from pathlib import Path

command = Path(__file__).resolve().parents[1] / ".venv/bin/wayward-wrangler"
os.execv(str(command), [str(command), "open-link"])
