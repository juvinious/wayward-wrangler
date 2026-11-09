> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Superseded initial development workflow proposal

Historical snapshot from commit `c4287a5`, originally
`docs/development-workflow-plan.md`. Superseded by the user's Git-backed parallel
development and delivery/resource instructions. The sole current proposal is
[the maintained plan](../../development-workflow-plan.md).

The single-editing-worker assumption, suggested time/call/repair ceilings,
separate discussion allowance and project-local candidate delivery path below are
retired guidance, not current requirements or authority. Source links are adjusted
for this archive location; original bytes remain in Git at the cited revision.

---

# Proposal: deliver a real development workflow

Planning only, 2026-10-15. Base: merged main `1813a2f7de5f10a8eb3ef05232236148af370509`
(PR #7), whose tree exactly matches cleanup checkpoint `0a1f9726dc21d100758239db46db3ac0744ab1f6`.
Branch: `plan/complete-development-workflow`. This proposal grants no execution
permission and does not change implemented contracts.

## Outcome and first project

**Waywr must develop Example Task App, a small offline Rust task-tracker CLI, through
its ordinary TUI.** The user starts in a new, separate directory with `waywr init .`
then `waywr`, discusses the application, selects models, approves scope and an
operating policy, and receives a runnable, checked project. No target exists yet.
The agent implementing Waywr must not write Example Task App's application directly
and count that as success.

Proposed scope: `add`, `list`, `done`, `reopen`, tags/filtering and Markdown export.
Data stays in a selected local JSON file; no account, server, sync, deployment or
runtime network. Recommend minimal Rust dependencies and a committed lockfile.

Definition of done:

- Add/filter tasks, complete/reopen stable IDs and export a readable Markdown list.
  Data survives restart; Unicode works. Invalid input/IDs and corrupt storage give
  useful errors without overwriting data; writes are atomic.
- Waywr workers create/edit actual files. Waywr records passing
  `cargo test --locked`, `cargo build --locked --release` and binary smoke checks
  against the integrated revision. Tests use isolated data directories.
- A separate review operation checks the diff/requirements. Substantive findings
  are resolved or explicitly decided. README covers Fish-compatible build/use
  examples, data location and recovery.
- The user can inspect/run the result. Waywr records closeout, notifies the TUI,
  and resumes with maintained knowledge/evidence. A real mid-run steering change
  and detach/reconnect demonstrate the complete development workflow.

## Gap and reuse

| Area | Current source | Required connection |
| --- | --- | --- |
| Startup/TUI | Project-local init, paired release, accepted Ratatui UI, continuous chat/queue and scope review | Provider selection, operating policy, work activity and closeout in that UI |
| Providers | Pinned Claude conversation; Codex/Claude text-only adapters | Role-aware configuration and a worker that edits files and runs checks |
| Execution | Tokio service, bounded SQLite writer, deduplication, fenced systemd hosts, cancellation/receipts | Admit authorized development work and record real file/check results |
| Work management | Scope cannot admit execution; queued-task edits; fixed drafting/four-file workflows | Dependency graph, active steering, review and integration |
| Context | Selected local documents/approved records, hashes/provenance and exact artifact review | Incremental source map, maintained decisions/conventions and input-aware reuse |

Reuse [task_scope](../../../backend/src/task_scope.rs), [continuous](../../../backend/src/continuous.rs),
[session_store](../../../backend/src/session_store.rs), [runner](../../../backend/src/runner.rs)
and [local_context](../../../backend/src/local_context.rs). Extend the existing service/
writer, not another orchestrator. Reuse validation and findings from
[workflow_policy](../../../backend/src/workflow_policy.rs) and
[static_files](../../../backend/src/static_files.rs), without imposing their fixed stage
and four-file shapes on software development.

## Provider evidence and recommendation

Read-only local observations, 2026-10-15: versions/help, selected nonsecret settings,
credential-file presence and Ollama inventory. No inference or login validation.

| Tool | Discovered/configured evidence | Usability for Waywr |
| --- | --- | --- |
| Codex | `~/.local/bin/codex`, 0.162.1; config/auth file present; configured `gpt-6.1-sol`/`xhigh`; cached model/effort inventory | First integration candidate; authentication and development execution unverified |
| Claude | `~/.local/bin/claude`, 2.1.300; settings/credential file present, configured alias `opus` | Does not match Waywr's 2.1.296/Sonnet 5.5 medium profile |
| OpenCode | `~/.opencode/bin/opencode`, v2.0.29; Fish includes it, this Bash PATH does not; standard config/auth files not found | Discovered; provider configuration and supported roles unverified |
| Ollama | Existing loopback server/client 0.40.4; `/api/tags` lists qwen3:8b Q4_K_M, about 5.23 GB | Inventory reachable; inference, working context size and worker adapter unverified |

Hardware: ~62.7 GiB RAM; RTX 4070, 12,282 MiB VRAM, 9,317 MiB free at inspection.
This makes a local helper worth evaluating, not a performance/context guarantee.

Recommend **Codex for coordinator and worker roles first**, using locally listed
`gpt-6.1-sol` at medium effort. Reuse conversation semantics/structured results while
removing Claude-specific routing from the new contract. This avoids requiring both
a Claude profile refresh and another client's execution integration. Keep old
Claude projects on their bound contract.

Introduce one small provider interface: inventory, supported roles/models/efforts,
launch, stream normalization, cancellation and completion/usage evidence. Distinguish
client from provider endpoint and **discovered**, **configured**, **usable for this
role**, with reasons and evidence freshness. Presence never proves usability. Bind
executable identity, adapter capability version and settings to each attempt; check
drift without recreating a historical trial program. No silent mock/model fallback.

Only Codex execution is required for this outcome. Discover the others in the product;
add their execution adapters independently later. Consider qwen3:8b for bounded
summaries/classification after evaluation. Show only genuinely supported efforts;
use “not supported” where appropriate. Never load a model during discovery.

## Implementation order — one connected delivery

1. **Discussion to approval.** Extend `rust_runtime`, settings and the conversation
   seam with discovery, coordinator/worker preferences and project knowledge. Add a
   versioned development contract for new projects; update bundle/schema/IPC handling
   together, preserving old records without implicit conversion or reauthorization.
   Provide one complete scope-and-policy review with exact revision/hash checks.
   Result: discuss Example Task App and authorize its development through the normal TUI.
2. **Approval to actual work.** Add work units/dependencies and policy reservations
   to the existing writer/scheduler. Extend [adapters](../../../backend/src/adapters.rs)
   and runner for Codex editing in project-owned task workspaces and supervised
   build/test commands. Preserve intent/claim/permit/receipt fencing; capture tool
   activity instead of rejecting every file/command event as the text-only path does.
   Result: real candidate files and check results, without per-step permission.
3. **Steering through closeout.** Reconcile active/queued work, route questions,
   review and integrate candidates, check the integrated revision and refresh memory.
   Show truthful task/check/save status, blockers, usage and run instructions. Fix
   misleading completion/save wording and narrow `--project=DIR` routing while
   touching those paths. Result: the usable lifecycle, including resume.
4. **Deliver Example Task App through Waywr.** With explicit runtime authorization, use
   a project-local paired release and the agreed empty target; no global installation
   is needed. Exercise the normal TUI, one scoped steering change and detach/reconnect.
   Inspect/run the result and retain receipts, diffs, checks and closeout. Fix issues
   in this path rather than declaring separate mock milestones complete.

This ordering does not imply four approval rounds. Complete the connected scope
under its authorization; ask only when an action exceeds it. All-provider support,
parallel workers, semantic search and broad legacy deletion are not prerequisites.

## Authorization, work and integration behavior

Human approval binds project identity/root, requirement revision, initial plan,
decision dependencies and operating policy. Policy covers paths, tools/commands,
network, permitted models, concurrency, execution/cost limits, retries and integration.
Work-unit grants derive from that approval and freeze inputs; models cannot create
authority. Coordinator refinements/splits and routine repairs within the approved
scope proceed automatically with recorded lineage. Boundary expansion needs a delta
review, not reapproval of unaffected work.

At setup, authorize provider use and a session allowance for discussion once; that
does not permit worker edits. Work approval adds execution authority. Carry prior
usage forward rather than resetting the budget when discussion becomes development.

Recommend one editing worker initially, with responsive coordinator chat alongside it,
isolated project-owned Git workspaces and one integration writer. Waywr may initialize
Git in an approved empty target. For existing projects, inventory and preserve dirty/
untracked work; explicitly include relevant contents in the baseline, never reset,
stash away or silently omit them.

Workers read relevant source/conventions, edit assigned workspaces and run approved
build/test/format commands. Service authority records and integration refs remain
application-owned. Enforce writes with supported client/host controls; validate diffs,
path ownership and symlink escapes. Exclude secrets/unrelated projects from model
context; existing login stays in client credential handling. Preflight the actual
boundary: `cwd` and prompts alone are insufficient. Retain the documented trusted-
client/same-UID limitation instead of claiming hostile-process confinement.

Recommend local edits/checks/documentation and clean integration under policy.
Allow provider transport; dependency fetching is a bounded application-owned
preparation step only if authorized and the Cargo cache is insufficient. No arbitrary
worker network, other-project writes, global install, remote push/merge or publication.

Editable starting limits: **60 minutes, 40 total native agent launches, 10 minutes
per worker, 3 minutes per coordinator turn, two repair attempts per work unit, one
editing worker**. Native launches are distinct from internal provider requests;
capture usage and client retry behavior where exposed. Enforce a money ceiling only
with reliable usage/pricing and reservation bounds; otherwise show cost as unknown
and use explicit time/launch limits. Reserve durably, stop new dispatch at exhaustion,
retain active-operation deadlines and request an explicit extension. Uncertain
execution is never refunded/replayed. These are configurable defaults, not product caps.

Each unit records ID/revision, requirement links, assigned model/effort/rationale,
dependencies, base/input hashes, context, allowed paths/tools, expected outputs,
acceptance commands, budget and result/review state. Route by capability, then
difficulty and user quality/cost preferences, within approved choices. Independent
review can use the same model in a fresh read-only operation. If no approved model
fits, ask for a decision rather than silently escalating cost/capability.

Suggested Example Task App units, refined and saved by Waywr during discussion:

| Unit/dependency | Expected output and acceptance |
| --- | --- |
| Storage/CLI skeleton | Buildable crate, data schema, atomic store; restart/corruption tests |
| Commands, after storage | Add/list/done/reopen; stable-ID and invalid-input binary checks |
| Filtering/export, after commands | Tags and Markdown export; Unicode/filter/export checks |
| Review/repair, after implementation | Requirement-linked findings and resolutions; repaired checks rerun |
| Integration/closeout, after review | Integrated revision, README, build/test/smoke receipts and memory update |

Waywr workers produce all application files; no externally generated scaffold.

Steering is durable before acknowledgement. Discussion alone changes no authority.
An explicit steering action records the requirement/decision delta and affected units;
ambiguous intent opens a proposed change for confirmation. In-scope changes follow
delegated policy. Recompute queued/downstream work, fence stale integration and cancel
affected active work or retain its eventual candidate according to policy. Never
mutate a frozen packet. Keep unaffected results; revalidate useful affected results
against changed inputs. Blocking questions pause dependent work only; saved answers
resume routine scheduling automatically.

Integration checks receipt, allowed diff, base revision, requirements/dependencies,
review disposition and check evidence. Journal intent/completion so restart reconciles
an already applied diff without applying twice. External edits/conflicts hold the
candidate for reconciliation without overwriting user work. Run acceptance on the
integrated revision before declaring done. Retain uncertain attempts/partial output;
missing processes never establish nonlaunch.

## Project memory and simplification

Extend local context/provenance into application-owned project knowledge: source/
architecture map, conventions, requirements/decisions, accepted work/checks, issues
and next actions. Canonical records live in project state with readable project-local
summaries. Initially map relevant source/build/convention files, excluding secrets,
generated data and runtime state. Track content hashes and dependencies; refresh
affected entries after external edits, steering, integration and resume. Summaries
cite source revisions and remain derived knowledge, never authorization. Provider
history is not canonical memory.

Reuse requires matching requirements/decisions, source inputs, dependency outputs,
relevant toolchain/configuration and acceptance evidence. Invalidate consumers
transitively; uncertain dependency coverage requires conservative revalidation.
Similarity can locate candidates, never prove validity. Check freshness at dispatch
and integration, including edits made while detached.

Retire six-call/coordinator-only/tool-free/four-file restrictions from the **new**
path, retaining old versioned semantics. Replace fixed drafting stages with work
units and repeated mandatory scope navigation with one complete approval review.
Keep optional detail panels, exact authority, bounded output and immutable evidence.
No new runtime/framework or Herdr dependency. Trial scripts stop being the main
product path. Python cleanup follows resource review: `rust_bundle.rs` still packages
three prompts from `src/agent_workflow/resources/skills`. Move those before deleting
legacy application/package code; keep useful focused harnesses.

## Verification and decisions for review

Test changed behavior and risks: grants/stale revisions, dependency invalidation,
adapter events, workspace escapes/dirty files, budgets, cancel/completion and receipt/
integration recovery. Focus TUI checks on selection, authorization, steering and
truthful status. Build the three locked Rust release binaries using permanent rustup
1.99.0. Read harnesses first; some launch services. The native TUI run delivering
Example Task App is the acceptance evidence. Mocks cover failure cases; no comprehensive
historical rerun or succession of mock acceptance rounds is required.

Three user decisions materially affect the next scope:

1. **Application/destination:** recommend Example Task App at new sibling
   `~/projects/example-task-app`, checking the path before any creation.
2. **Provider/resource envelope:** recommend Codex/`gpt-6.1-sol` medium for both
   roles, existing login and the editable limits above. Provider use needs explicit
   authorization before the live run; this plan authorizes no authenticated calls.
3. **Autonomy:** recommend project-local edits/checks/docs, local Git commits and
   clean integration automatically after scope/policy approval. Ask for substantive
   choices, conflicts and authority expansion. Allow bounded dependency fetching
   only if needed. Final closeout stays inspectable; publication remains separate.

This pass fetched origin, verified exact trees, synchronized clean main, created the
planning branch and inspected source/local inventory. No application code, target,
service, inference, installation or runtime test was created/run. Documentation-only
link/reference and diff checks apply.
