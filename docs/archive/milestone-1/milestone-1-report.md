> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Milestone 1 acceptance — October 10, 2026

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

**Milestone 1 passed with simulated workers.** Update: the operator subsequently completed the hands-on demo and approved its final artifact. All five demo stage counters are 1. See [current usability outcomes](../herdr-legacy/terminal-review-outcomes.md) for the recorded choice/approval and color/review-link follow-up. The evidence below describes the original acceptance checkpoint.

No authenticated Claude/Codex workers, shared configuration changes, Git initialization, another project application changes or assessment edits were made.

## Observed results

| Requirement | Runtime result |
|---|---|
| Actual visible workers | Direct argv launch created separately labeled Herdr execution panes; the supervisor and child ran there. |
| Streaming | Pane output contained step 1 while the fake child was live and no receipt existed. |
| Retained output | Completed and deliberate exit-17 failed workers remained inspectable in their original panes. |
| Controller exit/reconnect | Fresh controller reused the surviving runner/pane; invocation counter stayed 1. |
| Targeted cancellation | Only the selected delayed child exited (-15). Its pane stayed; controller and two other retained runners/panes remained alive. |
| Automatic handoffs | Five stages exchanged versioned artifacts. Revision received critique/choice; critique 2 received previous version and delta. |
| Durable choice | Record-only answer was consumed by a fresh process once; no repeated question or completed calls. |
| Duplicate prevention | Second controller and delayed duplicate runner were refused; valid receipt reused. |
| Review/authority limits | Third critique, worker-authored approval, conflicting answer and stale hash were refused. |
| Durable approval | Approval saved before graph consumption survived fresh resume; repeated resume/approval caused no calls. |
| Offline boundaries | Six unittest checks passed for immutable publication, hash/path integrity, worker authority, safe IDs and prompt graph shutdown. |
| Dependencies | Project-local pip check passed. |

The acceptance run made exactly one call for each of draft, critique-1, revision-1, critique-2 and revision-2.

Machine-readable evidence:
- `.local/evidence/foundation.json`
- `.local/evidence/milestone1.json`
- Stage packets, invocation counters, manifests and receipts under `.local/runs/acceptance-m1/`

The automated approval is a **simulated test response**, not operator approval of the hands-on draft or new implementation work. Test controller output remains in Herdr.

## Environment and adjustments

Linux; Python 3.12.3; Herdr client/server 0.9.3, private protocol 22. LangGraph 1.2.14, checkpoint **4.2.0**, SQLite saver 3.1.1.

Assessed source checkpoint 4.3.0 was unavailable from published packages. Official package requirements permit 4.2.0; it resolved and passed the runtime tests. All installations are project-local.

Direct argv launch worked; no shell-input fallback was needed. Sleeping retention processes deliberately keep completed terminal contents available. [Inventory](../herdr-legacy/retained-session.md).

Packets live in each operation directory rather than a duplicate separate packet directory. Visibility was verified with supported Herdr pane/process APIs; final operator usability acceptance remains yours.

## Unproven

Milestone 2 remains unimplemented: real Claude/Codex interfaces, authentication, model behavior, streaming, permission restrictions and native integration hook isolation.

Milestone 3 remains unproven: every hard-kill/publication boundary, lost-create response races, surviving orphan collection, full Herdr server restart/handoff, cancellation racing with publication and power-loss durability. Conservative checks exist, but the complete fault matrix has not been certified.

No claim of exactly-once remote billing/actions, hostile same-user isolation, concurrent runs, parallel scheduling, automatic retry or Git/PR automation.

No further implementation scope has been inferred. Review the hands-on experience before deciding on real workers.

## Final controller shutdown check

Installed LangGraph waits for its background tasks on executor exit. A cooperative stop-observing event now releases the graph node on Ctrl-C without cancelling its external worker. A subprocess test against the installed executor passed; the controller exited promptly. The hands-on controller was then restarted with this code while awaiting input: its pending decision and existing worker pane IDs/counters were unchanged. This supplements the real Herdr worker-survival test; it does not certify every hard-kill race.
