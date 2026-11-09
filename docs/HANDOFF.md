# Waywr handoff — model and thinking-level selection/display

Branch: `plan/usability-and-installation`. Base: `1aee2a3` (complete catalog and
keyboard controls). Follow [AGENTS.md](../AGENTS.md), the
[TUI keyboard contract](tui-controls.md) and [provider contract](provider-selection.md).
Local implementation/commits only. The user handles remote delivery. No operator
project/service/settings, account or global installation was changed.

## Current behavior

F3 Profiles has a Thinking field between Model and Options. Enter opens the same
keyboard picker used for models, populated by that model's SDK-advertised levels.
No Rust/provider-specific level list exists. The choice is `options.reasoning`,
part of the existing exact profile/grant identity and frozen operation packet.
Changing models preserves a supported explicit level or takes the new model's
advertised default. Profiles without a choice show the runtime default; manually
entered models without capability metadata remain unverified and editable.

F3 pins the draft model/level separately from the active pair. The main view has
a dedicated active coordinator model/level row, wrapping at narrow widths instead
of burying it after the project title. Selecting or saving a profile does not
silently switch the active coordinator or grant context. Existing switch/policy
amendments remain explicit. The prior keyboard/catalog and schema2 inspection-only
startup fixes remain in effect; historical work and old bindings stay preserved.

The shared runtime derives `thinking_levels` and `thinking_default` with SDK
helpers; custom models may pass `thinking_level_map` through to those helpers.
Catalog metadata version2 refreshes old caches offline on normal service opening,
without rewriting saved preferences/grants. Fresh profiles save an explicit
runtime default. Known unsupported levels refuse before auth/HTTP rather than
silently clamping. The selected level is supplied to the actual agent loop;
streaming SDK options no longer override its reasoning state independently.
Events/receipts retain the requested level. These are requested settings, not
provider attestation of hidden computation. Manual-model capabilities stay unknown.
The pinned standalone pi-ai/pi-agent-core versions and all dependency locks remain
unchanged; no native integration or provider/model allowlist was restored.

## Evidence and limits

Behavioral red showed absent per-model catalog levels and the missing selector/
main-view level display. Catalog and UI input/render checks now pass. The UI
chooses a runtime-defined level absent from any Rust enum, emits an explicit save,
shows draft versus unchanged active settings and preserves controls at supported
sizes. Existing frontend library checks pass; the sandbox-blocked Unix socket-pair
test passes with socket permission. The explicit30-second probe remains ignored.

Real SDK/agent-loop HTTP fixtures verify high reasoning in OpenAI Completions wire
requests and enabled thinking budgets in Anthropic Messages across both tool-loop
turns. Unsupported known-model levels refuse with zero HTTP requests. Offline
metadata covers custom supported/unsupported maps and nonreasoning models. Existing
dummy-key concurrency and missing-credential fixtures remain isolated, not account
calls. Backend save/select/reopen retains the level in opaque profile options.

The paired-binary terminal fixture chooses a model at the catalog tail and its
last advertised thinking level, explicitly saves and reopens after removing the
old bundle. Level, settings, draft and pause persist; no model call or manual
rebind occurs. The remapped locked build and public-content checks cover the local
review bundle. TestBackend captures provide layout evidence; ordinary human
verification remains needed. Real provider login/model quality/useful delivery and
public download/update/package channels remain unverified or incomplete.

## Next authorized action

The ignored `target/local-bin/waywr` entry points to a fresh temporary paired
review bundle `v0.4.0-local.thinking`, manifest SHA256
`e85dbf9d6aef59a28e2935a0661e5f5fa41e0249325b65bcb6e343a02ada92b6`.
Source, all three binaries and all19 bundle artifacts pass the public-content
check. Detach the currently open TUI before relaunching so the existing
idle service can hand over. F3 → choose Model → Thinking → choose level → S save.
Draft and active labels explain what still needs an explicit coordinator switch.
The user verifies ordinary use; fix its next concrete interruption without a new
provider-specific path or broad test campaign. No operator service was restarted
and no account/model call was made by the agent.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[product plan](usability-plan.md), [runtime direction](execution-adapter-decision.md),
[distribution](distribution.md), [verification](verify-usability.md).
