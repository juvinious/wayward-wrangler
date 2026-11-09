# Native boundary checkpoint 1 — blocked

Checked on 2026-10-13 from `ba27dc2d2777511adb573152d81a93c6d2ce9368`
on `feat/native-coordinator-integration`. The approved first-trial profile
requires packet-only, tool-free Claude and Codex roles. **That boundary is not
established for the pinned Codex CLI; native admission stays closed.** Checkpoints
2–4 depend on this gate and have not started. No schema, service, operator project,
installed release or shared configuration was changed.

## Evidence and conclusion

| Check | Observed evidence | Limit |
| --- | --- | --- |
| Installed versions | `codex-cli 0.162.1` SHA-256 `628e35487d888a28f0427fc37501da529dd3d81fcbad7a2c3421c1e8670d92e1`; Claude Code `2.1.296` SHA-256 `24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`. Versions match [`adapters.rs`](../backend/src/adapters.rs). | A version match does not attest that the installed binary was built from the inspected source tag. |
| Codex model metadata | The [pinned `models.json`](https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/models-manager/models.json) entry for `gpt-6.1-sol` has `apply_patch_tool_type: "freeform"` and `experimental_supported_tools: ["send_user_message_async", "clock"]`. | Bundled metadata may be refreshed at runtime; no authenticated resolved-model inspection was run. |
| Tool registration | [Pinned `spec_plan.rs`](https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/core/src/tools/spec_plan.rs) registers `ApplyPatchHandler` when an environment is advertised and model metadata has an apply-patch type. It also registers the async-message and clock tools from model metadata. The current adapter's `--disable` list does not govern these conditions. | Source-inspected, not an observed outbound request. `--sandbox read-only` limits writes but does not remove a tool or prove unrelated files cannot be read. Post-event refusal is too late to prevent a read. |
| Supported empty-tool control | Installed `codex exec --help` exposes feature toggles and read-only sandboxing, with no empty-tool allowlist. The [internal `ToolPolicy`](https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/ext/extension-api/src/tool_policy.rs) supports `allowed_tools: Some([])` for an embedding host, but the inspected [app-server `thread/start` fields](https://github.com/openai/codex/blob/rust-v0.162.1/codex-rs/app-server-protocol/src/protocol/v2/thread.rs) and CLI help do not expose it as the existing CLI adapter's per-call contract. | An undocumented interface may exist; it is not a supported basis for this trial. App-server `environments: []` alone does not exclude model-metadata utility tools. |
| Current adapter | [`command`](../backend/src/adapters.rs) disables `shell_tool` and other features, sets read-only and ignores user config, but does not pass the installed `--ignore-rules` switch. It already records an `apply_patch` caveat. | Adding `--ignore-rules` is a future correction, not a solution to residual built-in tools. No native invocation used this adapter. |
| Claude controls | Installed help and the [CLI reference](https://code.claude.com/docs/en/cli-reference) support `--print`, empty `--tools`, `--restricted`, `--safe-mode`, strict empty MCP, permission-prompt denial and structured stream output. [Settings precedence](https://code.claude.com/docs/en/settings) leaves managed policy in force. | Effective init tool list, hook exclusion, fallback, retry, auth and model entitlement remain unverified. The Codex blocker stopped before local mock endpoint work. |
| Retry / output contracts | Current Codex [config reference](https://learn.chatgpt.com/docs/config-file/config-reference) documents provider `request_max_retries` and `stream_max_retries`, while both installed helps offer structured output options. Existing parser handles captured JSONL/stream fixtures. | No proof of zero provider retries for the account's built-in provider, effective native structured output, or resolved model. One CLI launch is not a guaranteed one-request or dollar ceiling. |

The credential-free offline assertion at
`.local/native-coordinator-checkpoint1/offline-proof.json` passed. It checks the
selected catalog fields, registration conditions, internal allowlist, and absence
of a public empty-tool field in captured CLI help and thread-start schema. The
ignored evidence directory retains exact source files downloaded from the pinned
tag; `manifest.json` records URLs and SHA-256 hashes. Earlier installed help/version captures remain in
`.local/native-coordinator-planning/2026-10-13/`. This is **source inspection plus
one local structural check**, not an effective native tool-inventory test. No
provider CLI was run against an endpoint, no credentials were supplied, and no
model call occurred.

## Disposition and smallest next decision

Do not enable native conversation or the static-file workflow under the current
tool-free Codex profile. The smallest compatible resolution is a **supported
per-call empty-tool control for the Codex integration**, with an unauthenticated
effective-request/tool-inventory check for the pinned binary and requested model.
That could come from a future CLI release or an explicitly reviewed integration
exposing the existing startup `ToolPolicy`; it is not provided by the inspected
`codex exec` command. Either path requires separate authorization for the changed
interface/version and a fresh boundary proof. Using a different provider or
allowing residual Codex tools changes the approved role/profile and needs a
substantive operator decision. No automatic model substitution is permitted.

After that boundary is established, checkpoint 1 still needs credential-free
negative fixtures for inherited hooks/rules/docs/MCP/history, local mock
structured streams, effective fallback/retry settings, and explicit remaining
managed-policy/auth checks. Only then can the separately approved checkpoints
2–4 resume. Existing schema-3 state remains readable under its bound release;
rollback of this documentation-only checkpoint is a code/docs revert with no
state migration.
