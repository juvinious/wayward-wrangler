> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Waywr handoff — provider selection planning (2026-10-16)

## Checkpoint and scope

Fetched origin and verified local main, origin/main and initial HEAD all equal
`c2064bc534d8cbe753bbae93a8d85a98ce8c403a` (merged PR #8). The tree was clean;
created `plan/provider-model-selection` without discarding work. This planning commit
is identified by `git log -1`. No implementation, inference, workers, service or
installation changes, shared configuration edits, push or merge are authorized here.

The single next proposal is [provider/model selection](../../provider-model-selection-plan.md).
It extends the delivered development workflow through the production TUI. Completed
[implementation](../development-workflow/2026-10-16-implementation-closeout.md),
[assessment](../development-workflow/2026-10-16-delivery-assessment.md) and
[closeout handoff](../development-workflow/2026-10-16-handoff.md) are archived
selective evidence, not a renewed acceptance program or standing runtime permission.

## Delivered baseline to preserve

Rust/Ratatui TUI and service/runner deliver conversation, exact scope/policy approval,
concurrent task worktrees, frozen commit review, serialized integration, exact combined
checks, guarded local-main advancement, persistent `.waywr/history/` and selected
project context. Workers dispatch independently of coordinator/check calls. Existing
receipt, generation, dirty-worktree and uncertain-execution safeguards remain.
No mandatory task-time/call/repair ceilings apply; capacity waiting is supported.

The human-led Example Task App Python migration reached local main
`069a162fbcb118794515034d3ae91303e7c37135`, with retained evidence of 44 tests and seven
smoke scenarios. It was assisted; this does not prove every interaction or unassisted
reliability. Those checks were not rerun. Last recorded installed release is
`development-20261016-9`, Example Task App binding generation19. This pass did not inspect
or change that project's code/state/service/binding or the installation.

## Next increment and inspected gaps

F3 becomes project-local provider/model/effort and role-default selection. Add Claude
beside Codex for coordinator, implementation and exact-commit review, then safe
coordinator switching and optional visible fallback. Ollama/OpenCode inventory and
adapter assessment are included; their execution is subsequent, not a prerequisite.
Keep normal init/start/resume and optional Herdr. Linux portability and unrelated
refactors remain outside this increment.

`development.rs` currently hard-codes one Codex profile/policy/inventory and copies it
into all units; Settings repeats it. `runner.rs` selects Codex-specific development
parsing. Existing Claude safe launch/auth/stream/structured-result code is reusable,
but its old text-only/one-turn/deadline limits and pinned registry are not development
requirements. Native `select_coordinator` is currently allowance authorization, not
safe general replacement. Change these connected seams, not the delivered scheduler.

Separate routing preferences from authority. Apply task override → allowed coordinator
choice → role default → worker default, always constrained by the saved policy.
Freeze each attempt's exact profile/config/context. Save/switch within approved choices
needs no scope renewal; a new provider destination is a substantive policy amendment.
One binding governs discussion and development coordination, switching after current
calls settle while workers continue. Explicit interruption and fallback retain the
existing settlement/reconciliation rules. Context and request history remain owned by
Waywr, not a shared vendor session.

## Read-only evidence and next action

Local discovery found default Codex 0.163.0 and Claude 2.1.301; retained source-pinned
0.162.1/2.1.296 executables still exist. Auth-file presence is not authenticated role
proof. Codex cache reports client0.154.0 and omits the working gpt-6.1-sol profile;
do not silently replace it. Shared model/effort values are observations, not imported
preferences. Claude's installed `--bare` excludes saved OAuth/keychain; retain auth
through safe per-invocation settings rather than choosing that shortcut.

Existing Ollama0.40.4 metadata advertises qwen3:8b Q4_K_M completion/tools/thinking;
no inference ran. OpenCode2.0.29 has JSON run/model-variant flags but model listing
uses a server, so it was not invoked. Common config/auth locations were absent;
actual configuration remains unverified. All detailed evidence/limitations and
focused verification are in the proposal. No provider capacity or new role usability
is inferred.

Documentation link/anchor and diff checks cover this commit; no application tests,
builds or runtime acceptance were needed. Stop with the proposal ready for review.
Recommendations: preserve Codex defaults, make Claude an explicit project/role choice,
and keep automatic fallback disabled until named alternatives are selected. Subsequent
authorized implementation proceeds through connected recoverable checkpoints, not
prerequisite trials, allowance renewals or exhaustive provider matrices.
