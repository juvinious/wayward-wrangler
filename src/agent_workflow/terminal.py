"""Presentation only: ANSI never enters artifacts, receipts or structured stdout."""
import os
import sys

MODE = "auto"
COLORS = {"heading": "1;36", "action": "1;34", "success": "32", "warning": "1;33",
          "error": "1;31", "muted": "2", "coordinator": "1;36",
          "author": "36", "critic": "35"}

def enabled(stream=None):
    stream = stream or sys.stdout
    if MODE == "never":
        return False
    if MODE == "always":
        return True
    return bool(stream.isatty() and os.environ.get("TERM") != "dumb" and not os.environ.get("NO_COLOR"))

def paint(text, kind="heading", stream=None):
    text = str(text)
    return f"\033[{COLORS.get(kind, COLORS['heading'])}m{text}\033[0m" if enabled(stream) else text

def status(text):
    kind = ("success" if text in ("completed", "approved", "addressed") else
            "error" if text.startswith(("failed", "uncertain", "rejected", "output-invalid")) else
            "warning" if text in ("awaiting-input", "cancelled", "revision-incomplete") else "heading")
    return paint(text, kind)

def link(label, url):
    # Only application-generated review URLs are passed here; no worker-controlled escapes.
    if sys.stdout.isatty() and os.environ.get("TERM") != "dumb":
        return f"\033]8;;{url}\033\\{paint(label, 'action')}\033]8;;\033\\"
    return f"{label}: {url}"


def normalize_command(text):
    """Normalize terminal framing, never artifact identities or multiple commands."""
    from .storage import OperationalError
    # Fallback for terminals forwarding literal bracketed-paste framing.
    text = text.strip()
    if text.startswith("\x1b[200~") and text.endswith("\x1b[201~"):
        text = text[len("\x1b[200~"):-len("\x1b[201~")]
    if "\n" in text or "\r" in text:
        raise OperationalError("Paste one command only; multiline input was not submitted")
    if any((not c.isprintable()) and c not in "\t\u00a0" for c in text):
        raise OperationalError("Unrecognized terminal control characters; type the command again")
    parts = text.strip().split(maxsplit=1)
    return " ".join(parts)


def read_command(prompt):
    # Python's plain input() does not otherwise interpret bracketed-paste framing.
    # Readline collects a paste into one editable input, including embedded newlines.
    import readline
    readline.parse_and_bind("set enable-bracketed-paste on")
    return normalize_command(input(paint(prompt, "action")))
