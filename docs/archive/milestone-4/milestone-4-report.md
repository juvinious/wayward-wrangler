# Milestone 4 — installed, project-independent execution

**Branch:** `milestone-4-portability`, based on the committed proposal. No merge
or release is part of this checkpoint. The user's shared Herdr server and linked
plugin were not changed. All new workflow execution used fake workers.

## Implemented boundaries

- Wheel 0.2.0 includes the Python modules, canonical draft/critique/revision
  skills, fake workflow default and Herdr plugin assets. Both
  `wayward-wrangler` and `awp` are installed entry points. Controller and runner
  use the installed interpreter; fake children no longer need source PYTHONPATH.
- `--project` explicitly selects a target for run/inspection commands, even
  through `awp`. `init` creates a stable random project key, versionable project
  identity and fake workflow config. The default SQLite/run store is
  `PROJECT/.wayward-wrangler/state/`; an explicit `--state-dir` can move it. The
  generated `.gitignore` ignores that default state subdirectory only. It does
  not ignore project config or context. Briefs, config, rules and context
  selections resolve against the selected project or their containing config/
  selection file, with symlink escape checks for rules/context.
- New envelopes and operations freeze project key, version/protocol and relative
  project references. The user-local location index maps key to current
  project/state for review links and admission. It has no decisions, findings,
  graph state, transcripts or artifacts. New links include project key, run,
  artifact and SHA-256. Old run-only links refuse automatic routing.
- `integration herdr install` copies packaged plugin assets to user-local data,
  binds the *venv* interpreter path and registry path, then uses Herdr 0.9.3's
  `plugin link`. A registration at another directory is refused; switching the
  user's shared plugin was deliberately not performed. Real worker panes remain
  actual execution terminals. Controller/worker/review labels carry project
  identity. Runner receipts, not pane text, establish completion.
- Explicit `location-rebind` changes only user-local location metadata, after
  rejecting live workers and active/unresolved operations at both locations.
  New context snapshots use project-relative references and retain content
  hashes. Copied legacy state uses explicit `--state-dir` and, when validating a
  frozen context after a copy, `--legacy-origin` maps recomputed paths into the
  historical namespace. This never edits old packets, checkpoints, approvals,
  failure receipts, adoption records or recorded path strings.

New project runs explicitly refuse native providers, including reuse of a
historical trial ID; the existing real-run records remain inspectable. The
milestone does not authorize a new Claude/Codex call. Herdr stays the sole
execution integration; there is no hidden subprocess alternative.

## Verification

| Check | Result and evidence |
| --- | --- |
| Offline regressions | 70 tests passed; `pip check` passed. Includes state/key conflict, cross-project active-run refusal, missing binding, context symlink escape and refusal to reuse a historical native trial ID. |
| Wheel and pipx | Built local 0.2.0 wheel, SHA-256 `14bc51a3634171c02656c2dd60bd18da4046783301ca7c84644c15108f2e9fa2`; preserved at `.local/milestone4/acceptance/wayward_wrangler-0.2.0-py3-none-any.whl`. Archive contains skills, default workflow, plugin manifest/handler and both entry points. Installed into fresh `/tmp` pipx home (pipx 1.17.13, Python 3.12.3) with the checked-in lock as a pip constraint; all 41 locked dependencies matched. Module import resolves to pipx site-packages. No editable install or global `ensurepath`. |
| Isolated Herdr, installed public CLI | `.local/milestone4/acceptance/host/result.json`: copied plugin registration, visible separate controller/worker panes, live fake output, controller exit with active worker, fresh resume with the same pane and one draft invocation, retained completion output. Both projects completed `demo` independently with different decisions and approvals; link click opened B's hash-checked artifact in Vim; changed hash and old ambiguous link refused. Both projects' identical run IDs stayed separate. |
| Relocation | In the same isolated run, A was copied after all workers stopped, original test location renamed away, explicit location rebind succeeded, approval bytes matched, and selected context validated at the new root. Changing a selected document then produced stale-context refusal. |
| Source inaccessible | A separate network-isolated Docker run used only the pipx-installed site-packages, copied test driver, Herdr 0.9.3 binary and test evidence volume. The original checkout path was absent; the controller and fake workers completed the same two-project acceptance. Evidence: `.local/milestone4/acceptance/container/result.json`. This container reused the tested pipx site-packages and a module entry wrapper; the host run used the actual pipx entry point and included the Vim link click. |
| Copied legacy evidence | The installed package checked copied `context-01` context and adopted draft in a network-isolated container with the original checkout absent. The test copy and validator are retained at `.local/milestone4/acceptance/legacy-copy/` and `.local/milestone4/acceptance/legacy-check.py`. Result: `approved`, effective receipt `completed/recovered`; adoption file SHA-256 `dde41684f3e19952be09dad000c7bc729cdd45af5c603903eff58b2bd20b5d02`. The original envelope, A1 decision, failed receipt and adoption file all matched their precheck hashes afterward. |
| Recovery regression | The existing milestone-3 isolated Herdr matrix passed all 12 cases after the installed-launch changes: dispatch/permit boundaries, post-publication replay, surviving child, four cancellation races and missing/replaced server. `.local/milestone4/acceptance/recovery/result.json`; cleanup reports no retained test process. |

Acceptance answers and approvals above are **test fixtures**, not operator
decisions for a real project. All isolated servers and owned workers stopped.
The user's active server, tabs and checkout location remained intact.
Compact result and cleanup summaries are retained locally at
`.local/milestone4/verification.json` and adjacent files. The full final host,
container, recovery and copied-legacy test evidence, plus the wheel and dependency
freeze, is under ignored `.local/milestone4/acceptance/`. Every copied regular
file was SHA-256 checked against its `/tmp` original; file counts and manifest
digests are in `.local/milestone4/copy-verification.json`. These local files are
not part of GitHub. Historical run evidence was not changed.

## Review cleanup

The implementation diff was checked against the accepted proposal for project
and state isolation, packaged runtime resources, project-qualified review links,
explicit relocation and milestone-3 recovery guards. Review-link lookup now
checks the registered state's identity marker before opening an artifact; a
missing, changed or malformed marker gives a stale-location error. All six
focused portability tests passed, and the corrected legacy inspection command
returned the approved `context-01` record without starting a worker. The full
installed acceptance suite was not repeated for this narrow code fix.

## Remaining limits

- Native Claude/Codex execution was not invoked or revalidated after wheel
  packaging. The existing adapter versions, authentication and permission gates
  remain external prerequisites for a separately authorized trial.
- The user's currently linked checkout plugin was not switched. An explicit
  unlink/install action in that shared Herdr configuration is a separate step.
- New runs have project-relative selected context. Legacy verification requires
  an explicit original-root mapping and matching copied documents; missing or
  changed dependencies remain unverified, while stored approved artifacts remain
  inspectable. Active workers cannot be moved with a project.
- The index serializes admission for one local installation/registry. It does not
  coordinate separate machines or independent registries. Orphan recovery,
  concurrent scheduling, power-loss guarantees and arbitrary crash windows remain
  outside this milestone; uncertain execution still stops without a blind retry.
- A quiescent relocation verifies the project marker and available operation
  receipts. If the original location is gone, the tool cannot compare every
  copied state byte against it; preserve the old location or a verified copy
  manifest when that comparison matters.
- The container test used a copied installed site-packages tree because the
  available Docker build environment could not reach Debian apt repositories.
  It proves execution with the source path absent, alongside the separate fresh
  pipx installed-entry-point test; it does not prove a fresh pipx installation
  *inside* that container.

## Operator notes

Use the Fish-compatible commands in [README.md](../../../README.md). A new target
project must be initialized explicitly. Existing approved runs remain available
via `--project /path/to/old-checkout --state-dir .local`; there is no silent
migration and the historical absolute paths remain provenance. `location-rebind`
is for a quiescent copied project with its matching state marker. A missing or
ambiguous review location reports an action rather than searching by run ID.

No GitHub release was tagged or published. The versioned wheel remains a local
acceptance artifact; release hosting needs a later publication decision.
