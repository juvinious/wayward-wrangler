# Rust orchestration assessment

**2026-10-12 · planning only · `refactor/rust-backend`**

Recommendation: retain a **small application-owned durable transition engine**
for this migration. Do not add an agent framework to replace LangGraph. Defer
Rig as a possible future coordinator component, independently of workflow and
worker execution. This recommendation follows the recovery/ownership evidence
below; the number of nodes in today's drafting workflow is not its justification.

The application will own more transition/checkpoint code, but already owns the
hard external-execution invariants. The two closest workflow candidates do not
currently remove that work and introduce persistence/replay integration gaps.
Future configurable workflows remain a requirement; avoid baking today's five
worker slots into the storage or execution interfaces.

## Evidence and scope

Application source inspected at `f8b342a6b14a9ecd6973c0b1a18de9e11a6da38a`.
The preserved r9 baseline remains unchanged; see [handoff](../../HANDOFF.md).
Public repositories were read as source, including relevant tests and manifests.
**No third-party code, tests, builds or model calls were executed.** No new
Wayward runtime verification occurred. Existing baseline acceptance is historical
evidence, not proof that any replacement works.

Labels: **D** = documented claim; **S** = source-inspected behavior;
**T-read** = relevant test code inspected, not run; **U** = unverified.
Versions below are manifest versions at the pinned commits, not a claim that
every feature has shipped on crates.io. Links pin source rather than moving HEAD.
Only Juncture and Smooth received a deep workflow-engine assessment. Rig is
assessed separately for coordinator use; the other three received a bounded screen.

## 1. What LangGraph actually supplies today

Direct dependencies are LangGraph 1.2.14, checkpoint 4.2.0 and SQLite saver
3.1.1 ([manifest](../../../pyproject.toml)). Application imports are concentrated in
[workflow.py](../../../src/agent_workflow/workflow.py).

| Responsibility | Current owner and boundary (S) |
|---|---|
| Graph topology and routing | `build`: six nodes (`worker`, two gate preparations, two gates, `blocked_gate`), conditional edges and END. Application functions determine the route and stage semantics. |
| Progress/checkpoints | LangGraph state snapshots, next-node position, pending writes, `get_state`; SQLite saver commits synchronously during `drive`. `thread_id` is run ID inside a project-specific database. |
| Suspend/replay | `interrupt` and `Command(resume=...)`; `drive` uses `graph.invoke(None, ...)` to continue checkpointed work. Node replay means application side effects must already be idempotent. Recursion limit 60 is an engine bound, not the authorized call budget. |
| Human authority | Application `prepare_choice/final`, `answer`, gate functions and `records.save_response` validate saved answers and exact artifact/decision/context binding. LangGraph receives an already recorded answer; it does not authorize it. |
| External execution | Application `herdr_backend`, `runner`, `ownership`, `routing`, `recovery`: intent, claim, permit, attempt reservation, verified process identity, receipts and explicit uncertainty. LangGraph neither launches nor durably owns a CLI child. |
| Policy and continuity | Application `records`, `native`, `context`, `context_access`, `findings`, `quality`: invocation/review bounds, frozen inputs, explicit context selection, grants, findings and append-only recovery. |
| Application session | `session_service`, `session_lifecycle`, `session_bootstrap`, `supervision`: queue, IPC/events, command deduplication, project ownership, Herdr/systemd lifecycle, exit/detach/resume. These are outside LangGraph. |

Relevant sources: [records](../../../src/agent_workflow/records.py),
[Herdr backend](../../../src/agent_workflow/herdr_backend.py),
[runner](../../../src/agent_workflow/runner.py),
[session service](../../../src/agent_workflow/session_service.py),
[lifecycle](../../../src/agent_workflow/session_lifecycle.py),
[context access](../../../src/agent_workflow/context_access.py).
Current graph checkpoints and application writes can commit separately. Replacing
LangGraph is an opportunity to reduce that split, not evidence that it is already
atomic. The fault-comparison gates must check every boundary.

## 2. Candidate screen

### Exact source, licensing and maintenance

| Candidate | Inspected commit / version / declared license | Maintenance evidence inspected |
|---|---|---|
| Juncture | [`0a36f67315ea4f8f1f8348ae3af338a1341d02b5`][j-root], 0.4.0, Apache-2.0 | Oct 8 release, conditional-routing fix and CI/toolchain fixes; embedded durability tests and CI definitions. Young repository (June 2026). |
| Smooth Operator Core | [`58ba8859313d361b42d21d6445c7ea378fa60822`][s-root], Rust core 1.14.2, MIT | Oct 7 release and tool-history/step-bound fixes; polyglot CI, workflow/store/HITL tests. Repository dates from May 2026. |
| Rig | [`9e7cb6000355b7ae1a89c913bf98b8a6f17661ee`][r-root], 0.44.0, MIT | Oct 8 agent restart/committed-history changes, including breaking changes; extensive run serialization tests. Repository dates from June 2024. |
| AutoAgents | [`6301004371aa3fe274a309e493cdd3bfb07592db`][a-root], 0.4.0, MIT OR Apache-2.0 | Aug 26 dependency/security and history fixes; Aug 18 runtime error propagation; actor/runtime tests and CI. Repository dates from February 2025. |
| Polaris | [`5ff84646db63589d58bf87956976d749b8c754de`][p-root], 0.6.0, Apache-2.0 | Aug 26 version correction, Aug 25 graph/context work; graph/session tests and CI. Repository dates from February 2026. |
| OpenMirai | [`1610141d78db618e8bf3346281f2bbd73ac19895`][o-root], engine 0.7.0, Apache-2.0 | Aug 25 documentation merge; Aug 18 live tmux adoption; runner/resume and tmux tests, CI. Repository dates from May 2026. |
| Application-owned | Wayward source above; replacement not implemented; no new framework license (current project manifests have no license declaration) | Existing recovery/approval/context tests and r9 evidence are reusable contracts, not Rust-engine test results. All new engine maintenance is ours. |

All six repositories were unarchived when inspected. Recent commits and test
presence establish activity, not production reliability, coverage completeness
or a support guarantee. No CI-green claim is made. Declared licenses come from
the pinned manifests/license files; transitive license review remains a later
dependency-lock gate.

**Polaris identification confirmed:** the published
[`polaris-ai` crate metadata](https://crates.io/api/v1/crates/polaris-ai) reports
`https://github.com/RadiantAILabs/polaris`, version 0.6.0, Apache-2.0 and Rust
1.97.0. The repository's root manifest independently agrees. This is the
candidate assessed, not another project named Polaris.

### Fit and cost of reaching Wayward's outcome

| Candidate | Useful capability / embedding | Remaining work and disposition |
|---|---|---|
| Juncture | Typed async nodes, conditional graphs, interrupts, checkpoint trait/SQLite, state inspection/migration hooks. Core has no default features; provider adapters need not be used. Rust 1.88/edition 2024; Tokio/Serde, optional SQLx persistence. | Closest graph API fit. Application still owns processes and authority. Resume/persistence gaps below require a workaround or upstream correction. **Do not adopt at inspected revision.** |
| Smooth | `Workflow<S>` async nodes and nested/conditional graphs, provider/tool layer, several checkpoint stores. Rust core defaults empty but requires HTTP/WebSocket/Tokio dependencies; SQLite feature adds rusqlite 0.32. | Graph runner lacks durable cursor integration; checkpoint/HITL APIs primarily target its agent loop. Separate Temporal backend adds a service and another execution model. **Reject as migration workflow engine.** |
| Rig | Embeddable model/tool APIs and serializable agent stepper; no TUI replacement needed. Minimal `rig-agent` with defaults off can avoid transport/runtime; facade defaults are broader. Rust 1.95. | Potential future coordinator value; not our project workflow/supervisor. Requires explicit CLI bridge and application effect reconciliation. **Defer**, see §4. |
| AutoAgents | Async direct/actor agents, provider traits, typed tools, memory import/export. Core uses Tokio/ractor; WASM/CodeAct engines optional; Python bindings are optional, not a Rust runtime requirement. | Inspected runtime owns in-memory actors; memory export is conversation data, not a durable suspended workflow. `examples/pipeline` is provider retry/cache composition. No suitable durable workflow/approval store demonstrated. Adding one leaves most migration work plus an agent runtime. **Defer.** [Source/test entry points][a-core] |
| Polaris | Composable system/graph runtime, scoped contexts, session resource snapshots and resumption. Can embed; HTTP is optional for sessions. Rust 1.97, framework system/resource/plugin model. | `resume_session` restores registered resources and rebuilds context; inspected graph execution does not demonstrate persisted in-flight execution continuation. File store uses temporary-file rename but no explicit fsync. Session persistence is useful, but mapping app authority into its resource runtime is substantial work. **Defer.** [Session API][p-api], [store][p-store], [graph executor][p-graph] |
| OpenMirai | Actual external coding CLI integration exists: tmux-backed session manager, plus embeddable graph runner with cursor-based `resume`. Optional HTTP server, mandatory Tokio/reqwest/rusqlite 0.35; Rust 1.80. | Existing CLI support must not be dismissed as API-only. Its `SessionBackend` uses send-text/capture/kill terminal operations, unlike our structured runner/Herdr contract. Default graph retry is three retries; checkpoint callback errors are logged and execution continues. Resume is not exposed by its CLI/HTTP (D, consistent with source screen). Herdr adapter, failure semantics and app ledger integration remain ours. **Reject for this migration.** [Runner][o-runner], [backend][o-backend], [tests][o-tests] |

None requires replacing Ratatui just to embed a library. None of the inspected
integrations supplies Wayward's existing Herdr claim/permit/receipt contract.
No additional workflow server is necessary for the proposed custom engine,
Juncture core or the other local core libraries; Smooth's **optional Temporal
route** requires a separate service, which can be self-hosted. Optional vector
stores, built-in unrestricted tools, provider SDKs and
memory discovery should not be enabled to obtain basic routing. Different
SQLite crate versions would need dependency alignment if combined; no Cargo
resolution/build was performed to verify compatibility.

Screening tests read include AutoAgents' actor message/error-flow cases in
[`actor_integration_tests.rs`][a-tests], Polaris'
[`run_oneshot_preserved.rs`][p-tests] (read-only session refusal, save and resume),
and OpenMirai's `resume_continues_from_state` in the runner tests. These do not
prove safe recovery of Wayward CLI calls or exact-artifact approval semantics.

## 3. Deep workflow comparison

### Juncture: useful graph machinery, insufficient demonstrated continuation

**S:** [`CompiledGraph`][j-compiled] exposes invoke, interrupt resume, snapshot
inspection/update and streams. Async nodes can call a Wayward operation handle;
they need not be model agents. Retry is configured per node; leaving retry maps
empty avoids that retry wrapper. Application budgets must remain independent of
engine recursion/token counters.

Three material findings:

1. `resume` and `resume_stream` require an **Interrupt** checkpoint. Normal
   invocation constructs a new loop; the checkpoint read there injects a previous
   return value, not a scheduling cursor. Interrupt resume deserializes state,
   then calls `PregelLoop::with_error_handlers`, which initializes step zero,
   fresh channel trackers and entry-point pending tasks. It does not hydrate the
   saved pending-task cursor there. **Source implication:** prior nodes can be
   re-entered; arbitrary post-superstep crash continuation is not established by
   these public paths. A dispatch ledger would still prevent new calls, but
   non-idempotent reducers and human-gate replay also need proof. This is not a
   runtime-observed failure. [Constructor/execution source][j-loop]
2. Sync durability awaits checkpoint writes and propagates storage failures,
   but interrupt-state serialization failure logs a warning and returns success.
   That exception is unsuitable for an authoritative human gate. Built-in budget
   trackers are newly constructed on resume; they do not replace our durable
   attempt ledger. [Loop][j-loop], [budget][j-budget]
3. The [`CheckpointSaver` trait][j-checkpoint] separates `put` and `put_writes`;
   it does not accept our application transaction. The [SQLite saver][j-sqlite]
   has its own transactions/schema (including `checkpoints`, colliding with the
   legacy table name). We would need an application-specific saver and explicit
   reconciliation, or separate engine storage with more consistency boundaries.

**T-read:** `test_resume_validates_interrupt_source`, interrupt-value tests,
`test_sync_interrupt_checkpoint_write_failure_propagates`, async/exit durability
tests and SQLite interrupt/pending-write round trips. These test real pieces;
they do not establish Wayward's multi-node fresh-process/no-duplicate contract.
**Required change/workaround:** safe public continuation or guarded replay,
fail-closed checkpoint publication, and an app-ledger-aware saver. Maintaining
that integration/fork would offset much of the graph code saved.

### Smooth Operator Core: durable agent messages are not durable graph progress

**S:** [`Workflow::run`][s-workflow] starts from `entry` on every call and tracks
the current node/step in local variables. It has no checkpoint store or persisted
cursor parameter. At `max_steps` it returns `Ok(state)` with a warning; Wayward
must instead distinguish budget exhaustion from successful completion.

[`Checkpoint`][s-store] persists agent ID, conversation, iteration and
metadata. [`Agent::resume_or_new`][s-checkpoint] restores the conversation;
`maybe_checkpoint` logs a save error and continues. Neither provides a resumed `Workflow<S>`
instruction pointer. [`ConfirmationHook`][s-human] exchanges requests/answers
over in-memory channels with a timeout; a process exit loses that waiting
interaction unless the host supplies a separate durable gate.

**T-read:** `test_max_steps_prevents_infinite_loop`, checkpoint-store tests and
`confirmation_hook_blocks_matching_pattern` / `timeout_on_no_response` cover
their stated local contracts. They are not crash-resume approval tests.
The [Temporal crate][s-temporal] addresses durable orchestration through a
different backend; adopting a Temporal service does not remove external CLI
ambiguity and exceeds the local migration's needs.

**Required work:** durable graph cursor/store integration and fail-closed errors,
plus our question, budget, process and approval semantics. A wrapper doing all
of that would largely be the proposed custom engine, with extra dependencies.

### Requirements that determine the choice

| Requirement | Juncture | Smooth core | Proposed application engine |
|---|---|---|---|
| Variation: branch, bounded review, durable wait | Graph vocabulary available; restart caveat above | Graph vocabulary available; persistence must be added | Typed policy module + versioned workflow reference; implement current workflow first |
| Crash checkpoint/record consistency | Custom saver/reconciliation needed | New workflow persistence integration needed | One project SQL transaction for transition, gate consumption, applied result and event |
| Human questions / exact approval / retained receipts | App gate/records required around interrupts | App durable gate required around channel hooks | Reuse existing authority; cursor references it, never duplicates approval |
| CLI processes / Herdr / exit / cancellation | Async node/timeouts are not process ownership | Async node/hook is not process ownership | Port existing supervisor/runner and explicit deadlines |
| Idempotency, generation fencing, unknown launch | App execution ledger required | App execution ledger required | Existing invariants retained and fault-tested |
| Retry and budgets across restart | Disable node retries; persist app budgets | Bypass agent retries; persist app budgets | Reconcile same operation; no automatic ambiguous retry; reserved attempts persist |
| Isolation, releases, rollback | App boundaries plus framework state version | App boundaries plus agent/workflow state versions | Project-local schema and bindings; unknown versions refuse mutation |
| Offline faults / maintenance | Good unit-test surfaces; upstream fixes/integration tests owed | Mocks/stores available; durable scheduler largely ours | All transition/recovery tests owed by us; existing behavior fixtures reduce specification work |

## 4. Rig's separate coordinator role

Do not evaluate Rig only as an HTTP completion SDK. Its current
[`AgentRun` protocol][r-readme] is a serializable, non-I/O stepper returning
model/tool requests, with history, tool validation and turn accounting. This
could reduce future coordinator-loop work without owning the TUI or project
supervisor. Minimal features avoid an obligatory provider transport. Native
CLI adapters would still require our own invocation/response bridge; no tested
drop-in Claude/Codex CLI adapter was established here.

**S/T-read:** [run source and tests][r-tests] include durable human tool-approval
serialization, exhausted-budget preservation and mid-run round trips. Crucially,
`run_persisted_mid_model_call_resumes_by_reissuing_the_call` and its stream
counterpart explicitly expect the same call to be offered again after restart.
That is a step request, **not authorization to redispatch**: our host must match
it to a durable operation and return receipt/wait/uncertain. High-level bus
future cancellation is also not our external-process shutdown contract.

Rig documents same-version resume, not cross-version persistence compatibility;
serialized runs include conversation and raw provider responses. Making that
our canonical task history would introduce storage/version coupling and context
growth. **Defer until conversational coordination is separately scoped.** If
adopted then, Rig state is versioned coordinator-internal state; application
decisions, tool authorization, project context and operation records remain
authoritative. No provider switching, fallback or new tool access in migration.

## 5. Integration and authoritative records

Keep four boundaries, whichever library is used later:

| Authority | Owned records and consistency rule |
|---|---|
| Application project service | Task/queue versions, accepted commands/outcomes/events, workflow cursor, immutable definition reference and effect-application keys. One fenced writer per project; no framework independently schedules effects. |
| Human/authorization ledger | Frozen scope/models/limits, questions, answers, decision revision and exact candidate/context approval. Cursor consumes saved answers transactionally; repeated commands return their original outcome. Model text cannot approve. |
| Runner + application reconciliation | Runner publishes immutable claim/dispatch/child/output/receipt evidence. Service validates evidence and appends application/recovery records. A missing receipt is not a negative execution proof. |
| Context/artifact records | Immutable bytes/hashes/provenance, finding originals and appended assessments; explicit task selection and directed read grants. Selected context is frozen; the global location registry never enters packets. |
| Herdr / UI | Herdr provides verified project-owned terminals; UI presents projections and sends commands. Neither terminal text nor a library session becomes workflow authority. |

Proposed transaction/effect sequence:

1. Validate command ID, project, task revision, service generation, scope and
   budget. Persist workflow/effect reservation and event before external action.
2. Preserve current launch-intent → verified claim → permit → dispatch ordering.
   Once an attempt may have launched, absent output, timeout or engine replay
   cannot allocate a replacement. Reconcile verified identities/results or stop
   uncertain with evidence and an operator action.
3. Fsync/atomically publish immutable output/receipt files. In one SQL transaction,
   validate/apply the result once, update cursor/findings/gate/budget projection,
   and append event. A crash between file and SQL publication reuses those files.
   Conflicting hashes refuse application. No fictional filesystem+SQL atomicity.
4. Commit answers before acknowledging; consume them and advance once. Approval
   checks candidate bytes, context and decision revision in the transaction.
   Exit/drain/resume generations fence stale dispatch and old shutdown. Detach
   only changes UI attachment. Known-running/stalled and uncertain remain distinct.

Application-owned engine costs: designing/versioning transition records, writing
fault permutations and diagnostic projections, and maintaining policy evolution.
Avoid a general graph framework: separate the current drafting policy from the
durable step/effect store; pin `workflow_id`, version and definition/config hash
per run. Additional bounded workflow policies can later reuse those primitives.
Do not add a workflow DSL, arbitrary plugins or parallel scheduling now. Revisit
a library when an approved use case actually needs dynamic/nested graphs or
parallel joins and a candidate passes the same effect-boundary contract.

Project isolation includes locks, budgets, recovery and release bindings, even
for identical run IDs. Cross-project material is read-only, explicitly granted,
bounded and nontransitive; revocation blocks future reads/dispatch but preserves
past packet provenance. Framework memory/tool defaults must not bypass this.

## 6. Historical compatibility, gates and next action

No candidate's API resemblance or MessagePack/JSON support demonstrates Python
LangGraph checkpoint compatibility. Preserve original checkpoint blobs, artifacts,
receipts, decisions, approvals and provenance. Unfinished legacy runs remain on
their bound legacy release. Rust reads application-owned history without
decoding opaque continuation state; upgrades require a quiescent project and
versioned state/binding changes. No transparent conversion or dual writers.

Coexistence ends **per project**: complete legacy work there or obtain explicit
archival disposition, verify no execution is outstanding, then upgrade for new
Rust runs. Rust-backed projects have no Python fallback. Retain legacy releases
only while explicit project references require them; retirement checks references
and unresolved execution rather than deleting evidence. Global removal of every
legacy installation is not a prerequisite for a Python-free Rust release.

**Recommendation uncertainty:** no Rust replacement has yet demonstrated parity;
custom-engine maintenance may exceed estimates. The first task remains the
read-only record reader/comparison contract, which is useful under either choice.
The later stage-4 gate must demonstrate durable wait/replay, no double result or
answer application, call/review limits, invalid versus incomplete versus uncertain
outcomes, exact approval, unknown-state refusal and publication/crash recovery.
Stages 3/5/6 add real isolated process behavior and installed Python-free acceptance.

**Smallest optional experiment to reopen Juncture:** one three-node counter →
durable question → counter fixture, fresh process and SQLite, no agents/Herdr.
Interrupt after the first node, crash after a non-interrupt checkpoint, resume
twice, and inject a saver/serialization failure. Require a supported public path,
one application of each effect/answer, no false durable gate and unchanged budget.
Record both node re-entry and effect counts so an idempotency wrapper cannot hide
replay. This experiment is **not run or required before stage 1**; source gaps
already support declining the inspected revision. A future passing result would
still need the application transaction boundary proved before adoption.

No additional product decision is needed for the assessment. Approval of the
revised migration plan, including its explicit historical boundary, remains
pending. Proceed through all migration stages automatically only after that
separate authorization, following the [execution contract][migration].

[migration]: rust-backend-migration-proposal.md#execution-contract-after-authorization
[j-root]: https://github.com/greatwallisme/juncture/tree/0a36f67315ea4f8f1f8348ae3af338a1341d02b5
[j-compiled]: https://github.com/greatwallisme/juncture/blob/0a36f67315ea4f8f1f8348ae3af338a1341d02b5/crates/juncture-core/src/graph/compiled.rs
[j-loop]: https://github.com/greatwallisme/juncture/blob/0a36f67315ea4f8f1f8348ae3af338a1341d02b5/crates/juncture-core/src/pregel/loop_.rs
[j-budget]: https://github.com/greatwallisme/juncture/blob/0a36f67315ea4f8f1f8348ae3af338a1341d02b5/crates/juncture-core/src/pregel/budget.rs
[j-checkpoint]: https://github.com/greatwallisme/juncture/blob/0a36f67315ea4f8f1f8348ae3af338a1341d02b5/crates/juncture-core/src/checkpoint.rs
[j-sqlite]: https://github.com/greatwallisme/juncture/blob/0a36f67315ea4f8f1f8348ae3af338a1341d02b5/crates/juncture-checkpoint/src/sqlite.rs
[s-root]: https://github.com/SmooAI/smooth-operator-core/tree/58ba8859313d361b42d21d6445c7ea378fa60822
[s-workflow]: https://github.com/SmooAI/smooth-operator-core/blob/58ba8859313d361b42d21d6445c7ea378fa60822/rust/smooth-operator-core/src/workflow.rs
[s-checkpoint]: https://github.com/SmooAI/smooth-operator-core/blob/58ba8859313d361b42d21d6445c7ea378fa60822/rust/smooth-operator-core/src/agent.rs
[s-store]: https://github.com/SmooAI/smooth-operator-core/blob/58ba8859313d361b42d21d6445c7ea378fa60822/rust/smooth-operator-core/src/checkpoint.rs
[s-human]: https://github.com/SmooAI/smooth-operator-core/blob/58ba8859313d361b42d21d6445c7ea378fa60822/rust/smooth-operator-core/src/human.rs
[s-temporal]: https://github.com/SmooAI/smooth-operator-core/blob/58ba8859313d361b42d21d6445c7ea378fa60822/rust/smooth-operator-temporal/README.md
[r-root]: https://github.com/0xPlaygrounds/rig/tree/9e7cb6000355b7ae1a89c913bf98b8a6f17661ee
[r-readme]: https://github.com/0xPlaygrounds/rig/blob/9e7cb6000355b7ae1a89c913bf98b8a6f17661ee/crates/rig-agent/README.md
[r-tests]: https://github.com/0xPlaygrounds/rig/blob/9e7cb6000355b7ae1a89c913bf98b8a6f17661ee/crates/rig-agent/src/run/tests.rs
[a-root]: https://github.com/liquidos-ai/AutoAgents/tree/6301004371aa3fe274a309e493cdd3bfb07592db
[a-core]: https://github.com/liquidos-ai/AutoAgents/tree/6301004371aa3fe274a309e493cdd3bfb07592db/crates/autoagents-core/src
[a-tests]: https://github.com/liquidos-ai/AutoAgents/blob/6301004371aa3fe274a309e493cdd3bfb07592db/crates/autoagents-core/src/tests/actor_integration_tests.rs
[p-root]: https://github.com/RadiantAILabs/polaris/tree/5ff84646db63589d58bf87956976d749b8c754de
[p-api]: https://github.com/RadiantAILabs/polaris/blob/5ff84646db63589d58bf87956976d749b8c754de/crates/polaris_sessions/src/api.rs
[p-store]: https://github.com/RadiantAILabs/polaris/blob/5ff84646db63589d58bf87956976d749b8c754de/crates/polaris_sessions/src/store/file.rs
[p-graph]: https://github.com/RadiantAILabs/polaris/blob/5ff84646db63589d58bf87956976d749b8c754de/crates/polaris_graph/src/executor/run.rs
[p-tests]: https://github.com/RadiantAILabs/polaris/blob/5ff84646db63589d58bf87956976d749b8c754de/crates/polaris_sessions/tests/run_oneshot_preserved.rs
[o-root]: https://github.com/Data-Mirai/openmirai/tree/1610141d78db618e8bf3346281f2bbd73ac19895
[o-runner]: https://github.com/Data-Mirai/openmirai/blob/1610141d78db618e8bf3346281f2bbd73ac19895/engine/src/core/runner/graph_runner.rs
[o-backend]: https://github.com/Data-Mirai/openmirai/blob/1610141d78db618e8bf3346281f2bbd73ac19895/engine/src/sessions/backend.rs
[o-tests]: https://github.com/Data-Mirai/openmirai/blob/1610141d78db618e8bf3346281f2bbd73ac19895/engine/src/core/runner/tests.rs
