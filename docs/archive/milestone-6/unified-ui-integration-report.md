> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Unified Wayward Wrangler interface — simulated integration

## Operator hands-on completion (2026-10-12)

The operator continued the original recovered `start a new task` request in
the installed r9 UI. Project service records show task
`<private-record>` reached `approved` after the five simulated
worker stages (draft, critique-1, revision-1, critique-2, revision-2). The
saved `D-retention` answer is `7 days`. The saved `A1` approval is bound to
`artifacts/revision-2.md` at SHA-256
`64a524c9d440c23168fdbaaf4e768ae0be88ba0ccec0d7af0451c8bc3fe3ee40`;
the current file hash matches. These records are under
`/tmp/waywr-unified.VTkzZC/.wayward-wrangler/state/runs/<private-record>/`.
The operator reported completion; this is hands-on evidence of the installed
simulated flow, not native-agent or browser-application acceptance.

The operator also found the workflow state and proposal path difficult to
understand. `/plan` becomes unavailable after scope authorization; `/answer`
handles the choice gate and `/approve` handles final review. Text such as
`ok` and `go` entered in the default composer became separate task requests,
not confirmations or steering. The two original `ok` tasks remain blocked;
the later `ok` and `go` tasks are proposed and unapproved. `/steer` and
open-ended coordinator conversation are not connected in this installed
service. These are product usability and backend contract gaps, not evidence
that the approved simulated artifact represents useful work.

## Operator color-enabled failure and corrected paired build (2026-10-11)

The operator's first installed check used `/tmp/waywr-unified.VTkzZC`, project key
`<private-id>`, workspace `wC`, and the bound
`m6-unified-ui-r5` release. Its active unit was
`waywr-<private-id>.service`. The first request,
`<private-record>`, was durably queued and its **fake coordinator
runner** launched in `wC:p3`. The runner wrote `claim.json`, then exited 1
before a dispatch permit. No `permit.json`, `dispatch.json`, `child.json`,
`result.json`, or `receipt.json` exists. No proposal was published, no workflow
run ID was assigned, and no author/critic worker was launched. The two later
requests show the same boundary in `wC:p4` and `wC:p5`. The SQLite failure
events and Herdr server log were preserved. A missing receipt alone was **not**
used as proof of non-execution.

The runner called `terminal.paint(label, "coordinator")` after claiming its
pane. The color map omitted `coordinator`, causing `KeyError` when ANSI color
was enabled. The prior isolated harness inherited `NO_COLOR=1`, so it missed
this path. The old installed r5 build reproduced the same blocked/no-permit
condition with color enabled at `.local/milestone6/unified-ui/evidence/color-repro-r5-short/`.
The formatter now handles the coordinator role and unknown future roles. The
installed harness explicitly removes `NO_COLOR`; the final r9 bundle passed
the full simulated task/decision/review path under ANSI color.

The UI now says **Stopped**, not **Done**, for blocked coordinator work. It
uses “workflow not started” where `run_id` is absent, since the coordinator
runner did launch. Conversation events give a concise explanation and point to
`/inspect`; that panel shows the coordinator call, operation directory,
launch/claim/permit/child/receipt presence, and supported next action. The
old detailed failure event remains in the database. The live composer no
longer restores the scripted placeholder after submission.

**Final side-by-side release:** `m6-unified-ui-r9`, manifest SHA-256
`ae219c7918e92720723dc666f8cad8a0c487e6b077e296a3b10d0942453e3948`,
installed at `.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/`.
The old r5 release and default alias remain unchanged; the operator project
was subsequently upgraded as recorded below. Focused offline checks: 24 Python tests and the targeted Rust
stopped-state test passed. Installed r9 simulated acceptance passed with ANSI
color at `.local/milestone6/unified-ui/evidence/color-pass-r9/`. An isolated
copy of the failed r5 project was explicitly upgraded to r9 and recovered once
through `session recover-coordinator`; its original claim/failure remained,
while the replacement produced one completed receipt and proposal. Evidence:
`.local/milestone6/unified-ui/evidence/prepermit-recovery-r9/`. The evidence
directories contain SHA-256 copy manifests (170 and 87 regular files);
`color-repro-r5-short/` has 60 verified regular files.

The recovery command is limited to one fake coordinator replacement after
verifying frozen task/packet identity, a dead original runner and the absence
of permit, dispatch, child, output and receipt records. It appends a recovery
intent with original evidence hashes and a new routing target; it does not
delete the old failure or retry ambiguous execution. A repeated command ID is
idempotent. Project upgrade accepts this specific stopped pre-permit state
after the UI detaches, while all other unresolved work remains a barrier.

**Operator project follow-up (2026-10-12).** After the operator detached the
r5 UI, the affected project was explicitly upgraded to r9 (binding generation
2). Its own service was restarted through the installed application. The
supported attach operation found the former `wC` workspace unavailable and
created the project-owned UI in `wD:t2` / `wD:p2`. The one-time
`session recover-coordinator` operation was accepted for only
`<private-record>`: its saved `call-v1/recovery.json` cites the
original operation, packet, launch and claim hashes; the original files and
failure remain intact. Replacement `call-v1-r1/receipt.json` reports completed
with exit code 0, and the task was then `proposed`. At this recovery point, the
two later `ok` tasks remained `blocked`; no author/critic worker or workflow
run had yet been dispatched. The subsequent authorized simulated run is
recorded above. The project service remains active, and the corrected UI
remains in `wD`.

```fish
set -l waywr ~/projects/example-workspace/.local/milestone6/unified-ui/install-r5/releases/m6-unified-ui-r9/bin/waywr
set -l project /tmp/waywr-unified.VTkzZC
$waywr --project $project
```

The corrected fake coordinator produced a proposal for the original request.
Opening it did not authorize the worker workflow; the operator later authorized
the scope and completed the simulated run. The three historical r5 failure
events and attempts remain inspectable through `/inspect` and the saved
operation files.
This is simulated-only evidence; no authenticated model call or web
application was launched.

**Branch:** `milestone-6-interactive-shell`. **Build:** local paired release
`m6-unified-ui-r5`, manifest SHA-256
`2d9da1f987be0849a7a1e8ee6606d61996c38208e778f418521d78018defce14`,
installed under `.local/milestone6/unified-ui/install-r5/`. No release was
published or selected in shared configuration.

## What changed

The accepted Rust renderer, rich text and `ratatui-textarea` composer now drive
the installed `waywr` pane. The obsolete three-pane module was removed. Enter
submits; Shift+Enter adds a newline when the terminal reports it; F2 remains
the fallback. The composer contrast, blink, literal paste, semantic colors,
`> ` user blocks, `•`/`└` grouped output, full-width conversation, task drawer,
slash picker and restrained effects are shared with the accepted evaluator.
Production IPC events and task snapshots populate that view; scripted scenario
transitions never run in the installed client. The standalone evaluator remains
available only through its explicit `waywr-ui-prototype --scenario decision-log`
command, and its stable binary hash remained
`2299064937232ec252e5316fedc2d4a8b4db79c4a66b083dbb4aec228e2a0244`.

The selected project's Python service still owns planning, authorization,
questions, artifact validation, approvals, lifecycle, recovery and all worker
processes. The Rust client submits durable commands through the existing
versioned IPC/outbox, checks task/revision preconditions, and verifies exact
artifact bytes against the pending SHA-256 before enabling approval. It reads
only the selected project's state and presents the service's deterministic fake
coordinator/worker mode as **SIMULATED SERVICE**. `/exit`, `/detach` and
`waywr resume` use the implemented project lifecycle; no action opens a model.

## Verification and evidence

| Check | Result |
|---|---|
| Rust editor, rendering, live-view, IPC and launcher tests | 39 passed; the 30-second stress probe was deliberately skipped. |
| Python attach and lifecycle regressions | 12 passed. |
| Installed fake-only UI flow | Private Herdr server and disposable project: installed screen, Enter task delivery, visible scope authorization, durable question/fixture answer, exact artifact opening and fixture approval, then detach to shell. The installed backend imported from private `site-packages`. Evidence: `.local/milestone6/unified-ui/evidence/final-flow10/`. Fixture answer/approval are **not operator consent**. |
| Installed foreground lifecycle | The same installed binary displayed `/exit`, returned to the invoking shell, reopened with `waywr resume`, and detached while the private Herdr server stayed up. Evidence: `.local/milestone6/unified-ui/evidence/final-fg05/`. |

The final private checks stopped their own systemd units and Herdr servers. All
170 workflow and 65 foreground regular files copied from `/tmp` into the
evidence locations above were compared by SHA-256; each copy has a manifest.
Earlier failed private harness runs and
older lifecycle evidence remain untouched. The pre-existing user unit
`waywr-<private-id>.service` for
`.local/milestone6/ui-prototype/evaluations/operator-02` was observed active
and was **not** stopped or changed; it is outside these private checks. Shared
Herdr work and historical approvals were untouched.

## Actual limitations

- The existing coordinator backend is deterministic and converts each new
  request into a task proposal. It does not support open-ended conversation.
  The installed composer defaults to **New task request**; selecting the
  unsupported conversation or steering destination preserves the draft and
  explains the gap. This is not the final model-backed coordinator contract.
- The current choice gate accepts only listed options. The live question panel
  does not claim free-form answers. Scope revision, active steering, provider
  switching/fallback and direct worker-pane focus from the drawer need separate
  backend/UI contracts. Worker processes remain visible in their Herdr panes.
- Native profiles remain refused by this session service; no authenticated
  calls were made. The operator's earlier Ghostty/Herdr Shift+Enter and visual
  acceptance applies to the scripted evaluator. Physical input, colors and
  worker switching in this installed build still need operator observation.
- Project release bindings are respected. An existing project bound to an older
  release will continue to use that release until an explicit project upgrade.

## One Fish launch command

From an ordinary Fish terminal, the following creates a fresh disposable
project and launches the **installed unified interface**. It uses only fake
coordinator/workers and creates project-owned Herdr resources:

```fish
cd (mktemp -d /tmp/waywr-unified.XXXXXX); ~/projects/example-workspace/.local/milestone6/unified-ui/install-r5/bin/waywr --yes
```

Type a small task request and press Enter. `/plan` loads its scope; the visible
**Authorize** action is separate. `/answer` opens a pending choice, while
`/approve` opens a pending final review and **Read** verifies its artifact.
`/detach` closes only this pane; `/exit` pauses the project session. Run
`waywr resume` from that same disposable project directory after an exit.
The installed binary does not depend on source imports or development
configuration; this path names only the local installed prefix.
