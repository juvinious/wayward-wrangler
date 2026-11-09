> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Terminal presentation and review links — outcomes

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

October 10, 2026. Current usability follow-up to milestone 1; no milestone 2 work.

## Verified outcome

The operator completed the demo: stored status is **approved**, D-retention is
answered **30 days**, and A1 approves revision-2 at
`d4f8cae32c34a7da60468b87753d5326fe10c88334eefc75af617c5ac5310d20`.
All five stages have invocation count **1**. No question or approval was answered
by the development agent. The earlier literal `approve SHA256` attempt was
refused; the operator subsequently recorded the actual hash. No gate is pending.

This approves the simulated draft only. It does not authorize real workers or a
later implementation milestone. Retention remains a proposal, not a deletion setting.

## Changes delivered

- Cyan identifies author work/headings, magenta critic work, green successful
  outcomes, yellow decisions/warnings, red failures/refusals, blue actions.
  Status labels remain readable without color. Automatic color follows TTY
  detection, TERM and NO_COLOR; `awp --color always|never` overrides it.
  JSON status stays plain. Stored worker bytes, receipts and artifacts stay plain.
- The review artifact has an OSC 8 **Open in Vim** link and an explicit fallback
  command. `open` or `open critique-2` works at a newly started controller prompt.
- `awp open demo --artifact latest` uses the run's saved workspace and socket,
  even when invoked from a different workspace on that same Herdr server.
  It rejects a different server rather than targeting the wrong session.
- Read-only Vim uses direct argv, disabled modelines/startup scripts, no swap or
  viminfo, and a hash/path check before opening. It is a review viewer, not a
  permission sandbox against deliberate same-user editing.
- Repeat opening reuses an existing matching single-pane Vim tab in that run's
  workspace. Closing Vim with `:q` allows a later fresh review tab. A timeout
  produces an error; it does not silently retry creation. A later explicit
  attempt scans layouts first. Hard lost-response races remain unproven.
- The approval prompt now prints the complete `approve <actual hash>` command.
  Invalid input reprints the current summary. Approval validation is unchanged.

## Plugin installed

**local.agent-workflow-review 0.1.0** is linked and enabled. Source is
`herdr-plugin/` in this standalone project. Registration is user-wide in Herdr,
visible across workspaces; existing bindings/settings were not rewritten.
No build hook, startup hook, event hook, dependency installation, HTTP server,
worker launch, or automatic workflow action is included.

The only handler matches `https://awp.invalid/review/RUN/STAGE/SHA256`.
This reserved `.invalid` URL is a local routing token, not a hosted service.
The plugin accepts only known stage names and exact hashes, validates the saved
artifact, and invokes `awp open-link`. No URL fragment becomes a shell command.
Opening or clicking cannot record a decision or approval.

Use **Ctrl-click captured by Herdr**. A terminal-native bypass or disabled
plugin may send the token to a browser, which cannot resolve this `.invalid`
site; use `awp open` in that case. No external HTTP service is contacted by the
plugin. Ghostty/Herdr modifier handling still needs operator verification.

## Evidence and limits

| Check | Result |
|---|---|
| Existing offline suite plus presentation/review tests | **20 passed** |
| URL/manifest matching; arbitrary URL, path escape and stale hash refusal | Passed offline |
| Read-only direct Vim argv; run workspace selection; repeat-tab reuse | Passed with mocked Herdr |
| Review action leaves run/artifacts unchanged; cannot answer a gate | Passed offline |
| ANSI auto/NO_COLOR/dumb terminal; OSC 8/plain fallback; plain JSON | Passed offline |
| Plugin manifest accepted and linked by installed Herdr 0.9.3 | **Runtime verified** |
| Plugin listing reports enabled 0.1.0 | **Runtime verified** |
| Demo user approval and five invocation counters | **Verified from local persisted records** |
| Click Open in Vim in Ghostty/Herdr → review opens | **Operator confirmed working** |
| Repeat click reuses the tab; close/return behavior | **Not yet explicitly confirmed by operator** |
| Live worker display after color changes | **Not rerun**; no new workers launched |

Machine-readable outcome and source fingerprints:
`.local/evidence/terminal-review.json`.
Tests: `tests/test_terminal_review.py`, plus the existing contract, rebind and
controller-exit tests. No hidden detached worker or new run was introduced.
This turn created no runtime panes or retained processes.

## Exact Herdr interface inspected

Installed CLI: **0.9.3**. Official source tag v0.9.3, commit
**7b116c05bfda646af39d2524c54e70c751f57ee8**; installed binary was not rebuilt or
matched byte-for-byte. Source was already available in a separate /tmp checkout.

- [safe_web_url](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/src/app/actions.rs#L1035)
  permits http/https only. A file URL alone cannot provide this action.
- [Plugin manifest and link handlers](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/docs/next/website/src/content/docs/plugins.mdx)
  documents Ctrl-click interception, local linking, user-wide registration, and
  clicked URL context. Official current [plugin documentation](https://herdr.dev/docs/plugins/)
  was also checked; source at the pinned version controls this implementation.
- [Plugin runtime](https://github.com/herdrdev/herdr/blob/7b116c05bfda646af39d2524c54e70c751f57ee8/src/app/api/plugins/runtime.rs)
  supplies HERDR_ENV, socket path and clicked URL to action processes.
- [Ghostty configuration](https://ghostty.org/docs/config/reference)
  documents terminal-native URL opening through the system opener; it does not
  itself route an arbitrary file link into a Herdr Vim tab.

## Hands-on check (no new run)

From any shell in the same Herdr session:

```fish
cd ~/projects/example-workspace
.venv/bin/awp status demo
```

Expect a green approved status and an **Open in Vim** link. Ctrl-click it.
Expect a tab labeled `AWP review demo revision-2 d4f8cae3` in workspace
`example-workspace`. Inspect the file, return to the shell tab and click again;
expect the same review tab, not another copy. Exit Vim with `:q` when done.
A direct fallback is:

```fish
.venv/bin/awp open demo --artifact latest
```

Existing controller processes retain the old code until exited/reopened. A new
`status` invocation immediately shows the new output; do not launch a new demo
merely to refresh colors. The already-approved demo stays approved.

If needed, inspect plugin failures:

```fish
herdr plugin log list --plugin local.agent-workflow-review
```

Re-register after moving/changing the plugin manifest, or unregister explicitly:

```fish
herdr plugin link ~/projects/example-workspace/herdr-plugin
herdr plugin unlink local.agent-workflow-review
```

These are operator commands; unlink was not executed. Keep this report updated
with the operator's click/reuse result rather than assuming successful interaction.

## First operator click check

The operator reported that right-click showed a context menu and no review tab.
A fresh installed-plugin query still reports local.agent-workflow-review 0.1.0
enabled. The current server's plugin action log list was empty: no invocation
was recorded there. This does not demonstrate a Vim launch failure. Next check:
run a fresh `awp status demo` in Herdr and use **Ctrl + left-click** on the
new Open in Vim link. Old controller scrollback contains plain paths, not the
new link. Physical click routing remains unverified.

## Operator confirmation

The operator subsequently confirmed: “ok that worked I had to click on open in
vim...” The visible Open in Vim link successfully opened the review in their
Ghostty/Herdr session. This is operator-reported runtime evidence. Repeat-click
tab reuse and close/return behavior remain unconfirmed by the operator; offline
reuse checks already pass. No further run, decision or approval was triggered
by recording this outcome.

## Latest operator confirmation and next scope

The operator explicitly verified that the Herdr review plugin works. No additional
interactions are inferred. They subsequently authorized one bounded milestone-2
Claude/Codex trial; see [the separate milestone-2 report](../milestone-2/milestone-2-report.md).
The simulated demo and its approval remain unchanged.
