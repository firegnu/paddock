//! Settings › Agents › Integrations (DESIGN §13 P5-94): a row per agent corral supports, read in
//! the background from `crate::integrations`, and the confirmation before skills are written or a
//! plugin (amp's, P5-98) is installed or removed.
use super::*;
use crate::integrations::{self, Plan, PluginPlan, Row, Skills, Status};
use std::sync::atomic::AtomicBool;

/// The section's state in the Settings window.
#[derive(Default)]
pub(super) struct Integrations {
    /// The last reading, or why there is none.
    read: Option<Result<integrations::Report, String>>,
    checking: bool,
    /// The confirmation open over the window.
    confirm: Option<Confirm>,
    /// Writing, with what the heading says meanwhile.
    working: Option<&'static str>,
}

/// What a confirmation is for.
enum Confirm {
    Skills(Plan),
    Plugin(PluginPlan),
}

/// Below this width the Status and Skills columns go under the agent's name.
const NARROW: f32 = 600.0;

/// The Status and Skills columns.
const STATUS_WIDTH: f32 = 190.0;
const SKILLS_WIDTH: f32 = 210.0;

const BUILT_IN: &str = "Built in: status comes with corral each time it starts an agent.";
const GLOBAL: &str = "amp needs a plugin, installed once in ~/.config/amp/plugins. It does nothing \
outside corral, but each amp that loads it, the desktop app too, runs one idle plugin process.";
const SKILLS_NOTE: &str = "Skills teach an agent the corral and paddock commands.";

/// What the plugin confirmation says under the file: where it works and what it costs.
const PLUGIN_SAYS: [&str; 2] = [
    "It only works in amp started by corral. Your own amp sessions and the Amp desktop app may \
load it too, but it does nothing there.",
    "Each amp that loads it runs one more idle plugin process.",
];

/// `path` with the home folder written `~`.
fn tilde(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && path.starts_with(&home) => {
            format!("~{}", &path[home.len()..])
        }
        _ => path.to_owned(),
    }
}

/// `1 file`, `3 files`.
fn count(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// `a`, `a and b`, `a, b and c`.
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

impl SettingsView {
    /// Asks corral, ranch and paddock again, keeping the last answer up until the new one.
    pub(super) fn refresh_integrations(&mut self, cx: &mut Context<Self>) {
        if self.integrations.checking || self.integrations.working.is_some() {
            return;
        }
        let Some(corral) = crate::windows::report(cx).map(|report| report.corral) else {
            return;
        };
        self.integrations.checking = true;
        let task = cx
            .background_spawn(async move { integrations::read(&corral, &AtomicBool::new(false)) });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            let _ = this.update(cx, |this, cx| {
                this.integrations.checking = false;
                this.integrations.read = Some(read);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn ask_install(&mut self, kind: &str, cx: &mut Context<Self>) {
        if let Some(Ok(report)) = &self.integrations.read {
            self.integrations.confirm = integrations::plan(report, kind).map(Confirm::Skills);
            cx.notify();
        }
    }

    /// The plugin's buttons: install or update it, or with `remove`, remove it.
    fn ask_plugin(&mut self, kind: &str, remove: bool, cx: &mut Context<Self>) {
        if let Some(Ok(report)) = &self.integrations.read {
            self.integrations.confirm =
                integrations::plugin_plan(report, kind, remove).map(Confirm::Plugin);
            cx.notify();
        }
    }

    fn cancel_install(&mut self, cx: &mut Context<Self>) {
        self.integrations.confirm = None;
        cx.notify();
    }

    /// Does what the confirmation showed, says how it went in the bar, and reads again.
    fn install(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.integrations.confirm.take() else {
            return;
        };
        let Some(corral) = crate::windows::report(cx).map(|report| report.corral) else {
            return;
        };
        let (working, done, failed) = match &confirm {
            Confirm::Skills(plan) => {
                let verb = if plan.install { "Installed" } else { "Updated" };
                let files = count(plan.files.len(), "file");
                (
                    "Installing…",
                    format!("{verb} {files} for {}", plan.name),
                    format!("Couldn't install skills for {}", plan.name),
                )
            }
            Confirm::Plugin(plan) if plan.remove => (
                "Removing…",
                format!("Removed the {} plugin", plan.name),
                format!("Couldn't remove the {} plugin", plan.name),
            ),
            Confirm::Plugin(plan) => {
                let verb = if plan.status == "create" {
                    "Installed"
                } else {
                    "Updated"
                };
                (
                    "Installing…",
                    format!("{verb} the {} plugin", plan.name),
                    format!("Couldn't install the {} plugin", plan.name),
                )
            }
        };
        self.integrations.working = Some(working);
        let task = cx.background_spawn(async move {
            let cancel = AtomicBool::new(false);
            match confirm {
                Confirm::Skills(plan) => integrations::install(&corral, &plan, &cancel),
                Confirm::Plugin(plan) => integrations::plugin(&corral, &plan, &cancel),
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.integrations.working = None;
                match result {
                    Ok(()) => this.say(done, false, cx),
                    Err(error) => this.say(format!("{failed}: {error}"), true, cx),
                }
                this.refresh_integrations(cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// A quiet text button in the accent's link colour.
    fn text_button(
        &self,
        id: &'static str,
        label: &'static str,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: fn(&mut Self, &mut Context<Self>),
    ) -> Stateful<Div> {
        let hover = self.faint(0.07);
        div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(26.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_size(ui.px(12.0))
            .text_color(self.fg(|t| t.agents_branch))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| click(this, cx)))
    }

    /// The section: its heading with Refresh or what it is doing, the rows, and what the columns
    /// mean.
    pub(super) fn integrations_section(
        &self,
        width: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.integrations;
        let busy = state.checking || state.working.is_some();
        let dim = self.fg(|t| t.agents_dimmer);
        let heading = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.heading("Integrations", ui))
            .child(div().flex_1())
            .child(if busy {
                div()
                    .px(px(10.0))
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .child(state.working.unwrap_or("Checking…"))
                    .into_any_element()
            } else {
                self.text_button("integrations-refresh", "Refresh", ui, cx, |this, cx| {
                    this.refresh_integrations(cx)
                })
                .into_any_element()
            });
        let body = match &state.read {
            None => self
                .listing(vec![
                    div()
                        .px(px(16.0))
                        .py(px(8.0))
                        .text_color(dim)
                        .child("Checking…"),
                ])
                .into_any_element(),
            Some(Err(error)) => self
                .listing(vec![
                    div()
                        .px(px(16.0))
                        .py(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex_shrink_0()
                                .size(px(7.0))
                                .rounded_full()
                                .bg(self.fg(|t| t.agents_red)),
                        )
                        .child(format!(
                            "Couldn't ask corral which agents it supports: {error}"
                        )),
                ])
                .into_any_element(),
            Some(Ok(report)) => {
                let narrow = width < ui.scale(NARROW);
                let mut rows: Vec<Div> = integrations::rows(report)
                    .into_iter()
                    .map(|row| self.integration_row(row, narrow, busy, ui, cx))
                    .collect();
                if !narrow {
                    rows.insert(0, self.integration_columns(ui));
                }
                let card = self.card(None, rows).pb(px(4.0));
                card.when(state.checking, |card| card.opacity(0.6))
                    .into_any_element()
            }
        };
        let global = matches!(&state.read, Some(Ok(report)) if integrations::global(report));
        let note = self.fg(|t| t.agents_dim);
        self.section_with(heading, ui)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(body)
                    .child(self.under(BUILT_IN, note, ui))
                    .when(global, |notes| notes.child(self.under(GLOBAL, note, ui)))
                    .child(self.under(SKILLS_NOTE, note, ui)),
            )
            .into_any_element()
    }

    /// A heading row over a card, with what is beside the heading.
    fn section_with(&self, heading: Div, ui: &UiFont) -> Div {
        div().flex().flex_col().gap(ui.px(8.0)).child(heading)
    }

    /// The columns' names, over the rows.
    fn integration_columns(&self, ui: &UiFont) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(14.0))
            .pt(px(8.0))
            .pb(px(2.0))
            .pl(px(28.0))
            .pr(px(16.0))
            .text_size(ui.px(11.5))
            .text_color(self.fg(|t| t.agents_dimmer))
            .child(div().flex_1().child("Agent"))
            .child(div().w(ui.px(STATUS_WIDTH)).child("Status"))
            .child(div().w(ui.px(SKILLS_WIDTH)).child("Skills"))
    }

    fn integration_row(
        &self,
        row: Row,
        narrow: bool,
        busy: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let dim = self.fg(|t| t.agents_dim);
        let name = div()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(ui.px(13.5))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(row.name.clone()),
            )
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(row.detail.clone()),
            );
        let status = self.integration_status(&row, busy, ui, cx);
        let skills = self.integration_skills(&row, busy, ui, cx);
        let line = if narrow {
            div().flex().flex_col().gap(px(8.0)).child(name).child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(14.0))
                    .child(status)
                    .child(skills),
            )
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .child(name.flex_1())
                .child(div().flex_shrink_0().w(ui.px(STATUS_WIDTH)).child(status))
                .child(div().flex_shrink_0().w(ui.px(SKILLS_WIDTH)).child(skills))
        };
        line.min_h(ui.px(54.0))
            .py(px(7.0))
            .pl(px(28.0))
            .pr(px(16.0))
            .when(!row.found, |line| line.opacity(0.5))
    }

    /// A green pill with a dot: `Built in`, `Installed`.
    fn green_pill(&self, text: &'static str, ui: &UiFont) -> Div {
        let green = self.fg(|t| t.agents_green);
        div()
            .flex_shrink_0()
            .h(ui.px(22.0))
            .px(px(9.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded_full()
            .bg(green.opacity(0.14))
            .text_size(ui.px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(green)
            .child(div().size(px(6.0)).rounded_full().bg(green))
            .child(text)
    }

    /// The accent button that installs or updates, quiet while something else is under way.
    fn act_button(
        &self,
        id: String,
        label: &'static str,
        busy: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        let accent = self.fg(|t| t.agents_accent);
        let button = div()
            .id(ElementId::Name(id.into()))
            .flex_shrink_0()
            .h(ui.px(26.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded_full()
            .bg(accent.opacity(0.16))
            .border_1()
            .border_color(accent.opacity(0.45))
            .text_size(ui.px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(accent)
            .child(label);
        if busy {
            button.opacity(0.5)
        } else {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.24)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| click(this, cx)))
        }
    }

    /// A note beside a cell, cut short when there is no room.
    fn cell_note(&self, note: Option<String>, color: Hsla, ui: &UiFont) -> Option<Div> {
        note.map(|note| {
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(ui.px(12.0))
                .text_color(color)
                .child(note)
        })
    }

    /// `Built in`; for a plugin, Install, Installed with Uninstall, Update or Not managed; or
    /// corral's own word for a way paddock doesn't know.
    fn integration_status(
        &self,
        row: &Row,
        busy: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        let yellow = self.fg(|t| t.agents_yellow);
        let kind = row.kind.clone();
        let cell = div().flex().items_center().gap(px(8.0)).min_w(px(0.0));
        let mut plugin = |label| {
            let kind = kind.clone();
            self.act_button(
                format!("plugin-{kind}"),
                label,
                busy,
                ui,
                cx,
                move |this, cx| this.ask_plugin(&kind, false, cx),
            )
        };
        match &row.status {
            None => cell.text_color(dim).child("—"),
            Some(Status::BuiltIn) => cell.child(self.green_pill("Built in", ui)),
            Some(Status::Install) => cell.child(plugin("Install")),
            Some(Status::Update) => cell.child(plugin("Update")).children(self.cell_note(
                row.status_note.clone(),
                yellow,
                ui,
            )),
            Some(Status::Installed) => {
                let (rest, lit) = (self.faint(0.06), self.faint(0.12));
                let uninstall = div()
                    .id(ElementId::Name(format!("plugin-remove-{kind}").into()))
                    .flex_shrink_0()
                    .h(ui.px(24.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .bg(rest)
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .child("Uninstall");
                let uninstall = if busy {
                    uninstall.opacity(0.5)
                } else {
                    uninstall
                        .cursor_pointer()
                        .hover(move |style| style.bg(lit))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.ask_plugin(&kind, true, cx)
                        }))
                };
                cell.child(self.green_pill("Installed", ui))
                    .child(uninstall)
            }
            Some(Status::NotManaged) => cell
                .flex_col()
                .items_start()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(ui.px(12.5))
                        .text_color(dim)
                        .child("Not managed"),
                )
                .children(self.cell_note(row.status_note.clone(), yellow, ui)),
            Some(Status::Other(word)) => cell
                .text_size(ui.px(12.0))
                .text_color(dim)
                .child(word.clone()),
        }
        .into_any_element()
    }

    /// Up to date, Not managed, or the button with what it would fix.
    fn integration_skills(
        &self,
        row: &Row,
        busy: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        let quiet = |lead: Option<AnyElement>, text: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(ui.px(12.5))
                .text_color(dim)
                .children(lead)
                .child(text)
        };
        let cell = match row.skills {
            Skills::None => quiet(None, "—"),
            Skills::NotManaged => quiet(None, "Not managed"),
            Skills::UpToDate => quiet(
                Some(
                    footer_icon::icon(
                        Icon::Check,
                        self.fg(|t| t.agents_green),
                        ui.scale(13.0) / footer_icon::SIZE,
                    )
                    .into_any_element(),
                ),
                "Up to date",
            ),
            Skills::Act { install } => {
                let kind = row.kind.clone();
                let button = self.act_button(
                    format!("integration-{kind}"),
                    if install { "Install" } else { "Update" },
                    busy,
                    ui,
                    cx,
                    move |this, cx| this.ask_install(&kind, cx),
                );
                return div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .min_w(px(0.0))
                    .child(button)
                    .children(self.cell_note(row.note.clone(), self.fg(|t| t.agents_yellow), ui))
                    .into_any_element();
            }
        };
        div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .min_w(px(0.0))
            .child(cell)
            .children(self.cell_note(row.note.clone(), dim, ui))
            .into_any_element()
    }

    /// The confirmation over the window: which files are written where, who else reads them; or
    /// for a plugin, the one file and what it means.
    pub(super) fn integrations_confirm(
        &self,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let confirm = self.integrations.confirm.as_ref()?;
        let theme = &*self.theme;
        let (dim, yellow, accent, red) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_red),
        );
        let file_list = |files: Vec<(String, bool)>| {
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .px(px(12.0))
                .py(px(10.0))
                .rounded(px(8.0))
                .bg(self.ground())
                .font_family(MONO)
                .text_size(ui.px(12.0))
                .children(files.into_iter().map(|(shown, replace)| {
                    div()
                        .flex()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(shown),
                        )
                        .child(if replace {
                            div().text_color(yellow).child("replace")
                        } else {
                            div().text_color(dim).child("new")
                        })
                }))
        };
        let quiet = |text: String| div().text_size(ui.px(12.0)).text_color(dim).child(text);
        let (title, verb, color, content): (String, &'static str, Hsla, Vec<AnyElement>) =
            match confirm {
                Confirm::Skills(plan) => {
                    let verb = if plan.install { "Install" } else { "Update" };
                    let folders: Vec<String> = plan.folders.iter().map(|f| tilde(f)).collect();
                    let mut says = format!(
                        "Writes {} in {}.",
                        count(plan.files.len(), "file"),
                        listed(&folders)
                    );
                    if !plan.shared.is_empty() {
                        let folder = if folders.len() == 1 {
                            "folder"
                        } else {
                            "folders"
                        };
                        says.push_str(&format!(
                            " {} read the same {folder}, so they get it too.",
                            listed(&plan.shared)
                        ));
                    }
                    let one = (plan.folders.len() == 1).then(|| format!("{}/", plan.folders[0]));
                    let files = plan
                        .files
                        .iter()
                        .map(|(path, replace)| {
                            let shown = match &one {
                                Some(folder) => path
                                    .strip_prefix(folder.as_str())
                                    .unwrap_or(path)
                                    .to_owned(),
                                None => tilde(path),
                            };
                            (shown, *replace)
                        })
                        .collect();
                    (
                        format!("{verb} skills for {}?", plan.name),
                        verb,
                        accent,
                        vec![
                            div().text_color(dim).child(says).into_any_element(),
                            file_list(files).into_any_element(),
                            quiet("Agents already running pick it up after a restart.".into())
                                .into_any_element(),
                        ],
                    )
                }
                Confirm::Plugin(plan) if plan.remove => (
                    format!("Remove the corral plugin for {}?", plan.name),
                    "Remove",
                    red,
                    vec![
                        div()
                            .text_color(dim)
                            .child(format!("Deletes {}.", tilde(&plan.path)))
                            .into_any_element(),
                        quiet(format!(
                            "{0} started by corral will stay Starting until it is installed \
                             again; {0} already open keeps it until restarted.",
                            plan.name
                        ))
                        .into_any_element(),
                    ],
                ),
                Confirm::Plugin(plan) => {
                    let update = plan.status == "overwrite";
                    let verb = if update { "Update" } else { "Install" };
                    let mut content = vec![
                        div()
                            .text_color(dim)
                            .child("Writes 1 file:")
                            .into_any_element(),
                        file_list(vec![(tilde(&plan.path), update)]).into_any_element(),
                    ];
                    content.extend(PLUGIN_SAYS.map(|line| {
                        div()
                            .text_size(ui.px(12.5))
                            .text_color(dim)
                            .child(line)
                            .into_any_element()
                    }));
                    content.push(
                        quiet(format!(
                            "{} already open picks it up after a restart.",
                            plan.name
                        ))
                        .into_any_element(),
                    );
                    (
                        format!("{verb} the corral plugin for {}?", plan.name),
                        verb,
                        accent,
                        content,
                    )
                }
            };
        let button = |id: &'static str, label: &'static str, primary: Option<Hsla>| {
            let base = div()
                .id(id)
                .h(ui.px(30.0))
                .px(px(14.0))
                .flex()
                .items_center()
                .rounded_full()
                .cursor_pointer();
            match primary {
                Some(color) => base
                    .bg(color)
                    .text_color(self.ground())
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(color.opacity(0.85))),
                None => {
                    let (rest, lit) = (self.faint(0.07), self.faint(0.12));
                    base.bg(rest).hover(move |style| style.bg(lit))
                }
            }
            .child(label)
        };
        let panel = div()
            .id("integrations-confirm")
            .w(ui.px(460.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .px(px(20.0))
            .pt(px(20.0))
            .pb(px(16.0))
            .rounded(ui.px(12.0))
            .border_1()
            .border_color(self.faint(0.12))
            .bg(crate::popover::ground(theme))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .text_size(ui.px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .children(content)
            .child(
                div()
                    .mt(px(4.0))
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(button("integrations-cancel", "Cancel", None).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.cancel_install(cx)),
                    ))
                    .child(
                        button("integrations-go", verb, Some(color))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.install(cx))),
                    ),
            );
        Some(
            div()
                .id("integrations-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .bg(gpui::black().opacity(0.45))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.cancel_install(cx)),
                )
                .child(panel),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_read_as_english() {
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(listed(&names(&["Codex"])), "Codex");
        assert_eq!(listed(&names(&["Codex", "omp"])), "Codex and omp");
        assert_eq!(
            listed(&names(&["Claude Code", "Codex", "pi"])),
            "Claude Code, Codex and pi"
        );
        assert_eq!(count(1, "file"), "1 file");
        assert_eq!(count(4, "file"), "4 files");
    }
}
