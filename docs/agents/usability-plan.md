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

## Completed local slice: public installation

- **Outcome:** public HTTPS downloads need neither gh nor login, select the latest
  stable release by default and accept an explicit version. A real isolated
  archive installs through the stable launcher and opens the CLI/service/TUI.
- **Original failure:** exact-version installation called authenticated gh;
  installation without a version was rejected before download. Both behavioral
  failures were observed before changing the installer.
- **Acceptance:** actual archive/install/CLI components in isolated HOME pass;
  only HTTPS transport is a fixture. Corrupt/missing downloads preserve an
  existing selected release. Unit checks retain digest/archive validation and
  explicit private opt-in. Node >=22.19, Git and flock are checked before download.
- **Limits:** no public release download has occurred; Node remains external.
  No operator installation, account call, remote change or publication occurred.

See the [installer report](../reports/public-installer-2026-11-11.md) for exact
scope/artifacts. The user deferred the next items for review: hosted builder DNS,
CLI update/uninstall, then first release/publication and Debian packaging. Do not
silently include these in the completed authentication fix. Update-source metadata
and artifact collection belong with update/uninstall. No global/private Node
provisioning or cross-platform expansion was added.

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
