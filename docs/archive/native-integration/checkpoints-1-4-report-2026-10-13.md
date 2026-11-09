> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Native coordinator integration — implementation record

Scope: approved checkpoints 1A, bounded 1B, and 2–4 on
`feat/native-coordinator-integration` from amendment `167b983834413a3997fd5eaa583794c4165f7fa5`.
Checkpoint 5 and authenticated calls are excluded. The [proposal](coordinator-worker-proposal-through-live-check-2026-10-14.md)
owns intended behavior and the [boundary assessment](boundary-assessment-through-live-check-2026-10-14.md)
owns native admission criteria. This report records executed checks, not projected results.

| Gate | Code/state boundary | Result and evidence | Rollback |
| --- | --- | --- | --- |
| 1A capability/result contract | Base `167b983`; no persistent schema or operator-state change | `backend/src/native_contract.rs` fixes `static-file-packet-v1`, 6+1+4 launch ceilings, two reviews, a strict `coordinator-turn-v1` result/action type, classified utilities and compile-time closed native admission. `cargo test --offline --manifest-path backend/Cargo.toml native_contract --lib`: 2 passed. Repository-wide `cargo fmt --check` encountered preexisting formatting differences; only the new file was formatted, then tests passed. | Revert the 1A commit; no state migration |
| 1B effective provider boundary | 1A commit `fa90340`; only ignored disposable fixtures, no operator state | **Failed for the pinned Codex profile at B1.** A no-egress, credential-free container ran Codex 0.162.1 app-server with a private home, dummy key, explicit empty environments and disabled optional features. Its first mock outbound request nevertheless contained a model-visible `additional_tools` item advertising `functions.exec`, input tools and collaboration/spawn tools. These are outside `static-file-packet-v1`. No tool was exercised and no provider call was authenticated. B2–B7 and Claude effective profile remain unverified; Codex native admission stays closed. Evidence: ignored `.local/native-capability-proof/2026-10-13/` (details below). | Retain the proof bytes and failed-admission disposition; no application state rollback |
| 2 conversation authority | Base `f620700`; schema-4 only in new isolated state; schema-3 stays at migration 3 | Schema-4 conversation messages, turn reservations/results, questions/answers, binding generations, action proposals and 6+1+4 durable charges use the existing project SQLite writer. A validated `propose_task` result creates one unadmitted task; discussion creates none. Minor-2 `conversation_v1` read and snapshot routing are capability-scoped. Seven focused conversation tests, one schema-4 service routing test, five schema-3 session tests, existing SQLite busy and slow-client tests, and IPC schema consistency passed. No installed UI or model execution claim. | Before schema-4 admission, remove only isolated fixtures. After admission retain this compatible release and forward-repair; do not restore a prior snapshot or route schema-4 state to schema-3 code |
| 3 integrated coordinator | Base `a277cfe`; fresh schema-4 projects only, schema-3 projects keep their bound contract | The installed mock-only TUI separates discussion, task requests and steering intent; it routes durable questions, explicit consultation, scope review and fake binding replacement. A versioned project handoff includes accepted decisions, task/operation references and budgets in frozen packets. Pre-authorization steering creates a linked proposal and preserves the original. A saved mock result is adopted once after restart; missing completion evidence becomes uncertain without redispatch. Focused service/UI tests and installed PTYs passed. The first installed c3 fixture exposed a stale command revision after snapshot refresh; its refusal is retained. c3b corrected it and completed question → answer → consultation → proposal → detach, service restart, `/exit` and `resume`. c3c verified the later steering/handoff code through installed startup. No authenticated provider was launched. | Keep c3/c3b/c3c fixture state and paired releases for inspection. New schema-4 state cannot be opened by a schema-3 release. A code rollback after acceptance requires keeping a compatible release and forward repair, never restoring old state over newer actions |
| 4 static files/review | Base `574b342`; schema-4 isolated fixtures only, no operator cutover | **Passed the approved mock-only gate.** `static-files-v1` uses four allowlisted UTF-8 files, a canonical immutable manifest, application IDs for findings, appended author dispositions and independent review assessments, a four-task-call/two-review ceiling, durable worker questions, verified per-file/bundle reads, and exact-manifest approval. Source-matched `native-mock-c4-final4` passed installed TUI discussion → clarification → consultation → scope authorization → four supervised stages → final decision; targeted integrity/compatibility tests passed. Evidence below. | Preserve bound side-by-side releases and accepted fixture state. A schema-4 rollback changes only a quiescent compatible release binding with the full state fingerprint; never restore an old snapshot over accepted actions. |

Checkpoint-4 source and documentation were committed at `81cf6a8` after the
source-matched installed fake-only pass. The 1A module is not a provider dispatcher. The installed schema-3 release remains
unchanged; mock task authorization cannot dispatch a native provider through 1A.

## Operator mock-evaluation finding (2026-10-13)

The operator's retained `operator-eval` project is bound to
`native-mock-c4-final4` (key `<private-id>`). A
read-only SQLite check found four completed mock conversation turns, one saved
question and **zero tasks**. The ordinary message “ok proceed” consumed a turn
and produced the fixture's generic scripted fallback. This is not natural
conversation or a passing operator acceptance of the intended coordinator.
`backend/src/mock_coordinator.rs` uses phrase matching solely for deterministic
tests; `session_service::tick_conversation` currently calls it unconditionally
for schema-4 conversation. Do not expand those phrases to emulate an LLM.

The screenshot also showed “Connected” in the header and “not connected” at
the composer. The header reflects project-service IPC, while the composer had
a hard-coded live-mode label. The source label now distinguishes a connected
**mock** coordinator from IPC reconnecting, with one focused UI regression test
passing. The installed `native-mock-c4-final4` binary and operator state have
not been changed. Native coordinator routing is still blocked by the 1B
capability boundary and separate authenticated-call authorization; this UI
correction is not a native-readiness claim.

## Checkpoint 3 installed evidence

The side-by-side bundles are under ignored `.local/native-integration/checkpoint3/`.
`bundle/` and `install/releases/native-mock-c3/` preserve the first installed
candidate; `project/` contains its answered question and the rejected consultation
authorization. The first PTY check used an overlong private IPC path and failed
before TUI entry (`installed-tui-pty-2.bin`, exact test unit journal). The corrected
test used the short project-keyed `/run/user/1000/waywr/` socket. The c3
consultation PTY (`installed-tui-consult-pty.bin`) then showed a `stale_state`
refusal: event-thread snapshots refreshed the panel but not the command-side
revision. The code now checks IPC length before service launch and updates that
revision on every displayed snapshot. No earlier fixture or refusal was rewritten.

`bundle-c3b/`, `install/releases/native-mock-c3b/` and `project-c3b/` contain
the corrected release and state. `installed-tui-c3b-flow.bin` records installed
launch, mock clarification/answer, explicit consultation and proposal, and detach
with terminal restoration. SQLite observations: one question answered, one
consultation charged, one proposed task, zero task calls. Pre/post service-restart
counts (`c3b-pre-restart.json`, `c3b-post-restart.json`) matched; installed
`resume` and `/exit`/`resume` PTYs retained the proposal and 4 conversation + 1
consultation charges without repeated turns. Both exact test-owned services were
stopped after verifying no queued/reserved turn or authorized task; all evidence
and releases remain. `bundle-c3c/`, `install/releases/native-mock-c3c/` and
`project-c3c/` contain the final checkpoint-3 code. Its
`installed-tui-c3c-steering.bin` records a mock proposal, explicit steering
suggestion/review/application, one retained cancelled original and one linked
unadmitted replacement; two conversation charges, no task calls. Its exact test
service was stopped after verifying no queued/reserved turn or authorized task.
The automated PTY checks do not establish physical terminal
acceptance or native provider behavior.

## Checkpoint 4 — static file workflow, mock evidence

The service freezes the `static-files-v1` plan, source decision and bounded
context hashes before execution. The existing systemd host and runner retain
claim/permit/receipt ordering; a durable task budget charge follows the
invocation-reservation marker, not merely stage preparation. No mock task
authorization can choose a native provider. The result validator accepts only
four relative names (`index.html`, `styles.css`, `app.js`, `README.md`) and
32 KiB total; generated HTML/CSS/JavaScript is data and is never executed by
Waywr. Candidate publication refuses linked directories and writes an immutable
per-file/hash manifest. A review cannot invent application finding IDs; author
repair claims remain unresolved until independently assessed. Missing
dispositions retain the original finding and evidence. An unexpected worker
question has a versioned result, durable answer gate and linked follow-up; the
answer spends no call. If the frozen allowance cannot also cover independent
review, the task stops with its available candidate and explicit reason.

The TUI can inspect `/open FILE` for one allowlisted file; that does not mark
the full bundle read. `/open` loads the full verified bundle through bounded
pages and records reading progress for the exact request/manifest. The final
Approve action requires the full-bundle receipt; Reject remains a separate
human choice. A changed file or manifest prevents approval and an attempted
decision moves the task to durable `output-invalid`. Replayed stage application
returns the original state without republishing files. The final decision binds
the manifest, not an individual file or an unverified path.

Focused checks executed: seven `backend::static_files` tests (four-stage
approval/reopen, idempotent adoption, traversal/duplicate/size/tamper/linked
directory refusal, missing findings, ambiguous reference, worker question and
budget exhaustion); 35 passing `frontend::prototype::view` tests (one explicit
30-second latency probe remained ignored), five `session_store` compatibility
tests, release schema compatibility and state-fingerprint rollback tests; and
`tests/session_schema_consistency.py`. Existing stage-3 tests and host fault
evidence were reused; they were not rerun as a new broad acceptance campaign.

Ignored installed evidence is at `.local/native-integration/checkpoint4/` and
`.local/native-integration/final/`. The first final-prefix PTY retained
`installed-final-flow.bin` and `installed-final-summary.json`: `/open FILE`
followed by approval did not complete because slash-menu Enter ignored the
argument and the scripted modal return path was incomplete. The command parser
and driver were corrected. `installed-final2-summary.json` and
`installed-final3-summary.json` record full mock-only installed approval; the
second project `installed-peer-summary.json` completed one discussion with no
task, no project-A identity/content in its retained state, and no change to
project-A budgets. These are automated PTYs, not operator terminal acceptance.

The final3 project at `.local/native-integration/final/project-final3/` is bound
to `native-mock-c4-final3`. Its fake task consumed exactly 4/6 conversation,
1/1 consultation and 4/4 task calls; two mock reviews assessed `sf-1`. The
read receipt and human-equivalent scripted acceptance fixture both bind manifest
`0fe5877def01b5fbffb2310f500637a06b0a118ead1031610b097c5502178477`.
The fixture's approval is automated test evidence, **not operator approval of a
native deliverable or verification of a working weather page**. No browser was
opened. The c4b, final, final2 and final3 bundles/projects, failed PTYs and
retained host receipts remain ignored. Their exact test-owned project services
were stopped after all associated operation units were verified to have empty
cgroups; shared Herdr and operator services were untouched.

The final **source-matched** paired release is
`.local/native-integration/final/install/releases/native-mock-c4-final4/`,
manifest SHA-256 `afeafe312b10f9d87cdcc50583cfe811dc1973a5fcb9d91491fa6ebc93c0facf`.
The stable ignored `waywr-eval` launcher selects it and isolates the user-local
location index. Its retained project is `project-final4/`, key
`<private-id>`. `installed-final4-flow.bin` and
`installed-final4-summary.json` record the full installed mock TUI pass and
clean UI detach. One task was approved by the scripted fixture; budgets were
4/6 conversation, 1/1 consultation, 4/4 task, with four charged/completed
effects and two reviews. Its complete 2,014-byte bundle read and final decision
bind manifest `099448229d1195166ffa2865aa18b3cdd34bbc0f362eca65bbc71e635ba2b264`.
This is simulated acceptance only. The exact final4 service was stopped after
all four operation units showed `MainPID=0` and empty cgroups; its release,
project, approval fixture and immutable evidence remain. No authenticated
calls, physical-operator TUI observation or browser run occurred.
An empty ignored `operator-eval/` project and stable `waywr-eval` launcher are
prepared for a later optional hands-on mock evaluation. The launcher handles
its isolated location-index configuration internally; `init .` and TUI startup
still use ordinary project-local state and automatic service management.

## 1B proof detail and limit

The host user namespace/uid-map check failed with `Operation not permitted`, but
an already available disposable Docker fixture supplied `--network none`, read-only
root, no capabilities, no new privileges, private mounts and dummy credentials.
Loopback worked for a local mock endpoint. The pinned Codex and Claude binaries
started in this fixture; neither had access to the host credential directories.
The first Codex `thread/start` failed because the fixture sent `readOnly` instead
of the accepted `read-only` enum. Its original response remains at
`codex-thread-proof-invalid-sandbox.json` (SHA-256
`f149e8e67427d5cf608c0fd8c45ce57e5a39fce820df837c2a89923d582e4436`).
The corrected ephemeral thread returned `environments: []`,
`instructionSources: []`, requested model `gpt-6.1-sol`, effort `high` and a
read-only sandbox (`codex-thread-proof.json`, SHA-256
`f06cedf7e0a45d3fe1b2ab5cbf1ee0f4de269bcdbda8943412a7c38284aa0153`).

One `turn/start` reached only the loopback mock `/v1/responses`; the mock refused
inference with HTTP 400. The captured request had seven `input` items, including
a developer `additional_tools` item listing the unexpected tool namespaces. Its
top-level `tools` array was absent, which does **not** negate the model-visible
declarations in `input`. The disposable foreign-project canary content did not
appear in the captured request. `codex-turn-proof.json` has SHA-256
`0f5bb8e22a3171fc56da75e2bb0afef734e9f7341be28c2e881909077346dd1d`.
This is mock-request evidence, not proof that any listed tool can execute, nor
proof of startup isolation under the operator's real authentication. B1's allowed
inventory criterion failed before a safe B2 exercise. No actual handler denial,
structured final, retry suppression, managed-policy or account-entitlement claim
is made. The Claude profile was not advanced once the two-provider admission gate
was already blocked. The smallest next native step needs a separately reviewed
supported profile that prevents or mediates the extra model-visible execution and
delegation capabilities **before** a call; no such interface is established here.

## Native-readiness assessment — closed

The final4 installed build is mock-only. It has no admitted authenticated
coordinator, consultant, author or reviewer path. The pinned previously observed
CLIs are Codex `0.162.1` (SHA-256
`628e35487d888a28f0427fc37501da529dd3d81fcbad7a2c3421c1e8670d92e1`)
and Claude Code `2.1.296` (SHA-256
`24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`).
Their presence and versions are not proof of effective isolation, login state,
account entitlement or model availability. The [boundary assessment](boundary-assessment-through-live-check-2026-10-14.md)
owns exact capability classification and B1–B7 criteria. B1 failed for the
tested Codex app-server request because model-visible execution/delegation
capabilities exceeded the approved packet-only role. Handler denial and the
Claude effective profile were not tested. The required next prerequisite is a
supported, credential-free profile that prevents or mediates those operations
before an outbound native call, followed by the remaining effective-boundary
proof. Checkpoint 5 and its proposed 6 conversation + 1 consultation + 4 task
launches are **not ready or authorized**; no provider/model/effort substitution
or cost escalation is implied by this report.
The separately proposed target is `temp/native-weather-01/`: Claude
`claude-sonnet-5-5` at medium effort for up to six coordinator turns and two
author/repair calls, and Codex `gpt-6.1-sol` at high effort for one consultation
and two independent reviews. The four task calls include questions or repairs;
the total ceiling is eleven native launches. Model availability and effective
restrictions require later verification before any authorization to spend.
