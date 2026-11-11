# Tag release pipeline checks — 2026-11-11

Scope: finish the existing tag-push workflow and use a terse changelog as its
release notes. No operator installation, account/model call, tag creation, remote
write, hosted run, paid setting or publication occurred. The known container
package-mirror/DNS failure is unchanged and remains a hosted-build blocker.

## Outcome

Version-tag pushes now validate reviewed main ancestry, exact frontend version
and a dated changelog entry. Build and reusable security checks receive the same
validated source commit. Publish waits for validation/audit/package jobs, checks
the tag's current commit again and reads the extracted changelog notes. Only the
publish job has contents-write permission. Previews are prereleases and explicitly
not latest; normal releases select latest. Existing SHA-pinned actions, builder,
locked compiler/dependencies and production package/install checks remain.

CHANGELOG.md starts with Unreleased changes only; no published history was
invented. Release preparation must add the matching dated entry before a tag can
pass. Agent maintenance policy and the human release guide describe this split.

## Local evidence

Before implementation, execution of the existing publish shell command against
fixture gh proved it omitted reviewed notes and classified preview tags as normal
releases. A later regression observed that a moved tag still reached publication;
the commit check now refuses it before creation.

Eight focused tests now pass, including stable/preview notes, moved-tag refusal,
version/date/duplicate-entry failures and actual main/off-main ancestry in isolated
Git repositories. The validator/publisher commands are extracted from release.yml;
GitHub publication is a fixture; one unauthenticated GET of an upstream public
tag verified the namespaced tag-to-commit API route. No Waywr tag was created. The existing bootstrap regression
passes after selecting the named step in the expanded workflow.

Checksum-verified actionlint1.7.12 passes ci/release/security workflows; published
linter archive SHA256 is
`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`.
Workflow/Bash syntax, current changelog, local references, privacy and diff checks
pass. Gitleaks found no leaks in changed/new source.
Existing package/application evidence is reused because no Rust application,
archive assembly or installer behavior changed in this slice.

## Limits and next action

This verifies configuration and metadata behavior locally, not execution by hosted
Actions or a published CDN download. No GitHub tag/asset immutability setting was
changed; the workflow rechecks identity before publishing and does not clobber
existing releases. Do not retag published work. Resolve hosted builder DNS before
expecting a successful tag build. Update/uninstall and package channels remain
separate work. See [release guide](../releases.md),
[agent notes](../agents/releases.md) and [handoff](../HANDOFF.md).
