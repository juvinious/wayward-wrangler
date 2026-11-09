> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Milestone 3 — recovery and failure testing

## Authorized scope and expected outcomes (recorded before testing)

Branch: `milestone-3-recovery`, from clean `main` at `463a14c`.
Only deterministic simulated workers and isolated state/fresh IDs are authorized.
No authenticated calls, completed-trial reruns, directory move, portability refactor,
shared-server disruption, merge or later milestone. NAS backups are out of scope.
AGENTS.md/CLAUDE.md remain unchanged because approved context pins their bytes;
the current user authorization supersedes their historical milestone-3 restriction.

| Boundary | Expected behavior before testing |
| --- | --- |
| Controller dies before launch intent | No dispatch exists; resume may create one worker. |
| Intent persisted, creation response absent, no runner claim | No blind creation retry. Uncertain; inspect launch intent and Herdr token/ownership before separately authorized resolution. |
| Creation succeeded, before/after runner claim, before permit | Existing runner must wait without invoking a worker. Fresh controller verifies same server, token, claim and binding before granting permission; no new pane. |
| Controller dies immediately after permit | Existing runner may finish; resume reconciles it or its receipt, never another call. |
| Result/artifact published before node checkpoint | Replay validates receipt and republishes identical artifact idempotently; completed stage call count stays one. |
| Answer/approval recorded or node applied before its checkpoint | Fresh resume consumes the same durable response once logically; no duplicated decisions, changed approval or repeated completed stages. Automated responses are test fixtures only. |
| Runner dies while child survives | Uncertain, no replacement runner/child or automatic adoption. Expose orphan evidence and operator action. Test cleanup signals only independently verified test-owned processes. |
| Cancellation races with completion | One immutable terminal receipt wins. If cancellation is observed before the runner selects its terminal outcome, cancellation may win and no graph adoption occurs; a valid committed completion remains authoritative. Never rewrite completed receipt/approval. |
| Missing/replaced server before unresolved reconciliation | Uncertain with evidence/action, no launch/permit against replacement server. Valid already-published receipt remains sufficient without a live server. |

Use offline fault permutations and actual Herdr-owned processes where feasible.
The tool shell does not inherit HERDR_ENV; it will not inspect/control the focused
session. Explicit authorization for an isolated server permits test-owned instances
only, with private HOME/config/runtime/socket paths. If isolation cannot be proven,
server identity changes stay simulated and the live gap is reported.

New harness paths must be explicitly configurable and fresh. They must not fall
back to the default `.local/state.sqlite`. Preserve original `.local` evidence and
shared session settings; record before/after integrity checks. This is a test-state
boundary, not completion of the installable multi-project portability plan.

## Observed results

**The scoped matrix passed using simulated workers.** 65 offline tests pass (50
existing + 15 focused regressions); `pip check` passes. Two isolated integration
executions produced 15 passing case records (12 distinct cases; three cancellation
cases were repeated with stronger isolation assertions). There were **17 simulated
worker invocations, each operation counter exactly 1, and zero native model calls**.
The five-stage fixture workflow accounts for five of those invocations. No original
trial was resumed. All 494 pre-existing local evidence files retain their hashes.

| Scenario | Observed outcome and evidence |
| --- | --- |
| Before intent | Hard-killed controller; fresh process launched one worker. `live-01/launch-before`: count 1. |
| Intent persisted before create | Controller killed with no claim; resumed attempt refused after its claim wait. No pane and count 0. `live-01/launch-after`. Lost-create response additionally covered offline. |
| Created pane before claim | Test runner paused before publishing its claim; controller killed, runner released, fresh controller reconciled same launch token/pane. No call before permit; final count 1. `live-01/create-after`. |
| Claimed, before permit | Runner alive with count 0; fresh controller validated existing binding and granted permit. Same pane/count 1. `live-01/permit-before`. |
| Immediately after permit | Controller killed; original worker completed. Repeated resume reused exact receipt, count 1. `live-01/permit-after`. |
| Artifact published, checkpoint absent | Hard kill after `artifacts.publish` returned but before graph node returned. Fresh status lacked completed draft; resume adopted original receipt/artifact bytes and reached choice after critique. Draft count 1. `live-01/checkpoint/before-replay.json`. |
| Decision and approval replay | Record-only fixture answer, kill after choice node computed update; fresh resume reached final gate with one decision/revision 1. Record-only fixture approval, kill after final gate computed update; repeated fresh resume ended approved with unchanged exports. All five counters 1. `live-01/checkpoint/final-status.json`. |
| Dead runner, live child | SIGKILL targeted saved runner identity; child was observed alive. Original controller and fresh resume refused without another call/receipt. Read-only diagnostic exposed both liveness values. Harness then terminated only the verified test child via pidfd. `live-01/orphan/diagnostic.json`. |
| Cancel before permission | Existing claim, no permit: runner recorded cancellation without spawning child. Resume reused cancellation, count 0. `cancellation-02/cancel-before-permit`. |
| Cancel during child execution | Cancellation targeted the actual child; cancelled receipt/count 1. Another retained test runner remained alive. `cancellation-02/cancel-child-after`. |
| Cancellation after result processing / after receipt | Runner paused just before result publication, after it had selected successful processing: completion won the late request. Published receipt also defeated a later cancel. Receipt stayed immutable, count 1. `cancellation-02/cancel-result-before` and `cancel-receipt-after`. |
| Missing and replaced Herdr server | Stopped only owned test server while operation lacked permit. Missing socket refused. New owned server at same socket had a new identity; resume refused without creating a pane/permit, count 0. An earlier valid receipt remained usable under replacement. `live-01/server-change` and suite `result.json`. |

The automated answer/approval are **test fixture data**, not human approval of this
branch or any real draft. No agent or worker was permitted to authorize decisions.

## Fixes and focused regression coverage

- `herdr_backend.server_matches()` makes unavailable-server recovery explicitly
  uncertain with an operator action; server boot/PID/start identity is rechecked
  immediately before permit publication, after claim/pane discovery. Offline tests
  cover each identity component changing at either check. This narrows the race;
  it does not claim atomicity across the socket and filesystem.
- `ensure()` refuses claim/dispatch/child/permit evidence with missing launch intent.
  It cannot create a replacement pane merely because `launch.json` disappeared.
- `inspect_operation()` supplies read-only execution diagnostics. CLI status now
  reports dead runner/live child, missing/replaced server and malformed evidence as
  uncertain with an action. It does not rewrite persisted graph state: the run's
  last saved status may still say running after SIGKILL; worker diagnostics show
  the observed execution state. Completed receipt validation precedes server checks.
- Runner `--state-dir` and `operation_path(..., local=...)` let the controller pass
  its explicit state root while retaining containment checks. The default location
  is unchanged. The harness sets this root before importing application modules;
  it never writes the original application database. This is narrowly scoped test
  configuration, not the multi-project installation/portability refactor.
- `tests/test_recovery_boundaries.py` adds 13 tests for launch loss, claims, duplicate
  panes, identity permutations, dead runner/live child, receipt corruption, terminal
  cancellation races and state containment. `tests/test_m3_cleanup.py` adds two
  tests proving dead/reused identities are not signalled by harness cleanup.
- `tests/m3_actor.py` contains subprocess fault barriers and deterministic fixture
  setup. Production runtime code has no kill/barrier hooks. `tests/milestone3.py`
  owns private server configuration and all created process identities.

No receipt/adoption rules were relaxed. `recovery.py`, native adapters, permissions,
existing approvals and historical absolute-path provenance were not changed.
No new orphan adoption, forced retry or automatic server rebind was added.

## Evidence, retained output and cleanup

Paths relative to this checkout (local-only, excluded from Git):

- `.local/milestone3/baseline.json`: hashes for 494 original files before tests.
- `.local/milestone3/verification.json`: unchanged-file check, invocation totals,
  zero native calls and cleanup summary.
- `.local/milestone3/live-01/`: first complete suite, `result.json`, progress,
  per-case state, controller logs, fault markers, process identities, invocation
  counters, receipts/transcripts/artifacts, diagnostic/checkpoint snapshots.
- `.local/milestone3/cancellation-02/`: focused cancellation checks with sentinel
  liveness assertions, zero-dispatch cancellation and captured Herdr pane output.
- Each suite's `isolation.json`, `server-*.json` and `server-*.log` identify private
  HOME/config/runtime/socket paths and the actual socket peer. `cleanup.json`
  reports no remaining owned runner or child. No live test panes remain; completed
  output is inspectable in the saved transcripts and captured pane output.

Herdr 0.9.3 supported isolated headless execution. The private-server technique was
checked against its available source at commit
`7b116c05bfda646af39d2524c54e70c751f57ee8`, `tests/server_headless.rs:spawn_server`.
Each test instance used a separate HOME, XDG config/runtime and explicit socket;
server shutdown first verified socket peer against the owned subprocess identity.
The initial isolation probe also exited. No active user server/session was stopped,
restarted or reconfigured. Temporary isolated Herdr state is retained under the
`/tmp/ww-m3-*` paths recorded in suite evidence; no authentication was copied there.

The harness uses pidfd plus boot/PID/start validation for targeted cleanup. It
refuses reused identities; it does not kill by process name, broad process group,
workspace display label or unverified historical PID. Shared configuration and
another project were untouched. No NAS action is required or performed.

## Inspect and reproduce (Fish)

Read existing output without launching anything:

```fish
cd ~/projects/example-workspace
vim docs/milestone-3-report.md
cat .local/milestone3/verification.json
cat .local/milestone3/live-01/result.json
cat .local/milestone3/live-01/orphan/diagnostic.json
```

Offline regression command (no native calls or Herdr launches):

```fish
.venv/bin/python -m unittest discover -s tests -p 'test_*.py' -q
.venv/bin/pip check
```

Explicit live test command, **creates only an isolated simulated test server**;
choose a fresh evidence path. Existing paths are refused, never cleared/reused:

```fish
.venv/bin/python tests/milestone3.py --evidence-root .local/milestone3/review-(date +%Y%m%d-%H%M%S)
```

`--cases cancellation` limits it to the focused cancellation checks. Completed test
runs are evidence, not pending human gates. Do not point the ordinary CLI at a test
run in the production database; inspection uses saved snapshots or the test actor
with explicit state. Operator action for real uncertainty: inspect status and
operation files, establish original server/runner/child ownership, preserve evidence,
and request explicit resolution. Do not delete claims or rerun a stage to clear it.

## Remaining limitations and review boundary

- Tests prove selected local process-crash boundaries, not every instruction window,
  power loss, filesystem/device failure, arbitrary PID/server races or provider-side
  exactly-once effects. Native CLI failure behavior remains untested here.
- Surviving children are detected but not automatically collected/adopted by the
  application; only the owned test harness cleans its processes. Missing launch
  evidence, absent claim and server replacement deliberately remain uncertain.
- Cancellation is best effort: a request can arrive after the runner's successful
  outcome check and lose to completion before receipt publication. The immutable
  receipt is authoritative; a cancel request alone is not proof of cancellation.
- Isolation/state-path configuration does not make the installed utility portable.
  Source-derived installation paths, canonical context-path coupling and plugin
  packaging remain the separate scoped portability follow-up. Recovery planning
  must retain that requirement without rewriting frozen evidence.
- Completed text output survives in files; this suite intentionally stops its owned
  test servers, so test panes are not retained as live navigation targets.

Work is committed on the review branch only. No merge, PR, new milestone or native
trial is authorized by automated fixture approval. Review these changes before
choosing the next task.
