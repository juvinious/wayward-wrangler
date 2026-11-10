# Verify the daily-use loop

The next operator walkthrough is deferred until the
[interaction/lifecycle changes](usability-plan.md#finish-before-the-next-operator-test)
are complete. The instructions below describe the current build and later use
checks. Direct interrupt/leave/pause and automatic obsolete-session renewal are
implemented locally; live steering/follow-up and remaining editor/action work are
still incomplete. Normal startup retires obsolete/stranded session context rather
than preserving it as a live update dependency. Operator state has not been reset
by these isolated checks; do not reopen until the sequence is finished.

The candidate uses the shared model runtime on Linux x86_64 with a systemd user
manager, Node >=22.19 and `flock`. Configure a provider through the SDK's explicit
login/environment setup, then choose a provider/model in F3. No native Codex/Claude
CLI is required. Retired profiles need reconfiguration. Schema2 projects can open
for inspection and F3 setup while execution stays blocked. Choose a shared
provider/model, S save, then A reviews activation. Choose Activate and share project
context and Enter to request the switch after the current turn.
Existing request policy amendments remain explicit. See [provider setup](provider-selection.md).
Automated fixtures do not establish account login or model reliability.

The agent left the operator installation absent and made only local commits on
`plan/usability-and-installation`. You review and perform remote delivery.

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
Use `waywr models login PROVIDER --method oauth` (or `apiKey` when supported), then
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
When testing a paired bundle through the ignored project-local symlink instead,
detach the existing TUI and relaunch that symlink after it is refreshed. Idle
handover supports this portable layout without a global installation.
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
