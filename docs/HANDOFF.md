# Waywr handoff — mouse-wheel scrolling

Branch: `plan/usability-and-installation`. Application checkpoint: `cb094be`
(mouse wheel); current checkpoint records the pre-push review on 2026-11-11.
Follow [AGENTS.md](../AGENTS.md). Local commits only; the user handles remote
writes. No operator project/service/binding/provider configuration/credentials,
global installation or remote Git state was changed. No account/model call.

## Implemented outcome

Mouse wheel over the conversation scrolls three lines per notch using the existing
anchor/follow-latest behavior. New output preserves the reading position; scrolling
to the bottom resumes following latest. The draft, editor cursor, keyboard focus,
history and execution actions remain unchanged. Wheel input over controls/editor
or outside a popup is ignored rather than scrolling its background conversation.

Over a popup, wheel input scrolls its bounded content. The provider/model/thinking
picker moves its highlighted entry through the existing list navigation without
applying/saving it or changing its search query. F3 form scrolling disables focus
following until keyboard navigation resumes. Popup titles/footers remain fixed.

Production and scripted terminal guards enable Crossterm mouse capture and disable
it on leave/startup failure alongside raw mode, paste and alternate-screen cleanup.
Only vertical wheel events have actions; click/motion events do not select, change
focus or request redraw. Hit regions are the last rendered conversation/popup
rectangles, refreshed each render and cleared for a too-small display. No backend/protocol/dependency
change or new UI architecture was added. Terminal selection may require Shift.

## Evidence and limits

Behavioral red preceded implementation: wheel events left both conversation and
popup content stationary. Focused regressions now cover chat scroll/anchors/new
output/follow-latest, draft/focus retention, ignored outside regions, popup bounds
and model-list movement without applying. The real isolated PTY/scripted-TUI check
sends SGR wheel bytes through Crossterm, verifies persisted scroll up/down and
retained unsent draft, and checks mouse-capture disable plus exact terminal-mode
restoration on exit. It starts no service or account operation. This is the actual
terminal/input/view path, not proof of desktop trackpad behavior in every emulator.

The remapped paired release, public-content source/binary/bundle gates, relative
references and diff checks pass. CI/release checks are prepared locally; no hosted
workflow was run. Broader service/model campaigns were not repeated for this UI-only
change. The ignored target/local-bin/waywr points to the temporary paired candidate;
target/local-bin/candidate.json records its actual source/artifact identity.

## Existing behavior and next action

`5ebbb15` keeps the read-only event subscriber recovering after busy/error replies,
transport loss, noncontiguous pages and exhausted snapshot retry batches. Recovery
requires the verified owner and monotonic coherent snapshot; it never resends a
command/call. Disconnected cached work has no active elapsed timer. Read-only
operator receipts had confirmed completed model calls/usage behind the stale UI;
the initial transient failure itself was not retained in inspected metadata.
`1e09f59` adds /login and F3 G with SDK-advertised methods, private masked entry,
browser/manual callback handling and sanitized exchange/refresh diagnostics.
`2c6451a` adds Enter steering, separate Alt+Enter follow-ups, F7/E restore, multiline/
history and direct model/thinking/status controls. Direct interrupt/leave/pause,
renewable obsolete sessions and compatible busy-owner attachment remain. Preserve
source/Git/worktrees, exact execution ownership, approval boundaries and no replay;
keep model/provider catalogs/options outside core routing.

The user reports the current application looks good. No further local usability
audit fix is outstanding before branch review. Pre-push privacy checks cover the
current 459 source paths, changed blobs/paths in 28 commits since the locally
recorded origin/main, the three release binaries and19 bundle files. Gitleaks 8.30.1
reports zero source/history findings with the existing exact synthetic-fixture
fingerprint exclusion. Commit metadata contains public GitHub username/noreply
attribution or neutral example attribution; no private email/machine address was
found. Public GitHub attribution remains repository metadata and is
not compiled application content. [Audit record](security-audit-2026-11-08.md)
records scope and limits. The application candidate remains the tested cb094be;
this checkpoint changes documentation only.

Next: the user pushes this branch and opens/reviews its PR. Hosted build, current
dependency-advisory, secret and CodeQL checks must pass before merge; they have
not been queried or run remotely by the agent. The local origin/main reference
was inspected without fetching. No new publication/global installation or paid
feature is implied. Useful task delivery, broader account/model quality/refresh,
public installer/update/uninstall, workflow-wide usage and the substantial
application remain later product work, not a reason to defer this branch review.

Owners: [controls](tui-controls.md), [live input](live-input.md),
[providers](provider-selection.md), [plan](usability-plan.md),
[user walkthrough](verify-usability.md).
