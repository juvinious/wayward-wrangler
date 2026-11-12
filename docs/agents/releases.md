# Release implementation notes for agents

Read only when changing release automation. The human process is
[releases](../releases.md); current task authority is in AGENTS/HANDOFF.

- Keep CHANGELOG.md terse and user-facing. Accumulate bullets under Unreleased;
  release preparation creates a dated section matching frontend package.version
  and tag. Do not fabricate a published history or date the current Unreleased work.
- `scripts/release-notes.py` validates supported SemVer, unique/date-valid sections,
  source version identity and a nonempty tagged entry, then extracts only its body.
  Ordinary CI uses --check; synthetic CI bundle IDs do not need release entries.
- release.yml validates main ancestry, freezes the commit output, passes it to
  package checkout and the reusable security workflow, and uploads notes separately.
  Publish waits for every job, rechecks the tag's current commit and uses those
  notes. Preview tags use --prerelease --latest=false. Never clobber existing assets.
- security.yml accepts an optional ref for workflow_call. Ordinary events still
  audit github.sha; release callers supply the validated commit. No secrets inherit
  or new hosted feature/settings are required.
- `scripts/test-release-pipeline.py` exercises the actual publish/validate shell
  commands with fixture gh and isolated Git histories. It checks notes, previews,
  moved-tag refusal, main ancestry and metadata failures. No remote publication.
- `scripts/test-builder-prerequisites.py` selects the named bootstrap step so the
  earlier validation job cannot accidentally change which command it tests.
- Validate YAML/actions with checksum-verified actionlint and run focused scripts.
  Reuse existing package/install evidence unless packaging/application changes.
  A hosted tag workflow/public download remains unverified until the user triggers
  it. The known container DNS failure is separate; do not claim this work fixed it.

References: [GitHub reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows),
[tag push events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#push)
and [release CLI](https://cli.github.com/manual/gh_release_create).
