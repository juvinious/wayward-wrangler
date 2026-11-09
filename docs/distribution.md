# Binary distribution and guarded updates

`ci.yml` checks pull requests/main and builds a bundle; `release.yml` assembles a
version tag and publishes its assets through a separate contents-write job.
Actions are full-SHA pinned. The builder uses digest-pinned Ubuntu24.04, permanent
rustup1.99.0 and Cargo.lock. `scripts/package-release.sh VERSION ABS_OUTPUT` runs
the locked production build with machine-path remapping and existing `bundle-build`, then produces normalized
GNU tar/gzip plus SHA256SUMS. The manifest records source/dirty state, lock/toolchain
hashes, rustc, libc and builder identity. Normalized assembly is not a claim of
bit-identical binaries; apt package revisions and hosted infrastructure can change.
Two independent identical builds have not been demonstrated.

Initial published target: Linux x86_64 GNU, Ubuntu24.04/glibc2.39. This is not a broad
Linux portability claim. Systemd user execution, sandbox support and provider
requirements still apply. Python and Rust are not binary-install runtime requirements.

`scripts/install.sh --version vMAJOR.MINOR.PATCH` uses authenticated `gh` for a private
repository. It verifies exact tag, GitHub's checksum-asset digest, archive SHA256,
platform/libc, safe names, unique entries and regular-file/directory types before
extraction. It rejects path traversal, links and special files, then delegates to
existing staged/hash-validated `bundle-install`. Tokens stay with gh. Missing release
assets/digests refuse; there is no source-build fallback. `--repo OWNER/REPO`,
`--prefix ABSOLUTE` and `--no-select-default` are explicit options. Select `--repo OWNER/REPO` or set `WAYWR_REPOSITORY`; there is no embedded account
or repository destination. See [README](../README.md#versioned-binary-installation)
for Fish commands. Authenticated download trusts the selected repository/GitHub;
there is no separate signing or independent release-attestation trust root yet.

Layout remains `~/.local/share/wayward-wrangler/releases/ID`, `current`, `bin/waywr`
and `~/.local/bin/waywr`. Selecting the default never changes existing project bindings.
Explicit `waywr --project DIR session upgrade --release ID` checks closed UI/no pending
input and positively settled operations, stops only that quiescent project service
under execution fencing, then performs the existing guarded binding migration. PR delivery reconciles the exact
merged PR before checking a potentially auto-deleted publication branch; completion
and its effect/history records commit atomically.
Active/uncertain execution refuses; there is no forced stop, inferred settlement or
implicit resume. Intentional pauses and all approval/history/Git records remain.
A normal launch starts the selected service through Waywr; users need no systemctl
procedure. Legacy layout migration remains a separate supported operation.

`scripts/test-packaged-install.py ARCHIVE` uses the real versioned production
archive, shell installer, staged bundle installer and paired installed binaries in
an isolated HOME/XDG prefix. Only GitHub's download transport is a local fixture.
It checks manifest hashes, launcher resolution, damaged-resource refusal and idle
service/TUI startup outside the source tree with Rust absent from PATH. Its Python
runtime is a developer/CI dependency, not an installed application dependency. PR
and release package jobs run this check; no model or systemd bootstrap is involved.
Local verification checks code, shell/workflow structure and isolated installation
mechanisms. The PR/build workflow has executed successfully,
including the real archive/install/TUI check and artifact upload. Git trusts only the
owned `$GITHUB_WORKSPACE` in these ephemeral containers, not a wildcard path.
The version-tag release workflow has not executed.
Publishing requires an authorized push/merge, Actions enablement and a version tag;
the release job must succeed and its archive/checksum assets must exist before the
GitHub installer can work. Nothing has been published by this implementation task.

## Public build privacy and notices

Use `sh scripts/build-release.sh` for distributable binaries. It preserves Cargo
flags and remaps checkout, home and toolchain paths; ordinary source compilation
needs no Python. Packaging uses Python developer checks, fetches locked notice
sources, verifies `THIRD_PARTY_NOTICES.md` and scans the entire bundle for builder
paths. These tools are not installed application prerequisites.

New bundles include root `LICENSE` and `THIRD_PARTY_NOTICES.md` as two additional
manifest-hashed files (15 entries total), with explicit MIT/notice metadata. A new
declaration missing either text is refused; older manifests without notice
metadata retain their existing compatibility. The notice inventory covers189
normal/build dependencies for Linux x86_64 GNU. Upstream names and copyright
notices are required attribution, not operator-specific application content.

After dependency changes, run `python3 scripts/generate-notices.py`, review the
license inventory, and include it with source and bundles. No public release or
operator upgrade occurred during security remediation; see the
[audit/remediation record](security-audit-2026-11-08.md).
