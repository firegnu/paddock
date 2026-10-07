//! The right sidebar's Kanban tab: the focused pane's repository's task files as cards, in five
//! groups one above another, each folding; widened, five columns side by side. Git is read in the
//! background every few seconds while the tab shows (`kanban::read`), the agents come from the
//! left sidebar each frame, and the board is drawn again only when it changed. A hovered card
//! offers its task file, its agent and the agent's changes, nothing that changes the work; the one
//! thing it makes is a new draft task file, from New task (DESIGN §13 P5-29).
use crate::{
    agents::Status,
    card, changes,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kanban::{self, Board, Cache, Card, Column, Read, Seen, Tone},
    kind_icon, menu, popover,
    right_panel::Tip,
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Animation, AnimationExt, AnyElement, Bounds, ClickEvent, Context, Div, Entity, EventEmitter,
    Focusable, Font, FontWeight, Hsla, IntoElement, Pixels, Render, SharedString, Stateful,
    Transformation, Window, canvas, div, percentage, prelude::*, px, svg,
};
use std::{
    cell::Cell,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// How often the repository is read again while the tab shows.
const EVERY: Duration = Duration::from_secs(3);
/// How often a finished read or a due one is looked for, and the ages brought up to date.
const TICK: Duration = Duration::from_millis(200);

const SPIN_TRACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><circle cx="6" cy="6" r="4.5" fill="none" stroke="#000" stroke-width="2"/></svg>"##;
const SPIN_ARC: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5a4.5 4.5 0 0 1 4.5 4.5" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></svg>"##;

/// What the window tells the tab every time it draws.
pub struct Frame {
    /// The focused pane's directory: `None` with no pane in focus, `Some(None)` for a pane with
    /// no directory to read.
    pub cwd: Option<Option<String>>,
    /// The right sidebar is open on this tab: read, and keep reading.
    pub active: bool,
    /// The sidebar's width, in points.
    pub width: f32,
    pub theme: Rc<Theme>,
    pub mono: Font,
    /// The agents as the left sidebar lists them.
    pub agents: Vec<Seen>,
}

/// What the tab asks of the window.
pub enum KanbanEvent {
    /// Groups were folded or opened: save them with the layout.
    Folded(Vec<Column>),
    /// Bring this agent's pane to the front.
    GoTo(String),
    /// Bring this agent's pane to the front and show its changes.
    Changes(String),
    /// Open the New task panel for the main worktree `repo`, under the button at `anchor`.
    NewTask {
        repo: PathBuf,
        anchor: Option<Bounds<Pixels>>,
    },
}

impl EventEmitter<KanbanEvent> for KanbanView {}

struct Pending {
    cwd: String,
    cancel: Arc<AtomicBool>,
    out: Arc<Mutex<Option<Read>>>,
}

/// The theme's colours the tab uses.
#[derive(Clone, Copy)]
struct Colors {
    ground: Hsla,
    text: Hsla,
    bright: Hsla,
    muted: Hsla,
    dim: Hsla,
    faint: Hsla,
    rule: Hsla,
    green: Hsla,
    red: Hsla,
    yellow: Hsla,
    blue: Hsla,
}

impl Colors {
    fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        Colors {
            ground: hsla(theme.terminal().background, 1.0),
            text: fg(|t| t.agents_text),
            bright: fg(|t| t.agents_branch),
            muted: fg(|t| t.agents_dim),
            dim: fg(|t| t.agents_dimmer),
            faint: fg(|t| t.agents_faint),
            rule: fg(|t| t.agents_rule),
            green: fg(|t| t.agents_green),
            red: fg(|t| t.agents_red),
            yellow: fg(|t| t.agents_yellow),
            blue: fg(|t| t.agents_blue),
        }
    }

    /// A column's square in its header.
    fn column(&self, column: Column) -> Hsla {
        match column {
            Column::Queued => self.dim,
            Column::InProgress => self.blue,
            Column::ToReview => self.yellow,
            Column::Merged => self.green,
            Column::Done => self.faint,
        }
    }
}

pub struct KanbanView {
    theme: Rc<Theme>,
    colors: Colors,
    mono: Font,
    cwd: Option<Option<String>>,
    active: bool,
    width: f32,
    agents: Vec<Seen>,
    /// The groups folded.
    folded: Vec<Column>,
    /// DONE lists all its cards, not its last few: until the window closes, not in the layout.
    all_done: bool,
    /// The latest read, for whichever directory it was.
    read: Option<Read>,
    /// What is drawn from it with the agents, kept to tell when it changes.
    board: Option<Board>,
    pending: Option<Pending>,
    /// When the last read finished, or `None` to read at once.
    last: Option<Instant>,
    cache: Arc<Cache>,
    /// The card under the mouse, by task file: a draft may have no id, or another's.
    hovered: Option<String>,
    /// The width from which it is widened, at this interface size.
    wide_at: f32,
    /// Where New task was last drawn, for its panel to hang from.
    new_task_at: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl KanbanView {
    pub fn new(theme: Rc<Theme>, mono: Font, folded: Vec<Column>, cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |view, cx| view.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        KanbanView {
            colors: Colors::of(&theme),
            theme,
            mono,
            cwd: None,
            active: false,
            width: 0.0,
            agents: Vec::new(),
            folded,
            all_done: false,
            read: None,
            board: None,
            pending: None,
            last: None,
            cache: Arc::new(kanban::cache()),
            hovered: None,
            wide_at: changes::WIDE,
            new_task_at: Rc::default(),
        }
    }

    /// The window's latest: redraws only when something here changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        if frame.cwd != self.cwd {
            self.cancel();
            self.last = None;
            self.cwd = frame.cwd;
            changed = true;
        }
        if frame.active != self.active {
            self.active = frame.active;
            if self.active {
                self.last = None;
            } else {
                self.cancel();
            }
        }
        if frame.width != self.width {
            self.width = frame.width;
            changed = true;
        }
        if !Rc::ptr_eq(&frame.theme, &self.theme) {
            self.colors = Colors::of(&frame.theme);
            self.theme = frame.theme;
            changed = true;
        }
        if frame.mono != self.mono {
            self.mono = frame.mono;
            changed = true;
        }
        let wide_at = UiFont::get(cx).scale(changes::WIDE).max(changes::WIDE);
        if wide_at != self.wide_at {
            self.wide_at = wide_at;
            changed = true;
        }
        if frame.agents != self.agents {
            self.agents = frame.agents;
            changed |= self.rebuild();
        }
        if changed {
            cx.notify();
        }
    }

    fn cancel(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Takes a finished read, starts one when due, and brings the ages up to date.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = &self.pending {
            let done = pending.out.lock().ok().and_then(|mut out| out.take());
            if let Some(read) = done {
                let cwd = pending.cwd.clone();
                self.pending = None;
                self.last = Some(Instant::now());
                // A failed round keeps what is shown, unless there is nothing yet.
                let current = self.cwd.as_ref().and_then(Option::as_ref) == Some(&cwd);
                if current && (read != Read::Failed || self.read.is_none()) && self.accept(read) {
                    cx.notify();
                }
            }
        }
        if !self.active {
            return;
        }
        if self.rebuild() {
            cx.notify();
        }
        if self.pending.is_some() {
            return;
        }
        let Some(Some(cwd)) = self.cwd.clone() else {
            return;
        };
        if self.last.is_some_and(|last| last.elapsed() < EVERY) {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let out = Arc::new(Mutex::new(None));
        let (stop, put, cache, dir) =
            (cancel.clone(), out.clone(), self.cache.clone(), cwd.clone());
        let started = thread::Builder::new()
            .name("paddock-kanban".into())
            .spawn(move || {
                let read = kanban::read("git", &dir, &cache, &stop);
                if let Ok(mut out) = put.lock() {
                    *out = Some(read);
                }
            });
        if started.is_ok() {
            self.pending = Some(Pending { cwd, cancel, out });
        }
    }

    /// A read came back: whether what is drawn changed.
    fn accept(&mut self, read: Read) -> bool {
        if self.read.as_ref() == Some(&read) {
            return false;
        }
        self.read = Some(read);
        self.rebuild();
        true
    }

    /// The board again from the latest read and agents: whether it changed.
    fn rebuild(&mut self) -> bool {
        let board = match &self.read {
            Some(Read::Board(facts)) => Some(kanban::board(facts, &self.agents, now())),
            _ => None,
        };
        if board == self.board {
            return false;
        }
        self.board = board;
        true
    }

    fn toggle(&mut self, column: Column, cx: &mut Context<Self>) {
        if let Some(at) = self.folded.iter().position(|c| *c == column) {
            self.folded.remove(at);
        } else {
            self.folded.push(column);
            self.folded.sort();
        }
        cx.emit(KanbanEvent::Folded(self.folded.clone()));
        cx.notify();
    }

    fn hover(&mut self, file: &str, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(file.to_owned())
        } else if self.hovered.as_deref() == Some(file) {
            None
        } else {
            return;
        };
        if now != self.hovered {
            self.hovered = now;
            cx.notify();
        }
    }

    fn new_task(&mut self, cx: &mut Context<Self>) {
        let repo = match (&self.board, &self.read) {
            (Some(board), _) => board.repo.clone(),
            (_, Some(Read::NoTasks { repo, .. })) => repo.clone(),
            _ => return,
        };
        cx.emit(KanbanEvent::NewTask {
            repo,
            anchor: self.new_task_at.get(),
        });
    }
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

impl Render for KanbanView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(c.text)
            .text_size(ui.px(13.0));
        let empty = |title: &str, sub: &str| root_empty(title, sub, &c, &ui);
        let Some(cwd) = &self.cwd else {
            return root.child(empty(
                "No pane in focus",
                "Kanban follows the focused pane's repository",
            ));
        };
        let message = match (cwd, &self.read) {
            (None, _) | (_, Some(Read::NotRepository)) => Some((
                "Not in a repository",
                "Kanban shows the task files of the focused pane's repository",
            )),
            (_, Some(Read::NoMain { .. })) => {
                Some(("No main branch", "Task files are read from main"))
            }
            (_, Some(Read::NoTasks { .. })) => {
                let sub = "Task files go in docs/任务 on main. New task writes a draft to \
                           docs/任务 in the main worktree, making the folder if it's missing.";
                return root.child(empty("No task files", sub).child(self.first_task(&ui, cx)));
            }
            (_, Some(Read::Failed)) => Some(("Git couldn't be read", "Trying again shortly")),
            _ => None,
        };
        if let Some((title, sub)) = message {
            return root.child(empty(title, sub));
        }
        let Some(board) = self.board.clone() else {
            // Still reading for the first time.
            return root;
        };
        let wide = self.width >= self.wide_at;
        root.child(self.top(&board, &ui, cx))
            .child(div().flex_shrink_0().h(px(1.0)).bg(c.rule))
            .child(if wide {
                self.columns(&board, &ui, cx).into_any_element()
            } else {
                self.list(&board, &ui, cx).into_any_element()
            })
    }
}

impl KanbanView {
    /// The repository, main, how many need the user, are in progress and to review, and New task.
    fn top(&self, board: &Board, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let need_you = board.need_you();
        let mut summary = Vec::new();
        for (column, words, color) in [
            (Column::InProgress, "in progress", c.blue),
            (Column::ToReview, "to review", c.yellow),
        ] {
            let n = board.cards(column).len();
            if n > 0 {
                summary.push((format!("{n} {words}"), color));
            }
        }
        let mut right = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .text_size(ui.px(12.0));
        if need_you > 0 {
            right = right.child(self.needs_you(format!("{need_you} need you"), ui));
        } else if summary.is_empty() {
            right = right.text_color(c.dim).child("Nothing in progress");
        }
        for (n, (words, color)) in summary.into_iter().enumerate() {
            if n > 0 || need_you > 0 {
                right = right.child(div().text_color(c.faint).child("·"));
            }
            right = right.child(div().text_color(color).child(words));
        }
        let tip = Tip::new("New task", &self.theme, ui);
        let at = self.new_task_at.clone();
        let new_task = div()
            .id("kanban-new-task")
            .relative()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(22.0))
            // As tall as the line of text beside it.
            .my(ui.px(-3.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .hover(move |style| style.bg(c.text.opacity(0.09)))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(footer_icon::icon(
                Icon::Plus,
                c.muted,
                ui.scale(13.0 / footer_icon::SIZE),
            ))
            .child(
                canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.new_task(cx)));
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .pt(ui.px(2.0))
            .pb(ui.px(10.0))
            .pl(ui.px(16.0))
            .pr(ui.px(14.0))
            .child(footer_icon::icon(
                Icon::Folder,
                c.muted,
                ui.scale(13.0 / footer_icon::SIZE),
            ))
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(board.name.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(12.0))
                    .text_color(c.dim)
                    .child(format!("⎇ {}", kanban::MAIN)),
            )
            .child(div().flex_1())
            .child(right)
            .child(new_task)
    }

    /// New task under the empty state of a repository without task files.
    fn first_task(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let at = self.new_task_at.clone();
        div()
            .id("kanban-first-task")
            .relative()
            .mt(ui.px(4.0))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .h(ui.px(26.0))
            .px(ui.px(12.0))
            .rounded(ui.px(6.0))
            .border_1()
            .border_color(c.rule)
            .text_size(ui.px(12.0))
            .text_color(c.text)
            .cursor_pointer()
            .hover(move |style| style.bg(c.text.opacity(0.09)))
            .child(footer_icon::icon(
                Icon::Plus,
                c.muted,
                ui.scale(13.0 / footer_icon::SIZE),
            ))
            .child("New task")
            .child(
                canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.new_task(cx)))
    }

    /// Narrow: the five groups one above another, each folding.
    fn list(&self, board: &Board, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let mut list = div()
            .id("kanban-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pt(ui.px(4.0))
            .px(ui.px(8.0))
            .pb(ui.px(16.0));
        for column in Column::ALL {
            let open = !self.folded.contains(&column);
            // An empty group is one short, faint line.
            let empty = board.cards(column).is_empty();
            let mut head = div()
                .id(("kanban-group", column as usize))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(8.0))
                .h(ui.px(if empty { 22.0 } else { 30.0 }))
                .mt(ui.px(if empty { 2.0 } else { 8.0 }))
                .px(ui.px(8.0))
                .rounded(px(6.0))
                .when(empty, |head| head.opacity(0.5))
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.05)))
                .child(footer_icon::icon(
                    if open { Icon::Down } else { Icon::Forward },
                    c.dim,
                    ui.scale(9.0 / footer_icon::SIZE),
                ))
                .child(self.square(column, ui))
                .child(self.label(column, ui))
                .child(
                    div()
                        .text_size(ui.px(11.0))
                        .text_color(c.dim)
                        .child(board.count(column, self.all_done)),
                )
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.toggle(column, cx)));
            if let Some(note) = self.note(board, column) {
                let tip = Tip::new(note, &self.theme, ui);
                head = head.tooltip(move |_, cx| cx.new(|_| tip.clone()).into());
            }
            list = list.child(head);
            // A folded group still shows the cards that need the user.
            let shown: Vec<&Card> = board
                .listed(column, self.all_done)
                .into_iter()
                .filter(|card| open || card.needs_you)
                .collect();
            if !shown.is_empty() {
                let mut cards = div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap(ui.px(2.0))
                    .pt(ui.px(2.0))
                    .pb(ui.px(4.0));
                for card in shown {
                    cards = cards.child(self.row(card, ui, cx));
                }
                if open && column == Column::Done {
                    cards = cards.children(
                        self.show_all(board, ui, cx)
                            .map(|more| more.h(ui.px(28.0)).pl(ui.px(26.0))),
                    );
                }
                list = list.child(cards);
            }
        }
        list
    }

    /// Widened: five columns side by side, each scrolling on its own.
    fn columns(&self, board: &Board, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let mut row = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .gap(ui.px(10.0))
            .pt(ui.px(10.0))
            .px(ui.px(12.0))
            .pb(ui.px(12.0));
        for column in Column::ALL {
            let mut body = div()
                .id(("kanban-column", column as usize))
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(ui.px(6.0))
                .px(ui.px(6.0))
                .pb(ui.px(8.0));
            let cards = board.listed(column, self.all_done);
            for card in &cards {
                body = body.child(self.tile(card, ui, cx));
            }
            if column == Column::Done {
                body = body.children(
                    self.show_all(board, ui, cx)
                        .map(|more| more.h(ui.px(30.0)).justify_center()),
                );
            }
            if cards.is_empty() {
                body = body.child(
                    div()
                        .py(ui.px(14.0))
                        .px(ui.px(6.0))
                        .text_center()
                        .text_size(ui.px(12.0))
                        .text_color(c.dim.opacity(0.8))
                        .child("Nothing here"),
                );
            }
            row = row.child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .rounded(px(10.0))
                    .bg(c.faint.opacity(0.12))
                    .border_1()
                    .border_color(c.rule.opacity(0.6))
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap(ui.px(7.0))
                            .h(ui.px(34.0))
                            .px(ui.px(10.0))
                            .min_w(px(0.0))
                            .child(self.square(column, ui))
                            .child(self.label(column, ui))
                            .child(
                                div()
                                    .text_size(ui.px(11.0))
                                    .text_color(c.dim)
                                    .child(board.count(column, self.all_done)),
                            ),
                    )
                    .child(body),
            );
        }
        row
    }

    /// What a group's header says when the mouse is on it: DONE's only while it lists its last
    /// few of more.
    fn note(&self, board: &Board, column: Column) -> Option<&'static str> {
        let cut = board.listed(column, self.all_done).len() < board.cards(column).len();
        Some(column.note()).filter(|note| !note.is_empty() && (column != Column::Done || cut))
    }

    /// Under DONE's cards when it has more than its last few: Show all, or while all show, Show
    /// fewer. Kept only until the window closes.
    fn show_all(
        &self,
        board: &Board,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let c = self.colors;
        let total = board.cards(Column::Done).len();
        if board.listed(Column::Done, false).len() == total {
            return None;
        }
        Some(
            div()
                .id("kanban-show-all")
                .flex_shrink_0()
                .flex()
                .items_center()
                .rounded(px(6.0))
                .text_size(ui.px(12.0))
                .text_color(c.muted)
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.05)).text_color(c.text))
                .child(if self.all_done {
                    "Show fewer".to_owned()
                } else {
                    format!("Show all {total}")
                })
                .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                    view.all_done = !view.all_done;
                    cx.notify();
                })),
        )
    }

    /// On a card to review, under its agent, the repository's controller as the agent is drawn:
    /// its kind and status, its name and status, and that it reviews. A click brings its pane to
    /// the front, as a click on its card in the sidebar does.
    fn reviewer(
        &self,
        card: &Card,
        ui: &UiFont,
        size: f32,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let c = self.colors;
        let agent = card.controller.as_ref()?;
        let look = card::look(agent.status);
        let short = agent
            .name
            .split_once('/')
            .map_or(agent.name.as_str(), |(_, s)| s);
        let name = agent.name.clone();
        Some(
            div()
                .id(SharedString::from(format!("kanban-reviewer-{}", card.file)))
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .min_w(px(0.0))
                .mx(ui.px(-4.0))
                .px(ui.px(4.0))
                .rounded(px(5.0))
                .whitespace_nowrap()
                .text_color(c.muted)
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.06)))
                .child(self.avatar(agent, ui, size))
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(c.bright)
                        .child(short.to_owned()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(hsla(self.theme.fg(look.color), 1.0))
                        .child(look.label),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(10.5))
                        .text_color(c.dim)
                        .child("reviewer"),
                )
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    cx.emit(KanbanEvent::GoTo(name.clone()));
                })),
        )
    }

    fn square(&self, column: Column, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .size(ui.px(7.0))
            .rounded(px(2.0))
            .bg(self.colors.column(column))
    }

    fn label(&self, column: Column, ui: &UiFont) -> Div {
        div()
            .flex_shrink(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(ui.px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.colors.muted)
            .child(column.label())
    }

    /// A card in the narrow list: its id, title, marks and age; then its agent, state, branch
    /// and lines, the controller when it is to review, and what the user is waited on for; DONE
    /// faded, on one line.
    fn row(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let done = card.column == Column::Done;
        let first = self
            .first(card, ui)
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .children(self.id(card, ui))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(if done {
                        FontWeight::NORMAL
                    } else {
                        FontWeight::MEDIUM
                    })
                    .child(card.title.clone()),
            )
            .children(self.marks(card, ui))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(11.5))
                    .text_color(c.dim)
                    .child(card.age.clone()),
            );
        let meta = (!done)
            .then(|| self.meta(card, ui, 18.0, true))
            .flatten()
            .map(|meta| meta.mt(ui.px(5.0)).text_size(ui.px(12.0)));
        let reviewer = self
            .reviewer(card, ui, 18.0, cx)
            .map(|reviewer| reviewer.mt(ui.px(5.0)).text_size(ui.px(12.0)));
        let asks = self
            .asks(card)
            .map(|asks| asks.mt(ui.px(5.0)).text_size(ui.px(12.0)));
        self.hoverable(card, cx)
            .rounded(px(8.0))
            .pt(ui.px(8.0))
            .pb(ui.px(8.0))
            .pl(ui.px(26.0))
            .pr(ui.px(10.0))
            .opacity(fade(card))
            .hover(move |style| style.bg(c.text.opacity(0.05)))
            .child(first)
            .children(meta)
            .children(reviewer)
            .children(asks)
            .children(self.actions(card, ui, cx))
    }

    /// A card in a widened column: its id and age, the title on up to two lines, its marks, then
    /// its agent, state and lines, the controller when it is to review, and what the user is waited
    /// on for.
    fn tile(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let done = card.column == Column::Done;
        let first = self
            .first(card, ui)
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .children(self.id(card, ui))
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(11.0))
                    .text_color(c.dim)
                    .child(card.age.clone()),
            );
        let title = div()
            .mt(ui.px(4.0))
            .line_clamp(2)
            .text_ellipsis()
            .font_weight(if done {
                FontWeight::NORMAL
            } else {
                FontWeight::MEDIUM
            })
            .child(card.title.clone());
        let marks = self.marks(card, ui);
        let marks = (!marks.is_empty()).then(|| {
            div()
                .mt(ui.px(6.0))
                .flex()
                .flex_wrap()
                .gap(ui.px(4.0))
                .children(marks)
        });
        let meta = (!done)
            .then(|| self.meta(card, ui, 16.0, false))
            .flatten()
            .map(|meta| meta.mt(ui.px(7.0)).text_size(ui.px(11.5)));
        let reviewer = self
            .reviewer(card, ui, 16.0, cx)
            .map(|reviewer| reviewer.mt(ui.px(6.0)).text_size(ui.px(11.5)));
        let asks = self
            .asks(card)
            .map(|asks| asks.mt(ui.px(6.0)).text_size(ui.px(11.5)));
        self.hoverable(card, cx)
            .flex_shrink_0()
            .rounded(px(8.0))
            .p(ui.px(9.0))
            .px(ui.px(10.0))
            .bg(c.faint.opacity(0.22))
            .border_1()
            .border_color(c.rule)
            .opacity(fade(card))
            .hover(move |style| style.bg(c.faint.opacity(0.32)))
            .child(first)
            .child(title)
            .children(marks)
            .children(meta)
            .children(reviewer)
            .children(asks)
            .children(self.actions(card, ui, cx))
    }

    /// A card's ground that knows when the mouse is on it.
    fn hoverable(&self, card: &Card, cx: &mut Context<Self>) -> Stateful<Div> {
        let file = card.file.clone();
        div()
            .id(SharedString::from(format!("kanban-card-{}", card.file)))
            .relative()
            .flex_shrink_0()
            .on_hover(
                cx.listener(move |view, hovered: &bool, _, cx| view.hover(&file, *hovered, cx)),
            )
    }

    /// A card's first line, which tells why when the mouse is on a dropped one.
    fn first(&self, card: &Card, ui: &UiFont) -> Stateful<Div> {
        let first = div().id(SharedString::from(format!("kanban-first-{}", card.file)));
        let Some(why) = &card.dropped else {
            return first;
        };
        let note = Note::new(
            if why.is_empty() {
                "Dropped".into()
            } else {
                format!("Dropped: {why}").into()
            },
            &self.theme,
            ui,
        );
        first.tooltip(move |_, cx| cx.new(|_| note.clone()).into())
    }

    /// The id, for a card that has one.
    fn id(&self, card: &Card, ui: &UiFont) -> Option<Div> {
        (!card.id.is_empty()).then(|| {
            div()
                .flex_shrink_0()
                .font(self.mono.clone())
                .text_size(ui.px(11.0))
                .text_color(self.colors.muted)
                .child(card.id.clone())
        })
    }

    /// The words that mark a card: DRAFT, Dropped, Needs you.
    fn marks(&self, card: &Card, ui: &UiFont) -> Vec<Div> {
        let c = self.colors;
        let quiet = |words: &'static str| {
            div()
                .flex_shrink_0()
                .px(ui.px(5.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(c.rule)
                .text_size(ui.px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.dim)
                .child(words)
        };
        let mut marks = Vec::new();
        if card.draft {
            marks.push(quiet("DRAFT"));
        }
        if card.dropped.is_some() {
            marks.push(quiet("Dropped"));
        }
        if card.needs_you {
            marks.push(
                self.needs_you("Needs you".into(), ui)
                    .text_size(ui.px(10.5)),
            );
        }
        marks
    }

    /// The mark of what needs the user, in the colour of an agent waiting on them.
    fn needs_you(&self, words: String, ui: &UiFont) -> Div {
        let c = self.colors;
        div()
            .flex_shrink_0()
            .px(ui.px(6.0))
            .rounded(px(4.0))
            .bg(c.yellow.opacity(0.16))
            .border_1()
            .border_color(c.yellow.opacity(0.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c.yellow)
            .child(words)
    }

    /// What the task file says the user is waited on for, under the rest.
    fn asks(&self, card: &Card) -> Option<Div> {
        let asks = card.asks.as_ref().filter(|asks| !asks.is_empty())?;
        Some(
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(self.colors.muted)
                .child(asks.clone()),
        )
    }

    /// The second line: the agent's kind and status, its name (in the list), the state, the
    /// branch (in the list), and the lines added and deleted; `None` with nothing to say.
    fn meta(&self, card: &Card, ui: &UiFont, tile: f32, full: bool) -> Option<Div> {
        let c = self.colors;
        if card.agent.is_none() && card.state.is_none() && card.branch.is_none() {
            return None;
        }
        let mut meta = div()
            .flex()
            .items_center()
            .gap(ui.px(if full { 7.0 } else { 6.0 }))
            .min_w(px(0.0))
            .whitespace_nowrap()
            .text_color(c.muted);
        if let Some(agent) = &card.agent {
            meta = meta.child(self.avatar(agent, ui, tile));
            if full {
                let short = agent
                    .name
                    .split_once('/')
                    .map_or(agent.name.as_str(), |(_, s)| s);
                meta = meta.child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(c.bright)
                        .child(short.to_owned()),
                );
            }
        }
        if let Some((words, tone)) = &card.state {
            let color = match tone {
                Tone::Dim => c.dim,
                Tone::Agent => card.agent.as_ref().map_or(c.muted, |a| {
                    hsla(self.theme.fg(card::look(a.status).color), 1.0)
                }),
                Tone::Review => c.yellow,
                Tone::Merged => c.green,
            };
            meta = meta.child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_color(color)
                    .child(words.clone()),
            );
        }
        if full && let Some(branch) = &card.branch {
            meta = meta
                .child(div().flex_shrink_0().text_color(c.faint).child("·"))
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(format!("⎇ {branch}")),
                );
        }
        meta = meta.child(div().flex_1());
        if let Some(lines) = card.lines {
            let (added, deleted) = kanban::line_words(lines);
            meta = meta.child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap(ui.px(4.0))
                    .children(added.map(|added| div().text_color(c.green).child(added)))
                    .children(deleted.map(|deleted| div().text_color(c.red).child(deleted))),
            );
        }
        Some(meta)
    }

    /// The agent's kind on a square tinted in its colour, its status on the corner: an arc
    /// turning while it works, as on the left sidebar's cards, otherwise a dot in the status's
    /// colour.
    fn avatar(&self, agent: &Seen, ui: &UiFont, size: f32) -> Div {
        let brand = agent.kind.as_deref().map(card::brand);
        let color = hsla(
            self.theme
                .fg(brand.as_ref().map_or(|t| t.agents_dim, |b| b.color)),
            1.0,
        );
        let mark = match agent.kind.as_deref().and_then(kind_icon::of) {
            Some(icon) => icon.render(ui.px(size * 0.6), color).into_any_element(),
            None => div()
                .text_size(ui.px(size * 0.6))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(crate::sidebar::initial(
                    agent
                        .name
                        .split_once('/')
                        .map_or(agent.name.as_str(), |(_, s)| s),
                ))
                .into_any_element(),
        };
        let status = hsla(self.theme.fg(card::look(agent.status).color), 1.0);
        let ground = self.colors.ground;
        let badge: AnyElement = if agent.status == Status::Working {
            let spin = ui.px(size * 0.5);
            div()
                .absolute()
                .right(ui.px(-3.0))
                .bottom(ui.px(-3.0))
                .size(spin)
                .rounded_full()
                .bg(ground)
                .child(
                    svg()
                        .data(SPIN_TRACK)
                        .absolute()
                        .size(spin)
                        .text_color(status.opacity(0.25)),
                )
                .child(
                    svg()
                        .data(SPIN_ARC)
                        .absolute()
                        .size(spin)
                        .text_color(status)
                        .with_animation(
                            "spin",
                            Animation::new(Duration::from_millis(1100))
                                .repeat_synced()
                                .with_max_fps(30.0),
                            |arc, delta| {
                                arc.with_transformation(Transformation::rotate(percentage(delta)))
                            },
                        ),
                )
                .into_any_element()
        } else {
            let dot = size * 0.38;
            div()
                .absolute()
                .right(ui.px(-2.0 - 2.0))
                .bottom(ui.px(-2.0 - 2.0))
                .size(ui.px(dot + 4.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(ground)
                .child(div().size(ui.px(dot)).rounded_full().bg(status))
                .into_any_element()
        };
        div()
            .relative()
            .flex_shrink_0()
            .size(ui.px(size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(size * 0.28))
            .bg(color.opacity(0.14))
            .child(mark)
            .child(badge)
    }

    /// While the mouse is on the card, its ways out at the top right: the task file, and with an
    /// agent, its pane and its changes. Nothing that changes the work.
    fn actions(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        if self.hovered.as_deref() != Some(card.file.as_str()) {
            return None;
        }
        let c = self.colors;
        let theme = self.theme.clone();
        let button = |id: &'static str, icon: Icon, words: &'static str| {
            let tip = Tip::new(words, &theme, ui);
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(24.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.09)))
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .child(footer_icon::icon(
                    icon,
                    c.muted,
                    ui.scale(13.0 / footer_icon::SIZE),
                ))
        };
        let path = self
            .board
            .as_ref()
            .map(|b| b.repo.join(kanban::TASKS).join(&card.file));
        let mut bar = div()
            .absolute()
            .top(ui.px(6.0))
            .right(ui.px(8.0))
            .flex()
            .gap(ui.px(2.0))
            .p(ui.px(2.0))
            .rounded(px(7.0))
            .bg(c.ground)
            .border_1()
            .border_color(c.rule)
            .child(
                button("kanban-open", Icon::Document, "Open task file").on_click(
                    move |_, _, cx| {
                        cx.stop_propagation();
                        if let Some(path) = &path {
                            cx.open_with_system(path);
                        }
                    },
                ),
            );
        if let Some(agent) = &card.agent {
            let (go, show) = (agent.name.clone(), agent.name.clone());
            bar = bar
                .child(
                    button("kanban-agent", Icon::NewShell, "Go to agent").on_click(cx.listener(
                        move |_, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            cx.emit(KanbanEvent::GoTo(go.clone()));
                        },
                    )),
                )
                .child(
                    button("kanban-changes", Icon::Changes, "Show changes").on_click(cx.listener(
                        move |_, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            cx.emit(KanbanEvent::Changes(show.clone()));
                        },
                    )),
                );
        }
        Some(bar)
    }
}

/// The New task panel's width, in points at the base interface size.
pub const NEW_TASK_WIDTH: f32 = 320.0;

/// The New task panel: an id, offered as the next free one, and a title. Create or ↩ writes the
/// draft with [`kanban::create_draft`], and says why when it cannot without closing; Cancel or
/// Esc closes it. ↑↓ and Tab move between the fields.
pub struct NewTask {
    repo: PathBuf,
    theme: Rc<Theme>,
    id: Entity<TextInput>,
    title: Entity<TextInput>,
    /// Why the last Create was refused, until something is typed.
    refused: Option<String>,
    creating: bool,
}

pub enum NewTaskEvent {
    /// The draft was written here.
    Created(PathBuf),
    Cancel,
}

impl EventEmitter<NewTaskEvent> for NewTask {}

impl NewTask {
    /// For the main worktree `repo`, the title field taking the keys.
    pub fn new(
        repo: PathBuf,
        theme: Rc<Theme>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let colors = text_input::Colors {
            text: fg(|t| t.agents_text),
            placeholder: fg(|t| t.agents_dimmer),
            cursor: fg(|t| t.focus),
            selection: hsla(theme.fg(|t| t.focus), 0.3),
        };
        let id = cx.new(|cx| TextInput::new("", "ID", colors, cx));
        let title = cx.new(|cx| TextInput::new("", "Short title", colors, cx));
        for input in [&id, &title] {
            cx.subscribe(input, |this, _, _: &Changed, cx| {
                if this.refused.take().is_some() {
                    cx.notify();
                }
            })
            .detach();
        }
        window.focus(&title.focus_handle(cx), cx);
        // The next free id, from main, the worktree and every branch; unless one is typed first.
        let dir = repo.clone();
        let taken =
            cx.background_spawn(async move { kanban::taken("git", &dir, &AtomicBool::new(false)) });
        cx.spawn(async move |this, cx| {
            let next = taken
                .await
                .and_then(|ids| kanban::next_id(ids.keys().map(String::as_str)));
            let _ = this.update(cx, |this, cx| {
                if let Some(next) = next
                    && this.id.read(cx).text().is_empty()
                {
                    this.id.update(cx, |input, cx| input.set_text(next, cx));
                }
            });
        })
        .detach();
        NewTask {
            repo,
            theme,
            id,
            title,
            refused: None,
            creating: false,
        }
    }

    fn create(&mut self, cx: &mut Context<Self>) {
        if self.creating {
            return;
        }
        self.creating = true;
        let repo = self.repo.clone();
        let id = self.id.read(cx).text().to_owned();
        let title = self.title.read(cx).text().to_owned();
        let task = cx.background_spawn(async move {
            kanban::create_draft("git", &repo, &id, &title, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.creating = false;
                match result {
                    Ok(path) => cx.emit(NewTaskEvent::Created(path)),
                    Err(refused) => this.refused = Some(refused.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The other field takes the keys.
    fn switch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let to = if self.id.focus_handle(cx).is_focused(window) {
            &self.title
        } else {
            &self.id
        };
        window.focus(&to.focus_handle(cx), cx);
        cx.notify();
    }
}

impl Render for NewTask {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let (text, dim, rule, red, accent) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_rule),
            fg(|t| t.agents_red),
            fg(|t| t.agents_accent),
        );
        let sunken = hsla(theme.bg(|t| t.agents_bg), 1.0);
        let row = |label: &'static str, input: &Entity<TextInput>| {
            let focused = input.focus_handle(cx).is_focused(window);
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(10.0))
                .px(ui.px(9.0))
                .child(
                    div()
                        .flex_shrink_0()
                        .w(ui.px(38.0))
                        .text_color(dim)
                        .child(label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(ui.px(28.0))
                        .px(ui.px(8.0))
                        .flex()
                        .items_center()
                        .rounded(ui.px(6.0))
                        .bg(sunken)
                        .border_1()
                        .border_color(if focused { accent.opacity(0.6) } else { rule })
                        .overflow_hidden()
                        .child(input.clone()),
                )
        };
        let (id, title) = (row("ID", &self.id), row("Title", &self.title));
        let note = match &self.refused {
            Some(why) => div().text_color(red).child(why.clone()),
            None => div()
                .text_color(dim)
                .child("Opens the draft in your editor to add details."),
        };
        let cancel = div()
            .id("new-task-cancel")
            .px(ui.px(12.0))
            .h(ui.px(26.0))
            .flex()
            .items_center()
            .rounded(ui.px(6.0))
            .border_1()
            .border_color(rule)
            .text_color(text)
            .cursor_pointer()
            .hover(move |style| style.bg(rule.opacity(0.5)))
            .child("Cancel")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(NewTaskEvent::Cancel)));
        let create = div()
            .id("new-task-create")
            .px(ui.px(12.0))
            .h(ui.px(26.0))
            .flex()
            .items_center()
            .rounded(ui.px(6.0))
            .bg(accent)
            .text_color(sunken)
            .font_weight(FontWeight::SEMIBOLD)
            .child("Create");
        let create = if self.creating {
            create.opacity(0.5)
        } else {
            create
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.9)))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.create(cx)))
        };
        popover::panel(theme, &ui)
            .id("new-task")
            .w_full()
            // Tab moves between the fields as it moves in the palette's list.
            .key_context(menu::PALETTE)
            .on_action(
                cx.listener(|this, _: &menu::SelectNext, window, cx| this.switch(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::SelectPrevious, window, cx| this.switch(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::OpenSelected, _, cx| this.create(cx)))
            .child(popover::heading(theme, &ui, "NEW TASK"))
            .child(id.mt(ui.px(4.0)))
            .child(title.mt(ui.px(8.0)))
            .child(
                note.flex_shrink_0()
                    .mt(ui.px(10.0))
                    .px(ui.px(9.0))
                    .whitespace_normal()
                    .text_size(ui.px(12.0)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .justify_end()
                    .gap(ui.px(8.0))
                    .mt(ui.px(12.0))
                    .px(ui.px(4.0))
                    .pb(ui.px(4.0))
                    .child(cancel)
                    .child(create),
            )
    }
}

/// How faded a card is: a draft most, DONE less, one that needs the user not at all.
fn fade(card: &Card) -> f32 {
    if card.needs_you {
        1.0
    } else if card.draft {
        0.55
    } else if card.column == Column::Done {
        0.62
    } else {
        1.0
    }
}

/// A hover text whose words are not fixed, drawn as the right sidebar's [`Tip`].
#[derive(Clone)]
struct Note {
    text: SharedString,
    size: Pixels,
    color: Hsla,
    background: Hsla,
    border: Hsla,
}

impl Note {
    fn new(text: SharedString, theme: &Theme, ui: &UiFont) -> Self {
        Note {
            text,
            size: ui.px(11.5),
            color: hsla(theme.fg(|t| t.agents_text), 1.0),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: hsla(theme.fg(|t| t.agents_rule), 1.0),
        }
    }
}

impl Render for Note {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .max_w(self.size * 32.0)
            .px(px(7.0))
            .py(px(3.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(self.border)
            .bg(self.background)
            .shadow_md()
            .text_size(self.size)
            .text_color(self.color)
            .child(self.text.clone())
            .child(crate::browser::cover())
    }
}

/// A tab with nothing to show: a faint icon, a line, and a quieter line under it.
fn root_empty(title: &str, sub: &str, c: &Colors, ui: &UiFont) -> Div {
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(ui.px(10.0))
        .px(ui.px(40.0))
        .pb(ui.px(40.0))
        .child(footer_icon::icon(
            Icon::Kanban,
            c.faint.blend(c.dim.opacity(0.5)),
            ui.scale(26.0 / footer_icon::SIZE),
        ))
        .child(
            div()
                .text_size(ui.px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.bright)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_size(ui.px(12.0))
                .text_color(c.dim)
                .text_center()
                .child(sub.to_owned()),
        )
}
