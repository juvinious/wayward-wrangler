# Releasing Waywr

Releases are published by pushing a reviewed version tag. A successful workflow
builds the Linux bundle, checks installation, runs security audits and creates a
GitHub Release with that version's changelog notes. No calendar release job runs.

## Cadence and versions

Release on demand when a useful increment is ready. Use a preview for changes to
installation, startup, persistence or agent execution before offering them through
latest-stable installation. Ship urgent security fixes as soon as they are ready.

- `v0.4.0-rc.1`: a preview, available through an explicit installer version.
- `v0.4.0`: a normal release, offered by the latest-stable installer.
- Increment patch for fixes and minor for features. While the application is 0.x,
  use a minor bump for incompatible changes and name them in the changelog.

These are examples, not published releases. Non-preview 0.x releases remain
experimental; the release channel does not change the application's readiness.
Versions follow [Semantic Versioning](https://semver.org/).

## First-release readiness

Before the first release, complete a significant real Waywr journey: proposal,
human approval, coordinator decomposition, delegated implementation, exact review,
combined checks, browser inspection and accepted local delivery. The
[Workboard brief](../examples/release-acceptance-brief.md) defines the proposed app.
Working model replies or passing simulated transports do not establish this outcome.
Record the delivered app revision and any manual repair; an unfinished or broken
journey is not release readiness. Expected human approvals and normal review/rework
are part of the process. CI does not run paid provider calls on behalf of this check.

## Prepare the release commit

1. Keep a few user-visible bullets under `[Unreleased]` in
   [CHANGELOG.md](../CHANGELOG.md) during development. Avoid commit lists and test diaries.
2. Set `frontend/Cargo.toml` package.version to the release version, including any
   `-rc.1` suffix. Update its workspace Cargo.lock entry without upgrading dependencies.
3. Move the release bullets to `## [VERSION] - YYYY-MM-DD`, using the intended
   release date. Leave a fresh `[Unreleased]` section at the top.
4. Review and merge that commit through the normal PR checks. Wait for build,
   security and CodeQL checks on main to pass before pushing a release tag.

The tag must match the application version and its dated changelog section
exactly. The pipeline rejects an unmerged commit or missing/mismatched entry.
The metadata helper needs Python 3.11+ on a maintainer's machine; CI provides it.
Binary users do not need Python.

## Push the tag

From the reviewed release commit on main, the maintainer runs:

```bash
release_version=0.4.0 # replace with the version prepared above
python3 scripts/release-notes.py --tag "v$release_version" --output /tmp/waywr-release-notes.md
git tag -a "v$release_version" -m "Waywr v$release_version"
git push origin "refs/tags/v$release_version"
```

Push only the intended tag. Do not move a published tag or replace its assets;
release a new version for corrections. Use the same procedure for a preview,
with the application/changelog/tag all set to a version such as 0.4.0-rc.1.

## What the pipeline does

The [release workflow](../.github/workflows/release.yml) responds to
[version-tag pushes](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#push):

1. **Validate:** require the tagged commit to be contained in main, match the
   package version and extract its exact dated changelog entry.
2. **Build and audit:** pin both jobs to that validated commit. Build/test the
   Linux x86_64 GNU bundle and run its real isolated installation check. Run the
   existing secret/privacy and Rust/Python/npm advisory checks through the reusable
   security workflow. All must pass before publishing.
3. **Publish:** verify the tag still points to the validated commit, then attach
   `waywr-VERSION-linux-x86_64-gnu.tar.gz` and `SHA256SUMS` to its GitHub Release.
   The release body comes from CHANGELOG.md; no generated commit dump is added.

A preview is marked prerelease and never latest. A normal release is marked
latest. These use GitHub CLI's [release options](https://cli.github.com/manual/gh_release_create).
Only the publish job has repository contents-write permission, using GitHub's
workflow token; no personal access token is required.

A failed validation, build or audit publishes nothing. The builder needs working
DNS/package-mirror access. After success, confirm the assets and notes on the
release page and try the [public installer](distribution.md#install-a-public-binary-release)
from a clean user environment. A local package check does not establish a live
published download. The initial binary target is Linux x86_64/glibc 2.39+; other
platforms and package-manager channels are separate work.
