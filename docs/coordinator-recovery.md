# Coordinator-owned recovery

Implementation follows the [approved proposal](coordinator-recovery-plan.md) at
`d21271f`. Recovery is a derived explanation over existing workflow/unit/effect and
host evidence, never execution authority or a replacement scheduler.

Typed records bind cause, affected subject/operation, scope/policy/generation hash,
selected profile, source proof, blocked actions, phase, next action and wake condition.
The existing writer saves transitions alongside scheduling state and durable events.
Equal observations do not append duplicate events. Original receipts and packets stay
immutable. Ordinary conversation and unrelated authorized tasks remain available.

Preparation can be retried only after a scheduler-produced pre-ensure refusal and a
changed preparation signature under the same authority. Host attempts or possible-send
records cannot be reclassified as preparation proof. A changed release or useful input
allows new preparation; unchanged faults remain visible. Review assignment signatures
bind selected profile, authority, instructions, attempt revision, active evidence and
exact implementation/finding targets. Unrelated head/check/sibling progress cannot
repeat the same review preparation fault. Legacy immutable proofs are retained; a
subsequent preparation writes the narrower signature in a new operation. No OS/auth/configuration repair,
automatic upgrade, service restart or silent provider substitution is performed.

Unlaunched assignments with recorded grant/workspace blockers can proceed when the
exact selected grant validates or the owned worktree is positively clean at its
expected branch/base. Recovery performs no cleanup or permission expansion.

Existing validated coordinator/check receipts can be adopted through operational
attention. Saved Git effects reconcile using exact heads/parents and intent identity;
unknown dirty files, divergent heads or a possibly applied mutation without a result
remain fenced. Pause/cancel, substantive questions, provider grants and changed authority
are not cleared by recovery. A receipt is applied once; no replacement call is inferred
from missing processes. Capacity uses the existing persisted wait/fallback policy.

## Finding lineage

New review packets carry `finding_contract:1` and exact prior finding identities.
Each binds its originating result/context hash, target commit, requirement generation,
selected profile and wording. Legacy strings get deterministic identities in new
context; saved packets and accepted work are not rewritten.

`finding_dispositions` names the finding, origin hash, commit/generation, typed
blocking/resolved/advisory judgment, rationale and evidence references. Clearing a
blocker requires supplied evidence at that exact commit and the same selected
reviewer. Missing, stale, unrelated or wrong-profile judgments cannot clear it.
Native output/hash stays separate from the effective review and finding ledger.

A coordinator can assign a review clarification with `resolution_targets` naming
settled reviews and `review_targets` naming their exact implementation targets.
It retrieves relevant exact evidence and returns named judgments, with no whole-review
`reviewed_commits`. Current acceptance overlays validated dispositions without editing
old reviews/events. An unrelated passing review cannot erase an earlier blocker.
Full-diff, selected-reviewer and combined-check gates still apply to actual acceptance.
Evidence coverage is not a claim of comprehension or exhaustive source review. Coordinator working
summaries use the same effective linked disposition as acceptance; original blocking
wording stays in immutable unit evidence rather than reappearing as a current finding.

## Convergence

New frozen coordinator packets carry `convergence_contract:1`. Their compact frontier
identifies submitted work, producers, blocked unlaunched assignments, open obligation
names and a saved question. `working_summary.integration_checkout` gives the actual
combined worktree/branch/commit and exact-check status. Conversation and Inspect also
receive it; preview guidance must distinguish this checkout from older primary/main.

The coordinator can choose `retire` with assignment names and a concrete reason.
Application validation requires the exact supplied record, current authority, a
first-revision planned/ready state and positive scheduler nonlaunch evidence.
Running, uncertain, previously invoked, submitted or finding-bearing work cannot be
retired by this path. Outstanding dependent assignments must be adapted first.
The existing writer atomically records retirement once as supersession; original
assignments/refusals, findings and obligations remain. Retirement resolves no finding
or acceptance obligation. An unchanged preparation blocker is not a producer that
justifies repeated `wait`. Independent work continues; a child blocker does not
replace the whole request's working status.

Readable combined branches and legacy aliases follow the
[execution contract](contracts/execution-contract.md). Retained convergence state
requires a bundle advertising `development_convergence_contract: convergence-v1`;
the supported release-change guard refuses incompatible releases.

## Progress and presentation

Progress measures code trees, current authority/inputs, immutable supplied evidence
hashes/ranges, findings, obligations, check outcomes and settled execution. Recent
structural approaches ignore task/operation IDs, timestamps and reworded narration.
The bounded comparison window retains 16 approaches, not a call or repair allowance.
An equivalent decision/read/failure or A→B→A cycle gets one useful reassessment for
that unchanged condition; further equivalence persists a wait until real facts change.
A different useful action remains eligible. Released raw pages can be retrieved again.
Capacity waits retain their existing retry-after and approved fallback policy; no
inference is reserved merely to explain the wait.

Recovery details enter selected coordinator context and existing conversation activity,
worker rows, Inspect and append-only readable request history. Scoped unit recovery is
also visible on its workflow without changing scheduling. Cause/next/wake/decision
labels are deterministic; ordinary chat, steering and independent work remain available.
A new question uses the normal source-bound answer path; generic continuation cannot
clear execution uncertainty, a pause or an authority fence.

IPC1.9 advertises `coordinator_recovery_v1`; state schema stays9. New packets carry
`recovery_contract:1`. The project marker `coordinator-recovery-v1.json` prevents a
binding change to a bundle without `coordinator_recovery_contract: recovery-v1`.
Native result object hashes and immutable receipt/file hashes remain distinct;
projections never replace original bytes.

## Verification boundary

Subsequent ordinary use supplied one native observation: the Codex coordinator
retired `tui-repair-review` through a completed receipt and one application event;
the readable result alias points to the combined commit. This verifies that case,
not general autonomous convergence. The user still found the preview and acceptance
handoff cumbersome: deep worktree commands and repeated explanations of integration
versus main are required. No simple preview/acceptance flow is implemented, and the
current TUI request's human visual acceptance remains open. These are product gaps,
not failures of the focused checks below.

Isolated fixtures cover the recorded provisional finding/exact-source/nonblocking
result case, omitted/unrelated/wrong-profile resolution refusal, legacy schemas,
preparation cause changes, scoped concurrent work, repeated decisions/reads, circular
code state, capacity waiting, restart/adoption deduplication, interrupted integration,
dirty-state preservation, readable history, chat and release compatibility. They do
not establish fully autonomous native recovery. Progress is a structural comparison,
not a semantic proof; a new implementation strategy with no measurable delta may
need clarification or new evidence. Long cycles beyond the comparison window,
arbitrary provider output failures, OS/auth/configuration repair and unproven effects
remain limits. No new authenticated calls, runtime trial or direct Example Task App code
edits establish the convergence correction. Automated receipts remain fixtures; the
limited native observation above does not establish general autonomous recovery or
a usable end-to-end acceptance flow.
