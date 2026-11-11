# Live steering and follow-up

Enter sends an idle conversation message or steers the foreground coordinator at
its next agent boundary. Alt+Enter queues a separate follow-up, including when F2
multiline mode is enabled. Follow-ups start after the current turn completes and
receive its committed reply as context. Each keeps its own frozen packet, result
and receipt; SDK followUp replies are not folded into an earlier operation's
single validated result. Model/provider IDs remain runtime data.

F7 distinguishes pending, entering-context, applied and held steering from queued
follow-ups. A window of previews keeps the selected input visible; arrows reach
the whole queue and PageUp/PageDown scroll the selected details.
Select an input and E to withdraw it and restore its complete text to
the composer. Restore requires an empty draft; if another draft arrives while
the acknowledgement is pending it stays intact and the restored input is retained
for F7/E. Restored slash text remains literal until deliberately edited/submitted.
An accepted restore is journalled locally before removing its command outbox entry;
its command identity prevents duplicate acknowledgement from resurrecting text.

An applied/claimed input cannot be edited or withdrawn. Pending input wins an
atomic rename against the runtime's claim, so editing cannot falsely retract text
the agent already took. A late edit is refused in place. Holding, withdrawing or
restoring input does not authorize model calls, scope changes or workflow actions.

## Service and runtime ownership

The service advertises live_input_contract1 in the continuous conversation
snapshot. A compatible UI against an older owner queues follow-ups until idle
handover instead of pretending to steer that owner. Current shared-runtime
sessions accept conversation.steer for an exact reserved turn and subject, or
conversation.withdraw_input for one pending input. Commands use the existing
durable ledger/writer and reject a stopped, changed or foreign turn. There is no
provider/model allowlist in this routing.

conversation_live_inputs holds pending guidance in project SQLite. The service
publishes it only to the current attempt's private live-input/pending directory,
bound to project key, turn ID, attempt ID and frozen packet SHA256. The shared
runtime atomically claims pending files at its next finishTurn boundary and calls
the SDK's Agent.steer. It marks applied only when that SDK user message enters
agent context. The marker and runtime.input event identify the exact input.
The runtime ignores duplicate input identities; the service verifies applied
scope/text against its own record. No model prose is an acknowledgement.

Pending/withdrawn guidance does not enter ordinary conversation context. Applied
guidance is added after acknowledgement and becomes context for later turns in
that same subject. Approval/question cards still require their explicit actions;
steering does not grant authority or change frozen evidence requirements.
The original packet, model, tools, scope, grants and receipts
are never rewritten. The same mechanism works across supported SDK protocols.

Interruption keeps unconsumed steering and later messages held. Settlement or
reattachment does not replay them. Restore/edit or explicit continuation is a
separate human action. Applied means entered agent context, not proof a remote
provider read it or acted on it. A late input that misses a completed call stays
held. These files share Waywr's trusted same-UID boundary; worktrees are not an
OS sandbox.

Owners: [service inbox](../backend/src/live_input.rs),
[conversation state](../backend/src/continuous.rs),
[service dispatch](../backend/src/session_service.rs),
[runtime preparation](../backend/src/model_runtime.rs),
[runtime inbox](../model-runtime/live-input.mjs) and
[agent loop](../model-runtime/main.mjs); the existing Ratatui composer/queue.
No extra daemon, provider wrapper or dependency was added.
