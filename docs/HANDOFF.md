# Waywr handoff — reconcile progress and finish the everyday interaction loop

Branch: `plan/usability-and-installation`. Application checkpoint: `ba229e8`.
This checkpoint changes documentation only. The user asked what the plan has
actually delivered, wants remaining usability changes finished before reopening,
and asked to align normal input/interrupt/leaving behavior with Codex/Pi.
Follow [AGENTS.md](../AGENTS.md). Local commits only; no remote delivery.

## Delivered versus complete

The [reconciled progress table](usability-plan.md#reconciled-checkpoint--2026-11-10)
is the current status. Implemented locally: shared standalone provider/tool
libraries replacing native integrations; complete searchable provider/model and
thinking selection; saved/active labels; direct setup from Send; conversation
usage and worker activity; paired build/developer install/local launcher; reopen
and idle/portable handover; whole populated snapshots; clear-buffer controls;
scoped policy repair and reduced idle copying/polling.

These mechanisms do not complete daily usability. Direct interruption/leaving,
actual SDK steering versus follow-up, busy-owner reattachment after update and
one useful shared-runtime task remain incomplete. Workflow coordinator usage is
not yet in the conversation summary. Public unauthenticated installation,
update/uninstall, artifact collection and subsequent packaging remain deliverables.
Real account/model usefulness and the substantial web-app milestone are unproven.

The previous isolated release journey covered saved model/level activation from
blocked Send, preserved/cleared drafts, populated history/usage and reopening.
Its idle measurement was0.7% UI /2.7% service; the large-snapshot idle-tick workload
fell from about660ms to20ms. These are bounded fixture measurements, not proof of
performance or useful delivery on every operator project. Do not repeat broad
completed checks merely to claim progress.

## Comparison and revised target

The [planned everyday interaction](tui-controls.md#planned-everyday-interaction)
records current official Codex documentation and Pi documentation/source checked
on2026-11-10. Both distinguish guidance during active work from later follow-ups.
Pi defines separate interrupt/clear/exit actions; Codex documents contextual
shortcuts, command discovery, model/status controls and saved-session resume.
Adopt useful interaction principles while retaining the user's explicit
Escape/Ctrl-C clear-buffer preference and Waywr's project-service lifecycle.

Local inspection confirms the pinned agent library supplies steer, followUp and
abort. The Waywr runtime currently reads one input object before running, then
only supports abort via signals. Enter currently creates later turns; relabelling
that queue is not true active-agent steering. Existing UI interrupt/detach/exit
routes use confirmations or acknowledgement waits. Busy-owner update still
refuses attachment. These are concrete gaps, not completed capabilities.

A task can wait without locking the application. Keep drafting, reading, topic
selection, inspection and leaving available. Stop/settlement state belongs to the
affected operation; it does not justify trapping the terminal. Preserve exact
execution ownership, approved work and no-replay boundaries without forcing
unrelated conversations through old recovery/settings menus.

## Next authorized implementation

Finish the [pre-test sequence](usability-plan.md#finish-before-the-next-operator-test)
one visible journey at a time:

1. Direct foreground-coordinator interrupt and prompt leave, including stopping
   or disconnected state, without whole-workflow cancellation or confirmation.
2. Enter steering versus Alt+Enter follow-up, with truthful delivery, editable
   queued input and an actual service/runner/runtime connection to the SDK.
3. Inspect/control the busy owner after selecting a rebuilt application; hand
   over only once idle, without force-stop, replay or manual project rebinding.
4. Complete consistent editing/commands/error/model feedback. Then do one
   authorized start → redirect → leave → return → rebuild → continue walkthrough.

The target changes exit semantics: /exit and /quit leave like /detach, with pause
as a separate action. Current code still uses its old semantics. Reconcile their
owners when implemented; do not imply the planned controls already work. First
slice acceptance and exclusions are recorded in the plan. Use targeted isolated
checks during development; the user wants one later walkthrough after this work.

## Operator and authority state

The previous user explicitly authorized stopping their reported UI/service; the
exact processes were stopped and the project unit confirmed inactive. This review
did not start the application, install globally, change an operator binding,
read credentials, call a provider, delete project work or perform a remote write.
Earlier worktree disposal scope is unresolved and independent of process stopping.
Preserve unfinished work unless disposal is explicitly authorized.

The ignored target/local-bin/waywr still selects v0.4.0-local.unblocked, manifest
SHA256 `33d4e295b30ed19d8a52f19f30f2e1796816de6e0a342afbce2572cf5cc9b2a5`.
No build or runtime tests are needed for this documentation-only checkpoint.

Owners: [product plan](usability-plan.md), [controls](tui-controls.md),
[providers](provider-selection.md), [distribution](distribution.md),
[verification](verify-usability.md), [development approach](development-approach.md).
