> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Context continuity and agent-raised decisions — scoped proposal

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

**Historical proposal, subsequently approved for implementation and one trial.**
See [the execution report](context-continuity-report.md) for observed results and
the stopped trial. Milestone 3 remains unapproved. The sections below preserve
the original proposed scope, not a new authorization.

## Recommendation

Extend the existing packet builder and durable decision gate. Add one small
context resolver, an explicit context selection file, and a versioned question
contract. Keep the current SQLite records, LangGraph graph, Herdr runner and
Claude/Codex adapters. No separate memory service or additional model call is
needed to select context.

### What actually exists

Source inspected in this standalone checkout; there is no Git commit to cite.
Function references below identify the implementation inspected.

| Data | Current use | Across new runs |
|---|---|---|
| Frozen brief, rules, configuration and skills in `inputs/envelope.json` / `app_runs.envelope` | Inputs for every stage in that run | New creation rereads selected files; no prior-run import |
| Checkpoint decisions, findings, current artifact and review delta | Automatically included in later packets in the same run | `workflow.initial()` resets them to empty |
| `app_requests` / `app_responses` and exported `decision-*.json/.md` | Durable controller requests, answers and approval; resume consumes saved answers | Retained, but not selected for a new run |
| Versioned artifacts, result JSON, manifests | Read and hash-checked for handoffs within the run | Inspectable; no cross-run resolver |
| Operation reservation, packet, receipt, process identity and counter | Reconcile execution and prevent redispatch within the run | Execution evidence, not reusable task context |
| Transcripts, native events, outcome reports and HANDOFF | Operator inspection / development continuity | Not automatically included in worker packets |

Evidence: [cli.py](../../../src/agent_workflow/cli.py) `create()`;
[workflow.py](../../../src/agent_workflow/workflow.py) `initial()`, `worker()`,
`choice_gate()`, `drive()`;
[artifacts.py](../../../src/agent_workflow/artifacts.py) `packet()`, `content()`;
[records.py](../../../src/agent_workflow/records.py) `request()`, `save_response()`.

**Saved packet verification:** `m2-cli-01` is approved, with five counters of 1.
Draft and critique-1 have no accepted decisions. Revision-1, critique-2 and
revision-2 include `D-backup-policy: Offline only`. Critique-2 includes a
6,099-character delta. All five results have `questions: []`. Final findings
C1-001, C1-002 and C2-001 are marked addressed; this trial has **no recorded open
findings to inherit**. An author disposition is not an independent verification.
Revision-2 was human-approved without a third critique.

The exact barriers to genuine questions are:

- [milestone-2.toml](../../../config/milestone-2.toml) defines the backup-policy question;
  `cli.create()` requires that decision and hardcodes the sole authorized run ID.
- `artifacts.packet()` supplies `planned_decision`; `workflow.worker()` replaces
  questions after critique-1 with the configured question and forces revision-1.
- [real.py](../../../src/agent_workflow/adapters/real.py) `prompt()` tells workers to keep
  questions empty, as do [trial rules](../../../examples/milestone-2-rules.md).
- [herdr_backend.py](../../../src/agent_workflow/herdr_backend.py) `validate_result()`
  rejects every nonempty native-worker question list, although the native JSON
  schema already describes questions. Fake-worker questions do use the gate.

Thus the real trial verified a controller-scheduled choice, not agent discovery
of a choice. Existing skills invite substantive questions, but the trial-specific
prompt and validator override that capability.

## 1. Select reusable context without a second authority

**Proposed:** a project-local selection file, for example
`config/contexts/example-notes.toml`, used by a future `--context FILE` option.
It identifies `project_id`, task scope, canonical instruction/document paths,
and explicit source references. Initial selection:

- Decision: `(m2-cli-01, D-backup-policy)`, scoped to this release's backup policy.
- Artifact: `(m2-cli-01, revision-2)`, with approval reference `(m2-cli-01, A1)`.
- Findings: explicit IDs or `open findings from selected source run`; currently none.

Each entry carries a short relevance reason and topic key, e.g.
`release.backup-policy`. This is a **reference index**, not rewritten decision
prose. No searchable database index is needed at this size. The operator selects
a task context once; subsequent stages receive it automatically. Automatic
semantic retrieval and relevance scoring are deferred.

Authority and resolution rules:

1. Read accepted answers from `app_responses`, joining `app_requests` for the
   question, options and consequences. Export files are inspection projections;
   verify they agree if present. Do not independently edit them.
2. Read artifact bytes through the existing path/hash validation. Verify the
   source A1 response and terminal outcome approve those bytes. An inherited
   approval is evidence about the source artifact, never approval of the new run.
   Revision-2 still says “Not approved”: retain its immutable bytes and supply
   the actual approval as controller metadata rather than silently editing it.
3. Resolve finding status from the source terminal checkpoint and supporting
   result/disposition artifacts. Preserve source IDs, evidence and status origin;
   qualify IDs by source run so another C1-001 cannot collide. The new run may
   record its own disposition without altering the source finding.
4. Read canonical repository instructions through explicit configured paths;
   follow CLAUDE.md's pointer to AGENTS.md once, without duplicating the rules.
   Include only selected relevant documents/sections. Handoff/report prose may
   explain history but cannot supersede recorded human decisions. Never ingest
   transcripts or native session history as project policy.

At creation, resolve and freeze a `context_snapshot` in the existing envelope:
`schema_version, project_id, scope, selection_digest, entries[]`. Each entry has
`kind, qualified_id, source_run/path, source_record_digest, artifact_sha256
(if applicable), authority_ref, topic/scope, relevance_reason, selected content`.
Packets include the selected authoritative content and provenance; repeated
packet copies are immutable execution inputs, not new editable sources of truth.
Keep imported decisions separate from this run's response rows; do not fabricate
new human answers or overwrite their original timestamps/decision revisions.
Local `decision_revision` counts this run's responses, with the context snapshot
digest separately bound into packets, decision requests and final approval.

**Staleness:** check source hashes, approval, scope and selected canonical-file
hashes at creation, resume and before each new dispatch. Pin the selection digest;
do not silently reread an edited profile into an active run. Missing, altered,
conflicting or explicitly superseded sources stop continuation with a specific
context error. Completed packets/receipts remain unchanged. Source staleness
requires explicit context resolution or a new run, not blind replay.

New envelopes carry project identity and source lineage. Known later approved
runs in that project can be checked for explicit supersession references using
existing run/request/response records; no parallel policy registry is required.
Legacy `m2-cli-01` is associated by the selection reference, without migration.
Hashes detect changes, **not semantic obsolescence**. Undocumented policy changes
or unrelated unselected runs cannot be discovered reliably; new contradictory
evidence must be surfaced as a reconsideration request.

## 2. Turn a worker question into a durable gate

Use a new workflow/output-contract version only for future runs. Preserve the
legacy graph and packet validation for existing runs; do not reinterpret their
saved receipts or reconstruct their packets under new rules.

Extend the existing question shape with `topic_key, rationale, evidence_refs,
related_decision_refs, reconsideration_reason`. Keep question text, 2–3 options,
recommendation and consequences. Limit this extension to **one new substantive
blocking question per run**. The worker proposes a question, never an answer,
authorization, dependent stage, or execution instruction.

Controller sequence:

1. Validate structure, bounded sizes, unique topic, and references to supplied
   context/artifacts. A well-formed question is a successful worker result;
   publish the provisional artifact and receipt, then checkpoint the completed
   stage before entering the human gate. No mid-process interactive worker pause.
2. Derive a stable request ID from run + operation + question ID. Persist in the
   existing `app_requests`, binding the source result/packet digest, artifact,
   context digest, local decision revision and controller-chosen continuation.
   Replay of the same request is idempotent; changed content under that ID fails.
3. Reuse `prepare_choice → choice_gate → interrupt`. Show the question, rationale,
   consequences, evidence links and inherited constraints in the controller.
   Accept an explicit option (option number may map to its label); this small
   extension does not interpret arbitrary conversation as an authorization.
4. Save the operator answer through existing response records/exports before
   graph continuation. A fresh controller consumes it once. Subsequent packets
   contain the inherited policy, new answer, findings and appropriate delta.
   Worker calls remain completed; never relaunch the questioning stage to get
   an answer into its session.

**Continuation matters:** a question-bearing critique must not skip revision
merely because `needs_revision=false`. Track that the current artifact predates
an accepted answer and require the next available author revision to apply it.
After a draft question, critique-1 can evaluate the provisional draft against
the answer, followed by mandatory revision-1. A later question uses remaining
revision slots. If no slot remains, keep a durable blocked outcome explaining
that a separately authorized follow-up is needed; do not approve stale text or
invent a sixth call. More than one new question also stops explicitly without
discarding the output or automatically retrying.

**Inherited decisions:** exact repeats of an accepted topic are not presented
again; return that accepted answer in subsequent packets. A proposed change must
reference the prior decision plus concrete new evidence and a reconsideration
reason. The controller can validate references and detect exact repeats, but
cannot prove semantic novelty or spot every paraphrased duplicate. It must not
silently replace policy: flag an ambiguous conflict for the operator. A deliberate
reconsideration is a new request whose human response explicitly supersedes the
old decision for the stated scope; the original record remains immutable.

This reuses existing checkpoint boundaries and execution reconciliation. It does
not establish exactly-once provider execution or resolve milestone-3 hard-kill
windows. LangGraph persists transitions; receipts establish worker completion;
structured packets carry context; Herdr continues to expose actual workers.

## 3. One bounded follow-up acceptance trial

**Proposed, not authorized to run now:** `context-01`, same Herdr backend and
Claude author / Codex critic, explicit configured models/effort, at most two
critique rounds and five native invocations, with no retry after failure or
uncertainty. Inspect installed CLI compatibility again before a future launch.

Task: draft a small local manual-export interaction specification for Pocket
Notes. Inherit the accepted offline-only policy and approved revision-2. The
new brief requires exports to a user-selected local filename and says existing
backup files may already occupy it, but leaves collision handling undecided.
Do not put a question, options, preferred answer or stage-triggered gate in the
configuration. An agent must identify the missing product choice and emit the
structured question (e.g. preserve existing exports with unique filenames versus
allow replacement after confirmation). Both choices must honor the inherited
policy. The operator supplies the answer, then the workflow continues to final
human approval. Failure to raise a meaningful question fails this trial; do not
inject a fixture question afterward or retry to force success.

Acceptance criteria:

- First and later packets automatically include the source decision, approved
  artifact, scope and verifiable provenance. No transcript dump or manual context
  recopy; no new backup-policy vote without evidenced reconsideration.
- The request points to an actual completed worker result. No fixture/config
  supplies the runtime question. Its content identifies a genuinely unresolved
  collision policy, rather than routine implementation permission.
- At the question, dependent work stops and remains stopped after controller
  exit. Record the operator's answer without continuation, restart, and consume
  that same answer once; completed-stage counters and pane identities do not
  duplicate. This tests gate continuity, not milestone-3 in-flight fault injection.
- The next relevant author artifact applies the answer while preserving offline
  constraints. Critique receives the inherited and new decisions; second critique,
  if used, receives the actual delta. Final approval stays with the operator and
  binds artifact + decisions + selected context.
- Every stage counter is at most 1; at most five calls/two critiques, no automatic
  retry. Capture any model failure or uncertainty as a result, not a success.
- Offline fixtures cover open-finding import (the source trial has none), source
  mutation/supersession, cross-run ID collision, unchanged inherited decisions,
  invalid question references, duplicate requests, late questions with no budget,
  stale approval, and record-only answer/restart. Never mutate approved originals.
- Approved trial/demo artifacts, decisions and checkpoint values are unchanged.

## Exact implementation scope for the next approval

1. One `context.py` resolver plus a reference-only selection profile; extend
   frozen envelopes/packets and controller inspection to show selected provenance.
   No new database service or duplicated decision ledger.
2. Versioned changes in `workflow.py`, `artifacts.py`, `records.py`,
   `herdr_backend.validate_result`, `adapters/real.py` and controller presentation
   for genuine questions, inherited context, answer-driven revision and internal
   approval binding. Preserve version-1 behavior and completed runs.
3. Separate follow-up profile/brief/rules and a narrowly scoped authorization entry
   for the proposed new trial. Remove the planned-question injection only from
   the new workflow; do not broadly bypass `m2-cli-01`'s current authorization guard.
   Keep five calls, two reviews, one active run and existing worker permissions.
4. Offline tests, then the single bounded real acceptance trial **only if expressly
   authorized with implementation**; update outcomes/HANDOFF with actual evidence.

No milestones beyond this extension, tool permission expansion, native interactive
sessions, parallelism, semantic search, vector storage, automatic policy promotion,
Git/PR automation or broad memory platform. The main remaining uncertainty is
whether real workers consistently raise a useful novel question and honor inherited
scope; source inspection and schemas alone cannot establish that behavior.
