# Conversational coordination

The state9 development path advertises IPC1.8 / `conversational_coordination_v1`.
Older bound schemas and historical submissions keep their existing semantics.
See [installation](distribution.md) for available distribution methods.

The current request owns the composer, its questions, proposal and work summary.
Selecting a task inspects it. `/discuss` explicitly changes conversation to the
inspected task's request; `/new` starts another discussion. Request drafts and
submission context survive refresh/reconnect. Reply targets freeze when a draft
begins, including question revisions and proposal/provider-card identities; ordinary
recall deliberately targets the current context while Down restores the original draft. The submitted subject is frozen in
the durable input intent; a later drawer selection cannot change its destination.

Ordinary chat goes to the selected live coordinator, including recaps after delivery,
explanations and general topics. It does not require a plan, task or scope change.
Project-specific claims use the supplied context; general knowledge is allowed with
honest uncertainty. `action:null` is a completed conversational reply, never authority.
Optional references are constrained to the supplied allowlist in new native schemas.
An otherwise valid ordinary reply with unknown citations still displays; the exact
raw result is retained and recognized/unrecognized references are recorded separately
in message/completion evidence for Inspect/history. Unknown citations are unverified,
not action dependencies or verified provenance. Proposal, answer, steering and retrieval
actions retain strict reference, source, revision and authority checks. Older interaction
contracts retain their validation semantics. Invalid framing, capability use or execution
uncertainty still stops safely; the TUI names the reply problem and retains the message.

Normal Send carries versioned request/reply context and fresh-submission provenance.
A clear answer to one current question uses the existing answer command and saves
its continuation intent in the same transaction. The scheduler folds it into a
queued eligible turn or queues one continuation; an active frozen packet is unchanged.
Recovery pauses and uncertain execution are never cleared by an answer. The optional Save-answer menu uses the same continuation policy. Quoted answers
require the displayed source-bound Save action; explicit deferral stays unresolved.
Legacy
question commands without the new capability keep their original continuation policy.

Saved application-owned proposal cards name the exact proposal, decisions, operating
policy and conversation watermark. Presentation is recorded only after the TUI
renders the card. A directly submitted unambiguous approval uses the existing exact
scope command. Its decision retains original input identity/hash, provenance and
presented-card identity. No further confirmation or model classification is needed.
A changed card or pending same-request input refuses the old approval; it is never
rebased or applied to a future proposal. Quoted/imported transcripts do not grant
permission. Deliberately submitted plain pasted or recalled input remains usable.
Unclear assent offers one inline Approve action; conditional/mixed requests stay
revision discussion. Empty Enter only opens detail; deliberate focus and activation
is needed for inline actions. Menus remain optional views/editors.

Coordination inputs use the existing writer and command ledger. Each accepted input
retains one immutable child command; restart reconciles that same command rather
than selecting a new effect or replaying uncertain execution. Existing provider,
revision, scope, host and integration guards remain authoritative.

`session repair-answer-history --event SEQ --request ID --misrouted-events SEQ,SEQ`
is explicit stopped-project maintenance. It validates the actual saved question,
answer and subject/request linkage, then appends correction references and an
explanation. Original events, linkage and published frames remain unchanged. It
queues no continuation and never resubmits an approval. A resume hint restores the
correct owner for the old UI-state format; newer explicit user selections win.
Unsent legacy drafts remain with their original request and no decision target;
returning to one cannot bind its old assent to a newer proposal.

Normal chat also recognizes exact named session pause/resume and request
cancel/resume operations. Existing reconciliation gates remain; “continue” does not
clear uncertain execution. Clear steering imperatives enter the normal coordinator
turn with source identity. Its typed `propose_steering` must match the original human
message, current workflow/generation and unchanged scope/policy before the existing
steering command can run. Hypotheticals stay discussion; ambiguous or expanded scope
requires clarification/revised approval, never model-granted authority.

“Use Claude for review/implementation,” “switch coordinator to Claude” and the explicit
“interrupt and switch coordinator to Claude” reuse configured profiles and future
attempt preferences. No model/effort substitution occurs. Existing grants are reused;
a new destination/role receives a narrow saved inline grant, approved with
`allow provider` against its exact identity. F3 remains the detailed editor. Unclear
profile names or simultaneous decisions ask for an explicit target. Fallback and
frozen attempts are unchanged. Grant reconciliation can reuse a later scope approval
that already includes that destination instead of asking twice.

Inputs have immutable child command IDs/bodies, and chained provider steps retain
those IDs across restart. Current-card dispositions defer/reject only that revision;
rejection cannot reopen merely because another discussion message arrives. New work
after delivery creates a linked discussion from clear imperatives; free-form ambiguous
intent remains conservative and may need explicit `/new` or an inline action.

Bundle `interaction_contract: conversation-v1` prevents choosing an older release
that cannot interpret retained causal records. Pending effects and continued-answer
intents participate in the existing quiescent release-change guard.

Concise proposal cards are typed application rows, separate from activity text
classification. Full scope/policy remains in optional Details. Effective role choices
are labelled as defaults or fixed request pins, independently of permission sets.
[Request routing](provider-selection.md#runtime-and-model-selection) preserves human
source and enforces selected review profiles through assignment, reservation, launch
permit and exact-commit review gates. The approved source directive is routing intent,
never execution approval. Background presentation acknowledgements do not set message
Sending status; terminal completion wins late input receipts.

Reply targets belong only to nonempty drafts. Invoking a slash command discards its
reply target, empty targets are omitted from UI persistence and ignored on restore,
and the first input into an empty composer captures the current card/question again.
A nonempty draft keeps its original target across refresh/restart. Stale-card refusal
names the changed proposal/policy rather than claiming pending input when none exists;
it offers the current inline action, never silently replays approval at a newer hash.
