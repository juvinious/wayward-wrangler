# Waywr architecture

This maps the current implementation. The [product direction](execution-adapter-decision.md)
replaces provider/model-specific core integrations with one shared multi-provider
layer. Current native paths below are not a compatibility requirement for the
rewrite. Preserve project work and execution ownership; do not treat historical
provider restrictions as the target architecture. The runtime uses the independent pi-ai/pi-agent-core libraries; Pi's terminal app is not used.

The current Rust TUI/service/runner supports project-scoped conversation, authorized
development and supervised model/tool workers. Workflow/provider/conversation/context
work is merged through `799841c` (PR #9). Recovery/convergence and the ordinary-project
increment were squash-merged as `4218347` (PR #10); original ancestry remains under
`refs/archive/pr-10/`. The
[recovery contract](coordinator-recovery.md) owns its current behavior and limits.
The [approved plan](archive/product-workflow/approved-plan-b810bf1.md) is historical.
The [current contract](ordinary-project-workflow.md) covers setup, knowledge,
candidates, checkout ownership, retention and completion; HANDOFF records verification.

## Components and authority

| Owner | Responsibility |
| --- | --- |
| `frontend/src/main.rs`, `prototype/view.rs` | Accepted Ratatui composer, exact scope/policy approval, tasks and Inspect |
| `rust_runtime.rs`, `rust_bundle.rs`, `rust_migrate.rs`, `cli_schema.rs` | Init, paired stable install, stopped-service layout migration, CLI routing, service attachment |
| `session_service.rs`, `db_writer.rs` | One service per project, responsive IPC and bounded SQLite writer |
| `ipc_limits.rs`, `frontend/src/protocol.rs` | Shared1MiB incoming/64KiB command budgets,8MiB complete replies and correlated oversized-reply errors |
| `coordination.rs`, `coordination/providers.rs`, `prototype/coordination_view.rs` | Request-bound chat, presented cards, exact decisions, continued answers, provider choices and append-only repair |
| `continuous.rs`, `task_scope.rs`, `conversation_store.rs` | Saved discussion, exact proposal/decision authority and causal queue |
| `output.rs` | Typed result validation; ordinary citation provenance separate from action dependencies |
| `request_routing.rs` | Human source-bound request role pins, policy projection, exact-review gates and explicit retained-source reconciliation |
| `provider_selection.rs`, `model_runtime.rs`, `model-runtime/`, `prototype/provider_view.rs` | Schema3 preferences, shared provider/tool runtime, normalized events, handover/fallback and searchable F3 forms |
| `project_workflow.rs`, `project_knowledge.rs`, `prototype/project_view.rs` | Saved baseline/target/completion preferences, explicit initial source, canonical identity, incremental path/instruction map and scoped recipes |
| `development.rs`, `candidate.rs`, `project_resources.rs`, `project_publication.rs` | Approved parent, exact governing candidate, checkout handoff, child retirement, serialized local/PR completion |
| `model-runtime/main.mjs`, `tools.mjs` | SDK provider registry, custom protocol configuration, agent loop, task tools and private authentication store |
| `rust_bundle.rs`, `rust_maintenance.rs`, `scripts/`, `.github/workflows/` | Versioned staged installation, guarded quiescent updates and normalized release packaging |
| `development_context.rs` | Compact working context, exact read-only evidence grants/pages, continued retrieval and capacity diagnostics |
| `development_recovery.rs` | Typed derived recovery causes, source proof, authority-bound wake conditions; existing scheduler owns continuation |
| `review_findings.rs` | Stable finding lineage, selected-reviewer/evidence validation and read-only effective disposition overlays |
| `project_layout.rs` | Canonical `.waywr`, conflict/journal recovery, mutable location repair and immutable evidence aliases |
| `request_history.rs` | Stable request identity, public-event timestamp/link metadata, append-only readable projection and compact checkpoint |
| `host_attempt.rs`, `runner.rs`, `operation.rs`, `systemd_host.rs` | Durable intent, claim, generation permit, once-only spawn, bounded output, immutable receipt and reconciliation |
| `local_context.rs`, `packet.rs`, `context_access.rs` | Existing selected context/provenance and explicit bounded cross-project grants |
| `workflow_store.rs`, `static_files.rs`, old native adapters | Versioned older workflows; their fixed stages/limits do not govern state8 |

Git owns code history. SQLite owns scheduling, approval and acceptance. Native model
output proposes actions and claims; receipts and exact commit/check evidence establish
what actually happened. TUI and optional Herdr displays grant no authority.

[Canonical layout](project-layout.md) owns migration and evidence aliases.
[Retained delivery evidence](archive/conversational-coordination/2026-10-17-verified-checkpoint.md)
records real taskings and assistance, not general complex-project readiness.

## Connected development

New projects use state9 / conversation config6 / IPC1.10, with schema3
`development-profiles-v1` preferences. Models/providers/options are data; the shared
runtime owns protocol clients, catalogs and the tool loop. Defaults come from
runtime configuration/catalogs. Retired native schema1/2 profiles are unsupported
without explicit reconfiguration; files/history are preserved. The
[selection contract](provider-selection.md) owns supported client/effort capabilities,
per-attempt freezing, preference/policy separation, exact-commit review, binding
handover and opt-in settled-capacity fallback. No shared provider settings or
credentials are changed. Worktrees remain a trusted-client/same-UID boundary.

One explicit exact scope/policy approval creates a parent development record. Routine
assignments/checks/commits/integration then need no repeated approval. Workers own
separate task branches/worktrees under `.waywr/worktrees/`; canonical state
stays in the original project. Readable request namespaces identify the combined
branch and worker refs; legacy refs remain with a checked named result alias.
Coordinator context and Inspect distinguish the combined preview checkout from main.
New setup freezes a committed baseline and named target with completion/validation
choices on scope approval; checked target delivery preserves unrelated primary work.
Legacy projects keep their existing policy unless explicitly changed for future work; their retain policy remains compatible. New requests use selected retirement. Request-branch
checkout ownership, automatic safe cleanup, candidate-bound validation and PR policy
are implemented by the ordinary-project candidate/checkout contract, beyond readable ref names alone.
The coordinator may retire exact positively unlaunched obsolete assignments through
atomic writer adoption, preserving findings, obligations and execution fences.
A new repository gets an operational baseline without
application source. The first application source is worker-produced.

The coordinator reviews exact submitted commits from its immutable packet; a worker
finishing mid-call requires a new review before integration. Only the service mutates the
integration worktree. Atomic writer adoption prevents partial assignment batches. Explicit in-scope
steering records requirement generations and fences obsolete results. Saved expected
head, submitted SHA and intent identity precede
Git mutation. Restart compares actual parents/message/worktree state and records a
known applied merge once; dirty, conflicting or ambiguous state stays available for
reconciliation. Required obligations plus successful checks at the exact combined
revision gate checked clean local-main advancement. Submission alone is incomplete.

Worker dispatch is independent of pending coordinator/check calls. Settled blocked
worker reports retain their concrete cause without becoming frozen submissions;
known settled coordinator failures feed automatic replanning. Explicit operational
recovery notes persist as labelled context in the next coordinator packet and
request history, without changing scope/policy authority. Ambiguous effects
still require reconciliation.

Typed recovery now keeps unit host/preparation faults local and adopts validated
pending results or conclusive saved Git effects through operational attention.
Unchanged preparation faults wait for changed input/release proof under the same
authority. Unknown application faults remain blocked. Progress compares actual trees,
supplied evidence ranges, finding/obligation transitions, checks and execution settlement.
Equivalent work or a recent code cycle prompts reassessment; unchanged facts then wait.
Task IDs, timestamps and claimed strategy alone cannot establish progress.
New review continuations use explicit finding lineage/dispositions, separate native
output from effective status and support exact linked clarification. Old frozen
review schemas retain their original semantics. The
[proposal](archive/coordinator-recovery/approved-plan-d21271f.md#correction-to-the-review-diagnosis) identifies
that observed gap and the existing autonomous clarification that worked around it.
Recovery events and compact snapshots feed existing activity, Inspect and readable
request history. Capacity backoff reserves no narration calls. Recovery reuses these
owners and authority boundaries; no new orchestration service is introduced.

No mandatory call, repair or task deadline applies. Usage is observed, monetary cost
unknown unless reported. Explicit provider capacity refusal preserves work and waits;
unchanged fingerprinted partial input can continue after positive settlement.
Unexpected later changes remain preserved for reconciliation. There is no silent
fallback. Process/transport/shutdown health bounds are separate.

Changing the project conversation binding requires an explicit destination grant
and settlement of existing coordinator calls. Unrelated request policies do not
veto that choice: each request still checks its approved destination at dispatch,
and excluded workflows cannot resume or gain authority through a model switch.
The setup-blocked Send action opens the saved selection review with its draft intact.
Idle TUI reads borrow the large development projection; task details refresh on
project/task revision changes. The history projector reads a small revision
watermark before deciding whether retained events need projection.

Coordinator conversation telemetry uses `model_runtime::presentation` to project
bounded numeric usage and selected identity/activity from the published host
attempt into recent conversation turns. `conversation.activity` refreshes an
active call without reloading the full catalog/history. The main conversation
shows current or latest subject-local telemetry, including completed calls after
reopening; this display never changes scheduling or authority. The
[provider contract](provider-selection.md#execution-results-and-usage) owns usage
and estimate semantics. Autonomous workflow coordinator operations remain outside
this conversation summary; worker views retain their existing telemetry path.

The shared runner prepares non-worker child working directories independently
of retired native-client helpers. Preparation failures precede child dispatch and
retain explicit local nonlaunch proof; genuine post-dispatch uncertainties keep
their existing fences. Real runner fixtures cover conversation and workflow
coordinator startup as well as worker tools through two local protocols.

## Durable history and context

A request is identified before approval and linked across discussion, task assignments,
attempts, revisions and closeout. New committed public events carry timestamp/actor
sidecar metadata. `history/` holds readable append-only frames, projection
watermarks and compact deterministic checkpoints with labelled coordinator narrative.
Checkpoints retain command results/evidence links, obligation counts and bounded
open details; the continuous log retains full resolution rationale and narrative.
A separate task projects bounded pages. Incomplete append tails matching durable
append intent can be repaired without rewriting complete published entries; unexpected
manual data is preserved. Recognized credential text is redacted.
Native transcripts/tool output and hidden reasoning are not automatically projected.
Inspect exposes absolute log/summary locations. Logs and summaries are derived context,
never authority. Compact selected context replaces automatic whole-log replay.
Discussion enrichment is topic/scope-selected and fitted before the final packet
size check and immutable freeze. New topics receive compact current-project source
knowledge, not prior workflow plans or repeated obligation-resolution records.
[Context delivery](context-delivery.md) separates the soft working target from model
capacity and memory/transport bounds. Coordinators/reviewers can retrieve exact
source/diff/check/history pages through typed service continuations, with no native
filesystem tools or new execution authority. Unresolved findings and exact-review
gates remain; supplied evidence is not exhaustive source-review proof.

The incremental project index covers eligible tracked paths and exact nested
instructions/conventions; dirty hashes remain separate observations. Compact planning
summaries and paged exact retrieval disclose omissions. Approved finite project argv
recipes extend existing tracked-script/Cargo checks; previews remain user instructions.
No Example Task App command pair or application-specific task sequence is prescribed.

## Provider increment and remaining limits

[Provider selection](provider-selection.md) owns the connected state9 extension.
Focused local tests cover selection, save/import, policy separation, role packets,
handover, adapter boundaries and preservation. The recorded live delivery verifies
Codex coordination/implementation with Claude review, including a source-retrieval
continuation; fixtures do not establish other native role combinations. New requests
and approvals remain human-originated. Discovery establishes neither authentication
nor role usability.
The shared runtime supplies its SDK catalog and custom provider/model overlays
without inference during metadata inspection. F3 searches provider/model IDs and
edits options. All coding roles use the same runtime rather than brand allowlists.
The current SDK catalog has42 providers/1592 models; two protocol fixtures establish
request/tool/result plumbing, not every model's authentication or suitability.
Standalone native-Claude and Ollama transports and their superseded integration
harnesses were removed. Earlier fixed workflow readers remain historical code,
not alternative state9 execution routes. See the provider contract for same-UID
execution and account/ordinary-use verification limits.

## Distribution boundary

`frontend/src/rust_bundle.rs` owns paired offline bundle assembly/validation, staged
installation and atomic default selection. `scripts/dev-install.sh` wraps the
locked build, notice/privacy checks and paired local selection; `--version`
reports the selected artifact without initializing a project. `rust_runtime.rs`, `main.rs` and
`rust_maintenance.rs` check current storage/semantic compatibility and retain
explicit guarded migration/rollback. Ordinary startup no longer routes through
an old executable named in project release metadata. Compatible stopped projects
reopen with the selected paired application; a running service still enforces
its own release/owner identity. Idle services hand over under startup/execution fences after exact ownership and
quiescence checks; busy or uncertain work is preserved. Installed
bundles are independent of this source checkout. The GitHub PR/build workflow passed focused checks, production packaging, real
isolated installation/TUI startup and artifact upload. The version-tag release
workflow remains unexecuted; no release has been published. Versioned Linux binary artifacts, a download entry
point and guarded quiescent updates now use the [distribution contract](distribution.md); authenticated release download/publication, ordinary project PR delivery and broader platform support remain unverified.

## Conversational coordination

The [conversation contract](conversational-coordination.md) owns the state9 IPC1.8
interaction overlay. Stable request ownership is independent of task inspection.
Frozen draft/submission targets feed saved question decisions and once-only continuation,
exact presented-card approval and source-bound steering through existing commands.
Provider and lifecycle choices reuse their existing fences; no model output grants
human permission. New causal records require a compatible `conversation-v1` release;
older submissions retain their bound semantics. The label-management-example repair appends
cross-references rather than rewriting old events or replaying misrouted assent.
Ordinary conversation, including a verified post-delivery recap, uses the same live
coordinator without requiring a scope action. Unknown optional citations are retained
as unverified provenance rather than dependencies; typed actions keep strict guards.
[HANDOFF](HANDOFF.md) records installation, repair and verification limits.

## Contract owners

[State and history](contracts/state-contract.md), [IPC](contracts/session-protocol.md),
[execution/recovery](contracts/execution-contract.md), [TUI](contracts/tui-contract.md).
[State7 continuous interaction](contracts/continuous-coordinator-contract.md),
[state5](contracts/claude-coordinator-contract.md) and
[state6](contracts/live-scope-contract.md) retain old compatibility semantics,
not standing trial permissions or universal product limits.
