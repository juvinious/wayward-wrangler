> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Installed-provider handoff from `a2d376d`, archived during the 2026-10-17
> coordination review. Service-state and next-action statements below are historical.
> Links are relocated; see [current handoff](../../HANDOFF.md).

# Waywr handoff — installed provider selection (2026-10-16)

## Current checkpoint

`impl/provider-model-selection` starts at approved planning commit `38ce3a1`.
Source checkpoints: `0dfcbb0`, `49ecf3b`, `3d5cfbc`, `d81f631`; intermediate
installation record `7c14550`. Nothing pushed, merged or published.
Stable paired release `providers-20261016-3` (source `d81f631`) is installed and
selected under `~/.local/share/wayward-wrangler`; all 13 manifest hashes pass.
Example Task App's guarded upgrade completed at binding20, state9/config6/IPC1.7.
The session is paused and its service inactive. Shared provider settings/auth are intact.

## Implemented path

[Provider selection](../../provider-selection.md) owns F3 profiles, role preferences,
task overrides, explicit fallback, policy amendments and coordinator switching.
Codex gpt-6.1-sol/medium remains the default; Claude is selectable for coordinator,
implementation and exact-commit review. Attempts freeze their profiles. Both
coordinator lanes share one binding; interruption requires positive settlement.
Existing scheduling, concurrent worktrees, serialized integration and history remain.

Modal action rows accept Left/Right and Enter alongside Tab/Shift-Tab. Focused text
editors retain cursor movement; queue Up/Down selects messages. See the
[TUI contract](../../contracts/tui-contract.md) and [README controls](../../../README.md#tui-controls-and-activity).

## Verification and limits

Passed: 15 provider/recovery/migration/mixed-packet tests; 24 development and 11
continuous regressions; two Claude adapter tests; six worker-activity tests;
95 frontend tests (one existing optional latency probe skipped); normal-init and
schema-range binary tests; locked paired release build, installed backend identity,
manifest hashes, IPC source, documentation references and diff checks. Modal tests
cover selection without activation/draft loss, detach and explicit queue actions.
Fixtures do not establish native provider usability or human workflow acceptance.

The authorized upgrade followed verified UI detachment and idle-session checkpointing.
Preservation comparisons covered original rows in 27 record groups, 331 history,
receipt and project files, plus Git HEAD/refs/worktrees/status. Original records,
readable logs/summaries, code and receipts are unchanged. Maintenance appended
lifecycle events544/545 and refreshed timestamps in two projection metadata files.
The exact existing Codex binary, Codex-only role permissions and empty fallbacks remain;
Claude's discovered profile grants no new authority. Canonical/legacy aliases persist.

No authenticated inference, request, scope approval or demonstration was submitted.
Native mixed-provider work, handover, authentication/capacity and sandbox behavior
remain unverified until ordinary use. Managed restrictions, same-UID boundaries,
provider reliability and Linux portability remain limitations; Ollama/OpenCode
execution and distribution packaging are outside this increment.

## Next action and retained delivery

The user launches `waywr --project ~/projects/example-task-app resume`, chooses profiles
in F3, originates the next request and approves its scope/policy through Waywr.
No further implementation checkpoint approval is needed; do not seed a job.

Preserve the confirmed assisted human-led Example Task App Python migration at
`069a162fbcb118794515034d3ae91303e7c37135` and its retained 44-test/seven-smoke evidence;
those checks were not rerun. Actual readable history:
`~/projects/example-task-app/.waywr/history/20261016T135632Z-15a13c43b482.log`.
The initial agent-prepared demonstration is distinct; see the
[assessment](../development-workflow/2026-10-16-delivery-assessment.md).
The [approved plan](approved-plan-38ce3a1.md) is archived;
its proposed real-job exercise was superseded by the user's ordinary-use boundary.
