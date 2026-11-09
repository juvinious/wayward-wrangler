> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Claude conversation increment — verification checkpoint

Implementation and credential-free verification are complete on
`feat/native-coordinator-integration`. The operator subsequently authorized only
the exact three-call `claude-conversation-v5` check, launching and submitting
messages themselves. No preliminary or automated model call is authorized.
The operator completed the three messages and reported successful detach/resume.
Read-only evidence verification below establishes three completed native
conversation calls, with no allowance remaining. This is not broader product,
worker, consultation or working-weather-application acceptance. The scripted schema-4 evaluator
is superseded for conversational evaluation; its state and release remain intact.

## Delivered behavior and source

Ordinary text reaches the selected Claude client unchanged, through the existing
project supervisor, fenced operation host and runner. There is no natural-language
phrase matching in the schema-5 path and no fake fallback. The accepted production
TUI streams assistant text provisionally, commits validated completion, preserves
drafts/history/budgets across reconnect, and distinguishes service connectivity
from coordinator readiness. F3 reviews the pinned profile and explicit allowance.
Conversation output has no task, approval or execution authority; workers,
consultation, steering and file work are unavailable in this increment.

New projects use state schema 5 and protocol 1/minor 3. Existing schema 3/4 and
unfinished LangGraph releases/projects are not migrated. The
[Claude contract](claude-coordinator-contract.md) owns authority, bounds and rollback.

| Checkpoint | Scope |
| --- | --- |
| `8e08a6b` | Preserved starting checkpoint / state rollback boundary |
| `5333993` | Independent Claude capability proof |
| `b730aa6` | Conversation host, native adapter, schema 5 and production TUI |
| `e7d0c48` | Provisional text sanitization and failed-client diagnostics |
| `ac013b2`, `74e6598`, `332171f` | Lost-snapshot reconnect and refused-draft restoration |
| `1ceabf9` | Recheck exact receipt before reporting runner loss |
| `717348b4ae88e618a3768d9b226ec26ad91329be` | Cancelled outcome adoption and accurate stopped labels; final build source |

## Independent provider boundary

Pinned Claude Code **2.1.296** binary SHA-256:
`24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`.
Fifteen credential-free cases passed using the pinned binary, dummy credentials,
loopback mock endpoint, no external network, read-only container, dropped
capabilities and no host credential/config mounts. Existing image digest:
`sha256:df18428df8a0f9189c84e7424d214b4c50092af1f9dca5842a00c41c39bda316`.

Evidence: `.local/claude-conversation/boundary-05/summary.json`, requests, events,
sentinel access trace and marker assertions; reproducible harness
[claude_conversation_boundary.py](../tests/claude_conversation_boundary.py).
Text and literal @file/slash mentions completed without automatic file context;
forced Read/Write/Edit/Glob/Grep/Bash/Agent/SendMessage/WebFetch/WebSearch were
refused before observed sentinel access/effects. Tools/MCP/skills were empty;
foreign, project and user-rule canaries absent; hooks/MCP/shell markers absent.
HTTP 500, 529 and midstream disconnect each made one request then failed, with
model/effort unchanged and no retry/fallback. Built-in static plugin/agent names
in initialization metadata are not granted executable capabilities.

Controls are supported per-call safe/restricted mode, no setting sources, disabled
hooks/slash/history/memory/session persistence, strict empty MCP, empty/deny-all
tools, owned system prompt, exact model/effort, one agentic turn and retries zero.
The process environment is allowlisted. Known managed files cause refusal, not
policy bypass. Original Codex B1 failure remains independent and deferred.

This is pinned-client evidence, **not whole-process confinement** against a
malicious client/same-UID process. Saved-login file presence was checked without
reading/copying secrets. The subsequent three-call check succeeded with the
selected client/model and saved-login profile. Every account-policy path, native
interruption/crash recovery, latency bounds and wider terminal behavior remain
unverified. Trusted
client runtime/authentication access is distinct from model-selected content.
Primary references: [CLI](https://code.claude.com/docs/en/cli-reference),
[settings](https://code.claude.com/docs/en/settings),
[environment controls](https://code.claude.com/docs/en/env-vars).

## Focused automated checks actually executed

Source checks passed for closed authorization, durable three-call debit,
exhaustion/replay/restart, no task creation, cancelled-result replay, text-only
completion, discarded thinking, native UI readiness/draft/sanitization and stopped
labels. Existing SQLite busy refusal (<500 ms), slow-client status (<1 s), host
identity/retirement/cancellation and schema/release compatibility regressions
passed. Lost handshake/snapshot reconnect regression and IPC consistency passed.

Final installed normal-path check used **only the hash-pinned mock CLI** through
actual `init` → production TUI → explicit F3 authorization → streaming → detach
→ resume → service restart → completion/exhaustion. Three completed receipts,
three charged calls, zero tasks; status response **0.00527 s** (<1 s); draft
restored, results adopted once, old command replayed without another dispatch,
thinking canary absent. Additional owned fixtures validated cancellation with one
held call and runner-loss uncertainty with one spawn/no replay. Harness:
[claude_conversation_installed.py](../tests/claude_conversation_installed.py).
Evidence: `.local/claude-conversation/installed-final-05/{summary.json,cleanup.json}`,
PTY captures and project-local packets/attempts/transcripts/receipts/SQLite.
All three fixture services ended inactive/MainPID=0 after owned children finished.
No shared Herdr or operator service was touched. Automated PTY checks are not
operator terminal acceptance or natural-language quality evidence.

## Final isolated candidate and evidence preservation

Release: `.local/claude-conversation/install-01/releases/claude-conversation-v5/`.
Manifest SHA-256:
`ae55df4c9a259adf16506a75981179c8a01a2d4d3fbba4bc5b9b4137dbeafe6d`.
All 13 packaged files match the manifest; three binaries match the release build.
Rust/Cargo 1.99.0, pinned `rust-toolchain.toml`, locked/offline release builds.
Cargo.lock is unchanged across this increment. Exact source, compiler, lockfile
hash and evidence hashes: `.local/claude-conversation/final-build-receipt.json`.
No application Python interpreter/runtime is installed; Python is a test fixture.

Stable launcher: `.local/claude-conversation/waywr-live`; fresh initialized project:
`.local/claude-conversation/live-eval/`, key `<private-id>`.
At preparation: native profile, three calls, zero used and no service launched.
After evaluation: all three calls completed, allowance exhausted; see below.
The launcher selects the paired release/project and private locator internally.
It retains the existing saved-login route in place; it is not globally installed.
[Authorized live check](claude-live-acceptance.md) records the scope and outcomes.

Earlier bundles/installations, failed fixture evidence and original receipts are
retained under `.local/claude-conversation/`. The
[archived correction history](archive/native-integration/claude-conversation-corrections-through-v4-2026-10-13.md)
records timing/driver mistakes and actual replay/reconnect/receipt/cancellation
findings; its historical runtime paths are unchanged. Do not rebind old fixtures
or restore a pre-message snapshot over accepted state. The previous mock operator
project, r9, legacy runs, evaluations and services remain protected.

## Operator observations and read-only native outcome verification

**Operator observations (not automated acceptance):** after two replies, “Looks
good so far.” They then reported “detached and retached, which resumed,” with a
screenshot showing the third reply and exhausted 3/3 allowance. They also
identified literal Markdown `**…**` in coordinator replies. No broader acceptance
was stated; physical native exit, live interruption and worker behavior were not
tested. No model call was submitted by the development agent.

**Read-only verification:** the project remains bound to the exact v5 manifest.
Ledger: three charged discussion turns, all completed; six messages (three user,
three assistant); zero tasks/actions, zero consultation/task charges. Exactly
three attempts/spawn commitments and three exit-0 completion receipts. Packet,
result, message, claim/permit/runner/invocation identities and hashes match.
Each retained native initialization reports Claude Code 2.1.296,
`claude-sonnet-5-5`, dontAsk, empty tools/MCP/skills; execution argv specifies
medium effort and the approved restrictions. No tool-use action was recorded.
The third frozen packet contains all four prior messages with matching hashes;
provider resume was disabled. This corroborates continuity through Waywr-owned
context; the physical detach/resume itself is the operator's observation.

Derived audit and verified screenshot copies:
`.local/claude-conversation/live-verification-2026-10-13/{summary.json,screenshots.json}`.
Original packets, outputs, transcripts, receipts and state remain in `live-eval/`.
At audit, the project service was active (PID 62103) and left untouched; none of
the six exact saved runner/client identities remained live. Completion was
established by receipts, not process absence. **Three calls consumed, all
successful; no additional native call authorized.**

**Presentation correction:** the small formatter supported code/lists but had no
strong/emphasis parsing. The source fix styles paired bold/emphasis before
wrapping, preserves styles across visual lines, and leaves code, unmatched
markers and user-authored input literal. A byte-identical nonsecret saved plan
is the offline regression fixture. Fifty-three presentation/input tests passed;
one preexisting 30-second probe remains ignored. Dark/light/mono and narrow/wide
saved-response cases passed. No dependencies changed. The fix is source-only;
v5 binaries, binding and active service are unchanged, and this rendering
correction has not received installed/physical-terminal verification.

**Follow-up nuance:** model prose quotes the frozen handoff's pre-reservation
budget (e.g. one call remaining during call 3). The current UI/ledger correctly
shows 3/3 exhausted. Model text is not budget authority. Clarifying the packet's
as-of wording is a follow-up; original packet/response bytes are not rewritten.
