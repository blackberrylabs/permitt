# Changelog

## [Unreleased]

### Added

- `permitt report`: the inventory as a [`bbl-report/1`](https://github.com/blackberrylabs/bbl-report)
  document for the Blackberrylabs control tower (always JSON). Nothing in it is new: the four
  KPI figures, the findings, the files and the `act` hints the CLI and the app already show,
  secrets masked as everywhere else. Permitt's findings map onto the four common severities
  as follows — the tower never reinterprets:

  | Permitt | Common severity |
  |---|---|
  | shell bypass | 1 · act now |
  | runs without asking, a model decides | 2 · review |
  | deny list only, not honoured here | 3 · good to know |
  | unreadable, doc contradicts itself, state that cannot be read (`unknown`) | 4 · information |

- `permitt-app --select host[:capability]` opens the app on that cell, the way the tower
  does; `PERMITT_SELECT` still works for screenshots.

### Changed

- The interface building blocks, palette, preferences and translated strings now come from
  the shared crate [`bbl-ui`](https://github.com/blackberrylabs/bbl-ui), like the other
  Blackberrylabs tools. Nothing visible changes.
- The shared crate `hostrules` is now
  [`bbl-hostrules`](https://github.com/blackberrylabs/bbl-hostrules) 0.3.0 — a rename, same
  code. Nothing changes in the output.

## [0.1.0] — 2026-10-16

First release.

- Four capabilities (commands, files, network, MCP), eight states (`asks`, `auto`,
  `judged`, `allowlist(n)`, `denylist(n)`, `sandboxed`, `off`, `unknown`), six levels.
- 16 hosts, every claim sourced: Claude Code, Codex CLI, GitHub Copilot CLI, Visual Studio
  Code (Copilot), Cursor, Cursor CLI, Gemini CLI, Qwen Code, Cline, OpenCode, Goose, Zed,
  Kiro, Amp, Aider, Devin Desktop (Windsurf). Not read, with the reason: Roo Code, Kilo
  Code, Antigravity CLI, JetBrains, Warp, Trae, Factory Droid.
- Sourced defaults shown as `(default)`; settings applied in documented precedence order;
  per-project sheets where a project file changes the state; settings written where the
  host ignores them reported.
- Shell start-up files scanned for a closed list of 31 bypass flags and variables.
- Command line (`permitt`) and desktop app (`permitt-app`, iced), English and French,
  light and dark. Built on `hostrules` 0.2.0.
