> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Runtime inventory note — terminal usability update

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

The demo is now approved. This usability turn launched no workers, review tabs,
controllers or retained processes. Earlier pane IDs below are historical; the
full current layout was not re-inventoried. Review tabs opened by the operator
will use the run's effective workspace w8 and names `AWP review ...`.

The user-wide local.agent-workflow-review plugin is registered and enabled;
it has no persistent background process. Saved logs and artifacts remain intact.
See [current outcomes](terminal-review-outcomes.md).

## Historical milestone-1 inventory (not current)

# Retained Herdr session — October 10, 2026

All entries below are prototype-owned in the existing w5 workspace. **No fake worker child is still executing.** Eleven runner processes sleep to preserve completed/failed/cancelled terminal output; two test-controller processes retain reports; one interactive demo controller waits for your decision.

At the original checkpoint, the demo controller was **tab w5:tJ, pane w5:pT**. Its earlier controller exited; restart preserved the pending choice and both existing worker panes with counters still 1.

| Tab | Pane | PID | Run/stage or controller | State |
|---|---|---:|---|---|
| w5:t4 | w5:pC | 2886438 | foundation / stream-success | retained completed |
| w5:t5 | w5:pD | 2886653 | AWP controller — foundation | retained test output |
| w5:t6 | w5:pE | 2886660 | foundation / restart-live | retained completed |
| w5:t7 | w5:pF | 2886724 | foundation / retained-failure | retained failed |
| w5:t8 | w5:pG | 2886753 | foundation / cancel-target | retained cancelled |
| w5:t9 | w5:pH | 2887680 | AWP controller — M1 acceptance | retained test output |
| w5:tA | w5:pJ | 2887693 | acceptance-m1 / draft | retained completed |
| w5:tB | w5:pK | 2887723 | acceptance-m1 / critique-1 | retained completed |
| w5:tC | w5:pM | 2887786 | acceptance-m1 / revision-1 | retained completed |
| w5:tD | w5:pN | 2887826 | acceptance-m1 / critique-2 | retained completed |
| w5:tE | w5:pP | 2887876 | acceptance-m1 / revision-2 | retained completed |
| w5:tG | w5:pR | 2888231 | demo / draft | retained completed |
| w5:tH | w5:pS | 2888273 | demo / critique-1 | retained completed |
| w5:tJ | w5:pT | 2889471 | AWP controller demo | awaiting your choice |

Use Herdr navigation or the herdr tab focus command to inspect a listed tab. PIDs/IDs are session observations, not permanent identifiers. Verify current ownership before any manual process action.

Completed AWP tabs can be closed manually when you finish reading; their retention processes will exit. The active demo controller is awaiting your choice, not performing model work. Do not stop the shared server. Artifacts/transcripts remain in .local/ after pane closure.

The retained output includes the automated acceptance approval fixture; the separate demo run remains unapproved.
