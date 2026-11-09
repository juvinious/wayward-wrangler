> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

> Historical snapshot from `fce5ad3`, before the final operator authorization
> audit and v6 sequential-review correction. Current status is in
> [the current report](../../live-scope-report.md); these are not current instructions.

# Saved-scope evaluation — installed mock transport

Candidate **claude-scope-v5** uses the accepted production TUI/service/runner.
This launcher selects an isolated paired release and private fixture configuration;
it cannot switch to a native provider or rebind an existing project. Nothing is
installed globally. All replies are visibly **MOCK CLI STREAM** and scripted by
call number, not natural-language interpretation. Real scope conversation is
[separately proposed](../../claude-scope-live-acceptance.md), not authorized.

## Fish launch and reopen

The reusable fixture is initialized and bound to v5, with 4/6 mock calls charged
at the operator follow-up. Repeated `init .` preserves it; do not reset it:

```fish
cd ~/projects/example-workspace/.local/claude-scope/evaluation/operator-eval
set -l waywr ~/projects/example-workspace/.local/claude-scope/waywr
$waywr init .
$waywr
```

Later, from that directory: `$waywr resume`. No environment variables, manual
service/runner commands or Herdr setup. Repeated init preserves the project.
The launcher refuses unrelated projects and different release/native bindings.

## Short walkthrough

1. Empty-input Enter opens Settings (or F3). Review the mock six-call profile;
   Tab selects its explicit allowance action, Enter saves it. No call yet.
2. Send a discussion message: mock call 1 replies without making a task. Keep
   typing a draft during output. Enter sends; Shift+Enter/F2 and paste retain
   accepted behavior. Clear that draft explicitly before the next submission.
3. Send a clarification request: scripted call 2 asks about temperature units.
   Empty-input Enter opens **Answer question**. Arrows select a choice; Tab to
   **Save answer**, Enter saves without a call. The visible **Continue with
   coordinator** action opens a separate panel; Tab/Enter invokes call 3 once.
4. **Review scope** now opens the saved proposal. Read fields/choices; Up/Down
   scroll one line, PageUp/PageDown a page, with a hard content boundary. Opening
   the panel grants nothing. Tab selects actions; Escape returns to your draft.
5. Optional **Edit scope** reuses the textarea. It opens on Objective/Text;
   edit normally or paste. Tab to **Save revision**, Enter saves locally. To edit
   another field/rationale, Tab to Field, arrows choose, Tab returns to Text.
   Lists use one item per line. Escape retains the draft; Discard is explicit.
6. Wait for **Ready for explicit scope authorization**. Review/acknowledge listed
   unresolved choices, then separately select **Authorize scope**. The result is
   **Scope authorized · execution unavailable**. No worker or task allowance.
7. `/detach` closes only the interface; reopen with `$waywr resume`. `/exit`
   checkpoints/pauses; resume restores decisions, drafts and remaining allowance.
8. If exercising the rest: call 4 changes scope, invalidating effective authority;
   call 5 (for example “looks good”) grants no authority; call 6 changes it again.
   Inspect prior decisions and explicitly authorize the newest revision. At 6/6,
   local editing/reading/answering/authorization still work, but model continuation
   cannot. This sequencing is fixture behavior, not how a real LLM interprets you.

`/plan` reads the selected scope; `/inspect` shows revisions, provenance, decisions
and call/receipt references. `/output` shows provisional coordinator text, not
worker activity. F6 selects tasks if more than one exists. F4 distinguishes
selected-scope discussion from a new topic with no selected scope.

Mock allowances never authorize native calls. Workers, consultation, execution
admission, repository actions and generated files remain unavailable. The original
exhausted `claude-conversation-v5` live trial is untouched.

## Recorded observations and remaining check

The operator verified the mock discussion/question/review/local-edit path,
detach/re-attach and optional refinement. The read-only audit confirms three
revisions and four receipts, but **no scope authorization decision** yet.
Acknowledging unresolved choices does not authorize: reopen **Review scope**,
acknowledge the current revision, then select **Authorize scope** explicitly.
Expect **Scope authorized · execution unavailable**. No call is needed.
The existing generic “authorize again / prior decision” revision event is misleading
when no decision exists; saved records retain the correct `proposed` state.

Automated PTY results remain separate. Physical exit and other input observations
are not inferred. Native structured-response reliability, interruption and scope
continuity require separate authorization/evidence; mock responses do not test
natural-language quality. [Report](../../live-scope-report.md) owns the audit/evidence.
