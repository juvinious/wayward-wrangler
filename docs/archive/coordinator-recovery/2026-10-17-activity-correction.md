> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Activity outcome/caption correction — 2026-10-17

The user observed a red “Failed · Review reviewed” while the native reviewer
reported no findings. Read-only comparison at Example Task App event1041, workflow
`<private-record>`, confirmed status `reviewed`, completed receipt, zero
findings and both native/effective `requires_changes:false`. The model summary's
“failed-save preservation” matched the frontend's generic substring classifier.
This was a display defect, not a native review failure or finding-retention error.
No native result, reviewer judgment, request or original event was changed.

Source checkpoint `c3921b3347cbe5178086e188f255de3a166984ee`:

- Development labels derive from event type and recorded review/check/unit status,
  not words in summaries or task titles. Review complete, needs changes, stale and
  failed remain distinct; a completed review does not assert combined checks.
- Optional typed metadata preserves older UI journals. Replaying the exact event
  refreshes an older cache without message duplication or authority changes;
  untyped legacy review prose remains neutral.
- A topic started with `/new` shows its bound scope once saved, replacing the stale
  “no selected scope” caption. This changes neither destination nor frozen drafts.

Focused `cargo test --locked -p waywr --lib -- activity_ development_events_explain
recovery_`: 13 passed (four added regression cases plus nine related existing tests).
They cover misleading prose, actual failing/blocking/stale outcomes, old cache
rehydration and deduplication, caption/routing invariance, chat/drafts and worker rows.
Documentation links and diff checks passed. No backend behavior changed.

Permanent Rust1.99.0 locked release build succeeded for all three production binaries.
Paired bundle `target/waywr-bundles/activity-20261017-c3921b3`, all 13 hashes verified.
Manifest SHA256 `98d56460a910508383ce53914c308a2325767835f0c17d61258b01720d7b01c2`.
Frontend SHA256 `38159c571b4fb72817e8cb230210f8f2bf883a90b5acadbd9938602331733958`;
service/runner hashes match the previous [recovery build](2026-10-17-build-and-verification.md).

## Installation and preserved request

The user authorized the guarded update by detaching after the concrete build/checks.
The bundle was installed alongside the retained recovery release and selected as the
default while Example Task App remained bound to its old release. No running service or
native execution was changed. Installation and binding were deliberately separate.

The user-originated TUI request continued through repair workers, review and combined
checks. Its final checked integration commit is `7112b716ff98408fa096c217ca319f221bb57167`.
The coordinator stopped for the explicit **human visual smoke** obligation (source
`coordinator-79-ctx3d3e3622cb72`); it did not fabricate that result or deliver main.
The question asks about prompt visibility, wide/combining text, highlighting, feedback
and terminal restoration. Esc latency and incremental-output evidence remain unverified.
No caller, worker or check was running/pending when maintenance began.

Supported stop checkpointed generation44/revision1455 to **paused generation45,
revision1457**, events1456–1457. After the service exited, supported `session upgrade`
bound Example Task App to **activity-20261017-c3921b3**, binding32, at that same revision.
Installed/default/project manifest hashes match; all 13 installed files were verified.
Normal `~/.local/bin/waywr` resolves to that exact paired release. Service remains
inactive; no dispatch resume was performed.

Before/after preservation checks passed across 14 conversation/coordination/approval/
development tables, 1,882 immutable evidence files, four readable history prefixes,
existing events through1455, project identity/provider config and Git refs/worktrees.
The saved human question and checked integration commit are unchanged. Main remains
clean at `af358d7560d85f14d56aff481e6f34c5bd6c22a9`; the TUI feature is **not delivered**
while visual acceptance remains open. The implementing agent supplied no request,
approval, answer, retask, authenticated call or Example Task App code edit. No push or merge.

Ordinary use now supplies a limited native observation: the coordinator adapted an
unlaunched review-target preparation refusal and obtained a validated linked
clarification (nine advisory dispositions; native/effective requires_changes false).
The original review and refusal remain retained. This does not establish autonomous
recovery across other faults or complete visual acceptance, and is distinct from the
13 focused automated display checks. The corrected display has not yet been
human-confirmed in the reopened TUI.

```fish
waywr --project ~/projects/example-task-app
```

The session opens paused. The user controls resuming and answering the saved visual
smoke question through Waywr; there is no unchanged-scope approval to repeat.
