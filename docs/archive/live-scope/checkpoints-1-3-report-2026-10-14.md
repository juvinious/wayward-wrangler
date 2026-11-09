# Live proposal / scope implementation checkpoint

Implementation approved from `8c1bc7d912c33a9378429bdf03adcffc1d214a18`.
No native calls, operator upgrades, workers or consultation authorized.

## Checkpoint 1 — contract and authority

Code/state rollback baseline: planning commit `8c1bc7d`; disposable new schema-6
fixtures only. After accepted records, retain compatible schema-6 code or forward
repair; never restore old snapshots or attach schema-5 releases to new state.

Implemented strict result-v2 with duplicate/unknown field rejection and bounded
assistant-only provisional decoding. Immutable proposal revisions, head pointer,
append-only human decisions, answer-only saves and once-only continuation use the
existing SQLite writer. Exact hash/dependency/authorization identity checks reject
stale edits and competing decisions. Scope-only task status has no run or admission.
Service and workflow-store fences reject worker paths, including a forged ready row.

Verification: ten focused scope tests passed (including two existing output/view
checks selected by the filter); minor-1/2/3/4 IPC consistency passed. Isolated
conversation compatibility checks cover the unchanged schema-5 profile. Evidence:
`.local/claude-scope/checkpoint-1/`. No provider or runtime service launched.

Observed differences: a scope authorization needs an expected prior authorization
command identity as well as proposal hash/decision revision, so concurrent clients
cannot both authorize the same stale panel. Authorization itself does not change
content dependencies; its separate identity fences late results. This preserves
byte-identical saves as no-ops without invalidating permission. New schema-6 human
answers save without automatically queuing a call; older schema-4 semantics remain.
Initial compiler errors (SQLite integer representation and test API name) were
corrected before gates. The first compatibility run exposed an accidentally changed
legacy handoff query parameter; restored that query and repeated the failed gate.
Failed log retained; no external effect occurred.

Checkpoint-1 commit: `8830ace`.

## Checkpoint 2 — hosted adapter/context/recovery

Rollback baseline `8830ace`; new schema-6 fixtures only. Profile `claude-proposal-v1`
keeps the same pinned CLI/model/effort/login route/restrictions; no new tool or CLI
flag. Bounded strict JSON assistant text streams, then a result-v2 completion is
validated through runner and receipt adoption. Packets freeze only selected scope,
linked decisions and at most six previous messages; optional messages are removed
before the 96KiB ceiling, mandatory overflow refuses before reservation. Budgets
are labelled as-of reservation including the held call. Scope references are in
deterministic checkpoints; old conversation-only packets/handlers remain versioned.

Verification: 12 scope checks (atomic rollback/adoption, bounded packets and stale
reservation included), two provider completion tests, 12 existing conversation
regressions and IPC consistency passed. The pinned Claude binary passed three
no-egress/dummy-credential fixture cases with the new owned prompt: structured text,
literal unrelated path/slash mention, forced Bash denial; one loopback request each,
no tools or context canaries/sentinel access. Not authenticated/account proof.

Real host/mock CLI check passed: A had three calls/three receipts/one proposal;
B progressed while A awaited an answer, with no A metadata in its packet. Service
restart adopted the proposal once; continuation replay added no call. Cancellation
retained one debit, control response 0.00388s; killed runner became uncertain with
one spawn/no replay. Four test service processes ended; six exact empty hosts
stopped. Evidence `.local/claude-scope/checkpoint-2/{boundary-01,hosted-02}/`, logs
and unit-level fault checks. No native call, operator service or Herdr used.

Failed `hosted-01` retained: first fake client ran, but the final runner validator
still expected result-v1 and exited before receipt. Kept original packet/output/
claim/permit, journal and hash-verified copies of that build. Corrected runner and
receipt validators to select the explicit profile; invalid completed chat output
now publishes output-invalid rather than losing completion evidence. Added focused
validator regression. No replay/recovery of the original uncertain attempt.

After accepted records, retain the paired compatible release and forward repair;
old state snapshots and result-v1 interpretation are not rollback mechanisms.
Next: checkpoint 3 integrated production TUI, then installed mock acceptance.
Native format reliability and physical interaction remain unverified.

## Checkpoint 3 — production TUI

Rollback baseline `9dadaed`; new schema-6 fixtures only. Reused the accepted
production view/editor, visible activity actions, overlays and durable outbox.
Questions save with no call, then offer separate Continue; answered questions have
no Save action. Scope panels show every field, revision, unresolved acknowledgement,
local edit and coordinator revision actions. An explicit decision binds the exact
panel identity. Authorized scope shows execution unavailable and hides another
authorization action. Local textarea edits/drafts retain their original base across
refresh/reconnect/reopen; saved replies reconcile their command IDs. Unsaved conflicts
are retained, never silently merged. Main composer/paste/Shift+Enter/F2 unchanged.

Verification: seven focused scope UI checks; accepted presentation suite 58 passed,
one existing 30-second probe ignored; 18 CLI/outbox/draft checks passed. Narrow/wide
scope rendering, bounded scroll, literal paste, explicit decisions and saved-answer
routing covered. Nine authority checks include inherited clarification integrity
and projection bounds. Real host repeat added a malformed response: output-invalid,
one debit/one receipt/zero tasks; cancellation and uncertain no-replay still pass.
Evidence `.local/claude-scope/checkpoint-3/`; exact empty test hosts stopped, test
service processes ended. No authenticated call or physical observation.

Corrections: older synthetic UI question lacked backend status='open'; corrected
that fixture rather than allowing answered questions to appear actionable. Found
an inherited-question edge: proposals created before a project-level answer must
retain its dependency. Added live linked-question lookup/invalidation and regression;
original revision/answer evidence is untouched. Known invalid completed scope output
uses a profile-scoped output-invalid receipt; legacy receipt outcomes unchanged.
Schema-6 snapshots/events and edit/decision views are bounded; recent history
truncation is explicit, all records remain durable. Queue admission never accepts
more pending turns than remaining allowance. These bounds keep the negotiated frame
limit; the packet builder reads its own selected records, not the display window.

After accepted state, roll back only to a compatible minor-4/schema-6 frontend or
forward repair; old clients must refuse. Next: build/install one candidate and run
checkpoint-4 normal installed mock interaction; six-call native trial remains closed.
