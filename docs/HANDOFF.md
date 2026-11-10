# Waywr handoff — activate a saved model from Profiles

Branch: `plan/usability-and-installation`. Base: `d742c65` (saved versus active
model header). Follow [AGENTS.md](../AGENTS.md), [controls](tui-controls.md) and
[provider contract](provider-selection.md). Local commits only; the user handles
remote delivery. No operator service/state/settings or account was changed.

## Current outcome

F3 Profiles now offers S save → A review activation. The review shows the exact
saved provider/model, thinking level, destination and options. Keep current is
the default; arrows or Tab choose Activate and share project context, and Enter
on key press requests the existing after-current-turn coordinator switch with an
explicit context grant. Esc returns to the form. The review pins its summary and
actions while the details scroll. The save acknowledgement points to A.

Unsaved drafts cannot activate. The review freezes the saved profile, preference
revision, binding generation and conversation subject; changed preferences or
binding invalidate confirmation. Existing backend role/policy/context and
uncertain-call checks remain authoritative. Activation preserves pause and worker
assignments, does not sign in, and does not amend request policies. Section4 keeps
switch status, cancellation and interruption controls; section6 owns amendments.

The main header still shows the saved coordinator model/level first and any
different active binding with its readiness state. A preference save alone changes
neither the active binding nor its context grant. No model allowlist was added.

## Verification

The two input/render regressions first failed against the previous behavior. The
focused provider UI checks now pass, covering explicit confirmation, default Keep,
repeat-key suppression, unsaved edits, stale revisions and changed generations.
The locked remapped release build passes. A real isolated paired-binary TUI check
selects and saves a catalog-tail model and thinking level, explicitly activates it
while paused, removes the old application build, and reopens with the same binding,
draft, settings and populated history. No model calls or manual rebinds occurred.
The harness reconstructs positioned terminal text to read Ratatui diff frames.
CI/release definitions include this activation journey; hosted workflows have not
been run by the agent. Dependencies and locks are unchanged.

Source and review-bundle public-content checks pass. Documentation/control owners
are reconciled. The existing snapshot fix retains whole replies up to8MiB,
1MiB incoming and64KiB command budgets, correlated oversize errors and bounded
status handover with process/socket/project/idle ownership checks. No backend
execution or IPC code changed in this slice. Real account login, model quality,
useful task delivery and public download/update/package channels remain
unverified/incomplete; fixture success establishes none of those.

## Pending cleanup decision

The user proposed discarding old development-era state. Read-only inspection found
36 managed worktrees, one with a tracked edit and seven untracked entries, and nine
with commits absent from the primary checkout. The primary checkout is clean and
current provider configuration is schema3. The requested cleanup-scope question
has not been answered. No worktree, branch, history or state has been deleted.
Do not interpret later screenshots or permission to continue as disposal authority.

## Next authorized action

The ignored `target/local-bin/waywr` entry uses `v0.4.0-local.activation`, manifest
SHA256 `55f3884c182a896c13890b9acf49a9bbb02a1f059843fe252433d5d848e9d6af`.
The user detaches the open TUI and relaunches to try F3 Profiles → S → A. Activating
the operator project's binding remains the user's explicit action. Paused work
requires a separate resume. Continue from concrete ordinary-use friction, with
one visible acceptance outcome at a time. Resolve cleanup scope before destructive
work; do not run account/provider calls or change operator services to make checks
pass.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[distribution](distribution.md), [IPC](contracts/session-protocol.md),
[verification](verify-usability.md).
