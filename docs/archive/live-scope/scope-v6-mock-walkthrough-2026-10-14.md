> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical v6 mock walkthrough, archived at the continuous-conversation planning checkpoint. This is regression evidence, not a current hands-on request or native authorization.

# Sequential scope review — isolated mock evaluation

Candidate **claude-scope-v6** uses the production TUI/service/runner with the new
four-step review. Replies are labelled **MOCK CLI STREAM**, scripted by call number;
this evaluates interaction, not natural LLM behavior or a working weather page.
This mock launcher authorizes no native calls. The
[operator-owned live trial](claude-scope-live-acceptance.md) stopped on its first call;
mock verification does not substitute for that failed native gate.
Nothing is globally installed.

## Fish launch / reopen

V5's authorized project is preserved on v5; this separate fixture avoids rebinding
or restarting it. The launcher selects v6 and private test configuration internally:

```fish
cd ~/projects/example-workspace/.local/claude-scope/sequential-review/evaluation/operator-eval
set -l waywr ~/projects/example-workspace/.local/claude-scope/sequential-review/waywr
$waywr init .
$waywr
```

Later from that directory: `$waywr resume`. Repeated init preserves state. No manual
services, environment overrides, runners or Herdr setup. The launcher rejects native
profiles, different release bindings and unrelated projects.

## Reach the proposal

1. F3: review/save the mock six-call allowance. Opening settings spends nothing.
2. Send any discussion message (call 1), then another message (call 2 asks units).
3. Empty-input Enter opens **Answer question**. Arrows choose; Tab to **Save answer**,
   Enter saves without a call. Empty-input Enter opens **Continue with coordinator**;
   Tab/Enter on its explicit continuation uses call 3 and saves a proposal.

This fixed ordering is mock behavior, not phrase recognition in production.

## Review one step at a time

Empty-input Enter opens **Review scope**, focused on the primary action:

| Step | Content | Primary action |
| --- | --- | --- |
| 1 of 4 | Purpose and deliverables | Continue to boundaries |
| 2 of 4 | Scope, constraints and acceptance | Continue to choices |
| 3 of 4 | Unresolved choices and linked answers | Acknowledge choices · continue |
| 4 of 4 | Exact revision and permission consequences | Authorize scope |

Enter activates the focused action. Steps 1–3 save **no authorization**. After
acknowledging, the separate final action saves **Scope authorized · execution
unavailable**. If unavailable, the final step names the actual remaining condition.

Tab/Shift-Tab select **Previous step**, **Edit scope**, **Request revision · 1 call**
or **Not now**. These remain auxiliary, not a list of future steps. Escape/Not now
close without a decision. There is no unsupported decline/reject operation.
Up/Down scroll one line; PageUp/PageDown scroll a page, stopping at content bounds.

Edit uses the same textarea: Tab selects Field/Text/Save/Back/Discard; arrows choose
a field at Field focus; lists are one item per line. Enter inserts a newline in Text.
Save requires a rationale and creates a local revision without a call. Escape keeps
that draft. A changed saved revision restarts review at step 1; unchanged background
refresh preserves step, focus and position. No refresh opens or authorizes a panel.

After authorization, reopening shows the result, with no further authorization
button. `/detach` and resume should preserve history/decision/drafts; `/exit` saves
and pauses. **No worker launches or generated files are part of this check.**

## Verification distinction / preserved V5

V5 operator review/edit/refinement, explicit revision-3 authorization and physical
detach/reopen are recorded. Four mock calls/four receipts, one scope decision.
V6 automated installed interaction passed; **physical V6 acceptance is pending**.
[Report](live-scope-report.md) owns exact checks, source/binary hashes and evidence.
[Native trial](claude-scope-live-acceptance.md) is stopped after output-invalid;
no further submissions or renewal are authorized.

To inspect the preserved v5 project, keep using its unchanged launcher
`.local/claude-scope/waywr` from `.local/claude-scope/evaluation/operator-eval/`.
Do not launch v6 there or reset/rebind either project.
