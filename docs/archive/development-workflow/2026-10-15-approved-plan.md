> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Plan: deliver Git-backed parallel development through Waywr

Planning revision, 2026-10-15, on `plan/complete-development-workflow`, following
`7100fb2`, including persistent request history in the same delivery scope.
Source base remains merged main `1813a2f` (PR #7), tree-identical to
cleanup checkpoint `0a1f972`. This is the single current proposal, not implementation
or runtime authorization. The [initial proposal](archive/development-workflow/2026-10-15-initial-plan.md)
is superseded, including its single-worker assumption and mandatory resource limits.

## Product outcome and normal use

Deliver the existing production Rust TUI as a usable development orchestrator:

```fish
cd ~/projects/example-task-app
waywr init .
waywr
```

One stable installation supplies the launcher, paired Rust service/runner and
resources. Reuse bundle/install and automatic service attachment; users do not
assemble services, set fixture variables or navigate evaluation installations.
Internal release/schema compatibility still protects saved state. Herdr remains
optional read-only inspection, never an execution dependency.

Discover/configure providers, select coordinator/workers and supported effort,
converse naturally, refine requirements, then give **one understandable scope and
operating-policy approval**. Ordinary conversation remains available while workers
run. Sending discussion is not permission to edit; the work approval covers routine
edits, builds, tests, formatting, documentation, local commits and clean integration.
Ask for substantive decisions, unresolved conflicts or meaningful scope/authority
expansion. Remote publication and unrelated-project changes remain separate.
The policy names project paths, permitted commands, provider/fallback choices,
dependency-network access and main-integration behavior; resource caps are optional.

**Example Task App is the first real job**, retained at `~/projects/example-task-app`, not a
disposable fixture. Its application code, including scaffolding, must be produced
by workers launched through Waywr. The agent implementing Waywr does not build it
directly. No target is created during this planning revision.

Keep its scope small: an offline Rust CLI for `add`, `list`, `done`, `reopen`, tags/
filtering and Markdown export, with a selected local JSON store. Stable IDs, Unicode,
restart persistence, atomic writes and useful invalid-input/corrupt-store errors
without data loss are required. No server, account, sync or runtime network.

Completion means normal startup works, a real coordinator delegates to real workers,
reviewed changes are integrated, Example Task App builds and performs those functions,
and compact project knowledge plus readable request history are saved for future
work. Evidence includes the final commit/diff, actual `cargo test --locked`,
`cargo build --locked --release`, binary
smoke checks on isolated data, and usable Fish-compatible README instructions.
A commit, report, mock suite or proposed acceptance phase is not completion.

## Current gap and reusable mechanisms

| Existing mechanism | Connected implementation work |
| --- | --- |
| Ratatui TUI, continuous conversation, scope revisions/decisions | Provider selection, one work-policy approval and parallel task/obligation visibility |
| `session_tasks`, admissions, workflow records and bounded SQLite writer | Keep one approved workflow active per project; add concurrent child work units |
| Systemd host intents/claims/permits, runner receipts and once-only adoption | Route native coordinator decisions into real editing/checking operations |
| Durable commands/events, snapshots and generation fencing | Dependency inboxes, steering, Git effect reconciliation, reconnect and readable request history |
| Selected context, hashes, findings and exact artifact checks | Incremental source knowledge and review/acceptance tied to exact Git revisions |

[session_store](../backend/src/session_store.rs) has `session_one_workflow` and
`active()` selects one parent. **Keep that project-workflow invariant**; one workflow
can schedule several child workers. Do not reinterpret every worker as a separate
project workflow or build a general multi-workflow scheduler.
[session_service](../backend/src/session_service.rs) currently bypasses workflow
dispatch when native conversation is enabled. Connect conversational coordinator
actions to child scheduling in the same service, keeping chat responsive.
[adapters](../backend/src/adapters.rs) currently forbid file/command events and
[runner](../backend/src/runner.rs) has hard-coded 120/600-second native deadlines.
Change those assumptions for development; these are implementation tasks, not
prerequisite investigations, native-admission trials or approval rounds.

Reuse [continuous](../backend/src/continuous.rs), [task_scope](../backend/src/task_scope.rs),
[host_attempt](../backend/src/host_attempt.rs), [workflow_store](../backend/src/workflow_store.rs)
and [local_context](../backend/src/local_context.rs). Replace fixed drafting/four-file
stages in the new path while retaining useful validation, review and recovery code.
Old versioned records keep their meaning; no silent conversion or evidence rollback.

## Git ownership and minimal durable extensions

**Git owns code history. Existing Waywr records own scheduling, authorization,
decisions, execution and acceptance.** Link them by exact commits; no second code
history, task service or separate orchestration database.

For each approved body of work, the project service creates
`waywr/<work-id>/integration` and a dedicated integration worktree. Each editing
task gets `waywr/<work-id>/<task-id>` and its own worktree, initially from an exact
accepted base. Use ignored project-local `.wayward-wrangler/worktrees/` directories;
canonical state stays in the original project, outside worker trees. Child worktrees
do not run `waywr init` or acquire independent project identities/services.
For a new non-Git target, work approval permits the service to initialize Git and
create an empty baseline commit, without application source, before creating task
worktrees. The first source/scaffold commit comes from a Waywr-launched worker.

The coordinator chooses eligible results; the service's Git executor is the sole
serialized integration authority. Workers own edits and commits on assigned branches,
not the integration/main refs or another worker's files. Service-owned Git operations
create and synchronize worktrees. Worktrees separate editing, **not security**:
retain project-scoped client/host permissions, protected authority records, path/
symlink checks and the documented trusted-client/same-UID limitation.

Preserve dirty/untracked work and external user edits. Base tasks on accepted commits;
if relevant uncommitted input must be included, capture an explicit input snapshot
without resetting, overwriting or silently stashing the user's tree. Never remove
dirty, unresolved or uniquely committed worktrees. Retain task refs/evidence through
acceptance; later cleanup follows explicit project retention policy.

Use small versioned additions in the same SQLite writer, keyed to existing workflow,
task and operation IDs:

| Record extension | Required contents |
| --- | --- |
| Existing parent scope/admission | Requirements/policy revisions, integration branch/worktree, target-main policy and base |
| Child work unit | Requirement revision; assigned agent/profile/model/effort; base/result commits; branch/worktree; relevant context/contracts; deliverables/checks; state and remaining obligations |
| Dependency/obligation | Producer/consumer, start versus integration/acceptance condition, contract revision, accepted commit; placeholder owner/location/replacement and resolution evidence |
| Notification | Stable identity, recipient, dependency revision/commit, concise changes and acknowledged/applied state |
| Integration effect | Unique intent, expected branch head, submitted commits, requirement/contract revisions, operation ownership, actual resulting commit and check/acceptance evidence |
| Request identity/history projection | Stable project request ID linking discussion, workflow, child tasks and attempts; timestamps/attribution, source event references, projection cursor and compact summary provenance |

Extend the existing IPC capability/version negotiation and `command`, `task.get`,
`snapshot`, `events_read` and output/artifact paging. Add child records and actions
for work approval, steering, safe synchronization and conflict decisions; reuse
command IDs/exact bodies, stale-revision checks and event cursors. No new transport.
Worker submission and coordinator proposals remain validated inputs to the writer,
not authority created by model prose. Version state/protocol/bundle support together;
do not turn internal compatibility handling into user-facing evaluation profiles.

## Task, dependency and integration lifecycles

Work units move through **planned → ready → running → submitted → reviewed →
integrated → accepted**, with explicit blocked, capacity-waiting, superseded,
cancelled or uncertain states as needed. Submission records commits, evidence and
remaining obligations. The assigned portion may be finished while feature acceptance
is pending; release the worker instead of keeping an agent alive waiting for a peer.
Failures/feedback can create a revised attempt under the same approved scope.

Before useful parallel work, establish only the shared interface needed: signatures,
data structures, ownership, error behavior and observable semantics. Store its
revision and source commit; use actual headers/types/examples where useful. A small
interface decision inside planning or a worker task suffices; no architecture ceremony.

Dependencies specify **start conditions separately from integration/acceptance
conditions**. A consumer can start against an agreed contract while its producer
implementation is unfinished. Changes that are independently complete may integrate
earlier; a dependent feature cannot be accepted until its required real implementations
and combined checks are satisfied. Detect dependency cycles and split/revise work
instead of leaving an unexplained waiting graph.

Allow tracked stubs/mocks/placeholders for useful progress. For each significant
substitution, record purpose/location, contract, replacement dependency, owner or
follow-up obligation and verification needed to prove real behavior. Prefer separate
test fixtures/adapters to competing edits of production code. Tasks report substitutions;
review checks changed code, wiring and tests against the obligation register. Resolve
an obligation with exact replacement/integration commits plus behavioral evidence,
never merely a TODO search or a worker's completion claim. Required production behavior
still backed by a placeholder keeps the feature/workflow incomplete.

When a dependency is accepted for consumption, the writer durably records its exact
accepted commit, contract version and concise changes, plus a notification for each
affected task. “Available” means reviewed usable dependency evidence, not a new branch
head alone. Delivery is retryable; a unique recipient/dependency-revision/commit key
and saved acknowledgement prevent repeated delivery from creating duplicate work.

**Synchronization owner: the service Git executor.** An active worker receives a
pending-update notice through its supported control channel, or at its next turn;
its running packet is not rewritten. It finishes/yields a checkpoint, commits its
work and ends the attempt. Only after verified worker/child quiescence and a clean
task worktree does the executor merge the accepted dependency commit, record the
new base and launch the next task revision. No merge/rebase under active editing;
if yielding is unavailable, retain the update until completion or coordinated stop.
Dirty work is retained and reconciled, never cleaned automatically. If the consumer
already submitted, assign a follow-up integration task to any suitable available
agent rather than resurrecting the original agent just to wait.

Integration lifecycle: **eligible → intent saved → Git effect → combined checks →
acceptance recorded**, or conflict/failed-check/uncertain with retained evidence.
Serialize this per workflow and generation-fence ownership. Validate submitted commit
identity/ancestry, allowed diff, current requirements/contracts, dependencies, review
evidence and open obligations. Merge frozen submitted SHAs, never a moving task-branch
tip, into the integration branch; normal merges retain task commits, and integration
commits carry the intent identity and parents for reconciliation. A new task commit
requires a new submission revision. A conflict-free merge does not prove functional
compatibility. Run relevant combined
checks and retain the exact checked revision. Partial integration can be recorded
without falsely accepting the complete feature.

Before Git mutation, save the expected integration head and inputs. Afterward record
the resulting commit and check evidence. On restart compare the intent with branch
head, commit ancestry/operation metadata and worktree/index/MERGE_HEAD state: an
already-applied result is recorded once, a proven-not-started effect may proceed,
and a partial/conflicting/unexplained state waits for reconciliation. Never blindly
repeat a merge or model invocation. External ref changes invalidate stale intents;
missing processes alone never prove nonlaunch.
Recheck authority and requirements before acceptance/main advancement. If steering
wins during a merge/check, retain the resulting commit for adaptation but do not
accept or promote the obsolete feature.

Default recommendation: after the approved body passes combined acceptance, advance
clean local main through a checked fast-forward under the original operating policy.
Keep the user's primary worktree intact if dirty/diverged; hold delivery for a
substantive reconciliation decision. Other configured policies may integrate an
independently complete change directly, through the same serialized authority and
checks. Remote merge/push/publication is outside this permission.

## Steering, resources and durable context

Concurrency is configurable and selected from task independence, conflicting writes,
provider capacity and local CPU/RAM/VRAM. Permit useful independent editors; serialize
shared-file changes and integration where simpler. Use separate build-output paths
when parallel builds would interfere. Reserve service responsiveness for conversation,
steering and decisions instead of allowing worker activity to monopolize the service.

Authorization does not expire with time or a call count. **No mandatory elapsed-time
limit, invocation ceiling, fixed repair count or blanket per-call deadline.** Optional
user-configured spending/resource caps are allowed, not prerequisites. Track observed
usage and distinguish unknown cost/capacity from measured values; do not invent precise
remaining provider capacity or treat tokens as a consumption target.

Persist approach/input/result fingerprints, failure signatures and useful progress
(accepted changes, satisfied checks or resolved decisions). Detect equivalent repeated
calls, unchanged failing approaches, circular revisions, duplicate assignments and
consumption without progress. The coordinator adapts/narrows the approach, cancels
redundancy or asks a substantive decision; counters inform that diagnosis rather than
becoming arbitrary repair allowances. Long-running compilation/reasoning is not stalled
merely because it is quiet. Use transport/process observations and task-specific
progress evidence; keep health timeouts, output bounds and graceful shutdown handling
distinct from task deadlines.

On explicit rate/capacity refusal, preserve commits/partial work and record the wait
reason and retry-after/backoff. Schedule a persisted, bounded-frequency wakeup; do not
busy-loop or keep an agent running to wait. Resume useful work when capacity returns
after reconciling the prior attempt. Switch providers only within the configured
fallback policy, visibly recording it; otherwise wait or ask. Ambiguous execution is
still uncertain, not a rate-limit excuse for replay.

Steering records a requirement/interface delta and identifies affected tasks/results
transitively. Preserve unaffected work, revise pending assignments, fence obsolete
integration and coordinate updates/interruption of active attempts. Retain useful
completed commits for adaptation. In-scope refinements follow the approved policy;
only material boundary expansion requires new approval. Questions block dependent
work, not all unrelated tasks. Saved answers drive scheduling without another ritual.

Maintain compact application-owned project knowledge: source/architecture map,
conventions, requirements, decisions, accepted commits/checks, open obligations and
next steps. Extend context hashes/provenance and refresh affected entries after
integration, external edits, steering and resume. Give workers relevant slices, not
whole transcripts or repeated full scans. Reuse requires valid requirements, source
inputs, dependency/contract revisions and relevant toolchain/check evidence; similarity
only finds candidates. Knowledge remains project-scoped unless cross-project access
is explicitly requested. Model summaries are derived context, never authority.

### Persistent request history

Each project request has one continuously persisted, human-readable history across
task revisions, parallel workers, attempts, steering, detach/resume and coordinator
replacement. Assign its stable identity when the request/discussion is saved, before
work approval; link later workflow/task records to it. This is distinct from a child
task, provider session, IPC request ID or human-question ID. Renaming the work or
replacing an agent does not start a new log.

Use the existing canonical state directory, by default
`.wayward-wrangler/state/history/2026-10-15T143000Z-example-task-app-storage-<request-id>.log`.
Honor an explicitly selected state directory; introduce no `.waywr` tree or second
logging database. Persist the initial readable name, UTC timestamp and stable ID in
the filename. Git remains code history; this log is a readable projection of existing
application records, never approval, execution or acceptance authority.

Keep the live log chronological, with labelled, timestamped entries identifying
source event, actor, request, task/revision and attempt when applicable. Cover:

- **PROJECT TASK NAME:** request ID, initial goal, project, starting Git revision and
  timestamps, explicitly identifying a new repository with no starting commit.
- **CONVERSATION:** user/coordinator messages, clarifications, decisions, scope/policy
  approval and subsequent steering, including corrections and superseding decisions.
- **TASKS:** each assignment/revision and purpose, dependencies, agent/provider/model/
  effort, branch/worktree/base, expected deliverables and interface/placeholder
  obligations; reassignments/cancellations and their reasons.
- **OUTCOMES:** actual documents/files and submitted commits, checks/results,
  acceptance/rejection reasons, integration commits and resolved obligations,
  failures/recovery/deferred work, requirements-invalidated tasks, overall status and
  resulting revision. Worker claims remain claims until supported by saved evidence.
- **SUMMARY:** coordinator closeout stating delivery, failures/incomplete work,
  deferrals/unresolved decisions, recommendations with rationale, artifacts/commits
  and next steps. Label narrative/opinion separately from observed application facts.

Append facts from committed journal/events and linked records without model calls
to narrate events. Preserve original entries; append corrections with source links.
Coordinator explanations and closeout are saved through the existing conversation/
result path. If unavailable, write a deterministic checkpoint from saved records and
mark the narrative summary pending. An optional grouped closeout/compact summary
references the log through an event watermark; it never rearranges or replaces the
original chronology. Link bulky transcripts, tool output, diffs and artifacts rather
than copying them. Exclude credentials, secret environment values and hidden reasoning.

Extend the current event/subject linkage and checkpoint/context mechanisms with the
minimum versioned metadata needed. The current event schema has sequence/type/task/
body but no universal timestamp or root request identity; add these with lifecycle
events alongside dispatch/integration changes. Preserve legacy evidence and mark
missing timestamps/inputs unknown. The [state contract](contracts/state-contract.md)
owns continuous projection, restart deduplication and privacy rules. Select compact
summaries and relevant referenced events for future context; never automatically
replay the entire log into every agent packet. This adds no separate milestone,
trial, model allowance or approval step.

## TUI and initial provider delivery

Keep the accepted composer, focus/scroll behavior and optional drawers. Show parallel
work units with owner/model, activity, dependencies, submitted/integrated/accepted
status and remaining obligations. Show integration branch/check state, dependency
updates waiting for a safe point, capacity waits and actionable decisions. Link exact
commits/diffs/output in Inspect; do not crowd conversation with Git bookkeeping.
“Assigned work submitted; integration pending” and “feature complete” must differ.
Detach/resume restores this view from existing snapshots/events, without replaying work.
Inspect and closeout expose the selected request's absolute log and compact-summary
paths for direct opening in a terminal editor, plus projection freshness/attention
and narrative-pending status. Parallel tasks link to their common request history.

Initially use **Codex gpt-6.1-sol / medium** for coordinator and workers. Normalize
typed coordinator actions into the existing writer/scheduler and native worker events
into supervised operation evidence. One provider suffices; expose a small extensible
inventory/launch/stream/cancel/completion interface, supported role/model/effort choices,
worker preferences and configured fallback. Distinguish discovered, configured and
usable with reasons; never substitute mocks silently or run inference for discovery.

Prior read-only discovery (2026-10-15, not repeated here): Codex 0.162.1 with config/
auth-file presence and cached efforts; Claude 2.1.300 with settings/credential presence
but a Waywr 2.1.296 pin; OpenCode v2.0.29 at `~/.opencode/bin/opencode` in Fish's path,
configuration unverified; existing Ollama 0.40.4 lists qwen3:8b Q4_K_M (~5.23 GB).
Host: ~62.7 GiB RAM, RTX 4070 12,282 MiB VRAM, 9,317 MiB free at that inspection.
Presence is not authentication/performance proof. Other execution adapters and local
model tuning can follow useful delivery; their absence does not block Example Task App.

## Connected implementation sequence

1. **Connect normal use to supervised development.** Reuse stable Rust packaging,
   init/startup, provider selection, chat and exact scope/policy approval. Extend the
   existing parent workflow with child records and typed coordinator dispatch into
   host/runner operations. Remove development-only call/time/repair restrictions and
   text-only tool rejection. Add task branches/worktrees and actual commit submission.
   Carry stable request identity through discussion/dispatch and start continuous
   history projection from those same committed records.
2. **Deliver useful parallel tasks and integration.** Add minimal contract/dependency/
   obligation records, readiness/concurrency selection, durable notifications, safe
   synchronization and the serialized integration executor. Wire TUI visibility and
   existing generation/receipt/command reconciliation through these changes. Keep
   one active parent workflow; no exhaustive scheduler generalization. Extend the
   same history with assignment revisions, dependencies, obligations, steering and
   integration outcomes; expose its path in Inspect as these records become usable.
3. **Complete the real job through this path.** As soon as the connected path can run,
   start Example Task App through the stable installation and normal TUI. Complete steering,
   progress/capacity handling, review/repair, combined checks, main delivery, context
   updates and resume while using it. Include readable closeout, deterministic
   fallback checkpoint and compact history selection in those context updates.
   Fix defects encountered in that retained job; preserve its state/commits through
   compatible Waywr fixes. Do not defer the working
   application to another proposed acceptance phase.

These are recoverable implementation commits/progress records, not approval stops.
After implementation is approved with its necessary runtime scope, proceed through
them automatically. No trial profiles, prerequisite native-admission stages, fixed
allowances, separate evaluation installs or repeated scope approvals. Document
unrelated limitations unless they prevent this outcome or threaten project data.

Retire obsolete fixed-stage/four-file constraints from the development path. Correct
misleading save/completion wording and narrow CLI routing while touching those seams.
Keep useful checks; broad Python deletion is not on the critical path. Rust bundle
assembly still reads three prompts under `src/agent_workflow/resources/skills`; move
those resources before removing legacy package code. Preserve Cargo.lock and use
permanent rustup 1.99.0; add no new application runtime or Herdr requirement.

## Example Task App work split and focused verification

Waywr's coordinator refines this small split in conversation:

| Work | Start condition | Output / integration obligation |
| --- | --- | --- |
| Shared contract/build skeleton | Approved scope | Task record/store interface, ID/error/ownership semantics and crate skeleton; all generated by a Waywr worker |
| JSON persistence | Shared contract commit | Real atomic store, restart/corruption checks |
| CLI/task behavior | Same contract commit, concurrently with persistence | Commands, tags/filter/export using a separate in-memory test store; production-store wiring remains an explicit obligation |
| Integration/review/closeout | Submitted results and applicable dependency readiness | Wire real persistence, review/repair, binary checks, README, integrated commit and saved context |

The CLI worker can finish before storage; an integration worker then connects them.
If storage finishes first, notify the active CLI task and synchronize only at its
safe point. Keep test doubles as test fixtures; prove the shipped binary uses real
storage by writing, exiting and reopening data. Shared build-file edits have one
owner. Parallelism exercises actual existing functions without adding features.
Its one request history naturally captures the storage/CLI assignments, either
completion order, placeholder replacement, checks and integrated result. After
detach/resume, inspect that same history and use its compact closeout for follow-up;
Example Task App itself needs no extra logging feature to exercise Waywr's history.

Reuse focused command/writer, host-attempt, scope and result-adoption tests. Add only
the important new checks, using small temporary Git repositories where appropriate:

- Socket example in both completion orders: agreed `socketlib.h`; one task implements
  `socketlib.c`, another implements `main.c` against a separate tracked stub. Assert
  independent start, preserved submissions, no mutation under active editing, safe
  notification/synchronization and real-library linkage/behavior before acceptance.
- Interrupted integration before Git mutation and after Git commit but before writer
  acknowledgement: reconcile once, including duplicate dependency notification after
  restart. Preserve a conflicted/dirty tree; obsolete requirements cannot integrate.
- Focused progress/rate-limit checks: no equivalent-call loop, no silent fallback,
  persisted capacity wait, and healthy long work survives the old blanket deadline.
- TUI/native adapter checks for delegation, concurrent activity, steering/obligations
  and truthful completion, plus Example Task App's real build/tests and persistence smoke.
- Focused history projection checks: one request across attempts/steering/restart;
  interrupted append/cursor publication has no missing or duplicate events;
  corrections preserve prior entries; secret/hidden-reasoning exclusion and project
  isolation; coordinator-unavailable checkpoint and Inspect/closeout paths. Reuse
  the integration-interruption check to verify its history, rather than another trial.

Inspect harnesses before running; some launch services. No exhaustive provider,
failure/recovery or concurrency matrix, and no historical suite rerun without a
relevant change. The real retained application's working result is the delivery test.

## Product choices for review

Example Task App/destination and initial Codex model/effort remain the recommendation.
Routine engineering choices above are resolved. The meaningful policy tradeoffs are:

- **Local main:** recommend automatic checked fast-forward after the approved body's
  acceptance; retaining only the integration branch is available if final manual
  handover is preferred. No extra routine approval is required by the default.
- **Provider fallback:** recommend wait/resume on Codex capacity initially; alternative
  provider fallback is opt-in because it changes cost and where project context goes.
  Resource caps remain optional, with no suggested mandatory allowance.

This revision changes documentation only. It authorizes no Example Task App creation,
application implementation, inference, worker/service launch, installation, push or
merge. Stop after committing the plan/handoff/architecture revision for review.
