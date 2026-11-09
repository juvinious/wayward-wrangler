> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Current state

Branch: `refactor/supervisor-owned-execution`, pushed with
PR #5 targeting `main`.
This branch contains the
unmerged Python-to-Rust migration, Rust runtime/CLI corrections, direct TUI
startup, local init/reinit corrections, and four TUI-first host-execution
checkpoints. It is not merely a UI patch. `origin/main` is the PR base; no
merge or package publication is authorized by this handoff. GitHub reported
the PR mergeable with no status checks at creation. Its private-repository
branch-protection API returned HTTP 403 due to the account plan; that call
cannot establish a required merge gate.

Approved design base: `dafb4f8451c725611091e0b3b264ffe73e02b107`.
Host checkpoints: `39406a6` (isolated systemd proof), `39d76d3` (host
primitives), `be09765` (Herdr-free workflow and prelaunch recovery), `0dc2203`
(inspection and optional Herdr views), `78e8fa5` (installed simulated
acceptance). IPC inventory: `6a5700e` and `f709486`; final discoverability
corrections: `c34faf8` and `fe67c73`. The acceptance record is
[tui-first-checkpoint-4-report.md](tui-first-checkpoint-4-report.md).

The current Rust frontend/backend use project-owned systemd user services for
new schema-3 execution. The accepted conversation-first TUI runs in the
invoking terminal; `init` is local-only and `resume` reconciles. Herdr is
optional, explicitly requested, read-only inspection. The service owns task,
question, decision, approval, context, receipt, budget and recovery state.
Each project has independent state and service; one workflow executes per
project. Existing Herdr-backed and unfinished LangGraph runs keep their bound
legacy releases. Native Claude/Codex admission and context/tool isolation are
not accepted.

## Accepted simulated evaluation

The operator completed the **v3 simulated** weather-page workflow in a
physical terminal and reported the next-action, answer, candidate-review and
bounded-scrolling corrections present and verified. The selected task reached
`approved` with **View result** visible. This is operator observation,
separate from automated tests. It does not verify a working weather app or
native agents. Physical v3 detach/resume was not reported.

The local fake-only candidate is
`.local/supervisor-owned-execution/discoverability-install-01/releases/rust-host-discoverability-v3/bin/waywr`,
release `rust-host-discoverability-v3`, manifest SHA-256
`c0bf66e31579cb44a04cb97f3701ff47ccaff4fd74dedf1190445240e1e0e5a1`.
It is not globally installed or the default. The retained v3 project is
`.local/supervisor-owned-execution/discoverability-hands-on-v3/project/`, key
`<private-id>`, bound to v3. Its approved fake
candidate is `state/runs/<private-record>/artifacts/revision-2.md`
under that project's `.wayward-wrangler/`. The v2 project and bound v2
release remain inspectable; the earlier `rust-host-stage4-v3` operator project
was not touched.

Focused verification previously passed: 44 Rust frontend library tests, 16
binary tests (one preexisting timing probe ignored), and installed fake-only
PTY checks `discoverability-check-06/` and `-07/` (one coordinator plus five
fake worker calls each). The minor-1 schema/source consistency check passed
again at closeout. The schema describes implemented method inventory and
capabilities; `operation.retry_start` is a command action and Herdr views use
prepare/send/bind. The current [session protocol](session-protocol.md) is
consistent with the checked-in schema and service method list.

## Resource cleanup and preserved evidence

The bound v2 and v3 `session stop` commands completed: both sessions are
`paused` at generation 1/state revision 12, and their exact project services
are inactive. Each project retains six completed fake-call receipts. All 12
corresponding operation units have `MainPID=0`, empty cgroups and retained
`active/exited` metadata; no owned worker remains. All seven exact automated
fixture services are inactive. Three byte-identical, unreferenced staging
bundles were removed; their v1/v2/v3 installed releases remain. The empty v2
private runtime directory was removed. The v3 private runtime directory and
its foreground TUI remain: the TUI was still attached to an operator terminal
and was not killed to avoid corrupting terminal state. The operator can use
`/detach` in that UI; no worker is attached to it. No shared Herdr resource
was changed. The compact inventory is in the active report; ignored details
are in `.local/supervisor-owned-execution/discoverability-cleanup-2026-10-13.json`.

Retain `.local/supervisor-owned-execution/discoverability-check-{01..07}/`,
`discoverability-hands-on{,-v3}/`, both bound releases, approved artifacts,
receipts, transcripts, decisions and operation metadata. The earlier
`.local/supervisor-owned-execution/hands-on/project/` and separate blocked
legacy `~/projects/waywr-rust-sim-eval` are outside this cleanup.
Also preserve `decision-log-01`, `context-01`, `m2-cli-01`, `demo`, the r9
comparison baseline and all historical bytes, hashes, approvals and recovery
records. No operator service cutover was performed.

## Reopen and rollback

From Fish, first detach any still-attached v3 foreground UI in its operator
terminal. Then:

```fish
cd ~/projects/example-workspace/.local/supervisor-owned-execution/discoverability-hands-on-v3/project
../waywr.fish session status
../waywr.fish resume
```

The wrapper pins its release/private environment. Do not reinitialize or
rebind this project. New schema-3 release rollback requires reconciliation
of its exact service and host units and a compatible state fingerprint. Never
restore an old snapshot over accepted commands, decisions, approvals,
attempts or receipts. An uncertain operation is never blindly replayed.
Retain any release referenced by a project; historical legacy contracts stay
bound to their original releases.

## Next task and reading order

This checkpoint is open for PR review against `main`. Do not merge or publish
until separately authorized. The **next task**, once requested, is planning
native conversational coordinator and worker integration; do not begin that
planning or authenticated calls as part of this closeout. Other unverified
boundaries include native context/tool isolation, a real application artifact,
physical v3 detach/resume, logout/reboot survival and orphan stream
reattachment.

Read [docs index](README.md), [active acceptance report](tui-first-checkpoint-4-report.md),
[TUI contract](tui-contract.md), [architecture](architecture.md), [state
contract](state-contract.md), [session protocol](session-protocol.md), and
[approved TUI-first transition](tui-first-optional-herdr-proposal.md).
Consult [bootstrap refusal](bootstrap-refusal-correction.md) or archived
reports only for a specific legacy or evidence question.
