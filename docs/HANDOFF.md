# Waywr handoff

Branch: `plan/usability-and-installation`; parent checkpoint `cafc725`.
Current slice: public installer implemented and checked locally. The user owns
remote delivery and has deferred merge until distribution is ready. Read
[AGENTS](../AGENTS.md); load task notes/reports selectively.

## Completed

Public installation now uses HTTPS without gh/login, defaults to latest stable,
and accepts an exact version. Node/Git/flock preflight, checksum/asset/archive/
bundle verification and atomic selection remain. Bash PATH guidance replaces
per-project upgrade instructions. Private gh access requires explicit `--private`.

The real isolated archive/install/CLI/service/TUI check passes, including piped
input and failed-download preservation. HTTPS transport alone is a fixture; no
account, operator installation/service or remote change occurred. Exact evidence
and limits: [installer report](reports/public-installer-2026-11-11.md).
The ignored local-bin candidate was not refreshed by this installer task.

## Next review

The user deferred items 1, 3 and 4 until this fix: hosted bundle-build DNS failure,
CLI update/uninstall, then first published release and package channels. No work
on those items or publication is implied. Security passed on pushed revision `717d2a4`;
[security report](reports/security-audit-2026-11-08.md) records the inspected runs.

For the next slice, read [implementation scope](agents/usability-plan.md) and
[distribution notes](agents/distribution.md). Human instructions are in
[distribution](distribution.md); Node remains an external runtime prerequisite.
