> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler development

Start with `AGENTS.md`, `docs/HANDOFF.md`, and task-relevant current documents
listed in `docs/README.md`. Read archives selectively for evidence; do not load
the development history by default. Each new work item starts on a dedicated
branch; do not merge without operator approval. This project is independent of
another project. Do not use or modify that checkout without a separate request.

## Documentation checkpoint rule

**At every checkpoint**, reassess documentation against the current task and
implementation. This includes planning and implementation checkpoint commits,
handoffs, scope changes and task/branch transitions; it is not just branch cleanup.
Do the assessment before committing or handing off, even when no code changed.

- Replace `docs/HANDOFF.md` with concise current state: objective, authorization,
  branch/base/checkpoint, verified versus unverified behavior, blockers, protected
  resources and exact next action. Do not append development history.
- Reconcile changed requirements into the document that owns each contract.
  Keep implemented behavior, approved future work and unapproved proposals distinct.
- Reclassify current documents by relevance. Archive completed reports, superseded
  proposals and correction histories under `docs/archive/<topic>/`. First extract
  still-applicable requirements into current contracts; never bury an unresolved
  obligation. Keep a concise current issue and link to its original evidence.
- Update `docs/README.md`, entrypoint references and affected links. The active
  reading path should explain today's work without loading completed milestones.
  Preserve evidence, quoted historical paths and immutable archive snapshots.
- Verify link/reference changes and review the current set for contradictory or
  stale instructions. Record remaining documentation conflicts in the handoff;
  if no reorganization is needed, do not manufacture document churn.

A clean current documentation set preserves contracts and unresolved constraints;
it does not erase history or reset authorization. Reassess at each checkpoint,
not after each keystroke. This is the canonical policy; `CLAUDE.md` points here.

For an authorized PR merge, check required gates, then use `gh pr merge -sd`
when squash is allowed. `-s` squashes and `-d` deletes the merged local and
remote topic branches. If the operator or repository requires a different merge
method, use that method with `-d`. Verify the merge and branch cleanup; use
`git branch -d` only for a branch proven merged into main. Never force-delete
unmerged work or bypass a merge gate.

The accepted Rust/Ratatui TUI and Rust workflow backend are the current
implementation. New host-backed projects execute in project-owned systemd user
services; Herdr is optional, explicit read-only inspection for them. Existing
Herdr-backed and unfinished legacy runs retain
their bound execution contract. Simulated acceptance does not authorize
authenticated native coordinator or worker calls. Different projects own
separate state, queues, services, execution budgets and context. One executing
workflow per project is the current limit. Explicit cross-project reads require
a bounded operator grant; the global location registry is not agent context.

Preserve historical `.local` state, decisions, approvals, receipts, recovery
records and retained Herdr panes. The project supervisor owns execution and
recovery for new host-backed projects; runner receipts own completion evidence.
Historical LangGraph runs remain on their bound legacy releases.
Worker output cannot authorize decisions. Simulated acceptance fixtures cannot
authorize real approval. Do not change shared Herdr configuration, close
unrelated panes, or stop the shared server. Use Fish-compatible examples.
