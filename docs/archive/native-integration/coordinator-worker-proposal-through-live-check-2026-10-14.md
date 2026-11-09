> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Native conversation and workers — implementation proposal

**Current disposition:** the operator-authorized focused [Claude conversation
increment](claude-conversation-closeout-2026-10-14.md) is implemented and credential-free
verified through the installed production TUI. Its [live authorization request](claude-v5-live-authorization-and-outcome-2026-10-14.md)
now records the completed operator-only three-call check, exhausted allowance,
physical detach/resume observation and read-only outcome verification.
Worker integration, consultation and the weather trial below
are deferred; their eleven-launch scope is not this three-call increment.
Earlier checkpoint-4 mock evidence is not natural-conversation acceptance.
No broader acceptance or further native spending is inferred. The reported
Markdown omission has a focused source correction, not an operator cutover.

**Capability amendment approved at `167b983`; checkpoints 1A, bounded 1B, and 2–4 authorized.**
Checkpoints 1–4 were approved at `ba27dc2`, then stopped at the native boundary
checkpoint `45c36f77f93d7da010674532d78c0b22007b06bc`. The operator now replaces
empty advertised tools with an enforced capability policy and approved this bounded implementation. Checkpoint 5/authenticated spending is
still separate. The [implementation report](checkpoints-1-4-report-2026-10-13.md)
records passing checkpoints; this proposal is the approved target, not proof.
Original [proposal](proposal-before-capability-amendment-45c36f7.md)
and [boundary findings](boundary-checkpoint-1-45c36f7.md)
remain unchanged; the [current boundary assessment](boundary-assessment-through-live-check-2026-10-14.md)
owns source evidence, residual-tool classifications and the proposed proof matrix.

## Recommendation and review decisions

Deliver conversation and a **bounded static-file implementation workflow through
the accepted installed TUI**, using the existing Rust service and operation host.
Keep ordinary `cd PROJECT; waywr init .; waywr`, direct terminal attachment,
and optional read-only Herdr inspection. Initialization never spends model calls.

Recommended implementation direction under the amendment:

1. **Packet-only native roles for the first trial.** Agents receive selected
   text and return structured messages/files. Waywr validates and materializes
   the files. No model-controlled filesystem access, shell/process execution,
   discovery or external messaging may operate. A classified utility may operate;
   an advertised file tool is tolerable only if it is denied before any read/write.
   This is the first static-file profile, not a permanent limit on coding roles.
2. **No automatic provider fallback.** Support an explicit coordinator change
   between resolved calls, with a visible handoff and unchanged worker profiles.
   Do not substitute a provider for a failed/uncertain invocation. A broad picker
   and preauthorized fallback chains are deferred.
3. **One native weather-page trial, separately authorized after offline gates.**
   Recommended exact settings and maximum **11 CLI invocations** are below.
   Approval of this document is not approval to spend those calls.

**Recommended interface decision:** prove Codex's existing `app-server --stdio`
interface with explicit empty execution environments, then use it for Codex roles
if all gates pass. Keep Claude's restricted print interface. Both remain installed
CLI clients using the same intended authentication, providers, models and billing
routes. This is a transport change from `codex exec`, not adoption of another agent
runtime by Waywr. The empty-environments field is experimental in the pinned
protocol: explicit opt-in, version/hash pinning and negative tests are required.

The tradeoff is a small per-operation JSON-RPC adapter and stricter startup checks
in exchange for a source-visible denial **before** file-tool access. App-server
startup does not expose exec's `--ignore-user-config`/`--ignore-rules` switches;
configuration/auth separation remains a real admission gate, not an assumed
property. Do not transplant those flags, use undocumented `ToolPolicy`, or claim
this recommendation is already proven. Section 3 and the boundary report define
the bounded fallback if this interface fails. No provider/model/auth substitution
or shared-configuration change is approved by this recommendation.

## Evidence and gap map

Labels: **S** source inspected; **H** previously captured installed help/version;
**D** primary documentation; **R** previously recorded runtime evidence;
**P** proposed; **U** unverified. No application tests, CLI runtime experiments,
authentication checks or native calls were run in this amendment. H evidence is
from earlier checkpoints; captures/hashes remain in ignored
`.local/native-coordinator-planning/2026-10-13/`.

| Area | Existing / reusable evidence | Missing or unverified work |
| --- | --- | --- |
| Conversation | **S** [`Session::submit`](../../../backend/src/session_store.rs) creates a queued task for every `submit_message`; `tick_coordinator` in [`session_service`](../../../backend/src/session_service.rs) hardcodes fake provider and one proposal per task/version | **Missing:** conversation independent of tasks, clarification, consultation and real coordinator output contracts. Accepted visual conversation is not model conversation |
| Task authority | **S** same store: command deduplication, stale task/proposal checks, explicit admission, durable events; `steer_queued` refuses once frozen/started | Reuse authority; add boundary-applied steering, never edit dispatched packets |
| Native adapters | **S** [`adapters.rs`](../../../backend/src/adapters.rs): version checks, argv, JSON schemas, event/finish validation. [`runner.rs`](../../../backend/src/runner.rs): prompt file on stdin then EOF, Tokio pipes, evidence | Reusable helpers, **not runtime-verified native host support**. Current service plan requires fake roles; CLI `start` explicitly refuses native admission in [`rust_cli.rs`](../../../frontend/src/rust_cli.rs) |
| Supervision | **S/R** [`host_attempt`](../../../backend/src/host_attempt.rs), [`systemd_host`](../../../backend/src/systemd_host.rs), runner and service; [accepted simulated report](../tui-first/tui-first-checkpoint-4-report.md) | Reuse claim/permit/dispatch/receipt fencing, positive prelaunch recovery, cancellation, output bounds and generation-aware lifecycle. Native failure parsing remains unverified |
| Context | **S** [`local_context`](../../../backend/src/local_context.rs), [`context_policy`](../../../backend/src/context_policy.rs), [`context_access`](../../../backend/src/context_access.rs): explicit selection, hashes, accepted decisions/findings, bounded grants, revocation and dependency checks | Add conversation selection and handoff; no global discovery. Existing 48,000-byte snapshot / 16 KiB foreign-import limits are ceilings, not relevance discovery |
| Questions and review | **S** [`workflow_policy`](../../../backend/src/workflow_policy.rs), [`workflow_store`](../../../backend/src/workflow_store.rs), [`findings`](../../../backend/src/findings.rs): durable answers, separate reassessments, unresolved coverage and exact-artifact approval | Reuse for task questions; conversation questions must exist without a run/artifact. Current artifact is one Markdown file; executable file bundles need a versioned manifest binding |
| TUI / IPC | **S/R** [`prototype module`](../../../frontend/src/prototype/mod.rs), [`installed bridge`](../../../frontend/src/main.rs), [TUI contract](../../tui-contract.md), [minor-1 protocol](../../session-protocol.md) | Keep presentation and visible review/answer actions; replace submit-equals-task semantics, add bounded conversational events and accurate native/provider labels |
| Historical native trial | **R** [milestone 5](../milestone-5/milestone-5-report.md): Claude/Codex CLI calls, a genuine question, five calls, explicit recoveries and final provisional specification approval | Python/LangGraph/Herdr trial across builds; not current Rust-host isolation or working-app acceptance. No reuse of its authorization or modification of its evidence |

Reuse regression cases in [`workflow_faults.rs`](../../../backend/tests/workflow_faults.rs)
(record-only answer, output-invalid, incomplete revision, result replay, frozen
budget), [`systemd_host_operation.rs`](../../../backend/tests/systemd_host_operation.rs)
(claim/permit/receipt and both-stream bounds), and the existing installed PTY
checks. Their presence/earlier pass records are not new test executions.

## 1. Conversation, actions and scheduling

**P: narrow coordinator contract `coordinator-turn-v1`.** One result contains
`assistant_text`, evidence references, and at most one proposed application action:
`ask_question`, `propose_task`, `request_consultation`, or `propose_steering`.
A plain response has no action. No model-provided argv, SQL, tool execution,
approval, cancellation or raw IPC. The broker checks project, conversation
revision, coordinator generation, target task/version, context digest, permission
and budget before applying it. A stale reply remains inspectable but cannot
authorize work. New IDs are application-owned; receipt/result replay applies once.

- Ordinary text defaults to coordinator discussion; an explicit selected-task
  destination supplies context, not execution authority. “What do you think?”
  creates no task. Requesting implementation can produce **Review scope**, whose
  explicit authorization freezes inputs, selected context, files, profiles and limits.
- Clarification is a durable question with rationale, optional choices, free text
  and an answer reference. Multiple questions per conversation/task are allowed;
  initial bound: 8 outstanding/project, one dependent call per resolved turn.
  Saving an answer costs no model invocation; interpreting it can consume a turn.
  Scope authorization, final approval and operational notices need no model.
- Steering is recorded while work proceeds. Discussion never mutates frozen work.
  A proposed in-scope instruction is explicitly accepted and attached to the next
  **undispatched** packet with its provenance. Scope/profile/budget/output changes
  require a revised authorization; work affected by unresolved steering holds at
  the next boundary. Already dispatched calls finish under their original input.
  If no stage remains, offer a new task; do not reopen an approved result.
- Consultation is a named, one-call, read-only opinion with a frozen brief/context
  subset. It requires the operator's request or the session's explicit consultation
  allowance. No recursive delegation. Return its receipt-linked findings to the
  coordinator; synthesis is a separately counted coordinator turn. Advice is not
  a decision or an independently authorized task.
- One executing workflow **per project**, plus at most one coordinator/consultant
  call at a time. A worker and discussion turn may overlap. Keep one service owner,
  not a second scheduler. Queue at most 16 unsent conversational turns; excess is
  an explicit preacceptance busy response. Store accepted messages durably first.
  Existing Tokio/SQLite bounds apply; never wait for a model inside a transaction.

**P: state and IPC additions.** New application schema 4 adds messages, turns,
conversation questions, coordinator bindings, budget reservations and steering
records to the same writer. Each has schema version, project, sequence, immutable
content/ref hashes and provenance; provider transcripts remain execution evidence.
Protocol major 1/minor 2 advertises `conversation_v1` and `file_candidates_v1`.
Add idempotent command actions for message submission, answer, steering acceptance,
consultation authorization and coordinator selection, plus bounded conversation
reads. Reuse command IDs, revision fences, snapshots/event cursors, questions and
approval views; update schema consistency tests. Minor-1 clients cannot submit
new actions or silently use the old task semantics on a conversation session.

Stream assistant-text deltas as **uncommitted output**, separate from durable
final responses/actions. Never show internal reasoning as conversation. Reuse
operation output pages for raw diagnostics. Coalesce UI updates (at most 10 Hz,
64 KiB pending text/client); disconnect or gaps resync from bounded pages. Preserve
composer/focus/scroll; no panel opens or authorizations from a background event.

## 2. Provider contracts and budgets

**H/S (prior checkpoint):** local Codex **0.162.1** and Claude Code **2.1.296** match adapter pins.
Resolved binaries are their user-local standalone releases, not a Waywr-bundled
runtime. Capture exact executable hash/version at preflight and freeze per call;
an update requires revalidation, not an automatic upgrade. Use CLI authentication
already configured by the operator, never copy tokens into packets or receipts.
Auth validity, model entitlement and managed remote policy remain **U**.

Store versioned coordinator/consultant profiles and session allowances in project
configuration beside the existing worker profile. On first native launch, the
existing settings/review surfaces show those explicit profiles, context boundary
and allowance for confirmation; no live model catalog is needed. The resulting
authorization persists across reconnects, with remaining counters, not a fresh
budget per TUI launch. Routine chat turns within it need no repeated confirmation.
Worker scope authorization is separate. `init` remains local-only and call-free.

| Interface | Codex | Claude |
| --- | --- | --- |
| Invocation/output (**H/S**) | `exec`, stdin `-`, `--json`, `--output-schema`, `--output-last-message`; JSONL final and usage parsing already exists | `--print`, text stdin, `--output-format stream-json --verbose --include-partial-messages --json-schema`; init, partial and final-result parsing exists |
| Controls (**H/S**) | Explicit model; `model_reasoning_effort`; `--ignore-user-config`, `--ignore-rules` (not yet in adapter), ephemeral mode and disabled features | Explicit model and supported `--effort`; tools/settings/MCP restrictions; safe mode; `--permission-prompts none` and `--restricted` need integration verification |
| Continuity (**H/P**) | `exec resume SESSION_ID` exists; **do not use** `--last` | `--resume ID` exists; `--no-session-persistence` deliberately disables it |
| Cancellation (**S/U**) | Reuse verified child/process-group termination and host receipts, not terminal keystrokes; native interruption/result classification needs acceptance | Same application-owned cancellation; no native background/remote session escaping the operation host |

The table describes existing exec/print helpers, not proof of the amended profile.
For Codex, the proposed replacement is a fresh stdio app-server process under the
same operation host: initialize → ephemeral `thread/start` → one `turn/start`
with a frozen packet/output schema → bounded events → validated terminal result.
Set `environments: []` explicitly; never omit it or let model output choose a
thread, environment or RPC. Disable provider model fallback; send no dynamic
tools or capability roots. No daemon, websocket listener or native thread resume.
Waywr owns all RPC requests and termination; native session continuation is deferred.
The pinned protocol/structured result and denial fixtures must pass before this
adapter can be admitted. The current runner's prompt-file stdin/EOF cannot serve
this bidirectional protocol unchanged; add a bounded transport implementation
inside the same host/receipt lifecycle, not a second controller or daemon. Claude
remains a fresh restricted print process per turn. This avoids unrelated history and makes
replacement reproducible. [Codex noninteractive docs](https://learn.chatgpt.com/docs/non-interactive-mode)
document structured JSONL, saved-login use and session continuation; these are
interfaces, not a guarantee of network/model availability.
[Claude CLI reference](https://code.claude.com/docs/en/cli-reference) documents
the print/restricted surfaces; installed help is the version-specific baseline.

**Recommended first-trial settings (P):**

| Role / budget | CLI model and effort | Maximum launches |
| --- | --- | ---: |
| Primary conversation, clarification, consultation synthesis, proposal | Claude `claude-sonnet-5-5`, `medium` | 6 |
| Requested independent consultation | Codex `gpt-6.1-sol`, `high` | 1 |
| Implementation and optional repair | Claude `claude-sonnet-5-5`, `medium` | 2 |
| Independent review and optional re-review | Codex `gpt-6.1-sol`, `high` | 2 |

Thus 6 conversation + 1 consultation + 4 task calls = **11 total**, two reviews
maximum. Sonnet medium bounds routine author cost; Codex high preserves the
operator's previous critic preference. The [Claude model documentation](https://code.claude.com/docs/en/model-config)
supports pinning the full model rather than moving `sonnet`; installed version
supports Sonnet 5.5. [OpenAI's model reference](https://developers.openai.com/api/docs/models/gpt-6.1-sol)
documents high effort; it does not establish this account's CLI access. Effort
levels are provider-specific, not equivalent token/cost limits.

Freeze separate session and task budget ledgers. Reserve immediately before the
fenced native spawn; crashes/uncertainty retain the debit. Positive, fenced
nonlaunch proof can retire an infrastructure attempt without a new model debit.
The host's current `commit_spawn` reserves one logical operation; the legacy
runner's sibling-stage budget scan is skipped for schema-2 host operations.
Do not mistake either for an aggregate conversation/task accounting: enforce its
remaining allowance in the service permit and consume the matching reservation
once at spawn, with crash reconciliation and no double debit.
Persist the charged/held budget row **before** publishing its permit reference;
both reserved and unresolved launches count against availability. Host evidence
settles that row, never creates an independent spend counter. No transaction or
dispatch lock spans external I/O.
Answering, inspecting, approving, reconnecting and deterministic checkpointing cost
zero calls. Exhaustion keeps the TUI/service usable and holds dependent work; no
automatic increase, retry, model substitution or effort escalation. An operator
may explicitly change the coordinator for future resolved turns with the same
remaining conversation budget. Active worker profiles stay frozen. An ambiguous
turn is never replayed under another provider; dependent actions stay blocked.

CLI launches are **not HTTP request counts or guaranteed dollar caps**. Current
runner evidence already warns of provider transport retries. Codex exposes
`request_max_retries`/`stream_max_retries` in its
[configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference);
test zero-retry settings against the pinned client without live credentials before
using them. Claude transport retries are not established here: document the
effective behavior; never claim exactly one provider request per launch. Disable
CLI fallback, including Claude's separate flag-triggered switching, using verified
per-call settings (`fallbackModel`, `switchModelsOnFlag` and model restrictions);
do not edit shared settings. If suppression is not established, native admission
stays blocked. Report requested/resolved model only when evidenced, usage only
when emitted, and no invented cost. Codex's current stream does not independently
attest the resolved model. No automatic post-failure call in this first trial.

## 3. Context and isolation gate

**P: bounded context:** each turn selects the current goal, accepted decisions and
rationale, unresolved questions/findings, task state, exact artifact/delta refs,
and at most six recent exchanges relevant to that target. Cap total serialized
packet at 96 KiB, including the existing local-context/import sublimits; use a
32 KiB implementation-source cap for this trial. Evidence not included is linked,
not silently summarized as fact. If required evidence cannot fit, stop for a
narrower selection rather than omit a decision. No transcript dumping, vector
index, unrelated-project scan or automatic transitive import.

Checkpoint `conversation-handoff-v1` deterministically records goals, accepted
decisions/rationale, completed operations, open questions, next actions, counters
and supporting IDs/hashes at a sequence watermark. Optional model summaries are
non-authoritative. Exit/resume/provider replacement rebuild from these records
and reconcile later receipts first; do not require another model to save memory.
Preserve pending answers, active workers and frozen authorization. Revoked grants
block future reads/dispatch of dependent material, not rewrite delivered evidence.

### Capability policy: `static-file-packet-v1` (proposed)

Four separate surfaces must be evidenced; tool names alone cannot settle safety:

| Surface | Allowed behavior and owner | Forbidden / required evidence |
| --- | --- | --- |
| Advertised tools | Inventory every effective tool and schema for each pinned model/profile. UTC clock and local provisional messages may be classified as permitted; a non-operating patch tool may remain listed | No blanket residual-tool waiver. Unknown tools or changed semantics close native admission pending classification |
| Effective model operations | Selected packet → structured text/files; clock reads; bounded local provisional messages in the **same operation** | No model-selected files, shell/processes, network/connector access, other sessions, permission escalation or agents. File tools must refuse before any read/write, including patch verification. Test forced calls even when absent from the advertised inventory |
| Provider-client startup/auth | Trusted installed client may load necessary runtime libraries, its existing credentials/account routing and required policy; use an inventoried per-call scratch/config profile | No inherited hooks, skills, plugins, MCP, history, project docs or unrelated project metadata in outbound context; no automatic helper execution outside the approved profile. Managed policy stays enforced; incompatible policy refuses. Runtime/auth access is not model access |
| Waywr execution/decision authority | Service owns packets, launch permits, receipts, durable budgets/questions/decisions and candidate publication | No model IPC, command lines, state/receipt/approval mutation or direct candidate writes. Only validated final result/action reaches the broker, checked against project, generation, revision, context and budget |

The residual-tool matrix and exact evidence levels live in the
[boundary report](boundary-assessment-through-live-check-2026-10-14.md#residual-tool-assessment).
In particular, async native question/message events are **provisional output**,
not a Waywr question, saved answer, task, approval or instruction to another agent.
Only a valid terminal structured `ask_question` creates a durable question.
Do not parse client `accepted:true` or native `FinalAnswer` labels as authority.
Unexpected blocking provider input/permission requests are refused or interrupt
the call with retained evidence, never automatically answered or retried.

Application-side context selection enforces project scope; supported client
restrictions must prevent model tools bypassing it. A fresh cwd, read-only mode,
prompt rule, missing tool event or unchanged sentinel hash alone proves neither
read isolation nor prevention. A post-event detector is an incident tripwire.
This profile trusts the pinned client for runtime/auth access; **it is not
whole-process confinement** against a compromised client or another same-UID
process. No model credential access is authorized. The client must not discover
or transmit unrelated project identity/content as part of automatic context.

### Recommended proof candidate and fallback

Codex `thread/start.environments: []` has a documented-in-source no-environment
contract and upstream tests. The patch handler resolves an environment before
accessing its filesystem; with none it returns an error. This can allow harmless
clock/local-message utilities without accepting filesystem effects. It is a
**source-supported hypothesis about the installed binary**, not a passing gate.
Use the existing feature restrictions as defense in depth, including disabling
shell tools (disabling unified-exec alone leaves a one-shot execution path),
web/MCP/plugins/code mode, delegation, permission requests and optional plan,
sleep/context-window tools. No model RPC forwarding or automatic environment
attachment. Preserve both stdout/stderr capture and host deadlines/output bounds.

Before a native adapter is admitted:

- Prove startup customization exclusion separately. Inspect supported startup
  overrides and effective config before any turn; thread-level restrictions can
  arrive too late to stop startup hooks/MCP/account helpers. Preserve real auth
  **in place** with supported access; do not copy tokens or silently switch to API
  keys, `--bare`, a different provider, remote daemon or host user. A private dummy
  home proves fixture isolation, not preservation of the operator's real login.
- Claude retains safe/restricted print, empty model tools, strict empty MCP,
  disabled hooks, no history/prompt escalation. Its `StructuredOutput` mechanism
  is allowed only as inert schema-result delivery; no executable content handler.
  Captured help is not proof of effective exclusion or managed-policy compliance.
- Keep provider cwd/scratch separate from authoritative operation records. Current
  `runner.rs` uses the operation directory; a separate scratch path limits automatic
  discovery but is not an OS sandbox. Waywr writes the packet and retains output
  through pipes; the client never owns receipt/permit/approval/budget paths.
- Prove fallback suppression and transport retry settings or expose a concrete
  native blocker. Tool-driven inference can involve multiple requests in **one**
  launch; 11 launches is not an HTTP/token/dollar ceiling. Utility events use the
  existing bounded output/deadline policy, never a separate budget or continuation
  authority. Freeze binary, effective profile/config/catalog and schema identities;
  relevant changes require revalidation, not silent acceptance.

If empty environments or startup exclusions fail, keep that provider closed.
First report the exact failed requirement; do not automatically try another
interface. The smallest alternative is an explicitly approved **disposable confined
workspace/VM for the same client**, with only selected packet/scratch and the
minimum approved credential access, no host mounts/bus/sockets and verified egress.
The earlier user-namespace/uid-map denial means bubblewrap or a container label
cannot be assumed to enforce that boundary here. Assess available isolation first;
no host policy change, new sandbox platform or toolchain upgrade is authorized.
A VM needs a separate decision about credential placement/maintenance and is less
convenient than the supported no-environment interface. Waiting for an empty-tool
CLI feature is not the only path. Neither alternative may weaken read isolation.

**Bounded investigation:** one exact Codex app-server profile and one Claude print
profile; execute only the credential-free matrix in the boundary report once
implementation resumes. Correct fixture/protocol mistakes with retained evidence;
do not expand into reverse engineering other runtimes or trying live providers.
An unresolved provider proof blocks native admission and that adapter's native
completion claim, not the provider-independent checkpoints below.

## 4. First native trial: Sample Weather

**P:** one new named project `temp/native-weather-01/`, initialized only during
separately approved implementation/trial preparation. No reuse/rebinding of v3.
Open the installed `waywr` there. Ask for a small local weather page with fixed
sample data; discuss requirements, request one Codex opinion, and decide the
initially unresolved temperature-unit behavior (single unit or toggle/persistence).
The actual clarification/recommendation must come from the coordinator, not an
injected fake question or prefilled answer. Record the operator's scope decision.

Authorize one static-files task. Use existing policy primitives with a versioned
`static-files-v1` profile: implement → independent review → optional repair →
independent re-review → final human gate, at most four calls. No repair after the
last review under this budget. Unresolved issues remain explicit; failures stop
without additional calls. This is one concrete policy extension, not a workflow DSL.
An unexpected worker question is also a durable gate; a follow-up author call
consumes an available task slot. If clarification leaves too few slots for the
required implementation/review, stop with the available result instead of adding
calls or counting the answer itself as a model invocation.

Deliver `index.html`, `styles.css`, `app.js`, `README.md` with embedded fixed data;
no dependencies/build/install scripts, remote assets, backend, accounts or keys.
Waywr accepts only those relative paths, regular UTF-8 files and bounded sizes
(32 KiB total); reject symlinks, traversal and duplicate paths. HTML/CSS/JavaScript
are allowed **file content**, never authority to run commands or install scripts.
Publish an immutable candidate directory and canonical manifest binding every
file/hash plus scope/decision/context revision. Use that manifest as the exact
approval identity. Reuse approval authority; add file selection/read receipts.
Modified bytes or request identity invalidate read progress/approval. No shell or
generated JavaScript executes inside the supervisor. Application-owned static
checks may inspect files; opening the page is an explicit operator action.

**Observable acceptance:**

1. A genuine discussion/clarification occurs without creating a task; one actual
   independent consultation is returned with provenance. The operator's answer and
   visible scope authorization precede worker implementation.
2. Output is a working page, not only a specification: at least three labeled sample
   cities; changing city updates temperature, condition, humidity and wind from fixed
   data; units obey the recorded choice; no external requests. Keyboard operation,
   readable narrow layout (360 px), reload and any chosen persistence work.
3. Independent review refers to the **exact file manifest** and acceptance criteria.
   App records retain findings and any repair/re-review. Deterministic checks and
   the operator's browser walkthrough are separate evidence; no native review is
   claimed to be a browser test. If material requirements fail, do not call it accepted.
4. While work executes, typing/navigation/status/answer/cancel remain responsive.
   Detach/reopen and restart at a pending question preserve drafts, answer once,
   conversation, receipts and budget; no repeated call. No Herdr needed. One explicit
   coordinator handoff can be verified offline without spending a switch trial.
5. The operator opens the exact candidate, follows `README.md`, observes the browser
   result, and approves or rejects in the TUI. Approval of an earlier scope/spec is
   not working-app verification. Retain candidate/evidence regardless of outcome.

## 5. Bounded delivery checkpoints and rollback

This amendment is approved. Use two explicit checkpoint-1 outputs: **1A**, frozen
capability/action contracts plus mock fixture specifications; **1B**, effective
provider proof. Sequence: `1A → 2 → 3 (mock) → 4 (mock)`; `1B → native adapter
admission`. Checkpoint 5 requires **both** branches, the final installed candidate,
and separate spending authorization. A failed 1B never becomes a pass merely
because mock integration succeeds. Finish the bounded 1B attempt first, report its
outcome, then proceed with independent work if no shared integrity blocker exists.

| Step | Scope/dependency | Required gate and evidence |
| --- | --- | --- |
| 1A/1B. Contracts / effective boundary | 1A: versioned capability/action/result profile, mock schemas and admission-closed default. 1B: exact binary/config manifests and the report's credential-free matrix | Separate source, mock request, actual handler and authenticated evidence. Prove pre-access tool denial, startup exclusion, structured output and fallback/retry behavior before native admission. Auth/entitlement/managed-account checks remain explicit checkpoint-5 prerequisites |
| 2. Conversation authority | Needs 1A, **not 1B**: schema 4/minor 2, conversation/questions/bindings, action broker, budgets | Discussion creates zero tasks; replay/restart apply once; stale generation/proposal refuses; overlapping turns do not overspend; strict project isolation; inspect <1 s / SQLite busy refusal <500 ms. No native dispatcher bypass |
| 3. Integrated coordinator | Needs 2 and mock role contract: shared interface, accepted TUI, discussion/clarification/consultation/synthesis/proposals, steering and explicit replacement | Mock streams through installed startup; no manual service/runner/Herdr setup; input/answers/cancel responsive during work; drafts/questions/receipts/budgets survive detach/exit/restart/resume. Provider-specific auth/config/transport completion depends on 1B and stays visibly unavailable until proven |
| 4. File workflow and review | Needs 2–3 mock path, **not 1B**: static-file policy, manifest, source/delta handoffs and exact approval | Installed interaction launch → discuss → clarify → consult → review scope → authorize → mock execution → inspect files/review → explicit decision. Tamper/traversal/duplicates/coverage/limits/cancel/adoption/replay checks; four task calls/two reviews; prior schema-3 behavior preserved |
| 5. One native acceptance | Needs 1B for **both** providers, 2–4 installed acceptance, exact configuration review and separate authorization | One `native-weather-01`, at most 6+1+4 launches, no substitution/retry/escalation; stop on failed/uncertain call or policy/model mismatch. User answers/approves and verifies working page. No mock evidence counts as native acceptance |

No installed schema-4 candidate exists yet. Deliver one stable isolated launcher
for the paired release, owning fixture-specific environment internally. From its
selected project: initialize once with `waywr init .`, then `waywr`; service/runner
startup and reconnect are automatic. No separate UI, manually linked components,
`systemctl` ceremony or user-supplied environment. Checkpoints 2–4 label responses
as mock, preserve the real admission refusal path, and cannot start a provider
binary merely because an operator authorizes a fake task. Each passing checkpoint
is committed with refreshed contracts/handoff/index; bounded failed 1B evidence is
also checkpointed without labeling native behavior verified. Stop on a shared
integrity failure or a substantive decision, not for routine stage permission.

Before each step record code checkpoint and state boundary. New native work uses
a fresh schema-4 project and side-by-side release; existing schema-3 and legacy
projects remain bound and untouched. No automatic migration. New release must
inspect old records without mutating them, and route execution to their binding.
Version new coordinator, bundle, authorization and budget contracts; do not rewrite
historical prompt/output bytes. A rollback before any new admission can discard
only disposable fixtures. **After admission, code rollback requires compatible
state readers/writers and reconciled hosts**: keep the compatible release for
inspection/drain or forward-repair with appended records. Never restore a snapshot
over later messages/answers/receipts, point an old service at schema 4, or refund
uncertain calls. No operator-service cutover is implied.

## Separate native-trial authorization checklist

Before asking to spend calls, present the final installed release, target/key,
effective isolation proof/limits, auth mechanism (not secrets), CLI versions/hashes,
exact four role profiles, 6+1+4 launch ceilings/two reviews, selected inputs,
allowed output paths, provider transport-retry limitations, and stop rules.
The operator must authorize that trial; then routine transitions within its scope
need no repeated confirmation. Real clarification, scope authorization and final
approval remain human gates. No “successful” fake isolation test supplies permission.

Remaining uncertainties: current account entitlement/auth refresh under restrictions;
effective Codex tool denial and startup/auth separation; managed/remote customization; provider internal
retries and model attestation; native cancellation/output behavior under the Rust
host; and physical-terminal/browser acceptance. Offline proof resolves what it can;
the bounded authorized trial must resolve the rest without silently widening scope.
No provider API replacement, general agent sandbox/platform, broad model picker,
automatic context discovery, parallel worker scheduler, Python dependency or
accepted-TUI redesign is part of this proposal.
