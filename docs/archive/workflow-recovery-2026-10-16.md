> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Waywr handoff — workflow recovery and live activity (2026-10-16)

## Objective and evidence

Continue on `impl/development-workflow`. The human ran `waywr resume` and observed
no progress, then requested local activity timestamps and a subtle elapsed-time
running indicator. Their migration scope `<private-record>`, revision1,
remains authorized (event276). No new request, approval, trial application, push,
merge or publication is authorized. Recover this existing blocked workflow and let
its live coordinator choose further actions; do not prescribe application tasks.

Session resume succeeded (event328, generation22), but the workflow independently
remained in attention. Its integration HEAD is `80ef71b75bdf26c88be97a33df47326126a20989`.
Six untracked `__pycache__/*.cpython-312.pyc` files block the exact clean-tree guard.
Checks-7 was refused before execution because `python3 -B tracked-script` was not
recognized. The next native coordinator decision chose another check, but clean-tree
validation stopped it. Cache provenance is not inferred from filenames/timestamps.
All source, refs, worktrees and frozen receipts must be preserved.

## Implemented correction

- `/resume` distinguishes paused session lifecycle from an already-running session's
  selected operationally blocked workflow. Workflow recovery binds exact work/revision
  and current scope/policy. Saved questions require `/reply`; pending execution refuses.
- Recovery can retain only untracked CPython cache paths beside tracked Python source.
  Immutable manifests and exact bytes/hashes are recorded under the recovery operation;
  only then are those paths removed from integration. Replay/crash reconciliation uses
  the same manifest. Source edits, unknown files and symlinks remain fenced/preserved.
- Python tracked-script checks accept `-B`. Check subprocesses suppress bytecode writes;
  native development clients receive the same environment setting. Existing shell/interpreter
  command boundaries stay intact. Earlier refusals/receipts are never rewritten.
- Event/message snapshots expose committed sidecar timestamps. Headers show local
  `HH:MM` today or `MM/DD/YY HH:MM` for older events; missing times stay unavailable.
  Current coordinator/check/worker activity gets an elapsed-time line from saved
  operation/reservation metadata. Only that line shimmers in subtle mode; reduced/off/
  mono remain static. Idle, paused, blocked or unrelated work never falsely animates.

Checks passed: all15 development-module tests, including bytecode suppression, `-B`,
retention/replay and source/unknown-file/question refusals; operator recovery authority;
continuous coordinator regressions; frontend resume/time/motion regressions; all11
continuous-view tests; anchored chat scrolling/resize; locked release build of all
three binaries. Fixtures launch no native model. Prior command history and scope
selection repairs remain intact; no dependency/compiler changes.

## Installed checkpoint and observed continuation

Implementation commit `983fe38` is installed as paired release
`development-20261016-7`, selected by `~/.local/bin/waywr` and bound to Example Task App
at generation18 (upgrade revision328). Manifest SHA-256:
`e9479832ddb63578a4d95a80f38b35ae0e97679028c608a67c2024fb1ad9a271`.
After the human detached, only the idle project service was stopped/reconnected;
normal UI/host-settlement guards passed. The running lifecycle was preserved.

In response to the human's failed resume report, the implementing agent invoked
production TUI `/resume` for this same approved workflow. This was operational
recovery, not a human conversation, scope approval, new development request or
externally supplied task sequence. Event329 records workspace recovery. Exact
six-file bytes/hashes were verified in:

```text
~/projects/example-task-app/.waywr/state/session/development/<private-record>/cache-recovery-b0f4e6f8963bef57/manifest.json
```

Approval/scope and old readable-log prefixes match pre-recovery fingerprints.
Integration was recovered at its saved HEAD without application-source edits by
the implementing agent. The live coordinator then chose a new repair worker;
`foundation_check_repair` submitted `4db4f87619a28bdfac4794aa3e8327a1a93a6cee` and was
integrated at `791264d1a8b8f1e07013341eed92c02d013ebaaa`. The service's actual
`python3 tests/check_foundation.py --rust-reference` check passed at that exact HEAD.
The workflow is running; its full migration/acceptance/closeout is not yet complete.
No application sequence was supplied externally.

Production TUI inspection confirmed local activity times, the changing elapsed
counter/shimmer and real coordinator/worker activity. The implementing agent detached,
leaving the existing approved job running for the human to observe/steer. Unknown
source edits/files still require explicit reconciliation; generic resume cannot
answer a substantive question, bypass authority or replay uncertain execution.
No push, merge or publication of Waywr occurred.

Readable current request history remains:

```text
~/projects/example-task-app/.waywr/history/20261016T135632Z-15a13c43b482.log
```

Full interactive development/adaptation/closeout remains unverified; human-led planning,
approval and worker execution have occurred. [Assessment](../delivery-assessment.md)
owns the original run audit. Shared provider configuration/credentials remain untouched.
Current limits remain Linux/systemd, pinned initial Codex profile, no nonempty fallback/
general provider picker, custom-state migration unsupported and the documented
[Linux sandbox setup](../linux-sandbox-setup.md).
