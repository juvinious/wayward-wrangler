# Waywr handoff — clean repository baseline

This standalone source snapshot intentionally excludes the original private Git
and GitHub history. The original repository and exact runtime evidence remain
private. There are no inherited private branches, tags, PRs, logs or artifacts.
The public repository's checked-out HEAD is the new source baseline.

The source incorporates security-fixed dependencies, MIT licensing, required
upstream notices, generalized source/docs, explicit installer repository selection,
path-remapped builds and privacy/secret/advisory/CodeQL workflows. See the
[audit and verification record](../../security-audit-2026-11-08.md), [security policy](../../../SECURITY.md)
and [distribution contract](../../distribution.md) for exact coverage and limits.

The user authorized renaming/archiving the original privately and reusing the
original repository name for this independent source history. Ongoing work belongs
to this repository. The historical repository is read-only and receives no future
source, workflow or dependency updates. Its local remote is retired from active use.

The replacement's secret scanning/push protection, Dependabot, private reporting,
read-only workflow tokens and pinned-action policy are configured independently.
Main uses required PR/build/security checks, squash-only merge and automatic topic
branch deletion. The sole maintainer decides when to merge; no second reviewer is
required. Check actual hosted run results and branch rules before merging work;
this document is not a substitute for those gates. No release or operator upgrade
is implied by this source migration.

The accepted Rust/Ratatui application is IPC1.10/state9. Native reliability,
ordinary-project usability and portability remain experimental. Contracts:
[workflow](../../ordinary-project-workflow.md), [providers](../../provider-selection.md),
[recovery](../../coordinator-recovery.md), [context](../../context-delivery.md).
Historical public reports are generalized context, not fresh trial authority.
