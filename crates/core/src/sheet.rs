//! The sheet of one host in one context: resolve its files, read the settings the rule
//! describes, apply them in precedence order on top of the sourced defaults. Read-only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hostrules::read::{dig, line_of, read_tree};
use hostrules::resolve::{resolve, Resolved};
use hostrules::secrets;
use hostrules::Env;
use serde_json::Value;

use crate::model::{
    Capability, CapabilityState, Evidence, FileStatus, Finding, FindingKind, Guard, HostSheet,
    Level, PrecedenceInfo, PrecedenceStatus, SourceRef, State,
};
use crate::rules::{HostRule, Location, Setting, SettingKind};

/// Default precedence when a host documents none: the usual "closest to the project wins".
pub const DEFAULT_ORDER: [Level; 4] = [Level::Managed, Level::Local, Level::Project, Level::User];

/// A setting found in a file, before it is interpreted.
struct Found<'a> {
    setting: &'a Setting,
    level: Level,
    file: PathBuf,
    text: &'a str,
    /// Keys the wildcards of the key path took.
    wild: Vec<String>,
    value: &'a Value,
}

/// Per capability, what the settings said.
#[derive(Default)]
struct Acc {
    /// The state set by a mode / base value, with the level that set it.
    state: Option<(State, Level)>,
    /// Pattern counts per level: (allow, ask, deny).
    counts: BTreeMap<Level, (usize, usize, usize)>,
    bare_allow: Option<Level>,
    bare_deny: Option<Level>,
    evidence: Vec<Evidence>,
    ignored: Vec<Evidence>,
}

pub struct Built {
    pub sheet: HostSheet,
    pub files: Vec<FileStatus>,
    pub findings: Vec<Finding>,
}

/// Build the sheet of `rule` for `project` (None: the user's own levels only).
pub fn build(rule: &HostRule, env: &Env, project: Option<&Path>) -> Built {
    let mut files = Vec::new();
    let mut findings = Vec::new();
    let mut texts: Vec<(PathBuf, String, Value, usize)> = Vec::new(); // file, text, tree, loc idx
    let mut any_file = false;

    for (li, loc) in rule.locations.iter().enumerate() {
        let level = loc.ext.level;
        let per_project = matches!(level, Level::Project | Level::Local);
        if per_project && project.is_none() && loc.project_key.is_none() {
            continue;
        }
        let roots: Vec<PathBuf> = project.map(|p| vec![p.to_path_buf()]).unwrap_or_default();
        for Resolved { path, .. } in resolve(loc, env, &roots) {
            if !path.is_file() {
                if !path.to_string_lossy().contains('*') {
                    files.push(status(rule, loc, &path, false, None, 0));
                }
                continue;
            }
            any_file = true;
            match read_tree(&path, loc.format) {
                Ok((text, tree)) => {
                    files.push(status(rule, loc, &path, true, None, 0));
                    texts.push((path, text, tree, li));
                }
                Err(e) => {
                    files.push(status(rule, loc, &path, true, Some(e.to_string()), 0));
                    findings.push(Finding {
                        kind: FindingKind::Unreadable,
                        host: rule.id.clone(),
                        host_name: rule.name.clone(),
                        capability: None,
                        project: project.map(Path::to_path_buf),
                        message: format!("{}: {e}", short(&path, env)),
                        evidence: Vec::new(),
                    });
                }
            }
        }
    }

    // Collect the settings present in each file.
    let mut found: Vec<Found> = Vec::new();
    for (path, text, tree, li) in &texts {
        let loc = &rule.locations[*li];
        let level = loc.ext.level;
        // A per-project location inside a user file: narrow to the project's entry.
        let root: &Value = match (&loc.project_key, project) {
            (Some(pk), Some(p)) => {
                let want = p.to_string_lossy().to_string();
                let mut hit: Option<&Value> = None;
                for (wild, node) in dig(tree, &format!("{pk}.*")) {
                    let key = wild.last().cloned().unwrap_or_default();
                    let expanded = env.expand(&key).map(|e| e.to_string_lossy().to_string());
                    if key == want || expanded.as_deref() == Some(want.as_str()) {
                        hit = Some(node);
                        break;
                    }
                }
                match hit {
                    Some(n) => n,
                    None => continue,
                }
            }
            (Some(_), None) => continue,
            (None, _) => tree,
        };
        let mut n = 0;
        for s in &rule.ext.settings {
            if !s.levels.is_empty() && !s.levels.contains(&level) {
                continue;
            }
            if let Some(name) = &s.file {
                if path.file_name().and_then(|n| n.to_str()) != Some(name.as_str()) {
                    continue;
                }
            }
            for (wild, value) in dig(root, &s.key) {
                n += 1;
                found.push(Found {
                    setting: s,
                    level,
                    file: path.clone(),
                    text,
                    wild,
                    value,
                });
            }
        }
        if let Some(f) = files.iter_mut().find(|f| &f.file == path) {
            f.settings_found = n;
        }
    }

    // Apply, weakest level first, so a stronger level overrides.
    let order: Vec<Level> = rule
        .ext
        .precedence
        .as_ref()
        .map(|p| p.order.clone())
        .unwrap_or_else(|| DEFAULT_ORDER.to_vec());
    let mut acc: BTreeMap<Capability, Acc> = Capability::ALL
        .iter()
        .map(|c| (*c, Acc::default()))
        .collect();
    let mut guards: Vec<Guard> = Vec::new();

    for level in order.iter().rev() {
        for f in found.iter().filter(|f| f.level == *level) {
            let ev_base = |key: String, value: String, note: Option<String>| Evidence {
                level: f.level,
                file: Some(f.file.clone()),
                line: line_of(f.text, &last_key(&key)),
                key,
                value,
                note,
                source: SourceRef::from(&f.setting.source),
            };
            let shown_key = expand_key(&f.setting.key, &f.wild);
            let not_honoured = f.setting.not_honoured_in.contains(&f.level)
                && (f.setting.not_honoured_values.is_empty()
                    || f.setting.not_honoured_values.contains(&scalar(f.value)));
            if not_honoured {
                let ev = ev_base(
                    shown_key.clone(),
                    shown(&shown_key, f.value),
                    Some("not honoured at this level".into()),
                );
                let caps = caps_of(f);
                for c in &caps {
                    acc.get_mut(c).unwrap().ignored.push(ev.clone());
                }
                findings.push(Finding {
                    kind: FindingKind::NotHonoured,
                    host: rule.id.clone(),
                    host_name: rule.name.clone(),
                    capability: caps.first().copied(),
                    project: project.map(Path::to_path_buf),
                    message: format!(
                        "`{shown_key}` = {} in {} is ignored by {} at the {} level",
                        shown(&shown_key, f.value),
                        short(&f.file, env),
                        rule.name,
                        f.level
                    ),
                    evidence: vec![ev],
                });
                continue;
            }
            if let Some(g) = &f.setting.guard {
                if !is_empty_value(f.value) {
                    guards.push(Guard {
                        name: g.clone(),
                        evidence: ev_base(
                            shown_key.clone(),
                            shown(&shown_key, f.value),
                            f.setting.note.clone(),
                        ),
                    });
                }
            }
            match f.setting.kind {
                SettingKind::Mode => apply_mode(f, &shown_key, &mut acc, &ev_base),
                SettingKind::Rules => apply_rules(f, &shown_key, &mut acc, &ev_base),
                SettingKind::Map => apply_map(f, &shown_key, &mut acc, &ev_base),
                SettingKind::ObjectRules => apply_object_rules(f, &shown_key, &mut acc, &ev_base),
            }
        }
    }

    // Finalise each capability.
    let merged = rule
        .ext
        .precedence
        .as_ref()
        .map(|p| p.lists == "merged")
        .unwrap_or(false);
    let defaults = rule.ext.defaults.as_ref();
    let mut capabilities = Vec::new();
    for c in Capability::ALL {
        let a = acc.remove(&c).unwrap();
        let (allow, ask, deny) = if merged {
            a.counts
                .values()
                .fold((0, 0, 0), |s, v| (s.0 + v.0, s.1 + v.1, s.2 + v.2))
        } else {
            // The strongest level that has any pattern wins.
            order
                .iter()
                .find_map(|l| a.counts.get(l).copied())
                .unwrap_or((0, 0, 0))
        };
        let mut evidence = a.evidence;
        let mut is_default = false;
        let mut state = match a.state {
            Some((s, _)) => s,
            None => {
                is_default = true;
                match defaults.and_then(|d| d.state(c)) {
                    Some(s) => {
                        if let Some(d) = defaults {
                            evidence.push(Evidence {
                                level: Level::Default,
                                file: None,
                                line: None,
                                key: format!("defaults.{c}"),
                                value: s.label(),
                                note: d.note.clone(),
                                source: SourceRef::from(&d.source),
                            });
                        }
                        s
                    }
                    None => State::Unknown,
                }
            }
        };
        // Patterns refine the base.
        if a.bare_allow.is_some() {
            state = State::Auto;
            is_default = false;
        } else if a.bare_deny.is_some() {
            state = State::Off;
            is_default = false;
        } else {
            match state {
                State::Asks | State::Allowlist(_) if allow > 0 => {
                    state = State::Allowlist(allow);
                    is_default = false;
                }
                State::Auto if deny + ask > 0 => {
                    state = State::Denylist(deny + ask);
                    is_default = false;
                }
                State::Denylist(_) if deny + ask > 0 => {
                    state = State::Denylist(deny + ask);
                    is_default = false;
                }
                _ => {}
            }
        }
        // D10: a part of the state no documented file holds.
        // Patterns alone do not settle an unknown base: an allowlist under an unreadable
        // mode is still an unknown state (the list is kept in the evidence).
        for u in &rule.ext.unknown {
            let concerned = u.capabilities.is_empty() || u.capabilities.contains(&c);
            let untouched = a.state.is_none() && a.bare_allow.is_none() && a.bare_deny.is_none();
            if concerned && u.overrides_state && untouched {
                state = State::Unknown;
                is_default = false;
                evidence.push(Evidence {
                    level: Level::Default,
                    file: None,
                    line: None,
                    key: "(not readable)".into(),
                    value: String::new(),
                    note: Some(u.reason.clone()),
                    source: u
                        .source
                        .as_ref()
                        .map(SourceRef::from)
                        .unwrap_or_else(|| observed(&u.reason)),
                });
            }
        }
        capabilities.push(CapabilityState {
            capability: c,
            state,
            is_default,
            evidence,
            ignored: a.ignored,
        });
    }

    let installed = any_file || rule.ext.binaries.iter().any(|b| env.which(b).is_some());
    let precedence = match &rule.ext.precedence {
        Some(p) => PrecedenceInfo {
            order: p.order.clone(),
            status: p.status,
            note: p.note.clone(),
            source: p.source.as_ref().map(SourceRef::from),
        },
        None => PrecedenceInfo {
            order: DEFAULT_ORDER.to_vec(),
            status: PrecedenceStatus::Undocumented,
            note: None,
            source: None,
        },
    };
    let unknown = rule.ext.unknown.iter().map(|u| u.reason.clone()).collect();
    let sheet = HostSheet {
        host: rule.id.clone(),
        name: rule.name.clone(),
        kind: rule.kind.clone(),
        installed,
        project: project.map(Path::to_path_buf),
        capabilities,
        guards,
        shell: Vec::new(),
        precedence,
        unknown,
    };
    Built {
        sheet,
        files,
        findings,
    }
}

type EvFn<'a> = dyn Fn(String, String, Option<String>) -> Evidence + 'a;

fn apply_mode(f: &Found, shown_key: &str, acc: &mut BTreeMap<Capability, Acc>, ev: &EvFn) {
    let raw = scalar(f.value);
    let caps_from_tool = tool_caps(f);
    match f
        .setting
        .states
        .get(&raw)
        .or_else(|| f.setting.states.get("_any"))
    {
        Some(m) => {
            let note = m.get("_note").cloned().or_else(|| f.setting.note.clone());
            for (cap, st) in m {
                if cap == "_note" {
                    continue;
                }
                let Some(state) = State::parse(st) else {
                    continue;
                };
                let targets: Vec<Capability> = if cap == "all" {
                    if caps_from_tool.is_empty() {
                        Capability::ALL.to_vec()
                    } else {
                        caps_from_tool.clone()
                    }
                } else {
                    vec![Capability::parse(cap).unwrap()]
                };
                for c in targets {
                    let a = acc.get_mut(&c).unwrap();
                    a.state = Some((state, f.level));
                    a.evidence.push(ev(
                        shown_key.to_string(),
                        shown(shown_key, f.value),
                        note.clone(),
                    ));
                }
            }
        }
        None => {
            // A value the rule does not describe: said, not guessed.
            let targets = if caps_from_tool.is_empty() {
                caps_of(f)
            } else {
                caps_from_tool
            };
            for c in targets {
                let a = acc.get_mut(&c).unwrap();
                a.state = Some((State::Unknown, f.level));
                a.evidence.push(ev(
                    shown_key.to_string(),
                    shown(shown_key, f.value),
                    Some("value not described in the rule".into()),
                ));
            }
        }
    }
}

fn apply_rules(f: &Found, shown_key: &str, acc: &mut BTreeMap<Capability, Acc>, ev: &EvFn) {
    let Some(arr) = f.value.as_array() else {
        return;
    };
    let effect = f.setting.effect.as_deref().unwrap_or("allow");
    let mut per_cap: BTreeMap<Capability, (usize, bool, Vec<String>)> = BTreeMap::new();
    let mut unknown_tools: Vec<String> = Vec::new();
    for v in arr {
        let Some(p) = v.as_str() else { continue };
        let (tool, spec) = split_pattern(p);
        let caps = match map_tool(f.setting, tool) {
            Some(c) => c,
            None if f.setting.tool_map.is_empty() => f.setting.applies.clone(),
            None => {
                unknown_tools.push(p.to_string());
                continue;
            }
        };
        if caps.is_empty() {
            continue;
        }
        // Bare: the whole tool (`Bash`, `Bash(*)`, `edit`), never a pattern (`git *`,
        // `mcp__github__*`, `src/**`). A plain list without a tool map is bare only on `*`.
        let bare = if f.setting.whole_tools {
            true
        } else if f.setting.tool_map.is_empty() {
            p == "*"
        } else {
            (spec.is_none() || spec == Some("*")) && f.setting.tool_map.contains_key(tool)
        };
        for c in caps {
            let e = per_cap.entry(c).or_insert((0, false, Vec::new()));
            e.0 += 1;
            e.1 |= bare;
            e.2.push(p.to_string());
        }
    }
    for (c, (n, bare, pats)) in per_cap {
        let a = acc.get_mut(&c).unwrap();
        let counts = a.counts.entry(f.level).or_default();
        match effect {
            "allow" => {
                counts.0 += n;
                if bare {
                    a.bare_allow = Some(f.level);
                }
            }
            "ask" => counts.1 += n,
            _ => {
                counts.2 += n;
                if bare {
                    a.bare_deny = Some(f.level);
                }
            }
        }
        a.evidence.push(ev(
            shown_key.to_string(),
            format!(
                "{effect} {n}: {}",
                shown(shown_key, &Value::String(pats.join(", ")))
            ),
            f.setting.note.clone(),
        ));
    }
    if !unknown_tools.is_empty() {
        for c in caps_of(f) {
            let a = acc.get_mut(&c).unwrap();
            a.evidence.push(ev(
                shown_key.to_string(),
                shown(shown_key, &Value::String(unknown_tools.join(", "))),
                Some("tool not in the rule's map: not attributed".into()),
            ));
        }
    }
}

fn apply_map(f: &Found, shown_key: &str, acc: &mut BTreeMap<Capability, Acc>, ev: &EvFn) {
    let Some(obj) = f.value.as_object() else {
        // A scalar where a map was expected (OpenCode `"permission": "allow"`): a base.
        if let Some(effect) = effect_of(f.setting, f.value) {
            for c in caps_of(f) {
                set_base(
                    acc,
                    c,
                    &effect,
                    f.level,
                    ev(
                        shown_key.to_string(),
                        shown(shown_key, f.value),
                        f.setting.note.clone(),
                    ),
                );
            }
        }
        return;
    };
    if f.setting.tool_map.is_empty() {
        // pattern → effect for the capabilities in `applies`.
        let caps = f.setting.applies.clone();
        let (base, counts, shown_pats) = count_patterns(f.setting, obj);
        for c in caps {
            if let Some(b) = &base {
                set_base(
                    acc,
                    c,
                    b,
                    f.level,
                    ev(format!("{shown_key}.*"), b.clone(), f.setting.note.clone()),
                );
            }
            let a = acc.get_mut(&c).unwrap();
            let e = a.counts.entry(f.level).or_default();
            e.0 += counts.0;
            e.1 += counts.1;
            e.2 += counts.2;
            if counts.0 + counts.1 + counts.2 > 0 {
                a.evidence.push(ev(
                    shown_key.to_string(),
                    shown_pats.clone(),
                    f.setting.note.clone(),
                ));
            }
        }
        return;
    }
    // tool → effect, or tool → { pattern → effect }.
    for (tool, v) in obj {
        let Some(caps) = map_tool(f.setting, tool) else {
            for c in caps_of(f) {
                acc.get_mut(&c).unwrap().evidence.push(ev(
                    format!("{shown_key}.{tool}"),
                    shown(tool, v),
                    Some("tool not in the rule's map: not attributed".into()),
                ));
            }
            continue;
        };
        let key = format!("{shown_key}.{tool}");
        match v.as_object() {
            None => {
                if let Some(effect) = effect_of(f.setting, v) {
                    for c in &caps {
                        set_base(
                            acc,
                            *c,
                            &effect,
                            f.level,
                            ev(key.clone(), shown(&key, v), f.setting.note.clone()),
                        );
                    }
                }
            }
            Some(inner) => {
                let (base, counts, shown_pats) = count_patterns(f.setting, inner);
                for c in &caps {
                    if let Some(b) = &base {
                        set_base(
                            acc,
                            *c,
                            b,
                            f.level,
                            ev(format!("{key}.*"), b.clone(), f.setting.note.clone()),
                        );
                    }
                    let a = acc.get_mut(c).unwrap();
                    let e = a.counts.entry(f.level).or_default();
                    e.0 += counts.0;
                    e.1 += counts.1;
                    e.2 += counts.2;
                    if counts.0 + counts.1 + counts.2 > 0 {
                        a.evidence.push(ev(
                            key.clone(),
                            shown_pats.clone(),
                            f.setting.note.clone(),
                        ));
                    }
                }
            }
        }
    }
}

fn apply_object_rules(f: &Found, shown_key: &str, acc: &mut BTreeMap<Capability, Acc>, ev: &EvFn) {
    let Some(arr) = f.value.as_array() else {
        return;
    };
    let tool_field = f.setting.tool_field.as_deref().unwrap_or("tool");
    // With a wildcard in the key (`tools.*.always_allow`), the tool is the matched key and
    // every item is one pattern for it.
    let wild_caps = tool_caps(f);
    let mut per: BTreeMap<(Capability, String), (usize, bool)> = BTreeMap::new();
    for item in arr {
        let Some(t) = item.as_object() else { continue };
        let effect = match &f.setting.effect_field {
            Some(ef) => t.get(ef).and_then(|v| effect_of(f.setting, v)),
            None => f.setting.effect.clone(),
        };
        let Some(effect) = effect else { continue };
        if !wild_caps.is_empty() {
            for c in &wild_caps {
                let e = per.entry((*c, effect.clone())).or_insert((0, false));
                e.0 += 1;
            }
            continue;
        }
        let tools: Vec<String> = match t.get(tool_field) {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
            _ => continue,
        };
        // A rule with no match / pattern field, or a `*`, is bare.
        let is_all = |v: &Value| match v {
            Value::String(s) => matches!(s.as_str(), "*" | "**" | "./**"),
            Value::Array(a) => a
                .iter()
                .any(|x| matches!(x.as_str(), Some("*" | "**" | "./**"))),
            _ => false,
        };
        let bare = !t
            .keys()
            .any(|k| k != tool_field && f.setting.effect_field.as_deref() != Some(k))
            || t.values().any(is_all);
        for tool in tools {
            let (name, _) = split_pattern(&tool);
            if let Some(caps) = map_tool(f.setting, name) {
                for c in caps {
                    let e = per.entry((c, effect.clone())).or_insert((0, false));
                    e.0 += 1;
                    e.1 |= bare;
                }
            }
        }
    }
    for ((c, effect), (n, bare)) in per {
        let a = acc.get_mut(&c).unwrap();
        let counts = a.counts.entry(f.level).or_default();
        match effect.as_str() {
            "allow" => {
                counts.0 += n;
                if bare {
                    a.bare_allow = Some(f.level);
                }
            }
            "ask" => counts.1 += n,
            _ => {
                counts.2 += n;
                if bare {
                    a.bare_deny = Some(f.level);
                }
            }
        }
        a.evidence.push(ev(
            shown_key.to_string(),
            format!("{effect} {n} rule(s)"),
            f.setting.note.clone(),
        ));
    }
}

/// `*` → base effect; other keys counted (allow, ask, deny). Returns the base, the counts,
/// and a shown summary.
fn count_patterns(
    s: &Setting,
    obj: &serde_json::Map<String, Value>,
) -> (Option<String>, (usize, usize, usize), String) {
    let mut base = None;
    let mut counts = (0, 0, 0);
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in obj {
        let v = match (&s.value_field, v.as_object()) {
            (Some(vf), Some(o)) => o.get(vf).unwrap_or(v),
            _ => v,
        };
        let Some(effect) = effect_of(s, v) else {
            continue;
        };
        if k == "*" || k == "**" || k == "**/*" {
            base = Some(effect);
            continue;
        }
        match effect.as_str() {
            "allow" => counts.0 += 1,
            "ask" => counts.1 += 1,
            _ => counts.2 += 1,
        }
        parts.push(format!("{k}={effect}"));
    }
    let summary = secrets::scrub_text(&parts.join(", ")).shown;
    (base, counts, summary)
}

fn set_base(
    acc: &mut BTreeMap<Capability, Acc>,
    c: Capability,
    effect: &str,
    level: Level,
    ev: Evidence,
) {
    let state = match effect {
        "allow" => State::Auto,
        "ask" => State::Asks,
        _ => State::Off,
    };
    let a = acc.get_mut(&c).unwrap();
    a.state = Some((state, level));
    a.evidence.push(ev);
}

/// Raw value → `allow` | `ask` | `deny` through the rule's table.
fn effect_of(s: &Setting, v: &Value) -> Option<String> {
    let raw = scalar(v);
    s.effect_values.get(&raw).cloned()
}

/// Capabilities named by the wildcard component of a `mode` key, through `tool_map`.
fn tool_caps(f: &Found) -> Vec<Capability> {
    if f.wild.is_empty() || f.setting.tool_map.is_empty() {
        return Vec::new();
    }
    let tool = f.wild.last().unwrap();
    map_tool(f.setting, tool).unwrap_or_default()
}

/// A tool name against `tool_map`: exact, then glob (`mcp__*`, `mcp:*`), then `_default`.
/// The value `all` names every capability.
fn map_tool(s: &Setting, tool: &str) -> Option<Vec<Capability>> {
    let expand = |v: &str| -> Vec<Capability> {
        match v {
            "all" => Capability::ALL.to_vec(),
            "ignore" => Vec::new(),
            _ => Capability::parse(v).into_iter().collect(),
        }
    };
    if let Some(v) = s.tool_map.get(tool) {
        return Some(expand(v));
    }
    for (pat, v) in &s.tool_map {
        // A bare `*` is the literal key of a shorthand ("*": "ask"), matched exactly above.
        if pat.len() > 1 && pat.ends_with('*') && tool.starts_with(&pat[..pat.len() - 1]) {
            return Some(expand(v));
        }
    }
    s.tool_map.get("_default").map(|v| expand(v))
}

/// Capabilities a setting is about when nothing more precise is known.
fn caps_of(f: &Found) -> Vec<Capability> {
    if !f.setting.applies.is_empty() {
        return f.setting.applies.clone();
    }
    let mut caps: Vec<Capability> = Vec::new();
    for m in f.setting.states.values() {
        for k in m.keys() {
            if k == "all" {
                return Capability::ALL.to_vec();
            }
            if let Some(c) = Capability::parse(k) {
                if !caps.contains(&c) {
                    caps.push(c);
                }
            }
        }
    }
    for v in f.setting.tool_map.values() {
        if v == "all" {
            return Capability::ALL.to_vec();
        }
        if let Some(c) = Capability::parse(v) {
            if !caps.contains(&c) {
                caps.push(c);
            }
        }
    }
    caps
}

/// `Bash(git *)` → (`Bash`, Some(`git *`)); `shell(git:*)` → (`shell`, Some(`git:*`));
/// `mcp__x__y` → (`mcp__x__y`, None); `Bash` → (`Bash`, None).
pub fn split_pattern(p: &str) -> (&str, Option<&str>) {
    match p.find('(') {
        Some(i) if p.ends_with(')') => (&p[..i], Some(&p[i + 1..p.len() - 1])),
        _ => (p, None),
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

/// A value as it may be shown: scalars masked by key, structures summarised.
fn shown(key: &str, v: &Value) -> String {
    match v {
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(scalar).collect();
            let s = secrets::scrub_text(&items.join(", ")).shown;
            truncate(&s, 160)
        }
        Value::Object(o) => format!("{{{} key(s)}}", o.len()),
        _ => secrets::classify(&last_key(key), &scalar(v)).shown,
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n).collect();
        format!("{cut}…")
    }
}

fn last_key(key: &str) -> String {
    hostrules::read::split_path(key)
        .last()
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

fn expand_key(key: &str, wild: &[String]) -> String {
    let mut out = String::new();
    let mut w = wild.iter();
    for (i, part) in hostrules::read::split_path(key).iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        if part == "*" {
            out.push_str(w.next().map(String::as_str).unwrap_or("*"));
        } else {
            out.push_str(part);
        }
    }
    out
}

fn is_empty_value(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}

fn observed(reason: &str) -> SourceRef {
    SourceRef {
        kind: hostrules::SourceKind::Observed,
        url: None,
        quote: reason.to_string(),
    }
}

fn status(
    rule: &HostRule,
    loc: &Location,
    path: &Path,
    exists: bool,
    error: Option<String>,
    n: usize,
) -> FileStatus {
    FileStatus {
        host: rule.id.clone(),
        host_name: rule.name.clone(),
        level: loc.ext.level,
        file: path.to_path_buf(),
        exists,
        error,
        settings_found: n,
        source_kind: loc.source.kind,
    }
}

pub fn short(p: &Path, env: &Env) -> String {
    let s = p.to_string_lossy();
    let home = env.home.to_string_lossy();
    match s.strip_prefix(home.as_ref()) {
        Some(rest) if !home.is_empty() => format!("~{rest}"),
        _ => s.to_string(),
    }
}
