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

The current slice is reopening an idle compatible project after replacing the
application and removing its old build, without manual rebinding. First reproduce
that failure through the real launcher/TUI in isolation, preserving project state,
settings and history. Provider changes, installers, active-job handover and broad
storage migration are excluded from this slice. Record red/green evidence and
what the user can actually do; the full daily-use loop remains incomplete.

The earlier cleanup removed all 40 user-local releases, launchers, registry and
identified temporary artifacts. Source and project work/history were preserved.
Do not reinstall into the operator environment, change the preserved project or
make authenticated model calls under this task. Isolated acceptance artifacts
must be temporary, self-contained and cleaned up. The
[prior checkpoint](archive/handoff/2026-11-09-usability-planning-and-cleanup.md)
retains cleanup and repository-security context.

Baseline source builds and starts its TUI/service; exact project build routing
and provider version allowlists remain implemented limitations at this checkpoint.
Public release/download usability and reliable ordinary work are not established.
The private historical repository remains read-only; no paid features were enabled.

Next authorized action: observe the current slice's acceptance failure before
changing application behavior, then implement only the smallest complete fix and
reconcile the affected startup/distribution contracts. Local commits only.
