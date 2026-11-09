> Public copy: operator identities, locations and project names are generalized.
> This document is historical context, not exact runtime evidence.

# Milestone 2 — real CLI exchange

Project: **Wayward Wrangler** (formerly example-workspace). Historical names,
commands, paths and execution evidence below are retained; see [migration notes](../portability/project-migration.md).

October 10, 2026. **Milestone 2's bounded trial is complete and operator-approved.**
All five stages completed once. The operator chose **Offline only** and approved
**revision-2** through the controller. A fresh process verified the saved A1
response, matching artifact bytes, approved graph outcome and no pending gate.
Approval timestamp: 2026-10-10T20:47:47.043099+00:00. No active run remains.

## Authorization and operator confirmation

The operator authorized Claude/Codex adapters and one bounded real trial using
existing authentication, in this standalone project and its Herdr workspace.
They explicitly selected **latest Sol, high** for Codex workers. No shared
configuration changes, CLI upgrades, Git/PR operations, another project edits or
milestone-3 work are authorized.

The operator explicitly confirmed **the Herdr review plugin works**. This report
does not infer repeat-click tab reuse, close/return behavior or other interactions.
The simulated demo remains approved and preserved; its five counters remain 1.

## Versions, authentication and effective launch settings

| Role | Installed CLI | Requested model / effort | Runtime evidence |
|---|---|---|---|
| Author | Claude Code 2.1.296 | Saved user choice `opus`, high | Init and modelUsage resolved **claude-opus-5-5**; formatter result successful |
| Critic | Codex 0.162.1 | Operator choice **gpt-6.1-sol**, high | Exact model/effort passed explicitly; successful completed turn. Exec JSONL does not independently report a resolved backend model ID |

Claude authentication status reported logged-in Claude.ai first-party auth;
Codex reported ChatGPT authentication. No login, key generation, auth-file copy,
credential display or CLI upgrade was performed. Existing authentication remains
under the original HOME/CODEX_HOME. CLI-owned caches/logs or token refresh may
still occur; no claim is made that native CLI state is entirely write-free.

Claude uses safe mode, an empty setting-source list, an empty built-in tool list,
strict empty MCP configuration, `dontAsk` permissions, and explicit
`disableAllHooks`. Safe mode suppresses native customizations without disabling
authentication. Managed policy remains authoritative; the checked local managed
settings paths were absent. Runtime init reported only **StructuredOutput**, no
MCP servers and **dontAsk**. StructuredOutput formats the result; it is not a
filesystem/shell tool. No hook events or other tool calls were observed.

Codex uses `--ignore-user-config`, `--no-daemon`, `--ephemeral`, read-only sandbox,
and `--ask-for-approval never` (escalations denied). Web search, hooks, plugins,
apps, shell, shell snapshots, browser/computer tools, multi-agent delegation,
images, memories, goals, tool suggestions and code-mode host are disabled per
invocation. Automatic project instruction loading and bundled skill instructions
are disabled; supplied packet rules remain authoritative task context. No shared
config is rewritten. Exec JSONL showed no tool calls or native hook events.

**Important limitation:** installed-release source registers `apply_patch`
independently of `shell_tool`; read-only sandbox/denied escalation is the write
protection for any remaining advertised patch tool. This trial observed no patch
attempt and does not certify a hostile-agent sandbox. Any observed execution,
file-change, MCP, web or delegation event stops the stage. These event checks are
additional detection, not a replacement for permissions.

Each Codex critique emitted the same diagnostic: Code Mode unavailable because its host was disabled,
with fail-closed behavior. Its turn then completed normally with the requested
structured critique. This exact diagnostic is retained/allowed by the adapter;
other error items fail the stage. There was no retry to work around it.

## Input, output and handoffs

The runner previously used DEVNULL for fake-worker stdin. Native workers now
receive a frozen **prompt.txt opened read-only as stdin**, followed by EOF.
This is neither terminal scraping nor keystroke injection. The prompt embeds the
complete canonical packet and its hash. It includes the brief, explicit rules,
stage skill, candidate text, accepted decisions, open findings, critique result,
and (for critique 2) the previous reviewed draft and unified revision delta.
No native author session is reused by the critic or revision.

`output-schema.json` constrains the same result contract for both CLIs. Claude
uses `--json-schema`, stream-json, verbose and partial messages; the adapter
accepts one successful `structured_output`. Codex uses stdin `-`, `--json`,
`--output-schema` and a native final-message file; the adapter requires one
completed turn and parses that JSON file. The controller checks packet identity
and result shape before immutable artifact publication. Worker output cannot
approve or answer human gates.

Live child stdout/stderr passes through the existing actual Herdr runner pane.
Native stdout is also retained as `native-events.jsonl`; the transcript retains
both streams. Claude streaming was captured in its real pane before completion
(243 partial events). Codex JSONL events were observed before its receipt existed;
its actual pane was captured after completion. Codex exec exposes lifecycle and
completed-message events here, **not a verified token-by-token text stream**.

The first critique received the exact saved draft automatically and identified
both the offline/cloud conflict and the draft's overly broad manual-export
privacy claim. Both revisions and critique 2 received the accepted **Offline only** decision.
Critique 2 received the prior reviewed draft and a 6,099-character unified delta.
It found a remaining network-share export inconsistency (C2-001); revision 2
bounds app-created export to locally attached storage and rejects remote shares.
These packet contents and published results are runtime evidence of handoff;
they do not prove the model used every supplied field. The final author marks
findings addressed, but revision 2 has not received another independent critique. The controller deliberately schedules the known policy
choice after critique 1; models are instructed to leave the question array empty.
Unexpected model questions stop as an operational error rather than being answered.

## Actual invocation counts and historical tabs

| Stage | Provider | Invocations | Outcome | Tab / pane |
|---|---|---:|---|---|
| draft | Claude | 1 | Completed, artifact published | w8:t9 / w8:p9 |
| critique-1 | Codex | 1 | Completed, findings published | w8:tA / w8:pA |
| revision-1 | Claude | 1 | Completed after operator choice | w8:tB / w8:pB |
| critique-2 | Codex | 1 | Completed, remaining export finding | w8:tC / w8:pC |
| revision-2 | Claude | 1 | Completed, operator-approved | w8:tD / w8:pD |

**Total: 5 / 5 stage invocations.** These count native process launch attempts,
not individual provider HTTP requests or billing events. Claude reported two
internal turns for the draft including structured formatting. CLI-internal
transport retries may exist; the application performs no automatic re-dispatch
of a failed/uncertain stage. Do not confuse process-once bookkeeping with
exactly-once external inference.

The table identifies the original execution tabs. At completion verification,
all five recorded worker panes were closed; artifacts, transcripts, packets and
receipts remain on disk. This verification did not close panes or stop processes.
The original controller was w8:t8 / w8:p8; do not use that historical identity
as a current navigation target without checking Herdr.

## Completed human gate and inspection

The saved choice is **Offline only**. A1 records approval by **the operator** of
**revision-2**, decision revision 1. Its fingerprint is audit metadata:
`7b42658dffb436aae8aaff958fd7096c73cb4ff33bde1b6f7ebcf3d8c010f466`.
Revision-1 was the last independently reviewed version; the two-round limit
was respected. Approval does not imply a third independent critique occurred.

No continuation or further approval is needed. Read-only inspection:

```fish
cd ~/projects/example-workspace
.venv/bin/awp status m2-cli-01
.venv/bin/awp show m2-cli-01 --artifact revision-2
```

The interactive controller accepts **approve this revision** or **approve** and
binds the displayed artifact and decisions internally. The stored response says
“Approved through controller”; it does not preserve the exact typed phrase.
We therefore verify successful controller approval without inferring which alias
was entered or additional Vim/plugin interactions. The development agent did
not submit the choice or approval. No additional trial is authorized.

## Verification and remaining limitations

- **31 offline tests passed**, including frozen stdin delivery; exact model/
  reasoning arguments; tool/hook/permission flags; schema/normalizers; rejecting
  tool activity, stale input and unexpected errors; and controller-owned choice
  followed by mandatory revision. Existing M1/review tests continue to pass.
- Claude completed three stages and Codex completed two stages using existing authentication and their
  supported structured interfaces. Both were separately identified in Herdr.
- The completed demo's file hashes, checkpoint values and app record match the
  pre-trial baseline. Shared Claude/Codex settings hashes are unchanged.
- Real accepted-decision transfer and second-review delta delivery are verified
  from packets/results. Final controller approval is now **persisted and verified**
  from a fresh process. Natural approval and stale displayed-gate rejection pass
  offline tests; the precise operator input phrase was not logged.
- Hard interruption, post-publication failure windows, native cancellation races,
  worker/server restart, full policy adversarial testing and transport-level
  exactly-once execution remain outside this milestone. No milestone-3 fault
  injection or authenticated retry was performed.

Evidence: `.local/evidence/milestone-2-completion.json` (final state),
`milestone-2-before.json` (baseline), `milestone-2.json` (pre-approval snapshot),
`m2-live-draft.json`, `m2-live-critique-1.json`, and the run's `operations/` files.
Each operation has a packet, prompt, schema, execution argv/restrictions, native
stream, final result, counter, child identity, receipt and Herdr binding.

## Implementation and source evidence

- `src/agent_workflow/adapters/real.py`: both native adapters, explicit launch
  configuration, strict result parsing and scope monitoring.
- `runner.py`: read-only stdin file, visible stream forwarding, one launch
  reservation, native result handling and targeted stop on operational ambiguity.
- `artifacts.py`: real execution configuration and planned-choice context; original
  fake packet shape preserved. `workflow.py`: mandatory human gate after critique 1.
- `cli.py`: fixed single-trial authorization/profile validation.
- `config/milestone-2.toml`, `examples/milestone-2-brief.md` and
  `examples/milestone-2-rules.md`: explicit model choices, five-invocation bound and brief.
- `tests/test_real_adapters.py`: offline contract checks, no model calls.

CLI help/version and sanitized auth/config metadata were inspected locally.
[Claude CLI reference](https://code.claude.com/docs/en/cli-reference) and
[programmatic execution](https://code.claude.com/docs/en/headless) document
stdin, structured streaming, safe mode and restrictions. [Codex configuration
reference](https://learn.chatgpt.com/docs/config-file/config-reference) and
[hooks](https://learn.chatgpt.com/docs/hooks) document per-invocation controls.

Official Codex source inspected at **rust-v0.162.1**, commit
**092d3acd6bec3e3a14bdc7e7a2810ab628ab759d**:
[config.schema.json](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/config.schema.json)
verified setting types; [tools/spec_plan.rs](https://github.com/openai/codex/blob/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d/codex-rs/core/src/tools/spec_plan.rs)
verified independent shell and patch registration. Source was downloaded under
`/tmp/awp-m2-source`, not installed or built. Claude Code native source is not
available here; its CLI help, official documentation and emitted init/result
metadata are the evidence, not a source-code verification claim.

## Approval interaction correction

The operator's pasted hash matched the saved pending artifact exactly. The old
controller rejected it before the hash check, using a misleading generic message.
Raw input bytes were not saved, so hidden paste framing is a plausible cause,
not a proven diagnosis. Readline now collects bracketed paste as one input;
multiline commands and unknown control characters are refused. An isolated PTY
test verifies valid paste and multiline rejection without any workflow response.

The user objected to typing machine identifiers. Interactive approval now uses
explicit ordinary phrases and keeps fingerprints internal; changed artifacts or
decisions require renewed review. This remains a deterministic controller, not
a conversational agent interpreting arbitrary prose. No new worker, gate answer,
or milestone-3 fault injection was performed for this UI correction.

Completion verification changed documentation/evidence only. All five counters
remain 1; the original simulated demo and shared settings match the baseline.
Milestone 3 remains unstarted and requires separate authorization.
