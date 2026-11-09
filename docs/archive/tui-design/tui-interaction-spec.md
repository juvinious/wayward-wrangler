# Wayward Wrangler: terminal interaction specification

**Implementation update:** The accepted renderer/editor described here now
drives the installed `waywr` frontend. See the [unified UI integration report](../milestone-6/unified-ui-integration-report.md)
for simulated installed checks and the remaining service gaps. The historical
design status and checkpoint below record the original review boundary.

**Status: design for review; implementation remains paused.** Inspected 2026-10-11
on `milestone-6-interactive-shell`, checkpoint `ac5f8e8` (implementation through
`04e2419`, report at `6261a2f`). This document specifies the next UI prototype;
it is not a claim that the current application has these interactions.

This supersedes the milestone-6 proposal's earlier UI layout, colon commands,
Enter/Ctrl-S submission, mandatory insert/normal modes and continuous redraw.
It preserves the Rust/Ratatui frontend, Python backend, versioned IPC,
authorization, durable records, bounded recovery and independent projects.
The application owns work; detaching its frontend does not abandon that work.

## 1. Recommendation and verified starting point

Use a conversation-first alternate-screen application: a compact header, one
scrolling conversation, a small selected-task activity strip and an anchored
multiline composer. Tasks/workers live in a collapsible drawer. Questions,
scope authorization and final review have visible, keyboard-accessible actions.
Slash commands open the same views; they are shortcuts, not prerequisites.

The next approval should authorize **only an isolated scripted Rust UI
prototype** (§12). It should establish the interaction before connecting the
new presentation to real services. No production workflow or state rewrite is
needed to evaluate the UI. Service integration gaps below remain explicit.

### What exists, and what does not

| Evidence inspected | Actual behavior at checkpoint | Design consequence |
|---|---|---|
| `frontend/src/ui.rs`, `frontend/src/main.rs` | Ratatui 68/32 conversation/task grid; String composer with append/backspace; Enter newline, Ctrl-S submit; Esc/i editing modes; colon parser; no command picker. | Preserve Rust foundation, replace presentation/input. Slash corrections have **not** landed. |
| `frontend/src/ui.rs` | Bounded history, message-based scrolling, selected-task index; no full cursor editor, draft persistence, rich conversation model or contextual gate controls. | Add editor and presentation state; scrolling must use wrapped visual lines and stable message anchors. |
| `frontend/src/main.rs` | Bounded IPC channels, event cursor, local command outbox and terminal guard; redraw every 33 ms; `:open` suspends UI for Vim in its terminal. | Reuse transport/terminal concepts; audit reconnect settlement, make rendering event-driven, route explicit external inspection separately. |
| `src/agent_workflow/session_service.py` (`handle`, `plan`, `task_detail`, `dispatch_command`) | Project service, fake coordinator, task versions, durable commands, proposal/digest authorization, question/approval routing and explicit read grants. Not every endpoint in the earlier proposal exists. | Exact mappings and gaps are in §8; do not implement imagined IPC in the UI. |
| `src/agent_workflow/workflow.py` (`answer`), `session_service.py` (`modify_queued` via store) | Choices must match supplied options; final review accepts approval/rejection. Steering only modifies an unreserved queued task. | Free-form answers, proposal revision and active steering need contracts. Request changes is not rejection. |
| `src/agent_workflow/release_maintenance.py` (`manage`, `_quiescent`) | Session stop currently requires quiescence. | This conflicts with approved pause-and-stop semantics. Keep the gap open; do not redefine stop to justify current code. |
| Handoff/report and filesystem | `.local/milestone6/` stable evidence/install preparation has not landed. Prior evidence/installations remain referenced under `/tmp`. | Do not present a new stable installation or hands-on preparation as completed. |

The inspected fake coordinator copies the submitted request into a structured
proposal; it does not interpret arbitrary conversation. The new prototype will
use richer **scripted** responses, not a model and not proof of coordinator
intelligence. Existing simulated backend acceptance remains separate evidence.
No runtime checks, workers, service changes or installation occurred in this
design pass. Process status in the pause handoff is historical, not reverified.

Familiar patterns inform the design: editable input, discoverable `/` menus,
explicit submit/newline behavior and contextual choices. Terminal-dependent
modifier keys are a reason to provide a visible fallback. These patterns are
documented in [Claude Code interactive mode](https://code.claude.com/docs/en/interactive-mode)
and [Gemini CLI keyboard shortcuts](https://geminicli.com/docs/reference/keyboard-shortcuts/).
No shared application framework, branding or orchestration implementation is
assumed or imported from those products.

## 2. Screen system

### Persistent regions and focus

1. **Header:** project display name; coordinator/provider/model; connection;
   `SIMULATED` or `NATIVE`. Never infer model from conversation text. Full
   project path, release, identifiers and hashes live in Inspect.
2. **Conversation:** user/coordinator labels, task association and short
   application events. Application notices say `Waywr`, never impersonate the
   coordinator. Detailed runner output stays in worker inspection.
3. **Activity strip:** selected task title/status, observed calls versus limit,
   pending-question count and one primary action such as `[Answer question]`.
4. **Composer:** persistent target chip (`New task` or `Steer: <title>`), text,
   then a small toolbar with Send, multiline toggle and current hints.
5. **Drawer:** current project's tasks grouped Active / Needs attention /
   Queued / Finished, expanding one task's workers. All actions name a task.

Initial focus is the composer. Empty-state copy:

> Describe the work you want to plan. You'll review the scope before workers
> start. Type / for shortcuts.

In simulation add: **“Scripted demonstration — no models or real work run.”**
The mode label remains visible in every overlay and external-view placeholder.
No auto-opening overlays when a question arrives. The activity strip and an
unread marker announce it while the user keeps typing.

### Concrete layouts

Coordinates below are one-based terminal cells. These are normal layouts with
a short draft; growing the composer takes rows from conversation, not the
header/activity/actions. Empty rows are intentional reading space. Brackets
mean keyboard-focusable actions, not mouse-only buttons. The first wireframe
is a 24-row screen; wider views give exact row/column allocations below.

#### 80 × 24: one column, drawer as overlay

```text
Waywr · Project Decision Log              Connected · SIMULATED
Coordinator: scripted / decision-log-v1                 [Tasks 2]
────────────────────────────────────────────────────────────────────
You · Clarify alternatives
  Clarify the remaining choices without changing free editing.

Coordinator · scripted
  I can draft an addendum. Free in-place editing stays accepted.
  Supersession correction and cross-tab behavior remain open.

Waywr · Draft ready; independent review running.
  You can keep typing or inspect the worker.

                                        2 newer events · [Latest]

Clarify alternatives · Awaiting answer · 2 of 5 calls
May a decision have no alternatives?                  [Answer]
────────────────────────────────────────────────────────────────────
To: New task                                           [Change]
> Draft a short manual walkthrough after this addendum.
  |
[Send] [Multiline: off]                             [Tasks] [Help]
Enter send · F2 multiline · / commands
SIMULATED · Your answers affect this demonstration only
```

Rows 1–2 header; 3 separator; 4–15 conversation (including unread marker);
16–17 activity; 18 separator; 19 target; 20–21 editor; 22 toolbar; 23 contextual
hint; 24 persistent simulation/notice line. Columns 1–80 available; content
uses one-cell side padding. No right drawer competes for text at this width.
Tasks opens a full-width body view in rows 4–17 with `[Back to conversation]`;
the composer remains below it. Review uses rows 4–22, preserving but hiding
the composer; Escape restores its exact draft/cursor.

#### 120 × 36: conversation with optional compact drawer

```text
Waywr · Project Decision Log   Coordinator: scripted/decision-log-v1         Connected · SIMULATED
──────────────────────────────────────────────────────────────────────────────────────────────────
You · Clarify alternatives                                                │ Tasks [Hide]
  Clarify the remaining choices without changing free editing.             │ ! Clarify alternatives
                                                                         │   Awaiting answer
Coordinator · scripted                                                   │   [Answer question]
  Review the proposed addendum before simulated work begins.              │
  [Review scope]                                                         │ + Manual walkthrough
                                                                         │   Queued for planning
Waywr · Draft published; critique completed.                              │
  Question: may a decision have zero alternatives?                        │ Workers · selected task
  [Answer question]                                                      │ author · fake · draft
                                                                         │   Completed [Inspect]
                 ... conversation continues ...                          │ critic · fake · review
                                                                         │   Completed [Inspect]
──────────────────────────────────────────────────────────────────────────────────────────────────
Clarify alternatives · Awaiting answer · Workers 2/5 · Reviews 1/2                  [Answer] [Inspect]
──────────────────────────────────────────────────────────────────────────────────────────────────
To: New task [Change]
> Draft a manual walkthrough for the addendum.
  |
[Send] [Multiline: off]
Enter send · F2 multiline · / commands · F6 tasks · F1 help
SIMULATED · Scripted events; no models or real work
```

Exact allocation: row 1 header; 2 separator; rows 3–26 conversation; optional
drawer columns 85–120 in those rows, separator column 84, conversation 1–83.
Rows 27–28 activity; 29 separator; 30 target; 31–33 editor; 34 toolbar; 35 hint;
36 simulation/notice. The wireframe elides blank conversation rows only.
Drawer starts closed; reopening restores selection/scroll. With it closed,
conversation uses all 120 columns with prose capped at 96 cells.

#### 160 × 48: generous conversation and task drawer

```text
Waywr · Project Decision Log   Coordinator: scripted/decision-log-v1                        Connected · SIMULATED
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
You · Clarify alternatives                                                              │ Tasks [Hide]
  Clarify alternatives and preserve the approved free-editing decision.                  │ ! Clarify alternatives
                                                                                       │   Final review
Coordinator · scripted                                                                 │   [Review candidate]
  The candidate records your answer and leaves two other policy choices open.            │
  [Review candidate] [Selected context]                                                 │ + Manual walkthrough
                                                                                       │   Authorized · queued
Waywr · Revision ready. No further independent review has occurred.                      │
                                                                                       │ Workers · selected task
              ... anchored conversation, not a diagnostics grid ...                     │ author · fake · revision
                                                                                       │   Completed [Inspect]
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
Clarify alternatives · Final review · 3/5 simulated calls · 2 unresolved topics          [Review] [Inspect]
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
To: New task [Change]
> |
[Send] [Multiline: off]
Enter send · F2 multiline · / commands · F6 tasks · F1 help
SIMULATED · Answers and approvals here are demonstration records only
```

Exact allocation: row 1 header; 2 separator; rows 3–37 conversation, columns
1–115; separator 116; drawer 117–160. Rows 38–39 activity; 40 separator;
41 target; 42–45 editor; 46 toolbar; 47 hint; 48 simulation/notice. Drawer
defaults open at this size, with manual preference preserved across resize.
Prose within conversation remains capped at 96 cells; code/artifact views may
use the available width. Do not stretch reading lines merely to fill space.

### Overflow, panels and narrowing

- Editor grows to at most one-third of height (8/12/16 rows at these sizes),
  then scrolls internally. Show `line 7/12` when clipped. History scrolls by
  rendered lines; anchor it by message ID plus text offset across reflow.
- At width <110 use the drawer body view, not a squeezed side panel. At
  width <80 use a two-line header and stacked activity/actions; shorten titles
  with ellipsis, never truncate the meaning of approval/permission text.
- Minimum usable size is 60×18. Below that show “Resize to at least 60×18;
  work continues” with draft preserved, F1 help and Ctrl-Q detach still active.
  Disable mutation controls rather than displaying clipped confirmations.
- Long proposals, questions and reviews have scrollable content and a fixed
  action footer. At 80 columns they use the body; at ≥110 an inset panel up to
  100 columns. Default focus is content or Back, never Authorize/Approve.
  If actions wrap, reserve another row; never hide the focused control.
- Prose wraps on word/grapheme boundaries. Code blocks retain indentation and
  offer horizontal scrolling in their detail view; summary blocks are clipped
  with `Open full block`. Tables get a stacked narrow rendition. Exact bytes
  remain available in plain-text artifact view; rendering never changes hashes.

## 3. Input contract

There is no mandatory insert/normal mode. Ordinary typing always works when
an editable field has focus. Tab/Shift-Tab cycle composer → Send/multiline →
activity actions → conversation → open drawer → header/help, skipping absent
controls. Modal panels trap focus until dismissed. Focus has a marker and
contrast independent of color. Status updates never move it.

| Key | Composer / text field | Conversation / drawer / panel |
|---|---|---|
| Enter | Submit ordinary message in send mode. In multiline mode, insert newline. Empty/whitespace draft does nothing. Question text fields do not submit the form. | Activate focused action; open selected task detail. Never approve merely by entering a panel. |
| Alt-Enter | Insert newline without submitting, when delivered as a distinct key. | No workflow action. |
| Shift-Enter | Same newline shortcut **only when enhanced-key support is established**; do not advertise otherwise. | No workflow action. |
| F2 / visible Multiline toggle | Toggle `Enter: send` / `Enter: newline`; draft unchanged. In multiline mode Tab to Send then Enter is the reliable submit path. | Return to composer and toggle; not while a modal form is active. |
| Arrows; Home/End | Move cursor by grapheme/visual row and logical line edge. Never navigate sent history. Ctrl-Home/End reaches draft start/end. | Move selection/scroll; Home/End first/last item. In read-only code detail, Left/Right scroll horizontally. |
| Shift-Arrows; Ctrl-A | Extend selection; select all draft. | No task mutation. Read-only text can enter its Copy view. |
| Backspace/Delete | Delete grapheme/selection, undoable. | No action; never cancel a task. |
| Ctrl-Z / Ctrl-Y | Undo / redo editor operation, bounded to 100 edits. No process suspension. | No action; help explains no job-control suspension. |
| Ctrl-C / Ctrl-X / Ctrl-V | Copy/cut/paste **internal editor buffer** when appropriate; Ctrl-C without selection is a no-op with hint. Never stop work. | Ctrl-C does not detach or cancel. System clipboard uses terminal shortcuts, below. |
| Ctrl-R | Open current-project sent-message history; search and insert selected text into draft, never send it. Exclude answers/approvals/commands from history replay. | Same history picker, restores prior focus on dismiss. |
| PageUp/PageDown | Scroll conversation without changing draft/cursor. | Scroll focused view by a page. |
| Tab / Shift-Tab | Move focus, except command menu completion described in §4. Literal tabs arrive by paste; no hidden tab-to-space rewrite. | Move focus; arrows navigate list items, not Tab. |
| Escape | Dismiss menu; else clear selection; else stay in composer. Never clear draft or exit. | Close top overlay or return from drawer/conversation to composer. No task action. |
| F1 / F6 | Contextual help / task drawer; draft retained. | Same; Escape returns to previous focus. |
| Ctrl-L | Redraw after display corruption; no state change. | Same. |
| Ctrl-Q | Detach panel: “Work continues. This draft will be saved.” Choose Stay or Detach. | Same; never stops service or worker. |

**Multiline acceptance gate:** test Alt-Enter through Crossterm in a PTY and
the actual Ghostty→Herdr path, plus Shift-Enter with and without keyboard
enhancement negotiation. These bindings are proposed, not tested in this
design pass. Terminals that collapse modified Enter to Enter cannot be
distinguished by the application; show F2/toolbar fallback unless distinct-key
support is verified. The fallback requires only ordinary Enter, Tab and a
visible toggle, so no enhanced key is necessary. Do not reinterpret trailing
backslashes: paths and code must remain literal.

Bracketed paste is one insertion/undo operation, including its newlines; it
cannot submit or trigger a command. Keep 32 KiB UTF-8 draft limit; reject an
oversized paste as a whole while retaining the old draft. Display a precise
size error. Input handler consumes key press/repeat for editing; ignore release
events and suppress submit repeats. Repeated Enter while delivery is pending
does not mint additional command IDs.

Optional `Vim navigation` preference is off by default and affects read-only
conversation/drawer only: h/j/k/l, gg/G, Ctrl-U/D. It never captures text in
composer, search, question or reason fields. `q` and `i` are ordinary input
when editing; neither is a global action. Arrow navigation always works.

Drafts are per project and target, including cursor, mode and paste-literal
flag. Save atomically in a 0600 client cache on view change/detach and debounced
after edits; max 32 cached drafts/1 MiB, never silently evict a nonempty draft.
They are not backend history, authorization or model context. Transport outbox
is separate. A failed submission can restore its original text. A transient
disconnect keeps the draft in memory even if cache persistence fails; display
that failure instead of promising restart recovery.

## 4. Slash menu and conversation target

On an operator-typed `/` at byte zero of an otherwise empty, single-line
composer, open an anchored menu above it. Filter by prefix then substring of
name/description (stable ordering); show at most six results plus count. Each
row includes name, description, shortcut if one exists, and unavailable reason.
Up/Down selects; Tab or Enter completes the selected command and shows its
argument form; it **does not execute it**. A subsequent Enter opens the view.
Escape closes the menu with all text retained. Reopen by editing the slash
prefix; no menu trapping arrow movement once dismissed.

```text
/a
  /approve  Review candidate before approving    unavailable: no final review
  /answer   Answer a pending question              Enter to complete
```

Parsing is deliberately narrow: only a known complete command token at byte
zero in a single-line, non-pasted draft is a command. `/tmp/file`, leading
whitespace, fenced code, embedded slashes and any multiline draft are literal
coordinator text. Paste marks the entire draft literal until cleared or the
user explicitly chooses `Use as command`; pressing Enter after a paste cannot
execute `/approve`. Unknown `/word` offers `Send as message` or `Edit command`,
never guessed execution. Colon-prefixed text is ordinary text, not a hidden
compatibility UI command. Shell compatibility executables are unaffected.

Arguments use visible forms, not an ad hoc shell parser. Optional task labels
resolve only inside the current project; ambiguous labels open a picker. Do
not accept unreviewed arbitrary filesystem paths as an action target. `/answer
text` and `/reject reason` may prefill a form but cannot submit it.
The import form is the explicit exception for selecting another project's
source location: the user supplies it and reviews the bounded grant before any
content is imported. It does not browse the registry or grant recursive access.

### Small initial command set

All entries below either use existing backend capabilities or are local UI
navigation. Missing richer capabilities are not advertised as working commands.

| Command | Description / arguments | Availability and mapping |
|---|---|---|
| `/help` | Help for this view (F1) | Local. |
| `/tasks` | Choose task / inspect its workers (F6) | Local task list; explicit worker focus needs G3 below. |
| `/new` | Compose a new task | Local target selection; does not submit. |
| `/inspect` | Details for selected task | `task.get`; usage/context subviews only when supplied. |
| `/plan` | Review proposed scope; optional configured profile/context selection | `task.plan`, proposed task only. |
| `/confirm` | Open the same scope panel | Never an immediate authorization; requires current plan. |
| `/answer` | Open pending question; optional prefilled option/text | `answer_task`, only a live question. |
| `/open` | Read current candidate | Artifact inspection; G2/G3 before production integration. |
| `/approve` | Open final review | Candidate review binding required; no direct command-line-style approval. |
| `/reject` | Reject current candidate, with reason | `answer_task` with rejected outcome, explicit final-review panel. |
| `/steer` | Edit an unstarted queued request | `steer_queued`; disabled after coordinator reservation with explanation. |
| `/cancel` | Cancel selected task | `cancel_task`; confirmation names task and consequences. |
| `/grants` | Inspect explicit imports | `context.access.list`; no unrelated-project registry enumeration. |
| `/include` | Authorize selected read-only material | Form for operator-supplied source/record/purpose; `grant_context_access`. |
| `/revoke` | Stop future reads under selected grant | `revoke_context_access`; retained provenance explained. |
| `/detach` | Leave interface; keep work running (Ctrl-Q) | Local terminal cleanup; no service-stop/cancel call. |

No `/retry`, provider switch, shell execution, task-queue skipping or request-
changes command is invented. Theme/effects/navigation preferences are in Help
and local UI options, not a new workflow command surface. Session stop remains
an advanced lifecycle action pending G5; do not expose a misleading shortcut.

The target chip is always visible. **New task** is the default, including
while another task executes. Selecting a task for inspection never silently
retargets text. Choosing **Steer: <title>** is explicit; current production
availability is limited to an unreserved queued request. If that target starts
before submission, retain draft and say “This task has started. Your change
was not applied.” Offer Copy or New task; do not silently reroute it. General
conversation/active steering is the target experience but requires G4.

## 5. Tasks, authorization, questions and review

### State and wording

The UI is a projection, not a scheduler. Two axes remain distinct: local
delivery (`draft → pending → acknowledged / rejected / unknown`) and task
progress supplied by the application. No timer or coordinator prose implies a
durable transition.

| Condition | User-facing wording | Authority / visible action |
|---|---|---|
| Local submit, not acknowledged | `Sending… not yet saved` | Client outbox; inspect delivery, keep other input responsive. |
| Command committed | `Request saved` | Durable command result; provisional bubble becomes accepted once. |
| Waiting for coordinator slot | `Queued for planning` | Existing queued task event. Distinct from authorized worker queue. |
| Coordinator reservation/execution verified | `Coordinator working` | Persisted coordinator operation; no invented progress percentage. |
| Proposed task | `Scope ready to review` | Proposal event; Review scope. Clarification is a future contract G4. |
| Authorized, waiting for slot | `Authorized · queued` | Admission result / ready task state. |
| Worker active | `Drafting` / `Independent review` / `Revising` | Verified application stage; Inspect worker. |
| Human gate | `Your answer needed` / `Final review ready` | Pending request identity; Answer / Review. |
| Approved/completed | `Specification approved` | Exact approval record. Do not say “Application verified.” |
| Rejected/cancelled/failed | `Candidate rejected` / `Task cancelled` / `Task stopped` | Authoritative outcome plus reason and retained evidence. |
| Execution ambiguity | `Execution uncertain — automatic continuation stopped` | Application uncertain record; diagnostics, no blind Retry button. |

Question and review notifications append concise conversation events; task
progress updates the activity strip/drawer without inserting every poll or
replacing messages. Multiple pending questions have a count and explicit
request selection; answered questions stay inspectable. Never use a single
global “current answer” variable across tasks.

### Scope panel

Show: task title/purpose and intended deliverable; exclusions; exact selected
context with source/version; coordinator settings and separate author/critic
provider/model/effort; worker invocation ceiling and critique ceiling; current
project and whether all execution is simulated. Show estimates only if backed
by a described estimator, otherwise `Cost/token estimate unavailable`.

Actions: `[Back] [Revise request] [Cancel task] [Authorize simulated work]`
(native wording only for separately authorized native operation). Revise is
disabled for an already proposed task until G4 exists; tooltip explains why.
Do not implement it as cancel-and-resubmit. Authorize sends exactly the frozen
plan digest/task version/state revision through `admit_task`; the digest stays
in Inspect. Staleness invalidates the panel and requires rereview, not another
coordinator call or a silent refresh-and-approve. Routine authorized transitions
continue automatically; navigation never authorizes.

### Question panel

```text
SIMULATED · Clarify alternatives · Question 1
May a new decision have zero alternatives?
Why now: the approved specification leaves this validation rule open.
( ) Allow none; show “No alternatives recorded”
( ) Require at least one alternative
( ) Other answer…
Answer: [text field when applicable]
After saving: the author continues using your answer; saving uses no model call.
[Back] [Save answer]
```

No choice is preselected. Choices use arrows/Space; Tab reaches Save. Free-form
text is a required design affordance, but current backend rejects answers not
in its options. The prototype labels Other as **“UI preview; production answer
contract pending”** and records it only in its isolated journal. It must not
claim current production support. G1 must precede integration of this control.
Question explanation/next step come from structured records, not invented UI
rationale. Selection/draft is `Not sent`; outbox is `Sending answer…`; backend
ack is `Answer saved — continuation pending` until a stage event arrives.
Closing the panel preserves its draft and does not answer it. Restart/replay
cannot apply the answer twice. An answer consumes no invocation; a subsequent
agent call does consume the frozen budget.

### Final review panel

Show candidate label/version, readable exact content, unresolved finding IDs
with original evidence and later assessments, relevant accepted decisions and
whether the last revision was independently reviewed. Default fixture wording:

> Addendum candidate · 2 policy topics still unresolved.
> Free in-place editing remains accepted. This revision has not received
> another independent critique. Approving this specification does not verify
> a working application.

Actions: `[Back] [Read candidate] [Open in Vim] [Approve candidate] [Reject…]`.
`Request changes` is visibly unavailable with “No revision-request operation;
this will not reject the candidate or start a call.” It is not a slash command.
Approval stays disabled until the exact candidate is opened in the reader or
the verified external review action succeeds. This is a review opportunity,
not proof the user read it. Show a final summary of unresolved issues beside
the explicit Approve button; no hash copying or hidden default approval.
Bind task, pending request, version/revision and candidate hash internally.
Changed bytes or version invalidate the reviewed marker and refuse stale
approval. Never substitute the latest candidate after the user confirms.
Reject requires a reason and explains that it ends this review, not requests
another revision. Existing approval semantics remain intact.

## 6. Async interaction, reconnect and failure

Maintain scroll anchors, current target, focus and unsent drafts while updates
arrive. Follow newest only if already at the bottom; otherwise show `N newer
events · Latest`. Returning to Latest is explicit. Coalesce transient stage
activity at ≤4 Hz. Do not flood conversation with token chunks or timer ticks.

UI input must not wait for model work, Herdr calls, artifact reads or IPC.
Reuse bounded command/event channels (16 entries), 64-event batches and the
200-item/2 MiB history window; page older material on demand. Keep receipts,
transcripts and full history in their authoritative stores, not an unbounded
widget. Backpressure suspends fetching, not input. If sending cannot enqueue,
leave the draft and explain the local queue is full.

Connection states: `Connected` → `Disconnected — work may continue` →
`Reconnecting 1/3` (1/2/4-second bounded attempts) → `Connection unavailable`.
Cached history stays readable and typing stays enabled. Mutations are disabled
until project/release handshake and snapshot reconcile. Unsent drafts can be
saved locally but cannot become “queued work” without backend acknowledgment.

For a lost reply, show **“Delivery unknown; checking saved command.”** Query
the existing command ID; resend only the same ID/payload when the protocol
allows it. Never mint a replacement task/approval to recover a reply. A
transport error is not rejection. Show applied, rejected or still unknown
after reconciliation. Event replay deduplicates sequence IDs; refresh the
projection on a gap while preserving reading/draft state. Offer a bounded
Reconnect action after exhaustion, not an indefinite busy animation.

Failure cards say what happened, what is intact and what can be done:

> Worker completion could not be established. The draft, your answer and
> execution evidence are retained. This task is stopped as uncertain.
> [Inspect evidence] [Copy diagnostics] — No automatic retry.

> Workspace unavailable. Reconnecting owned panes (attempt 1 of 3).
> Your saved answer remains intact; no new agent attempt has been started.

Recovery success must be an application event confirming reconciliation, not a
UI timeout expiring. Task failure leaves the coordinator interface usable for
inspection and new input. An uncertain operation can block that project's next
dispatch when safety requires it; the UI explains this and never blocks another
project. Service-fatal failure stays distinct from a failed task.

**Worker inspection:** explicit action targets verified project/operation/pane
identity. Python's Herdr integration focuses that exact pane only on request;
background progress never steals focus. Before leaving, show the coordinator
tab title and “Return using Herdr's tab selector.” If the pane is gone, show
retained transcript/receipt; do not respawn it just for observation. Manually
typing/stopping a worker is not a recorded human answer. Report any resulting
cancellation/uncertainty through reconciliation. A log-only pane is not actual
worker execution. The standalone UI prototype shows a labelled transcript
preview, not proof of real Herdr visibility/focus behavior.

## 7. Context, lifecycle, accessibility and effects

### Context and usage

Inspect shows only selected task context: source project identity when explicitly
granted, record kind/ID, version/hash, purpose, grant direction/scope and current
revocation/staleness. No registry browser, neighboring-project search or automatic
related-project expansion. Unrelated projects contribute zero UI/model content.
Revoking a grant stops future reads; frozen history retains provenance. Only
explicit dependencies affect staleness. The global registry is routing data.

Show measured stage invocations/limit, critique rounds/limit and coordinator
calls separately. `3 of 5 calls used` is not a completion percentage. Pending
answers have no model-call cost. If tokens/cost are absent, show `Not reported`;
if provided, label measured versus estimated and source. No guessed ETA/spend.

### Distinct lifecycle actions

| Action | Meaning |
|---|---|
| Escape / Back | Dismiss view, preserve draft. |
| Clear draft (composer toolbar overflow) | Clear only this draft, undoable; no workflow change. |
| Detach | Save drafts/outbox, restore terminal, disconnect UI. Already authorized work continues. |
| Cancel task | Explicit named-task command/confirmation; outcome comes from backend, not disappearance of pane. |
| Stop supervision | Approved contract: persist dispatch pause, exit service; existing worker may finish, no next stage. Different from detach/cancel. Current implementation is quiescence-gated; G5 must reconcile this before exposing the action. |

Opening/closing a panel is never authorization. Normal question answers continue
authorized work; record-answer-and-exit remains an explicit test behavior only.
Rust owns presentation, draft cache and IPC client. Python owns durable tasks,
decisions, scheduler, execution and Herdr routing. UI cannot repair databases or
redefine budgets/recovery. Existing per-project release/service contracts stand.

### Terminal and accessibility

Use alternate screen for anchored editing and deterministic resize; conversation
history belongs to the app, not native terminal scrollback. Keep mouse capture
off initially, so Ghostty selection and Ctrl-Shift-C/V remain usable (terminal
bindings can vary). Scrolling uses keys; mouse wheel may be terminal behavior.
Provide a Copy view of plain text for an individual message/artifact with no
adjacent drawer content, stable while selected. No clipboard daemon dependency
or unsolicited OSC52 writes. Internal editor clipboard is not OS clipboard.

This is keyboard-first, not mouse-required. Focus, status and unavailable reasons
always have text/symbols; no color-only states. Unicode width/grapheme handling
must cover emoji, combining marks and CJK; supply ASCII markers in reduced mode.
Sanitize ANSI/OSC/control sequences from provider/pasted display text, display
unsafe controls visibly, never execute links automatically. Preserve raw artifact
bytes for verification. No remote images, HTML execution or network fetches from
Markdown. Large artifacts are paged; errors don't replace them with empty text.

Terminal guard + panic cleanup restores raw mode, alternate screen, cursor,
paste and any enhanced-key modes on exit/error/unwind. SIGINT/SIGTERM detach
gracefully; SIGKILL cannot restore the terminal (`reset` is documented repair).
No external editor may strand the TUI raw mode. Screen-reader support of a
full-screen TUI is limited: plain read-only Copy/Inspect and existing diagnostic
CLI remain available; do not claim accessibility compliance without evaluation.

### Theme and TachyonFX

Prefer terminal-default background/foreground. Offer explicit dark/light/mono
preferences; use safe default colors when theme detection is absent. Honor
`NO_COLOR` and `TERM=dumb` (dumb terminal gets a clear unsupported-interactive
message, not escape garbage). Color overrides are presentation-only.

| Semantic role | Dark palette | Light palette | Text/symbol fallback |
|---|---|---|---|
| Body / secondary | Default / light gray | Default / dark gray | Normal text / label; never faint essential copy |
| Focus | Cyan underline/reverse | Blue underline/reverse | `>` selected; `[Action]` |
| Activity | Cyan | Blue | `Running` |
| Attention | Amber | Dark ochre | `! Answer needed` |
| Success | Light green | Dark green | `Done` / `Approved` |
| Failure / uncertain | Light red | Dark red | `Failed` / `? Uncertain` |
| Simulation | Magenta label | Dark magenta label | `SIMULATED` always present |

Include TachyonFX only on accent cells: 100 ms menu/drawer edge transition;
one 250 ms attention-accent pulse for a newly arrived question; 150 ms completion
accent. Content, composer, approval wording and artifacts render immediately at
full contrast. No text scrambling, wipes over text, perpetual spinner/background
effects, or delay before accepting input. At most one effect per region; replace
pending accents rather than queueing them indefinitely.

Effects settings: `subtle` default, `reduced` (static accent only), `off` (no
effect scheduling). Reduced-color/mono defaults off. Max 30 effect frames/sec;
render on changed state/input/resize otherwise, with **zero periodic idle
repaint**. Stop timer when effects finish. Effects never generate or gate a
workflow event. Under load drop animation frames, not input or durable events.

## 8. UI action → existing IPC, and integration gaps

This is an implementation inventory, not a new generic protocol. Existing
`hello`, `snapshot`, `events_read`, `command_get` and `command` remain the
transport basis. Every mutation retains command ID, task/version/state checks
and project identity. UI labels do not change action semantics.

| User action | Existing request/action | Limits / additional work |
|---|---|---|
| Submit new task | `command: submit_message` | Today always a new task, not general conversation. Coordinator operation is budgeted. |
| Inspect task | `task.get` → `task.result` | Task/proposal/run/pending/current/findings/completed; missing richer projections G2. |
| Review scope | `task.plan` → `task.plan_result` | Proposed task, configured profile/context and owned workspace; frozen digest. |
| Authorize | `command: admit_task` | Exact displayed plan/task/state bindings. |
| Answer option | `command: answer_task`, outcome `answered` | Request binding; current options only. Free-form G1. |
| Approve / reject | `command: answer_task`, outcome `approved` / `rejected` | Approval exact artifact SHA; human reason on rejection. Request changes absent. |
| Edit queued request | `command: steer_queued` | Only before coordinator reservation. No active/proposed-task revision. |
| Cancel | `command: cancel_task` | Existing process/receipt safety checks; may refuse ambiguous execution. |
| Inspect/add/revoke import | `context.access.list`; `command: grant_context_access` / `revoke_context_access` | Explicit selected source records, pinned scope; never automatic discovery. |
| Attach/reconnect | `ui.attach`, `ui.register`; snapshot/event replay | Launcher/service ownership unchanged; UI reconnect is not worker retry. |
| Read artifact / inspect worker / open Vim | No matching current UI IPC endpoint | G2/G3. Current frontend directly opens returned artifact path in blocking Vim. |
| Detach | Client disconnect + terminal cleanup | No current `ui.detach` endpoint; don't send a fabricated command. |
| Stop / continue supervision | Not a current UI IPC operation | G5; maintenance CLI currently refuses busy stop. |

### Presentation work (prototype scope)

Editor/layout/focus, slash parser/menu, structured conversation rendering,
visible forms, immutable view-bound confirmation data, draft storage, bounded
scroll/unread state, semantic theme/effects, fixture event source and simulated
delivery journal. Reuse source concepts without changing production launcher,
workflow schemas, databases, adapters or services in this prototype.

### Backend/transport gaps (later integration, not silently simulated support)

| Gap | Evidence and consequence | Bounded resolution / acceptance before production wiring |
|---|---|---|
| **G1 Free-form questions** | `workflow.answer` only accepts an exact option. | Version question contract with answer mode/validation; retain legacy choice interpretation. Persist arbitrary allowed text with request/version binding, replay once, no model call for saving. Never map Other to a fabricated option. |
| **G2 Read projections** | `task_detail` lacks paged history/artifact bodies, complete decisions/context/usage and worker inventory; proposal's `history.read`/`artifact.read` are not handled. | Add narrowly typed read endpoints/projections for current project/task and bounded cursors. Hash-check candidate reads, include authoritative invocation counts and provenance. Cross-project material only under existing grants. |
| **G3 External inspection** | UI blocks in same-terminal Vim; `review.open_review` has managed-Herdr environment restrictions. `worker.observe`/`artifact.open` are proposed, not handled IPC. | Expose identity-bound display requests through existing supported supervisor Herdr connection; reuse safe Vim argv/containment, not spoofed environment. Verify exact owned pane, no focus steal, explicit return path, lost-response reconciliation. Prototype preview is not this proof. |
| **G4 Conversation / revise / active steering** | Fake coordinator echoes message; `steer_queued` refuses started tasks; no proposal-revise, intake clarification or request-changes action. | Initial wired UI must disable unsupported actions. A separately scoped contract must distinguish conversation/new-task/steering, append revision with stale checks and scope/budget consequences. Never cancel/resubmit, reject, or launch extra calls as an approximation. |
| **G5 Stop lifecycle** | `_quiescent` gates current maintenance stop. | Implement previously approved dispatch-pause/cooperative-detach contract with focused live-worker simulation before presenting stop/continue controls. Retain completed receipts, no next-stage permit until explicit queue continuation. |
| **G6 Delivery settlement** | Rust command thread retries twice then can exit; outbox removal includes generic `error` frames. | Classify terminal application rejection separately from transport/unknown result, preserve unresolved outbox, reconnect command channel and reconcile stable IDs. Test lost acknowledgment for answer and approval; never replay under new ID. |

These are concrete integration gates, not authorization for a backend redesign
or a full conversation engine. Prototype-only controls are labelled as such.
An installed production UI must omit/disable unsupported actions, never let a
fixture response imply the backend has performed them.

## 9. Components and compatibility decision

Primary manifests/docs inspected on 2026-10-11; dependency selection is proposed,
**not installed or compile-tested**. Pin versions and lock resolution in the
first approved prototype step. No general application framework is added.

| Component | Selection, license, compatibility evidence | Reuse decision |
|---|---|---|
| Ratatui / Crossterm | Current pins 0.30.0 / 0.29.0. Prototype target 0.30.2 / 0.29.0; MIT; Rust ≥1.88. Official [installation matrix](https://ratatui.rs/installation/) pairs these versions. | Keep existing rendering/event foundation. Patch upgrade only with focused compile/terminal checks. One event reader/backend version. |
| Multiline editor | **ratatui-textarea 0.9.3**, MIT, MSRV 1.86; [manifest](https://raw.githubusercontent.com/ratatui/ratatui-textarea/main/Cargo.toml) uses ratatui-core 0.1.1/widgets 0.3.1. Maintained Ratatui fork; [API](https://docs.rs/ratatui-textarea/latest/ratatui_textarea/struct.TextArea.html) exposes selection, cursor, wrap, undo/redo. | Reuse rather than build Unicode selection/undo. Disable default features; map Crossterm events explicitly to editor methods. Do not import default shortcuts that conflict with Enter/Tab/Ctrl-J. Widget stays on UI thread. Verify soft-wrap cursor/selection behavior. |
| Markdown / code | **tui-markdown 0.3.10**, MIT OR Apache-2.0, Rust ≥1.88; [workspace](https://raw.githubusercontent.com/joshka/tui-markdown/main/Cargo.toml), [crate manifest](https://raw.githubusercontent.com/joshka/tui-markdown/main/tui-markdown/Cargo.toml) use ratatui-core 0.1 / pulldown-cmark 0.13. | Reuse with `default-features=false`. Headings/lists/inline code/fences; code gets readable static styling. Defer optional syntect/ANSI highlighting dependencies. Local width/scroll adapter and sanitized plain-text fallback; no custom Markdown parser. |
| Completion / panels | Ratatui List/ListState/Paragraph primitives, MIT; small fixed command catalog. | Local component is simpler than another form/completion framework. Deterministic prefix/substring search, not a fuzzy-search service. |
| Effects | **TachyonFX 0.23.0**, MIT; [versioned manifest](https://docs.rs/crate/tachyonfx/0.23.0/source/Cargo.toml) uses ratatui-core 0.1, tests with ratatui 0.30/Crossterm 0.29. Newer [docs](https://docs.rs/tachyonfx/latest/tachyonfx/) exist; 0.23.0 is the inspected compatible baseline, not a claim of latest. | Include `default-features=false`, `features=["std"]`; no effect DSL. Small accent-only adapter per §7, easy to disable. Don't build a local animation engine. |

The manifests suggest compatible Ratatui core types; they do not prove a combined
build. First step checks `cargo tree` for incompatible duplicate core/backend
versions and compiles editor/render/effect composition. If the selected versions
cannot coexist, report the exact dependency conflict; don't swap toolkit or
silently drop required editing behavior. Record Cargo.lock, compiler version
and bundled license notices. No dependency installation is authorized by this
document alone.

## 10. Scripted Project Decision Log session

Fixture provenance: explicitly selected approved `decision-log-01/revision-2.md`,
SHA-256 `021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`;
accepted **B / free in-place editing**; open alternatives-minimum findings
C1-2/C2-1, supersession-correction C1-3/C2-2 and cross-tab behavior C2-3.
Read through existing selected evidence when preparing a safe fixture copy;
record provenance, never alter historical approval or run. The actual app is
not built. No backup/retention-policy content belongs in this scenario.

| Scene | Evaluator interaction | Scripted response / evidence |
|---|---|---|
| 1 Empty session | Type/paste “Clarify the alternatives rule and produce a specification addendum. Keep free in-place editing.” | Pending→saved→scripted coordinator working→proposal. Show source approval as inherited, not reopened. Other input is retained and acknowledged as unsupported by the fixed scenario, not semantically interpreted. |
| 2 Review scope | Open visible Review scope; inspect selected records/settings; authorize deliberately. | Fake author/critic, deterministic model labels, no reasoning setting, ≤5 worker calls/2 critiques, separate fake coordinator count. No calls before authorization. |
| 3 Responsive progress | Keep typing; open tasks; submit “Prepare a manual walkthrough for this addendum.” | First task's simulated draft/review progress continues. Second request saves as queued for planning; not secretly authorized. Demonstrate authorizing its scope when coordinator slot becomes available, then queued for execution. |
| 4 Substantive question | Open question: zero alternatives allowed versus at least one; optionally try Other. | No preselected answer. Preserve it while navigating; explicit Save creates simulator-only answer record. Never ask editing policy again. Other is marked G1 preview. |
| 5 Lost reply | Save answer; script drops reply/connection once. Keep draft/history readable; type while reconnecting. | Same command ID resolves as saved after reconnect, one answer consumption, no duplicate stage. Display saved/unknown distinction, then resume scripted author. |
| 6 Candidate | Read addendum reflecting the evaluator's answer, inspect original decisions and unresolved topics. | Alternatives assessment derived from selected answer; supersession and cross-tab remain unresolved. If free text is not interpretable by fixture, quote it faithfully and keep alternatives unresolved too. Revision has no further independent critique. |
| 7 Review | Open candidate/plain text; inspect hash in details; Approve or Reject explicitly. | Exact local demo candidate binding, never source-run approval. Request changes explains unavailable contract without dispatch. Review panel remains usable across reconnect. |
| 8 Failure/recovery | While navigating second task, see a simulated missing-workspace notice and bounded reconciliation, then inspect a scripted uncertain outcome later. | Recovery reuses same worker identity/receipt without a new call. Uncertain outcome has retained evidence and no Retry; input stays responsive. No real Herdr API/process is involved. |
| 9 Detach/reopen | Ctrl-Q, choose Detach; relaunch same prototype directory. | Draft, selection, read position, simulator journal and demo decisions restored once. Frozen script pauses while executable is absent; explicitly **not** proof of durable real supervisor execution. |

Use deterministic event delays (e.g. 2–5 seconds) and a manual `Next scene`
control in **Prototype controls** for testing; gate on actions, not arbitrary
text matching or hidden auto-approval. Never progress past authorization,
question or final approval without evaluator input. A rejection follows a
stopped branch; a fresh explicit Reset demo makes a new evaluation directory.
No scene requires consenting to unresolved choices to inspect the rest.

## 11. Objective acceptance criteria

Automated checks and human observations must be reported separately.

| ID | Pass/fail criterion |
|---|---|
| UI-1 Layout | TestBackend snapshots at 80×24,120×36,160×48,60×18 show header/mode, target, editor and complete actions without overlaps. Resize preserves cursor and message anchor. Smaller size disables clipped mutations. |
| UI-2 Input | PTY sends Enter, Alt-Enter, negotiated Shift-Enter, arrows, combining/CJK text, undo/redo and bracketed multiline `/approve` paste. Only explicit submit sends; paste never invokes actions. F2/Tab/Send fallback works with enhanced keys disabled. Real Ghostty/Herdr key behavior still needs operator observation. |
| UI-3 Discovery | `/` shows filtered names/descriptions/shortcuts/reasons; selection completes without executing; Escape preserves input. Paths/code/unknown commands cannot authorize. All exposed actions usable without memorizing a command. |
| UI-4 Async | During ≥30 seconds of scripted events, typing/editing/menu/cancel-panel interactions render within 100 ms on the test host. Burst 10,000 updates stays within bounded queues/window; no lost critical events, scroll jumps or focus steals. Record timing method and observed max. |
| UI-5 Decisions | Choice/free-text preview, review, rejection and stale candidate cases all require explicit evaluator action. Replay of an answer/approval ID creates one demo record; candidate mutation invalidates approval eligibility. Human answers do not increment fake invocation counters. |
| UI-6 Reconnect | Drop acknowledgment after demo commit; history/draft retained; query same command resolves once. Exhaust attempts→actionable offline state; no new command ID or fabricated rejection. Relaunch restores journal/drafts without a new fake invocation. |
| UI-7 Target/isolation | Inspecting task 2 never retargets task 1's draft/answer. Unavailable steering explains why and preserves input. No production sockets, registries, processes or unrelated-project reads by prototype; fixture context includes only selected provenance. |
| UI-8 Failure | Simulated infrastructure recovery doesn't increment stage counter; uncertain scene never retries, clear diagnostics remain accessible, next input is still editable. Detail explains retained evidence. |
| UI-9 Theme/effects | Dark/light/mono snapshots, effects off/reduced, no color-only state. Fixed simulation clock proves effects finish within limits and idle produces zero redraws over 5 seconds; text readable on first frame. |
| UI-10 Terminal | Normal detach/error/panic/TERM restore terminal settings; paste mode/cursor restored. Copy view contains exact selected text. Human checks selection/copy, wrapping, long history, return from worker-preview and all three sizes. |
| UI-11 Scope honesty | Persistent SIMULATED labels; no real models, fabricated usage or browser-app success claims. Final report lists G1–G6 untouched and distinguishes fixture persistence from service durability. |

## 12. Bounded implementation handoff for Sol — after design approval

Implement a separate `waywr-ui-prototype` binary in `frontend/`, sharing only
presentation/terminal utilities needed by the production client. Keep current
`waywr` launcher/IPC/backend operational. Do not route prototype commands through
production IPC or add a production fallback to fixture data. Suggested small
modules: `presentation` (view model/reducer), `composer`, `commands`, `views`,
`theme_effects`, and `scripted_session`. This is a local UI boundary, not a new
workflow engine or generic application framework.

Use a typed presentation event/action enum. Scripted events carry local sequence,
task/version, simulation label and provenance references; their simulator journal
has its own namespace/schema. This journal is never read as production state.
The eventual IPC adapter must translate authoritative events, not teach the
presentation to read SQLite or LangGraph internals. Reuse Rust receipt/command-ID
concepts only where they model observable delivery; no duplicate backend policy.

1. **Component/editor proof.** Pin the chosen dependencies, lock/notice files;
   build separate binary with no service/worker access. Demonstrate editor,
   paste, multiline fallback and terminal cleanup before laying out a full
   session. Complete UI-2 and dependency compatibility check. Report conflicts
   before changing the selected toolkit.
2. **Presentation and command discovery.** Implement §2–4 layouts, focus,
   task-target drafts, scroll anchors, menu, read-only copy/inspect, themes and
   accent effects. Test snapshots and local input/state reducers (UI-1/3/9/10).
   No production colon-parser replacement in this step; prototype public UI
   uses slash commands exclusively.
3. **Single coherent scripted session.** Implement §10 with a supplied
   `--scenario decision-log --data-dir <isolated-directory>`, versioned safe
   fixture and deterministic scheduler/command journal. Add question/review
   panels, lost-ack replay, progress/queue/failure, detach/reopen. Use test-owned
   data only, no Herdr control APIs, Python child or model calls. Worker/Vim
   buttons open clearly labelled internal previews; actual integrations wait
   for G2/G3. Complete UI-4–8/11; record call/answer counters.
4. **Reviewable prototype handoff.** Build runnable binary into an ignored
   stable `.local/milestone6/ui-prototype/` location, document Fish commands and
   fresh evaluation data directory, preserve summaries/screenshots/logs. Supply
   a short evaluator guide matching the scenario and list unobserved terminal
   interactions. No global PATH/plugin/service changes. Commit bounded source
   changes and stop for operator evaluation before production wiring.

Future command shape (not installed or runnable as a result of this design):

```fish
./.local/milestone6/ui-prototype/bin/waywr-ui-prototype \
    --scenario decision-log \
    --data-dir .local/milestone6/ui-prototype/evaluations/first
# Reopen with the identical command; use a new directory for a fresh demo.
```

Help exposes local `subtle/reduced/off` effects and dark/light/mono choices.
The prototype runs in an ordinary terminal or a manually selected Herdr pane;
it never creates/rebinds workspaces. Nothing requires a native agent session.
No authenticated native acceptance is authorized. Service-stop reconciliation,
stable copies of earlier acceptance evidence, final-build missing-workspace
acceptance and production hands-on setup remain separate pending work; do not
claim this UI prototype satisfies them.

**Approval scope:** the four steps above, fake events only, then terminal
evaluation. No backend gap implementation, new provider, coordinator switching,
concurrent scheduling, automatic context discovery, web app, packaging redesign,
shared configuration change, merge or release. Routine design choices have
recommended resolutions here; there is no additional product decision blocking
review of this scoped prototype.
