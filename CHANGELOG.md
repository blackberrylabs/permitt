# Changelog

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
