# Verify the daily-use loop

Use this walkthrough to install Waywr from source, do a small development task,
leave the application, rebuild it and return to the same project.

Waywr uses the shared model runtime on Linux x86_64 with a systemd user
manager, Node >=22.19 and `flock`. Configure a provider through the SDK's explicit
login/environment setup, then choose a provider/model in F3. No native Codex/Claude
CLI is required. Normal startup renews retired settings/bindings automatically;
valid shared model choices remain. For a model change, /model and /thinking open
their pickers; S saves and A reviews activation. Choose Activate and share project
context to switch after the current turn. /status shows the active state/usage.
Existing request policy amendments remain explicit. See [provider setup](provider-selection.md).

## Install and use

From this Waywr checkout, in Fish:

```fish
sh scripts/dev-install.sh
fish_add_path "$HOME/.local/bin" # once, if needed
waywr --version
```

Then enter the project you want to use and run `waywr`. A new project prompts for
setup. Existing project work/settings/history are preserved; there is no
per-project application version to select. Installation does not sign in or import another agent application's credentials.
Use `/login` in the TUI (or G in F3 Profiles) to choose the SDK browser/API-key
flow. Esc cancels and preserves the draft; Ctrl-B reopens a supplied browser URL.
The CLI fallback is `waywr models login PROVIDER --method oauth` (or `apiKey`), then
F3 →1 →F to select a model and S to save before requesting work.
Choose Thinking in the profile form to select one of that model's advertised
levels before saving. F3 labels draft and active selections; the main header
shows the saved coordinator model/level first and any different active binding
with its readiness state.
After a coordinator conversation call, the conversation also shows that call's
model/level, token/cache counts and any cost/context estimate. During a call it
refreshes read-only activity; after completion and reopening it retains the latest
subject-local call. Missing telemetry stays unknown and account quota is unavailable.
For an existing project, use A in Profiles after saving a different coordinator.
Review the model, thinking level and destination, select Activate and share project
context, then Enter. The default Keep current and Esc leave the binding alone.
Saving alone does not activate it; activation does not resume a paused session.
Section4 shows switch status and advanced controls. A policy exclusion still needs
an explicit amendment in section6.
If the active binding is retired, Send opens the saved model review directly;
there is no need to find the Switch section. Activation returns to the unsent
draft. An unrelated old request cannot veto a new discussion's model choice.
Ctrl-C or Escape in the plain composer clears nonempty input and its saved draft;
with empty input it interrupts only the foreground coordinator. Escape
inside a popup still returns one level without clearing the composer.

While a coordinator reply is active, Enter sends live steering at its next
boundary; Alt+Enter queues a follow-up for after the reply. F7 labels pending,
applied and held inputs; E restores a pending input for editing without sending.
Up/Down moves inside multiline input and recalls history at the first/last line.
Ctrl-J/Shift+Enter adds a newline; F2 remains the fallback. Interrupt, keep typing,
leave while stopping, then reopen and verify no automatic input replay.

Discuss one small useful development request. Review its scope/policy card and
authorize it only when correct. Local delivery keeps this first verification
within your checkout; remote actions remain your decision. Inspect the actual
changes and check results. Record any manual intervention, confusing choice,
unexpected block or missing explanation.

## Rebuild and return

Use `/pause` if you want to stop new dispatch, then Ctrl-Q or `/exit` to leave
immediately. Leaving alone keeps background work running. From the Waywr source checkout,
repeat `sh scripts/dev-install.sh` after a local application change. Then return
to the same project and run `waywr` again. `waywr --version` shows the new generated
artifact ID. No manual bundle IDs, rebinding or project upgrade command is needed.
Conversation, settings, draft/work and intentional pause should remain. Resume
only when you intend to restart dispatch.

An idle old service is replaced automatically. A compatible busy owner remains
attachable without replacement; obsolete/stranded sessions use automatic renewal.
Source and dirty worktrees remain. `/detach` deliberately leaves background work
running. Older installation artifacts are retained for now;
automatic cleanup and uninstall commands are subsequent work.

The success criterion is completing useful work, rebuilding Waywr and continuing
in that project without repair commands. Report concrete steps and the visible
failure when that loop is interrupted. Public curl installation, `waywr update` /
`uninstall` and additional package channels are not implemented by this increment;
no hosted binary release has been published.
