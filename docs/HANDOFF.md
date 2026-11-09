# Waywr handoff — local iteration ready for user verification

Work remains on `plan/usability-and-installation`. Follow
[AGENTS.md](../AGENTS.md), the [accepted approach](development-approach.md) and
[usability target](usability-plan.md). Use one observable slice at a time: establish
its behavioral red result before application changes, implement minimally, verify
the relevant outcome/fences, then assess actual use. No broad test campaign or
additional orchestration architecture is prescribed.

Git remains read-only for routine inspection, with staging owned changes and
local commits allowed. The user reviews and performs all remote delivery. Do not
push, modify PRs, merge remote work, publish releases or change hosted settings.
Earlier remote permissions are superseded. Preserve unrelated edits/history.

Implemented local iteration behavior:

- Compatible stopped projects open with the selected paired application, without
  needing the old build or rewriting legacy project release metadata.
- An idle old service hands over automatically after exact unit argv/PID/socket
  ownership and quiescence checks under startup/execution fences. It preserves selected caller environment
  and XDG locations rather than stale manager HOME/PATH. A live TUI,
  pending delivery or uncertain launch preserves the old owner and reports why.
- Native versions are diagnostic. New attempts freeze the current resolved CLI
  separately from provider/model/role approval; normal updates, symlink moves and
  removed historical default locators need no F3 save. Prepared attempts retain
  exact executable checks. Required interface/tool/sandbox/result checks remain.
  Legacy native packets without identity require a new attempt; fixture hashes
  remain strict. A frozen-client semantic marker guards incompatible downgrade.
- `sh scripts/dev-install.sh` builds/checks/selects a fresh paired local artifact
  with one command. Failed builds preserve the selected installation and project
  work. It changes no shell/provider settings and starts no service. `--prefix`
  isolates an install; `waywr --version` reports version/build without project setup.

Red evidence covered missing old-build routing, a still-running old service that
never settled, provider version/hash gates, missing frozen-client compatibility
checks, unavailable dev-install/version entry points, and a readiness check that
still tested a removed locator after resolving its replacement. Final cases also
proved known user-local discovery and caller HOME/PATH preservation in the actual
service rather than a stale manager environment. The corresponding
focused regressions pass. Source-build checks include notice/privacy validation.
Verification includes real stopped-project and systemd handover/TUI checks,
two real local builds/installs plus a compiler-failure control, installed CLI help
probes, focused provider/permission/semantic/CLI regressions, and real archive
install/startup outside the source tree with Rust absent and zero source accesses.
Source/binary privacy, notice inventory, shell syntax, workflow YAML and reference
checks pass. Ordinary native task delivery and subjective fluidity remain for the
user to verify; no hosted CI run was triggered.

Run [the verification guide](verify-usability.md): install from this checkout,
complete a useful local task, close/pause, rebuild and reopen the same project.
The agent has not installed into the operator environment, changed the preserved
project or called a model. Prior cleanup removed 40 operator installations;
source and project work/history remain. All acceptance installations/services
belong to temporary fixtures and must be cleaned up.

Boundaries: idle replacement is supported; busy/uncertain execution is not
force-stopped or replayed. Real CLI help checks establish interface metadata,
not authentication, sandbox usability or model reliability. Public curl download,
`waywr update`/`uninstall`, artifact collection and Debian packaging are later
slices after ordinary-use feedback. No hosted binary release or paid feature was
enabled. Historical repositories remain private/read-only.

Contracts: [distribution](distribution.md), [providers](provider-selection.md),
[layout](project-layout.md), [IPC](contracts/session-protocol.md),
[workflow](ordinary-project-workflow.md). The [core-slice checkpoint](archive/handoff/2026-11-09-usability-core-slices.md)
retains intermediate evidence. Next action: user review and verification; turn a
concrete observed interruption into the next failing regression before fixing it.
