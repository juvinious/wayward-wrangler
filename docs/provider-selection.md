# Project providers and models

State9 / conversation config6 / IPC1.10 now uses schema3 project preferences with
one shared model runtime. Existing schema1/2 native profiles are retired and need
explicit reconfiguration. Their files, work and history are not rewritten or
removed. The [old native contract](archive/provider-selection/2026-11-09-retired-native-provider-contract.md)
is evidence only. The [product direction](execution-adapter-decision.md) owns the
breadth requirement; no native compatibility route is maintained for this rewrite.

## Runtime and model selection

The Rust TUI, project service, scheduler and runner remain Waywr-owned. A bundled
Node process uses locked `@earendil-works/pi-ai` and `pi-agent-core` 1.5.0 libraries
for provider APIs/catalogs/authentication and the tool loop. Pi's terminal app/CLI
is neither installed nor invoked. Node >=22.19 and Linux util-linux `flock` are
runtime prerequisites; Rust/Python/npm are not needed by a built bundle.

The inspected bundled catalog currently contains42 providers and1592 chat models.
This is metadata coverage, not native verification of every provider/model. Provider
and model IDs are opaque strings to core routing. Manual IDs on a registered
provider use its protocol implementation; custom providers/models can be added
through configuration without rebuilding Rust. A new protocol can require a
runtime update. Catalog entries and tool capability do not establish model quality.

F3, section1 Profiles:

- F searches provider/model/name; Enter selects the first match. Enter a precise
  query to choose a particular model. V cycles providers; C cycles their models.
- Tab chooses provider, model, options JSON, Node binary, transport JSON or optional
  soft context-byte target. Enter edits/accepts. Provider/model IDs remain manually
  editable. Options are SDK options, not an invented universal effort scale.
- S saves a profile revision; L reloads. R refreshes local runtime/catalog/config
  metadata. It performs no inference, login, download or authenticated catalog call.
- `+` duplicates a profile. Sections2–6 retain role defaults/allowed profiles,
  fallback, coordinator switching, unreserved task overrides and policy amendments.
  A new provider/destination needs the normal saved scope/policy/context grant;
  routine attempts under that policy need no repeated approval.

Catalog inspection reports authentication as not checked. It does not read the
private credential store. Known catalog models must not be advertised as tested
or authenticated. Use `waywr models list --search QUERY` outside a project as well.

## Configuration and authentication

`WAYWR_MODELS_CONFIG` selects an absolute model configuration file. Otherwise a
new project uses the existing `$XDG_CONFIG_HOME/waywr/models.json`, or
`~/.config/waywr/models.json`. The chosen path is saved in the profile transport. Create it before initialization,
or set transport.config in F3 and save when adding a new file later.
See the [configuration example](../model-runtime/models.example.json). Providers
specify an ID, API protocol, endpoint, credential environment-variable name and
models. Built-in provider clients already come from the SDK; do not duplicate
those clients in the coordinator. Custom endpoints currently support the shared
OpenAI Completions/Responses, Anthropic Messages and Google Generative AI protocols.

Supported SDK credential variables from the calling terminal are forwarded to the
project service/runtime by name, never recorded in packets. Explicit
`waywr models login PROVIDER --method oauth` or `--method apiKey` uses the SDK's
provider-owned flow and a private Waywr `auth.json` under XDG config. It does not
import or modify Codex/Claude/Pi credentials. API-key input is hidden on a TTY;
writes use mode0600, atomic replacement and a cross-process lock to preserve
concurrent provider updates. Actual account login/token refresh is not verified
by the fixture checks. Providers needing additional ambient environment setup
retain their SDK requirements; catalog presence is not authentication proof.

Never place actual keys/tokens in model options or tracked configuration. Runtime
options cannot override credentials, headers, endpoint, signal or process identity.
Credentials remain outside project preferences, packets and readable projections.

## Execution, results and usage

Each operation goes through the same process interface: one frozen JSON request,
normalized JSONL text/tool/usage events and one terminal result. The library owns
the model/tool loop. Waywr supplies task read/write/edit/shell tools and a typed
`submit_result` tool using the existing result schema. Coordinator/review operations
submit typed decisions/evidence without arbitrary filesystem tools; workers receive
the assigned checkout. Role restrictions do not depend on provider/model names.

This is a trusted same-UID runtime, not an OS sandbox. The former CLI sandbox
flags are not applied or claimed. Worktree ownership, no remote publication,
project scope and result validation remain application contracts; model instructions
and cwd do not create a filesystem/network security boundary. A stronger portable
execution sandbox is not delivered by this slice.

New attempts resolve the current Node executable and bundled runtime. The saved
`bundled` locator does not pin a project to the release that initialized it. Each
attempt freezes resolved executable/runtime hashes and model configuration bytes.
Private operation snapshots prevent a later config edit from changing a launched call.
Retained shared-runtime attempts declare shared-models-v1; normal opening/maintenance
refuses an application that cannot read that execution contract.
Prepared attempts refuse changed artifacts; new attempts can use ordinary updates.
Selected custom provider route metadata is part of the saved grant. Changing its
endpoint/protocol/auth locator requires saving/reviewing the changed profile; it
cannot silently redirect existing approved work. Merely adding model metadata does
not change that route. Filesystem/tool effects are adopted only through the existing
host settlement, scope, exact commit/review and receipt checks.

Activity and usage are carried into existing worker views and retained evidence.
Token totals are cumulative per attempt; repeated UI refreshes do not add them.
Context remaining is a labelled estimate when capacity/usage exist. Missing usage,
cost or capacity remains unknown. SDK cost may be estimated from catalog prices;
no subscription quota or billed-account balance is inferred. The provider's canonical
model identity is not independently attested; records distinguish the selected SDK
route from a provider-reported identity. Hidden reasoning is not projected.

Lost/ambiguous post-send completion remains uncertain even after local process exit.
Cancellation fences adoption and settles owned local processes; it does not prove
remote generation completed. Do not automatically resend an uncertain operation.

## Verification boundary

The initial red rejected runtime options/new provider selection through normal
project initialization. Focused checks now cover saved selection/reopen, search,
unknown provider/model IDs, frozen artifacts/destinations and real runner decoding.
Real SDK/agent-loop checks against local HTTP fixtures cover two distinct protocols,
a worker file-writing tool, structured completion and usage/progress. Host permits
in that runner check are fixtures, not a systemd proof or authority for native use.
Dummy API-key setup checks preserve two concurrent entries in a temporary private
store without contacting accounts. Actual installed TUI/reopening is checked in
isolated projects separately. No real inference, provider account or ordinary
unassisted development delivery is established by these checks.
