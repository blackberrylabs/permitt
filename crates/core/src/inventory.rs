//! The inventory: every host's sheet without a project, then for every known project root;
//! the shell scan; the findings, ordered by what runs without asking first.

use std::path::PathBuf;

use bbl_hostrules::resolve::project_roots;
use bbl_hostrules::Env;
use serde::{Deserialize, Serialize};

use crate::model::{Capability, FileStatus, Finding, FindingKind, HostSheet, State};
use crate::rules::{Rules, ShellRules};
use crate::sheet::{build, short};
use crate::shell::scan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSheets {
    pub root: PathBuf,
    pub hosts: Vec<HostSheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// One sheet per host, user and managed levels only.
    pub hosts: Vec<HostSheet>,
    /// Per project root, the sheets of the hosts whose state differs there.
    pub projects: Vec<ProjectSheets>,
    pub files: Vec<FileStatus>,
    pub shell_files: Vec<PathBuf>,
    pub findings: Vec<Finding>,
    pub roots: Vec<PathBuf>,
    pub not_read: Vec<bbl_hostrules::rules::NotRead>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Skip the shell start-up files.
    pub no_shell: bool,
    /// Show hosts with no file found and no binary.
    pub all_hosts: bool,
}

pub fn run(rules: &Rules, shell_rules: &ShellRules, env: &Env, opts: Options) -> Inventory {
    let roots = project_roots(env);
    let mut hosts = Vec::new();
    let mut files = Vec::new();
    let mut findings = Vec::new();

    for rule in &rules.hosts {
        let built = build(rule, env, None);
        files.extend(built.files);
        findings.extend(built.findings);
        hosts.push(built.sheet);
    }

    // Shell scan, attached to the host each hit names.
    let mut shell_files = Vec::new();
    if !opts.no_shell {
        let s = scan(shell_rules, env);
        shell_files = s.files_read;
        for hit in s.hits {
            findings.push(Finding {
                kind: FindingKind::ShellBypass,
                host: hit.host.clone(),
                host_name: rules.name_of(&hit.host).to_string(),
                capability: None,
                project: None,
                message: format!(
                    "`{}` in {}:{} — {}",
                    hit.flag,
                    short(&hit.file, env),
                    hit.line,
                    hit.effect
                ),
                evidence: Vec::new(),
            });
            if let Some(h) = hosts.iter_mut().find(|h| h.host == hit.host) {
                h.shell.push(hit);
            }
        }
    }

    // Projects: only the hosts whose state changes there.
    let mut projects = Vec::new();
    for root in &roots {
        let mut sheets = Vec::new();
        for rule in &rules.hosts {
            let built = build(rule, env, Some(root));
            let base = hosts.iter().find(|h| h.host == rule.id);
            let differs = base.map(|b| differs(b, &built.sheet)).unwrap_or(true);
            // Files looked at under a project are always listed.
            files.extend(built.files.into_iter().filter(|f| f.file.starts_with(root)));
            findings.extend(built.findings);
            if differs {
                sheets.push(built.sheet);
            }
        }
        if !sheets.is_empty() {
            projects.push(ProjectSheets {
                root: root.clone(),
                hosts: sheets,
            });
        }
    }

    if !opts.all_hosts {
        hosts.retain(|h| h.installed);
        for p in &mut projects {
            p.hosts.retain(|h| h.installed);
        }
        projects.retain(|p| !p.hosts.is_empty());
        let kept: Vec<String> = hosts.iter().map(|h| h.host.clone()).collect();
        findings.retain(|f| kept.contains(&f.host));
    }

    // State findings, for the user context and for each project.
    for h in &hosts {
        state_findings(h, None, env, &mut findings);
    }
    for p in &projects {
        for h in &p.hosts {
            // Only what the project changes: a state equal to the user-level one is not
            // a second finding.
            let base = hosts.iter().find(|b| b.host == h.host);
            state_findings(h, base, env, &mut findings);
        }
    }
    dedupe(&mut findings);
    findings.sort_by_key(|f| {
        (
            f.kind.severity(),
            f.host_name.to_lowercase(),
            f.message.clone(),
        )
    });
    hosts.sort_by_key(|h| (h.rank(), h.name.to_lowercase()));

    Inventory {
        hosts,
        projects,
        files,
        shell_files,
        findings,
        roots,
        not_read: rules.not_read.clone(),
    }
}

fn differs(a: &HostSheet, b: &HostSheet) -> bool {
    if a.capabilities.len() != b.capabilities.len() || a.guards.len() != b.guards.len() {
        return true;
    }
    a.capabilities.iter().zip(&b.capabilities).any(|(x, y)| {
        x.state != y.state || x.is_default != y.is_default || x.ignored.len() != y.ignored.len()
    })
}

fn state_findings(h: &HostSheet, base: Option<&HostSheet>, env: &Env, out: &mut Vec<Finding>) {
    for c in &h.capabilities {
        if let Some(b) = base {
            if b.state(c.capability).map(|s| s.state) == Some(c.state) {
                continue;
            }
        }
        let (kind, verb) = match c.state {
            State::Auto => (FindingKind::RunsWithoutAsking, "without asking"),
            State::Judged => (FindingKind::JudgedByModel, "when a model allows it"),
            State::Denylist(_) => (FindingKind::DenylistOnly, "unless a deny list matches"),
            _ => continue,
        };
        let what = match c.capability {
            Capability::Commands => "runs shell commands",
            Capability::Files => "edits files",
            Capability::Network => "reaches the network",
            Capability::Mcp => "calls MCP tools",
        };
        let where_ = match &h.project {
            Some(p) => format!(" in {}", short(p, env)),
            None => String::new(),
        };
        // The evidence that set the state: the first entry of the strongest level present
        // (levels are applied weakest first, so the last entry's level is the strongest).
        let strongest = c
            .evidence
            .iter()
            .rev()
            .find(|e| e.level != crate::model::Level::Default)
            .map(|e| e.level);
        let because = strongest
            .and_then(|lvl| c.evidence.iter().find(|e| e.level == lvl))
            .map(|e| {
                format!(
                    " — `{}` = {}{}",
                    e.key,
                    e.value,
                    e.file
                        .as_ref()
                        .map(|f| format!(
                            " ({}{})",
                            short(f, env),
                            e.line.map(|l| format!(":{l}")).unwrap_or_default()
                        ))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_else(|| {
                if c.is_default {
                    " — by default, nothing changed it".to_string()
                } else {
                    String::new()
                }
            });
        out.push(Finding {
            kind,
            host: h.host.clone(),
            host_name: h.name.clone(),
            capability: Some(c.capability),
            project: h.project.clone(),
            message: format!("{} {what} {verb}{where_}{because}", h.name),
            evidence: c.evidence.clone(),
        });
    }
}

fn dedupe(findings: &mut Vec<Finding>) {
    let mut seen = std::collections::BTreeSet::new();
    findings.retain(|f| {
        seen.insert((
            f.kind.label().to_string(),
            f.host.clone(),
            f.message.clone(),
        ))
    });
}
