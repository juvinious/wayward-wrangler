> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Conversational correction — installed checkpoint, 2026-10-17

Source `f0b448f` on `impl/conversational-coordination`, stacked from `f5139db` over
unmerged provider work. Locked Rust1.99.0 paired release build; CLI help checked
against build/install and explicit repair arguments. No native inference or new
application job. Current behavior: [contract](../../conversational-coordination.md).

- Final release `conversation-20261017-2`, normal prefix
  `~/.local/share/wayward-wrangler`; launcher `~/.local/bin/waywr`.
- Manifest SHA256 `2f4c0cd5d2f43e7cca63bb6926abc274ac7162248ba5b1c2d395be0fe825f8bb`;
  all 13 file hashes verified; `interaction_contract: conversation-v1`.
- Example Task App key `<private-id>`: guarded pause/drain, inactive
  exact user service, then normal release changes binding20→21→22. State9/config6
  retained; IPC1.8. Intermediate release1 remains available, as do prior releases.
- Service left inactive and paused generation25, state revision562. Turns remain
  blocked 1/completed 6, no queued/reserved turn. Both workflows remain delivered.
- Eight backend coordination regressions; frontend 99 passed, one optional simulated
  latency probe ignored; five final UI regressions pass. Related provider 16,
  continuous 11, scope-contract 3 and task-scope 11 checks pass. IPC/source links/diff
  checks pass; CLI-parser2 and maintenance2 regressions also pass. These fixtures
  are not human or mixed-provider acceptance.

Supported stopped-project `session repair-answer-history` consumed event554,
request `<private-record>`, related555–558. Event561 appends its saved
answer/original timestamp/hash plus routing cross-references to the correct log;
event562 explains the misrouting in the old Rust log. Event556's empty history
sidecar is explicitly resolved from the retained canonical turn/subject association.
Original event bodies, sidecars and published frames were not changed. No approval
or continuation was replayed/queued; resume hint only repairs legacy UI ownership.

Before/after checks matched 12 selected protected table snapshots (including original
events/sidecars through 558, question decisions, conversations, authorization and
workflow records), 7,345 files, all three original log prefixes and Git HEAD, refs,
worktrees and clean status. Release binding intentionally advanced; no Example Task App
code, retained worktree files or shared configuration/authentication was changed.
Application HEAD remains `069a162fbcb118794515034d3ae91303e7c37135`.

Readable label-management-example history:
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
It contains original conversation/question and the full saved answer, with labelled
correction evidence, not an invented continuation or approval. No task assignments
exist for this unapproved request. Human continuation/actual proposal approval and
native conversational operation remain unverified until ordinary use. No push,
merge or publication occurred.
