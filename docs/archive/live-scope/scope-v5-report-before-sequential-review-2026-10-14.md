> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical snapshot from `fce5ad3`, before the final operator authorization
> audit and v6 sequential-review correction. Current status is in
> [the current report](../../live-scope-report.md); these are not current instructions.

# Saved proposal / scope authorization — checkpoint closeout

2026-10-14. Four checkpoints approved from `8c1bc7d`; all implementation gates
passed. **Installed mock checks and bounded operator observations recorded; no
native scope trial.** Exact-revision authorization remains operator-unverified
at the read-only audit below. Original v5 live conversation remains
exhausted and untouched. [Contract](../../live-scope-contract.md) owns current semantics;
[guide](../../live-scope-evaluation.md) owns mock use; [native request](../../claude-scope-live-acceptance.md)
is separately unapproved. Complete early evidence/rationale is
[archived](../../archive/live-scope/checkpoints-1-3-report-2026-10-14.md).

## Delivery and checkpoints

Branch `feat/live-task-proposal-authorization`, main/base `24fb317`, planning
checkpoint `8c1bc7d912c33a9378429bdf03adcffc1d214a18`. Planning branch retained.
No push, merge, global installation, operator cutover or dependency upgrade.

| Checkpoint | Passing scope / commit | Code rollback baseline |
| --- | --- | --- |
| 1 | Strict result-v2, immutable revisions, exact scope decisions and execution fences — `8830ace` | `8c1bc7d` |
| 2 | Hosted transport, bounded frozen context, atomic adoption/staleness and recovery — `9dadaed` | `8830ace` |
| 3 | Production question/scope/editor actions, durable drafts, bounded views — `fe0ca09` | `9dadaed` |
| 4 | Final paired installation, installed interaction/recovery, compatibility and closeout — `e1d7af3` | `fe0ca09` |

Scope authorization says **“Scope authorized · execution unavailable.”** It binds
saved revision/hash/dependencies/prior decision identity and explicit unresolved
acknowledgement. No run, admission, worker/consultation/task allowance. Ordinary
prose creates no task; only a validated inert proposal suggestion can materialize
one. Local answers/edits/decisions cost no calls; Continue is separate and once-only.
No phrase matching or canned response is added to production conversation.

## Final candidate and provenance

**claude-scope-v5** (different from historical **claude-conversation-v5**):
`.local/claude-scope/install/releases/claude-scope-v5/`, frontend/service/runner
Rust binaries. Manifest SHA-256
`04a8c0824bf3bc89a056cdc7f9040ed7eec2f0815e66ef10bf98a5eb6548f2fd`.
All 13 packaged files hash-verified. Stable mock launcher `.local/claude-scope/waywr`;
reusable project `.local/claude-scope/evaluation/operator-eval/`, now initialized
and bound to v5 after operator evaluation. It handles isolation and automatic startup
internally. The operator project is retained; do not reset or silently rebind it.
The external Python mock/wrapper is test tooling, not an application backend.

Built locked/offline with permanent rustc **1.99.0** (`b940084d7`), Cargo **1.99.0**,
target `x86_64-unknown-linux-gnu`; pin retained. `Cargo.lock` unchanged throughout
this increment. Source/binary/fixture/toolchain provenance is retained under
`.local/claude-scope/checkpoint-4/`; it records implementation commit `e1d7af3`.

## Executed verification

| Gate | Evidence / actual result |
| --- | --- |
| Authority/result | Initial strict invalid/duplicate/oversized/forged-action checks; atomic rollback/adoption, stale responses, concurrent clients, replay, inherited questions, exhausted local actions and execution fences passed. Final authority **10 passed**; source profile compatibility **12 passed**. Earlier exact counts in archived report |
| Production TUI | Final **61 passed, 1 existing ignored latency probe**; CLI/outbox/draft **18 passed**; scope refresh/deferred-panel/explicit-decision tests **9 passed**. Narrow/wide, multiline/paste, scrolling, unchanged/changed identity and draft conflict behavior covered |
| IPC | Minor-1/2/3/4 schema/service/capability consistency passed; old client cannot use schema-6 authority |
| Pinned client | Three credential-free/no-egress cases with new prompt: strict text JSON, literal unrelated path/slash mention, forced Bash denial. No tools/context canaries/sentinel reads; local mock request only. Earlier 15-case proof retained; not authenticated or whole-process confinement proof |
| Host recovery | Real systemd-host mock execution: restart/adoption once, continuation replay, malformed completion → output-invalid receipt, cancellation retaining debit, runner death → uncertain/no replay, independent project progress. `.local/claude-scope/checkpoint-{2,3}/` |
| Final installed scope | `.local/claude-scope/evaluation/final-02/`: normal init/start/F3/discussion/question/save/continue/proposal/local edit/authorization/detach/resume/revision invalidation/reauthorization. A **6 completed calls / 6 matching receipts**, B **1**, zero workers/consultation/runs/admissions; local authorization at 6/6, identical command replay adds no decision |
| Responsiveness | Final status max **0.002943s** (<1s); SQLite busy **0.252573s** (<500ms), command not accepted/not found afterward. Bounds were not relaxed |
| Exit / source independence | Same installed check included **three immediate exit/resume cycles**, persisted scope checkpoints, unchanged six calls/three decisions and terminal restoration. Rust `backend/` and `frontend/` sources/assets unreadable to UID 1000 throughout; 73 source files' bytes and directory permissions restored/verified. Source-denial JSON and PTY/file traces retained. This is source independence, not a model sandbox claim |
| Final schema 5 | `checkpoint-4/legacy-schema5-v5/`: unchanged three-call text profile, no tasks, three matching receipts, draft/resume/restart adoption/replay/exhaustion; separate cancellation and uncertainty each one charged call/no replay. Status **0.002547s**. Fresh fixtures only; original live project untouched |

Automated/mock PTY evidence does not establish physical Ghostty/Herdr interaction
or native structured-response reliability. No working weather page or worker result
is delivered. Native interruption, wider account policy and all crash windows remain
unverified. The [native request](../../claude-scope-live-acceptance.md) retains stop conditions.

## Operator observation and read-only follow-up

The operator reported “1–5 verified,” detach/re-attach verified, and the optional
revision scenario, with two screenshots. Record this as hands-on mock interaction,
including question/choice handling, scope review, local editing, revision display
and physical detach/reopen; it is separate from the automated tests above.

Read-only audit of project key `<private-id>` verified **four
completed mock calls/four matching receipts**, 4/6 charged, one answered question,
and three hash-verified linked revisions: coordinator v1, human offline-constraint
edit v2, coordinator refinement v3. Zero admissions, run requests or workflow
operations. **No scope authorization decision was saved**: task remains `proposed`.
The screenshot's unresolved-choice acknowledgement is not authorization. Do not
infer hands-on authorization or invalidation of prior authority from the report.
To finish that check, Review scope → acknowledge current choices → **Authorize
scope**; no model call needed. Worker execution remains unavailable.

Evidence: `.local/claude-scope/checkpoint-4/operator-observations-2026-10-14/`
contains the read-only audit and byte-verified screenshot copies with original
paths/hashes. No provider calls or tests launched for this follow-up. Exact v5
service `waywr-<private-id>.service` and verified operator UI
were active at audit; both left intact. Native scope, physical exit, broader input
behavior and a working application are not inferred from these observations.

## Meaningful failures, corrections and limits

- Initial hosted v1-result validation lost a fake completion receipt; original
  uncertain attempt/output/claim/permit and hash-verified old binaries retained.
  Corrected profile-specific result/receipt validators, with output-invalid coverage.
  That failed attempt was not retried or rewritten.
- Installed v1 exposed global-progress CAS refusing valid schema-6 discussion.
  Append-only input now uses writer lifecycle/project/target/budget/ID fences;
  exact scope writes still use strong stale checks. Schema-5 CAS is unchanged.
- Text-only replies exposed cached pending scope detail. Bounded periodic detail
  refresh and precise readiness wording correct that; background refresh never
  opens or approves a panel. Same-head acknowledgement/drafts stay intact.
- Exit/resume exposed socket closure between two read-only bootstrap reads.
  Bounded preparation retries preserve identity checks; uncertain mutation delivery
  still names its command ID and is never replaced/replayed blindly. Installed
  immediate cycles passed with generations/decisions/counters preserved.
- Driver failures matched event prose before the visible action, clicked an old
  revision, used compact immutable marker bytes, wrong checkpoint glob or prototype
  exit title. Corrected fixtures/waits, not authority fences. Failed logs/screens/
  input traces remain under `checkpoint-4/` and `evaluation/automated-*`/`final-01/`.
  Initial relative installation prefix refused; corrected to an absolute prefix.
- Launcher guards now reject configuration overrides and unrelated targets, including
  equals-form flags. Its five no-call guard/init checks passed. They exposed an
  existing bare CLI raw-routing limitation for `--project=DIR`; the test launcher
  normalizes to documented `--project DIR`. No production CLI expansion here; the
  follow-up remains in current limits. The first guard test declined setup and
  changed no state/service.

- Operator audit exposed misleading history wording: saving a changed proposal
  emits `scope.authorization_superseded` even with no previous decision
  (`backend/src/task_scope.rs`, `save`); the TUI says “authorize again” / “prior
  decision stays” (`frontend/src/prototype/view.rs`). Records remain correct:
  no authority is invented. A focused event/presentation correction is outstanding,
  not implemented or counted as passing in this documentation follow-up.

## Resources and state-aware rollback

Ignored evidence remains `.local/claude-scope/checkpoint-{1,2,3,4}/` and `evaluation/`;
packets, revisions, decisions, receipts, transcripts, failures and release paths
are preserved. V1–v4 bound failed/check fixtures and installations remain intact.
No historical paths were rewritten. No broad removal or shared Herdr change.

Final inventory checked **40 fixture project service identities** and **76 exact
host units**, no live owned process/cgroup. Project units inactive/unloaded. Twelve
remaining exited/empty compatibility-host units were stopped only after exact
intent/project/runner path+hash/cgroup checks; records retained. Details:
`checkpoint-4/{runtime-inventory,empty-host-cleanup}.json` and each fixture cleanup.
That inventory predates operator evaluation. The operator v5 service/UI are now
active as recorded above, with no pending turn; neither was changed by this audit.
Shared/unrelated services remain untouched.

Before new schema-6 state, planning-code rollback on empty disposable fixtures is
safe. After accepted records, preserve compatible minor-4/schema-6 code and paired
release or forward-repair; never overwrite with old snapshots or point schema-5
code at schema-6 state. Source rollback alone is insufficient. Existing schemas
3/4/5 and unfinished LangGraph runs retain their release/record interpretation.
Current candidate needs no operator migration. Next action: review candidate and
separate native authorization; do not begin workers or renew the old trial.
