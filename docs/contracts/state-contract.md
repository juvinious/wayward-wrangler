# Current project state and compatibility contract

This describes current Rust host-backed projects: schema 3 workflows, schema 4
mock conversation/static files, schema 5 Claude conversation-only, and schema 6 proposal/scope authority, schema 7 continuous interaction, and state8 connected development.
[Architecture](../architecture.md) owns component boundaries, this document owns
state/version/rollback policy, and the [execution contract](execution-contract.md)
owns dispatch, recovery and host lifetime. The
[old milestone-6 state contract](../archive/contracts/state-contract-pre-consolidation-2026-10-13.md)
is preserved for historical interpretation, not new execution.

## Version boundary

Ordinary-project IPC1.10 adds setup/candidate capabilities and optional generated
output inventories to existing state9 records. It does not increment the persistent
schema. The handshake advertises IPC minor10 separately from state schema9.

State9 IPC1.9 adds version1 recovery/progress/finding fields to existing development
records. `coordinator-recovery-v1.json` retains the `recovery-v1` bundle requirement
without a database-schema bump. Earlier frozen packets keep their original result
schema. Immutable native results remain distinct from validated effective dispositions;
linked reviewer judgments are overlays, not edits to earlier results or events.
Recovery transitions and decision adoption reuse the writer/pending-operation fence.
Readable history records those transitions continuously. See the
[recovery contract](../coordinator-recovery.md) for positive wake conditions and limits.

The state9 IPC1.8 [coordination overlay](../conversational-coordination.md) adds
writer-owned `coordination_inputs`, `coordination_cards`, `coordination_continuations`,
`coordination_turn_context`, `coordination_dispositions` and `coordination_repairs`.
These causal metadata tables reuse existing commands/authority; state9/config6 and
historical submission semantics are unchanged. Pending effects block release changes;
retained tables require a bundle with `interaction_contract: conversation-v1`.
Missing historical associations use append-only correction events, never altered
published frames or retroactive approval. New answer plus continuation is atomic.

The approved [development plan](../archive/development-workflow/2026-10-16-implementation-closeout.md) is implemented in
state8 through child records, dependency/obligation notifications, frozen Git
integration intents and persistent request history in the existing writer/database.
Git owns code history; one active approved parent owns concurrent workers.
Verification and the first retained application delivery are recorded
in the [delivery closeout](../archive/development-workflow/2026-10-16-handoff.md).

State9 / config6 / IPC1.7 is the [provider selection contract](../provider-selection.md):
writer-owned preference revisions, frozen routing/review records and coordinator
handover. A journaled quiescent state8 import advances the DB/contract version without
rewriting approval/task rows, worktrees, receipts or history. Codex remains the sole
allowed profile until explicit configuration/approval. Revision1 preserves exact legacy
Codex grants without rewriting historical approval hashes. Older binaries refuse state9.

The [continuous contract](continuous-coordinator-contract.md) owns state7/config4,
queue/subject disposition, typed outcomes and auditable question decisions. Older
schemas remain versioned. No automatic conversion, receipt reclassification or
snapshot restoration over accepted work. Trial authorizations are not part of this
maintenance contract; old runtime resources were explicitly discarded in the reset.

## State8 development and request history

State8 / conversation config5 uses the existing SQLite writer and versioned
`development_records` (workflow, unit, effect, notification), linked to exact scope
bases, policy hashes, commits and immutable operations. One active approved parent
is retained; task worktrees never become independent projects. No automatic state3–7
conversion or evidence replacement is performed. IPC1.6 advertises development and
request history capabilities.

`request_history_requests` owns stable request ID, initial name/timestamp/goal,
starting revision (explicitly absent for a new repository), subject/task linkage and
absolute log/summary paths. `request_history_events` adds timestamp/actor/request
metadata without rewriting legacy journal evidence. Public events project into the
canonical `.waywr/history/` directory (or an explicitly selected external state's
`history/`). [Layout migration](../project-layout.md) preserves immutable history
and authority while relocating mutable path records. Complete chronological frames
carry event ID, byte length and hash. Reconciliation validates complete frames and
repairs only an incomplete tail matching its durable append intent; unexpected manual bytes remain intact. A projection cursor follows durable file publication.
Corrections append new events. Raw native output is retained separately; secret and
hidden-reasoning fields are excluded from the readable projection. Recognized credential formats in public text are redacted; this is not a universal secret detector. Coordinator prose
is labelled narrative; deterministic checkpoints mark narrative pending until saved.

This projection never authorizes execution or acceptance. Inspect exposes source
records and log/summary paths. Compact summary/selected evidence supplies future
context; the whole readable log is not automatically replayed. Discussion context
selects the current topic/scope, compact unit/check/obligation facts and accepted
source knowledge matching current project HEAD for new topics. Prior workflow
plans/requirements and repeated resolution rationale are not replayed into a new
topic. Optional context is bounded against the final packet's remaining space;
selection counts disclose omitted details, which remain in Inspect.

Session/TUI development snapshots carry display metadata and compact obligation
statuses, not accumulated review diffs, native reports, receipts or resolution
evidence. Each projected record includes its source kind, content hash and omitted
field names. Full records remain unchanged in `state.sqlite`'s `development_records`
and exact operation files. These display projections must never become model input
or authorization records. Service readiness uses the small `session.status` method,
so lifecycle control does not depend on serializing conversation/evidence history.
Snapshot record counts are not yet paged; very large projects can still exhaust
the IPC frame budget and require further bounded inspection work.

The detailed semantics below remain the maintained requirements;
the handoff distinguishes what has actually been verified.

<a id="planned-request-history-projection--not-yet-implemented"></a>

## Request-history projection requirements

State8 implements the projection described above, including retained real-job
delivery. The handoff records verified behavior and remaining limits. Compact
checkpoints retain command outcomes/evidence links, resolved/open counts and up to
16 open-obligation details with explicit overflow count. Model text is bounded and
labelled; full rationale/narrative remains in the continuous log.

The [implemented development workflow](../archive/development-workflow/2026-10-16-implementation-closeout.md#persistent-request-history)
includes one human-readable log per project request. Extend existing subject/
workflow/task records with a stable root request ID, independent of transport,
question, child-task and attempt IDs. Record initial name/goal/project/base revision
and timestamp once; steering, reassignment and coordinator replacement retain that
identity. Each new committed lifecycle event carries UTC time, source identity and
actor/task revision/attempt attribution where applicable. Existing journal sequence
orders observations; do not invent a finer cross-worker execution ordering.

Default location is `PROJECT/.waywr/history/`; an explicitly selected external
`--state-dir` uses its own `history/`. New filenames combine compact UTC timestamp
and a 12-hex hash of initial prompt hash plus request identity (normally 33 characters
including `.log`). Collisions with saved paths or existing log/sidecar files extend
the hash. Full request text/name, stable ID and initial prompt SHA-256 stay in saved
records and readable entries, independent of the filename. Existing saved paths
remain unchanged. No global memory or separate logging database. The original
chronological log is append-only. Corrections/superseding decisions reference earlier entries; a grouped
closeout or compact summary is a separate derived view with source watermark/hash.
Approvals, receipts, Git commits and existing acceptance records retain authority.

The project service projects committed public records continuously in bounded
pages outside writer transactions and the interactive scheduler. Reuse the journal
cursor and existing writer for small projection metadata: request/path, renderer
version, acknowledged source sequence and complete-file offset/digest. Persist log
bytes before advancing the cursor. Stable source event IDs allow restart to recognize
a fully written entry whose cursor was not committed, and to complete an interrupted
final write from the saved source without duplicating entries. Verify the existing
prefix; preserve unexpected/manual bytes and surface attention instead of silently
rewriting them. A missing derived file can be rebuilt from retained canonical records;
do not delete its sources as a side effect of projection. A rendering-version change
must preserve already-written entries and resume compatibility.

Projection failure or lag remains visible and retryable without losing source events
or changing execution status. Resume catches up from the saved cursor; it never
repeats work to reconstruct prose. Legacy records keep their bytes/meaning and use
explicit unknown timestamps/revisions where evidence is absent. Public text and
allowlisted factual fields are rendered with terminal-control sanitization and
credential redaction; never dump environment/configuration objects, credentials,
hidden reasoning or raw provider streams. Project containment, restrictive file
permissions and symlink checks apply. Link bulky outputs/transcripts/diffs/artifacts
by scoped path and source/commit/hash rather than embedding them.

Facts come from canonical assignments, decisions, execution receipts, checks and
integration outcomes. No additional model call narrates each event. Save coordinator
explanations/closeout as attributed narrative in the normal conversation/result
records; never convert an unsupported completion claim into an observed result.
Without a coordinator closeout, derive a checkpoint listing observed status/commits,
delivered artifacts/checks, failures, open obligations/decisions, deferrals and next
actions, explicitly marking narrative pending and unknowns. Later narrative appends
to the history rather than replacing that checkpoint.

Expose log/summary paths, source watermark and freshness/pending/attention through
existing snapshots and task inspection. Context selects a compact summary and
relevant source events with project/request identity, provenance and requirement/
input/dependency validity; it does not ingest whole logs by default. Editing a
readable file never modifies authority. These requirements ship with the existing
workflow/context work, not as a prerequisite trial or independent logging phase.

## Locations and authority

`PROJECT/.waywr/project.toml` holds schema-2 project identity and
`execution.host = "systemd-user-v1"`. `workflow.toml` holds selectable profiles;
`release.json` binds the exact compatible release and manifest. Default
`PROJECT/.waywr/state/` contains the state identity,
`state-contract.json` declaring application state schema 3, 4, 5, 6 or 7, `state.sqlite`,
versioned packets, outputs, artifacts, attempts, receipts, checkpoints and
recovery evidence. An operator-selected `--state-dir` changes only this
location. `init` writes inside the project, preserves repeat-init identity
and binding, and launches nothing. The local `.gitignore` excludes runtime
state, not versionable configuration/context. New source initializes continuous proposal/scope
schema 7 and `.waywr/coordinator.json`; existing schema-4 mock projects
retain their distinct execution contract. Opening a project never upgrades it.

First service attachment may write a nonauthoritative location index at
`${XDG_STATE_HOME:-$HOME/.local/state}/wayward-wrangler/locations.json` (or
`--registry-dir`) for project-qualified links. It contains no project memory.
It cannot block a pristine same-path reinitialization without positive
surviving-state/service evidence. The completed correction and its old/new
fixture evidence are [archived](../archive/portability/stale-registration-reinit.md).
A same-user process with unrestricted filesystem access is outside the app's
project-isolation guarantee.

## Records and transitions

The Rust workflow ledger in `state.sqlite` owns stage state/revision,
operation association, task and command outcomes, budgets, human requests and
answers. It does **not** decode or write LangGraph checkpoints. SQLite
transactions are short and run through the bounded dedicated writer. A
recorded command ID plus exact canonical body deduplicates replay; changed
bytes under the same ID refuse. Task version and state revision reject stale
mutation. Events are ordered notifications, not substitutes for canonical
artifacts, decisions or receipts. The state schema/migration version and
release binding must match before new mutation.

Packets freeze project, input/configuration, role/model/budget and selected
context. Cross-project import records name explicit source identity, selected
objects, provenance/hash, purpose, dependency mode and revocation. Only
selected material reaches a model packet; no global registry or unrelated
project is automatically read. A previously delivered packet stays immutable
after grant revocation. Finding origins and later assessments are distinct;
omitted dispositions remain unresolved. Human answer and approval records
bind request version and the exact reviewed artifact hash. Worker text can
never create those records.

A logical operation may have linked host attempts. Before a host start, the
app persists intent, input and unique unit identity. Append-only observations,
claim, permit, dispatch commitment, child identity, output, receipt,
retirement and recovery explain its history. Validated receipts can be
adopted after controller/service restart without another call. A missing unit,
PID, heartbeat or output does not prove nonlaunch. Only positive prelaunch
proof plus retirement fence permits a linked replacement. An ambiguous call
remains uncertain; model-call budget is not reset by infrastructure recovery.
Original failed attempts and their output stay inspectable.

Schema-4 conversation tables store versioned project-keyed messages, turn
reservations and terminal results, open/answered questions, coordinator binding
generation, action proposals and separate conversation/consultation/task call
charges. The same project writer commits them and the session event cursor.
`submit_message` creates a conversation turn, never a task by itself. Only a
validated `propose_task` result materializes an unadmitted task; scope
authorization is still separate. Answers are durable, idempotent and queue a
dependent interpretation turn without spending a call until that turn is
reserved. A reserved, failed or uncertain call retains its budget charge;
same-result replay is idempotent. Application records, not provider sessions,
own these states. Checkpoint-3 installed mock verification covers the accepted
TUI's conversation, answer, consultation, detach and resume; missing mock result
evidence becomes uncertain without redispatch. Schema-4 native provider admission remains
closed. Schema-4 static-file tasks add versioned `static_runs`, reserved stage
effects and exact-manifest read receipts to the same writer. Four task calls and
two reviews are the frozen ceiling. Candidate files are immutable and hashed;
finding origins, author dispositions and independent review assessments remain
separate. A worker question consumes its completed call, while the durable human
answer consumes none. Insufficient follow-up/re-review allowance stops with the
available candidate. A tampered candidate cannot be approved; decision-time
integrity failure records `output-invalid`. Final approval binds the complete
manifest/request after full-bundle reading, not one file preview. The installed
checks use deterministic mock outputs only.

Session state, task execution state, UI attachment and service lifetime are
separate. `/detach` removes UI attachment only. `/exit` persists a dispatch
fence/checkpoint and drains current permitted work to a bounded foreground
deadline. `resume` advances ownership generation and reconciles before
permitting more work. Old exit generations cannot stop newly resumed work.
Known stalled execution and ambiguous execution remain distinct. A model
summary is optional context, never the authoritative checkpoint.

## Compatibility and rollback

Current host operations and completion receipts use schema 2; host-attempt,
recovery and owner-endorsement records are version 1. Direct UI attachment uses
binding schema 3 and process identity, without a Herdr server/pane prerequisite.
Optional read-only view bindings are separate schema-1 records. Mutable routing
pointers/indexes are projections; they cannot replace original execution evidence.
The [IPC contract](session-protocol.md) owns capability negotiation. Schema4/minor2 conversation and file capabilities are implemented with mock-only
evidence. Their native admission remains distinct from Claude conversation profiles.

Schema-3 host-backed projects use only a release that declares that contract.
Existing Herdr-backed and unfinished LangGraph projects remain on their bound
legacy releases; their original records, bytes, hashes and interpretation are
not converted. Historical read-only inspection is separate from execution.
An unknown or mixed schema refuses mutation. A release change is selected-
project-only and requires quiescence, empty/validated host units, a compatible
target release and state revision plus full state-file fingerprint. Rollback
changes the binding only; no old snapshot may replace newer accepted work.
A benign state write can conservatively refuse rollback and require a
compatible forward correction. Releases referenced by projects remain retained.

## Schema 5 — conversation-only release boundary

Projects with `[conversation] schema_version = 2` use application state 5;
no existing state is converted. Versioned conversation tables are reused, with
`claude_chat_authorization_v1`, `claude_chat_stream_v1` and
`claude_chat_cancel_v1` metadata. Frozen packets, host receipts and runtime usage
remain immutable execution evidence. Completed command outcomes are replayed
before readiness/exhaustion checks. Three held/charged calls persist across
restart; worker/consultation allowances are zero. An old schema-4 release is
incompatible, even if table layouts look similar. Keep a compatible release or
forward-repair after accepted work; never restore old snapshots over it.
[Claude contract](claude-coordinator-contract.md) owns this narrow increment.

## Schema 6 — saved scope, separate execution authority

`[conversation] schema_version=3` projects use state 6 and minor 4.
[Scope contract](live-scope-contract.md) owns immutable proposal revisions, head
pointers, append-only human decisions, exact hash/dependency/prior-authorization
fencing and once-only answer continuation. Scope authorization never creates a run
or execution admission. Local edits and decisions remain available at exhaustion.
Existing schema 3/4/5 projects are not converted. Keep a schema-6 compatible release
after accepted state; rollback cannot overwrite newer work or route an older schema
release to it. Historical reports distinguish mock and native observations.

State9 request-role choices use `development_records` kind `request_routing`, keyed
by subject with source-message identity/hash and append-only public change events.
Approval freezes `request_roles` into the policy while retaining broad allowed sets.
The supported stopped-project reconciliation preserves proposal/message/decision
rows and queues no turn. Release changes require `pinned-roles-v1` when these records
exist; an older release cannot silently ignore the pins.
