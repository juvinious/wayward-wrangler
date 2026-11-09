> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Coordinator recovery — build and verification, 2026-10-17

Later authorized follow-up: [installation and preservation record](2026-10-17-installation.md).
The uninstalled state described below is the original build closeout.

Implementation branch `impl/coordinator-owned-recovery`, clean planning base
`d21271ff07e9b42134f0ac6336da3c128b24397c`. Recoverable source checkpoints:
`ccc8f25` typed causes/reconciliation, `6cc29fb` finding lineage/resolution,
`67b93e796e4fa3e633024b6f7a59ddcdc7e7bcc7` progress, connected presentation and compatibility.
[Current contract](../../coordinator-recovery.md) owns behavior and limits;
[approved plan](approved-plan-d21271f.md) retains the intervention evidence.

## Exact production build

Permanent rustup, rustc1.99.0 (`b940084d7`), Cargo1.99.0 (`5f94df478`), locked dependencies,
Linux x86_64. Source and Cargo.lock unchanged during build:

```fish
~/.cargo/bin/cargo build --locked --release --bin waywr --bin waywr-service --bin waywr-runner
```

| Binary under `target/release/` | SHA256 |
| --- | --- |
| `waywr` | `75df559aaf7405033d6a4363ed967d1ea91ecf1ff01e6aea6437a7f49de09c02` |
| `waywr-service` | `9f518103502cb5efc76d17505eedea38ab67659ba0b1e22230dd147e550dfb52` |
| `waywr-runner` | `88a052c6811f26270a5e8bfa1d5a9ba9f4801ad93574f3ced2a202ba01411818` |

Paired local bundle:
`~/projects/wayward-wrangler/target/waywr-bundles/recovery-20261017-67b93e7`.
Manifest SHA256 `295f0986be0df442f11f71179a03ce6a640af64fc78f766f184c56ef68a83c25`;
13 file hashes verified. It declares state3–9, conversation-v1, pinned-roles-v1,
evidence-v1, working-memory-v2 and recovery-v1. This ignored build artifact is local,
not an installation, separate evaluation environment or publication.

## Focused verification

- Backend recovery group: 27 passed before the final scoped-continuation additions.
  Final connected context/finding/progress/scoped group: 22 passed. After final
  normalization, all six recovery-module tests passed. Groups overlap; do not sum.
- Supplemental group: 13 passed, including exact reviewer-role gating, atomic
  duplicate decision adoption, Git commit-before-writer-ack reconciliation with dirty
  preservation, and positive prelaunch/stale-owner linked-attempt retirement.
- Recorded review fixture: exact CLI source, explicit advisory resolution and
  unchanged nonblocking native object; omitted, unrelated, wrong lineage/commit/
  generation/profile or unsupplied-evidence resolutions cannot clear the blocker.
  Linked clarification accepts no whole-review commits; prior full-review/check
  requirements remain. Unknown legacy reviewer identity remains conservative.
- Scoped fixtures: unchanged grant/workspace fault waits; an exact authorized profile
  becoming selectable or a clean expected owned branch/base permits preparation.
  Dirty fixture files remain until the fixture explicitly removes its own edit.
  Independent work prepares without a host/model invocation.
- Repetition fixtures: duplicate coordinator/reviewer reads, equivalent settled
  failure, A→B→A state, useful evidence wake, persisted capacity wait without inference,
  restart deduplication and append-only readable history projection.
- Five TUI recovery tests and one stopped-release compatibility guard passed.
  Recovery events deduplicate; ordinary pasted discussion, drafts and independent
  operation labels remain usable. Chat cannot clear execution uncertainty.
- IPC schema/service minor1–9 consistency, maintained local documentation links and
  branch diff whitespace checks pass. Built `bundle-build`, `bundle-install` and
  `session upgrade` help match README syntax. Build products/runtime state excluded.

Fixtures use isolated local projects and synthetic native results. There was no
new user request, scope approval, fault injection into a real project, authenticated
model call, installation, service/binding change, push or merge. No fully autonomous
native recovery claim follows from these checks. Progress is structural and bounded
in retained comparison history, not a semantic guarantee or mandatory resource cap.
Unknown effects, integrity faults, unexpected work, OS/auth/config repairs and
provider/portability breadth remain the limits in the current contract.

## Existing supported install/upgrade path (not executed)

From this checkout in Fish, install this exact paired bundle only when intended:

```fish
./target/release/waywr bundle-install \
    --bundle "$PWD/target/waywr-bundles/recovery-20261017-67b93e7" \
    --prefix "$HOME/.local/share/wayward-wrangler" \
    --select-default
```

Existing project bindings remain on their prior release. Follow the
[README shutdown/quiescence procedure](../../../README.md#existing-projects-upgrade-versus-migration)
for the selected project: detached view, settled execution and inactive service.
Then use its supported guarded binding change, preserving state/history/authority:

```fish
~/.local/share/wayward-wrangler/current/bin/waywr \
    --project ~/projects/example-task-app session upgrade \
    --release recovery-20261017-67b93e7
waywr --project ~/projects/example-task-app
```

These are commands for a later authorized upgrade, not actions taken at closeout.
Do not resume dispatch merely to install. Guard refusal is not permission to delete
state or replay an uncertain operation. Last recorded Example Task App release remains
`conversation-20261017-6`, source `c6fadeb`, binding30; delivered commit
`af358d7560d85f14d56aff481e6f34c5bd6c22a9`, approval v3 and original evidence preserved.
Readable history remains recorded at
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
Future native observation belongs to the next real request originated by the user.
