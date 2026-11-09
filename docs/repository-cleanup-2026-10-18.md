# Repository branch cleanup — 2026-10-18

## Post-merge cleanup after PR #10

GitHub confirmed PR #10
squash-merged to `421834725ca995c685608ccd99872efbd38c52b7`. Fetched local/origin
main matched that commit before this documentation-only followup. Its complete tree
`88a492df38622f79ad8ddb3cd517d7e9ca41a67b` equals original PR head
`ee4d9b21664f9cc0df01ef28378c794c14b59d87`.

| Removed local branch | Former tip | Proof and retained archive ref |
| --- | --- | --- |
| `impl/coordinator-owned-recovery` | `d1dda2450389b310ccd3bca469692fffa75ade69` | Ancestor of PR #10 head; `refs/archive/pr-10/coordinator-owned-recovery` |
| `plan/ordinary-project-workflow` | `b810bf11ab2ba2030ad279d2723ccb0076db6e4b` | Ancestor of PR #10 head; `refs/archive/pr-10/ordinary-project-plan` |

`refs/archive/pr-10/implementation-head` retains the complete original PR history
at `ee4d9b21664f9cc0df01ef28378c794c14b59d87`, including the original production
build checkpoint. Neither removed branch had commits outside that preserved history,
a linked worktree or an active recovery reference. Remaining textual branch mentions
are historical documentation, not runtime dependencies. No branch configuration
sections needed removal.

One atomic compare-and-swap Git ref transaction created the archive refs and deleted
only the two verified branch tips. No forced branch deletion, object pruning or
runtime cleanup occurred. `impl/ordinary-project-workflow` was already removed by
the user's merge; fetch/prune removed its stale remote-tracking ref. Only local
branch `main`, remote branch `main` and the existing main worktree remain. The
post-merge handoff/cleanup documentation is committed locally, not pushed. Operator
projects, installations, services, bindings, approvals and histories were untouched.

## Earlier cleanup before ordinary-project implementation

The following inventory describes the earlier checkpoint, when recovery/convergence
was still unmerged. Its remaining-branch list was superseded by the cleanup above.

Completed for **Wayward Wrangler only**. Initial checkout was clean on
`impl/coordinator-owned-recovery` at `d1dda2450389b310ccd3bca469692fffa75ade69`.
Fetched main/origin main both remain `799841c9f61c4a09217bc9607795edb8380670bf`.
The new `plan/ordinary-project-workflow` starts at that unmerged implementation tip;
all 13 commits after main, including recovery and convergence, are preserved.

## Evidence and decisions

GitHub reports PRs #4–#9 merged. For each, the complete PR-head tree equals its
squash-merge tree; ancestry then establishes which older topics that head contains.
This checks squash integration, not merely ancestry against main or branch age.

| PR | Retained PR head | Merge on main; identical tree |
| --- | --- | --- |
| #4 | `79615442f94326787e81a137ffeeeaff8e476d1d` | `ae367f2698f643c7b2d9c6ed91c8ad4d65d9b99b` |
| #5 | `31273fd1085d3cfce0501d13b4a82ebc2ac7b829` | `2e5cf1212fe9a319b118c641c5028c851f66e86d` |
| #6 | `714904b28ace0ae9df90dabe966f773321ce4a27` | `24fb31712cc4a52553375b0e5bc8de667f85a985` |
| #7 | `0a1f9726dc21d100758239db46db3ac0744ab1f6` | `1813a2f7de5f10a8eb3ef05232236148af370509` |
| #8 | `150369b50e4128b08abf994508bdd964cae0177d` | `c2064bc534d8cbe753bbae93a8d85a98ce8c403a` |
| #9 | `437eb83b0429104021389b4b3ff65ab0750d4526` | `799841c9f61c4a09217bc9607795edb8380670bf` |

| Removed local topic | Former tip | Completion proof |
| --- | --- | --- |
| `fix/reinit-stale-project-registry` | `dafb4f8451c725611091e0b3b264ffe73e02b107` | Contained in PR #5 head |
| `refactor/rust-backend` | `7b5d69e79f6a67bc297d0133a55eee31df1f8936` | All 17 commits beyond its remote tip are contained in PR #5 head |
| `plan/native-coordinator-integration` | `ba27dc2d2777511adb573152d81a93c6d2ce9368` | Contained in PR #6 head |
| `plan/live-task-proposal-authorization` | `8c1bc7d912c33a9378429bdf03adcffc1d214a18` | Contained in PR #7 head |
| `plan/complete-development-workflow` | `6b04221804e47e1ebc9307b3f0f5b10e05a4a115` | Contained in PR #8 head |
| `impl/development-workflow` | `150369b50e4128b08abf994508bdd964cae0177d` | Exact PR #8 head |
| `plan/provider-model-selection` | `38ce3a19105589e39fb68439e0869fc598eb33ee` | Contained in PR #9 head |
| `impl/provider-model-selection` | `a2d376df8d2e394acfb57c490d2d72e7dbc857b9` | Contained in PR #9 head |
| `plan/chat-first-coordination` | `f5139dbd5af2b7fdd5b0472c2c81a868baae15d2` | Contained in PR #9 head |
| `plan/coordinator-owned-recovery` | `d21271ff07e9b42134f0ac6336da3c128b24397c` | Ancestor of preserved active implementation; plan implemented/superseded, original proposal archived |

Removed remote `milestone-6-interactive-shell` at
`79615442f94326787e81a137ffeeeaff8e476d1d` (exact PR #4 head) and
`refactor/rust-backend` at `d14765ba03e739f35f9450258e03bbd4f2dbe923`
(contained in PR #5 head). One atomic deletion used exact-tip leases; no branch
content was pushed. `git ls-remote --heads` afterward contains only main.
`origin/impl/conversational-coordination` at `437eb83...` was already absent remotely;
fetch/prune removed that stale tracking ref. The other removed topics had no live
remote branch. No PR was created or merged.

## Preserved resources and verification

- Remaining local branches: `main`, `impl/coordinator-owned-recovery` and
  `plan/ordinary-project-workflow`. No uncertain/unique work was deleted.
- Sole worktree: `~/projects/wayward-wrangler`, active and clean before
  documentation edits. No inactive worktrees existed; none were removed.
- Installed releases contain regular copied files, with no source-tree symlinks.
  Manifest/binding resolution uses release IDs and hashes, not these topic names.
  Default `current` still points to `releases/convergence-20261018-beb79ee`;
  launcher, installations and all managed-project bindings/state were untouched.
- One complete-history local bundle retains all former refs and original source
  lineage for reconstruction/provenance, without replacement topic branches:
  `.git/cleanup/2026-10-18-before-cleanup.bundle`.
  `git bundle verify` passed; SHA256
  `c376e417507c4e76bc31f1ba1972c15a20bcc29107dd0826ee35578c118466fa`.
  This is a local preservation artifact, not a published backup. No Git object
  pruning was performed. The inventory above is committed; the bundle is not.

No cleanup exception requires a user decision. Current product work is governed by
the [connected plan](archive/product-workflow/approved-plan-b810bf1.md), not the removed historical branches.
