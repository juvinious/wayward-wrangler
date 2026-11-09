# Persisted state and local protocol, milestone 6

This describes the current records for compatibility tests. The Rust client uses
`protocol/session-v1.schema.json`: newline-delimited UTF-8 JSON over a same-UID
Unix socket, one envelope `{v, kind, request_id, body}` per line, protocol major
1, 1 MiB frame ceiling, 64 KiB command ceiling. A hello binds project key,
release ID and channel. The service rejects a mismatched pair. Command IDs are
idempotency keys; replay of the exact command returns its saved result and reuse
with different bytes is refused. Task version and state revision reject stale
mutations. `events_read` is cursor-based, at most 64 events per response. Events
notify clients; they do not replace the authoritative workflow records.

Per-project `.wayward-wrangler/project.toml` holds a stable project key;
`release.json` holds release ID, manifest SHA-256 and binding generation.
Runtime state defaults to `.wayward-wrangler/state/`, independently for each
project. `state.sqlite` has additive `session_*` tables: commands store exact
requests/results, tasks store version/status/run binding, coordinator calls and
proposals store frozen packet/result hashes, admissions store the frozen plan,
and events have monotonic sequence and `schema_version=1`. A partial release
change has an immutable intent under `session/release-bindings/`; its complete
record is appended after the descriptor update. Status exposes an incomplete
binding. Only the selected quiescent project can change release. The installer
never rewrites project state, and releases still referenced by projects are
retained; release deletion is not implemented.

For the corrected Rust release, `waywr init` writes the project key, workflow
configuration, Herdr endpoint and release binding under the selected project's
`.wayward-wrangler/`, plus `state/project-identity.json` and the project's
`.gitignore` rule. Default initialization writes no user-local location index,
starts no service and creates no runtime socket or Herdr tab. An explicit
`--state-dir` can put authoritative state elsewhere by operator choice. First
service attachment records a project-key → project/state path in
`${XDG_STATE_HOME:-$HOME/.local/state}/wayward-wrangler/locations.json` (or
`--registry-dir`) for key-only review links. That external locator contains no
task history, decisions, artifacts or execution receipts and is not a backup
of project state. See [storage paths](stale-registration-reinit.md#storage-paths-and-intent).

The existing run ledger, LangGraph checkpoint, artifacts and receipts remain
authoritative for stage progress, exact output bytes and human gates. Artifacts
and source findings carry SHA-256 references. The approved artifact SHA-256 and
request ID bind an answer to the reviewed candidate. Execution intents,
Herdr routing, supervisor generations, recovery actions, context grants and
revocations are append-only records with `schema_version=1`; current routing
and display pointers are projections of those records. Unresolved execution
never becomes a fresh dispatch merely because a controller or service exits.
Finding origins and later assessments are distinct; omitted dispositions remain
unresolved. New workflow profiles may set `workflow_version=2` and
`max_questions` from 1 to 4; older profiles retain their original interpretation.

Cross-project grants belong to the recipient task and record source project key,
selected run/object identity, record/version hashes, purpose, dependency mode,
budget and revocation. The model-facing packet contains selected material and
provenance only. A grant gives read-only context, never authority over the source
project. `pinned` imports retain frozen bytes; `must_match_source` imports
check the selected source version at later dispatch. Revocation blocks future
delivery, while previously delivered packets remain evidence.

Historical records can lack these new fields. Their original bytes and
interpretation remain fixed; no schema conversion of active LangGraph
checkpoints occurs. A future backend replacement must reproduce externally
visible commands, receipts, hashes, decisions, failure states and review binding
before serving new runs. Python/LangGraph serialization and historical hash
canon are migration constraints, not the public IPC format.
