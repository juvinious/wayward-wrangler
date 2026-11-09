> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Live discussion → saved proposal → scope authorization

**Approved implementation baseline, 2026-10-14. Four checkpoints and all three
recommendations approved after planning commit `8c1bc7d`. Native calls remain separately
gated.** Base: `24fb31712cc4a52553375b0e5bc8de667f85a985`,
verified squash merge of PR #6.
Implementation branch: `feat/live-task-proposal-authorization`; planning branch retained.

## Outcome and boundaries

Keep `cd PROJECT; waywr init .; waywr`, the accepted production TUI and automatic
project service/runner management. Discuss naturally with Claude, clarify scope,
inspect/edit a saved proposal, and explicitly authorize its exact revision.
Authorization ends at **“Scope authorized · execution unavailable.”** No worker,
consultation, repository access, generated application, execution allowance or
queue admission is delivered here. Text such as “looks good” remains conversation.

Reuse the pinned Claude client/model/effort and saved-login route from the
[implemented contract](../../contracts/claude-coordinator-contract.md). Change only the owned
response contract/prompt and application proposal authority. No new tools, provider,
scheduler, UI prototype or model picker. Model output can suggest a proposal or
question; only the application commits records, and only a human command authorizes.

## Existing capabilities and actual gaps

Source inspected at the base above; no application tests or runtime experiments
were executed during planning. Historical results remain evidence for their exact
builds, not proof of this extension.

| Existing / reusable evidence | Gap to implement |
| --- | --- |
| [`claude_chat.rs`](../../../backend/src/claude_chat.rs), `argv`, `inspect`, `delta`, `finish`: pinned restricted print client, streaming text, validated single completion; `finish` returns `action:null` | Versioned response envelope that permits only question/proposal suggestions. Preserve the current text-only profile unchanged |
| [`native_contract.rs`](../../../backend/src/native_contract.rs), `CoordinatorTurn`: strict typed actions; schema-4 `ProposeTask { brief }` | A brief is insufficient: structured scope, immutable revisions and provenance are missing. Do not enable all old actions |
| [`conversation_store.rs`](../../../backend/src/conversation_store.rs), `conversation_apply`: writer transaction commits result/message/action and an unadmitted task once; validates packet/binding and deduplicates result hash | Native schema 5 explicitly rejects every action. Existing proposal uses one `session_proposals` row/task; no revision history or proposal-specific stale-result check |
| Same file: questions/answers, call reservations, durable commands, bounded handoff; tests `validated_proposal_creates_one_unadmitted_task`, `reservation_answer_binding_and_project_fences`, `overlapping_reservations_do_not_overspend` | Questions are currently mock-only. Schema-4 saving an answer queues another turn; new scope editing/answering must remain usable without spending or requiring calls |
| [`session_store.rs`](../../../backend/src/session_store.rs), `admit`, `plan_host`: exact plan hash, task version/state checks and durable admission | `admit` sets `ready`; [`session_service.rs`](../../../backend/src/session_service.rs), `promote_ready` dispatches. This is execution authorization, unsuitable for scope-only approval. `steer_queued` handles pre-coordinator text only |
| [`view.rs`](../../../frontend/src/prototype/view.rs), `action_label`, `live_modal_body`: scope/question/inspection panels, empty-composer action, drafts and reconnect | Scope panel describes simulated worker settings; native Inspect/target paths assume no task. Add scoped review/edit behavior through these existing components |
| [`session_service.rs`](../../../backend/src/session_service.rs), `tick_chat`; host receipts, bounded writer, release routing | Preserve the native host/adoption path; do not let schema 6 fall through schema-4 mock scheduling. Extend explicit capability gates and packet dependencies |
| [Completed trial report](../native-integration/claude-conversation-closeout-2026-10-14.md): three live replies/receipts, operator detach/resume; installed mock/replay checks | Native structured proposal reliability, live authorization flow and recovery of new records are unverified |

## User-visible flow and authority

| State / event | Visible next action and effect |
| --- | --- |
| Discussion, no saved task | Coordinator replies naturally. No automatic task per message. A valid first proposal creates one application-owned task ID; prose alone does not |
| Clarification ready | **Answer question** opens existing choices/free-text panel. **Save answer** records the answer only. **Continue with coordinator · uses 1 call** explicitly submits the saved answer once; ordinary conversation remains available |
| Draft / proposed revision | **Review scope · revision N** opens objective, deliverables, inclusions/exclusions, constraints, acceptance criteria, unresolved choices and selected-context references |
| Revise with coordinator | **Request revision** targets the selected proposal in the ordinary composer, preserving the previous draft. The operator sends their wording unchanged; no phrase matching. A new completed suggestion may save N+1 |
| Revise locally | **Edit scope** selects one fixed field in the same panel, using the existing multiline editor. **Save revision** commits a full revised body plus change rationale without a model call; Escape retains an unsent edit draft |
| Eligible proposal | Separate **Authorize scope · revision N** action, preceded by “Saves this scope only. Workers cannot run in this release.” Opening, reading, selecting or discussing never authorizes |
| Unresolved choices remain | Show them together. Require an explicit acknowledgement of the exact displayed list before scope authorization. Open clarification questions must be answered (an explicit deferral is a valid answer), not silently treated as resolved |
| Scope authorized | **View authorized scope**, with revision, decision and “No execution allowance; no workers launched.” Keep it in the task drawer; do not label it completed, queued or active |
| Saved content / relevant decision changes | **Review revised scope · authorization needed.** Append the supersession reason; retain the old revision and human authorization intact |
| Stale or interrupted response | Keep reply/evidence with “Based on an earlier scope; suggestion not applied” or the existing stopped/uncertain notice. Offer inspection and explicit revision request, never an automatic call |

Enter on empty composer/focused activity opens the relevant panel; nonempty input,
Shift+Enter/F2, literal paste, menu Enter/Tab, scrolling, selection and draft/focus
preservation keep their accepted behavior. `/plan` opens scope for the selected
task; `/answer` is available only for a real open question. Final artifact `/approve`
is unavailable here; scope authorization is not a completed-artifact approval.
No forced scroll-to-bottom gate for scope: the explicit action binds the displayed
revision. Unsaved local edits and pending turns concerning that scope disable
Authorize with a reason. Background refresh never opens a panel or overwrites edits.

**Materiality rule:** every changed canonical proposal field, selected-context
reference or relevant recorded answer invalidates effective scope authorization.
Do not ask a model to decide whether a change is material. Byte-identical saves
are no-ops (compare content/dependencies before assigning a new revision or source
record); display wrapping/theme changes do not affect the hash. This conservative
rule requires reauthorization for wording edits but avoids hidden scope changes.

Later execution will need a *separate* operator-approved execution plan referencing
the still-current scope hash, allowed effects/files, worker profiles, limits and
context. Scope permission grants none of these and reserves no task call. No such
execution-plan command is implemented in this increment. Retain both a service
command refusal and scheduler fence, including against forged legacy `admit_task`.

## Minimal result, record and IPC contract

### Provider result and validation

New profile `claude-proposal-v1` retains the proven text-stream CLI transport and
capability restrictions. Request one JSON text envelope in the owned prompt; do
**not** add a CLI structured-output tool, extraction call or new CLI flag:

```json
{"schema_version":2,"assistant_text":"Here is a scope to review.",
 "evidence_refs":["message:APPLICATION_SUPPLIED_ID"],
 "action":{"kind":"upsert_proposal","payload":{
   "base":null,
   "proposal":{"title":"Local sample weather page",
     "objective":"Display fixed sample weather locally",
     "deliverables":["One static page"],
     "in_scope":["Fixed sample data"],"out_of_scope":["Live weather API"],
     "constraints":["No credentials or network dependency"],
     "acceptance_criteria":["Named sample cities display their fixed temperatures"],
     "unresolved_decisions":["Celsius only or a unit toggle?"]}}}}
```

`action` is null for ordinary replies, or exactly one `ask_question` (prompt,
rationale, up to eight options) or `upsert_proposal`. Existing-question IDs and
proposal target come from frozen application context, never model-created IDs.
For revision, `base` must echo `{task_id, revision, sha256}` from that context;
null creates only when the frozen target is new. Questions inherit that target
and decision revision. An action cannot grant permissions, resolve a question,
authorize, execute, set budgets, or select another task/project. Evidence references
must resolve to the supplied packet's allowlist, never arbitrary paths/URLs.

Strictly reject unknown/duplicate JSON fields, multiple values/actions, mismatched
base/references and excessive nesting/size. No Markdown-fence stripping, prose
heuristics, or repair invocation. Limits: envelope 64 KiB; assistant text 32 KiB;
proposal canonical body 16 KiB; title 120 characters; objective 4 KiB; lists at
most 16 entries of 1 KiB, unresolved choices at most eight. Required fields must
exist; a draft may have empty lists, but authorization requires nonempty objective,
deliverables, in-scope and acceptance criteria. Arrays/strings are inert content,
including any code; no generated instructions are executed.

A bounded incremental JSON decoder exposes only the top-level `assistant_text`
string provisionally; split escapes/Unicode and incomplete tokens are tested.
Raw action JSON/thinking stays in diagnostics. Partial JSON never creates a task.
Successful CLI completion plus strict complete-envelope validation is required.
Malformed/failed completion preserves raw bytes, receipt and debit, marks output
invalid and stops new trial dispatch. A valid but stale action is a completed call
with an unapplied suggestion, **not** ambiguous execution or invalid output.

The existing proof covers the underlying client restrictions, not this new response
format's reliability. Check the changed prompt/envelope through the existing
credential-free local mock fixture before native admission. No tools are added.

### Durable records and concurrency

Use application schema **6**, `[conversation] schema_version = 3`, versioned
profile/allowance and result-v2; keep schema-5 `claude-chat-v1` behavior unchanged.
New task statuses are existing `proposed` and added `scope-authorized`; draft
completeness and needs-revision are derived head fields. A changed head/decision
returns `scope-authorized` to `proposed` atomically. Neither state gets a run ID.
The new profile freezes its separately approved conversation ceiling once; task
and consultation limits stay zero. Service restart never replenishes an allowance.
Within the existing writer/SQLite database add only:

- `task_proposal_revisions` (record v1): immutable canonical body, project/task,
  monotonic revision, parent hash, selected-context/decision references, source
  turn+packet+result or human command, author kind and change rationale; unique
  `(task_id, revision)` and source-action identity. Hash the complete canonical
  record except its hash using the existing codec. Preserve original provider bytes.
- `task_proposal_heads`: current revision pointer and dependency/decision revision;
  no second copy of authoritative content. Existing `session_tasks` projects title,
  version/status and keeps `run_id = null`. Existing `session_proposals` stays the
  legacy schema-3/4 record; readers resolve the appropriate version explicitly.
- `task_scope_decisions` (record v1): append-only human command, project/task,
  proposal revision/hash, acknowledged unresolved-list hash and decision snapshot.
  Supersession is an appended event/reference; never delete or rewrite an approval.
  These records are not `session_admissions` and cannot promote a task to ready.

Reuse conversation questions/messages/actions/commands/budgets, adding versioned
proposal-target and dependency fields where needed. An answer's exact bytes/hash,
source question and command remain authoritative; model summaries cannot replace
or modify answers. Saving an answer does not queue/charge a turn in this profile.
Continue explicitly links a unique answer/continuation ID, so reconnect cannot
schedule it twice. Without allowance, questions, local edits, inspection and scope
authorization still work; continuation explains exhaustion.

Freeze target/head hash, question/decision revision, coordinator generation and
user-message watermark in each packet. At result application recheck all relevant
dependencies in the writer transaction. New user messages after the watermark,
local saved edits/answers, authorization or changed target/binding make the action
stale; receipt and assistant reply remain, but no current proposal/question is
changed. Unrelated task events/output progress do not stale it. New-task creation
is unique to the accepted source turn/action; replay cannot create a second ID.
A model suggestion received while the local editor has an unsaved draft may save
a new head, but the editor retains its old base and shows a conflict on save; it
never silently merges fields or discards the draft.
Queued turns use the latest eligible saved context when *first* frozen; never
rewrite a previously frozen packet to make it pass after a crash.

Human mutations carry expected task/head revision/hash and decision revision.
Two clients editing/authorizing the same base serialize: one wins; the other gets
`stale_proposal` and keeps its draft. Never silently rebase or reauthorize. Replay
the same command ID/body returns its stored original outcome before current-state
checks, annotated in the refreshed UI if now superseded. A lost reply is resolved
with `command_get`, not a new command. Save revision/decision/projection/event/
command outcome in one short transaction; no external I/O inside it.

Receipt-validated result application, terminal turn status, assistant message,
action disposition and optional proposal/question commit atomically. A crash
before commit adopts the same saved receipt once; after commit replays the same
outcome. No second proposal, lost human decision or extra charge. Existing host
claim/permit/generation and result hashes remain required; missing processes or
files alone cannot prove nonexecution. Keep cancelled/failed/uncertain calls charged
and blocked. `/exit` adds no summary call; deterministic checkpoints include
proposal/decision references. Resume reconciles prior receipts before dispatch;
late results after local edits receive the same staleness checks.

### Context, versions and routing

Mandatory context: current user message, explicitly selected proposal revision,
its authorization status, linked questions and human decisions with provenance,
provider binding and budgets labelled **as of packet reservation**, including the
current held call. Optional: at most six preceding messages through the watermark.
Packet ceiling stays 96 KiB; omit oldest optional messages first. Never truncate a
mandatory proposal/decision silently; refuse before reservation with an actionable
context-too-large outcome. Include only current proposal dependencies, not all
tasks/history. Frozen packet/hash is immutable. The registry, unrelated projects,
repository scans and provider history remain excluded. Existing explicitly granted
imports can appear only as selected references/content within their recorded scope;
no new import or repository-access capability is added.

IPC remains major 1, adds **minor 4 / `task_scope_v1`**, required for schema 6.
Reuse `task.get`, `task.result`, `conversation.read`, `snapshot`, event/replay and
`command` envelopes; no new top-level request kind. Add command actions
`save_proposal_revision` and `authorize_scope`; enable the target-aware versions
of `submit_message`, `answer_conversation` and existing lifecycle/cancel commands.
A unique `submit_message` continuation payload references the saved answer; it is
not a third new command. Reads expose proposal/head/decision/dependencies and
`execution_available:false`. Existing `task.plan`/`admit_task` execution paths and
worker/consultation commands refuse in schema 6; `/plan` uses `task.get` instead.
Update the checked-in schema capability/action inventory and consistency test.
Do not silently broaden schema-5 allowed actions or decode v2 as old v1 output.

New isolated projects alone use schema 6; installations remain side by side.
Opening/repeating init on schema 3/4/5 or LangGraph projects retains their bound
release/semantics. No migration tool or operator cutover is part of this increment.
Old clients/releases refuse incompatible state before writes. Add schema-6 release
support explicitly; never just widen the existing `conversation_chat()` predicate
and accidentally enable mock workflow promotion. V5's trial, authorization,
exhausted 3/3 allowance and all original records/bindings remain untouched.

## Bounded implementation checkpoints (after approval)

At each gate: record code/state checkpoint, preserve failed evidence, correct
explained differences, run focused checks, commit and compact docs. Continue
through approved checkpoints without routine permission requests. Stop dependent
work for an integrity failure or unsupported contract; never lower a gate to pass.

| Checkpoint | Scope and focused passing gate | Rollback boundary |
| --- | --- | --- |
| 1 — Contract and authority | Result-v2/profile/schema-6 and immutable revisions/decisions through existing writer. Table-driven invalid/oversized/duplicate/forged-action cases; identical/changed save; exact-hash authorization; unresolved acknowledgement; answer at exhausted budget; two-client races; replay before/after supersession; all worker admission routes refuse | New disposable schema-6 state only; pre-init code rollback is safe. Once accepted, retain readable records and schema-6 compatible binary; never point v5 at it |
| 2 — Hosted conversation and recovery | Extend proven Claude adapter/stream decoder and target-aware packets; credential-free fixture with unchanged exclusions; completion→transaction fault injection, receipt adoption once, delayed stale response, overlapping messages, detach/exit/service restart, cancellation/uncertainty, budget reservation/context overflow and two-project isolation | Keep raw packet/result/receipts and old profile handlers; forward-repair accepted state. No saved-output reinterpretation under changed schema/prompt |
| 3 — Integrated TUI | Reuse scope/question/editor/drawer; visible revision/edit/authorize/status actions; no task per prose message. Test empty/nonempty Enter, literal paste, scrolling bounds, unchanged vs changed head, unsaved drafts across refresh/reconnect, stale command and explicit authorization with zero calls/dispatch | Frontend rollback only to a minor-4 compatible build; old frontend must refuse, not mislabel scope as execution |
| 4 — Installed mock acceptance | One stable isolated launcher; actual init→start→F3→discussion→question/answer→proposal→edit→authorization→detach/resume→revision invalidation→reauthorization. Script real CLI stream format, clearly mock; no phrase matching in production. Check receipts/counts, command replay, two projects, schema-5 routing, terminal restore; source checkout unavailable. Status/control <1 s, SQLite busy refusal <500 ms, existing output bounds | Preserve paired candidate and fixtures. Keep earlier releases/operator projects unchanged. After new accepted work use compatible forward repair, never snapshot restoration |

Reuse tests in `conversation_store.rs`, `session_service.rs`, production `view.rs`,
[`claude_conversation_installed.py`](../../../tests/claude_conversation_installed.py),
[`claude_conversation_boundary.py`](../../../tests/claude_conversation_boundary.py) and
[`session_schema_consistency.py`](../../../tests/session_schema_consistency.py), extending
only the changed gates. Check schema-3/4/5 behavior through isolated fixtures, not
by running historical/operator projects. No broad acceptance rerun by default.

## Separately proposed live check and decisions

After mock installed gates, request a new named **`claude-scope-01`** project and
exact paired release/CLI hashes. Recommend the same Claude Code **2.1.296**,
**claude-sonnet-5-5**, **medium**, existing saved login and capability restrictions.
New profile ceiling: **six coordinator invocations total**, including failures,
interruptions or uncertainty; task/consultation limits zero. No retries, automatic
repair, fallback, substitution or allowance renewal. This proposal grants no calls.
CLI invocations are not HTTP-request or dollar ceilings. Revalidate pinned-client
availability; never silently select another model/authentication route.

Operator scenario: discuss a fixed-sample weather page; answer a clarification and
request a scope (about two calls); request a material revision (third); say “looks
good” and verify it authorizes nothing (fourth); explicitly authorize in the panel,
detach/resume, then request another material change (fifth) and verify the old
authorization is historical and a new one is required. Sixth is available only for
an operator-submitted clarification if needed, not automated retry. Local edits,
reading and authorization cost no calls. If the flow cannot complete within the
ceiling, report the incomplete result rather than enlarge it.

Success: real conversation plus at least two saved proposal revisions; exact human
scope decision durable across resume; later change invalidates effective permission;
receipts/counts match at most six calls; **zero workers, consultation, execution
admissions or generated files**. App text clearly says scope-only. An uncertain/
failed/malformed call, profile drift, unauthorized capability or limit violation
stops further native dispatch with retained evidence. Operator observations and
read-only audit remain distinct; no working weather application is claimed.

Substantive recommendations for review:

- Permit scope authorization with explicitly acknowledged unresolved choices;
  preserve them for later execution planning. Requiring all choices settled now
  is stricter but needlessly blocks agreeing on a bounded scope that cannot run yet.
- Use conservative invalidation for every saved content/dependency change rather
  than a model's materiality judgment; this costs an extra explicit action for
  small wording edits and keeps the authorization rule auditable.
- Use the existing tool-free text transport with strict JSON validation. It may
  reject malformed model responses; a new CLI structured-output facility could
  change tool/retry behavior and would need a separate boundary proof. If this
  format proves unreliable, retain evidence and stop instead of adding repair calls.

No further product decision is required to make the plan implementable. Review
these recommendations and authorize implementation separately. The exact first
task is checkpoint 1's schema/result and writer-transaction authority slice;
no application code, dependencies, runtime resources or trial state changed here.
