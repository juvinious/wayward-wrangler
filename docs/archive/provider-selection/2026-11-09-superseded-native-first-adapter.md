> Superseded by the user: model/provider breadth is the target; preserving native
> integrations and a Codex-first wrapper are not the implementation priority.
> This record supplies historical rationale, not current instructions.

# Execution adapter decision

Decision checkpoint, 2026-11-09, against local source `5413780`. This is a design
decision, not implemented support or native acceptance evidence. It completes the
bounded comparison in the [product plan](../../usability-plan.md#narrowed-product-direction).

## Decision

Use a small external process-adapter interface. Deliver a Codex-native adapter
first, extracting the existing transport rather than inventing another agent loop.
Keep existing Claude/Ollama paths operational during migration. Provider/model
metadata and native options belong to the adapter package/configuration; project
preferences select them. The coordinator and TUI consume generic capabilities
and events. New models accepted by that adapter must not require rebuilding Waywr.

Do not adopt Pi as a mandatory runtime now. It remains a credible optional adapter
after the boundary works. This choice preserves existing client login behavior
and current execution controls while exposing the extension point. It does not
deliver more providers by itself or eliminate provider maintenance: that work
moves to adapters, and a shared-harness adapter could consolidate it later.

## What the current source establishes

| Source owner | Existing behavior and extraction pressure |
| --- | --- |
| `backend/src/provider_selection.rs`: `Preferences::parse`, `efforts`, `schema`, `discover_with` | Fixed provider enum, model-specific effort tables, curated catalog and initial Codex/model check. Manual unknown native IDs already work with default effort, so that alone is not a new extensibility demonstration. |
| `backend/src/development.rs`: `prepare_native`, `inspect_native`, `finish_native` | Native argv, role/tool rules, structured schemas, provider branches and completion decoding live beside workflow logic. Reuse schemas and validators; extract transport and metadata. |
| `backend/src/development_claude.rs` | Existing Claude schema output, native tool/sandbox settings, init/model validation and usage/cost extraction. Preserve during incremental migration. |
| `backend/src/runner.rs`: `execute`, `group_settled` | One attempt/process group, finite stdin, concurrent stdout/stderr draining, cancellation, exit/descendant settlement and receipts. Reuse this host; a detached TUI must not become its owner. |
| `backend/src/operation.rs`, `provider_selection::packet_authorized` | Frozen operation/client identity and policy checks. Adapter extraction must not reinterpret old packets or turn metadata into execution authority. |
| `frontend/src/prototype/provider_view.rs`, `development.rs` usage projection | Provider-specific form controls; usage exists in runtime evidence but lacks one coherent normalized live view. Do not claim usage collection is absent. |

These observations are source inspection. Prior native help and isolated lifecycle
checks remain evidence only for their recorded boundaries.

## One shared-harness candidate: Pi

Reviewed public upstream `main` documentation/types on the date above. The source
[package manifest](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/package.json)
reports 1.5.0; this is not an installed/released-version check or
a pinned runtime evaluation. No installation, login, credential read or model call
was performed. The table distinguishes documented capability from integration work.

| Requirement | Current native paths | Pi assessment |
| --- | --- | --- |
| New models/options without core releases | Some manual IDs, but efforts/defaults/provider routing are compiled in | User model configuration and provider extensions can supply IDs/options. Strong candidate for delegating catalogs. |
| Text/tool progress and usage | JSON events and some final usage/cost are captured; projection is fragmented | Shared JSON/RPC events include text/tool lifecycle and reported usage. Context estimates and cost totals are queryable. |
| Existing authentication | Uses selected native CLI's existing setup | Own OAuth/API-key configuration. Do not assume native CLI accounts transfer automatically. |
| Structured coordinator/review results | Native schema arguments plus application result/evidence validation | RPC prompt types have no result-schema field. A bridge/extension or strict final-text parser is needed; capability and reliability must be demonstrated. |
| Cancellation and completion | Runner signals its owned group and checks actual exit/descendants | RPC abort/queue and settled events exist. Their acknowledgments still need mapping to Waywr host settlement. |
| Session continuity | Waywr stores context; current native attempts are ephemeral | Native session persistence/resume exists. It does not transfer Waywr task ownership or supply exactly-once dispatch. |
| Task filesystem/network boundaries | Provider-specific restrictions already supplied | Pi explicitly relies on OS/container boundaries for isolation. Matching existing worker controls needs additional integration. |
| Integration size | Extract one existing per-attempt transport | RPC needs bidirectional process I/O; JSON one-shot mode is closer to the current runner but still needs result/control mapping. |

Evidence: [model configuration](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/models-configuration.md)
and [extensions](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md)
support the catalog/extension assessment. [JSON events](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)
document streaming; [RPC commands](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc-commands.md)
document usage, queues, abort and sessions. [RPC types](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/modes/rpc/rpc-types.ts)
define prompt fields. [Provider setup](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md),
[security boundaries](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md)
and [CLI modes](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md)
support the authentication/isolation/transport observations.

Inference: Pi can reduce model/provider work, but adopting it today also adds
result, isolation, authentication and lifecycle work before improving the user's
normal path. The native-first process interface is the smaller complete increment.
Tool selection is not OS isolation; this is a compatibility requirement, not a
reason to add repetitive human approval. No broader utility survey is needed.

## Smallest interface to implement

Two process operations are sufficient for the first version:

1. **Describe:** bounded, local metadata containing protocol version, adapter ID,
   supported execution capabilities, optional model catalog with provenance, and
   provider-specific option descriptors/defaults. No login, inference, downloads,
   server startup or implicit network refresh. An empty catalog is valid; manual
   IDs stay available. Missing data is unknown, not unsupported by inference.
2. **Run:** one frozen request on stdin, then EOF; LF-delimited JSON events on
   stdout and bounded diagnostics on stderr. Events cover text, activity/tools,
   usage and one terminal completion carrying the proposed structured result or
   classified failure. Adapter translates native protocol; core validates outcome.

The host launches the selected executable directly with argv, never shell command
interpolation. Register explicitly installed user adapters; do not auto-execute
project-supplied scripts. Executable adapters are trusted code, not a sandbox.
The first adapter should be a standalone Rust executable built from the existing
transport, with resource metadata; no new Rust dynamic-library ABI or Python
application runtime. A supplied protocol fixture demonstrates that arbitrary
adapter IDs work without a compiled provider-enum entry.

The request contains adapter/protocol identity, project/attempt identity, selected
provider/model IDs, options, instructions/context, result schema and the existing
execution grant. Host control fields (paths, network/tool grants and executable
selection) cannot be overridden by model options. Validate options through
adapter descriptors/probing; do not insert model or effort enums into core.
Core capabilities describe operations such as structured decisions, read-only
review and worktree edits; they do not encode provider-brand eligibility.

Freeze the adapter executable/resources, underlying resolved client, effective
configuration and approved destination for the attempt. New attempts resolve
normal updated installations. Retained attempts keep old semantics; extraction
must not re-freeze, redirect or replay them. A mutable adapter command that later
resolves an unrecorded client is insufficient. Generic capability claims do not
grant authority or prove enforcement; the native adapter must retain its actual
tool/sandbox checks and unsupported boundaries must be reported before launch.

Cancellation initially uses the runner's existing owned-process-group signals.
The bridge keeps children in that supervised boundary, drains pipes and waits;
it cannot daemonize work and report success. A terminal protocol record alone is
not settlement. Process exit alone is not task success. Existing result/schema,
scope, commit/review and host settlement checks continue to govern adoption.
Missing completion after dispatch retains uncertainty; no automatic resend.

Keep Waywr conversation/task state authoritative. Adapter session handles are
optional private execution metadata, scoped to the same project/attempt and
explicit destination grant. Durable session resume is not promised by version1;
neither a saved session ID nor a new TUI can authorize replay. RPC/interactive
steering can be added only when the corresponding user journey needs it.

Usage events distinguish per-response cumulative readings from increments and
identify their source so refreshes cannot double-count. Report input/output/cache
tokens, cost/currency and context used/capacity when known. Preserve unknowns and
estimates; cost from catalog pricing is not a billed-account balance. Do not
persist credentials or hidden reasoning to populate a status display.

## First implementation slice

**Outcome:** select a registered adapter, enter a model and native option absent
from Waywr core, save the project profile and run an approved bounded task through
the ordinary service/runner path, seeing activity and truthful usage. Reopening
preserves the selection. Existing native profiles remain usable.

**Present failure:** profile validation only accepts three providers, unknown
models only accept default effort, and preparation/event/result handling branches
on provider names. F3 cannot select arbitrary adapters/options. Ordinary manual
model entry with default effort is existing behavior, not the red example.

**Acceptance:** first use one isolated protocol adapter with a novel adapter ID,
model ID and declared option. Save/select it through public commands used by F3,
dispatch through the actual service/runner, observe streamed activity/usage and a
validated result, then reopen the same project. It must fail on current behavior
before production changes. Deliver the real Codex bridge through that same path,
including profile selection and visible status, rather than stop at the fixture.
The fixture cannot establish authentication, native sandbox behavior or model
quality; real inference remains separately authorized ordinary use.

Use focused fault cases for unsupported capability/option before native launch,
malformed or missing completion, cancellation, changed frozen artifacts and
retained-profile compatibility. Reuse existing evidence instead of rerunning a
broad suite. Finalize exact JSON field names with the red example; the two
operations and ownership split above are the decision, not another planning phase.

Implementation owners: extract one native transport from `development.rs`; add
adapter registration/description and normalized runner decoding; extend revisioned
preferences/routing and the existing provider form; carry activity/usage through
the existing IPC projection; package the native bridge with notices/privacy checks.
Do not fork the scheduler, introduce a second daemon, upgrade dependencies,
install Pi, replace remaining native paths, add remote execution or build a plugin
marketplace. Installer/lifecycle and reference-application phases stay next in order.
