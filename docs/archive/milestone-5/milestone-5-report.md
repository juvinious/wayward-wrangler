> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Milestone 5 — installed native specification trial

**Status: critique-2 was interpreted from its saved output; revision-2 completed; the operator approved the provisional specification with five open findings.** Branch
`milestone-5-native-workflows`; no merge or release. The application is not
built; specification approval is recorded, with five findings still open.

## Implemented

- New initialized projects require `execution_mode = "native"`, workflow
  version 2, `product-spec-v1`, explicit Claude author/Codex critic binaries,
  models and efforts, two critiques and a frozen invocation limit of at most
  five. Historical trial IDs cannot be reused to authorize a new project.
- `preflight` checks target inputs, context, supported installed CLI versions,
  Herdr workspace and auth status without a model call. It reports a plan
  digest. An interactive start displays project, inputs, roles/settings and
  limits and accepts `authorize RUN_ID`; scripts can supply the exact digest.
  The digest is an integrity binding, not a credential. Authorization and
  configuration are frozen in the run envelope and packets; resume checks the
  installed version and uses existing evidence without repeat confirmation.
- The runner enforces the frozen attempt limit before native process launch.
  A completed call requires a validated receipt; a crash can instead produce
  an explicit failed/uncertain status with preserved launch/claim/child/output
  evidence. No receipt is manufactured, and no failed or uncertain call is
  automatically dispatched again. Existing Herdr-visible execution, bounded
  graph, durable questions, context checks and artifact-bound approval remain.
- A versioned specification prompt permits a bounded, useful product spec.
  The saved historical `context-01` draft prompt still regenerates byte for
  byte (SHA-256 `93e627005c6bf2ec5edc89964ad8fbd43279d7bb56bc8e0b8a3268182d87cb00`).

## Verification and actual trial progress

- 75 offline unit tests passed after the parent-ignored-target initialization
  fix; four focused native-admission tests also passed after adding interactive
  confirmation coverage. The first installed `init` exposed that ignore check;
  it left three generated files, which were identified and removed before
  rebuilding. No worker was launched on that attempt.
- Built wheel 0.3.0, SHA-256
  `15df642d84f4a7a21637c86557f8c13cd934c6860f8caa9677b374218176f94c`,
  and installed it into a fresh ignored pipx environment. The imported module
  resolves to pipx site-packages. `init` and `preflight` ran from `/tmp` with
  explicit `--project` and no source `PYTHONPATH`. Target config, context and
  state are under ignored `temp/project-decision-log/`; package resources came
  from the wheel. Preflight succeeded for Claude Code 2.1.296 and Codex 0.162.1
  with existing auth status, Herdr 0.9.3 workspace `w9`, Claude `sonnet`/medium,
  Codex `gpt-6.1-sol`/high, five launches/two critiques. Plan digest:
  `2acc635b4a208e0eebccde5562ef1b64a810687c8883270ad28e1eb237a11a9e`.
  Preflight did not verify backend model entitlement.
- Captured the checkout-linked shared plugin, switched it to the copied installed
  plugin, and verified its registration and pipx interpreter binding. The
  installed handler opened the exact draft by project-qualified SHA-256 link in
  read-only Vim (`w9:t4`). The old plugin root/manifest hash and before/after
  registrations are retained under `.local/milestone5/` for rollback. No
  unrelated tab or Herdr server was closed.
- **At the first stop, one native stage invocation had occurred:** Claude draft completed once in
  `w9:t3`/`w9:p3`, with an immutable completed receipt and draft artifact SHA-256
  `68d815a3103a6d873488e9a271e9c1a6f29d6f0cae0029207f445ab2d448f917`.
  Requested `sonnet`/medium resolved to `claude-sonnet-5-5` in Claude's runtime
  init. No Codex call had occurred at that point. Claude raised an actual substantive question
  about editing an existing decision, with immutable, free in-place and
  history-preserving options. The question was not injected or answered by the
  controller.
- The original controller `w9:t2` exited at the unanswered gate. A fresh
  controller `w9:t5`/`w9:p5` saved the operator's answer to request
  `D-<private-id>` and exited before consuming it, as planned.
  The operator chose option B: free in-place editing without prior-version
  history. The saved response and exported decision both record that answer.
  `.local/milestone5/gate-snapshot.json` is the earlier unanswered snapshot;
  authoritative records are in the target's state directory.
- The operator then started a fresh installed controller. It consumed the
  saved answer and prepared `critique-1`, but Herdr returned
  `workspace_not_found` for the run's frozen workspace `w9`. At that point the
  run was `uncertain`, with `['draft']` completed and no pending human gate. The
  `critique-1` operation had no launch intent, runner claim, execution permit,
  completion receipt or worker binding. Thus **one native stage invocation
  had occurred at that point** (Claude draft); no Codex invocation had occurred. No
  automatic retry or further model call was made. The earlier preflight's
  `w9` check was valid then but did not guarantee that workspace would remain
  available after controller exit.

## Supervision recovery and earlier stop

The operator authorized deterministic supervision before any further native
stage. [Supervision design](../legacy-supervision/supervision-design.md) records the ownership and
bounded recovery rules, along with the standalone TUI/coordinator product
direction. The supervisor uses the explicitly configured Herdr Unix socket;
it does not impersonate `HERDR_ENV`. A user-local `systemd --user` unit restarts
the supervisor after process failure. A lifetime lock and ownership generation
fence controller replacement and dispatch. Operation provenance stays immutable;
an appended routing record changes only future execution location. Status and
durable notices explain unresolved outcomes.

Before live recovery, the saved `critique-1` had **no** launch, claim, dispatch,
child, permit, receipt, binding or invocation-counter file. The current Herdr
server had no matching launch-token pane; `w9` was absent. The supervisor
created workspace `wA`, appended `routing/rebind-0001.json`, and started one
owned controller. It did not edit the graph checkpoint or original operation.
The draft artifact remained SHA-256
`68d815a3103a6d873488e9a271e9c1a6f29d6f0cae0029207f445ab2d448f917`;
the operator's option-B decision export remained SHA-256
`c0c63a5b76bbb733e184cbfad535e775bc7c52a092a85816e255d5186444bc61`.
The service used a patched, locally built wheel with SHA-256
`630af744f834bea1104275bc04fbd1d4bb38d6acbaa9fbfbd231cf685c6ee399`.
After the fatal stop, the final wheel (with a persistent three-replacement
limit, duplicate-controller takeover guard and clearer diagnostic wording)
was rebuilt and installed locally, with an additional server-identity guard;
SHA-256 `cad43a0824080ad90994bf2c28311e02988430a1897b73e4b26b0c5956c1828f`.
The live service did not run that final rebuild. Previous local wheels are retained
under ignored `.local/milestone5/supervision-wheelhouse/`.

Codex `critique-1` completed once in visible pane `wA:p3`; Claude
`revision-1` completed once in `wA:p4`. Each has one validated native receipt
and one invocation counter. Total native stage attempts: **3 of 5** (draft,
first critique, first revision). No second critique or revision was launched.
The recorded argv requested `sonnet`/medium for both Claude calls; their
native init events reported `claude-sonnet-5-5`. Codex argv requested
`gpt-6.1-sol` with `model_reasoning_effort="high"`; its native event stream did
not independently report a resolved model identifier.
The revision result addressed findings `C1-1` and `C1-4` but omitted open
findings `C1-2` and `C1-3`; the workflow refused adoption with
`Revision omitted an open finding`. The run status is `uncertain`, with graph
completed stages `['draft', 'critique-1']` and no pending human gate. The
completed revision receipt and output remain intact. This is a deterministic
contract failure after a completed call, not evidence of an ambiguous child
launch; current status classification is conservative. No retry or additional
model call occurred. Final specification and approval have not occurred.

The supervisor appended a durable notice and invoked Herdr's supported
`notification.show` API; visual receipt in a client was not observed. A second
notice records the specific revision failure, which the installed `status`
command displays. Its
unit `wayward-wrangler-0c2d2965-decision-log-01.service` is inactive and
disabled after the fatal stop, so login will not repeat notices. The unit file
remains user-local for review. Its `ExecStart` points to the installed wheel's
project-local pipx environment inside this checkout; it imports site-packages,
but moving that test environment would require updating the unit. This is not
the future user-local installation path. Do not run `demo`, `resume` or re-enable the
service for this run without a separately reviewed recovery of the revision
failure. No output was altered or receipt manufactured.

Offline: 85 unit tests passed, including nine focused supervision tests.
Isolated Herdr test
`.local/milestone5/supervision-live-05/result.json` reproduced a missing
workspace after a record-only simulated answer, recovered it, confirmed all
five fake worker counters stayed at one, reused an existing controller after
supervisor process restart, replaced a lost controller with generation 2, and
verified the supported `notification.show` API. The isolated server was
stopped; no shared Herdr server was restarted. Missing worker reports and
ambiguous dispatch refusal are covered by offline recovery tests. A changed
server identity refuses workspace-ID reuse in the focused tests. A real
`systemd --user` unit was installed and observed active for the trial; its
crash/restart behavior was not deliberately exercised on the live native run.

Remaining limitations: the full conversational TUI, task queue, coordinator
provider switching, automatic orphan resolution and arbitrary power-loss
recovery are deferred. The immediate native trial cannot progress through its
failed revision without a new, explicit recovery decision. Earlier `w9` pane
IDs are historical and may no longer be live.

The installed shared plugin points to the pipx-bound copied assets; rollback
information is local at
`.local/milestone5/plugin-old-binding.json`.

## Revision quality recovery and bounded continuation

The following account records the earlier quality stop as it occurred. The
subsequent finding-protocol recovery and current state are recorded below.

The original `revision-1` refusal was a workflow-quality error, not ambiguous
execution. The completed Claude receipt binds saved `result.json` SHA-256
`c06deb4b48928126b27076cd1ceb0528170e2465013448fb0b5c401039dcc080`.
Its dispositions addressed C1-1 and C1-4 but omitted C1-2 and C1-3.
Inspection of the exact text found both issues explicitly provisional:
the minimum number of alternatives and reversal/retargeting of supersession.
The text did not choose either policy. Their original IDs, evidence, origin,
reviewed draft and open status are retained. The revision also proposed
C2-1, C2-2 and C2-3; these remained open. This is a usable, incomplete
candidate, not a resolved specification.

The corrected policy records `revision-incomplete` coverage in an immutable
quality record and passes the original open finding records to the next critic.
Its `revision_coverage` packet field and authority text explicitly name the
omitted dispositions and preserve evidence. Unknown or duplicate finding
identities are `output-invalid`; uncertain execution remains reserved for
unresolved launch/completion identity. Output-invalid completed calls cannot
be retried or used to dispatch dependent stages. Final review, if reached,
shows all remaining open findings and the exact artifact binding.

Eight focused offline tests cover missing dispositions, original evidence,
duplicate IDs, restart around the recovery record, mismatched evidence, the
second-review route and the five-stage boundary. The full offline suite passed
92 tests before live continuation; the final source passes 94 tests. The installed wheel used for continuation
was built locally from this branch, SHA-256
`a055b25e61df85f7985bbc35c0131339e4c64505613b42bec5c0eddabfacce6b`.
The final status, diagnostic and stopped-controller update was built and
installed as wheel SHA-256
`1d5f65117168e6995843953d5aea71f37886e8ddd30e08c364ac258b816bd6fa`.
From its installed site-packages, a read-only verifier checked the frozen
packet, prompt and schema, native completion, single invocation, exact output
hash, original rejection and graph replay boundary. It found only C1-2 and
C1-3 missing from revision-1 dispositions. The operator-authorized
`operations/revision-1/recovery/quality-adoption.json` appends hashes of the
original files, the correction, omitted finding records and authorization;
it changes none of the original result, receipt, artifact or notice bytes.
The application set status to recovering, and the owned supervisor replayed
the saved completed revision through LangGraph. The original revision-1
invocation count stayed one. `quality/revision-1.json` records the provisional
coverage. `critique-2/packet.json` contained C1-2 and C1-3 with their original
evidence, C2-1 through C2-3 as open, and a remaining stage budget of two.

Codex `critique-2` completed once in visible Herdr pane `wA:p6` / tab `wA:t6`.
Its validated native receipt binds result SHA-256
`17e13355c99e773892a6c61634b30b57268a114fcdb7401d468053eef774bf55`.
The result returned findings under already-used IDs C1-2, C1-3, C2-1, C2-2
and C2-3. The workflow could not safely treat these as distinct new findings
or substitute their evidence for the original records. It classified the
semantic identity collision as `output-invalid`, retained the full output,
receipt and worker pane, and did not checkpoint critique-2 or dispatch
revision-2. The provisional accepted candidate remains
`artifacts/revision-1.md`, SHA-256
`a56ec9217514907254f2e61e3f50e99a32705dfc1f9fbaff41e1f6c3b2b29065`.
`artifacts/critique-2.md` was published before the semantic rejection in the
old processing order; it is retained as rejected evidence, not an accepted
graph artifact. Validation now precedes publication for future outputs. An
append-only `quality/critique-2-invalid.json` binds the receipt, packet,
result hash and precise duplicate-ID reason. The current status and latest
notice name C1-2 as the first collision, and point to saved output inspection.

Actual native stage calls are **4/5**: draft, critique-1, revision-1 and
critique-2, each once. One authorized slot remains unused; no further call is
safe from the current rejected graph boundary. The operator's B answer and
its recorded hash are unchanged. At that earlier stop, no final approval or browser application
existed. The run status is `output-invalid`, with the exact condition visible
through `status` and a durable notice sent through Herdr's notification API
(visual delivery was not observed); the supervisor service
remains active so inspection is available. The current controller pane and
completed worker panes are retained. A future recovery of critique-2 would
require separate review of its duplicate finding semantics; it must not be
assumed from this authorization.
The stopped controller is `wA:t5`; completed critique-2 is `wA:t6`.
Ignored `.local/milestone5/quality-recovery-verification.json` records the
stage counters, receipt/output hashes, B decision export, recovery records and
graph boundary. It confirms four calls and no `revision-2` operation.
The stopped run keeps the single active-run slot occupied until a separate,
explicit resolution; starting a different run cannot bypass the quality stop.

## Finding-protocol correction and authorized continuation

The saved `critique-2` prompt (`d32cc2e8ec006188d4eb919d0e45191f577bea300e22441c9e5c9343a6e0881b`), schema (`556f634c9551dac57a48c158bc1a10f80026285d74718b04bf33f9fcb6dcd7fc`) and packet (`9d816b20d7e97fb86a1d87d74d9b3db33286f975e009e9d92b6abbcd7a1c0865`) used the legacy finding contract. It allowed `findings` and `dispositions` but gave no explicit field for reassessing an existing finding. The packet included the five original open findings and their evidence. In the exact saved result (`17e13355c99e773892a6c61634b30b57268a114fcdb7401d468053eef774bf55`), C1-2, C1-3 and C2-1 through C2-3 say to keep or leave the same issues open, citing the existing provisional sections. They are reassessments, not unrelated new findings. Dispositions for C1-1 and C1-4 reverify their addressed status. No structured new issue or question was returned. The legacy duplicate-ID rejection was correct under its schema but exposed the missing reassessment category; its receipt, output, original rejection, and prompt bytes remain unchanged.

Future packets use finding contract v2: `new_findings` have no model-chosen identity and receive an application-owned durable ID from operation identity and output index; `assessments` explicitly reference an existing finding and carry evidence, rationale and status. Original evidence is immutable, while stage-specific assessments are appended to `quality/<stage>-finding-ledger.json` and graph history. A model's `proposed-addressed` assessment does not silently close a finding. Unknown, conflicting or ambiguous references remain invalid. The legacy format remains readable for historical runs. Offline fixtures contain the byte-identical saved result and provenance; regression checks cover reassessment, fresh IDs, conflicting references, replay idempotency and original evidence.

The operator authorized an explicit, one-time interpretation of the saved critique. The installed verifier checked the frozen input, operation, prompt, schema, one completed native call, receipt/result integrity, original quality rejection and graph replay boundary. `.local/milestone5/critique2-interpretation.json` maps each of the five reused entries to an open assessment with an exact saved-text quote and item hash. `operations/critique-2/recovery/finding-interpretation.json` appends the interpretation and evidence hashes (record SHA-256 `baf199137c2e0e8583408615c3d510e75c312e89b0d99c44fe94cd3b5dc886fc`). A disposable replay proved checkpoint adoption, v2 revision packet construction and no revision-2 launch before live adoption. The installed controller then adopted critique-2 through LangGraph without a new critic call; `quality/critique-2-finding-ledger.json` records seven assessments (five open, two reverified addressed). The supervisor retired the held controller and created a replacement; no worker was repeated.

The replacement launched the **one remaining authorized call**, Claude `revision-2` with frozen `sonnet`/medium settings, in visible Herdr pane `wA:p8` / tab `wA:t8`. Its completed receipt binds result SHA-256 `dba65462bebe618ae2f96b224279c6d5ae2c09ac7332cc80bcb0e9cc05725d58`. It returned five explicit open assessments and zero new findings. The final artifact is `artifacts/revision-2.md`, SHA-256 `021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`. All five native stage counters are exactly one: draft, critique-1, revision-1, critique-2 and revision-2. Two critiques were used. There is no further critique after revision-2 and no model budget remains. The accepted editing decision B and source artifacts were preserved.

At completion of the native stages, the run was `awaiting-input` at final review. The specification is **provisional**: C1-2/C2-1 (minimum alternatives), C1-3/C2-2 (supersession relationship correction) and C2-3 (cross-tab behavior) remain open. These are substantive product choices not settled by the editing decision, and the authorized question budget was exhausted. The application shows the candidate, its exact version, accepted decision, all open findings and appended assessments. The final gate required inspection of this exact artifact before approval or rejection. The operator later approved this exact provisional revision through the controller. This milestone did not build or browser-test the application. The old failure notice remains in durable history; status no longer displays it as the current condition after recovery.

The continuation used ignored wheel `.local/milestone5/finding-protocol-wheelhouse/wayward_wrangler-0.3.0-py3-none-any.whl`, SHA-256 `3ac70d67ed83a8a02c9a49a99d64ed332223b6d1d077bb5d2e756a4aba532ed7`. The final installed wheel, including the corrected status display, is `.local/milestone5/finding-protocol-final-wheelhouse/wayward_wrangler-0.3.0-py3-none-any.whl`, SHA-256 `672cd1298c8b967e16f6fcf84a8ad18f0b0999b7aac60ea0f1bf6a767d1deaab`. The full offline suite passed 102 tests after the correction. Installed status first reported `awaiting-input` with the five open findings, without presenting the historical rejection notice as a current failure. The shared review plugin and Herdr server were not changed during this recovery. The owned service was restarted only to refresh its installed package and is active for inspection; worker panes remain retained.

## Final human approval

The controller recorded response `A1` as `approved`, with `artifact_sha256`
`021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`,
matching the current bytes of `artifacts/revision-2.md`. The saved response
retains decision revision 1 and the frozen context digest
`096f09776d5fad5398e8a0912ab5d66b6d5b70b0bd4ca0c35c39944dba43bcb9`.
The graph and installed CLI report `approved`, all five native stages completed,
and no pending gate. This approval was already in the controller's durable
record when checked; no duplicate response was submitted. It accepts the
provisional specification with C1-2, C1-3, C2-1, C2-2 and C2-3 still open.
It does not verify a working browser application or authorize another model
stage.

## Final-build closeout verification

Built the current branch as wheel 0.3.0, SHA-256
`021e2eb50d3be48df52710f7cf924cfb43f9f829991132151628e2e5552e90a6`,
and installed it with pinned dependencies in fresh disposable
`/tmp/ww-m5-closeout-venv`. The focused 40-test suite imported
`agent_workflow` from that venv's `site-packages`, not the checkout. It
covered application-owned new-finding IDs, explicit existing-finding
assessments, immutable original evidence, incomplete revision carry-forward,
ambiguous-reference refusal with a durable actionable `output-invalid` notice,
replay and recovery boundaries. `pip check` passed. No authenticated CLI was
called.

The same installed CLI and package ran deterministic fake workers against an
independent disposable Herdr server and project. Six checks passed: supported
notification; missing workspace before launch with no worker retry; automatic
workspace/controller recovery with five stage counters each equal to one;
supervisor restart reusing its controller; controller-loss replacement without
duplicate calls; and stale SHA refusal followed by exact-artifact approval
across another controller restart. The final simulated run was `approved`; its
question answer and approval survived restart. The test-owned Herdr server
stopped. The first closeout attempt used a manual `resume` on a supervised run;
that was correctly refused as unowned. The harness was corrected to reconnect
through `supervise`, and the fresh-state rerun passed. No production code defect
was exposed by closeout review. Ignored retained evidence:
`.local/milestone5/closeout/isolated-herdr-result.json` and
`installed-focused-tests.log` (copies matched the `/tmp` originals by SHA-256).

This final-build simulated verification is **separate** from `decision-log-01`.
The native trial completed through explicit saved-output and infrastructure
recoveries across multiple wheel builds. It was not an uninterrupted end-to-end
execution of this final build. Read-only closeout checks confirmed its approval
response still binds revision-2 SHA-256
`021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb`;
all five original native invocation counters remain one, and the original
quality rejection plus both adoption records remain present. The specification
remains provisional on alternatives minimum, supersession relationship
correction and cross-tab behavior. Revision 2 received no further independent
critique. Specification approval is not verification of a working web app.

At closeout, the shared Herdr server and workspace `wA` were reachable. The
owned controller process remains in `wA:t7` / `wA:p7`, showing the terminal
result. Completed workers remain inspectable in `wA:t3`, `wA:t4`, `wA:t6` and
`wA:t8`; the earlier `w9:t3` draft pane is closed. The supervisor unit is
enabled but inactive after its successful terminal exit, so it is not an
ongoing monitor for this approved run. No worker process was restarted or
closed for closeout. The TUI/coordinator is still a product target.

Follow-up priorities, not implemented here: (1) standalone interactive
startup, reconnect and coordinator interface; (2) multiple durable human
questions, with question handling independent of model-call budgets; and
(3) carrying this approved specification and its unresolved choices into a
subsequent task. No browser application was built.
