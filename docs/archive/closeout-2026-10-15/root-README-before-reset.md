# Wayward Wrangler

Persistent, project-scoped workflows for coding agents. `waywr` is the primary
command; `wayward-wrangler` and `awp` remain compatibility aliases.

## Ordinary use

With a compatible installed release on your `PATH`:

```fish
cd /path/to/project
waywr init .
waywr
```

`waywr init SOME_DIR` initializes that directory and returns to the shell.
Repeated initialization preserves its configuration, state, and release binding.
`waywr` opens the Rust conversation-first TUI in the invoking terminal;
`waywr resume` reconciles and restores that project's session. `--project`
selects a project explicitly when invoked elsewhere; use spaced `--project DIR`
(the bare CLI equals form has a recorded routing limitation). `/detach` closes the
interface while supervised work continues. `/exit` requests a project-scoped
dispatch pause and checkpoint; it is distinct from task cancellation.

Configuration, project identity, context, decisions, artifacts, and execution
records belong to the selected project. Default state is
`PROJECT/.wayward-wrangler/state/`; `--state-dir` is an explicit override.
Initialization does not start a service or create Herdr resources. New Rust
projects use project-owned systemd user services for execution. Herdr is an
optional, explicitly requested read-only output view; closing it does not stop
work. Linux with a compatible systemd user manager is the current execution
platform. Existing Herdr-backed and unfinished LangGraph runs retain their
bound legacy releases.

The current isolated **claude-dialogue-v8** candidate implements continuous Claude
conversation and immutable proposals/local edits/exact human **scope** authorization
through this production TUI. New projects use schema 7. Pending questions/scopes do
not block discussion; the queue and Interrupt are visible. Scope ends at **Scope
authorized · execution unavailable**: no workers, consultation, repository actions
or task-call allowance. Ordinary input reaches the selected adapter unchanged;
mock transport is explicit and never a native fallback.

No global installation occurred. [Retained trial commands](docs/continuous-coordinator-evaluation.md)
use one isolated launcher with automatic project-service management. The
[current report](docs/continuous-coordinator-report.md) records the accepted bounded
v8 native workflow: four completed Claude calls/receipts, saved proposal/revision,
exact human scope permission, physical detach/re-attach, then checkpointed exit.
Two unused calls remain within the original six-call ceiling; no renewal or further
call is requested. Worker execution and a working generated application remain unverified.
[Native readiness](docs/continuous-native-readiness.md) distinguishes the passed
builtin/context proof from remaining native queue/interruption/recovery coverage.
The previous [scope trial](docs/claude-scope-live-acceptance.md) remains blocked at
1/6 on its original v6 release after invalid framing; it is not repaired or resumed.
The original `claude-conversation-v5` is separate: three native calls/receipts and
operator detach/resume verified, now exhausted and preserved. Existing schema-5/6
projects retain their bound contracts; no automatic migration or allowance renewal.

The earlier operator-accepted simulated v3 comparison and schema-4 mock projects
retain their bound releases. Commands/evidence are in
[current limits and baseline](docs/current-limits.md); those fixtures are not
model-backed conversation. No working weather application is verified.

Start with the [documentation index](docs/README.md) and
[current handoff](docs/HANDOFF.md). Historical setup and milestone commands
remain in [the archived README](docs/archive/legacy-readme/README-2026-10-13-pre-consolidation.md).
