> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical approved plan at `f5139db`; implemented on the stacked conversation branch.
> Current behavior and verification limits: [contract](../../conversational-coordination.md), [handoff](../../HANDOFF.md).

# Correction proposal: conversation-first coordination

Review/planning only, 2026-10-17. Base `a2d376d` on
`impl/provider-model-selection`; clean tree preserved on
`plan/chat-first-coordination`. Build on the installed state9 production Rust TUI,
provider routing, development scheduler and history. No code, inference, worker,
installation, service, project-data or permission changes are made in this review.

## What failed in the supplied example

This is both a request-routing defect and an interaction-design problem, not a need
for the user to learn more menus. Read-only evidence from the selected Example Task App
history and canonical records establishes:

| Evidence | Finding and correction |
| --- | --- |
| Events548–553, request `<private-record>`, subject `discussion-<private-id>` | “Lets add a label management to the list.” correctly started a new request. The coordinator asked which operations to support; no proposal was saved (`scope_saved:false`). Keep this request as the conversation owner even before it has a task/scope. |
| Event554, question `q-<private-id>`, revision2 | The answer **“CLI: list tags, rename tags globally, and remove tags globally”** was saved. `continuation_required:false` leaves coordination idle. Save-and-continue should be the default; no second Continue or new message is needed. |
| Event554 history sidecar | `request_id` is empty, so the question decision is absent from the new request's readable log. Link decisions through the question's subject even when no task exists. |
| Events555–558, request `<private-record>`, subject `discussion` | “Ok I authorize the scope” went to the old delivered Rust request (`<private-record>`). Its response described that old authorization. This was not an approval of label-management-example work. |
| Screenshot | The composer/activity shows the old Rust scope and “delivered”; a “Queued” footer remains after a completion. Current-request status, task inspection and message delivery need separate identities. |

Relevant source: [view.rs](../../../frontend/src/prototype/view.rs) persists
`conversation_subject` separately from drawer selection, but `set_target(Coordinator)`
can rebind it to the selected task's subject. `current_scope` is selected-task based.
Those are hazardous transitions when a new discussion has no bound task. The records
prove the destination changed, not which exact key sequence caused it; do not claim
that Save answer alone necessarily caused the switch.
[continuous.rs](../../../backend/src/continuous.rs) explicitly saves a question decision
without continuation or request tagging in that path. The
[development discussion prompt](../../../backend/src/development.rs) directs users to TUI
approval and `/steer`/`/reply`, while [main.rs](../../../frontend/src/main.rs) dispatches
ordinary Send as discussion. A prompt-only change cannot fix these contracts.

Evidence paths, read only:

- New request: `~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
- Misrouted reply: the retained `2026-10-15T162749Z-please-prepare-the-scope-for-example-task-app-in-this-project--build-a-small-offline-<private-record>.log` in that directory, events555–558.
- Missing history linkage: `.waywr/state/state.sqlite`, `session_events` and
  `request_history_events` event554, joined to `dialogue_questions` by the above ID.

## Recommended experience

The composer is the normal way to discuss, answer, revise, approve and steer. Menus,
shortcuts and detailed editors remain optional views over the same application
commands. Do not replace the accepted TUI with a wizard or a second chat interface.

For this request, the corrected interaction is:

> You: Let's add a label management.
>
> Coordinator: I suggest listing tags with counts, renaming them globally and
> removing tags without deleting tasks. Include all three?
>
> You: Yes, all three.
>
> Waywr: Answer saved for **Label management**. Preparing the proposal…
>
> **Label management · proposal v1 · awaiting approval**
> List/count, global rename and remove; preserve tasks, Unicode and safe JSON saves.
> Implement model operations and CLI commands; verify behavior and update documentation.
> Use the displayed project providers and operating policy for edits, tests, local
> commits and checked integration. No remote publication or unrelated-project changes.
> **Reply “approve” to start, describe a change, or open details.**
>
> You: OK, I authorize the scope.
>
> Waywr: **Label management v1 approved. Starting work.**

The example's scope remains a proposal, not a new authorization or application task
submitted by this review. The real summary must come from its saved records and show
actual provider/network/main-policy choices and unresolved decisions. Keep it concise
but sufficiently explicit to approve; expandable details and the existing `/plan`
view expose full requirements, checks and policy.

If the user says “approve” before any saved proposal exists, continue the discussion
and present a concrete proposal to approve. Never apply that earlier assent to a
proposal created afterward. In the screenshot case, recover the saved answer and
prepare the label-management-example proposal; do not borrow the old Rust authorization.

## Interaction and authority decisions

**1. Stable conversation ownership.** Make current request/subject the source of
composer, current proposal, open decisions, activity and context. Task selection is
inspection only unless the user explicitly chooses “Discuss this request.” Going
back from questions, settings, output, review or history never silently retargets Send.
Preserve per-request drafts and outbox identity across refresh, reconnect and provider
handover. An unbound request shows “Label management · discussing,” never a previous task.

A new change after delivered work starts a follow-up request naturally, linked to
current project knowledge, without requiring `/new`. A question about earlier work
can remain discussion. When that distinction affects execution and is unclear, ask
one inline question. Do not reopen delivered authorization implicitly. Keep `/new`
and explicit request selection available. Other active work remains visible separately;
creating a discussion does not evade the one-active-workflow rule.

**2. Close question loops.** A clearly directed user answer is saved against the
exact question/revision and advances coordination automatically, in the same request.
Use the normal coordinator turn for interpretation; do not add a model call just to
classify every message. Preserve verbatim source input and label any interpretation.
Model-suggested answers are not user answers. “What do you recommend?” is discussion,
not a disposition; conflicting/multiple possible targets get inline clarification.
Deferral remains explicitly unresolved. Menu Save should have the same save-and-continue
behavior; optional “save for later” must be an explicit choice.

Commit answer plus a durable continuation/outbox intent together. Fold it into an
already queued eligible turn where possible; otherwise queue one continuation keyed
to question decision revision. If a prior call is active, wait for its settlement
and include the new answer in the next packet; never mutate its frozen packet.
Deduplicate across restart and do not resume held uncertain execution just because
an answer arrived. Show “Answer saved; waiting for coordinator capacity” when appropriate.

**3. Exact approval in chat, without mandatory menus.** After the writer saves a
proposal, render an application-owned approval card from that exact revision. Its
reply context binds project/request/subject, proposal and policy hashes, question/
decision revision, unresolved-choice acknowledgement and current authorization head.
Only a card actually presented in this conversation is eligible; streamed/model prose
is not the approval target. Backend validation remains the authority, not the card.

A directly submitted, unambiguous affirmative such as “approve,” “go ahead with this
plan,” or “OK, I authorize the scope” against the current eligible card becomes the
existing exact `authorize_scope` command. Record the original user-message ID/text
hash and card identity with the approval. No extra “confirm approval” follows a clear
approval, and no coordinator inference is needed merely to authorize it. An inline
**Approve** action offers the same command; `/plan` keeps detailed review available.
Inline **Change**, **Not now** and **Reject** retain their distinct meanings: editing
needs a new saved revision, deferral launches nothing, and rejection closes that
proposal without cancelling unrelated running work.

Use a conservative deterministic resolver for direct assent to a single presented
card, with negation, quotation, conditional and mixed-intent guards. It is not a
keyword search across transcripts. For free-form language the resolver cannot safely
recognize, the normal coordinator can suggest an interpretation, but it cannot commit
permission: show a concise inline “Approve Label management v1?” action/reply instead.
“Approved, except don't remove tags” requests a revision, not partial approval of the
old plan. A bare “yes” to an open product question answers that question, not work
approval; multiple current decisions require an explicit target.

No inferred approvals from assistant text, artifacts, imported history, quoted or
pasted logs, recalled commands without a fresh deliberate submission, or future
proposals. Preserve paste/recall provenance. Pending same-request messages not yet
reconciled with the proposal disable approval with a concrete inline reason; unrelated
worker activity must not stale it. Changed scope/policy/decisions produce a refreshed
card and a short delta, never an automatic retry with a newer hash. Lost replies
reconcile the original command, not a second approval.

**4. Discuss and steer through the same composer.** “What if we used a different
format?” is discussion; a clear “Change the export to include tag counts” is a steering
request. Preserve its source and affected requirement/task references. Apply only a
validated in-policy delta through the existing steering command; show what was changed
and which queued/active results are affected. If intent or scope compatibility is
unclear, show one bounded inline interpretation for confirmation. Scope/authority
expansion gets a revised approval card. Never make arbitrary model classification
an authorization source or rewrite active packets under the worker.

Open development questions can likewise be answered in chat through `development.reply`.
Routine work continues within permission, without repeated approvals. Clear pause/
cancel/resume requests use the existing lifecycle commands with a named target;
ambiguous “stop that” asks which work. Unknown execution remains a reconciliation
problem, not something a conversational “continue” may clear.

Provider requests such as “use Claude for review” use the existing saved-preference,
role/policy and safe-switch mechanisms. Display the effective change inline. Existing
grants need no renewal; a new context destination receives a specific inline grant
card. Coordinator switching preserves request identity and pending decision cards,
subject to fresh revision checks. F3 remains the detailed configuration option.

**5. Plain status and one relevant next action.** Show “Discussing,” “Preparing plan,”
“Awaiting your approval,” “Working,” “Needs your decision,” “Waiting for capacity,”
“Paused,” “Delivered,” or “Recovery needed” for the current request. Separate
message queued/running/completed from workflow state and request inspection.
Correlate footer notices with command/turn identity so completed messages do not
stay Queued. Replace routine “no call spent,” “formal decision recorded” and
“open the visible action” narration with the saved decision and what happens next.
Keep technical evidence and usage in Inspect. Empty Enter remains navigation, never
accidental approval; focused inline Approve requires deliberate activation.

## Minimal implementation and recovery changes

Reuse the current composer, snapshots/events, outbox, question decisions, scope
commands, provider policy amendments, binding generations, scheduler and history.
Add a small versioned interaction context to submissions: request/subject, optional
reply-to question/card, presented revision/watermark and input provenance. Preserve
this at submission; never resolve a queued input using today's drawer selection.
Persist pending decision/card identity and continuation deduplication in the same
writer/records. These are projections and causal metadata, not a second approval
ledger or a new conversation agent.

Extend typed coordinator results for source-bound answer/revision/steering suggestions
and follow-up routing as needed. The service validates allowed intent, source user
message, target and revisions before invoking existing commands. Keep separate labels
for “suggested,” “saved,” “authorized” and “started.” Update prompts to reflect these
real capabilities; remove mandatory-menu advice only when the corresponding path is
implemented. Negotiate the new IPC/interaction capability and preserve old bound
schema semantics; don't silently reinterpret already-saved historical messages.

Tag all pre-proposal decisions and continuations through subject/request linkage.
For the observed event554, a later authorized repair should append a correction
referencing its original timestamp, question and saved answer to the proper request
history; do not splice it into already published frames. Keep events555–558 in their
original history and append an explanation/cross-reference if needed. Never replay
“Ok I authorize the scope” into a new authorization. Resume the existing label-management-example
request and its answer; do not seed another job or alter Example Task App application code.
No such repair is performed during this planning pass.

## Connected delivery and focused checks

1. **Correct routing and truthfulness first.** Separate discussion ownership from
   inspected-task state, fix pre-proposal answer/history association and stale status.
   Reproduce the new-request + old-delivered-task case with the actual UI transitions.
2. **Finish normal conversational planning.** Wire source-bound answers and once-only
   continuation, saved proposal cards and direct exact approval through existing
   backend commands. Keep the menu path equivalent; avoid four mandatory review stops.
3. **Connect ongoing coordination.** Route clear replies/steering/provider changes and
   lifecycle requests through the same context/decision mechanism. Preserve responsive
   chat, revision fences, safe handover and recoverable history. Update help/contracts
   with the implemented behavior, not merely new suggested phrases.

These are implementation checkpoints, not separate trials or permission gates. Focus
verification on the screenshot regression, answer→proposal continuation, direct
approval→worker eligibility, and menu/chat equivalence. Include only important edges:

- New unbound topic while an old delivered scope is inspected; question Save, return
  to composer, refresh/reconnect and provider switch all retain the intended request.
- Answer recording/continuation crash and duplicate delivery; event554-style decisions
  appear once in the correct history, with originals retained.
- Valid current-card assent succeeds once; no proposal, stale card, conditional/negative/
  quoted/pasted assent, wrong subject, multiple decisions and pending edits cannot
  authorize a different revision. Newly arriving proposals never consume old assent.
- In-scope steering versus scope expansion and hypothetical discussion; workers and
  uncertainty fences remain intact, and completed-turn notices clear by identity.

After implementation is separately authorized, the user should be able to finish
this real label-management-example discussion, see its concrete proposal and approve from the
composer, with menus optional. The user originates/approves the work; an implementing
agent must not impersonate that approval or call fixtures human acceptance. No broad
provider matrix or rerun of the completed Example Task App migration is needed.

## Product decisions resolved for this proposal

Recommend conversation-first as the default, exact inline approval without mandatory
second confirmation, automatic continuation after clear answers, and optional menus
for detail/editing. Ambiguity causes one targeted inline clarification, not a workflow
wizard. Preserve original history and expose corrections explicitly. These choices
address the requested behavior; no additional product choice blocks the proposal.
Implementation and any live repair remain outside this review's authorization.
