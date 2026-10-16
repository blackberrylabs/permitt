//! Shell start-up files (D2): look for a closed, sourced list of bypass flags and variables
//! in `~/.bashrc` and friends. Only the matching line is kept, secrets masked. Read-only.

use std::path::PathBuf;

use hostrules::secrets;
use hostrules::Env;
use regex::Regex;

use crate::model::{ShellHit, SourceRef};
use crate::rules::ShellRules;

pub struct ShellScan {
    pub files_read: Vec<PathBuf>,
    pub hits: Vec<ShellHit>,
}

pub fn scan(rules: &ShellRules, env: &Env) -> ShellScan {
    let mut files_read = Vec::new();
    let mut hits = Vec::new();
    let patterns: Vec<(usize, Regex)> = rules
        .flags
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let re = match p.kind.as_str() {
                "env" => format!(r"(?:^|[\s;]|export\s+){}=", regex::escape(&p.pattern)),
                _ => format!(
                    r#"(?:^|[\s'"]){}(?:[\s'";]|$|=)"#,
                    regex::escape(&p.pattern)
                ),
            };
            Regex::new(&re).ok().map(|r| (i, r))
        })
        .collect();
    for spec in &rules.files {
        let Some(path) = env.expand(spec) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        files_read.push(path.clone());
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            for (i, re) in &patterns {
                let p = &rules.flags[*i];
                if !re.is_match(trimmed) {
                    continue;
                }
                if p.kind == "env" && !p.values.is_empty() {
                    // Only the listed assignments count (GOOSE_MODE=auto, not =approve).
                    let assigned = trimmed
                        .split(&format!("{}=", p.pattern))
                        .nth(1)
                        .map(|rest| {
                            rest.trim_matches(|c| c == '"' || c == '\'')
                                .split_whitespace()
                                .next()
                                .unwrap_or("")
                                .trim_matches(|c| c == '"' || c == '\'')
                                .to_string()
                        })
                        .unwrap_or_default();
                    if !p.values.iter().any(|v| v == &assigned) {
                        continue;
                    }
                }
                hits.push(ShellHit {
                    host: p.host.clone(),
                    file: path.clone(),
                    line: n + 1,
                    text: secrets::scrub_text(trimmed).shown,
                    flag: p.pattern.clone(),
                    effect: p.effect.clone(),
                    source: SourceRef::from(&p.source),
                });
            }
        }
    }
    ShellScan { files_read, hits }
}
