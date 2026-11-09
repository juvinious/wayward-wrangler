# Canonical project layout and migration

New projects use `.waywr/project.toml`, `.waywr/coordinator.json`,
`.waywr/state/`, `.waywr/worktrees/` and
`.waywr/history/<UTC-timestamp>-<hash12>.log` with an adjacent compact summary.
For example: `20261016T143000Z-a1b2c3d4e5f6.log` (33 characters). The hash derives
from the initial prompt hash plus stable request identity, so repeated prompts
remain separate requests. A reserved/existing filename or sidecar extends the hash
instead of overwriting or mixing histories. Full request text, readable name, prompt
SHA-256 and request ID remain inside the history. Saved legacy paths are unchanged.
Explicit external state selections retain their selected history root.
Init creates no request, task sequence, mock profile or execution approval.

A canonical project's legacy `.waywr/release.json` is retained metadata. Normal
opening checks project identity, supported storage/semantic contracts and current
bundle integrity; it does not require or execute the recorded old build. A
compatible stopped project can reopen after that installation is removed. No
historical approval, packet or receipt is relabelled. Idle-service handover verifies exact process/socket ownership and positive
quiescence. Live views, pending delivery and uncertain launches retain the current
owner; explicit older-format migrations keep their separate constraints.


For a legacy project, use a current installed release:

```fish
waywr --project PROJECT session migrate
```

Migration validates the target paired release's `waywr-v1` layout capability and
project identity. It refuses a live TUI or pending client delivery. The project
service receives the normal durable stop command, drains to paused, and stops.
Startup/supervision fences and exact host receipt/quiescence checks precede moving
anything. Uncertain execution is not replayed. No model calls or development job
are required for migration.

A persistent `.waywr-migration.json` journal makes interrupted relocation resumable.
A pre-existing `.waywr` is never overwritten or silently combined with legacy state.
Conflicts stay intact for explicit reconciliation. Re-running the migration repairs
its own recorded partial steps; it does not treat an unrelated directory as its own.

The legacy root moves to `.waywr`; Git's supported `worktree repair` updates linked
worktree references, including dirty retained trees. Only mutable worktree/history
location records and the exact project registry entry are relocated. Approval
payloads/hashes, task identities, code commits, packets, receipts and historical log
frames retain their bytes. The compatible release is bound through the existing
release-maintenance path. Migration leaves execution paused; ordinary startup
reconnects the new project service. `/resume` or `waywr resume` is an explicit user action.

Two validated compatibility symlinks preserve immutable old absolute references:
`.wayward-wrangler → .waywr` and `.waywr/state/history → ../history`. They point to
one physical canonical state/log tree, not copied writable state. New runtime writes
and Inspect paths use `.waywr`. Do not remove these aliases while evidence still
references them. Arbitrary links remain refused by maintenance checks.

Git local excludes retain existing contents and ignore canonical state, migration
metadata and the legacy alias without editing tracked application source. New init
updates project ignore rules. Runtime registration changes only this project key;
shared provider configuration, credentials and unrelated services are untouched.

Custom external `--state-dir` relocation is currently refused by `session migrate`;
those files remain in place. Cross-filesystem root relocation and active-view
migration are not supported. A migration error leaves its journal and evidence for
inspection/retry, never restores an old snapshot over accepted work.
