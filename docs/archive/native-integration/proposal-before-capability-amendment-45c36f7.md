> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Native conversation and workers — implementation proposal

**Checkpoints 1–4 approved for implementation on 2026-10-13; checkpoint 5 and
authenticated model calls remain separately gated.** Checkpoint 1 found a native
tool-boundary blocker; see the [current boundary report](native-coordinator-boundary-checkpoint-1.md).
The following proposal is the approved scope, not a claim that the steps passed.
Originally planned on branch `plan/native-coordinator-integration`; base
`2e5cf1212fe9a319b118c641c5028c851f66e86d`. PR #5 was squash-merged;
`main` and `origin/main` match that commit. Its tree matches checkpoint
`31273fd1085d3cfce0501d13b4a82ebc2ac7b829`; the old topic commits are
evidence, not ancestors of the squash. No retained project was changed.

## Recommendation and review decisions

Deliver conversation and a **bounded static-file implementation workflow through
the accepted installed TUI**, using the existing Rust service and operation host.
Keep ordinary `cd PROJECT; waywr init .; waywr`, direct terminal attachment,
and optional read-only Herdr inspection. Initialization never spends model calls.

Recommended decisions for implementation approval:

1. **Packet-only native roles for the first trial.** Agents receive selected
   text and return structured messages/files. Waywr validates and materializes
   the files; agents get no shell, repository-editing or discovery tools. This
   produces a runnable result while keeping the first isolation boundary small.
   General repository-editing agents need a separately proven execution sandbox.
2. **No automatic provider fallback.** Support an explicit coordinator change
   between resolved calls, with a visible handoff and unchanged worker profiles.
   Do not substitute a provider for a failed/uncertain invocation. A broad picker
   and preauthorized fallback chains are deferred.
3. **One native weather-page trial, separately authorized after offline gates.**
   Recommended exact settings and maximum **11 CLI invocations** are below.
   Approval of this document is not approval to spend those calls.

The substantive tradeoff in (1): trust the supported CLI to enforce its disabled
tool/customization contract while it accesses its own authentication. This is
not an OS sandbox against a malicious provider binary. If the operator requires
whole-process filesystem confinement now, stop and assess that narrower hosting
change before admitting native execution. Do not describe prompt scoping or
read-only mode as that guarantee.

## Evidence and gap map

Labels: **S** source inspected now; **H** installed help/version executed now;
**D** primary documentation; **R** previously recorded runtime evidence;
**P** proposed; **U** unverified. No application tests, authentication checks or
native calls were run in this planning pass. Help captures and hashes are in
ignored `.local/native-coordinator-planning/2026-10-13/`.

| Area | Existing / reusable evidence | Missing or unverified work |
| --- | --- | --- |
| Conversation | **S** [`Session::submit`](../backend/src/session_store.rs) creates a queued task for every `submit_message`; `tick_coordinator` in [`session_service`](../backend/src/session_service.rs) hardcodes fake provider and one proposal per task/version | **Missing:** conversation independent of tasks, clarification, consultation and real coordinator output contracts. Accepted visual conversation is not model conversation |
| Task authority | **S** same store: command deduplication, stale task/proposal checks, explicit admission, durable events; `steer_queued` refuses once frozen/started | Reuse authority; add boundary-applied steering, never edit dispatched packets |
| Native adapters | **S** [`adapters.rs`](../backend/src/adapters.rs): version checks, argv, JSON schemas, event/finish validation. [`runner.rs`](../backend/src/runner.rs): prompt file on stdin then EOF, Tokio pipes, evidence | Reusable helpers, **not runtime-verified native host support**. Current service plan requires fake roles; CLI `start` explicitly refuses native admission in [`rust_cli.rs`](../frontend/src/rust_cli.rs) |
| Supervision | **S/R** [`host_attempt`](../backend/src/host_attempt.rs), [`systemd_host`](../backend/src/systemd_host.rs), runner and service; [accepted simulated report](archive/tui-first/tui-first-checkpoint-4-report.md) | Reuse claim/permit/dispatch/receipt fencing, positive prelaunch recovery, cancellation, output bounds and generation-aware lifecycle. Native failure parsing remains unverified |
| Context | **S** [`local_context`](../backend/src/local_context.rs), [`context_policy`](../backend/src/context_policy.rs), [`context_access`](../backend/src/context_access.rs): explicit selection, hashes, accepted decisions/findings, bounded grants, revocation and dependency checks | Add conversation selection and handoff; no global discovery. Existing 48,000-byte snapshot / 16 KiB foreign-import limits are ceilings, not relevance discovery |
| Questions and review | **S** [`workflow_policy`](../backend/src/workflow_policy.rs), [`workflow_store`](../backend/src/workflow_store.rs), [`findings`](../backend/src/findings.rs): durable answers, separate reassessments, unresolved coverage and exact-artifact approval | Reuse for task questions; conversation questions must exist without a run/artifact. Current artifact is one Markdown file; executable file bundles need a versioned manifest binding |
| TUI / IPC | **S/R** [`prototype module`](../frontend/src/prototype/mod.rs), [`installed bridge`](../frontend/src/main.rs), [TUI contract](tui-contract.md), [minor-1 protocol](session-protocol.md) | Keep presentation and visible review/answer actions; replace submit-equals-task semantics, add bounded conversational events and accurate native/provider labels |
| Historical native trial | **R** [milestone 5](archive/milestone-5/milestone-5-report.md): Claude/Codex CLI calls, a genuine question, five calls, explicit recoveries and final provisional specification approval | Python/LangGraph/Herdr trial across builds; not current Rust-host isolation or working-app acceptance. No reuse of its authorization or modification of its evidence |

Reuse regression cases in [`workflow_faults.rs`](../backend/tests/workflow_faults.rs)
(record-only answer, output-invalid, incomplete revision, result replay, frozen
budget), [`systemd_host_operation.rs`](../backend/tests/systemd_host_operation.rs)
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

**H/S:** local Codex **0.162.1** and Claude Code **2.1.296** match adapter pins.
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

Initial design uses a fresh ephemeral CLI process per turn with the Waywr packet;
native session continuation is deferred. This avoids unrelated history and makes
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

| Role/component | Required reads | Allowed writes/execution |
| --- | --- | --- |
| Coordinator / consultant | Frozen message/context packet; consultant gets only its question and selected evidence | Structured response only; no filesystem/search/shell/network tools or direct task mutation |
| Implementer / reviewer | Scope, decisions, selected source bundle, findings and changed-file deltas in packet | Structured UTF-8 file bundle or review only; reviewer cannot publish or approve |
| Waywr service/runner | Selected project records, pinned binary and verified receipt | Existing fenced host operations; validated append-only candidate files; no model-provided command line |
| Provider client | Runtime libraries, its authentication, isolated call scratch and input/schema | Network to its configured provider, bounded output; client-side metadata/cache behavior must be inventoried |

**S:** current adapter drops most environment variables, but retains HOME,
provider config locations, PATH and the user bus. Both user configurations contain
hooks; Codex also contains plugin/MCP configuration. This planning inspection
recorded only categories, no credentials or unrelated project names. Two known
local managed-config paths were absent; this does not rule out other/remote policy.

**P: minimum enforcement before native admission:**

- Per-call allowlisted environment and verified CLI absolute path; no shell startup,
  inherited agent sockets, arbitrary helpers, MCP, plugins, skills or user/project
  auto-instructions. Preserve only required auth/CA/runtime inputs. Restrict bus
  access to what verified authentication actually needs. No credentials in evidence.
- Claude: safe/restricted mode, no model tools, strict empty MCP, disabled hooks,
  no session history, no permission prompts. Managed policy cannot be bypassed;
  refuse incompatible hooks/tools/auth helpers. `--bare` is not the default because
  its authentication behavior differs. Safe mode alone still includes some cwd/git
  metadata; test an empty call workspace against parent-discovery sentinels.
- Codex: ignored user config/rules, ephemeral history, no project documents/skills,
  explicit disabled tools/features, no approval escalation. The present adapter
  admits a residual `apply_patch` caveat; resolve its advertised capabilities before
  acceptance. `--sandbox read-only` is **not** proof of no unrelated reads.
- Verify effective outbound tool inventory and automatic context with pinned-client
  source/config inspection and a credential-free local mock endpoint where supported.
  Use hostile dummy hooks/docs/MCP/history and project-B sentinels; model-visible
  input must contain none. Fake packet tests alone do not prove real CLI behavior.
  Account/managed-policy effects still need a documented preflight/runtime check.
- Tool-event detection is a secondary tripwire, not prevention after a read occurred.
  If a required exclusion cannot be demonstrated without widening trust, keep the
  native gate closed and present the concrete blocker. Never silently enable a shell.

These are enforced application selection and provider-tool restrictions, **not
whole-process host isolation**. The trusted CLI can access credentials and may
maintain non-project client metadata. Provider-native sessions are not Waywr memory.
[Claude sandbox docs](https://code.claude.com/docs/en/sandboxing) explicitly
describe filesystem defaults broader than a working directory; provider sandbox
labels cannot establish project isolation by themselves.

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
(32 KiB total); reject symlinks, traversal, duplicates and executable instructions.
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

Checkpoints 1–4 are approved, subject to their gates. Each passing step gets a local commit,
evidence summary and concise handoff. Continue routine work within the approved
steps; stop on unexplained parity/integrity failures. Native spending stays gated.

| Step | Scope/dependency | Required gate and evidence |
| --- | --- | --- |
| 1. Native boundary proof | Pinned CLI capability/config manifests; isolated, unauthenticated mock-client fixtures; role-specific schemas and parser cases. No service cutover | Prove excluded hooks/tools/docs/history; record auth/managed-policy unknowns, fallback/retry behavior, exact models/efforts and stdout/stderr/error forms. If a read/tool remains unconstrained, do not enable native admission |
| 2. Conversation authority | Schema 4/minor 2, messages/questions/turns, action broker and durable budgets; reuse writer, host and receipt adoption | Discussion creates zero tasks; replay and restart apply messages/results/answers once; stale generation/proposal refuses; two projects share no context/IDs/authority; inspect remains <1 s and SQLite busy refusal <500 ms |
| 3. Integrated coordinator | Both adapters behind same interface; one coordinator lane, consultation, steering boundaries and explicit replacement; accepted TUI connected, no separate prototype | Mock native streams through installed app: discuss → clarify → consult → propose; concurrent slow worker doesn't block input; unavailable/exhausted/uncertain outcomes visible; no blind replay/fallback; exit/resume preserves pending work |
| 4. File workflow and final review | Static-files policy, manifest validation, source/delta handoffs, exact bundle review/approval, normal CLI inspection parity | Installed isolated tests for tampered files, traversal, coverage gaps, human answers, output cap/cancel, crash after output before checkpoint, lost replies and command duplication. Prior schema-3 simulated behavior preserved; no fake response presented as native |
| 5. One native acceptance | Review offline evidence, freeze approved plan/settings and independently obtain native-call authorization; one target/one trial | At most 11 launches as above; stop on failed/uncertain invocation, incompatible policy, model mismatch or exhausted budget. User observes working page and answers/approves. Report counts, hashes, receipts, remaining findings and any unverified terminal behavior |

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
effective Codex residual tools (now a checkpoint-1 blocker); managed/remote customization; provider internal
retries and model attestation; native cancellation/output behavior under the Rust
host; and physical-terminal/browser acceptance. Offline proof resolves what it can;
the bounded authorized trial must resolve the rest without silently widening scope.
No provider API replacement, general agent sandbox/platform, broad model picker,
automatic context discovery, parallel worker scheduler, Python dependency or
accepted-TUI redesign is part of this proposal.
