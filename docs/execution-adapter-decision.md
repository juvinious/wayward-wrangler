# Shared provider/model layer

## Decision

Waywr uses the independent `@earendil-works/pi-ai` and `pi-agent-core` libraries
at1.5.0 behind a Waywr-owned Node process. The first supplies provider APIs,
model catalogs and authentication; the second supplies the tool loop. Waywr
supplies project coordination, task tools, normalized events and validated results.
Pi's terminal application is not installed or invoked.

The Rust TUI, project service, scheduler and runner remain responsible for durable
state and execution ownership. Node >=22.19 is a runtime prerequisite. npm
versions are locked; compiled runtime resources and license notices ship in the
application bundle, without node_modules.

## Why this design

A provider library can share wire protocols, authentication and model metadata
across many models. Maintaining separate native CLI wrappers would duplicate that
work and keep model support tied to coordinator releases.

Provider/model IDs, catalogs, defaults and native options therefore stay below the
coordinator. New models on supported protocols are data/configuration changes;
a new protocol or authentication method may need integration code. Core routing
uses required capabilities rather than model-name allowlists. Credentials remain
outside project state, and selecting a destination does not itself grant context.

The shared path replaces the hardcoded native integrations directly. Obsolete
settings are handled by [startup renewal](project-layout.md), with source/worktrees
retained and old calls/approvals excluded from fresh execution. Parallel legacy
provider routes are not maintained.

## Reference architecture

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

## Consequences

The SDK owns provider-specific catalogs/protocols and Waywr owns the project
experience. Catalog entries are not proof of authentication, model quality or
support for every role. Unknown usage/cost stays unknown; context estimates do
not represent account quota. Endpoints and model-specific options are editable
without adding coordinator branches or silently substituting a destination.

The process boundary isolates JavaScript dependencies from Rust; it does not
create an OS sandbox. Tool execution uses the trusted same-UID boundary described
in [provider selection](provider-selection.md). The old native-first proposal is
[archived](archive/provider-selection/2026-11-09-superseded-native-first-adapter.md).
The [roadmap](roadmap.md) records the product priorities.
