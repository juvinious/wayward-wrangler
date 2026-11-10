# Current local session protocol

Current state9/conversation6 services use IPC1.10 with `provider_selection_v1`;
the frontend negotiates minor10. Provider/model options remain opaque profile
data. IPC1.9 added `coordinator_recovery_v1` alongside IPC1.8's
`conversational_coordination_v1`. Minor9 adds derived recovery/progress and effective
finding projections to existing snapshots/events, plus frozen review-result semantics;
no new transport or execution command. Retained
`recovery-v1` records require a compatible paired release. IPC1.8 added
`coordination.submit`, `coordination.present` and `coordination.disposition` command
actions plus `conversation.coordination` snapshot context. The
[conversation contract](../conversational-coordination.md) owns frozen input/card
identity and source provenance. Older bound
states retain their existing commands. Submitted legacy messages are not reclassified.

The Rust TUI and CLI communicate with the selected project's Rust service over
a same-UID Unix socket. Frames are newline-delimited UTF-8 JSON envelopes
`{v, kind, request_id, body}`; protocol major1 is required. Shared
`backend/src/ipc_limits.rs` keeps incoming frames at1MiB and commands at64KiB;
complete replies may be up to8MiB so the full model catalog and retained project
projection fit together. A larger reply returns a correlated `response_too_large`
error with kind/size/budget, leaving the connection usable. It is never presented
as a partial snapshot or permission to replay an operation. Retained project
data is unchanged; larger whole-project views will require scoped/paged reads.
`hello` binds project key, exact release
and channel before requests. New host-backed services advertise minor 1 and
`host_operations_v1`, `output_pages_v1`, `artifact_pages_v1` and
`herdr_views_v1` capabilities. Schema-4 services advertise minor 2 and
`conversation_v1` and `file_candidates_v1`, including
`conversation.read`/`conversation.result` and a
conversation object in `snapshot`; their `submit_message` command accepts a
conversation turn rather than creating a task. Legacy minor-1 projects retain
their original task-submission meaning. Clients must use the negotiated schema
and capability before issuing a command. A view channel has attempt-bound read grants
and cannot submit workflow mutations.

Current paired applications add `conversation.activity` with `{turn_id}` and
reply `conversation.activity_result` containing the project-local turn ID, its
stored status and an optional `runtime` display projection. The same projection
and subject ID appear on recent `snapshot.conversation.turns` entries. It contains
the published host attempt ID, observation time, selected provider/model/thinking,
a bounded activity label and numeric usage/cost/context fields. Each telemetry
file read is limited to64KiB; malformed, oversized or unavailable telemetry
becomes unavailable rather than preventing opening. Project/operation/attempt
identity and containment are checked. There is no raw packet/event/tool output.
The TUI polls active selected-subject calls once per second because file progress
does not itself create conversation events. Telemetry cannot change turn status,
scope, budget, settlement or execution authority; stale replies cannot replace a
newer observation or revive a completed call. Unknown turn IDs refuse. This is an
additive read for paired applications, not a new mutation or storage schema.

IPC 1.5 / `continuous_scope_v1` is documented in the
[continuous coordinator contract](continuous-coordinator-contract.md).
New schema-7 services advertise continuous_scope_v1 and require minor-5 clients.
Queue/recovery/question mutations are versioned command actions; existing frame
kinds remain. Negotiation and project/release binding precede every request.

State8 advertises IPC1.6 with `development_v1` and `request_history_v1`, adding
the exact-revision `development.reply` command for substantive coordinator questions,
project-owned child/workflow records, operating policy, history/summary paths and
projection/narrative status to existing inspection. Older IPC1.5 state7 clients and
records retain their prior contract. The [state contract](state-contract.md#state8-development-and-request-history)
owns continuous projection and restart behavior. No separate logging transport or
new authority is introduced.

Commands use application-owned IDs and exact payloads. A durable identical
replay returns the original result; ID reuse with different bytes refuses.
`busy` before acceptance is not durable admission. A lost reply requires
`command_get`/snapshot reconciliation, not a new command or model attempt.
Task version and state revision reject stale writes. A repeated `stop_session`
against exit-requested/draining/paused is an accepted durable no-op: it preserves
the original lifecycle fence and returns `already_stopped:true`. It needs no new
state revision/event. A stop against a resumed running session still requires
the current state revision. Event reads use a cursor
and bounded pages; events notify, while application records remain
canonical. Reconnect takes a snapshot watermark, reads later events and
resynchronizes gaps. Output and artifact reads are project-scoped, bounded and
hash/containment checked. View data cannot become an answer or approval.

The [architecture](../architecture.md) owns lifecycle and recovery semantics;
the [state contract](state-contract.md) owns authoritative records.
`backend/src/session_service.rs::dispatch` is the implemented request-kind
inventory for this release. The checked-in
[`protocol/session-v1.schema.json`](../../protocol/session-v1.schema.json) now
lists the implemented minor-1 methods and their advertised capability groups;
[`tests/session_schema_consistency.py`](../../tests/session_schema_consistency.py)
checks the schema, service dispatch and hello advertisement together, including
the schema-4 minor-2 through state9 minor-9 boundaries. This
schema describes the envelope and kind inventory, not every method body or
state-dependent authorization rule. `session.status` is a current base method.
`operation.retry_start` is an idempotent `command` action, not a frame kind.
The approved `view.herdr.open` is an operator action: the current CLI performs
the `view.prepare` → `view.sent` → `view.bind` handshake so uncertain sends
can be reconciled before creating another pane. The old `ui.attach` and
`ui.register` requests are recognized but refused by the direct-attachment
release. Legacy-only envelope kinds remain listed for historical readers.
The service advertises minor 1 for schema-3 host-backed projects, minor 2
for schema-4 projects, minor 3 for schema-5, minor 4 for schema-6 and minor 5 for schema-7 projects; clients must
check advertised capabilities before using these methods. The envelope schema
is not a generated client or runtime validator. Historical
[milestone-6 proof wording](../archive/contracts/session-protocol-pre-consolidation-2026-10-13.md)
is retained for provenance.

## Output, artifact and optional-view contract

These requirements remain current after archiving the completed host-transition
proposal. Invocation status and output inspection are distinct from execution
authority; use the [execution contract](execution-contract.md) for host recovery.

- Captured payload is bounded at 32 MiB combined stdout/stderr, with a 16 × 64 KiB
  runner queue, 50,000 structured-event cap and 65,536 × 32-byte index (2 MiB).
  Index/payload exhaustion is explicit, with retained bytes and a limit reason;
  it cannot silently drop authoritative output or allocate without bounds.
- Index records carry sequence, stream label, byte range and capture timestamp.
  Sequence describes observed interleaving, not exact cross-stream timing. Preserve
  original bytes; viewers sanitize terminal controls/OSC. Historical unlabeled
  streams display as combined legacy output.
- State8 `operation.list` accepts optional `active_only: true`, echoed in its result,
  to list only this task's running worker operations. Optional `activity` contains
  a presentation-only status/label from the exact published attempt. It reads at
  most the final 256 KiB of complete native-event frames; incomplete frames are
  ignored, malformed data is unavailable, and terminal receipts clear commands.
  No output/results/arguments/model text are copied. The TUI polls once per second,
  one outstanding list request, and discards mismatched task replies. This neither
  launches work nor changes execution authority or retained evidence.
- `operation.output.read` scopes task/operation/attempt and cursor
  `{format,attempt,sequence,offset}`. Pages return stream segments, next cursor,
  published high-water mark, status and any gap/limit. Read only complete index
  records; report incomplete crash tails without altering them. Pages cap at
  64 KiB raw / 256 KiB encoded, shrinking for expansion.
- Follow allows at most four reads/second/view, one outstanding request and four
  admitted readers/project. The TUI cache is bounded at 2 MiB; older pages reload.
  Reads use bounded blocking admission outside writer transactions. Slow viewers
  cannot backpressure workers. Reconnect deduplicates by attempt/sequence/offset;
  stale cursors report gaps and reset visibly, not by matching text or replaying calls.
- `artifact.read` validates project containment and selected hash. For schema-4
  static-file runs, `artifact: latest` pages the full verified four-file bundle
  and records contiguous read progress for the current review request. One
  allowlisted filename returns a verified single-file preview and no bundle
  approval receipt. Approval binds the complete exact manifest and request, not
  merely the displayed page or selected file. Opening a view is never approval,
  an answer, or another model invocation.
- Explicit `observe --viewer herdr` / TUI inspection uses a same-UID verified
  endpoint, project-owned workspace and attempt-limited read token. No stdin or
  signals pass through it. Repeated open reuses the verified view; lost creation
  replies reconcile saved intent/token/executable before another view is possible.
  Closing or losing a view changes display routing only. An absent server yields
  an unavailable-view message; ordinary operation never probes Herdr or inherited
  stale variables. Do not spoof its environment or start a server automatically.
- The explicitly requested inspection workspace may retain its bootstrap shell
  tab. Never replace unrelated foreground work to remove that display limitation.
  Ordinary TUI startup creates no panes. A stopped service may leave a view showing
  unavailable or retained output; opening it cannot resume scheduling.

Public errors distinguish preacceptance `busy`, stale state, uncertain delivery,
uncertain execution, exhausted proven prelaunch recovery, output limits, unavailable
views and incompatible contracts. Brief action text belongs in conversation;
complete diagnostics remain in `/inspect`. Schema4 mock conversation/file behavior and schema5/6/7 native-conversation profiles
retain distinct capability/admission semantics. Historical trial results/ceilings
are not protocol authorization; their runtime resources were explicitly discarded.

## Minor 3 — Claude conversation-only capability

Schema-5 services require minor-3 clients and advertise `claude_chat_v1` plus
conversation/output/host capabilities; file-workflow authority is unavailable.
The existing `conversation.read`/snapshot includes `chat` readiness/profile/hash
and optional bounded `provisional` text. `conversation.delta` events contain
only progress identity/hash/length; snapshots coalesce actual assistant text.
`select_coordinator` accepts an explicit `authorize:true` and exact
`profile_sha256`, preserving the original three-call allowance.
`cancel_conversation` targets a supplied current `turn_id` and records an
idempotent interruption intent; completion/receipt races retain existing host
semantics. Existing project/release/generation, command replay, frame limits and
reconnect rules apply. [Claude contract](claude-coordinator-contract.md) owns
role restrictions; no native text is converted into an IPC command.

## Minor 4 — proposal/scope authority

Schema-6 services require minor 4 and `task_scope_v1`; earlier state capabilities
and meanings remain versioned. [Scope contract](live-scope-contract.md) owns
`save_proposal_revision`, `authorize_scope`, answer-only `answer_conversation` and
target/answer-aware `submit_message`. Reads use existing `task.get`, snapshot and
conversation methods. Decisions bind exact base/hash, dependency revision, prior
authorization identity and unresolved-choice acknowledgement. `task.plan`,
`admit_task`, workers, consultation and execution budgets refuse in this profile.
No new envelope method or generated client. Ordinary messages append durably without
a global progress CAS; exact scope writes retain their stronger stale checks.
The consistency check covers all minor-1/2/3/4 method/capability boundaries.

## Minor 5 — continuous interaction

State-7 requires `continuous_scope_v1`; existing methods/envelope are reused. The
[continuous contract](continuous-coordinator-contract.md) owns subject/causality,
versioned question decisions, held release/withdraw, typed outcome acknowledgement
and action fencing. `submit_message` accepts explicit subject, optional revision
intent and exact recovery generation/hash; no global watermark invalidates speech.
`continue_held`, `withdraw_queued` and `confirm_question_disposition` are durable
command actions, not new frame kinds. New receipts bind typed completion evidence.
State-7 scope acknowledgements include their committed state revision; stopped
conversation events include their committed recovery identity. These are projections
of writer-owned records. Exit's frontend snapshot preparation uses existing reads;
its durable stop still passes the unchanged lifecycle CAS and ownership rules.

## Minor 6 — connected development

State9 negotiates minor7 / `provider_selection_v1` in addition to minor6 capabilities.
`development.providers` exposes revisioned preferences, current binding, switch and
bounded local discovery evidence. Durable `providers.save`, `providers.refresh`,
`providers.switch`, `providers.cancel_switch`, `providers.task_override` and
`providers.approve_policy` use existing command IDs, writer and revision guards.
No new transport or view mutation path is introduced. See the
[selection contract](../provider-selection.md) for authority and compatibility.

State8 clients negotiate minor6 and `development_v1`, `request_history_v1`, plus
existing scope/continuous/host/output capabilities. `snapshot` conversation data adds
`development` with workflow/unit/effect/notification records and readable history
paths. `task.get` includes exact operating policy with scope. `authorize_scope` binds
`operating_policy_sha256` alongside the existing exact revision/decision hashes.
`development.reply` answers a saved current coordinator question at an exact workflow
revision and leaves scope/policy authority unchanged. `development.resume` and
`development.cancel` require the selected work ID and
exact workflow revision. Neither grants scope authority or permits replay of
ambiguous execution. Old minors retain their
capabilities; no new transport or separate database is introduced.

State8 `development.resume` may include bounded `payload.recovery_note` operator
context. It is saved under the existing request, projected with operator attribution,
and supplied in the next coordinator packet. It does not change scope/policy or
requirement authority. Changed recovery context invalidates idle/action-input
fingerprints so a repaired environment can support a fresh assignment.

Minor6 presentation metadata may include numeric Unix-seconds `at` on event/message
rows from the request-history timestamp sidecar. Missing legacy times stay absent.
Active development pending/unit records and reserved discussion queue rows may have
`started_at`; clients must not infer execution authority from either timestamp.
`development.resume` retains its exact work/revision guard and now supports recorded
Python-cache retention; it refuses saved questions/pending execution and grants no
new scope permission. `/resume` distinguishes it from lifecycle `resume_session`.

## Ordinary project workflow additions

IPC minor10 adds `ordinary_project_workflow_v1`, `project.setup`, development snapshot
`project_setup`, `candidates`, `cleanup` and candidate exact-identity commands:
`candidate.present`, `.checkout`, `.accept`, `.feedback`, `.include`, `.end_validation`.
Candidate payloads carry work_id/candidate_id/commit; stale revisions and unpresented
acceptance refuse. Setup preferences never authorize execution. Existing conversational
input context freezes candidate identity alongside proposal/question/provider identity.
The Ollama HTTP child is an internal supervised transport, not a new arbitrary IPC
filesystem or execution grant. Profiles/attempts include exact optional transport identity.

## Application replacement boundary

Project opening no longer routes to an executable named in the legacy project
release record. This is separate from the running IPC connection's release and
owner identity checks, which remain enforced. A new frontend does not bypass the old service's release identity checks. It
verifies the exact unit process and socket peer, then replaces a positively idle
owner under startup/execution fences. Attached views, pending input or unresolved
launch evidence refuse replacement. The optional real-systemd acceptance case
covers that replacement; it never attaches across incompatible service identities.
