# Waywr handoff — usability planning and installation cleanup

The current task is planning, not another installation or testing campaign. The
user rejected per-project application build locks and native-client version
allowlists, requested removal of lingering installations/temp artifacts, and wants
a practical install/update/uninstall and ordinary-project experience. Review the
[usability plan](usability-plan.md) before implementation. No application code has
changed in this checkpoint, and no project binding was upgraded.

The first proposed milestone is daily use during development: use the app for a
real task, edit its source, rebuild/select with one command, reopen the same
project and continue. Public packaging follows that loop. The user will reassess
continued investment if normal use keeps requiring manual recovery; additional
orchestration features and broad test campaigns are not the immediate objective.

Public main `085cc6c` built and packaged successfully with privacy checks. Its
installed TUI and real systemd user service started in an isolated project, with
no model request. On the user's subsequent cleanup instruction, all 40 user-local
release directories, the launcher, location registry, empty runtime directory and
identified Waywr temporary artifacts were removed. Fresh PATH checks find no
`waywr`, `wayward-wrangler` or `awp`; no Waywr service/process was running at cleanup.
Source checkouts and project work/history remain. Source-tree build output is
not an installation. Temporary verification records were disposed of as requested.

Existing project release records still name their old build. They are preserved
legacy data, not a reason to reinstall that build. The proposed lifecycle must
open retained projects with the installed app, negotiate storage/IPC capabilities,
and preserve inspection while unsafe execution awaits reconciliation. Provider
versions should be diagnostic; execution identity/capability fences remain.
These changes are proposed, not implemented. Do not run another installer or
change the preserved project's binding as part of this planning checkpoint.

The independent public repository retains MIT licensing, generalized source/docs,
patched dependencies and privacy/security checks. The original repository stays
private, archived and read-only. Hosted main requires PR/build/security checks
and squash merge; no second reviewer is required. No paid security feature or
subscription was enabled. The [baseline handoff](archive/handoff/2026-11-09-clean-repository-baseline.md)
and [audit record](security-audit-2026-11-08.md) retain migration/security context.
No public binary release was published by this task.

The implemented Rust/Ratatui application remains IPC1.10/state9. Native reliability
and full ordinary-project usability remain experimental. Current contracts:
[workflow](ordinary-project-workflow.md), [distribution](distribution.md),
[providers](provider-selection.md), [recovery](coordinator-recovery.md) and
[context](context-delivery.md). Next action: user review of the proposed use cases,
lifecycle strategy and delivery order; then implementation within that agreed scope.
