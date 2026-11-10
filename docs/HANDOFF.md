# Waywr handoff — saved model header versus active binding

Branch: `plan/usability-and-installation`. Base: `6956061` (populated snapshots).
Follow [AGENTS.md](../AGENTS.md), [controls](tui-controls.md) and
[provider contract](provider-selection.md). Local commits only; the user handles
remote delivery. No operator service/state/settings or account was changed.

## Current outcome

The user's screenshot showed a saved shared-runtime profile with thinking high,
while the purple main header displayed the old blocked native binding's legacy
medium. The save had succeeded; the header described a different state without
making that distinction clear enough.

The primary model header now says Saved Model and reads the authoritative saved
coordinator preference, including provider/model and thinking level. A different
current binding appears on a separate Active (STATUS) line. Matching values need
no duplicate line; older modes without shared preferences keep their prior view.
A saved preference revision updates the primary line through normal snapshots.
Unsaved F3 edits remain Draft; neither display nor preference save changes the
active coordinator, grants context or authorizes model execution. The header
allows a bounded extra wrapped row so both labels can be read at normal widths.

## Verification

The input/render regression first failed because the primary line retained the
old binding. It now shows saved high and separately labels blocked legacy medium;
a later authoritative snapshot changes the saved level while preserving the active
binding and emitting no action. Existing provider/Thinking/control checks pass.
The locked remapped release build and actual paired-binary TUI save/reopen fixture
pass with populated history, model/thinking persistence, saved draft/pause and no
model calls. Only display/labels changed; backend execution and IPC are unchanged.
Public-content checks cover source/binaries and the review bundle. Local CI/release
workflow edits include the focused provider/header UI checks. Dependencies
and locks are unchanged. Hosted workflows have not been run by the agent.

The prior snapshot fix still delivers whole replies up to8MiB while retaining
1MiB incoming and64KiB command budgets. Oversized replies are correlated errors.
Bounded-status handover supports old services that cannot deliver their snapshot;
process/socket/project/idle ownership checks remain. No catalog/history truncation
or provider/model allowlist was added. Real account login/model quality/useful
delivery and public download/update/package channels remain unverified/incomplete.

## Pending cleanup decision

The user proposed discarding old development-era state. Read-only inspection found
36 managed worktrees, one with a tracked edit and seven untracked entries, and nine
with commits absent from the primary checkout. The primary checkout is clean and
current provider configuration is schema3. The requested cleanup-scope question
has not been answered. No worktree, branch, history or state has been deleted.
Do not interpret the later header screenshot as authorization to discard code.

## Next authorized action

The ignored `target/local-bin/waywr` entry uses `v0.4.0-local.header`, manifest
SHA256 `0045e341166fc674504766dbc5bdc1ea627333857471ec6485d210721f581ae7`. The
user detaches the open TUI and relaunches to see Saved versus Active explicitly.
Switch/context/policy changes remain explicit, not a consequence of rendering.
Resolve cleanup scope before any destructive work; otherwise continue from the
next concrete ordinary-use issue rather than broad testing or legacy migration.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[distribution](distribution.md), [IPC](contracts/session-protocol.md),
[verification](verify-usability.md).
