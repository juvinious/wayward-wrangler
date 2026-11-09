> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Continuous Claude readiness — offline gate passed; bounded v8 live workflow verified

The builtin inventory/AGENTS.md gap at `780f658` is resolved for the exact pinned
client/profile. The credential-free boundary evidence below is separate from the subsequent live trial.
[Report](continuous-coordinator-report.md) owns counts, failed attempts and provenance;
[completed live contract](claude-continuous-live-acceptance.md) records four successful
operator-driven calls, exact scope/revision permission and physical detach/resume acceptance.
The [original closed-gate findings](native-readiness-at-780f658-2026-10-14.md)
remain historical evidence. Original blocked scope-v6 stays **1/6**, untouched.

## What the two builtin entries actually mean

| Entry | Source-inspected behavior | Observed under the approved profile |
| --- | --- | --- |
| cc-plugin-agents-md | Registers instruction hooks; default fallback can load project/ancestor AGENTS.md where CLAUDE.md is absent. This is real functionality, not inherently inert metadata | Entry remains registered; no watched project/ancestor~ AGENTS.md or CLAUDE.md content opened or included; both fallback and CLAUDE-present cases checked |
| cc-plugin-plugin-authoring | Registers the plugin-authoring skill; no standalone worker authority | Entry remains registered; skill/slash lists empty, only StructuredOutput advertised; forced Skill and inherited skill invocations refused without effects |

Evidence is the exact 2.1.296 binary, SHA
`24972e3bc859fab2b46ed4c1e51f7d6130f06d3bd550811a114640de3370d0de`,
its `--help` and embedded registration/hook source (byte offsets and hashes retained).
Help explicitly says safe mode excludes installed customizations while builtin
plugins remain. Registration is distinct from instruction loading, model-visible
skills/tools, and an executable operation. Neither entry is safe unconditionally.

## Effective exclusion proof

Existing cached image, no-egress Docker network, dummy API key, read-only root,
UID1000/caps dropped/no-new-privileges, 1GiB/64 PID; no host HOME, credentials or
operator project mounts. Only pinned executable + owned disposable proof mounted.
No image download, new platform or shared policy change. Local mock endpoint only.

14 restricted cases + three positive controls, one inference POST each. Distinct
project/.claude, ancestor, HOME and user/.claude AGENTS.md canaries; CLAUDE fallback
and presence; enabled inherited plugin with real local marketplace/install records,
manifest, skill, hooks and MCP canaries. Inotify open/access/modify watches cover
all these content files and the foreign sentinel. A child-read control verifies
every watch; re-arm after fixture file replacement. Positive client controls without
the exclusions load project/ancestor instructions, inherited plugin and its skill.
Thus the strict absence checks do not depend on an undiscoverable plugin fixture.
User AGENTS locations are tested as exclusion sentinels, not claimed default loaders.

Restricted cases: no watched instruction/plugin-content access, canary request text,
extra tools, skills, slash commands, MCP or effects. Forced Read/Bash/Agent/SendMessage/
MCP/Skill calls refuse. No repair, fallback or transport retry. The trusted client
**does read/maintain its installed-plugin registry metadata**; that is not model
context or plugin instruction execution. We do not claim no startup filesystem
access or whole-process confinement. Original first assertion conflating registry
access with instruction loading failed and remains retained, then was corrected.

No new flags or exclusion exceptions were necessary: existing safe/restricted,
empty settings sources, disableAllHooks, disabled slash commands, empty tools,
explicit deny list, strict empty MCP, no history/memory/session persistence,
single model/turn and zero retry controls established the tested boundary.

## Narrow application policy and integration

`continuous_adapter::inspect` accepts only the exact unordered pair:
`{name:N, path:"builtin", source:N+"@builtin"}` for the two names above, with
`claude_code_version:2.1.296`. Extra fields, duplicates, partial pairs, arbitrary
entries, spoofed paths/sources, version drift and malformed inventory refuse.
Empty/missing inventory remains compatible with existing mock streams; null is invalid.
Tools must still be exactly StructuredOutput; skills/MCP and any reported slash
commands must remain empty. Non-sink tool use refuses. Metadata is never stripped.
Binary hash pin and preflight restrictions remain authoritative prerequisites.

44 unchanged real-client records through production decoder: 11 valid admissions,
33 refusals in the expected capability/format/unavailable categories. Nine valid
results also reach production application authority on separate mock-only records;
two fabricated answer sources correctly refuse. Duplicate adoption has one effect;
scope proposals remain unauthorized. Raw duplicate-key, size, framing, terminal/exit
and integrity regressions remain. No native allowance/process/service was created.

## Live result and remaining limits

The bounded v8 trial passed: four of six authorized coordinator invocations completed
with matching immutable receipts. Saved-login/model availability and native structured
discussion/proposal delivery worked for these calls. The operator accepted the presented
workflow and physical detach/re-attach retaining history/work. After operator exit,
checkpoint36 and inactive service were verified read-only. [Report](continuous-coordinator-report.md)
separates these observations from automated evidence and records two wording nuances.

Native queued-call overlap, interruption/format-error recovery, formal-answer/local-edit
paths and every crash window were not exercised in this live run. Future account/
server-policy availability is not guaranteed. Only the pinned trusted client is covered,
not arbitrary same-UID programs, future versions or a general filesystem sandbox.
Unexpected managed configuration/capability drift refuses; never bypass it. Native
worker/Codex admission and consultation remain closed independently.

Two unused calls remain in the original six-call ledger; no renewal/refund or additional
call is requested by closeout. Keep the exact release/profile/project binding. No old
trial may be resumed or reinterpreted under this policy. Await user direction before
another increment; do not consume calls to manufacture missing acceptance coverage.
