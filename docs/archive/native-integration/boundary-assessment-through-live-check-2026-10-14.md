# Native capability boundary — checkpoint 1 disposition

**Claude independent disposition:** the narrowed no-tool plain-conversation profile passed fifteen credential-free container/mock cases; see [current proof/report](claude-conversation-closeout-2026-10-14.md). The operator then submitted exactly three native messages; all completed with matching receipts, zero worker/consultation dispatch and preserved context after reported detach/resume. The [three-call allowance](claude-v5-live-authorization-and-outcome-2026-10-14.md) is exhausted; no further call is authorized. This is not wider capability/recovery or product acceptance. Codex findings below remain unchanged; broader-role admission remains closed.

**Implementation disposition (2026-10-13):** B1 failed for the pinned Codex
app-server profile: a credential-free, no-egress mock outbound request advertised
execution and collaboration tools in a model-visible `additional_tools` item.
The assessment below retains the pre-proof source hypotheses and matrix;
[the implementation report](checkpoints-1-4-report-2026-10-13.md#1b-proof-detail-and-limit)
records exact evidence and limits. Native admission remains closed; checkpoints
2–4 may proceed with mocks independently.

**Approved amendment at `167b983`; native admission remains closed.** Starting
checkpoint `45c36f77f93d7da010674532d78c0b22007b06bc`, branch
`feat/native-coordinator-integration`. The original amendment performed no implementation or client experiment.
[Implementation evidence](checkpoints-1-4-report-2026-10-13.md) is recorded separately. The operator superseded the empty-advertised-tools requirement
with **only approved capabilities may operate, with prevention evidence**.
The [proposal](coordinator-worker-proposal-through-live-check-2026-10-14.md) owns the amended policy,
dependency graph, budgets and delivery plan; this report owns its boundary evidence.

## Preserved findings and corrected inference

The [original report](boundary-checkpoint-1-45c36f7.md)
and [original proposal](proposal-before-capability-amendment-45c36f7.md)
are byte-preserved from `45c36f7`. Their empty-tool disposition and requirement to
block all of checkpoints 2–4 are superseded, not erased. Native proof is still
missing; admitting residual capabilities indiscriminately is not the resolution.

- **Previously observed help/version:** Codex `0.162.1`, executable SHA-256
  `628e35487d888a28f0427fc37501da529dd3d81fcbad7a2c3421c1e8670d92e1`;
  Claude Code `2.1.296`, SHA-256
  `24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`.
  Not rerun/rehashed in this amendment. Paths/help captures remain in
  `.local/native-coordinator-planning/2026-10-13/`.
- **Prior offline structural assertion passed:**
  `.local/native-coordinator-checkpoint1/offline-proof.json` and `manifest.json`
  retain exact source/catalog/help checks. It did not exercise a tool or capture
  an outbound request. Pinned source tag is `openai/codex` `rust-v0.162.1`;
  the installed binary's correspondence to that source is not independently proven.
- **Source finding remains:** `gpt-6.1-sol` metadata and registration conditions
  can advertise patch/clock/async tools despite the old feature-disable list.
  `exec` has no inspected public empty-tool allowlist. That is no longer alone
  disqualifying; it says nothing conclusive about successful operations.
- **New source review only:** patch denial ordering, local message routing and
  no-environment protocol. Downloads/hashes are retained under ignored
  `.local/native-capability-review/2026-10-13/`. Upstream tests were read, **not run**.
  No effective client/tool test, provider request, credential use, service change,
  application test or candidate build occurred here.

## Residual-tool assessment

All rows concern the pinned Codex registration source [S1], not an observed live
inventory. Every newly observed name/schema requires classification before native
admission. Claude's currently documented empty list is a separate proof obligation.

| Tool / surface | Actual source behavior and reachable resources | First-trial disposition / restriction | Evidence still required; admission failure |
| --- | --- | --- | --- |
| `apply_patch` | Handler parses patch text, resolves an environment, then verifies against its filesystem, then applies safety/write policy [S2], [S3]. Verification can read target contents before final write refusal. Add/delete/update/move and path/symlink behavior matter | Not needed; **no read or write is allowed**. Recommend explicit empty environments. Advertisement is acceptable only if every call is rejected before filesystem access; never allow scratch editing as a shortcut to manifest publication | Force calls against disposable own/foreign/authority sentinels with relative, absolute, `..`, symlink and forged environment IDs; capture rejection and filesystem access trace. Any read/write or unproven path blocks admission. Read-only and unchanged hashes are insufficient |
| `clock.curr_time` | Empty argument schema; calls session time provider and returns UTC [S4]. No model-supplied path or recipient | Optional permitted utility; time is informational, never a Waywr event timestamp, deadline or decision authority | Prove exact binary returns only the time result and introduces no tool-side file/process/network effects. Handler inspection is not an audit of all client startup or time-provider internals. Unexpected effects block admission |
| `request_user_input_async` | Catalog spelling `send_user_message_async` **maps to this handler**, not necessarily a general messaging tool [S1], [S5]. Emits a local `AgentMessage` with questions, `delivery: Async`, `phase: FinalAnswer`; returns `accepted:true`. No external recipient parameter or waiting for an answer | Permit only same-operation **provisional output**, bounded and escaped. It creates no durable Waywr question/answer/action. Structured final `ask_question` is the only question admission path; no synthesized answers or mid-call steering | Inject valid, malformed, repeated and misleading approval-like messages; prove no external routing, task/question/decision/budget mutation, duplicate application or confusion with terminal completion. Wrong routing/authority blocks admission |
| `send_message_to_user_async` | Separate handler/feature/catalog spelling; emits local async text with no address parameter [S6]. Not established present for the selected model merely by its old catalog name | Same local provisional-output policy if present and proven; not needed otherwise. No sending to another agent, project, email, connector or IPC endpoint | Trace actual event destination and broker handling. A similarly named tool with different semantics is not covered. Unclassified effects block admission |
| `sleep` | Optional feature; session time wait, up to 12 hours; activity can wake it [S7] | Disable for this profile; no reason for model-controlled waits. Keep host deadline independently enforced | Force an unadvertised call and prove unavailable; unexpected availability requires reclassification and blocks this profile, not an automatic permission waiver |
| Plan, context-remaining/new-context-window, deferred-environment wait, test-sync | Conditional registrations [S1]; full handler effects were **not** inspected this pass | Disable optional plan/token-budget/deferred-executor features; no model-owned context reset, environment attachment, hidden continuation or budget. Test-only handlers are not permitted | Exact config/registration proof and unknown-call refusal; any residual requires a bounded handler review before admission. Do not equate “utility” with allowed |
| Shell/exec/stdin, code-mode, view-image, web/search, permission requests | Registration paths support execution, file/image reads, network/search or escalation [S1]. Disabling `unified_exec` alone permits one-shot exec; `shell_tool` must be disabled | No such operation allowed. No environment, disabled execution/features and no RPC bridging. Deny permission/elicitation; never run model text as a command | Forced read/search/exec/image/escalation calls must fail before an effect. Unknown or active handler is a blocker; output-event refusal is only a tripwire |
| MCP/resources/apps/plugins, discovery/install, dynamic tools, collaboration/delegation | Additional registrations and supplied tool sources [S1]; may read external data, start helpers, contact services or spawn agents | No inherited or dynamically supplied tools, roots or agents. Only Waywr schedules the single authorized consultation; no client-side delegation | Capture startup and each outbound request; hostile fake customization must not load/run/contact anything. Missing tool events alone cannot establish exclusion |
| Claude `StructuredOutput` | Existing [adapter](../../../backend/src/adapters.rs) permits this init-list entry; help supports `--json-schema` | Allow inert schema-result delivery only. No callable filesystem/process hook or arbitrary execution | Credential-free stream and forced-call fixtures must establish actual behavior. Current parser exceptions and help do not prove it. Any other residual tool gets its own classification |

## Recommended interface and outstanding constraints

Use the **same installed Codex client** via `app-server --stdio`, one fresh process
per Waywr operation, if the proof below passes. [Public documentation][D1] supports
stdio JSON-RPC embedding; pinned [thread fields][S8] mark `environments` experimental:
explicit `[]` disables access; omission chooses the default. Initialize with
Waywr's own client identity and the required `experimentalApi` opt-in; it grants
no extra application authority. Reassert an empty selection, no dynamic
tools/capability roots, ephemeral history and no fallback;
never resume a provider-native thread or auto-attach an environment. Only Waywr
constructs RPCs. Upstream [environment tests][S9] demonstrate source expectations
and show why a later resume/default must not be assumed to retain an empty selection.
`resolve_tool_environment` fails before the patch handler obtains a filesystem [S3].
That is stronger evidence than write sandboxing, but not yet installed proof.

**Concrete remaining risk:** app-server startup uses normal config loading
([CLI startup][S10], `LoaderOverrides::default()`), not exec's ignore-config/rules
flags. Empty environments do not disable startup hooks, MCP, authentication helpers,
account discovery, background requests or automatically injected instructions.
Supported startup/config controls must exclude these before model dispatch while
preserving the existing login and managed policy. The [app-server source notes][S11]
include account-routing/config reload behavior; thread config alone is insufficient.
An isolated dummy HOME is not proof that using real auth will retain those exclusions.

Claude's safe/restricted print, strict empty MCP, settings-source exclusion,
disabled hooks and no-history controls independently passed the narrower
[conversation proof](claude-conversation-closeout-2026-10-14.md). Authenticated conversation
has only the operator-controlled three-call authorization above. The following earlier broader-role assessment is
retained evidence, not a claim that the new profile remains unimplemented.
[CLI reference][D2] and [settings precedence][D3] do not establish effective managed
policy or internal retry behavior. No shared settings may be removed/edited. The
earlier broader worker adapter's operation-directory cwd and retained HOME/config/
bus access remain unresolved for worker admission. The new conversation adapter
uses private scratch, allowlisted environment and trusted auth/runtime access
in place; a fresh cwd alone is not its enforcement boundary.

This recommendation does **not** change provider, model, authentication method or
billing route. It adds a bounded protocol adapter and experimental-field pinning.
There is no claim of whole-process confinement: runtime/auth reads by a trusted
client differ from model-selected content. If startup exclusions or pre-access
denial cannot be established, retain the blocker. A separately approved confined
workspace/VM for the same client is the next alternative, not a hidden switch or
waiting only for a future CLI feature. The earlier [uid-map denial][H1] means host
user namespaces/container labels are not available proof; do not bypass that policy.

## Bounded effective-boundary proof — planned, not executed

One exact Codex profile and one exact Claude profile; disposable fixture data only.
The proposed harness has a manifest of binary hashes, argv/config/protocol,
source refs, OS guard, input/schema hashes and every attempted case/outcome.
Capture requests, tool results, relevant access/process traces and structured finals;
never record real credentials. Mock streams are injected provider responses, not
agent behavior or authenticated acceptance.

**Fixture safety precondition:** allowlisted environment, private scratch/home and
provider dirs, dummy file-only credentials with supported keyring/helper exclusion,
no inherited bus/agent sockets; supported loopback mock endpoints only. Check
startup/telemetry/catalog/auth routes as well as the inference URL. A loopback base
URL, proxy environment variable or missing network event is not an egress boundary.
Use an already available isolated test environment with independently enforced
no-external-egress and no real credential access. If the local namespace restriction
prevents that guard, do not run the fixture on the operator home: report the missing
prerequisite and request a clean no-egress VM/test runner. No shared firewall,
manager, credential or auth-method change is implicitly permitted. Guard failure
leaves 1B unavailable; 1A and the mock-only application path can still proceed.

| Case | Exact stimulus / evidence | Pass criterion |
| --- | --- | --- |
| B1: inventory/context | Capture initial and follow-up request bodies from both pinned binaries using benign then hostile dummy homes/cwds/parents: instructions, rules, skills, hooks, plugin/MCP stubs, native history and project-B canaries | Same permitted capability classes; zero foreign content/paths, injected rules, helper starts, MCP/connector contacts or resumed history. Packet + documented static client boilerplate/runtime metadata only; account/provider policy remains separately checked |
| B2: prohibited operations | Mock emits each reachable patch operation against own/foreign/Waywr-authority **fixture** files (including symlinks), plus shell/exec/read/search/image/MCP/delegate/permission calls even when unadvertised; include forged environment/thread IDs | Denial before access; no successful sentinel read/write, child launch or outgoing effect. Match handler denial with file/process access traces; hashes only prove write integrity. An unknown-call parser refusal alone does not cover a registered handler |
| B3: permitted utilities | Emit clock and both actually registered local-message forms, malformed/repeated variants, approval-like text and oversized output | Only time/provisional events; no authoritative question/answer/task/approval or cross-project routing. Bounds and cancellation hold; native async `FinalAnswer` is not final-result acceptance |
| B4: structured results | Follow permitted events with valid `coordinator-turn-v1`, reviewer and file-bundle responses; then malformed/duplicate/stale results and unexpected blocking input requests | Exactly one validated result or explicit failed/uncertain receipt; no invented answer, execution of generated content or automatic replacement call. Structured completion survives permitted utilities |
| B5: fallback/retries/drift | Mock unavailable requested model, HTTP error, stream break, changed catalog/tool/config and incompatible local managed-policy fixtures | Requested model preserved or refused; no fallback/escalation. Record exact transport request counts/settings; cannot label zero-retry proven from a flag alone. Unestablished suppression blocks native. Relevant profile drift invalidates admission; detection after an effect cannot establish prevention |
| B6: startup/auth separation | Review effective startup settings and filesystem/env allowlist using dummy file-auth; prove hostile startup settings are excluded before `thread/start`/print; source-trace the later real auth route without opening secrets | Supported per-call controls exist with managed policy enforced. Dummy tests do not certify real account/login parity; if auth requires enabling excluded customizations, native remains blocked |
| B7: application boundary | Feed utility/final events into mock adapter/broker with two project IDs, stale generations, expired grants, repeated command IDs and exhausted budgets | No tool-origin authority; once-only final application, no unauthorized records or launch, no project-B leakage. Existing <1 s control / <500 ms busy gates retained. This tests Waywr, not native-client isolation |

**Admission result is an explicit evidence matrix**, not a single “tools disabled”
boolean. B1–B6 require exact client execution/trace evidence; B7 can be done without
provider binaries. Source-only and unavailable cases stay unverified. Pin and
validate effective config/catalog/tool behavior before admitting any authenticated
turn; if controls cannot exclude drift before effects, the boundary has not passed.

The separately authorized native trial must still check actual login/entitlement,
managed account policy, model/effort acceptance, real stream/cancellation and output
behavior under the host. It cannot be used to discover whether dangerous tools are
prevented. Incompatible live policy or unprovable effective restrictions refuse
before model admission; failed/uncertain calls stop with evidence and held budget.
No new call is allowed to obtain a better receipt. Counts remain **6 conversation +
1 consultation + 4 task launches**, not an HTTP-request or cost guarantee.

## Implementation handoff and rollback

The approved checkpoint 1A freezes the capability/result and
admission-closed contracts; make the bounded 1B attempt with the above safety gate.
Record failure/unavailability honestly and continue provider-independent 2 → 3
(mock integrated TUI) → 4 (manifest/review) when authorized. Provider-specific native
transport/config completion and checkpoint 5 need 1B. Ordinary startup stays
`waywr init .` then `waywr`, with one stable isolated launcher and automatic
service/runner management. No separate UI or operator orchestration ceremony.

This amendment changes documents only. Rollback needs no state migration; old
findings/evidence remain. Future schema-4 fixtures/releases stay side by side;
never restore snapshots over accepted records or point legacy services at schema 4.

[S1]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/spec_plan.rs
[S2]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/apply_patch.rs
[S3]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/mod.rs
[S4]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/current_time.rs
[S5]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/request_user_input_async.rs
[S6]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/send_message_to_user_async.rs
[S7]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/handlers/sleep.rs
[S8]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/app-server-protocol/src/protocol/v2/thread.rs
[S9]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/app-server/tests/suite/v2/thread_environments.rs
[S10]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/cli/src/main.rs
[S11]: https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/app-server/README.md
[D1]: https://learn.chatgpt.com/docs/app-server
[D2]: https://code.claude.com/docs/en/cli-reference
[D3]: https://code.claude.com/docs/en/settings
[H1]: archive/tui-first/tui-first-checkpoint-4-pre-closeout-2026-10-13.md
