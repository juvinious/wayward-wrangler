# Scripted UI prototype component notices

The production TUI uses these components. The complete Linux bundle notices are
in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md), generated from locked
normal/build dependencies. Bundles carry that file and the project's MIT license
under manifest hash validation. Required upstream attributions remain intact.

| Package | Version | License | Upstream |
|---|---:|---|---|
| Ratatui | 0.30.2 | MIT | https://github.com/ratatui/ratatui |
| Crossterm | 0.29.0 | MIT | https://github.com/crossterm-rs/crossterm |
| ratatui-textarea | 0.9.3 | MIT | https://github.com/ratatui/ratatui-textarea |
| unicode-width | 0.2 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-width |
| TachyonFX | 0.23.0 | MIT | https://github.com/ratatui/tachyonfx |

`Cargo.lock` records the full dependency tree. Regenerate notices after dependency
changes with `python3 scripts/generate-notices.py`; packaging checks they match.
Other platforms require their own dependency/notice assessment before distribution.
