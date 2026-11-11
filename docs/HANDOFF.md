# Waywr handoff — security fixes and distribution next

Branch: `plan/usability-and-installation`; parent checkpoint911c53a, PR13.
The user pushed the branch and explicitly deferred merge until installation/
distribution is addressed. Follow [AGENTS.md](../AGENTS.md). Local commits only;
the user owns all remote writes. This task queried GitHub read-only. No operator
project/service/binding/provider configuration/credentials, global installation,
model/account call or hosted setting was changed.

## Implemented outcome

The hosted strict Rust audit failed on libsqlite3-sys0.38.2 (RUSTSEC-2026-0459)
and rustix1.1.5 (RUSTSEC-2026-0486). Both were reproduced locally. Targeted patches
now select rusqlite0.41.1/libsqlite3-sys0.39.1 (SQLite3.54.1) and rustix1.1.6;
the legacy frontend lock also has patched rustix. No other crate version changed,
no advisory was suppressed, and toolchain1.99.0 remains. Notices are regenerated.

The separate Rust PR build failed before checkout because Ubuntu package-mirror
DNS was unavailable. CI/release bootstrap now uses bounded retries and fails
accurately on incomplete package indexes. An isolated apt-stub regression observes
the original transient failure, then verifies recovery and bounded permanent
failure against the actual workflow commands. Persistent hosted DNS failure
still requires infrastructure recovery; it is not established fixed locally.

## Evidence and limits

Strict cargo-audit0.22.2 passes both lockfiles against fresh database
fd6f39814c4d3c359fdbe223d0280dec4933f78f. Python OSV (41 locked identities) and
production npm audit are clean. The locked remapped three-binary release build,
22 SQLite persistence tests, three wheel regressions and actual PTY mouse/
terminal-restoration check pass. Public-content, notices, documentation-reference
and diff checks cover this checkpoint. These are local checks, not new hosted runs.

Hosted CodeQL succeeded for all four languages on pushed revision911c53a;
read-only queries returned no open alerts for inspected refs. The next user push
must establish passing hosted build/security checks for these dependency fixes.
See [audit record](security-audit-2026-11-08.md#hosted-security-follow-up--2026-11-11)
for runs, advisory sources and scope. The earlier branch privacy review remains
recorded there; no history was rewritten.

The ignored target/local-bin/waywr still points to the earlier tested cb094be
temporary paired candidate; candidate.json records its identity. The patched
release binaries are in target/release and have not replaced an operator service
or global installation. Do not describe the old candidate as the patched build.

## Product state and next authorized action

The user reports responsive use, working provider sign-in/model requests and
mouse scrolling. Broad shared-model execution, renewable obsolete sessions,
compatible busy-owner attachment, direct interrupt/leave/pause, live steering,
durable editable follow-ups and editor/model/thinking/status controls are
implemented. Verified event resync handles busy/disconnected reads without
replaying commands. Useful coordinated application delivery remains unproven;
do not restart a broad control/testing campaign before addressing distribution.

Pi and Codex now both document shell installers; Pi exposes `pi update`, while
Codex documents rerunning its installer. The comparison and proposed Waywr
journey are in [distribution](distribution.md#next-distribution-slice--proposed-after-installer-comparison).
Our current GitHub installer is private/authenticated and requires an exact
version. Public latest-stable installation, CLI update/uninstall, managed runtime
provisioning and additional package channels remain unimplemented.

Next product slice: public unauthenticated Linux installation through the existing
verified paired bundle, with a stable command, explicit version/prefix overrides
and failed-download integrity. Start with its behavioral failure in an isolated
HOME, then exercise real archive/install/CLI components. Only transport may be a
local fixture; do not invoke operator installation or publish. Resolve Node runtime
handling and public artifact trust explicitly. Follow with update/uninstall using
the same bundle/lifecycle, then additional package channels. Preserve project
source/Git/worktrees and actual execution ownership; no retired call/approval
replay. Keep catalogs/options outside coordinator routing.

Owners: [plan](usability-plan.md), [distribution](distribution.md),
[controls](tui-controls.md), [live input](live-input.md),
[providers](provider-selection.md), [user walkthrough](verify-usability.md).
