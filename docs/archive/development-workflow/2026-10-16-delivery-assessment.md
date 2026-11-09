> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical snapshot from merged checkpoint `c2064bc534d8cbe753bbae93a8d85a98ce8c403a`.
> Completed delivery evidence; permissions and next actions below are historical.
> Relative links are relocated; current work is in [the handoff](../../HANDOFF.md).

# Example Task App delivery assessment — corrected 2026-10-16

This initial audit concerns the original Rust run. Its completed application did
**not** verify human-initiated planning; earlier README/handoff/closeout claims of
user participation were too broad. A later human-led Python migration has now been
delivered; its evidence and remaining limits are recorded separately below.

## What the records establish

- **Request origin:** the implementing agent entered the initial request through
  the production TUI at 2026-10-15 16:27 UTC. Commands
  `1792081668602101278-568966-1` and `1792081668626680527-568966-2` contain the full
  request and an accidental duplicate fragment. Database role `user` means input
  through that channel; it does not establish that the human typed it.
- **Plan origin:** the approved implementation plan and that agent-entered request
  already proposed skeleton → parallel persistence/CLI → wiring/checks. The shipped
  coordinator prompt at `57bb545` also prescribed this application-specific sequence
  and Rust checks. This was inappropriate in the general application.
- **Actual assignments:** all eleven saved assignments match the task instructions
  in retained live Codex coordinator result files. There are 31 recorded native
  decisions: nine assignment batches, six integrations, three check decisions,
  nine waits, three substantive decisions and one completion. The recorded executable
  is Codex 0.162.1, gpt-6.1-sol/medium, not a mock fixture.
- **Approval:** command `1792094466410686841-623974-363`, at
  2026-10-15 20:01:06 UTC, saved `authorize_scope` for
  `<private-record>`, revision2, proposal SHA
  `52905a2636dd2efcc19fc6070bd107204d8e89e2b36c54020d2775c8ecb100b3`, policy SHA
  `f98a4bd1b7647852edee31bd2b6237468b981cec9d6bb1594c67e8788226bc7b`.
  It has the production TUI command identity/shape and corresponds to the human
  approval in this session. Events22/23 record authorization before worker dispatch.
  Persisted IPC records do not independently authenticate the physical typist.
- **Feedback/adaptation:** frozen coordinator packets contain worker status,
  reports, diffs, checks and retained recovery notes. For example coordinator18
  assigned parallel units; coordinator20 reviewed the actual persistence commit;
  coordinator26 reviewed smoke coverage; coordinator34 selected completion after
  checks33. Recovery assignments were model outputs responding to failures.
  The implementing agent repaired Waywr, installed releases, paused/resumed the
  service and sent three operational recovery notes. No external assignment script
  supplying all result records was found; the general prompt nevertheless strongly
  prescribed the successful application sequence.
- **Production TUI interactions:** the agent-entered request, native proposal/reply
  display, human scope approval, task/output/history inspection and lifecycle
  controls used the production TUI. Scheduling and most development ran unattended
  in the service. Operational recovery notes were CLI commands. The record does not
  establish that the human conversed to develop the plan, steered the running work,
  or answered its substantive development questions.

Deterministic readiness/concurrency/integration scheduling was appropriate. Live
model-generated assignments under an agent-supplied request and application-specific
prompt do not prove authentic human-led planning. The original application result remains retained:
checked commit `887b33f09199522b73a1b806f1ef8435475f29f0`, six accepted units, 32 resolved
obligations and final exact test/build/smoke evidence. Nothing is erased or reclassified.

## Readable history

The original actual log was:

```text
~/projects/example-task-app/.wayward-wrangler/state/history/2026-10-15T162749Z-please-prepare-the-scope-for-example-task-app-in-this-project--build-a-small-offline-<private-record>.log
```

It contains actual conversation text, approval payload, assignment instructions,
worker outcomes, integration/check results and labelled coordinator closeout—not
just database identifiers. The retained 1,805,914-byte log ends through event253.
The adjacent compact summary records delivered status and exact checks. No missing
conversation/approval/decision has been invented or reconstructed. The source
records do not independently identify every physical UI operator.

The supported migration has relocated its exact bytes to the canonical path below.
Pre/post fingerprints match for authority/conversation/events, 1,196 native evidence
files, Git refs, files across 12 retained worktrees and the complete log.
[HANDOFF](2026-10-16-handoff.md) records the installed release and verification:

```text
~/projects/example-task-app/.waywr/history/2026-10-15T162749Z-please-prepare-the-scope-for-example-task-app-in-this-project--build-a-small-offline-<private-record>.log
```

## Correction boundary

The general prompts no longer prescribe Example Task App, a task order or a Rust-only
completion gate. The coordinator plans from conversation/project context and chooses
assignments/checks from approved requirements and observed results. Waywr schedules,
validates and records these decisions. Explicit `/steer`, `/reply`, scope/policy
approval and history remain distinct authority paths. The stale `/plan` selection
exposed by the human's new topic was a frontend defect, since repaired; the new
proposal was retained.

## Observed human-led Python migration

The human resumed their own new topic and discussed a Rust-to-Python migration with
the live coordinator. Revision1 of `<private-record>` was approved at
event276. The implementing agent submitted no scope approval for this request and
supplied no application-specific assignment sequence. Its application changes came
from Waywr-launched workers. Native coordinator decisions reviewed results, assigned
repairs/follow-ups, integrated frozen commits and chose exact combined checks.

The run received implementation/operator assistance: Waywr packet, selection,
check-command, cache-recovery and presentation defects were repaired during use;
one earlier operational `/resume` was submitted by the implementing agent as
recorded in the [recovery checkpoint](../workflow-recovery-2026-10-16.md).
This was not an unassisted reliability demonstration.

Final checked promotion paused on the existing untracked `tasks.json`. After local
Git exclusion resolved that blocker while retaining the file, the human used the
production TUI's `/resume` (event528) and supplied the delivered screenshot. Saved
promotion evidence and events529/542/543 confirm checked local-main advancement to
`069a162fbcb118794515034d3ae91303e7c37135`, delivered status and recorded closeout.
The project worktree is clean; the local data file remains present.

Checks-43 records both `python3 -B tests/check_python.py` and
`python3 -B tests/check_migration.py` exiting0 on that exact commit: 44 tests and
all seven executable smoke scenarios passed, including isolated Python operation
without Cargo or a retained Rust binary. Receipt result paths and stdout/stderr
hashes were checked read-only after delivery; no checks or development were rerun.
Coverage remains platform-specific; concurrent-writer isolation and stronger
power-loss durability are not established.

The continuous readable log contains conversation text, actual assignment
instructions, outcomes, integrations/checks and labelled coordinator summary through
the delivered/promotion events, rather than merely database references:

```text
~/projects/example-task-app/.waywr/history/20261016T135632Z-15a13c43b482.log
```

The adjacent `.summary.md` is the compact delivered checkpoint. This establishes
one human-initiated request through planning, approval, worker execution, coordinator
adaptation, operational recovery, checked promotion and visible closeout. It does
not verify every steering/question interaction, unattended reliability or general
distribution readiness. Original Rust evidence and this request's evidence remain
separate and preserved; no conversation, approval or decision was invented.
