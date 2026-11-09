# Rust backend migration log

## Hands-on startup acceptance correction (2026-10-12)

The operator's extra-tab finding reopened the Rust installed-startup gate.
The [startup correction report](rust-migration-startup-correction.md) records
the actual Herdr 0.9.3 server/workspace/tab path, direct-terminal attachment,
project-owned worker workspace binding, installed fake/PTy evidence, and the
captured `stale_state` exit/resume race. Earlier Stage-5/6 and v7 simulated
passes did not verify this physical interaction. The previous v7 installation
and r9 baseline remain immutable. The v5 corrected candidate is a separate
release; no operator project/schema binding was changed. Rollback is to retain
each project on its existing bound release, not to restore an old database
snapshot over newer completed work. Remaining Python application runtime is
zero for Rust-bound projects; historical legacy releases stay explicitly
bound. Native and physical-terminal acceptance remain separate gates.

## Corrective migration boundary — runtime and CLI parity (2026-10-12)

The operator reopened the Stage-5/6 acceptance gates after the
[closeout review](rust-migration-review-addendum.md). The clean code checkpoint
is `cc721d649c29f9166c877852bacdf6c81ebfa2c8`, also retained by local
annotated tag `rust-migration-pre-corrections-cc721d6`. The r9 baseline and
Stage-6 verified installation stay immutable. Prior Stage-5/6 passes establish
**partial simulated coverage**, not full runtime or CLI migration acceptance.
All new fixtures must use fresh isolated project/state and private Herdr
resources. No operator project/schema/binding migration is authorized; code
rollback to the tag does not authorize restoring an older state snapshot over
newly accepted work.

Before stress testing, set these local acceptance bounds: with 32 connected
slow clients, a separate status/command lookup should return in under one
second; with a blocked SQLite writer, an unaccepted mutating request should
receive explicit busy/error within 500 ms and remain absent from the command
ledger; while an eight-second Herdr control call stalls, independent status
and queued-task control should still respond within one second. The writer
mailbox capacity is at most 64 requests, and the runner's pending stdout plus
stderr channel data is at most 1 MiB. Combined retained transcript/output is
limited to 32 MiB per invocation; over-limit execution must retain a failure
or uncertainty record and must never trigger an automatic retry. These bounds
continue the earlier under-one-second active-worker status criterion and cap
the previously unbounded queue. Tests must report observed maxima and not
weaken the bounds to obtain a pass.

## Corrective closeout — passing installed simulated gate (2026-10-13)

- **Code checkpoints:** original `cc721d649c29f9166c877852bacdf6c81ebfa2c8` and local annotated tag `rust-migration-pre-corrections-cc721d6`; bounds/checkpoint commit `644b9c6316ce82d203e1e2d6a79c92701688d747`; first runtime/CLI commit `31e8efb8de1ad4fb7157d8230f9f7e7846487772`; final verified code commit `aa426d332c8bf8bd250dfd7ff93bc052e6a9f7a4`. The earlier Stage-5/6 passes remain labelled **partial coverage**. The full command disposition and source evidence are in [rust-migration-correction-report.md](rust-migration-correction-report.md).
- **Scope:** Tokio service I/O, timers, signals and runner pipes; bounded dispatch and combined output; project SQLite writer with 64-entry queue, explicit pre-acceptance busy and command-ID reconciliation; Herdr tab-closure identity-scan repair; ordinary CLI contracts with Clap; project-private saved CLI command IDs, status and explicit replay. No coordinator feature, workflow type, provider or TUI redesign was added. Relative to `cc721d6`, `Cargo.lock` added 16 packages for Clap/Tokio and removed or upgraded none; the final frontend Tokio edge reuses the already locked version. These changes belong to the corrective commits, not the permanent-toolchain switch.
- **Installed result:** paired `.local/rust-migration/corrections/final-v7-install/releases/rust-corrective-final-v7/` with manifest SHA-256 `fdb638ac3d922a7cbbb38587deed883c42ad406713b18848e30c7cb4d006d392`. The prior Stage-6 installation and r9 baseline remain unchanged. No default link, operator project binding, shared plugin, service or Herdr resource was changed.
- **Behavioral comparison and faults:** `final-v7-service/{summary,cleanup}.json` passed installed fake TUI, active-work exit/restart, receipt reuse, missing-workspace routing, two-project isolation and exact review binding. `final-v7-cli/{summary,cleanup}.json` passed installed Rust versus separate installed r9 fake fixtures for run targeting, artifact/transcript bytes, record-only gates, exact approval and cancellation; saved command status/replay kept five invocations unchanged. `runner-regression-01/summary.json` covers seven private-Herdr execution/recovery cases. `runner-flood-06/summary.json` records one over-limit failure receipt and no retry, with 39,752 KiB runner peak RSS and a 32 MiB retained transcript. Locked/offline workspace tests and Clippy passed; logs are `final-v7-{cargo-test,clippy}.log`. All these paths are under ignored `.local/rust-migration/corrections/`.
- **Predeclared bounds:** the final test log reports 10 ms with 32 slow clients, 253 ms for locked-SQLite busy, 3 ms for status during stalled Herdr I/O, 1 ms for the answer/cancel/exit snapshot, and 501 ms for a deliberately blocked pane display write. These samples passed the stated thresholds; they are not native-load latency percentiles. The pending pipe queue remains at most 1 MiB and the combined transcript at most 32 MiB by code and flood fixture.
- **Observed variances and disposition:** r9 fake `preflight` is native-only; Rust permits fake validation and keeps native admission blocked after version/status-only authentication checks. The Rust project service is asynchronous while r9 `resume RUN` drives the old controller synchronously. Rust diagnostic output is structured JSON; `--color always` is explicitly refused. Nondefault `supervise --interval/--max-attempts` is refused rather than launching another scheduler. Historical trial recovery/probes stay on explicitly bound r9. These are documented changed contracts, not normalized away in the differential. The first installed drain timed out with an idle UI client; the Tokio shutdown signal fixed it. A later UI registration failure exposed unrelated Herdr tab closure between `tab.list` and `layout.export`; bounded identity rescan fixed it. Another installed CLI run exposed `session stop` treating a `stale_state` refusal as success; bounded rejection-only retry fixed it. Failed-attempt evidence is retained.
- **State-aware rollback:** no operator state was migrated. New isolated fixture records live under ignored `.local/` or `/tmp`; preserve their receipts and failure evidence before disposing of fixtures. The Rust schema-2 `session_run_requests` table, record-only field and CLI saved-command files are additive. A code rollback to the pre-correction tag is safe only for a project with no pending named run, held answer or new CLI command requiring reconciliation. Otherwise retain its bound compatible release until the command/run is resolved and quiescent. Never restore an old SQLite snapshot over completed work, convert unfinished LangGraph checkpoints, or point legacy state at Rust.
- **Remaining boundary:** Rust-bound projects require no Python application runtime. The Python r9 comparator and unfinished LangGraph projects remain on explicit legacy releases; Python test harnesses are not application runtime. Native context/tool isolation, authenticated workers and physical Ghostty/Herdr observation are not established by simulated fixtures. No operator cutover or release publication occurred.

## Stage 6 — Python-free paired release: passing simulated gate (2026-10-12)

- **Passing stage commit:** `b881cde64e22bcc235e3173518df471cf100a087`.
- **Scope and ownership:** Rust now supplies the installed launcher, project
  setup/routing, offline diagnostics, project-scoped maintenance, review-link
  handler, resource-complete bundle builder/installer and aliases `waywr`,
  `wayward-wrangler`, `awp`. It connects the Stage-5 Rust service, runner and
  accepted TUI. The bundle includes the fake profile, prompt contracts and
  skills, but no wheel, venv or Python code. Some trial-specific recovery
  commands and some ordinary r9 CLI controls refuse on a Rust-bound project;
  an explicitly legacy-bound project requires its compatible legacy release.
  The exact command inventory and impact are in
  [the closeout review](rust-migration-review-addendum.md). `legacy-inspect`
  reads app-owned approved history without interpreting LangGraph checkpoints.
- **Code/state checkpoint and rollback:** started from Stage-5 commit
  `7ef2ef1af9177ef8352c718c206b9aaed9299f4a`. Only fresh ignored
  Stage-6 projects, release prefixes, private Herdr sockets and test-owned
  units were written. The known-good Rust release is
  `.local/rust-migration/stage6/verified-install/releases/rust-migration-s6-verified-20261012/`
  (manifest SHA-256 `1ec1414e880051d7fa938e94a3dcc65cfbe5a4d0c4f944237d339cd4e2de4aec`).
  Its paired entry point is `verified-install/bin/waywr`; a second identical-code
  release ID in that prefix was used only for routing tests. Source/code
  rollback is the Stage-5 commit. An isolated quiescent fixture may restore
  its saved release binding and consistent state snapshot only before any new
  work, answer or receipt. After new work, retain that release/state and
  reconcile; never point schema-2 work at r9. The r9 manifest remains
  `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
- **Final verification:** `cargo test --workspace --locked --offline` passed
  37 frontend library, five frontend binary, 14 backend, and seven fault tests
  (one explicit timing probe ignored). The final installed, restricted-PATH
  fake flow passed at `stage6/verified-live/summary.json`: two independent
  projects, visible Herdr coordinator/worker panes, active-worker exit and
  receipt reuse, missing-workspace route, one durable question, five worker
  slots, saved answer, stale-hash refusal and exact fixture approval. The
  final artifact SHA-256 `542a9546c93ab8c1c31a684a55d847305bf3f9776354827d2ece9a6d787a565a`
  matches the Stage-5 equivalent fixture. `stage6/verified-routing/summary.json`
  verifies aliases, independent release bindings, unavailable unrelated
  project, quiescent upgrade and revision-bound rollback. The private plugin
  test at `stage6/verified-plugin/summary.json` reopened a completed artifact
  in one verified read-only Vim tab and focused the same tab on repeat.
  `stage6/verified-legacy-inspection.json` exactly equals the Stage-1 approved-run
  inspection (two responses, five artifacts). A second installation and
  full fake flow under `/tmp/waywr-rust-s6-verified-{install,live}` passed with
  project/release outside the checkout; all 196 regular evidence files were
  hash-verified into `stage6/verified-external-live/` with provenance in
  `stage6/verified-external-copy-verification.json`. An installed status trace at
  `stage6/verified-source-access.strace` shows no access to source `frontend/`,
  `backend/`, `src/` or `target/` paths. The application PATH in these runs
  contained Herdr/systemd/Vim helpers but no Python executable. All final
  private servers and test units report stopped in their `cleanup.json` files.
- **Observed differences and repairs:** offline `status` no longer starts a
  service just to inspect state; it reads the selected project's records.
  This is an intentional bounded diagnostic change. A pending gate briefly
  appeared in task detail before its status became `awaiting-input`; the
  projection and wait fixture now agree. A v2 review link initially ignored
  an explicit state location; it now uses the verified registry state path.
  Release changes now also refuse pending client outbox delivery and live
  UI claims, with a focused regression test; the earlier final-install
  candidate is retained only as evidence, not the verified baseline.
  Earlier `release-e/e2` and `plugin-h` failed due test expectations about
  asynchronous stop and the reconnected UI workspace; `final-plugin` passed
  its behavior but its cleanup failed because the restricted test PATH lacked
  `journalctl`. These attempts remain preserved; fresh final cases passed.
  Bubblewrap could not mask the original source checkout (`uid map: Permission
  denied`), so the external-prefix full run plus file-access trace are the
  source-independence evidence, not a claim of a mount-isolated execution.
- **Compatibility and limits:** Rust-backed projects need no application
  Python interpreter, wheel, venv or Python service. Python source and the
  installed r9 pair remain solely for explicitly bound unfinished legacy
  projects and the preserved comparator; no operator project/service was
  cut over. Native adapter restrictions were inspected and tested offline;
  authenticated CLI execution and same-user filesystem/tool isolation remain
  unverified and blocked. Physical Ghostty/Herdr appearance, Shift+Enter,
  copy/selection and return-to-shell still require operator observation.
  The Rust diagnostic CLI does not expose all old ordinary commands or flags;
  supported task answers/approval remain in the accepted TUI, which does not
  itself establish CLI compatibility. No new
  coordinator capability or provider was added.

## Stage 6 start boundary — Python-free release and compatibility (2026-10-12)

Code checkpoint: Stage-5 commit `7ef2ef1af9177ef8352c718c206b9aaed9299f4a`;
worktree clean at entry. Installed r9 comparison manifest remains SHA-256
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
Only fresh, ignored Stage-6 bundles, installation prefixes, private Herdr
servers, project fixtures and test-owned units may change. Existing operator
release bindings, services, historical evidence and shared plugin registration
remain untouched. Before each schema/binding upgrade test, save the exact
binding, consistent database copy and immutable file inventory. A rollback may
restore a quiescent fixture *only* before newer work/answers/receipts exist;
otherwise retain the new binding and reconcile or require an explicit state
migration. This stage must demonstrate actual installed execution with no
application Python runtime and clear legacy-release routing/refusal. This is
a start boundary, not a passing result.

## Stage 5 — Rust project service and accepted TUI: passing simulated gate (2026-10-12)

- **Scope and authority:** Added project-scoped Rust session commands, queue,
  coordinator reservation, admissions, events, task detail, lifecycle generation,
  deterministic checkpoints, IPC, systemd bootstrap, Herdr UI claims and routing,
  and a scheduler for the existing fake policy. The accepted Rust conversation
  frontend connects over protocol major 1. A project has one service owner lock;
  workers and UI remain distinct Herdr panes. Cross-project imports use explicit
  grants; packets draw only from the selected project and selected imports.
  The installed Rust test route remains fake-only. No operator binding or
  service was changed.
- **Checkpoint and state rollback:** started at `c5463e7`. Schema-2 state,
  project/release bindings, service identities and receipts were created only
  in fresh ignored Stage-5 fixtures. Code rollback is to Stage 4. A project
  that has accepted Rust work cannot be pointed at r9 or restored from a
  pre-answer/pre-receipt snapshot. Stop and reconcile its owned service and
  operations, then roll back only an isolated quiescent test binding together
  with a consistent state/file copy. No automatic legacy cutover exists.
- **Verification:** `cargo test --workspace --locked --offline` passed 37
  frontend library tests (one intentionally ignored timing probe), three
  frontend binary tests, 12 backend unit tests and seven operation/workflow
  fault tests. A subsequent focused bounded-scheduler test also passed,
  bringing the backend unit count to 13. The final copied Rust release passed a five-slot
  fake flow with one coordinator receipt, question, stop/restart, stale-hash
  refusal and exact fixture approval at `stage5/live-09/summary.json`.
  `stage5/active-exit-01/` proves foreground exit while draft was active,
  receipt collection, fresh-service continuation and no repeated draft call.
  `stage5/systemd-04/` proves test-owned systemd bootstrap, KILL restart and
  one coordinator receipt. `stage5/cold-03/` proves ordinary-terminal cold
  startup into a private Herdr server with a real PTY and test-owned unit.
  `stage5/live-09/` also covers two project stores with identical IDs, B
  cancellation while A awaited input, and verified missing-workspace routing
  with unchanged envelope and five receipts. Each live case has `cleanup.json`
  recording that its test service/server was stopped. The installed-r9 versus
  Rust no-worker command/event/lifecycle trace passed at
  `stage5/differential-03/summary.json`, normalizing only timestamps and the
  new Rust-only service-attention projection. Failed exploratory attempts
  `differential-01/02`, `systemd-01/02/03` and `cold-01/02` remain evidence;
  their harness setup defects were fixed in fresh namespaces.
- **Variances and disposition:** Rust initially returned an extra prose field
  for idempotency conflict; aligned with r9's exact code-only outcome and
  reran the differential. Rust adds versioned service attention to snapshots
  and bounded three-attempt unexpected-error reporting; this is an intended
  safety improvement, with durable `session.attention` events and no hidden
  repeated scheduler loop. The service uses bounded std threads and short
  SQLite WAL transactions rather than the proposal's Tokio/writer-thread
  implementation. The scoped tests demonstrated responsive IPC during fake
  execution, but not slow-client, database-contention or output-pressure
  behavior. Retention for asynchronous native workloads requires the focused
  checks in [the closeout review](rust-migration-review-addendum.md); no
  measured advantage or compatibility blocker was recorded. First cold PTY
  attempt reported zero terminal size; a properly sized PTY passed. The first systemd attempt exceeded Unix socket
  path length; the test now uses a short private runtime path. These were
  fixture/environment issues, not silently discarded behavior differences.
- **Remaining Python and next stage:** the Rust release's compatibility CLI,
  review plugin, bundle builder/installer and several maintenance commands
  still route to Python; no Python-free distribution is claimed. Stage 6 must
  replace or explicitly refuse those paths, install a resource-complete Rust
  release without venv/wheel/Python, verify release-specific routing and
  historical read-only compatibility, and repeat integrated isolated simulated
  acceptance. Native adapter restrictions are offline only and authenticated
  model calls remain blocked. Physical Ghostty/Herdr input and appearance
  still need operator observation after final installation.

## Stage 5 start boundary — project service, lifecycle and accepted TUI (2026-10-12)

Code checkpoint: `c5463e7` with a clean worktree. Installed r9 manifest
SHA-256 remains `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
Only isolated Rust-owned projects, sockets, units and Herdr resources may be
created. Existing operator projects and Python services stay bound to their
legacy releases. Before any schema-2 test upgrade, retain a consistent SQLite
backup and file/binding inventory; a test reset uses a fresh namespace. Code
rollback is Stage 4, but a project that accepts Rust work cannot be safely
pointed at Python or restored from an older snapshot. Release and state
rollback are project-specific; an unrelated unavailable project cannot block
this one. Expected gates: command ID replay/stale refusal, nonblocking status
during fake calls, exactly one project owner and workflow, queued work,
exit/drain/resume fencing, bounded recovery/uncertainty, missing-workspace
handling, two independent projects with identical run IDs, and the accepted
TUI through the same IPC. This is a start boundary, not a passing result.

## Stage 4 — application-owned durable workflow: passing simulated gate (2026-10-12)

- **Scope/ownership:** Added schema-2 `ww_workflows` and `ww_effects` with a
  versioned drafting policy. The store reserves each frozen packet/effect in a
  SQLite transaction before execution, reads the Stage-3 immutable receipt,
  publishes artifact/ledger files immutably, then atomically applies the result
  once. `app_requests` and `app_responses` remain the human authority; answers
  can be recorded in one process and consumed in another. Final approval binds
  the candidate SHA-256 and decision revision. The engine handles the existing
  five slots/two critiques, early final review, multiple configured questions,
  finding coverage and v2 app-owned finding identities. It does not decode or
  modify LangGraph checkpoints. `workflow-probe` is an explicit-state test
  driver, not an installed project service.
- **Code/state checkpoint:** start at Stage-3 commit `901256a`; installed r9
  manifest SHA-256 remains
  `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
  Only new ignored `.local/rust-migration/stage4/` fixture databases were
  created. Existing historical/approved records, operator state and release
  bindings were untouched. Code rollback is to Stage 3. Schema-2 fixture
  rollback uses a fresh test namespace; never restore an older database over
  a newer receipt, answer or approval. A real schema-2 project will require
  an exclusive, consistent state backup and binding rollback before new work;
  after new Rust work, no blind down-migration to Python is allowed.
- **Verification/evidence:** `~/.cargo/bin/cargo test -p waywr-backend --locked`
  passed eight module, two runner-fault and five workflow-fault tests; Clippy
  with `-D warnings` passed. The final installed-r9/Rust differential and
  private-Herdr run is under
  `.local/rust-migration/stage4/live-final/summary.json`, with per-stage
  packets, receipts, artifacts, SQLite state, controller/server logs and
  `cleanup.json` (no owned process remains). The v1 five-stage run passed
  record-answer/exit/fresh-consume, repeated answer/result idempotency, stale
  approval refusal, exact candidate approval and restart. Installed r9 with
  equivalent inputs reached the same five stages, approval status, decision
  revision, finding statuses and final artifact SHA-256
  `71cd03a21bb26c4a6233fa995764249b23919a8392b27de1537d5721733ad268`.
  The v2 Rust fixture used two durable human questions and two app-owned
  finding IDs; both remained open despite worker `proposed-addressed` claims,
  and final review stayed pending. Offline fault tests covered omitted revision
  disposition and next-critic coverage packet, invalid output, uncertain
  dead-runner evidence, exhausted budget, early final, unknown definition/state
  versions, no third critique, and a failure after artifact publication but
  before state commit with exact saved-output reuse. Earlier exploratory live
  attempts `live-01/`, `live-02/` and `live-03/` remain retained.
- **Variances/impact/disposition:** Generated run/operation IDs and timestamps
  differ between the isolated Rust and r9 fixtures; comparison normalized only
  IDs when comparing result structure, while final artifact bytes/hashes,
  decisions, stage count and outcomes were checked directly. The r9 fake's
  known v2 finding-contract defect prevents a valid r9 v2 end-to-end fixture;
  Rust v2 is tested against the intended contract and Stage-2 regression.
  Schema-2 state explicitly refuses old/unknown definitions instead of
  interpreting graph blobs. A prelaunch missing workspace is operational;
  unresolved launched execution is uncertain. No unexplained meaningful
  difference remains in the scoped gate. The final review in the v2 fixture
  was **not** answered or approved.
- **Remaining Python/next stage:** The installed launcher, project service,
  IPC, lifecycle, systemd bootstrap, UI attachment, release routing and
  supporting CLI/plugin remain Python-backed. Stage 5 replaces the project
  service and lifecycle for a new isolated Rust-owned project, connects the
  accepted TUI through the existing wire contract, and proves two-project
  isolation and release-specific routing. No operator-service cutover or
  authenticated model call is authorized.

## Stage 4 start boundary — application workflow state (2026-10-12)

Code checkpoint: `901256a` with a clean worktree. The r9 installed manifest
remains `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
Stage 4 may create schema-2 SQLite records only in new, ignored, isolated
fixture roots. Before each schema/fault case, preserve a separate fixture copy
and hash inventory. A test reset uses a fresh namespace; it must not restore a
pre-result database over newer receipts or answers. Production projects and
bindings remain on Python. Code rollback is to Stage 3; fixture state may be
discarded only after its owned runner has stopped and unique evidence is
preserved. LangGraph checkpoint tables in historical stores stay opaque.

Expected gate before implementation: reserve effect identity before execution;
publish immutable artifacts before atomically applying a result; reuse the
same result after a crash; request/answer/approval ledgers remain authoritative;
multiple questions, decision revisions, review limit, budget, findings and
exact candidate hash persist; unknown versions, invalid output and ambiguous
execution fail closed. These are expectations, not completed verification.

## Stage 3 — Herdr runner, execution evidence and recovery boundary: passing simulated gate (2026-10-12)

- **Scope/ownership:** Added Rust immutable packet/operation/launch/claim/permit/
  dispatch/child/result/receipt records, Linux process and Herdr server/pane
  identity checks, project-state containment, owner-generation dispatch fence,
  native attempt reservation and text-only CLI launch preparation, visible
  Herdr runner with fake child and captured live stream, targeted cancellation,
  and explicit uncertain reconciliation. `operation-probe` is an isolated test
  driver, not an installed service route. Workflow state, approval and task
  scheduling still belong to the Python baseline until later stages.
- **Code/state checkpoint:** start at `32e4cc79cbe0f195e9444b963237acee12c5d9f3`;
  installed r9 manifest remains `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
  Only fresh ignored `.local/rust-migration/stage3/` state and private Herdr
  HOME/config/socket were used. No operator project, existing service, shared
  Herdr server, release binding or production schema changed. Code rollback is
  revert of the Stage 3 commit. State rollback is **not** deletion of a
  possibly launched operation: retain its launch/claim/child/receipt evidence,
  resolve a live owned child first, and only remove disposable fixtures after
  verified targeted cleanup. Preserve all fault evidence.
- **Verification/evidence:** `~/.cargo/bin/cargo test -p waywr-backend --locked`
  passed seven module and two operation-fault tests; Clippy `-D warnings`
  passed. The final isolated test summary and each case's immutable records,
  controller/server logs, retained pane output, diagnostics and cleanup audit
  are in `.local/rust-migration/stage3/live-final-3/`. Seven live cases passed:
  one visible completed fake worker with exactly one invocation and replay,
  targeted cancellation while an unrelated test runner remained alive, the
  installed-r9 baseline comparison, missing workspace before launch, runner
  death with surviving child and no redispatch, missing/replaced Herdr server,
  and completed-receipt reuse without any live server. Late cancellation
  returned the completed receipt unchanged. The r9 and Rust fake results were
  equal after normalizing only generated operation ID and packet hash; statuses
  and invocation counts matched. Earlier passing attempts remain under
  `live-01/`, `live-02/`, `live-final/` and `live-final-2/`; these are not
  operator runs. Every final test-owned server, runner and child was stopped;
  `live-final-3/cleanup.json` records zero live owned processes.
- **Variances/impact/disposition:** Different generated operation IDs yield
  different packet/result hashes; normalized result content and control fields
  matched r9. A missing server or workspace **before** launch is classified as
  operational, while a lost or changed server after a possible launch is
  uncertain. This prevents presenting definitely unlaunched work as possibly
  executed. A spawn error after durable dispatch is conservatively uncertain.
  No meaningful discrepancy in the tested execution outcomes remains. The
  Rust runner preserves the baseline's ten-minute native deadline and no
  automatic retry; it has not launched a native CLI. Native tool and permission
  isolation remains an offline contract, not runtime proof. No Rust service or
  application routing uses this runner yet.
- **Remaining Python/next stage:** Python still owns the installed project
  service, LangGraph workflow state, task admissions, approvals, IPC and
  release routing. Stage 4 adds application-owned Rust workflow persistence,
  effect/result idempotency, decisions and exact-artifact approvals; compare
  complete simulated flows and crash boundaries on isolated copies before
  routing any project. The Stage 3 runner and r9 comparator remain available.

## Stage 3 start boundary — runner and Herdr execution (2026-10-12)

Code checkpoint: `32e4cc79cbe0f195e9444b963237acee12c5d9f3`; working tree
clean at entry. Preserved r9 manifest SHA-256 remains
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
No production schema, project binding or operator service will be changed in
this stage. New operation records and processes must live under a fresh ignored
`.local/rust-migration/stage3/` fixture with a private Herdr HOME/config/runtime
and socket. Rollback is code revert plus targeted cleanup of verified
test-owned identities. Keep uncertain/failure evidence. Never restore a pretest
snapshot over an operation whose external execution may have happened.

Expected outcomes before testing: no second pane/call after a possibly sent
launch; a claim without permit cannot execute; validated receipt can be reused
without a live server; lost claim, changed server, stale owner or dead runner
with live child stops as uncertain; a verified owned cancellation affects only
that child and a completed receipt can win a late cancel; missing output or a
timeout cannot authorize retry; a frozen native attempt is reserved before
spawn and a duplicate/sixth attempt is refused. Workflow review limits are
Stage 4 policy, not a runner permission. Offline fault permutations precede
isolated Herdr process checks. This paragraph records expectations, not results.

## Stage 2 — policy, context, packets and adapters: passing gate (2026-10-12)

- **Scope/ownership:** Added inert Rust modules for canonical packet encoding,
  explicit local and directed foreign context selection, grant revocation/stale
  checks, inherited decisions, question and output validation, finding identity
  and assessment, review deltas, frozen native plans, Claude/Codex text-only
  CLI schemas/arguments/prompts/event interpretation, and fake coordinator and
  worker results. None is routed into the installed service or TUI. A future
  project service remains the single authority for approvals and dispatch;
  these functions cannot start a worker or write an approval. The six static
  prompt instruction variants under `backend/resources/prompt-contract-v1/`
  were extracted from installed r9 and embedded in Rust, with full prompt
  bytes compared. TOML 1.1.8 adds seven locked packages; no existing package
  was upgraded. The separate toolchain pin is commit `5da61b7`.
- **Code/state checkpoint:** start at `5da61b7` (stage 1 `12cc4f5`), with
  installed r9 manifest SHA-256
  `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
  Stage 2 has no production routing, state schema, service or release binding
  change. Every SQL mutation in a fault fixture used a fresh ignored project
  copy. Original saved packets, receipts and source fixtures were rehashed
  unchanged. A failed exploratory packet comparator briefly opened SQLite
  transient WAL/SHM files on the stage-1 copy; after unbinding, its complete
  file/hash inventory matched the preserved source. The corrected harness uses
  another copy and checks both source and copy after connection shutdown.
- **Verification/evidence:** `~/.cargo/bin/cargo test -p waywr-backend --locked`
  passed six focused unit tests; Clippy with `-D warnings` passed. Installed r9
  private Python `-I` versus the Rust parity probe passed final isolated cases
  under ignored `.local/rust-migration/stage2/final-{policy,access,local,packets,
  v2,adapters,delta,output,fake}/`: 13 policy, 15 directed access, 14 local
  context, five frozen packet byte/hash checks, native plan plus two version-2
  packets, six adapter contract variants with stub versions/events/final files,
  120 difflib deltas including 200+ lines, 15 output/coordinator fault cases,
  and five fake workers plus one fake coordinator. `*-comparison.json` files
  retain case outcomes, digests and baseline provenance; final fake counters
  each equal one. Saved evidence and failed exploratory attempts remain under
  the same ignored stage-2 root. No authenticated CLI or Herdr service ran.
- **Variances/impact/disposition:** The installed Python fake emits legacy
  `findings`/`dispositions` on a version-2 finding packet, which its own
  validator rejects. Rust fake emits `new_findings`/`assessments` for that
  contract; the output regression verifies acceptance. This is an intentional
  fix enabling simulated version-2 testing, not a change to any saved result.
  The Python local context selector already rejects a foreign-labelled run;
  an exploratory assumption to the contrary was corrected before the final
  comparison. The exact schema `required` array order and finding insertion
  order affect packet bytes: schema order now matches all tested r9 versions,
  and new Rust workflow state must persist `finding_order` explicitly rather
  than relying on sorted JSON maps. The nonlexical-order version-2 packet case
  is its regression check. No unexplained parity mismatch remains in the
  scoped stage-2 cases. Native CLI tool isolation is still unverified at
  runtime; only stubbed/saved outputs were used.
- **Rollback:** Revert the Stage 2 code commit if needed, leaving stage 1 and
  the pinned toolchain intact. There is no application state migration or
  project binding to roll back. Preserve ignored comparison/failure evidence;
  never restore a fixture snapshot over newly completed operator work.
- **Remaining Python/next stage:** All installed service, runner, workflow,
  session, CLI and release paths still use Python; the Rust policy is inert.
  Stage 3 ports Herdr-visible runner ownership/claim/permit/receipt/cancel and
  uncertain recovery, first with offline fault cases and then an isolated
  test-owned Herdr server. The accepted TUI and operator services remain
  untouched.

## Stage 2 toolchain checkpoint — user-local Rust 1.99.0 (2026-10-12)

- **Code checkpoint:** stage-1 commit `12cc4f5deb3d17f49a73126dbe5ace1e3df971f9`.
  Stage-2 policy work remains inert; no production routing or state writes.
- **Selection:** temporary and permanent toolchains both report `rustc 1.99.0
  (b940084d7)` and `cargo 1.99.0 (5f94df4789)` for
  `x86_64-unknown-linux-gnu`. Existing manifests declared no `rust-version`;
  no toolchain file existed. Installed permanent `1.99.0` through user-local
  rustup and pinned `rust-toolchain.toml`. `Cargo.lock` was unchanged by this
  toolchain-switch commit; it changed elsewhere in the migration.
- **Build workflow:** use `~/.cargo/bin/cargo` without the temporary
  `CARGO_HOME`/`RUSTUP_HOME` overrides. The temporary toolchain is retained,
  and the r9 baseline and services are untouched.
- **Verification:** `~/.cargo/bin/rustup show active-toolchain` selected
  `1.99.0-x86_64-unknown-linux-gnu` from the repository pin. The selected
  `rustc --version --verbose` and `cargo --version --verbose` matched the
  temporary compiler and Cargo commit hashes. `~/.cargo/bin/cargo test -p
  waywr-backend --locked` passed with the permanent rustup and default Cargo
  home; no dependency or lockfile changes were made. There are currently no
  backend unit tests, so this is a compiler/build check, not a Stage 2 parity
  gate. Stage 1's behavioral harness remains separate evidence.
- **Rollback:** revert only the pin and documentation if needed; no state migration or
  release binding changed. Do not delete the temporary toolchain as part of
  rollback or before confirming no active build needs it.

## Stage 1 — read-only legacy reader: passing gate (2026-10-12)

- **Scope/status:** Implemented workspace `backend/` crate and `inspect-legacy`
  development command. It opens an explicit copied state root read-only, refuses
  unknown protocol/schema, preserves SQL JSON strings and file bytes, validates
  artifact paths/hashes/parents and request/response/export bindings, and exposes
  versioned finding ledgers. LangGraph checkpoint blobs remain opaque. No
  scheduler, mutation API, UI or release routing was added.
- **Baseline/code checkpoint:** `6067fd7f59887e0bb197d786609e43744865404e`;
  pinned installed r9 `bin/waywr` manifest SHA-256
  `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
  Stage commit is recorded as the next stage's starting checkpoint because this
  file cannot contain its own commit SHA.
- **State before changes:** No production/state schema writes. The preserved
  `final-flow10/project` simulated fixture (131 regular files) was copied to
  ignored `.local/rust-migration/stage1/fixture/`; source/copy hashes are in
  `copy-manifest.json`. Every behavioral case uses its own fresh copy. The
  project/operator state and r9 installation were untouched.
- **Verification:** `cargo test -p waywr-backend` built; zero Rust unit tests
  (behavioral verification is the comparison harness). `cargo test -p waywr --lib`
  passed 37 frontend tests, one pre-existing ignored probe, using the new root
  Cargo workspace lock. The installed r9 private Python plus the final Rust
  binary passed the seven-case comparison under ignored
  `.local/rust-migration/stage1/final-comparison/`: approved base run matches
  status, two response hashes, five artifact hashes, five finding-ledger hashes
  and five operation identities; both reject changed artifact, escaping path and
  conflicting decision export. Rust additionally refuses stale approval binding,
  unknown artifact schema and unknown state schema. `comparison.json`,
  `python-baseline.json` and `rust-inspection.json` retain details. Original
  fixture file hashes were checked unchanged. Earlier exploratory comparison
  outputs remain under `comparison/` and `comparison-v2/` as attempt evidence.
- **Differences:** Installed Python passive inspection accepts changed decision
  SHA in SQL when the export matches, though it conflicts with the original
  request; it also ignores an unfamiliar artifact schema or new state schema
  table in this read path. Rust refuses all three. Impact: stricter historical
  inspection, with no reinterpretation or mutation. Disposition: intentional
  fail-closed behavior required by versioned approval and release binding.
  Regression cases: `stale-approval`, `unknown-schema`, `unknown-state-schema`.
  Rust reports graph-only continuation as opaque rather than decoding MessagePack.
  Raw record bytes, including Unicode and numeric spellings, are hashed without
  Rust reserialization. No meaningful matched-field discrepancy remains.
- **Rollback:** Revert the stage-1 code commit if needed; there is no new state
  schema or release binding to roll back. Keep copied evidence. Any later stage
  with writes must use its own state-aware rollback, never restore this old
  snapshot over newly completed work.
- **Remaining Python/next stage:** All application Python dependencies and
  service/worker paths remain. Stage 2 ports deterministic policy, context,
  packet, finding, question and adapter logic behind the inert Rust crate,
  comparing only isolated inputs and stubbed CLIs. No native authentication.

## Framework assessment revision — 2026-10-12

Status: **planning only; implementation remains paused**.
Branch/source checkpoint: `refactor/rust-backend`,
`f8b342a6b14a9ecd6973c0b1a18de9e11a6da38a`.
Read [assessment](rust-orchestration-assessment.md) and revised
[proposal](rust-backend-migration-proposal.md). The assessment pins all six
external commits, licenses, source/test references and remaining unknowns.
Public checkouts/inventory used for static inspection are under
`/tmp/waywr-framework-assessment-yo98qzse/`; reproducible source links in the
report are the durable reference, not a runtime dependency on that scratch path.

- **Outcome:** retain the application-owned engine recommendation. Juncture and
  Smooth were the two deep workflow candidates; Rig was assessed separately for
  potential later coordinator use. AutoAgents, Polaris and OpenMirai were screened.
  `polaris-ai` is confirmed as `RadiantAILabs/polaris` by published crate metadata
  and the root manifest.
- **Material findings:** Juncture's inspected public resume reconstructs scheduling
  from entry and has a checkpoint serialization error path that reports success;
  Smooth graph progress is separate from conversation checkpoints; Rig's agent
  resume tests explicitly reissue in-flight calls. These are source/test-reading
  findings, not executed failure reproductions. App receipts, permits, approvals,
  budgets and isolation remain necessary with every candidate.
- **Plan changes:** state the framework decision's evidence; separate versioned
  workflow policy from generic durable step/effect records; strengthen stage 4's
  persistence/replay/refusal gate and per-project legacy retirement. Preserve all
  six stages, the read-only reader first task and automatic progression after
  later implementation approval. Explicitly forbid old snapshots overwriting new
  completed work and require impact/disposition/regressions for every variance.
- **Verification:** source, documentation, manifest and test inspection only.
  No application/third-party tests, dependency changes, builds, Python-to-Rust
  execution, workers, model calls, installations or service changes. Document
  links/diff checked separately; no parity gate has passed and no stage advanced.
- **Rollback/state:** docs-only revision; no schema, binding or evidence changes.
  All Python dependencies remain. r9 baseline and operator resources untouched;
  running status not rechecked.
- **Exact next action:** await review/implementation authorization. Then establish
  stage-1 baseline/state inventory and implement the read-only Rust record reader
  plus comparison contract. No third-party prototype is required first. Continue
  subsequent stages automatically only under the approved execution contract.

## Planning baseline — 2026-10-12

Status: **proposal awaiting review; implementation has not started**.
Branch: `refactor/rust-backend`.
Inspected source: `d14765ba03e739f35f9450258e03bbd4f2dbe923`.
Scope and execution contract: [migration proposal](rust-backend-migration-proposal.md).

Comparison baseline:

- Source checkpoint `79615442f94326787e81a137ffeeeaff8e476d1d`, merged
  checkpoint `ae367f2698f643c7b2d9c6ed91c8ad4d65d9b99b`.
- Tag `checkpoint/milestone-6-simulated-rust-migration-20261012`.
- Installed executable
  `.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr`.
- Rechecked manifest SHA-256:
  `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
  The enclosing default launcher selects r5 and is not the comparison baseline.

Inspection performed: source/dependency inventory; installed LangGraph serializer
source; read-only schema/type counts from preserved `final-flow10` SQLite;
primary Rust dependency documentation. The archive contains 11 MessagePack
checkpoints, not language-neutral workflow JSON. No broad tests, new acceptance
environments, model calls or runtime mutations were performed. All Python
responsibilities remain. The proposed first step is the read-only record/parity
boundary; there is no Rust backend implementation to roll back yet.

Baseline nuances to track during implementation:

| Item | Evidence / disposition |
|---|---|
| Legacy canonical JSON differs from compact session command serialization | `storage.canonical`, `session_service.Store`; preserve byte-level identities, define fixture codecs in stage 1. Unresolved implementation work. |
| Constant handshake owner generation | `Service.handle` returns 1; actual owner advances in `session_service.main`. Correct diagnostic generation in stage 5 with regression coverage. |
| Recovered coordinator inspection selects original call directory | `task_detail` versus `_coordinate_loop` replacement path. Preserve both original and effective evidence; correct reporting in stage 5. |
| Historical graph continuation is framework-specific | `SqliteSaver`, JsonPlus typed blobs and `workflow.drive`. Proposed boundary: read-only history, quiescent upgrade, no automatic in-flight conversion. Awaiting plan approval. |
| Current coordinator is fake/task-proposal-only | Preserve as the migration baseline; conversation/provider expansion deferred. |
| Native isolation remains unproven by simulated acceptance | Preserve restrictions; compare adapters offline; no authenticated acceptance authorized. |

## Required entry for every implementation stage

Copy this structure when work is authorized. Keep failed-attempt evidence and
do not mark a stage passed until every material discrepancy is explained.

- **Stage / status / scope:** components migrated and deliberately excluded.
- **Baseline / start checkpoint / resulting commits:** full SHAs, release and
  manifest hashes. Record a resulting SHA in the next commit/log entry.
- **State before changes:** schema versions, project/fixture identities,
  consistent SQLite backup, file/binding inventories and verification hashes.
- **Verification:** equivalent-input baseline/Rust comparisons, normalized
  fields and rationale, commands, outcomes, stable ignored evidence locations.
  Separate automated results from operator observations.
- **Differences:** every observed variance/nuance with evidence and impact,
  whether baseline defect or intentional contract, corrected / intentionally
  retained / unresolved, with rationale and regression coverage. Never normalize
  away authority or counts or weaken expectations to obtain a pass.
- **Rollback:** exact code checkpoint and state/files/binding procedure;
  applicability before/after new execution; evidence retained after rollback.
  Never overwrite newly completed work with an old snapshot or point a legacy
  release at incompatible state.
- **Remaining Python:** runtime/build/supporting dependencies and removal stage.
- **Next step / blockers:** continue automatically on parity pass; otherwise
  diagnose within stage or record the concrete reason approval/access is needed.

Future evidence belongs under ignored `.local/rust-migration/<stage>/<case>/`,
with manifests and baseline/Rust outputs. This planning turn has not created
that acceptance tree. Historical evidence stays at its original recorded paths.
## Corrective implementation checkpoint — 2026-10-12

Starting code checkpoint: `cc721d649c29f9166c877852bacdf6c81ebfa2c8`; predeclared stress limits and the recovery boundary were committed at `644b9c6`. The prior Stage-5/6 simulated passes are **partial coverage**, not complete runtime or CLI acceptance. No operator project, bound release, or retained r9 baseline was changed.

The live project service now uses a two-worker Tokio runtime for Unix sockets, deadlines, signals, client draining and scheduler timers. Synchronous policy/Herdr calls run in a bounded blocking pool (16 concurrent client dispatches, 64 clients total); the per-project SQLite writer has a 64-request nonblocking admission queue. SQLite busy maps to `busy` before durable acceptance; a writer failure instructs command-ID reconciliation. Runner stdout and stderr use a 16-chunk channel (64 KiB chunks), a combined 32 MiB transcript limit, and a nonblocking pane display write. Exceeding the transcript limit terminates the verified child, retains `output-limit.json`, and records a failed receipt without retry. A stalled pane display leaves the transcript/receipt authoritative and records `display-degraded.json`. The Herdr create call occurs after immutable launch intent and outside dispatch/fence locks. Startup schema initialization is still synchronous before socket acceptance; live commands use the writer. The accepted production TUI was not redesigned.

CLI restoration is in progress in the same checkpoint: Clap rejects malformed syntax before routing; explicit run IDs, record-only answers with an immutable release marker, run/task lookup, verified artifact and transcript inspection, worker focus, service-routed decisions/cancellation/rebind, simulated start, native-safe preflight, and project location maintenance are implemented. Native admission remains blocked. The ordinary-command differential and installed candidate are not yet complete; none of these source-level checks establishes native isolation.

Focused verification completed so far: `cargo test --workspace --locked --offline` passed before the latest race/display additions; later targeted tests passed for record-only answer reopening, explicit run ID, a 32-client slow-reader status bound, SQLite lock/replay under 500 ms, stalled Herdr I/O status under one second, nonblocking terminal display under one second, and concurrent answer/cancel/exit with a responsive read-only snapshot. `cargo clippy --workspace --all-targets --locked --offline` passed with only existing accepted TUI/module-order style lints excluded. The private-Herdr output flood at `.local/rust-migration/corrections/runner-flood-06/summary.json` recorded one invocation, a failed receipt, identical replay, a 32 MiB transcript, and 39,752 KiB peak RSS. The private-Herdr runner regression is `.local/rust-migration/corrections/runner-regression-01/summary.json`; all seven cases passed. Failed diagnostic attempts `runner-flood-01` through `03` remain retained; attempt 03 exposed a Tokio reactor-context panic, corrected before the passing 04 and 06 runs. These are isolated simulated checks, not authenticated or physical-terminal acceptance.

State rollback boundary: all new fixture states live in ignored `.local/` or temporary test roots. The new `session_run_requests` table and `record_only` response field are additive on Rust schema-2 states. A code-only rollback is safe for projects without pending explicitly named runs or held record-only answers; otherwise keep the compatible release until those records are consumed and quiescent. Never point an unfinished LangGraph run at Rust or overwrite completed state with a snapshot. Historical r9 records remain byte-for-byte on their bound release. This paragraph records the in-progress checkpoint; the passing simulated corrective gate and its remaining acceptance limits are recorded above.
