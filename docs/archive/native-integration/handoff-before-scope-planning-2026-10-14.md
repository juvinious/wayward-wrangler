> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-14)

## Current checkpoint and authorization

Branch `feat/native-coordinator-integration`; base/main
`2e5cf1212fe9a319b118c641c5028c851f66e86d` (merged PR #5).
Packaging, scoped cleanup and documentation are complete. Review PR:
GitHub #6, targeting main,
with the full unmerged branch scope. Closeout checkpoint `d12de89` is pushed.
**Merge and publication are not authorized.** Stop for review.
The three-call Claude trial is complete and exhausted. No further authenticated
call, retry, fallback, model/route substitution, budget renewal, workers or
consultation. Do not create another project to bypass the ceiling. No subagents.

Live Claude conversation is verified **only for this bounded trial**. Normal text
reaches the selected LLM unchanged through the accepted production TUI and existing
supervisor. No phrase matching or fake fallback. Worker execution, consultation
and a working generated application are not verified. No operator/global cutover.

## Relevant commits and complete PR scope

Earlier unmerged `fa90340`, `f620700`, `a277cfe`, `574b342`, `81cf6a8`: capability
contract/failed Codex proof; schema-4 mock conversation/questions/action proposals,
budgets and static-file workflow/manifest review/approval foundations. These are
mock-only, not native worker or browser acceptance. The PR must include them.
Focused conversation starts at protected `8e08a6b`; independent Claude proof
`5333993`, host/TUI `b730aa6`; reconnect/draft and receipt/cancellation safeguards
`ac013b2`, `74e6598`, `332171f`, `1ceabf9`, `717348b`.
V5 runtime source `717348b4ae88e618a3768d9b226ec26ad91329be`;
operator authorization `8db8853`; Markdown fix/v6 runtime source
`dd94d8e8bb7c176503c842ef487b9bead11ab02a`.

## Verified evidence versus operator observation

**Operator:** “Looks good so far” after two replies; reported detach/reattach/resume
after the third, and literal Markdown markers. No broader acceptance inferred.
**Read-only audit:** 3 completed native discussion calls, 3 attempts/spawns and
3 matching exit-0 receipts; 6 messages, zero tasks/workers/consultation. Exact
binding, packet/result/message and runner/claim/permit/invocation identities match.
Third packet preserved four prior messages. Native init: Claude Code 2.1.296,
claude-sonnet-5-5, dontAsk, empty tools/MCP/skills; argv medium effort.
No developer model call. Authoritative allowance is 3/3 consumed.

**Automated:** 15 pinned-client no-egress/dummy-credential boundary cases;
prior installed mock transport/restart/interruption/exhaustion/replay; status
<1 s, SQLite busy <500 ms source gates. Markdown/input suite 53 passed/1 prior
probe ignored; final focused rich-text and IPC checks passed. Installed v6 passed
normal closed-admission init/start/resume/detach and read-only saved/synthetic UI
replay at 80×24/120×60, provisional text, code and draft/terminal restoration.
These checks do not establish physical v6 acceptance or native crash recovery.

## Installed candidates and reinspection

**Fix packaged in v6:**
`.local/claude-conversation/install-01/releases/claude-conversation-v6/`.
Manifest `dea43989715b5ac837f71f5b6471cc69f46741bc5addc55b063047e2935fdfae`.
Only frontend binary differs from v5; service/runner/resources identical.
Rust/Cargo 1.99.0 pinned, locked/offline, Cargo.lock unchanged. No Python backend.
This side-by-side installation is not selected globally or bound to the live trial.

**Preserved live project remains v5:** `.local/claude-conversation/live-eval/`,
key `<private-id>`, manifest
`ae55df4c9a259adf16506a75981179c8a01a2d4d3fbba4bc5b9b4137dbeafe6d`.
Its conversation, authorization, 3/3 exhaustion, receipts and binding are intact.
Fish reinspection (automatic compatible service startup; no allowance renewal):

```fish
~/projects/example-workspace/.local/claude-conversation/waywr-live resume
```

Do not silently upgrade it to v6. Applying v6 to this project requires a separate
explicit compatible release change; no operator action is needed for closeout.
If reopened, `/detach` closes only the interface; `/exit` fences and checkpoints.

## Resource disposition and evidence

UI claims were stale; no operator TUI attached. Project lifecycle already paused,
service `waywr-<private-id>.service` inactive/MainPID=0;
not restarted/changed. Three empty completed transient hosts stopped after exact
receipt/unit/argv/identity/cgroup checks. No trial record/socket/lock removed.
The new startup fixture service is inactive; replay peer/UI processes ended.
Removed only `bundle-v1` through `bundle-v6`: 84 file hashes matched retained
installations; manifest copies/provenance preserved, no active dependency.
All installed releases, evidence, shared build cache and operator projects retained.

Ignored evidence under `.local/claude-conversation/`: `boundary-05/`,
`installed-final-05/`, `live-verification-2026-10-13/`, `packaged-v6/` (build receipt,
startup and rendering-02 summaries; original failed rendering assertion retained),
`closeout-2026-10-14/` (exact cleanup/staging inventory). Original runtime paths stay
immutable; removed staging paths are explained by inventory, not rewritten.

## Protected resources, limits and next action

Preserve r9, historical decisions/approvals/receipts, earlier schema-3/4 and
LangGraph releases/services, accepted v3 evaluations, prior mock operator project
`.local/native-integration/final/operator-eval/` (native-mock-c4-final4), and blocked
`~/projects/waywr-rust-sim-eval`. No shared Herdr changes or cleanup.
After accepted work retain compatible releases and forward-repair; never restore
old snapshots or bind incompatible code. Code rollback alone is insufficient.

Native interruption/crash/timing, wider account policy, whole-process confinement,
physical exit/v6 rendering and generated application remain unverified. Model
prose uses a pre-reservation budget snapshot; UI/ledger remains authoritative.
Codex/broader-role admission remains blocked; [limits](current-limits.md) links
original evidence. No additional native authorization exists.

Next proposed increment: connect live discussion to durable task proposals and
explicit scope authorization. **Do not plan or implement it during this closeout.**
The PR is open; no checked-in CI workflow or returned PR status checks. The private
repository branch-protection API returned the plan-feature 403; no merge gate was
bypassed or merge attempted. Stop for review, with no further implementation.
Read [AGENTS.md](../AGENTS.md), this handoff,
[Claude contract](claude-coordinator-contract.md), [report](claude-conversation-report.md),
then [TUI](tui-contract.md), [state](state-contract.md), [IPC](session-protocol.md),
[execution](execution-contract.md) as needed. [Index](README.md) owns current/archive
classification; retrieve archived planning/boundary evidence selectively.
