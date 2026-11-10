# Waywr handoff — renewable sessions and direct controls

Branch: `plan/usability-and-installation`. Base: `70a252b` (startup renewal); current changes add direct controls. Follow
[AGENTS.md](../AGENTS.md). Local implementation/commits only; the user handles
remote delivery. Keep the operator walkthrough deferred until the accepted
[interaction/lifecycle sequence](usability-plan.md#finish-before-the-next-operator-test)
is implemented. Do not ask the user to reopen after each small repair.

## Current objective and implemented outcome

The user explicitly authorized discarding obsolete Waywr configuration/session
context. Preserving a broken old session as a live startup prerequisite is no
longer the product contract. Source, Git history and dirty worktrees remain.

Normal startup now renews obsolete native settings/bindings, unsupported session
formats or stranded host uncertainty. rust_session.rs verifies identity, fences
new dispatch, stops the exact owned service/operation cgroups, retires state and
old target/endpoint/release configuration, and creates fresh current-contract
state. Valid shared model choices survive; retired native choices use current
catalog defaults. Fresh context uses current source/new instructions. Calls,
queues, approvals, budgets and execution bindings are not imported. Old outcomes
stay unknown; abandonment never fabricates completion or grants replay.
The private archive/journal contract lives in [project layout](project-layout.md).
The journal resumes interrupted retirement and healthy subsequent reopen retains
its new session. Archives are diagnostic material, not runnable projects.

Healthy compatible busy owners remain attachable through the current UI; their
release identity is used for IPC while execution stays with its existing owner.
Idle handover still verifies exact process/socket/project ownership. The service
accepts a compatible same-UID UI using its actual peer executable/manifest hash
and interaction protocol, rather than requiring the original build path. Service
versions predating this attachment capability still need their first idle
transition; no legacy native-model route was reintroduced.
Stop/cancel/command lookup target the exact current IPC owner and never initiate
session renewal or another model attempt. Their peer command/project/state are
verified even when the invoking application is from another build.

## Direct controls now implemented

Empty plain-composer Escape/Ctrl-C requests interruption of only the foreground
subject's exact coordinator turn. Nonempty input still clears; popup Escape backs
out one level. The visible primary action and /interrupt use that same command,
without workflow cancellation or a confirmation panel. Repeated requests show
Stopping; drafts/navigation/leave remain usable until actual settlement. A lost
connection does not pretend interruption was delivered.

Ctrl-Q leaves directly from all editors/overlays; empty-composer Ctrl-D and
/detach, /exit, /quit also leave immediately. The local draft is saved without
waiting for model/tool settlement or network acknowledgement. /pause separately
requests stop_session and keeps the UI open; it does not kill the foreground call
or imply that it has settled. Existing /resume semantics remain explicit.

## Verification and limits

The real isolated launcher/systemd/IPC/TUI regression first reproduced the user's
exact unresolved-attempt update refusal. It now stops a held fixture operation,
opens a fresh session with retained model choices/source/worktrees, discards the
obsolete target, reopens without resetting again, and replaces retired native
settings at startup. The separate busy fixture reconnects without changing the
owner, then hands over after the operation ends. The held operation is sleep,
not a model. Crash continuation, foreign-state refusal, peer executable/contract
verification and new shared-profile initialization have focused unit coverage.
The old init assertion still expected native schema2; it now checks the actual
schema3 contract. No dependency/lock changes or account calls.

The remapped release and source public-content checks pass. Portable real-systemd
handover with retained history/pending delivery/unresolved launch records passes.
The explicit retired-profile IPC fixture remains a UI inspection check, separate
from ordinary startup renewal, and its combined catalog/thinking/activation/usage/
draft/pause regression passed at the startup checkpoint (idle UI 1%, service 3%).

The --controls variant of test-session-renewal.py uses the actual SDK against one
held local HTTP stream and the real service/runner/IPC/TUI. It SIGSTOP-holds the
runner so interruption cannot settle, requests interrupt through the UI, keeps an
unsent draft, leaves in under one second, and reattaches to the stopping owner.
Resuming the runner produces its receipt; the UI then explicitly pauses without
leaving and leaves a disconnected service promptly. Only one HTTP fixture request
is made. All uniquely keyed fixture units are stopped; no operator state/account
or actual inference is used. Focused control/topic/late-action checks pass. No hosted workflows, publication or remote Git writes were performed.

Renewal requires verifiable process ownership and writable state/archive paths;
it cannot bypass permission failures or ambiguous project/layout ownership.
Cross-filesystem custom-state retirement is not implemented. Real authentication,
model quality, useful delivery and the complete ordinary loop remain unverified.

## Operator state and next authorized action

No operator service, bindings, state, project files or credentials were changed
by this implementation. The previous uncertain attempt was not replayed. The
ignored target/local-bin/waywr still points to the previous local.startup bundle;
do not present it as the new candidate or an instruction to test now.

Next is real steering/follow-up, then consistent multiline/history, direct
model/thinking/status actions and error feedback. Runtime abort is now connected
to direct UI interruption; steer/followUp are still not connected through the
runtime's single-input boundary. Do not relabel its later-turn queue as steering. A waiting operation must not trap navigation, drafting or leaving.
Only after the audit sequence, prepare the paired local candidate and arrange
the useful operator walkthrough. No installation/account use is inferred from
fixture success. Source/worktree disposal is not authorized.

Owners: [providers](provider-selection.md), [architecture](architecture.md),
[interaction target](tui-controls.md#planned-everyday-interaction),
[product plan](usability-plan.md), [user verification](verify-usability.md).
