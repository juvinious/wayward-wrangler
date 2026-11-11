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
- `scripts/install.sh`: private/authenticated gh transport, exact version,
  tag/checksum-asset digest/archive hash checks and safe extraction. The final
  printed per-project upgrade instructions are obsolete and need correction in
  the installer slice; current normal startup does not require rebinding.
- `.github/workflows/ci.yml` builds a PR bundle. `release.yml` packages version
  tags and publishes through a separate contents-write job. PR artifacts are
  not public release assets. The tag-release path remains unverified.
- Public unauthenticated/latest-stable installation, saved update repository,
  CLI update/uninstall, artifact collection and private Node provisioning are
  missing. Define public checksum/asset trust before replacing gh transport;
  there is no independent signing/attestation trust root today.

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
| `scripts/test-packaged-install.py ARCHIVE` | Real archive, shell installer, staged install, hashes, damage refusal and installed CLI/service/TUI startup outside checkout without Rust | Only gh transport is mocked; isolated HOME; direct service, no systemd bootstrap or account |
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

## Next implementation acceptance

A clean isolated HOME without gh credentials must install the public release,
run `waywr --version` and `--help`, and resolve the stable launcher. Start with
this behavioral failure against the existing installer. Exercise a real archive
and real install components; fixture only the download transport. Include a
corrupt download that leaves an existing working version selected. Settle Node
handling and public artifact trust explicitly, then implement update/uninstall
through the same path. Additional package channels follow this complete journey.
