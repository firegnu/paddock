//! Settings › Agents › Integrations (DESIGN §13 P5-94): a row per agent corral supports, read in
//! the background from `crate::integrations`, and the confirmation before skills are written.
use super::*;
use crate::integrations::{self, Plan, Row, Skills};
use std::sync::atomic::AtomicBool;

/// The section's state in the Settings window.
#[derive(Default)]
pub(super) struct Integrations {
    /// The last reading, or why there is none.
    read: Option<Result<integrations::Report, String>>,
    checking: bool,
    /// The confirmation open over the window.
    confirm: Option<Plan>,
    installing: bool,
}

/// Below this width the Status and Skills columns go under the agent's name.
const NARROW: f32 = 520.0;

/// The Status and Skills columns.
const STATUS_WIDTH: f32 = 110.0;
const SKILLS_WIDTH: f32 = 230.0;

const NOTE: [&str; 2] = [
    "Status comes with corral each time it starts an agent; there is nothing to install.",
    "Skills teach an agent the corral and paddock commands.",
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
        if self.integrations.checking || self.integrations.installing {
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
            self.integrations.confirm = integrations::plan(report, kind);
            cx.notify();
        }
    }

    fn cancel_install(&mut self, cx: &mut Context<Self>) {
        self.integrations.confirm = None;
        cx.notify();
    }

    /// Writes what the confirmation showed, says how it went in the bar, and reads again.
    fn install(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.integrations.confirm.take() else {
            return;
        };
        let Some(corral) = crate::windows::report(cx).map(|report| report.corral) else {
            return;
        };
        self.integrations.installing = true;
        let task = cx.background_spawn({
            let plan = plan.clone();
            async move { integrations::install(&corral, &plan, &AtomicBool::new(false)) }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.integrations.installing = false;
                match result {
                    Ok(()) => {
                        let done = if plan.install { "Installed" } else { "Updated" };
                        let files = count(plan.files.len(), "file");
                        this.say(format!("{done} {files} for {}", plan.name), false, cx);
                    }
                    Err(error) => this.say(
                        format!("Couldn't install skills for {}: {error}", plan.name),
                        true,
                        cx,
                    ),
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

    /// The section: its heading with Refresh or `Checking…`, the rows, and what the columns mean.
    pub(super) fn integrations_section(
        &self,
        width: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.integrations;
        let busy = state.checking || state.installing;
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
                    .child(if state.installing {
                        "Installing…"
                    } else {
                        "Checking…"
                    })
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
        let note = self.fg(|t| t.agents_dim);
        self.section_with(heading, ui)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(body)
                    .children(NOTE.map(|line| self.under(line, note, ui))),
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
        let status = self.integration_status(&row, ui);
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

    /// `Built in`, or corral's own word for a way paddock doesn't know.
    fn integration_status(&self, row: &Row, ui: &UiFont) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        match row.status.as_deref() {
            None => div().text_color(dim).child("—").into_any_element(),
            Some("Built in") => {
                let green = self.fg(|t| t.agents_green);
                div()
                    .flex()
                    .child(
                        div()
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
                            .child("Built in"),
                    )
                    .into_any_element()
            }
            Some(other) => div()
                .text_size(ui.px(12.0))
                .text_color(dim)
                .child(other.to_owned())
                .into_any_element(),
        }
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
        let note = |color: Hsla| {
            row.note.clone().map(|note| {
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(12.0))
                    .text_color(color)
                    .child(note)
            })
        };
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
                let accent = self.fg(|t| t.agents_accent);
                let kind = row.kind.clone();
                let button = div()
                    .id(ElementId::Name(format!("integration-{kind}").into()))
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
                    .child(if install { "Install" } else { "Update" });
                let button = if busy {
                    button.opacity(0.5)
                } else {
                    button
                        .cursor_pointer()
                        .hover(move |style| style.bg(accent.opacity(0.24)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.ask_install(&kind, cx)
                        }))
                };
                return div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .min_w(px(0.0))
                    .child(button)
                    .children(note(self.fg(|t| t.agents_yellow)))
                    .into_any_element();
            }
        };
        div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .min_w(px(0.0))
            .child(cell)
            .children(note(dim))
            .into_any_element()
    }

    /// The confirmation over the window: which files are written where, who else reads them.
    pub(super) fn integrations_confirm(
        &self,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let plan = self.integrations.confirm.as_ref()?;
        let theme = &*self.theme;
        let (dim, yellow, accent) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_accent),
        );
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
        let files = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(8.0))
            .bg(self.ground())
            .font_family(MONO)
            .text_size(ui.px(12.0))
            .children(plan.files.iter().map(|(path, replace)| {
                let shown = match &one {
                    Some(folder) => path
                        .strip_prefix(folder.as_str())
                        .unwrap_or(path)
                        .to_owned(),
                    None => tilde(path),
                };
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
                    .child(if *replace {
                        div().text_color(yellow).child("replace")
                    } else {
                        div().text_color(dim).child("new")
                    })
            }));
        let button = |id: &'static str, label: &'static str, primary: bool| {
            let base = div()
                .id(id)
                .h(ui.px(30.0))
                .px(px(14.0))
                .flex()
                .items_center()
                .rounded_full()
                .cursor_pointer();
            if primary {
                base.bg(accent)
                    .text_color(self.ground())
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(accent.opacity(0.85)))
            } else {
                let (rest, lit) = (self.faint(0.07), self.faint(0.12));
                base.bg(rest).hover(move |style| style.bg(lit))
            }
            .child(label)
        };
        let panel = div()
            .id("integrations-confirm")
            .w(ui.px(440.0))
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
                    .child(format!("{verb} skills for {}?", plan.name)),
            )
            .child(div().text_color(dim).child(says))
            .child(files)
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .child("Agents already running pick it up after a restart."),
            )
            .child(
                div()
                    .mt(px(4.0))
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(button("integrations-cancel", "Cancel", false).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.cancel_install(cx)),
                    ))
                    .child(
                        button("integrations-go", verb, true)
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
