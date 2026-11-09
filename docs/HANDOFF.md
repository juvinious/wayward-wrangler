# Waywr handoff — adapter decision ready for implementation

Work remains on `plan/usability-and-installation`, with local commits only.
Follow [AGENTS.md](../AGENTS.md), the [development approach](development-approach.md)
and the [narrowed product direction](usability-plan.md#narrowed-product-direction).
The user sets this order: extensible model/execution support, lifecycle fluidity,
then coordination honed through a substantial web application.

## Current decision and next action

Waywr owns project coordination and the Rust TUI/service experience. Provider/model
catalogs and options should leave coordinator code. The completed
[adapter decision](execution-adapter-decision.md) chooses a small external process
interface and an extracted Codex-native bridge first. Preserve Claude/Ollama paths
during migration. No third-party harness is adopted; Pi remains an optional adapter
candidate. Native-first preserves existing logins/control paths but does not itself
add providers or eliminate adapter maintenance.

Current native source and public Pi documentation/types were compared for model
configuration, events/usage, authentication, structured results, cancellation,
sessions and execution boundaries. Pi offers broad metadata and session support;
matching Waywr result/isolation/ownership requirements needs bridge work. This is
source evidence only: no utility installation, credential access or inference.
The decision records its sources and limits. No further utility survey is needed.

Next action is behavioral red followed by the complete
[first implementation slice](execution-adapter-decision.md#first-implementation-slice):
register/select a supplied adapter and model/option absent from core, save through
the ordinary profile command path, dispatch through the real service/runner, show
activity/usage and reopen with the selection intact. Then deliver the native Codex
bridge through that same path, not a fixture-only endpoint. Finalize JSON field
names with the red example; do not start another planning phase or general SDK.

Use two operations: local bounded description and one frozen request on stdin,
with normalized JSONL events/terminal outcome on stdout. Reuse runner process-group
cancellation, positive settlement, validators and receipts. Adapter metadata cannot
grant execution authority. Freeze bridge/resources and underlying client per
attempt; new attempts may resolve updates. Preserve old profiles/packets. A fixture
proves extensibility only; native usefulness requires separately authorized use.
Lifecycle fluidity and the reference web app follow. Model-independent interfaces
do not guarantee equal model quality; no provider-specific coordinator forks.

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
