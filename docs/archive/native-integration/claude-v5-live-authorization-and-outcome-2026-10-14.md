> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# First live Claude conversation — authorized operator check

**Operator authorized this exact three-call check on 2026-10-13.** The operator
will launch the TUI, review F3, save the allowance and submit messages personally.
The development agent must not spend any call on a greeting, connectivity check
or automated test. The operator has now used all three calls; read-only audit
verified three completed receipts and zero worker/consultation dispatch.
**Allowance exhausted. No additional native call is authorized.** Operator
observations remain separate from automated evidence in the [report](claude-conversation-closeout-2026-10-14.md#operator-observations-and-read-only-native-outcome-verification).
This authorization covers only conversation in the accepted production TUI. It does not
authorize consultation, tasks, workers, repository edits or the weather trial.

## Candidate and one launch command

Release `claude-conversation-v5`, source
`717348b4ae88e618a3768d9b226ec26ad91329be`; paired installation:
`.local/claude-conversation/install-01/releases/claude-conversation-v5/`.
Manifest SHA-256:
`ae55df4c9a259adf16506a75981179c8a01a2d4d3fbba4bc5b9b4137dbeafe6d`.
The launcher selects this release and the fresh `.local/claude-conversation/live-eval/`
project internally, without global installation or operator-project rebinding.
At preparation, initialization was complete and calls used: **0**. Verified
post-evaluation counts are now **3/3 used**, all completed. No manual environment, runner,
systemctl or Herdr setup is needed.

Launch from Fish:

```fish
~/projects/example-workspace/.local/claude-conversation/waywr-live
```

This check is complete; the launcher can inspect retained history or reconnect
with `resume`, but does not renew the allowance. During the check the operator
reviewed F3, submitted ordinary text, detached and reattached, and received the
third reply. No preliminary developer call occurred. The installed v5 still
contains the reported Markdown rendering defect; its focused source correction
has not been installed into this bound project.

## Exact scope

| Setting | Requested value |
| --- | --- |
| Provider/client | Claude Code 2.1.296, `~/.local/share/claude/versions/2.1.296` |
| Binary SHA-256 | `24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de` |
| Model / effort | `claude-sonnet-5-5` / `medium`; no substitution or escalation |
| Authentication / route | Existing saved Claude CLI login in `~/.claude/`, used in place; normal Anthropic route. No API-key injection, secret copying, provider/billing switch |
| Budget | At most **3 coordinator CLI invocations total**, workers 0, consultation 0; no automatic retry or fallback |
| Deadline / response | 120 s per call; assistant text ≤32 KiB; existing bounded stdout/stderr transcript policy |

CLI launches are not guaranteed HTTP-request, token or dollar limits. The account
must support the selected model/effort; refusal stops the check rather than
substituting another route/model. The application does not verify subscription
entitlement or billed cost from file presence.

## Context, client access and restrictions

Only the current message, at most six prior project messages, and a deterministic
project handoff (bounded metadata/decisions/budgets/provenance) enter the model
packet, ≤96 KiB. This fresh project has no imported files, tasks or prior provider
session. No registry/unrelated-project content or automatic file context. Waywr
owns authoritative history; each turn uses a fresh client, not provider resume.

Safe/restricted mode, no settings sources, disabled hooks/slash commands/history/
memory/session persistence, empty/deny-all tools, strict empty MCP, owned system
prompt, exact model/effort, one agentic turn and zero configured retries are
frozen. No model-controlled file reads/writes, shell/process/delegation, external
actions or Waywr-state mutation. Output is inert conversation text, never code
execution, authorization, approval or task admission.

The trusted client may use its existing runtime/auth directory and operation-local
scratch; secrets are never selected into model context or copied into evidence.
Only an allowlisted process environment is forwarded. Known managed configuration
refuses admission pending boundary review. This is not whole-process confinement
against a malicious client or another same-UID process.

The [report](claude-conversation-closeout-2026-10-14.md) records fifteen pinned-client no-egress,
dummy-credential boundary cases and installed mock-transport checks, followed by
three completed native calls and the operator's physical detach/resume report.
Native cancellation/crash recovery, latency bounds, wider account-policy paths
and physical exit remain unverified. No mock response is a fallback.

## Outcomes and stop conditions

- Before authorization: **Awaiting authorization**, service connection shown
  separately; no call. Missing CLI/login, changed binary/profile or incompatible
  policy gives actionable blocked guidance.
- Running: assistant text is visibly **provisional**; thinking is not shown.
  Validated completion saves the assistant message, receipt and reported usage
  (cost, if reported, is labelled a client estimate).
- Detach closes the UI while the current call remains supervised. Resume
  reconciles that call and never starts a duplicate.
- `/cancel` requires an explicit interruption action targeting the active turn.
  Completion/interruption races use the retained authoritative receipt.
- Failed, cancelled, output-invalid or uncertain: retained partial output and
  execution evidence remain inspectable; further trial calls stop. Ambiguous
  execution is never replayed. An unavailable model or exceeded deadline stops
  with evidence; no fallback or budget reset.
- Exhausted after three charged calls: history stays readable; new submission is
  refused and draft preserved. Reopening or reselection does not renew allowance.

Success means real, nonscripted conversation streams and persists in Waywr, with
correct continuity and no task creation. It is not acceptance of workers, coding,
the weather application or all recovery windows. Stop at the first unresolved
failure/capability drift; preserve evidence for diagnosis without another call.
