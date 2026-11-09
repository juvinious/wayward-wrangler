# Ordinary project workflow

Approved scope: [product workflow plan](archive/product-workflow/approved-plan-b810bf1.md), including `b810bf1`.
Implementation preserves recovery/convergence and existing approved policies.
This contract describes delivered behavior; HANDOFF tracks remaining connected work.

## Setup and baseline

New initialization records project-local setup in `.waywr/workflow.json`; it uses
Git `info/exclude` for Waywr state instead of editing tracked `.gitignore`.
Root, nested directory and linked-worktree attachment resolve one canonical project
identity. Existing project setup and approvals are not replaced by new defaults.

F3 → 7 Project selects committed baseline/ref, named target, checked local or GitHub
PR completion, human validation, retention and finite argv recipes. Scope approval
shows and freezes those choices. Saving preferences does not authorize execution.
New projects recommend committed HEAD, a detected default/upstream/current target,
checked local delivery and no universal human validation gate; both validation
choices remain explicit on the approval card. Ambiguous/missing baseline or target
blocks execution with the specific setup choice needed.

Non-Git/unborn input requires explicit `initialize-files` selection and the initial
scope approval of a hashed source manifest. Changes after approval refuse import.
Credential/state/dependency paths and symlinks are excluded. Existing files are never
implicitly committed. Committed-base development excludes dirty edits and can proceed
in isolated worktrees without stashing/resetting the primary checkout. Local delivery
uses the named target, a verified clean target checkout or CAS of an unowned target
ref. Changed targets and dirty target checkouts remain preserved and block delivery.

## Project knowledge and recipes

The incremental project index records every eligible tracked ordinary blob's identity,
module path and instruction/convention classification at an exact commit. Commit
changes refresh changed/removed paths; mutable file hashes are separate observations.
Applicable root/nested AGENTS/CLAUDE instructions enter assignment/review context.
The complete index is paged through the existing immutable evidence mechanism;
initial summaries disclose selection, omissions and exact retrieval handles.
Credential paths, symlinks and submodules are excluded from automatic ingestion.

Project manifests suggest scoped Cargo/package-manager/Go argv recipes, labelled
unexecuted. Exact approved check/build/inspect recipes can run through supervised
finite check execution; preview recipes remain instructions and do not start services.
Existing tracked-script checks remain compatible. No model call occurs at discovery
or initialization. General project guidance never substitutes for exact receipts.

## Candidate and resource lifecycle

New requests use their one readable governing `/integration` branch. Legacy aliases
remain compatible. Coordinator `candidate`/`complete` decisions supply typed,
project-specific environment, prerequisites, build/run/inspection, expected result
and shutdown instructions. Suggestions remain distinct from actual check receipts.
A saved candidate binds scope/generation, commit, reviews, checks and obligations.
Its conversation card is marked presented only after it is visible. Inspect retains
full details; conversation shows a concise selection.

After required validation is selected, the service persists a handoff, verifies a
clean exact governing checkout and detaches it, freeing the branch for ordinary
`git switch`. Say `check out candidate` to switch a clean registered primary
checkout through the TUI. Dirty or competing ownership refuses without overwrite.
The service holds governing writes during validation; independent workers continue.
`accept candidate` or `looks good` accepts only the exact presented candidate bound
to that input. `candidate feedback: …` returns scoped feedback to coordination.
Clear source-bound feedback in normal chat can also be classified by the live
coordinator through existing steering output; it becomes exact candidate feedback,
not a new approval or an unbound generation change.
`include committed changes` explicitly imports committed human edits as a submitted
unit needing review and fresh combined checks. `end validation` releases ownership;
required acceptance remains outstanding without repeated inference. A scope change
while a checkout is held asks for release/feedback first; it never strands an old
handoff by silently changing its generation. Ending a handoff leaves the user's checkout
detached at the actual input; it never restores an earlier branch silently.
Quoted/imported text and stale drafts cannot accept a different candidate.

Default `retire-clean-owned` retention removes eligible service-created temporary
worktrees/branches after disposition, positive settlement and cleared dependencies,
obligations and recovery. Cleanup saves exact intent and archival non-branch Git refs
before removal, then reconciles that same intent after restart. Unknown, changed, preexisting or user-owned content remains with a recorded reason.
F3 → Project can declare `generated_outputs` as exact relative output roots such as
`["target", "package/__pycache__"]`; these are included visibly in scope/policy
approval, default empty, and never amend an existing approval. No globs, state or
dependency directories are accepted. Worker/check execution saves immutable before
and terminal inventories. Retirement can unlink only newly produced ignored regular
files under those roots, at the recorded commit and unchanged hashes. Ignore rules
alone grant nothing; symlinks, nested repositories, nonignored or uninventoried files
remain. Hash manifests stay with execution evidence. Later checks carry forward output
ownership only when prior hashes still match at their start and the producing commit
is an ancestor of the new input; rebuilds then inventory their resulting bytes.
A user-modified preexisting output never regains ownership merely by being ignored. A new output declaration does
not retroactively classify old build products. There is no
force removal or broad prune. After confirmed delivery, the governing resource is
also retired when clean and unowned by the user; `refs/waywr/archive/REQUEST/governing`
keeps the combined commit reachable. Receipts/history and exact evidence remain.

`checked-local` uses expected-target/ref checks and clean target ownership. A newly
advanced target supersedes an unstarted promotion intent and can be integrated with coordinator `refresh_target`, then reviewed
and checked again. Non-fast-forward target changes need a substantive baseline decision. `github-pr` freezes owner/repository, remote, base and merge method
at scope approval. The service pushes the exact checked governing SHA, reconciles a
request-marked PR, waits for normal checks/reviews and merges without admin/force
bypass. Lost create/push responses with no matching identity remain explicit blockers;
no duplicate publication is guessed. Confirmed remote delivery does not silently reset
or advance the local target. Remote branch deletion is deferred to repository policy;
local owned resources follow the retention rules above.

Source owners: `candidate.rs`, `project_resources.rs`, `project_publication.rs` and
existing development/coordination/context/history/host machinery. Automated Git and
transport fixtures cover changed mechanisms; ordinary native end-to-end use and
GitHub publication are not established by those fixtures. Waywr remains experimental.
