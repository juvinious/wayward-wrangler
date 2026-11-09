# Claude conversational coordinator — current increment

Authorized from `8e08a6b`: implement and credential-free verify a real Claude
conversation path in the accepted production TUI. Authenticated calls are not
approved. Worker tasks, consultation and repository editing are deferred.

## Boundary checkpoint

Source/state rollback base: `8e08a6b9be5d6dbedf7dd3a07415d5782949d83a`.
No operator-state change. The existing Codex failed proof remains independent;
Claude is evaluated on its own evidence.

Pinned Claude Code: 2.1.296, binary SHA-256
`24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`.
The existing `python:3.12-slim` image
`sha256:df18428df8a0f9189c84e7424d214b4c50092af1f9dca5842a00c41c39bda316`
ran with network none, read-only root, all capabilities dropped, no new
privileges, UID 1000, private fixture mount and only the pinned client binary.
There were no host credential/config mounts. The mock used a dummy API key;
it establishes neither account access nor a change to the intended saved-login
route. No authenticated request occurred.

Final ignored proof: `.local/claude-conversation/boundary-05/` (15 cases);
`boundary-02/` and `boundary-03/` preserve earlier profiles;
`boundary-01/` retains the initial profile and mount-permission failure history.
`tests/claude_conversation_boundary.py` is the reproducible container-side probe.
Expected/observed: a text response streams and completes successfully; forced
Read, Write, Bash, Agent and WebFetch return explicit disabled-tool results;
sentinel inotify ACCESS/MODIFY/OPEN has no event, hook/MCP/shell markers and
unauthorized writes remain absent. Outbound tools/MCP are empty, foreign/project/
user rule canaries are absent, skills are empty after disabling slash commands.
Built-in plugin/agent names remain trusted static init metadata; no execution
capability was granted to them. HTTP 500 and 529 each produce one mock request,
requested model/medium effort unchanged, then an explicit failed result.

This is effective pinned-client mock evidence, not whole-process confinement
against a malicious client/same-UID process, or proof of every unknown future
tool. Real subscription authentication, account policy, entitlement, native
latency/interruption and physical terminal behavior remain live checks. Any
profile/binary drift requires revalidation. Managed customization or changed
routing must refuse, rather than bypass policy or enable fallback.

## Implementation contract

Use fresh `claude --print` per turn with safe/restricted mode, no setting sources,
strict empty MCP, empty tools plus deny-all tools, disabled slash commands/hooks,
owned system prompt, no provider history, no fallback and zero configured
transport retries. Plain assistant text is streamed provisionally; exactly one
successful JSONL completion is converted into an inert `coordinator-turn-v1`
with `action:null`. Thinking is excluded from the TUI. There is no model-owned
shell, filesystem, delegation or application authority.

New conversation-only projects use state schema 5; earlier schema 3/4 projects
and bound releases retain their existing contracts. No transparent migration.
The project service hosts calls through existing claim/permit/process/receipt
primitives. Three durable held/charged conversation reservations; no reset on
resume or coordinator selection. Workers/consultation have zero allowance.
Native authorization starts closed and must be explicitly saved for that exact
profile and allowance. Automated fixtures may use a separately labelled mock
CLI transport with a pinned fixture binary, never a fallback to a real client.

Application context is at most six selected recent messages plus the deterministic
project handoff, bounded to 96 KiB. No registry, unrelated project scan, provider
resume or native history. Authentication stays in the existing client home/config
location and is never copied into project records. Client startup/runtime/auth
access is trusted; model filesystem access is disabled by the verified interface.

## Verification and next gate

Boundary proof passed the above fifteen isolated cases. Source checkpoint `b730aa6`: hosting/streaming, closed admission, cancellation, schema 5 and the
production TUI integration are implemented; final installed acceptance is pending.
Eleven conversation-authority tests, the plain-result adapter test, native UI
readiness/draft test and schema-range test passed. Existing SQLite busy (<500 ms)
and slow-client status (<1 s) regressions passed. Cargo.lock is unchanged.
The first installed fixtures under `.local/claude-conversation/installed-01..05/`
preserve driver timing/detach mistakes and a real exhaustion/replay bug. The bug
was corrected by checking saved command outcomes before readiness refusal, with
a focused three-call regression. Earlier completed mock receipts remain intact. Provisional text uses the same
terminal-control sanitization as completed messages; native nonzero exits retain
reported usage and a bounded client failure explanation. No dependency changes. The next authorized work is conversation
hosting, streaming/reconciliation, honest TUI readiness and installed mock-stream
verification. The final deliverable is one stable launcher and a concrete request
for at most three real coordinator invocations; none are authorized yet.

Primary contracts: [CLI reference](https://code.claude.com/docs/en/cli-reference),
[settings precedence](https://code.claude.com/docs/en/settings),
[environment controls](https://code.claude.com/docs/en/env-vars). Installed help and
mock requests establish the pinned controls actually used; docs alone do not.

## Installed reconnect correction

The first optimized `claude-conversation-v1` fixture is retained at
`.local/claude-conversation/installed-final-01/`. Two calls completed and were
adopted once, but the TUI refused the third input after a service restart. Source
inspection and a controlled socket regression established a reconnect race: the
old service could accept a handshake then close before snapshot delivery; the
event pump silently exited, leaving mutation admission false and stale Connected
text. Reconnect now retries the complete handshake/snapshot sequence with a
six-second deadline (an already-started handshake can add up to two seconds),
reports reconnect/exhaustion, advances the verified cursor, and never sends a
model command. A dedicated lost-snapshot regression passed. Native Inspect now shows conversation calls/record paths, not empty task
diagnostics. Literal @file and slash/skill text passed the isolated proof with
no sentinel access or automatic context injection. Final candidate
verification follows on a fresh fixture; the v1 project/release are preserved.

The v2 installed fixture identified the second occurrence of the same race: a
snapshot requested after an event batch also had a silent-break error branch.
Both snapshot paths now use bounded verified reconnect. Input refused before
delivery restores its complete original draft (including literal paste and
multiline state), rather than only claiming it was retained. v2 and its two
completed receipts remain preserved at `installed-final-02/`; no call was replayed.

The v3 normal-flow subgate passed (three completed receipts, zero tasks,
0.0056-second status response, detach/draft restore, restart adoption, exhaustion
and command replay). Its interruption fixture exposed a controller receipt
observation race: `ensure_host` read no receipt, queried systemd, then found the
runner gone although a valid completed receipt had been published meanwhile.
It now revalidates the exact receipt immediately before declaring runner-loss
uncertainty. Identity/hash validation is unchanged; no replacement call occurs.
The old uncertain event and completed receipt remain intact in
`installed-final-03/interrupt-project/`. The focused interruption fixture now
SIGSTOPs only its verified mock child until durable cancellation is delivered,
then releases it, separating active interruption from a completion race.

The controlled v4 interruption produced a valid cancelled receipt, but its
conversation stayed reserved: the preexisting stopped-turn validator omitted
`cancelled`, and the caller did not check that refusal. The outcome contract now
admits cancellation with held debit and replay regression coverage. Original v4
receipt and refused/provisional state are retained at `installed-final-04/`.
The UI includes authoritative failed/uncertain/cancelled status in stop events,
preventing retained-evidence wording from incorrectly selecting a Done label.
