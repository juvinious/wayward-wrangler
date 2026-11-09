# Current application architecture

**Scope:** current Rust host-backed schema 3/4/5/6/7 projects. PR #5 (`2e5cf12`)
supplied the host-backed baseline; merged PR #6 (`24fb317`) added the focused
schema-5 Claude increment. The implemented [scope extension](live-scope-contract.md) adds schema-6 proposals
and human scope decisions with execution disabled; installed mock evidence is in
the [scope report](live-scope-report.md).
The [continuous extension](continuous-coordinator-contract.md) adds state-7 causal
queue, typed recovery and formal question decisions; its [report](continuous-coordinator-report.md)
owns offline installed evidence, not native acceptance.
The [execution contract](execution-contract.md) owns detailed host, lifecycle and
recovery rules; [current limits](current-limits.md) distinguishes acceptance from
unverified behavior. Existing Herdr-backed
operations and unfinished LangGraph runs remain bound to compatible legacy
releases; this document does not translate their records or targets. The
[prior architecture](archive/architecture/architecture-pre-consolidation-2026-10-13.md)
is historical evidence for those older contracts.

## Product and ownership

Wayward Wrangler is a standalone Rust application. `waywr init .` or
`waywr init SOME_DIR` creates local identity/configuration only. `waywr` opens
the accepted conversation-first TUI in the invoking terminal. `waywr resume`
reconciles the selected project and restores its saved session. `--project`
remains explicit selection. The application does not need an enclosing Codex,
Claude or Herdr session. New schema-7 projects use the restricted structured-result Claude
proposal/conversation adapter, initially unauthorized; existing schema-5 projects
retain conversation-only behavior. Bound schema-3/4 simulations
retain their deterministic/mock behavior. The operator's scoped three-call v5
conversation check completed. The [v8 checkpoint](continuous-coordinator-report.md)
now has accepted bounded live conversation/proposal/scope plus detach/re-attach;
broader product/native-worker acceptance remains pending.

| Component | Owns | Cannot authorize |
| --- | --- | --- |
| Project service and bounded SQLite writer | Task/command transitions, queue, questions, decisions, approvals, budgets, context grants, session lifecycle, record indexes | A worker output or display event as a human answer |
| Workflow policy (`backend/src/workflow*`, `session_*`) | Bounded draft → critique → revision transitions, questions, final review | A new external call merely because a checkpoint is missing |
| Host adapter and runner (`backend/src/{host,operation,runner}*`) | Fenced attempt, process identity, bounded stdout/stderr, result validation, receipt and targeted cancellation | Workflow completion from a PID or unit status alone |
| systemd user manager | Project-and-operation-attempt-scoped process hosting and corroborating cgroup/job identity | Durable workflow history or model-call retry |
| Rust TUI and CLI (`frontend/src/*`) | Presentation, local draft, idempotent command submission, bounded output/artifact inspection | Direct workflow database mutation or agent execution |
| Optional Herdr view | Explicit read-only mirror of existing attempt output | Launch, permit, cancel, answer or approval |

The service uses Tokio for I/O/timers and a bounded SQLite writer queue. Slow
clients, output viewers or manager calls may receive an explicit busy outcome;
transport loss alone never means the command or execution failed. The service
and runner are independent of the TUI and Herdr display lifetime. A project
may execute one workflow at a time while other projects proceed independently.

## Project and context boundaries

The selected project's `.wayward-wrangler/` owns identity, versionable config,
release binding and state. State defaults to `state/` under that directory;
`--state-dir` explicitly overrides it. `init` creates no external registry,
service, socket or Herdr tab. On service attachment, a user-local location
index may record project key and path for routing links; it is not task memory
and cannot block a pristine local reinit. See the
[storage contract](state-contract.md#locations-and-authority).

Every task, queue, budget, lock, supervisor generation, operation, output,
context selection, question and approval belongs to one project. Identical run
IDs in two projects do not share execution. The global locator is excluded
from model packets. Cross-project reads require an explicit, bounded,
revocable operator grant naming direction, purpose, source objects and version
or hash. Imports are read-only and nontransitive; source project decisions,
approvals and execution authority remain separate. Frozen delivered packets
remain evidence after revocation. These are application-side access rules,
not a hostile same-UID filesystem sandbox. Native CLI tools need their own
context/tool-isolation gate before authenticated use.

## Execution and recovery

For a new host-backed call, persist frozen input and a unique attempt intent,
then request an exact transient systemd unit. The runner claims with verified
project, operation, attempt, executable and process identities. The supervisor
checks the owner generation, retirement fence and host claim before issuing a
permit; the runner commits dispatch before spawning the agent child. Output,
structured result and validated completion receipt are published durably.
The service adopts the receipt into workflow progress. Application records,
not systemd unit status, own historical truth.

Start replies can be lost; the supervisor reconciles the saved intent, unit,
claim, child and receipt before acting. A missing PID/unit/output is never
proof that execution did not happen. A linked replacement attempt is allowed
only with positive prelaunch proof and a durable retirement fence that prevents
late dispatch by the old attempt. Infrastructure attempts and model-call
budgets remain separate. Ambiguous execution is `uncertain` and gets no blind
retry. A surviving child or completion/cancellation race is handled by
verified process identity and retained evidence. A task-level failure does not
crash the supervisor. No promise is made that a running call survives logout,
user-manager shutdown or reboot; no orphan-stream reattachment exists.

Combined worker stdout/stderr payload is capped at 32 MiB and its index at
2 MiB. Inspection is project-scoped and paged, with stream labels and a
cursor; old unlabeled output is shown as combined legacy output. A slow or
disconnected viewer does not backpressure the worker. Artifacts are read with
containment and exact-hash checks. Herdr `observe --viewer herdr` creates a
read-only mirror only on explicit action, reusing a verified existing view;
losing the view does not alter execution. Ordinary operation never probes
Herdr, even with inherited stale Herdr variables.

## Lifecycle, state and compatibility

`/detach` closes only the TUI; background authorized work continues.
`/exit` fences new dispatch, drains a permitted call to a bounded foreground
deadline, writes a deterministic checkpoint and returns to the shell. A later
resume increments ownership generation, reconciles receipts/attempts and
prevents an older exit from stopping the resumed session. Indefinitely running
known work remains attention-required; ambiguous execution remains uncertain.
Human answers and exact-artifact approvals persist independently of model
availability and cannot be manufactured from worker text. See
[state-contract.md](state-contract.md) and [session-protocol.md](session-protocol.md).

Installed releases are side by side. Each project binds a compatible release;
changing the default for new projects does not upgrade existing projects.
Only the selected project is checked for startup, stop, upgrade and rollback.
A release referenced by a project must be retained. Rollback after new accepted
work cannot restore an old state snapshot; it requires a compatible forward
correction or release. Historical approvals, output bytes, hashes, receipts
and provenance remain immutable. Schema/version mismatch refuses mutation.

**Verification boundary:** the pinned Claude client passed credential-free
effective-capability checks and installed mock-transport verification. Exactly
three native conversation calls then completed; the operator reported physical
detach/resume. No wider terminal/native-worker acceptance is claimed. The earlier
v3 simulation observation is separate evidence. A working application, physical
v3 detach/resume, live interruption and all crash windows remain unverified.
[Current limits](current-limits.md) carries those obligations; broader worker
integration is deferred.

## Current Claude conversational increment

Existing schema-5 projects use the existing supervisor/host with a real restricted
Claude conversation adapter, closed until explicit allowance authorization.
Ordinary messages are forwarded to the selected model; conversation-only output
has no task/consultation/approval authority. The accepted production TUI streams
provisional assistant text and shows service connectivity separately from model
readiness. Older schema-3/4 releases retain their own contracts. See the
[Claude contract](claude-coordinator-contract.md) and [current evidence](archive/native-integration/claude-conversation-closeout-2026-10-14.md).

Schema 6 reuses that host/provider boundary with strict result-v2 and bounded
selected proposal/answer context. The application owns immutable revisions and
append-only exact human scope decisions through its writer; provider suggestions
are inert. Authorization never promotes execution or grants task budgets.
Local editor/delivery files are UI drafts, not orchestration authority.
