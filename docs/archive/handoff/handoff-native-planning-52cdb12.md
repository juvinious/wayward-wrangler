> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Current objective and branch

**Planning native conversational coordinator and worker integration; awaiting review.**
Branch: `plan/native-coordinator-integration`, created from synchronized `main`
and `origin/main` at **`2e5cf1212fe9a319b118c641c5028c851f66e86d`**.
PR #5 is verified merged
(2026-10-13). It squash-merged the full Rust migration/corrections and TUI-first
host work. Its tree matches prior branch checkpoint
`31273fd1085d3cfce0501d13b4a82ebc2ac7b829`; original individual commits remain
historical evidence, not ancestors of the squash. Initial working tree was clean.

The [native integration proposal](native-coordinator-integration-proposal.md)
is the only new design under review. This pass inspected source, installed CLI
help/versions and primary documentation; it changed documentation only. No
application tests, authenticated calls, services, installations or retained
projects were changed. Planning is complete; **implementation and native spend
are not authorized**. Do not push, merge or publish this planning branch.

## Current implementation and proposed additions

Implemented: standalone Rust/Ratatui frontend and Rust backend; Tokio I/O,
bounded SQLite writer, durable commands/questions/approvals, scoped context,
receipts and recovery. New schema-3 projects use operation-scoped systemd user
services. `waywr init .` / `waywr init SOME_DIR` initialize locally and return;
`waywr` opens the accepted TUI in the invoking terminal. `waywr resume`
reconciles first. Herdr is optional explicit read-only inspection. One executing
workflow per project; project identity, state, budgets and services are separate.
No global installation/default candidate is implied by ordinary command examples.

Current limits: submitted text creates a deterministic task proposal, not a
conversation. Service plans accept fake roles only and CLI native admission is
blocked. Rust Claude/Codex adapter helpers exist, but native Rust-host execution
and tool/context isolation are unverified. Task artifacts are Markdown; the
accepted simulation built no weather application. Do not bypass these gates.

Proposed: bounded model-backed discussion independent of tasks, application-
validated actions, consultation, steering at undispatched boundaries, durable
conversation context and separate budgets. Reuse the accepted UI and host.
Recommend packet-only roles returning structured file bundles for a real static
weather page, rather than enabling unrestricted repository tools. Proposed schema
4/minor-2 capabilities are **not implemented or approved**. No existing state is
upgraded automatically. The proposal identifies concrete reuse, gaps and gates.

## Decisions and exact next action

Stop for operator review of the proposal. Recommended defaults are:

1. Tool-free CLI roles and application-validated static files for the first trial;
   trust the provider client/authentication, without claiming whole-process isolation.
2. Explicit coordinator changes only between resolved calls; no automatic fallback,
   ambiguous replay, worker model changes or scope/cost escalation.
3. A separately authorized `native-weather-01` trial: Claude `claude-sonnet-5-5`
   medium coordinator/author, Codex `gpt-6.1-sol` high consultant/reviewer;
   at most 6 conversation + 1 consultation + 4 worker calls, two reviews.

After implementation approval, first prove the pinned CLI's effective tools,
automatic context and customization exclusions using source/config inspection and
credential-free fixtures. Keep native admission closed for unexplained reads,
tools, fallback, policy or authentication conflicts. Then follow the proposal's
five bounded checkpoints. Native-call authorization remains a separate gate;
actual clarification, scope decision and final approval stay with the operator.
No new provider, broad model picker, automatic discovery, TUI prototype or Python
runtime is planned. If OS confinement of the trusted client is required now,
that is a substantive boundary decision before native admission.

## Evidence and verification boundary

Installed help/version inspection: Codex **0.162.1**, Claude Code **2.1.296**;
these match current adapter pins. Ignored captures/hashes and nonsecret config
category inventory: `.local/native-coordinator-planning/2026-10-13/`.
Existing user hooks and Codex plugin/MCP config make exclusion verification
necessary. No credential contents were recorded. Auth validity, managed remote
policy, model entitlement, residual tools and native interruption remain unknown.
Archived milestone-5 CLI trials used Python/LangGraph/Herdr and explicit recoveries;
they do not establish current native support or supply fresh authorization.

The operator accepted **v3 simulated** workflow and requested discoverability,
answer/review and scrolling corrections in a physical terminal. This is separate
from automated verification. No working weather app or native agent was verified;
physical v3 detach/resume remains unreported. See the
[current acceptance report](tui-first-checkpoint-4-report.md) for exact test counts,
IPC minor-1 reconciliation, retained evidence and cleanup inventory. No acceptance
suite was rerun for this planning task.
Planning checks passed: changed-document local links, whitespace, archived handoff
byte identity, and equality of the merged tree with the prior topic checkpoint.

## Preserved candidate, evaluation and resources

Accepted candidate:
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`.
Manifest SHA-256: `c0bf66e31579cb44a04cb97f3701ff47ccaff4fd74dedf1190445240e1e0e5a1`.
Reusable project: `.local/supervisor-owned-execution/discoverability-hands-on-v3/project/`,
key `<private-id>`, bound to v3. Preserve its approved fake
run `<private-record>` and all original records.

Last checked at prior closeout: v2/v3 project services paused/inactive, six completed
fake-call receipts each, no owned workers; seven exact automated fixture services
inactive. Their operation-unit metadata remains retained. A v3 foreground TUI was
left attached to its operator terminal; **not rechecked this pass**. Do not stop
it or modify services for planning. Prior detailed inventory remains ignored at
`.local/supervisor-owned-execution/discoverability-cleanup-2026-10-13.json`.

If the operator wants to reopen the retained evaluation, first detach any existing
foreground UI, then in Fish:

```fish
cd ~/projects/example-workspace/.local/supervisor-owned-execution/discoverability-hands-on-v3/project
../waywr.fish resume
```

The wrapper pins the release/private environment. Do not reinitialize or rebind it.
Also protect v2, earlier `.local/supervisor-owned-execution/hands-on/project/`, the
separate blocked legacy `~/projects/waywr-rust-sim-eval`, r9 baseline,
`decision-log-01`, `context-01`, `m2-cli-01`, `demo`, shared Herdr and unrelated
services. Keep historical bytes/hashes/approvals and all referenced releases.
Unfinished LangGraph/Herdr operations retain their bound contracts. Never replay
uncertain execution or restore old state over newly accepted work; release rollback
requires compatible state and reconciled exact project/operation ownership.

## Reading order

1. [Documentation index](README.md) and [native proposal](native-coordinator-integration-proposal.md).
2. [Architecture](architecture.md), [state](state-contract.md), [IPC](session-protocol.md).
3. [Accepted TUI contract](tui-contract.md), [acceptance report](tui-first-checkpoint-4-report.md).
4. [Approved host/recovery contract](tui-first-optional-herdr-proposal.md).
5. [Legacy blocked-task boundary](bootstrap-refusal-correction.md) or
   [archived native report](archive/milestone-5/milestone-5-report.md) only as needed.

[Pre-planning handoff](archive/handoff/handoff-pre-native-planning-2026-10-13.md)
is a byte-preserved historical snapshot; its PR status and next instructions are
superseded here. Relative links in that snapshot reflect its original `docs/` location.
