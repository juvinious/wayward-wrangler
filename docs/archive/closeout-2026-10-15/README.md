# Documentation map

Read [AGENTS.md](../AGENTS.md), [HANDOFF.md](HANDOFF.md), then task-relevant current
contracts. AGENTS.md owns every-checkpoint reassessment; archives are selective
evidence, never a default reading list or new authorization.

## Current checkpoint and review

[Continuous coordinator contract](continuous-coordinator-contract.md) owns state-7
interaction/authority and the exact builtin policy. [Acceptance report](continuous-coordinator-report.md)
owns the **accepted bounded v8 live checkpoint**:4/6 completed native calls, four
receipts, two proposal revisions/exact human authorizations, operator-verified physical
detach/re-attach, and later checkpointed exit. [Completed authorization](claude-continuous-live-acceptance.md)
preserves the six-call ceiling; two unused calls remain, no renewal/additional call requested.
[Reopen guide](continuous-coordinator-evaluation.md) identifies the retained live project
and one isolated launcher. No global installation or working program/worker acceptance.
[Readiness](continuous-native-readiness.md) owns the passed boundary and remaining
native queue/interruption/recovery limits. [Handoff](HANDOFF.md) owns next action:
await user direction, no further feature work or calls in this closeout.

[Historical live-scope report](live-scope-report.md) and
[stopped trial disposition](claude-scope-live-acceptance.md) preserve `claude-scope-01`:
one output-invalid native call, **blocked 1/6**, no proposal/decision. That trial cannot
resume under the successor policy. Original **claude-conversation-v5** separately
completed three native conversation calls with operator detach/resume; exhausted 3/3.
Neither historical success nor current mock checks establish new native scope behavior.

## Current source of truth

| Contract / information | Owner |
| --- | --- |
| Agent/checkpoint documentation policy | [AGENTS.md](../AGENTS.md) |
| Branch, authorization, checkpoint, resources and next action | [HANDOFF.md](HANDOFF.md) |
| Ordinary init/start/resume; installation caveat | [root README](../README.md) |
| Component ownership and project isolation | [architecture.md](architecture.md) |
| Host lifecycle, claim/permit, recovery/cancellation | [execution-contract.md](execution-contract.md) |
| Authoritative records, schema/release compatibility and rollback | [state-contract.md](state-contract.md) |
| IPC capabilities, commands, replay and bounded reads | [session-protocol.md](session-protocol.md) |
| Accepted presentation/input, next actions and review | [tui-contract.md](tui-contract.md) |
| Implemented Claude restrictions, context, streaming and authority | [claude-coordinator-contract.md](claude-coordinator-contract.md) |
| Implemented schema-6 scope authority/host/TUI and exact revision contracts | [live-scope-contract.md](live-scope-contract.md) |
| Implemented continuous interaction and independent offline gates | [continuous-coordinator-contract.md](continuous-coordinator-contract.md) / [report](continuous-coordinator-report.md) |
| Unresolved gates and protected historical baseline | [current-limits.md](current-limits.md) |

Future requirements are labelled proposed and linked to their current request; they do
not silently replace current contracts. Opening existing projects never upgrades
state or releases. Native workers, consultation and a working generated application
remain unverified; the pinned-client proof is not whole-process confinement.

## Archives — retrieve only relevant evidence

- [continuous-coordinator](archive/continuous-coordinator/): completed approved plan
  at `5cc578c`, pre-closeout handoffs, v7 report/evaluation and the resolved closed-gate
  finding at780f658; completed v8 boundary/preparation report and live/mock walkthroughs.
  These snapshots retain their original relative links, resolved
  through current contract/index locations. Current documents carry obligations;
  archived instructions do not authorize another trial or replace current state.

- [live-scope](archive/live-scope/):
  [pre-correction handoff](archive/live-scope/handoff-at-3d94b66-2026-10-14.md) and
  [v6 mock walkthrough](archive/live-scope/scope-v6-mock-walkthrough-2026-10-14.md)
  retain checkpoint context and completed regression instructions.
  [approved plan at 8c1bc7d](archive/live-scope/implementation-plan-8c1bc7d.md) and
  [completed stages 1–3](archive/live-scope/checkpoints-1-3-report-2026-10-14.md),
  including failed hosted attempt rationale.
  [V5 report snapshot](archive/live-scope/scope-v5-report-before-sequential-review-2026-10-14.md)
  and [superseded V5 walkthrough](archive/live-scope/scope-v5-walkthrough-before-sequential-review-2026-10-14.md)
  and [stopped live walkthrough](archive/live-scope/claude-scope-01-live-guide-at-53a9fb2.md)
  retain earlier evidence/instructions, not permission to continue. Current
  contracts/report carry remaining obligations; runtime evidence stays ignored.

- [native-integration](archive/native-integration/):
  [completed Claude closeout](archive/native-integration/claude-conversation-closeout-2026-10-14.md)
  owns exact v5 receipts/counts, separate operator observations, v6 build/replay,
  cleanup and provenance. It records the premerge checkpoint; PR #6 is now merged.
  [Completed authorization](archive/native-integration/claude-v5-live-authorization-and-outcome-2026-10-14.md),
  [earlier mock evidence](archive/native-integration/checkpoints-1-4-report-2026-10-13.md),
  [Codex/broader-role assessment](archive/native-integration/boundary-assessment-through-live-check-2026-10-14.md)
  and superseded proposals/handoff snapshots remain here. Unresolved gates remain
  visible in current limits.
- [tui-first](archive/tui-first/): completed host transition, accepted v3 simulated
  summary/cleanup and superseded walkthroughs. Physical v3 detach/resume unreported.
- [portability](archive/portability/): initialization corrections and protected
  blocked-legacy-task evidence.
- [milestone-5](archive/milestone-5/) and [context-continuity](archive/context-continuity/):
  Python-era native evidence, not current Rust native-worker proof.
- [rust-migration](archive/rust-migration/), [milestone-1](archive/milestone-1/),
  [milestone-2](archive/milestone-2/), [milestone-3](archive/milestone-3/),
  [milestone-4](archive/milestone-4/), [milestone-6](archive/milestone-6/) and
  [other groups](archive/): completed plans/reports and older contracts.

Immutable handoff/report snapshots retain original relative links; this index and
current contracts locate moved documents. Other moved links are rebased. Quoted
historical paths/hashes stay unchanged. Runtime evidence remains ignored under
`.local/` or its original project; never commit secrets or alter evidence to tidy docs.
