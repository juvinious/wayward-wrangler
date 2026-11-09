> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Bootstrap-tab refusal correction

**Current relevance:** this is the retained evidence and unresolved recovery
boundary for the operator's legacy `rust-local-init-v3` task. Its old
candidate/build and Herdr-required behavior are historical, not the current
host-backed release. The [handoff](../../HANDOFF.md) owns the task's last-known
status; the [current architecture](../../architecture.md) owns new operations.

## Operator finding

On 2026-10-12, the operator submitted a simulated task in
`~/projects/waywr-rust-sim-eval`. The project is bound to
`rust-local-init-v3`, under ignored
`.local/rust-migration/stale-registry-fix/install-v3/releases/`. The affected
task is `<private-record>`. Its frozen coordinator packet,
operation, and launch intent remain under the project's local
`.wayward-wrangler/state/session/coordinator/<task>/call-v1/`. The task event
records `Bootstrap tab contains foreground work; inspect before dispatch`.
There is no claim, dispatch, child, permit, result, or receipt in that
operation. These records have not been edited.

The installed source wrote `launch.json` before calling `Herdr::create`.
`Herdr::create` checked `pane.process_info` and returned this exact error
**before** `layout.apply`. Thus this application call did not send the worker
launch request. Absence of a process or receipt alone would not establish
that. A later operator screenshot showed an idle Fish prompt in the bootstrap
tab; it does not prove what process Herdr reported at the earlier instant.
The private Fish fixture did not reproduce that timing window. A startup
transient or unexpected foreground process remains the likely, unverified
trigger.

The old build classified every `create` error as possibly sent, marked the
coordinator call uncertain, and blocked the task. The inspector then said no
coordinator call was recorded because its projection omitted the saved call.
No proposal or workflow run was created. The old task is **not** automatically
retried or reclassified by this source change. Its service and release binding
remain untouched. Recovery of that saved call requires a supported,
append-only transition under a compatible installed release; do not delete
the launch intent, manually edit SQLite, or submit the same call again.

## Narrow correction

- Herdr creation now distinguishes a pre-send refusal from a call whose
  `layout.apply` outcome may be unknown. It waits up to two seconds for a new
  bootstrap shell to become idle. It refuses when the last process check
  reports foreground work. Herdr offers no atomic check-and-replace in this
  path, so a command arriving between that check and `layout.apply` remains
  a race to address when replacing Herdr-owned execution.
- A pre-send refusal writes `pre-send-refusal.json` bound to the immutable
  launch hash. Reconciliation reports it as blocked and never redispatches
  that operation. Errors after attempting `layout.apply` retain the uncertain
  no-retry behavior. The stage and coordinator paths preserve those distinct
  states.
- Task inspection now includes the saved coordinator call status, packet
  hash, and last failure. The conversation gives a concise stopped state and
  points to `/inspect`; it does not label blocked work complete.

## Verification and retained candidate

Focused Rust tests passed for pre-send refusal replay, possible-send
uncertainty, delayed idle bootstrap responses, task inspection, and TUI
failure wording. The corrected Rust-only bundle is under ignored
`.local/rust-migration/bootstrap-refusal-fix/bundle-v1/`; its manifest SHA-256
is `1cec0cfc36e1444294e47f765029fd2389cfd9651ace71c1785cb9cc7f9a16d4`.
The installed release is
`.local/rust-migration/bootstrap-refusal-fix/install-v1/releases/rust-bootstrap-fix-v1/bin/waywr`.

The installed private Fish fixture at ignored
`.local/rust-migration/bootstrap-refusal-fix/private-fish-installed-v1/`
submitted a task through project IPC, observed a completed fake coordinator
call and a proposed task, verified one worker workspace/tab, and exercised
direct terminal detach/resume. Its `summary.json` and `cleanup.json` report
pass and both test-owned service and private Herdr server stopped. This is
simulated verification, not reproduction of the operator's transient, native
agent acceptance, or permission to change the bound operator service.
