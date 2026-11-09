# Continuous conversation and reliable scope delivery — approved implementation plan

Planning checkpoint, 2026-10-14; source inspected at
`3d94b665c07d3f583ad84ecba8f313d432f16b7a` on
`feat/live-task-proposal-authorization`. **Approved at 5cc578c; implementation in progress.**
[Implementation report](continuous-coordinator-report.md) owns new verification;
[frozen contract](../../contracts/continuous-coordinator-contract.md) owns successor semantics.
This amends the prompt-only transport and whole-trial stop rules only for a future,
versioned profile. The original `claude-scope-01` remains blocked at 1/6. Its response,
receipt and authorization are neither converted nor reused. The original planning checkpoint ran no experiments; new execution evidence is
recorded separately in the implementation report.

## 1. Findings and reuse

| Current cause (source inspected) | Consequence / correction |
| --- | --- |
| [Claude arguments/decoder](../../../backend/src/claude_chat.rs#L79) use `stream-json` for transport, a prompt for result-v2, then parse the entire final text. No schema flag is supplied | Native reply was prose plus fenced JSON; refusal was correct. Use a supported, separately identified structured result, retaining application validation |
| [Readiness](../../../backend/src/conversation_store.rs#L105) searches all turns for failed/cancelled/output-invalid/uncertain, with no recovery distinction | Any such history permanently blocks new submissions and dispatch. Preserve that policy for old profiles; introduce typed outcomes and explicit user-directed continuation for proven safe cases |
| [Submit](../../../backend/src/conversation_store.rs#L425) accepts `available` **or** `running`, bounds queued/reserved turns at 16 and checks remaining allowance | Open questions/proposals do **not** themselves block composer submission. Reuse this durable queue; expose its meaning rather than replacing the scheduler |
| [TUI submission](../../../frontend/src/prototype/view.rs#L1232), [primary action](../../../frontend/src/prototype/view.rs#L1023) and [input routing](../../../frontend/src/prototype/view.rs#L1946) preserve nonempty input, but foreground the first pending question/continuation/review. `/cancel` is primarily discovered in Help/slash commands | Preserve composer behavior and optional panels. Add a visible active-call Interrupt action and per-message delivery/queue state; no compulsory review or automatic modal opening |
| [Scope capture/application](../../../backend/src/task_scope.rs#L70) compares the **global** latest-user watermark | Any subsequent message, even unrelated discussion, makes an earlier action stale. Replace this with exact subject, head, question/decision and authorization dependencies; retain message causality without treating all later discussion as a scope change |
| [Packet selection](../../../backend/src/task_scope.rs#L422) selects six messages with `seq < current user message seq` | If B is queued before A finishes, A's later reply is absent from B's history. Select completed causal predecessors at reservation, including replies arriving after B's submission |
| [Question application](../../../backend/src/task_scope.rs#L550) bumps scope epoch merely for asking; [scope authorization](../../../backend/src/task_scope.rs#L270) blocks pending task/null-target calls; [UI readiness](../../../frontend/src/prototype/view.rs#L837) blocks any queued/reserved turn | Discussion and clarification can invalidate or delay scope authority unnecessarily. Only committed content/relevant human-decision changes invalidate existing authority; pending conversation is shown separately |
| [Scope answer](../../../backend/src/task_scope.rs#L341) saves a human answer without a call; continuation must exactly resubmit its text. There is no model answer-recording action | Keep explicit answers; add an inert, source-linked answer/defer suggestion that a human can confirm inline. A normal next message also receives saved answers in context; continuation is an optional convenience |
| [Runner](../../../backend/src/runner.rs#L400) and [service adoption](../../../backend/src/session_service.rs#L1717) collapse capability, output and some application failures into broad statuses | A string such as `output-invalid` is insufficient to decide safe continuation. Preserve typed evidence through runner, receipt validation and writer adoption |

Reuse the Rust/Tokio service, bounded SQLite writer, operation host, claim/permit
ordering, immutable receipts, command outbox/deduplication, bounded transcripts,
editor, scope panels and exact-revision decisions. No workers, consultation,
execution allowance, provider switching, new scheduler or TUI redesign.

Existing tests cover queue bounds, idempotent submission, stale local edits,
linked answers, exhausted-budget local actions and explicit review. They were read,
not rerun. Existing mock passes and the original successful three-call schema-5
trial do not establish the new transport or interaction rules. The
[live-scope report](../live-scope/live-scope-report.md) owns the failed native evidence.

## 2. Smallest structured-result interface and admission proof

Recommend the **same pinned Claude print client**, saved login route, model and
effort, with `--json-schema` and `--output-format stream-json`. Consume the named
terminal `structured_output` field. No SDK runtime or direct-API/billing change.
The CLI documentation distinguishes transport JSON from schema output; the final
structured value is separate from ordinary `result` text. [Official print documentation](https://code.claude.com/docs/en/headless#get-structured-output)

Pinned evidence: `.local/native-coordinator-planning/2026-10-13/claude-help.txt`
includes `--json-schema`; SHA-256
`550db4f88ae1d97bf7244ca96dc89554aabd3978241ceb69f79001e816c35f09`.
Saved version is 2.1.296; binary and help hashes were rechecked read-only this pass.
Binary SHA-256 `24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`.
[Existing generic adapter](../../../backend/src/adapters.rs#L265) already constructs this
flag and reads `structured_output`, but lacks the narrower conversation profile's
complete checks. Reuse schema/field handling, **not its admission assumptions**.

Current primary documentation supports the flag, but is not a frozen 2.1.296
behavioral specification. The Agent SDK documentation explicitly describes
validation re-prompts, exhausted structured-output retries, and success without
`structured_output`. These are proof obligations, not permission to retry.
[Official structured-output documentation](https://code.claude.com/docs/en/agent-sdk/structured-outputs#error-handling)
`--max-turns` bounds agentic turns; it is not independently proof of one request.
[Official CLI reference](https://code.claude.com/docs/en/cli-reference)

### Approved credential-free gate

Reuse the retained boundary fixture design in
`.local/claude-conversation/boundary-03/fixture/proof.py`: disposable credentials,
private loopback mock endpoint, no external network route, no real HOME/login
mounts, hostile disposable hooks/MCP/history and read/write sentinels. First verify
the isolated environment is available; do not provision infrastructure, bypass
host policy or assume user namespaces work. Preserve request/event/access traces.

Freeze the schema and exact argv/environment before the proof. Start with the
current proven restrictions plus `--json-schema`; keep one agentic turn, transport
retry zero, no fallback. `StructuredOutput` may be an injected inert result sink.
If deny-all prevents it, investigate only a documented exact capability selection
for that sink, with all other capabilities excluded. Do not remove restrictions
wholesale or assume an allow rule overrides a deny rule. Record the effective
inventory and actual effects. Failure to prove this combination blocks admission.

| Focused fixture | Required outcome |
| --- | --- |
| Valid discussion (`action:null`), question, proposal and answer suggestion | One terminal structured result, exact schema/context identity, no file/process/external action. At most one inference POST, zero repair/fallback requests; successful case actually contains a usable result |
| Prose + fenced JSON only; absent structured field; malformed/unknown/duplicate keys; invalid nested action; excessive sizes | Zero application actions; terminal, explicit format failure when positively complete. No second inference request. Validate raw JSON before conversion can discard duplicate keys |
| StructuredOutput called with path/command/unknown fields; forced Read/Write/Bash/Agent/SendMessage/WebFetch/MCP calls | Inert payload or denial **before effects**. Sentinel access traces and captured requests prove exclusions; unchanged file hashes alone are insufficient |
| Hook/rule/history/MCP canaries, 401/429/529, truncated streams, output limit, cancellation | No inherited content/actions or hidden retry; retained typed failure, bounded exit, no false success. Pin exact terminal/stop-reason semantics rather than broadening success heuristically |
| Streaming structured value with text, escapes, code and adversarial fields | Only recognized assistant-text content reaches the UI provisionally. Thinking, other tool arguments and action payloads stay out; final text is reconciled once |

Cap the experiment at these cases and the one documented capability adjustment;
do not reverse-engineer an alternative runtime. If schema mode requires a second
model request even for valid output or cannot disable repair attempts, report that
specific conflict. Do not raise turns/retries, substitute a provider or call a
direct API. Provider-independent changes may proceed after their own gates, while
native admission stays closed. No current proof or account-level success is claimed.

### Proposed response contract

New profile `claude-proposal-v2`, response v3: retain `assistant_text`,
`evidence_refs` and at most one nullable inert `action`. Keep existing proposal
fields and limits; add only a source-linked question-disposition suggestion below.
Freeze schema/profile hashes into the packet and authorization. Strict duplicate,
unknown-field, size, reference, base and project validation remains mandatory.
The named structured field is a transport boundary, never arbitrary JSON extraction.
For proposal actions, validated `question:` evidence references select actual scope
dependencies; do not attach every question in the conversation automatically.

Track **speech completion** and **action disposition** separately. Display streamed
speech provisionally; never execute it. A complete, capability-valid speech channel
may be retained as conversation even when its action is rejected. Only independently
validated text from known provider text fields/blocks may be finalized in that case;
do not salvage `assistant_text` from a malformed JSON document. Otherwise retain it
as provisional evidence. When structured text is streamed, decode only that schema's
`assistant_text` field within the recognized result channel, never generic tool args.

Exactly one validated terminal completion is necessary for action processing;
transport success alone is insufficient. The application emits **Proposal saved ·
revision N**, **Not saved · invalid response**, or **Not applied · scope changed**
after its transaction/receipt checks. Model statements such as “saved” remain model
claims. Saved status and review availability come only from application records.
No format repair call, action inferred from prose, or salvage of the old trial.
Regression case: a reply says “saved” with absent/invalid action; no new proposal,
revision badge or new review action appears. Existing scope stays inspectable and
the application reports that no scope change was saved.

## 3. Conversation, queue and authority contract

### Input and interruption

| State visible in the existing TUI | Meaning / action |
| --- | --- |
| Draft · not sent | Local editable text; no call or scope effect. Keep typing during running/paused/blocked states. Reconnect never submits it |
| Sending · delivery pending | Stable command ID in the existing local outbox; reconcile acknowledgement before creating any new command. Not yet claimed queued |
| Queued · position N · sends after current call | Writer durably accepted the message; Send explicitly requested its next invocation within the allowance. FIFO, one active call, current 16-turn/32KiB bounds. No alteration of the active packet |
| Running · Interrupt | Always-visible, keyboard-focusable activity action; opens the existing confirmation. Composer/drafts remain usable. `/cancel` remains an alias, not required knowledge |
| Interrupt requested · awaiting confirmation | Exact active turn/generation is fenced. It is not yet “stopped”; retain debit and provisional output until reconciled terminal evidence |
| Queued · held after interruption/error | Saved messages are not lost and will not dispatch silently. Explicit Continue queued or a new Send is required under the policy below |

Provide a bounded queue inspection action and **Withdraw queued message**. Writer
CAS succeeds only while unreserved; retain the original message/event and mark
withdrawn, with no debit/refund. If reservation won, show Running and offer Interrupt.
Do not overwrite a newer composer draft when offering withdrawn text for editing.
Successful completion ordinarily advances the explicitly submitted queue. Detach
preserves that supervision; exit pauses dispatch and resume reconciles before
continuing. Error/interruption holds survive all lifecycle transitions.

Confirming Interrupt establishes an application action fence in the writer before
host cancellation. If result application committed first, report Already completed;
if interrupt committed first, later model actions cannot apply, even if the client
finishes before its process receives cancellation. Reuse verified process identities,
deadlines and cancellation escalation. Proven stopped/settled is distinct from an
interruption request or missing PID. No combined automatic interrupt-and-resend.

### Context and stale results

Keep an explicit conversation subject independent of which task the drawer happens
to inspect: existing scope ID, or a stable ID for the current new-scope discussion.
The existing New topic action creates that ID without creating a task. A successful
first proposal binds it once; queued follow-ups resolve that binding at reservation.
This avoids duplicate new proposals when two messages precede the first saved scope.
No model may retarget another subject/project; no new multi-chat product is proposed.

At reservation freeze subject, current head/hash, relevant question versions/answers,
authorization identity, binding generation, current message and causal predecessors.
Include preceding completed replies even if their event sequence is later than the
queued message's submission; exclude later queued user inputs. Keep six optional
prior messages, the mandatory current scope/decisions, provenance and 96KiB cap.
Mandatory overflow refuses before charging. Reconcile an already prepared packet
by its frozen identity; never rewrite it in place to fit changed state.

Later text is processed by its own turn. It does not modify the active packet,
bump scope decision revision or cancel that turn. Its earlier response is labelled
as answering the earlier message. A valid proposal suggestion can save against its
unchanged exact base while newer input waits; show **Later message queued; this
revision does not include it**. The following turn sees that saved revision and
the predecessor's reply. No global latest-user watermark veto.

Keep action-specific CAS checks: all actions bind subject/coordinator; proposal
updates also bind scope head/hash, relevant human-decision versions and authorization
identity; question suggestions bind exact question versions/source messages.
Informational speech/questions do not acquire a global scope dependency. Local edits or answers
that change those dependencies make the old action stale; preserve its text and
unapplied suggestion without overwrite, auto-rebase or extra call. Unrelated
discussion with `action:null` has no scope effect. Asking a question alone does
not revoke existing exact-revision authorization.
An informational question has no scope dependency unless a saved proposal explicitly
references it. Removing/changing an existing dependency is itself a saved revision
change and invalidates authorization; it cannot silently erase a decision.

Scope review remains an optional panel; closing/defer/navigation/acknowledgement
grants nothing. Ordinary discussion need not settle before explicitly authorizing
the **shown** revision. The final step names pending conversation and states it is
not included. An explicit pending local save or revision request for that scope
must settle first. Authorization changes its dependency identity so an older call
cannot overwrite that newly authorized scope. A later valid saved revision or
relevant confirmed answer invalidates effective authority and requires a new final
action. Retain old decisions; display **Scope authorized · execution unavailable**
only for its exact still-valid revision. No worker execution follows.

### Questions and answers without compulsory detours

Pending questions are visible context, not submission gates. User can discuss,
question the premise, add constraints or postpone a decision in ordinary text.
Panels never open automatically, and unsubmitted panel edits never enter context.
Keep the existing direct Save answer action and optional Continue convenience;
any normal next message receives relevant saved answers without repeating them.

Add inert `suggest_question_disposition` (answer or defer), binding question ID/
version, subject, exact source user message/hash and quoted UTF-8 span from the
frozen context. The application verifies provenance and presents **Record answer**
or **Defer question** inline. Recommend one explicit confirmation for a formal
decision; model interpretation alone cannot answer on the person's behalf. The
user may edit the suggestion or keep discussing. No phrase matching, automatic
continuation or call for saving. Confirmation produces an append-only human
question-decision record; projections can change, original answers cannot vanish.

Deferral is not an answer or scope permission. Keep it visible in context; final
authorization requires a saved revision that explicitly carries deferred questions
as unresolved decisions, with exact acknowledgement. Confirmed relevant answers/
deferrals change only linked scope dependencies; a different subject's question has
no effect. Suggested but unconfirmed answers do not affect authority. Save/edit/
defer actions still work with an exhausted allowance; no call is silently added to
incorporate them. Revision can be local or requested by the next user submission.

## 4. Recoverable errors and immutable outcomes

Apply these rules only to the new profile. Store typed outcome/evidence, not a
regular-expression interpretation of error prose. The old trial remains stopped.

| Outcome | Required evidence / next action |
| --- | --- |
| Known completed response, invalid proposal format/schema | Verified identity/receipt, clean terminal completion and settled host, unchanged capability boundary, zero accepted actions. Keep charged debit, speech/provisional status and failure. Hold queued messages. A deliberate new Send can acknowledge the exact failure and use remaining allowance; Continue queued explicitly releases held messages. Neither replays the failed call |
| Clean user interruption | Cancellation acknowledged and process/receipt reconciled; no late action application. Same explicit-new-turn policy, debit retained. No automatic replay or queue drain |
| Valid but stale suggestion | Completed call with unapplied action, not an integrity fault. Explain which saved dependency changed; ordinary queued discussion can continue against fresh context |
| Missing/contradictory completion, lost receipt or uncertain execution | Preserve uncertain state and hold dispatch. Reconcile the original operation; never use an absent process/unit as nonexecution proof or replace it automatically |
| Capability/profile drift, forbidden operation, cross-project evidence or packet/receipt corruption | Hard admission/integrity block; no new native turn until explicit investigation and correction. A new message cannot clear it |
| Auth/unavailability, non-success provider termination, transcript/event limit or unclassified failure | Actionable stopped outcome. Do not assume safe recovery from its name or refund a call. First increment automatically qualifies only the two positively proven recoverable cases above |

An acknowledgement is a durable command referring to the outcome digest and pause
generation, not a reset/delete of history or renewed allowance. A new Send after
a recoverable failure acknowledges that visible failure for its new turn; older
held messages stay held unless explicitly released/withdrawn. Eligible turns retain
their submission order. Two clients cannot clear a newer failure using an old
acknowledgement. Replayed commands neither release twice nor spend twice. Capacity
and remaining-budget checks include eligible queued promises; held/withdrawn entries
do not reserve calls. History remains bounded for display and durable for inspection.

## 5. Minimal records, delivery checkpoints and rollback

Propose state **7**, conversation config **4**, IPC **1.5** capability
`continuous_scope_v1`, profile `claude-proposal-v2`, response **v3**. Old clients
refuse new semantics; schemas 3–6 keep their explicit bound release contracts.
Add versioned subject/queue disposition, typed turn outcome and pause-generation
records, append-only question decisions and human continuation acknowledgements.
Reuse existing messages, heads, proposal revisions and authorization records.
New IPC mutations: withdraw queued, confirm question disposition, continue held;
extend submit with explicit subject and optional exact outcome acknowledgement.
Reads expose draft/delivery separately from authoritative queue, active turn,
recoverability and action disposition. No generated clients or alternate authority.

Use receipt schema 3 only for the new coordinator profile, carrying a digest of its
version-1 typed completion record (raw event/packet references and action validity).
Validate it in the existing host receipt path before writer adoption. Keep old
receipt versions/readers unchanged; missing typed proof never makes an old failure
recoverable. One transaction commits accepted text/actions/projections/events and
command outcome; no SQLite transaction or dispatch lock spans provider I/O.

| Checkpoint | Scope and focused acceptance gate |
| --- | --- |
| 1. Provider transport proof | Freeze new schema/capability and typed completion contracts; run only the isolated cases in §2. Pin exact stream/terminal behavior, no-effect enforcement and one-request limit. Retain failures. Native adapter admission stays closed if unavailable or failed; do not claim a mock server establishes model reliability |
| 2. Durable interaction authority | Queue/subject causality, scoped dependencies, question decisions, typed recovery and cancel/application fence. Deterministic tests: pending question + unrelated discussion; B queued during A gets A's reply; new constraint leaves A's packet unchanged; local edit makes A's action stale; new proposal binding once; malformed output + explicit new turn without refund/replay; concurrent clients/answer/authorize/cancel; receipt adoption after restart exactly once; exhausted local actions; two-project refusal |
| 3. Production TUI wiring | Visible draft/sending/queue/held/running/interruption states; optional review and inline answer confirmation; authoritative save status. Keyboard/paste/multiline/focus/scroll tests; 80/120-column rendering; interruption while composing; no auto-panel; empty Enter navigates, nonempty Enter sends/queues, scope final action alone authorizes. Package existing source-only Inspect fix |
| 4. One installed mock-transport candidate | Normal init/start/service startup, discuss with an open question, queue a constraint, interrupt, manually continue, generate/revise scope, local edit/answer/defer, explicit authorization, then material revision invalidates it; detach/exit/resume/restart, lost command replies and once-only application. Malformed structured result cannot save a proposal or trap all future discussion. No provider/worker/consultation launch; prior schema-5/6 fixtures retain old semantics |

Before each future checkpoint: commit/check source baseline, name state rollback
boundary; after passing: record evidence/variances, compact docs and commit. Preserve
under-1s status/control and under-500ms SQLite-busy refusal gates; bounded queue,
96KiB context, 32KiB speech and existing 32MiB transcript limits. No broad test rerun
without a relevant changed guarantee. Native proof and physical acceptance remain
separate from fixture passes. Subsequent live permission needs the new candidate,
profile/CLI hashes and fresh allowance/project; this plan spends no calls.

Rollback before new state: restore code checkpoint on empty isolated fixtures.
After new records/decisions: keep the compatible paired release or forward-repair;
never point schema-6 code at state 7, overwrite with a snapshot, refund a debit or
discard newly accepted work. Existing projects/services are not migrated. Cleanup
must retain bound binaries and immutable evidence. Failed stages do not advance
dependent work with unexplained differences.

## 6. Approved recommendations

1. **Admit only the proven inert structured-result capability** through the existing
   Claude CLI, with zero repair requests. Recommend it over prompt-only JSON; the
   proof must settle actual streaming/denial/retry behavior before native admission.
2. **Allow new user-directed turns after positively proven format failure or clean
   interruption**, within the same allowance. Recommend holding prior queued messages
   until explicitly continued. Ambiguity/capability/integrity remain hard stops; no
   retrospective change to the 1/6 trial.
3. **Confirm formal answers inline; keep scope review optional.** Recommend one
   confirmation for inferred answer/defer records, preserving free discussion and
   exact human authority. Automatic model interpretation would be smoother but could
   turn a hypothetical/negated statement into a durable decision. Do not adopt it.

Routine choices above need no separate architecture cycle. Implement
these checkpoints on this preserved source lineage, using isolated state/releases;
workers, consultation and authenticated acceptance remain separately authorized.
