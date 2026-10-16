# Hosts

How Permitt knows what each assistant may do without asking. One rule per host in
`hosts/*.toml`; the inventory reads the rules, then the files they point at, and nothing
else. **It never writes to any of them.**

Every claim carries its source — the vendor's documentation, the host's source code, or an
observation on a real machine — with the URL, the date it was read and the words it was
read from. That goes for the files, for the meaning of each setting, for the **defaults**
(what applies when nothing changed it) and for the **precedence** between files. A rule
without a source is refused when the binary is built. No rule rests on memory.

## The model

Four **capabilities**: `commands` (run shell commands), `files` (create, modify, delete),
`network` (fetch, search, outbound connections of the agent), `mcp` (call MCP tools).

One **state** per capability — a fact, not a score:

| State | Meaning |
|---|---|
| `asks` | asks every time |
| `auto` | acts without asking |
| `judged` | a classifier (a model) decides at each call; the user is not consulted |
| `allowlist(n)` | acts without asking for n patterns, asks otherwise |
| `denylist(n)` | acts without asking except for n patterns — which a command can work around |
| `sandboxed` | acts without asking inside an isolation mechanism |
| `off` | disabled or denied |
| `unknown` | cannot be read from a documented file, or the documentation contradicts itself; the reason is in the evidence |

**Levels**: `managed` (organisation policy), `user`, `project` (versioned file), `local`
(project file kept out of git), `shell` (a start-up file), `default` (the built-in behaviour).

## Rule schema

The common part (identity, locations, paths per OS, sources, validation) comes from the
shared crate [`bbl-hostrules`](https://github.com/blackberrylabs/bbl-hostrules). Permitt adds
`level` to every location and, per host, `binaries`, `[defaults]`, `[[settings]]`,
`[precedence]` and `[[unknown]]`.

```toml
id = "claude-code"
name = "Claude Code"
kind = "cli"                      # cli | editor | extension
status = "active"
binaries = ["claude"]             # on PATH ⇒ installed, even with no file yet

[defaults]                        # what applies when no setting changed it (sourced)
commands = "judged"
files = "judged"
network = "judged"
mcp = "judged"
contradictory = false             # true: the documentation disagrees; states say unknown where it does
note = "…"
[defaults.source]
kind = "documentation"            # documentation | source-code | observed
url = "https://…"
read_on = 2026-10-16
quote = "the words it was read from"

[precedence]
order = ["managed", "local", "project", "user"]   # strongest first
lists = "merged"                  # merged: pattern lists combine across levels · replaced: the strongest level's list wins
status = "documented"             # documented | undocumented | contradictory
note = "…"
[precedence.source]               # required when documented
kind = "documentation"
url = "https://…"
read_on = 2026-10-16
quote = "…"

[[unknown]]                       # a part of the state no documented file holds (D10)
capabilities = ["commands"]       # empty: all four
overrides_state = true            # true: the capability is unknown unless a file sets it · false: a note
reason = "where it lives and where to check, in one sentence"

[[locations]]
scope = "user"                    # user | project | system
level = "user"                    # managed | user | project | local — must fit the scope
format = "json"                   # json | jsonc | toml | yaml
path.any = "~/.claude/settings.json"   # or path.linux / path.macos / path.windows; lists allowed
project_key = "locations"         # a per-project table inside a user file, keyed by project path
[locations.source]
kind = "documentation"
url = "https://…"
read_on = 2026-10-16
quote = "…"

[[settings]]
key = "permissions.defaultMode"   # dotted path; `\.` literal dot; `*` wildcard names a tool
kind = "mode"                     # mode | rules | map | object-rules
applies = ["commands"]            # capabilities when no tool map decides
file = "settings.json"            # only in files of this name (optional)
levels = ["managed"]              # only at these levels (optional)
not_honoured_in = ["project", "local"]            # shown, not applied, reported
not_honoured_values = ["auto", "bypassPermissions"]
guard = "PreToolUse hooks"        # when found, listed as a safeguard
note = "…"
[settings.states.acceptEdits]     # mode: value → capability (or `all`) → state; `_note` shown; `_any` matches any value
commands = "allowlist"
files = "auto"
[settings.tool_map]               # rules / map / object-rules / wildcard mode: tool → capability | all | ignore
Bash = "commands"
"mcp__*" = "mcp"                  # glob prefix
_default = "mcp"                  # anything else; without it an unknown tool is reported, never guessed
[settings.source]
kind = "documentation"
url = "https://…"
read_on = 2026-10-16
quote = "…"
```

### Setting kinds

- **`mode`** — one scalar (string or boolean) → one state per capability through `states`.
  With a `*` in the key (`agent.tool_permissions.tools.*.default`) the matched key is the
  tool and `tool_map` says which capability it is.
- **`rules`** — an array of patterns with one `effect` (`allow` | `ask` | `deny`):
  `Bash(git *)`, `shell(git:*)`, `Exec(prefix)`, `git *`. The tool before `(` goes through
  `tool_map`; a plain list uses `applies`. A **bare** entry — the whole tool (`Bash`,
  `Bash(*)`, `edit`), or `*` in a plain list, or any entry when `whole_tools = true` —
  makes the capability `auto` (allow) or `off` (deny). Other entries are counted.
- **`map`** — an object `pattern → effect`, `tool → effect` or `tool → { pattern → effect }`
  (OpenCode's `permission`, VS Code's `chat.tools.terminal.autoApprove`). Raw values go
  through `effect_values` (`"true" = "allow"`); an object value's effect is read at
  `value_field`. The keys `*`, `**`, `**/*` set the base; others are counted.
- **`object-rules`** — an array of tables with a `tool_field` and an `effect_field` (Kiro's
  `rules`, Amp's `amp.permissions`, Copilot's `tool_approvals`). A rule whose match is
  `*`, `**` or `./**`, or that has no match field, is bare.

### How a state is computed

Levels are applied from the weakest to the strongest of `precedence.order`: a `mode` found
at a stronger level overrides. Within one level, settings apply in the order of the rule
file (later wins), so a host's rule lists the broad setting before the specific one.
Pattern counts are summed across levels when `lists = "merged"`, otherwise the strongest
level that has any pattern wins.

Then, per capability: the base is the mode's state, or the sourced default (marked
`(default)`); a bare allow makes it `auto`, a bare deny `off`; allow patterns turn `asks`
into `allowlist(n)`; ask or deny patterns turn `auto` into `denylist(n)`. An `unknown`
note that `overrides_state` makes the capability `unknown` unless a mode or a bare entry
set it — patterns alone do not settle an unknown base. A setting found where the host does
not honour it is kept apart (`ignored`) and reported.

### Shell start-up files

`hosts/_shell.toml` lists the files read (`~/.bashrc`, `~/.zshrc`, `~/.profile`, fish…) and
a closed list of flags and variables, each with the documentation that describes it:
`--dangerously-skip-permissions`, `--dangerously-bypass-approvals-and-sandbox`, `--yolo`,
`--approval-mode yolo`, `--allow-all-tools`, `--yes-always`, `--trust-all-tools`,
`GOOSE_MODE=auto`, `COPILOT_ALLOW_ALL`, `AIDER_YES_ALWAYS`… Only the matching line is
kept (file, line number, text with secrets masked); comments never match. `--no-shell`
turns the reading off.

## Hosts read (16)

| Host | Files (Linux) | Defaults | Not readable |
|---|---|---|---|
| Claude Code | `/etc/claude-code/managed-settings.json`, `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json` | judged (auto mode since v2.1.283) | server-managed settings |
| Codex CLI | `/etc/codex/{config,requirements,managed_config}.toml`, `~/.codex/config.toml`, `.codex/config.toml` | sandboxed commands and edits, network off, mcp unknown | cloud-managed defaults |
| GitHub Copilot CLI | `/etc/github-copilot/managed-settings.json`, `~/.copilot/settings.json`, `~/.copilot/permissions-config.json` (per project), `.github/copilot/settings*.json` | asks | trusted folders in `config.json`, server-managed settings |
| Visual Studio Code (Copilot) | `/etc/vscode/policy.json`, `~/.config/Code/User/settings.json`, `.vscode/settings.json` | allowlist commands, auto edits (reviewed after), asks | per-tool / per-server approvals (editor state) |
| Cursor | `~/.cursor/permissions.json`, `.cursor/permissions.json`, `~/.cursor/sandbox.json`, `.cursor/sandbox.json`, `~/.config/Cursor/User/settings.json` | asks, files auto | **Run Mode, protections, in-app allowlists** → `state.vscdb` (observed) |
| Cursor CLI | `~/.cursor/cli-config.json`, `.cursor/cli.json` | asks; files undocumented | precedence between the two files |
| Gemini CLI | `/etc/gemini-cli/settings.json`, `~/.gemini/settings.json`, `.gemini/settings.json` | asks | system-defaults file, policy TOML, `trustedFolders.json` |
| Qwen Code | `/etc/qwen-code/settings.json`, `~/.qwen/settings.json`, `.qwen/settings.json` | **contradictory** (auto vs ask) | idem |
| Cline | `~/.cline/data/globalState.json`, `~/.cline/data/settings/global-settings.json` | files, network, mcp auto; commands asks | pre-migration copy in `state.vscdb` |
| OpenCode | `/etc/opencode/opencode.json(c)`, `~/.config/opencode/opencode.json(c)`, `opencode.json(c)` | **auto for everything** | per-project auto-accept (SQLite) |
| Goose | `~/.config/goose/config.yaml`, `permission.yaml` | **auto for everything** | per-session mode in `sessions.db` |
| Zed | `~/.config/zed/settings.json`, `.zed/settings.json` | asks | worktree trust (SQLite) |
| Kiro | `~/.kiro/settings/permissions.yaml`, `sandbox.json`, `~/.kiro/agents/*.json`, `.kiro/settings/sandbox.json`, `.kiro/agents/*.json` | allowlist commands, asks | workspace rules under a hashed directory; IDE autonomy setting |
| Amp | `/etc/ampcode/managed-settings.json`, `~/.config/amp/settings.json(c)`, `.amp/settings.json(c)` | **does not ask** (built-in deny list for commands) | `amp.permissions` documented only by `--help` |
| Aider | `~/.aider.conf.yml`, `.aider.conf.yml` | files auto, commands asks, no network, no MCP | precedence **contradictory** (docs vs code) |
| Devin Desktop (Windsurf) | `~/.config/devin/config.json`, `.devin/config.json`, `.devin/config.local.json` | asks (Devin Local / CLI) | **Cascade's auto-execution level** (settings panel, protobuf) |

Not read yet, with the reason in `hosts/_not-read.toml`: Roo Code (everything in
`state.vscdb`), Kilo Code, Antigravity CLI, JetBrains AI Assistant, Warp, Trae, Factory
Droid (flags only).

## What we learnt writing the rules

- **The default is the setting.** Six hosts act without asking before any file exists:
  OpenCode, Goose, Amp, Cline (edits, MCP), Claude Code (auto mode), Cursor (edits). A
  sheet that only showed what files say would show nothing for them.
- **A model deciding is its own state.** Claude Code `auto`, Cursor Auto-review, Qwen
  `auto`, Goose `smart_approve`, VS Code Assisted, Devin Smart: the user is not asked, a
  classifier filters. Calling that `auto` would be wrong; calling it `asks` would be worse.
- **A sandbox is not always a permission.** Codex, Claude Code (`autoAllowBashIfSandboxed`)
  and Cursor run sandboxed commands without asking: `sandboxed`. Gemini, Qwen, VS Code and
  Kiro keep their prompts inside the sandbox: a guard, not a state.
- **Two hosts hide the one setting that matters.** Cursor's Run Mode and Cascade's Turbo
  mode are set in a panel and stored in a state database or a protobuf. Permitt says so,
  and where to look, rather than guessing.
- **Precedence is documented more often than expected** — and wrong once: Aider's docs say
  the current directory wins, its code makes the home file win.
