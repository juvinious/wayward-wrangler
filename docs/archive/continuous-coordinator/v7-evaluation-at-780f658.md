> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Corrected candidate — short isolated mock check

Current candidate **claude-dialogue-v7**, manifest
`5e51bc0fcd218b8e47a74ab83e8d647420e849fd8ba114e809a6317c7002aa6b`.
This is the installed production TUI with **MOCK CLI STREAM** regression transport,
not a native conversation or a global installation. No extra mock walkthrough is
required as native acceptance. Automated results are in the [report](continuous-coordinator-report.md).

```fish
cd ~/projects/example-workspace/.local/continuous-coordinator/evaluation/continuous-eval-01
set -l ww ~/projects/example-workspace/.local/continuous-coordinator/waywr
$ww init .
$ww
```

The reusable project is already initialized; repeat init preserves its binding and
configuration. It launches no service. The launcher keeps test configuration internal;
launch/resume automatically starts/reconnects the project service. It refuses other
projects/releases and cannot invoke the real provider. Do not rebind an old trial.

Later, in that directory: `$ww resume`. Normal product usage, once globally installed,
remains `cd PROJECT; waywr init .; waywr`; the evaluation binary is not on global PATH.

Optional quick observations:
- F3 reviews/authorizes six **mock** invocations. Ordinary nonempty Send still works
  with a pending question/scope or running call; no navigation or speech authorizes.
- Empty Enter opens the displayed action; while running it opens Interrupt confirmation.
  F7 shows queued/held messages, reason/generation and explicit release/withdraw controls.
  Escape closes a panel; Escape in the main view returns focus to the composer.
- Fixture turns 1–6 exercise discussion, question, source-linked formal-answer suggestion,
  proposal, discussion, revision. Only Record saves the inferred formal answer. Optional
  /plan review is Purpose → Boundaries → Choices → Authorization; only its final action
  saves exact scope permission. Local edits need no call.
- /detach closes the UI; /exit confirms checkpointed session exit. Neither resets budgets.
  Saved scope permission is labelled **Scope authorized · execution unavailable**.

The scripted scenario is not a natural-language interpreter. Do not use it to judge
LLM conversation quality. [Native readiness](continuous-native-readiness.md) is blocked on the plugin/context
proof. No live request is ready, and no native allowance was prepared or spent here.
