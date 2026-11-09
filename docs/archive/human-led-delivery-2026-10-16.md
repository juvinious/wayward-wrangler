> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Waywr handoff — TUI documentation and human-led delivery (2026-10-16)

## Current checkpoint

Continue on `impl/development-workflow`. The human confirmed that their Python
migration reached Delivered in the production TUI and requested documentation of
our TUI changes. Those controls and presentation behavior are now consolidated in
[README](../../README.md#tui-controls-and-activity), owned in detail by the
[TUI contract](../contracts/tui-contract.md). No new application request, approval,
native trial, push, merge or publication was performed or is pending.

## Installed application and TUI

Stable launch: `waywr --project ~/projects/example-task-app` (or `~/.local/bin/waywr`).
Paired release `development-20261016-9` remains selected and bound to Example Task App,
binding generation19, upgrade revision527. Runtime implementation is `4456a79` plus
`eff63fe`; manifest SHA-256:
`f0fb37a6096cdb1f4475f062a7cb4897c5780560330ca254c171ba77efe86521`.
This documentation-only checkpoint needs no reinstall or service interruption.

Documented behavior:

- Up/Down recalls composer command/input history; newest Down restores the draft.
  Alt+Up/Down moves the multiline cursor; Ctrl-R recalls without sending.
  PageUp/PageDown scrolls chat/live rows; overlays retain their own navigation.
- Activity headers show local time/date; current work has elapsed time and subtle
  shimmer, with static reduced/off/mono modes. Tree bullets show running worker
  titles/IDs and reported commands/tools; output stays in `/output`. One-second
  refresh, five-second expiry and bounded metadata tails use explicit fallbacks.
- F6, `/plan`, `/inspect` and `/output` distinguish scopes, approved authority,
  integration/check evidence and exact readable history. `/exit` checkpoints/pauses;
  `/detach` closes the view while work continues. `/resume` distinguishes paused
  sessions from a selected operationally blocked workflow; questions use `/reply`.

The human confirmed history and activity presentation during real use. Per-worker
operation rows retain focused fixture verification; no new native job was started
solely to exercise them. Implementation/install preservation checks are retained in
[worker activity rollout](tui-worker-activity-2026-10-16.md), earlier recovery
in [recovery checkpoint](workflow-recovery-2026-10-16.md).

## Completed human-led migration

Request `<private-record>`, scope `<private-record>`
revision1/event276, is Delivered. The live coordinator chose application tasks,
reviewed native worker results, adapted repairs and selected checks. Application
code came from Waywr-launched workers. After local exclusion of existing `tasks.json`
resolved the final dirty-tree blocker, the human used `/resume` (event528) and
reported the delivered TUI. Events529/542/543 record checked promotion/closeout.

Local main is `069a162fbcb118794515034d3ae91303e7c37135`; the worktree is clean,
and `tasks.json` remains present. Checks-43 passed on that exact commit: both
`python3 -B tests/check_python.py` and `python3 -B tests/check_migration.py` exited0,
with 44 tests and seven executable smoke scenarios, including isolated Python
operation without Cargo or a retained Rust binary. Saved stdout/stderr hashes were
verified read-only. No application tests or development were rerun at this checkpoint.

Continuous readable log (conversation, assignment instructions, outcomes, summary):

```text
~/projects/example-task-app/.waywr/history/20261016T135632Z-15a13c43b482.log
```

Adjacent `.summary.md` records the compact delivered state. The
[delivery assessment](../delivery-assessment.md) keeps this human-led request separate
from the original agent-prepared Rust run and records operational assistance.
This demonstrates one request through visible closeout, not every steering/question
interaction or unassisted reliability. Original project/evidence remain preserved.

## Limits and next action

Documentation links/anchors and diff checks are the relevant checks for this change.
No runtime code, dependency, compiler, service, binding or shared provider settings
changed. Existing limits: Linux/systemd and documented [sandbox setup](../linux-sandbox-setup.md),
pinned initial Codex profile, no nonempty fallback/general provider picker,
custom-state migration unsupported. Example Task App uses one writer; concurrent-writer
isolation and stronger power-loss durability remain unestablished. Wait for the
human's next request; no new job or approval is authorized on their behalf.
