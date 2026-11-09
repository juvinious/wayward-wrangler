# Waywr handoff — populated-project snapshot delivery

Branch: `plan/usability-and-installation`. Base: `d49ccee` (portable handover).
Follow [AGENTS.md](../AGENTS.md), [IPC contract](contracts/session-protocol.md) and
[distribution contract](distribution.md). Local commits only; the user handles
remote delivery. No operator service restart, preserved-project edit, global
installation or account/model call was performed by the agent.

## Observed failure and fix

The operator's handover succeeded, but normal opening twice returned connection
closed. Read-only status/journal/socket diagnosis established that the new service
was active, accepted hello and answered status, then closed on snapshot. Read-only
SQLite size aggregates showed the complete catalog alone uses560KB alongside
substantial retained work. No operator history/prose was copied into fixtures.
An isolated populated-project TUI reproduced the exact EOF before the composer.

IPC now separates requests from replies through shared `ipc_limits.rs` constants:
incoming frames stay1MiB, mutation commands stay64KiB, and complete replies may be
8MiB. No model entries or retained history are removed to fit. Replies above8MiB
return a correlated response_too_large error with kind/bytes/budget; the connection
remains usable. No partial snapshot is applied and the error grants no replay.
The whole-snapshot architecture remains bounded; larger projects will need scoped/
paged views. This slice does not claim arbitrary-size history delivery.

Handover uses the old service's bounded session.status reply to prove its project
instead of requiring a full snapshot that the old service cannot serialize.
Exact process/executable/command, socket peer PID, project key, ownership and
execution/quiescence checks remain. An unrelated, attached, pending or uncertain
owner is still preserved. The new service can then deliver the complete snapshot.

The Thinking selector, runtime-advertised levels, draft/active model-level labels,
complete model picker and broad shared SDK execution path are unchanged. No provider
or model allowlist was introduced. Saved profile/context/policy grants and all
retained evidence remain intact. Real account login, model quality/useful delivery
and public download/update/package channels remain unverified or incomplete.

## Verification

Frontend socket checks deliver a reply above1MiB while still rejecting expanded
request/command inputs and enforcing the reply ceiling. The service overflow check
receives the correlated error and successfully reads the next reply on the same
socket. Both use isolated socket pairs. The populated paired-binary/TUI fixture
opens, chooses/saves model and thinking settings, removes the old build and reopens
with the full history, settings, draft and pause preserved; no model call occurs.

A real-systemd portable upgrade fixture copies the previous1MiB binary into its
own temporary bundle/project. It first proves that service accepts hello/status
but closes its populated snapshot. The new launcher preserves busy UI/pending/
uncertain ownership, then hands over after detach and opens the complete project.
The old bundle is removed, PID/generation advance and retained settings/draft stay
unchanged. Test units self-clean; this is not an operator-project trial.

The remapped locked release build and source/binary/bundle public-content checks
pass. CI/release arguments include populated history plus focused IPC budget/error
checks; those workflow edits are local, not hosted CI evidence. Dependencies and
Cargo/npm locks remain unchanged. No remote operations were performed.

## Next authorized action

The ignored `target/local-bin/waywr` entry points to `v0.4.0-local.snapshots`,
manifest SHA256 `aad6e9c89aaefcf0087a2605559af9fb6cb4b02dfbc9f42bfe652f78105a561c`.
The user retries the same local command. The agent has left the operator's
running service/state unchanged; the user's launcher performs any verified idle
handover. Do not force-stop or infer settlement if a concrete ownership refusal
remains. Continue from ordinary-use feedback without another broad test campaign.

Owners: [IPC](contracts/session-protocol.md), [distribution](distribution.md),
[verification](verify-usability.md), [providers](provider-selection.md),
[controls](tui-controls.md), [architecture](architecture.md).
