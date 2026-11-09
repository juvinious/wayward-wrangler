> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Project session lifecycle — simulated implementation report

**Historical frontend evidence:** This report verified the earlier installed
three-pane release `m6-lifecycle-r10`. The accepted conversation-first renderer
now drives a later installed release; use the [unified UI report](../milestone-6/unified-ui-integration-report.md)
for current commands and checks. The retained r10 evidence is unchanged.

**Branch:** `milestone-6-interactive-shell`
**Scope:** project-scoped `/exit`, `/detach`, and `waywr resume` using the existing Rust frontend, Python service, LangGraph workflow and Herdr-visible fake workers. No authenticated agent calls, shared Herdr changes, application rewrite, merge or release publication.

## What changed

- The selected project's SQLite session metadata now stores a versioned phase (`running`, `exit-requested`, `draining`, `paused`), monotonic generation, exit ID, deadline and attention notice. Commands have durable IDs and revision preconditions. Exit/final-pause and resume use generation compare-and-swap; an older exit cannot stop a newly resumed session. Distinct stale clients receive a revision refusal, while replay of one command ID returns its recorded outcome.
- New worker stage reservation, Herdr launch and runner permit are fenced against exit under a project-local dispatch lock. An already-permitted worker may finish; its receipt and graph result are collected, then the next stage pauses. A saved human answer can be recorded while draining; the dependent stage waits for explicit resume. Uncertain execution blocks a new permit and has an inspectable notice.
- The service drains without blocking status or questions. A 30-second foreground deadline ends waiting, not execution. A verified live call retains its pane and gets an `attention-required` notice if it remains; ambiguous identity/evidence gets `execution-uncertain`. No lifecycle path kills or retries a worker. The runner's existing native ten-minute process policy is unchanged and was not runtime-tested here.
- A compact, versioned checkpoint projects task status, decision/request references, artifact hashes, open finding identities, selected context/grant references, limits, event cursor and next actions from canonical project records. Large projections fall back to canonical record references. Missing projections can be regenerated. Approvals and receipts remain authoritative in their original ledgers; the historical approved runs were not changed.
- `waywr resume` without a run ID routes to the project session; `waywr resume RUN_ID` retains legacy routing. The project bootstrap reuses a verified loaded systemd unit after a cooperative stop and waits for a prior stop to settle. `waywr session status` reads the durable phase/checkpoint even with the project service stopped. Existing release bindings remain project-local.
- The production Rust terminal shell now presents an explicit `/exit` confirmation panel, accepts `/detach` or Ctrl+Q, saves an unsent draft, and returns to its invoking shell when its owned UI pane exits. It signals only the foreground Herdr client launched by that invocation; it does not close the Herdr server, workspace or worker panes. A pasted slash command is literal. The separately accepted scripted evaluator was not replaced or modified.

## Verification

| Evidence | Result |
|---|---|
| Private Herdr client proof, `tests/lifecycle_herdr_client.py` | Herdr 0.9.3 foreground client returned to its shell on SIGTERM and SIGINT while the private server stayed up. Harness startup/socket-path failures were corrected and retained separately. Final proof: `.local/milestone6/lifecycle/herdr-proof-03/result.json`. |
| Focused Python regressions | 24 passed: lifecycle generations and stale replay after new ambiguous evidence, concurrent clients, live client shutdown, lost CLI reply, known-running/uncertain notices, project isolation, attach recovery and maintenance status. |
| Rust tests | 39 passed, one 30-second stress probe intentionally ignored. Includes explicit exit panel and route/draft checks; the accepted scripted UI tests remain passing. |
| Final paired installation | Local bundle `m6-lifecycle-r10`, manifest SHA-256 `b5989703252b050fc15e9d3973a9607c52c8efdc0ca270c6ad6fa7985ae42921`, installed at `.local/milestone6/lifecycle/install-r10/`. Python imports resolved from its private `site-packages`, not source checkout. |
| Installed fake workflow | Private Herdr server and disposable projects in `.local/milestone6/lifecycle/evidence/final-a17/`: active-work exit, draft completion and pause before critique, resume without duplicate draft, deliberate project-service crash/restart, identical run/task IDs in independent projects, B continuing after its UI pane closes, and resume superseding an old exit. Fake fixture answer/approval then rejected a stale SHA and retained the exact approved SHA through another service restart. This fixture approval is **not operator approval**. |
| Installed ordinary terminal | `.local/milestone6/lifecycle/evidence/final-fg07/`: visible `/exit` panel action returned to the shell, `waywr resume` restored the project session, `/detach` returned to shell with the session running, and the private Herdr server identity stayed unchanged. |

All 270 fake-workflow and 64 foreground-client copied regular evidence files were SHA-256 compared with their `/tmp` originals; each directory has a `copy-manifest.json`. The earlier failed fixture run `final-a14/` is retained: its harness attempted fixture approval while the task still had authoritative status `active`; waiting for `awaiting-input` fixed that test timing error without changing production code or historical data.

## Limits and retained resources

The final checks are simulated. The operator accepted the **isolated scripted UI** visual/input refinement, including full available conversation width, in the current Ghostty/Herdr setup; that observation does not validate the production lifecycle panel or native agents. The production shell still has its earlier compact diagnostic layout and Ctrl-S submission for task text. The exit/resume path and draft restoration need the operator's own terminal observation. The known-running/stalled and ambiguous notices have focused offline tests; a real indefinitely running worker after the full 30-second foreground deadline was not left alive for an installed test. Native provider context/tool isolation and authenticated acceptance remain blocked.

The final test-owned services and private Herdr servers were stopped. The project-local paired install, bundles and ignored evidence remain inspectable. Existing shared Herdr work, historical runs and approvals were untouched. The earlier milestone-6 `/tmp` preservation task, stable production installation work and final-build missing-workspace check outside this lifecycle addition remain separately pending.

## Fish hands-on walkthrough

Run from an ordinary Ghostty terminal to observe return to that shell. This uses only fake coordinator/workers; it creates a fresh project-owned Herdr workspace when you launch it.

```fish
set -g ww_app ~/projects/example-workspace/.local/milestone6/lifecycle/install-r10/bin/waywr
set -g ww_demo (mktemp -d /tmp/waywr-lifecycle.XXXXXX)
cd $ww_demo
$ww_app --yes
```

In the Wayward Wrangler coordinator pane, type `/detach`, then **Ctrl-S**. The interface closes; authorized work continues. From the returned shell:

```fish
$ww_app session status
$ww_app
```

To test an intentional pause, type `/exit`, press **Ctrl-S**, select **Exit session** with **Tab** and **Enter**. The shell prints the checkpoint reference and resume command. Inspect status, then resume:

```fish
$ww_app session status
$ww_app resume
```

For a fake active-work check, submit a small drafting request with **Ctrl-S**, inspect its proposed plan with the existing `:plan` and `:confirm` commands, then use `/exit` while a fake worker is running. Its Herdr pane remains inspectable; the service pauses before another stage. `waywr resume` reconciles that receipt and continues. Never use these fixture answers as consent for a real task. `waywr session stop` is the noninteractive graceful exit command for this project. The demo's `.wayward-wrangler/state/` contains the authoritative records; `session/checkpoints/latest.json` locates the compact handoff. Keep the demo directory if you want its evidence; clean it only after reviewing it.
