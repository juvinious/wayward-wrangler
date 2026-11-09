> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Frozen pre-authorization request at `aa9e009`, approved by the operator for v6.
> The [current trial instructions](../../claude-scope-live-acceptance.md) own setup
> and outcomes. This snapshot's pending status is historical.

# Proposed native scope check — authorization not granted

Implementation checkpoints 1–4 passed with isolated mock/credential-free evidence.
**No native scope calls are authorized.** The original three-call v5 trial remains
exhausted and untouched. This request covers a new project, not allowance renewal.

## Exact proposed authorization

- Trial: **claude-scope-01**, fresh isolated target
  `~/projects/example-workspace/.local/claude-scope/claude-scope-01`.
  It is reserved for a later approved setup; not initialized or bound in this pass.
- Release: **claude-scope-v6**, installed under
  `.local/claude-scope/install/releases/claude-scope-v6/`.
  Manifest SHA-256:
  `b60073595f485d5d85ac517bbd7d1f0087c06a33e16370b76a2894ff6eeebb13`.
  Frontend `5116265162f458bc20cd03c4e18ba7deea17fdd7048434f717fd4ea4b5f54d6f`;
  service `6af59acdb3ccce14e86fc5e053af8879efe374006c140114f55146777729c3f6`;
  runner `9f81738a3057528c8ddfdf1ce745f56685008d2d5a9d223c6087f51ed80cdb69`.
- Claude Code **2.1.296**, binary
  `~/.local/share/claude/versions/2.1.296`, SHA-256
  `24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`.
  **claude-sonnet-5-5**, **medium** effort; existing saved CLI login in place,
  unchanged provider/authentication/billing route. No secrets copied into state.
- Profile **claude-proposal-v1**: at most **six coordinator invocations total**,
  including failed, interrupted, cancelled, malformed or uncertain calls. User
  submits every message/continuation and explicitly saves the exact allowance in
  F3. No preliminary greeting, connectivity call or automated native check.
- **Zero workers, consultation, task/execution allowances, repository access,
  generated files, retry/repair calls, fallback, substitution or renewal.**
  CLI invocations do not bound all HTTP requests, tokens or dollars.

After separate approval, prepare the fresh isolated live launcher with ordinary
`init .` → `waywr` → `resume`, automatic services and these exact bindings. The
current [mock launcher](../../live-scope-evaluation.md) deliberately cannot run this trial.
Do not run the native release against the existing live trial or operator projects.

## Effective boundary and selected context

Same proven restricted print interface: safe/restricted, empty settings sources,
empty tools plus deny-all names, strict empty MCP, hooks/skills/slash commands/
memory/history/session persistence disabled, one model/agentic turn, no configured
transport retries or nonstreaming fallback. No inherited provider-key, proxy, bus,
Herdr, agent/plugin/customization environment. Known managed-policy files refuse
pending review. Fresh private scratch cwd contains only application-supplied input.
Trusted client runtime/saved-login access remains necessary; this is not a claim
of whole-process confinement or resistance to malicious same-user processes.

Credential-free pinned-client proof with the new prompt passed strict JSON text,
literal unrelated-path/slash mention and attempted Bash-denial cases, with local
mock endpoint/dummy credentials/no egress. No advertised tools or sentinel/context
canary access was observed. Existing 15-case boundary evidence is retained. This
proves the inspected client/profile behavior, not every future account/server-policy
path. Real result-v2 formatting, account availability and new live scope behavior
remain unverified; refusal/drift must not lead to silent substitution.

Only unchanged current user input, selected project proposal/hash/authority,
explicitly linked human questions/answers/decisions, provider binding and budgets
labelled as-of reservation including the held call; at most six preceding messages.
96KiB packet ceiling, provenance allowlist, no registry, unrelated projects,
repository scan, provider history or cross-project grant. The weather idea uses
**fixed sample data**, with no API key or external weather dependency. No file read,
write, shell command or external action is model-authorized. Waywr alone writes
records, frozen packets/results, receipts, checkpoints and UI drafts in the trial.

## Operator scenario and success

Discuss a sample weather page, settle a clarification, request a saved proposal,
and inspect/edit it locally. Obtain at least two proposal revisions. Saying “looks
good” must save only conversation, not authority. Use the four-step review to acknowledge unresolved
choices and separately authorize the exact revision. Detach/resume; request a further content
change and verify that the old decision is historical and a new decision is needed.
Local editing/answers/reading/authorization cost no calls. Use at most six messages
or explicit continuations; if the model needs more, report incomplete acceptance.

Success: natural live replies, validated proposal/question suggestions, durable exact
scope decision and subsequent invalidation, continuity across resume, matching
receipts/counts ≤6 and zero worker/consultation/admissions. UI must say **Scope
authorized · execution unavailable**. No working weather application is claimed.

Assistant-only partial output is visibly provisional. Completion requires a validated
single successful end-turn plus strict result-v2; raw reasoning/action JSON stays
out of conversation. Exhaustion disables sends/continuation while preserving local
review/edit/decisions. Failure/malformed output, drift, integrity/capability violation,
interruption, output/deadline limit or uncertainty stops further dispatch and retains
its debit/evidence. No retry or automatic allowance increase. Deadline is 120 seconds;
existing verified cancellation/receipt/generation policies apply. An unavailable
model cannot prevent deterministic exit/checkpoint. Resume never replays ambiguity.

After operator evaluation, audit saved records/receipts without further model calls;
record operator observations separately from automated evidence. No broader native
worker, consultation, repository-action or product acceptance is inferred.
