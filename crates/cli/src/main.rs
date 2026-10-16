//! Permitt command line: hosts | roster | findings | files | shell, `--json` for scripts.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use permitt_core::env::Os;
use permitt_core::model::{Capability, HostSheet, Level, State};
use permitt_core::{inventory, rules, Env, Inventory, Options};

#[derive(Parser)]
#[command(
    name = "permitt",
    version,
    about = "What each AI assistant on this machine may do without asking, and which setting says so. Read-only."
)]
struct Cli {
    /// Machine-readable output.
    #[arg(long, global = true)]
    json: bool,
    /// Inventory another home directory (a backup, a fixture) instead of yours.
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    /// Project roots to look at, in addition to the ones Claude Code knows.
    #[arg(long, global = true)]
    root: Vec<PathBuf>,
    /// Do not read shell start-up files.
    #[arg(long, global = true)]
    no_shell: bool,
    /// Also show hosts with no file and no binary on this machine.
    #[arg(long, global = true)]
    all: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// The sheet: one row per host, one column per capability (default).
    Roster,
    /// What runs without asking, and why — most serious first.
    Findings,
    /// The files that were looked at.
    Files,
    /// What Permitt knows about each host, and how.
    Hosts,
    /// The bypass flags found in shell start-up files.
    Shell,
}

macro_rules! out {
    ($($arg:tt)*) => {{
        let mut o = std::io::stdout().lock();
        let _ = writeln!(o, $($arg)*);
    }};
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let rules = rules::embedded();
    let shell = rules::embedded_shell();
    let mut env = match &cli.home {
        Some(h) => {
            let mut e = Env::fixture(h, Os::Linux);
            // A foreign home still lives on this machine: use its PATH too.
            e.path.extend(Env::current().path);
            e
        }
        None => Env::current(),
    };
    env.roots.extend(cli.root.iter().cloned());
    let opts = Options {
        no_shell: cli.no_shell,
        all_hosts: cli.all,
    };

    if matches!(cli.command, Some(Command::Hosts)) {
        return hosts(&rules, &shell, cli.json);
    }
    let inv = inventory::run(&rules, &shell, &env, opts);
    match cli.command.unwrap_or(Command::Roster) {
        Command::Roster => roster(&inv, &env, cli.json),
        Command::Findings => findings(&inv, &env, cli.json),
        Command::Files => files(&inv, &env, cli.json),
        Command::Shell => shell_cmd(&inv, &env, cli.json),
        Command::Hosts => unreachable!(),
    }
    Ok(())
}

fn short(p: &std::path::Path, env: &Env) -> String {
    permitt_core::sheet::short(p, env)
}

fn roster(inv: &Inventory, env: &Env, json: bool) {
    if json {
        out!("{}", serde_json::to_string_pretty(inv).unwrap());
        return;
    }
    let n_auto = inv
        .hosts
        .iter()
        .flat_map(|h| &h.capabilities)
        .filter(|c| c.state == State::Auto)
        .count();
    out!(
        "{} host(s) installed, {} capability(ies) run without asking, {} finding(s)",
        inv.hosts.len(),
        n_auto,
        inv.findings.len()
    );
    if inv.hosts.is_empty() {
        out!("No assistant found. Use --all to list every host Permitt knows.");
        return;
    }
    out!();
    table(&inv.hosts, env);
    for p in &inv.projects {
        out!();
        out!("In {} (project level):", short(&p.root, env));
        table(&p.hosts, env);
    }
    out!();
    out!("asks: every time · auto: without asking · judged: a model decides · allowlist(n) / denylist(n): except n patterns · sandboxed: without asking, inside a sandbox · off · unknown: see `permitt findings` and `permitt hosts`");
    out!("(default): nothing changed the built-in behaviour. Permitt changes nothing: act in each host's own settings.");
}

fn table(hosts: &[HostSheet], env: &Env) {
    let w = hosts.iter().map(|h| h.name.len()).max().unwrap_or(4).max(4);
    out!(
        "{:<w$}  {:<18} {:<18} {:<18} {:<18}",
        "Host",
        "commands",
        "files",
        "network",
        "mcp",
        w = w
    );
    for h in hosts {
        let cell = |c: Capability| -> String {
            let s = h.state(c).unwrap();
            let mut t = s.state.label();
            if s.is_default {
                t.push_str(" (default)");
            }
            t
        };
        out!(
            "{:<w$}  {:<18} {:<18} {:<18} {:<18}",
            h.name,
            cell(Capability::Commands),
            cell(Capability::Files),
            cell(Capability::Network),
            cell(Capability::Mcp),
            w = w
        );
        for hit in &h.shell {
            out!(
                "{:<w$}  ⚑ {}:{} `{}` — {}",
                "",
                short(&hit.file, env),
                hit.line,
                hit.flag,
                hit.effect,
                w = w
            );
        }
        if !h.guards.is_empty() {
            let names: Vec<&str> = h.guards.iter().map(|g| g.name.as_str()).collect();
            out!("{:<w$}  guards: {}", "", names.join(", "), w = w);
        }
    }
}

fn findings(inv: &Inventory, env: &Env, json: bool) {
    if json {
        out!("{}", serde_json::to_string_pretty(&inv.findings).unwrap());
        return;
    }
    if inv.findings.is_empty() {
        out!("No finding: every installed assistant asks before acting, as far as its files say.");
        return;
    }
    for f in &inv.findings {
        out!("{:<22} {}", f.kind.label(), f.message);
        for e in f.evidence.iter().filter(|e| e.level != Level::Default) {
            let loc = match (&e.file, e.line) {
                (Some(p), Some(l)) => format!("{}:{l}", short(p, env)),
                (Some(p), None) => short(p, env),
                _ => e.level.to_string(),
            };
            out!(
                "{:<22}   {loc} `{}` = {}{}",
                "",
                e.key,
                e.value,
                e.note
                    .as_ref()
                    .map(|n| format!(" — {n}"))
                    .unwrap_or_default()
            );
        }
        if let Some(e) = f.evidence.iter().find(|e| e.level == Level::Default) {
            out!("{:<22}   default — {}", "", e.source.quote);
        }
    }
    out!();
    out!(
        "{} finding(s). Permitt changes nothing: use each host's own settings to act.",
        inv.findings.len()
    );
}

fn files(inv: &Inventory, env: &Env, json: bool) {
    if json {
        out!("{}", serde_json::to_string_pretty(&inv.files).unwrap());
        return;
    }
    let mut files = inv.files.clone();
    files.sort_by(|a, b| {
        (!a.exists, &a.host_name, &a.file).cmp(&(!b.exists, &b.host_name, &b.file))
    });
    for f in &files {
        let status = match (&f.error, f.exists) {
            (Some(e), _) => format!("unreadable: {e}"),
            (None, true) => format!("{} setting(s)", f.settings_found),
            (None, false) => "absent".to_string(),
        };
        let observed = if f.source_kind == hostrules::SourceKind::Observed {
            " [observed]"
        } else {
            ""
        };
        out!(
            "{:<30} {:<8} {:<40} {status}{observed}",
            f.host_name,
            f.level,
            short(&f.file, env)
        );
    }
    for s in &inv.shell_files {
        out!(
            "{:<30} {:<8} {:<40} read",
            "(shell)",
            "shell",
            short(s, env)
        );
    }
}

fn shell_cmd(inv: &Inventory, env: &Env, json: bool) {
    let hits: Vec<_> = inv.hosts.iter().flat_map(|h| h.shell.iter()).collect();
    if json {
        out!("{}", serde_json::to_string_pretty(&hits).unwrap());
        return;
    }
    out!(
        "Read: {}",
        inv.shell_files
            .iter()
            .map(|p| short(p, env))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if hits.is_empty() {
        out!("No bypass flag or variable found.");
        return;
    }
    for h in hits {
        out!(
            "{}:{}  [{}] `{}` — {}",
            short(&h.file, env),
            h.line,
            h.host,
            h.flag,
            h.effect
        );
        out!("    {}", h.text);
    }
}

fn hosts(rules: &rules::Rules, shell: &rules::ShellRules, json: bool) -> Result<()> {
    if json {
        out!("{}", serde_json::to_string_pretty(&rules.hosts).unwrap());
        return Ok(());
    }
    for h in &rules.hosts {
        let d = h.ext.defaults.as_ref().unwrap();
        out!(
            "{} ({}) — binaries: {}",
            h.name,
            h.id,
            h.ext.binaries.join(", ")
        );
        out!(
            "  defaults: commands {} · files {} · network {} · mcp {}{}",
            d.commands,
            d.files,
            d.network,
            d.mcp,
            if d.contradictory {
                " (documentation contradicts itself)"
            } else {
                ""
            }
        );
        if let Some(n) = &d.note {
            out!("    {n}");
        }
        for l in &h.locations {
            let paths: Vec<String> = [l.path.any.as_ref(), l.path.linux.as_ref()]
                .into_iter()
                .flatten()
                .flat_map(|p| p.iter().map(String::from))
                .collect();
            out!(
                "  {:<8} {} [{:?}]",
                l.ext.level,
                paths.join(" | "),
                l.source.kind
            );
        }
        for s in &h.ext.settings {
            out!(
                "  setting  {} ({:?}){}",
                s.key,
                s.kind,
                s.guard
                    .as_ref()
                    .map(|g| format!(" — guard: {g}"))
                    .unwrap_or_default()
            );
        }
        if let Some(p) = &h.ext.precedence {
            out!(
                "  precedence: {} ({:?}){}",
                p.order
                    .iter()
                    .map(|l| l.label())
                    .collect::<Vec<_>>()
                    .join(" > "),
                p.status,
                p.note
                    .as_ref()
                    .map(|n| format!(" — {n}"))
                    .unwrap_or_default()
            );
        }
        for u in &h.ext.unknown {
            out!("  unknown: {}", u.reason);
        }
        out!();
    }
    out!(
        "Not read: {}",
        rules
            .not_read
            .iter()
            .map(|n| format!("{} ({})", n.name, n.reason))
            .collect::<Vec<_>>()
            .join("; ")
    );
    out!(
        "Shell files: {} · flags looked for: {}",
        shell.files.join(", "),
        shell.flags.len()
    );
    Ok(())
}
