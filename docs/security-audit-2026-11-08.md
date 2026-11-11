# Public-source audit and remediation

The initial audit inspected clean source `532e999f007a1b462594fb7c57b8fcd603aae506`
on November 8, 2026. Subsequent authorized remediation is on
`security/public-source-readiness`. The original repository remains private.
No confirmed authentication secret was found. Personal/operational disclosures
were present and have been generalized in the current public source copies.
Old Git/GitHub copies remain a separate publication boundary.

## Initial audit coverage

The audit covered 408 tracked paths; all seven local refs (81 reachable commits);
all local objects, including 1,938 blobs, 229 commits and two annotated tags; a
fresh remote mirror including ten retained PR heads; all ten PR descriptions;
five retained Actions logs; and four build artifacts. All remote non-tree objects
were included in the local complete-object scan. Comments/reviews returned no
additional records; releases, wiki and Pages were absent.

Gitleaks 8.30.1 scans included complete files, patches, commit/tag text, decoding
and binary printable strings. Two unique matches were project identities rather
than credentials. Supplemental checks covered private-key headers, credential
URLs, assignments, home paths and contact identities. No confirmed credential,
tracked runtime database or raw transcript was found. No credential rotation or
Git object deletion was warranted. Scanner detection cannot classify every
historical sentence or unknown credential format.

## Implemented remediation

| Finding | Current disposition |
| --- | --- |
| Tokio I/O safety, RUSTSEC-2026-0386 | Both exact pins and production lock updated from1.53.2 to1.53.3 |
| Unmaintained D-Bus dependencies | zbus5.19.0 migrated to6.2.1 with explicit Tokio/proxy features; native `Variant`/`OwnedObjectPath` imports replace deprecated aliases; the three unmaintained crates leave the production lock |
| Python CA trust advisory, GHSA-6q48-6c7w-9gw2 / PYSEC-2026-4320 | certifi2026.10.19 replaces2026.7.22 |
| Python namespace advisory, GHSA-6hmw-9432-67qq | langgraph-checkpoint-sqlite3.1.2 and required checkpoint4.3.0 replace3.1.1/4.2.0 |
| Personal locations and identities | Current source/config/docs contain no detected personal-home paths or known operator/account/private-project identifiers; opaque historical runtime IDs/session IDs are redacted; edited archives are labelled generalized copies |
| Embedded release destination | Installer now requires `--repo OWNER/REPO` or `WAYWR_REPOSITORY`; examples use an explicit destination; no personal account is compiled or configured as the default |
| Builder paths | `scripts/build-release.sh` remaps checkout, home and toolchain prefixes. Packaging checks binaries and the complete bundle for builder/personal paths |
| Licensing | User selected MIT; root license and Rust/Python package declarations added. Bundles include license and generated notices for189 normal/build Rust dependencies on supported Linux x86_64 GNU |
| Repeated detection | Pinned security workflow checks current source/new commit content with Gitleaks, strict Rust advisories and OSV Python identities. Privacy gate, negative-control tests, pinned CodeQL Rust/Python/Actions workflow and weekly Dependabot configuration added |

Required upstream copyright/contact notices are retained. Public archives are
historical context after generalization, not replacements for exact private
runtime evidence. Exact originals were preserved privately before editing;
original Git history and managed-project runtime state were not rewritten.
The older standalone frontend lock is not the production workspace lock.
Historical native profile examples use nonpersonal absolute-path placeholders.
The archived parity helper now requires explicit fixture run/project identities;
no retained private fixture was replayed during remediation.

## GitHub controls

| Control | Verified state after authorized changes |
| --- | --- |
| Dependabot alerts | Enabled; authenticated GET succeeded |
| Dependabot security updates | Enabled, not paused |
| Secret scanning / push protection | Canonical repository PATCH refused with422: feature unavailable for this repository; no billing/visibility change attempted |
| Private vulnerability reporting | Enable request returned404; not enabled/verified |
| Code scanning | API returned403, not enabled. Pinned workflow prepared locally; no hosted result is claimed before an authorized push and feature availability |
| Branch protection/rulesets | Prior read returned403 with a private-plan restriction; no new rules or paid feature purchase attempted |
| Token defaults | Read-only; Actions cannot approve PRs. Checked-in actions use exact commit pins and checkout does not persist credentials |

For a future public destination, enable secret scanning with push protection and
private reporting, run the checked-in scanning workflows, then require passing
checks/PR review on main and block force pushes/deletion. Confirm the exact status
check names from real runs before creating required-check rules. These controls
are recommended configuration, not a claim they are already active everywhere.

## Verification

- Strict production Rust audit reports zero advisories and zero warnings with
  cargo-audit0.22.2 and RustSec database
  `7a40350e1b8ba5b656d909f88222a8059a83d88d`; no suppressions were added.
- OSV checked all41 Python package identities and returned zero affected packages.
  The revised lock installs in an isolated environment; `pip check` passes.
- Locked Rust1.99.0 release build passes. All three binaries and the complete
  bundle pass builder/home-path checks, including a private identifier deny list
  held outside the public source. No new compiler or global installation was used.
- 146 Rust tests pass:111 frontend library, one bundle compatibility,13 ordinary
  backend,16 service IPC and five mock HTTP lifecycle tests. One existing frontend
  timing acceptance test remains ignored. Real systemd host acceptance was not run.
  A stale frontend assertion expecting main-only delivery was corrected to verify
  the existing named-target behavior; production event wording did not change.
- 28 credential-free legacy Python compatibility tests, four offline installer
  tests, two privacy-gate negative-control tests, IPC schema checks, actionlint1.7.12
  and shell syntax pass. Socket fixtures used the necessary local socket access;
  no authenticated model or operator service was invoked.
- The real15-file manifest bundle passed isolated shell installation, paired
  CLI/service/TUI startup and detach, all hashes, damaged-resource refusal and
  missing-license refusal. Rust was absent from runtime PATH; strace observed zero
  source-checkout accesses. Download transport alone was mocked.

The validation bundle was built from an uncommitted working tree and is not a
published release. Its archive SHA256 is
`0f0bf7755227bd1a0150e873d3f35d33aefb867985f9e2934b3212e59cb7df1f`;
manifest SHA256 is
`8cf854747b4d57cfd5d30d52475f97a6d73e4a75fa9f3d5ed94d0249bf465218`.
No authenticated model, operator upgrade, public push, release, destructive history
rewrite or shared infrastructure change occurred. Experimental workflow, native
reliability and portability limits remain in [README](../README.md#current-readiness).
The [Linux setup](linux-sandbox-setup.md) still records the sandbox/preflight gap.

## Publication boundary

A clean source snapshot with neutral contributor attribution avoids carrying old
Git authors, commit text, PR heads or Actions records into a new public repository.
Keep the original private repository and evidence private. Merely editing main or
force-pushing a replacement branch does not sanitize retained PRs, logs or other
copies. [GitHub documents that Actions history/logs become public when visibility
changes](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/setting-repository-visibility).
Publication destination and any original-history/PR disposal require a separate
explicit decision. The current source cleanup does not authorize either action.

The subsequent user decision authorized a new independent repository under the
original name and a private, read-only historical archive. No old branches, tags,
PRs or Actions evidence belong in the replacement. The new repository's protections
are configured separately; the table above records the earlier private-repository
state. Main requires PR/build/security checks with squash merge and topic-branch
deletion; maintainer approval is the decision to merge after passing checks.
The expanded privacy gate additionally checks new-commit blobs, contact addresses
and the account identifier supplied by GitHub's event metadata. Required upstream
attribution remains intact. See [SECURITY.md](../SECURITY.md) for the current policy.

Raw reports and original copies remain in private temporary storage outside Git.
No machine-specific audit storage location or real operator identifier is needed
in the public source. See [SECURITY.md](../SECURITY.md) for reporting and ongoing
privacy boundaries; [distribution](distribution.md) owns packaging requirements.

## Local iteration follow-up

The local usability work's history scan flagged the sequential synthetic project
ID in `backend/tests/provider_updates.rs` as a generic API key. That value is a
metadata-only fixture identifier and provides no service access. Current fixtures
use an obvious repeated-character ID. `.gitleaksignore` records only the exact
immutable historical fingerprint from `fc5bffd`; no rule, file or path wildcard
is excluded. The pinned scanner remains active for current source and introduced
history. Local history scanning passes with that documented false positive.

## Local branch review — 2026-11-11

The operator reports the current application behavior looks good. The pre-push
review at application checkpoint cb094be checks 459 current source paths and all
changed blobs/paths in 28 commits since the locally recorded origin/main. The
privacy gate reports zero issues for these sources/history, the three release
binaries and the paired 19-file bundle. Checksum-verified Gitleaks 8.30.1 reports
zero source/history findings, retaining only the exact previously documented
synthetic-project-ID fingerprint exclusion. No runtime credential store or
operator-state file was found in the scanned source snapshot.

Commit author/committer metadata was checked separately: 21 application commits
use public GitHub username/noreply attribution; seven use neutral example
attribution. No private email, machine address or personal-home path was found
there. Public GitHub authorship is repository attribution, separate from the
application's neutral paths/configuration. No authors or Git history were rewritten.
This checkpoint adds documentation only after those application checks.

This was local review, without fetching, pushing or querying hosted settings.
Latest hosted Rust/npm/Python advisories, secret scanning, build and CodeQL results
remain PR gates before merge; earlier dated advisory results are not presented as
fresh checks. The historical private archive remains a separate publication
boundary. Public installer/update/uninstall and broader product use are subsequent
work, not additional local publication blockers identified in this review.
