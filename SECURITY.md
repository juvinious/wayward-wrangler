# Security and privacy

Wayward Wrangler is experimental. Only the latest reviewed source is maintained;
older local releases are not automatically updated. See
[readiness](README.md#current-readiness) and [distribution](docs/distribution.md).

Use the repository's **Security → Report a vulnerability** action for private
reports when enabled. If it is unavailable, keep exploit details and sensitive
material private; a public issue may request that maintainers enable the private
reporting channel. Never post credentials, private project files or runtime records.

The service uses a same-UID trust boundary. Provider clients and project commands
can read authorized project data. Local `.waywr` state/history and selected project
paths are private operational data, not public distribution assets. A local user
can still view their own selected paths in the interface; no developer identity or
machine location should be embedded in shipped code or resources.

Before publishing source, inspect every public branch/tag/PR and retained Actions
log/artifact. Editing current files does not scrub old Git objects, commit authors,
PR descriptions, caches or copies held elsewhere. Keep the original private
repository private until an explicitly approved historical-content disposition.

CI checks current source, new commit content, dependency advisories and build-path
privacy. Releases carry the project MIT license and required upstream notices.
Run `python3 scripts/check-public-content.py` before sharing a source snapshot.

The privacy gate checks every changed blob in new commits, including content later
removed in the same PR. It rejects personal-home paths and non-example contact
addresses outside required upstream notices, plus credential/runtime filenames
even when no token format is recognized. In GitHub Actions it also derives
the personal repository owner's account identifier from the event payload and
rejects it in source without placing that identifier in the public configuration.
GitHub ownership and normal contribution metadata remain public platform metadata;
the source and distribution policy does not make an account anonymous.

Main requires a pull request with passing build, dependency/privacy/secret checks
and CodeQL. The sole maintainer approves by choosing to squash-merge after checks
pass; no second-person approval is required. Force pushes and deletion of main
are blocked, and merged topic branches are deleted automatically. Never weaken
those gates to resolve a failing check; repair the failure on the PR branch.
