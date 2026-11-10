# Wayward Wrangler

Waywr is a project-scoped development orchestrator. Initialize a project, discuss
and approve scope and operating policy, delegate to tool-capable workers, review exact
results, run checks and maintain project knowledge through its Rust terminal UI.
The intended workflow supports existing projects and substantial user requests.

## Current readiness

The current increment replaces hardcoded native clients with one shared provider/model
runtime beneath the Rust UI/service. Its bundled catalog exposes42 providers and1592
models, searchable in F3; custom model IDs, endpoints and SDK options are data.
The independent `pi-ai`/`pi-agent-core` libraries supply APIs and the tool loop;
Pi's terminal application is not used. See [provider setup](docs/provider-selection.md).

Waywr remains experimental. Local HTTP fixtures exercise two provider protocols
through the actual runner/tool loop, including worker edits, structured results and
usage. Installed TUI/reopening checks use isolated projects. These establish those
boundaries, not account login, all-model reliability or unassisted useful delivery.
Existing native profiles require reconfiguration; project files/history are retained.
Stopped compatible projects reopen without their old build. Public download,
update/uninstall and active-work update fluidity remain later increments. Follow the
[product plan](docs/usability-plan.md), [development approach](docs/development-approach.md)
and [handoff](docs/HANDOFF.md). No binary release has been published by this work.

The [public-source audit and remediation](docs/security-audit-2026-11-08.md) cover
credential scans, patched dependencies, generalized public documentation, build-path
privacy and security checks. The application is [MIT licensed](LICENSE). The original
Git/GitHub history is retained in a separate private, read-only historical archive.

## Ordinary use

With a compatible paired Rust release installed and `waywr` on your `PATH`:

```fish
cd PROJECT
waywr init .
waywr
```

`waywr init SOME_DIR` initializes that directory and returns. Init is project-local,
launches no service/interface/Herdr resource, and preserves configuration, state and
release binding on repetition. `waywr` opens the accepted TUI in the invoking
terminal and starts/reconnects its service automatically. `waywr resume` reconciles
and restores the saved session. Plain `waywr` reopens saved context while preserving
a paused dispatch state; `waywr resume` explicitly resumes execution. Repeated
`/exit` also closes an already paused view. Explicit selection uses `waywr --project DIR`;
both `--project DIR` and `--project=DIR` are routed.

Linux with a systemd user manager, Node >=22.19, Git and util-linux `flock` is the
current execution platform. Configure the selected provider's SDK authentication;
no Codex/Claude executable is required. The model runtime is trusted same-UID code,
not an OS sandbox. Herdr remains optional explicit read-only inspection. Both closing commands preserve your work:

- `/detach` closes the TUI while authorized work continues in the background.
- `/exit` checkpoints and pauses the session, then closes the TUI. Active permitted
  work may finish while it drains; no new work is dispatched. Reopen with `waywr`
  to inspect, or use `waywr resume` (or `/resume`) when ready to continue.

## Development workflow

F3 → 7 Project shows baseline, named target, completion policy, human-validation
choice, retention and command recipes before scope approval. New setup uses committed
HEAD and project-local Git exclusions; unrelated dirty edits remain outside that
baseline. Non-Git/unborn source needs explicit initialization selection and approval.
See the [ordinary-project contract](docs/ordinary-project-workflow.md) for current
implementation boundaries.

New projects select a default from runtime configuration/catalog data, not a model
constant in Rust. Configure authentication explicitly, then choose a provider/model
in F3 before requesting work:

```fish
waywr models list --search MODEL_OR_PROVIDER
waywr models login PROVIDER --method oauth # or --method apiKey when supported
```

F3 →1 Profiles supports F search, V provider, C model, manual IDs and options JSON.
S saves; R refreshes local catalog/configuration metadata. The project profile is
`.waywr/coordinator.json` (preferences schema3). `/plan` reviews saved scope/policy.
A saved proposal and operating-policy card appears in chat. Reply `approve` or choose
its inline Approve action to authorize that exact presented revision, with no second
confirmation. `/plan` keeps the detailed review/editor available. Discussion grants
no execution authority. F6 inspects tasks without changing the conversation owner;
`/discuss` explicitly selects the inspected task's request.

The concise proposal card shows effective providers separately from broad allowed
sets. An explicit configured reviewer choice for this request is fixed in saved
routing; another allowed provider cannot silently replace it. Details expands the
complete scope and policy without changing approval.

The selected live coordinator develops the plan and decomposition from your
conversation and bounded project context; Waywr schedules its validated decisions.
You can also ask for recaps, explanations or ordinary conversation without creating
a task or approving work. The live post-delivery recap has been human-verified;
permission boundaries apply to application actions, not discussion topics.
Normal chat accepts clear answers, revision requests and in-scope steering. A saved
answer continues coordination automatically; multiple targets or unclear approval
receive a targeted inline choice. Quoted/imported text never supplies permission;
deliberately submitted plain paste and command recall remain usable. Drafts retain
the proposal/question they began replying to across refresh and restart.

`/new` starts another topic; clear new work following delivery creates a linked
follow-up discussion. `/reply` and `/steer` remain optional explicit controls. In a
paused session, `/resume` resumes the session; Send preserves its draft until then.
In a running workflow, recovery reconciles validated receipts and exact saved Git
effects automatically. A preparation refusal can continue after its recorded cause
changes; equivalent work gets a useful reassessment, then waits for actual new facts.
Activity and Inspect show the cause, next action, wake condition and whether a decision
is needed. Chat and independent work remain available. A linked judgment from the
selected reviewer can resolve a provisional finding without repeating the whole review.
`/resume` remains an explicit operational control using the saved revision; it cannot
clear uncertainty or expand authority. Unknown execution and source edits stay preserved.
Python caches can be retained with a manifest during supported recovery. See the
[recovery contract](docs/coordinator-recovery.md) for limits. No request is seeded.
See the [conversation contract](docs/conversational-coordination.md) for exact routing,
approval, continuation and repair semantics and the [handoff](docs/HANDOFF.md) for
installation and ordinary-use verification limits.

There is no mandatory call, repair or task-time ceiling. Capacity refusals wait by
default. Explicitly configured and approved alternatives permit settled-capacity
fallback. All configured providers use the shared execution path. F3 Profiles → A
reviews activation of a saved model after the current turn. Section4 Switch retains
status, cancellation and explicit interruption controls. Preference edits
apply to future attempts; reserved profiles and useful work remain intact.

See [provider/role controls](docs/provider-selection.md) for complete F3 controls,
capabilities, configuration and policy amendments. The earlier human-led Codex/Claude
delivery is historical evidence for the retired integrations, not verification of
this runtime. Each new request and approval belongs to you.

## TUI controls and activity

| Control | Behavior |
| --- | --- |
| Up / Down in the composer | Move within multiline input; at the first/last line, recall history. Down past newest restores the draft and cursor |
| Enter / Alt+Enter | Send idle input or steer the active coordinator / queue a separate follow-up |
| Shift+Enter / Ctrl-J | Insert a newline where the terminal reports it; F2 is the multiline fallback |
| Alt+Up / Alt+Down | Move the multiline input cursor |
| PageUp / PageDown | Scroll chat and live activity eight wrapped lines |
| Alt+PageUp / Alt+PageDown | Scroll chat or the open content panel one wrapped line |
| Home / End in a content popup | Start/end of content; in the model picker, first/last result |
| Ctrl-R | Open command history; Enter recalls into the composer without sending |
| Left / Right in modal action rows | Select previous/next option; Enter activates it. Tab / Shift-Tab also selects |
| F3 | Provider/model and project settings; arrows/Tab/Shift-Tab move focus, Enter chooses/edits, Esc backs out one level |
| F6 | Inspect scopes and concurrent child tasks |
| Mouse wheel | Scroll the conversation or popup under the pointer, three lines per notch; provider/model pickers scroll without applying |
| F7, then E | Restore a pending steering input or queued follow-up for editing; applied inputs cannot be withdrawn |
| `/model`, `/thinking`, `/status` | Open the model picker, thinking picker, or current session/model/queue/usage summary |
| `/discuss` | Explicitly switch chat to the inspected task’s request |
| `/login`, or F3 → G | Sign in to a provider using its SDK methods; OAuth opens the browser, secret entry stays private |
| `/plan` | Review the current topic's saved scope and operating policy |
| `/output` | Read current/retained output; Left/Right switches attempts, arrows/page keys scroll |
| `/inspect` | Inspect integration, checks, obligations and exact readable history/summary paths |
| `/resume` | Resume a paused session or recover the selected blocked workflow after its blocker is resolved |
| `/detach` | Close the TUI while authorized work continues |
| Ctrl-Q, empty-input Ctrl-D, `/exit`, `/quit` | Save the draft and leave immediately; service-owned work continues |
| `/interrupt`, empty-input Escape / Ctrl-C | Interrupt the foreground coordinator directly; keep typing or leave while it stops |
| `/pause` | Request a stop to new dispatch; current calls settle separately and the UI remains available |

Ctrl-C clears nonempty main input. Escape clears it when no popup/menu is open;
inside a popup, Escape backs out without clearing the underlying draft.
With empty input these keys interrupt only the foreground coordinator; idle
Ctrl-C does not exit. Interruption never implicitly cancels a whole workflow.
History recall never sends or executes an entry. Modal text fields retain cursor
movement while editing; Tab moves focus to their actions. Queue Up/Down selects a
message, Left/Right selects its action, and page keys scroll. Leaving requires no
confirmation or settlement wait, including from editors or a disconnected UI.
Reopening `waywr` restores context and preserves an intentional pause.

Steering is marked pending until it reaches the SDK's next agent boundary, then
applied to context. Follow-ups wait for the current reply and have separate
results. Interrupting keeps unconsumed inputs held; reopening does not replay
them. Restoring queued text never sends it. See [live input](docs/live-input.md).

F3 → F opens the complete searchable model catalog. Type to filter, Up/Down to
select and Enter to place a model in the draft; S saves after returning to the
form. A reviews activation of that saved profile; choose Activate and share project
context and Enter to request the after-turn switch. The default Keep current or Esc
returns without switching. Activation preserves an intentional pause and existing
work policies. E permits exact IDs absent from the catalog. Fixed footer hints follow the
[TUI keyboard contract](docs/tui-controls.md).
The Thinking field lists the selected model's supported levels and saves the
choice with its profile. F3 shows draft and active model/level separately; the
main view shows the saved coordinator selection first and explicitly labels any
different active binding and its readiness state.

If a retired binding blocks sending, the main action reads Use saved model. Send
opens that model's activation review directly. Confirm activation, then Enter sends
your retained draft. An unrelated request's model policy no longer blocks a new
discussion; that request keeps its own permissions and may need an amendment.

The conversation shows its current or latest coordinator call's model, thinking
level and reported token/cache usage. Cost and context remaining are labelled
estimates when available; missing data stays unknown. Completed-call totals remain
visible after reopening. This reports conversation-call usage, not an account quota.

Development activity labels follow saved event status and review disposition, so
prose about failed-save tests cannot mark a passing review as failed. A completed
review is separate from combined checks and delivery. Once a new topic owns a scope,
the composer caption shows that scope without changing the input destination.

Activities show local `HH:MM` today or `MM/DD/YY HH:MM` for older entries. Active
coordinator/check/worker work has an elapsed-time line with a subtle shimmer; F3
reduced/off or monochrome mode keeps it static. Running workers appear beneath it:

```text
• Working · 2 workers (4m 14s) · /output inspect
  └ • Storage implementation
      Running: python3 -m unittest
    • CLI implementation
      Editing files
```

Worker titles fall back to IDs. Current command/tool labels refresh once per second;
before the first reported operation, the label reads **Preparing · waiting for a
reported operation**. The worker count excludes the coordinator, which has its own
indicator. Between reported commands, **Working · no current operation reported**
means no tool is currently reported active; it does not establish model inactivity.
Output stays in `/output`. Completed commands clear, and unavailable/stale activity
uses an explicit fallback. Blocked or paused work is not animated. Workers use
separate branches/worktrees; integration and exact combined checks are serialized.
New branches use `waywr/<short-request-title>-<12-character-id>/integration` for
combined work and `…/tasks/<task-id>/work` for workers. The title is bounded;
the suffix distinguishes requests with the same title. Retained legacy work keeps
its refs/worktrees and gains a checked named result alias during ordinary service
reconciliation. Old recovery branches are retained, not automatically deleted.

Before delivery, main can still contain the older application. `/inspect` shows the
combined branch, exact commit and **Combined checkout** directory. Use that directory
for preview or human verification; consult its README and keep preview data separate.
Passing automated checks does not supply a requested human visual result.
Dirty or ambiguous state is preserved for reconciliation.

Command history persists per project in `.waywr/state/session/command-history.json`
(recent 200 entries, up to 1 MiB). First use can seed recorded user messages;
unrecorded old slash commands cannot be recovered. Full conversation, assignments,
outcomes and closeout are in `.waywr/history/`, with short timestamp/hash filenames
and adjacent summaries. The [TUI contract](docs/contracts/tui-contract.md) owns detailed
input, freshness and presentation behavior.

## Prior delivery evidence

[Recorded taskings and outcomes](docs/archive/conversational-coordination/2026-10-17-verified-checkpoint.md)
distinguish human-led development from the original agent-prepared example.
[Historical application instructions](docs/archive/product-workflow/2026-10-18-readiness-and-example.md)
are retained as evidence, not a request to run another demonstration. New work belongs
to the user in their chosen project. [Project layout](docs/project-layout.md) owns
canonical state and supported migration.

## Versioned binary installation

No GitHub binary release has been published or verified yet. Use the
[one-command local source installation](#build-and-install-from-source) now.
After the user publishes a version release, the existing hosted installer can be
used from Fish:

```fish
set waywr_version v0.4.0 # choose an actually published version
set waywr_repo OWNER/REPO # select the repository hosting that release
set waywr_installer (mktemp)
gh auth login # once, with read access to the private repository
gh api -H 'Accept: application/vnd.github.raw+json' "repos/$waywr_repo/contents/scripts/install.sh?ref=$waywr_version" > "$waywr_installer"
and sh "$waywr_installer" --version "$waywr_version" --repo "$waywr_repo"
rm -f "$waywr_installer"
fish_add_path "$HOME/.local/bin"
waywr --project /absolute/path/to/project
```

Binary prerequisites: Linux x86_64/glibc2.39+, GitHub CLI, Git, GNU tar, gzip,
sha256sum, Node >=22.19 and normal systemd/provider execution prerequisites below. Rust is only
needed for source builds. The installer validates GitHub asset/checksum identity
and archive safety before the existing staged bundle install. It selects a new
user-local default while preserving project data. Compatible projects reopen
with that application; see [Existing projects](#existing-projects-upgrade-versus-migration).
[Distribution contract](docs/distribution.md) explains trust, platform limits,
packaging and publication steps still required.

## Build and install from source

Prerequisites:

- Linux, Git, a C compiler/linker and standard system headers. SQLite is built from
  the locked bundled dependency. Build on the target OS/architecture.
- Permanent rustup in `~/.cargo/bin` and the checked-in `rust-toolchain.toml`
  (Rust 1.99.0). Preserve `Cargo.lock`; the build may download locked dependencies.
  Do not override `CARGO_HOME` or `RUSTUP_HOME`.
- Node >=22.19 and npm for building the locked model runtime. Built bundles need
  Node and `flock`, but not npm, Rust or Python.
- For project execution: a systemd user manager/session bus, `systemctl`,
  `systemd-run`, Git and the selected SDK provider's credentials/endpoint. Installing
  Waywr does not sign in or change another agent application's provider settings.

From the Waywr checkout, build and select the paired application with one command:

```fish
sh scripts/dev-install.sh
```

This uses permanent rustup and the pinned toolchain, checks license notices and
source/binary privacy, creates a fresh artifact ID and atomically selects the
validated paired bundle. Python3 is needed by these source-build checks; it is not
an installed runtime dependency. A failed build leaves the working installation
selected. Project settings/history and older artifacts are preserved; shell
configuration, provider settings and services are not changed by installation.

If needed, add the launcher to Fish's PATH once, then use it normally:

```fish
fish_add_path "$HOME/.local/bin"
waywr --version
cd PROJECT
waywr
```

Plain `waywr` prompts for first-use setup when needed. After editing Waywr, close
its old TUI and repeat `sh scripts/dev-install.sh` from this checkout, then reopen
the same project with `waywr`. No bundle ID, per-project rebind or separate service
command is needed. Idle services are replaced; compatible busy work remains
attachable with its original owner. Obsolete or stranded sessions are renewed
automatically as described below. Use `resume` only when you
intend to resume a paused session. `--prefix ABSOLUTE` provides an isolated/custom
installation; its launcher is `PREFIX/bin/waywr`.

[Verification guide](docs/verify-usability.md) covers the first real task and
rebuild/reopen loop. `bundle-build` and `bundle-install` remain the underlying
advanced assembly/install commands; copying the frontend alone is insufficient.

`bundle-build` requires a new output directory. Release IDs allow at most 64
letters/digits/dots/hyphens/underscores (not a leading dot); use a fresh ID for changed contents.
Installation validates paired binaries, resources and manifest hashes and refuses
to replace an existing release ID with different contents. Copying only the
frontend is insufficient. Building/installing starts no service or development job
and does not publish a release.

| Installed path | Purpose |
| --- | --- |
| `~/.local/share/wayward-wrangler/releases/RELEASE_ID/` | Paired binaries, resources and manifest |
| `~/.local/share/wayward-wrangler/current` | Default selected by `--select-default` |
| `~/.local/share/wayward-wrangler/bin/waywr` | Stable launcher through `current`; legacy aliases are also created |
| `PROJECT/.waywr/release.json` | Retained initial/legacy installation metadata; ordinary opening does not pin a build |
| `PROJECT/.waywr/{state,worktrees,history}/` | Project-owned execution, task worktrees and readable history |

`fish_add_path` persists the installer's bin directory for future Fish sessions.
Check resolution with `type -a waywr`. This workspace's existing
`~/.local/bin/waywr` link may keep pointing into the same installation; the
installer itself creates aliases under `PREFIX/bin`. The absolute stable command
for this example is `~/.local/share/wayward-wrangler/bin/waywr`. No sudo is needed.

## Existing projects: upgrade versus migration

The selected application opens compatible stopped projects directly. Existing
`.waywr/release.json` metadata does not pin execution; startup checks project identity,
storage compatibility, required semantic contracts and the installed bundle's
integrity without looking for the former executable. Repeated init preserves the
saved record. A routine compatible rebuild needs no per-project rebind or
`session upgrade` command.

Use `/pause` when you want to stop new dispatch. `/exit` and `/detach` leave the
UI while background work continues. Opening a paused project preserves that pause;
`waywr resume` explicitly resumes dispatch. An idle service from another build is replaced automatically after its exact
process/socket ownership and quiescence are verified. Pending input or active work
keeps a compatible existing service attachable while its work continues.

Obsolete provider settings/bindings, unsupported session formats and stranded
uncertain host attempts start a fresh session automatically. Waywr stops the
old session's owned processes and removes its state and target configuration
from live use. Valid shared model choices are retained; retired native settings
are replaced from the current catalog. New context uses the current source and
new instructions. Code, commits and dirty worktrees remain. Old calls, queues
and approvals are never replayed. Private diagnostic archives and a resumable
renewal journal live under `.waywr`; see [project layout](docs/project-layout.md).

Installing a bundle does not convert every older storage format or authorize
work. Explicit `session upgrade`/`rollback` commands remain advanced maintenance
for their supported guarded migrations, including state8 provider import; see
[provider upgrade](docs/provider-selection.md#upgrade-and-verification-boundary).
These maintenance paths still validate their source/target records and installed
artifacts; ordinary reopening does not depend on them.

For a legacy `.wayward-wrangler` layout, invoke the new installed release's
migration command instead of renaming directories or upgrading the binding first:

```fish
set waywr_project /absolute/path/to/legacy-project
set waywr_prefix "$HOME/.local/share/wayward-wrangler"
"$waywr_prefix/current/bin/waywr" --project "$waywr_project" session migrate
```

Close the TUI first. Migration coordinates durable stop/drain, service shutdown,
relocation, supported Git worktree repair and compatible release binding. It keeps
project identity, approvals, task records, receipts and history, then leaves
dispatch paused. Conflicting directories are preserved without combining them.
Custom external state relocation remains unsupported. See
[canonical project layout and migration](docs/project-layout.md) for recovery,
compatibility aliases and limits. Argument references are also available through
`waywr bundle-build --help`, `waywr bundle-install --help` and
`waywr session upgrade --help`.

Development reading order: [AGENTS](AGENTS.md) → [handoff](docs/HANDOFF.md) →
[architecture](docs/architecture.md). [Documentation index](docs/README.md).
