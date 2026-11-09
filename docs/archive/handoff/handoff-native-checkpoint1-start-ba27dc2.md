> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Active task and authorization

**Native conversational coordinator/worker integration: planning complete,
awaiting operator review.** The [proposal](native-coordinator-integration-proposal.md)
is the active task document. Implementation, authenticated calls, service or
installation changes, operator-project changes, push/merge/publication are not
currently authorized. Stop for review; do not begin its first implementation step.

Current branch: `plan/native-coordinator-integration`.
Base: `2e5cf1212fe9a319b118c641c5028c851f66e86d`, verified synchronized main after
PR #5 was squash-merged.
Planning checkpoint: `52cdb12365664cee09079eb4531e85378ce571f6`.
The squash tree matches prior topic checkpoint `31273fd`; earlier migration/fix
commits remain preserved evidence. No unrelated changes were present at this
checkpoint's start. No branch reset, push or merge occurred.

## What exists and what is proposed

Implemented: accepted Rust/Ratatui TUI, Rust supervisor/runner, Tokio I/O, bounded
SQLite writer, durable commands/questions/approvals/context/receipts, scoped
systemd operation services and explicit read-only Herdr views. Each project owns
state, recovery and budgets. One workflow executes per project.

`waywr init .` / `waywr init SOME_DIR` initialize locally and return; `waywr`
opens the TUI in the invoking terminal; `waywr resume` reconciles. Ordinary
operation does not need Herdr. These examples assume a compatible installed
release on PATH; the retained comparison candidate is not globally installed.

Current submitted text enters a deterministic task-proposal flow. Native service
admission is blocked despite reusable Claude/Codex adapter helpers. Real
conversation, native tool isolation and working application artifacts are not
accepted. Historical Python native trials provide evidence, not fresh permission
or proof of current Rust-host support.

Proposed, **not approved or implemented**: real discussion independent of tasks,
validated coordinator actions, bounded consultation, boundary-applied steering,
conversation context/budgets, and structured static-file implementation/review.
Preserve the accepted UI and host; no alternate prototype or Python dependency.
The proposal describes schema 4/minor 2, while current contracts remain schema
3/minor 1. No automatic upgrade of existing projects is planned.

## Review decisions and next action

Await review of these recommended defaults:

1. Packet-only/tool-free native roles producing validated files; trust the provider
   client/authentication without claiming whole-process filesystem isolation.
2. Explicit coordinator changes between resolved calls; no automatic fallback,
   ambiguous replay, worker model switch or silent scope/budget escalation.
3. Separately authorized `native-weather-01`: Claude `claude-sonnet-5-5` medium
   coordinator/author, Codex `gpt-6.1-sol` high consultant/reviewer; maximum
   6 conversation + 1 consultation + 4 task calls, two reviews.

If implementation is approved, start with pinned CLI capability/context/tool
exclusion proof using source/config inspection and credential-free fixtures.
Unexplained reads/tools/fallback/policy keep native admission closed. Follow the
proposal's five checkpoints; native-call spending requires separate authorization.
Human clarification, scope decisions and final approval remain human gates.

## Verification and current limits

Planning inspection found installed Codex 0.162.1 and Claude Code 2.1.296,
matching adapter pins. Help hashes and nonsecret config categories are retained
at `.local/native-coordinator-planning/2026-10-13/`. User hooks and Codex
plugin/MCP configuration require effective exclusion proof. Auth validity,
managed remote policy, entitlement and residual tools remain unverified.

The operator accepted v3 simulated workflow/discoverability/input corrections.
That is separate from automated PTY verification and does not verify a weather
app or native execution. Physical v3 detach/resume remains unreported. The
[current limits register](current-limits.md) retains those obligations and links
completed evidence. No application tests or model calls ran during planning or
this documentation cleanup; link, archive and documentation checks only.

## Protected baseline and rollback

Keep the known accepted candidate:
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`.
Keep the one reusable comparison project:
`.local/supervisor-owned-execution/discoverability-hands-on-v3/project/`.
Its bound wrapper, project key, approved run, manifest and reopen commands are in
[current limits](current-limits.md#accepted-simulated-comparison-baseline).

Last checked at prior closeout: v2/v3 services paused/inactive, no owned workers;
seven exact automated fixture services inactive; one v3 foreground TUI left
attached. Not rechecked or changed this pass. Preserve approved records,
transcripts, referenced releases, r9, older evaluations, the separate blocked
legacy project, shared Herdr and unrelated services. The limits register names
the legacy task and its separate recovery requirement; archiving its report does
not resolve or authorize it.

Never replay uncertain execution, restore old state over newer accepted work or
point a legacy release at incompatible state. Reconcile exact project/host
ownership and require compatible state before release rollback. No operator
cutover or implicit historical migration.

## Documentation checkpoint and reading order

[AGENTS.md](../AGENTS.md#documentation-checkpoint-rule) now explicitly requires
reassessment at **every checkpoint**, including planning-only checkpoints.
Completed host-transition/report/correction material is archived; applicable
requirements live in current execution/state/IPC contracts. Unresolved issues
remain in current limits. The index no longer presents the closed evaluation as
active work. No known current-document conflict remains after link review.

Read [index](README.md), [native proposal](native-coordinator-integration-proposal.md),
then task-relevant [architecture](architecture.md), [execution](execution-contract.md),
[state](state-contract.md), [IPC](session-protocol.md), [TUI](tui-contract.md) and
[current limits](current-limits.md). Archives are selective evidence only.
