//! The `bbl-report/1` view of an inventory, for the Blackberrylabs control tower.
//!
//! Nothing here is new: every figure, finding, path and hint is already shown by the CLI or
//! the app, secrets masked upstream. The mapping of Permitt's findings onto the four common
//! severities is in [`severity_of`] and in the CHANGELOG.

use std::collections::BTreeSet;

use bbl_report::{Action, ActionKind, Evidence, Finding, Module, Open, Report, Severity, Subject, SubjectKind, Summary, Tone};

use crate::model::{Capability, FindingKind, HostSheet, Level, State};
use crate::rules::Rules;
use crate::sheet::short;
use crate::{Env, Inventory};

/// Binary name of the app, as packaged.
pub const APP: &str = "permitt-app";
/// Binary name of the command line, as packaged.
pub const CLI: &str = "permitt";

/// Who we are, for the report header.
pub fn module() -> Module {
    Module {
        id: "permitt".into(),
        name: "Permitt".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        verb: "inspect".into(),
        tagline: "What your AI agents may do on this machine without asking".into(),
        app: Some(APP.into()),
        cli: Some(CLI.into()),
        home: "https://github.com/blackberrylabs/permitt".into(),
    }
}

/// Permitt's findings on the common scale.
pub fn severity_of(kind: FindingKind) -> Severity {
    match kind {
        // A flag in a shell file bypasses every prompt of the host, in every project.
        FindingKind::ShellBypass => Severity::ActNow,
        // The agent acts, nobody asks — or a model asks itself.
        FindingKind::RunsWithoutAsking | FindingKind::JudgedByModel => Severity::Review,
        // A setting that does less than it looks: a deny list alone, a key the host ignores.
        FindingKind::DenylistOnly | FindingKind::NotHonoured => Severity::GoodToKnow,
        // Permitt could not tell.
        FindingKind::Unreadable | FindingKind::Contradictory => Severity::Info,
    }
}

/// The slug of a kind, as it appears in the report and in finding ids.
pub fn kind_slug(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::RunsWithoutAsking => "runs-without-asking",
        FindingKind::JudgedByModel => "judged-by-model",
        FindingKind::DenylistOnly => "denylist-only",
        FindingKind::ShellBypass => "shell-bypass",
        FindingKind::NotHonoured => "not-honoured",
        FindingKind::Unreadable => "unreadable",
        FindingKind::Contradictory => "contradictory",
    }
}

/// Build the report of an inventory. `duration_ms` is the time the inventory took.
pub fn build(inv: &Inventory, rules: &Rules, env: &Env, duration_ms: u64) -> Report {
    let mut report = Report::new(module());
    report.duration_ms = duration_ms;

    let auto = inv
        .hosts
        .iter()
        .flat_map(|h| &h.capabilities)
        .filter(|c| c.state == State::Auto)
        .count();
    let shell: usize = inv.hosts.iter().map(|h| h.shell.len()).sum();
    let zero_is_good = |n: usize| if n == 0 { Tone::Good } else { Tone::Alert };
    report.summary = vec![
        Summary { key: "hosts".into(), label: "Hosts installed".into(), value: inv.hosts.len().to_string(), tone: Tone::Neutral },
        Summary { key: "auto".into(), label: "Run without asking".into(), value: auto.to_string(), tone: zero_is_good(auto) },
        Summary { key: "findings".into(), label: "Findings".into(), value: inv.findings.len().to_string(), tone: zero_is_good(inv.findings.len()) },
        Summary { key: "shell".into(), label: "Bypass flags in shell files".into(), value: shell.to_string(), tone: zero_is_good(shell) },
    ];

    let mut ids = BTreeSet::new();
    for f in &inv.findings {
        // `<kind>/<host>/<capability>[/<project>]`; a shell finding is keyed by its flag.
        let third = match (f.kind, f.capability) {
            (FindingKind::ShellBypass, _) => slug(f.message.split('`').nth(1).unwrap_or("-")),
            (_, Some(c)) => c.label().to_string(),
            (_, None) => "-".to_string(),
        };
        let mut id = format!("{}/{}/{third}", kind_slug(f.kind), f.host);
        if let Some(p) = &f.project {
            id.push('/');
            id.push_str(&short(p, env));
        }
        let id = unique(&mut ids, id);
        let (title, detail) = split_message(&f.message);
        let subject_label = match &f.project {
            Some(p) => format!("{} in {}", f.host_name, short(p, env)),
            None => f.host_name.clone(),
        };
        let mut evidence = Vec::new();
        let mut files = BTreeSet::new();
        let mut doc_url = None;
        for e in f.evidence.iter().filter(|e| e.level != Level::Default) {
            let label = match (&e.file, e.line) {
                (Some(p), Some(l)) => format!("{}:{l} — {} = {}", short(p, env), e.key, e.value),
                (Some(p), None) => format!("{} — {} = {}", short(p, env), e.key, e.value),
                _ => format!("{} — {} = {}", e.level, e.key, e.value),
            };
            if let Some(p) = &e.file {
                files.insert(p.clone());
            }
            if doc_url.is_none() {
                doc_url = e.source.url.clone();
            }
            evidence.push(Evidence { label, path: e.file.as_ref().map(|p| p.display().to_string()), line: e.line.map(|l| l as u32) });
        }
        // A shell finding carries no evidence of its own: the hit it comes from has the
        // file, the line and the matching text (secrets masked).
        if f.kind == FindingKind::ShellBypass {
            let hits = inv.hosts.iter().filter(|h| h.host == f.host).flat_map(|h| &h.shell);
            for hit in hits {
                let at = format!("`{}` in {}:{}", hit.flag, short(&hit.file, env), hit.line);
                if f.message.starts_with(&at) {
                    evidence.push(Evidence {
                        label: format!("{}:{} — {}", short(&hit.file, env), hit.line, hit.text),
                        path: Some(hit.file.display().to_string()),
                        line: Some(hit.line as u32),
                    });
                    files.insert(hit.file.clone());
                    if doc_url.is_none() {
                        doc_url = hit.source.url.clone();
                    }
                }
            }
        }
        let mut actions = Vec::new();
        if let Some(act) = rules.host(&f.host).and_then(|r| r.hint("act")) {
            actions.push(Action { kind: ActionKind::Hint, label: act.to_string(), target: None });
        }
        for p in &files {
            actions.push(Action { kind: ActionKind::OpenFile, label: format!("Open {}", short(p, env)), target: Some(p.display().to_string()) });
        }
        if let Some(url) = doc_url {
            actions.push(Action { kind: ActionKind::OpenUrl, label: "Documentation".into(), target: Some(url) });
        }
        report.findings.push(Finding {
            id,
            severity: severity_of(f.kind),
            kind: kind_slug(f.kind).into(),
            title,
            subject: Subject { kind: SubjectKind::Host, label: subject_label },
            detail,
            evidence,
            actions,
            open: Some(open(&f.host, f.capability)),
        });
    }

    // What Permitt cannot read (D10), shown in the app's context line.
    for h in &inv.hosts {
        push_unknown(&mut report, &mut ids, h, rules, env);
    }

    report.notes.push(format!("{} shell file(s) read", inv.shell_files.len()));
    if !inv.roots.is_empty() {
        report.notes.push(format!("{} project root(s) looked at", inv.roots.len()));
    }
    if !inv.not_read.is_empty() {
        report.notes.push(format!("{} host(s) known and not read in this version", inv.not_read.len()));
    }
    for f in inv.files.iter().filter(|f| f.error.is_some()) {
        report.errors.push(format!("{}: {}", short(&f.file, env), f.error.as_deref().unwrap_or_default()));
    }
    report
}

fn push_unknown(report: &mut Report, ids: &mut BTreeSet<String>, h: &HostSheet, rules: &Rules, env: &Env) {
    let caps: Vec<Capability> = h.capabilities.iter().filter(|c| c.state == State::Unknown).map(|c| c.capability).collect();
    if caps.is_empty() {
        return;
    }
    let names: Vec<&str> = caps.iter().map(|c| c.label()).collect();
    // The host's unknown notes (D10), or failing that the note of the default that says so.
    let mut reasons: Vec<String> = h.unknown.clone();
    if reasons.is_empty() {
        for c in h.capabilities.iter().filter(|c| c.state == State::Unknown) {
            for e in c.evidence.iter().filter(|e| e.level == Level::Default) {
                if let Some(n) = &e.note {
                    if !reasons.contains(n) {
                        reasons.push(n.clone());
                    }
                }
            }
        }
    }
    let mut id = format!("unknown/{}", h.host);
    if let Some(p) = &h.project {
        id.push('/');
        id.push_str(&short(p, env));
    }
    let mut actions = Vec::new();
    if let Some(act) = rules.host(&h.host).and_then(|r| r.hint("act")) {
        actions.push(Action { kind: ActionKind::Hint, label: act.to_string(), target: None });
    }
    report.findings.push(Finding {
        id: unique(ids, id),
        severity: Severity::Info,
        kind: "unknown".into(),
        title: format!("{}: {} cannot be read", h.name, names.join(", ")),
        subject: Subject { kind: SubjectKind::Host, label: h.name.clone() },
        detail: reasons.join(" · "),
        evidence: Vec::new(),
        actions,
        open: Some(open(&h.host, caps.first().copied())),
    });
}

fn open(host: &str, cap: Option<Capability>) -> Open {
    let target = match cap {
        Some(c) => format!("{host}:{}", c.label()),
        None => host.to_string(),
    };
    Open { app: APP.into(), args: vec!["--select".into(), target] }
}

/// `"Title — detail"` → (`Title`, `detail`); a message without a dash is all title.
fn split_message(m: &str) -> (String, String) {
    match m.split_once(" — ") {
        Some((t, d)) => (t.to_string(), d.to_string()),
        None => (m.to_string(), String::new()),
    }
}

/// Lowercase ASCII letters, digits and `-`, one dash between words.
fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn unique(ids: &mut BTreeSet<String>, id: String) -> String {
    let mut candidate = id.clone();
    let mut n = 2;
    while !ids.insert(candidate.clone()) {
        candidate = format!("{id}#{n}");
        n += 1;
    }
    candidate
}
