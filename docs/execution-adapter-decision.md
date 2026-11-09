# Shared provider/model layer

Current direction, 2026-11-09. The user corrected the earlier native-first
recommendation: broad, seamless model support is the requirement. Pi is an
architectural reference; its terminal app is not the application/runtime dependency. This
supersedes the [native-first decision](archive/provider-selection/2026-11-09-superseded-native-first-adapter.md).
Implementation is now in progress. The runtime choice below supersedes the
earlier unselected-dependency status; no Pi CLI is installed or invoked.

## Selected implementation

Use the independent `@earendil-works/pi-ai` and `pi-agent-core` libraries at
1.5.0, packaged as a Waywr-owned Node process behind the Rust service/runner.
The former supplies provider/catalog/protocol/auth support; the latter supplies
the tool loop. Waywr supplies task tools and normalized events/results. This
reuses the inspected maintained layers without adopting Pi's terminal app.
Node >=22.19 is a runtime prerequisite for this first implementation. Cargo.lock
and the Rust toolchain remain unchanged. npm dependencies are locally locked;
bundle resources include the compiled runtime and its notices, not node_modules.

## Product precept

Waywr coordinates projects and work through one shared provider/model layer.
Provider/model IDs are data, not coordinator branches or role allowlists. A
provider can expose many models. Adding a model on an implemented protocol must
require metadata/configuration or catalog refresh, not a core release. A genuinely
new protocol or authentication method can require an integration module; do not
promise that arbitrary future APIs work with no code.

Do not equate wrapping Codex/Claude executables with broad model support. Do not
preserve those hardcoded paths as a migration requirement: the user explicitly
waived compatibility with current integrations/profiles. A replacement may make
old configuration unsupported with a clear explanation. Preserve project files,
work/history and uncertain execution evidence; do not overwrite them to make
migration appear seamless. Their retention does not require keeping obsolete
execution routes functional.

## What Pi source actually does

The inspected upstream `main` source separates several responsibilities:

| Layer | Mechanism and lesson for Waywr |
| --- | --- |
| Provider registry | Providers own authentication, catalogs and model operations. Registration is separate from coordinator decisions. |
| Protocol implementations | Multiple providers share wire protocols; provider factories compose catalogs, endpoints, authentication and protocol implementations. Breadth does not require a native CLI per model or brand. |
| Model metadata | Generated catalogs supply defaults; refreshed/cached metadata and user configuration extend or override them. Catalog visibility and authenticated availability are separate. |
| Unified events | Provider output becomes common text/tool/usage events; UI code does not need a renderer for every model. |
| Agent loop | Tool execution and continuation sit above model streaming. Changing a model does not require a separate coordinator implementation. |

Primary source evidence: [provider registry and interfaces](https://github.com/earendil-works/pi/blob/main/packages/ai/src/models.ts),
[built-in composition](https://github.com/earendil-works/pi/blob/main/packages/ai/src/providers/all.ts),
[OpenRouter protocol composition](https://github.com/earendil-works/pi/blob/main/packages/ai/src/providers/openrouter.ts),
[catalog overlays/cache refresh](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/remote-catalog-provider.ts),
[coding-agent model runtime](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-runtime.ts)
and [independent agent loop](https://github.com/earendil-works/pi/blob/main/packages/agent/src/agent-loop.ts).
The [model-library documentation](https://github.com/earendil-works/pi/blob/main/packages/ai/README.md)
describes its separate package and normalized events. These are public-source
observations, not local execution, account access or a pinned runtime validation.

Pi still has provider-specific code and generated catalogs. Its extensibility
comes from keeping that knowledge below the agent/application layer, with shared
protocol implementations and configuration. Copying its displayed model list
would not supply transport, authentication or tool execution.

## Direction for implementation

Prefer a maintained multi-provider library/runtime behind a small process boundary
to rebuilding every vendor protocol in Waywr. Evaluate the provider/model layer
independently of a terminal application or full agent harness. The selected
implementation uses the independent provider and agent-loop libraries above.
The source informed that choice; no Pi CLI is installed or invoked.

The existing Rust TUI/service retains project coordination, durable tasks and
execution ownership. The shared layer supplies provider/model registration,
configuration/discovery, authentication integration, model-specific options,
normalized output, tools/session capabilities, cancellation and usage. A provider
SDK supplies model calls; if it lacks an agent/tool loop, that missing component
must be explicitly accounted for. Do not disguise a catalog-only SDK as a complete
coding runtime or accidentally start another custom harness project.

Expose searchable provider/model selection, known versus configured/available
status, metadata provenance, supported options and actionable setup errors. New
IDs must be configurable even when absent from a cached list. Unknown context,
usage or cost stays unknown; estimates are labelled. Do not equate context
headroom with account quota. Support capability differences without hardcoding
model-name role gates or silently substituting a destination.

A process interface may isolate language/package choices from Rust; it is a means
to the shared layer, not the feature being delivered. Keep project ownership,
structured result validation and live-work settlement, without treating old
provider-specific settings as permanent compatibility constraints. Existing
local-only Git, no-operator-install and no-authenticated-call instructions still
apply; waiving integration compatibility does not change those permissions.

## First implementation slice

**Outcome:** browse models grouped by provider, configure/select them through the
same UI, and execute a bounded development request with progress and reported
usage through a shared multi-provider path. Coordinator, implementation and review
use capabilities of that path rather than a fixed brand list.

**Current failure:** Rust contains provider enums, model/effort/default checks,
provider-specific form controls and native execution branches. A different model
name with default effort or an externally wrapped Codex command alone does not
resolve this failure.

**Acceptance:** before changing behavior, exercise one ordinary save/select/
dispatch journey with provider/model/option data absent from core. Complete that
journey through at least two provider identities using the shared runtime and a
representative tool-capable worker, with visible activity/usage and saved selection
on reopen. An isolated protocol/HTTP fixture can establish routing and event
behavior, but not inference, login, provider reliability or model quality. Real
model use remains separately authorized. Catalog breadth must correspond to an
implemented request path; a long list alone is not support.

The independent provider/tool libraries selected above now implement this slice.
The [provider contract](provider-selection.md) records the actual interfaces and
verification limits; ordinary account/model usefulness remains unverified. Do not
restart a broad utility survey or make another native-wrapper milestone. No
parallel legacy integration maintenance, compatibility shims, plugin marketplace
or model-specific coordinator branches. Lifecycle fluidity follows, then the
substantial web application across selected models, per the [product plan](usability-plan.md#narrowed-product-direction).
