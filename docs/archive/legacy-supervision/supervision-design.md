# Bounded workflow supervision (milestone 5 recovery)

## Target operator experience

Wayward Wrangler is the standalone application and primary user interface; it
does not require an enclosing Codex, Claude or other agent session. This Codex
session is only a development tool. The intended normal entry point is
`cd <project>; wayward-wrangler`. It will
recognize or initialize that project, create or reconnect to an owned Herdr
workspace through the supported integration, and present a persistent,
responsive terminal coordinator. The coordinator will accept conversational
tasks, show active/queued/blocked/completed/failed work, take additional tasks
while workers run, support task-specific steering/cancellation, and show human
questions and artifact review in place. Disconnecting will not stop supervised
work; reconnecting will not require a workspace ID, hash, internal install path
or manual resume. `--project`, explicit state paths and noninteractive commands
remain available for automation and diagnostics. This is a **product target**,
not a claim that this recovery patch implements that coordinator.

The model-backed coordinator owns conversation, planning, delegation and
interpretation of steering. Claude, Codex and future providers are
interchangeable coordinator or worker integrations. The
durable supervisor owns execution identity, lifecycle, reconciliation, bounded
recovery, scheduling and notifications. Wayward Wrangler owns the project
session, task history, decisions, approvals, artifacts and execution records;
provider-native sessions are optional continuity aids. Workers receive scoped
packets and run visibly in Herdr with retained evidence. The TUI remains the
place to direct work, inspect status, answer questions and switch between
tasks. Steering becomes a durable record applied at a
stage boundary; a change of scope, budget or prior-work validity must be shown
to the operator instead of rewriting frozen inputs. Asynchronous interaction
and a sequential queue come before concurrent execution. Concurrent workers
need separate isolation and conflict rules and are deferred.

A future coordinator-provider switch keeps the same Wayward Wrangler session
and supervisor ownership. The incoming coordinator receives a bounded handoff
with provenance-linked task state, accepted decisions, unresolved findings and
artifact versions. Active workers and pending human gates remain in place;
switching cannot redispatch a task or silently change its frozen worker models.
This patch does not implement the TUI or coordinator switching.

## Ownership and lifecycle

A per-project supervisor is the only driver of a supervised run's LangGraph
transitions. It uses an explicitly configured Herdr Unix socket, verifies the
socket peer's user and process identity, and creates the project's workspace,
controller display pane and worker panes through the supported Herdr API. It
does not set `HERDR_ENV` or simulate a terminal. The controller pane is a view
and input surface; it records a human response but never launches a worker.
The supervisor consumes recorded responses and continues routine transitions.

The operator installs and starts one `systemd --user` unit for the selected
project. `Restart=always` restarts the supervisor after process failure; the
service manager, not the lost Herdr workspace, owns its lifetime. The unit's
`ExecStart` uses the installed wheel entry point with explicit project, state,
run and socket. A supervisor lock and monotonically increasing ownership
generation fence duplicate supervisors. Every dispatch checks the current
generation before publishing launch intent or permission. A stale supervisor
or controller must refuse further work. The existing controller lock still
serializes graph writes.

## Reconciliation

The run envelope, operation packet, receipt and historical terminal binding
remain immutable. New workspace/controller routing is appended with cause,
old/new Herdr identities, generation and evidence hashes. Before changing a
binding, the supervisor verifies the saved run, operation and process records.
A validated completed receipt is reused. A prepared operation with **no**
launch intent, claim, dispatch, child, permit or receipt may be retargeted to
an owned replacement workspace; this is infrastructure recovery, not a new
model attempt. Any possibly sent launch, live child with dead runner, identity
mismatch or missing output after permit remains uncertain. A timeout alone
never grants another execution attempt.

Recovery has a fixed attempt limit and delay. Every attempt records its cause,
action and outcome. The run reports `recovering`, `awaiting-input`, `failed`
or `uncertain`, with a durable notification on fatal/unresolved outcomes. An
answered ordinary choice is consumed automatically; record-answer-and-exit is
reserved for explicit restart tests. Final approval remains a human gate.

## Bounds and validation

One active run per project, one supervisor service and one Herdr backend. No
automatic retry of a possibly launched worker, orphan adoption, model change,
budget reset, concurrent stage dispatch or graph checkpoint editing. Test
workspace loss, controller loss, supervisor restart, stale generation, missing
worker report and exhausted recovery with deterministic fake workers before
applying the same path to `decision-log-01`. The live trial may continue only
after its prepared `critique-1` is proven unlaunched and the replacement
workspace/server identity is verified. The existing Claude draft and human
answer stay byte-identical.
# Revision quality and stopped-task behavior

A validated completed worker receipt settles **execution**, not the usefulness
of its text. `revision-incomplete` means the candidate is structurally usable
but lacks dispositions for supplied findings. The application records the
exact omitted IDs and original evidence, keeps them open, and sends them to
the next authorized critique. It may reach final review only as a clearly
provisional artifact with every open finding displayed. `output-invalid`
means the saved result cannot safely be interpreted, including duplicate
finding identities; it stops dependent work without another model attempt.
`uncertain` remains for execution or ownership evidence that cannot establish
whether a call ran or completed. These conditions must not be conflated.

An explicit quality adoption appends a record that binds the frozen packet,
schema, prompt, completed receipt, saved output and original rejection. A
controller replay uses the existing receipt and LangGraph public checkpoint
interface. The supervisor owns no policy resolution and cannot fabricate a
disposition or human answer. A stopped quality task keeps its controller pane
available for status and artifact inspection while the supervisor remains
active. The current replacement limit remains three controller creations;
exhaustion requires operator review and does not authorize a worker retry.
