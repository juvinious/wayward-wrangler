> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# TUI-first checkpoint 4 — installed simulated acceptance

## v3 follow-up retest: answer, review and panel navigation (2026-10-13)

The operator completed and detached a v2 simulated task, then reported three
interaction defects: an Answer panel displayed **Save answer** while final
review was pending; content panels could scroll beyond the rendered end; and
Approve/Reject appeared before the required candidate-read step. This was
operator feedback before the v3 retest. The existing v2 project and
its approved task remain bound to v2 and were not reinitialized or rebound.

### Operator hands-on result

On 2026-10-13 the operator used the v3 fixture in a terminal, completed the
simulated workflow, and reported that the requested UI changes are present and
verified. The supplied screenshot shows the selected weather-page request as
**approved** with **View result** available. This is hands-on acceptance of
the corrected simulated interaction, separate from the automated checks below.
The operator did not report a v3 detach/resume check or authenticated agent
use. Approval is of the fake workflow's artifact; it does not verify or create
a working weather page.

The project remains bound to v3. Its retained candidate is under ignored
`discoverability-hands-on-v3/project/.wayward-wrangler/state/runs/<private-record>/artifacts/revision-2.md`.
The exact project service was active when checked after the operator's report;
it was not stopped or changed.

The isolated `rust-host-discoverability-v3` release is installed at
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`.
Manifest SHA-256:
`c0bf66e31579cb44a04cb97f3701ff47ccaff4fd74dedf1190445240e1e0e5a1`.
It is not the installation default or a global command. It adds one-line
Up/Down scrolling in content panels, clamps PageUp/PageDown and arrow scrolling
to the rendered content end, and reclamps after a detail or terminal-size
change. Up/Down still select question choices, tasks and input destinations.
When no current choice question exists, Answer shows no Save action and cannot
emit an answer. Final review shows only Read candidate until the exact
candidate is fully loaded; only then are Approve and Reject visible and
selectable. A changed candidate removes those actions again. Backend
decision validation and historical records are unchanged.

Focused frontend verification passed: 44 library tests and 16 binary tests,
with the preexisting 30-second input probe ignored. New regressions cover
unavailable Answer, valid choice selection, no answer intent, one-line scroll,
bottom clamp, content shrink, and decision-action visibility before and after
reading. The installed fake-only PTY run in ignored
`discoverability-check-07/` passed scope, question, exact-candidate read,
approval, result inspection and detach with terminal restoration. It used one
fake coordinator plus five fake stages. Its exact test-owned service is
inactive per `cleanup.json`; no Herdr socket was contacted. This automated
check alone did not establish physical terminal usability or native-agent
behavior; the operator's later interaction is recorded separately above.

A separate fake-only project was initialized under ignored
`discoverability-hands-on-v3/project/` and is bound to v3. It now retains the
operator's approved simulated run. The private Fish wrapper was syntax
checked and identified the paired Rust release. To evaluate from Fish:

```fish
cd ~/projects/example-workspace/.local/supervisor-owned-execution/discoverability-hands-on-v3/project
../waywr.fish
```

Use `../waywr.fish resume` to reconnect and `../waywr.fish status` to inspect.
The deterministic fake coordinator proposes the submitted text, and the
retention question is scripted. At review, **Read candidate** is the first
action; after the candidate is fully loaded, Esc returns to distinct
Approve/Reject actions. Approval completes this simulated task; it does not
build the requested application. The operator subsequently accepted this
simulated hands-on interaction as recorded above.

## Preserved v2 discoverability candidate (2026-10-13)

The operator's prior hands-on check **did not accept** next-action discovery.
The focused correction is installed as the fake-only, isolated
`rust-host-discoverability-v2` release at
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v2/bin/waywr`.
Its manifest SHA-256 is
`40a71f30914a260ecaa7f875efad0cb179b53affc428b22b6e12bd82eb091d20`.
The original `rust-host-stage4-v3` installation and operator evaluation
project remain unchanged and bound together. Do not rebind or restart that
project for this retest.

The selected task now offers **Review scope**, **Answer question**, **Review
candidate**, or **View result** from project-service state. Enter with an empty
composer, or Enter on the focused activity action, opens the relevant panel;
nonempty text still submits normally. Opening never authorizes, answers or
approves. The final-review panel focuses **Read** first; after the exact
candidate is fully loaded, returning focuses a separate **Approve** action.
The locally verified read is retained only for the same project/task/request
ID/artifact hash. Task detail freshness checks both task version and state
revision. A new approval request or candidate invalidates the marker; service
approval validation and stale-candidate refusal remain authoritative.

The checked-in protocol-v1 envelope now includes the implemented minor-1
methods and capability groups. `operation.retry_start` remains a durable
command action; explicit Herdr view opening uses the existing
prepare/send/bind handshake. This IPC reconciliation was committed separately
as `6a5700e`; no service protocol behavior changed.

Focused verification: `cargo test -p waywr --locked --offline` passed 42
frontend library tests and 16 binary tests (one existing timing probe ignored);
`python3 tests/session_schema_consistency.py` passed. New frontend regressions
cover empty versus nonempty Enter, focused activity, no implicit decision,
80-column action hint, draft/focus/reading-position preservation, unchanged
versus changed exact candidate, same task version with a newer state revision,
and one intent per explicit approval. Existing outbox and backend tests retain
durable command replay and stale-approval coverage. The installed fake-only PTY
check in ignored `discoverability-check-06/` passed scope authorization,
scripted question, exact-candidate read, approval, result inspection, detach,
terminal restoration and no Herdr contact. Its summary records six fake
invocation files: one coordinator plus five worker stages. The test-owned
service was stopped by exact unit name; `cleanup.json` says inactive.

Failed isolated fixtures `discoverability-check-{01..05}/` remain as evidence.
Fixture 02 exposed the real stale-gate defect: task version stayed constant
while state revision advanced, so the old question reopened at final review.
Fixtures 01, 03, 04 and 05 exposed PTY harness timing or incremental ANSI-text
matching assumptions; their durable project records and traces remain
inspectable. Each test-owned unit was stopped. The v1 correction bundle and
installation are retained only for provenance; v2 is the retest candidate.
Automated PTY results do **not** establish Ghostty/Herdr operator acceptance.

### Fresh reusable retest project

The ignored `discoverability-hands-on/project/` is newly initialized, has
fake author/critic roles and no active service, and is bound to v2. The small
ignored `discoverability-hands-on/waywr.fish` wrapper pins the candidate and
private environment. It was syntax-checked with Fish and its `--backend-info`
reported `rust-host-discoverability-v2`. From Fish:

```fish
cd ~/projects/example-workspace/.local/supervisor-owned-execution/discoverability-hands-on/project
../waywr.fish
```

Submit a short request, then follow the selected-task action label with Enter
on the empty composer. In **Review scope**, Tab then Enter authorizes. At the
scripted question, choose a listed option, then Tab and Enter saves it. At
**Review candidate**, Enter reads the exact artifact; Esc returns to the
separate approval/rejection actions. The coordinator and workers are
deterministic fake implementations; the retention question is scripted and
unrelated to the wording of the request. No browser application is built.
To reconnect or inspect without memorized installation paths, use
`../waywr.fish resume` or `../waywr.fish status` from the same directory.
`../waywr.fish session stop` stops only this fixture's project service after
inspection. Preserve its project directory for decisions, output and receipts.
Physical typing, focus and action clarity remain for the operator to verify.

## Preserved checkpoint-4 candidate and prior operator project

Approved base: `dafb4f8451c725611091e0b3b264ffe73e02b107`.
Implementation branch: `refactor/supervisor-owned-execution`.
This is **simulated implementation acceptance**, not native-agent or operator
approval. Existing operator projects, legacy releases, historical records and
the shared Herdr server were not changed.

## Candidate and correction

The preserved checkpoint-4 paired Rust candidate is the ignored installation
`.local/supervisor-owned-execution/stage4-install-01/releases/rust-host-stage4-v3/`.
Its manifest SHA-256 is
`5552fa1da5a756523974fa60543dad48ed18ec9203c70f067cffe404d5c95673`.
The installed frontend, service and runner use schema-3 host state and require
no application Python runtime. The retained stage-3 release is installed
beside it for explicit release routing. Native admission remains blocked.

Checkpoint 4 found that release maintenance still recognized only the former
schema-1 project/state. It now accepts only an exact schema-2
`systemd-user-v1` project and compatible schema-3 Rust releases. It fences
service startup, rejects unresolved attempts, validates receipts, verifies
empty host units, and records a full state-file fingerprint alongside the
SQLite revision. Rollback changes only the binding; it refuses if either
the state revision or fingerprint changed. The full digest is deliberately
conservative: a harmless state write may require forward repair instead of
rollback. No snapshot is restored over newer work.

The isolated host proof had already established that a completed unit may be
retained as `active/exited` with `MainPID=0` and an empty cgroup. The first
maintenance correction rejected that verified state. The final correction
allows it only when the exact unit properties and invocation ID match a
validated application receipt. A running, unknown or unresolved unit still
refuses the release change. The uninitialized-project check also now accepts
the exact local project identity, host contract and empty startup lock written
by `init`.

## Automated gates

| Gate | Result and retained evidence |
| --- | --- |
| Offline Rust regression | `cargo test --workspace --locked --offline`: frontend 39+16 passing; backend 44 passing; operation faults 3 and workflow faults 6 passing. One frontend timing probe, one D-Bus adapter test and two process-host tests remain opt-in. Those host/output process tests passed in checkpoints 1 and 3; they were not rerun merely for the maintenance correction. |
| Installed two-project workflow | `stage4-fixture-06/summary.json`: A and B used the same `shared-01` run ID under different keys and releases. Both reached separate human gates; neither packet contained the other project's path or key. Replaying A's accepted request reused its coordinator call. Cancelling B left A's choice pending. A stopped and restarted at the gate, consumed its answer, used exactly five fake worker calls, refused a stale hash and accepted an exact-hash fixture approval. |
| Release routing and rollback | The v3 launcher routed B to bound stage-3 v5. A third disposable host project upgraded, rolled back before work, upgraded again, accepted and cancelled work, then refused rollback without changing its release binding. All exact fixture units were inactive in `stage4-fixture-06/cleanup.json`. |
| Optional Herdr view | `stage4-view-fixture-01/`: the v3 release began with zero private workspaces/tabs, created a view only on explicit request, reused the same pane, refused view-channel mutation, retained output after view/server close, and kept the invocation counter at one. The private server and selected project service were stopped. |
| Installed TUI | `stage4-pty-smoke-03/`: the installed TUI launched and resumed in a sized isolated PTY with poisoned Herdr variables. Ctrl-Q opened the detach confirmation; selecting Detach restored the alternate screen both times. No Herdr endpoint/view was created. Its project service was stopped by exact name. This is automated PTY evidence, not a Ghostty/Herdr operator observation. |
| External-source-path check | The complete two-project fixture also passed from `/tmp/ww-stage4-paired-proof-01/` with all projects and services under `/tmp`. Its command environment omitted `PYTHONPATH` and Herdr from `PATH`. Ten host intents bound runners in the external release. A `strace -f -e trace=file` status check saw no source-checkout path. All 375 regular files from `/tmp/ww-stage4-external-fixture-01/` were copied byte-identically to ignored `stage4-external-proof-01/`; see `copy-verification.json` and `source-independence.json`. |

All relative evidence names in this report are under ignored
`.local/supervisor-owned-execution/`. Failed fixtures `stage4-fixture-{01..05}/`
and `stage4-pty-smoke-{01,02}/` remain inspectable. Fixture 01 had a null
pending-gate harness assumption; 02 asserted an unavailable `status`
`release_id` field; 03 omitted the explicit resume transition after saving
an answer while paused. Fixture 04 exposed the application’s schema-2 init
maintenance refusal. Fixture 05 exposed the retained `active/exited` host
unit policy defect. The first PTY attempt used a zero-size terminal; the
second reached the detach panel but did not confirm it. Each failed fixture's
test-owned service was stopped; no old evidence was rewritten.

The mount-namespace hard-denial experiment was unavailable:
`bwrap` returned `setting up uid map: Permission denied`. External installation,
external project paths, host-runner provenance and file-access tracing support
source independence, but do not prove a mandatory-access denial for every
systemd service read. The installed bundle contains its runtime resources and
the verified ordinary path uses no Python service, venv or wheel.

## Limits and rollback

Authenticated Claude/Codex execution and native context/tool isolation have
not been accepted. The later operator screenshots show one simulated TUI task
reaching an approved display state, with a discoverability failure recorded
below; they do not verify all Ghostty/Herdr typing and paste behavior or the
optional view action. Logout/reboot process survival,
orphan stream reattachment and interactive Herdr attachment are not promised.
Local CLI output inspection may start an inactive project service but does not
itself resume a paused session; optional Herdr views require an existing
service and never start execution.

Source rollback is the checkpoint-3 commit. It is safe for a disposable
project only after its exact project service/host units are reconciled, the
session is quiescent and its schema-3 state remains on a compatible release.
The v3 release must remain installed for any project bound to it. Do not point
a schema-3 project at a legacy release, restore an earlier SQLite snapshot
over accepted commands/decisions/receipts, or alter the existing blocked
legacy operator task. The conservative release fingerprint can refuse
rollback after even nonsemantic state writes; forward-fix on a compatible
schema-3 release in that case.

## One reusable hands-on project

The ignored project
`.local/supervisor-owned-execution/hands-on/project/` was initialized with
v3 and has only fake author/critic roles. It had no active service, Herdr
binding or task **when prepared**. Its key is
`<private-id>`. The operator has since used it; do
not assume it is idle or reset its state.
These Fish commands use a private runtime/registry and the final installed
candidate. They do not register a shared Herdr plugin:

```fish
set ww ~/projects/example-workspace/.local/supervisor-owned-execution/stage4-install-01/releases/rust-host-stage4-v3/bin/waywr
set project ~/projects/example-workspace/.local/supervisor-owned-execution/hands-on/project
set runtime /tmp/ww-s4-eval-runtime
set state_home ~/projects/example-workspace/.local/supervisor-owned-execution/hands-on/user-state
mkdir -p $runtime $state_home
chmod 700 $runtime $state_home
cd $project
env XDG_RUNTIME_DIR=$runtime XDG_STATE_HOME=$state_home $ww
```

Type a request such as “Draft a short local decision log proposal.” The
coordinator is deterministic and scripted: it proposes the task from the
request, while fake workers produce a retention question and revision.
Open the visible scope action to authorize the fake plan; opening the panel
does not authorize. Use `/tasks`, `/inspect`, `/output`, `/answer`, `/open` and
`/approve` as the gates appear. `Ctrl-Q` opens an explicit detach confirmation;
background work continues. Reconnect with:

```fish
env XDG_RUNTIME_DIR=$runtime XDG_STATE_HOME=$state_home $ww resume
```

Read status without launching the TUI:

```fish
env XDG_RUNTIME_DIR=$runtime XDG_STATE_HOME=$state_home $ww status
```

`/exit` requests a project-scoped dispatch pause after safe draining; it is
different from detach or task cancellation. For test cleanup after reviewing
records, stop only this project's service:

```fish
env XDG_RUNTIME_DIR=$runtime XDG_STATE_HOME=$state_home $ww session stop
```

Retain the project directory to keep its decisions, receipts and output.

## Operator evaluation in progress — action-strip finding (2026-10-13)

This is the preserved finding against the earlier v3 candidate. Its workaround
and "proposed UI correction" below are superseded by the isolated v2 retest
candidate above; they are not current operator instructions.

Screenshots show the operator submitted and authorized “Create a hello world
application in rust.” as task `<private-record>`. The fake
workflow asked its scripted transcript-retention question; the task later
reached an `awaiting-input` final review for `revision-2.md`. A subsequent
operator screenshot shows **Done · approved** for that task. This is an
operator-observed UI outcome, not an independent read of the durable approval
record or acceptance of the installed application. The answer text was not
inspected here.
The fixed retention question is unrelated to the submitted Rust request;
this demonstrates workflow plumbing, not task-specific coordinator reasoning.

The selected-task strip says “Enter opens this action,” but Enter with an
empty focused composer does nothing. Even after focus moves to the activity
strip, its live `awaiting-input` handler only requests `task.get`; it does not
open the pending question/review panel (`frontend/src/prototype/view.rs`,
`submit_composer` and `primary`). This is a real interaction defect and
operator acceptance of the interface remains pending. The `/inspect` view
exposes the pending record but is not itself the review action.

Current installed-build workaround: close Inspect with Esc, type `/open` and
Enter, read the candidate and press `n` until all pages are verified if
prompted. Esc returns to Final review. Tab to **Approve** and press Enter
only if approving this exact artifact; **Reject** is a separate action.
Opening `/approve` again after reading can refresh task detail and clear the
UI's local “opened” marker (`apply_detail`), so remain in the review panel.
The operator reports that this path was cumbersome and required guidance.
Treat this as a **failed next-action discoverability check**. The focused
correction is to show `Answer question` or `Review candidate` from the
authoritative pending-gate kind, display a short next step in the activity
strip and conversation, and make Enter on an empty composer or focused action
open that panel. A nonempty composer still submits its text. Opening the
panel must never answer or approve. Final review must lead through exact
candidate reading, then a distinct explicit approval/rejection action; the
reviewed marker must survive a same-candidate detail refresh. Add focused
keyboard, stale-candidate and no-duplicate-command checks before a new
isolated candidate is handed over. These are proposed UI corrections, not
implemented behavior or permission to change the running evaluation.
