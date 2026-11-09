# Continuous Claude native readiness — CLOSED

## Exact blocker, not evidence of a malicious plugin

Pinned Claude Code 2.1.296 SHA
`24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`
reports two builtin registry entries in the retained successful structured-output run:

| name | path | source |
| --- | --- | --- |
| cc-plugin-agents-md | builtin | cc-plugin-agents-md@builtin |
| cc-plugin-plugin-authoring | builtin | cc-plugin-plugin-authoring@builtin |

The implemented `continuous_adapter::inspect` rejects a nonempty plugin inventory.
The approved capability policy excludes inherited/outside-profile plugins; these
builtin metadata entries are **not by themselves proof** of extra tools, execution,
context leakage or inherited user plugins. The implementation may be stricter than
necessary. Removing that predicate or silently whitelisting entries without examining
their effects/context loading would not settle the approved boundary.

The 27-case proof established four successful named result shapes, one inference
POST each, denied operation effects and CLAUDE.md/hook/MCP/history exclusions.
It did **not** check the application plugin predicate or place an AGENTS.md canary.
That omission matters for cc-plugin-agents-md. No blanket full-boundary pass follows.

Checkpoint-4 coupling replay caught the conflict: all 27 retained real-client event
records were passed unchanged through the actual application decoder. Four raw payload
shapes parse correctly in a separate check; **zero native results are admitted**.
No event field was removed or normalized to manufacture parity. Evidence:
`.local/continuous-coordinator/checkpoint-4/decoder-replay.log` (initial failed positive),
`decoder-replay-closed.json` / `.log` (explicit closed-gate replay), original
`checkpoint-1/final/valid.json` and the other immutable proof records. Replay made
zero CLI/provider invocations. [Report](continuous-coordinator-report.md) owns counts.

## Current enforcement and candidate

**claude-dialogue-v7** is the final isolated mock candidate, manifest
`5e51bc0fcd218b8e47a74ab83e8d647420e849fd8ba114e809a6317c7002aa6b`.
For new state-7/profile-v2 native mode, readiness says blocked, F3 offers no grant,
and preflight refuses before any allowance reservation or process launch. Legacy
profile substitution into state 7 refuses. A native-mode fixture regression verifies
zero charged calls/no queued turn; the UI regression verifies the unavailable action.
No real binary/login/credential is needed for those negative tests.

Mock transport remains explicit and usable through the [stable launcher](continuous-coordinator-evaluation.md).
It cannot launch a real provider. Existing schema-5/6 releases/projects keep their
own contracts; this gate neither changes nor revives their trial permissions.
The original blocked 1/6 trial and exhausted native v5 remain untouched.

## Smallest recommended next action

Review/classify these exact builtin entries against actual startup/context/effect
behavior using the retained requests and supported pinned-client interfaces. A focused
credential-free extension should place project/ancestor/user AGENTS.md canaries,
verify inherited plugin exclusion and capture effective outbound context/tools plus
sentinel effects. Use the existing no-egress/dummy-credential fixture; no authenticated
call, shared setting, provider/model/auth switch, policy bypass or new sandbox platform.
If metadata is proven inert under the same restrictions, correct the overly strict
inventory predicate with exact identity/effect regressions and rerun the coupling gate.
If it loads unauthorized context, require a supported exclusion or keep admission closed.
Do not rely on post-event detection to prevent an unauthorized action.

This is a concrete remaining proof/contract issue, not a request for another broad
architecture plan or a demand to wait for a future empty-tool CLI feature. No additional
capability or native trial is authorized by this document. The independent queue,
question, scope/recovery and installed mock gates passed as the approved plan allows.
A live acceptance request is **not ready**; no new native operator project, launcher,
allowance or trial was prepared. Resolve this admission gate before asking to spend calls.
