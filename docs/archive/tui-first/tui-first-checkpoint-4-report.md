> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# TUI-first simulated acceptance — current checkpoint

## Accepted result (2026-10-13)

The operator completed the v3 **simulated** weather-page workflow in a physical
terminal and confirmed that the requested next-action, answer, candidate-review,
and bounded-scrolling corrections were present. The selected fake task reached
`approved`, with **View result** available. This is operator observation,
separate from the automated checks below. It does **not** verify a working
weather application or authenticated coordinator/worker execution. Physical
v3 detach/resume was not reported.

The fake coordinator deterministically proposes submitted text; its retention
question is scripted, unrelated to the weather request. Approval was bound to
the retained simulated artifact, not to a built application. The exact approved
candidate remains at ignored
`.local/supervisor-owned-execution/discoverability-hands-on-v3/project/.wayward-wrangler/state/runs/<private-record>/artifacts/revision-2.md`.
Its project key is `<private-id>`.

## Installed release and verification

The accepted local, fake-only release is
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`
(`rust-host-discoverability-v3`, manifest SHA-256
`c0bf66e31579cb44a04cb97f3701ff47ccaff4fd74dedf1190445240e1e0e5a1`).
It is neither global nor the default installation. The v2 and earlier
`rust-host-stage4-v3` installations remain available for their bound projects.

- The v2 correction derives **Review scope**, **Answer question**, **Review
  candidate**, and **View result** from authoritative task detail. Empty
  composer Enter or focused-action Enter opens the panel without authorizing
  work or recording a decision. The exact candidate must be fully read before
  Approve/Reject appears; changed candidates invalidate the local read marker.
- The v3 correction hides Save when no choice question is pending, moves content
  panels one line with Up/Down, and clamps scrolling at the rendered end.
- Focused Rust tests passed: 44 frontend library and 16 binary tests; one
  preexisting timing probe was ignored. Installed fake-only PTY evidence in
  ignored `discoverability-check-06/` (v2) and `-07/` (v3) covers scope,
  question, exact-candidate review, approval, result inspection, detach and
  terminal restoration. Each used one fake coordinator and five fake stages.
  This automated PTY evidence is separate from the operator's physical v3
  observation.
- The protocol-v1/minor-1 schema inventory and implemented service surface
  were reconciled in `6a5700e` and `f709486`. The focused
  `python3 tests/session_schema_consistency.py` check passed again at closeout.
  `operation.retry_start` remains a command action; optional Herdr view opening
  retains its prepare/send/bind handshake. No protocol behavior was added.

The original detailed report, including superseded v2 walkthroughs and
historical correction analysis, is preserved in
[the dated archive](tui-first-checkpoint-4-pre-closeout-2026-10-13.md).
The [execution log](tui-first-execution-log.md) carries the
four host-transition checkpoints and earlier failed attempts.

## Cleanup inventory (2026-10-13)

| Resource | Outcome |
| --- | --- |
| v2 project `<private-id>` and v3 project `<private-id>` | Both had approved tasks and six completed fake-call receipts each. Bound `session stop` commands reached durable `paused` generation 1, state revision 12. Exact project services are inactive. |
| Their 12 operation units | All had completed receipts, `MainPID=0`, empty control groups and `active/exited` unit metadata. No owned worker remains; unit metadata and all application evidence were retained. |
| `discoverability-check-{01..07}` | Exact recorded fixture services are inactive with `MainPID=0`. Fixture records and traces remain ignored under `.local/`. |
| v1/v2/v3 discoverability staging bundles | Three byte-identical, unreferenced bundle copies (14 files each) were removed after comparison with retained installed releases. Ignored `discoverability-cleanup-2026-10-13.json` records paths and manifest hashes. |
| Private runtime paths | The empty v2 `/tmp/ww-discoverability-eval-runtime/` was removed. The v3 private runtime path remains because its foreground TUI is still attached. No stale socket was found. |
| v3 foreground TUI | Its exact process was still attached to an operator terminal after project-service shutdown. It was left running to avoid an ungraceful terminal teardown. It owns no worker and can be detached by the operator with `/detach`. |

The approved v2/v3 project directories, release bindings, binaries, artifacts,
decisions, receipts, transcripts and historical failure/recovery evidence were
not removed. The earlier operator evaluation, separate blocked legacy project,
shared Herdr server, and unrelated services/panes were not touched. There is
no unresolved **worker** execution in the selected evaluations.

## One reusable hands-on project

To inspect or resume the preserved v3 evaluation from Fish, using its bound
release and private environment:

```fish
cd ~/projects/example-workspace/.local/supervisor-owned-execution/discoverability-hands-on-v3/project
../waywr.fish session status
../waywr.fish resume
```

`resume` intentionally restarts only this paused project service and
reconciles its saved session; it does not repeat approved calls. The existing
foreground TUI may still be attached in its operator terminal. Return to it
and use `/detach` before opening another UI attachment, or inspect the saved
state with the offline `session status` command. Do not reinitialize or rebind
the project. The current release remains fake-only.

## Remaining boundaries

Native Claude/Codex behavior, model-backed conversation, native context/tool
isolation, a working weather application, physical v3 detach/resume, and all
possible failure windows remain unverified. No global installation, service
cutover, package publication, or shared Herdr change occurred. Historical
LangGraph and Herdr-backed runs keep their bound execution contracts. New
schema-3 project rollback is state-aware: never restore an old snapshot over
accepted commands, decisions, attempts or receipts; retain referenced releases
and reconcile the exact service/host units before changing a binding.
