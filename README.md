# Wayward Wrangler

Waywr is a project-scoped development orchestrator. Initialize a project, discuss
and approve scope and operating policy, delegate to native workers, review exact
results, run checks and maintain project knowledge through its Rust terminal UI.
The intended workflow supports existing projects and substantial user requests.

## Current readiness

The accepted [usability plan](docs/usability-plan.md) targets removing application build
locks from projects, removing provider version allowlists and simplifying public
installation/update/uninstall. Follow the [development approach](docs/development-approach.md);
the new lifecycle is not implemented yet.

Waywr remains experimental. This increment implements existing-project setup,
exact candidates/checkout handoff, conservative resource retirement, project-aware
completion, Ollama coordinator/review and versioned distribution code. Focused
fixtures support these mechanisms; a full ordinary-project workflow, native Ollama
quality, ordinary project PR delivery, release publication and broad Linux
portability remain unverified. The PR/build workflow and real isolated packaged
installation passed; see [verification](docs/ordinary-project-verification.md). Small prior
deliveries do not establish complex-project readiness. See the
[workflow contract](docs/ordinary-project-workflow.md), [handoff](docs/HANDOFF.md) and
[retained evidence](docs/archive/conversational-coordination/2026-10-17-verified-checkpoint.md).

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

Linux with a systemd user manager and a usable Codex sandbox is the current execution
prerequisite. See [Linux sandbox setup](docs/linux-sandbox-setup.md) for the temporary
AppArmor testing workaround and distribution gaps. Herdr is
optional explicit read-only inspection. Both closing commands preserve your work:

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

New projects preserve Codex gpt-6.1-sol / medium as the coordinator and worker default.
The project-local profile is `.waywr/coordinator.json`. F3 edits project provider profiles, role defaults, fallback and task overrides;
`/plan` reviews the current conversation topic’s saved scope and operating policy.
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
fallback. Claude is an explicit coordinator, implementation or review profile;
existing Codex-only approvals remain Codex-only after upgrade. F3 → Switch defaults
to after the current turn, with explicit interruption available. Preference edits
apply to future attempts; reserved profiles and useful work remain intact.

See [provider/role controls](docs/provider-selection.md) for complete F3 controls,
capability provenance, policy amendments and upgrade semantics. Native clients are
trusted within the documented same-UID boundary. The human-led label-management-example request
used Codex coordination/implementation and Claude review through checked delivery.
Each new request and scope/policy approval still belongs to you. This assisted run
does not verify every provider role, handover or fallback combination.

## TUI controls and activity

| Control | Behavior |
| --- | --- |
| Up / Down in the composer | Recall older/newer entries, including typed slash commands; Down past newest restores your draft and cursor |
| Alt+Up / Alt+Down | Move the multiline input cursor |
| PageUp / PageDown | Scroll chat and live activity eight wrapped lines |
| Alt+PageUp / Alt+PageDown | Scroll chat or the open content panel one wrapped line |
| Ctrl-R | Open command history; Enter recalls into the composer without sending |
| Left / Right in modal action rows | Select previous/next option; Enter activates it. Tab / Shift-Tab also selects |
| F3 | Inspect settings/policy and choose subtle, reduced or off effects |
| F6 | Inspect scopes and concurrent child tasks |
| `/discuss` | Explicitly switch chat to the inspected task’s request |
| `/plan` | Review the current topic's saved scope and operating policy |
| `/output` | Read current/retained output; Left/Right switches attempts, arrows/page keys scroll |
| `/inspect` | Inspect integration, checks, obligations and exact readable history/summary paths |
| `/resume` | Resume a paused session or recover the selected blocked workflow after its blocker is resolved |
| `/detach` | Close the TUI while authorized work continues |
| `/exit` | Checkpoint and pause dispatch, then close the TUI |

History recall never sends or executes an entry. Modal text fields retain cursor
movement while editing; Tab moves focus to their actions. Queue Up/Down selects a
message, Left/Right selects its action, and page keys scroll. Closing commands open
a panel: select **Detach** or **Exit** with Right and confirm with Enter;
leaving the default **Stay** selected keeps the view open. Reopening `waywr` restores
context and preserves an intentional pause.

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

This branch supplies release workflows; no GitHub release has been published or
verified yet. After an authorized version release succeeds, use Fish:

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
sha256sum and normal systemd/provider execution prerequisites below. Rust is only
needed for source builds. The installer validates GitHub asset/checksum identity
and archive safety before the existing staged bundle install. It selects a new
user-local default without changing existing projects. Upgrade them explicitly
under [Existing projects](#existing-projects-upgrade-versus-migration).
[Distribution contract](docs/distribution.md) explains trust, platform limits,
packaging and publication steps still required.

## Build and install from source

Prerequisites:

- Linux, Git, a C compiler/linker and standard system headers. SQLite is built from
  the locked bundled dependency. Build on the target OS/architecture.
- Permanent rustup in `~/.cargo/bin` and the checked-in `rust-toolchain.toml`
  (Rust 1.99.0). Preserve `Cargo.lock`; the build may download locked dependencies.
  Do not override `CARGO_HOME` or `RUSTUP_HOME`.
- For project execution: a systemd user manager/session bus, `systemctl`,
  `systemd-run` and an authenticated supported Codex client (0.162.1/0.163.0) with a usable sandbox. Claude roles additionally require Claude Code2.1.296/2.1.301/2.1.302 and its Bash sandbox dependencies (bubblewrap and socat).
  [Linux setup](docs/linux-sandbox-setup.md) documents the temporary AppArmor
  workaround and distribution gaps. Installing Waywr does not authenticate Codex
  or change shared provider settings.

Run this in Fish from the checkout root, after installing rustup normally.
The toolchain command uses that permanent installation. Waywr's Rust runtime
requires no Python, venv, pip or Herdr; legacy Python source and optional developer
harnesses remain pending a separate source-cleanup assessment.

```fish
~/.cargo/bin/rustup toolchain install 1.99.0 --profile minimal
sh scripts/build-release.sh

set waywr_source (pwd -P)
set waywr_release "local-"(date -u +%Y%m%dT%H%M%SZ)"-"(git rev-parse --short=12 HEAD)
set waywr_bundle "$waywr_source/target/waywr-bundles/$waywr_release"
set waywr_prefix "$HOME/.local/share/wayward-wrangler"

./target/release/waywr bundle-build \
    --release-id "$waywr_release" \
    --output "$waywr_bundle" \
    --source-root "$waywr_source" \
    --frontend "$waywr_source/target/release/waywr" \
    --service "$waywr_source/target/release/waywr-service" \
    --runner "$waywr_source/target/release/waywr-runner"

./target/release/waywr bundle-install \
    --bundle "$waywr_bundle" \
    --prefix "$waywr_prefix" \
    --select-default

fish_add_path "$waywr_prefix/bin"
waywr --help
```

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
| `PROJECT/.waywr/release.json` | Project's exact release/manifest binding |
| `PROJECT/.waywr/{state,worktrees,history}/` | Project-owned execution, task worktrees and readable history |

`fish_add_path` persists the installer's bin directory for future Fish sessions.
Check resolution with `type -a waywr`. This workspace's existing
`~/.local/bin/waywr` link may keep pointing into the same installation; the
installer itself creates aliases under `PREFIX/bin`. The absolute stable command
for this example is `~/.local/share/wayward-wrangler/bin/waywr`. No sudo is needed.

## Existing projects: upgrade versus migration

Selecting a default affects new projects. Existing projects stay pinned by
`.waywr/release.json`; ordinary startup and repeated init honor that binding.
Installing another bundle does not upgrade a project, move files, convert
older states3–7 to the current schema or grant scope approval. A normal quiescent state8 upgrade imports state9 provider preferences while preserving Codex-only authority; see [provider upgrade](docs/provider-selection.md#upgrade-and-verification-boundary). Retain installed releases referenced
by bindings or recovery records.

For a canonical `.waywr` project, install the new bundle first. Close its TUI with
`/exit` to checkpoint/pause; `/detach` alone leaves work running. Alternatively:

```fish
set waywr_project /absolute/path/to/project
waywr --project "$waywr_project" session stop
waywr --project "$waywr_project" session status
```

Wait until status reports `"phase": "paused"` and close any attached view.
Using the build variables above, stop only that project's service and use the
guarded binding upgrade:

```fish
waywr --project "$waywr_project" session upgrade --release "$waywr_release"
```

The upgrade command checks for a detached view, pending input and unsettled work,
then stops only the selected quiescent service itself. Active or uncertain work
refuses without interruption. Records, approval and intentional pause state stay
intact. Launch normally with `waywr --project "$waywr_project"`; use `resume` only
when you intend to resume paused dispatch. Selecting an installation default alone
never upgrades project bindings.

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
