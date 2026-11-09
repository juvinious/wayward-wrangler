# Project providers and roles

State9 / conversation config6 provider selection uses the IPC1.7 capability;
current releases add the IPC1.8 [conversational overlay](conversational-coordination.md).
Codex `gpt-6.1-sol` / `medium` stays the initial coordinator and worker default.
Claude is an explicit profile choice for coordination, implementation and review.
Ollama supports ordinary conversation, coordination, read-only review and existing
application-mediated evidence continuation over loopback HTTP. Coding workers stay
Codex/Claude only. Gemini/OpenCode are discoverable clients with execution visibly
unsupported. No request, approval or inference is seeded by setup/discovery/install.

Discovery searches saved explicit paths, PATH and known user-local locations for all five clients, deduplicating canonical executables, with
bounded version checks. Ollama metadata reads `/api/version`, `/api/tags`, `/api/show`
through a proxy-free, redirect-free HTTP client; no services, pulls or inference start.
Codex cached model metadata is projected through an allowlist, without credentials; curated working models remain even if absent from cache. Catalog completeness, timestamps and provenance are recorded. Adapter support is separate from
model suitability; local model quality remains unverified until ordinary use.

## Production TUI

Normal chat also accepts exact configured profile choices such as `use Claude for review`
and `switch coordinator to Claude`. New destinations require a narrow saved grant;
existing permission is reused. F3 remains the full editor. See the
[conversation contract](conversational-coordination.md) for routing and grant identity.

F3 opens project-local forms; 1–7 selects the section. Left/Right selects a named
profile; Up/Down selects a profile, role or task. PageUp/PageDown scrolls the view.
S saves preferences with an exact revision guard; the accepted saved revision refreshes
the form automatically. L reloads the saved version and
discards the form draft. R refreshes local client/catalog metadata only. Discovery
never calls a model, signs in, starts a model server or downloads a model. Its
observations are not authentication, capacity or role-usability proof.

| Section | Controls and meaning |
| --- | --- |
| 1 Profiles | Tab selects model, absolute binary path, native effort token, optional working-set byte target or advanced HTTP transport options; Enter edits/accepts the field. `+` duplicates; V switches Codex/Claude in the draft; C cycles native catalog models and E their supported effort tokens. O cycles discovered compatible local Ollama models into named profiles for coordinator/review; choose role defaults explicitly in section2. Enter an exact native model ID when needed. Saving a changed binary captures its SHA256; changed binaries refuse launch until the binary field is re-entered and explicitly saved. |
| 2 Roles | Space toggles the selected profile in that role's allowed set; Enter sets its default. W sets the project worker default; I makes implementation/review inherit it. The role default must belong to its allowed set. |
| 3 Fallback | Space appends/removes a role's alternatives in order. Empty means capacity waiting. Alternatives must belong to the configured and approved role sets. |
| 4 Switch | Enter requests an after-current-turn handover to the saved selected profile. I selects explicit interrupt-and-switch. G explicitly grants selected project context to a genuinely new destination, when needed. C cancels a handover; an interruption must first positively settle, and its evidence remains retained. The header shows a pending switch. |
| 5 Tasks | Up/Down selects a child task in the selected request. P pins the selected profile; A toggles an allowed set; I inherits normal routing. Changes affect only unreserved attempts. Frozen selections remain visible. |
| 6 Policy | Review the current request policy and saved proposed routing/fallback; Enter approves an additive provider amendment. This preserves scope approval and accepted work; it does not expand project/task scope. New requests approve their complete saved scope/policy card in chat or `/plan`. |

Save a preference edit before requesting a switch or approving a policy amendment.
Those actions use saved profiles, not an unsaved form draft. Appearance controls
and the older version-specific conversation-only Settings remain compatibility
behavior. Composer command recall and chat scrolling are unchanged.

## Preferences and authority

`.waywr/coordinator.json` is the version2 `development-profiles-v1` document. It
contains named exact profiles, coordinator/worker defaults, per-role defaults and
allowed sets, ordered fallback and existing concurrency/network/main policy.
It contains no credentials. SQLite's writer records revisions before atomic file
publication. Startup reconciles interrupted publication. Valid external edits import
as a new preference revision; conflicting edits during publication are preserved
and require reconciliation. They cannot grant authority or switch a binding.

Task pin/allowlist takes precedence over a coordinator's requested profile, then
role default, then worker default. Every selection must fit the saved operating
policy. Each attempt freezes profile ID, provider, model, native effort, binary
identity, preference revision, routing source/reason, policy and selected context in
its operation packet. Preference edits never rewrite reserved packets or receipts. Reservation guards retry
unlaunched preparation if a preference revision changes. Native permit/spawn checks
use frozen approved grants, including the exact revision1 Codex import for old scopes,
rather than comparing attempts with today's defaults.
A changed current coordinator profile takes effect through Switch. New destinations
need one explicit context/policy grant, not repeated approval for routine attempts.
Existing Codex-only approvals do not authorize Claude after an upgrade. An approved
provider amendment resumes a known policy block without clearing substantive questions.
Closed request policies remain historical. Profiles use the existing user CLI login;
per-profile account selection and remote endpoints are not offered.

Supported launch interfaces are Codex 0.162.1/0.163.0 and Claude Code 2.1.296/2.1.301/2.1.302. The 2.1.302 public help exposes every current adapter flag in the focused local metadata check; native execution of this newly allowed version is not established by help.
The existing project's exact Codex binary is retained on import. Curated effort
choices are provider/model-specific: `gpt-6.1-sol` accepts low, medium, high, xhigh,
max; the retained Claude `claude-sonnet-5-5` choice supports medium in this adapter's
catalog. `provider_default` omits the effort flag. A manually entered unknown model
requires `provider_default`; it remains unverified, and a different reported model
fails rather than being substituted. No universal effort scale or model alias
resolution is claimed. The optional context-byte field is a soft working-set target,
separate from model capacity and application memory/transport limits. Existing values
stay unchanged; ordinary use needs no tuning. [Context delivery](context-delivery.md)
owns compact packets, exact evidence retrieval, estimates and headroom diagnostics.

Capability sources: [OpenAI model reference](https://developers.openai.com/api/docs/models/gpt-6.1-sol),
[Claude CLI](https://code.claude.com/docs/en/cli-reference),
[Claude permissions](https://code.claude.com/docs/en/permissions) and
[Claude sandbox](https://code.claude.com/docs/en/sandboxing), plus local version/help
observations recorded at planning commit `38ce3a1`. Authentication remains the user's
existing CLI login. Shared settings, aliases and credential contents are not imported.

## Execution and handover

Both coordinator lanes share one generation-guarded binding. After-turn switching
fences new coordinator reservations and adopts existing settled outcomes once;
workers, checks and serialized Git effects continue independently. Explicit
interruption fences old actions before cancellation, requires positive settlement
and retains the interrupted evidence. Uncertain execution blocks replacement. A recorded preparation refusal proves that
no model process started; correcting configuration or explicitly switching can recover
that block. Authentication/configuration errors never trigger provider fallback.
Unreserved queued input is rebound in order; conversation, decisions, questions,
worker worktrees, approvals and integration state stay in Waywr.
Replacement rechecks current request policies before applying. An explicit change to
a different profile releases the old profile's capacity delay; switching to the same
profile retains its delay. Uncertain work remains blocked in either case.

Implementation workers use their existing isolated task worktrees and native tool
permissions. Claude uses explicit role tool rules and a fail-closed Bash sandbox,
without an unconditional permission bypass. Its shell sandbox needs Linux bubblewrap
and socat; managed restrictions or unavailable sandbox support remain visible failures.
Native clients/worktrees are still a trusted-client, same-UID boundary.

Review units receive exact target commits/bases, indexed diffs, contracts and checks.
The context-delivery path requires complete target diff delivery, using pages when
needed; indexed source is retrieved read-only at exact commits.
They return exact reviewed commits and findings, with no fabricated commit. Stale
review evidence cannot become a valid review outcome. Corrections use implementation
workers; the coordinator still decides integration and actual combined checks gate
delivery. Required context that exceeds application/model bounds is refused before
launch; the working preference alone is not a hard ceiling. Inspect retains full
records. Full target diff delivery is not exhaustive source-review proof.

Fallback is disabled by default. Only an explicit, positively settled provider
capacity refusal can select the next configured **and approved** alternative.
Worker continuation checks retained partial-work fingerprints; coordinator fallback
uses the same binding handover. Authentication/configuration/policy failures and
uncertain execution never authorize substitution.

## Upgrade and verification boundary

New projects initialize at state9. Existing release bindings remain unchanged until
normal `waywr --project DIR session upgrade --release ID` maintenance. A quiescent state8
upgrade journals the old profile/config/contract, imports Codex as the sole allowed
profile, advances the DB/contract version and preserves identity, approval/task rows,
Git worktrees, receipts and readable history. Interrupted publication is recoverable
through maintenance; unknown conflicts refuse overwrite. Older states3–7 keep their
contracts. Old binaries cannot mutate state9; an older release cannot roll state9 back
to state8. No active UI, pending delivery or uncertain host attempt may cross upgrade.

Focused fixtures cover routing, policy, save/recovery, streams and handover separately
from live acceptance. The human-led label-management-example request now provides one assisted
Codex coordinator/implementation and Claude review delivery, including Claude's
exact-source retrieval/continuation. The user also verified a post-delivery recap.
[Recorded taskings/evidence](archive/conversational-coordination/2026-10-17-verified-checkpoint.md)
identify the actual path and recovery assistance. Claude coordination/implementation,
other profiles and the complete switch/fallback/interruption matrix are not live-verified
by that request. Provider/sandbox portability and unassisted reliability remain gaps.
Each new request, participating profiles and scope/policy approval belong to the user.

## Request-specific choices

A clear pre-approval directive such as `Use Claude as the reviewer for this work,
with Codex coordinating and implementing` saves source-bound request role pins from
configured allowed profiles. This leaves project defaults and the broader permission
set intact. Cards distinguish fixed request roles from defaults; the exact scope
approval freezes these pins in the operating policy. Coordinator pins require the
matching current binding; a different coordinator uses the existing safe handover.
Unknown/ambiguous choices need F3 or clarification, never silent substitution.

Task/coordinator routing and the runner permit cannot bypass a request pin. Capacity
fallback cannot substitute another profile. An explicitly selected reviewer must
review the exact submitted commits, with blocking findings resolved, before those
commits integrate or deliver. Future preference edits cannot weaken this constraint.
Active task overrides keep their existing unreserved-attempt guards; chat does not
rewrite a running attempt or an already approved request policy.

Explicit stopped-project `session reconcile-request-routing --request ID --message ID`
can save a choice from one retained human message in its own request. It neither
creates a new conversation turn nor authorizes work; original proposal revisions
and messages remain intact. The refreshed policy needs its own presentation and
actual human approval. Releases retaining these records require bundle
`request_routing_contract: pinned-roles-v1`.

## Ollama transport boundary

An Ollama profile freezes canonical loopback HTTP origin, server version, local model
name/digest, advertised capability/context/thinking metadata and native options.
Unknown context capacity is labelled unknown; estimated prompt/schema/instruction and
response headroom are compared with explicit num_ctx, independently of memory bounds
and soft byte preferences. No token/byte equivalence or silent truncation is assumed.
Only provider_default effort is supported; it is not mapped to a native thinking level.
Remote/cloud aliases, redirects, proxies and native coordinator/reviewer tool calls
are refused. Profiles apply to future attempts; old Codex/Claude encodings remain valid.

The existing supervised runner owns the HTTP child, send intent, public stream, strict
result adoption and receipts. Thinking fields are discarded before persistence. A lost
post-send response without terminal completion remains uncertain, even after client
exit. Explicit cancellation can abandon only a positively settled client with no
adoptable action; server completion is still reported unknown. Capacity refusals wait
under existing policy. Evidence requests use the same exact read-only grants/pages,
not arbitrary model filesystem access. Fixture checks do not establish native quality,
latency, autonomous reliability or every server/model combination.

API provenance: [chat](https://docs.ollama.com/api/chat),
[model catalog](https://docs.ollama.com/api/tags),
[model details](https://docs.ollama.com/api-reference/show-model-details).

Ollama lifecycle evidence now includes a real paired runner against local HTTP
fixtures: pre-send cancellation, post-send client settlement, terminal/adoption
races, saved receipt recovery and ambiguous-loss redispatch refusal. Exact-evidence
continuation exercises both coordinator and reviewer HTTP schemas using the shared
read-only context machinery. Metadata-only HTTP and decoder-only checks are recorded
separately. These fixtures make no inference and establish no model suitability.
Cancellation fences action adoption; a settled client or closed connection never
proves remote generation completion. Only the terminal protocol response does.
