# Waywr handoff

Branch: `plan/usability-and-installation`. Security checkpoint: `5937110`.
Documentation checkpoint: `717d2a4`, pushed by the user. This checkpoint makes
Bash the documentation default; application code is unchanged.
The user handles remote delivery and has deferred merge until distribution is ready.
Read [AGENTS](../AGENTS.md); task details are selective, not a startup reading list.

## Current state

- Security checks passed on pushed revision `717d2a4`. The bundle job still fails before
  checkout on Ubuntu mirror DNS after all retries; CodeQL was in progress when
  inspected. Local advisory/build evidence and exact run IDs are in the
  [security report](reports/security-audit-2026-11-08.md#hosted-security-follow-up--2026-11-11).
- The user reports responsive controls, sign-in/model requests and wheel scrolling.
  Useful coordinated application delivery remains unproven.
- Ignored target/local-bin/waywr still selects tested cb094be; candidate.json records
  it. Patched binaries are in target/release. No operator service/install was replaced.

## Next action

Implement public unauthenticated Linux installation using the existing paired
bundle. Start with its behavioral failure in isolated HOME; resolve Node handling
and public checksum trust. Then update/uninstall, then Debian packaging. No remote
writes, operator installation or account calls are implied.

Read the [implementation slice](agents/usability-plan.md) and
[distribution notes](agents/distribution.md) for this task. The human-facing
[distribution guide](distribution.md) describes current methods and planned commands.
Documentation checks cover Bash syntax, references, privacy and diffs; no
application tests or operator changes are needed for this documentation checkpoint.
