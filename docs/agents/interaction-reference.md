# Interaction reference notes

Audience: agents revisiting control design. These upstream observations were
recorded on2026-11-10; they are not current product controls or task permissions.
[Waywr controls](../tui-controls.md) owns the implemented behavior.


Codex documents Enter as steering during work, Tab as a queued follow-up, prompt
history/search, slash-command discovery, model/status controls, keymap editing,
saved-session resume and direct exit. Its keymap is contextual. See the official
[interactive shortcuts](https://learn.chatgpt.com/docs/developer-commands?surface=cli#interactive-shortcuts),
[keymap command](https://learn.chatgpt.com/docs/developer-commands?surface=cli#remap-tui-shortcuts-with-keymap)
and [resume/exit commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli).

Pi documents Enter for steering, Alt+Enter for follow-up and Escape for abort;
its editor supports newline/history actions and restores queued input after abort.
Its configurable named actions separate interrupt, clear and exit, with model and
thinking shortcuts. See [terminal usage](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/docs/usage.md),
[keybindings](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/docs/keybindings.md)
and [action definitions](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/src/core/keybindings.ts).
These are upstream observations checked on the date above, not a claim that every
shortcut or delivery boundary is identical across versions and terminals.

Waywr uses the shared libraries, not Pi's application. The pinned agent class
provides steer, followUp and abort. The [runtime entry](../../model-runtime/main.mjs)
now claims an attempt-scoped inbox between agent steps and calls Agent.steer.
Follow-ups remain separate service turns, each with its own validated result.
See [live input](../live-input.md) for pending/applied/held state, editing and replay
boundaries. An older owner without this capability queues until idle handover.


## Installer comparison recorded on2026-11-11

[Pi's current quickstart](https://pi.dev/docs/latest/quickstart) defaults to a
shell installer with pinned dependencies and `pi update`. It also offers npm and
Nix; those channels have their own update ownership. Its
[installer source](https://pi.dev/install.sh) stages a managed npm release behind
a stable launcher and can provision a private Node runtime. Pi is not distributed
solely as one dependency-free native executable.
[Codex's CLI guide](https://learn.chatgpt.com/docs/codex/cli) likewise defaults to
a shell installer on macOS/Linux and updates by rerunning it. npm and Homebrew
are additional channels. These are delivery references, not a reason to adopt
either application's lifecycle or expand Waywr's supported platforms.
