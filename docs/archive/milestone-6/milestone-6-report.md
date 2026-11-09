# Milestone 6: simulated interactive shell

## Unified frontend follow-up

The accepted conversation-first renderer and editor now drive the installed
`waywr` binary. The previous three-pane Rust module was removed; the scripted
scenario remains available only through the explicit evaluator binary. See the
[unified UI integration report](unified-ui-integration-report.md) for the final
paired installation, private Herdr checks, current commands and remaining
backend gaps. The sections below retain earlier milestone evidence and should
not be read as the current installed screen or keybindings.

## Session lifecycle follow-up

The approved exit/detach/resume addition is implemented and simulated-tested
after the original report. See [session lifecycle report](../session-lifecycle/session-lifecycle-report.md)
for its installed evidence, limits and current Fish walkthrough. The operator
accepted the separate scripted UI visual/input refinement, including full
available conversation width, as a hands-on observation. Automated checks and
the operator observation are distinct; neither authorizes native calls.

**Scope:** implemented on `milestone-6-interactive-shell` after approval of the
Rust/Ratatui, Python-backend and project-independence plan at `16bb5ce`.
Simulated coordinator and workers only. No authenticated model call, browser
application, release publication or shared Herdr configuration change occurred.

## Delivered

- `waywr` is a Rust terminal client with multiline input, paste limits,
  conversation scroll, task navigation, read-only Vim review, task-scoped
  answer/approval/steering/cancellation commands and terminal restoration.
  Its command/event IPC is a versioned, bounded, same-UID Unix socket protocol;
  command IDs, task versions, state revisions and an outbox protect replay.
- A per-project Python systemd user service owns task admission, fake
  coordinator calls, one workflow driver, supervised Herdr topology, recovery,
  event history and durable state. Rust owns no workflow database or worker
  execution. Model-facing packets use current-project selections by default.
  Explicit read-only cross-project grants select individual records with
  provenance and budget; revocation blocks future delivery.
- Multi-question workflow v2 can pause twice for independent operator answers;
  each answer survives service exit and costs no model call. Final approval
  remains bound to the exact candidate SHA-256. Historical profiles/runs keep
  their old interpretation.
- A paired Linux bundle installs a Rust binary and private Python wheel
  environment side by side from an offline, hash-pinned wheelhouse. The default
  alias changes only for new projects. Existing projects route by their release
  binding, and maintenance checks only the selected project. The review plugin
  code can route a project-qualified link through that binding when explicitly
  installed from the paired release; the user's existing registration was not
  changed. Python 3.12, venv/ensurepip, compatible Linux/glibc, Herdr and Vim
  remain external requirements. The Rust build toolchain lived under `/tmp`;
  no native system-wide Rust install was needed.

## Verification and evidence

| Check | Observed result |
|---|---|
| Rust/IPC proof | Five Rust tests pass. Earlier isolated proof covered frame/version refusal, outbox replay, PTY paste/resize/restoration, 10,000 backpressured events in one minute and acknowledgement during a long fake operation. |
| Python focused tests | Release lifecycle, review routing, context access, session store and attach recovery tests pass (20 focused tests after closeout changes). Saved offline findings/continuity fixtures also passed during step 5. |
| Actual Herdr startup | Isolated server cold/warm ordinary-terminal and inside-Herdr reconnect passed. The owned coordinator pane, visible fake workers, UI claim and server identities matched. UI replacement and service SIGKILL/restart were verified without a second worker dispatch. |
| Fresh paired installation | `/tmp/waywr-m6-accept-install/releases/m6-accept-r3` installed from a 42-wheel offline bundle and reports frontend 0.4.0/backend 0.3.0 from private `site-packages`; test processes ran from `/tmp` with Python `-I`, no source import. Required packaged resources and all three aliases were inspected. |
| One installed project | `/tmp/waywr-m6-accept-project-final/.wayward-wrangler/state` records a completed fake run after service generation advanced to 2, one human-choice fixture answer and SHA-bound fixture approval. Five worker-stage counters each equal 1. This is simulated test consent, not operator approval. |
| Two installed projects | `/tmp/waywr-m6-two-3n0jyy10` records A and B on the same isolated Herdr server with the **same run ID** and distinct project keys, sockets and state. B reached fixture approval while A waited at a question; cancelling A left B's task and approval unchanged. Both test services/server were stopped afterward. |
| Mixed installed releases | `/tmp/waywr-m6-two-jhdbw_5x` records A bound to R1 and B to R3 executing the same run ID concurrently on one isolated server. A remained at a question while B reached fixture approval. An initial harness request used R3 for A and was correctly refused by the IPC handshake; the corrected harness passed. |
| Context continuity | An installed-pair Decision Log follow-up from copied source evidence selected exactly eight records, including the approved editing decision and five open findings. Two distinct questions, service restart, five single-count fake stages and fixture approval passed at `/tmp/waywr-m6-project-n`. The original approved source run was not modified. |
| Release and location | R1/R2 side-by-side binding, selected-project upgrade/rollback, unavailable unrelated project and busy selected-project refusal passed. The completed fake run was copied to `/tmp/waywr-m6-relocated-final`, explicitly rebound, inspected at the same approved hash, then inspected again from a fresh `/tmp/waywr-m6-reinstall` prefix. Original evidence was retained. |

The final packaged code is the R3 bundle (manifest SHA-256
`e03e2a847b9b49546cc6a35c3b8aaa873e27265022cab2e452fe33efe3ba53ef`).
The isolated final one-project and two-project checks used that build. Earlier
proof scripts and state remain under `/tmp/waywr-m6-*`; they are disposable test
evidence, not a published installation. No `waywr-*` test unit was running at
closeout. Shared Herdr sessions, historical approved runs, and their tabs were
left untouched.

## Limits and next gate

The final installed build has **simulated** end-to-end evidence. Ghostty/Herdr
focus, live worker readability, long-history scrolling, Vim navigation and
terminal restoration still need the operator's hands-on observation. The
missing-workspace path has focused service tests and earlier isolated recovery
evidence, but was not repeated by forcibly closing a live workspace in the
final paired-install run. Protocol schema, project-release selection and review
link routing have automated coverage; switching the shared plugin was outside
scope. At the time of this original report, project-service stop was explicit
and quiescence-gated; the later session-lifecycle implementation supersedes
that behavior with a project-scoped graceful exit while an already-permitted
worker may finish. Detach leaves work running. Installation has no automatic
release deletion or GC.

Native isolation is **not established**. Installed Claude 2.1.296 and Codex
0.162.1 interfaces and adapter flags were inspected offline. The existing
Claude adapter requests no tools/hooks/MCP; Codex uses a read-only sandbox and
feature disables, but that is not a filesystem read boundary. Provider-native
auto-context and general read/search/tool access must be constrained and
runtime-verified before native acceptance. No provider switch or real
coordinator call was tested. The separately authorized native trial remains
blocked; recommended ceiling is two coordinator calls and five worker stages
with two critiques, subject to explicit settings and authorization.

## Fish hands-on check

Use a disposable project and the retained local test installation. This will
create a new project-owned Herdr workspace on the server selected by your
environment; it will run **fake** workers only.

```fish
set demo (mktemp -d /tmp/waywr-hands-on.XXXXXX)
cd $demo
/tmp/waywr-m6-accept-install/bin/waywr
```

Accept project initialization. In the coordinator pane, type a small drafting
request and press Ctrl-S. Select its task with Esc and j/k, then i. Enter
`:plan`, press Ctrl-S, inspect the displayed provider/budget, then `:confirm`
and Ctrl-S. Observe the separate Herdr worker panes. At a question, use
`:inspect`, then `:answer 7 days` (or another displayed option) and Ctrl-S.
At final review use `:open`, inspect the exact candidate in Vim, and return to
the coordinator. `:approve` is an operator action; do it only if you approve
that candidate. Esc then q detaches. Running `waywr` again from the same
directory reconnects; `waywr session status` inspects the selected project,
and `waywr session stop` stops its service only when quiescent. Retain the
project if you want to inspect its artifacts and receipts later.
