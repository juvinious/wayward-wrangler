> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Context correction installation — 2026-10-17

Historical implementation/recovery evidence; current semantics belong to
[context delivery](../../context-delivery.md), readiness to [HANDOFF](../../HANDOFF.md).
No new request or human approval was submitted for this correction.

## Source and preserved authority

Branch `impl/conversational-coordination`, stacked on provider selection.
Commits: `8e47b38` compact/context retrieval, `f8283b7` immutable cursor/coverage,
`91a11f5` actual host preparation boundary, `8d07500` active read set/factual memory.
The original universal 98304-byte packet ceiling was application policy, not a
Codex or Claude model limit. Soft preference, application memory/transport bounds
and model/headroom estimates now have distinct meanings; exact native tokenizer
and negotiated capacity remain unknown.

Existing Example Task App request `<private-record>`, task
`<private-record>`, retained human-approved v3 SHA
`8b1ef61b4f2f5dc3cc21e7a0477589aa1dca3b72b5f018e5090c83fb38bafaa4`.
Events 655–661 record the human's exact typed approval. Codex coordinator and
implementation, Claude reviewer pins remain enforced; fallback remains disabled.
Implementation `da98c4ff6e50d29ad9bef15fdc81a5be3d7228ff` and three completed
older Claude reviews remain intact. Older reviews are evidence, not blanket v3
acceptance. No application code was edited by the implementing agent.

## Refusals and continuation

Event 663 refused 104008 bytes before host preparation/reservation, pending null,
last completed sequence 9. No coordinator-10 attempt had launched. Guarded release
1 upgrade followed normal drain 664/665; supported recovery 666/667, resume 668.
Actual native coordinator-10 completed and requested evidence at 670.

Continuation 11 exposed another legacy check in `operation::prepare_host`, before
host preparation/attempt. Release 2 fixed this; drain 674/675, recovery 676/677 and
resume 678. Actual Codex turns then consumed compact packets up to 122117 bytes,
exact diff/source pages and immutable record snapshots. They repeatedly requested
prior pages because v1 retained only the newest batch and narrated intentions.

Release 3 retains the active read set until a decision, public factual notes
separate from narration, explicit hash-based raw-page release, and legacy frozen
result validation. Normal drain 724–726 let coordinator-22 complete successfully:
receipt attempt `<private-id>`, exit 0. Its pending adoption
was preserved. Session resume 727 adopted that result at 728–730, once, then fresh
coordinator-23 used 18 retained pages (213515 raw bytes; 294412 compact packet
bytes). No completed call was replayed and unchanged scope was not reapproved.
Coordinator-23 and later assignment decisions exposed a separate recovery defect:
the write-path gate treated the superseded original implementation as an active
owner. It rejected disjoint current work because the retained old assignment owned
broad project paths. The correction releases only terminal superseded ownership;
running supersede-pending attempts remain fenced. Rejected decisions retain active
reads for automatic replanning. Normal drain 758–760 preserved coordinator-31's
successful completed assignment result for adoption under the corrected scheduler.
Release 4 (code `9109572`) upgraded the paused project to binding 28, preserving
all original rows in 46 tables, 4009 immutable evidence/config/log prefixes, exact
pending result and application Git. All 13 installed files verified; manifest SHA
`c29c05983cbefa485b6fe21cfcf27d1036b51422c54116df1b0951476e11070d`.
Preservation/comparison records use the same `/tmp/waywr-context-delivery4-*` names
as release 3's records below. Resume 761 adopted coordinator-31 once; events
762–766 planned and reserved its Codex `complete-tag-model` assignment. The original
approval/reviewer pins remained identical. No application-specific sequence was
supplied by external code. Inspection then distinguished reservation from actual
launch: there was no host attempt, because `1792258300.7197313` reparsed as
`1792258300.719731`, invalidating the frozen packet hash. A pure-data regression
reproduced the failure. Enabling the existing serde_json `float_roundtrip` feature
(no package/lock upgrade or canonical encoder change) preserves original bytes/hash.
A repeated deterministic hash mismatch now surfaces a preparation refusal instead
of being silently retried. Normal drain 770–771 leaves the reserved worker packet
unchanged for launch after the corrected parser is installed. Further outcomes are recorded in actual request history
and the current handoff.

## Stable installation and preservation

Release 3 used the normal user-local prefix `~/.local/share/wayward-wrangler`, launcher
`~/.local/bin/waywr`, project binding generation 27:
`context-20261017-3`, source `8d07500`, manifest SHA
`b9c6671594330f944f8f877b6270b39a2d59f957b16bb996589b866b72541b6d`.
All 13 bundle files hash-verified. Guarded upgrade required detached view, paused
session, inactive service and positively settled host attempts. Prior releases
retained; shared provider config/authentication unchanged.

Read-only before/after snapshots `/tmp/waywr-context-delivery3-preservation.json`
and `/tmp/waywr-context-delivery3-post-install.json` verified all original rows in
46 tables, all 3714 immutable evidence/config/log prefixes, exact workflow and
application Git HEAD/refs/worktrees/status unchanged across installation.
`/tmp/waywr-context-delivery3-install-verification.json` records the comparison.
Earlier release 1/2 preservation snapshots and verification files remain in `/tmp`;
these are local verification records, not committed runtime state.

Actual readable history:
`~/projects/example-task-app/.waywr/history/20261017T060719Z-9031376ba7ac.log`.
It contains conversation, approvals, assignments, outcomes and retrieval/recovery
entries, with the continuous summary beside it; it is not merely a DB record index.

Release 5, code `690fdd9`, is the stable default and project binding 29:
`context-20261017-5`, manifest SHA
`62b4f6c29809a662959186756230857a71bddce1f3f63cff3026017d5e8b726d`.
Before/after `/tmp/waywr-context-delivery5-*` records verify all original rows in
46 tables, 4048 immutable evidence/config/log prefixes, workflow, Git and 13 bundle
hashes preserved. The real worker packet now parses to its original SHA
`5109cda13e7f7e2f6850e344b4a14ef7bc5e094ea43d212376fd4c63ee181910`.
Session resume 772 launched its first host attempt
`<private-id>`; native child and 55KiB+ native command stream
positively observed. This distinguishes prior running/reserved display from actual
execution. No old packet was rewritten and no model call was replayed.
The worker then submitted `33e2ab8cfbc15db3401461a31d6369826030780f`.
The live coordinator created its Claude `review-completed-tag-model` assignment
(events 788–791), whose first native child was positively observed. Its 38959-byte
compact input targets exactly that commit and supplies the full 9817-byte diff,
range `[0,9817]`, `complete:true`. This verifies actual participating role selection
and frozen evidence delivery. The review subsequently completed with
`requires_changes:false`, exact reviewed commit `33e2ab8...`, and explicit caveats:
no tests run by the reviewer, imports not inspected beyond the supplied diff, and
CLI/persistence/combined checks still pending. This does not claim final delivery.

## Focused checks and limits

Context tests 8 pass: authority/findings with >96KiB mandatory input through actual
host preparation and identity creation (no launch), immutable exact-revision paging, missing-diff gate,
existing approval recovery, once-only service conversation continuation, old snapshot
cursor/range receipts, v1 compatibility and active reads/release. Compile, locked
three-binary release build, changed-document links and diff whitespace checks pass.
Exact timestamp round-trip regression 1 also passes (failed before the repair).
Recovery tests 20 also pass, including superseded ownership and running-writer
fencing, atomic adoption and provider/lifecycle recovery. Earlier changed-path checks: coordination 9, role pins 1, profile freeze 1, concurrent
worker scheduling 1, Claude adapter 2 and Inspect UI 1; no unrelated acceptance rerun.

Read delivery establishes bytes supplied, not comprehension or exhaustive review.
4 MiB packet/known-input memory, 32 MiB evidence reader and 16 KiB page bounds are
application safeguards, not provider context capacities. Estimates/headroom are
honestly uncertain; large required input refuses explicitly. Native-client later
worker tool accumulation remains client-managed. Actual Claude review completion with a complete diff is observed. Claude paged
continuation and full v3 delivery remain unverified. Existing provider/sandbox
portability and uncertain-execution reconciliation limitations remain.

## Subsequent delivered checkpoint

The same approved request subsequently delivered `af358d7...` with recorded exact
combined checks and all 40 tracked obligations resolved. Claude's CLI review used
a hash-verified exact-source retrieval and fresh review continuation; the human later
verified an ordinary recap through the production TUI. The
[verified work/tasking summary](../conversational-coordination/2026-10-17-verified-checkpoint.md)
records these later observations separately from the installation-time limits above.
