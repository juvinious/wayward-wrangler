> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — Rust backend migration

## TUI-first optional-Herdr transition — all four simulated checkpoints passed (2026-10-13)

The implementation is on `refactor/supervisor-owned-execution`, based on
approved plan `dafb4f8451c725611091e0b3b264ffe73e02b107`. Passing local
checkpoints are `39406a6` (isolated host proof), `39d76d3` (D-Bus host and
fenced attempts), `be09765` (Herdr-free fake workflow/recovery), and `0dc2203`
(bounded TUI/CLI inspection and explicit read-only Herdr views). Checkpoint 4
adds compatible schema-3 release maintenance and final installed simulated
acceptance; use the latest branch commit for its source. See the
[execution log](tui-first-execution-log.md) and
[checkpoint-4 report](tui-first-checkpoint-4-report.md) before further work.

The final **fake-only** paired release is ignored
`.local/supervisor-owned-execution/stage4-install-01/releases/rust-host-stage4-v3/`
(manifest SHA-256
`5552fa1da5a756523974fa60543dad48ed18ec9203c70f067cffe404d5c95673`).
Installed two-project, release routing/rollback, exact approval,
restart/replay, private optional Herdr view and isolated PTY detach/resume
checks passed. A full duplicate fixture ran from external `/tmp` installation
and projects; its 375 regular evidence files were hash-verified into ignored
`stage4-external-proof-01/`. Earlier failed attempts remain under
`stage4-fixture-{01..05}/` and `stage4-pty-smoke-{01,02}/`; the final passing
fixtures are `stage4-fixture-06/`, `stage4-view-fixture-01/` and
`stage4-pty-smoke-03/`. Exact fixture project units and private Herdr server
were stopped. The external `/tmp/ww-stage4-paired-proof-01/` installation and
`/tmp/ww-stage4-external-fixture-01/` remain test-owned; stable evidence is
under ignored `.local/`. Existing operator/legacy services, approved records,
retained Herdr panes and the shared Herdr server were untouched.

The single reusable fake hands-on project is ignored
`.local/supervisor-owned-execution/hands-on/project/`, initialized but with no
active service or Herdr binding. Use the report's Fish launch/reconnect/status
commands with a private runtime and registry. The next action is operator
physical-terminal evaluation; no authenticated native admission, operator
cutover, push, merge or publication is authorized. The fake coordinator is
deterministic and only proposes a task from submitted text; this does not
implement open-ended model conversation. A PTY pass is distinct from Ghostty
or Herdr operator observation. A hard mount-denial test was unavailable
because `bwrap` uid mapping was denied; external-path and file-access evidence
is recorded instead. No process survival across logout/reboot or orphan stream
reattachment is claimed. Do not advance into native calls from these fixtures.

## Supervisor-owned execution checkpoint 3 passed (2026-10-13)

Checkpoint 3 is ready for a local commit on
`refactor/supervisor-owned-execution`. The accepted TUI and CLI now page
live/retained labelled output and exact-hash artifacts from project-scoped
records. `/herdr` and `observe ... --viewer herdr --socket PATH` explicitly
open read-only mirrors with attempt-bound IPC grants; normal schema-3 startup
and worker execution require no Herdr connection. The private installed fake
fixture passed under ignored `.local/supervisor-owned-execution/stage3-fixture-08/`
using `stage3-install-05/releases/rust-host-stage3-v5`. Its private Herdr
server and exact project service are stopped. Focused stdout/stderr overflow
evidence is retained in `stage3-output-flood-01/`. Earlier stage-3 failed
fixtures and distinct bundles remain retained for diagnosis. See the
[execution log](tui-first-execution-log.md) checkpoint 3 for causes, tests,
manifest digest and rollback boundary. No native calls, operator services or
shared Herdr resources were touched.

The next authorized work is checkpoint 4: a fresh paired installation away
from source/Python/Herdr for ordinary fake workflow, two independent projects,
release-specific routing, lifecycle/recovery, exact approval and rollback
checks. Use a separate private Herdr fixture only for explicit views. Verify
paused-service inspection and the remaining stated gaps; do not claim physical
terminal or native-agent acceptance from simulated tests. Commit the passing
checkpoint 3 first, then proceed automatically under the approved contract.

## Supervisor-owned execution checkpoint 2 passed (2026-10-13)

The approved TUI-first implementation remains on
`refactor/supervisor-owned-execution`. New schema-2 projects can now run the
installed fake coordinator/worker workflow under project-owned systemd user
services without Herdr. The accepted Rust TUI attaches directly, while
historical/legacy projects remain release-bound. An installed isolated run
reached both human gates, consumed a record-only answer across supervisor
restart, used exactly five fake worker invocations, and retained an exact-hash
simulated approval after another restart. This is **simulated implementation
evidence**, not native-agent or physical-terminal acceptance. See
[execution log](tui-first-execution-log.md) checkpoint 2 and ignored
`.local/supervisor-owned-execution/stage2-*` evidence. The v1 `/plan` failure,
v2 correction and v3 final checkpoint-2 build remain inspectable. All
stage-2 project services were stopped after testing; operator services and
Herdr resources were untouched.

The next approved checkpoint is bounded stdout/stderr and artifact inspection
in the accepted TUI and CLI, followed by optional explicit Herdr read-only
views. Continue to checkpoint 4 after checkpoint 3 passes; do not treat
retained transcripts as already satisfying live in-TUI inspection. Native
calls stay blocked, and no operator service or project is cut over.

## Supervisor-owned execution implementation in progress (2026-10-13)

Checkpoint 1b now adds the Rust D-Bus host adapter, versioned attempt records,
generation/retirement dispatch gate, host-runner entry point and saved-receipt
validation. Six offline fault tests and an isolated fake systemd runner test
passed. The test-owned units were stopped by exact name. See the
[execution log](tui-first-execution-log.md) and ignored
`.local/supervisor-owned-execution/dbus-runner-{01,02,03}/` evidence; 03 is
the final passing retained attempt. This is **not yet** a Herdr-free project
workflow or an installed acceptance build. Existing projects still route to
their bound releases. The next checkpoint is project-service/schema routing,
bounded infrastructure recovery and fake workflow integration; do not use
the host runner for native calls during this transition.

The TUI-first, optional-Herdr proposal at `dafb4f8451c725611091e0b3b264ffe73e02b107`
was approved for all four checkpoints. Work is on the new branch
`refactor/supervisor-owned-execution` from that exact commit. The older
`fix/reinit-stale-project-registry` branch and all migration/corrective commits
remain in ancestry. No operator release, service, blocked task or historical
record was changed. Native calls remain blocked.

Checkpoint 1a, the **isolated fake systemd host proof**, passed. See
[tui-first-execution-log.md](tui-first-execution-log.md) and ignored
`.local/supervisor-owned-execution/host-proof-{01,02,03}/`. The expanded 02
fixture caught and preserved a fast-exit identity assertion failure; 03 passed
after checking the retained `ExecStart` PID and invocation ID. All test-owned
units were stopped by exact name. This verifies host behavior, not yet the
application host backend or full workflow. The immediate next task is to add
versioned Rust host attempts and real project/operation dispatch fencing, then
route isolated fake operations only after their fault tests pass.

The recommended proposal defaults and rollback/legacy boundaries remain as
documented below. Continue automatically through the approved checkpoints,
committing each passing boundary. No authenticated calls, shared Herdr changes,
operator-service cutover, push or merge.

## Finalized TUI-first transition plan — awaiting review (2026-10-13)

**Implementation is paused.** The current documentation branch is
`fix/reinit-stale-project-registry`, inspected at
`ea9126f3963390193b59c740f228c0c7d41adb63`. It contains the migration branch
`refactor/rust-backend` at `7b5d69e79f6a67bc297d0133a55eee31df1f8936` plus
five commits: `bd69906`, `cecdada`, `672ac9a`, `835f733`, `ea9126f`.
The working tree was clean before this documentation pass. Preserve that ancestry;
do not restart from the older migration tip and omit the fixes.

The finalized [transition proposal](tui-first-optional-herdr-proposal.md) specifies:

- Project supervisor ownership independent of TUI/Herdr, with unique transient
  systemd user services for runner attempts; no automatic runner restart.
- Durable claim/permit/dispatch fencing, receipt reconciliation and separate
  infrastructure-attempt/model-call budgets. Only positively proven pre-launch
  failures can create linked replacement attempts; ambiguous execution cannot.
- Complete ordinary workflow and bounded live/retained output in the accepted TUI,
  including with no Herdr executable/socket or with unusable inherited variables.
- Optional explicit Herdr read-only output mirrors. Closing a view cannot cancel
  work. No interactive attachment, new coordinator capability or native acceptance.
- Versioned configuration/state/records and project-specific rollback that cannot
  restore a snapshot over newly accepted work. No operator cutover or implicit
  conversion of existing Herdr/LangGraph operations.

This supersedes older mandatory worker-pane/Herdr startup requirements for the
**proposed new execution contract**, including stale historical prose in earlier
plans and AGENTS.md. It does not describe the current installed implementation:
that still requires Herdr. The Rust TUI, Tokio runtime, bounded SQLite writer,
authorization/approval semantics and project isolation are retained.

**Proposed implementation branch:** `refactor/supervisor-owned-execution`, based
on the final planning commit containing this section on the current fix branch.
Do not create it until implementation is separately approved. At that point
resolve/record the full base SHA with Git; inspect any existing branch of that
name rather than resetting it. No merge or push is authorized here.

**Exact first task after approval:** build only the proposal section-3 isolated
fake runner/child host harness. Verify `Type=exec`, `ExitType=cgroup`, no restart,
retention, lost-start reply, supervisor replacement, runner death with surviving
child, stale permit/retirement and targeted cleanup. Use unique test-owned units
and durable counters; do not restart the shared manager. Record evidence under
ignored `.local/` and checkpoint a passing proof before wiring production launch.
If host semantics fail the declared gates, preserve evidence and report the
specific blocker. Then follow the four implementation checkpoints in the proposal.

Review defaults: Linux/systemd 255+/cgroup v2 for execution; no automatic lingering
or reboot replay; no orphan stream reattachment; an explicitly requested Herdr
inspection workspace may retain its bootstrap shell to avoid unsafe replacement.
These are recommendations for approval, not new completed capabilities.

**Verification in this pass:** source/ancestry and primary systemd v255/API review
only; no build, test, worker, process-host experiment, service or installation
changes. The package version came from package metadata, not a host experiment.
No new runtime acceptance is claimed. Existing evidence remains in
`.local/rust-migration/bootstrap-refusal-fix/`,
`.local/rust-migration/stale-registry-fix/`, and
`.local/rust-migration/startup-direct/`; r9 and other comparison candidates remain.

The operator's `waywr-rust-sim-eval` project, blocked task
`<private-record>`, `rust-local-init-v3` binding and evidence are untouched.
Its separate supported recovery still requires a legacy-compatible correction
and explicit safe release/service change; the transition is not that migration.
See [bootstrap-refusal-correction.md](bootstrap-refusal-correction.md). The service
status below is **last-known from 2026-10-12**, not freshly verified this turn:
`waywr-<private-id>.service`, PID 3808905, active then.
No operator/shared resources were modified. Stop for proposal review.

## TUI-first direction and bootstrap refusal checkpoint (2026-10-12)

The operator's current product direction is **Waywr as the primary TUI;
Herdr as optional, explicitly requested inspection**. `init` remains local
initialization only; `waywr` launches the TUI. The supervisor must eventually
run coordinator and worker operations without Herdr, while the TUI offers
live/retained output and all human gates. Do not infer pane-creation consent
from `HERDR_ENV`. The bounded transition and its separate acceptance gates
are in [tui-first-optional-herdr-proposal.md](tui-first-optional-herdr-proposal.md).
This is planning only; current Rust execution still depends on Herdr.

The operator's simulated task in
`~/projects/waywr-rust-sim-eval` stopped before a proposal. It
is bound to `rust-local-init-v3`; its task and frozen operation evidence remain
unchanged. The source path proves Herdr's bootstrap-tab refusal occurred before
this call sent `layout.apply`, but the old release classified it as possibly
sent. The operator later showed an idle Fish prompt; that does not establish
the earlier `pane.process_info` response. See
[bootstrap-refusal-correction.md](bootstrap-refusal-correction.md) for the
exact evidence, source cause, narrow correction and unresolved legacy task.
Do not delete evidence or blindly retry that task. No operator release/service
was changed.

Read-only `systemctl --user show` at this checkpoint reports the affected
project unit `waywr-<private-id>.service` active and
running with PID 3808905, using the `rust-local-init-v3` service binary. Its
state and UI attachment were not modified here.

The new source distinguishes pre-send refusal from ambiguous send, waits a
bounded two seconds for a new Fish tab, preserves a refusal record, and shows
the saved coordinator call and reason in `/inspect`. Focused Rust tests and an
installed private Fish simulated path passed. The ignored corrected candidate
is `.local/rust-migration/bootstrap-refusal-fix/install-v1/releases/rust-bootstrap-fix-v1/bin/waywr`;
its bundle and private fixture evidence are under
`.local/rust-migration/bootstrap-refusal-fix/`. The fixture's own service and
Herdr server were stopped. The operator's older bound service and panes were
not touched. Native model calls, Herdr-free execution and physical-terminal
acceptance remain unverified. The existing saved task has no supported
append-only legacy pre-send recovery transition yet; that is a concrete
remaining blocker before continuing it. Next, review the transition proposal
and decide whether to authorize its first isolated execution checkpoint.

## Stale project reinitialization correction (2026-10-12)

The operator identified that deleting local project files removed default
workflow state but left a user-local location reservation, so a later `init`
in the same directory failed. Their storage requirement is explicit: by
default, `init` writes only under the selected project; authoritative project
memory stays in `<project>/.wayward-wrangler/state/`. It may not create a user
registry, service, runtime socket or Herdr tab. An explicit `--state-dir` is an
operator-selected exception. The scoped fix is on
`fix/reinit-stale-project-registry`, branched from `refactor/rust-backend`.
See [stale-registration-reinit.md](stale-registration-reinit.md) for the exact
path inventory, same-path/state/service safeguards and isolated old-versus-new
CLI evidence. A user-local location index remains a nonauthoritative locator
for review links after first service attachment; it must not block local init.
This is still an external runtime metadata write, so do not claim active use is
entirely free of external writes.
An explicitly older-release-bound project still routes to its historical
release; do not describe that legacy `init` as changed by this fix.
The affected operator project and global registry were inspected but not
modified. Its state directory is absent and its new configuration key differs
from the older registry key. Existing candidate releases and their evidence
remain retained; no shared Herdr resource or service was changed.
The earlier v2 candidate wrote the location index during init and is superseded;
its evidence is retained. The final installed candidate is
`.local/rust-migration/stale-registry-fix/install-v3/releases/rust-local-init-v3/bin/waywr`
(manifest SHA-256 `4b4bfefe1e0ee3f2a384ff44558e660e44508e43b3e0521fa3ffc3168bfbee60`).
Six focused Rust tests passed. `installed-check-v3/summary.json` demonstrates
fresh and repeated local-only init and the old failure. The private simulated
first-attach fixture `private-attach-v3/{summary,cleanup}.json` passed and its
test-owned service and Herdr server were stopped. This does not establish
physical terminal or native-agent acceptance.
The operator subsequently reinitialized the directory with that candidate
and opened the TUI; the bootstrap refusal above is the resulting new finding.
Strict Clippy is not clean due to previously existing warnings
outside this fix; see the correction note. No native worker call occurred.

## Direct-terminal startup correction — hands-on finding addressed (2026-10-12)

The operator found that the Rust candidate opened an extra coordinator tab
when launched from Herdr. The trace, correction, private installed evidence,
and Fish retest commands are in
[rust-migration-startup-correction.md](rust-migration-startup-correction.md).
`init .` / `init SOME_DIR` now return without launching a service, workspace or
TUI. `waywr` and `waywr resume` use the invoking terminal, with Herdr reserved
for separately visible worker execution. The old service-created UI-tab IPC
path refuses in the new candidate. Project worker workspace identity and UI
attachment are distinct; a live UI cannot be silently duplicated.

The corrected immutable candidate is
`.local/rust-migration/startup-direct/install/releases/rust-startup-direct-v5/bin/waywr`
(manifest SHA-256 `54444db203932edb4a1f481d5e91abe39c8d255209c4a56e8f9f0a1c076b29ba`).
The v7 candidate and its operator evaluation state remain retained. Private
simulated evidence is under ignored `.local/rust-migration/startup-direct/`;
`installed-check-v5-final/{summary,cleanup}.json` passed and all of that
fixture's owned service/Herdr processes stopped. The previously observed
`stale_state` exit/resume race and its failed-attempt evidence remain retained.
No operator project was rebound, and shared Herdr/services were untouched.
Physical Ghostty/Herdr observation for v5 is still pending; native agent calls
remain blocked. The new direction is direct UI in the invoking terminal, with
the project supervisor retaining durable workflow authority.

## Corrective Rust migration checkpoint — installed simulated gate passed (2026-10-13)

Branch `refactor/rust-backend`; verified corrective code commit `aa426d332c8bf8bd250dfd7ff93bc052e6a9f7a4`. The original Stage-5/6 passes remain **partial simulated coverage**, as explained in the [correction report](rust-migration-correction-report.md) and [migration log](rust-migration-log.md). The reviewed runtime/CLI gaps were corrected without changing the accepted production TUI or operator releases. This is not native-agent or physical-terminal acceptance.

The final paired candidate is `.local/rust-migration/corrections/final-v7-install/releases/rust-corrective-final-v7/bin/waywr`, manifest SHA-256 `fdb638ac3d922a7cbbb38587deed883c42ad406713b18848e30c7cb4d006d392`. Its default link was not selected. The earlier Stage-6 installation remains at `.local/rust-migration/stage6/verified-install/`, and the preserved r9 comparator remains at `.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/`. All are ignored local resources; do not add their binaries or runtime state to Git. `Cargo.lock` now directly lists Tokio for the frontend status-only CLI checks; no package version was upgraded by that edit. The pinned permanent Rust toolchain remains 1.99.0.

Installed private-Herdr simulated evidence is under ignored `.local/rust-migration/corrections/final-v7-{service,cli}/`, with summaries and `cleanup.json` confirming all test-owned servers/services stopped. The bound tests covered active-work exit/restart, missing workspace, two projects, exact approval, ordinary CLI artifacts/logs, record-only decisions, cancellation, and saved command status/replay with no duplicate invocation. Private runner/flood evidence is under `runner-regression-01/` and `runner-flood-06/`. Failed diagnostic attempts remain preserved, including `diagnostic-service-01/` (`layout_not_found`) and `final-v4-cli/` (stale stop refusal). Locked/offline test and Clippy logs are `final-v7-cargo-test.log` and `final-v7-clippy.log`. The [report](rust-migration-correction-report.md) has the r9 command/flag checklist, measured fault timings, limitations and Fish hands-on walkthrough.

The read-only final process inventory still found the Python-bound `m6-lifecycle-r10` and `m6-unified-ui-r9` operator project services (PIDs 3318698 and 3378246) and the shared Herdr server (PID 1356852). Their running state was not changed or used for these fixtures; no corrective Rust service or runner remains. Native context/tool isolation and authenticated CLI execution remain blocked; only mock status scripts were used to check native preflight logic. Physical Ghostty/Herdr input and visuals for this candidate still require operator observation. No operator project cutover, merge, push or release publication occurred. Next action is review of the corrective report and simulated candidate; do not infer native acceptance from it.

## Migration closeout review — documentation-only findings (2026-10-12)

On `refactor/rust-backend`, [review addendum](rust-migration-review-addendum.md)
audits the approved runtime design, the frozen r9 versus Rust CLI, and the
lockfile history. It identifies two acceptance concerns: the std-thread/SQLite
deviation lacks slow-client/contention/output-pressure evidence, and ordinary
r9 CLI operations/flags are missing or changed beyond the documented legacy
checkpoint boundary. No runtime repair or new test has been performed for this
review. Do not claim full asynchronous or CLI parity from the passing fake
workflow. Existing operator projects, services, r9 comparator and ignored
Stage-5/6 evidence remain unchanged. Next action is review of the addendum and
focused acceptance disposition before runtime changes.

## Approved migration — six simulated stages passed; Rust release ready for review (2026-10-12)

Branch `refactor/rust-backend` now has the application-owned Rust workflow,
Herdr runner, project service, accepted TUI, supporting CLI, review integration
and paired Rust bundle. The Stage-6 passing commit is `b881cde64e22bcc235e3173518df471cf100a087`.
The final installed test entry point is
`.local/rust-migration/stage6/verified-install/bin/waywr`, bound to release
`rust-migration-s6-verified-20261012` (manifest SHA-256
`1ec1414e880051d7fa938e94a3dcc65cfbe5a4d0c4f944237d339cd4e2de4aec`).
It does not require an application Python interpreter, wheel, venv or Python
service. The root `Cargo.lock` changed during migration Stages 1, 2, 3 and 5;
it was unchanged during the permanent-toolchain switch and Stage 6. Permanent
rustup selects pinned Rust 1.99.0 on `x86_64-unknown-linux-gnu`; the temporary
Rust roots are still retained. The preserved r9 Python comparison release remains at
`.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/` with
manifest SHA-256 `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.

Stage-6 results, failed attempts, differences, state-aware rollback and limits
are recorded at the top of [rust-migration-log.md](rust-migration-log.md).
Primary ignored evidence is `.local/rust-migration/stage6/verified-{live,routing,plugin,external-live}/`,
plus `verified-legacy-inspection.json`, `verified-source-access.strace` and
`verified-external-copy-verification.json`. Earlier `final-*` roots are retained
exploratory evidence. Final isolated units and private
Herdr servers were stopped; operator projects, services, shared Herdr and
historical approvals were not touched. The `/tmp/waywr-rust-s6-verified-install`
and `/tmp/waywr-rust-s6-verified-live` external test copies also remain; the
known-good migration comparison installation is the `.local/` one above.

**Acceptance boundary:** all new Rust execution evidence is simulated. The
fixture answered and approved only its isolated test run; no authenticated
Claude/Codex call or operator-service cutover occurred. Native CLI isolation
and physical terminal behavior still need separately authorized observation.
Legacy LangGraph runs remain on their explicitly bound releases. Do not
interpret Stage-6 historical read-only inspection as transparent resumption.
The Rust TUI supports task decisions and approval; several ordinary r9 CLI
controls are also refused or changed on Rust projects, as inventoried in the
review addendum. Do not merge, push, publish or remove the r9 pair under this
migration authorization.

## Approved migration — Stage 5 simulated gate passed; continue Stage 6 (2026-10-12)

The Rust project service, protocol-major-1 IPC, systemd bootstrap, Herdr UI
attachment, project-scoped queue/lifecycle and accepted Rust TUI now operate
on **new isolated fake-only schema-2 projects**. The r9 installed Python pair,
historical approvals and operator services remain unchanged. Stage-5 evidence
and variance/rollback details are at the top of
[rust-migration-log.md](rust-migration-log.md), under ignored
`.local/rust-migration/stage5/`. The full fake flow, active-work exit/restart,
missing-workspace routing, two-project isolation, systemd restart, cold PTY
startup and installed-r9 command differential passed in isolated fixtures.
Fixture answers and approvals are not operator authorization. Test-owned
services and private Herdr servers were stopped; their retained output is
inspectable under the cited evidence roots.

**Next:** Stage 6 must remove remaining Python CLI/plugin/build/install paths
from the *new* Rust release, build/install it in a fresh prefix without a
wheel/venv or source checkout, then prove the final integrated fake workflow,
release routing and historical read-only boundary. Native calls and operator
cutover are still unauthorized. Use the pinned permanent Rust 1.99.0 toolchain;
the temporary toolchain and r9 baseline remain preserved.

## Approved migration — Stage 4 simulated gate passed; start Stage 5 (2026-10-12)

Stage 4 adds an **unrouted** schema-2 Rust workflow store and existing
draft/review policy. The final isolated simulated fixture and installed-r9
comparison passed; see the top of [rust-migration-log.md](rust-migration-log.md)
for verification, differences and state-aware rollback. Evidence is under
ignored `.local/rust-migration/stage4/live-final/`; its cleanup audit records
zero live test-owned processes. The v1 final artifact bytes/hash match r9;
the v2 fixture demonstrates two durable questions and provisional finding
assessments. Automated fixture responses are not operator authorization.

**Next:** establish Stage 5's project/release and schema rollback boundary,
then replace the Python project service and connect the accepted Rust TUI in
isolated Rust-owned projects. Compare commands, lost replies, lifecycle,
recovery, responsive control and two-project isolation. Do not route an
operator project, change a shared Herdr service or launch native providers.
Use the pinned permanent Rust 1.99.0 toolchain; retain the temporary one.

## Approved migration — Stage 3 simulated gate passed; start Stage 4 (2026-10-12)

Stage 3 adds an **unrouted** Rust Herdr runner and immutable execution
reconciliation. Seven isolated live Herdr cases, nine Rust tests, and Clippy
passed; details and all variances are in the top of
[rust-migration-log.md](rust-migration-log.md). Final evidence is ignored under
`.local/rust-migration/stage3/live-final-3/`, including retained pane output,
server/controller logs, receipts, diagnostics and `cleanup.json` confirming no
test-owned server/runner/child remains. The r9 installed baseline is unchanged.
No native provider was called. The Python service remains the installed route;
the Rust runner is not a production cutover.

**Next:** establish Stage 4's code and state rollback boundary, then add
application-owned Rust workflow/effect persistence, decisions and exact
artifact approval on isolated simulated runs. Compare against installed r9
behavior, preserving historical records and approved projects. Continue through
passing stages without routine approval requests. Keep the permanent pinned
Rust 1.99.0 toolchain; retain the temporary one until no build needs it.

## Approved migration — Stage 2 passed; start Stage 3 (2026-10-12)

Branch `refactor/rust-backend` has passing Stage 1 and Stage 2 boundaries.
Stage 1 commit: `12cc4f5`; pinned permanent Rust 1.99.0 toolchain commit:
`5da61b7`. Stage 2 adds **inert** Rust policy/context/packet/adapter code and
passes installed-r9 comparisons on copied state. See the top of
[rust-migration-log.md](rust-migration-log.md) for exact cases, intentional
fake-v2 correction, attempt evidence and state-aware rollback. Final comparison
summaries are under ignored `.local/rust-migration/stage2/final-*/`; no runtime
project or service routes to these Rust modules yet.

**Next:** establish the Stage 3 code and disposable-state checkpoint, then port
the Herdr-visible runner with launch/claim/permit/receipt/cancellation and
uncertain recovery. Use only an isolated, test-owned Herdr server and fake
workers; do not disturb operator services, the retained r9 Python baseline or
shared Herdr. Continue through passing stages automatically under the approved
execution contract. The installed Python backend and native acceptance remain
unchanged. The temporary Rust toolchain is still retained; development now
uses `~/.cargo/bin/cargo` with root `rust-toolchain.toml`.

## Pinned user-local Rust toolchain (2026-10-12)

The temporary stage-1 toolchain and the operator's permanent rustup both
reported `rustc 1.99.0 (b940084d7)` and `cargo 1.99.0 (5f94df4789)` on
`x86_64-unknown-linux-gnu`. No `rust-toolchain.toml` or Cargo `rust-version`
previously existed. Installed the exact `1.99.0` channel into the permanent
user-local rustup and pinned it in root `rust-toolchain.toml`; `Cargo.lock` was
not upgraded. Use `~/.cargo/bin/cargo` from the repository root (or `cargo` if
Fish already includes `~/.cargo/bin` in PATH), with no temporary `CARGO_HOME`
or `RUSTUP_HOME` overrides. The temporary `/tmp/waywr-{cargo,rustup}` roots
remain intact until the pinned build is verified and no build uses them.

`~/.cargo/bin/rustup show active-toolchain` selected pinned 1.99.0, and
`~/.cargo/bin/cargo test -p waywr-backend --locked` passed. That command
currently verifies compilation (zero backend unit tests), not the stage-2
behavioral gate. The permanent toolchain now replaces the temporary one for
development; the temporary roots remain preserved, with no active build using
them at the time of the check.

This toolchain maintenance is within stage 2's inert-development checkpoint.
It does not change any running service, release binding, r9 comparison baseline,
or operator project. The focused check result is recorded in the migration log.

## Approved migration — stage 1 passing; continue stage 2 (2026-10-12)

The operator approved the revised six-stage plan and historical compatibility
boundary at `6067fd7f59887e0bb197d786609e43744865404e`. Remain on
`refactor/rust-backend`; continue through all passing stages automatically under
the [execution contract](rust-backend-migration-proposal.md#execution-contract-after-authorization).
The previous “awaiting review” entries below are historical planning records.

Stage 1 adds a root Cargo workspace, a read-only `backend/` legacy inspector and
an installed-r9/Python-versus-Rust comparison harness. The accepted TUI library
still passes 37 tests (one existing ignored timing probe). Stage 1's seven
isolated cases compare one approved simulated run and six integrity/schema
faults. Rust intentionally refuses stale approval and unknown versions that
pass the old passive read. See [migration log](rust-migration-log.md) for evidence,
differences and rollback. The stage commit becomes stage 2's start checkpoint.

Preserved r9 manifest SHA-256 remains
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
Ignored test copies and manifests are in `.local/rust-migration/stage1/`. No
operator project, service, historical artifact, release binding or shared Herdr
resource changed. No authenticated call was made. All Python runtime
dependencies remain; Rust execution/service cutover has not occurred.

**Next:** establish the stage-2 checkpoint and isolate policy/packet/adapter
fixtures. Do not route an operator project to Rust. Maintain the accepted TUI,
project boundaries and no-retry execution rules.

## Orchestration framework assessment awaiting review (2026-10-12)

Current branch: `refactor/rust-backend`. Implementation remains **paused**.
Read [Rust orchestration assessment](rust-orchestration-assessment.md), the
revised [migration proposal](rust-backend-migration-proposal.md) and
[migration log](rust-migration-log.md). Source checkpoint at entry:
`f8b342a6b14a9ecd6973c0b1a18de9e11a6da38a`; the tree was clean.

Recommendation remains an application-owned durable Rust transition engine,
now grounded in the requested candidate assessment rather than the size of
today's workflow. Juncture and Smooth received the deep workflow review;
their inspected continuation/persistence boundaries leave significant app work.
Rig's serializable agent stepper may help a future coordinator, but its in-flight
reissue semantics must never authorize duplicate external execution. AutoAgents,
Polaris (`RadiantAILabs/polaris`, confirmed from `polaris-ai` metadata) and
OpenMirai were screened. Exact commits, licenses, tests read and uncertainties
are in the assessment. No third-party code was executed.

Keep current drafting policy separate from versioned workflow/effect persistence.
The application remains authoritative for approvals, answers, receipts, budgets,
project isolation and execution reconciliation. No framework or native provider
session becomes a second owner. No transparent LangGraph checkpoint conversion:
unfinished projects retain their legacy binding until safely resolved/explicitly
archived and upgraded. Rust-backed projects must operate without Python fallback.

**Exact next action:** operator reviews the revised migration plan. After separate
implementation authorization, start stage 1's read-only record reader/comparison
contract, then proceed through all six passing stages without routine approval
requests. Stage gates require intended behavior, discrepancy evidence and
regression coverage, plus state-aware rollback; never restore an old snapshot
over newly completed work. The optional Juncture probe is documented for a future
reconsideration, not required before the reader or authorized in this turn.

This commit changes documentation only. No application/dependency/configuration
changes, tests, workers, builds, installations, services or shared Herdr resources
were touched. r9, operator projects and historical evidence below remain intact.
Service status was not rechecked. No push, merge or publication. Installation,
TUI and coordinator feature work remain paused.

## Migration proposal awaiting review (2026-10-12)

Branch: `refactor/rust-backend`. Read the
[Rust backend migration proposal](rust-backend-migration-proposal.md) and
[migration log](rust-migration-log.md) next. Planning inspected source at
`d14765ba03e739f35f9450258e03bbd4f2dbe923`, the retained r9 manifest,
installed LangGraph serializer source, and archived SQLite metadata. The tree
was clean at entry. This checkpoint adds documentation only; implementation
has not started. No tests, workers, installations, service changes or shared
configuration changes were made; nothing is being pushed or merged.

Recommended architecture: preserve the accepted Rust TUI/IPC client; replace
Python with an internal Rust backend library, explicit durable workflow
transitions, project service and Herdr runner. One authority per project;
no Python/Rust dual writers. The final installed release must have no Python,
venv, wheel, Python plugin handler or Python service dependency, including
compatibility CLI and installation paths. Existing native restrictions remain;
simulated/offline adapter parity is not native-isolation acceptance.

The substantive review choice is historical compatibility: recommend Rust
read-only inspection and explicit reuse of approved legacy context, with
quiescent versioned project upgrades. Do not transparently convert suspended
LangGraph runs or discard unresolved tasks. Those remain on their explicitly
bound legacy release until resolved or separately archived. Rust must not
decode checkpoint blobs as if they were portable application state. Existing
operator services/bindings and approved records remain untouched.

After plan approval and implementation authorization, proceed through its six
stages automatically once each parity gate passes. Each stage needs a Git
checkpoint **and state/schema rollback**, isolated behavioral comparison with
the retained Python baseline, discrepancy resolution, a commit, log and
handoff update. Pause for concrete substantive/access/integrity blockers, not
routine successful stage transitions. First bounded task: a read-only Rust
record reader and comparison fixtures; no launcher/service cutover yet.

The baseline executable, preserved evidence and last-known active services are
listed below. Their running status was **not rechecked** during planning.
Installation improvements, TUI modifications and coordinator feature work
remain paused. Await operator review before implementing the migration.

## Merged checkpoint and local cleanup (2026-10-12)

PR #4 merged the simulated milestone-6 checkpoint into `main` at
`ae367f2698f643c7b2d9c6ed91c8ad4d65d9b99b`. The original branch
checkpoint was `79615442f94326787e81a137ffeeeaff8e476d1d`; annotated tag
`checkpoint/milestone-6-simulated-rust-migration-20261012` points to the merged
commit and records that original SHA. Current development branch:
`refactor/rust-backend`. At that cleanup checkpoint migration planning had not
started; the proposal above now awaits review. Implementation has not started;
installation, TUI, and coordinator feature work remains paused.
The fully merged local `milestone-6-interactive-shell` branch was deleted with
`git branch -d` after PR #4; its remote branch remains because that PR was
merged before the `gh pr merge -sd` cleanup convention was recorded in
`AGENTS.md`.

**Migration comparison baseline:** the known-good paired Rust frontend/Python
backend executable is
`.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr`
(manifest SHA-256 `ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`).
The local `install-r5/bin/waywr` alias still selects r5, so use the explicit
r9 path for comparison. The operator project `/tmp/waywr-unified.VTkzZC` is
bound to r9. The retained `operator-02` project is bound to
`.local/milestone6/lifecycle/install-r10/releases/m6-lifecycle-r10/`; both
installations and their active services were left untouched. The old r5
release remains because the local default launcher points to it. The accepted
scripted evaluator and both operator evaluation projects also remain.

Exact cleanup provenance is under ignored `.local/milestone6/cleanup-20261012/`:
`inventory.json` and `tmp-build-cleanup.json` list every removed path, original
manifest hash, and verified archived manifest. Removed: 14 superseded local
installation prefixes plus the co-located unified r8 release, 18 reproducible
bundles, 12 wheel staging directories, `frontend/target`, four inactive
isolated color-test fixtures (`/tmp/wc-r5`, `wc-r6`, `wc-r8`, `wc-r9`), and seven
unreferenced `/tmp` build/install roots. The unique r8 fixture files were
copied and hash-verified under
`.local/milestone6/unified-ui/evidence/color-pass-r8/` before removal. Existing
r5/r6/r9 fixture evidence was reverified against its originals before removal.
All 57 selected paths are absent and archived manifest hashes verified. This
does not rewrite historical source paths in receipts or reports.

Retained dependencies include `/tmp/waywr-m6-accept-install`, referenced by
registered routing test projects, and `/tmp/waywr-m6-reinstall`, whose r3 release
is still bound by preserved test fixtures, plus `/tmp/waywr-rustup` and
`/tmp/waywr-cargo` for upcoming Rust work and other fixtures whose unique
evidence was not audited for deletion. Do not
remove these based only on their names. At the final read-only check,
`waywr-<private-id>.service` and
`waywr-<private-id>.service` were active. No operator
project, decision, approval, transcript, receipt, running service, Herdr pane,
or shared setting was removed or changed. No tests or model calls were
launched for this checkpoint/cleanup.

## Simulated implementation checkpoint for Rust backend migration (2026-10-12)

The current installed pair is `m6-unified-ui-r9`: the accepted Rust/Ratatui
conversation frontend runs against the existing Python project service,
workflow engine, adapters, and records. Its executable is
`.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr`;
the local `install-r5/bin/waywr` default still selects r5. The corrected r9
release is bound to the operator's disposable project
`/tmp/waywr-unified.VTkzZC` (binding generation 2). Release files and
runtime state are local evidence, not Git content.

The operator completed the installed **simulated** flow after the color-enabled
coordinator failure and explicit pre-permit recovery. The saved `7 days`
decision and exact revision-2 approval are documented below and in the
[integration report](unified-ui-integration-report.md). This checkpoint is
neither native-agent acceptance nor complete-product acceptance. Known limits:
new composer text becomes a task proposal, open-ended coordinator conversation
and steering are unavailable, only listed choice answers are supported, and
the approved fake brief did not establish a useful application specification.
No authenticated coordinator or worker call was made for this check.

Retained ignored evidence includes `.local/milestone6/unified-ui/evidence/`
(r5 color failure, r9 simulated pass, pre-permit recovery), the local r9
installation above, and the project's records under
`/tmp/waywr-unified.VTkzZC/.wayward-wrangler/state/`. Preserve original
failure, recovery, decisions, approval and worker receipts. At this checkpoint,
read-only `systemctl --user` inspection showed both
`waywr-<private-id>.service` (operator disposable
project) and `waywr-<private-id>.service`
(`operator-02`) **active**. Their Herdr/server resources were not changed;
this is a last-known status, not a lifetime guarantee.

The migration proposal now awaits review on `refactor/rust-backend`; see the
current entry above. Installation improvements, TUI changes and new coordinator
features are paused. Preserve the existing protocol, state semantics, recovery
evidence and project isolation; this handoff is not implementation approval.

## Latest operator check (2026-10-12)

The original recovered task in `/tmp/waywr-unified.VTkzZC` progressed through
all five simulated worker stages and is now `approved`. Its saved retention
answer is `7 days`; the `A1` approval binds the exact `revision-2.md` artifact
at SHA-256 `64a524c9d440c23168fdbaaf4e768ae0be88ba0ccec0d7af0451c8bc3fe3ee40`
(verified against the file). This is an operator-completed **simulated** run,
not a native-model trial or a verified browser application. Original failure,
recovery and worker evidence remain in the project's state. See
[the integration report](unified-ui-integration-report.md).

The operator had difficulty locating proposals and understanding task state.
In this build, `/plan` applies only to proposed tasks, `/answer` to pending
choices, and `/approve` to final review. Default composer text creates a new
task: later `ok` and `go` entries are separate unapproved proposals. The two
earlier `ok` tasks remain blocked. `/steer` and open-ended coordinator
conversation are unavailable. Do not describe this simulated task as a useful
specification or these UI gaps as resolved.

## Installed unified UI failure correction (2026-10-11)

The operator's first color-enabled r5 session at `/tmp/waywr-unified.VTkzZC`
exposed a fake coordinator runner crash before dispatch permit. The operator
detached that UI on 2026-10-12; the affected project was then explicitly bound
to `m6-unified-ui-r9` (generation 2). Its corrected UI is in the project-owned
Herdr workspace `wD`, tab `wD:t2`, pane `wD:p2`, and its project service is
active. First task: `<private-record>`. Its original coordinator
launched and claimed `wC:p3`, then exited before permit/child/receipt. No
proposal or author/critic worker ran. The next two `ok` tasks failed at the
same boundary. “Not dispatched” referred only to the later workflow run and
was misleading about the coordinator launch.

The root cause was `terminal.paint` indexing a missing `coordinator` color
after the claim in a color-capable pane. The old private acceptance had
inherited `NO_COLOR=1`. The corrected formatter, concise **Stopped** status,
workflow-stage labeling, `/inspect` coordinator evidence, and explicit
one-attempt pre-permit recovery are in the new paired `m6-unified-ui-r9`
release. Its manifest SHA-256 is
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`.
It is installed side by side under
`.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/`;
the `install-r5/bin/waywr` default still points at r5.

Read [the corrected integration report](unified-ui-integration-report.md) for
the exact affected project/service, evidence, Fish reopen command and limits.
Old r5 color-enabled failure and final r9 simulated pass were reproduced via
the installed UI on isolated Herdr servers. A separate
isolated copy of the failed project was upgraded and explicitly recovered:
the original claim/failure remained, and the replacement yielded one
completed coordinator receipt and proposal. Evidence is hash-verified under
`.local/milestone6/unified-ui/evidence/color-repro-r5-short/`,
`color-pass-r9/`, and `prepermit-recovery-r9/`. Focused checks: 24 Python
tests and targeted Rust stopped-state test passed. No authenticated calls.
The original operator project's **first task** was recovered through the
supported one-time pre-permit path. Original `call-v1` records and failure were
preserved; appended `call-v1/recovery.json` records the original evidence
hashes, and `call-v1-r1/receipt.json` records a completed fake coordinator
call. At that recovery point the first task was `proposed`, with no workflow
run/worker dispatched; its later simulated completion is recorded above. Both
original `ok` tasks remain `blocked`. The r9 service and UI remain active.
To reopen the affected project in Fish:

```fish
~/projects/example-workspace/.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr --project /tmp/waywr-unified.VTkzZC
```

Shared Herdr work and the older `operator-02` unit remain untouched.

## Unified installed frontend — current implementation (2026-10-11)

**Branch:** `milestone-6-interactive-shell`. The operator clarified that the
accepted conversation-first Rust TUI is the intended application interface.
The installed `waywr` now uses that renderer/editor for project-service events
and actions; the old three-pane module is removed. Scripted Decision Log scenes
remain behind the separate explicit evaluator command only. Future UI work must
target this **one installed application**, not add features to the deleted
three-pane view.

Read [the unified UI integration report](unified-ui-integration-report.md) for
the exact paired release (`m6-unified-ui-r5`), Fish launch command, test evidence
and current backend gaps. The Rust frontend owns presentation and draft state;
the selected project's Python service owns authoritative task state, context,
decisions, approvals, recovery and Herdr worker execution. Installed private
acceptance exercised task submission, scope authorization, a choice, verified
artifact opening and fixture approval, followed by detach. A second private
check exercised exit, `waywr resume` and detach through the same interface.
These were simulated checks; fixture decisions are not operator approval.
Focused Rust checks (39) and Python attach/lifecycle checks (12) passed. The
stable accepted evaluator binary remains byte-identical. Final evidence copies
under `.local/milestone6/unified-ui/evidence/` were SHA-256 verified (170 and
65 regular files); private test units and servers stopped. The pre-existing
`waywr-<private-id>.service` for
`.local/milestone6/ui-prototype/evaluations/operator-02` remains
active and was not touched.

The backend still turns new text into deterministic task proposals rather than
open-ended coordinator conversation. The installed UI labels its default
destination **New task request**, marks its service simulated, and refuses
unsupported conversation/steering targets while keeping drafts. The backend
accepts listed choice answers only. Direct worker-pane focus from the drawer,
proposal revision and provider switching remain separate contracts. No native
model calls or shared Herdr configuration changes occurred. The operator's
earlier UI acceptance remains an observation of the scripted evaluator;
physical Ghostty/Herdr behavior of this installed build still needs their
observation. Do not merge or publish without approval.

The lifecycle r10 section below is historical evidence for the previous
frontend. Its old Ctrl-S/three-pane walkthrough is superseded by the report
above. Preserve its evidence and historical approvals unchanged.

## Session lifecycle implemented; final installed simulated checks (2026-10-11)

**Branch:** `milestone-6-interactive-shell`. The operator accepted the latest
**isolated scripted UI** appearance/input, including full available
conversation width. This is a hands-on observation, separate from automated
verification; it does not accept production integration or native agents.

The approved [session lifecycle proposal](session-lifecycle-proposal.md) now
has a project-scoped implementation. Rust presents `/exit` with an explicit
Exit session action, `/detach`/Ctrl+Q, and no-run-ID `waywr resume`; Python
owns the generation/revision phase, drain, dispatch fence, checkpoint and
reconcile-before-resume. A 30-second foreground deadline never kills or
retries a worker; verified live and ambiguous work get distinct durable
attention notices. The older exit cannot stop a newer resumed session.

Read [the lifecycle report](session-lifecycle-report.md) for the exact tests,
known limits and Fish walkthrough. Final paired local release is
`m6-lifecycle-r10` at `.local/milestone6/lifecycle/install-r10/` (manifest
SHA-256 `b5989703252b050fc15e9d3973a9607c52c8efdc0ca270c6ad6fa7985ae42921`).
The private Herdr proof, final installed fake workflow, fixture approval and
ordinary-terminal exit/resume/detach evidence are in
`.local/milestone6/lifecycle/`, including `evidence/final-a17/` and
`evidence/final-fg07/`. The 270 and 64 copied regular files were hash-verified against their originals and have copy manifests. Focused
verification: 24 Python tests, 39 Rust tests (one long probe ignored), and
both final installed simulated checks passed. The fake fixture approval is not
operator approval. The production lifecycle panel and live long-running
attention behavior still need operator observation; native calls remain
blocked. Final test-owned units/private servers are stopped. Shared Herdr
sessions, historical approvals and retained panes were untouched.

The earlier milestone-6 `/tmp` preservation, stable production installation
and final-build missing-workspace check remain separately pending. Do not
merge or publish without operator approval.

## Session lifecycle authorization at start of work (historical, 2026-10-11)

The operator verified and accepted the latest **isolated scripted UI**,
including full available conversation width. This is operator observation,
separate from the 32 Rust tests and 12 isolated PTY sessions; it does not
accept production integration or native agents. The
[session lifecycle proposal](session-lifecycle-proposal.md) is approved for
implementation with explicit resume-versus-exit/shutdown fencing and durable
attention notices for known-running/stalled calls. Implement the isolated
Herdr foreground-client return-to-shell gate first; do not touch the shared
client, workspace or worker panes. Keep all work on
`milestone-6-interactive-shell` and preserve historical evidence.

## Scripted UI accepted; follow-up refinement and lifecycle design for review (2026-10-11)

**Operator layout correction after the prior checkpoint:** conversation text
must use the full width available in the terminal, limited only by an open
task/worker drawer pane. The arbitrary 96-column cap was removed. The final
scripted evaluator uses all 160 columns at 160×48 with its drawer closed and
the 115-column conversation pane when the drawer is open. This is a requested
change with automated verification, not a new operator observation of the
rebuilt screen. The prior stable binary and manifest were byte-copied to
`.local/milestone6/ui-prototype/evidence/build-ed7db2af/` before replacement.

**Branch:** `milestone-6-interactive-shell`. The operator personally verified in
the current Ghostty/Herdr setup that physical Shift+Enter inserts a newline;
the composer contrast and lack of underline, readable slash descriptions,
task/proposal grouping, indentation, colors and action presentation; and F2
behavior plus its existing footer wording. These are operator observations,
not automated checks, and accept only the prior scripted UI visual/input
refinement. Production integration and native calls remain unaccepted.

The isolated scripted evaluator was further refined: blinking cursor,
left-aligned composer on dark `#41454C`, dark `#4A4D54` user-message block
starting `> `, grouped `•`/`└` coordinator and action results, the full colored
Wayward Wrangler title with one short subtle-mode startup effect, and slash
Enter acting immediately while Tab only completes. F2 footer text is
unchanged. The stable binary is
`.local/milestone6/ui-prototype/bin/waywr-ui-prototype`, SHA-256
`2299064937232ec252e5316fedc2d4a8b4db79c4a66b083dbb4aec228e2a0244`.
Its final manifest and 12-session PTY summary are in
`.local/milestone6/ui-prototype/`; the previous binary/manifest were preserved
under `evidence/build-dcd00591/`, and previous evaluation journals remain.
32 focused Rust tests passed with one long probe ignored; final binary PTY
checks passed for modified Enter, F2, paste, slash actions, reopen, narrow/wide
layouts and terminal restoration. No model, worker, service or shared Herdr
resource was launched or changed. See [current prototype report](tui-prototype-report.md)
and [Fish guide](tui-prototype-guide.md); the new appearance still needs a
hands-on observation. A crate-wide `cargo fmt --check` reported formatting
differences, including untouched files; no broad formatting rewrite was made.

At this earlier checkpoint the [session lifecycle proposal](session-lifecycle-proposal.md)
was **design only**. The top section now records its subsequent implementation
and isolated Herdr 0.9.3 return-to-shell verification.

**Still pending:** prior milestone-6 `/tmp` acceptance evidence preservation,
stable paired production installation, actual stop-lifecycle correction and
final-build missing-workspace check. Native isolation remains unverified, and
authenticated calls remain blocked. Keep historical approvals, all retained
Herdr work and project release bindings untouched. Next action is operator
review of the new scripted evaluator and lifecycle proposal, then separate
authorization for any lifecycle integration.

## Previous scripted UI refinement checkpoint (superseded evaluator, 2026-10-11)

**Branch:** `milestone-6-interactive-shell`. The isolated Rust evaluator was
refined after the operator's screenshots and feedback. The then-current binary was
`.local/milestone6/ui-prototype/bin/waywr-ui-prototype`, SHA-256
`dcd00591571edf25375cf3c02ea568d8e81183193fd6599d440f38435ab987c4`.
The prior `operator-02` data directory remains preserved; the current Fish
commands are in [tui-prototype-guide.md](tui-prototype-guide.md). The prior binary and manifest
are archived in `evidence/build-17170b8a/` and `evidence/build-5556f2f7/`;
existing `operator-01`, journals,
historical evidence and approved runs were not changed. The then-current
`manifest.json` hashes the evaluator/source and names the release PTY evidence.

The composer has a padded contrast surface and **›** prompt; the textarea's
default cursor-line underline is disabled. Shift-Enter inserts a newline when
reported distinctly, Enter submits, and F2 remains the multiline fallback.
The app requests and restores CSI-u keyboard enhancement locally. The view has
semantic dark/light/mono styling, indented task/action groups, readable
Markdown/code, compact delivery status, simplified speaker labels and a
responsive slash menu. Production UI/backend behavior was not changed.

Verification: 27 prototype Rust tests, five existing Rust client/library tests
and ten final-release isolated PTY sessions passed. PTY coverage includes
60×18/160×48, CSI-u Shift-Enter, literal pasted commands, multiline draft
reopen, detach and SIGTERM restoration; idle repaint output was zero. The
summary is `evidence/refinement-release-02/pty-summary.json`. A separate
legacy modified-Enter PTY probe did not add a newline; it is preserved in
`evidence/ghostty-legacy-sequence-01/`. Ghostty 1.3.1 supports enhanced keys,
but this Herdr session uses tmux 3.4 with `extended-keys off`. Physical
Ghostty/Herdr Shift-Enter, contrast, selection and effects still require the
operator's observation. No shared terminal/Herdr setting was changed.

All test-owned processes exited; no new service or Herdr resource is retained.
No authenticated model call or worker was launched. The prototype remains a
fixed script; G1–G6 production integration gaps and native isolation remain
open. Earlier milestone-6 evidence preservation, paired production installation,
stop-versus-detach lifecycle correction and final-build recovery work remain
pending. Do not conflate this UI refinement with those tasks or proceed to
production integration without separate scope.

## Scripted Rust UI prototype ready for operator evaluation (2026-10-11)

**Branch:** `milestone-6-interactive-shell`. The approved UI-only prototype in
[tui-interaction-spec.md](tui-interaction-spec.md) is implemented. Read the
[prototype guide](tui-prototype-guide.md) for exact Fish launch/reopen commands
and the [prototype report](tui-prototype-report.md) for verified behavior and
limits. The stable evaluator binary and ignored journal/evidence root are
`.local/milestone6/ui-prototype/`. Final binary SHA-256:
`17170b8a2af03ab9eaa332e0d5f92def7f910afb92abf78f3de90e0404a93343`.
`manifest.json` records source hashes and evidence references; previous
evaluation binaries remain in `evidence/build-*`.

Checkpoint commits `fa3361c` (component/editor proof) and `f6fb5c9`
(scripted session/presentation) precede this closeout. The separate
`waywr-ui-prototype` binary uses Ratatui/Crossterm, ratatui-textarea,
tui-markdown and TachyonFX. It has a conversation-first interface, explicit
composer target, slash picker, visible scope/question/review controls,
illustrative coordinator settings, local fake worker preview, and a bounded
Decision Log scenario with preserved simulated answer and candidate binding.
No production UI/backend source was changed; the shared Rust manifest/lock was
updated to compatible component versions. Existing Rust client/library tests
still pass. The production UI retains colon commands and its older editor.

Verification: 19 focused prototype Rust tests and five existing production
Rust tests pass. An additional explicit 30-second TestBackend probe passed
(1,924 samples; 18.90 ms maximum input-plus-render time on this host). The
final stable binary passed isolated source-free PTY checks for bracketed paste,
resize, Alt-Enter, F2/Tab/Send fallback, detach/reopen and SIGTERM restoration;
no idle repaint bytes were emitted in the measured window. See
`evidence/final-pty-06/pty-summary.json` and `evidence/final-latency-30s.log`.
The selected approved historical specification hash was reverified. All
test-owned UI processes exited. No model, worker, Python service, Herdr control
call or shared resource change occurred. Ghostty/Herdr look, focus, selection,
modified-key behavior and effects still need the operator's evaluation.

The interface remains a scripted demonstration. It does not prove model-backed
conversation or real Herdr worker inspection. G1–G6 production integration
gaps in the interaction spec remain. Native calls stay blocked. Do not proceed
to production UI replacement or backend gap fixes without separate scope.

**Earlier hands-on preparation remains pending:** preserve previous `/tmp`
milestone-6 acceptance evidence in stable `.local`, install the paired
production bundle, reconcile session-stop semantics with the approved plan,
repeat final-build missing-workspace recovery and prepare the production
disposable project. The new `.local/milestone6/ui-prototype/` evidence covers
only this scripted UI. Preserve all historical runs, shared Herdr tabs and
services. Stop for the operator's simulated UI evaluation; do not merge or
publish.

## TUI interaction design ready for review (2026-10-11)

**Branch:** `milestone-6-interactive-shell`. Implementation remains paused.
Read [tui-interaction-spec.md](tui-interaction-spec.md) and the updated
[milestone-6 proposal](milestone-6-interactive-shell-proposal.md). This design
pass inspected the checkpoint `ac5f8e8`, frontend, service/IPC, review and
workflow answer code, plus the explicitly selected Project Decision Log
specification. The working tree was clean before documentation changes.

The design selects a conversation-first alternate-screen UI with an anchored
multiline editor, Enter submission, visible multiline fallback, optional Vim
navigation outside editable fields, slash-command discovery and contextual
authorization/question/review actions. It supplies concrete 80×24, 120×36 and
160×48 layouts, key/command tables, themes/accessibility constraints, bounded
TachyonFX accents and exact source-backed IPC mappings. Component manifests
were researched; dependency compatibility is proposed, not compile-tested.

The proposed next approval is limited to a separate `waywr-ui-prototype`
scripted Rust binary, with a coherent Project Decision Log scenario and local
demo-only journal. Its four bounded steps and acceptance checks are in the
specification. No real services/models/workers or production workflow mutations
belong in that prototype. Scripted responses are explicitly not model-backed
interpretation. Historical free-editing approval remains inherited; the demo
may ask about the unresolved alternatives minimum, retaining supersession and
cross-tab findings. It never reapproves or edits the historical run.

Source inspection identified integration gaps G1–G6: free-form question
answers; paged history/artifact/context/usage/worker projections; supported
Herdr worker/Vim display IPC; proposal revision/general conversation/active
steering; approved stop-versus-detach lifecycle; and outbox settlement after
lost replies. The UI prototype must not claim these are implemented. In
particular, the existing stop's quiescence requirement is an unresolved
deviation, not an approved replacement for pausing dispatch while a worker
finishes. No backend redesign or gap fix was performed in this turn.

Rechecked: colon commands and old editor remain in source. Slash correction,
command picker, new panels and tests have not landed. `.local/milestone6/`
still does not exist; evidence copying, stable paired installation, fresh
hands-on project and final-build missing-workspace check remain unstarted.
The old report/README describe the old runnable interface; the new spec is
design, not new runnable instructions. Historical `/tmp` evidence references
are preserved unchanged. This turn launched no tests, workers or services,
installed nothing and did not change shared Herdr resources. The process
inventory in the pause entry below was not revalidated; no current service
status is inferred from it.

Only the specification, proposal and this handoff are changed. Wait for design
review before implementation. Native calls remain blocked; no merge/publish.
Routine design choices are resolved; no additional substantive product choice
is required to review the proposed UI-only scope.

## Paused for TUI interaction specification (2026-10-11)

The operator paused the milestone-6 hands-on preparation before implementation
and requested a comprehensive TUI interaction specification. The branch was
clean at `6261a2f` when this pause was received; the interrupted preparation
turn had made no source or documentation edits, launched no tests, and created
no `.local/milestone6/` directory. This handoff update is the only checkpoint
change. Do not resume UI implementation or redesign until the design prompt.

The requested slash-command correction (`/plan`, `/confirm`, `/inspect`,
`/answer`, `/open`, `/approve`, etc.), `/` command picker, contextual gate
actions, and corresponding help/tests/walkthrough are **unstarted**. The
current TUI still uses colon-prefixed commands. The requested reconciliation
of session-stop semantics and final-build missing-workspace check are also
unstarted; do not treat the current report's quiescence statement as an
accepted lifecycle change. Evidence preservation, verified copies, stable
`.local/milestone6/` installation, and fresh hands-on project are unstarted.

The earlier `/tmp/waywr-m6-accept-r3` bundle,
`/tmp/waywr-m6-accept-install` and `/tmp/waywr-m6-reinstall` private
installations, and final single/two-project evidence paths listed below still
exist. They were neither moved nor rewritten. No test-owned `waywr-*` user
unit or `agent_workflow.session_service` process was running at this pause.
Shared Herdr sessions and historical evidence were not touched. No tests were
run in this pause turn. Native calls remain blocked; do not merge or publish.

**Branch:** `milestone-6-interactive-shell`. The Rust/Ratatui frontend and
existing Python workflow backend now have a paired, locally installed simulated
build. The approved plan is [milestone-6 proposal](milestone-6-interactive-shell-proposal.md)
at `16bb5ce`; the current results and operator walkthrough are in the
[milestone-6 report](milestone-6-report.md). The latest implementation boundary
commit before closeout documentation is `04e2419`. Do not merge or publish
without review. Native coordinator/worker acceptance remains separately gated.

The service, fake coordinator, durable queue, multiple human questions,
artifact-bound approval, explicit cross-project read grants, release routing,
and Rust TUI are implemented. One project may have one executing workflow;
different projects execute independently, even with the same run ID. The
reviewed Rust/Python ownership and recovery contracts remain in force. The
versioned persisted semantics are in [state-contract.md](state-contract.md).

Final paired test release R3 is under `/tmp/waywr-m6-accept-install`, with a
second fresh prefix `/tmp/waywr-m6-reinstall`. Final simulated one-project
evidence is `/tmp/waywr-m6-accept-project-final`; its quiescent copy and
location rebind are `/tmp/waywr-m6-relocated-final`. Independent two-project
evidence is `/tmp/waywr-m6-two-3n0jyy10`; the earlier two-question Decision
Log continuity fixture is `/tmp/waywr-m6-project-n`. A mixed R1/R3
same-run-ID fixture is `/tmp/waywr-m6-two-jhdbw_5x`. These test-owned local
resources may disappear with `/tmp` cleanup. No test-owned `waywr-*`
systemd unit remained running at closeout. The shared Herdr server, historical
approved runs, current user services and tabs were not changed.

Focused offline checks: 20 Python tests and five Rust tests pass. The final
installed R3 pair passed a service restart and five single-count fake worker
stages, then the independent same-run-ID A/B test with targeted cancellation.
Aliases, private packaged resources, side-by-side release binding,
upgrade/rollback, relocation and prefix reinstall were exercised. Terminal
focus/readability, long history and Vim navigation still need operator
observation. The live missing-workspace case was not repeated on the final
pair; mocked service recovery and earlier isolated evidence remain. Native
tool/context isolation is not established; authenticated acceptance stays
blocked. No native model calls, web app, TUI provider switch, global install,
shared plugin update, branch merge or release occurred.

## Previous planning handoff (historical)

**Branch:** `milestone-6-interactive-shell`. Latest implementation checkpoint:
`a03d2cfc863c2c80f68c7e5af722cba108e417b8`. Durability/authorization architecture
review: `7c7924f`. The user selected Rust + Ratatui and authorized the current
planning revision only. Do not resume implementation until approved. The latest
project-independence correction supersedes installation-wide admission and
upgrade restrictions from `fde2b2f`; the Rust/Python split is retained.

Read the revised [milestone-6 proposal](milestone-6-interactive-shell-proposal.md).
It supersedes Textual and recommends a paired Rust/Python Linux release bundle;
source, dependencies and entry points from the paused checkpoint remain intact.
The Rust planning pass inspected that diff, Python source and official
Ratatui/Crossterm/Python/pip documentation. The current focused correction
rechecked admission, registry, context, adapter permissions and review routing. No installs, workers, services,
shared configuration or coding subagents were involved.

## Revised implementation handoff

- Rust `waywr` owns CLI parsing, terminal client attachment and replaceable UI.
  A Python bootstrap helper owns project setup/unit provisioning; the Python
  project supervisor owns Herdr topology/UI claims, durable state, execution
  and recovery. Rust never writes workflow databases or owns model processes.
- Proposed IPC v1 uses same-UID Unix sockets and bounded newline-delimited JSON,
  typed commands/results, task/version checks, command-ID replay, cursor-based
  event batches and bounded client queues. Protocol/state semantics are
  independent of Python classes and LangGraph internals. No such IPC or queue
  existed at pause; these are planned additions.
- Recommend one Linux x86_64 glibc bundle with Rust binary, backend wheel,
  complete hash-pinned CPython 3.12 wheelhouse and stdlib installer. System
  Python 3.12 plus venv is an explicit prerequisite. Installer creates a private
  environment at a final versioned prefix; one owned `waywr` link and aliases
  select a default for new projects. Existing projects re-exec their own bound
  compatible release before backend startup. Upgrade/rollback checks apply only
  to the selected project; installation/default selection never scans all stores.
  Referenced releases are retained even if another project is unavailable. Public
  publishing remains outside scope; this recommendation awaits plan approval.
- Six steps: (1) Rust/IPC simulated proof and minimal paired-install discovery;
  (2) Python bootstrap/minimal service plus real isolated Rust Herdr gate;
  (3) full project service lifecycle and existing graph driver;
  (4) durable queue/fake coordinator; (5) multiple questions/Decision Log
  continuity; (6) full installed simulated acceptance and operator walkthrough.
  Inside-Herdr reconnect and closed UI replacement must be proved, not inferred
  from the earlier cold/warm Python test.
- Queue “skip” shorthand is removed: use explicit operator cancellation or
  abandonment only when execution is resolved and no owned process survives.
  Never advance past an uncertain task within that project. Replace the
  registry-wide loop in `cli.no_other_execution` and `create`'s registry lock
  with project-scoped checks/admission locks. Retain the per-store unique index
  and execution fences; independent projects may run concurrently.
- Retaining Python is pragmatic for this milestone. Document versioned record
  semantics and public behavioral tests now; later replace bounded backend
  capabilities, then new-run execution, then separately authorized quiescent
  legacy migration. LangGraph checkpoints, exact historical hash serialization,
  recovery/process identity and adapter permissions are migration constraints.
  No engine rewrite, migration/export framework or replacement backend now.

## Project-independence correction

- A project owns its queue/conversation/context, state, approvals, supervisor,
  locks, counters, budgets, workspace/pane identities and release binding.
  A busy, blocked, failed or uncertain A does not block B, including identical
  run IDs. One executing workflow and at most one coordinator call are limits
  **per project**. No installation-wide scheduling owner is introduced.
- Registry metadata is only deterministic project lookup; it never enters model
  context or acts as agent discovery. Default packets contain task-selected
  current-project data only. Separate model-visible projection from execution
  metadata for new packet contracts; retain historical prompt/output bytes.
- Foreign context requires an operator grant or recorded configured relationship:
  direction B→A, purpose, finite selectors, scope, budget, revision and status.
  Reads are read-only and bounded (16 KiB foreign material within 48 KiB total),
  with source identity/hash/provenance. No B execution authority, transitive
  imports or full conversation import. Covered reads do not ask again.
- Default imports pin selected snapshots; only explicit must-match dependencies
  track source changes. Unrelated B changes do not invalidate A. Revocation
  blocks future reads and agent delivery, including cached/derived imports,
  while preserving audit history and reporting already-permitted exposure.
- Source limitations: `context.resolve` has no cross-project grant feature and
  hashes overly broad final objects for selected findings; new imports need
  per-object dependencies. Codex read-only mode is not a read-access boundary;
  denying all general read/search tools and auto-context must be established
  before native acceptance. Post-event rejection cannot undo disclosure.
  This plan promises application-enforced context isolation, not an OS boundary
  against trusted same-user binaries or provider-side account retention.
- Release install/default selection is independent of project upgrades. A stable
  release descriptor routes bare `waywr`, aliases and review links to the
  selected project's paired release. Project-local maintenance checks and
  append-only binding records permit A/R1 and B/R2 to coexist. Reference checks
  belong to future release deletion; missing projects retain references but
  cannot block unrelated startup/install/upgrade. No automatic release GC.
- Shared mutable dependencies are enumerated in the proposal: metadata locks,
  default link/release refs, systemd user manager, Herdr server, auth/provider
  resources and review plugin. Each action targets verified project resources;
  shared host/server failures remain possible, without coupling task authority.

Acceptance additions: concurrent A/B identical-ID runs; B progresses while A
awaits input or is uncertain; wrong-project cancellation/approval/recovery/UI
routing refusal; mixed bound releases; unavailable unrelated project; foreign
sentinels absent without a grant; selected-only imports; revocation across
restart; no transitive reads or unrelated-state invalidation. Keep these public
CLI/IPC/packet/receipt checks usable for a later backend replacement. All are
planned, not runtime-verified in this correction. No source code was changed.

No blocking product clarification remains for planning. The main installation
tradeoff remains the explicit Python prerequisite; a bundled interpreter is deferred.
Approval to resume and native model/budget authorization are still separate.
Recommended eventual native settings remain Claude sonnet/medium coordinator
and author, Codex gpt-6.1-sol/high critic; at most two coordinator calls and five
worker stages/two critiques, no automatic retries. Simulated responses never
authorize a human choice or final approval.

## Paused checkpoint inventory (historical, not new runtime verification)

## Implemented and partially implemented

- `src/agent_workflow/shell.py` adds a proposed bare `waywr` entry and routes
  existing noninteractive commands to the old CLI. It infers the exact cwd,
  initializes a new project, saves an explicit Herdr socket in project config,
  and creates or focuses an owned Herdr workspace/TUI pane through the socket
  API. It records UI launch intents, binding, and process claims in the target
  state. The actual `waywr` launcher reached the same owned TUI pane from cold
  and warm ordinary terminals in an isolated test.
- `src/agent_workflow/tui.py` is only a Textual placeholder with a header,
  project/socket label and Detach key. It does not have a conversation, tasks,
  questions, review, steering, status, or service connection. It is
  preserved pending the approved Rust replacement, not selected for further work.
- `pyproject.toml` adds the `waywr` console script, routes compatibility aliases
  through the new entry, and provisionally pins `textual==8.2.8`. These entry
  points were exercised from source with `python -m agent_workflow.shell`, not
  from a built wheel. `requirements.lock.txt` has not been updated. No package
  version bump or installed-wheel acceptance was performed.
- The existing per-run supervisor, workflow and state code from earlier
  milestones is preserved; there was no project IPC or queue implementation.
  **No new project service, IPC, queue, task-version, coordinator,
  multiple-question, or recovery implementation was started.** The historical
  per-run controller remains the only implemented workflow controller.

## Checks and failures

- Static Herdr 0.9.3 source/schema review preceded the test. The isolated
  ordinary-terminal proof used `HERDR_SOCKET_PATH` and `HERDR_CONFIG_PATH`
  under `/tmp`, with no `HERDR_ENV` supplied to the launcher. The final generic
  check confirmed cold start, API protocol 22, warm attachment and a spawned
  pane with Herdr-injected `HERDR_ENV=1` and matching pane/workspace IDs.
- The actual source launcher then passed cold and warm startup on an isolated
  server/project. Both terminals used the same bound `w2:p2`/`w2:t2`; the live
  TUI wrote a matching process claim. Test root:
  `/tmp/waywr-m6-actual-axp73n8e`. The server was stopped afterward.
- Earlier test harness attempts failed because a zero-sized pseudo-terminal
  was rejected, an incorrect `server.status` method was used instead of
  `ping`, and a warm client did not exit promptly on SIGTERM. The corrected
  harness used a sized PTY, `ping`, and stopped only its isolated server.
  These were test-harness failures. They do not prove inside-Herdr launcher
  reconnect, lost-response reconciliation, or startup failure cleanup.
- `.venv/bin/python -m agent_workflow.shell --help` and `git diff --check`
  passed. No offline suite, wheel build, systemd lifecycle test, native call,
  or installed-package check was run for the partial implementation.

## Retained state and current limitations

No milestone-6 test process or service is running. The shared Herdr server,
client, approved `decision-log-01` controller and historical tabs remain as
before. Test-owned `/tmp/waywr-m6-gate-*` and
`/tmp/waywr-m6-actual-axp73n8e` directories retain isolated logs/config and
project state; their Herdr servers were stopped. The ad hoc harnesses are
`/tmp/waywr_m6_gate.py` and `/tmp/waywr_m6_actual_gate.py`.
`textual==8.2.8` and its dependencies are installed only in this checkout's
`.venv`; no shared installation or Herdr configuration was changed.

The checkpoint diff contains exactly `pyproject.toml`,
`src/agent_workflow/shell.py`, `src/agent_workflow/tui.py`, and this handoff.
Known incomplete behavior: no project service owns the UI lifecycle; a closed
TUI leaves an existing pane/binding that the current reconnect verifier will
refuse if its process is dead. The launcher thread can wait while an attached
client owns the terminal, and bootstrap error cleanup has not been proven.
No ordinary task execution, queue, human gate or durable conversation is
available from the placeholder. The Herdr cold/warm result establishes the
integration path only. Use the revised Rust plan for the next approval; the
current code still reflects the paused Python frontend experiment.

## Accepted architecture baseline before the pause

The reviewed [milestone-6 proposal](milestone-6-interactive-shell-proposal.md)
remains the record of requirements and evidence. Its Textual and distribution
choices are superseded by the current Rust proposal. Preserve the reviewed
supervisor/IPC/queue/authorization contracts carried into that proposal.
The approved Decision Log
artifact and native-trial budget remain untouched; native acceptance requires
separate authorization.

# Wayward Wrangler — milestone 5 native workflow trial

## Current branch and authorization

PR #2 is merged at `1fc74c6b01d807016a233b77a6f758dfd0889e98`.
Fetched `origin/main`; clean local `main` matched it before creating
`milestone-5-native-workflows`. **Every new work item starts on a dedicated
branch; do not merge a work branch without the user's approval.** Current
authorization now includes milestone-5 implementation and the single
`decision-log-01` native specification trial, with Claude sonnet/medium and
Codex gpt-6.1-sol/high, at most five stages/two critiques. No automatic
retry, model substitution, effort escalation, web-app build, checkout move,
merge or release is authorized. AGENTS.md and CLAUDE.md remain byte-identical
because approved context evidence pins them; their old milestone statements
are historical and superseded by current user authorization.

## Milestone 5 state and next action

Read [milestone-5 report](milestone-5-report.md) and the accepted
[proposal](milestone-5-native-workflows-proposal.md). The code, wheel 0.3.0,
fresh ignored pipx install and initialized target are in place. The shared
review plugin was switched from the checkout link to the installed copy;
its previous registration and rollback root are saved under
`.local/milestone5/`. A direct handler invocation opened the exact draft in
read-only Vim. Existing tabs/server were preserved.
The finding-protocol correction wheel is installed in the project-local pipx
environment (SHA-256 `672cd1298c8b967e16f6fcf84a8ad18f0b0999b7aac60ea0f1bf6a767d1deaab`).
Earlier wheels remain under ignored `.local/milestone5/` evidence.

**`decision-log-01` is approved as a provisional specification.** The operator's editing-policy answer B (free in-place editing,
no prior-version history) remains unchanged. The original Claude draft,
Codex critique-1, Claude revision-1 and Codex critique-2 each completed one
native call. A general finding-protocol correction interpreted critique-2's
five reused IDs as explicit reassessments of existing issues. Its original
output, receipt and rejection remain immutable; the interpretation is appended
at `operations/critique-2/recovery/finding-interpretation.json`, with a
stage-specific finding ledger. The graph then adopted the saved critique and
launched the one remaining authorized call: Claude revision-2, sonnet/medium,
completed in Herdr pane `wA:p8` / tab `wA:t8`. **Total native calls: 5/5;
two critiques.** No prior call was repeated and no further critique is
available. The owned supervisor service exited successfully at terminal approval; its unit remains enabled but inactive. The controller pane and completed worker tabs remain for inspection.

Review `temp/project-decision-log/.wayward-wrangler/state/runs/decision-log-01/artifacts/revision-2.md`,
SHA-256 `021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`.
The specification remains provisional with five open finding IDs: C1-2,
C1-3, C2-1, C2-2, C2-3. They cover alternatives minimum, supersession
relationship correction and cross-tab behavior; the accepted editing decision
settles none of them. Do not claim that revision-2 received an independent
critique. **Final approval was recorded by the controller as response `A1`, bound to the
exact artifact SHA-256 above.** Installed status is `approved` with no pending
gate; the five findings remain open. The user can inspect the artifact in Vim
through the review link or the installed `open decision-log-01 --artifact revision-2`
command. No duplicate approval or additional native stage is needed. This is an application specification, not
a built or browser-tested app.

The earlier duplicate-ID failure is historical. Source prompt, packet, schema,
result, receipt and rejection hashes are in the [report](milestone-5-report.md).
Future finding contract v2 separates application-owned new IDs from explicit
assessments of original evidence. Offline saved-output fixtures cover the
interpretation and replay. The old failure notice remains in event history;
current status suppresses it after successful recovery. The Herdr service unit
`wayward-wrangler-0c2d2965-decision-log-01.service` is enabled but inactive after the terminal run. The owned controller remains alive in `wA:t7` / `wA:p7`; completed worker tabs `wA:t3`, `wA:t4`, `wA:t6` and `wA:t8` remain inspectable. No worker processes are expected to be active. No coding subagents were used.

Installed CLI:
`.local/milestone5/pipx-bin/wayward-wrangler` with explicit
`--project ~/projects/example-workspace/temp/project-decision-log`.
Authoritative state is in the target's `.wayward-wrangler/state/`; status and
gate snapshot are described in the report. No coding subagents were used for
this development task.

## Milestone 5 closeout and next priorities

Final installed-build verification is separate from the historical native trial.
A wheel from the current branch (SHA-256
`021e2eb50d3be48df52710f7cf924cfb43f9f829991132151628e2e5552e90a6`)
was installed in a fresh `/tmp` venv. Forty focused tests passed from
installed site-packages, and one isolated Herdr server/disposable fake-worker
run passed six checks, including missing-workspace and controller restart,
stale approval refusal, exact-artifact approval across restart, and one call
per stage. The test server stopped; copied summary and log are under ignored
`.local/milestone5/closeout/`. Historical run evidence was only read.
The native trial required explicit infrastructure, revision and critique recoveries
across multiple wheels; it was not an uninterrupted run of this final wheel.
See the report for scope and remaining limitations.

Follow-up priorities (unimplemented): standalone interactive startup/reconnect
and coordinator UI; multiple durable human questions with question handling
separate from model-call budgets; and passing the approved provisional
specification plus unresolved choices into a subsequent task. The approved
specification did not verify a browser application. Do not start these without
a new scoped authorization.

## Completed milestone 4 outcome

Implementation is merged in PR #2. See [milestone-4 report](milestone-4-report.md)
and [README](../README.md) for installed setup and explicit legacy access.
Wheel 0.2.0 contains packaged skills, fake defaults and copied plugin assets;
fresh isolated pipx installation supplied both CLI names. New runs select a
project and separate state. Project-qualified links and a user-local index
route reviews; explicit quiescent rebind handles copied project locations.
Copied `context-01` context/adoption verified without the original root in a
container, while the original approved evidence hashes stayed unchanged.

70 offline tests and isolated installed-wheel Herdr checks passed, including
controller exit/worker survival, same-run-ID project isolation, plugin routing,
and stale context after relocation. A second container run completed with the
source path absent. Its site-packages were copied from the fresh pipx install;
the host run used the actual pipx entry point and clicked plugin handler. Test
answers/approvals are fixtures only. All test-owned servers/workers stopped;
the shared Herdr session and linked plugin were untouched. No native calls.
The milestone-3 isolated fault matrix also passed all 12 cases after the runner
change (`.local/milestone4/acceptance/recovery/result.json`), with no retained test
process. Final host, source-hidden container, recovery and copied-legacy evidence
is retained under ignored `.local/milestone4/acceptance/`; every copied regular
file matched its `/tmp` original by SHA-256. See
`.local/milestone4/copy-verification.json` and the report for exact paths.
The report's adoption SHA was corrected against the original evidence. A focused
review-link fix now refuses missing, changed or malformed state identity markers.

Native adapters were not runtime revalidated under the wheel;
active-run relocation, orphan recovery and arbitrary power-loss windows remain
outside this milestone.

Read AGENTS.md, README.md and docs/milestone-4-portability-proposal.md. Standalone
project only. Preserve all approved runs and their immutable source/decision evidence.

## Identity and checkpoint

Project **Wayward Wrangler**, package/repository/primary CLI **wayward-wrangler**,
description **Persistent workflows for coding agents.** `awp` remains an alias.
Milestones 1, 2 and context-01 remain complete; do not rerun approved trials.
The user authorized this naming change, initial `main` checkpoint and private
`OWNER/REPO` GitHub repository. No previous Git history existed.
The repository URL is https://github.com/OWNER/REPO; verify remote
and push status with `git remote -v` and `git status --short --branch`.

The root remains `~/projects/example-workspace`, workspace w8 retains
its label. Physical move remains deferred to preserve this active session and
the user's linked plugin. New installed runs do not depend on that root;
legacy evidence remains at its historical path or can be explicitly mapped in
a verified copy. Follow [project migration and local state](project-migration.md).
AGENTS.md and CLAUDE.md remain byte-identical because approved context pins them.

At the naming checkpoint, 50 offline tests passed, including both installed CLI
entry points and controller recognition for both names; `pip check` passed. Preserved source run files,
checkpoint values, run records, context-01 approval/answer, adopted receipt and
shared CLI settings were verified. Evidence is local-only at
`.local/evidence/wayward-wrangler-checkpoint.json`. Git ignores runtime evidence,
venvs and credentials. The user explicitly declined NAS backups; do not track them
as pending work or a migration prerequisite. GitHub holds the tracked project;
ignored runtime evidence and external Herdr/session state remain local-only.

## Architectural boundary and historical planning

Installed application, selected target, versionable project config/context,
state/records and Herdr display integration are separate. See
[architecture](architecture.md), [historical portability direction](portability-plan.md)
and the [145-reference inventory](path-reference-inventory.md). Herdr remains
required; no second backend was introduced.

## Prior authorization — portability planning (superseded)

Milestone 3 is complete and merged. GitHub PR #1 reports MERGED at
2026-10-10T23:13:39Z, merge commit `acc2e894eefe6e675c3023eaf6a4c7d009bcfbf8`.
After fetching, local main was clean and exactly matched origin/main before
creating `milestone-4-portability-plan`. Historical statements that milestone 3
is unapproved or awaiting merge are superseded; do not resume that work.

The proposal and three defaults were subsequently approved. Its five steps are
implemented and tested as reported above. Publication and switching the user's
shared plugin remain separate decisions.

## Completed milestone 3 outcome

Read [milestone-3 report](milestone-3-report.md) for predeclared expectations,
observed outcomes, fixes, inspection commands and remaining limitations.
65 offline tests pass; pip check passes. Two isolated Herdr suite executions
recorded 15 passing case results, 17 total simulated invocations (each counter 1),
zero native calls. The original 494 local evidence files are unchanged.
Evidence: `.local/milestone3/verification.json`, `live-01/` and `cancellation-02/`.
All owned test servers/runners/children were stopped; no test panes remain alive.
The active user Herdr server and Codex session were not disrupted.

Fixes: conservative status/action for uncertain execution, identity recheck before
permit, refusal to create when launch intent is missing but execution evidence
exists, and explicit runner state-root propagation for isolated tests. Workflow
and answer replay reused original receipts/exports without repeated calls or
logical decisions. Cancellation races have one authoritative immutable receipt.
Runner death with a surviving child and unresolved server/claim cases remain
uncertain; there is no new general adoption, blind retry or orphan cleanup feature.

Portability remains required in future planning. These configurable test paths
are not a source-independent installation or directory migration. Original scope,
permissions and recovery authorization are preserved. No authenticated model calls,
shared-setting changes, another project edits, NAS work or subsequent milestone occurred.

## Prior verified outcome — context continuity

- context-01 is **approved**, no pending human gate or active run.
- User chose **block, never overwrite** (not the model's recommended replacement).
  The inherited **Offline only** decision remained unchanged and was not reopened.
- All five native stage counters are exactly 1: original Claude draft, Codex
  critique-1, Claude revision-1, Codex critique-2, Claude revision-2. Two critiques
  total. Exactly four additional calls followed adoption; no repeated draft.
- Original draft receipt remains failed; exact output was separately adopted with
  append-only recovery evidence. Do not rewrite failure history as clean success.
- Controller 2941257 exited at unanswered gate; controller 2941261 saved the actual
  answer and exited before consumption; fresh controller 2941808 consumed it once.
  Phase snapshots prove unchanged completed-stage counters/bindings at restart.
- Both decisions reached every later stage. Critique-2 received the previous draft
  and 5,662-character delta. Final A1 binds revision-2, decision revision 1 and
  context digest. Final revision was not independently reviewed a third time.
- All recorded worker panes are closed; transcripts/receipts/artifacts remain.
  Historical controller w8:tH/pJ is not a verified live navigation target.
- 48 offline tests passed during recovery. Completion verification changed only
  documentation/evidence; no extra worker calls or test approvals were submitted.
- Approved source/demo files, checkpoints, run records and shared CLI settings
  still match .local/evidence/context-before.json.

## Scope and evidence

Explicit context selection is resolved/delivered automatically; this is NOT
project-wide relevance discovery. One new substantive question/run is a prototype
constraint, not a permanent requirement. Open-finding reuse and supersession
remain offline-tested. The selected milestone-3 process-failure matrix now passes;
see its report for remaining gaps.

Final evidence: .local/evidence/context-completion.json. Earlier failure,
recovery and unanswered-gate snapshots are preserved separately. Exact provenance,
recovery details and final approval binding are in docs/context-continuity-report.md.
Inspect with `.venv/bin/wayward-wrangler --project ~/projects/example-workspace --state-dir .local status context-01` or show revision-2. No gate needs
continuation and no further model calls are authorized for this run.

## Session continuity

Conversation: <private-session>. Workspace root was verified
as this standalone project; it is the active writable project. Resume here:
`codex resume <private-session> --cd ~/projects/example-workspace`.
