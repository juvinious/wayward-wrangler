# Continuous coordinator contract — implemented; offline native gate passed

This owns implemented state7/config4/IPC1.5/profile-v2 semantics. It is a
maintenance contract, not a trial authorization or the complete product design.
[Architecture](../architecture.md) distinguishes current limits and product direction.
Older schemas retain their separately versioned contracts; no automatic migration.
Completed boundary/trial reports are historical under `../archive/continuous-coordinator/`.
Their runtime evidence was intentionally deleted during the 2026-10-15 reset.

The state9 IPC1.8 [conversation overlay](../conversational-coordination.md) implements
source-bound answers with once-only continuation and direct human approval of an
exact presented proposal. State7 inputs keep the semantics below; no historical
message is reinterpreted and model output cannot supply human authority. New
versioned packets extend result-v3 with source-bound `propose_steering`; legacy
result schemas are unchanged.

## Provider and result boundary

Claude Code 2.1.296, exact hash in the architecture snapshot, Sonnet 5.5 medium, existing saved login.
Fresh restricted/safe print process, no inherited hooks/configuration/MCP/history,
empty built-in tools, explicit prohibited-operation deny list; only generated inert
StructuredOutput. One agentic turn, no transport retry/fallback. The offline proof
observed one inference POST even though the client counts the inert tool step as
num_turns=2. Raw validation, receipts and admission checks remain required. Historical offline
proof and bounded native observations do not authorize new calls. The profile still
requires explicit application allowance authorization and rejects client/policy drift.
Its six-call conversation ceiling and zero worker/consultation allowances are
implemented first-increment limits, not enduring product requirements.

Registry inventory allows only the exact two pinned builtin tuples for
cc-plugin-agents-md and cc-plugin-plugin-authoring (builtin path/source and client
2.1.296), or empty/missing mock metadata. Extra fields/entries, duplicate/partial
pairs, spoofed identities, malformed inventory and version drift refuse. Reported
skills/MCP/slash commands stay empty; only StructuredOutput tool use is allowed.
These builtins have real instruction/skill functions outside this restricted profile;
canary/effect proof establishes their exclusion here. Trusted-client plugin registry
metadata access is separate from model context and permitted client runtime access.
No arbitrary plugin, whole-process sandbox or general filesystem confinement is implied.
Existing supported exclusions and binary/policy/login preflight remain unchanged.

Frozen response schema [v3](../../tests/fixtures/continuous-coordinator/result-v3.schema.json):
assistant_text, evidence_refs, and at most one inert nullable question/proposal/formal
answer-or-deferral suggestion. Strict duplicate raw keys, unknown fields, bounds,
provenance/project/base/decision checks. No prose or fenced-object extraction.
Named structured_output is the action channel; only recognized speech is displayed
provisionally. Model claims are never saved status. Applications emit save/revision/
not-applied status only after their writer commits.

Typed completion schema 1 binds packet, attempt and raw-event hashes, terminal
status, exit code, speech/action validity and category. New coordinator receipt
schema 3 binds its digest; host receipt validation precedes once-only adoption.
Completed-format-failure requires positive client/provider completion, settled host,
unchanged capabilities and zero applied action. The pinned client can report
error_max_turns for a rejected/absent schema without another request; generic failure
or a matching error string alone cannot establish this proof. Unknown/capability/
integrity/ambiguous outcomes stay blocked. No old receipt is reclassified.

## Durable interaction and authority

One writer/scheduler/operation host per project. Send durably queues unchanged text;
one active call, 16 pending messages, 32KiB each. Freeze packet on reservation and
never mutate it for later inputs. Context includes preceding completed replies even
if they arrived after this queued message's submission, excludes later queued input,
and stays within six optional messages plus mandatory scope/decisions and 96KiB.
Subjects bind independently of task-drawer selection; new-scope binding happens once.

Later text does not invalidate scope authority. Actions recheck their actual head,
question/decision, binding and authorization dependencies. Local edits/relevant
confirmed decisions make older actions stale. Merely asking a question does not
revoke authorization. Scope review is optional; only its explicit final human
command authorizes the shown exact revision. Pending conversation is visibly not
included. Content/dependency changes require renewed exact authorization; no workers,
consultation, execution admission or task allowance follow.

Formal model-inferred answer/deferral is an inert source-linked suggestion. Human
confirmation or direct Save answer writes an append-only question decision; ordinary
discussion needs no confirmation. Saved answers enter subsequent context without
an automatic continuation call. Deferral stays unresolved and requires exact
unresolved-choice acknowledgement in a saved scope before authorization.

Interrupt first commits an action fence, then requests host cancellation. Late
result actions cannot cross that fence. Completion-first is reported completed;
request-first is retained interrupted/stopping until positively reconciled.
Recoverable outcome holds older queued messages durably. A deliberate new Send
acknowledges only that visible outcome/generation and requests its own new turn;
older held messages remain held. Continue held and Withdraw queued are explicit,
idempotent commands. Reservation/withdrawal race reports its winner, never refunds
an invoked call. Ambiguity, integrity and capability failures cannot be cleared by
new text. No automatic replay, repair, renewed allowance or silent queue release.

## Compatibility and rollback

New state 7/config 4/IPC 1.5 continuous_scope_v1/profile claude-proposal-v2/response 3.
Old release/schema/receipt contracts remain bound and readable through compatible
installations. Code rollback on empty fixtures is safe; after accepted new records,
keep a compatible paired release or forward-repair. Never restore an old snapshot
over accepted work, point old code at state 7 or mutate original trial evidence.

Schema6 retains its original failure policy; state7 never reclassifies older receipts.

## Production controls

State-7 UI: nonempty composer Send queues normally; empty Enter opens the displayed
Interrupt/question/held-queue/scope action without executing it. F7 opens bounded
queue inspection; release and withdrawal are separate durable controls. Draft and
explicit revision-request intent are retained in project-local UI state. Informational
chat never needs confirmation. Formal suggestions open one source-bound confirmation;
a refresh changing its identity disables recording. Scope review remains available
through /plan while discussion runs, naming conversation not included in permission.
Application completion/save badges come from committed events, not provider prose.

Recovery events carry the committed pause generation/hash so a visible failure and
its acknowledgement cannot depend on a later snapshot. Queue displays the recovery
generation and reason. A changed outcome refuses an obsolete release; no automatic
retry follows. Definitively refused submission text is restored only into an empty
composer of the same subject; a newer draft is preserved, and refusal records live
under project-local session/refused-deliveries/. Sending is always a separate action.

Scope command acknowledgements include the committed state revision in state 7.
Exit prepares an asynchronous fresh snapshot, then persists one stop command using
that exact revision. Existing lifecycle CAS/ownership checks still decide acceptance;
a subsequent race produces an explicit refusal, not an automatic changed-body retry.

Unique init, provider stop and terminal records plus the observed exit/is_error pair
are required for action admission. Raw/named result disagreement is integrity failure,
not recoverable formatting. Missing/duplicate/contradictory completion proof remains a
hard stop. Known success/exit 0 or max-turn-format-error/exit 1 can establish completion;
this does not bypass the capability inventory or authorize an action.

## State8 final discussion context

State8 enriches discussion with current-scope compact workflow facts, the selected
request's history paths and bounded read-only project context before the final
96 KiB packet check. New topics exclude previous workflow plans/requirements and
use accepted project source knowledge only when its commit matches current HEAD.
Mandatory current message/scope/decisions remain intact; older messages can be
trimmed by the existing scoped conversation loop. Optional development context
uses at most 24 KiB of remaining space with available counts/omission labels.
No later enrichment bypasses final validation. Identity and measured-size refusals
are distinct and occur before reservation/native launch.

An explicit later state8 Send can include earlier blocked user input from that
same subject, labelled as unexecuted saved input. It does not requeue or replace the
blocked turn/packet, infer an assistant response, grant scope or replay execution.
Older state7 message-selection behavior is unchanged.
