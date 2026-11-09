> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Verified workflow checkpoint — 2026-10-17

This records completed work and evidence, not authority for another job.
[HANDOFF](../../HANDOFF.md) owns current readiness; maintained contracts own behavior.
Branch `impl/conversational-coordination` retains the unmerged provider increment.
Stable default and Example Task App binding 30: `conversation-20261017-6`, code `c6fadeb`.

## Waywr work completed

- Provider selection: project defaults, implementation/review preferences and task
  overrides; exact per-attempt profiles; Codex default preserved; Claude explicit;
  after-turn or reconciled interrupt switching; fallback disabled unless configured
  and approved. [Provider contract](../../provider-selection.md).
- Conversational coordination: stable request ownership, saved answers and automatic
  continuation, exact presented proposal/policy approval, source-bound steering and
  narrow provider choices; append-only repair of the saved label-management-example answer.
  Ordinary discussion and delivery recaps do not require a task or scope action.
  [Conversation contract](../../conversational-coordination.md).
- TUI/lifecycle: concise expandable cards, truthful delivery/preparation status,
  command recall on Up/Down, chat on PageUp/PageDown and Alt+Page line scrolling,
  modal navigation, timed activities and current worker operations. Detach closes
  the view; exit drains/pauses the session. [TUI controls](../../../README.md#tui-controls-and-activity).
- Persistence: canonical `.waywr` migration with identity/approval/worktree/receipt
  preservation, readable linked request history and short timestamp/hash filenames.
  [Layout](../../project-layout.md) and [state/history](../../contracts/state-contract.md).
- Context/recovery: compact working summaries, independent soft preference/model
  estimates/application bounds, exact read-only evidence retrieval and active memory,
  Inspect diagnostics, settled-call continuation and frozen-hash preservation.
  Superseded records no longer hold active paths. [Context contract](../../context-delivery.md).
- Existing concurrent task worktrees, serialized integration and checked local-main
  advancement remain the execution path; no application-specific task template added.

## Actual label-management-example taskings

Human request `<private-record>`, parent
`<private-record>`, approved proposal v3 SHA
`8b1ef61b4f2f5dc3cc21e7a0477589aa1dca3b72b5f018e5090c83fb38bafaa4`.
Codex coordinated/implemented; the saved human choice pinned Claude review.
The live coordinator selected these assignments and subsequent actions. The table
records this project, not a sequence prescribed by the general application.

| Task | Produced work / disposition |
| --- | --- |
| `implement-label-management-example` | Original Codex submission `da98c4ff...`, retained as superseded with its three earlier Claude reviews; not directly integrated under v3. |
| `complete-tag-model` | Codex `33e2ab8...`: counts, rename/remove semantics and focused model regressions; accepted after `review-completed-tag-model` by Claude. |
| `complete-tag-cli` | Codex `02823012...`: CLI parsing, no-save paths, real JSON persistence and executable regressions; Claude `review-completed-tag-cli`, then `clarify-cli-review-disposition`; accepted. |
| `complete-tag-docs` | Codex `23995f7...`: README, behavior contract and request history; accepted after Claude `review-completed-tag-docs`. |
| `checks-66` | Coordinator requested the service's actual Python aggregate and focused tag checks at exact combined commit `af358d7560d85f14d56aff481e6f34c5bd6c22a9`. |
| Closeout/promotion | Service recorded checked clean advancement to local main; all 40 tracked obligations resolved. No remote publication. |

The human requested the split before the context correction; that approval was
preserved. Waywr preparation defects required assisted recovery: the old 96 KiB
application ceiling, a second host check, lost read memory, superseded path ownership
and JSON timestamp precision. Completed calls/evidence stayed immutable and were
not replayed. These interventions distinguish this assisted delivery from a claim
of unassisted reliability.

Claude's CLI review retrieved exact `example_app/cli.py` at `02823012...` through
the service: 6029 bytes, complete, immutable exchange hash verified. Its complete
16995-byte diff arrived in two pages; a fresh second review attempt consumed the
retrieved source. The result said no blocking defects but retained
`requires_changes:true`/a provisional finding. The coordinator requested a separate
Claude disposition clarification; it returned `requires_changes:false` for the same
commit, identifying advisory issues and pending combined checks. The earlier result
was preserved, not rewritten or simply described as an immediate pass.

Saved `checks-66` receipts have exit 0 / passed:true for
`python3 tests/check_python.py` and `python3 tests/check_tags.py`: 61 tests, seven
production-launcher smoke scenarios and 11 focused tag checks. Captured output hashes
were verified during closeout; no acceptance suite was rerun for these docs.
Coverage includes Store doubles, finite corrupt data and injected syscall failures;
not every check is an independent semantic oracle. Option-like tag restrictions,
single-writer storage, platform coverage and power-loss durability limits remain.

## Human verification of ordinary conversation

After release 6, the human asked for the delivered work/actions to be summarized
through the production TUI, then explicitly confirmed “Verified” with its screenshot.
Turn `<private-record>` completed under live Codex `gpt-6.1-sol`:
`action:null`, `evidence_refs:[]`, native receipt exit 0, attempt
`<private-id>`. Events 965–970 record fresh human recovery
acknowledgement/input, reservation, coordinator response and completion. The request
remained delivered; no new development action was created. Earlier failed recap
961 and its raw evidence remain historical records.

This verifies the observed post-delivery recap path, alongside the delivered
Codex implementation/Claude review path and Claude evidence continuation. It does
not verify every conversational intent, profile/model combination, switching,
fallback or interruption scenario. Broader general-topic replies remain fixture
coverage rather than new live acceptance. Exact native token capacity remains unknown.

## Continuation boundary

The initial Rust Example Task App run was agent-prepared and is not proof of authentic
user tasking. The later human-led Python migration delivered `069a162f...`
(44-test/seven-smoke evidence); the human-led label-management-example extension now delivered
`af358d7...`. Preserve all three histories and their different origins.

Normal attach: `waywr --project ~/projects/example-task-app`. The user can discuss the
completed request or originate the next expansion, select participating profiles
and approve its exact saved scope/policy. No next feature or request is approved by
this checkpoint. Linux/systemd/AppArmor portability, trusted-client/same-UID
isolation, native reliability and honest capacity estimates remain limitations.
Claude coordination/implementation, arbitrary profiles and the full handover/fallback
matrix are not claimed live-verified by this run.

Readable history:
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
Prior [installation evidence](2026-10-17-ordinary-chat-correction.md) and
[context recovery evidence](../context-delivery/2026-10-17-installation.md) retain
source commits, preservation comparisons and the checks behind this checkpoint.

Later evidence clarification (coordinator-recovery planning `d21271f`): the native
CLI result was nonblocking. Waywr restored the earlier provisional string into its
effective disposition; this was an application retention gap, not contradictory
native Claude output. See the [exact comparison](../coordinator-recovery/approved-plan-d21271f.md#correction-to-the-review-diagnosis).
The delivery and original evidence above remain unchanged.
