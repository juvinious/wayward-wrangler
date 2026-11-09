> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Rust migration corrective closeout

2026-10-13 · `refactor/rust-backend` · simulated and offline verification only. The [review addendum](rust-migration-review-addendum.md) remains the finding record. This report records the corrections to its runtime and CLI blockers; the original Stage-5/6 passes were **partial coverage**. No operator project or service was cut over, and no authenticated agent worker ran.

## Installed candidate and evidence

The paired candidate is `.local/rust-migration/corrections/final-v7-install/releases/rust-corrective-final-v7/bin/waywr`; manifest SHA-256 `fdb638ac3d922a7cbbb38587deed883c42ad406713b18848e30c7cb4d006d392`. Its three installed binary hashes match the post-commit `target/debug` binaries. It includes the Rust frontend, service, runner, prompts and schemas, with no application Python runtime. The earlier Stage-6 installation and preserved r9 comparator remain in place. The candidate did not replace the default executable or any project release binding.

- `final-v7-service/{summary,cleanup}.json`: private Herdr, installed TUI, active-work exit and restart, one receipt, missing-workspace routing, two isolated projects, one choice and exact candidate approval. The same untraced installed path also passed twice for `rust-corrective-final` before the native preflight correction.
- `final-v7-cli/{summary,cleanup}.json`: installed Rust and separate installed r9 fake fixtures. It compared run targeting, artifact/transcript bytes, record-only answer, exact-hash approval, cancellation, and replay without a second invocation. The fixture also exercised private plugin installation, worker focus, service controls, quiescent location rebind, refusal of changed frozen context, and status/replay of the exact saved CLI command ID.
- `runner-regression-01/summary.json`: seven private-Herdr cases, including verified cancellation, surviving child uncertainty, server identity change, saved receipt reuse and r9 result normalization. `runner-flood-06/summary.json`: interleaved stdout/stderr, 32 MiB combined transcript cap, one failed receipt and identical replay, one invocation, 39,752 KiB peak runner RSS. Private resources were stopped by the fixture.
- `final-v7-cargo-test.log` and `final-v7-clippy.log`: locked/offline workspace tests and Clippy passed. Clippy retained the previously documented TUI/module-order style exclusions. Parser and mock native-auth unit tests passed; no installed native CLI authentication check was executed.
- Retained failed attempts `installed-c1-01`, `runner-flood-01..03`, `final-installed-service*`, `diagnostic-service-01`, and `final-v4-cli` are not rewritten. The diagnostic UI failure exposed a Herdr tab-list/layout-export race; its saved `ui-launch-errors/*.json` names `layout_not_found`. The v4 CLI run exposed a stale-state refusal that `session stop` had printed as success. `final-service-pass-01`, `final-service-pass-02`, `final-v3-service`, and `final-v7-service` passed without tracing after the bounded identity-scan fix.

All relative evidence paths above are under ignored `.local/rust-migration/corrections/`. The private Herdr servers and Rust test services for the passing final fixtures report stopped in their `cleanup.json`. Existing operator services and shared Herdr were not touched.

## Runtime correction and limits

The project service now uses a two-worker Tokio runtime for Unix sockets, signals, client shutdown, timers and scheduler timing. Its blocking pool is limited to 16 concurrent dispatches; connections are capped at 64. Synchronous Herdr/policy work runs in the blocking pool, and runner child stdout and stderr are read by Tokio tasks into a 16 × 64 KiB channel. The runner retains at most 32 MiB combined transcript bytes, terminates only its verified child on overflow, records `output-limit.json` plus a failed receipt, and never automatically retries. Pane display writes have a 500 ms limit; a stall records `display-degraded.json` while the transcript remains authoritative. Sources: `backend/src/{session_service,runner,herdr}.rs`.

One project-owned SQLite writer thread accepts at most 64 queued jobs through nonblocking admission. Live mutations use that writer; snapshot connections read WAL independently. SQLite transactions remain short, with a 250 ms busy timeout. A full queue or locked database yields a typed busy response before command acceptance; writer loss instructs lookup by the original command ID. Service exit signals clients, drains or records attention, then stops the writer with a five-second deadline. Startup schema checks occur before socket acceptance and remain synchronous. Sources: `backend/src/{db_writer,session_store,workflow_store,session_service}.rs`.

The predeclared bounds in [migration log](rust-migration-log.md) were status/command response under one second with 32 slow clients, SQLite busy under 500 ms, independent status under one second during an eight-second Herdr stall, pending pipe chunks at most 1 MiB, and transcript at most 32 MiB. Focused Rust tests assert those thresholds, bounded shutdown, concurrent answer/cancel/exit with responsive snapshot, idempotent command replay, and preserved uncertain outcomes. They do not benchmark an authenticated asynchronous workload. The output-flood fixture measured runner peak RSS; it does not establish a whole-service memory ceiling.

The final `--nocapture` test log reports observed times of 10 ms for 32 slow-client status, 253 ms for SQLite busy, 3 ms for status during stalled Herdr I/O, 1 ms for the answer/cancel/exit snapshot, and 501 ms for the intentionally blocked pane display write. These are observed samples from the named fault cases, not production latency percentiles.

The Herdr startup race came from `tab.list` including a tab that closed before `layout.export`. `Herdr::find` restarts the complete identity scan at most four times on `layout_not_found`; it never treats a missing target pane as successful identity evidence. A mock-socket unit test reproduces the race; the normal installed path passed afterward. A launched UI that still fails now preserves a local `ui-launch-errors/<token>.json` diagnostic without changing task authority.

## CLI checklist against r9

The parser is `frontend/src/cli_schema.rs`; handlers are `frontend/src/{rust_cli,rust_review,rust_maintenance,rust_runtime}.rs`. Clap validates syntax before project registration, service startup or mutation. The parser unit tests cover valid forms and misspelled, missing or unsupported flags. The installed differential covers the ordinary behavior named above, not every row below. **P** means preserved for Rust-owned schema-2 projects, **C** means an explicit changed contract, **L** means intentionally restricted to a project explicitly bound to r9.

| r9 form and flags | Disposition on a Rust-bound project | Evidence / effect |
|---|---|---|
| `waywr`, `waywr resume`; `wayward-wrangler`, `awp` | P | Installed TUI/service flow and release aliases; project session reconnects. |
| `init`; `location-rebind` | C / P | `init` creates Rust schema-2 state; quiescent relocation appends registry provenance and retains approval bytes. Installed CLI fixture. |
| `preflight --run --brief --profile/--config --context [--workspace --socket]` | C | Version and status-only authentication checks have a 15-second bound; explicit workspace must match the project binding. Native admission remains explicitly blocked by the isolation gate. Fake preflight is allowed. Mock CLI test only; real authentication untested. |
| `integration herdr install [--destination --socket]` | P | Old spelling and new `install-herdr` route to the same isolated plugin handler. Private plugin fixture. |
| `open RUN --artifact STAGE [--sha256]`; `open-link` | P / C | Exact artifact selection and hash validation restored; v2 project-keyed links reject ambiguous legacy links. The earlier Stage-6 private plugin fixture covered links; manual Vim opening on this build remains unobserved. |
| `rebind RUN --workspace` | P | Service verifies quiescence and owned workspace before appended routing. Installed CLI fixture. |
| `start [--run --brief --profile/--config --context --workspace --socket --authorize-native]` | C | Explicit fake run, settings and authorization, then asynchronous project-service continuation. Frozen paths, context hash, target and command replay with the same ID; changed input is refused without another call. Native start stops at the isolation gate. Installed CLI fixture. |
| `demo`, `open-demo`; `probe-*`; `adopt-*` | L | Trial-specific entry and recovery remain with unfinished LangGraph runs on explicitly bound r9; Rust refuses rather than converts checkpoints or retries. Parser recognizes `--exit-at-choice`, `--record-choice-and-exit`, `--owner-generation`, `--owner-token`, probe `--seconds/--mode/--detach`, and adoption `--interpretation` only to give a clear compatibility refusal. |
| `supervise RUN [--socket --interval --max-attempts --once]`; `supervisor-install RUN --socket` | C | Same project service owns supervision; no second run-level service is installed. Default interval/attempt values are accepted; custom values fail clearly before side effects. Private CLI fixture covers `--once`/install. |
| `notices RUN`; `resume RUN` | C | Run-targeted task plus project attention; `resume RUN` releases a held record-only answer, while bare `resume` reconnects the project session. Installed record-only fixture. |
| `status [RUN] [--watch --json]`; `workers RUN`; `show RUN [--artifact STAGE]`; `logs RUN --operation STAGE`; `observe RUN --operation STAGE` | P / C | Selection, watch and verified saved artifact/transcript bytes restored. Diagnostics are always JSON where structured; `--json` is accepted. `observe` verifies pane/server identity before focus. Installed CLI fixture; `--watch` syntax has parser coverage. |
| `cancel RUN --operation STAGE` | P | Stage must match active worker; cancellation routes through project service and retains one receipt. Installed private fixture. |
| `decide/approve/reject RUN --request` with `--answer-file`, `--sha256`, `--reason-file`, `--record-only` | P | Project-local files, exact pending request/hash, immutable record-only answer and later single release. Installed Rust/r9 differential. |
| `session status/stop/bind/upgrade/rollback/recover-coordinator` | P / C | Project authority, release checks and prelaunch-only recovery; output shape differs. Stop retries only an explicit stale-state refusal, up to four times. Stop and recovery passed isolated service fixtures; prior Stage-6 release-routing fixture covers upgrade/rollback, not repeated here. |
| `session command-status ID`, `session command-replay ID` | Added diagnostic | Each CLI mutation saves its exact command in a private project-local file before delivery. Status queries the authoritative service ledger; explicit replay uses the saved ID/body and cannot duplicate an accepted command. Installed fixture replayed a completed start with five invocations unchanged. |

Global `--project`, `--state-dir`, and `--registry-dir` retain explicit selection. `--color auto/never` is accepted for plain diagnostics; `--color always` and Rust-bound `--legacy-origin` fail clearly. `--release` is confined to maintenance. New `--herdr-socket`/`--ipc-dir` select verified integration paths. Unknown flags and invalid stages fail before side effects. The installed Rust fixture records exit codes and stdout/stderr for each exercised command. No Rust-bound command silently invokes Python; `legacy-inspect RUN --state-dir DIR` is read-only, and incompatible unfinished state routes only through its bound release.

Rust-only `bundle-build` and `bundle-install` retain the paired release path, while `--backend-info`, `--protocol-probe`, and `--lost-reply-probe` are internal diagnostics. Internal `--socket`, `--project-key`, `--release-id`, `--outbox-dir`, `--title`, and `--ui-token` remain typed launch arguments; `--yes` is the explicit noninteractive setup flag. They are parsed strictly and do not replace ordinary CLI controls. The full source parser covers their argument shape; the final installed fake fixtures did not execute every diagnostic or physical Vim action.

## Remaining acceptance boundary

This closes the concrete simulated runtime and ordinary-CLI gaps from the addendum. It does **not** establish native agent context/tool isolation, authenticated execution, physical Ghostty/Herdr behavior for this candidate, or unrestricted conversation/provider switching. Native admission remains blocked. Custom supervisor timing is an explicit changed contract; if operator scripts rely on nondefault values, they need a project-service configuration decision rather than a hidden second scheduler. The final build's standalone resource layout matches the prior Stage-6 bundle, but source-checkout-denied execution was not rerun for this corrective candidate.

## Fish hands-on check

From an ordinary terminal, use a new disposable project. This launches only the packaged fake coordinator and fake workers; their proposals and answers are scripted, not model interpretation.

```fish
set ww ~/projects/example-workspace/.local/rust-migration/corrections/final-v7-install/releases/rust-corrective-final-v7/bin/waywr
set project ~/projects/waywr-rust-sim-eval
mkdir -p $project
$ww --project $project init
rg 'provider = "fake"' $project/.wayward-wrangler/workflow.toml
cd $project
$ww
```

Type a short decision-log specification request, inspect the fake proposal, and use its visible authorization action. Watch the separately identified Herdr worker panes, answer the displayed choice, and inspect the exact final artifact before deciding. Use `/detach` to close only the interface, then run `$ww resume` in the same project; use `/exit` to request a drained session checkpoint, then `$ww resume` to continue. `$ww session status` and `$ww status` provide terminal diagnostics. Retain the disposable project's evidence until the hands-on check is complete.
