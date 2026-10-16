//! Permitt desktop app (iced): one window — the sheet of hosts × capabilities, the detail
//! of the selected cell with the evidence behind its state, the findings underneath.
//! Reads the same core as the CLI; writes only its settings (BRIEF R8).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, pick_list,
    row, scrollable, text, text_input,
};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use hostrules::SourceKind;
use permitt_core::model::{
    Capability, CapabilityState, Evidence, Finding, FindingKind, HostSheet, Level, State,
};
use permitt_core::rules::ShellRules;
use permitt_core::{inventory, Env, Inventory, Options, Rules};

use bbl_ui::palette::{alert, amber, good, muted as unknown, neutral};
use bbl_ui::widgets::{card, inner, pill, small_button};
use bbl_ui::{Lang, Prefs, Strings};

const EN: &str = include_str!("../../../locales/en.json");
const FR: &str = include_str!("../../../locales/fr.json");

/// Permitt's own preference, next to the language and theme `bbl-ui` keeps for every app.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Extra {
    /// `false` to leave `~/.bashrc` and friends unread (BRIEF R4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    read_shell: Option<bool>,
}

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .window_size((1280.0, 820.0))
        .run_with(App::new)
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Arc<Inventory>, String>),
    Rescan,
    SetLang(Lang),
    ToggleTheme,
    Filter(String),
    Scope(String),
    ToggleInstalledOnly(bool),
    ToggleFindingsOnly(bool),
    ToggleReadShell(bool),
    SelectCell(String, Capability),
    SelectFinding(usize),
    Copy(String),
    OpenFile(PathBuf),
    OpenUrl(String),
}

struct App {
    prefs: Prefs<Extra>,
    lang: Lang,
    strings: Strings,
    dark: bool,
    env: Env,
    rules: Arc<Rules>,
    shell_rules: Arc<ShellRules>,
    inventory: Option<Arc<Inventory>>,
    loading: bool,
    error: Option<String>,
    filter: String,
    /// `None`: user level; `Some(root)`: that project.
    scope: Option<PathBuf>,
    installed_only: bool,
    findings_only: bool,
    read_shell: bool,
    selected: Option<(String, Capability)>,
    status: String,
}

impl App {
    fn new() -> (App, Task<Message>) {
        let prefs: Prefs<Extra> = Prefs::load("permitt");
        let lang = prefs.initial_lang();
        let dark = prefs.initial_dark();
        let strings = Strings::load(EN, FR, lang);
        let env = Env::current();
        let rules = Arc::new(permitt_core::rules::embedded());
        let shell_rules = Arc::new(permitt_core::rules::embedded_shell());
        let read_shell = prefs.extra.read_shell.unwrap_or(true);
        let status = strings.t("header.scanning", &[]);
        let task = load(rules.clone(), shell_rules.clone(), env.clone(), read_shell);
        let app = App {
            prefs,
            lang,
            strings,
            dark,
            env,
            rules,
            shell_rules,
            inventory: None,
            loading: true,
            error: None,
            filter: String::new(),
            scope: None,
            installed_only: true,
            findings_only: false,
            read_shell,
            selected: None,
            status,
        };
        (app, task)
    }

    fn title(&self) -> String {
        self.strings.t("app.title", &[])
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn remember(&mut self) {
        self.prefs.remember(self.lang, self.dark);
        self.prefs.extra.read_shell = Some(self.read_shell);
        self.prefs.save();
    }

    fn reload(&mut self) -> Task<Message> {
        self.loading = true;
        self.status = self.strings.t("header.scanning", &[]);
        load(
            self.rules.clone(),
            self.shell_rules.clone(),
            self.env.clone(),
            self.read_shell,
        )
    }

    fn refresh_status(&mut self) {
        self.status = match &self.inventory {
            None => self.strings.t("header.scanning", &[]),
            Some(inv) => self.strings.t(
                "footer.summary",
                &[
                    (
                        "files",
                        &inv.files.iter().filter(|f| f.exists).count().to_string(),
                    ),
                    ("shell", &inv.shell_files.len().to_string()),
                    ("roots", &inv.roots.len().to_string()),
                    ("known", &self.rules.hosts.len().to_string()),
                    ("not_read", &inv.not_read.len().to_string()),
                ],
            ),
        };
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(inv)) => {
                self.inventory = Some(inv);
                self.loading = false;
                self.error = None;
                // `PERMITT_SELECT=host:capability` preselects a cell (documentation screenshots).
                if self.selected.is_none() {
                    if let Ok(v) = std::env::var("PERMITT_SELECT") {
                        if let Some((h, c)) = v.split_once(':') {
                            if let Some(c) = Capability::parse(c) {
                                self.selected = Some((h.to_string(), c));
                            }
                        }
                    }
                }
                self.refresh_status();
            }
            Message::Loaded(Err(e)) => {
                self.loading = false;
                self.error = Some(e);
            }
            Message::Rescan => return self.reload(),
            Message::SetLang(l) => {
                self.lang = l;
                self.strings = Strings::load(EN, FR, l);
                self.remember();
                self.refresh_status();
            }
            Message::ToggleTheme => {
                self.dark = !self.dark;
                self.remember();
            }
            Message::Filter(f) => self.filter = f,
            Message::Scope(choice) => {
                self.scope = self
                    .scope_options()
                    .into_iter()
                    .find(|(_, label)| *label == choice)
                    .and_then(|(root, _)| root);
            }
            Message::ToggleInstalledOnly(b) => self.installed_only = b,
            Message::ToggleFindingsOnly(b) => self.findings_only = b,
            Message::ToggleReadShell(b) => {
                self.read_shell = b;
                self.remember();
                return self.reload();
            }
            Message::SelectCell(host, cap) => self.selected = Some((host, cap)),
            Message::SelectFinding(fi) => {
                if let Some(inv) = &self.inventory {
                    if let Some(f) = inv.findings.get(fi) {
                        self.scope = f.project.clone();
                        self.selected =
                            Some((f.host.clone(), f.capability.unwrap_or(Capability::Commands)));
                    }
                }
            }
            Message::Copy(s) => {
                self.status = self.strings.t("footer.copied", &[]);
                return iced::clipboard::write(s);
            }
            Message::OpenFile(p) => {
                let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
                self.status = self
                    .strings
                    .t("footer.opened", &[("file", &short(&p, &self.env.home))]);
            }
            Message::OpenUrl(u) => {
                let _ = std::process::Command::new("xdg-open").arg(&u).spawn();
            }
        }
        Task::none()
    }

    // ------------------------------------------------------------------ data

    /// The sheets of the current scope: the user level, or a project's (hosts whose state
    /// does not differ there fall back to their user-level sheet).
    fn sheets(&self, inv: &Inventory) -> Vec<HostSheet> {
        let project = match &self.scope {
            None => return self.all_sheets(inv),
            Some(p) => p,
        };
        let base = self.all_sheets(inv);
        let overlay = inv.projects.iter().find(|p| &p.root == project);
        base.into_iter()
            .map(|h| {
                overlay
                    .and_then(|p| p.hosts.iter().find(|x| x.host == h.host).cloned())
                    .unwrap_or(h)
            })
            .collect()
    }

    /// Every host known to the rules, installed or not, in the inventory's order first.
    fn all_sheets(&self, inv: &Inventory) -> Vec<HostSheet> {
        let mut out: Vec<HostSheet> = inv.hosts.clone();
        if !self.installed_only {
            for rule in &self.rules.hosts {
                if !out.iter().any(|h| h.host == rule.id) {
                    let built = permitt_core::sheet::build(rule, &self.env, None);
                    out.push(built.sheet);
                }
            }
        }
        out
    }

    fn scope_options(&self) -> Vec<(Option<PathBuf>, String)> {
        let mut v = vec![(None, self.strings.t("filter.scope_user", &[]))];
        if let Some(inv) = &self.inventory {
            for p in &inv.projects {
                let name = p
                    .root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.root.display().to_string());
                v.push((
                    Some(p.root.clone()),
                    self.strings.t("filter.scope_project", &[("name", &name)]),
                ));
            }
        }
        v
    }

    fn host_has_finding(&self, inv: &Inventory, host: &str) -> bool {
        inv.findings
            .iter()
            .any(|f| f.host == host && f.project == self.scope)
            || inv
                .findings
                .iter()
                .any(|f| f.host == host && f.project.is_none())
    }

    fn visible(&self, inv: &Inventory, sheets: &[HostSheet]) -> Vec<usize> {
        let needle = self.filter.trim().to_lowercase();
        (0..sheets.len())
            .filter(|&i| {
                let h = &sheets[i];
                if self.installed_only && !h.installed {
                    return false;
                }
                if self.findings_only && !self.host_has_finding(inv, &h.host) {
                    return false;
                }
                if !needle.is_empty() && !h.name.to_lowercase().contains(&needle) {
                    return false;
                }
                true
            })
            .collect()
    }

    fn state_color(&self, s: State) -> Color {
        match s {
            State::Auto | State::Denylist(_) => alert(self.dark),
            State::Judged => amber(self.dark),
            State::Asks | State::Allowlist(_) | State::Sandboxed => good(self.dark),
            State::Off => neutral(),
            State::Unknown => unknown(),
        }
    }

    fn state_label(&self, s: State) -> String {
        let base = match s {
            State::Asks => self.strings.t("state.asks", &[]),
            State::Auto => self.strings.t("state.auto", &[]),
            State::Judged => self.strings.t("state.judged", &[]),
            State::Allowlist(_) => self.strings.t("state.allowlist", &[]),
            State::Denylist(_) => self.strings.t("state.denylist", &[]),
            State::Sandboxed => self.strings.t("state.sandboxed", &[]),
            State::Off => self.strings.t("state.off", &[]),
            State::Unknown => self.strings.t("state.unknown", &[]),
        };
        match s {
            State::Allowlist(n) | State::Denylist(n) if n > 0 => format!("{base}({n})"),
            _ => base,
        }
    }

    fn cap_label(&self, c: Capability) -> String {
        match c {
            Capability::Commands => self.strings.t("table.commands", &[]),
            Capability::Files => self.strings.t("table.files", &[]),
            Capability::Network => self.strings.t("table.network", &[]),
            Capability::Mcp => self.strings.t("table.mcp", &[]),
        }
    }

    // ------------------------------------------------------------------ views

    fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match (&self.inventory, &self.error) {
            (Some(inv), _) => {
                let sheets = self.sheets(inv);
                row![
                    column![self.view_filters(), self.view_table(inv, &sheets)]
                        .spacing(8)
                        .width(Length::FillPortion(13))
                        .height(Length::Fill),
                    container(self.view_side(inv, &sheets))
                        .width(Length::FillPortion(9))
                        .height(Length::Fill),
                ]
                .spacing(14)
                .height(Length::Fill)
                .into()
            }
            (None, Some(e)) => container(text(e.clone()).size(16))
                .center(Length::Fill)
                .into(),
            (None, None) => container(
                text(self.strings.t("header.scanning", &[]))
                    .size(16)
                    .style(text::secondary),
            )
            .center(Length::Fill)
            .into(),
        };
        let theme_label = if self.dark {
            self.strings.t("theme.light", &[])
        } else {
            self.strings.t("theme.dark", &[])
        };
        let footer = bbl_ui::widgets::footer(
            self.status.clone(),
            self.lang,
            Message::SetLang,
            theme_label,
            Message::ToggleTheme,
        );
        column![self.view_header(), body, footer]
            .spacing(14)
            .padding(16)
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let s = &self.strings;
        let inv = self.inventory.as_deref();
        let n = |v: Option<usize>| v.map(|x| x.to_string()).unwrap_or_else(|| "—".into());
        let installed = inv.map(|i| i.hosts.len());
        let known = self.rules.hosts.len();
        let auto = inv.map(|i| {
            i.hosts
                .iter()
                .flat_map(|h| &h.capabilities)
                .filter(|c| c.state == State::Auto)
                .count()
        });
        let total = inv.map(|i| i.hosts.len() * 4).unwrap_or(0);
        let findings = inv.map(|i| i.findings.len());
        let shell = inv.map(|i| i.hosts.iter().map(|h| h.shell.len()).sum::<usize>());
        let dot = |v: Option<usize>| {
            v.map(|x| {
                if x == 0 {
                    good(self.dark)
                } else {
                    alert(self.dark)
                }
            })
        };
        row![
            bbl_ui::widgets::kpi(
                s.t("kpi.hosts", &[]),
                n(installed),
                s.t("kpi.hosts_known", &[("n", &known.to_string())]),
                None
            ),
            bbl_ui::widgets::kpi(
                s.t("kpi.auto", &[]),
                n(auto),
                s.t("kpi.auto_sub", &[("n", &total.to_string())]),
                dot(auto)
            ),
            bbl_ui::widgets::kpi(s.t("kpi.findings", &[]), n(findings), "", dot(findings)),
            bbl_ui::widgets::kpi(
                s.t("kpi.shell", &[]),
                n(shell),
                s.t(
                    "kpi.shell_sub",
                    &[(
                        "n",
                        &inv.map(|i| i.shell_files.len()).unwrap_or(0).to_string()
                    )]
                ),
                dot(shell)
            ),
        ]
        .spacing(12)
        .into()
    }

    fn view_filters(&self) -> Element<'_, Message> {
        let s = &self.strings;
        let options: Vec<String> = self.scope_options().into_iter().map(|(_, l)| l).collect();
        let selected = self
            .scope_options()
            .into_iter()
            .find(|(root, _)| *root == self.scope)
            .map(|(_, l)| l)
            .unwrap_or_else(|| s.t("filter.scope_user", &[]));
        let scope = pick_list(options, Some(selected), Message::Scope)
            .text_size(12)
            .padding([4, 8])
            .width(Length::Fixed(200.0));
        row![
            text_input(&s.t("filter.placeholder", &[]), &self.filter)
                .on_input(Message::Filter)
                .size(13)
                .width(Length::Fill),
            scope,
            checkbox(s.t("filter.installed_only", &[]), self.installed_only)
                .on_toggle(Message::ToggleInstalledOnly)
                .size(16)
                .text_size(12),
            checkbox(s.t("filter.findings_only", &[]), self.findings_only)
                .on_toggle(Message::ToggleFindingsOnly)
                .size(16)
                .text_size(12),
            checkbox(s.t("filter.read_shell", &[]), self.read_shell)
                .on_toggle(Message::ToggleReadShell)
                .size(16)
                .text_size(12),
            if self.loading {
                button(text(s.t("header.scanning", &[])).size(12)).style(button::secondary)
            } else {
                button(text(s.t("header.rescan", &[])).size(12))
                    .style(button::secondary)
                    .on_press(Message::Rescan)
            },
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_table(&self, inv: &Inventory, sheets: &[HostSheet]) -> Element<'_, Message> {
        let s = &self.strings;
        let visible = self.visible(inv, sheets);
        let head = column![
            text(s.t("table.title", &[])).size(16),
            text(s.t("table.subtitle", &[]))
                .size(11)
                .style(text::secondary),
        ]
        .spacing(2);
        if inv.hosts.is_empty() && self.installed_only {
            return card(
                column![
                    head,
                    container(text(
                        s.t("table.none", &[("n", &self.rules.hosts.len().to_string())])
                    ))
                    .padding(20)
                ]
                .spacing(10)
                .into(),
            )
            .height(Length::Fill)
            .into();
        }
        let header_cell =
            |label: String, w: Length| text(label).size(12).style(text::secondary).width(w);
        let header = row![
            header_cell(s.t("table.host", &[]), Length::FillPortion(3)),
            header_cell(self.cap_label(Capability::Commands), Length::FillPortion(3)),
            header_cell(self.cap_label(Capability::Files), Length::FillPortion(3)),
            header_cell(self.cap_label(Capability::Network), Length::FillPortion(3)),
            header_cell(self.cap_label(Capability::Mcp), Length::FillPortion(3)),
        ]
        .spacing(6)
        .padding([4, 6]);
        let mut rows = column![header].spacing(2);
        if visible.is_empty() {
            rows = rows
                .push(container(text(s.t("table.empty", &[])).style(text::secondary)).padding(12));
        }
        for i in visible {
            let h = &sheets[i];
            // Context line under the host name.
            let mut ctx: Vec<String> = Vec::new();
            if !h.installed {
                ctx.push(s.t("table.not_installed", &[]));
            }
            for hit in &h.shell {
                ctx.push(format!(
                    "▲ {}:{} `{}`",
                    short(&hit.file, &self.env.home),
                    hit.line,
                    hit.flag
                ));
            }
            if !h.guards.is_empty() {
                let names: Vec<&str> = h.guards.iter().map(|g| g.name.as_str()).collect();
                ctx.push(s.t("table.guards", &[("list", &names.join(", "))]));
            }
            if h.capabilities.iter().any(|c| c.state == State::Unknown) && !h.unknown.is_empty() {
                ctx.push(s.t("table.unknown", &[]));
            }
            let mut name_col = column![text(h.name.clone()).size(13)].spacing(2);
            if !ctx.is_empty() {
                name_col = name_col.push(text(ctx.join(" · ")).size(10).style(text::secondary));
            }
            let mut r = row![name_col.width(Length::FillPortion(3))]
                .spacing(6)
                .padding([6, 6])
                .align_y(iced::Alignment::Center);
            for c in Capability::ALL {
                let cs = h.state(c).unwrap();
                let selected = self.selected.as_ref() == Some(&(h.host.clone(), c));
                let mut cell = row![pill(self.state_label(cs.state), self.state_color(cs.state))]
                    .spacing(4)
                    .align_y(iced::Alignment::Center);
                if cs.is_default {
                    cell = cell.push(
                        text(s.t("table.default", &[]))
                            .size(10)
                            .style(text::secondary),
                    );
                }
                let cell = container(cell)
                    .padding([3, 4])
                    .width(Length::FillPortion(3))
                    .style(move |theme: &Theme| {
                        let p = theme.extended_palette();
                        container::Style {
                            background: if selected {
                                Some(p.primary.weak.color.into())
                            } else {
                                None
                            },
                            border: Border::default().rounded(5),
                            ..container::Style::default()
                        }
                    });
                r = r.push(mouse_area(cell).on_press(Message::SelectCell(h.host.clone(), c)));
            }
            let line = container(r).width(Length::Fill).style(|theme: &Theme| {
                let p = theme.extended_palette();
                container::Style {
                    background: Some(p.background.weak.color.into()),
                    border: Border::default().rounded(3),
                    ..container::Style::default()
                }
            });
            rows = rows.push(line);
        }
        card(
            column![
                head,
                scrollable(rows).height(Length::Fill),
                text(s.t("legend", &[])).size(10).style(text::secondary),
            ]
            .spacing(10)
            .into(),
        )
        .height(Length::Fill)
        .into()
    }

    fn view_side(&self, inv: &Inventory, sheets: &[HostSheet]) -> Element<'_, Message> {
        let s = &self.strings;
        let head = column![
            text(s.t("panel.title", &[])).size(16),
            text(s.t("panel.subtitle", &[]))
                .size(11)
                .style(text::secondary),
        ]
        .spacing(2);
        let detail: Element<'_, Message> = match &self.selected {
            Some((host, cap)) => match sheets.iter().find(|h| &h.host == host) {
                Some(h) => self.view_detail(h, *cap),
                None => container(
                    text(s.t("detail.pick", &[]))
                        .size(12)
                        .style(text::secondary),
                )
                .padding([8, 0])
                .into(),
            },
            None => container(
                text(s.t("detail.pick", &[]))
                    .size(12)
                    .style(text::secondary),
            )
            .padding([8, 0])
            .into(),
        };
        card(
            column![
                head,
                scrollable(detail).height(Length::FillPortion(3)),
                horizontal_rule(1),
                column![
                    text(s.t("findings.title", &[])).size(14),
                    text(s.t("findings.subtitle", &[]))
                        .size(11)
                        .style(text::secondary),
                ]
                .spacing(2),
                scrollable(self.view_findings(inv)).height(Length::FillPortion(2)),
            ]
            .spacing(10)
            .into(),
        )
        .height(Length::Fill)
        .into()
    }

    fn view_detail(&self, h: &HostSheet, cap: Capability) -> Element<'_, Message> {
        let s = &self.strings;
        let cs: &CapabilityState = h.state(cap).unwrap();
        let status = match h.precedence.status {
            permitt_core::model::PrecedenceStatus::Documented => {
                s.t("detail.precedence.documented", &[])
            }
            permitt_core::model::PrecedenceStatus::Undocumented => {
                s.t("detail.precedence.undocumented", &[])
            }
            permitt_core::model::PrecedenceStatus::Contradictory => {
                s.t("detail.precedence.contradictory", &[])
            }
        };
        let order: Vec<&str> = h.precedence.order.iter().map(|l| l.label()).collect();
        let sub = format!(
            "{} · {} · {}",
            h.kind,
            if h.installed {
                s.t("detail.installed", &[])
            } else {
                s.t("detail.not_installed", &[])
            },
            s.t(
                "detail.precedence",
                &[("order", &order.join(" > ")), ("status", &status)]
            )
        );
        let mut col = column![
            row![
                text(format!("{} — {}", h.name, self.cap_label(cap))).size(18),
                horizontal_space(),
                pill(self.state_label(cs.state), self.state_color(cs.state)),
            ]
            .align_y(iced::Alignment::Center),
            text(sub).size(11).style(text::secondary),
        ]
        .spacing(6);
        if let Some(p) = &h.project {
            col = col.push(
                text(format!("↳ {}", short(p, &self.env.home)))
                    .size(11)
                    .style(text::secondary),
            );
        }

        // Evidence, strongest first (the core applied weakest first).
        col = col.push(
            text(s.t("detail.evidence", &[]))
                .size(12)
                .style(text::secondary),
        );
        for e in cs.evidence.iter().rev() {
            col = col.push(self.view_evidence(e));
        }
        if !cs.ignored.is_empty() {
            col = col.push(
                text(s.t("detail.ignored", &[]))
                    .size(12)
                    .style(text::secondary),
            );
            for e in &cs.ignored {
                col = col.push(self.view_evidence(e));
            }
        }
        if !h.shell.is_empty() {
            col = col.push(
                text(s.t("detail.shell", &[]))
                    .size(12)
                    .style(text::secondary),
            );
            for hit in &h.shell {
                let block = column![
                    row![
                        text(format!("{}:{}", short(&hit.file, &self.env.home), hit.line))
                            .size(12)
                            .width(Length::Fill),
                        small_button(s.t("detail.open", &[]), Message::OpenFile(hit.file.clone())),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                    text(hit.text.clone()).size(11).font(iced::Font::MONOSPACE),
                    text(format!("`{}` — {}", hit.flag, hit.effect))
                        .size(11)
                        .color(alert(self.dark)),
                ]
                .spacing(3);
                col = col.push(inner(block.into()));
            }
        }
        if let Some(act) = self.rules.host(&h.host).and_then(|r| r.hint("act")) {
            col = col.push(text(s.t("detail.act", &[])).size(12).style(text::secondary));
            col = col.push(inner(text(act.to_string()).size(12).into()));
        }
        // Other capabilities, one click away.
        let mut others = row![text(s.t("detail.other", &[("host", &h.name)]))
            .size(11)
            .style(text::secondary)]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        for c in Capability::ALL {
            if c == cap {
                continue;
            }
            let st = h.state(c).unwrap().state;
            others = others.push(
                mouse_area(
                    row![
                        text(self.cap_label(c)).size(11),
                        pill(self.state_label(st), self.state_color(st))
                    ]
                    .spacing(3)
                    .align_y(iced::Alignment::Center),
                )
                .on_press(Message::SelectCell(h.host.clone(), c)),
            );
        }
        col = col.push(others);
        container(col)
            .padding(iced::Padding {
                top: 4.0,
                right: 14.0,
                bottom: 4.0,
                left: 0.0,
            })
            .into()
    }

    fn view_evidence(&self, e: &Evidence) -> Element<'_, Message> {
        let s = &self.strings;
        let where_ = match (&e.file, e.line) {
            (Some(p), Some(l)) => format!("{}:{l}", short(p, &self.env.home)),
            (Some(p), None) => short(p, &self.env.home),
            (None, _) if e.level == Level::Default => s.t("detail.default", &[]),
            (None, _) => e.level.to_string(),
        };
        let kind = match e.source.kind {
            SourceKind::Documentation => s.t("detail.source.documentation", &[]),
            SourceKind::SourceCode => s.t("detail.source.source-code", &[]),
            SourceKind::Observed => s.t("detail.source.observed", &[]),
        };
        let value_line = if e.value.is_empty() {
            e.key.clone()
        } else {
            format!("{} = {}", e.key, e.value)
        };
        let place = if e.level == Level::Default {
            where_
        } else {
            format!("{} · {}", e.level, where_)
        };
        let mut head = row![
            text(place).size(11).style(text::secondary),
            horizontal_space()
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        if let Some(p) = &e.file {
            head = head.push(small_button(
                s.t("detail.open", &[]),
                Message::OpenFile(p.clone()),
            ));
            head = head.push(small_button(
                s.t("detail.copy_path", &[]),
                Message::Copy(p.display().to_string()),
            ));
        }
        let mut block = column![
            text(value_line)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Length::Fill),
            head
        ]
        .spacing(3);
        if let Some(n) = &e.note {
            block = block.push(text(n.clone()).size(11));
        }
        let mut src = row![text(format!("{kind} — “{}”", e.source.quote))
            .size(11)
            .style(text::secondary)
            .width(Length::Fill)]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        if let Some(u) = &e.source.url {
            src = src.push(small_button(
                s.t("detail.open_doc", &[]),
                Message::OpenUrl(u.clone()),
            ));
        }
        block = block.push(src);
        inner(block.into())
    }

    fn view_findings(&self, inv: &Inventory) -> Element<'_, Message> {
        let s = &self.strings;
        if inv.findings.is_empty() {
            return container(
                text(s.t("findings.none", &[]))
                    .size(12)
                    .color(good(self.dark)),
            )
            .padding([6, 0])
            .into();
        }
        let mut col = column![].spacing(6);
        for (fi, f) in inv.findings.iter().enumerate() {
            let color = match f.kind {
                FindingKind::JudgedByModel => amber(self.dark),
                FindingKind::Unreadable | FindingKind::Contradictory | FindingKind::NotHonoured => {
                    neutral()
                }
                _ => alert(self.dark),
            };
            let where_ = finding_where(f, &self.env.home, s);
            let mut block = column![row![
                text(s.t(&format!("kind.{}", kind_slug(f.kind)), &[]))
                    .size(10)
                    .color(color),
                text(
                    f.message
                        .split(" — ")
                        .next()
                        .unwrap_or(&f.message)
                        .to_string()
                )
                .size(12),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),]
            .spacing(2);
            if let Some(w) = where_ {
                block = block.push(text(w).size(11).style(text::secondary));
            }
            if let Some(act) = self.rules.host(&f.host).and_then(|r| r.hint("act")) {
                block = block.push(text(format!("→ {act}")).size(11).style(text::secondary));
            }
            let c = container(block)
                .padding(8)
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(Background::Color(p.background.base.color)),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                });
            col = col.push(mouse_area(c).on_press(Message::SelectFinding(fi)));
        }
        container(col)
            .padding(iced::Padding {
                top: 4.0,
                right: 14.0,
                bottom: 4.0,
                left: 0.0,
            })
            .into()
    }
}

/// The one line under a finding: the file and key that set it, or the default's quote.
fn finding_where(f: &Finding, home: &Path, s: &Strings) -> Option<String> {
    if let Some(e) = f.evidence.iter().rev().find(|e| e.level != Level::Default) {
        let loc = match (&e.file, e.line) {
            (Some(p), Some(l)) => format!("{}:{l}", short(p, home)),
            (Some(p), None) => short(p, home),
            _ => e.level.to_string(),
        };
        return Some(format!("{loc} · {} = {}", e.key, e.value));
    }
    if let Some(e) = f.evidence.iter().find(|e| e.level == Level::Default) {
        return Some(format!(
            "{} — “{}”",
            s.t("findings.by_default", &[]),
            e.source.quote
        ));
    }
    f.message.split(" — ").nth(1).map(String::from)
}

fn load(rules: Arc<Rules>, shell: Arc<ShellRules>, env: Env, read_shell: bool) -> Task<Message> {
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let inv = inventory::run(
            &rules,
            &shell,
            &env,
            Options {
                no_shell: !read_shell,
                all_hosts: false,
            },
        );
        let _ = tx.send(Arc::new(inv));
    });
    Task::perform(
        async move {
            rx.await
                .map_err(|_| "the inventory thread stopped unexpectedly".to_string())
        },
        Message::Loaded,
    )
}

fn short(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

fn kind_slug(k: FindingKind) -> &'static str {
    match k {
        FindingKind::RunsWithoutAsking => "runs-without-asking",
        FindingKind::JudgedByModel => "judged-by-model",
        FindingKind::DenylistOnly => "denylist-only",
        FindingKind::ShellBypass => "shell-bypass",
        FindingKind::NotHonoured => "not-honoured",
        FindingKind::Unreadable => "unreadable",
        FindingKind::Contradictory => "contradictory",
    }
}
