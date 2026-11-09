> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Fresh approval and preparation correction — installed, 2026-10-17

User reported typed “I approve” required menus and requested an initial preparing
label. Source `ee45671` fixes those paths; `1ed4c30` fixes the concrete reopening
blocker discovered during guarded installation. Same stacked implementation branch;
no push/merge/publication or application work submitted by this maintenance turn.

## Evidence and behavior

Request `<private-record>`, subject
`discussion-<private-id>`, task
`<private-record>`, proposal v2 SHA
`cf233a5fa4acbfb9318996194775fba8850058260bf18126828f300e3d87f0b4`
remain unchanged. Prior [routing/repair evidence](2026-10-17-proposal-routing-correction.md)
records the human's Claude reviewer choice and preserved saved answer.

Event 587 retained typed “I approve”, input `1792252209171374847-372768-22`, targeting
old policy card `e3626c44…` while current presented card was `194623b6…`. Phrase
recognition succeeded; stale frontend context prevented authorization. A consumed
slash command retained an empty draft's reply target, which could survive restart.
The correction clears consumed-command context, persists/restores targets only for
nonempty drafts and captures current context at first fresh input. Nonempty stale
drafts still refuse; clarification distinguishes old targets from pending input.

The human legitimately approved via scope details: message 588, authorization 589/590,
result 591; command `interaction-<private-id>`, input
`1792252239048643549-372768-75`. That approval is preserved. No maintenance action
replays 587, rebases assent or submits a new approval. Worker public activity now
shows Preparing until its first known public operation; completed operations clear.
Unknown/truncated/invalid tails do not imply initial preparation or expose reasoning.

Actual human-approved work ran during the build: implementation commit
`da98c4ff6e50d29ad9bef15fdc81a5be3d7228ff` was submitted. Claude review units
`review-label-management-example`, `review-label-management-example-evidence`, `review-complete-frozen-diff`
have completed receipts and `requires_changes:false`. Earlier coordinator attempts
to substitute Codex review and integrate without frozen review were rejected by
service guards; the live coordinator subsequently obtained these Claude reviews.
This demonstrates actual enforced provider routing, not completed delivery or a
claim of efficient/reliable autonomous planning.

The workflow naturally settled at event 637 into attention, pending=null: mandatory
coordinator packet 102424 bytes exceeds selected 98304-byte budget; no next attempt
launched. Packet fitting remains unresolved. No task was retasked, budget enlarged,
integration forced or execution retried during installation.

## Installation and preservation

The user detached. Standard CLI stop could not pass readiness because its full
snapshot disconnected. Same-release read-only hello/session.status succeeded;
full snapshot failed. Accumulated raw development records exceeded 1 MiB.
An initial conditional stop replay was automatically rejected for uncertain delivery;
subsequent read-only `command_get` positively returned `not_found`. The retained
lifecycle command `cli-exit-<private-id>` was then delivered
through the supported same-UID command protocol at revision 637, accepted at 638;
normal drain reached paused 639/generation 29 and service inactive. No ambiguous
application execution or approval was replayed.

Display snapshots now select metadata/compact obligations with source hashes and
omitted field names; full records/evidence remain unchanged. Service readiness
uses small session.status. This fixes observed reopening without changing model
packets/authority. Record-count paging and coordinator packet fitting remain limits.

Paired release `conversation-20261017-5` installed at
`~/.local/share/wayward-wrangler`; all 13 file hashes verified.
Manifest SHA256 `bac563ed07353c721aceaab4ead34d19a93de2d666e6f6f14156d787e7098398`,
contracts conversation-v1 / pinned-roles-v1. Guarded upgrade advanced binding 23→24
at revision 639. Earlier releases remain retained. Unbound release 4 was superseded
by 5 after the concrete snapshot correction.

Fresh pre-stop preservation snapshot compared after upgrade/startup: original rows
in all 45 captured tables remain byte-identical; Git HEAD/refs/worktrees/status match.
Of 3213 captured history/operation files, 3208 original evidence/log prefixes match;
five mutable projection/append/summary files refreshed through normal projection.
Only lifecycle events 638/639 were appended. Conversation turns remain blocked 1 /
completed 8; no new turn/attempt/approval. Provider configuration/auth/preferences
were not changed.

Installed service started paused solely for read-only verification: snapshot loaded
at 307259 bytes with 3 workflows/25 units and retained attention. Exact service was
then stopped while paused. Final state inactive/paused 639, binding 24. History:
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.

## Verification boundary

- Frontend coordination 6 and backend coordination 9 pass, including typed command →
  restart → fresh “I approve” without modal, stale refusal and once-only approval.
- Worker activity 7, command recall 3 and continuous refresh 1 pass. Added bulk
  snapshot regression verifies unchanged source records and retained role/attempt
  metadata with small display payloads. Fixtures supply no human/native authority.
- Locked paired release build, protocol consistency, installed hashes/read-only
  snapshot, preservation, documentation links and diff checks pass.

Launch `waywr --project ~/projects/example-task-app` to inspect/steer. No job was resumed
for testing. Fresh typed approval and full human mixed-provider delivery remain
unverified by ordinary use; `/resume` alone cannot remove the unchanged size error.
