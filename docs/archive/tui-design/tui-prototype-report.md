# Scripted UI prototype — current evaluator

**Later integration:** The accepted renderer/editor now also drives installed
`waywr`; scripted scenario transitions remain confined to the explicit
evaluator binary. See the [unified UI integration report](../milestone-6/unified-ui-integration-report.md).
The operator observation below still applies to the earlier scripted
evaluation, not to unobserved physical typing in the new installed build.

## Operator acceptance and follow-up refinement (2026-10-11)

**Latest operator observation:** The operator verified and accepted the final
scripted UI refinement, including its full available conversation width. This
is hands-on acceptance of the isolated presentation, not an automated test or
acceptance of production integration, lifecycle behavior, or native agents.

**Operator layout correction:** Conversation content must use the full width
available in the terminal. An open task/worker drawer defines the pane boundary;
there should be no arbitrary text-width cap. The renderer previously used
`min(96)` and left unused space on wide terminals. It now fills the terminal,
or the conversation pane beside the drawer. A TestBackend check verified all
160 columns at 160×48 with the drawer closed and 115 columns with it open.
This records the requested behavior, not a new hands-on acceptance of the
rebuilt screen.

**Operator observation, separate from automated verification:** In the current
Ghostty/Herdr setup, physical Shift+Enter inserted newlines; the composer was
clearly distinguished and input no longer underlined; slash-menu descriptions
were readable; task/proposal grouping, indentation, colors and actions looked
correct. The operator also confirmed F2 behavior and its existing footer
explanation, which was left unchanged. This accepts the prior scripted UI
visual/input refinement only. Production integration and native-agent behavior
remain unaccepted.

The follow-up evaluator has a blinking composer cursor, left-aligned text
starting close to the prompt, dark composer background `#41454C`, and a
color-coordinated **Wayward Wrangler** title with one short startup effect in
subtle mode. Reduced/off modes do not schedule that effect. A user message is
one `#4A4D54` dark-theme block starting `> `, with subsequent lines aligned
under the text; the separate “You” heading is gone. Coordinator and action
messages use one bullet heading and one `└` result marker, with wrapped and
multiline continuations aligned beneath the result. Light and monochrome
themes retain their own readable styling. Slash-menu Enter now invokes the
selected command's existing handler; Tab only completes its text. Proposal,
question and review actions still preserve explicit gates. Pasted `/` text
stays literal. No `/exit` or resume lifecycle behavior was added.

**Automated results for the final isolated build:** 32 focused Rust tests
passed; one existing 30-second latency probe remained intentionally ignored.
The exact release binary passed 12 isolated PTY sessions at narrow/wide sizes,
including CSI-u Shift+Enter, F2 multiline, literal pasted slash text,
`/settings` and `/tasks` Enter actions, draft reopen, detach and SIGTERM
restoration. The idle blink produced 52–53 bytes in a 700 ms observation
window, rather than a continuous repaint. See
[PTY summary](../../../.local/milestone6/ui-prototype/evidence/refinement-release-07/pty-summary.json).
The byte-verified stable executable is
`.local/milestone6/ui-prototype/bin/waywr-ui-prototype`, SHA-256
`2299064937232ec252e5316fedc2d4a8b4db79c4a66b083dbb4aec228e2a0244`.
The previous evaluator and manifest were copied and byte-compared under
`evidence/build-ed7db2af/`; the earlier `build-dcd00591/`, journals and
other evidence were preserved.
All PTY processes exited. Ghostty/Herdr appearance of these **new** changes
still needs operator observation.
An initial PTY assertion window ended before the 550 ms cursor tick, and a
settings-panel assertion assumed ANSI redraws kept full phrases contiguous;
the focused harness was corrected and the final run passed. `cargo fmt --check`
reported formatting differences across the broader Rust crate, including
untouched files; no mass formatting change was made for this isolated UI
refinement.

The proposed production session exit/detach/resume contract is in
[session-lifecycle-proposal.md](../session-lifecycle/session-lifecycle-proposal.md). It is design
only. The pending production evidence preservation, paired installation,
stop-versus-detach correction and recovery checks remain open.

## Prior refinement evidence (superseded evaluator)

The isolated Rust evaluator was refined on `milestone-6-interactive-shell` after
the operator's screenshots and hands-on feedback. The then-current binary was
`.local/milestone6/ui-prototype/bin/waywr-ui-prototype`, SHA-256
`dcd00591571edf25375cf3c02ea568d8e81183193fd6599d440f38435ab987c4`.
Prior evaluators and manifests are preserved in
`.local/milestone6/ui-prototype/evidence/build-17170b8a/` and
`evidence/build-5556f2f7/`; no earlier journal
or capture was changed. The source and current evidence hashes are in the
ignored `manifest.json`.

The composer now has a contrasting padded surface, a **›** prompt, height that
grows with the draft, plain input text and a distinct reversed cursor cell.
`ratatui-textarea` underlined the cursor line by default; explicitly clearing
that style fixes the observed underlining while retaining selection, undo,
paste and saved drafts. Enter submits and Shift-Enter inserts a newline when a
modified Enter event arrives. The app requests CSI-u disambiguation only for
its own terminal lifetime and pops it on detach/error. F2 multiline mode remains
the fallback. Alt-Enter is no longer a newline shortcut.

The conversation presents **You**, **Coordinator**, and compact labelled
activity, with delivery transitions updated in the task summary. Sending and
request-saved transitions remain in the journal but no longer appear as two
conversation messages. The activity area groups task progress, artifact and
the focusable next action; opening a proposal still does not authorize it.
The slash picker reserves a wrapped explanation for the selected command.
A small local formatter styles paths, numbered choices, inline code, lists and
Rust/Python/JSON/shell fences; user text stays plain unless explicitly marked
up. Dark/light/mono and subtle/reduced/off effects remain available.

**Automated verification:** 27 focused prototype Rust tests and five existing
production Rust tests pass; one earlier 30-second latency probe remains
intentionally ignored in the normal suite. The final release binary passed ten
source-free PTY sessions, including 60×18 and 160×48 sizes, CSI-u Shift-Enter,
literal pasted `/approve`, F2 multiline, saved multiline draft reopen,
detach/reopen, and SIGTERM. Terminal modes and keyboard protocol were restored
in each; measured idle repaint bytes were zero. See
[refinement PTY summary](../../../.local/milestone6/ui-prototype/evidence/refinement-release-02/pty-summary.json).
The changed renderer was also checked at 80×24 and 120×36 with Ratatui's
TestBackend. A first debug PTY attempt missed final SIGTERM output because its
harness stopped reading on process exit; the corrected harness and final release
run passed. Failed harness evidence remains in `evidence/refinement-debug-01/`.

**Terminal limitation:** the installed Ghostty is 1.3.1 and advertises the
[Kitty keyboard protocol](https://ghostty.org/docs/features). The PTY proves
the app accepts CSI-u Shift-Enter and balances the app-local push/pop request;
it does not prove physical Ghostty or Herdr delivery. The current Herdr session
runs tmux 3.4 with `extended-keys off`; [tmux documents](https://github.com/tmux/tmux/wiki/Modifier-Keys)
that modified-key forwarding depends on the multiplexer path. An isolated PTY
probe of Ghostty's legacy modified-Enter sequence yielded joined text, not a
newline. The actual Ghostty → Herdr key path needs the operator check in the
[guide](tui-prototype-guide.md). No shared setting was changed.

This is still a fixed scripted scenario. It makes no model calls, contacts no
Herdr control or Python service, and launches no workers. Existing production
UI/backend code, historical approvals and artifacts remain untouched. All
test-owned processes exited. The prior milestone-6 evidence preservation,
paired installation, stop lifecycle and recovery checks remain pending in
[HANDOFF.md](../../HANDOFF.md). Native calls remain blocked.

## Initial prototype evidence (superseded evaluator)

The following records describe the earlier evaluator archived under
`evidence/build-17170b8a/`. Its Alt-Enter behavior and component list are
historical; use the current refinement and guide above for evaluation.

**Scope:** the four-step isolated Rust prototype in
[tui-interaction-spec.md](tui-interaction-spec.md), approved at `c100c70`.
This report concerns the scripted UI only. The production `waywr`/Python
workflow, historical approved runs and shared Herdr resources were not used by
the prototype. No authenticated calls or worker/service launches occurred.

## Built

The initial `waywr-ui-prototype` binary was a Rust/Ratatui/Crossterm
alternate-screen UI with a `ratatui-textarea` composer, `tui-markdown` content
rendering, a local slash menu and TachyonFX accent effects. It has semantic
dark/light/mono colors and subtle/reduced/off effects. Enter submits, F2 plus a
visible Send action is the multiline fallback, and bracketed paste stays
literal. Questions, scope authorization, exact-candidate review, task
navigation, coordinator settings and detach have visible actions. Plain
conversation, a New task request, unavailable active steering and deterministic
commands have distinct targets.

The fixed Project Decision Log script inherits the real approved revision-2
SHA-256 `021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`
(reverified against the actual file). It keeps accepted free in-place editing,
asks about the alternatives minimum, carries supersession/cross-tab issues as
unresolved, and creates a separate local demo candidate. A second walkthrough
task demonstrates a queue and recovery/uncertain notice. Its per-task local
journal is versioned and stores simulated event sequence, provenance,
invocation counters, answer and reviewed candidate digest. The scripted
answer uses no worker call. Approval binds to actual candidate bytes and is
refused after mutation. Closing/reopening the UI does not replay an accepted
demo action.

The header's Settings entry describes the active scripted coordinator and
clearly labelled illustrative primary provider/model/effort and worker role
profiles. Future fallback policy, limit-reached/switch states and provider
switching are labelled unavailable. Settings controls only theme/effects; no
settings action can contact a model.

The then-stable executable was
`.local/milestone6/ui-prototype/bin/waywr-ui-prototype`, SHA-256
`17170b8a2af03ab9eaa332e0d5f92def7f910afb92abf78f3de90e0404a93343`.
`manifest.json`, `Cargo.lock`, notices and automated evidence are beside it.
Selected prior prototype binaries are retained under `evidence/build-ed56c41/`,
`evidence/build-6208e028/` and `evidence/build-10e34661/`. The final evaluator binary was copied from the optimized
build and byte-compared; its PTY process started with `/` as cwd and did not
import source files or use checkout resources beyond the binary under `.local`.

## Automated verification

| Check | Result / evidence |
|---|---|
| Component compatibility | Ratatui 0.30.2, Crossterm 0.29, textarea 0.9.3, Markdown 0.3.10 and TachyonFX 0.23 resolved and compiled. `cargo tree -d` showed one Ratatui core and one Crossterm. Rust 1.99 in isolated `/tmp` toolchain; `Cargo.lock` pins versions. |
| Focused Rust tests | 19 prototype tests pass; one 30-second load probe is intentionally ignored in the normal suite and passed when explicitly run. Existing production client/library tests: 5 pass after dependency resolution. Tests cover four screen sizes, menu/paste, history, input, scope/question/review, candidate binding, source provenance, cancellation isolation, restart, 10,000 bounded updates, scroll anchoring and effects. |
| PTY, initial evaluator | Five isolated sessions passed from `/`: paste/resize/normal detach, reopen, F2 fallback, Alt-Enter and SIGTERM cleanup. Terminal settings restored in each. Source-free reopen retained one coordinator fixture call. After the first frame, measured idle output was zero bytes. See [initial PTY summary](../../../.local/milestone6/ui-prototype/evidence/final-pty-06/pty-summary.json); raw captures and journals are beside it. |
| Responsive input | [30-second probe](../../../.local/milestone6/ui-prototype/evidence/final-latency-30s.log): 1,924 scripted input/render samples; maximum 18.90 ms on this host, under the proposed 100 ms bound. Fake progress updated while input continued. This measurement preceded a scope-panel wording edit; the final binary passed focused tests and PTY afterward. This is a TestBackend measurement, not a Ghostty latency measurement. |
| Effects | A fixed-time test confirms a subtle accent finishes, reduced/off modes schedule none, and an idle state schedules no redraw. Real terminal color and motion still need operator observation. |
| Source scope | Prototype source does not open production IPC, invoke Python/Herdr, spawn agents or read the global project registry. Only the explicit demo data directory is writable at runtime. The source specification hash is an embedded provenance reference, not a runtime dependency on its historical path. |

The visual-line count used to hold a conversation message in place while
scrolling comes from Ratatui's `unstable-rendered-line-info` feature. It passed
focused resize/new-event tests at the pinned version; a future Ratatui upgrade
must recheck that API and wrap behavior before changing the pin.

The first source-free PTY harness attempt failed because the harness passed a
relative data directory while setting `/` as cwd. The harness was corrected to
resolve that directory first. Its empty `evidence/stable-pty-01/` directory is
retained; all subsequent PTY runs passed. This was a test harness path error,
not a production or prototype execution retry.

## Still unproven or deferred

- Ghostty/Herdr readability, selection/copy, modified keys, focus and effects
  need the operator walkthrough in [tui-prototype-guide.md](tui-prototype-guide.md).
  Alt-Enter passed a legacy PTY sequence; Shift-Enter is not advertised without
  actual enhanced-key verification. SIGTERM/detach restoration passed; forced
  SIGKILL and panic cannot be claimed to restore terminal settings.
- Worker and Vim actions show labelled local previews. The prototype never
  creates/focuses a Herdr worker pane or runs Vim. The prior Herdr-visible
  production foundation is separate evidence. Reconnect here is a local
  scripted command/journal exercise, not the real service's uncertain-command
  settlement or worker reconciliation.
- The coordinator's responses are fixed fixture responses, not general
  conversation or model-backed interpretation. Free-form Other answers are
  quoted but not semantically interpreted. The approved editing decision is
  inherited; the demo does not reopen historical approval.
- The production UI still has colon commands and its earlier editor. G1–G6 in
  the [interaction spec](tui-interaction-spec.md) remain production integration
  gaps: free-form backend questions, read projections, Herdr display routing,
  general conversation/steering, stop lifecycle, and delivery settlement.
  Native context/tool isolation remains unverified and native calls blocked.
- The previously requested preservation of older `/tmp` milestone-6 evidence,
  stable paired installation, final-build missing-workspace check and
  stop-versus-detach lifecycle fix remain pending as recorded in
  [HANDOFF.md](../../HANDOFF.md). This UI-only prototype does not satisfy those tasks.

All PTY child processes exited; no test-owned service or Herdr resource is
retained by this prototype. The stable binary, manifests, fixture journals and
captures remain in ignored `.local/milestone6/ui-prototype/` for inspection.
