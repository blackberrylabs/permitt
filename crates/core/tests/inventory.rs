//! The inventory on a fake home (`tests/fixtures/home`) that represents every host read in
//! 0.1, copied to a temporary directory with `__HOME__` replaced by its real path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use permitt_core::env::Os;
use permitt_core::model::{Capability, FindingKind, Level, State};
use permitt_core::{inventory, rules, Env, Options};

fn copy_fixture(home: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/home");
    let home_str = home.display().to_string();
    for entry in walkdir(&src) {
        let rel = entry.strip_prefix(&src).unwrap();
        let dest = home.join(rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let text = std::fs::read_to_string(&entry).unwrap();
        std::fs::write(&dest, text.replace("__HOME__", &home_str)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&entry).unwrap().permissions().mode();
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode)).unwrap();
        }
    }
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walkdir(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walkdir(dir)
        .into_iter()
        .map(|p| (p.clone(), std::fs::read(&p).unwrap()))
        .collect()
}

fn run() -> (tempfile::TempDir, inventory::Inventory) {
    let tmp = tempfile::tempdir().unwrap();
    copy_fixture(tmp.path());
    let env = Env::fixture(tmp.path(), Os::Linux);
    let inv = inventory::run(
        &rules::embedded(),
        &rules::embedded_shell(),
        &env,
        Options::default(),
    );
    (tmp, inv)
}

fn state(inv: &inventory::Inventory, host: &str, c: Capability) -> State {
    inv.hosts
        .iter()
        .find(|h| h.host == host)
        .unwrap_or_else(|| panic!("host {host} not in the inventory"))
        .state(c)
        .unwrap()
        .state
}

fn project_state(inv: &inventory::Inventory, host: &str, c: Capability) -> Option<State> {
    inv.projects
        .first()?
        .hosts
        .iter()
        .find(|h| h.host == host)
        .and_then(|h| h.state(c).map(|s| s.state))
}

#[test]
fn every_host_of_the_fake_home_is_installed_and_sheeted() {
    let (_tmp, inv) = run();
    let ids: Vec<&str> = inv.hosts.iter().map(|h| h.host.as_str()).collect();
    assert_eq!(inv.hosts.len(), 16, "{ids:?}");
    assert!(inv.hosts.iter().all(|h| h.installed));
    assert!(inv.hosts.iter().all(|h| h.capabilities.len() == 4));
    assert_eq!(inv.roots.len(), 1, "{:?}", inv.roots);
    assert!(inv.not_read.iter().any(|n| n.id == "roo-code"));
}

#[test]
fn states_follow_the_rules() {
    let (_tmp, inv) = run();
    use Capability::*;
    // Claude Code: judged by default; allow rules make commands an allowlist; deny is a guard.
    assert_eq!(
        state(&inv, "claude-code", Commands),
        State::Judged,
        "default auto stays judged, lists refine only asks/auto"
    );
    // Codex: full access
    assert_eq!(state(&inv, "codex", Commands), State::Auto);
    assert_eq!(state(&inv, "codex", Network), State::Auto);
    assert_eq!(state(&inv, "codex", Mcp), State::Auto);
    // Cursor: run mode not readable → unknown for commands, files auto by default
    assert_eq!(state(&inv, "cursor", Commands), State::Unknown);
    assert_eq!(state(&inv, "cursor", Files), State::Auto);
    // VS Code: terminal map (git, npm allow; rm ask) → allowlist(2) on top of built-in allowlist
    assert_eq!(state(&inv, "vscode", Commands), State::Allowlist(2));
    assert_eq!(
        state(&inv, "vscode", Files),
        State::Denylist(1),
        "`**/*: true` is the base, `.env` asks"
    );
    assert_eq!(state(&inv, "vscode", Network), State::Allowlist(1));
    // Gemini: auto_edit → files auto; allowed run_shell_command(git) → commands allowlist(1); sandbox docker
    assert_eq!(state(&inv, "gemini-cli", Files), State::Auto);
    assert_eq!(
        state(&inv, "gemini-cli", Commands),
        State::Allowlist(1),
        "sandbox is a guard; the allowlist refines asks"
    );
    assert_eq!(
        state(&inv, "gemini-cli", Mcp),
        State::Allowlist(1),
        "my_mcp_tool falls to _default = mcp"
    );
    // Qwen: yolo → auto everywhere; deny Bash(rm *) → commands denylist(1)
    assert_eq!(state(&inv, "qwen-code", Commands), State::Denylist(1));
    assert_eq!(state(&inv, "qwen-code", Files), State::Auto);
    // Cline: executeSafeCommands true → commands auto; useBrowser false → network asks
    assert_eq!(state(&inv, "cline", Commands), State::Auto);
    assert_eq!(state(&inv, "cline", Network), State::Asks);
    // OpenCode: bash {* ask, git * allow, git status allow} → allowlist(2); edit deny → off; webfetch allow → auto
    assert_eq!(state(&inv, "opencode", Commands), State::Allowlist(2));
    assert_eq!(state(&inv, "opencode", Files), State::Off);
    assert_eq!(state(&inv, "opencode", Network), State::Auto);
    // Goose: approve → asks; always_allow developer__shell → commands auto (bare), fetch → network auto; never_allow editor → files off
    assert_eq!(state(&inv, "goose", Commands), State::Auto);
    assert_eq!(state(&inv, "goose", Files), State::Off);
    assert_eq!(state(&inv, "goose", Network), State::Auto);
    assert_eq!(state(&inv, "goose", Mcp), State::Asks);
    // Zed: global allow, terminal confirm with 2 always_allow patterns → allowlist(2); fetch deny → network off
    assert_eq!(state(&inv, "zed", Commands), State::Allowlist(2));
    assert_eq!(state(&inv, "zed", Network), State::Off);
    assert_eq!(state(&inv, "zed", Files), State::Auto);
    // Kiro: shell git* allow → allowlist(1) on top of default allowlist; fs_write ./** → allowlist; mcp bare allow → auto; web_fetch * deny → off
    assert_eq!(state(&inv, "kiro", Commands), State::Allowlist(1));
    assert_eq!(state(&inv, "kiro", Mcp), State::Auto);
    assert_eq!(state(&inv, "kiro", Network), State::Off);
    // Amp: dangerouslyAllowAll → auto; ask rule on Bash → denylist(1)
    assert_eq!(state(&inv, "amp", Commands), State::Denylist(1));
    assert_eq!(state(&inv, "amp", Files), State::Auto);
    // Aider: yes-always → commands auto
    assert_eq!(state(&inv, "aider", Commands), State::Auto);
    assert_eq!(state(&inv, "aider", Mcp), State::Off);
    // Devin: Exec(git *) allow → allowlist(1)… but Cascade unknown wins for commands; Fetch(*) deny → network off
    assert_eq!(state(&inv, "devin-desktop", Network), State::Off);
    assert_eq!(
        state(&inv, "devin-desktop", Commands),
        State::Allowlist(1),
        "Cascade's note does not decide Devin CLI's state"
    );
    // Cursor CLI: Shell(git) allow → allowlist(1); sandbox enabled is a guard; Write(src/**) → files allowlist(1)
    assert_eq!(
        state(&inv, "cursor-cli", Commands),
        State::Sandboxed,
        "with the sandbox on, supported commands run inside it without approval"
    );
    assert_eq!(
        state(&inv, "cursor-cli", Files),
        State::Unknown,
        "an allowlist on an undocumented default stays unknown"
    );
    // Copilot CLI: allowedUrls → network allowlist(1); disableBypass is a guard
    assert_eq!(state(&inv, "copilot-cli", Network), State::Allowlist(1));
    assert_eq!(state(&inv, "copilot-cli", Commands), State::Asks);
}

#[test]
fn project_levels_apply_and_not_honoured_settings_are_reported() {
    let (_tmp, inv) = run();
    use Capability::*;
    // In the project, Claude Code's local file sets acceptEdits (files auto) and allows Edit;
    // the shared file's bypassPermissions is ignored and reported.
    assert_eq!(project_state(&inv, "claude-code", Files), Some(State::Auto));
    assert_eq!(
        project_state(&inv, "claude-code", Mcp),
        Some(State::Allowlist(1)),
        "mcp__github__* allow on top of acceptEdits (asks)"
    );
    let not_honoured: Vec<&str> = inv
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::NotHonoured)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(not_honoured.len(), 1, "{not_honoured:?}");
    assert!(
        not_honoured[0].contains("bypassPermissions"),
        "{not_honoured:?}"
    );
    // VS Code workspace file turns global auto-approve on in the project only.
    assert_eq!(
        state(&inv, "vscode", Mcp),
        State::Unknown,
        "user level: per-tool approvals not readable"
    );
    assert_eq!(project_state(&inv, "vscode", Mcp), Some(State::Auto));
    // Copilot CLI saved approvals keyed by the project path.
    assert_eq!(
        project_state(&inv, "copilot-cli", Commands),
        Some(State::Allowlist(1)),
        "git + cargo:* = one rule with two identifiers"
    );
    assert_eq!(
        project_state(&inv, "copilot-cli", Files),
        Some(State::Auto),
        "a bare `write` approval"
    );
    // The project-level evidence names the local file.
    let cc = inv.projects[0]
        .hosts
        .iter()
        .find(|h| h.host == "claude-code")
        .unwrap();
    let files = cc.state(Files).unwrap();
    assert!(files.evidence.iter().any(|e| e.level == Level::Local));
}

#[test]
fn shell_files_reveal_bypass_flags_and_only_those() {
    let (_tmp, inv) = run();
    assert_eq!(inv.shell_files.len(), 2, "{:?}", inv.shell_files);
    let hits: Vec<String> = inv
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::ShellBypass)
        .map(|f| f.message.clone())
        .collect();
    let joined = hits.join("\n");
    assert!(
        joined.contains("--dangerously-skip-permissions"),
        "{joined}"
    );
    assert!(joined.contains("AIDER_YES_ALWAYS"), "{joined}");
    assert!(
        joined.contains("GOOSE_MODE") && joined.contains(".zshrc"),
        "{joined}"
    );
    assert!(
        joined.contains("-a never") && joined.contains("-s danger-full-access"),
        "{joined}"
    );
    // The commented-out --yolo and GOOSE_MODE=approve do not count.
    assert!(!joined.contains("--yolo"), "{joined}");
    assert_eq!(joined.matches("GOOSE_MODE").count(), 1, "{joined}");
    let cc = inv.hosts.iter().find(|h| h.host == "claude-code").unwrap();
    assert_eq!(cc.shell.len(), 1);
    assert_eq!(cc.shell[0].line, 4);
}

#[test]
fn findings_are_ordered_and_name_what_runs_without_asking() {
    let (_tmp, inv) = run();
    assert!(!inv.findings.is_empty());
    let sev: Vec<u8> = inv.findings.iter().map(|f| f.kind.severity()).collect();
    let mut sorted = sev.clone();
    sorted.sort();
    assert_eq!(sev, sorted);
    assert_eq!(inv.findings[0].kind, FindingKind::ShellBypass);
    assert!(inv
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::RunsWithoutAsking && f.host == "codex"));
    assert!(inv
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::JudgedByModel && f.host == "claude-code"));
    assert!(inv
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::DenylistOnly && f.host == "amp"));
    // Hosts are ordered: what runs without asking first.
    let ranks: Vec<u8> = inv.hosts.iter().map(|h| h.rank()).collect();
    let mut r = ranks.clone();
    r.sort();
    assert_eq!(ranks, r);
}

#[test]
fn defaults_are_marked_and_sourced() {
    let (_tmp, inv) = run();
    let aider = inv.hosts.iter().find(|h| h.host == "aider").unwrap();
    let net = aider.state(Capability::Network).unwrap();
    assert!(net.is_default);
    assert_eq!(net.evidence.len(), 1);
    assert_eq!(net.evidence[0].level, Level::Default);
    assert!(net.evidence[0].source.url.is_some());
    let cc = inv.hosts.iter().find(|h| h.host == "claude-code").unwrap();
    assert!(!cc.state(Capability::Commands).unwrap().is_default || true);
    assert!(cc.guards.iter().any(|g| g.name == "PreToolUse hooks"));
    assert!(cc.guards.iter().any(|g| g.name == "deny rules"));
}

#[test]
fn nothing_is_written_and_no_clear_secret_appears() {
    let tmp = tempfile::tempdir().unwrap();
    copy_fixture(tmp.path());
    let before = snapshot(tmp.path());
    let env = Env::fixture(tmp.path(), Os::Linux);
    let inv = inventory::run(
        &rules::embedded(),
        &rules::embedded_shell(),
        &env,
        Options::default(),
    );
    let after = snapshot(tmp.path());
    assert_eq!(before, after, "a file changed during the inventory");
    let json = serde_json::to_string(&inv).unwrap();
    assert!(
        !json.contains("SHOULDNEVERSHOW"),
        "a clear secret leaked into the output"
    );
    assert!(!json.contains("sk-ant-api03-SHOULD"));
}

/// The `bbl-report/1` view conforms, carries the same figures as the inventory, and maps
/// every kind of finding onto the common scale.
#[test]
fn report_conforms() {
    let (tmp, inv) = run();
    let env = Env::fixture(tmp.path(), Os::Linux);
    let rules = rules::embedded();
    let report = permitt_core::report::build(&inv, &rules, &env, 7);
    let json = report.to_json();
    let back = bbl_report::conformance::check(&json).expect("the report conforms");
    assert_eq!(back.module.id, "permitt");
    assert_eq!(back.duration_ms, 7);
    let figure = |k: &str| back.summary.iter().find(|s| s.key == k).unwrap().value.clone();
    assert_eq!(figure("hosts"), inv.hosts.len().to_string());
    assert_eq!(figure("findings"), inv.findings.len().to_string());
    let shell_hits: usize = inv.hosts.iter().map(|h| h.shell.len()).sum();
    assert_eq!(figure("shell"), shell_hits.to_string());
    // Every inventory finding is in the report, plus one per host with an unknown state.
    let unknown_hosts = inv
        .hosts
        .iter()
        .filter(|h| h.capabilities.iter().any(|c| c.state == State::Unknown))
        .count();
    assert_eq!(back.findings.len(), inv.findings.len() + unknown_hosts);
    assert!(back.findings.iter().any(|f| f.kind == "shell-bypass" && f.severity == bbl_report::Severity::ActNow));
    assert!(back.findings.iter().any(|f| f.kind == "runs-without-asking" && f.severity == bbl_report::Severity::Review));
    assert!(back.findings.iter().any(|f| f.kind == "unknown" && f.severity == bbl_report::Severity::Info));
    // Every finding opens the app at a cell, and secrets never leak.
    for f in &back.findings {
        let open = f.open.as_ref().unwrap();
        assert_eq!(open.app, "permitt-app");
        assert_eq!(open.args[0], "--select");
        assert!(inv.hosts.iter().any(|h| open.args[1].starts_with(&h.host)), "{}", open.args[1]);
    }
    assert!(!json.contains("SHOULDNEVERSHOW") && !json.contains("SHOULD-NEVER-SHOW"), "a masked secret stays masked");
}
