# Rust backend migration — proposal

**Status: approved at `6067fd7`; six simulated migration stages completed on
`refactor/rust-backend`.** Results and deviations are in
[rust-migration-log.md](rust-migration-log.md). This proposal was inspected on 2026-10-12 at
`d14765ba03e739f35f9450258e03bbd4f2dbe923`, with the framework assessment
revision inspecting `f8b342a6b14a9ecd6973c0b1a18de9e11a6da38a`, on
`refactor/rust-backend`.
No tests, workers, installations or service changes were performed while
writing the original proposal. The migration log distinguishes the later
implemented behavior from these planning assumptions.

**Workflow choice reviewed:** [Rust orchestration assessment](rust-orchestration-assessment.md)
screens Juncture, Smooth Operator Core, Rig, AutoAgents, Polaris and OpenMirai.
It recommended the application-owned engine below after inspecting public source
and tests, without executing third-party code. Juncture's public continuation
and checkpoint error paths and Smooth's separation of graph execution from
conversation persistence leave substantial recovery integration to us. Rig is
deferred for a future coordinator scope. The user subsequently approved this
choice; the implemented engine is verified only in the simulated scope recorded
in the migration log.

## Objective and baseline

Deliver the accepted Rust TUI and orchestration backend without an
application-owned Python interpreter, venv, wheel or Python service. Keep
`waywr`, its compatibility aliases, project selection, Herdr visibility and
the existing installation/release layout. External Herdr, systemd, Vim and
selected agent CLIs remain prerequisites. Do not add conversation capabilities,
providers, workflow types or a new installation experience.

The preserved comparison executable is
`.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr`.
Its manifest SHA-256 was rechecked:
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
It contains frontend 0.4.0, backend 0.3.0 and Python 3.12. The parent
`install-r5/bin/waywr` still selects r5: **do not use that alias as the oracle**.
Source checkpoint: `79615442f94326787e81a137ffeeeaff8e476d1d`, merged as
`ae367f2698f643c7b2d9c6ed91c8ad4d65d9b99b`, tagged
`checkpoint/milestone-6-simulated-rust-migration-20261012`.

The operator accepted the refined presentation and completed a simulated r9
workflow after explicit recovery. This is not native-agent or complete-product
acceptance. [The integration report](../milestone-6/unified-ui-integration-report.md) records
the limitations: text becomes a task request, coordinator output is a fake
proposal, open-ended conversation/provider switching are absent, and live UI
steering is unavailable. Preserve these boundaries; do not use migration to
pretend they are implemented.

## 1. Responsibility and dependency inventory

Paths below refer to the inspected commit. Existing Rust reuse is primarily
presentation/client code; there is no Rust workflow store or scheduler yet.

| Area and source evidence | Rust reuse / required replacement | Compatibility obligation |
|---|---|---|
| Coordinator/admission: [session_service.py](../../../src/agent_workflow/session_service.py), `Store.reserve_coordinator`, `publish_proposal`, `admit`, `_promote_ready`, `_coordinate_loop` | Reuse Rust proposal panels and command outbox. Replace fake coordinator invocation, frozen proposal/plan validation, transactional admission and sequential project queue. | A coordinator call can exist before a workflow run ID. Preserve that distinction, explicit scope authorization, task versions, command IDs and stale-plan refusal. No new native coordinator. |
| Worker adapters: [adapters/real.py](../../../src/agent_workflow/adapters/real.py), [native.py](../../../src/agent_workflow/native.py), [adapters/fake.py](../../../src/agent_workflow/adapters/fake.py) | Port prompt/schema construction, argv/environment, structured stdin + EOF, streaming event validation, result parsing, preflight and deterministic fake child. | Freeze provider/model/effort, CLI/application contract versions and limits. Existing support pins Claude 2.1.296/Codex 0.162.1; no silent upgrade, substitution, retry or permission expansion. Existing authentication stays external. |
| Workflow: [workflow.py](../../../src/agent_workflow/workflow.py), `build`, `drive`, `context_change` | Replace LangGraph routing/checkpoints/interrupt continuation with an application-owned Rust transition engine; keep drafting policy separate from durable execution primitives. | Same five stage slots/two critiques for the current policy, conditional early final review, durable questions and answer-driven revision. A human answer is not a model attempt. Completion replay must reuse the existing operation. |
| Store/events/checkpoints: [records.py](../../../src/agent_workflow/records.py), [storage.py](../../../src/agent_workflow/storage.py), `session_service.Store` | Port SQLite transactions, immutable publication, locks, command outcomes and projections. Reuse Rust SHA-256 and IPC types. | One active workflow **per project**, atomic command/outcome/event changes, WAL/FULL durability, persisted generations, byte-preserved history. LangGraph blobs are not application JSON. |
| Context/artifacts/findings: [context.py](../../../src/agent_workflow/context.py), [artifacts.py](../../../src/agent_workflow/artifacts.py), [findings.py](../../../src/agent_workflow/findings.py), [quality.py](../../../src/agent_workflow/quality.py) | Port bounded explicit selection, provenance/staleness, artifact manifests, review deltas and versioned finding logic. | New application-owned IDs and separate assessments; original evidence immutable. Missing coverage remains unresolved. V2 `proposed-addressed` never silently resolves; retain legacy v1 semantics when interpreting history. |
| Questions/approval: `workflow.prepare_choice`, `choice_gate`, `prepare_final`, `final_gate`, `answer`; `records.save_response` | Reuse accepted question/review UI. Replace gate preparation, response commit/consumption and approval checks. | Multiple configured questions, independent question/call budgets, idempotent answers, exact artifact + decision revision + context binding. Neither worker output nor a reconstructed summary grants approval. |
| Execution/recovery: [herdr_backend.py](../../../src/agent_workflow/herdr_backend.py), [runner.py](../../../src/agent_workflow/runner.py), [ownership.py](../../../src/agent_workflow/ownership.py), [routing.py](../../../src/agent_workflow/routing.py) | Replace process identity, runner, permit, cancellation and reconciliation; reuse Linux/libc dependency. | Persist possible-launch intent before Herdr RPC, claim before permit, reserve attempts before spawn, validate receipt before reuse. PID/start/boot and pane/server identity matter. Missing receipt/process is not proof of non-execution. |
| Supervision/lifecycle: [supervision.py](../../../src/agent_workflow/supervision.py), [session_lifecycle.py](../../../src/agent_workflow/session_lifecycle.py), `session_service.Service`, [session_bootstrap.py](../../../src/agent_workflow/session_bootstrap.py) | Reuse Rust owned foreground-client return-to-shell handling; replace project service, dispatch fencing, bounded recovery and systemd bootstrap. | Separate old run supervisor compatibility from current project service. Detach continues; exit fences/drains; resume reconciles. Old exit generation cannot stop resumed work. Task failures leave control responsive. |
| IPC/UI attachment: [frontend/src/main.rs](../../../frontend/src/main.rs), [protocol.rs](../../../frontend/src/protocol.rs), [shell.py](../../../src/agent_workflow/shell.py) | Preserve conversation TUI in `frontend/src/prototype/{editor,rich,view}.rs`, input, colors, drawer, effects, draft/outbox and client transport. Replace Python socket server and attach/bootstrap calls. | Same explicit service authority, replay cursors, bounded channels, same-user peers and responsive input. Scripted scenario stays explicit demo/test mode. |
| Identity/releases/access: [runtime.py](../../../src/agent_workflow/runtime.py), [release_maintenance.py](../../../src/agent_workflow/release_maintenance.py), [context_access.py](../../../src/agent_workflow/context_access.py) | Reuse Rust `route_bound_release`; port project markers, registry/location checks, release maintenance and grants. | No installation-wide execution lock or implicit upgrade. Grants are directed, task-scoped, bounded, revocable and nontransitive. Registry is deterministic routing only; never model context. |
| Supporting runtime: [cli.py](../../../src/agent_workflow/cli.py), [review.py](../../../src/agent_workflow/review.py), [integration.py](../../../src/agent_workflow/integration.py), [packaging](../../../packaging), [resources](../../../src/agent_workflow/resources) | Replace Python CLI dispatch, review-plugin handler, installer/build helper and resource lookup. | Keep `wayward-wrangler`/`awp`, explicit paths, review URLs and diagnostics. Include skills, fake profile and schemas in the release. No hidden Python fallback for `status`, `open-link`, install or fake workers. |

**Python dependencies:** [pyproject.toml](../../../pyproject.toml) directly pins
LangGraph 1.2.14, checkpoint 4.2.0 and SQLite saver 3.1.1. Application source
imports their graph/checkpoint/interrupt APIs only in `workflow.py`; it uses
stdlib SQLite, asyncio/threads, subprocess, sockets, flock, TOML, JSON and
hashing elsewhere. [requirements.lock.txt](../../../requirements.lock.txt) includes
LangChain/LangSmith, HTTP clients, Pydantic, msgpack, sqlite-vec and other
transitive packages. There is no application vector store, model SDK or network
service to recreate merely because those packages are installed. `difflib`
review deltas and Python JSON formatting do need explicit parity coverage.

## 2. Recommended Rust ownership and dependencies

Add a small `backend/` library crate beside the existing frontend; a Cargo
workspace joins them without moving the accepted UI. Modules: `records`,
`workflow`, `context`, `adapters`, `execution`, `herdr`, `session`, and
`project`. These are ordinary internal modules, not a general plugin framework.
The existing `waywr` executable hosts launcher/CLI plus internal service,
runner and fake-worker subcommands. The TUI continues to use IPC; it never
opens the workflow database or owns agent execution.

- **Project service:** sole workflow/state writer, scheduling and lifecycle
  authority; one lock and monotonic service generation per project. One
  executing workflow plus durable queue per project; unrelated projects run
  independently. Coordinator results are proposals submitted to validation.
- **Runner:** separate real Herdr-pane process, one frozen operation/child;
  owns streaming evidence and completion, never transitions tasks or approves.
  Completed panes retain output as today. No hidden-worker backend.
- **Storage:** a dedicated SQLite writer thread with bounded requests and short
  transactions. Async RPC/process waiting never holds that thread or a database
  transaction. Full mailboxes return retryable busy **before acceptance**;
  durable command lookup resolves uncertain delivery. Bounded readers serve
  status while a child runs. No new task concurrency policy.
- **Workflow:** typed state + deterministic transition function returning
  recorded effect intents. Commit stage reservation before effects; reconcile
  claim/permit/receipt on restart; commit result application once using unique
  operation/result identity. Filesystem publication and SQLite cannot be one
  transaction: verify/reuse immutable files after a crash, never infer a new
  call is needed. This replaces LangGraph replay with explicit application
  semantics. The choice rests on the assessed libraries' persistence/effect
  boundaries, not on assuming the current workflow is the entire product.

The engine stores a frozen `workflow_id`, contract version and definition/config
hash, while a typed policy module defines the existing drafting/review flow.
Keep stage identities and invocation limits in that policy; the durable store
handles steps, gate references and effects without a global five-stage enum.
This preserves a path to configurable workflows without adding a DSL, dynamic
graph editor, provider runtime or second workflow during migration. Future
conversation libraries may propose actions through the existing validator;
they cannot own approvals, worker retries or project scheduling. Reconsider a
graph library only against an approved workflow-variation requirement and the
same recovery gates, rather than building a general framework preemptively.

Preserve lifecycle bounds: 30-second foreground drain is not cancellation;
the current project unit uses `Restart=on-failure`, one-second delay and a
three-start/30-second limit. Native workers have a ten-minute execution limit;
fake workers have no equivalent deadline. Expiry cannot authorize another call.
Known-running/stalled work gets a durable attention notice; missing execution
identity stays uncertain. Receipt validation wins over a late cancellation.
Port explicit SIGTERM/three-second escalation only for a verified owned child,
never as an effect of dropping a UI, future or service handle.

| Dependency choice | Concrete use and constraint |
|---|---|
| Existing Serde/serde_json, sha2, libc and UI dependencies | Keep the accepted Ratatui/Crossterm/textarea/TachyonFX versions. Strict typed records plus explicit duplicate-key rejection at authority boundaries; Serde defaults alone are insufficient. |
| Tokio 1.x (docs inspected at 1.53.2) | Unix sockets, timers, signals, bounded channels and asynchronous child pipes. Use selected features, not a network/RPC framework. [Process documentation](https://docs.rs/tokio/1.53.2/tokio/process/index.html) makes clear that dropping a handle is not durable cancellation; retain/reap children and explicitly verify targets before signals. |
| rusqlite (docs inspected at 0.40.2), `bundled` + `backup` | Existing SQL/constraints, explicit transactions and consistent rollback snapshots. Avoid an ORM or async database pool. [Backup API](https://docs.rs/rusqlite/0.40.2/rusqlite/backup/index.html), [bundled SQLite feature](https://github.com/rusqlite/rusqlite#optional-features). Disable extension loading; no vector extension. |
| toml (docs inspected at 1.1.8), clap 4.x | Parse existing project/profile/selection files and replace argparse compatibility commands. Preserve accepted config/CLI semantics through fixtures; do not rewrite target files to suit the parser. [TOML docs](https://docs.rs/toml/latest/toml/), [Clap docs](https://docs.rs/clap/4.6.7/clap/). |

No orchestration/agent library is added by this recommendation. The assessment's
optional Juncture experiment is a narrow way to reopen that choice if needed,
not a prerequisite for the independent record-reader task or permission to
build multiple engines. Pin compatible releases in Cargo.lock when their stage
is implemented; verify the existing toolchain/MSRV then. Use std/libc for file durability, OS entropy,
process groups and systemd CLI invocation. No HTTP stack, agent SDK, embedded
Python, msgpack/LangGraph reimplementation or model-based supervisor. Port the
small delta behavior against fixtures before considering a diff dependency.

## 3. State, protocol and historical compatibility

### What Rust can read

| Existing record family | Treatment |
|---|---|
| `app_runs`, `app_operations`, `app_requests`, `app_responses`, `app_rebindings`; `session_*` tables | SQLite scalar columns/JSON are language-independent. Read with explicit schema detection and retain raw JSON strings. Preserve IDs, event sequence, accepted command outcomes, versions and approval bindings. New writes require the versioned upgrade below. |
| Packets, envelopes, manifests, Markdown, decision exports, launch/claim/permit/dispatch/child/receipt, findings and recovery JSON | Read/hash original bytes, preserve permanently. Interpret only known versions and verify cross-references. Missing/unknown evidence means unavailable/refused, not inferred success. Legacy absolute paths stay provenance; explicit current project/state bindings resolve access. |
| UI draft/outbox, grant/revocation files, release/location bindings, shutdown intents/completions, compact session checkpoints | Preserve and validate. Session checkpoint JSON is a bounded navigation projection, **not** sufficient authority to reconstruct a workflow or approval. Pending commands retain IDs and bodies across reconnect. |
| LangGraph `checkpoints` and `writes` | Preserve opaque; do not deserialize into Rust scheduling state or write them. Use historical app records for inspection; report graph-only detail as unavailable when no authoritative app record supplies it. |

**Framework evidence:** read-only inspection of the preserved
`.local/milestone6/unified-ui/evidence/final-flow10/project/.wayward-wrangler/state/state.sqlite`
found 11 `msgpack` checkpoints and `msgpack`/`null` pending writes. Installed
`langgraph/checkpoint/sqlite/__init__.py::setup` defines BLOB tables;
`serde/jsonplus.py::{dumps_typed,loads_typed}` uses tagged MessagePack and
Python reconstruction extensions. These are not portable JSON snapshots.
`workflow.drive` also depends on graph interrupts, `next`, and synchronous
checkpointing; copying just `State` loses replay position.

**Byte integrity:** `storage.canonical` means sorted keys, two-space indentation,
UTF-8, trailing newline; some session command keys instead use compact sorted
JSON. Hash original bytes whenever available. Specify and test these legacy
encodings, including Unicode/numbers, for derived historical references.
Never reserialize evidence to “fix” its hash. Version new record codecs; freeze
raw packet bytes at creation and use those bytes in every subsequent check.
Artifact bytes/hashes are never normalized by parity tests.

### New execution state and upgrade

Introduce application state schema 2 with `ww_schema_migrations` and
`ww_workflows`: run ID, workflow contract version, revision, next transition,
JSON state, frozen workflow definition/config hash, pending gate ID and
applied-operation references. Reuse app/session
ledgers; do not introduce a parallel approval or decision database. Store
completed slots, current/last-reviewed artifact refs, finding history/coverage,
decision revision, context digest, question count/limit and outcome explicitly.
An effect reservation and its applied-result marker have unique keys. Commit
state revision, answer consumption/result application and observable event
together; an already-saved answer is consumed without another request/call.
The application request/response ledger remains human authority; workflow state
holds references and consumption markers, not a competing copy of approval.
Runner receipts remain execution evidence. Engine re-entry only reconciles the
same effect identity and cannot authorize a replacement invocation.
Persist `engine_kind` and contract version per run. Old rows lacking them are
legacy, never candidates for automatic Rust scheduling. Runtime ownership
checks and maintenance gates must include both coordinator and worker records.

Keep IPC major 1, its NDJSON envelope, command IDs/preconditions, 1 MiB frame /
64 KiB command limits, cursor replay and same-UID/project/release handshake.
Publish actual state schema/owner generation and reject incompatible peers.
Bring the schema document into agreement with implemented lifecycle messages;
do not derive the new server from the stale fixture-only protocol description.
Any genuinely incompatible wire change needs a version bump, never silent
reinterpretation. Release manifests get an explicit runtime kind and state
compatibility range; no Python version field in a Rust-only release.

**Recommended compatibility boundary (requires plan approval):** no conversion
of an in-flight or suspended LangGraph run in this migration. Even a human gate
contains graph-specific continuation. Keep such projects pinned to their
explicit legacy release; Rust can inspect their application-owned history
read-only. Finish them there, or separately authorize archival/abandonment;
never silently drop queued/proposed/blocked work. An upgraded quiescent project
can start Rust workflows and explicitly reuse validated approved historical
decisions/artifacts/findings. Known trial-specific adoption commands become
read-only inspection/idempotent reporting of existing recovery records; requests
to create a new legacy recovery explain that the bound legacy release is
required. Do not generalize old one-off authorization into a new recovery power.

Upgrade only the selected project after verified quiescence: no pending client
delivery, unresolved tasks/calls, live UI or unreceipted execution. Preserve the
existing verified pre-permit recovery exception **within legacy releases**;
it is not permission to migrate unfinished work to a different engine.
Hold project maintenance/start/dispatch ownership, verify service stopped, take
a consistent SQLite backup plus hashed file/outbox/binding inventory, then
apply a transactional additive migration. Retain opaque graph tables and
original app rows unchanged. Append upgrade intent/completion and only then
switch release binding, reconciling partial updates after interruption.

Old Python code does not understand the new schema fence. **Never point it at
an upgraded writable database.** Before any new Rust execution, rollback may
restore the complete pre-upgrade state/binding under exclusive ownership.
After Rust accepts new work, no blind down-migration: retain all new evidence
and either use a compatible Rust checkpoint or require a separately reviewed
state recovery. A Git revert is not a data rollback.

Legacy releases remain explicit compatibility installations, not dependencies
bundled or auto-installed by Rust. Normal attachment respects their bindings
and reports that execution is legacy/Python; it cannot silently upgrade them.
Read-only historical inspection must work through the Rust binary without
executing the old release. Complete removal of Python from **all existing
deployments** additionally requires explicit retirement of those legacy owners;
this plan does not authorize stopping the operator's retained services.
Coexistence ends per project after its unfinished legacy work is completed or
explicitly archived, outstanding execution is resolved, and its quiescent upgrade
passes. Release deletion checks remaining references. New Rust-backed projects
never require the legacy runtime or call it as a fallback. Similar graph APIs,
serialization formats or a library's “LangGraph port” claim do not establish
checkpoint or external-execution compatibility.

## 4. Ordered implementation and parity gates

Build Rust replacements behind tests until the project service is complete;
do not insert a Rust scheduler alongside a Python owner. Stages 1–4 have no
production routing change. The preserved Python executable is a comparator
against **separate fixtures**, never a second writer of the Rust project.
Stage 5 enables a Rust-owned isolated project; stage 6 removes Python from the
new distribution. No temporary Python RPC bridge is necessary.

| Stage | Scope and required parity before advancing | Checkpoint/data rollback |
|---|---|---|
| **1. Record boundary and comparison harness** | Add backend crate, typed legacy read-only reader, byte/hash codecs and fixture contract. Compare approved artifact/decision/finding inspection, missing/tampered records, schema refusal and path containment against baseline. Separate raw evidence from normalized reports. | Commit before/after; no runtime or schema writes. Delete only disposable outputs to rerun; retained baseline/evidence untouched. |
| **2. Policy, packets and adapters** | Port explicit context/grants, artifact/finding/question validation, frozen plans, fake coordinator/worker, native argv/schema/event handling. Compare selected context, inheritance, revocation/no transitive imports, stale plans, review deltas, missing dispositions and meaningful errors. Use stub CLI executables/saved outputs; no authentication or provider calls. | Inert production code; isolated immutable fixtures only. Version any intentional new encoding; do not rewrite saved prompts. |
| **3. Runner and Herdr execution** | Rust runner/launch/claim/permit/receipt/cancel with retained visible output. Compare every existing recovery boundary: lost RPC, stale owner/server, missing claim, completed receipt before state commit, dead runner/live child, cancel/completion race, no sixth attempt or third critique. Exercise actual processes on an isolated Herdr server after offline fault permutations. | No live project cutover. Preserve failure evidence; stop only verified test-owned processes. Archive post-snapshot evidence and resolve test processes before restarting a disposable case in a fresh namespace; never reuse an ambiguous operation ID. |
| **4. Application-owned durable workflow** | Replace graph progress in a harness using a versioned policy/definition reference and one authoritative app transaction boundary. Compare full fake flow, early final review, multiple durable questions, record-answer/exit/fresh consumption, incomplete revisions, invalid/uncertain stops, exact approval and publication/checkpoint interruptions. Inject failed persistence, repeated result/answer delivery and unknown state/definition versions. No false durable gate, duplicate application, repeated completed call or reset budget. | Create schema-2 disposable stores; backup before schema edits. Preserve post-snapshot results before test rollback; restore store/files only into a fresh isolated case. Python owns separate unchanged comparator state. |
| **5. Project service and lifecycle** | Port session commands/queue/events, generations, systemd bootstrap, UI attachment/routing, exit/detach/resume and deterministic handoff. Connect the **accepted TUI** unchanged to Rust IPC. Compare lost replies/stale commands, responsive control during execution, resume/drain/shutdown races, stalled versus uncertain, bounded recovery, missing workspace and two-project independence including identical run IDs. | Side-by-side test release and project-specific binding only. Test upgrade interruption and rollback on quiescent copies; never attach Python to schema 2. Retained services unchanged. |
| **6. Python-free installation and integrated acceptance** | Port remaining CLI families/plugin/build/install paths; embed/package resources. Install into a fresh isolated prefix without source access, venv/wheels/interpreter. Verify normal `waywr`, aliases, review links, approvals, context reuse, cold/warm/inside-Herdr startup, service restart, lifecycle/recovery and distinct per-project releases through that installed interface. Inspect process trees/unit commands and deny Python execution in the fixture environment. Historical approved runs inspect/reuse without LangGraph; unresolved legacy execution fails clearly or routes only by its explicit legacy binding. | Capture final manifest and parity evidence before removing Python from the new bundle/source runtime path. Preserve comparator separately. Test release/state rollback; do not change operator bindings/shared plugin. |

Reuse focused cases in `tests/test_{contracts,context_continuity,context_access,
finding_protocol,revision_quality,native_admission,real_adapters,saved_output_adoption,
recovery_boundaries,coordinator_recovery,session_store,session_lifecycle,
session_attach_recovery,release_maintenance,project_identity,portability}.py`;
use saved finding-protocol fixtures and installed acceptance drivers as inputs,
not as evidence that the Rust implementation already passes. Add Rust
externally observable scenarios so core parity can run after Python is removed.
The Python comparator remains a development-only migration oracle, removable
from required acceptance once reviewed golden contracts and Rust checks exist.

CLI coverage includes diagnostics (`status/show/logs/workers/notices`), decisions
and review, location/release maintenance, preflight/start/resume/cancel,
foundation probes and explicit integration installation. Run-oriented control
and `supervise`/`supervisor-install` must use the same project ownership/fences,
not create a second scheduler. Preserve legacy command parsing; unsupported
legacy mutation reports its compatibility boundary rather than invoking Python
behind the operator's back. The plugin's Rust handler keeps the existing
project/run/artifact/hash URL and verified read-only Vim action. Test its copied
installation on an isolated Herdr registration; leave shared registration alone.

### Execution contract after authorization

Approval authorizes proceeding through all six stages without asking to continue
after each pass. For **every** stage:

1. Record a recoverable Git checkpoint, baseline release/source hashes, isolated
   state/file snapshot and schema/binding rollback requirements before changes.
2. Implement only its scope. Compare equivalent inputs against the preserved
   Python implementation: transitions, packets/artifacts, authorization,
   questions/approval, errors, recovery, budgets and duplicate-dispatch refusal.
3. Normalize only incidental timestamps, generated IDs and fixture locations,
   using a consistent identity map that still checks reference relationships,
   ordering/generation, ownership and project separation. Do not normalize
   artifact bytes, decision content, hashes, model settings, counters or outcomes.
   Different delta formatting needs explicit review as a packet-contract change.
4. Diagnose differences before advancing. Log baseline defects separately from
   intended semantics; correct defects within scope rather than copying them.
   Record every variance/nuance with evidence, impact, disposition and regression
   coverage, including intentionally retained differences. A passing gate means
   intended behavior, not matching a known bug or changing expectations to pass.
   Preserve failing inputs/output. No dependent work on unexplained differences;
   repair within the stage or use its code **and state-aware** rollback procedure.
   Never restore an old snapshot over newly completed work or route a legacy
   release to incompatible state. Test resets use fresh isolated namespaces after
   retaining evidence and resolving owned processes; production rollback obeys
   the before/after-new-work boundary in §3.
5. After parity passes, commit, update [the migration log](rust-migration-log.md)
   and handoff with evidence, compatibility and remaining Python dependencies,
   then continue automatically. Record resulting commit in the next log entry
   or closing documentation commit rather than attempting a self-referential SHA.

Pause only for a substantive product decision, unsafe/irreversible migration,
unavailable required access, or a blocker outside approved scope. Report the
evidence and recommended resolution. No authenticated calls are implied by
migration approval: native adapter comparison uses fixtures only. Native
isolation/real-provider acceptance remains separately authorized.

**Completion condition:** the final installed Rust release runs the accepted
simulated interface/workflow, survives the specified failures, reads historical
approvals and reuses explicit approved context with Python unavailable. Its
launcher, service, runner, fake child, aliases, plugin handler, installer and
diagnostics contain no Python execution path. No application wheel/venv is
created or shipped. External CLI requirements and legacy deployment exceptions
are documented. Automated PTY checks cover input/restoration; physical
Ghostty/Herdr Shift+Enter, copy/selection, appearance and return-to-shell still
need operator observation after integration. Keep those results separate.

## 5. Risks, known nuances and the review decision

- **History versus resumability:** app evidence can preserve approval authority
  without reconstructing a LangGraph interrupt. Recommended boundary above
  trades automatic in-flight migration for a small, auditable implementation.
  The current operator project has unresolved extra requests; its one approved
  run does not make the whole session quiescent. Do not upgrade it automatically.
- **External execution cannot be exactly-once by database transaction alone.**
  At-most-one authorized dispatch plus explicit uncertainty is the preserved
  guarantee. Tokio, persisted workflow state and heartbeats do not solve a lost
  launch acknowledgement or surviving orphan.
- **Record/SQL divergence:** receipts, exports, graph updates and session state
  currently commit at different boundaries. Differential fault injection must
  prove the Rust reconciliation rules, not just happy-path final text.
- **Baseline inconsistencies, not new product requirements:** `Service.handle`
  reports `owner_generation: 1` despite the persisted service generation;
  `task_detail` inspects original `call-vN` while recovery may execute
  `call-vN-r1`; old architecture/protocol prose still describes registry-wide
  exclusion or a fixture service. Log and correct actual generation/effective
  evidence reporting in stage 5, retaining original records. Do not treat stale
  prose as authority over current project-isolation and lifecycle contracts.
- **Native restrictions are not proven by fake packets.** The adapters disable
  tools/hooks/config and refuse observed tool activity; Codex's own comment
  acknowledges a remaining advertised-tool caveat. Same-user filesystem access
  is not a hostile-process sandbox. Port restrictions without relaxation and
  keep native acceptance blocked pending separate isolation validation.
- **Hidden Python dependencies:** plugin manifest currently invokes `python3`;
  Rust `backend_info`, `legacy_cli` and `python_bootstrap` require the venv;
  release validation requires `venv/bin/python`. Stage 6 must cover these and
  old service/install commands, not just delete LangGraph from dependencies.

**Substantive decision requested:** approve the conservative historical boundary:
read/verify/reuse approved legacy context, but no transparent in-flight LangGraph
continuation or automatic migration of retained operator services. If live
legacy continuation is required in Rust, a separately scoped, versioned export
of continuation state at verified boundaries is needed before cutover; it cannot
be supplied by decoding arbitrary checkpoint blobs. Other technical choices
above can be executed routinely within the plan.

## 6. First independently reviewable task

Implement **stage 1 only as the first commit-sized boundary**: `backend/` crate,
versioned legacy record reader and migration fixture/normalization contract.
Given an explicit archived state root, inspect one approved run's artifact,
decision, approval and finding provenance using read-only SQLite/file access;
report unsupported graph-only detail without decoding it. Fixtures cover
Unicode/hash bytes, changed artifact, conflicting approval/export, unknown
schema and escaping path. Compare with the pinned Python baseline on disposable
copies, verify inputs unchanged, and record results in the migration log.
No launcher routing, production SQL writes, services or worker execution in
this first task. After its parity checkpoint, continue to stage 2 when the full
migration has been authorized.
