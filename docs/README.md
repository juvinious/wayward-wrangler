# Documentation

[Ordinary project workflow](ordinary-project-workflow.md) owns the implemented contract from approved `b810bf1`; HANDOFF records build and verification limits.

## Start here

1. [AGENTS.md](../AGENTS.md): working policy and preservation.
2. [HANDOFF](HANDOFF.md): current checkpoint, objective and next authorized action.
3. [Architecture](architecture.md): current source owners, implemented behavior and limits.
4. [Root README](../README.md): normal use, controls and source build/install instructions.

## Current product work

[Usability and installation plan](usability-plan.md#narrowed-product-direction)
sets the narrowed order: extensible model/execution support, lifecycle fluidity,
then coordination through a substantial web application. The corrected
[shared model-layer direction](execution-adapter-decision.md) targets broad support;
The standalone pi-ai/pi-agent-core libraries now implement that shared boundary;
Pi's terminal application is not used. Existing native profiles are retired.
A Codex-only wrapper remains insufficient for the product requirement.
[Acceptance-driven development](development-approach.md) defines the implementation
loop; AGENTS.md owns the policy. Work stays on `plan/usability-and-installation`
with local commits only; the user performs remote delivery. Stopped-project
reopening, real-systemd idle handover, provider updates and one-command local
build/install have focused acceptance evidence. The shared-runtime slice now has
two-protocol SDK/tool/runner checks and searchable persistent selection. A
startup fix opens retired schema2 projects with execution blocked and an unsaved
F3 replacement form; real isolated systemd bootstrap and idle handover pass.
Ordinary provider setup and useful model execution remain unverified. The next
phase is lifecycle fluidity informed by ordinary use. [User verification](verify-usability.md)
remains available; native task delivery and fluidity remain to be observed.
Public download, update/uninstall and additional package channels remain incomplete.

[Ordinary project workflow](ordinary-project-workflow.md), [providers](provider-selection.md)
and [distribution](distribution.md) own the connected increment. The
[TUI keyboard contract](tui-controls.md) owns focused navigation, fixed control
hints and the complete searchable provider/model picker. The
Thinking selector uses per-model runtime metadata; F3 and the main header show
the model/level, with draft and active bindings distinguished. Profiles now offers
A to review and explicitly activate a saved selection after the current turn,
preserving pause and existing work policies. The
main Send action now leads directly to setup when a retired binding blocks it;
unrelated request policies do not veto new discussions. Ctrl-C/Escape clears plain
composer input. Idle UI reads borrow large projections, task details refresh after
state changes, and history projection checks a small revision watermark. The
main conversation now shows current or latest subject-local coordinator-call
activity and reported usage, with labelled cost/context estimates and retained
completed-call metadata. The
idle-service handover also supports a direct temporary paired-bundle symlink,
with exact process/socket/project checks and preserved busy-owner refusals. The
complete snapshot reply budget now accommodates the catalog plus retained work;
oversized replies return a correlated error. Bounded-status handover can replace
an older service whose full snapshot cannot be delivered. The
[approved plan](archive/product-workflow/approved-plan-b810bf1.md) is archived evidence.
It builds on preserved recovery/convergence, not older main.
The [repository cleanup record](repository-cleanup-2026-10-18.md) records both repository-only cleanups, PR/tree proof, former tips and preserved
pre-squash archive refs. These docs authorize no
managed-project operation or release publication. [Readiness](../README.md#current-readiness)
remains experimental; no additional demonstration or acceptance program is prescribed.

[Current local build/verification record](ordinary-project-verification.md) separates
automated fixtures and hashes from native/GitHub behavior awaiting ordinary use.

[Public-source audit/remediation](security-audit-2026-11-08.md) records patched
dependencies, generalized public source/docs, MIT licensing, bundle notices and
privacy/security checks. The independent public repository has its own required
PR/build/security gates and free available security controls. The original history
remains private in a read-only archive. Source migration is complete; binary release
publication is separate.

## Maintained contracts — read when relevant

| Owner | Scope |
| --- | --- |
| [Conversation](conversational-coordination.md) | Request-bound discussion, exact inline approval, questions, steering and source identity |
| [Distribution](distribution.md) | Versioned private binary installer, real isolated package check, guarded update and GitHub packaging workflows |
| [Providers](provider-selection.md) | Project roles/preferences, frozen attempts, effort support, switching/fallback and current discovery limits |
| [Shared model-layer direction](execution-adapter-decision.md) | Source-backed provider/catalog/protocol separation, breadth requirement and superseded native-first recommendation; design only |
| [Recovery](coordinator-recovery.md) | Typed reconciliation, finding lineage, progress comparison, convergence and verification limits |
| [Context](context-delivery.md) | Compact working context, exact evidence retrieval/continuation and capacity diagnostics |
| [Layout](project-layout.md) | Canonical `.waywr` state, identity, migration and evidence preservation |
| [State/history](contracts/state-contract.md) | Durable records, versions, request history and rollback boundaries |
| [Execution](contracts/execution-contract.md) | Host ownership, receipts, Git integration and current retention behavior |
| [IPC](contracts/session-protocol.md) | Versioned commands, capabilities, delivery and bounded inspection |
| [TUI](contracts/tui-contract.md) | Production input, activity, candidate cards and exact validation handoff |
| [Linux setup](linux-sandbox-setup.md) | Actual systemd/sandbox requirements and temporary AppArmor workaround |

## Selective history

[Verified workflow checkpoint](archive/conversational-coordination/2026-10-17-verified-checkpoint.md)
records prior human-led deliveries and their assistance/verification limits.
[Recovery/convergence correction](archive/coordinator-recovery/2026-10-18-convergence-correction.md)
records current source/install evidence and the limited native observation.
[Previous handoff](archive/product-workflow/2026-10-18-prior-handoff-d1dda24.md)
retains the unresolved managed-project visual question without authorizing continuation.
[Archive index](archive/README.md) locates superseded plans and earlier evidence.

[State7 continuous interaction](contracts/continuous-coordinator-contract.md),
[state5](contracts/claude-coordinator-contract.md) and [state6](contracts/live-scope-contract.md)
retain compatibility semantics; their trial limits/permissions do not govern current
development. Historical plans and Example Task App commands are not a startup reading list.
