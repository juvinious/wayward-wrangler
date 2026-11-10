# Working on Wayward Wrangler

Read this file, [docs/HANDOFF.md](docs/HANDOFF.md), and task-relevant source.
[README.md](README.md) explains use; [docs/architecture.md](docs/architecture.md)
maps components and targeted contracts. Archives are selective evidence, not a
startup reading list. `CLAUDE.md` points here as the canonical working policy.

## Working conventions

- Verify repository root, branch, status and checkpoint. Preserve unrelated edits
  and Git history. Continue authorized work without routine confirmation; pause
  only for substantive conflicts, unsafe actions or concrete blockers.
- Work only in this checkout. Do not modify another checkout,
  shared Herdr/tmux infrastructure, provider configuration or credentials without
  explicit authorization. Use Fish-compatible user examples.
- Reuse the accepted Rust/Ratatui TUI and Rust service components. Inspect current
  implementation before adding another architecture or abstraction.
- Use isolated projects for runtime checks. Authenticated calls, operator service
  or binding changes, global installation and publication need explicit task
  authorization. Mock fixtures grant no native-call or human approval authority.
- Preserve user state and immutable evidence unless disposal is explicitly
  authorized. Never restore old snapshots over accepted work, infer nonlaunch
  from absent processes, or replay ambiguous execution.
- Keep projects' state, budgets, context and authority separate. Cross-project
  context requires an explicit bounded grant. Model prose/display state cannot
  authorize work. Preserve exact-revision/artifact checks and runtime fencing.

## Development precepts

Use [the acceptance-driven development approach](docs/development-approach.md)
and the current slice in [the usability plan](docs/usability-plan.md).

The product order is broad provider/model support, lifecycle fluidity, then
coordination through a substantial real application. Follow the
[shared model-layer direction](docs/execution-adapter-decision.md):

- Keep provider/model IDs, catalogs, defaults and native options outside
  coordinator logic. New models on supported protocols are data/configuration,
  not core releases. New protocols/authentication can require integration code.
- Deliver a shared multi-provider execution path with usable selection, activity
  and usage. A model list, generic fixture or another Codex/Claude wrapper alone
  does not satisfy support. Pi's app is a reference; its independent provider/agent
  libraries are now the chosen implementation behind the shared process boundary.
- The user waived compatibility with current hardcoded integrations/profiles.
  Replace them directly; do not maintain parallel legacy routes or shims as a
  migration goal. Preserve project source/worktrees and actual execution ownership.
- Startup must upgrade usable configuration or retire obsolete/stranded session
  state and create a fresh context. The user authorized discarding old Waywr
  configuration/context; it must not remain a live startup dependency. Retired
  calls/approvals are not replayed or imported. Verify/stop the exact old owned
  processes before renewal; retain source, Git history and dirty worktrees.
  Compatible busy owners remain attachable across application builds.
- Use capability-based coordination across selected models. Do not silently
  narrow later milestones to one brand, add model-specific coordinator branches,
  or turn developer testing rules into arbitrary product-agent restrictions.

- Start with one user-visible outcome, its present failure, an acceptance example
  and explicit exclusions. Finish that vertical slice before expanding scope.
- Before changing application behavior, run an acceptance/regression test against
  the existing code and observe the expected behavioral failure. A broken harness,
  compiler error or missing dependency is not a valid red test.
- Use focused TDD for necessary underlying logic: red, minimal implementation,
  green, then refactor. Assert observable outcomes, not implementation structure.
- Exercise real installation, persistence, IPC and service components where those
  are the behavior under test. Mocks support narrow logic/fault cases; they do not
  establish ordinary usability or native execution.
- Keep checks proportionate and targeted. Do not turn the slice into an unrelated
  refactor, dependency upgrade, abstraction or broad acceptance campaign. Surface
  substantive scope changes instead of silently adopting them.
- Complete authorized work autonomously. A passing mechanism is not completion
  of the promised user journey. Report what now works, remaining intervention and
  verification limits; test counts are supporting detail.
- Follow the [TUI keyboard contract](docs/tui-controls.md) for new and changed
  panels: contextual arrows, Tab/Shift-Tab, Enter and layered Escape; visible
  focus and fixed control hints. Do not silently truncate selectable catalogs.
- Use ordinary work to assess fluidity when authorized. Convert reproducible
  friction into regressions; preserve the distinction between automated evidence
  and actual use. Never call a model or install into the operator's environment
  merely to make a test green.

## Git and review boundary

- Use read-only Git inspection for routine tasks. Staging this task's changes and
  making local commits on the current work branch are authorized checkpoints.
- Leave remote writes to the user: do not push, create/update PRs, merge remote
  work, tag/publish releases or change hosted settings. Earlier remote permissions
  do not carry forward. Prepare local commits and evidence for the user's review.
- Do not reset, clean, discard, rebase, delete branches or change checkout/history
  as routine cleanup. Preserve unrelated edits; seek explicit task authorization
  for any additional Git mutation. Isolated test repositories may use the local
  Git setup/commits necessary for the documented acceptance case.

## Build and verification

Use permanent rustup (`~/.cargo/bin`) and checked-in `rust-toolchain.toml` (1.99.0).
Preserve `Cargo.lock`; do not combine compiler work with dependency upgrades.
Do not use temporary `CARGO_HOME`/`RUSTUP_HOME` overrides.
For distributable binaries, use `sh scripts/build-release.sh` to remap builder
paths; packaging runs the public-content and notice checks.

```fish
cargo build --locked --release --bin waywr --bin waywr-service --bin waywr-runner
cargo test --locked -p waywr-backend TEST_FILTER
cargo test --locked -p waywr TEST_FILTER
```

Choose checks proportionate to the change and concrete risks. Read harnesses
before running them: some launch private processes/services. Do not repeat broad
acceptance or completed checks without a relevant reason. Documentation-only work
needs link/reference and diff checks, not application execution. Python harnesses
are optional developer tools, not a Rust application runtime requirement.

## Documentation at every checkpoint

Before committing or handing off, reassess docs against current task and source.
This includes routine checkpoints and task transitions, not just new branches.
It does not require another planning phase or a full historical acceptance matrix.

1. Replace the concise handoff with current objective/checkpoint, implemented
   behavior, unresolved issues, verification limits and next authorized action.
2. Reconcile contracts into their owners. Distinguish product requirements,
   implemented limitations, proposals and trial-only restrictions.
3. Archive completed/superseded reports, plans and walkthroughs after carrying
   forward applicable contracts and unresolved issues. Read archives selectively.
4. Update the short index and affected references; check consistency.

Do not manufacture churn when reassessment finds no change. Historical permissions
never authorize a new trial. Keep quoted paths as historical evidence references
when the user has authorized deleting their resources.

Handoff local commits to the user for review and remote delivery. Hosted PR/build
and security requirements remain applicable to the user's eventual publication;
they do not authorize agent remote writes.
