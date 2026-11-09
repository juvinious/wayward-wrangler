# Waywr handoff — extensible execution direction

Work remains on `plan/usability-and-installation`, with local commits only.
Follow [AGENTS.md](../AGENTS.md), the [development approach](development-approach.md)
and the [narrowed product direction](usability-plan.md#narrowed-product-direction).
The user sets this order: extensible model/execution support, lifecycle fluidity,
then coordination honed through a substantial web application.

## Current decision and next action

Waywr owns project coordination and the Rust TUI/service experience. Provider/model
catalogs and options should leave coordinator code. An execution-adapter boundary
is the recommended design: existing native clients or a shared third-party harness
can implement it. Pi was a comparison; adoption was not requested. No third-party
harness, plugin protocol or runtime replacement has been selected or implemented.

Next action is a bounded adapter decision using current paths and one
shared-harness candidate: model discovery/manual IDs, provider options, streamed
activity/usage, cancellation, sessions, structured results and existing execution
boundaries. Start with public protocol/source inspection. Record the smallest
workable choice; do not build a general plugin SDK or survey every provider.

The first implementation slice must demonstrate configuring/selecting a supplied
adapter and a model/option absent from core source through the normal dispatch
path, without rebuilding Waywr. Establish behavioral red first. Preserve existing
profiles and retained attempts. A protocol fixture proves extensibility only;
ordinary model quality/authentication/usefulness needs separately authorized use.
Thereafter address detach/reattach/update continuity, then the reference web app
in running increments across selected model profiles. Model-independent interfaces
do not guarantee equal model quality. No new provider-specific coordinator forks.

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
checkpoint changes planning/policy docs only, not application behavior.

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
