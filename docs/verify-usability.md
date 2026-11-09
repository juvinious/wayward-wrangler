# Verify the daily-use loop

The local implementation is ready for user verification on Linux x86_64 with a
systemd user manager and a working authenticated Codex CLI. Claude is an optional
explicit profile choice. See [source prerequisites](../README.md#build-and-install-from-source)
and [sandbox requirements](linux-sandbox-setup.md). Automated metadata checks do
not establish authentication, sandbox usability or model reliability.

The agent left the operator installation absent and made only local commits on
`plan/usability-and-installation`. You review and perform remote delivery.

## Install and use

From this Waywr checkout, in Fish:

```fish
sh scripts/dev-install.sh
fish_add_path "$HOME/.local/bin" # once, if needed
waywr --version
```

Then enter the project you want to use and run `waywr`. A new project prompts for
setup. Existing project work/settings/history are preserved; there is no
per-project application version to select. Your existing CLI login remains in
place; installation does not sign in or modify provider configuration.

Discuss one small useful development request. Review its scope/policy card and
authorize it only when correct. Local delivery keeps this first verification
within your checkout; remote actions remain your decision. Inspect the actual
changes and check results. Record any manual intervention, confusing choice,
unexpected block or missing explanation.

## Rebuild and return

Use `/exit` to save/pause and close the TUI. From the Waywr source checkout,
repeat `sh scripts/dev-install.sh` after a local application change. Then return
to the same project and run `waywr` again. `waywr --version` shows the new generated
artifact ID. No manual bundle IDs, rebinding or project upgrade command is needed.
Conversation, settings, draft/work and intentional pause should remain. Resume
only when you intend to restart dispatch.

An idle old service is replaced automatically. A live TUI, pending client input
or uncertain launch keeps its existing service; the error explains why replacement
waits. Do not force-stop it or delete its state to proceed. `/detach` deliberately
leaves background work running. Older installation artifacts are retained for now;
automatic cleanup and uninstall commands are subsequent work.

The success criterion is completing useful work, rebuilding Waywr and continuing
in that project without repair commands. Report concrete steps and the visible
failure when that loop is interrupted. Public curl installation, `waywr update` /
`uninstall` and additional package channels are not implemented by this increment;
no hosted binary release has been published.
