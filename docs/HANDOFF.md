# Waywr handoff — coordinator conversation activity and usage

Branch: `plan/usability-and-installation`. Base: `0572e44` (Profiles activation).
Follow [AGENTS.md](../AGENTS.md), [providers](provider-selection.md) and
[protocol](contracts/session-protocol.md). Local commits only; the user handles
remote delivery. No operator service/state/settings, installation or account was
changed. Only the previously authorized ignored local launcher is refreshed.

## Current outcome

The main conversation shows its current coordinator conversation call, or latest
call with telemetry for the selected subject. It names that call's selected
provider/model and thinking level and displays cumulative input/output/cache
counts, cost estimates and estimated context headroom at the last response.
It keeps unknown values explicit, separates usage from saved model preferences,
labels disconnected or stale activity, and retains completed-call data on reopen.
A newly reserved call never borrows the previous call's usage. Other subjects'
telemetry is excluded. Rendering at80×24 and120×32 keeps the summary visible in
the normal conversation viewport; existing scrolling reads earlier conversation.

Runtime started events now retain selected identity/level in attempt progress.
The backend projects bounded display fields from the published attempt, checking
project/operation/attempt identity and containment. Each metadata read is capped
at64KiB. Missing, malformed, oversized or mismatched telemetry becomes unavailable
rather than preventing opening. Raw packets, prompts, tool output, credentials and
extra runtime fields are not projected. Recent snapshot turns include subject and
runtime metadata. A small read-only conversation.activity refresh polls an active
selected-subject call once per second without reloading catalog/history; file
progress alone creates no conversation event. Late replies cannot revive a
completed call or replace a newer observation. No scheduling, budget, policy,
settlement or execution authority changes. No model/provider allowlist was added.

F3 Profiles still offers S save → A review activation. Keep current is the default;
explicit Activate and share project context requests the existing after-turn
switch. Unsaved/stale targets refuse. Activation preserves pause, workers and
request policies; it does not sign in or amend policies. Saved versus different
active model/level labels remain distinct in the main header.

## Verification and limits

The initial behavioral checks reproduced absent snapshot metadata and absent
conversation rendering. Focused checks now cover progress → snapshot, retained
result → reopen, unknown turns, private-field filtering, bounded invalid data,
host-attempt mismatch, subject separation, unknown values, fresh/stale updates,
late replies and terminal viewport rendering. The locked remapped release builds.
The real runner/SDK loop retains selection metadata and usage through two local
HTTP protocol fixtures. A real paired-binary IPC/TUI check displays synthetic
completed-call telemetry, rejects a foreign turn, saves/activates model/thinking
while paused, removes the old application build, and reopens with usage, binding,
draft, settings and populated history intact. It makes no model calls or manual
rebinds. Fixtures prove these boundaries, not actual account/model reliability.

Public-content checks cover source and the review bundle. Documentation owners
and local CI/release definitions include this journey. Dependencies/locks are
unchanged. Hosted workflows were not run by the agent. Autonomous workflow
coordinator operations are not yet included in the conversation summary; workers
retain their existing activity path. There is no account/subscription quota or
billed-cost claim. Real login, model quality, useful task delivery and public
download/update/package channels remain unverified/incomplete.

## Pending cleanup decision

The user proposed discarding old development-era state. Earlier read-only
inspection found36 managed worktrees, one with a tracked edit and seven untracked
entries, and nine with commits absent from the primary checkout. That inspection
found a clean primary checkout and schema3 provider configuration. The cleanup
scope question remains unanswered. No worktree, branch, history or state was
deleted. Permission to continue is not disposal authority.

## Next authorized action

The ignored target/local-bin/waywr entry uses v0.4.0-local.usage; its manifest
hash is recorded below. Detach and relaunch to use it. Operator binding changes,
resume and account calls remain explicit user actions. Continue with one concrete
ordinary-use/lifecycle interruption at a time; busy-owner attachment across an
application update remains a lifecycle gap. Preserve its existing execution
owner rather than force-stop/replay. Resolve cleanup scope before destructive
work. No push, operator service changes or provider calls are authorized here.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[distribution](distribution.md), [architecture](architecture.md),
[user verification](verify-usability.md).

Review-bundle manifest SHA256: `5908ca24383c14cfc7cd8094d9f036999f4f73b9b148958d40724035e5a693ca`.
