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
| Main composer | Up/Down recalls history. Alt+Up/Down moves the multiline cursor. Enter sends; F2 toggles multiline. Tab/Shift-Tab changes focus. |

Esc closes a popup only after its nested picker/editor has been left. Page and
Escape handling must not bypass the focused editor. Holding navigation keys
repeats movement; holding Enter must not advance approvals or repeatedly apply
settings. Modal footer hints must match actual behavior, including all sections.

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
model changes the draft only. S persists it; coordinator switching and exact
context/policy grants remain explicit in their existing sections. Node/runtime
and JSON option fields remain available without provider-specific key bindings.

F1 and popup footers provide contextual help. This contract standardizes the
existing Rust/Ratatui interface; it does not authorize operator services, provider
login or inference. See [provider setup](provider-selection.md) for those steps.
