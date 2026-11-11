# Significant real-workflow acceptance

Audience: agents preparing/observing the first release readiness run. The user
requires one significant proposal → coordinator → workers → reviewed working
outcome before release. The concrete [Workboard brief](../../examples/release-acceptance-brief.md)
is proposed; it is not an execution receipt or an approved application scope.

## Slice

- **Outcome:** Waywr delivers a working, nontrivial full-stack application through
  its ordinary user entry point and local completion flow.
- **Present gap:** isolated mechanisms and completed conversation calls have been
  checked; useful end-to-end delivery through the shared runtime is unproven.
- **Acceptance:** an actual coordinator proposes, the human approves the presented
  scope/policy, real workers implement, exact reviews/checks run, the human inspects
  the candidate and Waywr completes local delivery. Verify the resulting app.
- **Exclusions:** model-response fixtures, agents outside Waywr doing its assigned
  application work, operator project changes, cloud deployment, remote Git writes,
  global installations and an all-provider benchmark/fault-injection campaign.

## Prepare before calls

1. Build a fresh remapped paired candidate from the recorded Waywr source commit;
   do not reuse the old ignored local-bin candidate without checking its identity.
2. Create a separate local test Git repository with a committed specification and
   no application solution. Preserve its source and execution evidence. Record
   candidate manifest hash and project baseline in private local run metadata.
3. Obtain the user's role/model choices and explicit permission to use existing
   provider authentication for this run. Do not read/copy credential bodies or
   another project's context. Any usage limit is the user's, not a new product cap.
4. In normal setup select committed HEAD, local main delivery and required human
   validation. Approve finite check/build recipes `["sh","scripts/check.sh"]` and
   `["sh","scripts/build.sh"]` with the scope; these need not exist at initialization.
   The coordinator/workers must create them. Source validation permits explicit
   bounded recipes without pretending a blank project already has a package manifest.
5. Show the actual proposal/model/context/policy card for human approval. Draft
   preparation does not permit an agent to fabricate this application's human assent.

## Run one ordinary journey

Use the real TUI/service/runner/shared SDK path, not direct calls to a model or a
hand-built operation fixture. Let the coordinator decide the task graph and actual
review/rework sequence; require at least two meaningful worker assignments rather
than a predetermined number of calls. Provider/model IDs remain user choices.
Use two selected models/providers where configured and approved, but do not imply
that one successful selection establishes broad model quality.

During work, detach once and reattach to the same owned service. Preserve a draft
and confirm work continues without duplicate dispatch. Send one clarification
already within scope (for example URL-persisted filters and matching CSV export).
Do not inject crashes or force a manufactured bad review just to add coverage.

At candidate presentation, use normal checkout/feedback/accept controls. Real
findings return through candidate feedback or reviewer findings, then get rechecked
at the resulting exact revision. A human performs browser validation and explicitly
accepts the candidate; accepted local delivery must point to that checked result.

## Pass and failure recording

Pass requires useful running software, exact reviewed/check receipts and local
delivery, plus responsive drafting/inspection/leave/return throughout. Expected
human approvals and real clarification questions are ordinary interaction. Normal
code review/rework is allowed; direct source implementation by this outside agent,
manual state surgery, fake receipts or forced status changes invalidate the run.

A crash, unrecoverable block, lost input/ownership, duplicate execution, endless
no-progress loop or inability to deliver is a recorded Waywr failure, not a passing
fixture. Keep the first reproduction and exact candidate identity. Convert a
reproducible product defect to a focused regression before fixing it. Continue from
preserved work when valid, but record interventions; do not describe a repaired or
partly simulated run as an uninterrupted pass. Finish a successful normal journey
before proposing a release. No silent reset of source/evidence to improve results.

Record only the necessary run report: Waywr/app commits, selected role models,
request/task/receipt/check/candidate references, human decisions, measured usage
where reported, observed intervention, app acceptance results and explicit limits.
Raw runtime paths/transcripts/credentials stay private and outside Git. No result
is claimed until this actual journey has completed.
