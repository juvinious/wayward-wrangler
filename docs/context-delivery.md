# Context delivery

State9's `evidence-v1` release contract supplies compact working summaries and an
application-owned evidence index. Canonical packet hashes and old serialized files
retain their original encoding; new native prompts use compact JSON. Exact JSON
floating-point round trips preserve timestamps and their original frozen hashes;
existing immutable packets are never normalized or rewritten. The full
current requirements, one approved policy, frozen execution selection, unresolved
findings and relevant execution state remain mandatory. Repeated policy/routing,
completed reports and resolved obligation rationale become concise outcomes and
exact evidence references. Existing approvals and provider pins are unchanged.
Relevant submitted/review-target diff pages are supplied initially when they fit
the soft working target; explicit retrieval supplies the remaining or other details.

`context_bytes` is a soft working-set preference (legacy values remain valid), not
a model capacity or authority grant. Mandatory input and newly requested pages may
exceed that target. The independent application packet memory bound is 4 MiB;
evidence objects have a 32 MiB reader bound and pages carry at most 16 KiB raw.
Oversized required material refuses explicitly, never silently truncates.
TUI IPC keeps its separate frame/command bounds; model packets travel through
project-owned files, not that IPC frame.

Capacity diagnostics include instructions and output schema plus reserved response,
tool-result and native-system headroom. Published capacities are sourced from the
[exact Codex model](https://developers.openai.com/api/docs/models/gpt-6.1-sol) and
[exact Claude model](https://platform.claude.com/docs/en/models/sonnet-5-5/overview).
These API specifications do not negotiate account/CLI effective capacity. The
native transports currently expose no verified exact tokenizer/counting interface;
Unicode-character estimates are labelled heuristic with high uncertainty, never
byte/token equivalence. Actual native receipt usage is retained separately. Unknown
models/capacities stay unknown. Later worker tool traffic is native-client managed;
headroom is an application planning reserve, not a guarantee of unlimited context.

## Read-only retrieval and continuation

Development coordinators return `action: retrieve` with `evidence_requests`;
conversation coordinators use `retrieve_evidence` in their existing action envelope.
Reviewers use `evidence_requests` with no reviewed commits until final review.
Each request names an indexed evidence ID, an optional tracked source path and an
empty initial or exact returned continuation cursor. Requests are bounded per read
batch, without imposing a task/call/repair count ceiling. Workers retain their
existing assigned-worktree tools and result contract.

The service validates the frozen project/request/scope/manifest identity, then reads:

- Git source/tree blobs and complete diffs tied to exact full commit IDs. Review
  source grants include only its target bases/commits. Paths cannot traverse the
  filesystem or read Waywr/Git state; Git symlink blobs are data, never followed.
- Content-addressed immutable snapshots of task/workflow records, combined checks,
  hash-verified captured check output, accepted knowledge and relevant request history.
  An unpublished readable log uses the existing redacted journal projection rules.

Pages identify content hash, range, total bytes, encoding, next cursor and completeness.
Snapshot cursors cannot silently switch to newer record/history bytes. Unavailable
reads return explicit no-bytes/error pages; they never fall back to mutable files.
The same project/provider grants apply, with no new filesystem tools, writes by
reviewers, arbitrary paths, remote endpoints or routine permission prompts.
Delivery ranges merge in working context; immutable exchange references retain the
complete audit chain. A previous snapshot remains available for its continuation
cursor even when a newer record/history snapshot is indexed.

A validated completed retrieval result saves an immutable exchange and atomically
records its continuation. Working-memory v2 retains the deduplicated active read set
across retrieval turns, with a public factual `context_summary` separate from user
narration. Coordinators/reviewers can explicitly release supplied content hashes
after summarizing useful facts; new reads are added after release. An accepted coordinator
execution decision ends the raw read set while retaining its factual notes, immutable
exchanges, verbatim unresolved findings and delivery receipts. Rejected decisions
retain the active read set for automatic correction. Older v1 read chains
are reconstructed from hash-verified exchanges without replaying calls. Full raw
pages stay outside SQLite workflow records. Working-memory-v2 release guards prevent
downgrades that would discard this behavior; frozen older result schemas remain valid. Conversation
continuations are labelled service input, not invented human messages or approvals.
Review continuations use fresh attempt revisions; completed calls are never replayed.
Interrupted/uncertain execution retains existing reconciliation fences.
Coverage from an older scope or requirement generation cannot satisfy a current review.

New review packets carry exact finding lineage and selected-reviewer dispositions;
[recovery](coordinator-recovery.md#finding-lineage) owns validation and linked
clarification. Unaddressed concerns stay blocking, while supplied exact evidence and
a valid linked judgment can clear the applicable concern without a whole review.
Earlier frozen review schemas preserve their original string-retention behavior.
The [historical diagnosis](archive/coordinator-recovery/approved-plan-d21271f.md#correction-to-the-review-diagnosis)
records that Waywr restored a provisional concern despite nonblocking native output;
original results and Example Task App delivery are not rewritten.

Final reviews and coordinator integration require full relevant diff ranges to have
been supplied across frozen inputs. Service-recorded coverage identifies exact bytes
and commits, not comprehension or exhaustive source review. Unresolved concrete findings
survive retrieval and remain blocking. Current requirements/policy and exact-commit
review/combined-check gates remain authoritative.

Inspect shows role/profile, section bytes, instructions/schema, estimates, headroom,
omissions, retrieval availability and the retained diagnostic file. Frozen packets,
exchange files and public metadata record what was supplied/retrieved. Raw source
pages are not duplicated into public activity events. Compatible-release guards
preserve retained continuation records; historical packets/receipts are not rewritten.

New finding-contract review packets preserve original native output separately from
the effective disposition. Exact linked judgments can resolve prior concerns using
supplied source evidence; omitted judgments remain blocking. See
[recovery/finding lineage](coordinator-recovery.md#finding-lineage). Old frozen review
contracts remain valid and accepted history is not rewritten.
