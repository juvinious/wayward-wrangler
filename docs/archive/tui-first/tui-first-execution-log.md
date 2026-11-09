# TUI-first execution transition log

Approved baseline: `dafb4f8451c725611091e0b3b264ffe73e02b107`.
Branch: `refactor/supervisor-owned-execution`.
Source contract: [transition proposal](tui-first-optional-herdr-proposal.md).
This log records implementation checkpoints and preserves failed-attempt evidence.

## Checkpoint 4 — installed simulated compatibility acceptance (2026-10-13)

- **Scope and rollback checkpoint:** based on checkpoint 3 `0dc2203`.
  The only application change extends project-scoped release maintenance to
  schema-2/config and schema-3/state host projects. It validates exact
  initialized identity, quiescence, host receipts and empty retained units,
  fences startup, and binds rollback to the complete state-file fingerprint
  plus SQLite revision. No historical state, operator binding or shared
  manager was changed. The explicit current/previous release pair is
  `rust-host-stage4-v3`/`rust-host-stage3-v5`; no schema downgrade occurs.
- **Passing final build:** `stage4-bundle-03/` and `stage4-install-01/releases/rust-host-stage4-v3/`
  with manifest SHA-256
  `5552fa1da5a756523974fa60543dad48ed18ec9203c70f067cffe404d5c95673`.
  `stage4-fixture-06/` passed installed fake two-project/same-run isolation,
  command replay, targeted cancellation, stopped-gate answer and restart,
  five single-count fake stages, stale-hash refusal, exact approval, distinct
  release routing and state-aware rollback refusal after accepted work.
  `stage4-view-fixture-01/` passed private optional Herdr inspection, and
  `stage4-pty-smoke-03/` passed installed TUI launch/resume/detach in an
  isolated PTY. Their exact test-owned services/server were stopped.
- **Source path evidence:** a full duplicate fixture from an external `/tmp`
  paired installation/projects passed. Its 375 regular evidence files were
  hash-verified into ignored `stage4-external-proof-01/`. The CLI file-access
  trace contains no source-checkout path; all ten host intents bind runners
  under the external prefix. `bwrap` could not establish a hard mount denial
  because uid mapping was denied, so that stronger check remains unavailable.
- **Offline regression:** `cargo test --workspace --locked --offline` passed
  frontend 39+16, backend 44, operation faults 3 and workflow faults 6.
  Prior isolated host/output fault evidence from checkpoints 1 and 3 remains
  the process-specific coverage; no broad native or logout/reboot claim follows.
- **Failed attempts and variance:** `stage4-fixture-01` to `03` were corrected
  harness assumptions (nullable pending gate, absent status release field,
  explicit resume after paused answer). `04` found the actual new-project
  maintenance rejection; `05` found the retained `active/exited` systemd
  behavior missing from its initial quiescence check. The corrected v3
  recognizes only a validated-receipt, empty-cgroup completion. PTY smoke
  01 lacked terminal dimensions; 02 opened but did not confirm Detach.
  All are retained with their failure/cleanup evidence; none authorizes
  retrying an ambiguous execution. This is an intended correction, not a
  difference normalized away.
- **State-aware rollback:** the source rollback point remains `0dc2203`, but
  a schema-3 fixture with accepted work must remain on a compatible bound
  release. Quiesce exact owned units and compare current state before changing
  a fixture binding. The maintenance fingerprint is intentionally conservative
  and may refuse after a harmless write. Never restore an old state snapshot
  over a newer command, answer, approval, receipt or output. See the
  [checkpoint-4 report](tui-first-checkpoint-4-report.md) for evidence matrix,
  limits and the single Fish hands-on project.

## Checkpoint 1a — isolated systemd host proof (2026-10-13)

- **Scope:** added `tests/host_lifecycle_probe.py`, a Python standard-library-only
  fake runner/child fixture. Python is a test harness, not an application runtime.
  It uses uniquely named `waywr-proof-*` transient user services, never the
  operator project service or shared Herdr. Each test-owned unit is stopped by
  exact name after capturing status. The user manager was not restarted or changed.
- **Properties tested:** `Type=exec`, `ExitType=cgroup`, `Restart=no`,
  `RemainAfterExit=yes`, `CollectMode=inactive`, `KillMode=control-group`,
  10-second start timeout and 3-second stop timeout. Verified claim PID,
  invocation ID, cgroup and retained `ExecStart` after fast exit.
- **Evidence:** ignored `.local/supervisor-owned-execution/host-proof-01/` passed
  initial lost-reply, supervisor-generation, child-survival, stale-permit,
  retention and targeted-cleanup checks. Expanded run `host-proof-02/` failed
  one harness assertion: a runner that refuses a retired attempt can exit
  before inspection, so systemd reports `MainPID=0`. Its retained `ExecStart`
  PID and `InvocationID` matched the claim. This is expected systemd behavior,
  not permission to trust an unverified missing PID. That failed run and its
  successful exact-unit cleanup are preserved.
- **Passing expanded evidence:** `host-proof-03/{summary,cleanup}.json` and
  per-case claim/permit/retirement/host/child records. It passed lost reply
  and supervisor replacement with one child, runner death with surviving child,
  pre-permit retirement, delayed runner start after retirement, linked fresh
  host, completed receipt with retained exited unit, cancellation/completion
  race, and targeted cancellation leaving an unrelated control unit active.
  All eight test units were stopped; their post-cleanup `LoadState` is
  `not-found`. No authenticated worker, project service or Herdr operation ran.
- **Meaning:** these results establish the proposed systemd property/lifetime
  assumptions in this user manager. They do not establish application fencing,
  exact service API response loss, native output recovery or full workflow parity.
  Integration must retain complete identity checks and uncertainty handling.
- **Rollback:** remove only the test harness source commit if needed. The ignored
  evidence remains; no application state/schema or operator release changed.
  The next task is a versioned Rust operation host with real claim/permit
  fencing and fake-only fault checks before routing a project to it.

## Checkpoint 1b — Rust host primitives and isolated fake runner (2026-10-13)

- **Scope:** pinned `zbus 5.19.0` with its Tokio feature and no default
  async-io runtime. Cargo.lock adds 26 packages; no existing package version
  changed. `systemd_host.rs` uses the supported user-manager D-Bus API with
  exact operation-unit names and bounded connect/start/status/stop requests.
  `host_attempt.rs` adds schema-2 logical operations with unique schema-1
  attempt intents, immutable claim/permit/commit/receipt evidence, an owner
  generation fence, explicit retirement and linked replacement only after
  positive no-launch proof. `waywr-runner host-run` executes in that unit and
  exits after publishing a receipt; it does not open or require Herdr.
- **Gate:** six offline fault tests passed for stale owner/permit, replay,
  unproven retirement refusal, linked proven-no-launch replacement, cancel
  before commitment, result/receipt reuse and cancellation after completion.
  The ignored isolated D-Bus adapter test passed. The separate ignored
  `systemd_host_operation` integration test passed with one real test-owned
  unit and one fake coordinator child. Its retained operation has one
  invocation counter, matching claim/permit/committed records, validated
  output and a completed receipt. The exact unit was stopped; no matching
  test unit remains. This tests the runner primitive, not project scheduling.
- **Evidence:** the application-runner pass is under ignored
  `.local/supervisor-owned-execution/dbus-runner-03/`; earlier passing
  incremental evidence is under `dbus-runner-01/` and `dbus-runner-02/`.
  Initial attempts exposed transient claim/permit lock contention and a
  missing attempt-local packet for the fake child. The first two attempts
  were ephemeral and their exact error outputs were recorded in the work
  session; their temporary directories were not retained. The corrected
  integration fixture retains the complete third attempt. A separate offline
  test exposed a repeated-observation timestamp conflict; immutable marker
  replay now compares stable fields and preserves the first timestamp.
- **Limit:** only the fake operation is routed through the host in this gate.
  The project scheduler, state/config versions, ordinary TUI/CLI attachment,
  native adapter budget mapping, bounded recovery episode and output index
  still use the old contract. No installed Herdr-free workflow or native
  isolation claim follows from this test. The host runner has no orphan stream
  reattachment and records unresolved execution as uncertain.
- **Rollback:** remove this commit's Rust adapter/attempt entry point and
  restore the prior Cargo.lock; no existing state or operator release was
  written. Do not delete ignored test evidence. Before a later rollback of
  an accepted schema-2 attempt, reconcile its unit and receipt; code rollback
  alone cannot interpret that state.

## Checkpoint 2 — project-hosted fake workflow and prelaunch recovery (2026-10-13)

- **Scope:** new project configuration schema 2 and project-local state schema
  3 select `systemd-user-v1`. The direct Rust TUI attaches in its invoking
  terminal without a Herdr lookup. The project service routes the fake
  coordinator and five sequential fake workflow stages through versioned
  host attempts. Existing schema-1 project and operation routing stays bound
  to its installed release. No native adapter call was enabled.
- **Execution:** project and operation fences, unique attempt units, exact
  runner claim/permit/commit/receipt checks and immutable invocation
  reservations survive supervisor replacement. A positively refused start
  creates a linked attempt after 1/2-second backoff, up to three automatic
  infrastructure attempts per episode. Ambiguous send/claim/child state
  remains uncertain. `session retry-start TASK_ID` can authorize three more
  infrastructure attempts only after all prior attempts are positively
  retired with no model invocation. It preserves the original task and frozen
  operation. An immutable grant precedes writer-backed reopening; replay of
  the same command ID repairs a crash between those steps. A new pointer's
  predecessor intent repairs a missing recovery link after a crash.
- **Installed evidence:** ignored `stage2-bundle-03/`, `stage2-install-03/`
  (`rust-host-stage2-v3`, manifest SHA-256
  `09329c6bf88162917a8dd371ac3e2f5497cfed50cf75d900258fbbb892bdfc54`)
  and `stage2-project-03/` under `.local/supervisor-owned-execution/`.
  With `HERDR_ENV=1` and an unusable inherited Herdr socket, installed `init`
  created no Herdr binding, `start` authorized a schema-2 plan, the service
  reached a human gate, and all five fake worker attempts completed with one
  invocation counter and validated receipt each. The answer was recorded
  while the private supervisor was stopped, then consumed after restart.
  Final fake approval bound `revision-2.md` at SHA-256
  `6204d29b0154454bd326095e1407c6a0d83de76e9abcf01e8531f8369365a70a`;
  it remained approved after another private supervisor restart. The private
  service was subsequently stopped through `session stop` and is inactive.
  This approval is a simulated fixture, not an operator approval.
- **TUI/lifecycle evidence:** private `stage2-project-02/` launched and
  resumed the accepted TUI in a tool PTY with poisoned Herdr variables,
  restored the invoking terminal on detach, and moved from `paused` to
  `running` with a new lifecycle generation. Focused host exit tests show an
  old exit ID cannot pause that newer generation. The older project remains
  bound to v2; `stage2-release-route.execve` shows the v3 launcher executed
  its bound v2 binary. No operator project was attached or migrated.
- **Corrections and failed attempts:** installed v1 reached `proposed` but
  `/plan` still called the legacy workspace check and returned `EINVAL`.
  Its ignored project/installation are retained. The route was corrected and
  covered by `host_task_plan_uses_project_execution_contract_without_herdr`;
  v2 and v3 reached authorization. Offline retry tests first exposed a float
  timestamp mismatch on replay and then a mistaken one-second expectation
  for the third attempt's two-second backoff. The timestamp became an
  integer millisecond field, and the test now checks the specified backoff.
  Both failed assertions were test-only and the source/evidence remain in Git
  and the work log. Seven host-attempt tests, coordinator/worker retry tests,
  host plan routing and host exit-generation tests passed. The old
  sub-second status/control and 500 ms SQLite-busy gates remain unchanged.
- **Limits:** output is retained, but labelled bounded TUI paging and optional
  Herdr views are checkpoint 3. The final fresh-install, two-project and
  rollback matrix is checkpoint 4. This test uses fake workers; it does not
  prove native context/tool isolation, native acceptance or physical Ghostty
  behavior. No logout/reboot survival or orphan stream reattachment is claimed.
- **Rollback:** the checkpoint-1b commit remains the code rollback point for
  source, but it cannot operate the new schema-3 fixture. Keep v3 installed
  for `stage2-project-03` or stop/quiesce that test project before changing
  its binding. Never restore an older state snapshot over the accepted fake
  decision or approval. Historical schema-2 projects and operator projects
  retain their existing bindings and records.

## Checkpoint 3 — bounded inspection and optional Herdr mirrors (2026-10-13)

- **Scope:** new host attempts publish bounded `transcript.log` plus a 32-byte
  stream index. The combined payload remains capped at 32 MiB; index metadata
  is capped at 65,536 records (2 MiB). The project IPC serves at most 64 KiB
  raw bytes/page and four simultaneous output readers. Pages name the attempt,
  stream, cursor, high-water mark, terminal status and output limit. Missing
  unpublished output returns an empty page; older combined transcripts are
  labelled as such. A pipe read or structured-output failure cannot become a
  successful receipt. The accepted TUI adds `/output`, a bounded cache, stage
  selection, cursor following and verified candidate paging. `workers`,
  `logs`, `observe` and `show` use the same project-scoped records. Named
  artifact wrappers are resolved through their immutable `manifest` member;
  the artifact hash is rechecked before each page. Approval remains blocked
  until the complete candidate has been loaded and verified.
- **Optional display:** `/herdr` or `observe RUN --operation STAGE --viewer
  herdr --socket PATH` explicitly opens a read-only viewer for an existing
  operation attempt. The service issues a project/task/stage/attempt-bound
  token and its `view` IPC channel refuses mutations or another operation's
  output. A saved create intent precedes `layout.apply`; repeated opening
  reconciles the token and installed executable before focusing a pane. A
  lost create reply with no verified pane refuses a second create. One
  explicitly requested workspace can retain a bootstrap shell. Completed
  output remains visible in its viewer tab until the user closes it. Closing
  the view or losing Herdr changes no execution record or invocation count.
  No ordinary startup, worker or status path contacts Herdr for schema-3
  projects, including with a stale inherited `HERDR_SOCKET_PATH`.
- **Automated evidence:** focused output-index tests cover labelled stdout
  and stderr, cursor replay, unpublished output, index exhaustion and a
  sleeping reader. The slow-reader fixture published one MiB within its
  predeclared two-second bound while the viewer slept. An isolated fake
  systemd runner produced over 32 MiB on both pipes; its one invocation
  ended with a failed receipt, exact 32 MiB retained payload and explicit
  output-limit record in ignored `stage3-output-flood-01/`. The frontend test
  covers duplicate output pages, wrong-task rejection, partial/stale artifact
  refusal, complete-candidate binding and 80×24/160×48 output rendering.
  Installed bundle `stage3-bundle-05/` (manifest SHA-256
  `158a47dcaabda6271c6dc76fa03fc699668d944d2100c374438d14baa29890e1`)
  was installed in `stage3-install-05/`. Fake fixture `stage3-fixture-08/`
  passed initial zero Herdr workspaces/tabs, one explicit inspection
  workspace plus bootstrap/view tabs, exact same-pane repeat open, read-only
  mutation refusal, verified draft artifact bytes, targeted view close, and
  retained logs after stopping its private Herdr server. Its draft invocation
  counter remained one. The exact private service and server were stopped;
  `cleanup.json` records that result. All paths are under ignored
  `.local/supervisor-owned-execution/` unless otherwise noted.
- **Offline regression nuance:** the first full workspace run exposed a
  one-ULP difference in a pre-existing test's fractional decision timestamp
  after JSON/SQLite reload. The test now compares all decision fields exactly
  and bounds only the timestamp representation within one millisecond. The
  record schema, saved decision and replay behavior were not changed. The
  focused replay test and the full offline workspace suite then passed:
  frontend 39+15, backend 44, operation faults 3, workflow faults 6; two
  opt-in process tests remained ignored by the default suite (the output
  flood test was executed separately and passed).
- **Failed attempts retained:** `stage3-fixture-01/` used a conflicting init
  directory; `02/` exceeded Unix socket length; `03/` exposed a listing race
  between frozen operation and attempt publication; `04/` requested logs
  before an attempt existed; `06/` found the named-artifact wrapper bug; `07/`
  showed completed read-only views exited and their Herdr tabs disappeared.
  Each corrected candidate has a distinct retained bundle/install/fixture.
  Test-owned services from failed fixtures were stopped by exact unit name.
  No historical run or operator project was modified.
- **Boundary:** this is fake-only automated evidence. Physical Ghostty input,
  live native-agent isolation, viewer behavior across user logout/reboot, and
  orphan stream reattachment remain unverified or excluded by the approved
  proposal. A view cannot resume a stopped project service; it reports the
  service unavailable. Local CLI retained inspection currently uses the
  project service and may start it through normal project routing; checkpoint
  4 must verify paused-service behavior and avoid equating an optional view
  with execution authority. The prior checkpoint-2 release remains installed
  for its bound fixture; do not relabel or migrate it.
- **Rollback:** checkpoint-2 `be09765` is the source rollback point. Any new
  schema-3 run with indexed output or view records stays on its stage-3
  release until inspected/quiesced; code rollback alone cannot make that state
  compatible. Never restore older snapshots over accepted decisions or
  approved artifacts. The next checkpoint is a fresh source-independent
  installed simulated acceptance, project/release isolation and state-aware
  rollback verification.
