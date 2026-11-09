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
requirements still apply. Bundles now include a manifest-hashed shared JavaScript
model runtime and its legal notices. Node >=22.19 and util-linux `flock` are binary
runtime prerequisites; npm, Python and Rust are source-build tools only. Existing
profiles need schema3 reconfiguration; retained project work is not deleted.
Schema2 profiles no longer stop service startup: the project opens with execution
blocked and F3 available for explicit replacement and coordinator switching.

For local development, run `sh scripts/dev-install.sh` from the source checkout.
It wraps the existing locked remapped build, notice/privacy checks, paired bundle
assembly and staged installation with a fresh generated ID. It selects the new
application atomically without starting a service, changing shell/provider settings
or binding each project. Failed compilation/checks leave the previous installation
selected. `--prefix ABSOLUTE` isolates an installation; Rust/Python are source-build
tools, not installed runtime dependencies. `waywr --version` reports the frontend
version and selected artifact; the unbundled diagnostic needs no project/manifest.
Older artifacts are currently retained; automatic collection/uninstall is later
work. They are not permanent project dependencies.

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
and `~/.local/bin/waywr`. Selecting the default preserves project metadata.
Normal startup now opens compatible stopped projects with the selected application;
it does not route through an old build named in `.waywr/release.json`. That retained
record supplies project identity/diagnostics, not an executable dependency. The
installed bundle's hashes, storage schema and retained semantic contracts remain
checked. An idle running service can be replaced under exact owner/quiescence checks;
attached views, pending delivery and uncertain launches preserve its owner.
This also works for a manifest-validated paired bundle launched directly through
a temporary/project-local symlink; an installed releases directory is not
required. Handover verifies the old unit's real executable (including a deleted
old file), exact project/state/key/release/IPC/runner/frontend command, socket
peer PID and bounded project status reply, then applies the existing quiescence fence. It
does not locate the old service by guessing a sibling installation directory.
Using status instead of the full snapshot also permits upgrading an older
service whose large project projection exceeds its former1MiB response ceiling.
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
manifest-hashed files, with explicit MIT/notice metadata. A new
declaration missing either text is refused; older manifests without notice
metadata retain their existing compatibility. The notice inventory covers189
normal/build Rust dependencies for Linux x86_64 GNU. The shared runtime separately
includes npm dependency attributions and license texts under
`resources/model-runtime/THIRD_PARTY_NOTICES.md`. Its lockfile and SDK versions are
recorded in source; production npm audit, Dependabot and JavaScript CodeQL coverage
are prepared in the local workflows. Upstream names and copyright
notices are required attribution, not operator-specific application content.

After dependency changes, run `python3 scripts/generate-notices.py`, review the
license inventory, and include it with source and bundles. No public release or
operator upgrade occurred during security remediation; see the
[audit/remediation record](security-audit-2026-11-08.md).

## Reopening after a compatible replacement

`python3 scripts/test-project-reopen.py` uses the real bundle builder/installer,
launcher, service, IPC and TUI in a temporary HOME. It composes an unsent draft,
saves a pause, selects another bundle identity, deletes the former installation,
and reopens normally without editing the project binding. Draft/settings/legacy
metadata and the intentional pause survive; invalid project identity and unsupported
storage are refused without rewriting their metadata. No model request is sent.
Services are started directly by this fixture, so it does not establish systemd
bootstrap, live service handover, different-version protocol compatibility or a
complete ordinary development workflow. Both bundles use the supplied paired
binaries; the regression isolates application-build identity changes.

The `--systemd-handover` variant uses the real user manager and normal launcher to
start and replace only its own uniquely keyed unit. It removes the old installation
while the service runs, checks refusal for a live TUI, pending input and uncertain
launch evidence, then verifies a new process/generation restores the saved draft.
This establishes idle handover, not interruption or resumption of active model work.
Adding `--portable` uses a temporary local-bin symlink directly to the paired
bundles without installing them. That real-systemd variant reproduces the local
development path and checks the same busy/uncertain-owner refusals and idle
replacement after removal of the old bundle.
`--populated-history` exercises complete snapshot delivery above1MiB using inert
retained fixture records. With `--systemd-handover --portable`, an explicit
`--previous-binaries ABS_BIN` tests an older service that demonstrably closes its
oversized snapshot, followed by safe idle replacement and successful reopening.

`python3 scripts/test-dev-install.py` runs two actual builds/installations with an
explicit temporary prefix (including spaces) and the permanent Rust toolchain. It
removes the former build and reinitializes the same project without changing its
saved settings. A fixture compiler failure then proves the selected working
installation and project data remain intact. No service/model/operator install is
involved. See [user verification](verify-usability.md) for ordinary use, which is
separate from these automated checks.

Service bootstrap forwards the existing allowlisted runtime variables and XDG
locations from the calling terminal. This preserves normal CLI discovery and
configuration locations instead of relying on stale user-manager HOME/PATH.
It does not import arbitrary caller token variables or edit shared configuration.
The real-systemd regression checks the service's actual HOME/PATH environment.
