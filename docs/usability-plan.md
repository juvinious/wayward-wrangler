# Usability and installation plan

Status: accepted implementation target on `plan/usability-and-installation`.
Follow the [acceptance-driven development approach](development-approach.md).
Local implementation, isolated checks and local commits are authorized. The user
handles all remote delivery. Operator installation, preserved-project changes and
authenticated model calls remain outside the current authorization.

## Outcome

A user installs Waywr once, opens any supported project with `waywr`, gives it a
real development request, reviews the result and can return later. Updating Waywr
must not require upgrading each project or reinstalling the build that created it.

Keep the existing Rust/Ratatui frontend, Rust service/runner, project identity,
scope approval, exact commit review, durable receipts and execution fences. Fix
the lifecycle around them before adding more orchestration features.

The first deliverable is a short daily development loop: **use Waywr → edit its
source → rebuild/install with one command → reopen the same project → continue**.
The user must be able to work with the application while it improves. Public
release automation and additional packaging must not delay this loop. If this
small increment cannot support useful work without repeated manual recovery,
pause feature expansion and reassess whether to continue the project.

## Current state and cleanup

Public main at `085cc6c` builds and produces a privacy-checked paired bundle. Its
installed command opened the real TUI and started its real systemd user service
in an isolated project. That check sent no model request and proves startup only.

At the user's request, all 40 user-local release directories, the owned launcher,
application location registry, empty runtime directory and identified Waywr
temporary artifacts were removed. No Waywr service/process was running at cleanup;
the command is absent from PATH afterward. Source repositories, project data,
worktrees/history, provider installations/credentials and shared infrastructure
were preserved. Build output inside source checkouts is not an installation.

No new installation, project binding change, authenticated model call, PR merge
or release publication is part of this planning checkpoint. Existing project
release records are retained data; the proposed reader must handle them without
requiring the old executable.

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

Next slice: reopen with an idle service still owned by the previous application
build, using the same acceptance-first cycle. Active/uncertain jobs keep their
existing fences; provider and installer changes remain later slices.

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
services. Mac/Windows, additional provider integrations and broad autonomous
reliability are outside this increment. A narrow supported platform is preferable
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

1. **Daily-use vertical slice.** Remove exact project build/client version locks
   for the current supported state/IPC contracts, handle legacy build records
   without needing their executable, add `--version` and the one-command local
   build/install wrapper, and preserve safe service ownership. Verify reopening
   with the former build removed and refusal to replay an uncertain operation.
   With explicit user authorization after planning, install the candidate and use
   the current provider CLI for one useful real task in a chosen project. Fix an
   observed application issue, rebuild once and continue in that same project.
   This complete loop is the first milestone; ship no additional orchestration
   features before it works. Keep older unsupported state inspectable and give a
   concrete recovery route; do not promise arbitrary forward/backward compatibility.
2. **Install/update/uninstall.** Add public/latest installation and CLI lifecycle
   commands using the existing bundle. Check checksum failure leaves the current
   installation intact, two projects survive an update/removal of the old build,
   and uninstall preserves project work. Produce the release assets and concrete
   publication instructions for review. No installation during the planning task.
3. **First public release and package.** Keep using the application during its
   development; retain observed defects as the work queue. After the daily loop
   and clean public install work, publish the user-approved release. Build the
   Debian package from that same artifact/lifecycle. Prioritize interruptions and
   lost-time defects over expanding providers, abstractions or acceptance suites.

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
