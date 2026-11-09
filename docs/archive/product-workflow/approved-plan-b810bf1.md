> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

Archived after implementation. Current contracts: [workflow](../../ordinary-project-workflow.md), [providers](../../provider-selection.md), [distribution](../../distribution.md). Native/GitHub verification limits remain in [HANDOFF](../../HANDOFF.md). Historical observations are evidence, not new authority.

# Ordinary project development — connected implementation plan

Proposal, 2026-10-18, based on preserved `impl/coordinator-owned-recovery` at
`d1dda2450389b310ccd3bca469692fffa75ade69`, not a reset to main. Main is verified at
`799841c` (PR #9); the 13 unmerged recovery/convergence commits remain included.
[Repository cleanup](../../repository-cleanup-2026-10-18.md) is completed. Everything
below is proposed application work; no runtime changes, managed-project actions,
installation or publication are authorized by this planning pass.

The outcome is ordinary development on **the user's existing projects and actual
requests**: initialize, select roles, discuss and approve work/policy, delegate,
review a usable candidate, apply feedback, complete the selected integration policy,
and resume with maintained context. Preserve the production Rust/Ratatui interface,
one active workflow with concurrent tasks, optional Herdr inspection, exact authority,
supervised execution, recovery and project isolation. No further Example Task App demo,
new orchestration framework or checkpoint approval chain.

## Implementation assessment

| Capability | Current implementation | Remaining work |
| --- | --- | --- |
| Normal startup/conversation | Rust init/start/resume, project bindings, causal chat, exact inline approval, questions and steering; no seeded state9 job. | Connect setup, candidate validation and policy completion into understandable ordinary use. Current readiness remains experimental. |
| Delegation and recovery | Independent task worktrees; start/accept dependencies; exact reviews, combined receipts and serialized integration. Unmerged recovery adds typed causes, finding lineage, progress reassessment and obsolete unlaunched-task retirement. | Preserve this work. Add candidate/ownership/cleanup transitions using its safeguards; do not redo the completed recovery increment. |
| Request Git lifecycle | Readable `waywr/<title>-<id>/integration`, temporary task namespaces, legacy result aliases; exact Git-effect reconciliation. | Current code requires primary checkout on literal `main` and only `checked-fast-forward`. No candidate checkout handoff, policy-driven PR completion or automatic resource retirement. Retention is currently indefinite. |
| Delivery guidance | Conversation/Inspect exposes combined branch, directory, commit and exact check status; saved questions can request human feedback. | A versioned, project-specific delivery card tying build/run/validation instructions, feedback and any required acceptance to the actual candidate. No special app launcher is needed. |
| Providers | Codex/Claude roles, per-project profiles/defaults/task overrides, native effort tokens, pinned attempts, safe switching and opt-in capacity fallback. | Discovery is PATH/version-only for four client names; Gemini and local model catalogs are missing. Add usable Ollama coordinator/reviewer execution, not just catalog entries. Manual IDs and narrow version allowlists cause avoidable setup friction. |
| Existing-project knowledge | Root instruction/build-file excerpts, tracked-file listing, exact source/diff retrieval, active memory and journal-derived history; knowledge refreshed on integration. | Current map is shallow: eight root documents at 4 KiB each, first 160 non-dot tracked paths; accepted contracts select first 24 matching files within 32 KiB. No comprehensive nested-instruction or incremental architecture/convention map. |
| Checks | Coordinator chooses checks; no universal Example Task App command pair. Executor accepts Cargo, tracked shell/Python scripts and worktree executables. | Respect existing project toolchains directly through scoped argv recipes instead of requiring every project to add a wrapper. Separate long-running preview/deployment from finite checks. |
| Distribution | `frontend/src/rust_bundle.rs` builds/validates/installs three paired binaries plus resources; staged install, hash validation, stable launcher and guarded binding upgrade/rollback exist. | No `.github` workflows or releases. Add CI, repeatable release assembly, authenticated downloads and simple binary installation. |

Source owners: `development.rs`, `development_context.rs`, `development_recovery.rs`,
`review_findings.rs`, `provider_selection.rs`, `coordination.rs`, `request_history.rs`,
`frontend/src/rust_runtime.rs`, `rust_bundle.rs`, `rust_maintenance.rs` and existing
TUI view modules. Example Task App names found in current Rust development logic are
regression fixtures, not a production task template. Rust-specific worker guidance
and Python cache handling exist; generalize only where they obstruct actual project
conventions. Nonempty non-Git/unborn projects currently refuse implicit baselining;
keep preservation, but provide the decision through the TUI.

## 1. Establish the project's real baseline and operating policy

At init/attach, inspect Git root/common directory, branch/HEAD, remotes/default branch,
linked worktrees, dirty/untracked state and applicable instructions. Distinguish a
selected code baseline from the user's working directory. Recommend current committed
HEAD for existing work, with a clearly named integration target inferred from configured
upstream/default branch. Never assume `main`, switch the user away from their work,
stash/reset, or silently include uncommitted changes. Offer an explicit choice only
when dirty edits must be included or the baseline/target is ambiguous. Committed-base
work can proceed in isolated worktrees while unrelated local edits remain untouched.
Non-Git/unborn input requires an understandable baseline decision; do not implicitly
commit user files. Init's current `.gitignore` edit must not become unexplained dirt
that later blocks delivery; prefer project-local Git exclusion for Waywr state.
Keep one canonical project identity/state location across request worktrees; checking
out a candidate must not initialize another project or duplicate its authority.

Existing projects retain their saved, agreed integration policy, including any human
validation requirement; discovery, upgrade and new defaults must not replace it.
For a new project, setup visibly establishes the initial baseline, named target and
completion policy: checked local integration or PR delivery, and explicitly whether
human validation precedes integration. Show these choices in the scope/policy card;
do not infer acceptance policy from an empty repository or a default branch name.

Keep one scope/operating-policy approval. Include baseline/target, provider grants,
routine project commands, validation conditions, integration/publication policy and
retention. Preserve existing automatic checked local advancement where selected.
Offer a GitHub PR policy for projects that use it, with allowed remote/base/merge
method and publication authority agreed up front. Repository rules/checks still
apply. Remote permission absent from the policy requires a narrow decision once;
routine local commits, review, clean integration and cleanup need no further approval.
Human validation is a requirement only when the user/scope/policy asks for it.

Extend existing project knowledge with a paged path/module index, applicable root
and nested instructions, architecture/conventions, manifests/lockfiles/CI recipes,
requirements/decisions and next steps. Preserve native client instruction handling;
provide the same relevant constraints to the coordinator and reviewers. Inspect
only project-owned paths, honor exclusions, and never ingest credentials or another
project through symlinks. Mark missing/truncated/unverified context explicitly.
On integration, steering, resume or external changes, compare saved commits/file
hashes and refresh affected entries, not the whole transcript or repository.
Record dirty excerpts separately from committed evidence. Reuse requires valid
requirements, input and dependency identities; similar descriptions are insufficient.

## 2. Finish the governing-branch and candidate lifecycle

Use the existing readable request branch as the **one governing branch**, separate
from the target branch. Its current `/integration` suffix can remain; do not rename
active legacy refs or create competing user-result branches. Workers own temporary
branches/worktrees. The coordinator selects exact results; the service alone applies
serialized integration/check/promotion effects. Dependencies, interface contracts,
tracked placeholders, finding lineage and remaining obligations retain their current
meaning. Worker completion is separate from integrated feature completion.

For substantial work, derive accountable units from requirements with relevant
context, owned paths, expected outputs and acceptance checks. Agree only the interfaces
needed for independent progress; distinguish dependencies needed to start from those
needed to accept. Track temporary substitutions until real functionality is verified.
Steering revises queued work, coordinates active changes and fences obsolete results
while retaining useful/unaffected work. Use configured concurrency where tasks are
independent, rather than forcing either serial work or parallelism.

Lifecycle:

```text
approved → developing → candidate ready → validating (when requested)
         → feedback/revision → new candidate
candidate satisfying agreed acceptance → local integration or PR policy
         → delivered → request resource cleanup
```

A candidate record binds request/generation, governing commit, relevant check and
review receipts, unresolved obligations, delivery instructions and required validation.
Candidate readiness does not mean accepted/delivered. Conversational feedback and
any acceptance target the presented candidate identity, not whichever HEAD exists
later. New code/requirements produce a new candidate; old feedback remains evidence
and is not silently treated as acceptance of the new one. No mandatory acceptance
click is added to requests whose agreed policy already permits automatic delivery.

**Normal checkout without competing writers:** at a clean, quiescent integration
point, persist a validation handoff and stop writes to the governing branch. Detach
the service integration worktree at that candidate commit, freeing the branch for
ordinary `git switch` in the user's chosen checkout. The TUI's “Check out candidate”
action may perform the same transition for a clean selected checkout; otherwise
explain which worktree owns it and offer the existing candidate directory. Never
use `--ignore-other-worktrees`, force checkout or overwrite dirty files.

Persist who owns the checkout/write handoff, its commit and prior checkout. While
the user validates, independent workers may continue but governing-branch writes
and promotion wait. Feedback/end-validation releases the handoff only after checking
actual branch/worktree state. Do not silently restore the user's previous branch.
User edits/commits are preserved and need explicit inclusion as input plus applicable
review/checks; they do not inherit old receipts. An out-of-band checkout/ref change
fences writes. Restart reconciles Git and the saved handoff; elapsed time never
revokes it. This is coordination of trusted clients, not a security boundary.

**Automatic worker cleanup:** after exact result integration/disposition, remove
eligible request-owned worktrees/branches when no active writer, dependent consumer,
open obligation, validation or recovery needs them. Do not wait until the entire
request closes if a unit is safely disposable. Persist a cleanup intent with exact
path/ref/head/ownership; recheck under Git/supervision fences. Dirty, unknown,
untracked or unclassified ignored content is retained with a concise reason. Only
known generated outputs covered by the saved retention policy may be discarded.
Never broad-prune worktrees or infer safety from missing processes.

Keep Git objects reachable before deleting refs: integrated parents normally remain
reachable through the governing branch; preserve rejected/superseded exact results
in a verified request Git bundle or non-branch archival ref when required. Squash/PR
completion must preserve original task commits before deleting the governing ref.
Keep durable assignments, receipts, finding/obligation history and content-addressed
artifacts in `.waywr`; retrieval must work from Git/artifacts after a directory is
gone. Append resource-retirement locations without rewriting original evidence.
Restart completes the same cleanup intent once. Unknown ownership remains retained.

After acceptance, perform the already-selected local merge/PR policy automatically.
Record exact expected target/head and reconcile publication by saved branch/PR
identity before repeating it. Changed target/conflicts return to integration/review;
do not silently force-push or bypass repository gates. Distinguish candidate accepted,
PR awaiting checks/merge, merged and delivered. Clean remaining owned resources only
after confirmed completion and release of validation/recovery dependencies. If the
governing branch is still checked out by the user, retain it and explain the deferred
cleanup instead of forcing their checkout.

## 3. Give every project a usable delivery handoff

Add one compact candidate card to the current conversation, with optional Details
and Inspect. It names the branch/commit and checkout, passed/failed/not-run checks,
remaining work, and exactly why the target has or has not advanced. Include specific
instructions derived from the candidate's README/manifests/conventions: prerequisites,
build command, run/inspect/test command, expected result, relevant paths/URLs and
shutdown/cleanup. Label worker/coordinator suggestions separately from actual receipts.

Use the project's appropriate environment: terminal apps run in a terminal, desktop
apps in a graphical session, web apps expose a configured URL, services name their
endpoint/lifecycle, libraries provide build/tests/examples. Cross-machine deploys
name the target and prerequisites. Do not imply everything runs inside Waywr or
create a Example Task App launcher. Existing supervised finite commands can execute
approved build/check recipes; optional terminal/browser/editor actions are convenience
controls, not acceptance. Long-lived preview processes require recorded ownership
and stop behavior; deployment, external effects or new environments require their
actual policy authority, not a generic “Run” grant. External manual validation is
valid and saved as a user report, not a fabricated execution receipt.

“Looks good” may satisfy an explicitly requested human check only for the current
presented candidate. Conditional feedback remains steering. The card shows the
resulting next action automatically; the user should not need to ask why main is
unchanged or locate a deep worktree to discover what to run.

## 4. Complete discovery and simplify existing role selection

Read-only local observations on 2026-10-18 (versions/help, file presence and an
already-running loopback catalog; no inference/login/service launch):

| Client/service | Actual observation | Waywr capability today |
| --- | --- | --- |
| Codex | PATH client 0.163.0; local auth file and cached model catalog exist. Cache lists models/effort tokens but does not currently list the saved `gpt-6.1-sol` default. | Coordinator/implementation/review supported; prior ordinary use proves specific saved profiles only. Cache absence must not delete/substitute a working pinned profile. |
| Claude Code | PATH client 2.1.302, local credential file exists; help exposes model/effort/JSON output. | Working adapter supports recorded 2.1.296/2.1.301; 2.1.302 is outside its current allowlist. Treat as detected/unverified compatibility until focused interface checks support it; do not broadly allow every version. |
| Gemini CLI | `/usr/bin/gemini`, package 0.63.0; help exposes headless JSON/stream output and sandbox/approval modes. Authentication unverified. | Not discovered or executable by Waywr yet. JSON support alone does not prove tool policy, result integrity or role usability. |
| Ollama | CLI 0.40.4; existing `127.0.0.1:11434` catalog advertises `qwen3:8b`, digest `500a1f06…`, about 5.23 GB on disk. | Client detection only, no model-service discovery or execution adapter. Presence/size is not loaded status, VRAM use, capacity or suitability. |
| OpenCode | Not on this session's PATH; explicit `~/.opencode/bin/opencode` is v2.0.29 and offers a models command. Authentication unverified. | PATH-only discovery misses it; no execution adapter. Identify this exact client/interface rather than assuming a similarly named tool's provider behavior. |

Separate **client/transport**, underlying provider/account or local endpoint, model,
native effort and supported role. Preserve existing Codex/Claude profile encodings
on import; new descriptors add explicit identity/capability metadata without copying
secrets. Discover PATH, saved explicit paths and known install locations; deduplicate
canonical executables. Prefer safe cached/configuration metadata and supported
non-inference catalog/status interfaces. Probe only configured or known local
endpoints; do not scan networks, auto-start servers, authenticate or pull models.
The [Ollama tags API](https://docs.ollama.com/api/tags) lists local models without
generation. Client metadata calls must have operational bounds and leave an
unavailable/unverified result when unsupported.

Show separate detected, configuration/model-discoverable, auth/availability-known,
adapter-supported and role-observed states, each with provenance/time. Credential
file presence is not authentication proof; catalog advertisement is not an allowed
Waywr role. Preserve existing authentication mechanisms; store references only.
Effort choices come from the exact supported client/model contract; unknown means
unverified/provider default, never a universal low/medium/high conversion.

Use existing F3 and conversation choices: select Coordinator, Implementation and
Review; pick discovered client/provider/model and supported effort; show the
effective defaults and save project-only preferences. Keep detailed fields available
without making them the normal setup path. Preserve precedence: approved request
pins/grants constrain task override → coordinator choice → role default → worker
default. Each attempt freezes its selection. Display “future attempts” versus the
current coordinator binding; switching defaults to after the current turn, preserves
application context and reconciles explicit interruption. Fallback remains off unless
configured/approved; wait/resume is valid, uncertainty blocks replacement, and a new
destination never receives context silently.

Deliver discovery for all five, preserve Codex/Claude execution, and include usable
**Ollama coordinator and reviewer execution in this increment**. Gemini and OpenCode
remain discoverable but disabled for execution, with the reason shown. Inspection
establishes no existing reusable execution adapter for either: Gemini's
[headless interface](https://geminicli.com/docs/cli/headless/) and OpenCode's client
still need tool-policy, identity and receipt integration. Their execution adapters
are explicit follow-ons, not prerequisites to this delivery.

### Minimal Ollama execution path

`development.rs` already supplies role instructions/result schemas and validates
typed coordinator/reviewer output; `development_context.rs` supplies exact evidence
continuations. Both roles operate without native filesystem tools. Extend the
existing prepare/inspect/finish seams, profile selection and runner, keeping the
scheduler, authority checks and result adoption shared with Codex/Claude.

- Add an Ollama HTTP adapter as a supervised Rust child mode of `waywr-runner`,
  using a maintained HTTP client, not another daemon or orchestration framework.
  Support ordinary coordinator discussion/scope proposals, authorized development
  decisions and exact-commit review. Send selected application-owned context to
  local [`/api/chat`](https://docs.ollama.com/api/chat) with the existing role
  schema in `format`; use streamed responses for progress/cancellation. No vendor
  session or whole-transcript replay. Typed retrieval, assignment and integration
  proposals go through existing service validation; the model never executes them
  directly. [Structured output support](https://docs.ollama.com/capabilities/structured-outputs)
  does not establish correct decisions or review quality.
- Extend profile encoding with a discriminated HTTP endpoint identity; do not
  require a fictitious client binary. Preserve old serialized profiles/hashes.
  Freeze endpoint, server compatibility, model/digest observation, supported native
  options, context selection and policy with each attempt. Initially support local
  models at the configured loopback service, excluding cloud-backed model aliases,
  redirects and implicit proxy routing. No service start, pull or shared configuration
  change. Check model identity against saved metadata; changed/uncertain identity
  is visible, never a silent substitution. A catalog digest is an observation,
  not remote attestation of which weights ran.
- Reuse compact memory and paged evidence across provider changes. Obtain model
  capabilities/context metadata through non-generating
  [`/api/show`](https://docs.ollama.com/api-reference/show-model-details); distinguish
  advertised capacity from the selected runtime context window. Fit instructions,
  required evidence and response headroom without silent truncation. Page optional
  evidence; report when mandatory input cannot fit. Show only supported native
  thinking options, separately from cloud effort scales; otherwise provider default.
  Never persist hidden thinking fields in transcript/history/context; retain public
  response content and factual transport metadata only.
- Persist send intent and dispatch evidence under existing attempt ownership before
  sending. Receipts bind request/attempt, frozen selection, input/result hashes,
  terminal response, reported usage and validation outcome. Require a complete
  response and strict role/schema/identity validation before action adoption; partial,
  truncated or contradictory output is not success. Reuse exact review targets,
  finding lineage, context retrieval and progress-based recovery. A settled invalid
  response can drive a changed approach; repeated equivalents trigger reassessment,
  not a fixed retry allowance or an automatic cloud substitution.
- Cancellation fences result adoption first, closes the stream and settles the
  supervised client. Record client settlement separately from server generation
  completion: child exit/socket closure alone is not remote completion evidence.
  For these tool-free calls, deterministic reconciliation may mark an explicitly
  cancelled attempt abandoned once the client has exited and the writer proves no
  result/action remains adoptable. Retain remote compute status as unknown; this
  permits an already-requested switch under saved policy, not replay of that attempt.
  A racing adopted result or pending Git effect must follow its existing reconciliation.
  Proven pre-send failures may continue after correction; restart adopts a saved
  valid terminal result once. Lost responses without conclusive abandonment/settlement
  remain visible and block automatic replay/replacement; do not invent a
  server request-status API or stop a shared Ollama service. Keep transport health
  bounds separate from useful-work deadlines. Capacity waits and only configured,
  approved fallback retain existing positive-settlement requirements.

The TUI must distinguish **adapter-supported role** from **observed model suitability**.
Allow explicit selection of a compatible local model for coordinator/review while
labelling unobserved roles; no discovery inference or separate admission trial.
`qwen3:8b` is a locally advertised candidate, not a promise of reliable coordination,
review or sufficient context for every request. Surface failures and evidence from
ordinary use. Recommend retaining working Codex/Claude defaults until the user chooses
otherwise. Ollama implementation/coding-worker selection is disabled with “tool
execution adapter not implemented”; model tool-call capability alone cannot enable
it. That follow-on needs real tool execution, worktree permissions and tool receipts.

## 5. Ship a versioned binary installation path

Repository visibility is currently **private**, with no releases. Preserve visibility.
Initial supported artifact: Linux x86_64 GNU on the verified Ubuntu 24.04 baseline
(host glibc 2.39). Build and verify that baseline; record actual ABI dependencies.
Do not advertise macOS, Windows, ARM or arbitrary Linux compatibility from compilation
alone. Document systemd user manager/session bus, Git and native-client sandbox
requirements, including current AppArmor limitations and Claude's bubblewrap/socat.
Installing binaries does not establish provider execution support or change OS policy.

- `pr-checks.yml`: read-only token, pinned action SHAs, locked Rust 1.99.0/toolchain;
  relevant Rust tests, protocol consistency, docs links/diff and three-binary build.
  Select changed behavior, reuse current checks and read harnesses before enabling
  them. Do not run native inference or old installed-acceptance programs on every PR.
  Keep service-dependent checks isolated and explicit; hosted CI is not proof of a
  user's sandbox/session setup.
- `release.yml`: explicit version tag/manual release workflow on an authorized
  commit; same locked build in a digest-pinned baseline image, stable build path,
  path remapping and source-derived timestamp. Assemble with existing `bundle-build`;
  normalize archive ordering, ownership/modes and timestamps. Record source SHA,
  toolchain, lockfile, builder digest, target/ABI and contract versions. Compare two
  clean release builds before claiming byte reproducibility; diagnose any difference.
  Produce versioned archive, manifest, `SHA256SUMS`, install script and build metadata.
  Test extraction, paired integrity and installation in a temporary user prefix.
  Publication uses a narrowly scoped job, never untrusted PR code or model credentials.

Provide one documented release installer: download the versioned `install-waywr.sh`
and verified archive, then `sh ./install-waywr.sh --version vX.Y.Z`. The script selects
the supported asset, verifies the expected GitHub release/tag/digest and checksums,
rejects escaping archive paths/symlinks, stages the download, then invokes that
bundle's existing `bundle-install`. Users need neither Rust nor a checkout. Public
artifacts can use HTTPS; for this private repo, use the user's authenticated
[GitHub CLI release download](https://cli.github.com/manual/gh_release_download)
or scoped API credentials through existing auth, never tokens in URLs/logs or copied
into Waywr state. Private release reads need repository Contents read permission.
[GitHub release API](https://docs.github.com/en/rest/releases/releases?apiVersion=latest).

Retain `~/.local/share/wayward-wrangler/releases/ID`, `current`, `bin/waywr` and
existing `~/.local/bin/waywr` aliases; show Fish PATH setup once. An update installs
alongside the old release and may atomically select the new default for new projects.
It **never silently upgrades an active project's service/binding**. Offer a named
project upgrade through existing quiescent drain/settlement/compatibility checks;
wrap that existing maintenance path so routine users need no manual systemctl step.
Cancelled/failed downloads or staging leave the old default usable. Preserve unknown
staging content; resume only a matching verified install intent. Default rollback
selects a retained bundle; project rollback still checks schema/contract/state
boundaries and never restores an old database over new work. Retain referenced releases.

Checksums detect corruption against trusted release metadata; they are not an
independent signature if both assets are replaced. Add provenance verification where
available, without pretending this private repository has it: GitHub attestations
for private repositories require Enterprise Cloud. Keep authenticated GitHub as the
explicit initial trust root; stronger signing can be configured without changing
visibility. [GitHub attestation requirements](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).

## Connected delivery order and focused verification

Implement in this order on the preserved source, committing recoverable checkpoints
and proceeding through them without approval stops:

1. **Existing-project baseline/context and governing policy**: remove literal-main
   coupling; add selected baseline/target, relevant nested instructions and scoped
   check/run recipes. Preserve existing policy; show new-project baseline, target,
   completion and human-validation choice in one approval. Test policy retention
   and both validation choices, an existing non-main repository, dirty unrelated edits,
   missing baseline, nested instructions and external-edit invalidation. Reuse
   source-retrieval/isolation tests; no new demonstration project.
2. **Candidate handoff and lifecycle**: bind the card/feedback to exact commits,
   implement checkout ownership and policy completion, then cleanup as part of
   that same lifecycle. Test linked-worktree restrictions, user edits, stale feedback,
   new commits during validation, interrupted handoff/merge/cleanup, dependent/uncertain
   work retention, Git object reachability and already-authorized automatic delivery.
   Use the existing real-Git fixtures and TUI action tests; no exhaustive matrix.
3. **Discovery, role setup and usable Ollama execution**: add catalog/status and HTTP
   profile support, then wire discussion, development decisions, review and evidence
   continuations through the existing runner/scheduler and TUI. Preserve frozen
   Codex/Claude behavior and test the actual client-version parsing change. Reuse
   adapter fixtures with a local HTTP stub for streamed structured results, exact
   review/retrieval, invalid/truncated output, model mismatch and context overflow;
   cover cancel before/after send and adoption races, terminal-result recovery and
   no duplicate dispatch after interruption. Assert thinking fields are excluded
   from persisted output.
   Cover non-PATH/offline discovery without inference, unsupported role/effort rejection,
   old pins, after-turn switching and no unconfigured fallback. These focused checks
   verify transport/authority, not Qwen quality. Ordinary user-chosen work exercises
   model suitability; no demo, prerequisite trial or broad provider matrix.
4. **Distribution**: wire focused CI checks alongside the preceding changes, then
   finish release assembly/installer/update UX. Check checksum/platform mismatch,
   private download failure, interrupted install, unchanged existing project binding
   and refused unsafe rollback. Reuse Rust bundle/maintenance logic; old Python-era
   harnesses with legacy paths are evidence, not the new runtime or mandatory suite.

State additions belong to existing workflow/unit/effect/knowledge/provider records:
baseline/target/policy, candidate/feedback identity, checkout owner, cleanup intent
and archive locator, source hashes and discovery evidence; Ollama adds a transport
descriptor and HTTP dispatch/terminal/cancellation evidence to existing profiles and
attempt receipts, with separate client/server settlement. Reuse writer transactions,
command IDs, receipt/Git reconciliation, bounded snapshot/Inspect and request history.
Negotiate new IPC/result capabilities and bundle compatibility where semantics change;
do not reinterpret old approvals or delete resources under a new retention policy
retroactively. Persist compact facts and referenced evidence, not another bookkeeping
database. No mandatory duration, call or repair ceilings; retain implemented
progress-based reassessment and distinguish long work from stalled processes.

After delivery: install a supported binary, `cd` to the real project, `waywr init .`,
`waywr`; inspect/select roles, discuss the actual request, approve scope/policy once,
steer concurrent work, and receive a candidate with usable environment instructions.
Validate if required, give feedback against that candidate, and let Waywr finish
the agreed integration and cleanup. Next attach uses maintained context. The user
chooses the real work; no request is seeded and no small-project success is promoted
to a claim of general complex-project readiness.

## Recommendations and boundaries

- Retain each existing project's agreed policy. For new projects, visibly recommend
  checked local advancement after required checks, with human validation selectable
  before integration; offer PR delivery where appropriate. Confirm baseline, target
  and completion choices in the initial approval, not another universal approval gate.
- Pause governing-branch writes during user validation; independent workers may
  continue. Prefer this clear ownership rule over a moving candidate or competing refs.
- Ship the verified Linux x86_64/private-release path first. Discover all requested
  clients honestly; preserve Codex/Claude and deliver Ollama coordinator/reviewer
  execution together with role setup. Model suitability remains evidence-specific.

These recommendations resolve routine choices for implementation review. A particular
project may still need the user's baseline, product semantics, remote authority or
deployment destination; ask in that project when concrete. No repository-cleanup
exception needs a decision. Defer cross-platform hosting, Gemini/OpenCode execution,
Ollama coding workers and remote/cloud Ollama endpoints, automatic OS/auth repair,
arbitrary dirty-worktree recovery, unrelated refactors and
deployment-platform automation. Runtime changes and publication remain subsequent work.
