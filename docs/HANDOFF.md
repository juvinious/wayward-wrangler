# Waywr handoff

Branch: `plan/usability-and-installation`; parent `b37a818` (public installer).
Current checkpoint prepares tag-release automation and a terse CHANGELOG.
The user owns remote delivery and has deferred merge until distribution is ready.
Read [AGENTS](../AGENTS.md); task reports are selective, not startup context.

## Implemented locally

Version tags validate main ancestry, exact frontend version and dated changelog
notes. Package/audit use the same frozen commit; publish checks tag identity again
and uses those notes. Preview tags never replace latest. Local metadata/Git/
publisher regressions and actionlint pass; no tag, remote run or publication.
Details: [release report](reports/release-pipeline-2026-11-11.md).

Public HTTPS latest/exact installation was already checked through a real isolated
archive/CLI/service/TUI journey; only transport was a fixture. See the
[installer report](reports/public-installer-2026-11-11.md). No operator install or
local-bin candidate refresh occurred. Node remains external.

## Next review

CHANGELOG currently has Unreleased only. A first tag needs a reviewed release
commit with matching frontend/Cargo.lock version and dated entry. The known
hosted builder DNS failure remains unresolved; update/uninstall, first published
download and package channels remain. No remote write is implied.

Use [release notes](agents/releases.md) or [distribution notes](agents/distribution.md)
for the next relevant task. Human guidance is in [releases](releases.md) and
[distribution](distribution.md). Keep changes local for user review/delivery.
