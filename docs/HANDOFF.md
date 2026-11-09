# Waywr handoff — keyboard navigation and complete model picker

Branch: `plan/usability-and-installation`. Base: `bff06fd` (retired-profile
startup fix). Follow [AGENTS.md](../AGENTS.md) and the
[TUI keyboard contract](tui-controls.md). Only local implementation, isolated
checks and local commits are authorized; the user performs remote delivery.
No operator project/service/settings, account or global installation was changed.

## Delivered behavior

F3 previously showed only the first16 models for the selected provider. Its arrows
cycled profiles, appearing inert with one profile; global Escape closed the form
even while editing. F3 now offers a complete viewport-based provider/model picker.
Type or paste to filter provider/model IDs or names. Up/Down or Tab/Shift-Tab moves
the highlighted result, page keys move a visible page, Home/End reaches first/last,
Enter explicitly chooses, and Esc returns one level. Providers with no catalog
models remain listed. Empty results explain manual entry rather than selecting a
fallback. No provider/model allowlist or new native integration was added.

Profiles use Up/Down or Tab/Shift-Tab for fields and Left/Right for profiles.
Enter opens the applicable picker/editor; E edits any exact value or ID, including
IDs absent from the catalog. Enter accepts an edit, Tab/Shift-Tab accepts and moves
focus, Esc cancels only that edit. Project settings use the same field controls.
The focused field follows scrolling; section changes reset it. Section tabs,
control hints and the provider-setup warning stay fixed. Search and value editors
show a real text cursor. Validation/save feedback stays visible in the modal.
Read-only content popups support Home/End as well as line/page navigation.
AGENTS and the control contract establish these conventions for future panels.

Picker selection changes a draft only. S persists preferences; coordinator
switching, exact context grants and request policy amendments stay explicit.
The schema2 inspection-only startup fix remains: legacy work/history opens while
old execution stays blocked. Old revisions and execution ownership are preserved.
The shared standalone pi-ai/pi-agent-core runtime, model-independent coordination
and existing recovery boundaries are unchanged. Node >=22.19/flock remain runtime
prerequisites. Real model authentication/quality and useful delivery remain
unverified; the runtime is trusted same-UID code, not an OS sandbox.

## Evidence

Behavioral reds reproduced the inert Down key, unreachable catalog tail, Escape
closing an editor, absent Home/End content navigation, edit-Tab behavior and nested
picker return. The real-terminal check exposed a setup warning scrolling away on
reopen; its fixed-header regression also began red. Focused input/render checks now
cover these outcomes, empty results/providers, manual IDs, explicit save/switch,
and focused rows/control visibility at60x18,80x24 and120x40. TestBackend render
captures were visually reviewed; ordinary user judgment on fluidity remains needed.
Frontend library checks passed apart from a sandbox-blocked Unix socket-pair test,
which passed when rerun with socket permission. The explicit30-second latency
probe remains ignored. No backend/model execution code changed in this slice.

The remapped locked build and actual paired-binary/IPC/TUI fixture exercise F3
through terminal input, choose a model near the end of the SDK catalog, explicitly
save it, remove the old build and reopen with settings/draft/pause preserved.
No model calls occur. The CI/release fixture arguments now cover this route;
workflow edits remain local, not hosted CI evidence. Public download/update and
additional package channels remain incomplete.

## Next authorized action

The ignored `target/local-bin/waywr` test entry is refreshed to a temporary paired
bundle identifying itself as `v0.4.0-local.controls`, manifest SHA256
`1c352b161fc7126eb357653853538c0c89a9ac3cfa01fa208b0a244365e57c8c`.
Source, all three binaries and all19 bundle artifacts pass the public-content
check. Detach the currently open TUI before relaunching so its idle
service can hand over normally. In F3 use F for all models, type to filter, arrows
and Enter to choose, then S to save. Switch/context grants remain explicit.
The user verifies this in ordinary use; fix the next concrete interruption rather
than adding a model-specific route or another broad acceptance campaign.
No operator service was restarted and no account/model call was made by the agent.

Owners: [controls](tui-controls.md), [providers](provider-selection.md),
[product plan](usability-plan.md), [runtime direction](execution-adapter-decision.md),
[distribution](distribution.md), [verification](verify-usability.md).
