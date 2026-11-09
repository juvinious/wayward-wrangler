# Schema5 Claude conversation compatibility contract

This documents implemented schema5/claude-chat-v1 behavior, not new-project defaults
or universal product restrictions. [Current conversation](continuous-coordinator-contract.md)
owns state7. The completed three-call trial is historical; its installation, allowance
and runtime records were intentionally discarded on 2026-10-15. No trial is reopened.

## Authority and compatibility

Existing `[conversation] schema_version = 2` projects use application state schema 5.
No automatic conversion of schema 3/4 or LangGraph projects. Releases declare
schema compatibility; a schema-4 release cannot attach to new state. Existing
release routing, project identity and rollback fingerprints remain authoritative.
Conversation records retain their versioned messages/turns/binding/budget tables;
versioned `claude_chat_*_v1` metadata stores authorization, partial text and
cancellation. This is application memory, not provider-native session history.

One project supervisor owns the queue, SQLite writer, operation host and recovery.
Each ordinary message goes unchanged into an owned `conversation` operation and
the selected Claude client. No natural-language phrase matching. Replies are
conversation text; no task proposal, consultation, execution, approval or state
mutation is inferred from them. Clarification is ordinary durable conversation.
Those broader coordinator actions are deferred.

## Pinned profile and authentication

Migrated project-local `.waywr/coordinator.json` (formerly
`.wayward-wrangler/coordinator.json`) selects `claude-chat-v1`,
Claude `claude-sonnet-5-5`, effort `medium`, and three calls. Native mode uses
Claude Code 2.1.296, exact validated binary hash in the report; init resolves its
versioned executable without executing it. No provider/model/effort substitution.
Settings (F3) shows this profile and offers explicit three-call authorization.
Opening settings costs nothing; repeated selection never renews an allowance.
Authentication remains the existing CLI saved-login directory in place. The
application checks file presence, never copies/prints credential content. Missing
CLI/login or changed binary gives actionable refusal before dispatch.

Fresh print process per turn: safe + restricted mode, no settings sources,
empty tools and deny-all tool names, strict empty MCP, disabled slash commands,
owned system prompt, disabled hooks/memory/history/session persistence, one
agentic turn, single-model allowlist, no fallback, zero configured transport
retries and no nonstreaming fallback. Only trusted client runtime/auth access is
permitted outside its scratch cwd. No inherited bus, Herdr, agent, provider-key,
proxy, plugin or customization environment is forwarded. Native uses the normal
Anthropic route; mock endpoints/dummy keys occur only in the no-egress proof.

Known managed settings/MCP files cause preflight refusal pending review; policy
is not bypassed. The historical three-call check succeeded with this selected client/model and
saved login. Wider account/server-policy paths and future availability remain
unverified. The proof covers the pinned client, not a malicious client,
another same-UID process or every future CLI version. Drift requires revalidation.
The exact mock CLI fixture is separately hash-pinned and labelled; it cannot be
replaced with a real provider binary or used as automatic fallback.

## Dispatch, streaming and recovery

Freeze the selected message, at most six prior messages at/before its watermark,
the deterministic project handoff, provider profile, and hashes in a packet
(96 KiB ceiling). No future queued message, registry scan or unrelated project
content. A packet prepared before a crash is reused only with matching message,
project and authorization; it is never rewritten to fit new context.

Reserve/hold a durable call allowance before publishing host execution authority.
Existing systemd intent → claim → fenced permit → once-only spawn → process
identity → immutable result/receipt ordering applies. Supervisor restart adopts
validated results once or continues observing the same operation. Missing output,
process or receipt alone never authorizes replay. Infrastructure recovery does
not create another model reservation. Failed/cancelled/uncertain calls block
further trial dispatch, retaining evidence and the debit.

Only assistant `text_delta` events appear provisionally in the conversation.
Thinking/tool arguments never appear as conversation. Retain stdout/stderr under
existing 32 MiB transcript and bounded pipe/event policies; displayed response
text is limited to 32 KiB. Coalesce progress at scheduler refresh intervals,
retain an unread indicator and reading position, and keep composer input/focus.
Exactly one validated successful end-turn result becomes a durable assistant
message with `action:null`; invalid/missing completion cannot claim success.
Usage is preserved when reported in the immutable runtime result, labelled as
reported usage/client cost estimate. No invented cost, ETA or token percentage.

Deadline: 120 seconds per conversation call. Interruption uses the existing
verified process-group cancellation and receipt race policy, then up to three
seconds before bounded escalation. Never kill unrelated processes or infer
nonexecution from absence. Retained partial text remains explicitly provisional.

Detach closes the TUI; an already authorized call remains supervised. Exit fences
new dispatch, collects permitted results and checkpoints without a model summary.
Resume reconciles before continuing; existing lifecycle generations protect a
new resumed session from an earlier exit. Conversation handoff/budgets are
included in deterministic checkpoints. Authoritative SQLite persists continuously.

## IPC and UI

Protocol 1/minor 3 adds `claude_chat_v1` snapshot/readiness/provisional fields and
`cancel_conversation`; schema-5 attachment requires a minor-3 client. Existing
frame transport, command IDs, replay, queue bounds and stale-state checks apply.
Allowed mutations: discussion submission, exact-profile authorization,
coordinator interruption, project stop and resume. No second scheduler.

Header shows native versus mock transport, selected model/readiness and separate
project-service connection. States: awaiting authorization, available, running,
exhausted, blocked. Inspect retains exact turn outcome, reason and execution
record path; file `runtime-result.json` contains reported usage. Normal typing,
Enter/Shift+Enter/F2, paste, colors, drawer and terminal restoration are preserved.
Ordinary use remains `waywr init .` then `waywr`; services/runners start
internally. Historical isolated launchers are no longer retained. No global installation is implied.
