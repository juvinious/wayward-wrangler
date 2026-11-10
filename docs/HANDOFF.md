# Waywr handoff — live input and editor controls

Branch: `plan/usability-and-installation`. Checkpoint: live-input/editor slice after
`e4ac2ac`. Follow [AGENTS.md](../AGENTS.md). Local commits only; the user handles
remote delivery. No operator project, service, binding, provider configuration or
credentials were changed. No global installation or authenticated inference.

## Implemented outcome

Enter while a coordinator conversation is active sends live steering to that
exact reserved turn/subject. The service durably records guidance and publishes
an inbox bound to the attempt/project/frozen packet. The existing shared SDK
claims it between steps and calls Agent.steer. Pending, entering-context, applied
and held states are distinct. Applied means entered SDK context, not proof of
remote action. Duplicate publication is ignored. No model-specific branch,
extra daemon, dependency change or execution authority was introduced.

Alt+Enter queues a separate follow-up, also in F2 multiline mode. It waits for
the current reply, receives the committed reply/applied guidance as context and
has its own frozen packet/result/receipt. SDK followUp is deliberately not used
to fold multiple replies into one operation's single validated result. Pending,
withdrawn or held guidance never silently enters later conversation context.
Interruption holds unconsumed guidance and later turns; reopening never replays
them. Formal approvals/questions and scope changes retain explicit actions.

F7 identifies delivery states; Up/Down reaches every returned queue row while a
window keeps selection visible. E withdraws/restores full pending text for editing
without sending. Claim versus withdrawal uses atomic rename; a late edit is
refused. Restoring retains another draft/topic, journals before clearing the
command outbox and rejects duplicate acknowledgement after reopening. Applied
inputs cannot be withdrawn. Held guidance must be restored/resubmitted explicitly.

Up/Down moves within multiline input, recalling history only at its boundary.
Ctrl-J/Shift+Enter inserts a newline; F2 remains the terminal fallback. /model,
/thinking and /status open direct controls without execution. Pickers still edit
preferences; S saves and A explicitly reviews activation. Help and footer hints
explain the current controls. Older compatible owners without live_input_contract1
queue later turns until idle handover; build/model IDs do not gate the feature.

The preceding direct interrupt/leave/pause and renewable-session slices remain:
empty plain-composer Escape/Ctrl-C or /interrupt stops only the foreground call;
nonempty input clears. Ctrl-Q leaves promptly from any overlay/editor, including
while stopping/disconnected; empty-composer Ctrl-D and /exit also leave. /pause
separately stops new dispatch and stays open. Obsolete/stranded state is renewed
after exact-owned-process fencing, retaining source/Git/worktrees and usable
shared choices; retired calls/approvals are never imported or replayed.

## Evidence and limits

Behavioral red cases preceded implementation: Enter only queued later work,
SDK requests missed live guidance, duplicate publication caused an extra request,
pending text leaked into later context, claimed guidance could be falsely
withdrawn, replayed restore acknowledgements resurrected input, and long previews
hid queue focus. Focused regressions now pass for these cases.

The remapped release passes the real isolated --steering journey in
scripts/test-session-renewal.py: actual TUI/systemd/service/runner/SDK, multiline
input, queue withdrawal/edit/requeue, two completed turns from three local HTTP
requests and reopen without replay. Its --controls journey holds the runner,
interrupts a real local SDK stream, keeps a draft, leaves in under one second,
reattaches while stopping, holds unapplied steering/follow-ups without context
leak/replay, pauses separately and leaves disconnected. Only exact fixture units
are stopped. These checks use no accounts, operator project or actual inference.

SDK tool-loop checks cover OpenAI-compatible and Anthropic-compatible protocols.
Focused backend tests cover durable replay, exact subject, reopening and the
claim/withdraw race; UI checks cover editor, delivery, restoration, visibility and
shortcuts. Existing affected history/card/protocol checks pass. Schema consistency,
source/binary public-content gates, locked notices and release build pass.
A deliberately ignored latency simulation remains ignored. No dependency/lock
updates, hosted workflow runs or remote Git writes were performed.

Actual login, model quality, useful task delivery and the full daily workflow
remain unverified. Steering waits for an agent boundary; it cannot interrupt a
provider stream by itself. /interrupt is separate. Same-UID worktrees are not an
OS sandbox. Queue snapshots retain bounded recent applied/held previews; full
pending text remains durable. Public installer/update/uninstall/package work is
still subsequent work, not completed by this slice.

## Next authorized action

The interaction/lifecycle sequence is implemented locally. The ignored
target/local-bin/waywr is the local candidate launcher; target/local-bin/candidate.json
records its paired temporary bundle identity. Leave global/operator installation
alone. The next product evidence is
one bounded useful [operator walkthrough](verify-usability.md), including redirect,
interrupt, leave, rebuild and return. Account calls/operator changes need explicit
authorization; fixtures never supply it. Convert concrete friction into focused
regressions; do not start another broad test campaign or orchestration redesign.

Owners: [live input](live-input.md), [controls](tui-controls.md),
[providers](provider-selection.md), [architecture](architecture.md),
[plan](usability-plan.md), [project layout](project-layout.md).
