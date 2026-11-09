# Scripted TUI prototype: hands-on guide

This is the **SIMULATED** Project Decision Log interaction prototype. It runs a
fixed Rust script with fake counters. It makes no Claude/Codex calls, starts no
worker, does not contact Wayward Wrangler's Python service or Herdr control
socket, and cannot alter the historical approved specification. The fake worker
preview is inside this UI; there are no new Herdr worker panes to inspect.

The stable binary is under `.local/milestone6/ui-prototype/bin/`. The scripted
journal is in the `--data-dir` you choose. The binary works from a different
working directory; the commands below derive absolute paths from the repository.
Use a terminal at least 80×24 for the main evaluation. Running it in one of your
existing Herdr panes is fine; it neither creates nor changes a workspace.

## Launch, reopen and inspect (Fish)

From the Wayward Wrangler repository root, run:

```fish
set -l ww_root (git rev-parse --show-toplevel)
set -l ww_ui "$ww_root/.local/milestone6/ui-prototype/bin/waywr-ui-prototype"
set -l ww_demo "$ww_root/.local/milestone6/ui-prototype/evaluations/operator-04"
"$ww_ui" --scenario decision-log --data-dir "$ww_demo"
```

After detaching, run the same last command to reopen the same scripted session.
In a new Fish process, repeat the three `set` lines first. A fresh evaluation
uses a different `--data-dir`, such as `operator-04`; it does not reset or
overwrite the first journal.

To inspect the saved demo after detaching:

```fish
vim -R "$ww_demo/scripted-journal.json"
```

The journal records local simulated events, task versions, the selected source
hash, question answer, exact demo candidate hash, fake invocation counters and
UI drafts. It is **not** a production run record. The approved source remains
`temp/project-decision-log/.wayward-wrangler/state/runs/decision-log-01/`.

## Walkthrough

1. Type `Clarify the Decision Log alternatives rule; keep the approved free in-place editing choice.` in the shaded composer after **›**, then press **Enter**. Delivery changes in the task summary from **Sending** to **Request saved**; it does not create separate conversation messages. Wait for the scripted coordinator suggestion (about three seconds per scene), or use **F8** to advance. The suggestion is not worker authorization.
2. Press **Tab** four times from the composer to focus **Create task proposal**; its highlight and **Enter opens a proposal** hint should be visible. Press **Enter** to open the scope panel. This creates a proposal but does not authorize execution. The panel shows the selected approved source, unresolved findings, fake role settings and call limits. Press **Tab**, then **Enter** on **Authorize simulated work** only if you choose to run the fake stages.
3. Let the fake draft/review stages advance (or use **F8**). While they progress, press **F4**, **Down**, **Enter** to choose **New task**. Type `Prepare a short manual walkthrough for the addendum.` and press **Enter**. The second task has a separate proposal and later queues behind the first; it is not created by every coordinator message. At 80 columns, **F6** opens tasks: **Down**, **Enter** selects the second task. Focus its activity action with Tab and review/authorize its scope separately. F6/Up/Enter returns to the first task.
4. When the first task says **Your answer needed**, focus its activity action and press **Enter**. The question is whether a decision may have zero alternatives. Press **1** for `Allow none`, **2** for `Require one`, or **3** and type an Other answer. No option is preselected. **Tab**, **Enter** saves the answer. Free-form text is saved and quoted without model interpretation. The simulated reply is lost after the answer is saved, then the script reconciles the same command ID. Your draft stays available. This answer is not a model call.
5. At **Final review ready**, open the visible Review action. **Tab**, **Enter** reads the exact local candidate. It quotes your answer and still lists supersession correction and cross-tab behavior as unresolved. **Enter** returns to review, then **Tab**, **Enter** approves this *demo candidate* if you choose. Reject requires a reason. “Request changes” is visibly unavailable and never silently starts another call. The historical approval is untouched.
6. If the second task was authorized, let it advance or press **F8** through Active → Recovering → Recovered → Uncertain. The fake call count stays one during infrastructure recovery, and uncertainty offers evidence inspection without a Retry button. **F7** opens a labelled *fake worker preview*. It does not focus a real worker pane.
7. Press **F3** to inspect the coordinator-settings panel. The active coordinator is scripted. Claude/Codex provider/model/effort rows are labelled examples. Fallback and provider switching are unavailable. `t` cycles dark/light/mono; `e` cycles subtle/reduced/off effects. Neither key launches work.
8. Press **Ctrl-Q**, then **Tab**, **Enter** to detach. Reopen with the identical command above. The demo journal, answers, review state, scroll anchor, task selection and unsent drafts should remain.

Useful controls: `/` at the start of an empty composer opens a searchable
command menu; Up/Down selects, **Enter acts immediately**, Tab completes the
command text without acting, and Escape dismisses without losing text. `/help`,
`/settings` and `/tasks` open the same panels as their function keys. `/plan`
opens a proposal, but does not authorize it. The menu shows unavailable
reasons. **F1** is contextual help; **PageUp/PageDown** scroll conversation;
**Ctrl-R** recalls sent message text into the draft without resending it.
**F6** toggles the drawer at ≥110 columns and opens a task overlay below that.
**F4** changes the explicit composer target. Tab/Shift-Tab cycle focus.

**Enter** sends ordinary composer text. **Shift-Enter** inserts a newline when
the terminal reports the modifier distinctly. The prototype requests enhanced
keys for its own lifetime and restores the prior protocol on exit. **F2** toggles multiline:
then Enter inserts newlines and Tab to **Send**, Enter submits even without
enhanced keyboard support. Bracketed paste inserts literal text, including a
leading `/`, without executing a command. The UI keeps mouse capture off so
Ghostty can select and copy rendered text. Its artifact reader styles Markdown and
the supported Rust, Python, JSON and shell code fences;
Vim/worker routing through Herdr is a future production integration.

### Shift-Enter in Ghostty and Herdr

The operator confirmed that **physical Shift-Enter inserts a newline in the current Ghostty/Herdr setup**. This is a hands-on observation, separate from the CSI-u PTY check; it does not establish the behavior on every terminal or Herdr configuration. The earlier read-only tmux `extended-keys off` result and legacy-sequence probe remain historical diagnostics, not a reason to override the observed key behavior. No shared setting was changed. If another terminal cannot report Shift-Enter distinctly, use **F2**, Enter for newlines, then Tab to **Send** and Enter.

## What to observe

Please check the new blinking cursor, left-aligned input, `> ` user-message
block, grouped coordinator/action results, title animation, slash Enter/Tab
behavior and background colors at 80×24, 120×36 and 160×48 in Ghostty/Herdr.
At 160×48, conversation content should fill the terminal width. An open drawer
should limit it only at the drawer boundary.
Also check paste and text selection; wrapped history and unread marker while
scrolled up; effects subtle/reduced/off; and terminal restoration after detach.
Automated PTY and TestBackend checks cannot establish your terminal's focus,
selection, contrast or modified-key behavior.

The executable and automated evidence are retained under
`.local/milestone6/ui-prototype/`. To remove only your chosen evaluation after
you finish inspecting it:

```fish
rm -r -- "$ww_demo"
```

Leave the stable `evidence/` directory intact for implementation review.
