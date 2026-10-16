//! Permitt's host rules: the common schema comes from `bbl-hostrules`; this module adds what a
//! permission inventory needs — the level of every file, and per host the sourced defaults,
//! the meaning of each setting, the precedence and the parts that cannot be read.
//! Schema documented in `docs/hosts.md`.

use std::collections::BTreeMap;

use bbl_hostrules::rules::{Source, Status};
use bbl_hostrules::{Extension, HostExtension};
use serde::{Deserialize, Serialize};

use crate::model::{Capability, Level, PrecedenceStatus, State};

/// Per location: which level of settings the file holds.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermLocation {
    pub level: Level,
}

impl Extension for PermLocation {
    fn validate(loc: &bbl_hostrules::Location<Self>) -> Result<(), String> {
        use bbl_hostrules::Scope;
        let ok = match loc.ext.level {
            Level::Managed => loc.scope == Scope::System,
            Level::User => loc.scope == Scope::User,
            Level::Project | Level::Local => loc.scope == Scope::Project,
            Level::Shell | Level::Default => false,
        };
        if !ok {
            return Err(format!(
                "level `{}` does not fit scope `{:?}`",
                loc.ext.level, loc.scope
            ));
        }
        Ok(())
    }
}

/// How a setting's value is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettingKind {
    /// One scalar value (string or boolean) → one state per capability, via `states`.
    /// With a `*` in the key, the matched key names the tool (`tool_map`).
    Mode,
    /// An array of pattern strings (`Bash(git *)`, `shell(git:*)`, `git *`) with one
    /// `effect`; each pattern is attributed to a capability through `tool_map`, or to
    /// `applies` when there is no map.
    Rules,
    /// An object `pattern → effect` (or `tool → effect`, or `tool → { pattern → effect }`):
    /// `*` gives the base, other keys are counted as allow / ask / deny patterns.
    Map,
    /// An array of tables, each with a tool field and an effect field (Kiro, Amp, Copilot).
    ObjectRules,
}

/// One setting the host reads, and what its values mean.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Setting {
    /// Key path in the file (`permissions.defaultMode`, `agent.tool_permissions.tools.*.default`).
    pub key: String,
    pub kind: SettingKind,
    /// Capabilities concerned when no tool map decides (map / rules without `tool_map`).
    #[serde(default)]
    pub applies: Vec<Capability>,
    /// `mode`: value → (capability or `all` → state name). Booleans are the strings
    /// `true` / `false`; the value `_any` matches whatever is there (presence). A `_note`
    /// entry is shown with the evidence.
    #[serde(default)]
    pub states: BTreeMap<String, BTreeMap<String, String>>,
    /// Tool name or pattern (before `(`, glob `*` allowed) → capability, `all`, or `ignore`
    /// (a tool deliberately outside the four capabilities, such as reading). The key
    /// `_default` catches anything else; without it an unknown tool is reported, never
    /// guessed.
    #[serde(default)]
    pub tool_map: BTreeMap<String, String>,
    /// `rules`: the effect of every pattern (`allow` | `ask` | `deny`).
    pub effect: Option<String>,
    /// `rules`: every entry names a whole tool, never a pattern (Goose), so each one is bare.
    #[serde(default)]
    pub whole_tools: bool,
    /// `object-rules`: field names.
    pub tool_field: Option<String>,
    pub effect_field: Option<String>,
    /// `map` / `object-rules`: raw value → `allow` | `ask` | `deny` (`"true" = "allow"`,
    /// `"reject" = "deny"`). Missing raw values are reported.
    #[serde(default)]
    pub effect_values: BTreeMap<String, String>,
    /// `map`: when a value is an object, the field that holds the effect (`approve`).
    pub value_field: Option<String>,
    /// Levels where the host ignores this setting (shown, not applied, finding) — for the
    /// values in `not_honoured_values`, or any value when that list is empty.
    #[serde(default)]
    pub not_honoured_in: Vec<Level>,
    #[serde(default)]
    pub not_honoured_values: Vec<String>,
    /// Levels where this setting may appear (empty: any).
    #[serde(default)]
    pub levels: Vec<Level>,
    /// File name this setting lives in, when the host has several files at one level
    /// (`sandbox.json`); empty: any file of the host.
    pub file: Option<String>,
    /// When present and the key is found, a safeguard with this name is listed.
    pub guard: Option<String>,
    pub note: Option<String>,
    pub source: Source,
}

/// The built-in behaviour when no setting changes it (D9), with its source.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Defaults {
    pub commands: String,
    pub files: String,
    pub network: String,
    pub mcp: String,
    /// The documentation disagrees with itself; the states above say `unknown` where it does.
    #[serde(default)]
    pub contradictory: bool,
    pub note: Option<String>,
    pub source: Source,
}

impl Defaults {
    pub fn state(&self, c: Capability) -> Option<State> {
        State::parse(match c {
            Capability::Commands => &self.commands,
            Capability::Files => &self.files,
            Capability::Network => &self.network,
            Capability::Mcp => &self.mcp,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Precedence {
    /// Strongest first.
    pub order: Vec<Level>,
    /// `merged` (lists combine across levels) or `replaced` (the strongest level's list wins).
    #[serde(default = "default_lists")]
    pub lists: String,
    pub status: PrecedenceStatus,
    pub note: Option<String>,
    pub source: Option<Source>,
}

fn yes() -> bool {
    true
}

fn default_lists() -> String {
    "replaced".into()
}

/// A part of the state that cannot be read from a documented file (D10).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnknownNote {
    /// Empty: every capability.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// True (default): the missing part decides the state, so the capability is `unknown`
    /// unless a file sets it. False: an explanation shown with the sheet, the state stands.
    #[serde(default = "yes")]
    pub overrides_state: bool,
    /// Where it lives and where to check, in one sentence.
    pub reason: String,
    pub source: Option<Source>,
}

/// Per host: what Permitt knows about it as a whole.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermHost {
    /// Executables that show the host is installed, looked up on `PATH`.
    #[serde(default)]
    pub binaries: Vec<String>,
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub settings: Vec<Setting>,
    pub precedence: Option<Precedence>,
    #[serde(default)]
    pub unknown: Vec<UnknownNote>,
}

impl HostExtension for PermHost {
    fn validate<X: Extension>(rule: &bbl_hostrules::HostRule<X, Self>) -> Result<(), String> {
        if rule.status != Status::Active {
            return Ok(());
        }
        let d = rule
            .ext
            .defaults
            .as_ref()
            .ok_or("an active host needs [defaults] with a source (D9)")?;
        for c in Capability::ALL {
            if d.state(c).is_none() {
                return Err(format!("defaults.{c}: unknown state name"));
            }
        }
        if d.source.quote.trim().is_empty() {
            return Err("defaults.source needs a quote".into());
        }
        let p = rule
            .ext
            .precedence
            .as_ref()
            .ok_or("an active host needs [precedence] (D4)")?;
        if p.order.is_empty() {
            return Err("precedence.order is empty".into());
        }
        if p.status == PrecedenceStatus::Documented && p.source.is_none() {
            return Err("a documented precedence needs a source".into());
        }
        if p.lists != "merged" && p.lists != "replaced" {
            return Err(format!(
                "precedence.lists: `{}` is not merged | replaced",
                p.lists
            ));
        }
        for (i, s) in rule.ext.settings.iter().enumerate() {
            let at = |m: String| format!("settings[{i}] `{}`: {m}", s.key);
            if s.source.quote.trim().is_empty() {
                return Err(at("source needs a quote".into()));
            }
            if s.source.kind != bbl_hostrules::SourceKind::Observed && s.source.url.is_none() {
                return Err(at(
                    "a documentation or source-code source needs a URL".into()
                ));
            }
            for cap in s.tool_map.values() {
                if cap != "all" && cap != "ignore" && Capability::parse(cap).is_none() {
                    return Err(at(format!("tool_map value `{cap}` is not a capability")));
                }
            }
            match s.kind {
                SettingKind::Mode => {
                    if s.states.is_empty() {
                        return Err(at("mode needs `states`".into()));
                    }
                    for (v, m) in &s.states {
                        for (cap, st) in m {
                            if cap == "_note" {
                                continue;
                            }
                            if cap != "all" && Capability::parse(cap).is_none() {
                                return Err(at(format!("states.{v}: `{cap}` is not a capability")));
                            }
                            if State::parse(st).is_none() {
                                return Err(at(format!("states.{v}.{cap}: `{st}` is not a state")));
                            }
                        }
                    }
                    if s.key.contains('*') && s.tool_map.is_empty() && s.applies.is_empty() {
                        return Err(at("a wildcard key needs `tool_map` or `applies`".into()));
                    }
                }
                SettingKind::Rules => {
                    let e = s
                        .effect
                        .as_deref()
                        .ok_or_else(|| at("rules need `effect`".into()))?;
                    if !matches!(e, "allow" | "ask" | "deny") {
                        return Err(at(format!("effect `{e}` is not allow | ask | deny")));
                    }
                    if s.tool_map.is_empty() && s.applies.is_empty() {
                        return Err(at("rules need `tool_map` or `applies`".into()));
                    }
                }
                SettingKind::Map => {
                    if s.tool_map.is_empty() && s.applies.is_empty() {
                        return Err(at("map needs `tool_map` or `applies`".into()));
                    }
                    if s.effect_values.is_empty() {
                        return Err(at("map needs `effect_values`".into()));
                    }
                }
                SettingKind::ObjectRules => {
                    if s.tool_field.is_none() && !s.key.contains('*') {
                        return Err(at(
                            "object-rules need `tool_field` (or a wildcard key)".into()
                        ));
                    }
                    if s.effect_field.is_none() && s.effect.is_none() {
                        return Err(at("object-rules need `effect_field` or `effect`".into()));
                    }
                    if s.tool_map.is_empty() {
                        return Err(at("object-rules need `tool_map`".into()));
                    }
                }
            }
            for (raw, eff) in &s.effect_values {
                if !matches!(eff.as_str(), "allow" | "ask" | "deny") {
                    return Err(at(format!(
                        "effect_values.{raw}: `{eff}` is not allow | ask | deny"
                    )));
                }
            }
        }
        for u in &rule.ext.unknown {
            if u.reason.trim().is_empty() {
                return Err("an unknown note needs a reason".into());
            }
        }
        Ok(())
    }
}

pub type Location = bbl_hostrules::Location<PermLocation>;
pub type HostRule = bbl_hostrules::HostRule<PermLocation, PermHost>;
pub type Rules = bbl_hostrules::Rules<PermLocation, PermHost>;

/// A bypass flag or variable to look for in shell start-up files (D2): a closed, sourced list.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShellPattern {
    pub host: String,
    /// `flag` (a command-line option, matched as a whole word) or `env` (`NAME=` at a word
    /// boundary, with `values` restricting which assignments count).
    pub kind: String,
    pub pattern: String,
    #[serde(default)]
    pub values: Vec<String>,
    /// What it does, in the words of the documentation.
    pub effect: String,
    pub source: Source,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShellRules {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub flags: Vec<ShellPattern>,
}

impl ShellRules {
    pub fn load(text: &str) -> Result<Self, toml::de::Error> {
        let r: ShellRules = toml::from_str(text)?;
        Ok(r)
    }
}

/// The rule files shipped inside the binary.
pub const EMBEDDED: &[(&str, &str)] = &[
    (
        "_not-read.toml",
        include_str!("../../../hosts/_not-read.toml"),
    ),
    ("aider.toml", include_str!("../../../hosts/aider.toml")),
    ("amp.toml", include_str!("../../../hosts/amp.toml")),
    (
        "claude-code.toml",
        include_str!("../../../hosts/claude-code.toml"),
    ),
    ("cline.toml", include_str!("../../../hosts/cline.toml")),
    ("codex.toml", include_str!("../../../hosts/codex.toml")),
    (
        "copilot-cli.toml",
        include_str!("../../../hosts/copilot-cli.toml"),
    ),
    ("cursor.toml", include_str!("../../../hosts/cursor.toml")),
    (
        "cursor-cli.toml",
        include_str!("../../../hosts/cursor-cli.toml"),
    ),
    (
        "devin-desktop.toml",
        include_str!("../../../hosts/devin-desktop.toml"),
    ),
    (
        "gemini-cli.toml",
        include_str!("../../../hosts/gemini-cli.toml"),
    ),
    ("goose.toml", include_str!("../../../hosts/goose.toml")),
    ("kiro.toml", include_str!("../../../hosts/kiro.toml")),
    (
        "opencode.toml",
        include_str!("../../../hosts/opencode.toml"),
    ),
    (
        "qwen-code.toml",
        include_str!("../../../hosts/qwen-code.toml"),
    ),
    ("vscode.toml", include_str!("../../../hosts/vscode.toml")),
    ("zed.toml", include_str!("../../../hosts/zed.toml")),
];

pub const EMBEDDED_SHELL: &str = include_str!("../../../hosts/_shell.toml");

/// The embedded rules, validated. Panics only if the repository ships an invalid rule, which
/// the test suite prevents.
pub fn embedded() -> Rules {
    Rules::load(EMBEDDED.iter().map(|(n, t)| (*n, *t))).expect("embedded host rules are valid")
}

pub fn embedded_shell() -> ShellRules {
    ShellRules::load(EMBEDDED_SHELL).expect("embedded shell rules are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_rules_load_and_every_claim_has_a_source() {
        let rules = embedded();
        assert!(rules.hosts.len() >= 15, "{} hosts", rules.hosts.len());
        assert!(rules.not_read.len() >= 5);
        for h in &rules.hosts {
            let d = h.ext.defaults.as_ref().expect("defaults");
            assert!(
                !d.source.quote.is_empty(),
                "{}: defaults without quote",
                h.id
            );
            assert!(h.ext.precedence.is_some(), "{}: no precedence", h.id);
            for l in &h.locations {
                assert!(
                    !l.source.quote.is_empty(),
                    "{}: location without quote",
                    h.id
                );
            }
            for s in &h.ext.settings {
                assert!(
                    !s.source.quote.is_empty(),
                    "{}: setting {} without quote",
                    h.id,
                    s.key
                );
            }
        }
        let shell = embedded_shell();
        assert!(shell.flags.len() >= 25, "{} flags", shell.flags.len());
        assert!(shell.files.contains(&"~/.bashrc".to_string()));
        for f in &shell.flags {
            assert!(
                rules.host(&f.host).is_some() || rules.not_read.iter().any(|n| n.id == f.host),
                "shell flag for unknown host {}",
                f.host
            );
            assert!(!f.source.quote.is_empty());
        }
    }

    #[test]
    fn a_host_without_defaults_is_refused() {
        let text = r#"
id = "x"
name = "X"
kind = "cli"
status = "active"
[precedence]
order = ["user"]
status = "undocumented"
[[locations]]
scope = "user"
level = "user"
format = "json"
path.any = "~/x.json"
[locations.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
"#;
        let err = Rules::load([("x.toml", text)]).unwrap_err();
        assert!(err.to_string().contains("defaults"), "{err}");
    }

    #[test]
    fn a_level_must_fit_its_scope() {
        let text = r#"
id = "x"
name = "X"
kind = "cli"
status = "active"
[defaults]
commands = "asks"
files = "asks"
network = "asks"
mcp = "asks"
[defaults.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
[precedence]
order = ["user"]
status = "undocumented"
[[locations]]
scope = "user"
level = "project"
format = "json"
path.any = "~/x.json"
[locations.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
"#;
        let err = Rules::load([("x.toml", text)]).unwrap_err();
        assert!(err.to_string().contains("does not fit scope"), "{err}");
    }

    #[test]
    fn a_mode_with_an_unknown_state_name_is_refused() {
        let text = r#"
id = "x"
name = "X"
kind = "cli"
status = "active"
[defaults]
commands = "asks"
files = "asks"
network = "asks"
mcp = "asks"
[defaults.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
[precedence]
order = ["user"]
status = "undocumented"
[[locations]]
scope = "user"
level = "user"
format = "json"
path.any = "~/x.json"
[locations.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
[[settings]]
key = "mode"
kind = "mode"
[settings.states.on]
commands = "yes-please"
[settings.source]
kind = "observed"
read_on = 2026-10-16
quote = "seen"
"#;
        let err = Rules::load([("x.toml", text)]).unwrap_err();
        assert!(err.to_string().contains("is not a state"), "{err}");
    }
}
