> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — identity, directory migration and local state

Project: **Wayward Wrangler**. Repository/package/primary CLI: **wayward-wrangler**.
Description: **Persistent workflows for coding agents.** Version remains 0.1.0.

## This checkpoint

- Package metadata and future CLI/controller/review labels use the new name.
  `wayward-wrangler` and compatibility alias `awp` call the same entry point.
- Internal Python module `agent_workflow`, plugin ID `local.agent-workflow-review`,
  `awp.invalid` review URLs, and `awp-runner` metadata source remain stable protocol
  identifiers. The repo-local plugin display name is Wayward Wrangler Review.
  Its handler uses the primary command; no global plugin reinstall was performed.
- Existing real-trial profiles retain recorded machine-specific binary paths and
  model choices. They contain no authentication material and do not authorize
  fresh real runs. Installations are project-local, not global.
- AGENTS.md and CLAUDE.md are intentionally byte-for-byte preserved: the approved
  context-01 envelope pins their hashes. Their generic project instructions still
  apply. This handoff/migration note carries the rename authorization and identity.
- Reports preserve historical names, original failure/recovery, paths, hashes and
  approvals. No `.local` evidence was rewritten for branding. Git excludes it.

## Physical move deferred

Current location: `~/projects/example-workspace`.
Intended location: `~/projects/wayward-wrangler`.
The target did not exist at inspection. The active Fish/Codex session and helpers
use the old cwd; editable metadata and script shebangs use the old path. Herdr's
registered plugin points into the old directory. Sixty-two saved JSON/Markdown/text
files contain 145 old-root references, in addition to SQLite envelopes/checkpoints.
There are no active workflow runs; all four recorded runs are approved. Only the
ongoing development tab was present in the project workspace at inspection.

Moving now would disrupt the current session and leave unresolved runtime paths.
Only the physical move is deferred. The first Git checkpoint/private remote do
not depend on it. The existing Herdr workspace label `example-workspace` remains
unchanged, with recorded workspace ID w8; display-name changes can be done later.

## Required follow-up before any physical move

**A symlink alone is insufficient.** `context.project_file()` canonicalizes paths;
`context.resolve()` serializes those absolute paths and `context.check()` compares
the whole reconstructed snapshot to the frozen one. Moving the tree changes that
snapshot even with an old-path symlink. `recovery.verify()` checks this context on
receipt inspection, so an adopted completion would become unverifiable. No move
or runtime-path refactor is included in this checkpoint.

1. Plan and separately authorize the scoped [portability follow-up](portability-plan.md).
   Separate immutable provenance from current project/state location bindings;
   cover `storage.ROOT/LOCAL`, context resolution/checking and recovery validation.
   Test unchanged frozen context, decisions, hashes and original failure/adoption
   evidence in a relocated fixture with the old location absent. Do not weaken
   stale-content checks or broaden the authorized saved-output recovery.
2. Only after those checks pass, confirm no active project worker, runner,
   controller or editor. Exit this Codex
   session normally and leave its cwd in every shell. Preserve unrelated Herdr
   panes and the shared server. Confirm the destination does not exist.
3. Move the entire directory, including `.git`, `.local` and `.venv`, from another
   shell. These Fish commands are **conditional on steps 1–2**, not safe migration
   instructions for the current implementation:

   ```fish
   cd ~/projects
   test -d example-workspace; or exit 1
   test ! -e wayward-wrangler; or exit 1
   test ! -L wayward-wrangler; or exit 1
   mv -- example-workspace wayward-wrangler; or exit 1
   cd wayward-wrangler
   ```

4. Use the installation, explicit project/state binding and standalone plugin
   registration commands implemented and verified by that follow-up. Today's
   editable install, old venv shebangs and linked plugin require migration; a
   symlink is at most a temporary compatibility measure, not the intended runtime
   dependency. Never mass-replace paths or recalculate hashes in `.local`.
5. Run offline checks and inspect approved run status/review artifacts. Verify
   source/decision/output bytes and failed/adoption records against their saved
   integrity hashes. No new worker calls should be required. Verify operation with the old
   directory absent, including the Herdr review handler. Resume this conversation
   from the new directory only after verification:

   ```fish
   codex resume <private-session> --cd ~/projects/wayward-wrangler
   ```

Update the handoff with actual installation/state locations after migration. The
current directory remains the supported runtime location until this work passes.

## Local-only state

The user explicitly declined NAS backups. They are not pending work or a
prerequisite for migration. GitHub stores the tracked source and documentation.
The following remain local-only under the existing exclusions:

- `.local/`: SQLite state, frozen inputs, native outputs/transcripts, decisions,
  approvals, packets, process bindings, invocation counters and recovery evidence.
- `.venv/`: reconstructible project-local environment.
- External Herdr session/plugin registration and Codex conversation history.
- Existing Claude/Codex authentication, which remains outside Git.

Preserve existing evidence during future work. This instruction does not change
Git exclusions or authorize uploading runtime records or credentials.
