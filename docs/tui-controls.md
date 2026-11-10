# TUI keyboard contract

Controls follow the focused component. Keep the applicable keys visible in a
fixed footer, keep the focused row/field in view, and give an explicit empty state
instead of silently truncating results. New panels must follow this contract.
Labels describe local draft changes separately from durable saves, context grants
and execution approvals. Browsing never grants authority or executes work.

| Context | Controls |
| --- | --- |
| Settings form | Up/Down moves fields or rows. Tab/Shift-Tab moves forward/backward. Left/Right chooses a profile or changes a discrete project setting. Enter chooses/edits the focused field. |
| Settings editor | Arrows/Home/End move the text cursor. Enter accepts into the draft. Tab/Shift-Tab accepts and moves focus. Esc cancels only the edit. Invalid JSON stays in the editor with a visible error. |
| Model/provider picker | Type or paste to filter by provider, model ID or name. Up/Down or Tab/Shift-Tab moves the highlighted result. PageUp/PageDown moves a visible page. Home/End selects first/last. Enter explicitly chooses. Esc returns one level. |
| Read-only content popup | Up/Down scrolls a line; PageUp/PageDown scrolls eight lines; Home/End goes to the start/end. Alt+PageUp/PageDown moves one line. |
| Modal action row | Left/Right or Tab/Shift-Tab selects an action. Enter activates it. The initial choice stays non-destructive. |
| Existing task, queue, question and history lists | Up/Down selects a row. Actions remain in their own action row. Page keys scroll displayed content. |
| Main composer | Up/Down moves within multiline input and recalls history at its first/last line. Ctrl-J or Shift+Enter inserts a newline; F2 is the fallback. Enter sends idle input or steers an active coordinator; Alt+Enter queues a follow-up. Ctrl-C/Esc clears nonempty input or interrupts with empty input. Tab/Shift-Tab changes focus. |
| Conversation queue | F7 opens pending/applied steering and queued/held follow-ups. Up/Down selects; E withdraws/restores a pending input for editing. Applied/claimed inputs cannot be edited. Restore never sends. |
| Leave | Ctrl-Q from any overlay/editor; Ctrl-D with empty input; `/detach`, `/exit`, `/quit`. Save local draft and leave immediately without confirmation or settlement wait, including disconnected state. |
| Interrupt / pause | Visible Interrupt action or `/interrupt` stops only the foreground coordinator directly. Show Stopping until settlement; typing and leaving remain available. `/pause` separately stops new dispatch; `/resume` continues it. |

Esc closes a popup only after its nested picker/editor has been left. Page and
Escape handling must not bypass the focused editor. Holding navigation keys
repeats movement; holding Enter must not advance approvals or repeatedly apply
settings. Modal footer hints must match actual behavior, including all sections.

Clearing resets history recall and the saved unsent draft without removing history
entries or cancelling work. Escape in a popup/editor still returns one level and
preserves the composer underneath. When a retired binding needs setup, Send opens
the saved model activation review directly. Its Keep current and Escape actions
return to the composer with the message intact; confirming activation also returns
there, leaving the message unsent until the user presses Enter again.

## Settings and model selection

F3 opens settings. Sections1–7 select Profiles, Roles, Fallback, Switch, Tasks,
Policy and Project. S saves the current preferences/project draft, L reloads,
R refreshes public runtime metadata. Section changes reset content scrolling.

In Profiles, F or `/` opens all catalog models, V opens providers and C opens
models for the current provider. Enter on provider/model fields opens the same
pickers. E edits an exact ID or value directly, including IDs absent from the
catalog. `+` duplicates a profile. Provider results include registered providers
with zero catalog models; a zero-result model list explains manual entry.

The Thinking field opens levels supplied by the selected model's runtime
metadata, using the same picker controls. F3 pins draft and active model/level
labels; the main header shows the saved coordinator choice first and any different
active binding with its readiness state. Choosing or saving a different
level does not switch the active coordinator or expand its context grant.

The full filtered catalog is navigable; only the visible viewport is rendered.
Counters distinguish total matches from the highlighted position. Selecting a
model changes the draft only. S persists it; A in Profiles reviews activation of
the selected saved profile. The review pins the target model/level and defaults
to Keep current. Left/Right, Up/Down or Tab/Shift-Tab selects the action;
Enter confirms on key press only. Activate and share project context requests
an after-turn switch with an explicit context grant. Esc returns to the form;
PgUp/PgDn and Home/End scroll the details while actions remain visible. A changed
saved revision or binding invalidates the review. Section4 retains advanced
switch/status/cancellation controls; section6 retains policy amendments. Node/runtime
and JSON option fields remain available without provider-specific key bindings.

F1 and popup footers provide contextual help. This contract standardizes the
existing Rust/Ratatui interface; it does not authorize operator services, provider
login or inference. See [provider setup](provider-selection.md) for those steps.

## Planned everyday interaction

Target recorded on2026-11-10, following the user's request to align ordinary use
with Codex/Pi and finish it before another operator walkthrough. This section is
implemented locally for interrupt/leave/pause, steering/follow-up, multiline
history/newlines and direct model/thinking/status actions. The useful operator
walkthrough remains separate evidence. A waiting
task must not make the interface unusable. Drafting, browsing, inspection, changing
topics and leaving remain available while an operation is waiting or stopping.

### Reference observations

Codex documents Enter as steering during work, Tab as a queued follow-up, prompt
history/search, slash-command discovery, model/status controls, keymap editing,
saved-session resume and direct exit. Its keymap is contextual. See the official
[interactive shortcuts](https://learn.chatgpt.com/docs/developer-commands?surface=cli#interactive-shortcuts),
[keymap command](https://learn.chatgpt.com/docs/developer-commands?surface=cli#remap-tui-shortcuts-with-keymap)
and [resume/exit commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli).

Pi documents Enter for steering, Alt+Enter for follow-up and Escape for abort;
its editor supports newline/history actions and restores queued input after abort.
Its configurable named actions separate interrupt, clear and exit, with model and
thinking shortcuts. See [terminal usage](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/docs/usage.md),
[keybindings](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/docs/keybindings.md)
and [action definitions](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/src/core/keybindings.ts).
These are upstream observations checked on the date above, not a claim that every
shortcut or delivery boundary is identical across versions and terminals.

Waywr uses the shared libraries, not Pi's application. The pinned agent class
provides steer, followUp and abort. The [runtime entry](../model-runtime/main.mjs)
now claims an attempt-scoped inbox between agent steps and calls Agent.steer.
Follow-ups remain separate service turns, each with its own validated result.
See [live input](live-input.md) for pending/applied/held state, editing and replay
boundaries. An older owner without this capability queues until idle handover.

### Target actions and precedence

| Action/context | Target behavior |
| --- | --- |
| Enter while idle | Send the draft. Required setup offers the relevant action in place and keeps the draft. |
| Enter while the current agent works | Deliver steering at the next supported agent-loop boundary; show pending versus applied. It does not silently expand approved work. |
| Alt+Enter while busy | Queue a follow-up for after the current task. Queue controls can edit, withdraw or restore it without sending. |
| Escape in a popup/editor | Return one level. Do not clear the underlying composer or interrupt unrelated work. |
| Escape in the composer with text | Clear the draft, honoring the user's requested behavior. |
| Escape with empty composer and active agent | Request interruption immediately, preserve input/work, show Stopping until actual settlement, and keep the UI usable. |
| Ctrl-C | Clear nonempty composer input; with empty input and an active agent, interrupt. Empty idle input does not implicitly terminate work or exit. |
| Visible Interrupt action; `/interrupt` | Stop the foreground coordinator without a confirmation panel. A selected-worker stop must name that worker; it must not reuse whole-workflow cancellation implicitly. |
| Ctrl-Q, Ctrl-D with empty input, `/detach`, `/exit`, `/quit` | Save local draft and leave the UI promptly without confirmation or waiting for model/tool settlement. Active service-owned work continues; this is detach, not pause. |
| `/pause` | Stop new dispatch for the selected session/work; report current-call settlement separately. Keep typing, inspection and leave available. `/resume` explicitly continues a pause. |
| Shift+Enter or Ctrl-J | Insert a newline where terminal reporting permits. Retain F2 as a fallback. Up/Down moves within multiline input; history is recalled at the editor boundary. |
| `/model`, `/thinking`, `/status`, `/help` | Direct discoverable actions; F3 and existing navigation remain available. Show active, saved and pending choices truthfully. |
| Waiting/error | Show the affected task/call, concrete reason and available action. Preserve the draft; allow another topic or leaving. An unrelated request must not lock the whole application. |

The Escape/text and Ctrl-C choices deliberately retain the user's clear-buffer
preference rather than copying Pi's precedence wholesale. Alt+Enter avoids taking
Tab away from Waywr's existing focus controls. Key handling and help should share
one action definition; configurable bindings can follow without delaying the
everyday loop. Model/provider behavior must remain below this interaction layer.

The former exit confirmation/pause coupling is removed. /exit and Ctrl-Q leave
promptly; /pause is separate. The controls above are implemented locally, with
the useful operator walkthrough still required for ordinary-use evidence.

Interruption is a request to stop one operation. Uncertain settlement still fences
replay, replacement or conflicting writes for that operation; it cannot prevent
closing the UI. Whole-workflow cancellation and destructive resource removal are
separate explicit actions. Neither detach nor a new message discards prior work.
