> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Active objective

Implement approved native integration checkpoints 1A, bounded 1B, and 2–4 on
`feat/native-coordinator-integration`, starting from amendment
`167b983834413a3997fd5eaa583794c4165f7fa5`. Checkpoint 5, authenticated
calls, global installation, operator cutover, push/merge/publication and shared
Herdr changes remain outside scope. Continue after each passing gate without
routine approval; stop only a truly dependent path when native proof fails.

Checkpoint **1A is implemented and focused tests passed** at `fa90340`.
[`native_contract.rs`](../backend/src/native_contract.rs) freezes the
first static-file capability/result profile and holds native admission closed.
Checkpoint **1B failed** at the pinned Codex outbound tool inventory; the
credential-free, no-egress mock request included model-visible execution and
collaboration tool declarations outside the approved profile. Checkpoint **2**
at `a277cfe` implements schema-4 conversation authority. Checkpoint **3** now
has an installed mock-only TUI path through discussion, clarification, answer,
consultation, proposal and detach. Focused tests and isolated PTY checks passed,
including service restart and exit/resume. No authenticated provider dispatch
exists. A later installed fixture confirmed explicit pre-authorization steering
creates a linked proposal while retaining the original; frozen mock packets carry
a versioned bounded handoff of accepted decisions and budget provenance.
[Implementation report](native-coordinator-integration-report.md)
tracks gate evidence and rollback. The prior [boundary findings](archive/native-integration/boundary-checkpoint-1-45c36f7.md)
remain byte-preserved, as do the pre-amendment proposal and handoff snapshots.

## Current baseline and unverified behavior

The accepted production TUI, Rust/Tokio project service/runner, bounded SQLite
writer, host-owned systemd operations, decisions/approvals/receipts/context and
optional read-only Herdr views are implemented for the prior schema-3 simulated
workflow. Schema-3 messages still create deterministic task proposals; new
schema-4 projects use the accepted TUI with deterministic mock coordinator
responses. Static-file bundles and native roles are not yet integrated.
Operator acceptance of v3 was
**simulated**, not a native call or a
working weather application. Physical v3 detach/resume is unreported.

Preserve the comparison executable
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`
and bound evaluation project
`.local/supervisor-owned-execution/discoverability-hands-on-v3/project/`.
These are not globally installed or migrated. Ordinary compatible-release use
remains `cd PROJECT; waywr init .; waywr`.

## Native proof and next action

Native admission remains **closed**. The [proposal](native-coordinator-integration-proposal.md)
sets the capability policy and dependencies; the [boundary assessment](native-coordinator-boundary-checkpoint-1.md)
specifies B1–B7. The corrected app-server thread reported empty environments,
but the first mock request still advertised `functions.exec` and collaboration
capabilities to the model. Effective handler denial, startup/auth separation,
Claude's restricted profile, managed policy and account entitlement are unverified.
No provider/model/auth/billing substitution or budget expansion is permitted.

Next: commit the passing mock integration, then implement provider-independent
checkpoint 4: bounded static-file candidate, manifest, review and exact approval.
Do not advance the failed native-admission path without a separately reviewed
supported capability boundary.
Use one stable isolated launcher and the accepted integrated TUI; mock responses
must be conspicuous, and mock authorization must never start a native process.

The proposed later `native-weather-01` requires separate authorization and at
most **6 conversation + 1 consultation + 4 task launches**, two reviews. Frozen
models/efforts proposed: Claude `claude-sonnet-5-5` medium author/coordinator and
Codex `gpt-6.1-sol` high consultant/reviewer; availability remains unverified.
Never replay ambiguous execution, approve on the user's behalf or let generated
content execute as commands. Candidate files require manifest validation.

## Evidence, rollback and protected resources

Prior help/version and structural-source evidence:
`.local/native-coordinator-planning/2026-10-13/`,
`.local/native-coordinator-checkpoint1/`; additional read-only source review:
`.local/native-capability-review/2026-10-13/`; the credential-free 1B mock
request and failed fixture are retained at
`.local/native-capability-proof/2026-10-13/`. Checkpoint-3 mock paired bundles,
failed and corrected PTYs, and three disposable project states are retained at
`.local/native-integration/checkpoint3/`. The exact test-owned services were
stopped. No authenticated calls or operator-state changes occurred. For checkpoint 1A,
code rollback needs no state migration. Later schema-4 work uses isolated fixture
state and a side-by-side release. After admission, preserve compatible readers,
receipts and accepted actions; never restore an old snapshot over newer evidence.

Preserve all historical runs, r9, approved artifacts/decisions/receipts, retained
v2/v3 evaluations, bound releases, the separate blocked legacy
`~/projects/waywr-rust-sim-eval`, and shared Herdr resources. No
current process/service inspection has been performed for these operator assets;
last-known status is in [current limits](current-limits.md).

## Reading order

[AGENTS.md](../AGENTS.md) → this handoff → [proposal](native-coordinator-integration-proposal.md),
[implementation report](native-coordinator-integration-report.md), [boundary assessment](native-coordinator-boundary-checkpoint-1.md),
then task-relevant [architecture](architecture.md), [execution](execution-contract.md),
[state](state-contract.md), [IPC](session-protocol.md), [TUI](tui-contract.md) and
[limits](current-limits.md). Archives are selective evidence, not required startup reading.
