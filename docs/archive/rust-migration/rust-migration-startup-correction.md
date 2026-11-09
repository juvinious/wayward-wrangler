> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Rust migration startup correction — installed simulated check

The operator's hands-on check of the corrective candidate found that launching
`waywr` inside Herdr left the invoking shell and opened a second coordinator
tab. That was a migration acceptance failure, not a new feature request.

## Trace and correction

The old Rust launcher called service `ui.attach` from a managed pane and
returned. The service used `layout.apply` to make a separate coordinator tab.
Outside Herdr, the launcher first opened a Herdr client, then requested the
same tab; the client could show its default shell tab before the coordinator.
This was one configured Herdr server with extra tabs, not evidence of a second
server or independent workflow controller. The installed Herdr version inspected
was **0.9.3**, socket protocol **22**. Its supported `workspace.create` creates
one bootstrap tab; `layout.apply` with that `tab_id` replaces it, while
`layout.apply` with `workspace_id` adds a tab. The latter behavior was checked
on a private server before changing the runner.

`waywr init .` and `waywr init SOME_DIR` now create or preserve project
configuration, state identity and release/Herdr bindings, then return without
starting a service or Herdr. Conflicting positional and `--project` selections
fail before project creation. New nested directories are supported; repeated
init preserves existing configuration.

`waywr` and `waywr resume` now run the accepted TUI in the invoking terminal.
Inside Herdr the service verifies the configured socket's server identity,
`pane.get` workspace/tab IDs, `pane.process_info` foreground PID, peer UID/PID,
and the bound release executable before claiming that pane. Outside Herdr the
launcher starts a headless configured Herdr server only if absent, without a
client or workspace, and the service verifies its peer/executable before a
project-scoped direct-terminal UI claim. A live claim in another process refuses
a second UI. Herdr worker workspace binding is separate from UI attachment;
the first worker replaces its verified idle bootstrap tab. The legacy
service-created coordinator-tab IPC requests now refuse in this Rust release.
Historical releases and records were not changed.

The exit/resume fixture exposed an additional confirmed race: exit finalization
could advance the state revision after `waywr resume` read its snapshot. Resume
now rereads and retries only after explicit `stale_state` or `service_stopping`
refusals, within 40 seconds. Uncertain command delivery still stops with the
command ID for inspection; it does not blindly resubmit.

## Evidence and limits

The immutable corrected candidate is
`.local/rust-migration/startup-direct/install/releases/rust-startup-direct-v5/`
(manifest SHA-256 `54444db203932edb4a1f481d5e91abe39c8d255209c4a56e8f9f0a1c076b29ba`).
The previous v7 candidate and its operator evaluation state remain unchanged.
The final private installed evidence is
`.local/rust-migration/startup-direct/installed-check-v5-final/`, with
`summary.json`, PTY captures, and `cleanup.json`. Earlier v1–v3 experiments
and the captured `stale_state` failure remain under the same ignored parent.

Focused Rust backend/frontend tests passed (37 + 9 + 25 + 2 + 6 tests, one
ignored). The installed private-Herdr fixture passed current/named/repeated
init, no service/tab on init, ordinary-terminal direct launch (0 workspaces,
0 tabs), first fake worker (1 workspace, 1 tab), inside-Herdr direct launch
without changing its 2-workspace/2-tab topology, duplicate UI refusal,
detach, reconnect, exit, resume after exit, startup failure return to shell,
refusal of the obsolete coordinator-tab request,
and project worker-workspace non-adoption. It used only fake workers. Three
additional v3 repetitions passed the previously failing resume race. The
fixture's unique test-owned service and private Herdr server were stopped.

This is simulated and PTY/API verification. The operator has not yet observed
the v5 candidate in Ghostty/Herdr. Native agent isolation and authenticated
calls remain blocked. A Herdr server identity change or ambiguous worker
execution still requires explicit reconciliation; the direct UI does not
authorize a new model attempt. A verified but unresolved workspace-routing
notice stops further dispatch for inspection.

## Fish hands-on check

Use a fresh disposable project because existing projects remain bound to
their recorded release. The profile initialized here is fake-only.

```fish
set ww ~/projects/example-workspace/.local/rust-migration/startup-direct/install/releases/rust-startup-direct-v5/bin/waywr
set project ~/projects/example-workspace/temp/waywr-direct-startup-eval
mkdir -p $project
cd $project
$ww init .
$ww
```

From the same directory, use `$ww resume` after `/exit`; use `$ww` again after
`/detach`. Run the same `$ww` command from an existing Herdr pane to verify
that its pane becomes the TUI and returns to its shell on detach. Workers
remain separately visible in the project's owned Herdr workspace. These
commands do not select a default release or modify shared Herdr settings.
