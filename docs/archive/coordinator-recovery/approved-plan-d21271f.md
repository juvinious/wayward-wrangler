> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Coordinator-owned recovery during ordinary development

Planning proposal, 2026-10-17. Base: fetched `main` = `origin/main` =
`799841c9f61c4a09217bc9607795edb8380670bf` (PR #9); clean checkout before creating
`plan/coordinator-owned-recovery`. No runtime changes or new job are authorized by
this document. The development workflow and label-management-example delivery are completed.

**Deliver a small recovery loop inside the existing scheduler and conversational
TUI:** reconcile facts, obtain useful coordinator/reviewer interpretation when
needed, continue safe authorized work, and explain any remaining blocker. Preserve
the Rust runtime, current providers, task worktrees, serialized integration,
application-owned context and readable request history. No second orchestrator,
repair service, trial program or checkpoint approval sequence.

## Evidence and intervention inventory

Reviewed the [verified checkpoint](../conversational-coordination/2026-10-17-verified-checkpoint.md),
its directly relevant repair reports, selected read-only journal records and the
immutable CLI review result. Event numbers below are `session_events.seq` in
`~/projects/example-task-app/.waywr/state/state.sqlite`, also referenced by readable
history `~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
Request `<private-record>`, workflow `<private-record>`;
approved v3 SHA `8b1ef61b4f2f5dc3cc21e7a0477589aa1dca3b72b5f018e5090c83fb38bafaa4`.
This was assisted delivery, not evidence that every listed situation needed help.

| Observed intervention / evidence | Cause and outside assistance | Already in merged main; remaining work |
| --- | --- | --- |
| Saved answer did not continue; approval chat addressed an old delivered request (548–558; append-only correction 561–562). | Application routing/continuation defect; outside source repair and retained-answer association repair. | Request-owned composer, frozen input targets, continued answers and exact inline approval are implemented. **Closed**, not another routing increment. |
| Human chose Codex coordination/implementation and Claude review (572), but proposal prose did not enforce it; retained-source reconciliation saved pins (583). | Application defect, distinct from the legitimate human provider choice. Outside repair made the choice enforceable. [Routing report](../conversational-coordination/2026-10-17-proposal-routing-correction.md). | Role pins now guard assignment, reservation, launch and exact review. Proposal activity/Sending labels were corrected too. **Closed**; recovery must retain pins. |
| Typed approval used an obsolete empty-draft target (587); human approved through Details (588–591). Oversized display snapshot then blocked normal stop/readiness (637–639). | Application defects; outside frontend/snapshot repair and supported command delivery after positive `command_get:not_found`. [Approval/preparation report](../conversational-coordination/2026-10-17-approval-preparing-installation.md). | Empty-draft handling, truthful Preparing display, compact snapshots and small status/readiness are fixed. No reapproval replay. **Closed**; record-count paging remains an unrelated scale limitation. |
| Mandatory packet exceeded 96 KiB (637, 663); continuation hit a second host preparation ceiling (673). Operator recovery notes 666/676 followed corrected releases. | Application limits incorrectly treated as model capacity. Outside source/install repairs and operational resume; no next call had launched. | Separate soft working target, memory bounds and model estimates; exact retrieval and preparation checks fixed. **Closed defects**. Remaining: evidence-driven continuation after a positively resolved preparation blocker, without an operator-authored note. |
| Repeated evidence reads before active-memory repair; coordinator-22 finished during drain and was adopted at 728. | Application context loss, not proof of provider incapacity. Outside active-read-set repair and lifecycle resume; completed call retained. [Context report](../context-delivery/2026-10-17-installation.md). | Active pages/factual memory and once-only adoption implemented. **Closed memory defect**; improve detection of equivalent reads/decisions that still make no progress. |
| Valid assignments rejected as overlapping retained superseded work (732, 750, 756); coordinator-31 adopted once after repair (762–766). | Application ownership defect; outside source/install repair. | Terminal superseded paths released, unsettled writers fenced. **Closed**. Useful completed decisions already survive drain; do not add a replacement scheduler. |
| Model task reserved at 766 but had no host attempt: JSON float round trip changed its frozen packet hash. Corrected parser allowed first launch on resume 772, submission 776. | Application identity defect, externally diagnosed/repaired. Reservation was not execution. | Exact float round trip and explicit pre-host hash refusal implemented. **Closed**. Recovery must diagnose deterministic integrity faults, never retry or rewrite them into success. |
| CLI review retrieved exact `cli.py` (849), continued fresh (851), then effective review appeared contradictory (855); coordinator assigned Claude clarification (869–874) before integration (879–888). | Retrieval worked without an operator. **Application finding-retention gap**, detailed below, made a resolved provisional concern blocking. No intervening human instruction is recorded. | Autonomous clarification already worked. Add explicit finding resolution/lineage so retained concerns can be resolved without an unnecessary whole review and without allowing a later pass to erase them. |
| Completed docs review did not immediately advance: producer-free wait rejected (917), integration lacking frozen review rejected (919); new evidence read (921), integration (925–933), checks and delivery followed (941, 947–957). | Recoverable coordinator decisions/stale input; application safeguards worked. **No external intervention recorded** for this segment. | Replanning, frozen review and producer-free wait rejection already implemented. Add durable progress comparison and reliable wake/adoption through recovery attention, rather than another acceptance phase. |
| Post-delivery recap rejected for optional citations outside its allowlist (961); fresh human recap succeeded (965–970). | Application provenance-validation defect; outside repair/install. | Ordinary reply citations separated from action dependencies; original failed turn retained. **Closed**; do not repeat delivered acceptance. |
| Human chose tag operations (554), providers (572), requested splitting work (646), and approved revised v3 (655–661). | Genuine user decisions, not operational failures. The split occurred amid a context defect but is actual saved human intent. | Preserve the exact decisions. Future in-scope decomposition/repair belongs to the coordinator; unchanged authority needs no renewal. |

The reviewed records do **not** establish a native provider outage, ambiguous worker
execution or interrupted Git merge in this request. These are existing recovery
boundaries to preserve with focused checks, not additional claimed incidents. Most
outside work above shipped fixes to Waywr itself. This increment cannot promise
to cure arbitrary application defects by asking a model to restart or patch Waywr.

### Correction to the review diagnosis

The archived summary describes the effective review at 855. The immutable native
result from `review-review-completed-tag-cli-r2-ctx1462fb70d234`, attempt
`<private-id>`, actually has `requires_changes:false` and seven
findings. Its file SHA matches receipt `result_sha256`:
`110d883ec7f4ebe85c7b5350d29987a148b7ed15008cde6a9eb734e32daeb364`.
It resolves the provisional option-like-tag concern using exact source at
`02823012af461d75c48689da026935bf0c284b5a`.

`tick_development_work` adds any earlier unresolved finding absent as an identical
string and sets the effective disposition to true. Event 855 therefore has eight
findings. No structured resolution path exists. The coordinator correctly treated
that effective result as blocking and obtained a separate Claude clarification.
Preserve both results and the successful delivery; do not blame contradictory raw
provider output or retroactively change Example Task App records. This source/evidence
comparison refines the historical diagnosis, rather than reopening acceptance.

## Minimum implementation

### 1. Reconcile facts, then let the coordinator choose useful work

Use the existing scheduler/writer; record a typed recovery cause at its source
instead of routing every error string into undifferentiated workflow attention.
Attach recovery to the affected operation/unit or workflow. Keep its identity,
source event/receipt/effect hashes, current requirement/policy/profile binding,
blocked actions, next action and wake condition. Transitions are observed →
reconciling / coordinating / waiting / needs-decision → resolved or blocked.
Execution records remain authoritative; recovery status is a derived explanation.
Persist transitions with the action/continuation intent through the existing writer;
on restart reconcile that identity rather than issuing a second command or call.

| Situation | Application responsibility | Coordinator responsibility / user boundary |
| --- | --- | --- |
| Missing relevant context | Serve existing exact-revision source/diff/check/history grants; preserve pages, cursors and findings across fresh continuations. | Retrieve only missing evidence, summarize and revise the decision. No user permission for an already granted read. |
| Positive prelaunch refusal | Verify existing nonlaunch/retirement proof under fences; recheck authority, input and frozen routing. Continue a transient start using the same logical operation. A preparation fault requires proof its cause changed before re-preparing; retain old artifacts. | Adapt input/decomposition when allowed. Hash/identity corruption, missing credentials or broken installed code remain explicit blockers; no automatic configuration/install repair or provider substitution. |
| Settled agent/check failure | Retain validated receipt, actual failure and partial work. Existing follow-up assignments/checks continue under policy; never pretend failure is a submission. | Change the approach using diagnostics; assign a repair or narrower evidence request. Ask only for a substantive decision, not generic permission to try again. |
| Interrupted/ambiguous execution | Reconcile the exact host intent, claim/permit, spawn commitment, child identity, receipt and saved Git effect. Adopt a valid result once; monitor the same live attempt; fence dependent dispatch/integration while uncertain. | Interpret the resulting evidence, not process absence. Cannot declare nonlaunch, synthesize a receipt or authorize replay. Explain irreducible uncertainty and any real choice, rather than ask the user to attest unknown facts. |
| Settled result awaiting adoption / workflow advance | Poll/reconcile through operational attention; invalidate the relevant scheduling fingerprint after actual new evidence. Adopt under the original result identity, then schedule an eligible decision once. Saved merge parents/intent or exact promotion head prove applied Git effects; mismatches remain fenced. | Review current frozen results, choose integration/check/follow-up/closeout. A passed check or commit is not delivery by itself. |

Recovery is not a bypass for intentional pause, cancellation, changed requirements,
an unanswered product question or a new provider grant. Unaffected safe tasks and
normal conversation remain available; do not block all work because one task needs
clarification. Keep one integration authority. An unresolved merge, unknown dirty
files or unproven external effects stay preserved; no reset, silent stash, blanket
cleanup, automatic service restart or speculative replacement call.

### 2. Resolve findings explicitly, retaining their history

Add stable finding identities anchored to originating review/result, target commit
and requirement generation. Keep the original wording. A reviewer continuation or
coordinator-assigned clarification names each prior finding and supplies a typed
disposition: still blocking, resolved with evidence, or advisory with rationale.
For the observed case, the supplied exact `cli.py` page justifies the advisory
disposition under the existing requirements. Disagreement about the actual product
semantics goes to the user only if those requirements cannot decide it.

The coordinator requests clarification/repair through existing assignments and
retrieval. The selected reviewer supplies review judgments; application validation
checks lineage, exact commits/generation, allowed evidence and pinned profile.
Unaddressed findings remain open. A new passing review cannot silently clear earlier
blocking findings; a valid linked resolution changes the effective status while
original native results and published events remain immutable. Combined checks and
all other acceptance gates remain required. Legacy string findings receive derived
identities on read; do not rewrite saved packets or reinterpret old accepted work.

### 3. Measure progress and expose recovery in conversation

Extend existing context/action/assignment fingerprints with a compact progress
record: requirement/input/dependency commits, evidence hashes and supplied ranges,
finding/obligation transitions, actual changed source, check outcomes, execution
settlement and coordinator strategy. Ignore incidental timestamps, operation IDs,
reworded narration and repeated delivery of the same bytes. Repeated equivalent
failures or an A→B→A revision cycle with unchanged evidence trigger reassessment;
a renamed task or claimed new strategy alone does not establish progress.

The next useful coordinator turn gets the prior approach, failure and evidence
delta. It may narrow work, retrieve missing evidence, cancel redundancy or choose
an authorized alternate approach. If nothing useful can change, persist the blocker
and a wake condition; do not keep making equivalent calls. Provider retry-after or
health-based backoff can wait for a transient condition. A capacity wait is not
wasted effort and never silently switches providers. Resume when the condition
changes, preserving existing fallback policy and frozen attempts. Transport and
shutdown health timeouts remain distinct from useful work duration. No mandatory
attempt, call, time or repair ceilings; optional user resource caps still apply.

In the current activity/conversation view, show the observed cause, recovery action
and reason, what can continue, and whether a decision is needed. Examples:

- “CLI review retained an earlier concern. Claude is resolving it against the saved
  source; CLI integration is waiting. No decision needed.”
- “The worker result finished before reconnect. Its receipt is verified; applying
  that result once. No replacement worker is being launched.”
- “The launch outcome is uncertain. I’m checking its saved execution records;
  dependent integration is paused. Independent work can continue.”
- “The same preparation fault remains. No model call launched. This needs an
  application repair; your work and approval are saved.”

Use existing inline questions/actions and optional Inspect, not a recovery wizard.
Natural status questions/steering continue with source-bound intent; “continue”
never clears uncertainty or approves expanded scope. Request history continuously
records facts from durable events; coordinator explanations are labelled narrative.
If the coordinator is unavailable, deterministic status/checkpoint remains visible.
Expose the existing history/evidence paths. Do not send hidden reasoning or full
transcripts to the TUI or future context, and do not call a model merely to narrate
a retry. Resolved attention must clear without erasing earlier failures.

## Reuse and necessary state/protocol changes

- `development.rs`: existing unit/workflow/effect records, atomic decision adoption,
  failure feedback, dependency notifications, fingerprints and Git reconciliation.
  Add compact recovery/progress fields here; use the journal for chronology, not an
  unbounded copied failure transcript or another database.
- `session_service.rs`, `host_attempt.rs`, `operation.rs`: existing positive
  settlement/retirement, command IDs, dispatch fences and receipt adoption. Preserve
  development's existing exemption from legacy three-start retry allowances.
- `development_context.rs`, `request_routing.rs`: existing retrieval/index/active
  memory and exact review gates. Add structured finding resolutions and clarification
  links to the frozen review result contract; keep raw and effective results distinct.
- `coordination.rs`, existing conversation queue and `request_history.rs`: reuse
  causal input/decision handling, service continuations and append-only projection.
  Recovery must not create synthetic human instructions or approval records.
- Add bounded recovery metadata to existing snapshot/Inspect and selected coordinator
  context. Extend existing typed decisions only for a necessary recovery disposition
  or finding resolution; reuse assign/retrieve/check/decision for actual work.
  Negotiate a capability/version for new semantics and require compatible bundles
  once retained; older frozen schemas remain valid. No new IPC transport or vendor
  session, unrestricted shell tool, cross-project grant or shared provider config.

## Connected delivery sequence and focused checks

1. Implement typed recovery/progress records and scheduler reconciliation at the
   existing receipt/preparation/adoption boundaries. Verify proven prelaunch versus
   possible-send refusal, unchanged-authority continuation, settled-result adoption
   through attention, and intentional pause/cancel remaining fenced. Reuse the
   existing context-preparation, host-retirement and atomic-adoption fixtures.
2. Implement finding lineage/resolution with current retrieval and pinned reviewer
   assignments. Reproduce the observed provisional finding → exact-source retrieval
   → nonblocking native result case. Check omitted resolution remains blocking,
   clarification preserves prior evidence, and wrong commit/generation/profile or
   an unrelated passing review cannot clear a finding. No native call required.
3. Connect progress-based reassessment and conversational recovery presentation.
   Check duplicate reads/failures, a circular revision, useful new evidence permitting
   continuation, persisted capacity wait, restart deduplication and a concurrent
   unaffected worker. Check status/steering remain responsive and unchanged approval
   is not requested again. Reuse request-history and conversational TUI checks.
4. Reuse the existing interrupted-integration fixture (Git commit before writer ack)
   for once-only adoption, with its dirty/divergent-state refusal, and exact combined
   check/review gates. Build the changed Rust binaries and run only relevant checks.
   Checkpoints are recoverable commits, not approval stops. Document remaining
   limitations; no exhaustive fault/provider/concurrency matrix or acceptance rerun.

Future hands-on use is the **next real feature the user requests through Waywr**.
Do not invent, seed or submit it. Once separately authorized, ordinary startup,
discussion/approval, real workers, review, integration/checks and saved project
context must produce useful software; routine recovery should happen in that same
TUI without external shell diagnosis or unchanged-scope approval. Fix defects found
on that path; retain the distinction between fixture coverage and observed recovery.
Do not manufacture failures to make another demonstration. Completion is working
development with this behavior, not a report or a new proposed acceptance phase.

## Boundaries and product decisions

Recommendation: routine recovery is included in the existing operating policy,
with visible actions and user pause/steering. Seek input only for a genuine product
ambiguity, authority expansion, configured-provider change or unresolved ownership
conflict. Technical review disagreement first goes to the pinned reviewer with exact
evidence; do not ask the human to referee stale strings or approve each repair.

No new product choice blocks implementation review. Defer automatic Waywr upgrades,
OS/authentication/configuration repair, arbitrary dirty-worktree/conflict repair,
recovering missing native output by inference, provider expansion, packaging
portability, unrelated TUI redesign and broad scheduler refactoring. Unknown execution
may remain blocked when facts cannot resolve it; explain that limit without claiming
user assent can make replay safe. Preserve existing manual inspection/maintenance
tools for genuine defects, but routine supported recovery must not depend on them.
