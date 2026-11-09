# Waywr handoff — broad model support, no legacy integration requirement

Work remains on `plan/usability-and-installation`, with local commits only.
Follow [AGENTS.md](../AGENTS.md), the [development approach](development-approach.md)
and the [narrowed product direction](usability-plan.md#narrowed-product-direction).
The user sets this order: extensible model/execution support, lifecycle fluidity,
then coordination honed through a substantial web application.

## Current decision and next action

The user corrected the prior decision twice: existing hardcoded integrations need
not remain usable, and Pi is a reference for breadth/seamlessness, not an adoption
target. Do not return to a Codex-first wrapper or parallel legacy-route maintenance.
The [shared model-layer direction](execution-adapter-decision.md) supersedes that
recommendation; its original rationale is selectively archived there.

Pi's public source separates provider authentication/catalogs, shared API protocol
implementations, refreshable metadata, normalized events and an independent agent
loop. This is the relevant pattern. Prefer a maintained multi-provider/tool runtime
behind one small boundary beneath the Rust TUI/service. No third-party library or
harness has been selected or installed. Source inspection made no authenticated
call and read no credentials. Do not mistake a model list or provider SDK alone for
complete coding/tool execution.

Next action: select the concrete maintained provider/tool runtime from this bounded
requirement, record the dependency/transport choice, and implement the
[first slice](execution-adapter-decision.md#first-implementation-slice). Start with
behavioral red for provider/model/option data absent from core through ordinary
save/select/dispatch. Deliver the same path across at least two provider identities,
including a tool-capable worker, visible activity/usage and persisted selection.
No further open-ended comparison, generic SDK or native-only wrapper milestone.

Existing integration/profile compatibility is explicitly waived. Unsupported old
configuration may require reconfiguration; keep project work/history and uncertain
execution evidence intact. Do not auto-redirect or replay old work. Normal execution
ownership and result validation remain app responsibilities. A protocol fixture
proves its boundary, not authentication/inference/model quality. Lifecycle fluidity
and the substantial web app follow; no model-specific coordinator branches. This
product precept now lives in AGENTS so subsequent tasks must retain the breadth.

## Implemented baseline and limits

At `7f68ee8`, compatible stopped projects reopen without their old application;
an exactly owned, positively idle service can hand over; native versions are
diagnostic and new attempts freeze the resolved client; one-command local
build/install and `--version` work. These have focused isolated acceptance evidence.
The [prior checkpoint](archive/handoff/2026-11-09-local-iteration-before-execution-direction.md)
retains verification details, including package/source isolation and the exact
synthetic-ID secret-scan exception. No broad rerun is needed for this planning work.

Current source still couples provider names, role eligibility, model-specific
efforts and curated defaults to Rust. Unknown native model IDs already work in
some paths with provider-default effort; this is not a general extension boundary.
The [provider contract](provider-selection.md) records actual restrictions. Native
Codex/Claude execute coding workers; Ollama is conversation/coordination/review;
other discovered clients are not implemented coding adapters.

Ordinary unassisted task delivery and subjective fluidity remain unestablished
for this candidate. Active/uncertain work cannot be force-replaced; native-client
help probes do not establish inference or sandbox reliability. Public download,
update/uninstall, artifact collection and Debian packaging are incomplete. The
user's new execution direction precedes more installer implementation. This
checkpoint changes design/planning/policy docs only, not application behavior.
Local reference and diff checks are appropriate; no application test rerun is
needed until implementing the slice.

## Authority

Routine Git inspection is read-only; owned local commits are permitted. The user
performs all remote delivery: no pushes, PR changes/merges, publication or hosted
settings changes. Preserve unrelated work/history. No paid feature was enabled.

Operator installation, preserved-project changes, credentials/provider settings
and authenticated model calls require explicit task authorization. Prior cleanup
removed operator installations; none was reinstalled. Use isolated projects for
authorized runtime checks and clean their owned services/artifacts. Development
rules must not become arbitrary product-agent restrictions; preserve justified
project/data/execution boundaries while reducing repeated interaction.

Contracts: [distribution](distribution.md), [providers](provider-selection.md),
[layout](project-layout.md), [IPC](contracts/session-protocol.md),
[workflow](ordinary-project-workflow.md). [User verification](verify-usability.md)
remains available for the implemented baseline; it does not authorize a new run.
