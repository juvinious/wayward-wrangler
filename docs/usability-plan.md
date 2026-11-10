# Usability and installation plan

Status: accepted implementation target on `plan/usability-and-installation`.
Follow the [acceptance-driven development approach](development-approach.md).
Local implementation, isolated checks and local commits are authorized. The user
handles all remote delivery. Operator installation, source/worktree disposal and
authenticated model calls remain outside the current authorization. Discarding
obsolete Waywr configuration/session context is authorized; implementation and
isolated checks precede the deferred operator walkthrough.

## Outcome

A user installs Waywr once, opens any supported project with `waywr`, gives it a
real development request, reviews the result and can return later. Updating Waywr
must not require upgrading each project or reinstalling the build that created it.

## Reconciled checkpoint — 2026-11-10

Source checkpoint: session-renewal work after `4c804cd`. The mechanisms below are implemented locally;
their isolated evidence does not establish a fluid, useful daily workflow.

| Plan area | Delivered | Still needed |
| --- | --- | --- |
| Shared model layer | Standalone pi-ai/agent-core runtime replaces the native integrations; catalogs/configuration and manual IDs stay outside core routing; two protocol/tool/runner fixtures | Real account setup and useful delivery through selected models; catalog breadth is not verified account access |
| Selection and visibility | Full searchable provider/model pickers, runtime thinking levels, saved/active labels, direct setup from Send, worker activity and conversation-call usage/cost/context estimates | Simpler ordinary model changes and actionable authentication/errors; workflow coordinator usage is not yet in the conversation summary |
| Local iteration | Paired build/local launcher, compatible reopen, idle/portable handover, compatible busy-owner attachment, automatic renewal of obsolete/stranded sessions without old target/call reuse | Complete start → leave → return → rebuild → continue journey with useful model work; existing services predating cross-build UI attachment require their first idle transition |
| Input and responsiveness | Contextual arrows/Tab/Enter/Escape, literal paste and history, Ctrl-C/Escape clearing, draft persistence, reduced idle copying/polling | Direct interruption, real steering versus follow-up, consistent normal editing and prompt leave controls |
| Isolation of blockers | Unrelated policies do not veto new discussion; stranded old sessions stop being live update dependencies | Errors and settlement waits must remain scoped while navigation, drafting, inspection and leaving stay available |
| Distribution | Bundle validation, source/binary privacy checks, notices, local staged selection and version diagnostics | Public unauthenticated installer, update/uninstall and artifact collection; publication remains the user's action; Debian packaging follows |
| Substantial application | Direction and proposed reference brief | Shared-runtime real task delivery, then a useful persisted web-app increment across selected models |

The shared execution foundation is substantially implemented. Application fluidity
is the current work, not a completed milestone. The reference application has not
been delivered through this runtime. Operator state has not been changed by this
renewal implementation; the operator walkthrough and account calls stay deferred.

Startup outcome: obsolete configuration/bindings, unsupported session formats or
unresolved host uncertainty create a fresh session from current source and usable
model settings. The old session is abandoned after stopping its exact owned
processes; its calls, approvals and target configuration leave live state. Project
source, Git work and dirty worktrees remain. Real isolated launcher/systemd/IPC/TUI
checks cover renewal and healthy busy-owner attachment; no model was called.
The [project layout](project-layout.md) owns the journal/retirement contract.

## Finish before the next operator test

The user asked to finish the interaction/lifecycle changes before asking them to
open the application again. Follow the [planned everyday interaction](tui-controls.md#planned-everyday-interaction)
in this order. These are targets, not claims of current behavior.

1. **Interrupt and leave directly.** Expose a visible interrupt action and normal
   shortcut without a confirmation maze. Stop the foreground coordinator, not the
   entire workflow. Detach/exit returns the terminal promptly while service-owned
   work or settlement continues separately. Acceptance: start a held streaming
   fixture, interrupt, continue drafting, leave while stopping, then reopen to the
   exact saved status without duplicated work. Include a disconnected UI. Do not
   wait for agent settlement just to restore the terminal.
2. **Steer or follow up while busy.** Enter sends guidance to the active agent at
   its next supported boundary; Alt+Enter explicitly queues later work. Show which
   disposition was accepted and allow queued messages to be edited/withdrawn.
   Connect the existing SDK methods through the service/runner/runtime boundary;
   changing a label on the current next-turn queue is not steering. Acceptance:
   a held two-step tool fixture receives steering before continuing its next step,
   while a follow-up waits until the active task finishes. Drafts/queued input
   survive interruption without silent replay or scope expansion.
3. **Return to busy work after rebuilding.** The compatible UI/owner attachment
   mechanism is implemented and isolated checks pass. Complete it in the useful
   interrupted/redirected-work walkthrough. Open a compatible current UI against
   the existing owner, or provide an immediate usable view through its retained
   pair. Replace only after settlement. Acceptance: A starts work, the launcher
   selects B, reopening can inspect/control A without force-stopping it; idle
   reopening selects B with retained conversation/work. Unsupported compatibility
   must expose usable inspection and an action, not a generic connection failure.
4. **Finish everyday defaults and recovery.** Align editor history/newlines,
   command discovery, model/status shortcuts, error recovery and truthful usage
   with the same action contract. Authentication/setup failures retain the message
   and offer the relevant action. A request may wait; the application must remain
   usable. Acceptance: ordinary editing and one missing-credential/request-policy
   failure through isolated real UI/service components leave the draft and normal
   navigation intact. Use one focused example per changed boundary.

After1–4, do one operator walkthrough: a useful bounded task, interrupt/redirect,
leave/return, rebuild and continue. Real model use belongs to that authorized
walkthrough, not a prerequisite inferred from fixture checks. This replaces
repeated requests to reopen the application after each small mechanism fix.

Public packaging remains a deliverable after this usable local loop. Do not add
another provider framework, reintroduce native model allowlists, run account calls,
publish, or delete source/worktrees as part of these interaction changes. Old
Waywr configuration/session disposal is authorized through startup renewal. Real account use
still needs its corresponding authorization; fixture evidence cannot supply it.

## Narrowed product direction

The user has set this order: extensible model/execution support, application
fluidity, then coordination refined through a substantial web application.
Waywr owns project coordination and the user experience. Its core should not own
a closed catalog of provider/model names. Pi's application is a comparison. Its
standalone provider/agent libraries are the selected shared runtime dependencies.

The target is one shared multi-provider model/execution layer beneath the existing
Rust TUI/service. The [corrected direction](execution-adapter-decision.md) uses Pi's
registry/protocol/catalog separation as a reference. The standalone pi-ai/agent-core
libraries now implement the shared boundary; Pi's terminal app is not used.
A process adapter can separate dependencies from Rust, but wrapping one native
client is insufficient. The user waived compatibility with existing hardcoded
integrations/profiles: replace them without parallel legacy-route maintenance.
Preserve project work/history and execution ownership. Do not build a general
plugin SDK, marketplace or another agent loop as an incidental workstream.

### 1. Extensible execution and model selection

- Model identifiers are data supplied by the adapter, discovered metadata or an
  explicit user entry. An incomplete catalog must not prevent selecting a new
  model. No Rust change or Waywr release should be required for a model supported
  by an already installed adapter. Provider-specific defaults and option catalogs
  belong with that adapter/configuration, rather than coordinator logic.
- An adapter reports supported operations and options, streams text/tool/activity
  events, identifies the requested and observed model when available, reports
  usage, and exposes cancellation and session behavior. Unknown metadata remains
  unknown. Native effort tokens are not mapped onto an invented common scale.
- Core routing checks the capabilities needed by a task. A missing capability
  gives a specific reason; a provider name alone must not decide role eligibility.
  Supporting an interface does not establish model quality or reliable planning.
- Waywr retains scope, project isolation, execution ownership, exact result/review
  evidence and durable state. Adapter configuration does not grant project or
  cross-provider data access. Credentials remain owned by the execution utility.
  Endpoint/account/model changes must not silently redirect an approved task.
- Live and retained views should expose the selected executor/model, current
  activity, tool progress, token usage, cost and context remaining where reported.
  Distinguish measured data from estimates and unavailable values. Context
  headroom is not subscription/account quota; avoid invented remaining balances.

**Correction before application changes:** inspect the provider/model library
separately from a full harness. Pi source composes provider authentication/catalogs
with shared protocol implementations, normalized events and refreshable metadata.
Use the selected standalone provider/agent-loop libraries, with catalogs and
protocol configuration outside the coordinator. The [direction](execution-adapter-decision.md) records source
evidence and the first slice. The native-first recommendation is superseded.

Then establish behavioral red for the ordinary save/select/dispatch journey with
provider/model/option data absent from core. Deliver it through at least two
provider identities using the shared runtime, including a tool-capable worker,
visible activity/usage and saved selection on reopen. Fixtures prove their stated
protocol boundaries only; real inference remains separately authorized. Existing
profiles need not remain runnable. Retain their work/history and do not replay
uncertain attempts. Neither a catalog nor a single native wrapper completes this
milestone. A universal plugin framework is excluded.

### 2. Fluid application lifecycle

The target journey is **start work → detach → reattach → update Waywr → continue**.
Detach leaves intentionally running work under the project service; pause stops
new dispatch according to its documented boundary; cancel reports actual
settlement. Closing a terminal must have a predictable documented effect.

Reattachment restores conversation, active tasks, queued input and decisions.
Updating selects the application for new launches without project rebinding or
discarding work. Busy work can retain its service/artifact until settled; its
status must remain inspectable and attachable. Update must not force-stop or
replay it. Artifact removal follows positive ownership/settlement evidence.
Do not promise arbitrary storage compatibility or native-client session resume.

Work through one observed interruption at a time. Public install/update/uninstall
remain deliverables, using the existing bundle. Additional packaging follows a
working lifecycle rather than delaying it. Existing reopening, idle-handover and
local-install checks are reusable evidence, not proof of active-work continuity.

### 3. Coordination through a real application

Proposed reference workload: a project/work management web application with
authentication and roles, persistent projects/tasks/comments, search/filtering,
an API, migrations, a background export job and a documented local run path.
This is a suggested target, not an authorized model run or a fixed product brief.
Agree the brief before execution; it must describe user behavior and deliverables,
without prescribing a canned sequence of coordinator decisions.

Grow it in useful increments: first one complete persisted UI/API journey, then
parallel work across real dependencies, review and integration, a changed
requirement, and detach/update/recovery during work. The application must run and
be inspectable after each increment. Record manual rescues, lost/duplicated work,
time to a useful result and review quality; do not substitute test counts.

Apply the same bounded work brief through at least two selected model profiles
without changing coordinator code. Include distinct execution adapters when
claiming adapter independence. Observe quality differences rather than promising
that all models coordinate equally well. Prefer model-independent fixes to
observed coordinator failures over special cases for a model or the reference app.
This is ordinary product use, not a new broad acceptance matrix.

### Scope and authority

Developer rules govern changes to Waywr; they must not become arbitrary runtime
agent restrictions. Review a restriction against a concrete project, data,
execution or external-effect boundary. Preserve justified safeguards and remove
redundant interaction. Existing authority limits remain: local work/commits only,
no operator installation, preserved-project changes, authenticated model calls or
remote delivery without the corresponding explicit task authorization.

Keep the existing Rust/Ratatui frontend, Rust service/runner, project identity,
scope approval, exact commit review, durable receipts and execution fences. Follow
the narrowed order above before adding more orchestration features.

The lifecycle phase must deliver a short daily development loop: **use Waywr → edit its
source → rebuild/install with one command → reopen the same project → continue**.
The user must be able to work with the application while it improves. Public
release automation and additional packaging must not delay this loop. If this
small increment cannot support useful work without repeated manual recovery,
pause feature expansion and reassess whether to continue the project.

## Earlier distribution checkpoint and cleanup

Public main at `085cc6c` builds and produces a privacy-checked paired bundle. Its
installed command opened the real TUI and started its real systemd user service
in an isolated project. That check sent no model request and proves startup only.

At the user's request, all 40 user-local release directories, the owned launcher,
application location registry, empty runtime directory and identified Waywr
temporary artifacts were removed. No Waywr service/process was running at cleanup;
the command is absent from PATH afterward. Source repositories, project data,
worktrees/history, provider installations/credentials and shared infrastructure
were preserved. Build output inside source checkouts is not an installation.

Implementation uses isolated acceptance installations only. The agent has not
reinstalled into the operator environment, changed the preserved project, called
a model or performed remote delivery. Existing project release records are
retained data; the current reader opens compatible state without their old binary.
Ordinary-use verification remains outstanding; the narrowed execution direction
now precedes further lifecycle implementation.

## First slice — reopen after replacement (implemented)

- **Outcome:** an idle project with compatible current storage opens through the
  installed `waywr` after its previous application build has been removed.
- **Present failure:** startup routes through `.waywr/release.json` to the former
  binary and refuses when that installation is missing.
- **Acceptance:** initialize/save a project using installation A; replace it with
  B and remove A; open through the ordinary launcher without rebinding. Assert
  the actual TUI opens and retained project state, settings and history survive.
- **Exclusions:** provider changes, developer/public installers, package channels,
  arbitrary old/future storage migrations and operator-project upgrades. Active
  or uncertain execution is a separate slice and must retain its existing fences.

The acceptance check failed against the initial code with a missing-file error
before the composer opened after the former installation was deleted. After the
minimal fix, `python3 scripts/test-project-reopen.py` opens the real TUI and restores
the saved draft, settings and pause without rebinding. Invalid project metadata
and unsupported storage are refused. Both distinct bundle identities use the
supplied paired binaries; services are started directly inside the fixture. This
does not establish systemd handover, model-driven work or the full daily-use loop.

## Second slice — idle service replacement (implemented)

The real-systemd acceptance check initially failed because the previous build's
service never matched the new launcher's release identity. Normal opening now
verifies the exact prior unit process, argv and socket peer, then uses the existing
quiescence/receipt checks under startup/execution fences to stop only that idle
owner and start the selected application. New UI claims share the execution fence.
No old installation files or project rebind are required. A live TUI, pending
client input or uncertain launch evidence refuses replacement and preserves the
existing owner. Selected calling-terminal HOME/PATH/XDG/config locations are
forwarded without arbitrary token variables; the real service environment is
checked. Storage and execution records retain their identity.

Run `python3 scripts/test-project-reopen.py --systemd-handover` on a host with a
systemd user manager. It creates and removes its own unique temporary unit,
installation and project. CI's existing direct-service variant remains portable;
the real-systemd variant is a local host check. No model call is made.

## Third slice — provider updates (historical native implementation)

The native routes below were subsequently removed by the shared-runtime change.
This is prior evidence, not current native integration support. The continuing
contract is that new attempts resolve current runtime artifacts while prepared
attempts retain their frozen identity; the [provider contract](provider-selection.md)
owns its shared-runtime implementation.

The metadata-only regression first reproduced version allowlists, changed-binary
F3 requirements and legacy version/hash gates. New attempts now freeze a resolved
client separately from saved provider/model/role approval; ordinary CLI updates
and stable symlink changes need no preference edit. Prepared attempts retain their
old executable identity. Required help options, role/tool grants and runtime
sandbox/structured-event checks remain enforced. The frozen-client semantic marker
also prevents guarded downgrade to an application that ignores those identities.

Provider lifecycle, policy/switch/fallback and exact builtin-capability regressions
pass. The installed Codex and Claude CLIs also passed the explicit metadata-only
interface check; no authentication or inference was performed. This does not
establish a real development task.

## Fourth slice — one-command local iteration (implemented)

The initial red check found no `dev-install.sh` entry point and no `--version`
option. `sh scripts/dev-install.sh` now builds/checks/installs a paired local
application and selects a fresh generated artifact ID. Failed compilation leaves
the existing installation selected. It changes no shell/provider configuration,
starts no service and preserves project work. `waywr --version` works both in a
paired installation and as an unbundled diagnostic without creating project state.

The real local-install regression rebuilds twice in a temporary custom prefix,
removes the former build, preserves project settings and verifies recovery from
an injected compiler failure. Rust uses its permanent toolchain; operator
installation is not changed. The user can now install and assess the complete
ordinary-use/rebuild/reopen loop using the [verification guide](verify-usability.md).
The agent has not made authenticated model calls or established useful task delivery.

The shared-runtime slice is implemented locally; its [provider contract](provider-selection.md)
records protocol/tool/runner and selection evidence. Real accounts/model quality
and ordinary unassisted delivery remain unverified. The current next work is the
interaction/lifecycle sequence above, before the next operator walkthrough.
Public curl, update/uninstall commands, artifact collection and additional
package channels are not complete. Earlier checks remain available for the
subsequent lifecycle phase; do not rerun them as a substitute for useful work.

## Required everyday use cases

| Use case | Expected experience | Completion evidence |
| --- | --- | --- |
| Iterate while using it | One command from the Waywr checkout rebuilds/selects the paired local application; ordinary `waywr` opens the user's project afterward | Complete useful work, rebuild after an actual application change, then continue in that same project with its conversation, work and settings intact |
| Install | One documented public download command; no Rust, Python, GitHub login or manual release selection | Clean user environment can run `waywr --version` and `waywr --help` |
| Open a project | `cd PROJECT; waywr`; minimal first-run setup when needed, saved work on return | Both a fresh project and an existing Git project open without choosing an application build |
| Do useful work | Discuss one request, approve its scope/policy, implement, review and run relevant checks, then inspect and accept a useful result | One bounded real development task delivered through the application, with recorded assistance and actual check results |
| Close and return | Clear pause versus background-work choices; reopen the conversation and exact saved work | Restart preserves accepted work, pending decisions and an intentional pause without duplicate execution |
| Update | `waywr update`; subsequent launches use the installed application across projects | Two projects created before an update reopen after the old installation is removed, with no per-project upgrade command |
| Uninstall | `waywr uninstall` removes owned application files; preserves project work by default | No command, service or installation remains; project files/history stay intact |

The first supported distribution remains Linux x86_64 GNU with systemd user
services. Mac/Windows and broad autonomous reliability remain outside this
increment. Additional integrations follow the adapter decision, not ad hoc
provider branches in the coordinator. A narrow supported platform is preferable
to installation instructions that imply unverified portability.

## Application and project lifecycle

Retire `.waywr/release.json` as a requirement to run one exact application build.
Project identity, storage schema and saved semantic contracts determine whether
the installed application can read the project. Build IDs remain diagnostic
metadata and identify immutable execution artifacts; they do not lock a project.

Read legacy release records through a compatibility import. Opening a project
must not exec an old frontend, require its manifest or fail because that old
installation no longer exists. Do not rewrite past approvals, packets, commits
or receipts to pretend they were produced by the current build.

Separate **opening/inspection** from **dispatch/resume**. A pending operation or
unsupported saved execution format must leave conversation/history and its
concrete recovery state accessible. It can prevent unsafe continuation without
preventing the entire project from opening. An actually unsupported storage
schema needs a clear recovery/export path rather than a missing-binary error.

Negotiate frontend/service compatibility through the IPC version and supported
capabilities. Report build identity for diagnostics, while retaining project,
service-generation and command identity checks. Never start a second owner or
replay an operation because the app was updated.

Migrate compatible storage automatically, with a recoverable journal and the
existing startup/execution fences. Do not infer that an operation settled from a
missing process. A package update can wait for an active service generation to
finish; temporary retention of its files is tied to live execution, not a
permanent project pin. Idle projects need no individual update procedure.

## Provider lifecycle

Remove native-client version allowlists and hardcoded native executable hashes,
including legacy paths that enforce them. Versions are diagnostic observations.
Discover executable clients on PATH, with an optional explicit user override;
do not encode a developer's installation location or select a historical release
directory as a permanent default.

Validate the interface Waywr actually uses: required invocation options,
structured responses, role/tool restrictions and sandbox behavior. A missing
capability produces a specific actionable error. Do not silently weaken a sandbox,
accept additional tools or substitute a provider/model.

Freeze the resolved executable identity and execution contract for each launched
attempt. An in-progress attempt must not silently resume using changed bytes.
New attempts may use a normally updated client; users should not have to edit F3
merely because their CLI updated. Keep destination/model/role approval separate
from the provider's incidental version or filesystem location.

## Distribution strategy

Provide a `sh scripts/dev-install.sh` command in the source
checkout. It wraps the existing locked build, paired bundle creation and atomic
user-local selection, chooses its own artifact ID and prints the installed
version. No hand-written bundle IDs, paths, per-project rebinds or GitHub release
are needed. Local artifacts are installed only when requested. An idle service
can move to the new application on reopen; an active attempt retains its exact
execution identity until safely settled. Failed builds leave the usable install
unchanged. This is the initial path for using Waywr during development.

Use that same supported binary bundle and lifecycle for distribution. Extend the existing
installer to support unauthenticated HTTPS downloads from a selected public
GitHub repository and select its latest stable release by default. Keep an
explicit version option for reproducibility and an optional authenticated path
for private repositories. Public installation must not require `gh auth login`.

Document a Fish-compatible curl download-and-run command, including the ability
to save and inspect the installer. Add `waywr --version`, `waywr update` and
`waywr uninstall`. Store the selected release repository in installation metadata
so updates need no repeated repository option; do not compile a personal account
or local path into the application.

Retain checksum/digest verification, safe archive extraction, paired resource
validation and atomic installation. Fail before replacing the working
installation on download, integrity or compatibility failure. Detect shell PATH
issues and give the exact next command rather than claiming success prematurely.
Uninstallation must inventory owned files and running services; project data
deletion is a separate explicit action, never the default.

The user publishes the versioned GitHub release after reviewing local commits
and satisfying the normal PR and release gates. The agent prepares the artifacts
and instructions without pushing, modifying PRs or publishing. A prepared workflow or local
archive is not an available public installer. Then add a Debian package built
from the same verified bundle and using the same state/service lifecycle. Its
upgrade/removal must pass the same two-project reopening check. Defer additional
package channels until one installation/update/uninstall route works end to end.

## Delivery order and bounded checks

1. **Shared model/execution layer.** Follow the corrected direction and deliver
   broad model selection and the multi-provider journey above. Compatibility with
   existing hardcoded profiles is not required; preserve their work/history.
   Fixtures prove their stated boundaries only; ordinary model use is separately
   authorized. The implementation uses the standalone pi-ai/pi-agent-core libraries
   behind one process boundary; Pi's terminal app is not used.
2. **Lifecycle fluidity.** Demonstrate detach/reattach and safe update while work
   exists, then complete public installation and owned-file uninstall. Fix actual
   interruptions without expanding orchestration. The four implemented slices
   above support this phase but do not complete it.
3. **Substantial application.** Agree a reference brief, deliver a running first
   increment, then exercise dependencies, concurrent work, review and recovery
   across selected model profiles. Improve coordination from observed failures.
   No model-specific coordinator fork or full-application test campaign first.

Publication remains the user's action after local review and normal gates. A
Debian package follows the same verified bundle/lifecycle when useful; it is not
a prerequisite for the three phases. No hosted release is available yet.

Choose targeted checks for changed contracts and the few real lifecycle cases
above. Reuse accepted tests; do not introduce another broad acceptance program or
declare usefulness from fixture counts. Each checkpoint must deliver an
observable user behavior and state what still requires operator intervention.

## Contract owners to reconcile during implementation

| Area | Existing owners |
| --- | --- |
| Startup and old-build routing | `frontend/src/main.rs`, `rust_runtime.rs`, `rust_cli.rs`, `rust_maintenance.rs` |
| Pairing, installation and CLI | `rust_bundle.rs`, `cli_schema.rs`, `scripts/install.sh`, release workflows |
| Service compatibility and execution ownership | `backend/src/session_service.rs`, session protocol, `host_attempt.rs`, `systemd_host.rs` |
| Provider compatibility and discovery | `provider_selection.rs`, `development.rs`, `claude_chat.rs`, `continuous_adapter.rs` |
| User-facing contracts | [Distribution](distribution.md), [providers](provider-selection.md), [project layout](project-layout.md), [ordinary workflow](ordinary-project-workflow.md), [README](../README.md) |

Those documents describe the implementation until their owning changes land.
Acceptance of this target does not make the new lifecycle available yet.
