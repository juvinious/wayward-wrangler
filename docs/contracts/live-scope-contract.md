# Schema6 proposal/scope compatibility contract

This documents implemented schema6/claude-proposal-v1 semantics, including its
prompt-only result-v2 and original stop policy. It is not current state7 behavior
or standing trial authorization. [Current conversation](continuous-coordinator-contract.md)
uses named structured results, continuous discussion and typed recovery. No old
record is transparently converted. The failed 1/6 trial and mock walkthroughs are
historical; their runtime resources were intentionally discarded on 2026-10-15.

## Authority and records

Ordinary messages reach the selected coordinator unchanged. The new
`claude-proposal-v1` profile returns strict result-v2 with assistant text and at most
one inert question/proposal suggestion. The owned text transport and pinned-client
restrictions are preserved; no structured-output tool or extraction/repair call.
Unknown/duplicate fields, invalid references/base, multiple values and excessive
sizes/nesting refuse. Only assistant text appears provisionally; incomplete JSON
cannot create a task. Invalid completed output produces a retained stopped outcome.

Immutable `task_proposal_revisions` records bind the body, parent hash, context/
decision references and human command or coordinator packet/result source.
`task_proposal_heads` is a pointer/dependency projection; `task_scope_decisions`
are separate append-only human authority. All accepted state, events and command
outcomes commit together in the existing bounded SQLite writer.

Human saves/authorization bind proposal revision/hash, decision revision and prior
authorization command identity. Identical content/dependencies is a no-op. Changed
content or relevant answers invalidate effective permission; old revisions and
decisions stay intact. Concurrent clients cannot silently rebase or both authorize
from an outdated panel. Exact command replay returns its stored original outcome,
even after supersession; ID reuse with different bytes refuses.

Eligible scope requires an objective, deliverables, inclusions and acceptance
criteria, no unanswered linked questions/pending scope turns, and acknowledgement
of the exact unresolved-choice list. Human answers save with no invocation or
implicit continuation; explicitly submitting the saved answer is once-only and
requires remaining allowance. Local review/edit/authorization work at exhaustion.

`scope-authorized` means **“Scope authorized · execution unavailable.”** No run ID,
execution admission, task reservation, worker launch or consultation follows. Both
command authority and scheduler/store fences reject execution, including forged
legacy admission/ready work. Future execution needs a distinct approved allowance.

## Context, recovery and compatibility

Freeze the selected head, linked human answers, binding, budgets as-of reservation
including this held call, user watermark and at most six previous messages. The
96KiB packet limit removes oldest optional messages first; mandatory overflow stops
before reservation. No registry/unrelated project metadata, scans, provider history
or new cross-project grant. Result application rechecks target/dependencies/binding/
watermark; a valid stale suggestion remains a completed call with an unapplied
action, never overwrites current scope or authorizes work.

Validated receipts adopt once. Partial/failed/cancelled/uncertain calls retain their
debit and stop trial dispatch; absence alone never permits replay. Existing
claim/permit/process identities, bounded output, cancellation, generation fencing
and deterministic exit/resume checkpointing apply. Scope references are checkpointed
continuously; a model summary never owns decisions.

Schema6 uses `[conversation] schema_version=3`, state 6 and IPC minor 4 /
`task_scope_v1`. New actions: `save_proposal_revision`, `authorize_scope`;
`answer_conversation` saves only and `submit_message` can explicitly continue an
answer. Scope reads use `task.get`; `task.plan`/`admit_task` execution paths refuse.
The profile freezes six conversation calls, zero task/consultation calls,
closed until exact-profile authorization. That application gate is not permission
for this development session to spend native calls.

Existing schema 3/4/5 and legacy projects retain their bound releases/semantics.
Schema-5 chat still uses result-v1/action:null and its own three-call allowance.
No automatic migration or operator cutover. Side-by-side releases remain compatible
only with their declared schemas; rollback after accepted schema-6 state needs
compatible code or forward repair, never restoring an old snapshot over new work.

## Production panel and bounded views

Empty-input Enter opens the authoritative next action. Saved answers offer a
separate Continue action; no Save when no open question. Review scope presents one
of four numbered steps at a time: purpose/deliverables, boundaries/acceptance,
unresolved choices, explicit exact-revision authorization. Only the final step
offers authorization; acknowledgement advances to it without a decision. Each step
has one primary action and relevant auxiliary edit/revision/previous/close controls.
These local presentation steps create no new durable authority or protocol fields.
An authorized scope is a result view, not another approval. Local edit uses the
same textarea with per-field selection,
list items per line, a rationale, Save and explicit Discard; Escape retains it.
Drafts and pending command IDs are project-local UI files, never database authority.
Unchanged refresh preserves step, position, focus and acknowledgement; changed
identity invalidates them and returns to step 1 without opening a closed panel.
Backend validation always remains decisive. `/plan` reads scope via `task.get`.

Schema-6 conversation display caps recent messages at 256KiB, recent nonmessage
status at 128KiB and event pages at 256KiB. Older messages stay in durable records;
display truncation is explicit. Model context selection reads canonical selected
records independently. Decision inspection shows the latest 32 retained decisions.
Local scope edit storage caps at 128KiB; field/proposal limits apply while editing.

## Frozen response and editing limits

Result v2 is exactly `{schema_version:2, assistant_text, evidence_refs, action}`.
`action` is null, `ask_question {prompt,rationale,options}`, or
`upsert_proposal {base,proposal}`. Base is null for a new task or exactly
`{task_id,revision,sha256}` for the selected saved task. Proposal has only title,
objective, deliverables, in_scope, out_of_scope, constraints, acceptance_criteria
and unresolved_decisions. Every field is required. Evidence references must belong
to the frozen packet allowlist. Unknown/duplicate keys at every depth, multiple JSON
values, Markdown wrappers, invalid evidence or base refuse; no extraction/repair call.

Ceilings: nesting 16, result 64KiB, assistant 32KiB, proposal 16KiB, title 120
characters, objective 4KiB, lists 16 nonempty entries of 1KiB, unresolved choices 8.
Open questions cap at eight; result application and action/proposal/message/turn
completion are one transaction. A known invalid completed response produces a
profile-specific output-invalid receipt, retains its debit and blocks trial dispatch.

Schema-6 discussion is an append-only, idempotent input: unrelated progress/global
state revision cannot reject it. Writer checks still enforce lifecycle, selected
project/task, binding, message bounds and pending allowance; reservation freezes
context and result application fences the watermark. Revision saves and decisions
retain their exact base/dependency/prior-authorization checks. Schema 5 is unchanged.

Scope detail refreshes at most twice a second, with one outstanding request, because
a text-only completion can clear pending work without changing a task version.
The panel names its readiness or exact remaining requirement. Refresh never approves
or opens a panel automatically. The launcher retries bounded read-only preparation
if an old exit closes its socket; an uncertain mutation retains its command ID and
is not replaced. Existing generation fencing remains decisive.

Schema6 frozen choices: unresolved choices may be acknowledged explicitly; all
saved content/dependency changes invalidate permission conservatively; strict text
JSON is used with no tool/schema facility or additional model repair call.
