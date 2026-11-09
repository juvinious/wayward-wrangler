> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Coordinator recovery installation — 2026-10-17

Authorized installation follow-up to clean implementation checkpoint
`ba9547b4c091139f5aa6dc73f21970f238ef675d` on `impl/coordinator-owned-recovery`.
No source changes, rebuild, broad tests, new request or native call in this step.
[Build and focused verification](2026-10-17-build-and-verification.md) records the
source `67b93e7`, binaries and automated coverage; those checks were reused.

## Installed and bound

Verified bundle and installed release `recovery-20261017-67b93e7`: all 13 file hashes
match. Manifest SHA256:
`295f0986be0df442f11f71179a03ce6a640af64fc78f766f184c56ef68a83c25`.
Installed at
`~/.local/share/wayward-wrangler/releases/recovery-20261017-67b93e7`.
Normal command resolves through `~/.local/bin/waywr` → installation `bin/waywr` →
`current/bin/waywr` to that exact release. `waywr --backend-info` reports the new ID.
The installer selected it as the default; older installed releases were retained.

Example Task App project `~/projects/example-task-app`, key `<private-id>`,
was upgraded through supported `session upgrade`, binding30 → **31**, from
`conversation-20261017-6` to `recovery-20261017-67b93e7`. Manifest hash matches its
`.waywr/release.json`; `.waywr/release-history/31.json` records the guarded transition.

## Activity and preservation

Initial inspection found delivered workflows, no queued/reserved conversation turn,
no running development unit, no pending workflow operation or client delivery, and
no running/starting project host unit. A verified live TUI prevented upgrade; the
user detached it before maintenance. No view or native process was interrupted.

Supported `session stop` checkpointed running generation42/revision970 to **paused,
generation43/revision972** (events971–972). The service exited on drain; its transient
unit was collected before the redundant `systemctl stop` could run. A subsequent
read-only check positively showed inactive/dead, MainPID0. The supported upgrade
then validated quiescence, retained host receipts/units and compatible contracts.
No service restart or dispatch resume followed; status checks remained offline.

Before/after checks verified unchanged rows across 14 protected conversation,
coordination, approval and development tables; all existing event bodies through970;
1,156 immutable packet/result/completion/receipt/evidence files; all three readable
history prefixes; project identity/provider configuration; Git refs, worktree identities
and main status. Lifecycle/checkpoint and release-binding records changed as expected.
Preservation digests were kept locally under `/tmp`, without copying conversation or
credential contents into the repository.

All three workflows remain delivered. Main stays clean at
`af358d7560d85f14d56aff481e6f34c5bd6c22a9`; 13 conversation turns remain retained,
with none queued/reserved. Approvals, Codex/Claude role pins, completed work and
original reviewer output are unchanged. Label-management readable history remains:
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.

## Ordinary use and limits

```fish
waywr --project ~/projects/example-task-app
```

The session remains intentionally paused after maintenance; launch/inspection is not
an instruction to resume development. The user owns the next actual request and its
scope approval. Focused automated recovery checks passed, and installation/binding/
preservation were verified here. **Native autonomous recovery has not yet been
observed.** There was no request, approval, development resume, model call, cleanup,
push or merge. Current recovery and provider/portability limits remain in the
[contract](../../coordinator-recovery.md).
