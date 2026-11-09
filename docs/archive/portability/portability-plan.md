> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Portability direction — proposed, not implemented

**Historical planning record.** Milestone 4 implements the accepted scoped
proposal; see [milestone-4 report](../milestone-4/milestone-4-report.md) for verified behavior
and remaining limits. The descriptions below record the earlier checkout-bound
baseline and should not be read as current runtime instructions.

Wayward Wrangler is intended to be an **installable utility usable across multiple
projects, independently of its own source checkout**. This checkpoint records the
boundaries and current dependencies; it does not implement a portability refactor,
another execution backend, a new trial, or milestone 3. Include portability when
planning future recovery work without expanding the approved recovery-test scope.

## Proposed boundaries

| Concern | Intended owner and location | Current dependency / proposed change |
| --- | --- | --- |
| Installed application and dependencies | An isolated installation containing the CLI, Python modules and packaged default skills/resources | `storage.ROOT` derives from `__file__`; launches assume `ROOT/.venv`. Use installed resources and an installation-owned interpreter/entry point. Never write run state into site-packages. |
| Target project | Explicit project directory and stable project identity; not the installation root | `context.project_file()` restricts selection to the source checkout. Resolve relative documents against the selected project, with containment and stale-content checks. |
| Per-project configuration and durable context | Explicit configuration location; versionable rules, role/model settings and context selections; canonical decisions/findings remain in existing durable records | Profiles currently live in checkout `config/`; rules resolve relative to the profile. Keep explicit selection and canonical documentation; no duplicate decision database, transcript dump or relevance service. |
| Runtime state, artifacts and execution records | Explicit state location, partitioned by stable project ID; database, envelopes, outputs, receipts, answers and approval bindings retained together | `LOCAL = ROOT/.local`. Relate current storage locations to stable run/operation/artifact IDs and relative paths, separately from historical provenance. Preserve one active run and sequential execution in this follow-up. |
| Execution/display integration | Herdr connection/workspace, pane creation/discovery/focus and terminal observation | Herdr visibility is required. Runner identity/receipts prove completion; Herdr panes do not authorize decisions. Workflow state, decisions, context logic and Claude/Codex adapters must not depend on Herdr layout or plugin paths. |

Herdr remains the **single execution integration**. Define a narrow boundary for
launch/reconcile/observe/cancel requests and terminal bindings; no second backend,
backend registry or speculative abstraction hierarchy is needed. LangGraph owns
transitions/checkpoints, application records own human responses and approval
bindings, runner evidence owns execution completion, adapters own CLI I/O and
permissions. Today `herdr_backend.py` also validates worker results; separating
that contract from display code is part of the proposed boundary, not a change
made here. CLI decisions continue through the controller, never terminal scraping
or unrecorded input to a worker.

## Verified current coupling

Source inspection (not relocation runtime validation):

- `storage.py`: checkout-derived ROOT/LOCAL and operation containment. Records and
  workflow use that LOCAL for application tables and LangGraph checkpoints.
- `cli.py:create`: checkout skills/default profiles; freezes rule source paths and
  text. `herdr_backend.ensure`, `gate_demo` and controller/review commands use
  checkout `.venv` paths. `runner.py` supplies checkout `src` as fake-worker
  PYTHONPATH. These are active dependencies even though not part of the 145 count.
- `context.py:project_file/resolve/check`: canonical absolute selection/document
  paths and dependency-map keys participate in exact snapshot comparison.
  `recovery.verify` invokes that comparison when validating the adopted draft.
  An old-path symlink canonicalizes to the new directory and does not solve it.
- `recovery.adopt` records a validator hash by reading checkout source. Installed
  version/resource evidence must replace this lookup for future operations;
  preserve the existing adoption record and its narrow authorization.
- `adapters/real.py`: configured absolute native binaries and exact CLI version
  checks; existing authentication stays external. `execution.json` records
  actual schema/output arguments. Do not replay recorded command lines after a
  move; resolve current destinations for newly authorized operations only.
- `herdr-plugin/open_review.py`: finds the executable beside its linked checkout.
  The installed Herdr registration points to that checkout too. `awp.invalid`
  review links encode run/artifact/hash, but no project identity: multi-project
  routing needs explicit project/state binding and ambiguity refusal.
- Herdr socket/workspace and process boot/PID/start identities are machine/session
  bindings, not portable evidence of a live worker after relocation. Existing
  operation/receipt identities must remain unchanged.

## What the 145 references mean

The original inventory counted literal occurrences of
`~/projects/example-workspace` in **62 `.local` JSON/Markdown/text
files**. It excludes SQLite, JSONL, logs, installed scripts, source/docs/config and
the new checkpoint evidence. It is not a count of all relocation dependencies.
[Exact per-file inventory](path-reference-inventory.md) identifies all 62 files.

| Occurrences | Fields/files | Current use |
| ---: | --- | --- |
| 45 | Fifteen `.local/evidence/*.json` snapshots | Historical inspection evidence: frozen summaries, commands, cwd, transcript locations and context snapshots; not current launch/path configuration. |
| 34 | `instructions/0/source` in four input envelopes and their twenty stage packets, plus ten saved real-worker prompts | Historical attribution alongside frozen instruction text/hash. Resume uses the supplied content; it does not reopen the instruction source path. |
| 4 | `context-01/inputs/envelope.json`: context `selection_path`, two `dependencies` keys, one entry `source_path` | **Active validation coupling**, mirrored in the canonical SQLite envelope: selection is reopened; resolved document paths and keys are recomputed and compared. These remain immutable provenance but currently require the old canonical root. |
| 20 | Same four context fields in five context-01 `packet.json` files | Frozen worker inputs/hash evidence. Not separately dereferenced as live configuration; recovery verifies exact frozen packet/prompt content. |
| 20 | Those four context paths in five context-01 `prompt.txt` files | Immutable bytes delivered to workers and checked during recovery; not current path resolution. |
| 14 | Controller launch/pane records under demo, m2-cli-01 and context-01 | Historical launch/cwd evidence; current reconciliation searches live Herdr identity instead of executing these saved argv. Those historical controllers have exited. |
| 8 | Four Codex critique `execution.json` files, two argv paths each | Historical `--output-schema`/`--output-last-message` paths. No current replay of those command lines; new execution builds current paths. |
| **145** | **Total** | **141 evidence-only occurrences and 4 path-sensitive context-envelope occurrences (also mirrored in SQLite).** |

“Evidence-only” does not mean disposable: packet/prompt hashes and execution records
may be required for validation. Preserve every original byte. The distinction is
whether the literal old pathname must exist or is recomputed, not whether the record
is authoritative. The exported context envelope is compared with the canonical DB
record; counting only its four fields understates the total DB/checkpoint coupling.

## Scoped follow-up and acceptance

Proposed interface, **not available yet**:
`wayward-wrangler --project DIR --config FILE --state-dir DIR ...`.
Make paths explicit initially; defer automatic discovery. Keep project identity in
configuration and state, reject mismatches, and namespace identical run IDs across
projects. Configuration/rules are read from the target project; CLI binaries and
Herdr connection are explicit machine bindings, never credentials in project Git.

1. **Non-editable installation:** package required skills/resources and plugin
   assets, build/install an artifact into a fresh isolated environment. Run help,
   offline checks and a fake workflow from a cwd outside the checkout with the
   checkout inaccessible. No `ROOT/.venv` or source PYTHONPATH dependency remains.
2. **Explicit project and state:** use two disposable target directories with
   different rules/contexts and separate state. Execute sequentially, not in
   parallel. Prove no cross-project decisions, artifacts, locks or review routing;
   same run IDs cannot select another project's data. Project directories are
   not assumed to contain a Python environment or this utility's source.
3. **Independent Herdr plugin installation:** install versioned assets from the
   application distribution to an explicit integration location, with an executable
   binding independent of source symlinks. Pass/resolve project and state identity
   for review links, verify hash and scope before opening Vim; retain old-link
   compatibility through explicit unambiguous bindings. Any shared registration
   changes require their own authorization. Workers remain individually visible.
4. **Relocation and recovery:** append a verified current-location binding outside
   immutable historical records; use stable identities/relative artifact paths for
   new records. Legacy lookup translates only through an explicit scoped binding,
   verifies original content/hash and compares provenance in its original namespace.
   Never regenerate approval hashes or rewrite frozen paths, receipts or checkpoints.
   A completed simulated output must reconcile after restart/relocation with the old
   root absent and no duplicate call. Confirm decisions/approval survive, stale
   source content is still rejected, and missing/mismatched evidence stops uncertain.
   Never treat a new pane/process as the original surviving worker based on path.
   Refuse moving active worker storage until supported reconciliation proves it safe.
5. **Verify and document:** fresh installation + second disposable project are
   required acceptance checks, including Herdr live visibility/review and installation
   path independence. Use synthetic fixtures or verified copies for legacy adoption
   validation; never dispatch another approved trial or broaden saved-output adoption
   beyond context-01. Record original-versus-current path bindings and checksums.

No historical path is required to exist by the *intended* new runtime design.
That is a requirement, not a capability verified today. Portability work must not
silently weaken evidence checks to pass relocation tests. Failure-injection and
recovery cases remain within separately approved scope; this plan does not authorize
milestone 3, new native model calls, permission expansion or retry behavior.
