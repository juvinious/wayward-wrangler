> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Existing demo workspace rebind

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

The operator approved rebinding the existing demo to example-workspace (w8).

## What changed

- New CLI command: awp rebind RUN --workspace ID.
- An append-only application routing record selects the workspace for future operations.
- The worker graph node reads the effective target when preparing a new operation.
- Original frozen envelope, completed operation bindings, claims, receipts, artifacts, graph checkpoint and human decisions are not rewritten.
- Status displays the effective workspace and distinguishes closed recorded panes from present panes.
- Explicit workspace arguments on an existing interactive demo are no longer silently ignored.

The rebind is limited to the same Herdr socket, an unanswered human gate, completed validated operations and no live controller/runner/child. It refuses in-flight, uncertain or answered work. The controller lock serializes the operation. Repeating the same destination is idempotent; JSON export can be regenerated from the committed routing record.

## Verified for this demo

Future target is w8. Original file hashes and full graph state matched before/after. Draft and critique-1 invocation counts remain 1. D-retention remains unanswered. No new run, worker call, graph transition or milestone-2 work occurred.

The existing controller was reopened at w8:t2 / w8:p2. Its actual terminal output shows the input prompt. Codex remains at w5:p1.

Eleven offline tests passed, including preservation/idempotency, future operation target selection, and refusal of answered gates, incomplete operations, live workers and existing controllers. No live worker was launched to test the new target; the next stage will occur only when the operator answers the demo.

Evidence: .local/evidence/demo-before-rebind.json, demo-rebind-verified.json, and .local/runs/demo/routing/rebind-0001.json.

## Reuse

Run rebind only when all preceding execution has finished, all retained runner/controller processes for that run are closed, and a human choice is still unanswered. Reopen the existing controller with awp demo --run RUN --workspace ID in a Herdr shell. Rebinding does not recreate closed historical worker panes.
