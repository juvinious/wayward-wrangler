# Waywr handoff — acceptance-driven usability work

Work on `plan/usability-and-installation`. The user accepted the
[usability target](usability-plan.md) and [development approach](development-approach.md).
[AGENTS.md](../AGENTS.md) now requires user-visible vertical slices, an observed
behavioral red test before application changes, minimal green implementation,
focused checks and explicit ordinary-use limits. Do not expand into another
architecture or broad testing campaign.

Git boundary: read-only routine inspection, staging owned changes and local
commits. The user reviews and performs all remote delivery. Do not push, modify
PRs, merge remote work, publish releases or change hosted settings; earlier remote
permissions are superseded. Preserve unrelated edits and history.

The first slice now opens compatible stopped projects after the old application
installation is removed, without routing to it or rebinding project metadata.
Startup validates the installed bundle, project identity, storage range and
retained semantic markers. Repeated init and offline status use the current app;
legacy release metadata stays unchanged. IPC release/owner and native execution
fences remain enforced. Live service handover and provider version gates are not
fixed by this slice.

Acceptance evidence: `python3 scripts/test-project-reopen.py` initially failed with
`No such file or directory` before the replacement TUI could open. After the fix,
the same scenario opens the real TUI, restores its unsent draft, preserves settings
and intentional pause, and requires no manual rebind. Negative controls reject
foreign project metadata and unsupported storage without rewriting it. It uses
two bundle identities of the supplied paired binaries and directly started
isolated services; no systemd bootstrap or model call is involved. It cleans its
temporary HOME/project/installations. The locked remapped release build passed.
Both existing recovery/convergence release-contract guard tests passed. Source
and remapped-binary privacy checks, changed workflow YAML parsing, documentation
references and diff checks passed. The new acceptance check is wired into local
workflow source for future CI/release runs; no hosted run was triggered.

The earlier cleanup removed all 40 user-local releases, launchers, registry and
identified temporary artifacts. Source and project work/history were preserved.
Do not reinstall into the operator environment, change the preserved project or
make authenticated model calls under this task. Isolated acceptance artifacts
must be temporary, self-contained and cleaned up. The
[prior checkpoint](archive/handoff/2026-11-09-usability-planning-and-cleanup.md)
retains cleanup and repository-security context.

The source builds and starts its TUI/service. Exact project build routing is
removed for compatible stopped-project opening; provider version allowlists and
cross-build running-service attachment remain implemented limitations.
Public release/download usability and reliable ordinary work are not established.
The private historical repository remains read-only; no paid features were enabled.

Next authorized action: define and observe the regression for opening with a
still-running idle service from the previous build, then implement that bounded
handover slice. Do not force-stop/replay active or uncertain work. Provider changes
and the one-command local installer follow separately. Local commits only; no
operator installation, preserved-project modification or authenticated model calls.
