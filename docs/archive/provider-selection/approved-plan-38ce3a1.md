> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Archived approved provider-selection plan

Planning checkpoint: `38ce3a1`. Source implementation: `0dfcbb0`, `49ecf3b`, `3d5cfbc`.
The later implementation instruction reserves the next real request and approval
for the human through Waywr; it supersedes this plan’s proposed ordinary-use
exercise. No demonstration or authenticated development request was launched.
Current controls/contracts: [provider selection](../../provider-selection.md).
Installation status and remaining ordinary-use verification: [handoff](../../HANDOFF.md).

---

# Provider/model selection and mixed-provider development

Planning proposal, 2026-10-16. Base: `c2064bc534d8cbe753bbae93a8d85a98ce8c403a`
(PR #8); fetched `origin/main`, local main and initial HEAD match exactly. Branch:
`plan/provider-model-selection`, created with a clean tree. The development workflow
is delivered. This increment extends its production TUI and execution path; it does
not reopen Example Task App acceptance or authorize runtime work during planning.

## Outcome and TUI flow

Normal `waywr init .`, `waywr` and resume remain unchanged. Extend F3 / Settings with
**Providers**, **Coordinator**, **Worker defaults/roles** and **Fallback** views inside
the accepted TUI. Keep its composer, activity, focus, scrolling and optional Herdr.

- Providers shows executable/endpoint, version, authentication-reference status,
  model inventory source/freshness, supported settings and role evidence separately.
  Refresh performs bounded local discovery only; opening Settings never infers,
  signs in, starts a provider server, downloads models or changes shared settings.
  Missing authentication links to the provider's normal login instructions.
- Select a named profile's client, model and model-specific effort/options. Hide
  inapplicable controls; show unsupported values disabled with a reason, and unknown
  availability as **unverified**, not unavailable or proven usable. Allow an explicit
  model ID when discovery cannot enumerate it; preserve that uncertainty. Validate
  required adapter capabilities locally, then establish role usability during ordinary
  authorized work, without a separate admission trial.
- Save project preferences; show the effective profile, where it came from, and
  whether it applies now or to a future attempt. Saving is not scope authorization.
  Review any newly added provider/context destination as a policy expansion once;
  choices already permitted by the policy need no renewed approval.
- F6 exposes role and planned/effective provider/model/effort for each task. A task
  override can inherit, pin one profile or permit an explicit set. Running attempts
  keep their frozen selection. Inspect/history expose routing rationale, capability
  evidence, switch/fallback status and requested versus provider-reported settings.

Useful delivery means a normal TUI development request uses real Codex and Claude
roles, reviewed exact commits pass combined checks and integrate, and conversation,
decisions, history and selected context survive a coordinator change and resume.
Retain the existing working Codex default; make Claude selectable for discussion,
development coordination, implementation and independent review. Ollama/OpenCode
execution is subsequent work, not a dependency of this outcome.

## Inspected source and local evidence

Source already supplies the scheduler, independent worker lane, task worktrees,
serialized integration, receipts, capacity waiting and durable context/history.
The remaining coupling is concrete:

| Source | Reuse and change |
| --- | --- |
| [development.rs](../../../backend/src/development.rs) | `profile`/`policy` hard-code Codex gpt-6.1-sol/medium and empty fallback; readiness is a literal inventory. Split capability discovery, routing resolution and approved policy from execution. Dispatch currently copies one workflow profile into all units. |
| [runner.rs](../../../backend/src/runner.rs), [adapters.rs](../../../backend/src/adapters.rs) | Retain supervised spawn, output bounds, cancellation and receipts; dispatch prepare/event/completion/capacity decoding through the selected adapter instead of Codex-specific development functions. |
| [claude_chat.rs](../../../backend/src/claude_chat.rs), [continuous_adapter.rs](../../../backend/src/continuous_adapter.rs) | Reuse CLI authentication references, safe settings, stream/structured-result parsing and attribution. The old text-only profile, exact binary registry, one-turn setting and 120-second deadline are historical semantics, not the development adapter. |
| [conversation_store.rs](../../../backend/src/conversation_store.rs), [continuous.rs](../../../backend/src/continuous.rs), [task_scope.rs](../../../backend/src/task_scope.rs) | Reuse commands, binding generation, causal queue, exact policy decisions and selected context. Current native selection is allowance authorization, not general switching; state8 initialization also writes Codex binding values. Replace those assumptions in the new path. |
| [prototype/view.rs](../../../frontend/src/prototype/view.rs), [rust_runtime.rs](../../../frontend/src/rust_runtime.rs) | Extend existing Settings and init preferences; remove literal Codex presentation in the new path. Retain normal packaging/startup. |
| [request_history.rs](../../../backend/src/request_history.rs), [local_context.rs](../../../backend/src/local_context.rs) | Add routing/switch facts to current events and compact context; no new history system or vendor-session dependency. |

Read-only discovery on 2026-10-16 used local versions/help, selected non-secret config
fields, credential-file presence and metadata from the already-running Ollama endpoint:

| Client | Observed interfaces/configuration | What is not established |
| --- | --- | --- |
| Codex | Default 0.163.0; retained 0.162.1 binary exists. `exec` supports JSONL, output schema/final file, ephemeral execution, ignoring user config/rules. Auth/config files exist; shared config says gpt-6.1-sol/xhigh. | Current source accepts 0.162.1 and explicitly selects medium. The model cache reports client 0.154.0 and advertises other models, omitting gpt-6.1-sol; it cannot invalidate the retained working profile or prove new access. No calls made. |
| Claude | Default 2.1.301; retained 2.1.296 exists. Saved credential/settings files exist; shared settings select `opus`/high. Help offers print/stream-json, JSON schema, model/effort, explicit tools/settings, safe/restricted modes and host permission handling. | Aliases are not exact model identities. CLI effort tokens low/medium/high/xhigh/max do not prove every model supports each. Current source's claude-sonnet-5-5/medium profile is older conversation evidence, not development-role proof for 2.1.301. |
| Ollama / Qwen | Existing loopback Ollama 0.40.4 advertises qwen3:8b, Q4_K_M, 5.23 GB; metadata advertises completion/tools/thinking and 40,960 context length. No model loaded by discovery. | Actual usable context, tool reliability, latency and development-role quality are unverified. Thinking capability is not a universal effort scale. |
| OpenCode | `~/.opencode/bin/opencode` 2.0.29; `run --format json`, `provider/model#variant`, standalone/private-server option. `models` uses a server. | Common legacy config/auth paths were absent; actual configuration/auth remain unverified, not proven absent. No server/model-list command was started. Client is an agent frontend; record its underlying model provider separately. |

Do not import shared model/effort preferences implicitly. Preserve exact executable
paths for working profiles; an upgraded PATH binary is newly discovered evidence,
not permission to change a frozen attempt. Support the installed interfaces with
focused compatibility checks as implementation work, not another trial program.

## Profiles, routing and authority

Evolve the existing project-local `.waywr/coordinator.json` into a versioned preferences
file with named profiles, coordinator selection, worker default, role preferences,
per-role allowed alternatives and optional ordered fallback. Do not add a parallel
provider configuration tree. A profile names adapter/client, absolute executable or
endpoint, model ID, native effort/options and authentication **reference**, never
credentials. Cache discovery/evidence in existing project state; runtime records
retain immutable preference revisions/hashes. Persist UI edits through the writer,
then atomically publish the file; reconcile interrupted publication. Detect external
file edits by hash and validate/import as a new revision, never overwrite silently.

Use this explicit precedence for a new attempt:

1. The approved operating policy constrains every choice: allowed destinations,
   profiles/model options, role capabilities, project access and optional caps.
2. A user task override wins; a pinned profile cannot be changed by the coordinator.
3. Otherwise the coordinator may name a suitable profile within the task/role's
   allowed set, based on capability, difficulty and the user's quality/cost preference.
4. Without a valid explicit choice, use the role default, then project worker default.
   The coordinator has its own selection. Never inherit an arbitrary vendor default.

Reject unsupported choices visibly rather than silently clamping effort or changing
models. Reuse the assignment decision's concise `reason` for routing; add no explanation
call. Record the chosen profile revision, source of choice and rationale. Effort is
an adapter/model option: explicit native token, explicitly selected provider default,
or not applicable. Do not translate Codex xhigh into Claude high or Qwen thinking.
Resolve model aliases to exact IDs when authoritative metadata allows; otherwise show
an unresolved alias and record requested and reported identity. If runtime identity
violates the selected profile/policy, preserve evidence and stop that attempt's adoption;
do not call the substituted result a success. Unknown reported effort remains unknown.

Separate **routing preferences** from **permission**. Changing a default within the
approved set does not stale requirements, invalidate useful commits or renew scope.
Adding a new destination/permission needs a narrow, explicit policy amendment; retain
the original decision and unaffected work. Existing Codex-only authorization remains
Codex-only after upgrade. Initial mixed-provider approval can include both providers
and their allowed roles/options, so routine routing and switching need no ceremonies.

## Adapter and role responsibilities

Use a small Rust adapter boundary, extracted from current development functions:
local discovery/capabilities; profile validation; per-attempt launch arguments and
sanitized environment; event/activity normalization; strict final-result/identity
validation; usage and explicit capacity/error classification. Supervision, cancellation,
scheduling, Git, context selection and authorization remain application-owned.
No new general agent framework, service or scheduler is needed.

Claude uses existing saved login in place and per-invocation safe settings; its
`--bare` mode explicitly excludes OAuth/keychain in the installed help, so it is not
an interchangeable isolation shortcut. Suppress inherited hooks/plugins/MCP/rules,
provider sessions and automatic fallback while retaining provider auth. Coordinator
calls produce typed decisions with tools disabled. Implementation gets the assigned
worktree and approved edit/build/test/commit access; reuse project/common-Git permission
boundaries. Wire Claude's supported host permission mechanism to the saved operating
policy for routine actions, including Git commits; escalate only genuine expansion.
Do not rely on `acceptEdits` alone for command permission or unconditional bypass.
Keep managed policy restrictions visible; never weaken shared configuration to launch.
Retain the documented trusted-client/same-UID boundary; worktrees alone are not a
security sandbox. Credential references stay outside prompts and public history.

Add an explicit **review** role to existing units. Freeze the exact submitted commits,
base/diff, contracts, obligations and check evidence in its packet. Review returns
structured findings and the exact reviewed revision, not a mandatory new code commit.
Use bounded supplied evidence and, where needed, a read-only checkout; request checks
through existing service execution. Corrections are implementation assignments.
The coordinator consumes the report and existing integration checks still gate
acceptance. A reviewer, including one from another provider, cannot mutate integration
or approve its own prose as fact. Extend schemas/validators to distinguish review
reports from worker commit submissions; do not invent an empty fake commit.

Subsequent adapters reuse this boundary: Ollama needs an application-owned bounded
context/tool loop and host-mediated project tools because it is an inference server,
not an editing client. Start with narrow review/analysis roles when later authorized;
metadata alone does not prove an 8B model suitable for coordination. OpenCode needs
supervised private-server lifetime, explicit model/variant and underlying destination,
permission events, structured completion and shutdown handling; never attach an
unrelated daemon. Inventory can show both now with execution marked unavailable.

## Safe switching and fallback

Freeze profile/model/effort, executable identity, capability evidence, policy revision
and context hash at reservation for **each** attempt. Preference edits affect only
unreserved future attempts. Reserved-but-not-spawned operations stay frozen unless
explicitly withdrawn with positive nonlaunch proof; never mutate their packets.
Queued tasks show recalculated choices before reservation. Running workers continue
when their coordinator changes.

A coordinator switch is a persisted request with expected binding/config revision.
Default **switch after current turn** fences new old-provider discussion and workflow
coordination reservations, lets existing calls settle/adopt once, then atomically
advances binding generation for both lanes. Rebind only unreserved conversation input;
preserve order, questions, decisions, scope, task records and pending Git intents.
Workers and already-authorized service checks/integration can continue. The new
coordinator reads a consistent checkpoint after any adoption, including current
integration intent, without replaying commands or treating incomplete integration as
accepted. A brief switch wait leaves the composer and unrelated work responsive.

An explicit **interrupt and switch** uses existing targeted cancellation and waits
for positive settlement; ambiguous old execution blocks replacement of that lane.
After the committed generation change, late old-generation actions cannot adopt.
Persist requested/waiting/applied/blocked/cancelled switch state, deduplicate commands
and recover it on restart. Preserve old receipts even when their actions are fenced.
One project coordinator selection governs conversation and development coordination;
separate concurrent coordinator personalities are outside this increment.

Fallback defaults to wait/resume. User-configured ordered alternatives are per role,
within the approved destination/capability policy, with visible reasons and history.
Disable opaque client fallback so Waywr owns every change. On explicit capacity refusal,
persist retry-after/backoff; do not spin or infer precise remaining provider capacity.
Only after settlement reconcile saved files, commits and partial-input fingerprints;
then schedule a linked replacement attempt with the chosen alternative and updated
selected context. Uncertainty, dirty unexplained edits, auth/config errors or policy
refusal are not automatic fallback triggers. Preserve completed work and original
failure evidence. If no eligible alternative remains, wait or ask a substantive
question; never invent a call/time/repair ceiling. A coordinator fallback uses the
same binding handover, not an invisible provider swap.

## State, context and implementation sequence

Keep the existing database/writer, events, operation packets, request history and
IPC transport. Minimal additions are preference revisions/discovery provenance,
allowed-profile policy references, unit role/override/routing reason, frozen attempt
selection, review evidence and switch/fallback state. Extend negotiated snapshots,
Settings/task reads and idempotent commands for save/override/switch/policy amendment.
Use exact config/binding/policy revision guards and existing command IDs.

Recommend the next state/config/IPC compatibility versions (state9/config6/IPC1.7)
for these changed authorization/role meanings. Preserve state8 operation decoders and
immutable packets/receipts. A normal selected-project release upgrade with the service
quiescent can import the old profile as the sole allowed profile, without rewriting
scope approval or granting Claude access. Leave state3–7 semantics untouched. Old
binaries must refuse new-state mutation. This is compatibility work inside delivery,
not an evaluation installation or separate migration program.

Application-owned context supplies recent relevant discussion, accepted decisions,
current requirements/policy, task/dependency/check state, source knowledge and compact
request summaries with source links. Fit it to the selected model's supported context
budget, disclose omissions and preserve authority-bearing inputs; retrieve further
selected evidence if needed. Keep full transcripts/hidden reasoning out. Vendor
session continuity is optional transport metadata, never the source of project memory.

1. **Make current selection real in the TUI.** Extract the working Codex adapter,
   add provenance-bearing discovery and project profile/precedence editing, separate
   preference changes from permission, and freeze resolved selections per attempt.
   Preserve current Codex behavior and expose Claude readiness honestly.
2. **Connect mixed-provider work.** Add Claude launch/permission/stream/result handling
   for conversation, coordination and editing; add exact-commit review units and
   policy-validated coordinator profile choices. Wire activity/history and independent
   scheduling through the existing records, worktrees and integration authority.
3. **Complete switching/fallback in ordinary use.** Implement generation-safe handover,
   selected-context transfer and optional settled-attempt fallback. Use one useful
   authorized development request through the stable TUI: Codex coordination/editing
   with Claude review, then a safe coordinator change and a subsequent appropriately
   routed task/turn. Exercise Claude editing as useful work, not a disposable trial.
   Run the project's relevant checks and integrate; fix defects along that path.

Checkpoints are recoverable commits, not approval stops. Focus tests on routing
precedence/unsupported effort, config-save recovery and policy expansion, immutable
attempts under edits, both adapter stream/result/permission boundaries, review evidence
staleness, switch completion/cancel/restart races and settled versus uncertain fallback.
Reuse existing receipt, worktree, history and context tests; include an assertion that
no secrets or cross-project context enter discovery/logs. UI checks cover Save/inherit,
readiness labels and pending switch while workers continue. One real mixed-provider
job demonstrates the connection; no exhaustive provider/failure matrix or repetition
of Example Task App's completed migration is required. Linux portability, unrelated cleanup
and every-provider execution remain outside scope.

## Product choices

- **Initial defaults:** recommend keeping the proven Codex gpt-6.1-sol/medium profile
  unchanged and making Claude an explicit project/role choice. Seed its model choices
  from provenance-bearing metadata or a user-entered exact ID; do not silently adopt
  the shared `opus` alias/high setting or replace Codex because a cache omitted it.
- **Fallback/context sharing:** recommend disabled automatic fallback by default.
  Opt in to named destinations/roles once in the operating policy; default switching
  waits for the current coordinator turn. This makes cost and context destinations
  predictable while allowing unattended routine routing within that policy.

No other product decision blocks implementation planning. This pass changes documents
only: no inference/workers, services, installation/shared settings, push or merge.
