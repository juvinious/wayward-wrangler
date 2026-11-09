> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Saved proposal / scope authorization — current checkpoint

2026-10-14. Four approved implementation checkpoints are complete. The operator
verified the **v5 mock workflow, local editing, explicit scope authorization and
physical detach/reopen**, then requested sequential guidance because authorization
was not obvious. **V6 implements that focused UI correction; its physical retest
was not reached: the first native call failed strict result framing. No native saved-scope workflow or working application was verified on v6. The
separate [v8 successor report](../continuous-coordinator/continuous-coordinator-report.md) now records bounded
live scope acceptance; it does not change the failed v6 evidence.**

[Scope contract](../../contracts/live-scope-contract.md) and [TUI contract](../../contracts/tui-contract.md) own
semantics; the [archived mock guide](scope-v6-mock-walkthrough-2026-10-14.md) retains v6 regression use. The
[six-call native trial](claude-scope-live-acceptance.md) **stopped on its first native
call**: output-invalid, 1/6 charged, no proposal. Physical v6 scope-flow acceptance
and a working application are not established.

## Successor correction — separate release

Source audit/counter correction `3d94b665c07d3f583ad84ecba8f313d432f16b7a` is preserved
and packaged in the new claude-dialogue-v8 candidate. The approved continuous
correction's [contract](../../contracts/continuous-coordinator-contract.md) and [report](../continuous-coordinator/continuous-coordinator-report.md)
own its implemented state-7 authority, structured transport and passed offline gates.
This report still describes the retained v5/v6 scope checkpoint and failed trial.
No successor proof reinterprets the old response, renews its allowance or establishes
new native/physical scope acceptance. The old 1/6 trial remains blocked.

## Checkpoints and preserved authority

Branch `feat/live-task-proposal-authorization`, base `24fb317` (PR #6), planning
`8c1bc7d`. Passing checkpoints: `8830ace` authority, `9dadaed` host/context/recovery,
`fe0ca09` integrated UI, `e1d7af3` installed acceptance; `fce5ad3` observations.
This correction uses `fce5ad3` as its source rollback checkpoint. No push/merge.
[Prior report](scope-v5-report-before-sequential-review-2026-10-14.md)
retains exact full earlier test counts, failed attempts and rollback evidence;
[stages 1–3](checkpoints-1-3-report-2026-10-14.md) retain implementation detail.

Only an explicit application action grants exact-revision scope permission.
Conversation, opening a panel, Continue and acknowledgement grant none. Scope
permission saves no execution allowance/admission/run; workers/consultation stay
unavailable. Local decisions/edits/answers cost no calls. Backend authority, result
validation, budgets, context, release routing and IPC minor 4 are unchanged here.

## Operator observations — separate from automated checks

Operator reported steps 1–5, physical detach/re-attach and optional refinement
verified with screenshots. First read-only audit confirmed four completed mock
calls/four matching receipts, one answered question and three linked hash-verified
revisions (v2 is a human offline-constraint edit), but no scope decision yet.
A later screenshot showed **Scope authorized · execution unavailable**; a second
read-only audit confirmed **one exact revision-3 decision**, task `scope-authorized`,
call count unchanged at 4/6, no run. The operator explicitly found the simultaneous
actions unintuitive; do not label that flow intuitive or infer broader acceptance.

Retained project `.local/claude-scope/evaluation/operator-eval/`, key
`<private-id>`, remains bound to **claude-scope-v5**, manifest
`04a8c0824bf3bc89a056cdc7f9040ed7eec2f0815e66ef10bf98a5eb6548f2fd`.
Ignored audits/byte-verified screenshot copies with original paths/hashes:
`checkpoint-4/operator-observations-2026-10-14/` and
`checkpoint-4/operator-authorization-2026-10-14/` under `.local/claude-scope/`.
No calls, state mutations, renewal, rebind or service restart during these audits.
Physical exit, every editing/input case, authorization *invalidation* after a prior
saved decision and native result-v2 reliability are not inferred from observations.

## V6 sequential correction and candidate

Only current-step content and one primary action appear:
1. Purpose/deliverables → Continue to boundaries.
2. Boundaries/acceptance → Continue to choices.
3. Unresolved choices → Acknowledge and continue (no decision).
4. Explicit authorization → Authorize scope (durable decision only after acceptance).

Edit, request revision (one call), previous and Not now/Escape remain contextual.
No unsupported decline operation is added. Same identity refresh keeps stage,
focus, reading position and acknowledgement; changed identity resets stage 1.
Known-repeat Enter events cannot skip steps. Authorized scope is a read-only result
view, not another approval. Changed-scope history wording no longer invents an
existing prior decision; the older generic event remains protocol-compatible.

**claude-scope-v6**: `.local/claude-scope/install/releases/claude-scope-v6/`;
manifest `b60073595f485d5d85ac517bbd7d1f0087c06a33e16370b76a2894ff6eeebb13`.
All 13 files verified against manifest/build outputs. Service/runner bytes are
identical to v5; only frontend changes. Locked/offline permanent rustc/Cargo 1.99.0;
Cargo.lock/pin/dependencies unchanged. Provenance/build logs remain ignored under
`.local/claude-scope/sequential-review/`. New mock-only launcher there is `waywr`;
new empty reusable project `evaluation/operator-eval/`. Old launcher/v5 state stay.

## Executed verification and resources

- Focused scope checks **13 passed**; complete UI library **64 passed, 1 existing
  ignored latency probe**. Covers step content/action routing, no early authority,
  exact decision, stale and unchanged refresh, repeat keys, draft/edit/back, no
  unresolved choices, scrolling bounds and 80×24/120×36/160×48 layouts.
- Installed production path, mock only: init/start/F3/discuss/question/save/Continue,
  local edit, all four visible steps, separate exact authorization, command replay,
  detach/resume with authority/draft, authorized read-only view and exit/restoration.
  **Three completed calls/three receipts, one decision, zero workers/consultation/
  admissions**, 120/80-column terminals. Saved records/packet/result hashes matched.
- Evidence `sequential-review/evaluation/automated-01/`: PTY/input/file traces,
  per-step screens, summary/cleanup; source logs/provenance alongside. No source
  denial, broad acceptance rerun or provider-boundary experiment in this pass.
  Prior v5 systemd/recovery/schema-5/responsiveness/isolation evidence is retained;
  it is not re-labelled as new v6 execution or physical acceptance.

Only the new fixture service and its three empty completed host units were stopped;
exact identities and inactive properties retained. New operator fixture has not
launched. Existing v5 operator service/UI were last observed active and left intact.
V1–v6 installations, original native conversation trial, approvals and shared Herdr
remain untouched. No staging copies removed and no default/global installation.

## Native trial outcome — failed format gate, no acceptance

Trial project key `<private-id>`, bound v6. The user confirmed
F3 and submitted one weather-plan request. Native init recorded **Claude 2.1.296,
claude-sonnet-5-5, empty tools/MCP/skills, dontAsk**; execution profile/medium effort/
owned system prompt and all approved release/client hashes match. No substitution.
The normal 2.1.299 alias was bypassed. Preparation used zero calls.

Claude exited 0 with a single successful `end_turn`, but its result was **Markdown
prose followed by a fenced JSON envelope**. [finish_profile](../../../backend/src/claude_chat.rs#L252)
passes the whole result to strict parsing; the approved [plan](implementation-plan-8c1bc7d.md)
expressly forbids wrapper stripping, new schema flags or extraction/repair calls.
Its `expected value at line 1 column 1` refusal is therefore correct. The weakness
is prompt-only structured delivery, not an absent runner or release/profile drift.
Its “draft is saved” statement was unaccepted provisional text; no proposal exists.

Read-only audit: **1 invocation / 1 matching output-invalid receipt / 1 of 6
charged**, zero tasks/revisions/questions/scope decisions/workers/consultation/
admissions. Receipt has null result hash; database `validated-receipt` is an
adoption/stop sentinel, not a candidate SHA. Raw reply/usage remain in native events;
no result.json/runtime-result.json or validated assistant message exists. No attempt
was retried or salvaged. Prior stopped-turn readiness blocks further trial calls.
Five unused calls are not permission to continue after this failure.

Ignored evidence `.local/claude-scope/live-audit/first-call-failure-2026-10-14/`:
**22 byte/hash-verified regular-file copies**, original path/hash manifest,
screenshot, response, read-only database and exact-unit audit, focused test logs.
Original packets, receipt, authorization, failure and binding remain unchanged.
UI/project service remain active and untouched; the completed host is retained
active/exited, MainPID 0/empty cgroup. Native cost is a reported estimate, not billing.

Operator screenshot shows the blocked call and Inspect; later feedback says the
application is rigid, with inadequate steering/interruption while waiting. This is
**an unmet product requirement**, not accepted physical/native scope workflow.
Active-call `/cancel` exists in source; availability/discoverability and continuous
conversation across questions/proposals need a bounded follow-up. No native
interrupt, four-step review, saved scope or detach/resume was verified in this trial.

Inspect's `/3` denominator was a [hardcoded display bug](../../../frontend/src/prototype/view.rs#L2795),
while ledger/activity correctly showed 1/6. Source now reads the authoritative limit
and accurately distinguishes diagnostics after invalid completion. **One offline
3/6-call rendering check passed; one strict prose-plus-fence refusal check passed**.
No provider, broad suite or new installed build ran. Retained v6 is unchanged and
still has the display omission. Decoder/authority/capability policy are unchanged.

Recommended next: a separately approved, credential-free native structured-result
interface proof, preserving strict validation and qualifying only inert result
delivery; no Markdown salvage or formatting repair call. The archived boundary
assessment identifies `StructuredOutput` as a candidate, not proven enforcement.
Reconcile that explicit transport change with the prompt-only contract before a new
candidate/live authorization. Address natural discussion/visible interrupt controls
at pending decisions, steering at safe boundaries and preserved frozen authority.
No general UI/backend redesign, provider/model substitution or worker expansion.
Original native v5 and authorized mock v5 resources remain untouched.

No schema/record/protocol change: rollback new empty fixtures to v5-compatible code
is possible; retain each project's bound release. Accepted state must never be
replaced with old snapshots. Reverting UI cannot revoke durable decisions; no
operator cutover is authorized. The failed trial is closed to further dispatch;
next contract correction requires review. Workers remain deferred.
