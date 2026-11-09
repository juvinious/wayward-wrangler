> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-14)

## Objective and authorization

Implement the approved [continuous coordinator correction](../../continuous-coordinator-correction.md).
Checkpoints 1–4 are authorized; credential-free transport proof/contract freeze and checkpoint-2 authority/host
gates complete; checkpoint-3 production TUI gates pass. Continue checkpoint 4 automatically after their
focused gates. No authenticated calls, workers, consultation, execution admission,
global installation, operator cutover, push/merge or publication.
Product requirement: keep discussion/composing available at pending questions and
scope gates; make queue and interruption visible; scope review is optional and its
final authorization explicit. Preserve accepted TUI, Rust/Tokio, bounded writer,
project isolation, immutable evidence, exact revisions and no ambiguous replay.

Branch `feat/live-task-proposal-authorization` (user explicitly requested this branch).
Source/planning base `3d94b665c07d3f583ad84ecba8f313d432f16b7a`, clean at start.
Main/base `24fb317` is merged PR #6. Scope implementation checkpoints `8830ace`,
`9dadaed`, `fe0ca09`, `e1d7af3`; sequential UI `aa9e009`; native setup `53a9fb2`.
All retained. [Pre-planning handoff](../live-scope/handoff-at-3d94b66-2026-10-14.md)
preserves the longer resource/checkpoint record; retrieve selectively.

## Current implementation versus proposal

Implemented: schema-6 scope authority through production TUI/service/host, immutable
proposal revisions, local edits, explicit exact scope permission, questions,
bounded context/budgets and receipt adoption. Scope permission launches no work.
Four-step review: Purpose → Boundaries → Choices → Authorization; opening/advancing
never authorizes. Schema-5 conversation and older projects keep bound behavior.

Source inspection found: submit already queues while running; questions do not
block submission. A global prior-error query blocks future turns; global latest-
user watermark stales earlier suggestions; context can omit an earlier queued
turn's later reply; asking a question bumps scope epoch. Queue/interrupt visibility
and optional discussion flow are insufficient. Details/source links are in the plan.

Implemented core and production TUI: same pinned Claude print client with named structured output, a
bounded credential-free enforcement/retry proof, causal queue context, scoped
staleness, visible Interrupt, inline confirmation of inferred formal answers,
and deliberate new turns after proven format failure/clean interruption. Ambiguous,
capability and integrity failures stay blocked. No old failure is reclassified.
State 7/config 4/IPC 1.5/profile v2 core is implemented. Installed
candidate is next; old bound releases and trial policy remain intact.

Approved recommendations: inert structured-result capability under a one-request proof;
new user-directed turns after positively proven recoverable outcomes, retaining
debits; one inline confirmation for model-suggested formal answer/deferral. Scope
review remains optional. No provider/model/auth/billing substitution proposed.

## Evidence and verification limits

[Live-scope report](../../live-scope-report.md) owns executed evidence. V5 mock operator
review/edit/refinement, revision-3 scope authorization and detach/reopen verified:
4 mock calls/receipts, 3 revisions, 1 decision, zero workers. V6 automated four-step
installed path passed (3 mock calls/receipts, 1 decision); physical v6 review was
not reached. Original schema-5 native trial separately completed 3 calls/receipts.

Current native scope trial: **1 native call / 1 output-invalid receipt / 1 of 6
charged**, zero proposals/questions/decisions/workers/consultation/admissions.
Client returned prose plus fenced JSON; strict framing refusal was correct. No
saved proposal exists despite provisional model prose. Five unused calls remain
blocked. Operator reports rigid flow; no native scope acceptance or working page.

`3d94b66` corrects Inspect's hardcoded /3 denominator in source and adds two passing
offline regressions (counter/rendering; wrapped JSON refusal). Fix is integrated; packaging is next.
New [implementation report](../../continuous-coordinator-report.md) owns checkpoint evidence:
27 no-egress dummy-credential cases, one POST each, four successful result shapes,
no forbidden effects/retries. The inert sink requires explicit denial names instead
of wildcard. Client num_turns=2 counts tool handling, not a second inference POST.
Raw duplicate keys/size checks remain application gates. Three contract tests pass;
native application admission waits for remaining validator/receipt/install checks.
No authenticated call or operator-service change. Account/model checks remain later.

## Preserved candidates and operator projects

Current installed **claude-scope-v6**:
`.local/claude-scope/install/releases/claude-scope-v6/`; source `aa9e009`.
Manifest `b60073595f485d5d85ac517bbd7d1f0087c06a33e16370b76a2894ff6eeebb13`.
Live project `.local/claude-scope/claude-scope-01/`, key
`<private-id>`; unchanged launcher `.local/claude-scope/waywr-live`.
It may inspect/reopen saved state but cannot retry or renew this stopped trial.
[Trial disposition](../../claude-scope-live-acceptance.md) owns exact profile and evidence.
Claude Code 2.1.296, `claude-sonnet-5-5`, medium, original saved login; default
Claude alias is newer and must not substitute for the pinned executable.

Retain authorized mock v5 `.local/claude-scope/evaluation/operator-eval/`, key
`<private-id>`, and its unchanged `.local/claude-scope/waywr` launcher.
Retain exhausted original native `.local/claude-conversation/live-eval/`, key
`<private-id>`, bound **claude-conversation-v5**, 3/3 charged.
Neither is migrated/rebound/retried. Preserve all referenced installs and records.
Mock v6 walkthrough moved to [archive](../live-scope/scope-v6-mock-walkthrough-2026-10-14.md);
it is regression reference, not another requested operator evaluation.

## Resources and recovery boundaries

Last prior read-only audit: scope-v6 UI/project service active and untouched;
`waywr-<private-id>.service`. Completed host retained active/exited,
MainPID 0/empty cgroup with positive receipt evidence. No service checks/changes in
this planning pass. User may normal `/detach` or `/exit` with confirmation.
V5 mock service was last active; original native v5 last paused/inactive. No cleanup.

Ignored evidence `.local/claude-scope/live-audit/first-call-failure-2026-10-14/`:
22 verified regular-file copies/provenance, screenshot, DB/unit audit, offline logs.
Original attempts, raw native events/usage, receipt, allowance and binding unchanged.
Prior mock/build evidence under `.local/claude-scope/checkpoint-{1,2,3,4}/`,
`evaluation/` and `sequential-review/`; pinned help under
`.local/native-coordinator-planning/2026-10-13/`; boundary fixture in
`.local/claude-conversation/boundary-03/`. Do not rewrite historical evidence.

After new-schema work is accepted, code rollback alone is insufficient: preserve
its compatible paired release or forward-repair, never restore an old snapshot over
accepted work or point old code at new state. No existing operator cutover approved.
Also protect r9, schema-3/4/LangGraph/operator projects, decision-log-01 approvals,
accepted simulated v3 and the blocked `~/projects/waywr-rust-sim-eval`.
Shared Herdr and unrelated services/resources remain outside scope. Existing bare
`--project=DIR` routing limitation remains; documented `--project DIR` works.

## Exact next action and reading order

Checkpoint 1 `d27884e`; checkpoint 2 `569e694`; checkpoint 3 current commit.
Next: one installed mock candidate and focused installed interaction/recovery gates. Eleven focused checks, actual mock-host restart/lost-reply/
held/release/isolation checks pass; see implementation report for exact evidence. Each passing checkpoint commits and reassesses
docs. No new live allowance implied. Preserve the original stopped trial.
Read AGENTS → this handoff → [corrective plan](../../continuous-coordinator-correction.md)
→ [scope contract](../../live-scope-contract.md) / [Claude contract](../../claude-coordinator-contract.md)
→ [TUI contract](../../tui-contract.md) → [report](../../live-scope-report.md).
[Documentation index](../../README.md) locates other contract owners and selective archives.
