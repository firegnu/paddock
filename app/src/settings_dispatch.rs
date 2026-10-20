//! Settings › Dispatch (DESIGN §13 P5-161): ranch's dispatch table as a graph, and one cell
//! changed through ranch. The cells down the left, each with ranch's description and its model and
//! effort; the agents they go to in the middle, a coloured line from each cell to each of its
//! agents; cross review and rework as dashed and dotted arrows round the left. A cell picked is
//! edited on the right: its agents, model, effort, cross review and rework cell, saved with
//! `ranch dispatch edit` or put back with `reset`, each checked against the version read; the
//! table ranch answers with is shown. An agent picked shows its template, view only. Which cells
//! and agents there are, and their order, come from ranch.
use super::*;
use crate::{
    dispatch::{self, Cell, Draft, Problem, Refusal, Table},
    fence::Cancel,
};
use gpui::{PathBuilder, point};
use std::sync::atomic::{AtomicBool, Ordering};

/// What the right side shows.
#[derive(Clone, Debug, PartialEq)]
enum Selected {
    Cell(String),
    Agent(String),
}

/// The open list under one of the editor's choices.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Menu {
    Model,
    Effort,
    Review,
    Rework,
}

/// The page's state in the Settings window.
#[derive(Default)]
pub(super) struct Dispatch {
    /// The table as last read or written; none before the first read.
    read: Option<Result<Table, Refusal>>,
    /// Stops a read still running once it is not wanted.
    reading: Option<Cancel>,
    selected: Option<Selected>,
    /// What is changed in the selected cell and not saved.
    draft: Draft,
    /// The model's field, for typing a model of one's own.
    model: Option<(Entity<TextInput>, Subscription)>,
    open: Option<Menu>,
    /// A save or reset is under way.
    saving: bool,
    /// Why ranch refused the last save or reset.
    refused: Option<Refusal>,
}

impl Dispatch {
    fn table(&self) -> Option<&Table> {
        self.read.as_ref()?.as_ref().ok()
    }

    /// The selected cell as read.
    fn cell(&self) -> Option<&Cell> {
        match &self.selected {
            Some(Selected::Cell(name)) => self.table()?.cell(name),
            _ => None,
        }
    }

    /// The selected cell is changed and not saved.
    fn pending(&self) -> bool {
        self.cell()
            .is_some_and(|cell| !self.draft.options(cell).is_empty())
    }
}

/// The graph's measures at the base interface size. The cells' column takes what room there is
/// between these, the editor beside it getting at least `EDITOR_MIN` (a third of the page).
const GUTTER: f32 = 36.0;
const CELL_MIN: f32 = 190.0;
const CELL_MAX: f32 = 320.0;
const CELL_H: f32 = 50.0;
const CELL_GAP: f32 = 8.0;
const LINK_W: f32 = 64.0;
const AGENT_W: f32 = 88.0;
const AGENT_H: f32 = 34.0;
const AGENT_GAP: f32 = 10.0;
/// The editor beside the graph is at least this wide; less room puts it under the graph.
const EDITOR_MIN: f32 = 230.0;
const EDITOR_GAP: f32 = 20.0;

const ABOUT: &str = "Which agents each kind of work goes to, with what model and effort. Read \
from `ranch dispatch table`; a change takes effect at the next dispatch.";

const DEFAULT: &str = "Each agent's default";

/// How a line is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Stroke {
    /// A cell to an agent it goes to.
    GoesTo,
    CrossReview,
    Rework,
}

/// One line of the graph, in the graph's own points.
#[derive(Clone)]
struct Line {
    from: (f32, f32),
    to: (f32, f32),
    stroke: Stroke,
    color: Hsla,
    width: f32,
    /// How far an arrow round the left bulges.
    bend: f32,
}

/// Paints `lines` from `origin`; `unit` scales the dashes and arrowheads.
fn paint_lines(origin: gpui::Point<Pixels>, lines: &[Line], unit: f32, window: &mut Window) {
    let at = |(x, y): (f32, f32)| origin + point(px(x), px(y));
    for line in lines {
        let mut path = PathBuilder::stroke(px(line.width));
        path = match line.stroke {
            Stroke::GoesTo => path,
            Stroke::CrossReview => path.dash_array(&[px(5.0 * unit), px(4.0 * unit)]),
            Stroke::Rework => path.dash_array(&[px(1.6 * unit), px(3.0 * unit)]),
        };
        let (from, to) = (line.from, line.to);
        path.move_to(at(from));
        match line.stroke {
            Stroke::GoesTo => {
                let half = (to.0 - from.0) / 2.0;
                path.cubic_bezier_to(at(to), at((from.0 + half, from.1)), at((to.0 - half, to.1)));
            }
            Stroke::CrossReview | Stroke::Rework => {
                let head = 6.0 * unit;
                // Round the left, ending where the arrowhead starts.
                path.cubic_bezier_to(
                    at((to.0 - head, to.1)),
                    at((from.0 - line.bend, from.1)),
                    at((to.0 - line.bend, to.1)),
                );
                let mut arrow = PathBuilder::fill();
                arrow.move_to(at(to));
                arrow.line_to(at((to.0 - head, to.1 - 3.5 * unit)));
                arrow.line_to(at((to.0 - head, to.1 + 3.5 * unit)));
                arrow.close();
                if let Ok(arrow) = arrow.build() {
                    window.paint_path(arrow, line.color);
                }
            }
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, line.color);
        }
    }
}

/// A short sample of `stroke`, for the legend.
fn sample(stroke: Stroke, color: Hsla, ui: &UiFont) -> impl IntoElement {
    let (width, height) = (ui.scale(26.0), ui.scale(8.0));
    let unit = ui.scale(1.0);
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.6 * unit));
            path = match stroke {
                Stroke::GoesTo => path,
                Stroke::CrossReview => path.dash_array(&[px(5.0 * unit), px(4.0 * unit)]),
                Stroke::Rework => path.dash_array(&[px(1.6 * unit), px(3.0 * unit)]),
            };
            let y = bounds.origin.y + px(height / 2.0);
            path.move_to(point(bounds.origin.x, y));
            path.line_to(point(bounds.origin.x + px(width), y));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .flex_shrink_0()
    .w(px(width))
    .h(px(height))
}

/// `n` cells in words.
fn cells(n: usize) -> String {
    if n == 1 {
        "1 cell".into()
    } else {
        format!("{n} cells")
    }
}

impl SettingsView {
    /// Reads the table again, unless a change is drafted or being saved: what is being changed
    /// stays checked against the version it was read at.
    pub(super) fn refresh_dispatch(&mut self, cx: &mut Context<Self>) {
        if self.dispatch.saving || self.dispatch.pending() {
            return;
        }
        self.read_dispatch(cx);
    }

    /// Another page: a read still running stops, and an open list closes.
    pub(super) fn leave_dispatch(&mut self) {
        self.dispatch.reading = None;
        self.dispatch.open = None;
    }

    /// Reads the table off the UI thread; what is drafted stays.
    fn read_dispatch(&mut self, cx: &mut Context<Self>) {
        let reading = Cancel::default();
        let flag = reading.flag();
        let stopped = flag.clone();
        self.dispatch.reading = Some(reading);
        let task = cx.background_spawn(async move { dispatch::table(dispatch::PROGRAM, &flag) });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            let _ = this.update(cx, |this, cx| {
                // Stopped for another page or a newer read: its answer is not wanted.
                if stopped.load(Ordering::Relaxed) {
                    return;
                }
                this.dispatch.reading = None;
                this.take_dispatch(read, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Shows `read`: what was selected stays when it is still there, else the first cell.
    fn take_dispatch(&mut self, read: Result<Table, Refusal>, cx: &mut Context<Self>) {
        let state = &mut self.dispatch;
        if let Ok(table) = &read {
            let there = match &state.selected {
                Some(Selected::Cell(name)) => table.cell(name).is_some(),
                Some(Selected::Agent(name)) => table.agent_order().contains(name),
                None => false,
            };
            if !there {
                state.selected = table
                    .cells
                    .first()
                    .map(|cell| Selected::Cell(cell.name.clone()));
                state.draft = Draft::default();
            }
            // Read again: a conflict is no longer the news.
            if state
                .refused
                .as_ref()
                .is_some_and(|refusal| refusal.problem() == Problem::Conflict)
            {
                state.refused = None;
            }
        }
        state.read = Some(read);
        self.load_model(cx);
    }

    /// The model's field shows the selected cell's model as drafted.
    fn load_model(&mut self, cx: &mut Context<Self>) {
        let text = self
            .dispatch
            .cell()
            .map(|cell| self.dispatch.draft.model(cell).to_owned())
            .unwrap_or_default();
        if let Some((input, _)) = &self.dispatch.model {
            input.update(cx, |input, cx| input.set_text(text, cx));
            return;
        }
        let colors = self.input_colors();
        let input = cx.new(|cx| TextInput::new(text, DEFAULT, colors, cx));
        let typed = cx.subscribe(&input, |this, input, _: &text_input::Changed, cx| {
            let text = input.read(cx).text().to_owned();
            let state = &mut this.dispatch;
            if let Some(cell) = state.cell()
                && (state.draft.model.is_some() || text != state.draft.model(cell))
            {
                state.draft.model = Some(text);
                cx.notify();
            }
        });
        self.dispatch.model = Some((input, typed));
    }

    /// The model's field in the theme's colours, after the theme changed.
    pub(super) fn recolor_dispatch(&mut self, cx: &mut Context<Self>) {
        let colors = self.input_colors();
        if let Some((input, _)) = &self.dispatch.model {
            input.update(cx, |input, cx| input.set_colors(colors, cx));
        }
    }

    /// Another cell or agent on the right; what was drafted for the one before is dropped.
    fn select_dispatch(&mut self, selected: Selected, cx: &mut Context<Self>) {
        let state = &mut self.dispatch;
        state.open = None;
        if state.selected.as_ref() != Some(&selected) {
            state.selected = Some(selected);
            state.draft = Draft::default();
            state.refused = None;
            self.load_model(cx);
        }
        cx.notify();
    }

    /// Changes the draft of the selected cell.
    fn draft_dispatch(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&Cell, &mut Draft)) {
        let state = &mut self.dispatch;
        state.open = None;
        if let Some(cell) = state.cell().cloned() {
            change(&cell, &mut state.draft);
        }
        cx.notify();
    }

    fn save_dispatch(&mut self, cx: &mut Context<Self>) {
        let state = &self.dispatch;
        let (Some(table), Some(cell)) = (state.table(), state.cell()) else {
            return;
        };
        let Some(version) = &table.local.version else {
            return;
        };
        if let Some(args) = dispatch::edit_args(cell, &state.draft, version) {
            self.change_dispatch(args, "Saved. The next dispatch uses it.", cx);
        }
    }

    fn reset_dispatch(&mut self, cx: &mut Context<Self>) {
        let state = &self.dispatch;
        let (Some(table), Some(cell)) = (state.table(), state.cell()) else {
            return;
        };
        let Some(version) = &table.local.version else {
            return;
        };
        let args = dispatch::reset_args(&cell.name, version);
        self.change_dispatch(args, "Back to the default. The next dispatch uses it.", cx);
    }

    /// Runs a save or reset off the UI thread; the table ranch answers with is shown, or why it
    /// refused, the draft kept.
    fn change_dispatch(&mut self, args: Vec<String>, done: &'static str, cx: &mut Context<Self>) {
        let state = &mut self.dispatch;
        state.saving = true;
        state.refused = None;
        state.open = None;
        // A read still running would show the table from before this change.
        state.reading = None;
        let task = cx.background_spawn(async move {
            dispatch::change(dispatch::PROGRAM, &args, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.dispatch.saving = false;
                match answer {
                    Ok(table) => {
                        this.dispatch.draft = Draft::default();
                        this.take_dispatch(Ok(table), cx);
                        this.say(done, false, cx);
                    }
                    Err(refusal) => this.dispatch.refused = Some(refusal),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// After a conflict: the table as it is now, what is drafted kept over it.
    fn reload_dispatch(&mut self, cx: &mut Context<Self>) {
        self.dispatch.refused = None;
        self.read_dispatch(cx);
    }

    /// Each agent's colour: Claude's and Codex's own, the others in turn from the theme's.
    fn dispatch_colors(&self, table: &Table) -> Vec<(String, Hsla)> {
        let others = [
            self.fg(|t| t.agents_blue),
            self.fg(|t| t.agents_green),
            self.fg(|t| t.agents_sandbox),
            self.fg(|t| t.agents_purple),
            self.fg(|t| t.agents_red),
            self.fg(|t| t.agents_accent),
        ];
        let mut names: Vec<String> = table.agents.iter().map(|(name, _)| name.clone()).collect();
        for name in table.agent_order() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let mut next = 0;
        names
            .into_iter()
            .map(|name| {
                let color = match name.as_str() {
                    "claude" => self.fg(|t| t.claude),
                    "codex" => self.fg(|t| t.codex),
                    _ => {
                        next += 1;
                        others[(next - 1) % others.len()]
                    }
                };
                (name, color)
            })
            .collect()
    }

    /// The page.
    pub(super) fn dispatch_page(
        &self,
        width: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (dim, red) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_red));
        let quiet = |text: String| {
            div()
                .px(px(4.0))
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child(text)
                .into_any_element()
        };
        let table = match &self.dispatch.read {
            None => return vec![quiet("Reading the dispatch table…".into())],
            Some(Err(refusal)) => {
                return vec![
                    div()
                        .px(px(4.0))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(10.0))
                        .text_size(ui.px(12.5))
                        .child(
                            div()
                                .text_color(red)
                                .child(format!("Couldn't read the dispatch table: {refusal}")),
                        )
                        .child(self.dispatch_button(
                            "dispatch-retry",
                            "Retry",
                            false,
                            self.dispatch.reading.is_none(),
                            ui,
                            cx,
                            |this, cx| this.read_dispatch(cx),
                        ))
                        .into_any_element(),
                ];
            }
            Some(Ok(table)) => table,
        };
        let mut items = vec![self.dispatch_header(table, ui).into_any_element()];
        if !table.editable() {
            items.push(self.dispatch_note(
                "This ranch can't change the table from here (it is older than R35): update \
                 ranch to edit it. View only meanwhile.",
                self.fg(|t| t.agents_yellow),
                ui,
            ));
        }
        if table.cells.is_empty() {
            items.push(quiet("ranch lists no cells.".into()));
            return items;
        }
        // The cells' width: what is left beside the editor, or under it when too little is.
        let fixed = ui.scale(GUTTER + LINK_W + AGENT_W);
        let editor_w = ui.scale(EDITOR_MIN).max(width / 3.0);
        let room = width - ui.scale(EDITOR_GAP) - editor_w - fixed;
        let beside = room >= ui.scale(CELL_MIN);
        let cell_w =
            if beside { room } else { width - fixed }.clamp(ui.scale(CELL_MIN), ui.scale(CELL_MAX));
        let graph = self.dispatch_graph(table, cell_w, ui, cx);
        let editor = self.dispatch_editor(table, ui, cx);
        let body = if beside {
            div()
                .flex()
                .items_start()
                .gap(ui.px(EDITOR_GAP))
                .child(graph)
                .child(div().flex_1().min_w(px(0.0)).child(editor))
        } else {
            div()
                .flex()
                .flex_col()
                .gap(ui.px(22.0))
                .child(graph)
                .child(editor)
        };
        items.push(body.into_any_element());
        items
    }

    /// What the page is, the file and how many cells it changes; the legend at the right.
    fn dispatch_header(&self, table: &Table, ui: &UiFont) -> Div {
        let (dim, dimmer) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_dimmer));
        let local = &table.local;
        let state = if !local.exists {
            "not created yet".to_owned()
        } else {
            match table.changed() {
                0 => "no cell changed".to_owned(),
                n => format!("{} changed", cells(n)),
            }
        };
        let legend_item = |stroke: Stroke, color: Hsla, label: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(sample(stroke, color, ui))
                .child(label)
        };
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap(ui.px(16.0))
            .child(
                div()
                    .flex_1()
                    .min_w(ui.px(280.0))
                    .px(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(ui.px(12.5))
                            .line_height(relative(1.5))
                            .text_color(dim)
                            .child(ABOUT),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.0))
                            .text_size(ui.px(12.0))
                            .text_color(dimmer)
                            .child(
                                div()
                                    .font_family(MONO)
                                    .text_color(dim)
                                    .child(shown_path(std::path::Path::new(&local.path))),
                            )
                            .child(format!("· {state}")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(14.0))
                    .text_size(ui.px(11.5))
                    .text_color(dim)
                    .child(legend_item(Stroke::GoesTo, dim, "Goes to"))
                    .child(legend_item(Stroke::CrossReview, dim, "Cross review"))
                    .child(legend_item(
                        Stroke::Rework,
                        self.fg(|t| t.agents_yellow),
                        "Rework moves to",
                    )),
            )
    }

    /// A line of note: a warning sign and `text` in `color`.
    fn dispatch_note(&self, text: impl Into<SharedString>, color: Hsla, ui: &UiFont) -> AnyElement {
        div()
            .px(px(4.0))
            .flex()
            .items_start()
            .gap(px(8.0))
            .text_size(ui.px(12.0))
            .line_height(relative(1.5))
            .text_color(color)
            .child(div().mt(px(2.0)).child(footer_icon::icon(
                Icon::Warning,
                color,
                ui.scale(12.0) / footer_icon::SIZE,
            )))
            .child(div().flex_1().min_w(px(0.0)).child(text.into()))
            .into_any_element()
    }

    /// The cells, the agents and the lines between.
    fn dispatch_graph(
        &self,
        table: &Table,
        cell_w: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let (text, dim, dimmer, accent, yellow) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_yellow),
        );
        let s = |n: f32| ui.scale(n);
        let colors = self.dispatch_colors(table);
        let color_of = |name: &str| {
            colors
                .iter()
                .find(|(known, _)| known == name)
                .map_or(dim, |(_, color)| *color)
        };
        let order = table.agent_order();
        let rows = table.cells.len();
        let cells_h = rows as f32 * s(CELL_H + CELL_GAP) - s(CELL_GAP);
        let agents_h = order.len() as f32 * s(AGENT_H + AGENT_GAP) - s(AGENT_GAP);
        let height = cells_h.max(agents_h);
        let slot = height / order.len().max(1) as f32;
        let cell_y = |index: usize| index as f32 * s(CELL_H + CELL_GAP) + s(CELL_H) / 2.0;
        let agent_y = |index: usize| slot * (index as f32 + 0.5);
        let (cells_right, agents_left) = (s(GUTTER) + cell_w, s(GUTTER + LINK_W) + cell_w);
        let selected = self.dispatch.selected.as_ref();
        // Lines into or out of what is selected stand out; the rest fade.
        let lit = |cell: &str, agent: Option<&str>| match selected {
            None => true,
            Some(Selected::Cell(name)) => name == cell,
            Some(Selected::Agent(name)) => agent == Some(name.as_str()),
        };
        let look = |color: Hsla, on: bool| {
            if on {
                (color, s(2.2))
            } else {
                (color.opacity(0.28), s(1.2))
            }
        };
        let mut lines = Vec::new();
        let mut arrows = Vec::new();
        for (index, cell) in table.cells.iter().enumerate() {
            for agent in &cell.agents {
                let Some(place) = order.iter().position(|name| name == agent) else {
                    continue;
                };
                let (color, width) = look(color_of(agent), lit(&cell.name, Some(agent)));
                lines.push(Line {
                    from: (cells_right, cell_y(index)),
                    to: (agents_left, agent_y(place)),
                    stroke: Stroke::GoesTo,
                    color,
                    width,
                    bend: 0.0,
                });
            }
            let links = [
                (&cell.cross_review, Stroke::CrossReview, dim, 10.0, 3.5),
                (&cell.rework, Stroke::Rework, yellow, 6.0, 2.5),
            ];
            for (target, stroke, color, base, step) in links {
                let Some(target) = target.as_deref() else {
                    continue;
                };
                let Some(to) = table.cells.iter().position(|c| c.name == target) else {
                    continue;
                };
                let on = lit(&cell.name, None) || lit(target, None);
                let (color, width) = look(color, on);
                let apart = index.abs_diff(to) as f32;
                arrows.push(Line {
                    from: (s(GUTTER), cell_y(index)),
                    to: (s(GUTTER), cell_y(to)),
                    stroke,
                    color,
                    width: width.min(s(1.8)),
                    bend: s((base + step * apart).min(GUTTER - 6.0)),
                });
            }
        }
        // The lit ones over the faded.
        lines.sort_by_key(|line| line.width > s(1.5));
        lines.extend(arrows);
        let unit = s(1.0);
        let mut graph = div()
            .relative()
            .flex_shrink_0()
            .w(px(s(GUTTER + LINK_W + AGENT_W) + cell_w))
            .h(px(height))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| paint_lines(bounds.origin, &lines, unit, window),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let field = self.field_ground();
        for (index, cell) in table.cells.iter().enumerate() {
            let on = selected == Some(&Selected::Cell(cell.name.clone()));
            let small = |words: String, color: Hsla| {
                div()
                    .flex_shrink_0()
                    .px(px(6.0))
                    .h(ui.px(17.0))
                    .flex()
                    .items_center()
                    .rounded(px(4.0))
                    .bg(color.opacity(0.14))
                    .text_size(ui.px(10.5))
                    .text_color(color)
                    .child(words)
            };
            let mut title = div().flex().items_center().gap(px(6.0)).child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(ui.px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(text)
                    .child(cell.name.clone()),
            );
            if cell.changed() {
                title = title.child(small("Changed".into(), accent));
            }
            if cell.agents.is_empty() {
                title = title.child(small("No agent".into(), yellow));
            }
            title = title.child(div().flex_1()).child(
                div()
                    .flex_shrink_0()
                    .max_w(px(cell_w * 0.5))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .px(px(6.0))
                    .h(ui.px(17.0))
                    .line_height(ui.px(17.0))
                    .rounded(px(4.0))
                    .bg(field)
                    .font_family(MONO)
                    .text_size(ui.px(10.5))
                    .text_color(dim)
                    .child(cell.tag()),
            );
            let name = cell.name.clone();
            let hover = self.lifted(0.07);
            let card = div()
                .id(ElementId::NamedInteger(
                    "dispatch-cell".into(),
                    index as u64,
                ))
                .absolute()
                .left(px(s(GUTTER)))
                .top(px(index as f32 * s(CELL_H + CELL_GAP)))
                .w(px(cell_w))
                .h(px(s(CELL_H)))
                .px(px(10.0))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.0))
                .rounded(px(8.0))
                .border_1()
                .cursor_pointer()
                .child(title)
                .children((!cell.description.is_empty()).then(|| {
                    div()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(ui.px(11.5))
                        .text_color(dimmer)
                        .child(cell.description.clone())
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.select_dispatch(Selected::Cell(name.clone()), cx)
                }));
            graph = graph.child(if on {
                card.bg(self.card_ground().blend(accent.opacity(0.1)))
                    .border_color(accent.opacity(0.55))
            } else {
                card.bg(self.card_ground())
                    .border_color(self.faint(0.06))
                    .hover(move |style| style.bg(hover))
            });
        }
        for (index, name) in order.iter().enumerate() {
            let color = color_of(name);
            let on = selected == Some(&Selected::Agent(name.clone()));
            let used = table.cells.iter().any(|cell| cell.agents.contains(name));
            let chosen = name.clone();
            let hover = self.lifted(0.07);
            let card = div()
                .id(ElementId::NamedInteger(
                    "dispatch-agent".into(),
                    index as u64,
                ))
                .absolute()
                .left(px(agents_left))
                .top(px(agent_y(index) - s(AGENT_H) / 2.0))
                .w(px(s(AGENT_W)))
                .h(px(s(AGENT_H)))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(7.0))
                .rounded(px(8.0))
                .border_1()
                .cursor_pointer()
                .text_size(ui.px(12.5))
                .text_color(if used { text } else { dimmer })
                .child(div().flex_shrink_0().size(px(7.0)).rounded_full().bg(color))
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(name.clone()),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.select_dispatch(Selected::Agent(chosen.clone()), cx)
                }));
            graph = graph.child(if on {
                card.bg(self.card_ground().blend(color.opacity(0.14)))
                    .border_color(color.opacity(0.8))
            } else {
                card.bg(self.card_ground())
                    .border_color(color.opacity(0.35))
                    .hover(move |style| style.bg(hover))
            });
        }
        graph
    }

    /// The right side: the selected cell to edit, or the selected agent's template.
    fn dispatch_editor(&self, table: &Table, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let panel = div()
            .px(px(16.0))
            .py(px(14.0))
            .flex()
            .flex_col()
            .gap(ui.px(12.0))
            .rounded(px(12.0))
            .bg(self.card_ground());
        match &self.dispatch.selected {
            Some(Selected::Agent(name)) => panel.children(self.dispatch_agent(table, name, ui)),
            Some(Selected::Cell(name)) => match table.cell(name) {
                Some(cell) => panel.children(self.dispatch_cell(table, cell, ui, cx)),
                None => panel,
            },
            None => panel,
        }
    }

    /// A label over what it names.
    fn dispatch_label(&self, words: &'static str, ui: &UiFont) -> Div {
        div()
            .text_size(ui.px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.fg(|t| t.agents_dimmer))
            .child(words.to_uppercase())
    }

    /// An agent's template, view only.
    fn dispatch_agent(&self, table: &Table, name: &str, ui: &UiFont) -> Vec<AnyElement> {
        let (text, dim, dimmer) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
        );
        let color = self
            .dispatch_colors(table)
            .into_iter()
            .find(|(known, _)| known == name)
            .map_or(dim, |(_, color)| color);
        let title = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().size(px(8.0)).rounded_full().bg(color))
            .child(
                div()
                    .text_size(ui.px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text)
                    .child(name.to_owned()),
            );
        let Some(agent) = table.agent(name) else {
            return vec![
                title.into_any_element(),
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .child("ranch lists no template for this agent.")
                    .into_any_element(),
            ];
        };
        let mono = |words: String| {
            div()
                .font_family(MONO)
                .text_size(ui.px(11.5))
                .line_height(relative(1.6))
                .text_color(dim)
                .child(words)
        };
        let labels = if agent.labels.is_empty() {
            div()
                .text_size(ui.px(12.0))
                .text_color(dimmer)
                .child("None")
        } else {
            mono(agent.labels.join("  "))
        };
        vec![
            title
                .child(
                    div()
                        .text_size(ui.px(11.5))
                        .text_color(dimmer)
                        .child(format!(
                            "Template · {}",
                            crate::fence::origin(&agent.source)
                        )),
                )
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(self.dispatch_label("Command", ui))
                .child(mono(shell_words::join(&agent.argv)))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(self.dispatch_label("Labels", ui))
                .child(labels)
                .into_any_element(),
            div()
                .text_size(ui.px(11.5))
                .text_color(dimmer)
                .child("View only. Templates are changed in ranch's file, not here.")
                .into_any_element(),
        ]
    }

    /// The selected cell: what it goes to, its model, effort, cross review and rework, how it
    /// starts, why ranch refused the last change, and Reset to default and Save.
    fn dispatch_cell(
        &self,
        table: &Table,
        cell: &Cell,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (text, dim, dimmer, accent, yellow) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_yellow),
        );
        let state = &self.dispatch;
        let draft = &state.draft;
        let can = table.editable() && !state.saving;
        let mut items = Vec::new();
        let mut title = div().flex().flex_wrap().items_center().gap(px(8.0)).child(
            div()
                .text_size(ui.px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .child(cell.name.clone()),
        );
        title = title.child(
            div()
                .text_size(ui.px(11.5))
                .text_color(if cell.changed() { accent } else { dimmer })
                .child(if cell.changed() {
                    "Changed on this Mac".to_owned()
                } else {
                    crate::fence::origin(&cell.source)
                }),
        );
        items.push(title.into_any_element());
        if !cell.description.is_empty() {
            items.push(
                div()
                    .text_size(ui.px(12.0))
                    .line_height(relative(1.5))
                    .text_color(dim)
                    .child(cell.description.clone())
                    .into_any_element(),
            );
        }
        let agents = draft.agents(cell).to_vec();
        if agents.is_empty() {
            items.push(self.dispatch_note(
                "No agent: the controller can't dispatch work from this cell.",
                yellow,
                ui,
            ));
        }
        // Goes to: every agent, those chosen lit in their colour.
        let colors = self.dispatch_colors(table);
        let mut chips = div().flex().flex_wrap().gap(px(6.0));
        for (index, (name, color)) in colors.iter().enumerate() {
            let on = agents.contains(name);
            let chosen = name.clone();
            let chip = div()
                .id(ElementId::NamedInteger(
                    "dispatch-goes".into(),
                    index as u64,
                ))
                .h(ui.px(26.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .rounded_full()
                .border_1()
                .text_size(ui.px(12.0))
                .child(div().size(px(6.0)).rounded_full().bg(*color))
                .child(name.clone());
            let chip = if on {
                chip.bg(color.opacity(0.16))
                    .border_color(color.opacity(0.6))
                    .text_color(text)
            } else {
                chip.border_color(self.faint(0.1)).text_color(dim)
            };
            chips = chips.child(if can {
                let hover = self.faint(0.06);
                chip.cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        let chosen = chosen.clone();
                        this.draft_dispatch(cx, |cell, draft| {
                            let mut agents = draft.agents(cell).to_vec();
                            match agents.iter().position(|name| *name == chosen) {
                                Some(at) => {
                                    agents.remove(at);
                                }
                                None => agents.push(chosen),
                            }
                            draft.agents = Some(agents);
                        })
                    }))
            } else {
                chip
            });
        }
        items.push(self.dispatch_row("Goes to", chips, ui).into_any_element());
        // Model: typed, or picked from what the chosen agents suggest.
        let models = table.models(&agents);
        let model = draft.model(cell);
        let model_field = match (&state.model, can) {
            (Some((input, _)), true) => div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_size(ui.px(12.5))
                .child(input.clone()),
            _ => div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .text_size(ui.px(12.5))
                .text_color(if model.is_empty() { dimmer } else { text })
                .child(if model.is_empty() {
                    DEFAULT.to_owned()
                } else {
                    model.to_owned()
                }),
        };
        let mut model_choices = vec![(String::new(), DEFAULT.to_owned())];
        model_choices.extend(models.iter().map(|m| (m.clone(), m.clone())));
        let model_box = self.dispatch_select(
            Menu::Model,
            model_field,
            model_choices,
            model,
            can,
            ui,
            cx,
            |this, value, cx| {
                if let Some((input, _)) = &this.dispatch.model {
                    let shown = value.clone();
                    input.update(cx, |input, cx| input.set_text(shown, cx));
                }
                this.draft_dispatch(cx, |_, draft| draft.model = Some(value));
            },
        );
        items.push(self.dispatch_row("Model", model_box, ui).into_any_element());
        // Effort: what every chosen agent takes, or each one's own.
        let efforts = table.efforts(&agents);
        let effort = draft.effort(cell);
        let mut effort_choices = vec![(String::new(), DEFAULT.to_owned())];
        effort_choices.extend(efforts.iter().map(|e| (e.clone(), e.clone())));
        let effort_box = self.dispatch_select(
            Menu::Effort,
            self.dispatch_shown(effort, DEFAULT, ui),
            effort_choices,
            effort,
            can,
            ui,
            cx,
            |this, value, cx| this.draft_dispatch(cx, |_, draft| draft.effort = Some(value)),
        );
        let mut effort_row = self.dispatch_row("Effort", effort_box, ui);
        if !effort.is_empty() && !efforts.iter().any(|e| e == effort) {
            effort_row = effort_row.child(self.dispatch_note(
                format!("Not an effort every chosen agent takes: ranch will refuse {effort}."),
                yellow,
                ui,
            ));
        } else if efforts.is_empty() && !agents.is_empty() {
            effort_row = effort_row.child(
                div()
                    .px(px(4.0))
                    .text_size(ui.px(11.5))
                    .text_color(dimmer)
                    .child("The chosen agents take no effort in common: each uses its own."),
            );
        }
        items.push(effort_row.into_any_element());
        // Cross review and rework: another cell, or none.
        let others: Vec<(String, String)> = table
            .cells
            .iter()
            .filter(|other| other.name != cell.name)
            .map(|other| (other.name.clone(), other.name.clone()))
            .collect();
        let none = || (String::new(), "None".to_owned());
        for (menu, label, value) in [
            (Menu::Review, "Cross review", draft.cross_review(cell)),
            (Menu::Rework, "Rework moves to", draft.rework(cell)),
        ] {
            let mut choices = vec![none()];
            choices.extend(others.iter().cloned());
            let select = self.dispatch_select(
                menu,
                self.dispatch_shown(value, "None", ui),
                choices,
                value,
                can,
                ui,
                cx,
                move |this, value, cx| {
                    this.draft_dispatch(cx, |_, draft| match menu {
                        Menu::Review => draft.cross_review = Some(value),
                        _ => draft.rework = Some(value),
                    })
                },
            );
            items.push(self.dispatch_row(label, select, ui).into_any_element());
        }
        // How it starts, as saved.
        let mut starts = div().flex().flex_col().gap(px(4.0));
        if cell.choices.is_empty() {
            starts = starts.child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child("Nothing: no agent."),
            );
        }
        for choice in &cell.choices {
            starts = starts.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(ui.px(52.0))
                            .text_size(ui.px(11.5))
                            .text_color(dimmer)
                            .child(choice.agent.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .font_family(MONO)
                            .text_size(ui.px(11.5))
                            .line_height(relative(1.55))
                            .text_color(dim)
                            .child(choice.command.clone()),
                    ),
            );
        }
        let starts_label = if state.pending() {
            "Starts with (as saved)"
        } else {
            "Starts with"
        };
        items.push(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(self.dispatch_label(starts_label, ui))
                .child(starts)
                .into_any_element(),
        );
        if let Some(refusal) = &state.refused {
            items.push(self.dispatch_refusal(table, refusal, ui, cx));
        }
        // Reset to default and Save.
        let pending = state.pending();
        let saving = state.saving;
        items.push(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(self.dispatch_button(
                    "dispatch-reset",
                    "Reset to default",
                    false,
                    can && cell.source != "builtin",
                    ui,
                    cx,
                    |this, cx| this.reset_dispatch(cx),
                ))
                .child(div().flex_1())
                .child(self.dispatch_button(
                    "dispatch-save",
                    if saving { "Saving…" } else { "Save" },
                    true,
                    can && pending,
                    ui,
                    cx,
                    |this, cx| this.save_dispatch(cx),
                ))
                .into_any_element(),
        );
        items
    }

    /// A label and the control under it.
    fn dispatch_row(&self, label: &'static str, control: impl IntoElement, ui: &UiFont) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(5.0))
            .child(self.dispatch_label(label, ui))
            .child(control)
    }

    /// A choice's value as shown in its button: `empty` for none.
    fn dispatch_shown(&self, value: &str, empty: &str, ui: &UiFont) -> Div {
        div()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_size(ui.px(12.5))
            .text_color(if value.is_empty() {
                self.fg(|t| t.agents_dimmer)
            } else {
                self.fg(|t| t.agents_text)
            })
            .child(if value.is_empty() {
                empty.to_owned()
            } else {
                value.to_owned()
            })
    }

    /// A field showing `shown`, with a chevron that opens `menu`: each choice its value and
    /// words, the current one ticked; picking one runs `pick`.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_select(
        &self,
        menu: Menu,
        shown: Div,
        choices: Vec<(String, String)>,
        current: &str,
        enabled: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
        pick: impl Fn(&mut Self, String, &mut Context<Self>) + Clone + 'static,
    ) -> Div {
        let (text, dimmer) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_dimmer));
        let open = self.dispatch.open == Some(menu);
        let lit = self.lifted(0.11);
        let chevron = div()
            .id(ElementId::NamedInteger("dispatch-open".into(), menu as u64))
            .flex_shrink_0()
            .size(ui.px(22.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.0))
            .child(footer_icon::icon(
                Icon::Down,
                dimmer,
                ui.scale(10.0) / footer_icon::SIZE,
            ));
        let chevron = if enabled {
            chevron
                .cursor_pointer()
                .hover(move |style| style.bg(lit))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.dispatch.open = if this.dispatch.open == Some(menu) {
                        None
                    } else {
                        Some(menu)
                    };
                    cx.notify();
                }))
        } else {
            chevron.opacity(0.4)
        };
        let field = div()
            .h(ui.px(30.0))
            .pl(px(10.0))
            .pr(px(4.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded(px(7.0))
            .bg(if open { lit } else { self.field_ground() })
            .child(shown)
            .child(chevron);
        let mut wrap = div().relative().child(field);
        if open && enabled {
            let hover = self.faint(0.07);
            let mut list = div()
                .id(ElementId::NamedInteger("dispatch-menu".into(), menu as u64))
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.dispatch.open = None;
                    cx.notify();
                }))
                .min_w(ui.px(220.0))
                .max_h(ui.px(320.0))
                .overflow_y_scroll()
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .rounded(px(10.0))
                .bg(self.lifted(0.08))
                .border_1()
                .border_color(self.faint(0.1))
                .shadow_lg();
            for (index, (value, words)) in choices.into_iter().enumerate() {
                let on = value == current;
                let pick = pick.clone();
                list = list.child(
                    div()
                        .id(ElementId::NamedInteger(
                            format!("dispatch-choice-{}", menu as u8).into(),
                            index as u64,
                        ))
                        .h(ui.px(28.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .text_size(ui.px(12.5))
                        .text_color(if value.is_empty() { dimmer } else { text })
                        .child(div().flex_1().child(words))
                        .children(on.then(|| {
                            footer_icon::icon(
                                Icon::Check,
                                self.fg(|t| t.agents_accent),
                                ui.scale(12.0) / footer_icon::SIZE,
                            )
                        }))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            pick(this, value.clone(), cx)
                        })),
                );
            }
            wrap = wrap.child(deferred(
                anchored()
                    .snap_to_window_with_margin(px(8.0))
                    .child(div().mt(px(4.0)).child(list)),
            ));
        }
        wrap
    }

    /// Why ranch refused the last save or reset, and what to do about it.
    fn dispatch_refusal(
        &self,
        table: &Table,
        refusal: &Refusal,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let red = self.fg(|t| t.agents_red);
        let said = |words: String| {
            div()
                .text_size(ui.px(12.0))
                .line_height(relative(1.5))
                .text_color(red.opacity(0.85))
                .child(words)
        };
        let mut body = div()
            .px(px(12.0))
            .py(px(10.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .rounded(px(8.0))
            .bg(red.opacity(0.08))
            .border_1()
            .border_color(red.opacity(0.3));
        match refusal.problem() {
            Problem::Conflict => {
                body = body
                    .child(said(
                        "The table was changed elsewhere since paddock read it. Nothing was \
                         saved. Reload to see it as it is now; your changes here stay."
                            .into(),
                    ))
                    .child(div().flex().child(self.dispatch_button(
                        "dispatch-reload",
                        "Reload",
                        false,
                        self.dispatch.reading.is_none(),
                        ui,
                        cx,
                        |this, cx| this.reload_dispatch(cx),
                    )));
            }
            Problem::FormatNotPreserved => {
                body = body
                    .child(said(format!(
                        "Nothing was saved: ranch can't change {} without rewriting how it is \
                         laid out. Edit the file by hand.",
                        shown_path(std::path::Path::new(&table.local.path))
                    )))
                    .child(said(refusal.message.clone()));
            }
            Problem::Refused => {
                body = body.child(said(format!("Nothing was saved. {refusal}")));
            }
        }
        body.into_any_element()
    }

    /// A button: the accent's when `main`, else quiet; faded and inert unless `enabled`.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_button(
        &self,
        id: &'static str,
        label: &'static str,
        main: bool,
        enabled: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: fn(&mut Self, &mut Context<Self>),
    ) -> Stateful<Div> {
        let accent = self.fg(|t| t.agents_accent);
        let button = div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(28.0))
            .px(px(13.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_size(ui.px(12.0))
            .child(label);
        let (button, hover) = if main {
            (
                button
                    .bg(accent.opacity(0.16))
                    .border_1()
                    .border_color(accent.opacity(0.45))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(accent),
                accent.opacity(0.24),
            )
        } else {
            (
                button
                    .bg(self.faint(0.07))
                    .text_color(self.fg(|t| t.agents_text)),
                self.faint(0.12),
            )
        };
        if enabled {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| click(this, cx)))
        } else {
            button.opacity(0.45)
        }
    }
}
