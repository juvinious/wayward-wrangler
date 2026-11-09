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

For an authorized PR merge, satisfy gates and use `gh pr merge -sd` when squash is
allowed (`-d` deletes topic branches). Verify merge/cleanup. Never force-delete
unmerged work or push/merge without task authorization.
