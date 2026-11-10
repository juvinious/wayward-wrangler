# Waywr handoff — renewable sessions and busy-owner attachment

Branch: `plan/usability-and-installation`. Base: `4c804cd`. Follow
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
from ordinary startup renewal. Final review/check results belong with this local
checkpoint. No hosted workflows, publication or remote Git writes were performed.

Renewal requires verifiable process ownership and writable state/archive paths;
it cannot bypass permission failures or ambiguous project/layout ownership.
Cross-filesystem custom-state retirement is not implemented. Real authentication,
model quality, useful delivery and the complete ordinary loop remain unverified.

## Operator state and next authorized action

No operator service, bindings, state, project files or credentials were changed
by this implementation. The previous uncertain attempt was not replayed. The
ignored target/local-bin/waywr still points to the previous local.startup bundle;
do not present it as the new candidate or an instruction to test now.

Continue with direct foreground interrupt and prompt leave, including stopping
or disconnected state; then real steering/follow-up and consistent editor/error
feedback. Runtime abort exists, but steer/followUp are not connected through its
current single-input boundary. Current confirmations/exit semantics still need
replacement. A waiting operation must not trap navigation, drafting or leaving.
Only after the audit sequence, prepare the paired local candidate and arrange
the useful operator walkthrough. No installation/account use is inferred from
fixture success. Source/worktree disposal is not authorized.

Owners: [providers](provider-selection.md), [architecture](architecture.md),
[interaction target](tui-controls.md#planned-everyday-interaction),
[product plan](usability-plan.md), [user verification](verify-usability.md).
