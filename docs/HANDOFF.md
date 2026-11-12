# Waywr handoff

Branch: `plan/usability-and-installation`; task base `b8a98a7`.
Current checkpoint: README main-workflow badges and local fixes for the failing
PR13 startup and PR11 SHA-2 compilation checks. The user owns all remote delivery;
no push, merge or release. Read [AGENTS](../AGENTS.md), then task-relevant notes.

## Implemented and checked

[CI review](reports/ci-review-2026-11-12.md) records the three PRs and evidence.
Chunked IPC replies pass real packaged install/startup/detach with delayed reads.
Hash encoding preserves values and compiles with current and proposed PR11
dependencies; this branch's pins/locks are unchanged. Badges retain privacy gates
outside canonical public README workflow URLs. PR12 was already green.

## Next action and limits

User reviews/pushes the local changes and updates PR11 with the compatibility fix;
hosted CI must rerun. Later hosted steps and public download remain unverified.
No operator environment or provider credentials were changed; no model calls.

Then return to the significant proposal → coordinator → workers → working app
journey before release: [Workboard brief](../examples/release-acceptance-brief.md)
and [acceptance notes](agents/release-acceptance.md). Native-run permission and
role/model choices remain pending; human approval must cover the actual scope.
Do not substitute fixture replies or direct app implementation for that journey.

Tag automation/changelog and public installer are implemented; their
[release](reports/release-pipeline-2026-11-11.md) and
[installer](reports/public-installer-2026-11-11.md) reports retain evidence.
CLI update/uninstall and first publication remain separate work.
