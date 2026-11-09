> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical snapshot from merged checkpoint `c2064bc534d8cbe753bbae93a8d85a98ce8c403a`.
> Completed delivery evidence; permissions and next actions below are historical.
> Relative links are relocated; current work is in [the handoff](../../HANDOFF.md).

# Development workflow — implementation and assessment

The approved plan at `6b04221` is retained as an
[immutable planning snapshot](2026-10-15-approved-plan.md).
Example Task App was completed on 2026-10-15 in an agent-prepared demonstration.
The [corrected assessment](2026-10-16-delivery-assessment.md) explains why this did not verify
the intended human-led interactive workflow. Application-specific instructions
are removed from the general product.
[HANDOFF](2026-10-16-handoff.md) records the installed release, exact application commit,
verification and remaining limitations. Branch closeout and GitHub merge are now
explicitly authorized; release publication and additional runtime work are not.

## Implemented outcome

The existing Rust/Ratatui TUI and Rust service/runner now connect discussion,
exact scope/policy approval, native Codex coordination, concurrent task worktrees,
frozen commit review, serialized integration, combined checks, obligation closure
and checked clean local-main advancement. One active parent remains project-scoped;
workers are child units, not independent project services.

The supported initial profile is Codex 0.162.1, gpt-6.1-sol/medium, two concurrent
workers, capacity waiting and an empty fallback list. General provider/model/effort
selection and nonempty fallback remain unimplemented. There are no mandatory task
time/call/repair ceilings; bounded output and transport/process/shutdown health
handling remain. Approval covers routine development within its saved scope and
operating policy; discussion and model prose grant no execution authority.

Independent tasks continue while coordinator/check calls are pending. Settled
failures feed replanning; ambiguous effects remain fenced for reconciliation.
In-scope steering records a new requirement generation. Dependencies distinguish
start from acceptance, updates wait for safe checkpoints, and placeholders remain
explicit obligations until real combined behavior is verified. Active workers
finish/checkpoint when an interactive yield channel is unavailable.

## Persistent request history

A stable request identity is assigned before approval and follows discussion,
parallel assignments, attempts, revisions, recovery, integration and delivery.
One append-only readable log lives under the selected canonical state root's
`history/` directory. UTC/actor/source-event labels and framed append recovery keep
it continuous across detach/restart. Unexpected manual tails are preserved.

Inspect exposes absolute log and compact-summary paths, projection freshness and
narrative-pending state. The deterministic checkpoint records current status,
exact commit/check evidence, obligation counts, a bounded open-obligation list and
labelled coordinator narrative; full details remain in the continuous log. History
is derived context, never approval, receipt or Git authority.

## Original delivered demonstration

All Example Task App application source, tests, scaffolding and documentation were
produced by Waywr-launched workers. Persistence and CLI tasks ran concurrently,
followed by production wiring, binary coverage and final documentation review.
Waywr recorded six accepted units, resolved 32 obligations, ran final locked tests,
release build and isolated binary smoke checks, and advanced clean local main to
`887b33f09199522b73a1b806f1ef8435475f29f0`.

The CLI implements add/list/done/reopen, tags and filters, Markdown export, stable
IDs, Unicode, restart persistence, atomic file replacement and non-destructive
invalid/corrupt-input errors. It is single-writer; concurrent mutation protection
and power-loss durability are not established.

## Human-led Python migration

The human subsequently initiated a new migration request through the live coordinator,
approved its scope/policy and used production TUI recovery/closeout. Native workers
produced the Python replacement; coordinator-selected repairs, integration and checks
led to checked local main `069a162fbcb118794515034d3ae91303e7c37135`. Both retained
acceptance commands passed at that exact commit: 44 tests and seven executable smoke
scenarios, including Python operation without Cargo or a retained Rust executable.

The [assessment](2026-10-16-delivery-assessment.md) records request origin, operational assistance,
readable history and exact verification limits. This demonstrates one human-led
request through visible delivery, not every interaction or unassisted reliability.

## Maintained owners and remaining work

[Architecture](../../architecture.md) maps source and authority. Detailed contracts live
in [state/history](../../contracts/state-contract.md),
[execution/recovery](../../contracts/execution-contract.md),
[IPC](../../contracts/session-protocol.md) and [TUI](../../contracts/tui-contract.md).
[README](../../../README.md) owns normal launch/build instructions.

[Linux setup](../../linux-sandbox-setup.md) documents the user-approved temporary AppArmor
testing workaround. Distribution sandbox preflight/setup, broader provider selection
and execution on other operating systems remain future work. They are not claimed
as verified by this retained delivery. No additional trial or publication is
implicitly authorized by the archived plan or this implementation record.
