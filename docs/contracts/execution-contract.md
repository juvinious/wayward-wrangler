# Current execution and recovery contract

This document owns the detailed requirements for Rust host-backed,
`systemd-user-v1` operations. [Architecture](../architecture.md) owns component
boundaries, [state](state-contract.md) owns persisted records and compatibility,
and [IPC](session-protocol.md) owns transport, capabilities and bounded inspection.
This is the maintained process-host contract; completed transition reports are
historical.

State8 connects concurrent child tasks under one approved workflow and serialized
Git integration. It removes mandatory development call/time/repair ceilings while
preserving frozen attempt semantics, bounded output and process/transport/shutdown
health handling. Older serialized profiles retain their original semantics.

State9 [provider roles](../provider-selection.md) reuse these supervised operations.
All current providers use the shared runtime; coordinator/review versus worker
tool capabilities depend on the operation, not a provider name. The old native
profile routes are retired; work/history and frozen evidence are preserved.
Native permit and once-only spawn guards validate frozen grants, with additive
provider amendments preserving older attempts. Preparation refusals carry positive
nonlaunch proof. Shared-runtime configuration failures distinguish a settled local
process from a positively unsent model request; post-send ambiguity remains uncertain. Provider fallback and interruption require positive settlement; uncertain execution
never authorizes a replacement. Review results contain exact commits/findings, not
fabricated commits. Scope and checked integration authority remain application-owned.

[Context delivery](../context-delivery.md) adds typed application-mediated read-only
retrieval to these same attempts. A successful retrieval turn saves an immutable
exchange and continues with a fresh frozen input; it never replays the completed
call. Review continuation advances its attempt revision only after a completed
receipt. Preparation identities include their context/release revision, preserving
older refused/prepared artifacts. Existing grants, role pins, generation fences,
exact-commit review and checked integration remain authoritative. JSON parsing
preserves floating-point round trips so frozen timestamps keep their original
canonical hashes; the canonical encoder and immutable files are unchanged. A
deterministic packet hash mismatch refuses before host-attempt creation and is
surfaced for recovery, rather than being silently retried.

## Asynchronous development and native reports

The live coordinator derives project-specific decomposition and checks from the
human conversation, approved scope, current project context and worker results.
The scheduler never supplies a Example Task App sequence. Completion requires nonempty
successful combined receipts at the exact integration commit and resolved required
obligations; no language-specific command pair is universally required. Execution
checks support Cargo, tracked project scripts and worktree executables. Other
toolchains use a project-owned check script under the same scope/policy.
An explicit `development.reply` records an answer to an exact current coordinator
question, resumes review and leaves scope, policy and requirement authority intact.

State8 starts eligible independent assignments even while a coordinator or combined
check is pending. Already running units keep being polled; only Git integration is
serialized. Context changes trigger compact coordinator decisions rather than idle
narration calls. Positively settled coordinator failures retain receipts and replan
with diagnostic feedback; ambiguous execution or Git effects cannot be replayed.

A worker may return an empty `commit` with its concrete blocked summary/checks.
That is a terminal failure report, never a frozen application submission. Source
acceptance still requires the exact clean branch/commit, ancestry and assigned paths.
Older failed native reports may be added as labelled diagnostic projections without
changing immutable receipts or accepting their claims as code authority.

Current shared-runtime commands execute as the local user; worker worktrees are
not an OS sandbox. The [Linux requirements](../linux-sandbox-setup.md) and
[security policy](../../SECURITY.md) describe that trust boundary. The prior Codex
AppArmor exception is [historical](../reports/retired-native-linux-setup-2026-10-15.md).

Combined-check pending records bind the same scope base, requirement generation
and exact checked commit as their frozen packet. Integration accepts a unit only
when that exact submitted commit was present in the coordinator review packet;
a peer finishing during the call requires a fresh review. The saved effect links
the reviewed commit and immutable packet/result hashes.

New request namespaces use `waywr/<ASCII-title-slug>-<12-hex-request-id>` (slug
at most 48 characters). Their combined branch is `…/integration`; worker branches
are `…/tasks/<task-id>/work`. The suffix derives from immutable work identity,
not a mutable title. Frozen legacy namespaces remain valid. Worker write roots
validate the saved namespace against that identity and the actual checked-out ref.
The current implementation retains existing refs/worktree directories: an active legacy
workflow can gain a readable combined-result alias through a saved compare-and-swap
intent. Divergent/existing unowned aliases are preserved and refused. Restart adopts
only the exact intended ref update; main and application files are untouched.

Legacy frozen retain/literal-main policies remain compatible. New requests use the
[ordinary project contract](../ordinary-project-workflow.md#candidate-and-resource-lifecycle)
for exact candidate ownership, named targets/PR policy and automatic retirement only
after positive ownership/settlement/dependency checks and durable Git retention.
Existing managed-project resources and policies are not retroactively changed.
Optional frozen `generated_outputs` roots use before/terminal hash inventories;
only new ignored regular files whose exact bytes remain unchanged can retire.
Preexisting, unknown, modified, dependency and user-owned content is preserved.
See the ordinary-project contract for the authority and inventory boundary.
Cancellation and terminal receipt publication share a gate: a recorded cancellation
fences late result adoption while preserving original decoded evidence; a saved
validated receipt wins a later cancellation.

## Host and process ownership

Canonical project roots and explicit stopped-service migration are owned by
[project layout](../project-layout.md). Native evidence retains its original paths
through validated compatibility aliases; it is not rewritten to make relocation
appear to be a fresh execution.

Linux with a systemd user manager is the execution prerequisite; the accepted
baseline used systemd 255/cgroup v2. Required properties must be verified rather
than assuming older-manager compatibility. Initialization and offline inspection
do not require the manager. Never enable lingering, restart shared infrastructure
or promise live process survival across logout, user-manager shutdown or reboot.

The project supervisor owns scheduling. Each operation attempt gets a unique
`waywr-op-<project-key32>-<operation-digest16>-<attempt-id32>.service` with generated
hex components. Names are never reused after retirement, collection or reboot.
The narrow Tokio D-Bus adapter requests `StartTransientUnit` with mode `fail`.

| Host property | Contract |
| --- | --- |
| Execution | `Type=exec`, absolute bound runner argv/cwd; record release/executable identity |
| Lifetime | `ExitType=cgroup`, `Restart=no`, `RemainAfterExit=yes`, `CollectMode=inactive` |
| Independence | No `PartOf`/`BindsTo` or stop propagation from supervisor, TUI or Herdr |
| Stop policy | `KillMode=control-group`, 3-second stop grace, `SendSIGKILL=yes`; only verified cleanup or explicit cancellation |
| Start | 10-second request/claim window; possibly delivered timeout is uncertain |
| Environment/output | Explicit adapter environment; no Herdr routing. Runner stdin/stdout/stderr null; bounded application streams and diagnostic file own evidence |
| Failure | `OOMPolicy=continue`; no automatic operation restart; external shutdown remains possible |

Verify user-manager/UID, exact unit properties and executable, `InvocationID`,
cgroup, release manifest and PID/start/boot identity. Neither an inherited ID,
successful start job, missing unit nor missing PID establishes completion/nonlaunch.
The runner is ready only after its verified claim, not merely after `Type=exec`.

## Dispatch and crash consistency

```text
reserved → start-intent → start-maybe-sent → claimed → permitted
         → spawn-committed → running → receipt-validated → result-applied
proven prelaunch refusal → retired → linked new attempt (same logical operation)
unresolved send/execution → uncertain (no replacement)
```

1. Freeze logical operation, packet, authorization and budget references. Persist
   a unique attempt/intent before external send, and possibly-sent evidence before
   enqueue. Ambiguous send never falls back to another launch.
2. Runner locks its attempt, verifies input/identity and claims before touching a
   worker. Supervisor verifies host/claim and publishes a generation-bound permit.
3. Under project execution fence → operation gate (then applicable budget lock),
   recheck active attempt, retirement/refusal, owner, phase, input and permission.
   Reserve invocation durably before spawn commitment. A partial commitment or
   crash-spent reservation cannot be refunded on absence alone.
4. Release locks before spawn/wait; persist dispatch-before-spawn and verified
   child identity. Completed work retains its original identity across owner change.
5. Publish result/validated receipt before applying the transition once, keyed by
   operation/result hash. Receipt-before-checkpoint replay reuses output.

There is no distributed SQLite/file transaction. Use synced atomic publication;
execution/recovery evidence precedes its rebuildable index. SQL intent precedes
external effects. Conflicting identity/hash stops with diagnostics. Never wait on
a writer job holding dispatch gates or hold database transactions across external
I/O. The bounded writer remains the sole database mutation path.

A replacement owner fences the old generation. Unconsumed older permits require
verified same-attempt endorsement; committed calls do not get a new invocation.
Pre-permit runner wait is 30 seconds. Expiry publishes a terminal pre-dispatch
refusal under the gate; a delayed permit/start cannot override retirement/refusal.

## Reconciliation and bounded recovery

| Condition | Required behavior |
| --- | --- |
| Supervisor restart / lost start reply | Reconcile saved intent, exact unit, claim, commitment, child and receipt; monitor the same attempt or report uncertainty |
| Runner death / surviving child | Retain child/cgroup identity; no replacement runner or stream reattachment. Child survival is not recovered output or a receipt |
| Both processes absent | Adopt a valid receipt if present; otherwise uncertain, never assume nonlaunch |
| Cancel/completion race | Serialize intent and publication; an already validated completion wins. Record actual signals/outcome, never manufacture a cancelled receipt |
| Live-runner cancellation | Durable request; verify group identity before TERM and any existing 3-second KILL escalation |
| Dead-runner cancellation | Explicit action may stop only verified attempt cgroup/InvocationID/processes. Preserve observations; without receipt execution remains uncertain |
| Boot change | Old process identities are invalid; retained receipts remain usable. Unknown calls are not recreated |

Automatic replacement is limited to positive proof: a typed never-enqueued refusal,
a fenced never-enqueued stale-owner retirement, or a runner terminal pre-dispatch
refusal. Validate all evidence, absence of conflicting commitment **under that
protocol**, and the durable retirement fence before linking a replacement attempt.
Task, frozen authorization, input and failure history remain unchanged. Generic
D-Bus errors, missing files, timeouts, empty output and heartbeats are not proof.

On older non-development paths, transient proven prelaunch failures allow three
total host-start attempts per episode, with persisted 1-/2-second backoffs and
10-second request/claim windows.
Restart does not reset counters. Invalid configuration, foreign identity or
possibly-sent failure stops. Exit/cancel fences pending recovery. After exhaustion,
show the no-call-launched condition and explicit **Retry infrastructure** only
while proof remains valid; `operation.retry_start` is a deduplicated command action,
not permission to replay a model call. Infrastructure counters cannot reset call
budgets. Per-profile model budgets are distinct from host-start counters. Older state3/4
workflow paths and state5/6/7 conversation profiles retain their own admission and
budget semantics. State8/9 development already bypasses that three-start allowance
and its renewal requirement, retaining positive proof and persisted backoff.
The older conversation-only profile does not itself admit workers or consultation;
current development schedules workers through its separate supervised assignments.

The [current recovery contract](../coordinator-recovery.md) connects typed causes and
positive preparation/receipt/Git reconciliation through the existing scheduler.
It does not weaken the execution proof above. Unit-scoped operational faults preserve
independent dispatch. Explicit finding resolution, progress-based reassessment and
persisted waits use existing decision/receipt identities; unknown faults remain explicit.
A new input does not prove an ambiguous attempt safe to replay.

## Session lifetime and cleanup

`/detach` closes only the UI. `/exit` fences dispatch, checkpoints deterministically
and drains permitted calls; the 30-second foreground deadline may return to the
shell while supervision remains. It adds no kill/retry rule. Preserve frozen
execution deadlines (the generic helper has 600 seconds; the conversation profile
has 120 seconds), existing targeted
cancellation/output-limit policies, and the distinction between known stalled work
(attention-required) and ambiguous execution. Native helper presence is not native
admission. Bare startup preserves intentional pause; `resume` reconciles and uses
generation/exit-ID checks so an older exit cannot stop newly resumed work.

The project service's bounded restart policy is distinct from operation `Restart=no`.
Service Tokio observation/timer tasks have bounded admission and owned shutdown;
ending them does not stop manager-owned workers. Exhaustion exposes retained
diagnostics, not an endless busy state or reset recovery allowance.

After a reconciled terminal outcome, capture final properties and cleanup evidence;
verify empty cgroup before exact-unit stop/reset-failed. Keep unresolved hosts for
explicit inspection. No prefix-wide cleanup; unit metadata is corroborating evidence,
not authoritative history. Keep every referenced release and immutable application
record. No automatic operator migration, retirement or cutover.

Regression gates retain status/control responses below one second and SQLite busy
refusal below 500 ms for the covered cases. A two-second limit applies only to
distinctly documented new manager/view scenarios; it does not relax those gates.
State-aware rollback and legacy execution boundaries remain in [state-contract.md](state-contract.md).

## State8 native development

Native invocation
extends Codex's standard workspace permission profile with read-only common Git
metadata and specific write grants for the worker index/HEAD directory, shared
objects, its own ref namespace and reflog namespace, plus the approved Cargo cache.
Main/integration refs, repository config/hooks and peer indexes stay read-only.
Existing branch names/evidence are preserved; an old failed task is recovered by a
new follow-up assignment rather than rewriting its frozen attempt. Profiles are
per-invocation overrides; shared provider configuration is untouched.

Development reuses host schema2 intent, claims, generation-bound permits, once-only
spawn commitment and immutable receipts. Codex events permit native commands/file
changes for assigned workers; coordinators have tools disabled and only typed
decisions are adopted. No blanket 120/600-second task deadline or fixed call/repair
ceiling applies to this version. Output, shutdown and process health bounds remain.
Explicit capacity refusals with settled execution can schedule persisted waits;
on retry, preserved partial input must match its saved continuation snapshot before
launch. A later unexpected edit blocks relaunch and is retained. This exception
does not relax the clean-input check for ordinary fresh assignments. Ambiguous
outcomes never justify replay or provider fallback. Dirty worktrees and
Git effects without conclusive resulting evidence are preserved for reconciliation.

State8 operational resume preserves the exact scope/policy and workflow revision.
An idle integration tree dirty only with untracked `__pycache__/*.cpython-*.pyc`
files beside tracked Python source can be recovered through `development.resume`.
Under the existing supervision and Git fences, it writes an immutable manifest,
retains exact bytes/hashes in the operation directory, then removes only those
cache paths from the integration tree. A repeated/crashed command reconciles the
same manifest; source edits, unknown files, symlinks, pending execution or saved
questions refuse recovery. The history/Inspect record links retained evidence.
Tracked-script Python checks accept `-B`; checks suppress bytecode generation for
child interpreters, and native development clients receive the same environment
setting. Shell `-c` and arbitrary interpreter-option grants remain refused.

Request-specific role pins are stricter than broad provider grants. Assignment,
reservation and the runner permit require the fixed profile/hash; task overrides or
fallback cannot bypass it. A fixed reviewer policy requires successful review of each
exact submitted implementation commit, with `requires_changes:false`, before integration
or delivery. Requests without such a pin retain their bound prior behavior.

New ordinary-project requests freeze named baseline/target/completion/validation
and retention choices. The [workflow contract](../ordinary-project-workflow.md) owns
candidate checkout ownership, conservative automatic resource retirement, archival
Git refs and exact local/PR completion; legacy frozen retain/main policies remain
compatible. The retired native Ollama boundary is documented in the
[historical provider contract](../archive/provider-selection/2026-11-09-retired-native-provider-contract.md#ollama-transport-boundary).
