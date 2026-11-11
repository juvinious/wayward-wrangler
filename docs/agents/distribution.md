# Distribution implementation notes

Audience: agents implementing installation or lifecycle changes. Read only for
that task. The human-facing contract is [installation and distribution](../distribution.md).
Current authority and next action come from [AGENTS](../../AGENTS.md) and
[HANDOFF](../HANDOFF.md), not this evidence inventory.

## Source owners and current gaps

- `frontend/src/rust_bundle.rs`: paired assembly, manifest/hash validation,
  staging and atomic default selection. `bundle-build` requires a fresh output
  directory; an existing release ID cannot be reused with different contents.
- `scripts/dev-install.sh`: remapped locked build, notices/privacy checks, fresh
  local ID and staged installation. Failed builds retain the selected version.
  Its printed PATH hint still favors Fish; make installer guidance Bash-first
  while implementing this slice, with other shells as optional notes.
- `scripts/install.sh`: public curl HTTPS transport, latest stable/exact version,
  metadata parsing through the existing Node runtime, checksum-asset digest/archive
  hash and safe extraction before existing bundle-install. No gh/auth fallback;
  `--private` explicitly opts into gh. Node/Git/flock are checked before download.
  The success message gives Bash PATH guidance and no per-project upgrade command.
- `.github/workflows/ci.yml` builds a PR bundle. `release.yml` packages version
  tags and publishes through a separate contents-write job. PR artifacts are
  not public release assets. The tag-release path remains unverified.
- CLI update/uninstall, saved update repository metadata, artifact collection,
  first published release and private Node provisioning remain missing. These
  are separate from the completed public installer fix. Public trust uses HTTPS
  to the selected GitHub repository and its asset digest; no independent signing/
  attestation root was added. Public curl ignores user curlrc/auth configuration.


## Build and attribution

Use `sh scripts/build-release.sh` for distributable binaries: it preserves Cargo
flags and remaps checkout, home and toolchain paths. Packaging uses permanent
Rust1.99.0/Cargo.lock, digest-pinned Ubuntu24.04 and full-SHA Actions pins.
`scripts/package-release.sh VERSION ABS_OUTPUT` runs the build and checks,
assembles the paired bundle, then normalizes GNU tar/gzip and writes SHA256SUMS.
The manifest records source/dirty state, locks/toolchain, libc and builder identity.
Normalized assembly does not establish bit-identical independent builds; apt
revisions and hosted infrastructure can change.

Root LICENSE and THIRD_PARTY_NOTICES.md are manifest-hashed with explicit MIT/
notice metadata. New declarations missing either are refused; older manifests
without that metadata retain their compatibility. Rust notices cover189 Linux
normal/build dependencies. Runtime npm attributions/license texts ship separately
in resources/model-runtime/THIRD_PARTY_NOTICES.md. Regenerate Rust notices after
dependency changes with `python3 scripts/generate-notices.py`, review and check.
Python/npm/Rust are build tools, not installed runtime requirements; Node and
flock remain runtime requirements. Scan source, all three remapped binaries and
the complete bundle with `scripts/check-public-content.py`.

## Existing lifecycle mechanism

Normal startup uses the selected paired application, not the executable named in
project release metadata. Compatible busy owners remain attachable. Idle handover
verifies the old unit's actual executable (including deleted files), exact project/
state/key/release/IPC/runner/frontend command, socket peer PID and bounded status,
then applies the quiescence fence. It does not guess an old installation directory
or need the old full snapshot (which can exceed its former1MiB limit). This works
through an installed launcher or a direct temporary paired-bundle symlink.

Obsolete/stranded state follows [startup renewal](../project-layout.md). Preserve
source/Git/worktrees, exact ownership and no replay. The explicit session-upgrade
maintenance path is not the ordinary update journey. Service bootstrap forwards
allowlisted runtime variables and XDG locations from the terminal rather than
relying on stale user-manager HOME/PATH; arbitrary token variables are not imported.

## Reusable focused checks and their limits

Read each harness before running it. Some variants use the real user manager;
none of these examples supplies permission to change operator state or call an
account. Use the relevant existing example instead of repeating all campaigns.

| Harness | What it establishes | Boundary |
| --- | --- | --- |
| `scripts/test-packaged-install.py ARCHIVE` | Real archive, shell installer, staged install, hashes, damage refusal and installed CLI/service/TUI startup outside checkout without Rust | Only HTTPS transport is mocked; isolated HOME; direct service, no systemd bootstrap or account |
| `scripts/test-project-reopen.py` | Draft/pause/settings survive selecting a second bundle and deleting the first; invalid identity/storage refused | Same paired binaries under different IDs; direct services, not arbitrary version compatibility |
| `test-project-reopen.py --systemd-handover` | Exact idle service replacement and retained draft; existing owner kept during pending delivery/uncertain execution | Real uniquely keyed fixture user unit; not active-model resumption |
| Above with `--portable` | Same lifecycle through a temporary local-bin symlink after removal of old bundle | Temporary paired layout; no global installation |
| Above with `--populated-history` and optional `--previous-binaries ABS_BIN` | Full snapshot above1MiB and safe replacement of an older service that closes it | Inert retained fixture records, not model delivery |
| `scripts/test-session-renewal.py` | Old unresolved-attempt refusal, exact inert owner stop, obsolete state retirement and fresh reopen | Real isolated systemd/IPC/TUI; held sleep, no account |
| Above with `--busy` | Same compatible active owner across builds, followed by idle handover | Owned fixture process, not model usefulness |
| Above with `--controls` | SDK/local HTTP interruption, prompt leave while SIGSTOP-held settlement, busy reattachment, pause and disconnected leave | One local fixture request; no account; exact fixture units stopped |
| `scripts/test-dev-install.py` | Two real isolated-prefix builds, former build removal, same-project reopen and failed-compiler preservation | Permanent toolchain; no operator install, service or model |

PR packaging previously passed the archive/install/TUI and artifact-upload path;
see the [dated closeout report](../reports/ordinary-project-verification.md).
That result does not establish current hosted checks or public release availability.
Security fixes and the later mirror/DNS failure are in the
[security report](../reports/security-audit-2026-11-08.md#hosted-security-follow-up--2026-11-11).

## Public installation evidence and next review

The [public installer report](../reports/public-installer-2026-11-11.md) records
behavioral red, final checks and archive/manifest hashes. The package harness now
exercises piped shell input, default latest-stable and explicit version, then
corrupt/missing download refusal with the working application still selected.
Its helper blocks gh in public mode and fixtures only HTTPS transport; all paired
executables are real. Runtime tracing reports no source-checkout access.

The user deferred hosted build/DNS, update/uninstall and publication/packaging
for subsequent review. No remote rerun, global install or account call is implied.
The remaining dev-install Fish-only PATH hint can be standardized when its output
is next changed; documentation already uses Bash by default.

API reference: [GitHub release metadata](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)
and [release asset digest/downloads](https://docs.github.com/en/rest/releases/assets#get-a-release-asset).
Public endpoints allow unauthenticated access. Metadata URLs and asset download
URLs are built from validated repo/tag values; returned URLs are not evaluated.
