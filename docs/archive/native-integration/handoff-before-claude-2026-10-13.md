> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler — current handoff (2026-10-13)

## Objective and authorization

On `feat/native-coordinator-integration`, checkpoints 1A, bounded 1B, and 2–4
were authorized from capability amendment `167b983834413a3997fd5eaa583794c4165f7fa5`.
Checkpoint 1A is `fa90340`, 1B is `f620700`, conversation authority is
`a277cfe`, integrated mock coordinator is `574b342`, and checkpoint 4 is
`81cf6a8`. Checkpoint 4 adds the
bounded static-file workflow and installed mock acceptance. Checkpoint 5,
authenticated calls, weather native trial, global install, operator cutover,
shared Herdr changes, push, merge and publication remain outside authorization.

## Current implementation and verification

New schema-4 projects use the accepted Rust/Ratatui TUI and Rust/Tokio service.
`waywr init .` initializes project-local state and returns; `waywr` starts or
reconnects the service automatically. Discussion creates no task. Mock
clarification, durable answers, consultation, proposal review, authorization,
steering before frozen work, output inspection and final decisions use project
IPC. The four-file `static-files-v1` task uses supervised fake workers, at most
four task invocations and two reviews. Files/manifest, finding identities and
assessments, full-bundle reading and exact-manifest approval are durable.
`/open FILE` previews one allowlisted file without granting approval; `/open`
reads the full bundle. A worker question consumes its completed invocation;
answering it consumes none. Incomplete review and insufficient remaining budget
stop with retained evidence. A tampered candidate cannot be approved.

Focused backend/UI/schema/release tests passed, including replay, tamper,
traversal, duplicate files, linked directories, worker-question budget,
incomplete findings, file selection and schema-3 compatible release bounds. The
source-matched `native-mock-c4-final4` paired installation passed one automated
installed PTY through discuss → clarify → consult → scope → authorize → four fake
worker stages → full review → final decision. Its fixture charged 4/6
conversation, 1/1 consultation and 4/4 task calls; two reviews were recorded.
The exact approved *fixture* bundle manifest is
`099448229d1195166ffa2865aa18b3cdd34bbc0f362eca65bbc71e635ba2b264`.
A separate peer project completed one discussion and zero tasks without project-A
content or state changes. These are automated simulated checks, not physical
operator approval, authenticated native behavior or browser verification of the
weather page. Prior schema-3 projects retain their bound behavior.

## Operator finding, native admission and next action

The operator's physical `operator-eval` check **did not accept the natural
conversation flow**. It is bound to `native-mock-c4-final4`, key
`<private-id>`. A read-only check found four mock
turns, one answered question, zero tasks. “ok proceed” received the canned
fallback. The phrase-matching mock is a test fixture, not a substitute for
routing conversation to the selected LLM. The source-only TUI correction now
labels a connected fake coordinator accurately and reserves “reconnecting” for
lost service IPC; one focused test passed. The installed release, operator
project, budget and records were not changed. [Report](native-coordinator-integration-report.md#operator-mock-evaluation-finding-2026-10-13).

**Native admission is closed.** The no-egress, dummy-credential Codex 0.162.1
app-server B1 request still advertised model-visible execution/delegation tools
outside the approved packet-only profile. No tool execution, real authentication,
Claude effective restriction, managed-policy or account entitlement was proven.
B2–B7 were not completed. The [boundary assessment](native-coordinator-boundary-checkpoint-1.md)
records exact evidence and remaining proof. Do not launch checkpoint 5 or
substitute providers/models/efforts. Proposed later weather trial remains a
separate decision, ceiling 6 conversation + 1 consultation + 4 task launches.
The proposed target is `temp/native-weather-01/`: Claude
`claude-sonnet-5-5` medium for coordinator/author work, Codex `gpt-6.1-sol`
high for consultation/review, with no fallback or automatic retry. Availability
and effective permissions remain unverified.

Next action is to resolve the native capability boundary through a supported,
evidence-backed route, then separately authorize authenticated use. Ordinary
messages should reach the selected coordinator provider only after admission;
do not add mock phrase branches to simulate natural language. No native trial is
ready. Preserve the current mock release and failed operator flow for inspection.

## Installed candidate and retained evidence

Stable isolated launcher:
`.local/native-integration/final/waywr-eval`. It selects paired release
`.local/native-integration/final/install/releases/native-mock-c4-final4/` and
isolates its location index; it is **not** a global `waywr` installation.
Its manifest SHA-256 is
`afeafe312b10f9d87cdcc50583cfe811dc1973a5fcb9d91491fa6ebc93c0facf`.
The completed final evidence fixture is
`.local/native-integration/final/project-final4/`, key
`<private-id>`, with bound release and retained
approved fake task. For a fresh operator mock evaluation, the single empty
project is `.local/native-integration/final/operator-eval/`. In Fish:

```fish
cd ~/projects/example-workspace/.local/native-integration/final/operator-eval
../waywr-eval init .
../waywr-eval
```

Later `../waywr-eval resume` reconnects; `../waywr-eval session status` reads
status. The final4 evidence project service was stopped after four exact host
units showed empty cgroups. The earlier
checkpoint-4 c4b and final-prefix failed/corrected fixture projects, PTY logs,
receipts and side-by-side releases remain under ignored
`.local/native-integration/checkpoint4/` and `.local/native-integration/final/`.
Their test-owned project services were stopped after worker checks; no operator
service or shared Herdr resource was changed. Earlier checkpoint-3 evidence is
at `.local/native-integration/checkpoint3/`. The 1B proof is at
`.local/native-capability-proof/2026-10-13/`.

Preserve r9, historical approved runs, schema-3 and LangGraph bound releases,
the separate blocked `~/projects/waywr-rust-sim-eval` task,
operator evaluation projects, decisions, receipts, transcripts and shared Herdr
resources. Schema-4 state is compatible only with a release that declares it.
Rollback changes a quiescent project binding after full state-fingerprint checks;
never restore an old snapshot over accepted work or silently migrate a bound
legacy project. See [current limits](current-limits.md).

## Reading order

[AGENTS.md](../AGENTS.md) → this handoff →
[implementation report](native-coordinator-integration-report.md),
[proposal](native-coordinator-integration-proposal.md),
[boundary assessment](native-coordinator-boundary-checkpoint-1.md), then
[architecture](architecture.md), [execution](execution-contract.md),
[state](state-contract.md), [IPC](session-protocol.md),
[TUI](tui-contract.md) and [current limits](current-limits.md).
[Documentation index](README.md) maps archives; consult them selectively.
