> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Milestone 4 proposal — installed, project-independent execution

**Status: proposal only; awaiting operator approval.** No application changes,
installation, wheel build, worker execution, plugin registration, directory move or
historical evidence modification is authorized by this document.

## Baseline and acceptance goal

Inspected code: `acc2e894eefe6e675c3023eaf6a4c7d009bcfbf8` on `main`.
PR #1 merged on
2026-10-10 at 23:13:39 UTC. After fetch, local main and origin/main matched that
commit and the working tree was clean. Planning branch: `milestone-4-portability-plan`.
Milestone 3 is complete, not pending work to resume.

Install a **locally built Python wheel through pipx into a fresh environment**;
run the existing bounded workflow against two disposable projects sequentially,
with separately visible Herdr workers. Stop/resume the controller, answer through
the durable gate, and inspect decisions/artifacts with the source checkout
inaccessible. Same run IDs must remain isolated between projects.

The user selected wheel + pipx. Versioned wheels will be hosted on GitHub Releases
**after local installation acceptance passes**. Public PyPI and OS packages are
excluded. Building a wheel is not evidence that the installed workflow works.

## What the code actually supports today

These are source-verified facts; proposed behavior below has not been runtime-tested.

| Current code | Consequence / required change |
| --- | --- |
| `storage.py:11–12`, `records.py:7–23` | ROOT derives from the module path and LOCAL/DB are imported globals. A normal wheel would derive state from the installation. Replace these with explicit project/store dependencies, not another environment-variable patch. |
| `cli.py:create/parser`, `herdr_backend.py:ensure`, `runner.py:main`, `gate_demo.py` | Skills/defaults come from checkout folders; controller/runner commands assume `.venv`; fake children get source PYTHONPATH. Use packaged resources and the installed interpreter. Milestone-3 `--state-dir` alone does not solve this. |
| `pyproject.toml`, `skills/`, `herdr-plugin/` | Both CLI aliases exist, but external Markdown/plugin assets have no package-data declaration. Include required assets in the wheel and inspect the built archive. |
| `adapters/real.py:prompt/schema` | Native prompts/schemas are Python code already eligible for packaging; do not create redundant template copies. Preserve exact legacy generation because saved-output verification regenerates these bytes. |
| `context.py:resolve/check` | Frozen absolute paths are compared to recomputed paths. A symlink or simple ROOT replacement breaks legacy validation. Add versioned relative references and a narrow legacy location adapter. |
| `context_snapshot.project_id` | This is already a logical context scope such as `example-notes`, not a filesystem/store identity. Add a separate project key; do not reinterpret historical values. |
| `review.py:LINK_PATTERN`, `routing.py:controller_panes` | Links and controller lookup use run ID without project identity. Two projects named `demo` can be confused despite separate databases. Namespace links and controller discovery. |
| `recovery.py:verify/adopt` | Existing adoption validates exact context, packets, prompts, schemas and evidence. Validator source hashing also assumes checkout layout. Preserve that narrow recovery contract using installed module resources. |
| `tests/test_project_identity.py`, `tests/m3_actor.py` | Existing tests inspect checkout metadata/scripts or patch globals before imports. They cannot prove wheel independence. Add acceptance via installed entry points; preserve fault barriers as test-only code. |

Herdr 0.9.3 CLI help offers `plugin install` for GitHub sources and `plugin link`
for local directories. Its inspected source at
`7b116c05bfda646af39d2524c54e70c751f57ee8`, `src/cli/plugin.rs:plugin_link`, registers
a supplied directory; it does not provide a general local-wheel plugin installer.
Copying packaged assets to an application-owned integration directory and registering
that directory is the smallest supported path. No development-checkout symlink is
needed. This new packaging/registration path still needs runtime validation.

`pipx` was not found on the tool shell's PATH during inspection. Python is 3.12.3.
Availability and bootstrap of pipx are implementation prerequisites, not completed
checks; do not alter global Python or Fish configuration automatically.

## Proposed boundaries and defaults

| Concern | Proposed location / owner |
| --- | --- |
| Installed application | pipx-managed environment: Python modules, default role skills, safe fake profile/example brief and Herdr plugin assets. Read-only at runtime; never holds project state or authentication. |
| Target project | Explicit `--project DIR`, canonicalized once. Application source is an ordinary possible target, never an implicit default. |
| Project configuration | `PROJECT/.wayward-wrangler/project.toml`, created only by explicit `init`. Contains a stable generated `project_key`, schema version, two configured roles, limits and rule/context references. Optional `--config FILE` override; keep `--profile` as its alias. |
| Durable context | Explicit selection TOML plus referenced canonical project docs; accepted decisions/findings remain in the existing per-project records/artifacts. No second memory store or automatic relevance discovery. |
| Runtime state | Default `PROJECT/.wayward-wrangler/state/`; explicit `--state-dir DIR` supported. One database and run tree per project, with a project-key identity marker. Git-ignore state, keep suitable config versionable. |
| Local location index | `${XDG_STATE_HOME:-~/.local/state}/wayward-wrangler/`: minimal project-key → current project/state binding and binding history for link routing and explicit relocation. No transcripts, decisions or workflow progress duplicated here. An explicit `--registry-dir` makes tests/plugin routing isolated and reproducible. |
| Herdr integration | Copied, versioned assets under `${XDG_DATA_HOME:-~/.local/share}/wayward-wrangler/herdr/`; installation binding points to the installed interpreter, not the checkout or whatever Python is first on PATH. Explicit plugin command performs registration. |

A small immutable `RuntimePaths`/project binding object and a store bound to it
replace imported ROOT/LOCAL/DB values. Build the existing graph with that store
captured by its nodes; do not persist Python runtime objects in checkpoints or
change existing node names. Storage, context and adapters must not depend on Herdr
pane discovery. Herdr receives explicit execution/display requests; runner receipts
continue to own completion evidence. Do not add a second backend or scheduler.

Keep the existing single-active-run guard in each store. For the current sequential
scope, serialize new dispatch admission with a local registry lock and inspect
registered stores for active/unresolved work before admitting a different project.
The index is only location metadata: existing app records/receipts remain authoritative.
An unavailable registered store cannot be assumed idle. No cross-project scheduling,
parallel workers or support for distributed/independent registries is promised.

### Predictable path resolution

- `--project`: relative to invocation cwd, or absolute. Require it for run commands;
  do not search parents, infer from Herdr focus, or silently select the last project.
- `--config/--profile`, `--brief`, `--context`, `--state-dir`, `--answer-file` and
  `--reason-file`: relative to the selected project unless absolute. Display resolved
  locations in `status --json` and errors. Requiring a brief for new runs avoids
  silently using the application's demonstration brief.
- Paths *inside* config remain relative to that config file; document entries remain
  relative to their selection file. Resolve and enforce target-project containment
  for rules/context documents, including symlink escapes. Machine CLI binaries are
  separately configured absolute paths, never interpreted as project documents.
- `--registry-dir` and explicit plugin destination: absolute, so a clicked link has
  no cwd-dependent meaning. The plugin records the registry binding when installed.
- On resume, use frozen run inputs and validated project/store binding. A newly
  supplied config must not silently change an existing run's inputs or permissions.
- Database, operation, response and artifact lookups always use the selected store.
  Keep SQL run/thread IDs local to that database; externally identify work by
  `(project_key, run_id)`. Reject state roots bound to another key, duplicate project
  keys at another location, ambiguous legacy links and implicit location takeover.

Proposed CLI shape (not implemented; Fish-compatible placeholders):

```fish
wayward-wrangler --project /path/to/project-a init
wayward-wrangler --project /path/to/project-a demo --run demo --brief brief.md
wayward-wrangler --project /path/to/project-a status demo
wayward-wrangler --project /path/to/project-a resume demo
wayward-wrangler --project /path/to/project-a show demo --artifact revision-2
awp --project /path/to/project-b status demo
```

The interactive controller remains the place to answer and approve; `open` reviews
in Vim and “approve this revision” binds the exact artifact internally. Existing
record-only decision commands remain available. Roles/models stay configurable;
acceptance uses only the fake provider, at most five calls/two critiques per run.

## Packaging and execution

Move canonical skill assets under package resources and load them with
`importlib.resources`. Include plugin manifest/handler and minimal initialization
assets through setuptools package data. Ship generated-schema code in its current
module; persist per-operation schemas as today. Do not ship `.local`, credentials,
recorded trial inputs or machine-specific real-trial profiles as installation defaults.
Source documentation/examples may remain in the repository without being runtime
requirements. Avoid duplicate editable copies of canonical skills.

Runner, fake child and controller commands use the running installation's
`sys.executable -m agent_workflow...` plus explicit project/state identifiers. Remove
checkout PYTHONPATH and `.venv` lookups; use installed distribution metadata for
version reporting. Preserve sanitized worker environments and all native permission
restrictions. Native adapters remain packaged, but no new native execution permission
is introduced. The current native start gate remains limited to the specifically
authorized historical trial IDs; arbitrary real-project model runs are not certified
or enabled by simulated portability acceptance. Record the installed application/
protocol version for new operations;
refuse an incompatible in-flight protocol on resume rather than silently changing it.

External prerequisites: Linux with the process identity/PTY behavior tested in
milestone 3; Python (initial acceptance: 3.12; declared minimum remains 3.11), pipx,
Herdr (initially 0.9.3) and Vim for review. Claude/Codex are **optional for simulated
runs**, required externally only when those roles are selected. Current adapters pin
Claude 2.1.296 and Codex 0.162.1; version/model/permission checks remain in force.
Existing CLI authentication stays external. Do not bundle, upgrade or authenticate
agent CLIs during this milestone. No hosted service is required.

Build the wheel in a disposable build environment, install it with pipx under fresh
PIPX_HOME/PIPX_BIN_DIR and isolated user directories, and install the existing locked
dependencies. Verify dependency resolution against the lock (a staged wheelhouse and
constraints can make acceptance reproducible). No editable install, `pipx inject`
of the source, or source PYTHONPATH. Bootstrap pipx in a disposable environment if
needed; no global install or automatic `ensurepath`/shell edit. Use explicit binary
paths for the test. Exact pipx flags will be verified against the installed version.
After acceptance, a versioned wheel (recommended 0.2.0), checksum and prerequisite/
install notes are suitable GitHub Release assets. Upload/tag creation is a separate
explicit publication step; this planning turn authorizes neither.

## Herdr plugin and project isolation

An explicit `integration herdr install` copies packaged assets to the integration
directory, writes a validated executable/registry binding, and uses Herdr's supported
local registration interface. Installing the Python wheel itself does not modify
Herdr settings. Test registration in an isolated Herdr instance; retain the user's
current linked plugin until a separately approved switch. Refuse overwriting another
installation's registration without an explicit replacement action.

New links carry a version, project key, run, artifact and SHA-256, e.g.
`https://awp.invalid/v2/review/PROJECT_KEY/RUN/ARTIFACT/SHA256`. This is a local handler
identifier, not a network request. Validate the complete link, registered binding,
project/state identity and artifact hash before opening the existing read-only Vim
command. Never infer project from focused workspace. Keep the existing plugin ID;
add support for old link syntax only with an explicitly designated legacy binding,
or explicit `--project` through the CLI. Do not scan every project and pick a matching
run. Missing/stale bindings fail with a command the operator can use to repair them.

Controller and worker labels include project, run, stage, role and provider/model.
Discovery matches project/store plus launch identity; matching `--run demo` alone
must not block/reuse/cancel another project's controller. Preserve claim, permission,
receipt and cancellation safeguards; display state cannot authorize workflow actions.
Reinstalling the utility may require explicitly refreshing the plugin's installation
binding; supporting a stale interpreter path by falling back to a checkout is forbidden.

## Historical evidence and existing runs

The [145-reference inventory](../portability/path-reference-inventory.md) remains historical, not
a complete dependency census: 141 evidence occurrences and four path-sensitive
context-envelope fields, with more copies in SQLite. Milestone-3 state-path passing
has not fixed those active context dependencies.

1. New records get a separate location-format version, project key and project-relative
   selection/document references with content hashes. Existing workflow/result schema
   contracts stay versioned independently. Current artifact manifests already have
   relative paths; retain that design.
2. Existing v1/v2 runs stay readable in an explicitly selected legacy state root
   (`--state-dir /path/to/legacy-state`). No checkpoint migration, mass path replacement,
   new hashes, altered approvals or historical launch replay. Both CLI names remain
   installed. Older commands need the explicit project option; this is an intentional
   documented CLI change, not a promise that unqualified commands select the old checkout.
3. Register legacy/current root bindings in a sidecar outside immutable run evidence.
   For copied legacy state, verify the original envelope/evidence digests and permitted
   path prefixes. Re-resolve current files and hashes, then compare using the frozen
   historical namespace. Do not mutate the snapshot or treat a translation as permission
   to accept changed content, selection, decisions or an unrelated directory.
4. Distinguish execution readiness from historical inspection: approved artifacts and
   decisions remain inspectable from their stored records even if source docs are gone.
   A context/adoption verification that needs missing docs reports unverified/stale;
   it does not dispatch, reopen approval or falsely certify the adopted receipt.
   Full verification on a relocated copy requires mapped, matching dependencies.
5. Preserve the exact saved-output adoption contract and original failed receipt.
   Use installed resources for validator identity without rewriting past validator
   hashes; retain legacy prompt/schema generation byte-for-byte. No new adoption IDs,
   automatic recovery, retry or repeated draft call. Test on synthetic fixtures and
   disposable copies; do not apply bindings/migrations to the original runs here.

An explicit location rebind is permitted only for a quiescent project/store after
identity/content checks, with an append-only sidecar record. Refuse it while workers,
controllers or unresolved execution exist; do not pretend a surviving process moved.
Relocate disposable fixtures for tests, never this development directory. Old root
locations must not need to exist for **new** runs, and copied legacy verification
must work through explicit mappings without recreating the old path or symlink.

## Small implementation sequence and acceptance checks

| Step | Deliverable | Observable pass/fail gate |
| --- | --- | --- |
| 1. Package resources | Wheel metadata/assets, installed launch resolution, updated entry-point tests | Inspect wheel contents; fresh pipx install resolves dependencies, both aliases run, all required resources load. Installed imports originate in pipx site-packages, no editable metadata/source PYTHONPATH. No workflow proof claimed yet. |
| 2. Bind project and state | Runtime/store dependency object, explicit init/paths, identity markers and minimal location index | From an unrelated cwd, absolute project and project-relative inputs resolve predictably. Two distinct projects may both own `demo`, with separate DBs/artifacts/answers. Wrong/shared state, duplicate key, escaping paths and implicit selection are refused. |
| 3. Preserve context/legacy contracts | Versioned relative context refs, legacy read/verification adapter, explicit quiescent location binding | New context remains valid after disposable relocation; changed bytes/selection still fail. Copied legacy approvals, packets, decisions, original failure and adoption bytes/hashes remain identical. Missing dependencies do not prevent stored artifact inspection, but cannot certify continuation. |
| 4. Install Herdr integration | Copied plugin assets, installed executable binding, project-qualified links/labels/discovery | Plugin registers in isolated server without checkout link. Same run ID in A/B opens the right hash-checked artifact; wrong/ambiguous/stale links refuse. Actual worker execution stays visible, distinct and inspectable. No tail-only pane. |
| 5. Prove end-to-end independence | Installed-CLI acceptance harness and report; scoped recovery regressions | Full checks below pass in fresh installation with source inaccessible. Only then prepare versioned release assets; stop for review/publication decision. |

End-to-end acceptance must use **public installed commands**, not milestone-3's
module-global patching as a substitute for real configuration:

- Build from a disposable source export. Run the installed controller, worker, plugin
  and test driver inside a test-only filesystem isolation boundary where both that
  export and the original checkout cannot be read. Mask paths only in the test
  process namespace; never move/chmod the active checkout. Verify the source-read
  attempt fails. Availability of suitable OS isolation is a preflight check; if it
  cannot be established, report that limitation instead of claiming full independence.
- Use isolated HOME/XDG/pipx directories and a separate Herdr server as in milestone 3.
  Use packaged fake defaults and target-owned brief/rules. Put the controller in a
  visible pane and each actual worker in its own labeled pane. Capture live output,
  bindings and retained completion/failure output.
- In project A, use `demo`, stop the controller while a worker is active, resume with
  a fresh process and prove the same operation/pane and invocation count. At the
  durable choice, stop again, save a fixture answer record-only, then consume it
  after restart. Final fixture approval binds exact artifact/decisions; repeated
  resume/answer/approval has no repeated calls or logical applications.
- Run project B sequentially with the **same run ID** but distinct brief, rules and
  answer. From a third cwd, status/show/logs and plugin review resolve only its own
  records. Assert no cross-project decision/context import, cancellation or controller
  match. Cross-project active-work admission must refuse while A is unresolved.
- Resolve explicitly selected accepted context from a prior fixture in A; B cannot
  access it merely by matching the run ID or context scope string. Fake adapter
  currently emits v1 results: extend its deterministic fixtures only as needed to
  exercise the existing v2 question contract, without adding a workflow type. Keep
  the one-new-question constraint and unchanged native prompt/schema contracts.
- Reuse milestone-3 offline permutations against the explicit store dependencies;
  retain live installed-runner checks for dispatch/permission, post-publication
  replay, cancellation, surviving orphan and missing/replaced **isolated** server.
  Ambiguity remains uncertain; cleanup verifies only owned test process identities.
- Copy legacy test state and dependency files into a disposable target, make its
  old fixture root unavailable, bind explicitly and verify unchanged provenance and
  approvals. Real approved runs are never resumed or rewritten. Hash original local
  evidence before/after any read-only compatibility checks.

## Decisions for review and exact approval scope

Already settled: wheel distribution, pipx installation, later GitHub Release assets,
Herdr-only integration, simulated acceptance, sequential operation and immutable history.

| Remaining choice | Recommended default | Tradeoff |
| --- | --- | --- |
| Runtime-state default | Project-local `.wayward-wrangler/state`, external override allowed | Visible ownership and simple isolation; a home-based store could avoid writing under target projects but needs more location management. |
| Unqualified legacy commands | Require `--project`; preserve `awp` as a true alias | Explicit selection prevents wrong-project actions; old shell commands need one added option. |
| Plugin/location index | Small user-local index and explicit installation command; test only in isolated Herdr first | Enables project-qualified click routing without embedding filesystem paths in links; adds location metadata to maintain, not another context service. |

Approval can accept those defaults together or change them; no additional product
choice is needed before this proposal can be reviewed. The next implementation
scope is steps 1–5, their offline/isolated simulated checks, and updated installation/
recovery documentation. It does **not** include switching the user's shared plugin,
GitHub publication, moving this checkout, native model trials, provider additions,
new workflows, concurrent scheduling, automatic discovery, orphan recovery, NAS work,
public PyPI or OS packages. Those actions require separate scope/authorization.

Main technical risk: separating live location binding from frozen context/adoption
verification without weakening integrity checks, while keeping project-qualified
routing consistent across the controller, runner and plugin. Fresh wheel installation,
plugin registration and source-inaccessible execution remain unproven until this
proposal is approved and the acceptance checks run.
