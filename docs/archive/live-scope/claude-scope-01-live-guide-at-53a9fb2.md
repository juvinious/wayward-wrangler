> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical authorized walkthrough before the first-call failure. Further native
> dispatch is now stopped; the [current disposition](../../claude-scope-live-acceptance.md)
> owns the trial outcome. Do not reuse this snapshot as permission to continue.

# Authorized native scope trial — awaiting operator evaluation

Implementation checkpoints 1–4 passed with isolated mock/credential-free evidence.
The operator authorized this exact **claude-scope-v6** six-call trial on 2026-10-14,
including physical sequential review. **Only the operator confirms F3 and submits
messages/continuations. Preparation spent zero calls; no acceptance is inferred.**
The original exhausted native v5 trial and authorized mock v5 project are preserved.

## Exact authorized profile

- Trial: **claude-scope-01**, fresh isolated target
  `~/projects/example-workspace/.local/claude-scope/claude-scope-01`.
  Initialized for this trial; key `<private-id>`, bound to v6.
  No service, database, F3 allowance or native call existed at preparation.
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

## Short Fish launch / resume

The stable **live** launcher pins this project/release/client/profile and handles
private routing/runtime plus automatic service startup. It refuses drift and
configuration overrides. The existing mock launchers remain unchanged.

```fish
cd ~/projects/example-workspace/.local/claude-scope/claude-scope-01
set -l waywr ~/projects/example-workspace/.local/claude-scope/waywr-live
$waywr init .
$waywr
```

Later, from that directory: `$waywr resume`. Init is already prepared; repeating it
preserves all state and cannot renew the allowance. No manual service, runner,
environment or Herdr setup. In F3 verify **native / Claude / claude-sonnet-5-5 /
medium / six calls**, then explicitly save the allowance. Opening F3 costs nothing.
If settings/readiness differ, stop and report before sending.

Preparation verified the manifest and all 13 files, pinned client hash, login-file
presence only, managed-policy absence, repeated init and no service/call. **The
normal Claude alias points to 2.1.299; it is not used.** This fresh project's exact
profile pins the approved 2.1.296 before any allowance or runtime. No credential
content was read/copied, no Claude executable or auth/connectivity check was run.
Ignored provenance: `.local/claude-scope/live-setup/preparation-evidence.json`.
The Python acceptance guard execs the unchanged installed Rust frontend; it is
setup tooling, not a Python backend/service. Existing bound projects are never reset
or rebound. The [mock guide](../../live-scope-evaluation.md) is optional regression
reference, not another required operator walkthrough.

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

## Adaptive walkthrough — six invocations maximum

Count actual submissions/continuations, not these navigation steps. Skip a model
request when its result already exists; do not spend unused calls for coverage.

1. Discuss: “I want a local weather page using fixed sample data, with no API or
   network access. Help refine its scope; ask a substantive clarification if needed.”
   If a durable question appears, **Save answer** records your choice with **zero
   calls**; **Continue with coordinator** is a separate explicit submission using
   **one call**. Ordinary conversational questions can be answered in your next
   message. Saving alone does not ask the coordinator to continue.
2. If there is no saved proposal yet, request one: “Please save the proposal with
   objective, deliverables, scope, constraints, acceptance criteria and remaining
   choices.” Empty-input Enter opens Review scope. Inspect Purpose → Boundaries →
   Choices → Authorization. Continue/acknowledgement grant no permission; use Not now
   before the final action until you have completed the checks below.
3. **Edit scope**: change one meaningful field and save the local revision. This
   uses **zero calls**. Confirm the revision increments; review restarts at step 1.
4. Send “Looks good” as conversation, then verify the task still has no scope
   permission. Agreement in prose cannot authorize; a model may suggest a revision,
   which likewise needs a decision. This message uses **one call**.
5. Review the current revision's four steps. The final **Authorize scope** action
   saves exact-revision permission with **zero calls**. Expect **Scope authorized ·
   execution unavailable**. No worker runs. Inspection/history show the bound decision.
6. `/detach`, then `$waywr resume` (zero calls). Confirm history/decision remain.
   Request a genuine material change to the actual proposal, for example changing
   single-city selection to a three-city comparison, and ask to save the revision.
   This uses **one call**. Expect a new revision needing authorization; the old
   decision remains historical. Optionally authorize the new revision locally.

Usually 3–5 calls suffice; six is a ceiling, not a target. Clarification and saved
proposal may share a turn. If more are needed, record incomplete coverage; do not
renew. Any documented failure below ends further trial dispatch immediately, even
if allowance remains. Do not request formatting repairs or retry an interrupted call.

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
