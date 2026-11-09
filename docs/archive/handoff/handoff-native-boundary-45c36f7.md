# Wayward Wrangler — current handoff (2026-10-13)

## Active objective and authorization

Implement checkpoints 1–4 of the [native coordinator proposal](native-coordinator-integration-proposal.md) on `feat/native-coordinator-integration`. The operator approved these stages with an integrated production TUI, side-by-side schema-4 release, mock-only verification and no authenticated calls. Checkpoint 5 and `native-weather-01` spending require separate authorization. No push, merge, global installation, operator cutover or shared Herdr change is authorized.

Base/planning checkpoint: `ba27dc2d2777511adb573152d81a93c6d2ce9368`; feature branch was created directly from it with a clean tree. Main contains the accepted PR #5 Rust/schema-3 simulated application. The planning branch remains intact.

**Checkpoint 1 is blocked at its native-isolation gate.** The pinned Codex CLI does not expose a supported per-call empty-tool allowlist for the requested `gpt-6.1-sol` role. Its pinned model catalog and registration source indicate residual `apply_patch` and utility tools despite the adapter's disabled feature flags. The [boundary report](native-coordinator-boundary-checkpoint-1.md) separates source inspection, the credential-free offline assertion, and unverified runtime behavior. Native admission remains closed. Checkpoints 2–4 have **not started** because they depend on this gate. Do not claim an installed schema-4 candidate or an end-to-end native-ready TUI.

## Implemented and verified baseline

The existing application has the accepted Rust/Ratatui TUI, Rust/Tokio service and runner, bounded SQLite writer, project-owned systemd execution, durable task commands/questions/approvals/context/receipts, and optional explicit read-only Herdr inspection. Ordinary usage for a compatible installed release is `cd PROJECT; waywr init .; waywr`; `resume` reconciles. Current messages still become deterministic task proposals. Real conversation, consultation, native roles and working application artifacts are not implemented or accepted.

The operator accepted the v3 **simulated** workflow and discoverability/input corrections. This did not verify a weather app or native agent. Physical v3 detach/resume remains unreported. Retain the known comparison executable at `.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr` and its bound evaluation project at `.local/supervisor-owned-execution/discoverability-hands-on-v3/project/`. They are not globally installed and were not used or changed at this checkpoint.

Checkpoint-1 inspection rechecked installed Codex `0.162.1` and Claude Code `2.1.296` with executable hashes in the boundary report. An offline structural assertion passed using downloaded pinned source, catalog and captured help. Ignored evidence: `.local/native-coordinator-checkpoint1/`; prior help/version captures: `.local/native-coordinator-planning/2026-10-13/`. No provider endpoint, credential, model call, service, test worker or installed candidate was launched. Effective tool inventories, managed policy, auth, model entitlement, retries and native output remain unverified.

## Blocker, decision and next authorized action

Keep the approved packet-only/tool-free boundary. The smallest recommended path is a supported Codex per-call empty-tool control plus a credential-free effective tool/request proof for the exact binary/model. The inspected CLI does not provide one. A future CLI/interface change, a different provider, or allowing residual tools is a substantive scope decision; do not silently substitute or relax restrictions. Pause dependent implementation for operator review. Once a supported boundary is selected and proven, finish checkpoint 1's negative fixtures and then proceed automatically through approved checkpoints 2–4, reassessing docs and committing each passing gate. Checkpoint 5 still needs separate trial authorization.

Before any future schema-4 admission, record the release/state rollback boundary. This checkpoint changed documentation only: no project state/schema, host, budget or binding changed. Existing schema-3, legacy and historical projects remain on their bound releases. Never restore older state over accepted work or replay uncertain execution. Preserve the separate blocked legacy project, all approved records, receipts, artifacts, r9 baseline, operator evaluation, shared Herdr server/panes and unrelated services. Current last-known v3 service/attachment observations are in [current limits](current-limits.md); they were not rechecked here.

## Reading path

Read [AGENTS.md](../AGENTS.md), this handoff, [documentation index](README.md), [native proposal](native-coordinator-integration-proposal.md), [checkpoint-1 boundary report](native-coordinator-boundary-checkpoint-1.md), and only task-relevant [architecture](architecture.md), [execution](execution-contract.md), [state](state-contract.md), [IPC](session-protocol.md), [TUI](tui-contract.md) and [current limits](current-limits.md). Archives are selective evidence. The previous handoff is preserved at [archive/handoff](archive/handoff/handoff-native-checkpoint1-start-ba27dc2.md).
