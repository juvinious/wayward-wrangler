# Waywr handoff — shared provider/model execution implemented locally

Branch: `plan/usability-and-installation`. Follow [AGENTS.md](../AGENTS.md) and the
[product plan](usability-plan.md). Broad model support remains a project precept;
next phases are lifecycle fluidity and coordination through a substantial web app.
Local commits only. The user performs every remote delivery; no push/PR/merge,
publication or hosted settings change is authorized.

## Implemented

The Rust TUI/service/scheduler/runner now uses one shared process interface backed
by locked standalone pi-ai/pi-agent-core 1.5.0 libraries. This uses provider and
agent-loop libraries, not Pi's terminal application. The runtime supplies42 provider
catalogs/1592 models, custom protocol/model configuration, SDK authentication,
normalized activity/text/usage and typed `submit_result` completion. Core routing
has no provider/model allowlist. SDK catalog coverage is not all-model verification.

F3 searches providers/models and edits IDs/options; normal profile save/selection
persists across reopen. `waywr models list --search QUERY` works outside a project;
`waywr models login PROVIDER --method oauth|apiKey` explicitly invokes SDK setup.
Private credentials use an atomic mode0600 store and a cross-process lock, outside
project packets. Caller SDK credential variables are forwarded as process
configuration, never logged into packets. No real login/inference was performed.

Preferences are schema3. Old schema1/2 native profiles are deliberately unsupported;
the user waived compatibility. Files/work/history remain. The standalone native
Claude/Ollama modules and their obsolete integration harnesses were removed.
Remaining older workflow readers are historical, not parallel state9 execution.

The saved `bundled` runtime locator avoids pinning projects to a release directory.
Each attempt freezes actual Node/runtime/config identity. Config bytes are copied
into its private operation before dispatch; custom destination metadata belongs to
the saved grant. Changes to endpoint/protocol/auth locator require a new saved
profile rather than silently redirecting approved work. Worker checkout identity,
owned process groups, positive settlement, exact result/review and receipts remain.
The generic tool runtime is trusted same-UID code, not an OS filesystem/network
sandbox; former native CLI sandbox flags are not claimed for it.

Bundles contain the compiled JavaScript runtime and npm notices, without
node_modules. Source builds install locked npm dependencies with lifecycle scripts
disabled, then bundle them; Cargo.lock/compiler/dependencies are unchanged. Node
>=22.19 and util-linux `flock` are runtime prerequisites. npm/Python/Rust are build
or verification tools. Public installer/update/uninstall remain incomplete.
Local workflow changes add pinned Node, npm advisory/Dependabot coverage and
JavaScript CodeQL; they have not run on GitHub or enabled any paid features.

## Evidence and limits

The first behavioral red rejected new runtime options/provider selection in normal
project opening. Selection/save/reopen now passes. Real SDK/agent-loop fixtures
exercise OpenAI Completions and Anthropic Messages, custom provider/model IDs,
a worker writing a file through a tool, structured results and usage/progress.
Real runner checks use linked Git worktrees and fixture host permits; these are
not model-quality, human-approval or systemd proofs. Dummy API-key setup preserves
two concurrent provider entries in a temporary private store; missing credentials
refuse before HTTP. Actual provider login/token refresh remains unverified.

Existing ordinary candidate/checkout/retirement and conversation checks were
reconciled with current profile fixtures. F3 search/save and CLI/build checks pass.
The installed TUI/reopen check preserves draft/settings/pause with the old bundle
removed and no rebind. Source and rebuilt artifact privacy checks pass after
removing a source-path fallback from the binaries. Production npm audit reports
zero known vulnerabilities at this checkpoint. The real archive installer/service/TUI check passes outside the source checkout
with Rust absent, zero source-file accesses and damaged-resource refusal. Its
GitHub transport is a fixture; it performs no model calls or systemd bootstrap.
The final shared-models marker/legacy-dispatch cleanup also passes its release
guards and the real installer/TUI check on the final review bundle. Its archive
SHA256 is `f24e4621877c1f150e4cac4f1a8a41cb82c0747659fda0f933202b7685b249b1`.
All operator/remote boundaries remain.

No operator installation, preserved project, provider credentials/settings or
shared infrastructure was changed. Runtime tests use temporary projects/HOME;
fixtures never authorize native calls. The earlier installation cleanup remains
in effect. No hosted binary release exists and no ordinary unassisted useful task
has been verified with this model runtime.

## Next authorized action

After the local checkpoint, obtain ordinary-use feedback when the user authorizes installation/account/model use.
Fix a concrete interruption within the lifecycle journey (detach/reattach/update
without losing work) rather than adding provider-specific branches or another
planning/test campaign. Keep future model IDs/options in the shared layer.

Owners: [providers](provider-selection.md), [runtime choice](execution-adapter-decision.md),
[distribution](distribution.md), [architecture](architecture.md),
[verification guide](verify-usability.md). Earlier native evidence remains in the
[archive](archive/README.md); it is not current runtime acceptance or authority.
