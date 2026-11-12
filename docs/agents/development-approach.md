# Acceptance-driven development

Accepted working approach. [AGENTS.md](../../AGENTS.md) owns the operational policy;
the [usability plan](usability-plan.md) owns the product target. This approach
applies to subsequent implementation, maintenance and bug fixes.

## Target

Deliver one complete user journey at a time. The current journey is:

**Open a project → do useful work → close it → rebuild Waywr → reopen → continue.**

TDD drives correctness inside that journey. Ordinary use determines whether the
experience is fluid. Passing internal checks alone does not establish usefulness.
Documentation-only work uses reference/diff checks, not manufactured red tests.

The [narrowed product direction](usability-plan.md#product-order)
sets the next order: extensible execution/model selection, lifecycle fluidity,
then coordination through a substantial real application. Development constraints
are not runtime agent capability limits. Evaluate restrictions against concrete
failure boundaries; do not equate model-independent architecture with equal model
quality or make a model-specific coordinator to pass the reference workload.
Model support must remain broad throughout the project: use the
[shared layer](../execution-adapter-decision.md), keep model catalogs/options out of
coordinator code, and demonstrate multiple provider identities through the same
path. A model list or single-client wrapper is not completion. Existing hardcoded
integration/profile compatibility is waived; preserving project work remains
required. Pi's application is a reference; the selected independent libraries supply the shared runtime.

## Slice agreement

Before implementation, record four short items in the plan or handoff:

1. What the user will be able to do.
2. What currently prevents it.
3. The acceptance example that will demonstrate the improvement.
4. What is outside the slice.

Keep one slice in progress. Complete necessary implementation decisions without
routine confirmation. Surface a substantive scope change and its dependency;
do not convert an adjacent discovery into an additional workstream.

## Red, green, use and refine

Write the behavior check first and run it against the current application.
Establish a failure caused by the missing behavior. Repair a broken harness
before counting it as red. When behavior already works, characterize it and
identify the actual missing outcome rather than forcing an artificial failure.

Implement the smallest complete change. Add focused lower-level red/green tests
for nontrivial logic and concrete risks as they emerge. Do not mirror private
functions or weaken an assertion merely to get green. Refactor with the behavioral
checks passing. Existing meaningful tests remain; avoid repetitive broad runs.

Use actual application components at the boundary being promised. A fixture can
isolate download transport or a provider response, but its scope must be stated.
A mocked service manager cannot prove actual systemd bootstrap; synthetic model
output cannot prove native model reliability. Acceptance checks belong in isolated
projects and must not use operator state or authenticated calls implicitly.

Exercise the relevant ordinary workflow when separately authorized. Record manual
intervention and confusing or repetitive steps. Turn reproducible failures into
regressions before fixing them; use human judgment for presentation and fluidity
instead of claiming that text assertions prove good UX.

## Completion and evidence

A slice is complete when its promised normal action works, its relevant safety
and persistence checks pass, and remaining limits are explicit. Do not report the
whole daily-use milestone complete because an isolated slice passes.

At each checkpoint record:

- What works through the normal user entry point and what still requires help.
- The observed initial failure and the final behavioral evidence, including any
  fixture boundary and whether ordinary use has occurred.
- The smallest next action within the accepted target and any concrete blocker.

An intermediate red-test checkpoint is valid evidence of work begun, not a claim
that the behavior is delivered. Keep it explicit and continue authorized work
toward green; do not substitute a test-only report for the promised outcome.

## Review and authority

The agent inspects Git read-only for routine tasks, stages owned changes and makes
local commits on the current branch. The user reviews and performs all remote
delivery. No pushing, remote PR changes/merges, release publication or hosted
settings changes are authorized by this approach. Do not mutate unrelated work or
history. See AGENTS.md for the full boundary.

Local application implementation and isolated acceptance checks are authorized.
The earlier no-install instruction still applies to the operator environment;
the preserved project is not a test fixture. Installing a daily-use candidate or
making an authenticated model call requires explicit task authorization. A
document, mock fixture or passing test cannot supply that authority.
