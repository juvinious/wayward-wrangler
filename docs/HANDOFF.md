# Waywr handoff — recover live updates without replay

Branch: `plan/usability-and-installation`. Base: `1e09f59` (in-app sign-in).
Follow [AGENTS.md](../AGENTS.md). Local commits only; the user handles remote
writes. Operator inspection in this slice was read-only: process/unit status,
SQLite event/turn metadata and sanitized attempt receipts. No operator process,
service, binding, configuration, credential or source was changed; no request
was restarted or resent. Tests use isolated projects/Unix sockets/local HTTP.

## Observed problem and implemented outcome

The user reports successful sign-in and responsiveness, followed by “Last observed
coordinator call” with unknown usage and an elapsed “Coordinator working” display.
Read-only live records confirmed completed shared-runtime calls, completed host
receipts and usage events, with no reserved conversation turn at inspection.
The UI's event worker had ended while its command worker remained. The display
was cached; these observations do not establish completion of useful development
work or explain the first transport interruption.

A source regression reproduces a concrete cause: a valid temporary busy/error
reply was treated as a fatal event-sequence gap, permanently stopping updates.
Exhausting a snapshot recovery batch also ended the worker permanently. Both
paths left the UI usable but unable to receive later completions.

The existing subscriber now lives in frontend/src/event_subscription.rs. Busy/error
responses, malformed/noncontiguous pages, failed reads and interrupted snapshots
fence mutations and request verified snapshot recovery. The existing exact owner,
project/release identity and monotonic cursor checks remain. Exhausting one bounded
retry batch schedules another background attempt with backoff; it never starts
another model call, resubmits a command or trusts an unverified owner. Dropping the
UI update receiver ends the worker. Recovered snapshots restore current state,
then normal event polling resumes. No backend/wire/dependency change was needed.

Disconnected cached work no longer has an active elapsed/spinning indicator.
Last-observed telemetry remains explicitly labelled; tokens/cost/context can be
unknown until a provider response supplies usage. Account quota remains unavailable.

## Verification and limits

Behavioral red preceded the fixes: a busy socket reply permanently ended the
subscriber, and a disconnected cached reservation kept its working timer. Both
regressions now pass. The actual Unix-socket subscriber test supplies a busy page,
six failed snapshot reads, then a verified completed-state snapshot. It observes
mutation fencing, the recovered cursor/revision and only snapshot/events requests.
The existing lost-snapshot/handshake recovery regression also passes. UI checks
retain the connected active timer and suppress only the disconnected claim.

The remapped paired release passes the existing real isolated --controls journey:
actual TUI/systemd/service/runner/SDK, one held local HTTP stream, interrupt,
draft retention, prompt leave while stopping, same-owner reattachment, held
unconsumed steering/follow-ups without replay, separate pause and disconnected
leave. Only fixture units are stopped; no account is contacted. Source/binary/bundle
public-content, schema/reference and diff checks pass. Hosted checks are only
prepared in source, not run remotely. No lock/dependency upgrade or global install.

The original transient service/transport failure was not retained in the inspected
metadata. The source-level fatal-recovery paths are reproduced and fixed; ordinary
operator confirmation of the refreshed UI remains. Actual sign-in and completed
provider calls are now reported/observed for the operator's selected route, not
proof of all account/model access, token refresh, model quality or useful delivery.

## Existing behavior and next action

`1e09f59` implements /login and F3 G using SDK-advertised methods, private masked
entry, browser/manual callback handling and cancellation. SDK static OAuth flows
are bundled; login/refresh diagnostics suppress response bodies. Keys/codes stay
outside project IPC/outboxes/history. Sign-in never releases held inputs.
`2c6451a` implements exact-turn Enter steering, separate Alt+Enter follow-ups,
F7/E restoration, multiline/history and direct model/thinking/status controls.
Earlier direct interrupt/leave/pause and renewable-session contracts remain:
obsolete/stranded state is retired only after exact-owner fencing; source/Git/
worktrees remain, compatible busy owners stay attachable and retired calls never
replay. These mechanisms must not narrow provider/model support.

The ignored target/local-bin/waywr is the paired temporary local launcher;
target/local-bin/candidate.json records its actual source/artifact identity.
Next: the user leaves the stale UI with Ctrl-Q and relaunches that launcher to
load retained results, then continues the bounded [walkthrough](verify-usability.md).
Do not resubmit work merely to repair its display. Operator installation/account
calls remain explicit; no broad test campaign or orchestration redesign first.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[live input](live-input.md), [architecture](architecture.md), [plan](usability-plan.md).
