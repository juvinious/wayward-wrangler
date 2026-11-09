> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Proposal routing/card correction — installed, 2026-10-17

Source `3e04c78` / `b7dd8d8` on the existing stacked implementation branch. User
requested effective Claude review enforcement, truthful proposal/delivery status,
concise expandable cards and Alt+page fine scrolling; they explicitly detached.
No approval or application work was submitted on their behalf.

Read-only evidence through event580 established:

- Same label-management-example request `<private-record>`, subject
  `discussion-<private-id>`.
- Human event572/message `m-<private-id>` explicitly requested Claude
  review and Codex coordination/implementation, before approval. Live coordinator
  saved proposal v2, but no enforced request routing was recorded. Project review
  default remained Codex; both providers were allowed. Prose alone was insufficient.
- Task `<private-record>` was actually `proposed`, never failed. Generic
  activity substring classification read words in the full proposal as failure.
- Background `coordination.present` delivery set Sending and its receipt intentionally
  did not replace that notice. Late input results could also overwrite terminal state.

Final paired release `conversation-20261017-3`, stable normal prefix
`~/.local/share/wayward-wrangler`; manifest SHA256
`e973319b6466db7de857d2ddef93f4e7f6ae5587aac2e8413cdcfee04a6c9ee2`.
All 13 file hashes verified, with `request_routing_contract: pinned-roles-v1`.
Normal pause/drain and guarded upgrade advanced binding22→23; no service/worker
was interrupted. Service left inactive/paused generation27, state revision584.

Supported stopped-project `session reconcile-request-routing` consumed the retained
human message in its own request. Event583 records Codex coordinator/implementation
and Claude review pins, separate from broader allowed sets. Event584 saves the
refreshed policy card. No new turn or approval was queued; the card remains unpresented
and unapproved until the human returns. Proposal v2 and SHA
`cf233a5fa4acbfb9318996194775fba8850058260bf18126828f300e3d87f0b4` are unchanged.

Before/after hashes matched 17 selected original table snapshots (original events and
history sidecars through 580; proposals, heads, decisions, messages, turns, questions,
bindings, preferences and prior development records), 7,345 files, all three original
readable log prefixes and Git HEAD/refs/worktrees/status. Only the new request-routing
record and appended public events/card were added; release/lifecycle records advanced
through supported commands. Project preferences and shared provider configuration/auth
were not changed. Example Task App code/worktrees and earlier deliveries remain intact.

Focused local checks: frontend 100 passed/one optional simulated latency probe ignored;
request-role regression covers retained-source idempotence, unchanged real fixture
proposal, role/default/grant separation, task/coordinator conflict, frozen approval,
runner permit, capacity fallback and exact-review gating. Coordination 8, provider 16,
continuous 11, task-scope 11 and CLI-parser 2 checks pass. Locked paired release build,
CLI help, IPC consistency, links and diff checks pass. These are fixtures plus actual
read-only/install preservation evidence, not native Claude review or human end-to-end
delivery acceptance.

Human launch: `waywr --project ~/projects/example-task-app resume`. Review and approve
the actual refreshed card. Normal chat/history and saved answer remain available.
