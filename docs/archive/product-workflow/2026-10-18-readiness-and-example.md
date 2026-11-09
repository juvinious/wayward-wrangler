> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Prior readiness and Example Task App example

Historical README sections from `d1dda2450389b310ccd3bca469692fffa75ade69`.
These are retained observations/examples, not instructions to run another job.
The [product plan](../../product-workflow-plan.md) owns next work.

## Current readiness

Waywr remains experimental and is not ready for routine use. Real Example Task App
deliveries and focused automated checks establish working components; the overall
workflow still requires substantial user guidance and is cumbersome.

The current gap is the handoff from checked integration to human acceptance. Preview
requires copying commands and changing into a deep `.waywr/worktrees/` directory.
There is no simple application-provided preview launcher or acceptance handoff yet.
The user still has to establish which checkout to test, why main has not advanced,
and why old branches/worktrees remain. Readable branches
and corrected recovery do not establish a smooth end-to-end experience. Human visual
acceptance and a usable preview-to-delivery interaction remain open. See the
[current handoff](../../HANDOFF.md) for observed progress and remaining limits.


## Delivered Example Task App

The human-led label-management-example extension delivered local main
`af358d7560d85f14d56aff481e6f34c5bd6c22a9` at `~/projects/example-task-app`.
Codex produced model, CLI/persistence and documentation units; Claude reviewed their
frozen commits, including a separate CLI disposition clarification. Saved exact
combined checks passed 61 tests, seven production-launcher smoke scenarios and
11 focused tag checks. All 40 tracked obligations were resolved. The user subsequently
verified a post-delivery recap from the live coordinator in the production TUI.

```fish
cd ~/projects/example-task-app
waywr
# Use the delivered CLI in another terminal (Python 3.12+):
./example-task-app list
./example-task-app tags list
```

The CLI supports add/list/done/reopen, tag counts/list, global tag rename/removal,
filtering, Markdown export, Unicode, stable IDs and atomic JSON replacement.
Use one writer at a time. Option-like tag restrictions remain; cross-platform behavior
and stronger power-loss durability are not established. Coverage includes test doubles,
finite corrupt fixtures and injected failures; the retained checks were not rerun here.

The earlier human-led Python migration at `069a162fbcb118794515034d3ae91303e7c37135`
has retained 44-test/seven-smoke evidence. The original Rust result
`887b33f09199522b73a1b806f1ef8435475f29f0` was agent-prepared and is distinct from
both human-led requests. The [verified work/tasking summary](../conversational-coordination/2026-10-17-verified-checkpoint.md)
and earlier [assessment](../development-workflow/2026-10-16-delivery-assessment.md)
record those origins, operational assistance and limits; these runs do not establish
unassisted reliability or exhaustive interaction coverage.

See [canonical project layout and migration](../../project-layout.md) for stopped-service
migration from `.wayward-wrangler` to `.waywr`, including history and Git worktrees.
