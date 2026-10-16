//! The common model: four capabilities, one state per capability, and the evidence each
//! state rests on. A state is a fact with a source, never a score.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What an assistant may do on the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    /// Run shell commands.
    Commands,
    /// Create, modify, delete files.
    Files,
    /// Reach the network (web fetch, search, outbound connections of the agent).
    Network,
    /// Call MCP tools.
    Mcp,
}

impl Capability {
    pub const ALL: [Capability; 4] = [
        Capability::Commands,
        Capability::Files,
        Capability::Network,
        Capability::Mcp,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Capability::Commands => "commands",
            Capability::Files => "files",
            Capability::Network => "network",
            Capability::Mcp => "mcp",
        }
    }

    pub fn parse(s: &str) -> Option<Capability> {
        match s {
            "commands" => Some(Capability::Commands),
            "files" => Some(Capability::Files),
            "network" => Some(Capability::Network),
            "mcp" => Some(Capability::Mcp),
            _ => None,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Whether the host asks before acting. Ordered from "the user decides" to "nobody does",
/// with the undecidable last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// The capability is disabled or denied.
    Off,
    /// Asks every time.
    Asks,
    /// Acts without asking inside an isolation mechanism (sandbox).
    Sandboxed,
    /// Acts without asking for the listed patterns, asks otherwise.
    Allowlist(usize),
    /// Acts without asking except for the listed patterns — which a command can work around.
    Denylist(usize),
    /// A classifier (a model) decides at each call; the user is not consulted.
    Judged,
    /// Acts without asking.
    Auto,
    /// The setting could not be read or is not documented; the reason is in the evidence.
    Unknown,
}

impl State {
    /// Parse the spelling used in rule files: `asks`, `auto`, `allowlist`, `denylist`,
    /// `judged`, `sandboxed`, `off`, `unknown`. Counts are filled from the data.
    pub fn parse(s: &str) -> Option<State> {
        Some(match s {
            "off" => State::Off,
            "asks" => State::Asks,
            "sandboxed" => State::Sandboxed,
            "allowlist" => State::Allowlist(0),
            "denylist" => State::Denylist(0),
            "judged" => State::Judged,
            "auto" => State::Auto,
            "unknown" => State::Unknown,
            _ => return None,
        })
    }

    pub fn label(self) -> String {
        match self {
            State::Off => "off".into(),
            State::Asks => "asks".into(),
            State::Sandboxed => "sandboxed".into(),
            State::Allowlist(0) => "allowlist".into(),
            State::Allowlist(n) => format!("allowlist({n})"),
            State::Denylist(0) => "denylist".into(),
            State::Denylist(n) => format!("denylist({n})"),
            State::Judged => "judged".into(),
            State::Auto => "auto".into(),
            State::Unknown => "unknown".into(),
        }
    }

    /// Does the host act without asking the user for at least part of this capability?
    pub fn runs_without_asking(self) -> bool {
        matches!(
            self,
            State::Auto
                | State::Judged
                | State::Denylist(_)
                | State::Allowlist(_)
                | State::Sandboxed
        )
    }

    /// Rank for ordering: what runs without asking first.
    pub fn rank(self) -> u8 {
        match self {
            State::Auto => 0,
            State::Denylist(_) => 1,
            State::Judged => 2,
            State::Allowlist(_) => 3,
            State::Sandboxed => 4,
            State::Asks => 5,
            State::Off => 6,
            State::Unknown => 7,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// Where a setting was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Organisation policy (managed settings, MDM, system directory).
    Managed,
    User,
    /// Shared project file, versioned.
    Project,
    /// Project file kept out of version control.
    Local,
    /// A shell start-up file (alias, exported variable).
    Shell,
    /// The built-in behaviour when nothing sets it.
    Default,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Managed => "managed",
            Level::User => "user",
            Level::Project => "project",
            Level::Local => "local",
            Level::Shell => "shell",
            Level::Default => "default",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Where a claim was read: the rule's source, carried into the output so every line can be
/// checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: bbl_hostrules::SourceKind,
    pub url: Option<String>,
    pub quote: String,
}

impl From<&bbl_hostrules::Source> for SourceRef {
    fn from(s: &bbl_hostrules::Source) -> Self {
        SourceRef {
            kind: s.kind,
            url: s.url.clone(),
            quote: s.quote.clone(),
        }
    }
}

/// One fact behind a state: a key in a file, with its (masked) value, or a default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub level: Level,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub key: String,
    /// The value as it may be shown (secrets masked).
    pub value: String,
    pub note: Option<String>,
    pub source: SourceRef,
}

/// The state of one capability for one host, with what it rests on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityState {
    pub capability: Capability,
    pub state: State,
    /// True when no setting changed the built-in behaviour.
    pub is_default: bool,
    pub evidence: Vec<Evidence>,
    /// Settings found in a file where the host does not honour them (shown, not applied).
    pub ignored: Vec<Evidence>,
}

/// A safeguard present on the host (hooks, a sandbox turned on, a deny list), shown as such.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    pub name: String,
    pub evidence: Evidence,
}

/// A dangerous flag or variable found in a shell start-up file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHit {
    pub host: String,
    pub file: PathBuf,
    pub line: usize,
    /// The matching line, trimmed; secrets masked.
    pub text: String,
    pub flag: String,
    pub effect: String,
    pub source: SourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrecedenceStatus {
    Documented,
    Undocumented,
    Contradictory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecedenceInfo {
    pub order: Vec<Level>,
    pub status: PrecedenceStatus,
    pub note: Option<String>,
    pub source: Option<SourceRef>,
}

/// The sheet of one host in one context (no project, or one project root).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSheet {
    pub host: String,
    pub name: String,
    pub kind: String,
    /// Any of its files found, or its binary on `PATH`.
    pub installed: bool,
    pub project: Option<PathBuf>,
    pub capabilities: Vec<CapabilityState>,
    pub guards: Vec<Guard>,
    pub shell: Vec<ShellHit>,
    pub precedence: PrecedenceInfo,
    /// Reasons a part of the state cannot be read (D10).
    pub unknown: Vec<String>,
}

impl HostSheet {
    pub fn state(&self, c: Capability) -> Option<&CapabilityState> {
        self.capabilities.iter().find(|s| s.capability == c)
    }

    /// Highest-ranked (least asking) state of the host, for ordering.
    pub fn rank(&self) -> u8 {
        self.capabilities
            .iter()
            .map(|s| s.state.rank())
            .min()
            .unwrap_or(7)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// A capability acts without asking, for everything.
    RunsWithoutAsking,
    /// A capability is decided by a model, not the user.
    JudgedByModel,
    /// Only a deny list stands between the agent and the capability.
    DenylistOnly,
    /// A bypass flag or variable in a shell start-up file.
    ShellBypass,
    /// A setting written where the host does not honour it.
    NotHonoured,
    /// A file could not be read.
    Unreadable,
    /// The documentation contradicts itself on this point.
    Contradictory,
}

impl FindingKind {
    pub fn severity(self) -> u8 {
        match self {
            FindingKind::ShellBypass => 0,
            FindingKind::RunsWithoutAsking => 1,
            FindingKind::DenylistOnly => 2,
            FindingKind::JudgedByModel => 3,
            FindingKind::NotHonoured => 4,
            FindingKind::Unreadable => 5,
            FindingKind::Contradictory => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FindingKind::RunsWithoutAsking => "RUNS WITHOUT ASKING",
            FindingKind::JudgedByModel => "A MODEL DECIDES",
            FindingKind::DenylistOnly => "DENY LIST ONLY",
            FindingKind::ShellBypass => "SHELL BYPASS",
            FindingKind::NotHonoured => "NOT HONOURED HERE",
            FindingKind::Unreadable => "UNREADABLE",
            FindingKind::Contradictory => "DOC CONTRADICTS ITSELF",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub host: String,
    pub host_name: String,
    pub capability: Option<Capability>,
    pub project: Option<PathBuf>,
    pub message: String,
    pub evidence: Vec<Evidence>,
}

/// A file that was looked at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub host: String,
    pub host_name: String,
    pub level: Level,
    pub file: PathBuf,
    pub exists: bool,
    pub error: Option<String>,
    pub settings_found: usize,
    pub source_kind: bbl_hostrules::SourceKind,
}
