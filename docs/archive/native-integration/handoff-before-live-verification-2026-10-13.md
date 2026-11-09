> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Task and authorization

Branch `feat/native-coordinator-integration`. Focused Claude conversation
implementation and credential-free verification are complete. **The operator has
authorized the exact three-call live check for candidate claude-conversation-v5.**
They will launch, review F3, authorize the allowance and submit messages themselves.
Do not launch the TUI, submit a greeting/connectivity check or spend a model call.
Wait for their evaluation, then inspect durable outcomes/counts read-only. Record
operator observations separately; do not infer broader acceptance.

Workers, consultation, repository editing, task steering, weather trial,
provider/model/route substitution, global install, operator cutover, shared
configuration, push/merge and publication remain excluded. No new UI prototype,
phrase matching or automatic fake fallback. No coding subagents.

## Checkpoints and current implementation

Preserved starting checkpoint: `8e08a6b9be5d6dbedf7dd3a07415d5782949d83a`.
Independent boundary proof: `5333993`; integration: `b730aa6`; reconnect/draft:
`ac013b2`, `74e6598`, `332171f`; receipt race: `1ceabf9`; final build source:
`717348b4ae88e618a3768d9b226ec26ad91329be`; installed closeout `ae9c3c0`.
Commits are local only.

New schema-5 projects select Claude Sonnet 5.5 medium, three durable calls,
closed initial authorization. F3 opens exact-profile review and explicit
allowance authorization; opening never calls the model. Ordinary messages reach
Claude unchanged through supervisor → fenced systemd host → runner. Only
assistant text streams provisionally, then validated completion saves it.
Thinking is excluded; model output cannot create/authorize tasks or change state.
Conversation and budgets belong to Waywr, not a provider session.

Existing Rust/Tokio service, bounded writer, ownership/receipt/recovery,
accepted composer/input/styles, project isolation and lifecycle remain in use.
Minor-3 IPC advertises `claude_chat_v1`; old schemas retain their releases.
Missing CLI/login, profile drift, failure, uncertainty and exhaustion give honest
readiness/inspection guidance. Stop after failed/cancelled/uncertain output;
never reset a debit, replay an ambiguous call or silently substitute a model.

## Verified versus unverified

Claude Code 2.1.296 exact hash independently passed fifteen no-egress,
dummy-credential cases: streamed text, literal file/slash mention, denied
filesystem/shell/delegation/network/messaging effects, customization exclusion,
and HTTP/overload/midstream failure without retry/fallback. This is effective
pinned-client evidence, not whole-process confinement against malicious clients
or same-UID processes. Known managed files refuse rather than bypass policy.
Existing login file presence was checked without reading/copying secrets.

Final installed mock-transport test passed through actual init/start/TUI/F3:
three completed receipts, zero tasks, 0.00527 s status (<1 s), streaming while
typing, detach/draft restore/resume, service-restart adoption once, exhaustion and
old-command replay. Separate interruption/runner-loss fixtures stopped correctly
with one debit; uncertain execution had one spawn and no replay. Source tests
cover authority, text validation, UI/drafts, cancelled replay, reconnect, schemas,
host identity/cancellation, slow clients and SQLite busy (<500 ms). IPC consistency
passed. Original failed fixtures/receipts are preserved, not rewritten.

**Unverified:** real authentication, entitlement, server/account policy, native
latency/interruption, physical terminal acceptance of this build and all crash
windows. No working weather app, consultation or native worker is verified.
Codex remains independently blocked/deferred; it does not block this Claude gate.

## Candidate and exact next action

Paired release: `.local/claude-conversation/install-01/releases/claude-conversation-v5/`.
Manifest SHA-256:
`ae55df4c9a259adf16506a75981179c8a01a2d4d3fbba4bc5b9b4137dbeafe6d`.
Build source `717348b`, pinned Rust/Cargo 1.99.0, locked/offline; Cargo.lock
unchanged across this increment. Thirteen packaged file hashes and three built
binaries were verified. Exact provenance: `.local/claude-conversation/final-build-receipt.json`.

Stable launcher: `~/projects/example-workspace/.local/claude-conversation/waywr-live`.
Reusable native project: `.local/claude-conversation/live-eval/`, key
`<private-id>`; at preparation, native profile initialized,
zero calls and no service launched. Operator evaluation status is not yet reported.
Launcher handles release/project/private locator internally;
no manual environment or runner commands, and no global PATH change.

[Authorized live check](claude-live-acceptance.md): Claude Code 2.1.296,
claude-sonnet-5-5, medium, existing saved login; at most three invocations total,
including failed/uncertain calls. Workers/consultation zero; no retries, fallback,
substitution or budget increase. The operator opens the launcher and types.
Do not authorize the profile or submit messages on their behalf. After evaluation,
verify records, charged calls, attempts, receipts and outcomes without any model
call. Failure conditions stop further dispatch; preserve all evidence.

## Evidence, protected resources and rollback

Ignored evidence: `.local/claude-conversation/boundary-05/`,
`installed-final-05/{summary.json,cleanup.json}`, PTY captures, project-local
packets/transcripts/receipts/state, `focused-final.{log,json}`, build receipt,
`live-preparation.json`. Earlier attempts/bundles/releases remain intact.
All three final fixture project services ended inactive/MainPID=0 after owned
children finished; operation metadata remains. No shared Herdr/operator service
was changed. At preparation the live project had not started a service; this is
not a current claim about the operator's subsequent evaluation.

Prior mock operator project `.local/native-integration/final/operator-eval/`, key
`<private-id>`, remains bound to `native-mock-c4-final4`:
last read four canned turns, one answered question, zero tasks; untouched.
Preserve r9, schema-3/4 installations, historical approved runs, accepted v3
evaluations and blocked `~/projects/waywr-rust-sim-eval`.
No legacy conversion, retirement, service restart or operator cutover is implied.

Schema 5 is isolated new state, not a transparent upgrade. After accepted work
retain its compatible release and forward-repair; never restore a pre-message
snapshot over it or bind old schema-4 code. Code rollback alone is insufficient;
release fingerprints/quiescence and persistent compatibility remain authoritative.
Historical paths, failed outcomes and receipts stay immutable.

## Reading order

[AGENTS.md](../AGENTS.md) → this handoff →
[Claude contract](claude-coordinator-contract.md) →
[current report](claude-conversation-report.md) →
[live request](claude-live-acceptance.md). Then
[execution](execution-contract.md), [state](state-contract.md),
[IPC](session-protocol.md), [TUI](tui-contract.md), [limits](current-limits.md).
The [broader native proposal](native-coordinator-integration-proposal.md) is
worker/consultation scope deferred from this increment; the
[boundary assessment](native-coordinator-boundary-checkpoint-1.md) preserves Codex
findings. [Index](README.md) locates archives selectively, not as startup reading.
