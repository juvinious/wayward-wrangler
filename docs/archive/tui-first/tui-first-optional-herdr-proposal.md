> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# TUI-first execution with optional Herdr views

**Current status (2026-10-13): approved at
`dafb4f8451c725611091e0b3b264ffe73e02b107` and implemented through
checkpoint `78e8fa5` for simulated acceptance.** The
[checkpoint-4 report](tui-first-checkpoint-4-report.md) owns observed behavior,
variances and pending operator evaluation. This document remains the approved
architecture and recovery baseline; references below to proposed work,
preimplementation source and pending approval preserve the original planning
context and do not override the report or [current architecture](../../architecture.md).

**Original planning status:** finalized proposal for review; implementation paused.
Source inspected at `ea9126f3963390193b59c740f228c0c7d41adb63` on
`fix/reinit-stale-project-registry`, 2026-10-13. This branch contains migration
checkpoint `7b5d69e79f6a67bc297d0133a55eee31df1f8936` plus five fix/planning
commits. Preserve them. That planning review ran no builds, tests, host
experiments, workers or services and changed no operator state. At the time,
the contracts below were proposals; their present implementation and test
status is in the checkpoint-4 report.

## 1. Recommendation and product boundary

- `waywr init .` / `waywr init SOME_DIR` initialize only the selected project's
  configuration, state identity and release binding, then return. Repeated init
  preserves them. No service, user registry write, Herdr connection or TUI starts.
- `waywr` opens the accepted Rust TUI in the invoking terminal. `waywr resume`
  reconciles the same project's saved session and explicitly resumes it. Bare
  startup does not clear an intentional exit pause. Explicit project/state
  selection and compatibility executable aliases remain supported.
- Existing coordinator interaction, monitoring, live/retained output, questions,
  artifacts and approvals are accessible in Waywr. Keep the accepted composer,
  keys, message grouping, slash actions and drawer; no new coordinator capability.
- The project supervisor owns scheduling/recovery. Each invocation runs through a
  runner in a **project-and-operation-attempt-scoped transient systemd user
  service**, independently of TUI and Herdr lifetime.
- Herdr is an explicit **read-only output mirror**. Environment detection never
  authorizes connecting, starting a server or creating panes. Opening a view never
  launches an agent; closing it never cancels work. Interactive attachment is out.

Keep Linux/systemd as the execution prerequisite. Recommend **systemd 255+ with
cgroup v2 and required properties verified**, matching inspected host package
metadata `255.4-1ubuntu8.17`, rather than promising untested older-manager support.
Init and offline inspection do not need a running manager. Do not enable lingering,
restart the manager or change shared settings automatically. Logout/manager
shutdown and reboot may interrupt execution; records survive, processes may not.

This supersedes the earlier mandatory Herdr worker-pane contract **for new
operations**. Existing operations keep their bound execution contract. Calling a
new view an output mirror is deliberate; it is not an interactive worker terminal.
No Python runtime, second scheduler, multiplexer or general plugin framework.

## 2. Evidence, existing components and authority

Source paths below refer to the inspected commit. Prior automated results are
linked evidence, not tests rerun or extended during this planning pass.

| Source | Confirmed current behavior / required boundary change |
| --- | --- |
| `frontend/src/rust_runtime.rs::{initialize,prepare,launch,ensure_herdr}` | Local-only init is implemented but writes Herdr endpoint configuration. Runtime bootstrap still requires Herdr. |
| `backend/src/session_service.rs::{bound_workspace,worker_target,attach_direct,tick_coordinator}` | Creates/binds worker workspace; server identity is required even for direct-terminal UI attachment and coordinator launch. |
| `backend/src/session_store.rs::{plan,envelope,coordinator_reserved}`; `workflow_store.rs::{effective_target,append_routing}` | Socket/workspace enter frozen plans, targets and user-visible events. Display routing is an execution dependency today. |
| `backend/src/operation.rs::{Control,Ensure,cancel,receipt}` | Launch intent precedes `layout.apply`; claim/server/pane checks precede permit. Application records and validators prove completion. |
| `backend/src/runner.rs::{run,execute}` | Requires Herdr variables; owns child pipes, execution limits, cancellation and receipt; idles after completion to retain the pane. Identity-checks permit, but needs a generation/retirement check immediately at dispatch commitment. |
| `backend/src/{execution,db_writer,session_service}.rs` | Atomic immutable publication, Linux PID/start/boot identity, Tokio I/O, bounded SQLite writer, deduplicated commands and session lifecycle already exist. Reuse them. |
| `frontend/src/{rust_cli,rust_review,rust_maintenance}.rs`; `frontend/src/prototype/view.rs` | Local diagnostics exist, but service bootstrap, observe, preflight/review helpers retain Herdr dependencies. Worker panel points outside the TUI. Project-scoped maintenance checks exist. |

The runner currently limits combined stdout/stderr to **32 MiB**, its queue to
**16 × 64 KiB**, and structured events to **50,000**. `transcript.log` lacks stream
labels. The service bounds connections at 64, blocking dispatch at 16, writer jobs
at 64 and IPC frames at 1 MiB. Preserve these limits and their overload behavior.

[Bootstrap refusal evidence](../portability/bootstrap-refusal-correction.md) proves a specific
pre-`layout.apply` refusal. [Direct-startup evidence](../rust-migration/rust-migration-startup-correction.md)
proves direct TUI attachment with Herdr still required. Their installed fake tests
are prior verification; neither establishes Herdr-free operation or this host design.

| Owner | Authority | Excluded authority |
| --- | --- | --- |
| Project service / existing SQLite writer | Tasks, transitions, command outcomes, questions, decisions, approvals, context grants, lifecycle; references/indexes execution evidence | A unit or pane status cannot approve/complete a task |
| Application records / runner | Frozen inputs, attempts, invocation reservations, process identity, output, validated results and receipts | Missing PID, heartbeat or output cannot establish non-execution |
| systemd | Hosting, job/invocation/cgroup/process observations | No workflow, budget, model retry or authoritative history |
| Rust TUI | Presentation, drafts, commands with stable IDs, bounded inspection | No direct database mutations or worker ownership |
| Herdr view adapter | Explicit display routing and view identity | No execution permits, scheduling, cancellation or context discovery |

One workflow executes per project. Locks, budgets, recovery counters, unit names
and view bindings are project-scoped. A shared installation, manager, Herdr server
or locator does not couple projects. Never target resources by global focus,
process name or name prefix alone. Registry metadata stays out of model packets.
These are application ownership/context boundaries, not a sandbox against a
malicious same-UID executable. Preserve existing native tool restrictions and
the separate isolation gate; systemd hosting does not expand or prove them.

## 3. Operation-host lifecycle

### Host properties and identity

Use a narrow Tokio D-Bus adapter to `StartTransientUnit`, mode `fail`, no auxiliary
units. Recommend `zbus` with Tokio enabled and its default async-io runtime disabled;
pin a compiler-compatible version during implementation. No shell/PTY launcher,
`--collect`, automatic restart, or second scheduler.
The inspected [zbus 5.19.0 runtime documentation](https://docs.rs/zbus/5.19.0/zbus/#special-tokio-support)
supports this feature selection; it is not yet a selected/tested dependency in
this repository. This planning pass changes no lockfile.

| Property | Proposed value / rule |
| --- | --- |
| Unit name | `waywr-op-<project-key32>-<operation-digest16>-<attempt-id32>.service`; generated hex components, never reused even after collection/reboot |
| Executable | `Type=exec`; exact absolute bound runner argv, project-local operation cwd, release manifest and executable hashes recorded before start |
| Lifetime | `ExitType=cgroup`, `Restart=no`, `RemainAfterExit=yes`, `CollectMode=inactive`; no `PartOf`/`BindsTo`/stop propagation from supervisor, TUI or view |
| Stop | `KillMode=control-group`, `TimeoutStopSec=3s`, `SendSIGKILL=yes`; unit stop only for verified-empty cleanup or explicit verified cancellation fallback |
| Startup | `TimeoutStartSec=10s` plus a 10-second application request/claim reconciliation window. Timeout is possibly delivered, never permission to repeat start. No new manager runtime-kill deadline. |
| Environment | Explicit adapter environment/authentication restrictions, no Herdr routing variables. Runner stdin null; worker structured stdin remains adapter-owned. No permission expansion. |
| Output | Application-owned bounded streams/diagnostics. Runner opens its diagnostic file before claiming; its stdout/stderr are null. No duplicate unbounded worker journal. Manager status diagnoses pre-exec failures. |
| Host failure | Request `OOMPolicy=continue`; no automatic restart after OOM/external stop. This does not prevent kernel, administrator or manager shutdown. |

Documented behavior: `Type=exec` waits for executable invocation, not application
readiness; `ExitType=cgroup` tracks remaining processes; `RemainAfterExit` can keep
an exited unit active. None proves a completed call.
([systemd v255 service reference](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.service.xml))
`service_sigchld_event` confirms main-process exit does not initiate the usual stop
transition with `ExitType=cgroup` while that cgroup remains populated. This supports,
but does not runtime-verify, the proposed child-survival behavior.
([v255 source](https://raw.githubusercontent.com/systemd/systemd/v255/src/core/service.c))
Transient units can be collected and do not survive reboot; unit/job identities
are not durable application history.
([v255 D-Bus API](https://raw.githubusercontent.com/systemd/systemd/v255/man/org.freedesktop.systemd1.xml))
Collection and group termination are explicit controls, not proof of non-launch.
([unit reference](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.unit.xml),
[kill reference](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.kill.xml))

Verify UID/user-manager connection, exact properties/argv, `InvocationID`, cgroup,
runner path/hash, bound release manifest and PID/start/boot identity. An inherited
invocation ID is a claim to corroborate. Keep referenced releases installed.

### Ordering and fencing

```text
reserved → start-intent → start-maybe-sent → claimed → permitted
         → spawn-committed → running → receipt-validated → result-applied

Before invocation reservation: proven-no-launch → retired → linked new attempt
Unresolved send/execution boundary: uncertain; no automatic replacement
Terminal: completed / cancelled / failed / output-invalid
Usable incomplete revision: provisional under the existing finding policy
```

1. Reserve one logical operation for the frozen task/stage. Publish packet/schema,
   authorization and budget references. Create a unique attempt and immutable
   launch intent before requesting a host.
2. Persist `start-maybe-sent` before handing the request to the async transport.
   Once enqueued, timeout/lost reply requires reconciliation of that exact name,
   never another start. `UnitExists` means inspect, not replace. Missing unit/PID
   is not positive proof that this request never ran.
3. Runner takes its lifetime attempt lock, publishes claim, waits. Supervisor
   verifies completed host startup and full host/claim/input identity, then
   publishes a generation-bound permit.
   A successful systemd job is not permission to invoke an agent.
4. Runner takes the project execution fence then logical-operation gate immediately
   before dispatch. Recheck active attempt, retirement, owner endorsement, session
   phase and authorization. Durably reserve the invocation and publish
   `spawn-committed`. A reservation spent before a crash is not refunded. Partial
   commitment stops uncertain; it is not eligible for automatic replacement.
5. Release gates before spawn/wait; retain dispatch-before-spawn and child identity
   publication. Once committed, the invocation may finish under its original
   identity even if a new exit/cancel races it. No second attempt is permitted.
6. Publish validated result/receipt, then apply once in a short SQLite transaction
   keyed by logical operation/result hash. Receipt-before-checkpoint crash reuses
   the saved result; it does not launch again.

Record project-owner and dispatch/session generations. Replacement supervisor
fences the old owner before issuing permits. An unconsumed older permit requires
an endorsement for the same attempt; committed work continues without a new call.
A delayed old start can at most create a runner that refuses stale/retired state.
Bound the runner's pre-permit wait to 30 seconds. On expiry it takes the operation
gate and either observes an already-committed dispatch or publishes a terminal
pre-dispatch refusal and exits. A late permit cannot override that refusal. This
adds positive evidence; it does not make a mere supervisor-side timeout proof.

Lock order is project fence → operation gate → existing budget lock. No inverse
order or waiting for a SQLite writer job while holding those gates. Local durable
file updates form short critical sections; no transaction/dispatch lock spans
D-Bus/socket I/O, process waiting or output streaming. Exit/resume/retirement use
this same fence. Persist command intent, apply durable fence, then record command
completion; crashes leave a pending command to reconcile before dispatch. Never
acknowledge exit while its fence is only an in-memory intent.

There is no file/SQLite distributed transaction. Reuse `execution.rs` publication
(sync, atomic publish, parent sync). Publish execution/recovery evidence before its
SQL index; replay repairs missing indexes. SQL transition intent precedes external
side effects. Conflicting identity/hash stops uncertain; do not select the newest
record arbitrarily. SQL owns workflow state; immutable execution evidence owns
what ran; projections do not become competing receipts.

### Reconciliation and lifetime

| Event | Required result |
| --- | --- |
| Supervisor restarts; runner continues | Verify exact unit/claim/commit/receipt, adopt monitoring. Same runner/counter; no new permit for committed work. |
| Start reply lost | Existing matching unit/claim can continue the same attempt. Missing or conflicting evidence remains uncertain, not a second start. |
| Runner dies; child survives | Preserve verified child/cgroup and mark execution uncertain with a known surviving process. No replacement runner. Pipes can close/SIGPIPE; child survival is not recovered output or a receipt. |
| Runner and child missing | Validate saved receipt/result first; otherwise uncertain, never infer non-launch. |
| Cancel races completion | Serialize cancellation intent and receipt publication with operation gate. Valid completion already published wins. Earlier cancel records actual signals/outcome, not an invented cancelled receipt; apply once. |
| Cancel with live runner | Existing durable request; verified group TERM, then existing 3-second KILL grace; recheck identity before signals. |
| Cancel with dead runner | Explicit command may stop only verified attempt cgroup/InvocationID/processes. Capture before/after identities and append cleanup outcome. Without a receipt execution remains uncertain; unverifiable ownership refuses signals. |
| Detach | Close UI only; service, worker and output recording continue. |
| Exit / graceful project stop | Fence new dispatch, deterministic checkpoint, drain calls. Existing 30-second foreground deadline returns to shell if needed, leaving service monitoring; never stop hosts just to finish exit. |
| Resume races exit | Keep exit-ID/revision/generation checks: old exit cannot pause/stop newly resumed service. Reconcile results before allowing dispatch. |
| Stalled / deadline | Preserve frozen runner deadlines (currently native 600 seconds), cancellation and output-limit policies. Foreground timeout adds no kill/retry policy. Verified overdue work is stalled/attention-required; unresolved execution is uncertain. |
| Reboot / manager loss | Validate saved results. Old boot identities cannot be current. Unresolved committed/maybe-sent calls remain uncertain; do not recreate their units. |

New runners exit after receipt and verified child termination; remove only their
terminal-retention idle loop. Successful/failed unit observations remain available
without an idle runner. Before automatic cleanup, preserve final properties plus
an append-only cleanup record and verify a fully reconciled terminal attempt and
empty cgroup. Stop/reset-failed only that exact unit, never manager-wide cleanup.
Retired proven-no-launch attempts follow the same rule. Unresolved hosts remain
for explicit inspection/cleanup; manager collection does not erase uncertainty.
Names never reenter the allocator. Evidence retention is independent of units.
The existing project service's bounded `Restart=on-failure` supervision remains
separate from operation units' `Restart=no`. It reconciles records on restart;
it must never propagate its restart/stop to operation units. All host requests,
timers and observation tasks belong to that service's Tokio task set, with bounded
admission and shutdown/join; ending those tasks does not stop manager-owned calls.
Unresolved work keeps the service responsive with a durable attention notice,
not an indefinite “working” indicator. If bounded service restarts exhaust, startup
shows retained diagnostics and affected operation identities; it neither resets
the restart budget silently nor starts replacement calls.

### Smallest isolated host experiment — not run

One test-owned project, fake runner/child harness, unique units using the properties
above. Fault barriers/drop replies and kill only verified test processes. Capture
unit properties, identities, claims, counters, output and cleanup under ignored
`.local/`. No shared-manager restart or operator service changes.

Expected outcomes fixed before implementation:

1. Kill/restart fake supervisor after enqueue and after claim: reconcile one host;
   child count one after permission, no implicit runner restart.
2. Lose start reply, including fast completion: retained identity resolves or
   explicit uncertainty; absence never causes another call.
3. Kill runner with an independently heartbeating child: child remains hosted;
   stream failure is recorded separately, no replacement, explicit targeted cleanup.
4. Retire a pre-permit attempt then deliver delayed start/old permit: old child
   count zero; linked replacement can execute once.
5. Race cancel/receipt and inspect successful/failed-unit retention and collection:
   one application outcome, no unrelated signal, retired name never reused.

Failure to preserve expected child ownership/lifetime blocks dependent integration.
Do not compensate with automatic restarts, `KillMode=none` or weaker identity checks.
Reboot behavior initially uses boot-change/missing-manager fixtures; these are not
proof of a real reboot, which needs an isolated VM and separate available access.

## 4. Proven pre-launch recovery

Recover the **same logical operation with a linked replacement host attempt**.
Keep task, authorization, frozen inputs and failed-attempt history. Versioned
append-only recovery records identify source attempt/hash, proof/evidence hashes,
validator version, retiring generation, reason, replacement and result. Stable
recovery IDs make repeated processing idempotent.

Positive proof of no agent invocation is limited to:

- A typed adapter refusal proving the start request was never enqueued, bound to
  its immutable intent; or
- A runner terminal pre-dispatch refusal published under the execution gate,
  proving it never crossed invocation reservation/commitment.

Both need complete valid evidence, no conflicting invocation reservation/commit,
and durable retirement under the gate all runners must consult. Checking for a
missing commit is meaningful only under this enforced versioned protocol; absence
alone is not proof. Retire before enabling the replacement. A delayed runner must
reject its tombstone even if it cached an earlier permit.

Generic D-Bus errors, exit codes, missing PID/unit/receipt, empty output, timeout
and heartbeat loss are not proof. Do not automatically retry manager-side pre-exec
errors until a tested negative guarantee and application fence establish equivalent
proof. Retiring a possibly executed attempt does not prove it never ran.

Automatically recover only proven pre-launch **transient** failures, e.g. failure
to connect before enqueue. Limit to **three host-start attempts total per recovery
episode**, including initial, with persisted 1- and 2-second backoffs and 10-second
request/claim windows. Restart does not reset counters. Possibly sent timeout stops
uncertain. Invalid configuration/executable, foreign unit or identity conflict
stops immediately. Exit/cancel fences pending recovery.

After exhaustion show: **“Could not start this operation. No agent call was
launched. Infrastructure recovery exhausted.”** `/inspect` offers an idempotent
**Retry infrastructure** action (`operation.retry_start`, expected task/operation
revision) only if the same proof still holds. It starts one new bounded recovery
episode without changing scope/models/budget. This is not retrying an agent call.
If commitment exists or execution is ambiguous, refuse and display the evidence.

Infrastructure attempts/backoff have a separate durable ledger. Only invocation
reservation at dispatch spends the coordinator/worker call; crash-spent reservations
remain spent. Never refund/reset a budget through recovery. Existing worker
five-call/two-critique and coordinator budget rules are unchanged.

### Existing blocked legacy task remains untouched

Task `<private-record>`, project
`~/projects/waywr-rust-sim-eval`, stays bound to `rust-local-init-v3`.
Its saved error plus dispatch ordering prove the specific bootstrap refusal before
`layout.apply`; missing files alone would not. It lacks the new fence/recovery
contract. `recover_prelaunch_coordinator` currently handles a different reserved,
no-launch workspace case, not this stopped task.

Separate options: retain blocked for inspection; or separately authorize a
**legacy-compatible Herdr release**, audited append-only recovery of this evidence,
and an explicit safe service/release change. There is no supported current command
to continue it. New backend installation is not that recovery. No target rewriting,
operator cutover, implicit new task, fabricated receipt or legacy migration here.

## 5. Removing Herdr from the entire ordinary path

| Dependency / source | New default | Explicit or legacy path |
| --- | --- | --- |
| `rust_runtime::{initialize,prepare,service_command}`, service CLI | No required/default Herdr endpoint; project execution host `systemd-user-v1`; service argv independent of Herdr | Preserve existing endpoint files byte-for-byte; optional view config separate |
| `launch`, `ensure_herdr`, `attach_direct` | Verify IPC peer UID/PID/start/boot, bound executable/hash, project and UI token; one direct-terminal attachment, no Herdr calls | Pane metadata only for explicit inspection; never UI ownership |
| `session_store::{plan,envelope}`, coordinator admission | Freeze execution contract/settings/context/budgets without display routing | Old plan hashes/targets remain unchanged |
| `bound_workspace`, `worker_target`, workflow routing | New operations use project host targets; no workspace creation/loss recovery | Version-dispatched old calls remain on bound backend |
| `operation::Control/Ensure`, runner claim/permit | Host/claim/fence identity, no Herdr environment | Old pane/server identities remain immutable evidence |
| CLI start/preflight/maintenance and transitive bootstrap | Same strict target selection/meaningful results; manager/host preflight, no authenticated call | Herdr flags only on explicit view or legacy commands; otherwise reject before effects, never silently ignore |
| Workers/logs/observe/process inspection/cancel | Project operation status and output; observe defaults to TUI view; verified runner/cgroup cancellation | `observe --viewer herdr` explicitly opens mirror |
| `rust_review`, artifact/open/review links | TUI verified artifact/decisions view; internal exact-hash approval. `open` uses it; read-only local Vim can suspend/restore invoking TUI | Explicit Herdr viewer/plugin remains optional; no automatic install/switch |
| Detach/exit/resume/recovery | Existing lifecycle generations/checkpoints reconcile hosts/receipts; Herdr loss does not affect execution | Old actual worker panes retain their old lifetime implications |
| TUI Worker/Inspect/status | Role/model/stage, live/retained output and supported actions in Waywr; blocked/uncertain never “Done” | Label old combined output and read-only mirrors honestly |

Audit remaining `Herdr`, `HERDR_*`, socket/workspace/pane and plugin references;
any retained dependency must be an explicit display operation or legacy routing.
Unset, stale, unusable or wrong-project inherited Herdr variables have **no effect
on ordinary operation**. Do not adopt an invoking workspace or send its metadata
to a model. Actual TTY capabilities drive terminal handling. Only an explicit
inspection may use the environment as an endpoint hint, with verification first.

### Versioned configuration, records and IPC

| Surface | Proposed version/contract |
| --- | --- |
| `project.toml` | Schema 2; existing key plus `execution.host = "systemd-user-v1"`; optional `integrations.herdr` absent/disabled by default. State override and release descriptor remain separate. |
| Application state | Schema 3 with attempt/recovery/view indexes; unsupported write schema rejected by database open and release routing. Existing schema-2 projects stay bound. |
| Plan/envelope/operation | Schema 2; execution contract replaces display target. Logical operation stable; `attempts/<id>/` contains version-1 host intent, observations, claim, permit, commitment, child and receipt. Receipt binds project/op/attempt/input/output and host. |
| Recovery / ownership | Version-1 append-only retirement, recovery, owner endorsement and cleanup records; mutable pointers are rebuildable. Old validators remain for read-only historical evidence. |
| UI / views | UI binding schema 3 requires direct process identity, no server/pane. Separate view schema 1 binds project/op/attempt/token/generation/server/tab/pane/explicit command. |
| IPC | Existing framing major 1, additive minor 1; negotiated `host_operations_v1`, `output_pages_v1`, `artifact_pages_v1`, `herdr_views_v1`. Mutations require compatible bound release; unknown methods/versions refuse before effects. |

Read-only additions: `operation.list`, `operation.inspect`, `operation.output.read`,
`artifact.read`. At the operator level, `operation.retry_start` is an explicit
retry action and `view.herdr.open` is an explicit display action. In the
implemented minor-1 IPC, retry uses the durable `command` envelope with its
command ID, payload digest and expected revision. Herdr opening uses
`view.prepare` → `view.sent` → `view.bind`: these are a bounded identity and
lost-send handshake, not a `view.herdr.open` frame or worker execution
command. This split preserves the approved rule that uncertain pane creation
cannot blindly create a second view. Reads scope to project and
selected task/operation; never accept arbitrary paths. Distinguish `busy` before
acceptance, `stale_state`, delivery uncertainty, execution uncertainty, no-launch
exhaustion, output limit, view unavailable and incompatible contract. Keep full
evidence in `/inspect`; concise conversation text gives the actual next action.
Human questions, approval binding, context provenance and grants remain unchanged.

### TUI output protocol and bounds

New output has a versioned fixed-width index: sequence, stdout/stderr/runner label,
raw byte range and capture timestamp. Preserve bytes; sanitize terminal controls
and OSC sequences only in viewers. Cross-stream sequence describes observation
order, not exact timing. Old unlabelled files display **combined legacy output**.

Retain the 32 MiB combined payload and bounded queue. Cap index at **65,536 × 32-byte
records (2 MiB)**; coalesce adjacent same-stream chunks up to 64 KiB. Metadata
exhaustion is an explicit output-limit failure, not silent dropping or unbounded
allocation. Keep structured-event cap. Persist bytes retained/reason and enforce
existing overflow termination with verified targeting. Record this additional
metadata bound as an intentional policy change and regression-test it.

`operation.output.read` takes task/op/attempt, cursor `{format,attempt,sequence,offset}`
and bounded size. Return labelled segments, next cursor, durable high-water mark,
live/terminal status and any gap/limit. Maximum **64 KiB raw / 256 KiB encoded**
per page; shrink payload if encoding expands. Never read beyond a complete
published index record. Ignore/report incomplete crash tails; do not overwrite
original bytes or claim delivery of an unpublished segment.

Follow uses at most **4 reads/sec/view**, one outstanding read, maximum four
admitted output readers/project; no server subscriber queue. TUI caches at most
**2 MiB** of rendered pages, evicting/reloading on scroll. Reads run off Tokio
executor threads with bounded admission, not in writer transactions. Reconnect
uses the same cursor; deduplicate by attempt/sequence/offset, not matching text.
Service epoch changes refresh status without resetting position or executing a
call. Stale cursors receive an explicit gap/reset; scrolled views show unread state.

A slow viewer cannot backpressure execution. Storage can still stall/fail: keep
stream memory bounded and report I/O failure/uncertainty, not a fabricated receipt.
Artifact paging has the same project/containment bounds and verifies the selected
hash. Approval binds the complete artifact, not just visible pages. Worker output
never becomes a question/decision/action without existing validated contracts.

### Optional Herdr views

Explicit **Inspect in Herdr** or `observe ... --viewer herdr` starts a bound-release
read-only viewer mode, never a runner. Its token is limited to one project/op/attempt
and read verbs; enforce this in IPC, not only its UI. No stdin forwarding or signals.
It reads the existing service or verified retained records if paused; it cannot
resume scheduling implicitly. Service unavailability may be displayed, not hidden.

Use supported direct socket control with explicit same-UID endpoint/server identity.
The [reviewed integration](../milestone-6/milestone-6-interactive-shell-proposal.md) distinguishes
Herdr's managed-pane **agent-skill** restrictions from its supported deterministic
application socket API. Never spoof `HERDR_ENV`; models never control Herdr.
See [pinned API documentation](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/docs/next/website/src/content/docs/socket-api.mdx).
Do not start an absent server automatically: **“Herdr view unavailable; output
remains available here.”** Ordinary operation must remain unaffected.

Persist create intent before API send. Repeated open reuses/focuses the matching
verified view. Lost reply reconciles token/executable/owned workspace; unresolved
creation does not create another view. Missing/closed view permits a new view
generation only on explicit reopen, never a new operation attempt. Target a
project-owned inspection workspace, not global focus or the invoking workspace.
Routing enumeration, if needed, stays out of model packets and unrelated context.

Current `workspace.create` adds a bootstrap shell; idle-check then replacement is
not atomic. Avoid that hazard: add a viewer using `layout.apply(workspace_id, argv)`
without replacing a foreground tab. The first explicitly requested inspection
workspace may retain its empty bootstrap tab. Prefer this display-only limitation
to risking shell replacement; ordinary TUI startup creates no Herdr tabs. Focus
changes only on explicit inspect. Closing/server loss changes view routing alone.

## 6. Implementation checkpoints and acceptance gates

After approval, create a new implementation branch from this planning checkpoint.
Each passing checkpoint gets a local commit, evidence summary, variance log and
state rollback boundary. No operator cutover.

1. **Host proof, then primitives.** First build only the section-3 isolated fake
   harness and evidence capture; pass start/claim/runner-death gates before routing
   production calls. Then implement host adapter and versioned attempts/fences,
   receipt reuse and cancellation. Gate: counters, lost replies, delayed stale
   starts, child survival and targeted cleanup. Failure blocks dependent work.
2. **Herdr-free workflow and pre-launch recovery.** Wire new config/schema, TUI
   attachment, plans/admission, coordinator/workers, CLI and lifecycle. Implement
   linked recovery and bounded retry action. Gate: installed fake workflow with
   no Herdr, poisoned environment, exit/restart races, exact budgets/approvals and
   preserved old release routing.
3. **Complete inspection and optional views.** Connect bounded output/artifacts to
   accepted TUI actions and ordinary CLI diagnostics. Only then add explicit Herdr
   mirrors. Gate: slow viewers, both streams/limits, reconnect cursors, retained
   review, view open/close/server loss without another invocation.
4. **Installed simulated compatibility acceptance.** Fresh Rust install without
   source/Python/Herdr for ordinary tests; separate private Herdr fixture for views.
   Two projects concurrently, fixture upgrade/rollback, unchanged history, release
   routing. Retain candidates/evidence, supply Fish walkthrough; physical terminal
   observations and native isolation/calls remain separate gates.

| Scenario | Required evidence (not executed this turn) |
| --- | --- |
| Init current/named/repeated | Selected local writes only; exact config/binding preservation; zero service, registry, view or worker starts |
| Herdr absent/stale environment | Complete installed fake TUI path including decision/review/approval/output; no Herdr executable lookup, socket connection or pane creation |
| Supervisor restart / lost launch reply | Same attempt or explicit uncertainty, never duplicate call/result; include before-claim and after-receipt boundaries |
| Recovery crash windows / exhaustion | Crash before/after retirement, replacement and index publication; old delayed counter zero; one linked replacement; persistent attempt/backoff limit; no model budget reset |
| Runner death / surviving child | Retained verified process identity, explicit uncertainty/output limitation, no replacement; cancellation affects only test-owned resources |
| Cancel / completion / lost replies | Both race orderings, one actual outcome, command replay idempotent, no manufactured receipt |
| Output limits / slow viewers | 32 MiB payload + 2 MiB index bounds, labelled streams, explicit limit failure, no deadlock/viewer stall; cursor replay no duplicates; terminal controls inert |
| Responsiveness | Slow manager/viewer/writer fixtures: status/answer/cancel responds or returns explicit busy within 2 seconds. No false acceptance on timeout; set bounds before tests, do not relax to pass |
| Detach / exit / resume | Work continues after detach; exit fences new calls, 30-second foreground deadline; resumed generation cannot be stopped by old exit; stalled distinct from uncertain |
| Changed boot / lost manager | Validate saved receipts, reject old live identities, no restart of unknown calls; simulated identity test not reported as real reboot proof |
| Two projects, identical run IDs | Independent units, output, budgets, cancellation, approvals and views; A uncertain/awaiting input does not block B; no unrelated model context |
| Optional views | Explicit-only creation, repeat-open reuse, lost-reply refusal/reconciliation, close/server loss no worker signal/counter change; wrong-project/token mutations refused |
| Historical / rollback | Original bytes/hashes unchanged, unfinished operations bound, unsupported schema refuses before write, no snapshot restoration over newly accepted work |

## 7. Remaining uncertainties and review decisions

- **Host experiment is the technical gate.** Documentation/source supports the
  selected properties, not all runtime timing/retention outcomes. Run the small
  experiment first after approval; don't infer safety from happy-path completion.
- **Recommended recovery limit:** no transparent stream/runner reattachment after
  runner death. Preserve uncertainty and explicit cleanup; do not add a second
  capture daemon or orphan recovery in this transition.
- **Recommended platform tradeoff:** require the Linux user manager for execution;
  no automatic lingering or promise of live-call survival across logout/reboot.
  Durable inspection/resume reports uncertainty instead of replaying calls.
- **Recommended display tradeoff:** a first optional Herdr inspection may leave
  its owned bootstrap shell tab; never risk replacing foreground work. Normal
  Waywr startup remains tab-free. Supported future API improvements can remove it.

These bounded defaults were submitted for review and later approved at the
commit identified above; this paragraph records the historical decision gate.
No other frontend/provider/framework selection is needed. Simulated packet/process
checks do not prove native tool/context isolation; authenticated calls remain gated.

## 8. State-aware rollback and historical compatibility

New projects use the new contract. Existing Herdr-backed and LangGraph runs retain
explicit bound releases. Historical read-only inspection may understand multiple
versions; execution must never silently convert targets, packets or permissions.

Only disposable acceptance projects are candidates for an explicit quiescent
upgrade: snapshot consistent SQLite/WAL and record manifests; introduce schema 3
and config 2 alongside byte-identical evidence; append the release-change record.
No unfinished old operation enters the new host. Reconcile service/UI/host ownership
before binding changes; keep referenced binaries. No operator migration shortcut.

- **Before new accepted work:** rollback code/binding and verified state snapshot
  only after test hosts are empty and the full event/attempt/output manifest still
  matches the boundary. SQLite revision alone is insufficient. Archive failed
  attempt evidence before fixture resets, never erase it to claim a clean rollback.
- **After new accepted work:** never restore the older snapshot. Forward-fix or use
  a release explicitly compatible with schema 3/execution contract, retaining
  current records and hosts. Source rollback is not state downgrade permission.
- Unknown/mixed schema, missing compatible release, changed manifest or unresolved
  old execution refuses with diagnostics. Never point a legacy release at new
  state or stop operator services to force migration.

Preserve r9, current Rust candidates, operator projects and failure/recovery evidence.
This changes future execution ownership, not historical interpretation or retention.
