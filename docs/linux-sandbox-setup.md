# Linux requirements and execution boundary

The initial binary target is Linux x86_64 with glibc2.39 or newer (Ubuntu24.04 is
the build baseline). Waywr starts one project service through the systemd user
manager. The terminal session must have access to that manager/session bus and
provide `systemctl` and `systemd-run`.

A built application also needs Git, `/bin/sh`, Node >=22.19 and util-linux `flock`.
Rust, npm and Python are source-build tools; they are not required to run a binary
bundle. See [installation](distribution.md) for the current methods and limitations.

## Model access

Choose a provider/model in F3 and sign in with `/login`, or use the SDK's supported
credential environment variables. The shared runtime uses provider libraries; it
does not need a separately installed Codex or Claude CLI. See
[provider setup](provider-selection.md#configuration-and-authentication).

## Trust boundary

The model runtime and project commands run as your local user. Worker worktrees
separate task files and Git work; they are not an operating-system sandbox. Task
instructions and exact-result acceptance do not prevent a process running as your
user from accessing other files that user can read. Use trusted projects and
provider configurations. See [security](../SECURITY.md).

The former version-specific Codex AppArmor setup does not apply to this shared
runtime. It is retained only as a [historical report](reports/retired-native-linux-setup-2026-10-15.md).
A systemd connection failure needs the user-manager/session setup repaired; a
missing Node or flock needs that runtime prerequisite installed. Installing Waywr
does not change system security profiles or global provider configuration.
