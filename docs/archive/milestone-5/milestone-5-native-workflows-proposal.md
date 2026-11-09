# Milestone 5 proposal — installed native drafting workflow

**Status:** approved for implementation and the single bounded trial in a later
operator turn; see [observed results](milestone-5-report.md). Branch
`milestone-5-native-workflows` starts at merged PR #2 (`1fc74c6`). At proposal
time this document authorized no model call, plugin switch, application build
or release. The disposable target directory is ignored by this repository.
The later authorization selected the recommended models, made the plan digest
internal for interactive confirmation, and clarified that a crash may end in
explicit uncertain state without a fabricated completion receipt.

## Goal and boundary

Use the **installed wheel**, from a working directory outside this checkout, to
draft and approve an actionable specification for **Project Decision Log**: a
local browser application to create decisions with title, rationale and
alternatives, list and view them, mark one superseded by another, filter current
and superseded entries, and retain data across reloads. No accounts, backend,
external service or AI feature belongs in the app. Existing-decision editing is
intentionally unresolved. The agents must identify its tradeoffs and raise a
substantive structured question through the existing controller gate;
the brief and configuration supply no answer or recommendation. Specification
approval will establish requirements, **not** verify a working application.

The target owns `.wayward-wrangler/project.toml`, a versionable native workflow
profile, brief, rules and explicit context selection. Its default runtime store
is `temp/project-decision-log/.wayward-wrangler/state/`. Installed package
resources supply skills and schemas; no parent-checkout import, `PYTHONPATH`,
development profile or source-relative resource is allowed. The target's
`project_key` and state marker distinguish it from Wayward Wrangler's legacy
runs. The location index only routes links; decisions, findings, packets,
checkpoints, receipts and artifacts remain in target state.

## Small implementation sequence

1. **Native admission and preflight.** Replace `cli._create`'s hard-coded
   `m2-cli-01`/`context-01` allowlist for *new, initialized projects* with
   `execution_mode = "native"` in a target-owned profile and a run-specific
   `--authorize-native PLAN_SHA256` on `start`. A read-only `preflight` command
   computes that digest from project key, run ID, brief/context/config hashes,
   installed application/protocol, role assignments, and limits. Start recomputes
   and requires an exact match, then records the authorized plan in the immutable
   envelope. Resume rejects an incompatible installed adapter/protocol rather
   than silently changing its contract. The digest confirms deliberate,
   unchanged launch inputs; it is not
   an authentication credential. Resume uses frozen inputs without asking for
   stage-by-stage approval. Historical run IDs remain unavailable for a new
   project, and historical envelopes, prompts, receipts and approvals are not
   rewritten. Preflight checks absolute binaries, pinned supported CLI versions,
   relevant noninteractive flags, profile shape, project-local rules/context,
   Herdr target and auth *status* (`claude auth status`, `codex login status`)
   without printing credentials or launching a model. Model entitlement and
   actual backend selection remain unknown until execution.
2. **Freeze and enforce the native budget.** Require exactly Claude author and
   Codex critic for this trial, with explicit provider, binary, model, reasoning
   effort, `review_limit = 2`, `max_invocations <= 5`, and workflow version 2.
   Freeze those values in each run envelope and packet; bind the runner's
   invocation reservation to the frozen limit before process launch. Keep the
   existing no-redispatch behavior after failed or uncertain execution. Retain
   text-only permissions, one Herdr-visible pane per worker, frozen stdin,
   structured output, receipt reconciliation, durable questions, decision
   replay, context checks and artifact-bound final approval. Do not turn the
   reusable profile into standing permission to start a native run.
3. **Fit the specification artifact.** The current version-2 native prompt asks
   for under 450 words, too tight for a useful app specification and browser
   acceptance checks. Add a frozen, versioned `product-spec-v1` prompt contract
   for new runs only, with a bounded roughly 1,200-word artifact, explicit
   acceptance criteria and a short author walkthrough. Keep the historical
   version-2 prompt bytes unchanged for saved-output verification. Use the
   existing result/question schema and graph, not a new workflow. The new
   target's explicit context selection supplies its own requirements and any
   later accepted decisions; it does not discover relevance automatically.
4. **Verify offline, then run one authorized trial.** Test admission denial,
   stale plan digest, frozen role/budget replay, installed-resource resolution,
   question/answer restart, stale approval refusal and preservation of legacy
   evidence. Install a freshly built wheel with pipx, run preflight outside the
   checkout, then launch only the approved `decision-log-01` trial. Inspect the
   native invocation records, model/effort evidence, actual Herdr panes, focused
   handoffs and final artifacts. Do not auto-retry a failure or uncertain call.

## One bounded acceptance trial

- `decision-log-01`: Claude draft → independent Codex critique → Claude
  revision, with optional critique 2/revision 2. At most **five** native stage
  launches and **two** critiques. Each launched stage has one invocation
  reservation and one matching immutable receipt; no duplicate pane or call on
  resume. A question raised by draft or first critique pauses dependent work.
  If no genuine editing-policy question is raised in time, the trial fails its
  question criterion; no injected question or extra call repairs it.
- At the unanswered gate, exit the controller. Record the operator's own answer
  through `decide --record-only`, then use a fresh controller to consume it once.
  Later packets must include the accepted answer, relevant open findings, exact
  prior artifacts and critique-2 revision delta. Accepted choices must not be
  reopened without new evidence. An unresolved/late question with no author
  slot, changed context, failed worker or uncertain execution stops with an
  explicit operator action.
- The final artifact must give implementable data fields, create/list/detail/
  supersession/filter behavior, local persistence semantics, and browser-
  verifiable checks for each capability and each explicit exclusion. It must
  state the operator's editing decision accurately and include a short manual
  walkthrough for the requesting author. The controller displays the exact
  artifact and decision revision; only the operator may approve/reject it.
  Approval of the specification does not claim that the browser app exists or
  passed those checks.

## Herdr integration and rollback

The current shared plugin is linked to this checkout's `herdr-plugin/` and
handles historical run-only links. At the approved switch, first capture
`herdr plugin list --json` and the old plugin root/manifest, confirm the pipx
installation and packaged plugin assets, then `herdr plugin unlink
local.agent-workflow-review` and run the installed
`wayward-wrangler integration herdr install`. Verify the new root is the
user-local copied integration directory and click a project-qualified review
link for this target. Perform this only with no active review action; workflow
workers and state are independent of the plugin. If registration or click fails,
unlink the new registration and `herdr plugin link OLD_PLUGIN_ROOT` using the
captured checkout plugin path; verify the old
registration. Historical links remain explicit-`--project` inspection paths
under the new plugin; do not silently route run-only links.

## Operator choices before execution

Recommended cost-aware settings: Claude `sonnet` with `medium` effort for author
stages; Codex `gpt-6.1-sol` with `high` effort for critiques, retaining the
operator's earlier Sol/high preference. These are proposed names/settings, not
verified backend model identities or cost quotes. Current read-only version
checks returned Claude Code `2.1.296`, Codex `0.162.1`, Herdr `0.9.3`; installed
wheel, authentication and model availability must be preflighted again before
launch. The operator must approve the milestone-5 implementation scope, exact
role/model/effort choices and the single native trial before either is run.
The editing-policy answer and final artifact approval remain separate decisions
made at their durable gates.
