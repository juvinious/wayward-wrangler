# Waywr handoff — acceptance-driven usability work

Work on `plan/usability-and-installation`. The user accepted the
[usability target](../../usability-plan.md) and [development approach](../../development-approach.md).
[AGENTS.md](../../../AGENTS.md) now requires user-visible vertical slices, an observed
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
fences remain enforced. The first slice did not include live service handover or provider changes;
the second slice's idle handover is recorded below.

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
[prior checkpoint](2026-11-09-usability-planning-and-cleanup.md)
retains cleanup and repository-security context.

The source builds and starts its TUI/service. Exact project build routing is
removed for compatible stopped-project opening; cross-build attachment to a busy service remain implemented limitations.
Public release/download usability and reliable ordinary work are not established.
The private historical repository remains read-only; no paid features were enabled.

Second slice: real-systemd idle handover is implemented. Its initial red result
was a timeout waiting for the old build's active service to settle. The launcher
now proves the prior unit process/argv and IPC peer, then reuses the existing
quiescent stop under startup/execution fences. UI claim registration shares that
fence. The real-systemd regression passes idle replacement with the old files
removed and preserves the owner for live views, pending input and uncertain launch
evidence. It uses only a temporary project, HOME and uniquely keyed unit; cleanup
removes the unit and all fixture files. No model or operator operation is involved.

Third slice: provider version and permanent native hash gates are removed. New
reservations freeze `execution.client` separately from approved profile choices;
old packets keep exact identity checks. CLI updates/symlink moves need no F3 save.
Missing help options report the actual interface deficiency. Legacy fixture hashes
remain strict; historical replay constants are diagnostic only. New native legacy
conversation packets freeze before reservation; old native packets missing an
identity refuse execution and require a new attempt. A persisted semantic marker
and bundle capability prevent guarded downgrade that would ignore frozen clients.

Observed red cases covered version rejection, stale saved fingerprints, legacy
gates and missing downgrade guards. Metadata fixture regressions, installed CLI
help-interface probes, all targeted provider policy/switch/fallback checks, exact
builtin-capability validation and release semantic guards pass. None called a
model. The remapped release build and source/binary privacy checks also passed.

Next authorized action: implement/test one-command local build/install and version
reporting. Then hand the candidate and commands to the user for ordinary-workflow
verification. Do not install into the operator environment or call a model. Public
curl/update/uninstall and package channels remain subsequent slices. Local commits
only; remote delivery belongs to the user.
