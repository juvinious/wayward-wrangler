# Waywr handoff — shared coordinator child working directory

Branch: `plan/usability-and-installation`. Base: `f97b183`; prior application was
`ba229e8`. Follow [AGENTS.md](../AGENTS.md). Local commits only; remote delivery
belongs to the user. This is a startup repair interrupting the accepted
[pre-test interaction sequence](usability-plan.md#finish-before-the-next-operator-test),
not completion of that sequence or authorization for another model trial.

## Observed failure and implemented repair

The user reported Host execution uncertain with Spawn outcome uncertain after
dispatch reservation: No such file or directory. Read-only inspection of the
actual conversation attempt found its Node executable and runtime entry present,
no child record/receipt and no client-scratch directory. The shared runner used
that directory as cwd but never created it. The old native helper had created it;
worker fixtures used an existing checkout and missed the new coordinator path.

The runner now creates required non-worker child working directories before
child dispatch/spawn. This covers conversation/workflow coordinators and local
checks without depending on a retired native helper. Workers keep their assigned
checkout. If directory preparation fails, the runner records a settled local
configuration failure with positive no-provider-process proof, then publishes
its failed outcome. It never writes a dispatch, child or uncertain record for
that preparation failure. This does not weaken uncertainty checks for a process
that may actually have started or rewrite an older ambiguous attempt.

## Verification and limits

The local directory-preparation regression initially reproduced an uncertain
spawn error when the cwd name was a regular file. It now reports a settled
failure with no child/dispatch. Existing provider preflight/nonlaunch checks pass.
The corrected isolated host fixture includes the reserved conversation binding
and project context required by the real host gate. Against the old paired
runner it reproduced the user's exact ENOENT uncertainty. Against the repaired
runner, conversation and workflow coordinators complete typed replies through
both local OpenAI-compatible and Anthropic-compatible HTTP fixtures. Existing
worker tool/file/usage checks also pass. Coordinators receive only submit_result,
not worker file tools. Fixture processes/SQLite/Git worktrees are temporary;
the host permit fixture is not proof of real systemd orchestration or account use.

The locked remapped release builds and source/review-bundle public-content checks
pass. Local CI/release checks include both hosted coordinator cases and the local
preparation refusal. Dependencies and locks are unchanged. No broad TUI/lifecycle
rerun was needed for this child-startup change. Hosted workflows were not run.
Real authentication, model quality and useful task delivery remain unverified.

## Existing operator attempt is unresolved

The reported attempt was inspected read-only. Its existing uncertain.json and
spawn-committed evidence were not removed, overwritten, reclassified or replayed.
The agent did not stop/restart the operator service, change its binding or submit
another model call. The previous user-authorized process stop is historical; the
new report shows the user subsequently attempted another call. Do not assume the
operator service remains stopped or that this new build clears its parked call.

Existing generic spawn errors do not provide the new typed preparation proof.
Reconcile the specific old attempt explicitly before any operator retry; missing
processes alone are not nonlaunch proof. Keep the operator walkthrough deferred
as requested. Old worktree disposal remains a separate unresolved scope.

## Remaining planned work and next authorized action

The [progress table](usability-plan.md#reconciled-checkpoint--2026-11-10) distinguishes
implemented model/build/selection/usage mechanisms from incomplete daily use.
Continue with direct foreground interrupt and prompt leave, including stopped or
disconnected state; then real steering/follow-up, return to a busy owner after an
update, and consistent editor/error/model feedback. Runtime abort already exists;
steer/followUp are not connected through the current one-input runtime boundary.
Current UI confirmations and old exit semantics still need the planned changes.
A waiting operation must not trap the interface or block leaving. Preserve exact
execution ownership and prevent replay rather than discarding unresolved evidence.

The ignored target/local-bin/waywr selects v0.4.0-local.startup, manifest below.
It is a prepared local bundle, not an instruction to reopen/test now. Do not start
an account call, reset operator state, publish or push without corresponding
explicit authorization. No operator action was taken in this repair.

Owners: [providers](provider-selection.md), [architecture](architecture.md),
[interaction target](tui-controls.md#planned-everyday-interaction),
[product plan](usability-plan.md), [user verification](verify-usability.md).

Review-bundle manifest SHA256: `3c4c938e39a5a3f672481450f892752574b1ee7ddd7f08b2cec6766f6c3e1890`.
