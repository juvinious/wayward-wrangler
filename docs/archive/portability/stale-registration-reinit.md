# Stale project registration after local deletion

## Finding

Deleting a project's `.wayward-wrangler/` directory removes its default
authoritative state, but the user-local `locations.json` registry still reserves
that state path for the old project key. The previous Rust `init` wrote a new
`project.toml`, then refused registration with “State directory is registered
to another project.” The registry is a location index, not a backup of tasks,
decisions, receipts or approvals. This behavior was reproduced through the
previous installed `rust-startup-direct-v5` command in an isolated project.

The operator's affected project currently has a new project key and no
`.wayward-wrangler/state/`. Its location index still has the older key pointing
to that default state path; the old project systemd unit is not loaded. This
inspection did not alter the operator project or registry.

## Storage paths and intent

| When | Path | Content and authority |
| --- | --- | --- |
| `init` | `<project>/.wayward-wrangler/project.toml`, `workflow.toml`, `herdr.json`, `release.json` | Project identity, fake profile, selected Herdr endpoint and release binding. |
| `init` | `<project>/.wayward-wrangler/state/project-identity.json` | Local state identity. It contains no conversation or task history. |
| `init` | `<project>/.gitignore` | Ignores the local `state/` directory; it does not store memory. |
| After execution starts | `<project>/.wayward-wrangler/state/` | Authoritative task, context, decision, approval, artifact, receipt and recovery records, including `state.sqlite`. |
| After service attachment | `${XDG_STATE_HOME:-$HOME/.local/state}/wayward-wrangler/locations.json` | Project-key → project/state path locator for key-only review links. It is not authoritative project memory. `--registry-dir` overrides this location. |
| While running | `${XDG_RUNTIME_DIR:-/run/user/$UID}/waywr/<project-key>.sock`, a user service and Herdr resources | Ephemeral communication and visible execution infrastructure, not a second workflow store. |

**Intent:** default `waywr init .` or `waywr init SOME_DIR` writes only inside
the selected project and starts no service or Herdr process. It neither creates
nor changes the user-local location index. The default authoritative memory is
inside the project; deleting its `state/` deletes that memory, although backups
or retained terminal transcripts may exist independently. An explicit
`--state-dir` is an operator-selected exception and may place state elsewhere.

Runtime review-link routing still creates the external *locator* on first
service attachment. Its stale entries must not block local initialization or
stand in for missing project state. Eliminating even this routing metadata
would require a separate review-link lookup change; the current release does
not claim that active use makes no external writes.
Projects explicitly bound to an older installed release still route to that
release, whose historical `init` behavior is unchanged. The local-only contract
here applies to the corrected Rust release's own project initialization.

## Correction

`init` no longer registers a project globally. It creates a local state marker
and preserves repeat-init identity. On first service attachment, under the
registry lock, registration may replace a conflicting old entry only
when it names the **same canonical project and state path**, the previous state
path is absent or contains only the new local init marker, the old key is well formed,
the old systemd service unit is not found, and its configured IPC socket is
absent. A symlink, old marker or execution record is not a pristine state.
An inability to verify those conditions refuses replacement. Entries
for another project and any old state or execution evidence remain untouched. Repeated `init`
preserves the new key, state and release binding. This does not restore data
that was deleted with the old state directory, and it does not delete any
archived historical evidence.

The supported service lifecycle is systemd managed. A manually launched
service using an unrecorded, nonstandard IPC directory cannot be proven absent
from the location index alone; do not delete a project directory while its
service or workers are active. A loaded old unit or socket requires explicit
operator reconciliation rather than automatic replacement.

## Verification

Focused Rust unit tests cover safe same-path replacement, a pristine local
init marker, rejection when execution evidence exists, repeat init,
preservation when state exists or service verification fails, another project's
entry, and malformed old keys. The installed CLI fixture archives its first
project files as evidence, confirms the previous release fails after removal,
then confirms the corrected release reinitializes with a new key, leaves the
old registry unchanged during init, and leaves the archive unchanged. A fresh
init creates no registry at all. Fixture state, registry, installed bundle and
summary are retained under ignored `.local/rust-migration/stale-registry-fix/`.
The final installed candidate is `install-v3/releases/rust-local-init-v3/`
(manifest SHA-256
`4b4bfefe1e0ee3f2a384ff44558e660e44508e43b3e0521fa3ffc3168bfbee60`).
`installed-check-v3/summary.json` records the previous release's refusal,
the corrected release's local-only init, and repeat-init success. Six focused
Rust tests and the installed CLI fixture passed. The separate
`private-attach-v3/` simulated fixture passed first attachment, fake worker
visibility, detach/exit/resume, and cleanup of its test-owned service and
private Herdr server. A strict Clippy run was blocked by
pre-existing warnings in `backend/src/db_writer.rs` and prototype UI files;
those unrelated files were not changed.

This is simulated CLI verification. No Herdr, coordinator or worker was
launched. The operator's affected project has not been changed by the test.
