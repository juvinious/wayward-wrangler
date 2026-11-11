# Current implementation plan for agents

Read the [product roadmap](../roadmap.md) for outcomes and
[AGENTS](../../AGENTS.md) for policy. [HANDOFF](../HANDOFF.md) selects the current
slice. This document is the implementation sequence, not a user guide or evidence
of completed ordinary use.

## Product order

1. Broad provider/model support through the shared runtime. It is implemented;
   do not reintroduce native model allowlists or provider-specific coordinators.
2. Application fluidity and distribution. Interrupt/leave/pause, live input,
   compatible busy attachment and obsolete-session renewal are implemented.
   Complete public install, update and uninstall using the existing paired bundle.
3. Coordination through a substantial application. Use actual task delivery across
   selected models to improve it; catalog coverage and fixtures are insufficient.

## Current slice: public installation

- **Outcome:** a clean Linux user environment installs from a public release,
  runs the stable waywr command and needs no source checkout, Rust, Python or gh login.
- **Present failure:** scripts/install.sh requires an exact version and
  authenticated gh transport; no public latest-release route or hosted release
  availability is established.
- **Acceptance:** use an isolated HOME, real archive/install/CLI components and
  fixture only the download transport. Verify version/help and launcher resolution;
  corrupt downloads must leave an existing working install selected. Resolve
  Node availability and the public artifact/checksum trust model explicitly.
- **Exclusions:** operator/global installation, remote publication/settings,
  cross-platform expansion, another provider framework and package-manager channels.

[Distribution notes](distribution.md) identify existing owners, precise checks and
limits. Finish this user journey before expanding scope. Then implement update
and owned-file uninstall through the same bundle/lifecycle. Reuse existing busy
attachment/idle handover; no per-project application rebind or execution replay.
Debian packaging follows, with other channels deferred until that route works.

## Preserve while implementing

Keep source, Git history, dirty worktrees, exact execution ownership and frozen
approval/result checks. Old Waywr configuration/context can be retired through
startup renewal; it must not remain a live dependency. Compatibility with retired
native provider routes is not a migration goal. Catalogs/options stay outside core
coordination, and model capabilities do not imply equal model quality.

Use one observable acceptance failure, focused underlying TDD and proportionate
verification. A broken harness/compiler is not behavioral red. Account calls and
operator installation are not inferred from a fixture. Git writes stay local.
See [development approach](development-approach.md) and AGENTS for the full rules.

## Selective prior evidence

The [earlier implementation plan](../archive/usability/2026-11-11-implementation-plan-before-doc-split.md)
retains completed slices and their observations; do not load it by default.
[Distribution notes](distribution.md), [interaction checks](../reports/interaction-verification-2026-11-11.md)
and [security findings](../reports/security-audit-2026-11-08.md) retain focused evidence.
Use these only when relevant; old permissions/status are not current authority.
