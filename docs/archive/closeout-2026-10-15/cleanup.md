> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Repository and runtime reset — 2026-10-15

## Scope and historical outcome

User explicitly authorized discarding all old disposable Waywr evaluation state,
transcripts, decisions, receipts, artifacts, installations and generated resources,
superseding their earlier preservation rules. No replacement backup was requested.
Committed source, Git history, architecture and historical reports remain.
Starting branch `feat/live-task-proposal-authorization`, clean at
`6f7bbcf0632a4c3922cec4b8a4f17b258472fd3e`; this closeout changes documentation,
ignore-comment wording and one moved-document path in an evaluator diagnostic only.
No application behavior, schema, dependencies or provider profile changed.

Latest operator feedback: the current evaluation seemed okay and workflow felt
better. This is broad feedback; no new specific test outcomes or complete native
acceptance is inferred. Separately, the existing v8 summary/post-exit audit recorded
four completed Claude calls/four matching receipts, two proposal revisions and two
exact human authorizations, zero workers/consultations/admissions, then paused
checkpoint36 and inactive service. Earlier physical detach/re-attach was operator-
reported. No working generated application was verified. Original report retains
its dated observations; this reset does not expand their scope. No calls/tests were
made to reproduce or extend them. Detailed runtime evidence was then deleted.

## Ownership and shutdown

Inspected process executables, argv, ancestry/cgroups, unit properties, saved
project identity/release bindings and fixture records. No process was selected
solely by a broad name match. Four running supervisors had no runner, worker or
TUI descendants; supported systemd stop/SIGTERM completed within the bounded wait.
No KILL escalation was needed.

| Verified unit | Disposition |
| --- | --- |
| `waywr-<private-id>.service` | Python lifecycle operator-02 supervisor stopped |
| `waywr-<private-id>.service` | Python unified `/tmp/waywr-unified.VTkzZC` supervisor stopped |
| `waywr-<private-id>.service` | Rust direct-start fixture supervisor stopped |
| `waywr-<private-id>.service` | Rust sim-eval supervisor stopped; old uncertain record was not replayed/reclassified |
| 158 completed/failed empty operation units | Exact executable paths verified; stopped/reset-failed and unloaded |
| `wayward-wrangler-0c2d2965-decision-log-01.service` | Already inactive; disabled, exact user definition and enablement link removed |

Total 163 identified units unloaded; `daemon-reload` completed. No associated
Waywr timer/socket/path activator was found. Post-cleanup process/unit scans found
no identified Waywr execution or enabled restart definition. No system-wide unit
referencing this checkout was found. No repo-mounted Docker container remained.

## Removed

- Repository-local `.local/` (~21GiB before cleanup), `.venv/` (~110MiB), `target/`
  (~15GiB), Python `build/`, disposable `temp/project-decision-log/`, two generated
  egg-info directories and six `__pycache__` directories. No tracked file was inside
  a deleted root. No retained candidate/launcher or evaluation project remains.
- 365 exact verified external roots (including 24 lifecycle fixtures found in the final identity-marker scan): old paired Python/Rust installations, wheels,
  isolated fixture homes/runtime directories, evaluation projects, private Herdr
  proof data, source-inspection copies and task helper/staging files. Ownership used
  project markers/bindings, manifests/package contents, unique source references or
  a documented fixture prefix **plus** its private layout, never a name alone.
  Included `~/projects/waywr-rust-sim-eval` (only Waywr state/ignore)
  and `/tmp/waywr-unified.VTkzZC`; unrelated project directories were not removed.
- Nine verified obsolete entries plus lock/index in the dedicated
  `~/.local/state/wayward-wrangler/` locator; its now-empty directory removed.
  Five remaining non-listening project sockets and empty `/run/user/1000/waywr/`
  removed after original identity verification. Other stopped sockets cleared
  through their supported shutdown. Deleted fixture roots include their private locks.
- Root-owned `/tmp/ww-m4-container-evidence` initially refused ordinary deletion.
  Noninteractive sudo was unavailable. Existing local image `df18428df8a0`
  (`python:3.12-slim`) removed only that bind-mounted fixture's contents, with no
  network, read-only container root, no-new-privileges and no image download.
  The ephemeral cleanup helper exited/was removed; no test or service was launched.

No broad deletion globs, database rewriting, historical path replacement or new
backup/evidence archive. Filesystem disposal cannot be undone by Git rollback.

## Deliberately retained / attention

- Shared Herdr/tmux servers, unrelated provider/development-agent sessions, Ollama,
  credentials, model files, permanent rustup and user-wide directories unchanged.
  Old temporary toolchain/cache `/tmp/waywr-cargo`, `/tmp/waywr-rustup` and
  `/tmp/waywr-rustup-init` are retained as compiler tooling, not app runtime.
- Legacy `~/.local/share/wayward-wrangler/herdr/` (three small integration files)
  is referenced by the shared Herdr plugin registry (`local.agent-workflow-review`).
  Retained to honor the no-shared-Herdr-change boundary. It has no automatic launch
  path; explicit old review links report missing installation after deletion. Removing
  the registration/files together would require a separate shared-integration action.
- No Waywr TUI was attached. Two ordinary Fish shells were inside deleted evaluation
  directories (PIDs 3197517 and 4036699 at inspection). Preserved as user shells; use
  `cd ~` in those terminals. No raw-mode terminal restoration was required.
- The following inert `/tmp` scratch items were not sufficiently tied to disposable
  Waywr ownership/dependencies; retained rather than guessing. None was identified
  as an active Waywr process, service or bound project. The symlink targets the
  shared keyring SSH socket and was not followed/removed:

- `/tmp/waywr-codex-app-server-thread-0.162.1.rs`
- `/tmp/waywr-codex-models-0.162.1.json`
- `/tmp/waywr-codex-spec-plan-0.162.1.rs`
- `/tmp/waywr-codex-tool-policy-0.162.1.rs`
- `/tmp/waywr-codex-tree-0.162.1.json`
- `/tmp/waywr-herdr-api-schema.json`
- `/tmp/waywr-herdr-debug-8lwt37va`
- `/tmp/waywr-herdr-schema.json`
- `/tmp/waywr-lifecycle.iXQUcc`
- `/tmp/waywr-m6-herdr.sock.agent`
- `/tmp/waywr-prototype-rustfmt.log`
- `/tmp/waywr-refinement-rustfmt.log`
- `/tmp/ww-m3-api-schema.json`
- `/tmp/ww-m4-accept-01`
- `/tmp/ww-m4-pipx-bootstrap`
- `/tmp/ww-m4-requirements.lock.txt`
- `/tmp/ww-m6-herdr-schema.json`

## Source cleanup left separate

Tracked `src/agent_workflow/`, `pyproject.toml`, `requirements.lock.txt` and
`packaging/*.py` are legacy Python application/package material, not requirements
for the current Rust runtime. Do not run `pip install .` as the Rust setup path.
Rust bundling still copies Markdown skill resources from that tree; removing it
needs a small source/resource/reference cleanup, including legacy test disposition.
`tests/*installed.py`, boundary/lifecycle fixtures and `scripts/*evaluator.py` also
include useful developer tooling (many use standard-library Python); inspect each
harness's prerequisites when relevant. Legacy `tests/test_*.py` may import the old
application/dependencies. No replacement venv was created or requested.

## Documentation / verification

Active path is AGENTS → concise HANDOFF → architecture; root README owns use/build.
Seven precise maintenance/compatibility contracts are grouped under `docs/contracts/`.
Completed continuous/scope reports, readiness assessments and authorization/
walkthrough material are archived. Pre-reset active handoff and entrypoint snapshots
are in this dated group. Historical quoted evidence paths intentionally remain;
they are no longer runnable instructions or a renewed allowance.

Verification is ownership/process/unit inspection, generated-root absence, permanent
compiler selection, moved/current link checks and final Git diff review. No rebuild,
application suite, model call, service startup, new project or next implementation
plan. The next objective is in HANDOFF, for a fresh context and separate direction.
