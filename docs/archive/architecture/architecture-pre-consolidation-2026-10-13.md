> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Workflow architecture

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](project-migration.md).

The original protocol below is retained. Milestone 2 added restricted native CLI
adapters and structured stdin; context continuity added durable agent questions.
The [milestone-3 report](milestone-3-report.md) records the tested recovery matrix.
Milestone 4 adds explicit project/state bindings and packaged runtime resources;
see the [installed acceptance report](milestone-4-report.md).

Wayward Wrangler is the standalone application and intended primary TUI. An
enclosing Codex or Claude session is not part of its runtime. Herdr supplies
visible execution panes. A model-backed coordinator will handle conversation,
planning, delegation and steering; a deterministic supervisor owns durable
process lifecycle, reconciliation, scheduling and notifications. The application
owns project sessions, tasks, decisions, approvals, artifacts and execution
records. Provider-native sessions are optional continuity aids. A future
coordinator-provider switch must use a bounded, provenance-linked handoff
without restarting workers, reopening human gates or changing frozen worker
models. The current bounded supervisor and deferred TUI boundary are detailed
in [supervision design](supervision-design.md).

## Ownership

| Component | Owns |
|---|---|
| Herdr | Actual PTY/pane process lifetime, rendering, navigation and terminal access. |
| herdr_backend.py | Supported socket calls, targeting, discovery, binding and dispatch permit. |
| runner.py | One child, execution claim, identity, live stream capture, cancellation, receipt and retained terminal. |
| artifacts.py / records.py | Immutable packets/artifacts, hashes, findings/decisions, stage reservations and one active run. |
| workflow.py | One LangGraph graph, conditional transitions, public SQLite checkpoints, interrupts and final outcome. |
| cli.py | Controller commands, interactive operator input and observation. |
| runtime.py | Selected project/state identity, state marker, user-local location index and explicit quiescent relocation. It stores no decisions or workflow progress. |
| integration.py / review.py | Copied Herdr plugin binding and project-qualified, hash-checked review routing. |

A generic graph worker node reconciles its stable stage operation before executing. This is not a second workflow scheduler. Milestone 4 keeps Herdr as the sole execution integration and accepts only fake workers for new runs.

The installed application is read-only runtime code plus packaged skills and plugin
assets. `--project` selects the target; `PROJECT/.wayward-wrangler/project.toml`
supplies a stable key, and `PROJECT/.wayward-wrangler/state/` is the default store.
Configuration and explicit context selections are target-owned/versionable.
The user-local location index routes Herdr links and admits only one unresolved
run across its registered stores; application tables, LangGraph checkpoints,
receipts, decisions and artifacts stay in the selected store. `--state-dir` and
`--registry-dir` are explicit overrides. A relative context snapshot survives a
quiescent disposable move; a changed selection or document fails validation.
Legacy snapshots can be checked in their frozen path namespace through an
explicit `--legacy-origin`, without modifying the historical envelope.

## Graph and packets

Draft → critique 1 → revision 1 → critique 2 → revision 2 → final review. Critiques needing no change can skip to final review. Substantive choices use separate preparation/interrupt nodes.

Packets include the frozen brief/rules/role skill, candidate, accepted decisions, finding dispositions and exact preceding critique. Critique 2 also gets the previous reviewed draft and a deterministic diff. Author session history is not passed to the critic.

Artifacts record kind/version, operation/input identity, relative path, SHA-256 and parents. Decisions are append-only application records exported to Markdown/JSON. Stage reservations derive the review count; no separate mutable counter can drift.

## Execution protocol

1. Controller lock serializes graph mutations; a unique active-run index prevents concurrent workflows.
2. Persist the packet/operation and a may-have-been-sent launch record before calling Herdr.
3. Herdr launches argv directly in a new one-pane tab. No replacement tab, terminal typing, hidden worker or log-tail execution pane.
4. Runner claims its operation lock and writes boot/PID/start-time plus actual Herdr IDs, then waits visibly for a permit.
5. Controller verifies one matching launch token, server process identity from socket peer credentials, live runner and pane/terminal binding. It publishes a claim-bound permit.
6. Runner records dispatch before launching its child. The fake increments its durable invocation counter and streams structured events.
7. Validated result/receipt publication is flushed to disk. Runner retains the original terminal after child completion.
8. LangGraph adopts the receipt and advances with synchronous durability.

A lost creation response is never retried automatically. Unresolved identity becomes uncertain. API IDs are correlation values, not idempotency keys. Linux boot/start-time checks guard against PID reuse.

Receipt publication and graph checkpoints are separate commits. Ordinary replay uses the receipt; selected hard-failure boundaries are now tested in milestone 3; see its limitations. The runner cannot advance the graph.

The runner drains actual stdout/stderr into the pane and transcript; fake child stdin is disabled; native adapters supply a frozen prompt file and EOF. Pane-read is used only in acceptance tests to verify visibility, never as authoritative workflow state.

## Decisions and approval

A stable request is prepared before the dedicated interrupt node. Response commands validate current request, artifact bytes/hash and decision revision, commit the answer, and normally continue. Record-only mode exposes the commit/consumption boundary.

Resume feeds an already saved response through public Command/resume. Consumed answers are not resubmitted. Identical response repetition is idempotent; conflicting answers fail. Workers cannot produce authoritative approvals.

Interactive demo input is controller input. Worker terminal text is not. Recorded username is local OS provenance, not proof of human authorship; automated acceptance responses are labeled fixtures in the report.

## Retention and limitations

A sleeping runner retains its real terminal after child completion/failure/cancellation, because direct command panes otherwise close when the command exits. Closing a completed tab ends this process. Full transcripts remain local; no deletion automation exists.

Cancellation records target one claim; the runner verifies child identity/group before signaling. Manual pane closure or runner death may require uncertainty. Same-user files/prompts are not a hostile-process security boundary.

Milestone-3 tests use no actual model CLI or provider authentication; earlier native
trials and their restrictions remain documented in the milestone-2 report.

## Sources and versions

Herdr installed 0.9.3; inspected source commit 7b116c05bfda646af39d2524c54e70c751f57ee8:
[layout handler](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/src/app/api/layouts.rs),
[pane runtime](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/src/pane.rs).

Public LangGraph [interrupt semantics](https://docs.langchain.com/oss/python/langgraph/interrupts) and [persistence](https://docs.langchain.com/oss/python/langgraph/persistence) underpin the graph. Installed graph/checkpoint/SQLite-saver versions are 1.2.14/4.2.0/3.1.1.

Milestone 3 adds a second server identity check immediately before permission and
refuses missing launch intent when execution evidence exists. Read-only operation
diagnostics expose uncertain execution and the operator action without changing
graph progress. Milestone 4 passes the selected project/state identity to the
installed runner; its fake child uses the installed interpreter, without source
PYTHONPATH. A project key and protocol version are frozen in new operations.

Still unproven: native-process fault permutations, automatic orphan collection,
arbitrary crash windows, power loss and relocation of active processes. Selected
controller/publication, cancellation and isolated server replacement boundaries
are runtime-tested; this does not establish automatic recovery from every uncertain state.
