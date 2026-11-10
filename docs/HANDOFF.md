# Waywr handoff — unblock Send, clear input and reduce idle CPU

Branch: `plan/usability-and-installation`. Base: `2d4567a` (conversation telemetry).
Follow [AGENTS.md](../AGENTS.md), [providers](provider-selection.md) and
[controls](tui-controls.md). Local commits only; the user handles remote delivery.

## Current outcome

The user could not send because a saved shared-runtime model differed from the
retired active binding. The footer still directed them through the old Switch
section. Read-only inspection also found an unrelated attention request whose
policy would veto activation globally, although no coordinator was running or
uncertain. These were product usability failures.

Setup-blocked Send now opens the saved coordinator model's activation review
with the draft intact; the main action says Use saved model. Keep current and Esc
return to the composer. Explicit activation also returns there with the message
unsent; after application, Enter sends normally. F3 Profiles → S → A remains
available. Outdated section4 instructions are replaced. Saving still does not
grant context or invoke a model. Activation retains revision/generation checks,
explicit context grant and reserved/uncertain-call settlement fences.

Unrelated request policies no longer veto the project conversation choice.
Requests keep their exact policies and cannot dispatch through an excluded model.
Switch application leaves excluded workflow state untouched, including blocked
attention requests; it cannot resume or amend them. A later coordinator policy
refusal is classified as that request's provider-policy block. Workers retain
their assignments. No model/provider allowlist was added.

Ctrl-C clears the main composer. Escape clears it when no popup/menu is open;
inside a popup/editor it still backs out one level and preserves the underlying
draft. Clearing resets history navigation and the saved draft without erasing
history or cancelling work. Clear controls are included in footer hints.

Idle UI checks now borrow the large development projection instead of repeatedly
cloning catalogs and retained work. Task detail refreshes follow project/task
revision changes, rather than unconditional half-second reads. The readable
history projector checks a small revision watermark instead of parsing a full
historical snapshot just to detect changes. No history is truncated or removed.

## Verification and limits

Behavioral red reproduced inert blocked Send, the unrelated request veto, missing
clear controls and slow idle ticks with1600 models and46 retained units. The
idle-tick workload fell from about660ms to20ms. Focused checks cover direct review
and preserved drafts, both clear keys including history recall, unchanged excluded
requests and dispatch grants, late policy changes, generation races, uncertainty,
workers/questions and independent revision reads. The locked remapped release
build passes. Dependencies/locks are unchanged.

The real isolated paired-binary IPC/TUI journey selects/saves model and thinking,
uses Send to review activation while paused, retains the draft, clears it with
Ctrl-C and Escape, removes the former build and reopens with ready binding,
settings, history and conversation usage intact. No model call or manual rebind
occurred. Idle measurement with the full catalog and populated retained-event
history was0.7% UI /2.7% service. This fixture does not reproduce every aspect of
the operator project or guarantee identical CPU on another machine. Source/bundle
public-content checks pass; documentation and local CI/release definitions include
these repairs. Hosted workflows were not run by the agent.

Conversation usage remains read-only and subject-local; unknown values and
cost/context estimates stay labelled. Autonomous workflow coordinator operations
are not yet included in that conversation summary. Real account login, useful
model-driven delivery and public download/update/package channels remain
unverified/incomplete. Available readiness checks local runtime configuration,
not authentication or model quality.

## Operator action and cleanup scope

The user explicitly requested killing the reported UI/service. Their supplied
processes were verified, terminated, and the exact project systemd unit was
stopped and confirmed inactive. The agent did not restart it, alter its binding,
call a provider, install globally or delete project data/worktrees.

Earlier inspection found36 managed worktrees, one with uncommitted content and
nine with commits absent from the primary checkout. Disposal scope for those
resources remains unresolved; process termination does not authorize discarding
unfinished code. Do not restore snapshots, delete branches/worktrees or perform
remote writes as routine cleanup.

## Next authorized action

The ignored target/local-bin/waywr points to v0.4.0-local.unblocked, manifest hash
below. The user can relaunch and use Send → activate the saved model → Enter to
send the retained message. Operator launch, binding selection, resume and provider
calls remain explicit user actions. Continue from observed ordinary-use friction;
no further broad acceptance run is needed without a relevant change or failure.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[architecture](architecture.md), [distribution](distribution.md),
[user verification](verify-usability.md).

Review-bundle manifest SHA256: `33d4e295b30ed19d8a52f19f30f2e1796816de6e0a342afbce2572cf5cc9b2a5`.
