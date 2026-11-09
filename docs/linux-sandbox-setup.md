# Linux worker sandbox setup

Waywr currently launches Codex 0.162.1 with its workspace-write sandbox. The Linux
host must support that sandbox in addition to the systemd user manager. A working
conversation does not prove worker command execution is available.

## Retained testing setup, 2026-10-15

The first Example Task App workers reported `bwrap: setting up uid map: Permission denied`
before editing files or creating commits. The user installed and loaded
`/etc/apparmor.d/waywr-codex-0.162.1` to allow user namespaces for one installed
binary. Its home path is generalized below; this is not a copyable profile rule:

```text
~/.codex/packages/standalone/releases/0.162.1-x86_64-unknown-linux-musl/bin/codex
```

The profile uses `flags=(unconfined)` with `userns,`. It enables the executable to
create its own sandbox; it is not itself a filesystem confinement profile. Codex's
workspace-write restrictions remain enabled. No global kernel restriction was
disabled, and shared provider configuration/credentials were not changed.

A local `codex sandbox` invocation under a systemd user unit subsequently ran
`/usr/bin/pwd` successfully (exit0). This verifies sandbox startup, not application
delivery, model correctness or all sandbox boundaries. Native workers and their
actual submitted code/checks establish the development result; Example Task App has now
passed final combined checks and reached checked clean local main, as recorded in
[HANDOFF](HANDOFF.md).

This is an explicitly accepted **temporary testing workaround**. The profile is
specific to a username, installation path and CLI version; moving/updating the
executable can invalidate its match. Do not copy it as a portable installation
recipe or silently broaden it. Preserve it unless the operator authorizes changes.

## Distribution work still required

[Official Codex sandbox prerequisites](https://learn.chatgpt.com/docs/sandboxing?sandbox-os=ubuntu-debian)
recommend the distribution-provided `bubblewrap` executable and, where needed, its
AppArmor profile. Prefer that supported setup when preparing distribution.

Waywr's current native preflight checks the CLI version, not sandbox usability.
Before publication, detect unavailable worker execution before dispatch, explain
the concrete cause in the TUI, and document supported distribution/CLI setup and
upgrade behavior. Any required administrator step must be visible and explicit;
ordinary operation should not require `sudo`. Never silently disable sandboxing or
global OS restrictions. Current execution support is Linux with a systemd user
manager; other operating systems are not established by this testing setup.

The produced offline Example Task App CLI does not use Codex or require this worker
sandbox setup at runtime. Its delivery status is recorded in [HANDOFF](HANDOFF.md).

## Git metadata is a separate permission boundary

A real worker subsequently built/tested the skeleton but could not create Git
`index.lock`: Codex's workspace preset protects Git metadata even with a broad
`--add-dir`. Waywr uses an explicit profile extending `:workspace` with narrow
worker-owned Git metadata/ref grants. Main/integration refs remain read-only.
This is a project invocation configuration fix, not another AppArmor exception.
A metadata-only local check demonstrated a task commit and rejection of main-ref
mutation. Subsequent native workers committed the application, the service reviewed
and integrated their exact submissions, and final delivery succeeded.
