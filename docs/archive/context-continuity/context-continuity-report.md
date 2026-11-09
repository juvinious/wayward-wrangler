> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Context continuity — implementation and trial report

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

October 10, 2026. **The authorized trial is complete and operator-approved after
explicit recovery of its original draft validation failure.** The operator chose
**block, never overwrite**, then approved revision-2 at 2026-10-10T21:19:10.750602+00:00.
Five native calls total, two critiques, no repeated draft call. No pending gate or
active run remains. The original failure and append-only adoption remain recorded.

## Scope and explicit limitations

- This implements **explicit context selection with automatic resolution and
  delivery**, not automatic project-wide relevance discovery.
- **One new substantive question per run is a prototype constraint**, not a
  permanent workflow requirement. It can be expanded later without another
  memory service or execution backend.
- One Herdr-visible sequential execution backend, existing native CLI permissions,
  one authorized real trial `context-01`, at most five calls/two critiques.
  No milestone-3 work, additional trial, automatic failed-call retry, shared
  configuration change, Git/PR operation or permission expansion occurred.

## Implemented behavior

- `context.py` resolves the reference-only selection in
  `config/contexts/example-notes.toml`. It reads canonical request/response rows,
  verifies exported response consistency and source approval, checks artifact
  hashes, imports qualified open findings, and follows CLAUDE.md's AGENTS.md
  pointer once. No transcripts or native session histories are included.
- The existing envelope stores the selected content and provenance as a frozen
  context snapshot. New packets automatically include it and effective accepted
  decisions. Imported answers are not inserted as new human response records.
- Source/selection/document changes, conflicting topic selection and explicit
  approved supersession stop continuation. Hashes do not prove semantic currency;
  unregistered changes and paraphrased duplicate questions remain limitations.
- Version-2 schema/prompt/validation permits evidence-linked agent questions.
  The graph records a completed stage before preparing its durable request and
  interrupt. Requests bind source result/packet, current artifact, context and
  local decision revision. No controller-configured question exists in this trial.
- Accepted answers require a subsequent available author revision even when a
  critic says `needs_revision=false`. Late answers without a remaining revision
  slot and excess questions block explicitly; no sixth invocation is introduced.
- Approval binds the current artifact, decisions and selected context internally;
  the interactive phrase remains `approve this revision`. Human answers alone
  can supersede a prior decision, and only through explicit reconsideration.
- Version-1 packet shape, planned-question trial behavior and existing decisions
  remain preserved. The graph uses version-specific branches; approved historical
  runs and their receipt validation are unchanged.

Implementation: `src/agent_workflow/context.py`, version-aware changes in
`workflow.py`, `artifacts.py`, `records.py`, `herdr_backend.py`, `adapters/real.py`
and `cli.py`; `gate_demo.py` demonstrates controller process boundaries without
supplying input. New profile, brief and rules are under `config/` and `examples/`.
There is no Git commit; exact source hashes are in the evidence snapshot.

## Offline results

**43 tests passed before launch, 44 after the validation correction, and 48
after adding the explicitly authorized recovery tests.**
Adoption tests cover publication through the existing graph, mismatched evidence
refusal, repeated adoption, retained failure history, and refusal to consume a
saved human answer during repeated adoption. Project-local `pip check` passed. No authenticated calls are made by these tests.

New isolated tests cover explicit delivery/provenance, canonical pointer handling,
open findings qualified across runs, changed source/selection/export refusal,
approved supersession, duplicate topic/request detection, question references,
legacy native-question refusal, exact inherited-question suppression,
reconsideration routing, question-budget blocking, mandatory revision, exhausted
revision slots, stale approval/context, saved answers consumed by fresh graph
instances, and terminal approval without redispatch. Fixture approvals remain
fixture-only, never operator authorization.

## Exact inherited context

Project: `example-notes`; scope: **Example Notes initial offline-only release**.
Selection: `config/contexts/example-notes.toml`.

- Selection file SHA256: `dbb518df55289bc31ac117ba01103bd7b2fda5e1caa7e92fc1eb355123bd8f6f`.
- Frozen context SHA256: `af34fce59dcebfab7da20af985529e0e5933d9984f1228a5432a3cd16ad46ed5`.
- `m2-cli-01/D-backup-policy` (decision): source record SHA256 `5ca70a2031687357ecf07fb09c0779454ba482b24905d3c96cdb393f2a1a4a3c`. Authority: `m2-cli-01/A1`; request/response pair SHA256 `4a48ce2eddfe5e51035f99d9a2eb00f8eb6a38e0e5b43bf21c40169e7a496232`.
- `m2-cli-01/revision-2` (artifact): source record SHA256 `a0f74ea28445647c9d3a61e5cc9e4194455e2257dd9aa7dc34a3a169f8d0f3d0`. Authority: `m2-cli-01/A1`; request/response pair SHA256 `4a48ce2eddfe5e51035f99d9a2eb00f8eb6a38e0e5b43bf21c40169e7a496232`. Artifact bytes SHA256 `7b42658dffb436aae8aaff958fd7096c73cb4ff33bde1b6f7ebcf3d8c010f466`.
- `document:AGENTS.md` (document): source record SHA256 `1a2d6e1c3279c5d9fdd6ec007326235c430460b6983ad1241062248eef09c54b`.

The inherited answer is **Offline only**. Source approval is A1 from the approved
`m2-cli-01` run; revision-2's historical “Not approved” sentence was left intact,
with the actual approval supplied as metadata. No open findings existed in that
source final state; open-finding reuse is verified with isolated fixtures only.
Document dependency hashes (including the CLAUDE.md pointer) and all selected
content are in `inputs/envelope.json`. No duplicated editable policy file exists.

## Real trial: what actually happened

| Stage | Native calls | Observed outcome | Herdr tab / pane |
|---|---:|---|---|
| draft | 1 | Native success; original failed receipt retained, output explicitly adopted | w8:tG / w8:pH |
| critique-1 | 1 | Completed using both accepted decisions | w8:tJ / w8:pK |
| revision-1 | 1 | Completed; applied operator answer | w8:tK / w8:pM |
| critique-2 | 1 | Completed with prior version and delta | w8:tM / w8:pN |
| revision-2 | 1 | Completed and operator-approved | w8:tN / w8:pP |

**Final total: 5/5 native stage invocations, 2 critiques.** The four permitted
post-recovery calls completed once each. No budget remains. The native child exited 0.
Claude Code **2.1.296**, configured **opus/high**, resolved **claude-opus-5-5**;
336 partial streaming events were recorded. Actual live pane output was captured
before any receipt existed. Init reported `dontAsk`, only `StructuredOutput`,
no MCP; no permission bypass was added. The unchanged restriction flags disable
hooks and tool use. A successful scoped call does not establish hostile-process
isolation or exactly-once provider billing.

Codex **0.162.1**, configured **gpt-6.1-sol/high**, completed both version-2
critiques under the existing read-only/never/no-hook restrictions. The exact
model/reasoning settings are in execution records; exec JSONL does not independently
report the resolved backend model. Both critiques contain the expected allowed
code-mode-host-disabled diagnostic, followed by successful completion. Claude
revisions resolved **claude-opus-5-5**, configured opus/high, with only StructuredOutput,
dontAsk and no MCP. No additional permission or authentication scope was introduced.

Claude's draft explicitly cited the inherited policy/source artifact and kept
collision behavior provisional. It did not reopen the backup-policy choice.
It generated this new question itself:

> When a user exports to a filename that already exists in the chosen local folder (for example, an earlier backup), what should Example Notes do?

Options (generated by the agent, not injected into the profile):

1. Block: never overwrite. Show a message and require the user to choose a different filename or folder.
2. Confirm before replacing: show a 'Replace existing file?' prompt with Cancel as the default. Replace only after explicit confirmation, and leave the old file unchanged if the export fails.
3. Never overwrite: automatically save under a new numbered name (for example, 'notes (2).txt') and show the actual path in the confirmation.

**Why the stage failed:** my validator required the recommendation to equal one
option string exactly. The model returned a valid prose recommendation favoring
confirmation before replacement. The planned contract did not require exact
string equality. Native JSON-schema output succeeded; the application validator
rejected it with “Question needs 2–3 distinct options and a matching recommendation.”

That overstrict check is now removed; recommendation remains nonempty bounded
advisory text and never becomes a human answer. A regression test passes, and the
**exact captured output passes the corrected validator offline**. The original
failed receipt, invocation count and graph state have not been rewritten. The
saved output is preserved for inspection in
`.local/evidence/context-rejected-output.json`, explicitly marked unaccepted.
No failed native call was retried and no replacement question was generated or inserted.
The inspection copy retains its historical unaccepted classification; the later
adoption record supplies current authority without rewriting that history.

## Explicit recovery — verified

The operator subsequently authorized only this saved-output recovery and continuation
within the original budget. `awp adopt-saved-draft context-01` is intentionally
restricted to this run/stage and pins the original inspection/native/output hashes.
It is not general automatic recovery.

Validation checked original operation and reservation, packet and frozen envelope,
prompt hash and exact packet encoding, frozen JSON schema against the execution
argv, dispatch permit/claim/binding/runner identities, count 1, exited native child,
absence of cancellation/uncertainty, one successful native completion, and agreement
between native final representations and the authorized saved structured output.
The exact output passed its original schema and the corrected application validator.

- Native stream SHA256: `15fe5745b7da261e7d5d4eb253e5063f6499725d38f37c84071e21e7a016fde4`.
- Canonical structured output SHA256: `70d12044908269c903396742c19a3aa90cd78889b83fa667ab56dd89e5a80a79`.
- Published draft content SHA256: `6ab298e8a31f7f4ee6f2c4175aec63f5552a2496c5123e105196c44981a0111c`.

The original `receipt.json` remains failed and byte-for-byte preserved. The
append-only `operations/draft/recovery/adoption.json` identifies output/evidence
hashes, validation correction, validator source hash, original failure, recorder,
authorization and adoption result. `published.json` binds its published artifact
and durable question. `result.json` serializes the exact original structured
object; no draft text, question or option was edited.

The application reactivated the same run and called its existing `drive()` /
LangGraph `invoke(None)` path. The pending worker node reused completion evidence
from explicit adoption, published normally, and checkpointed before the question.
No checkpoint internals were edited. No human answer was manufactured.
A second real adoption invocation left graph state and native count unchanged;
repeat adoption never consumes an operator answer or dispatches a draft.

## Controller restart, answer consumption and approval — verified

The recovery controller used **w8:tH / w8:pJ** in `example-workspace` (w8).
These are historical IDs, not guaranteed live navigation targets. At final
inspection all five recorded worker panes were closed; disk evidence remains.
This verification did not close panes or stop any process.

1. Controller PID **2941257** exited with code 2 at the unanswered question.
2. Fresh controller PID **2941261** recorded the operator answer, then exited
   with code 2. Its exit snapshot contains the saved response while the graph
   still has no locally consumed decisions and only the draft completed.
3. Fresh controller PID **2941808** started with that exact saved response and
   consumed it through the existing gate. The final graph contains one local
   decision, decision revision 1. Counters/bindings/receipts are identical across
   the answer-recording boundary; no completed draft call or pane was repeated.

The operator chose:

> Block: never overwrite. Show a message and require the user to choose a different filename or folder.

This differs from Claude's recommendation. The recorded human choice, not the
recommendation, reached both critiques and revisions, alongside inherited
**Offline only**. No worker asked the inherited question again, and no later
worker raised another question. Critique-2 received the previous reviewed draft
and a **5,662-character unified delta**. Final revision explicitly blocks existing
paths, forbids automatic renaming, and checks again at write time.

A1 records approval by **the operator** of revision-2, bound to local decision
revision 1 and the original selected context digest. Final artifact SHA256:
`1afbcb4e189aa8961133732c0f544fa68e79396443ba8ee450bcd18a077cd3d3`.
The last independent critique reviewed revision-1; revision-2 did not receive a
third critique. Its historical “Provisional” text is immutable; the controller's
A1 record is authoritative for approval. No exact typed approval phrase or extra
Vim/plugin interaction is inferred from the stored response.

No continuation or further approval is needed. Inspect without advancing:

```fish
cd ~/projects/example-workspace
.venv/bin/awp status context-01
.venv/bin/awp show context-01 --artifact revision-2
```

## Preserved evidence and remaining work

All approved-run files, checkpoint values and application run records match
`.local/evidence/context-before.json`; shared Claude/Codex settings hashes also
match. `demo`, `acceptance-m1` and `m2-cli-01` remain approved and unchanged.

Evidence:

- `.local/evidence/context-completion.json`: final approval, all counts, answer/restart and preservation checks.
- `.local/evidence/context-recovery.json`: earlier adoption/unanswered-gate snapshot, preserved.
- `.local/evidence/context-original-failure-report.md`: original failure report preserved.
- `.local/evidence/context-offline.json`: pre-launch tests and source hashes.
- `.local/evidence/context-live-draft.json`: live worker before receipt.
- `.local/evidence/context-continuity.json`: current state, provenance, counts,
  execution metadata, preservation checks and source hashes.
- `.local/evidence/context-rejected-output.json`: retained unaccepted question.
- `.local/runs/context-01/operations/draft/`: immutable failed receipt, invocation,
  frozen packet/prompt/schema, actual native events and runtime model metadata.
- `.local/runs/context-01/gate-restart-evidence/`: first controller identity and exit.

## Remaining limitations

The bounded drafting/decision/review/approval path passed after explicit recovery;
the first attempt was not an uninterrupted success. Source open findings and
supersession behavior remain offline-tested only. Explicit selection is automatically
resolved/delivered; project-wide relevance discovery is not implemented. One new
substantive question per run remains a prototype constraint, not a permanent
workflow requirement. Schemas cannot establish semantic novelty or catch every
paraphrased repeated question. Packet inclusion is observed, not proof that a
model used every supplied field.

No milestone-3 fault matrix, broad automatic recovery, adversarial permission
certification or exactly-once provider inference is claimed. Invocation counters
track native process attempts, not internal HTTP requests or formatting turns.
No new trial, additional recovery, permission expansion or later milestone is
authorized by completion of this run.
