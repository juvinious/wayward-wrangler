> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Wayward Wrangler

Persistent workflows for coding agents.

The current Rust migration's direct-terminal startup correction is documented
in [docs/rust-migration-startup-correction.md](docs/rust-migration-startup-correction.md).
For its separately installed simulated candidate, `waywr init .` or
`waywr init SOME_DIR` initializes without launching a UI or Herdr tab;
`waywr` starts the TUI in the invoking terminal and `waywr resume` reconciles
the saved session there. The older Milestone 6 installation described below
remains a historical compatibility release.

The corrected Rust `init` keeps default project configuration and state under
`<project>/.wayward-wrangler/` and makes no user-local registry write. On first
service attachment, a separate `${XDG_STATE_HOME:-$HOME/.local/state}/wayward-wrangler/locations.json`
locator may be created for Herdr review links. That locator contains paths, not
project conversation or execution records. See
[project storage paths](docs/stale-registration-reinit.md#storage-paths-and-intent)
for the exact paths and explicit `--state-dir` exception.

This utility automates drafting, independent critique, revision and human
approval in Herdr. Milestone 6 pairs the accepted conversation-first Rust/Ratatui
interface with the existing Python project service. The current installable
bundle uses a deterministic fake coordinator and fake workers; authenticated
coordinator/worker acceptance has not been authorized. Historical runs remain
unchanged.

## Milestone 6 unified simulated interface

The primary command is `waywr`; `wayward-wrangler` and `awp` remain aliases.
The local Linux release bundle contains a Rust executable, a private Python
environment installed from hash-pinned wheels, and packaged runtime resources.
Its external prerequisites are matching Linux architecture/glibc, Python 3.12
with `venv`/`ensurepip`, Herdr, Vim, and a terminal with paste/resize support.
Rust is needed to build a release, not to use the compiled bundle. Development
uses the user-local rustup installation and the version pinned in
`rust-toolchain.toml` (currently 1.99.0). From the repository root, run
`~/.cargo/bin/cargo test -p waywr-backend` for a focused backend check. The
earlier milestone build used an isolated toolchain under `/tmp`; it remains
retained as historical build provenance.

From a project directory, `waywr` offers to initialize that project, creates
or reconnects its own Herdr workspace and coordinator pane, and attaches to a
project-scoped service. `--project` remains available from another directory.
The shaded composer defaults to **New task request**. Enter submits;
Shift+Enter inserts a newline, with F2 multiline mode as a fallback. Bracketed
paste is literal. Type `/` to choose discoverable actions. `/plan` opens the
selected task's scope and the visible **Authorize** action admits it. `/answer`,
`/approve`, `/open`, `/inspect`, `/tasks`, `/settings`, `/cancel`, `/detach` and
`/exit` use the same presentation. Answer and approval remain explicit user
actions. The installed UI uses the accepted conversation layout, colors,
message grouping and task drawer; the former three-pane frontend is removed.

The current service turns a new task request into a deterministic proposal. It
does **not** support open-ended coordinator conversation or active steering yet;
the UI says so and preserves a draft addressed to that unavailable target.
The separate scripted Decision Log evaluator remains an explicit demo binary
and cannot submit to the project service. See the [unified UI report](docs/unified-ui-integration-report.md)
for installed verification and limits.

The installed simulated interface accepts `/detach` (or Ctrl-Q) to
close only the interface. `/exit` opens a confirmation panel; selecting
**Exit session** pauses new work, lets an already-permitted call finish under
supervision, writes a project checkpoint and returns to the shell. Run
`waywr resume` from that project to reconcile and continue; `waywr resume
RUN_ID` keeps its historical run meaning. `waywr session status` shows the
durable phase even when the selected project service has stopped. See the
[unified UI report](docs/unified-ui-integration-report.md) for the current Fish
launch command and simulated-only limits. The earlier [lifecycle report](docs/session-lifecycle-report.md)
records the older frontend's historical acceptance evidence.

Build and install a paired bundle using `packaging/build_bundle.py` and the
bundle's `install.py`; `requirements.lock.txt` supplies the pinned dependency
set. Installs are side by side under `PREFIX/releases/ID`; `PREFIX/bin/waywr`
selects the default for new projects. Existing projects bind a compatible
release in `.wayward-wrangler/release.json`. `waywr --project DIR session
status|stop|bind|upgrade|rollback` applies to that project only. Explicit
`--state-dir` and `--registry-dir` remain available. Releases still referenced
by projects must be retained. The [state contract](docs/state-contract.md)
defines the v1 IPC and durable records.

The sections below describe prior wheel/pipx and noninteractive workflows.
Those commands remain for compatibility; they do not install the paired Rust
shell.

## Identity and local installation

Package/repository: **wayward-wrangler** (Python backend version 0.3.0).
The paired frontend is version 0.4.0; its primary command is **waywr**.
`awp` remains a compatibility alias for existing scripts and recorded commands.
The internal Python namespace `agent_workflow`, Herdr plugin ID and `awp.invalid`
review links remain stable. Historical reports retain names/paths used at execution.

## Install and select a project

Build a local wheel from a clone, then install it with pipx. Python 3.11+, pipx,
Herdr 0.9.3+ and Vim are external prerequisites. Claude/Codex CLIs and their
existing authentication are required only for separately authorized native runs.
The tested installation used Python 3.12.3 and pipx 1.17.13. New native runs
require an explicit target-owned native profile, no-model preflight, and
run-specific operator authorization; see the [milestone-5 report](docs/milestone-5-report.md).

```fish
cd /path/to/wayward-wrangler
python3 -m pip wheel --no-deps . -w dist
pipx install --pip-args '--constraint /path/to/wayward-wrangler/requirements.lock.txt' dist/wayward_wrangler-0.2.0-py3-none-any.whl
wayward-wrangler --help
```

Use a target directory of your choosing; it need not contain Wayward Wrangler's
source or a Python environment. `init` writes a versionable project key and fake
workflow configuration under `.wayward-wrangler/`, and adds only
`.wayward-wrangler/state/` to the target's `.gitignore`. Briefs, rules and context
selections remain target-owned files. All run commands, including `awp`, require
`--project`. Relative paths are resolved from that project, even from another cwd.

```fish
set project /path/to/target-project
wayward-wrangler --project $project init
printf '# Draft brief\n\nDescribe a small change.\n' > $project/brief.md
wayward-wrangler --project $project open-demo --run review-01 --brief brief.md --workspace WORKSPACE_ID
wayward-wrangler --project $project status review-01
awp --project $project show review-01 --artifact latest
```

From inside Herdr, the controller can inherit its workspace. Each fake worker
runs visibly in its own labeled pane. The controller presents substantive choices
and final approval. `status --json` reports the selected project, state path and
project key. Default state is `PROJECT/.wayward-wrangler/state/`; `--state-dir`
selects another location, bound to the same project key. One active run is admitted
across locations in the same user-local index; concurrent scheduling is not offered.

Install the copied review plugin explicitly when ready to replace a prior link.
The wheel install itself never changes Herdr configuration. An existing plugin
registered elsewhere must first be unlinked explicitly. The milestone-5 trial
switched the shared registration to its installed package; the old checkout
registration was captured for rollback.

```fish
wayward-wrangler integration herdr install
```

The local index is `${XDG_STATE_HOME:-~/.local/state}/wayward-wrangler/`.
`--registry-dir /absolute/path` makes it explicit for a separate environment.
Review links encode project key, run ID, artifact ID and SHA-256; missing, stale
or legacy run-only links refuse automatic routing. The plugin copies its assets
under `${XDG_DATA_HOME:-~/.local/share}/wayward-wrangler/herdr/` and binds the
installed interpreter, independently of this checkout.

For a quiescent copied project with the same project key and state marker, use
`wayward-wrangler --project /new/path location-rebind` after the original workers
and controller have stopped. It appends location metadata outside run evidence.
It refuses active/unresolved operations or a live worker. A moved project must
also retain its selected context documents; changed bytes remain stale.

Existing approved runs remain in this checkout's `.local` and were not migrated.
Use explicit legacy selection to inspect them with the installed CLI:

```fish
set legacy /path/to/old-checkout
wayward-wrangler --project $legacy --state-dir .local status context-01
wayward-wrangler --project $legacy --state-dir .local show context-01 --artifact revision-2
```

For a **copy** of legacy state at a new root, add `--legacy-origin /original/root`
for exact frozen-namespace verification. The old root need not exist, but the
copied state and pinned documents must match. The mapping never rewrites recorded
paths, hashes, decisions or approval. Historical run-only review links need
explicit `--project`; they cannot be auto-routed among projects.

See [milestone-4 report](docs/milestone-4-report.md) for acceptance and limits.
The checked-in real-trial profiles document completed, specifically authorized
runs; they are not permission to start new authenticated workers.

## Completed: milestone 3 recovery tests

Milestone 3 adds conservative recovery diagnostics and fault
coverage. 65 offline tests pass; isolated Herdr tests cover controller crash windows,
checkpoint replay, surviving children, cancellation races and server replacement.
All workers were simulated; existing approved runs remain unchanged. Test servers
are stopped and evidence remains local. See the [milestone-3 report](docs/milestone-3-report.md).
Merged into `main` in PR #1. Milestone 4 is merged in PR #2;
see its [accepted proposal](docs/milestone-4-portability-proposal.md).

## Completed: context continuity trial

**context-01 is operator-approved.** Your **block, never overwrite** answer
survived controller exit and was consumed by a fresh controller. Both that answer
and inherited **Offline only** reached subsequent workers. All five stage counts
remain 1, with two critiques and no repeated draft. The original draft failure
and explicitly authorized recovery remain recorded.

```fish
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local status context-01
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local show context-01 --artifact revision-2
```

No pending gate remains. See [final outcomes and recovery history](docs/context-continuity-report.md).
48 offline tests passed at context completion; 50 pass after the naming checkpoint.
Explicit context selection is automatically resolved and
delivered; it is **not project-wide relevance discovery**. One new substantive
question per run is a prototype constraint, not a permanent workflow requirement.
The later milestone-3 recovery tests are summarized above.

## Completed: milestone 2

The authorized real trial **m2-cli-01** is **approved**. All five stages ran once;
your **Offline only** choice and approval of **revision-2** are saved. No human
gate remains. See [milestone-2 outcomes](docs/milestone-2-report.md).

```fish
cd /path/to/wayward-wrangler
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local status m2-cli-01
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local show m2-cli-01 --artifact revision-2
```

Historical worker panes are closed; transcripts and artifacts remain on disk.
Final revision 2 was approved by you after the two permitted critique rounds;
it did not receive a third independent critique. The later milestone-3 recovery tests are summarized above.

Claude: opus/high (resolved claude-opus-5-5). Codex: gpt-6.1-sol/high.
Existing authentication is used with per-invocation text-only restrictions.
Historical runs are complete and cannot authorize a new trial by ID reuse.

The remaining sections document the preserved simulated demo and general commands.

## Current outcome and review

Your `demo` is **approved**. The saved choice is **30 days**; final approval is
bound to revision-2. Each of the five simulated stages ran exactly once.

See [historical outcomes and verification](docs/terminal-review-outcomes.md) for
completed checks and the original linked review plugin.

From a Herdr shell (any workspace in the same session):

```fish
cd /path/to/wayward-wrangler
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local status demo
```

**Ctrl-click Open in Vim** to review the artifact in a separate tab in
`example-workspace`. Vim is read-only; `:q` closes the review tab. Opening the
same artifact again reuses its existing review tab. Keyboard alternative:

```fish
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local open demo --artifact latest
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local open demo --artifact critique-2
```

The original checkout-linked `local.agent-workflow-review` plugin is registered and enabled
in the user's shared Herdr configuration; milestone 4 did not switch it.
You have confirmed the Open in Vim link works in Ghostty/Herdr. No hosted service
is involved. Colors: cyan author/headings, magenta critic, green success,
yellow input/warnings, red errors, blue actions. TTY detection, NO_COLOR and
`wayward-wrangler --color never` support plain output; `--color always` forces color.

`status` is read-only. At a newly started interactive controller, type `open`
to review the current draft or `open critique-2` for that critique. Return to
the controller for decisions. Final review accepts “approve this revision” and records its exact hash
internally; opening a file does not approve it.

Already-running controller/worker processes keep their old presentation code.
A new `status` invocation loads the update immediately. No new run is needed.
The historical worker panes can be closed without losing artifacts/logs.

### Decision commands for a future pending run

The commands below illustrate the earlier demo interaction; this demo has
already been answered and approved. Do not submit a different answer to it.


```fish
cd /path/to/wayward-wrangler
printf '%s\n' '7 days' > answer.md
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local decide demo --request D-retention --answer-file answer.md
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local show demo --artifact latest
```

After reviewing the artifact, replace the placeholder with its actual hash:

```fish
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local approve demo --request A1 --sha256 REVIEWED_SHA256
```

Vim can compose answer/rejection files. Choices currently accept the displayed exact options. `--record-only` persists a response without graph continuation; `resume` consumes it later.

An interactive controller waiting for terminal input does not poll for answers entered elsewhere. Press Enter there to refresh, or continue through commands. Conflicting answers are refused.

## Exit, resume and cancellation

Ctrl-C in the **controller** exits that invocation and leaves workers running under Herdr. A child may finish while the controller is absent; subsequent stages wait for resume.

```fish
cd /path/to/wayward-wrangler
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local resume demo
```

Run this in an existing Herdr shell to use that pane as the controller. To restore the interactive prompt there:

```fish
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local demo --run demo --workspace w8
```

Resume reuses the graph thread, existing worker panes and validated receipts. Ambiguous execution stops as `uncertain`; there is no force-retry shortcut.

`open-demo` is only for new runs. Initialize a separate target project and supply
its brief before starting a fresh simulated run:

```fish
wayward-wrangler --project /path/to/target-project open-demo --run demo-next --brief brief.md --workspace WORKSPACE_ID
```

The workspace ID describes this live session. Commands inside Herdr can inherit its workspace; outside it, pass `--workspace`. No new server/session is started automatically.

Cancel a currently running stage:

```fish
wayward-wrangler --project /path/to/wayward-wrangler --state-dir .local cancel demo --operation revision-1
```

Only its verified child process group is stopped. The pane/runner stays for inspection. Cancelling an already completed historical worker has no effect. Ctrl-C in a **worker pane** requests cancellation; ordinary terminal text is not forwarded to the child and never becomes a decision.

Closing a live pane or killing its runner may destroy evidence and cause uncertainty. Close completed Wayward Wrangler tabs manually when finished; that ends their sleeping retention processes. Artifacts/transcripts remain on disk. Do not stop the shared server or close unrelated tabs.

## Configuration and data

- `PROJECT/.wayward-wrangler/project.toml`: stable project key; `workflow.toml`: role mappings and fake duration. Other project files hold versionable rules/context.
- Packaged `skills/`: draft, independent critique and revision templates frozen per run. The source checkout's `skills/` paths link to the same canonical assets.
- `PROJECT/.wayward-wrangler/state/state.sqlite`: LangGraph checkpoints and application tables.
- `PROJECT/.wayward-wrangler/state/runs/RUN/inputs/envelope.json`: frozen inputs/configuration/target.
- Each operation packet carries focused instructions, candidate, decisions, findings and review delta.
- Each operation directory also contains launch/claim/binding/permit records, child identity, invocation counter, transcript, structured result and completion receipt.
- `PROJECT/.wayward-wrangler/state/runs/RUN/artifacts/`: immutable versioned Markdown/JSON, decisions and approval.
- This checkout's `.local/` remains legacy evidence; milestone 4 acceptance evidence is in disposable isolated locations named in its report.

Two persisted critique slots enforce the limit. One active run is allowed. Routine stages advance automatically; substantive choices and final approval pause.

This is not a hostile-process sandbox. Real CLI workers use the restrictions documented in the milestone-2 report; no credential files are copied into run state.

## Environment and checks

Installed-wheel acceptance used Python 3.12.3, LangGraph 1.2.14,
checkpoint **4.2.0** and SQLite saver 3.1.1. Resolved transitive versions
are in `requirements.lock.txt`. Hosted SDK and sqlite-vec arrive transitively
but are not used for hosted execution or a vector database. Controller tracing
is disabled.

No activation or Fish/global configuration is needed:

```fish
cd /path/to/wayward-wrangler
.venv/bin/python -m unittest discover -s tests -p 'test_*.py' -v
.venv/bin/pip check
```

These checks launch no workers. `tests/milestone4.py` is opt-in and uses a
separate Herdr server and disposable projects; its command and evidence paths
are in the milestone-4 report. The older live scripts use historical fixture
IDs; do not rerun them blindly. Runtime records remain local-only.

Exit codes: 0 successful command/terminal outcome; 2 awaiting input; 3 refused/stopped/uncertain; 130 interrupted controller. Use status to distinguish approval, rejection, cancellation and failure.
