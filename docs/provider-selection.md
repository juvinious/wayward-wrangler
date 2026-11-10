# Project providers and models

State9 / conversation config6 / IPC1.10 now uses schema3 project preferences with
one shared model runtime. Existing schema1/2 native profiles are retired and need
explicit reconfiguration. Their files, work and history are not rewritten or
removed. The [old native contract](archive/provider-selection/2026-11-09-retired-native-provider-contract.md)
is evidence only. The [product direction](execution-adapter-decision.md) owns the
breadth requirement; no native compatibility route is maintained for this rewrite.

Schema2 settings can be read to open the project and inspect its history. They
cannot execute. F3 offers a fresh, unsaved schema3 draft from the shared catalog;
it does not translate old native choices or invoke their executables. Choose the
provider/model and thinking level, S save, then A reviews activation in Profiles.
Choose Activate and share project context and Enter to request the switch. Saving alone
does not replace the active binding. Existing request policies may require a
separate amendment in section6 Policy. Old preference revisions and bindings stay
as evidence; no prior approval grants a newly selected destination authority.

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

- F or `/` opens all models; V opens providers; C opens the current provider's
  models. Type/paste to filter, arrows or Tab/Shift-Tab select, page keys move a
  page, Home/End selects first/last, Enter chooses and Esc returns one level. The
  complete catalog is navigable; providers without catalog models remain visible.
- Up/Down or Tab/Shift-Tab chooses provider, model, options JSON, Node binary,
  transport JSON or optional soft context-byte target. Enter opens a picker or
  editor. E edits any exact ID/value, including models absent from the catalog.
  Enter accepts a text edit; Tab/Shift-Tab accepts and moves focus; Esc cancels
  the edit. Options are SDK options, not an invented universal effort scale.
- The Thinking field opens the selected model's SDK-advertised levels. These
  include only the levels that model reports; Rust does not enumerate them.
  The choice is saved as `options.reasoning`. Selecting another model keeps a
  compatible explicit level or selects that model's advertised default. Other
  model-specific options reset when changing models. F3 labels the draft and
  active model/level separately. The main view puts the saved coordinator choice
  first; any different active binding is separately labelled with its status.
  S saves preferences without silently changing the active binding.
- S saves a profile revision; L reloads. R refreshes local runtime/catalog/config
  metadata. It performs no inference, login, download or authenticated catalog call.
- A opens an activation review for the selected saved coordinator-allowed profile.
  Unsaved edits must be saved first. The review shows the exact model, thinking
  level, destination and options, and defaults to Keep current. Left/Right or
  Tab/Shift-Tab chooses the action; Enter confirms, Esc returns to Profiles.
  Activate and share project context explicitly grants the selected project context
  and requests the existing switch after the current turn. Changed preferences or
  binding generation invalidate the review. Activation preserves pause, worker
  assignments, existing request policies and uncertain-call fences. It does not
  sign in or amend an existing request's policy. Section4 retains switch status,
  cancellation and explicit interruption controls.
- `+` duplicates a profile. Sections2–6 retain role defaults/allowed profiles,
  fallback, coordinator switching, unreserved task overrides and policy amendments.
  A new provider/destination needs the normal saved scope/policy/context grant;
  routine attempts under that policy need no repeated approval.

Catalog inspection reports authentication as not checked. It does not read the
private credential store. Known catalog models must not be advertised as tested
or authenticated. Use `waywr models list --search QUERY` outside a project as well.

## Configuration and authentication

The runtime supplies supported thinking levels and its default through catalog
metadata. Old catalog caches refresh offline when the new service opens; saved
profiles and grants are not rewritten. Existing profiles without an explicit
level display `default (LEVEL)` when metadata is available. For manually entered
models with no capability metadata, levels remain unverified and exact options
stay editable. The display describes a requested setting, not provider attestation
of internal computation. Known unsupported levels refuse before authentication or
HTTP; they are not silently clamped. The agent loop receives the selected level
explicitly, and runtime events/receipts retain the requested level.

Custom model entries may set `reasoning: true` and `thinking_level_map` using
SDK level keys and native values; `null` marks an unsupported level. The runtime
derives choices/defaults with the SDK helpers, rather than a provider-name table.

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
The main conversation also shows its current coordinator conversation call, or
the most recent call with telemetry for the selected subject. It names that
call's selected provider/model and thinking level, separate from saved preferences.
Active calls refresh through a small read-only telemetry request once per second;
missing refreshes and disconnected views are labelled. Completed-call totals remain
visible after reopening. Starting a new call does not borrow the previous totals.
This summary covers conversation calls; autonomous workflow coordinator operations
are not yet included in it. Existing worker activity remains separate.
Token totals are cumulative per attempt; repeated UI refreshes do not add them.
Input/output and cache read/write counts stay separate. Context remaining uses
the last response's usage and catalog capacity, labelled as an estimate rather
than a prediction of the next prompt or subscription quota. Missing usage,
cost or capacity remains unknown. SDK cost may be estimated from catalog prices;
no subscription quota or billed-account balance is inferred. The provider's canonical
model identity is not independently attested; records distinguish the selected SDK
route from a provider-reported identity. Hidden reasoning is not projected.
Only bounded numeric usage and selected identity/activity fields enter this
projection. Raw prompts, tool output and extra runtime fields remain outside it.

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
