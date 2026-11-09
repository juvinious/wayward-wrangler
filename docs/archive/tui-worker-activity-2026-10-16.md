> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Waywr handoff — per-worker live activity (2026-10-16)

## Current checkpoint

Continue on `impl/development-workflow`, now installed as `development-20261016-9`
(implementation `4456a79` plus request serialization `eff63fe`). The human confirmed
command/chat history and timed live output work, then requested a tree beneath
Working showing each running worker's title/ID and current operation, without output.
No new development request, trial, scope approval, push, merge or publication.

## Implemented

- Existing project-scoped `operation.list` accepts `active_only` and publishes a
  small activity status/label for running worker operations. It resolves the exact
  published host attempt even before a unit's attempt ID is saved. A bounded 256 KiB
  complete-frame tail tracks tool starts/updates/completions; terminal receipts clear
  commands. No model text, reasoning, command output, results or tool arguments.
- The production TUI polls once per second with one outstanding request, separate
  from output paging/selection. Under Working, tree bullets show each title (ID
  fallback) and command/tool operation. Missing/between-operation/stale details say
  so explicitly; metadata older than five seconds cannot retain a displayed command.
  Starts older than the bounded tail may lack detail. Reduced/off/mono motion remains
  unchanged. Idle/paused/unrelated or completed workers do not appear.
- PageUp/PageDown scroll live rows as well as chat; scrolling inside the live tail
  releases any old message anchor. Draft and command-history navigation stay intact.

Focused backend parser/operation-list tests and frontend row/output/scroll tests
passed; locked release build of all three binaries passed. Existing motion, message-anchor/resize and project-scoped output-paging regressions
also passed. Fixtures launched no native model, service or application development.
Dependencies/compiler unchanged.
Contracts are in [TUI](../contracts/tui-contract.md) and
[IPC](../contracts/session-protocol.md). Documentation/index assessed; existing index
already points to both owners. Prior recovery/install evidence is retained in the
[recovery checkpoint](workflow-recovery-2026-10-16.md).

## Installation and live request

Final implementation `eff63fe` is installed as paired `development-20261016-9`,
selected by `~/.local/bin/waywr` and bound to Example Task App at generation19,
upgrade revision527. Manifest SHA-256:
`f0fb37a6096cdb1f4475f062a7cb4897c5780560330ca254c171ba77efe86521`.
The earlier -8 bundle remains immutable but was never bound to this project.

The current native job continued independently while the feature was implemented.
Once workers and coordinator attempts settled naturally, the normal UI/outbox/host
settlement guards passed. Only this quiescent project service was stopped/upgraded
and restarted via the normal production TUI. Inspection and `/detach` sent no new
prompt, approval, resume or task assignment. The session stays running, generation22;
the existing workflow stays in attention: guarded local-main advancement found the
project root dirty (`?? tasks.json`). The user file and application refs/code were
not changed by this implementation. No automatic recovery was submitted.

Production TUI reconnection and installed IPC active-only/full listing were checked.
No workers were running after upgrade, so live worker rows were verified with focused
fixtures, not a new native development job. Exact approved proposals/decisions, 103
identity/config/receipt files and both existing readable-log prefixes match before/
after fingerprints. New receipts/history created by the existing coordinator were
preserved. Prior history, timestamps, motion and output paging remain intact.

The human-led migration `<private-record>` remains revision1/event276.
Its live coordinator chose tasks and consumed results independently;
this implementation only reads metadata. The requested checked local-main advancement
and full interactive closeout remain incomplete/unverified. Preserve source, refs, receipts and request history:

```text
~/projects/example-task-app/.waywr/history/20261016T135632Z-15a13c43b482.log
```

Original Example Task App orchestration audit remains in
[delivery assessment](../delivery-assessment.md); never call that initial seeded run
human-led. Shared provider configuration/credentials remain untouched. Existing
limits: Linux/systemd, pinned Codex profile, no configured fallback/general provider
picker, custom-state migration unsupported and documented
[Linux sandbox setup](../linux-sandbox-setup.md).
