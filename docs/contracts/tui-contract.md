# Production TUI contract

This owns the accepted Rust/Ratatui presentation and input behavior. It does not
make historical trial limits universal. [Architecture](../architecture.md) records
current capability gaps; [continuous contract](continuous-coordinator-contract.md)
owns state7 authority. Older schemas keep their bound semantics.

The state9 IPC1.8 [conversation contract](../conversational-coordination.md) owns
stable request routing, answer-and-continue, saved proposal cards and exact inline
approval, with menus optional. Draft/reply identity survives refresh, recall and
restart. The menu-only/separate-answer semantics below remain the older bound
version; they do not constrain newly negotiated conversational inputs.

Ordinary discussion and post-delivery recaps display as coordinator replies without
requiring a scope change. Unknown optional citations remain unverified metadata;
application actions retain their exact source/authority checks. Genuine result or
provider failures name the unavailable reply and preserve the user's message, without
marking a delivered development request failed. The human-verified recap and its
limits are recorded in the [checkpoint](../archive/conversational-coordination/2026-10-17-verified-checkpoint.md).

## Presentation

Conversation is primary. A compact header identifies Wayward Wrangler, project,
transport/mode, provider/model/readiness and separate project-service connectivity.
The composer stays anchored/left-aligned. Selected-task activity remains visible;
tasks/workers use a collapsible drawer. Panels expose visible focusable actions.
Content uses available terminal width, with readable narrow/wide layouts.

User blocks use dark-theme #4A4D54, `> ` on the first line and aligned continuation.
Composer uses #41454C, a `›` marker, readable foreground/selection and blinking
focused cursor. Light/monochrome equivalents retain meaning. Coordinator messages
group under headings such as `• Coordinator` with indented `└` content. Paths,
artifacts, inline/fenced code, choices and statuses have distinct styling; meaning
must survive without color. IDs/hashes live in Inspect unless needed for a decision.

Paired strong/emphasis markers render bold/italic across wrapping. Code preserves
literal markers; unmatched streaming markers and user input remain literal.
Startup effects are optional/subtle, respect reduced/off mode and never gate input.
Idle state does not repaint continuously. Mock transport is conspicuous; it cannot
be silently substituted for a native coordinator or presented as natural conversation.

Development event labels use typed presentation metadata derived from saved status,
check outcomes and effective review disposition. Words in model summaries/task titles
cannot create a failure, cancellation or uncertainty label. Review outcomes distinguish
complete, needs changes, stale and failed; completion of a review does not assert
combined checks or delivery. Optional metadata defaults absent in older UI journals;
replayed exact events can refresh that display cache without appending messages or
changing original events/authority. Legacy untyped review prose remains neutral.
For conversational topics created with `/new`, a later saved scope updates the composer
caption only; topic ownership, routing, frozen draft intent and text remain unchanged.

Activity and coordinator headers use the committed event timestamp in local time:
`HH:MM` today, otherwise `MM/DD/YY HH:MM`. Missing historical timestamps explicitly
say unavailable; observation time never substitutes for occurrence time. Events
retain their original order. The final live-work line shows the current coordinator,
checks or worker count with elapsed time from saved operation/reservation metadata.
Only that line shimmers in subtle mode; reduced/off/mono stay static. No running
indicator is shown for idle/blocked work, another scope, or a paused session.
Running workers appear beneath that line as tree bullets with a task title (or ID)
and a separate current-operation label. Labels use tool lifecycle metadata from the
published host attempt, never command output, model text or reasoning. They refresh
once per second independently of `/output` selection/cursors and drafts. Completed
commands clear immediately on the next refresh; terminal receipts show finishing.
Missing/stale metadata shows an explicit generic label; a command label expires
five seconds after its last successful refresh. Opening `/output` waits for any
outstanding background list read before fetching the full attempt list. The reader
inspects at most 256 KiB of complete native-event frames per active worker, so an operation starting
before that tail may have no detail. PageUp/PageDown also scroll these live rows.

## Input, focus and bounded scrolling

- Enter submits nonempty input; distinct Shift+Enter inserts a newline. F2 multiline
  mode is the terminal fallback. Bracketed paste inserts literal content without
  submitting or invoking slash commands. Preserve drafts, cursor and target.
- Empty Enter or focused activity action opens the authoritative next panel. It
  never answers, approves, rejects, authorizes or launches work. Tab/Shift-Tab and
  Escape have consistent focus/back behavior. No refresh opens a panel unexpectedly.
- Modal action rows accept Left/Right as previous/next selection alongside
  Shift-Tab/Tab; navigation never activates an option. Enter activates the selected
  action. Focused text editors retain cursor movement; Tab leaves editing. Scope
  editor Field uses Up/Down, queue messages use Up/Down, and page keys scroll.
  Provider forms and output attempt selectors retain their specialized arrow controls.
- Composer command history: Up/Down recalls older/newer sent or attempted input,
  including typed slash commands. Down past newest restores the exact unsent draft,
  cursor, multiline and literal-paste flags. Recalled entries can be edited; recall
  itself never sends, invokes commands, changes targets or grants authority.
  Alt+Up/Down moves the multiline cursor. Menus/overlays keep their arrow selection.
- PageUp/Down scrolls chat eight wrapped lines; Alt+PageUp/Down moves one wrapped
  line. Arrows retain command recall/cursor controls. Ctrl-R
  opens the project-local history picker; Enter there only fills the composer.
  Escape returns to the composer. History persists in `session/command-history.json`
  under the canonical state root (recent 200 entries, up to 1 MiB), with locked merge
  and atomic private-file replacement. Invalid existing files are preserved/refused.
  First-use seeding uses recorded user messages as literal input, never inferred
  commands, approvals or decisions. Unrecorded old slash commands remain unavailable.
  Changing targets ends recall and preserves the original target’s unsent draft.
- Inspect/Review/Scope/Artifact/Output: Up/Down moves one wrapped line, PageUp/Down
  eight lines, or one with Alt. Stop at actual content ends, also after resizing/refresh. Choice/task/
  target lists retain arrow selection; page keys scroll overflowing content.
- Background refresh preserves draft, focus and reading position for unchanged
  identity. New activity marks unread content when scrolled away. Optional Vim
  navigation affects read-only views, never composer text.
- Typed single-line `/` opens a searchable contextual command menu. Enter invokes
  selected available actions; Tab completes without execution. Escape preserves
  input. Pasted slash text, paths, code and multiline text remain literal.

`/help`, `/settings`, `/tasks`, `/inspect`, `/output`, `/plan`, `/answer`, `/open`,
`/approve`, `/cancel`, `/detach`, `/exit` and explicit `/herdr` reuse the same
handlers as function keys. Unavailable actions explain why. `/open FILE` preserves
its argument. No unsupported action may masquerade as rejection or another call.
Closing an overlay, clearing a draft, detaching, exiting and cancelling are distinct.

## Continuous conversation and scope

State7 shows draft/not sent, delivery pending, queued/running/held counts and
Queue F7. One active frozen packet is immutable. Nonempty Send remains discussion
while questions/review are pending; no compulsory scope panel or phrase matching.
Empty Enter can open the discoverable Interrupt confirmation without losing the
composer. Release/withdrawal of held inputs are explicit durable actions.
Formal inferred answer/deferral requires source-bound Record; ordinary discussion
needs no confirmation. Saving an answer does not submit a continuation call.
An Answer panel without an open question has no Save action.

Scope review is optional via `/plan` or its visible next action. `/plan` resolves
the active conversation subject’s saved task, including after resume; an unbound
new topic reports no saved scope rather than displaying an earlier request. A newly
saved topic scope becomes the activity selection without opening a panel. Refresh
preserves explicit task inspection and open reviews; F6 retains earlier scopes.
Activity status and active worker count belong to the selected scope, never another
request’s latest workflow.
Switching selected tasks clears review progress/acknowledgement, and stale detail
responses cannot replace the current task/revision. It presents one
numbered step at a time, one primary action and relevant auxiliary controls:

1. Purpose/deliverables → Continue.
2. Boundaries/acceptance → Continue.
3. Unresolved choices → Acknowledge and continue (or Continue if empty).
4. Exact saved revision → explicit Authorize scope.

Steps1–3, opening/acknowledgement and conversational assent grant nothing. Previous,
Edit scope, Request revision and Not now remain available where applicable. Escape/
Not now records no scope decision. Known-repeat Enter cannot advance/authorize.
Already-authorized scope is a read-only result view, not another approval prompt.
Local editing uses the textarea, field selection/list lines, rationale, Save and
explicit Discard; Escape retains the original-base draft. Never silently rebase it.

Unchanged project/task/revision/hash/dependency/authority preserves review step,
focus, position and acknowledgement. Changed identity resets step1 without opening
a closed panel. Backend stale checks remain authoritative. Scope permission binds
exact revision/hash; material changes need renewed explicit permission, while
unrelated discussion does not revoke it. Pending conversation is visibly excluded.
Current native profile says **Scope authorized · execution unavailable**, with no
worker admission/allowance. Local edits/answers/decisions remain usable at exhaustion.

Application save/revision/not-applied badges follow committed records, never model
claims. Streaming text stays provisional until validated completion; omit internal
thinking/tool arguments. Readiness distinguishes awaiting authorization, available,
running, exhausted and blocked. F3 reviews the exact profile and allowance, never
silently renews it. Allowance ceilings are profile-specific, not UI product policy.

## Existing workflow/artifact paths

Older state3/4 workflows retain their versioned next actions: Review scope, Answer
question, Review candidate or View result. State4 mock static-file reviews use the
full verified bundle; `/open index.html`, `/open styles.css`, `/open app.js` and
`/open README.md` are individual previews, not full-bundle approval receipts.
Initial final review shows Read candidate only; explicit Approve/Reject appear
after full exact-candidate reading. Preserve read progress for identical project,
task, request and hash; invalidate on change. Stale-candidate refusal and backend
approval checks remain decisive. A completed task offers result inspection.

These existing simulated paths are not native worker readiness. State5 chat and
state6 scope semantics are maintained in their compatibility contracts; state7
continuous rules supersede earlier global pending-turn conversation blocks.

## Request-history visibility

No example request, plan, task sequence or approval is seeded at startup. The human
uses coordinator conversation and `/new` to initiate work, reviews the resulting
scope/policy and explicitly authorizes it. `/reply` targets a current substantive
development question by workflow revision. `/steer` remains the separate explicit
in-scope delta path. Paused Send preserves its draft and explains `/resume`; neither
reopening context nor viewing history resumes execution. `/resume` first distinguishes
session pause from a running session’s selected workflow in attention. Operational
recovery binds work ID/revision and keeps scope/policy authority unchanged. A saved
question accepts a clear answer in chat or `/reply`, never a generic resume. Typed
recovery events show cause, next action, wake condition, what can continue and whether
a decision is needed. Workflow and worker rows retain scoped recovery status; they do
not replace current operations of independent workers. Equal events deduplicate by
cursor. Ordinary paste, command recall and discussion remain usable while recovery
waits. A generic continuation never clears uncertainty or approves unchanged scope.

State8 Inspect exposes absolute history/compact-summary paths plus saved projection
watermark, freshness/attention and narrative-pending state. Child tasks and attempts
share the same root request. Coordinator narrative is distinguished from saved
application facts; paths survive detach/resume. Displayed history grants no approval
or acceptance. No additional panel or approval is required to save events.

State8 conversation presents readable task/check/integration/delivery milestones;
mechanical bookkeeping stays in canonical events/history and Inspect. Suppressed
bubbles still advance the event cursor. Retained/approved prose does not imply Done.
F6 derives development status from the parent workflow, shows current unresolved
obligation counts before retained attempts, and labels unit obligations as reported
claims rather than current unresolved state. Older run IDs do not determine state8
delivery status. Inspect shows the readable combined branch, exact commit and
combined checkout directory. Before delivery that directory can contain newer code
than primary/main. Preview is not human acceptance or delivery. A scoped assignment
blocker stays visible without labelling unrelated ongoing work globally blocked.
Retired assignment activity names the coordinator's reason and preserved obligations;
named result-branch activity identifies its preview checkout. Both enter readable history.

Candidate checkout removes the requirement to navigate a private worktree manually.
Project build/run commands and human visual inspection still run outside Waywr.
The [ordinary-project contract](../ordinary-project-workflow.md) owns setup,
visible candidate cards, exact feedback/acceptance and clean checkout handoff.
Conversation offers `check out candidate`, `accept candidate`, `candidate feedback: …`,
`include committed changes` and `end validation`; quoted/stale input cannot accept
newer code. Build/run/inspect/shutdown suggestions are tied to the candidate and are
separate from execution receipts. Optional Inspect retains full details. Native
end-to-end usability remains awaiting ordinary use; fixture results do not establish it.
