//! The right sidebar's Kanban tab: the focused pane's repository's task files as cards, in five
//! groups one above another, each folding; widened, five columns side by side. Git is read in the
//! background every few seconds while the tab shows (`kanban::read`), the agents come from the
//! left sidebar each frame, and the board is drawn again only when it changed. A hovered card
//! offers its task file, its agent and the agent's changes, nothing that changes the work; the
//! things it writes are a task file, new or queued, from the task dialog (New task, or Edit on a
//! queued card), and, on a card its task file says needs the user, Clear: that line taken out and
//! committed once confirmed; on a queued card, its priority line, committed the same way or, in a
//! draft, only written (DESIGN §13 P5-29, P5-60b).
use crate::{
    activity,
    agents::Status,
    card, changes,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kanban::{self, Board, Cache, Card, Column, Doer, Priority, Read, Seen, Tone},
    kind_icon, markdown, menu, popover,
    right_panel::Tip,
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, BoxShadow, ClickEvent, Context, Div, Entity,
    EventEmitter, FocusHandle, Focusable, Font, FontStyle, FontWeight, HighlightStyle, Hsla,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point, Render, ScrollStrategy,
    SharedString, Stateful, StyledText, Subscription, TextRun, Transformation, UnderlineStyle,
    UniformListScrollHandle, Window, WindowTextSystem, anchored, deferred, div, percentage, point,
    prelude::*, px, relative, svg, uniform_list,
};
use std::{
    collections::{HashMap, HashSet},
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
/// The buttons on a card while the mouse is on it, in a tray at its top right: each one's size, in
/// the narrow list and in a widened column, and the room between them; the tray's padding, and how
/// far down from the card's top and in from its right edge it sits, on a card and on a DONE line.
const BAR_BUTTON: f32 = 22.0;
const TILE_BUTTON: f32 = 20.0;
const BAR_GAP: f32 = 2.0;
const BAR_PAD: f32 = 2.0;
const BAR_TOP: f32 = 5.0;
const DONE_BAR_TOP: f32 = 3.0;
const BAR_RIGHT: f32 = 8.0;
/// The cards (P5-71): the narrow list's side padding; a widened board's side padding, the room
/// between its columns and inside each; a card's padding (more on the left, for the priority
/// stripe), the room between two cards, and its corner.
const LIST_X: f32 = 12.0;
const COLUMNS_X: f32 = 12.0;
const COLUMN_GAP: f32 = 10.0;
const COLUMN_X: f32 = 6.0;
const CARD_LEFT: f32 = 14.0;
const CARD_RIGHT: f32 = 12.0;
const CARD_Y: f32 = 10.0;
const CARD_GAP: f32 = 6.0;
const CARD_RADIUS: f32 = 10.0;
/// Room kept from the right sidebar's own edge when reckoning the width a text is cut to.
const SPARE: f32 = 2.0;
/// A card's face, edge and the light inside its top edge, in the text colour, resting and with the
/// mouse on it, and the dark inside its bottom edge, as the left sidebar's cards over the material
/// (P5-51b); a draft's face and dashed edge; one that needs the user, in amber.
const FACE: f32 = 0.055;
const FACE_HOVERED: f32 = 0.085;
const EDGE: f32 = 0.07;
const EDGE_HOVERED: f32 = 0.11;
const LIGHT: f32 = 0.07;
const LIGHT_HOVERED: f32 = 0.09;
const BOTTOM: f32 = 0.25;
const DRAFT_FACE: f32 = 0.03;
const DRAFT_FACE_HOVERED: f32 = 0.05;
const DRAFT_EDGE: f32 = 0.12;
const DRAFT_EDGE_HOVERED: f32 = 0.18;
const WAITING_FACE: f32 = 0.05;
const WAITING_FACE_HOVERED: f32 = 0.08;
const WAITING_EDGE: f32 = 0.34;
/// The ground of the small tags and pills, and the rule over a card's agent line, in the text
/// colour.
const SOFT: f32 = 0.06;
const RULE: f32 = 0.06;
/// Text sizes: the id, an age, a DONE line's title, the agent line in the list and in a column,
/// the branch, and the controller's `reviewer`; a group's folding arrow.
const ID_SIZE: f32 = 11.0;
const AGE_SIZE: f32 = 11.0;
const DONE_SIZE: f32 = 12.5;
const FOOT_SIZE: f32 = 12.0;
const TILE_FOOT_SIZE: f32 = 11.5;
const BRANCH_SIZE: f32 = 11.0;
const REVIEWER_SIZE: f32 = 10.5;
const ARROW: f32 = 9.0;
/// The share of the room a branch keeps, cut, before the name beside it gives way.
const BRANCH_SHARE: f32 = 0.4;

const SPIN_TRACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><circle cx="6" cy="6" r="4.5" fill="none" stroke="#000" stroke-width="2"/></svg>"##;
const SPIN_ARC: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5a4.5 4.5 0 0 1 4.5 4.5" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></svg>"##;
/// Line drawings for a card's Edit and the task dialog: a pencil, a padlock (the id of a task being
/// edited), angle brackets (the file's head).
const PENCIL: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M7.5 2.5l2 2L4 10H2V8z" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const LOCK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><rect x="2.5" y="5.5" width="7" height="5" rx="1" fill="none" stroke="#000" stroke-width="1.3"/><path d="M4 5.5V4a2 2 0 0 1 4 0v1.5" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/></svg>"##;
const BRACKETS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><path d="M5 3.5L1.5 7 5 10.5M9 3.5L12.5 7 9 10.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// A link, on the mark of what a queued card waits for; chevrons on Show all and Show fewer.
const LINK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M4 6l2-2M3.2 4.6l-1 1a1.6 1.6 0 002.2 2.2l1-1M6.8 5.4l1-1a1.6 1.6 0 00-2.2-2.2l-1 1" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/></svg>"##;
const CHEVRON_DOWN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 3.8L5 6.3l2.5-2.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const CHEVRON_UP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 6.2L5 3.7l2.5 2.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

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
    /// Open the task dialog for a new task in the main worktree `repo`.
    NewTask { repo: PathBuf },
    /// Open the task dialog on the task file `file` in the main worktree `repo`'s `docs/任务/`, a
    /// draft or on main.
    EditTask {
        repo: PathBuf,
        file: String,
        draft: bool,
    },
}

impl EventEmitter<KanbanEvent> for KanbanView {}

struct Pending {
    cwd: String,
    cancel: Arc<AtomicBool>,
    out: Arc<Mutex<Option<Read>>>,
}

/// Where a card's Clear, or a change of its priority, stands.
#[derive(Clone, Debug, PartialEq)]
enum Clearing {
    /// Asking to confirm, or which priority, while the mouse stays on the card.
    Asking,
    Running,
    /// Committed, or a draft written: until the board is read again.
    Done,
    /// Nothing was changed, and why; until the mouse leaves the card.
    Refused(String),
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
    /// The cards being cleared of `待用户：` or asked to be, by task file.
    clears: HashMap<String, Clearing>,
    /// The cards whose priority is being changed or asked for, by task file.
    priorities: HashMap<String, Clearing>,
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
            clears: HashMap::new(),
            priorities: HashMap::new(),
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
                // Begun after the commit, so the board it gives no longer has the line.
                let before = self.clears.len() + self.priorities.len();
                for acts in [&mut self.clears, &mut self.priorities] {
                    acts.retain(|_, clearing| *clearing != Clearing::Done);
                }
                if self.clears.len() + self.priorities.len() != before {
                    cx.notify();
                }
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
        // Leaving takes back the question, or why it was not cleared or changed.
        if !hovered {
            for acts in [&mut self.clears, &mut self.priorities] {
                if take_back(acts, file) {
                    cx.notify();
                }
            }
        }
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

    /// Clear on a card: ask first, unless it is being cleared already.
    fn ask_clear(&mut self, file: &str, cx: &mut Context<Self>) {
        if matches!(
            self.clears.get(file),
            Some(Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        take_back(&mut self.priorities, file);
        self.clears.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// The priority button on a card: ask which, unless it is being changed already.
    fn ask_priority(&mut self, file: &str, cx: &mut Context<Self>) {
        if matches!(
            self.priorities.get(file),
            Some(Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        take_back(&mut self.clears, file);
        self.priorities.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// A priority picked: its line written in the background, committed for a card on main, and
    /// the board read again once it is. The one it has already just closes the question.
    fn set_priority(&mut self, card: &Card, priority: Priority, cx: &mut Context<Self>) {
        let file = card.file.clone();
        if self.priorities.get(&file) != Some(&Clearing::Asking) {
            return;
        }
        if card.priority == Some(priority) {
            self.priorities.remove(&file);
            cx.notify();
            return;
        }
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        self.priorities.insert(file.clone(), Clearing::Running);
        let (name, draft) = (file.clone(), card.draft);
        let task = cx.background_spawn(async move {
            if draft {
                kanban::set_draft_priority(&repo, &name, priority)
            } else {
                kanban::set_priority("git", &repo, &name, priority, &AtomicBool::new(false))
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.priorities.insert(file, Clearing::Done);
                        // A read begun before the change would bring the old one back a while.
                        view.cancel();
                        view.last = None;
                    }
                    Err(why) => {
                        view.priorities
                            .insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Confirmed: the line taken out and committed in the background, and the board read again
    /// once it is.
    fn clear(&mut self, file: &str, asks: &str, cx: &mut Context<Self>) {
        if self.clears.get(file) != Some(&Clearing::Asking) {
            return;
        }
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        self.clears.insert(file.to_owned(), Clearing::Running);
        let (name, asks) = (file.to_owned(), asks.to_owned());
        let task = cx.background_spawn(async move {
            kanban::clear_asks("git", &repo, &name, &asks, &AtomicBool::new(false))
        });
        let file = file.to_owned();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.clears.insert(file, Clearing::Done);
                        // A read begun before the commit would bring the line back for a while.
                        view.cancel();
                        view.last = None;
                    }
                    Err(why) => {
                        view.clears.insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn new_task(&mut self, cx: &mut Context<Self>) {
        let repo = match (&self.board, &self.read) {
            (Some(board), _) => board.repo.clone(),
            (_, Some(Read::NoTasks { repo, .. })) => repo.clone(),
            _ => return,
        };
        cx.emit(KanbanEvent::NewTask { repo });
    }

    /// Read the repository again at once: the task dialog wrote to it.
    pub fn read_again(&mut self) {
        self.cancel();
        self.last = None;
    }

    /// The tasks the task dialog offers to wait for: every card's, newest id first.
    pub fn tasks(&self) -> Vec<TaskChoice> {
        let mut tasks: Vec<TaskChoice> = self
            .board
            .iter()
            .flat_map(|board| board.columns.iter().flatten())
            .filter(|card| !card.id.is_empty())
            .map(|card| TaskChoice {
                id: card.id.clone(),
                title: card.title.clone(),
                stands: if card.draft {
                    "draft"
                } else {
                    match card.column {
                        Column::Queued => "",
                        Column::InProgress => "in progress",
                        Column::ToReview => "to review",
                        Column::Merged => "merged",
                        Column::Done if card.dropped.is_some() => "dropped",
                        Column::Done => "done",
                    }
                },
            })
            .collect();
        tasks.sort_by(|a, b| kanban::natural(&b.id, &a.id));
        tasks
    }

    /// Edit on a card: the task dialog on its task file, or, when its head cannot be read there,
    /// the file in the user's editor.
    fn edit(&self, card: &Card, cx: &mut Context<Self>) {
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        match card.editing() {
            Some(kanban::Editing::Dialog) => cx.emit(KanbanEvent::EditTask {
                repo,
                file: card.file.clone(),
                draft: card.draft,
            }),
            Some(kanban::Editing::Editor) => {
                cx.open_with_system(&repo.join(kanban::TASKS).join(&card.file));
            }
            None => {}
        }
    }
}

/// A task the task dialog offers to wait for.
#[derive(Clone, Debug)]
pub struct TaskChoice {
    id: String,
    title: String,
    /// Where it stands, as the board has it: `draft`, `done` and the like; nothing while queued.
    stands: &'static str,
}

/// Where a card is drawn: in the narrow list, or in a widened column.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    List,
    Tile,
}

impl Place {
    /// A card's agent line here: its text size in points, the avatar's size and the room between
    /// two things on it, in pixels.
    fn foot(self, ui: &UiFont) -> (f32, f32, f32) {
        match self {
            Place::List => (FOOT_SIZE, ui.scale(18.0), ui.scale(8.0)),
            Place::Tile => (TILE_FOOT_SIZE, ui.scale(16.0), ui.scale(6.0)),
        }
    }
}

/// Measures one line as the tab draws it, so that a text sharing a line with others is cut here,
/// with `…`, to the room it has: GPUI's own ellipsis misjudges such text and shows `..` (P5-71; the
/// left sidebar does the same). Text alone on its lines, a title or what the user is waited on
/// for, GPUI cuts.
struct Fit {
    text: Arc<WindowTextSystem>,
    family: SharedString,
    mono: Font,
    ui: UiFont,
}

impl Fit {
    fn width(&self, line: &str, size: f32, font: Font) -> f32 {
        if line.is_empty() {
            return 0.0;
        }
        let run = TextRun {
            len: line.len(),
            font,
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = SharedString::from(line.to_owned());
        f32::from(
            self.text
                .shape_line(line, self.ui.px(size), &[run], None)
                .width,
        )
    }

    /// In the interface font, at `size` points.
    fn normal(&self, line: &str, size: f32) -> f32 {
        self.width(line, size, gpui::font(self.family.clone()))
    }

    fn mono(&self, line: &str, size: f32) -> f32 {
        self.width(line, size, self.mono.clone())
    }
}

/// A pill of words: a small tag with round ends, its edge there but unseen unless coloured.
fn pill(ui: &UiFont) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .px(ui.px(8.0))
        .rounded_full()
        .border_1()
        .border_color(gpui::transparent_black())
        .whitespace_nowrap()
}

/// An agent's name without its group (`paddock/dev-x` is `dev-x`).
fn short_name(name: &str) -> &str {
    name.split_once('/').map_or(name, |(_, short)| short)
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

impl Render for KanbanView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        let fit = Fit {
            text: window.text_system().clone(),
            family: ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into()),
            mono: self.mono.clone(),
            ui: ui.clone(),
        };
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
            .child(div().flex_shrink_0().h(px(1.0)).bg(c.text.opacity(RULE)))
            .child(if wide {
                self.columns(&board, &fit, &ui, cx).into_any_element()
            } else {
                self.list(&board, &fit, &ui, cx).into_any_element()
            })
    }
}

impl KanbanView {
    /// A card is asking whether to clear its task, or which priority it gets.
    pub fn confirming(&self) -> bool {
        let files: HashSet<&str> = self
            .board
            .iter()
            .flat_map(|board| board.columns.iter().flatten())
            .map(|card| card.file.as_str())
            .collect();
        confirming(&self.clears, self.active, &files)
            || confirming(&self.priorities, self.active, &files)
    }

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
            right = right.child(
                pill(ui)
                    .h(ui.px(22.0))
                    .px(ui.px(10.0))
                    .bg(c.yellow.opacity(0.14))
                    .text_size(ui.px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.yellow)
                    .child(need_you_words(need_you)),
            );
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
        let new_task = div()
            .id("kanban-new-task")
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(26.0))
            .rounded_full()
            .bg(c.text.opacity(SOFT))
            .cursor_pointer()
            .hover(move |style| style.bg(c.text.opacity(0.12)))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(footer_icon::icon(
                Icon::Plus,
                c.text,
                ui.scale(12.0 / footer_icon::SIZE),
            ))
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
                pill(ui)
                    .px(ui.px(7.0))
                    .bg(c.text.opacity(SOFT))
                    .font(self.mono.clone())
                    .text_size(ui.px(11.0))
                    .text_color(c.muted)
                    .child(kanban::MAIN),
            )
            .child(div().flex_1())
            .child(right)
            .child(new_task)
    }

    /// New task under the empty state of a repository without task files.
    fn first_task(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        div()
            .id("kanban-first-task")
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
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.new_task(cx)))
    }

    /// Narrow: the five groups one above another, each folding.
    fn list(&self, board: &Board, fit: &Fit, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut list = div()
            .id("kanban-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pt(ui.px(4.0))
            .px(ui.px(LIST_X))
            .pb(ui.px(16.0));
        for (n, column) in Column::ALL.into_iter().enumerate() {
            let open = !self.folded.contains(&column);
            list = list.child(self.group(board, column, open, n == 0, ui, cx));
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
                    .gap(ui.px(CARD_GAP))
                    .pt(ui.px(4.0))
                    .pb(ui.px(2.0));
                for card in shown {
                    cards = cards.child(self.card(card, Place::List, fit, ui, cx));
                }
                if open && column == Column::Done {
                    cards =
                        cards
                            .children(self.show_all(board, ui, cx).map(|more| {
                                div().flex().pt(ui.px(2.0)).pl(ui.px(4.0)).child(more)
                            }));
                }
                list = list.child(cards);
            }
        }
        list
    }

    /// A group's header in the narrow list: the arrow that folds it, its colour, its name and how
    /// many it has. An empty group is one faint line with no arrow, having nothing to fold.
    fn group(
        &self,
        board: &Board,
        column: Column,
        open: bool,
        first: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let empty = board.cards(column).is_empty();
        let mut head = div()
            .id(("kanban-group", column as usize))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(26.0))
            .mt(ui.px(if first { 2.0 } else { 8.0 }))
            .px(ui.px(4.0))
            .rounded(ui.px(6.0))
            .text_size(ui.px(12.0));
        head = if empty {
            head.text_color(c.dim)
                .child(div().flex_shrink_0().w(ui.px(ARROW)))
                .child(self.dot(column, true, ui))
                .child(self.label(column).text_color(c.dim))
                .child(div().text_size(ui.px(11.0)).child("0"))
        } else {
            head.cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.05)))
                .child(footer_icon::icon(
                    if open { Icon::Down } else { Icon::Forward },
                    c.dim,
                    ui.scale(ARROW / footer_icon::SIZE),
                ))
                .child(self.dot(column, false, ui))
                .child(
                    self.label(column)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.bright),
                )
                .child(self.count(board, column, ui))
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.toggle(column, cx)))
        };
        if let Some(note) = self.note(board, column) {
            let tip = Tip::new(note, &self.theme, ui);
            head = head.tooltip(move |_, cx| cx.new(|_| tip.clone()).into());
        }
        head
    }

    /// Widened: five columns side by side, each scrolling on its own.
    fn columns(&self, board: &Board, fit: &Fit, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let mut row = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .gap(ui.px(COLUMN_GAP))
            .pt(ui.px(10.0))
            .px(ui.px(COLUMNS_X))
            .pb(ui.px(12.0));
        for column in Column::ALL {
            let mut body = div()
                .id(("kanban-column", column as usize))
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(ui.px(CARD_GAP))
                .px(ui.px(COLUMN_X))
                .pb(ui.px(8.0));
            let cards = board.listed(column, self.all_done);
            for card in &cards {
                body = body.child(self.card(card, Place::Tile, fit, ui, cx));
            }
            if column == Column::Done {
                body = body.children(
                    self.show_all(board, ui, cx)
                        .map(|more| div().flex().justify_center().pt(ui.px(2.0)).child(more)),
                );
            }
            let empty = cards.is_empty();
            if empty {
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
            let label = self.label(column).font_weight(FontWeight::SEMIBOLD);
            row = row.child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .rounded(ui.px(CARD_RADIUS))
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
                            .text_size(ui.px(12.0))
                            .child(self.dot(column, empty, ui))
                            .child(if empty {
                                label.text_color(c.dim)
                            } else {
                                label.text_color(c.bright)
                            })
                            .child(self.count(board, column, ui)),
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

    /// Under DONE's cards when it has more than its last few, a button: Show all, or while all
    /// show, Show fewer. Kept only until the window closes.
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
        let (words, chevron) = if self.all_done {
            ("Show fewer".to_owned(), CHEVRON_UP)
        } else {
            (format!("Show all {total}"), CHEVRON_DOWN)
        };
        Some(
            div()
                .id("kanban-show-all")
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .h(ui.px(26.0))
                .px(ui.px(12.0))
                .rounded_full()
                .bg(c.text.opacity(SOFT))
                .text_size(ui.px(12.0))
                .text_color(c.bright)
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.11)).text_color(c.text))
                .child(words)
                .child(svg().data(chevron).size(ui.px(10.0)).text_color(c.muted))
                .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                    view.all_done = !view.all_done;
                    cx.notify();
                })),
        )
    }

    /// A group's colour, faint for an empty one.
    fn dot(&self, column: Column, empty: bool, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .size(ui.px(6.0))
            .rounded_full()
            .bg(if empty {
                self.colors.faint
            } else {
                self.colors.column(column)
            })
    }

    fn label(&self, column: Column) -> Div {
        div()
            .flex_shrink(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .child(column.label())
    }

    /// How many a group has, in a pill: `3 of 130` while DONE lists its last few.
    fn count(&self, board: &Board, column: Column, ui: &UiFont) -> Div {
        let c = self.colors;
        pill(ui)
            .px(ui.px(7.0))
            .bg(c.text.opacity(SOFT))
            .text_size(ui.px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.muted)
            .child(board.count(column, self.all_done))
    }

    /// The width inside a card's padding, in the narrow list or a widened column, that its texts
    /// sharing a line are cut to.
    fn room(&self, place: Place, ui: &UiFont) -> f32 {
        let card = match place {
            Place::List => self.width - 2.0 * ui.scale(LIST_X),
            Place::Tile => {
                let row = self.width - 2.0 * ui.scale(COLUMNS_X) - 4.0 * ui.scale(COLUMN_GAP);
                row / 5.0 - 2.0 - 2.0 * ui.scale(COLUMN_X)
            }
        };
        (card - ui.scale(CARD_LEFT + CARD_RIGHT) - 2.0 - SPARE).max(0.0)
    }

    fn hovered(&self, card: &Card) -> bool {
        self.hovered.as_deref() == Some(card.file.as_str())
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

    /// A card (P5-71), in the narrow list or a widened column, lifted off the panel as the left
    /// sidebar's cards are (P5-51b): its id, any warning and its age; its title on up to two lines;
    /// its marks; what the user is waited on for; a question asked on it; then, under a rule, its
    /// agent, and the controller on one to review. A DONE card that needs nothing and was not
    /// dropped is one short line.
    fn card(
        &self,
        card: &Card,
        place: Place,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if card.column == Column::Done && !card.needs_you && card.dropped.is_none() {
            return self.done_row(card, place, fit, ui, cx);
        }
        let c = self.colors;
        let hovered = self.hovered(card);
        let room = self.room(place, ui);
        let (face, edge, lines) = self.face(card, hovered);
        // While the mouse is on the card its buttons sit where its age is, and the age moves down
        // to its agent's line when it has one.
        let footer = self.footer(
            card,
            place,
            hovered,
            room,
            c.ground.blend(face),
            fit,
            ui,
            cx,
        );
        let first = div()
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .min_w(px(0.0))
            .min_h(ui.px(18.0))
            .children(self.id(card, ui))
            .children(self.warning(card, ui))
            .child(div().flex_1())
            .children((!hovered && !card.age.is_empty()).then(|| {
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(AGE_SIZE))
                    .text_color(c.dim)
                    .child(card.age.clone())
            }));
        let title = div()
            .line_clamp(2)
            .text_ellipsis()
            .line_height(relative(1.4))
            .font_weight(if card.column == Column::Done {
                FontWeight::NORMAL
            } else {
                FontWeight::MEDIUM
            })
            .text_color(if card.draft { c.bright } else { c.text })
            .child(card.title.clone());
        let marks = self.marks(card, ui);
        let marks =
            (!marks.is_empty()).then(|| div().flex().flex_wrap().gap(ui.px(6.0)).children(marks));
        let size = match place {
            Place::List => BAR_BUTTON,
            Place::Tile => TILE_BUTTON,
        };
        let bar = self.actions(card, size, ui, cx).map(|bar| {
            self.tray(bar, ui)
                .top(ui.px(BAR_TOP))
                .when(place == Place::Tile, |bar| {
                    bar.flex_wrap().justify_end().max_w(px(room))
                })
        });
        let foot = footer.is_some();
        self.hoverable(card, cx)
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .pt(ui.px(CARD_Y))
            .pb(if foot { px(0.0) } else { ui.px(CARD_Y) })
            .pl(ui.px(CARD_LEFT))
            .pr(ui.px(CARD_RIGHT))
            .rounded(ui.px(CARD_RADIUS))
            .bg(face)
            .border_1()
            .border_color(edge)
            .when(card.draft, |ground| ground.border_dashed())
            .shadow(lines)
            .opacity(fade(card))
            .text_size(ui.px(13.0))
            .children(self.stripe(card, ui))
            .child(first)
            .child(title)
            .children(marks)
            .children(self.asks(card, ui))
            .children(
                self.clearing(card, ui, cx)
                    .map(|row| row.text_size(ui.px(12.0))),
            )
            .children(
                self.prioritizing(card, ui, cx)
                    .map(|row| row.text_size(ui.px(12.0))),
            )
            .children(footer)
            .children(bar)
    }

    /// A DONE card that needs nothing and was not dropped: one short line, its id, title and age,
    /// the title cut with `…` to the room they leave it.
    fn done_row(
        &self,
        card: &Card,
        place: Place,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let hovered = self.hovered(card);
        let mut room = self.room(place, ui);
        let mut gaps = 0;
        if !card.id.is_empty() {
            room -= fit.mono(&card.id, ID_SIZE);
            gaps += 1;
        }
        if !card.age.is_empty() {
            room -= fit.normal(&card.age, AGE_SIZE);
            gaps += 1;
        }
        room -= gaps as f32 * ui.scale(8.0);
        let title = card::elide(&card.title, &|cut| fit.normal(cut, DONE_SIZE) <= room);
        let bar = self
            .actions(card, BAR_BUTTON, ui, cx)
            .map(|bar| self.tray(bar, ui).top(ui.px(DONE_BAR_TOP)));
        self.hoverable(card, cx)
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .py(ui.px(8.0))
            .pl(ui.px(CARD_LEFT))
            .pr(ui.px(CARD_RIGHT))
            .rounded(ui.px(CARD_RADIUS))
            // An edge no one sees, so its words line up with the cards'.
            .border_1()
            .border_color(gpui::transparent_black())
            .when(hovered, |row| row.bg(c.text.opacity(0.04)))
            .whitespace_nowrap()
            .text_color(c.muted)
            .children((!card.id.is_empty()).then(|| {
                div()
                    .flex_shrink_0()
                    .font(self.mono.clone())
                    .text_size(ui.px(ID_SIZE))
                    .text_color(c.dim)
                    .child(card.id.clone())
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_size(ui.px(DONE_SIZE))
                    .child(title),
            )
            .children((!card.age.is_empty()).then(|| {
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(AGE_SIZE))
                    .text_color(c.dim)
                    .child(card.age.clone())
            }))
            .children(bar)
    }

    /// A card's face, edge, and the lines inside its top and bottom edges, resting or with the
    /// mouse on it, as the left sidebar's cards over the material (P5-51b): a draft's fainter and
    /// dashed, without the lines; one that needs the user rimmed in amber.
    fn face(&self, card: &Card, hovered: bool) -> (Hsla, Hsla, Vec<BoxShadow>) {
        let c = self.colors;
        if card.draft {
            return if hovered {
                (
                    c.text.opacity(DRAFT_FACE_HOVERED),
                    c.text.opacity(DRAFT_EDGE_HOVERED),
                    Vec::new(),
                )
            } else {
                (
                    c.text.opacity(DRAFT_FACE),
                    c.text.opacity(DRAFT_EDGE),
                    Vec::new(),
                )
            };
        }
        let (face, edge, light) = if hovered {
            (FACE_HOVERED, EDGE_HOVERED, LIGHT_HOVERED)
        } else {
            (FACE, EDGE, LIGHT)
        };
        let lines = vec![
            BoxShadow::new(px(0.0), px(1.0), c.text.opacity(light)).inset(),
            BoxShadow::new(px(0.0), px(-1.0), gpui::black().opacity(BOTTOM)).inset(),
        ];
        if card.needs_you {
            let face = if hovered {
                WAITING_FACE_HOVERED
            } else {
                WAITING_FACE
            };
            return (
                c.yellow.opacity(face),
                c.yellow.opacity(WAITING_EDGE),
                lines,
            );
        }
        (c.text.opacity(face), c.text.opacity(edge), lines)
    }

    /// The small tray the buttons over a card sit in, at its top right.
    fn tray(&self, bar: Div, ui: &UiFont) -> Div {
        bar.absolute()
            .right(ui.px(BAR_RIGHT))
            .p(ui.px(BAR_PAD))
            .rounded(ui.px(8.0))
            .bg(self.colors.ground)
            .border_1()
            .border_color(self.colors.text.opacity(0.08))
    }

    /// Under a rule at the foot of a card under way, to review or merged: its agent's line and,
    /// on one to review, the controller's. `ring` is the card's face over the panel, round the
    /// avatars' status dots. `None` with nothing to say.
    #[allow(clippy::too_many_arguments)]
    fn footer(
        &self,
        card: &Card,
        place: Place,
        hovered: bool,
        room: f32,
        ring: Hsla,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if matches!(card.column, Column::Queued | Column::Done) {
            return None;
        }
        let size = match place {
            Place::List => FOOT_SIZE,
            Place::Tile => TILE_FOOT_SIZE,
        };
        let meta = self.meta(card, place, hovered, room, ring, fit, ui);
        let reviewer = self.reviewer(card, place, room, ring, fit, ui, cx);
        if meta.is_none() && reviewer.is_none() {
            return None;
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap(ui.px(6.0))
                .mt(ui.px(4.0))
                .ml(ui.px(-CARD_LEFT))
                .mr(ui.px(-CARD_RIGHT))
                .pt(ui.px(8.0))
                .pb(ui.px(9.0))
                .pl(ui.px(CARD_LEFT))
                .pr(ui.px(CARD_RIGHT))
                .border_t_1()
                .border_color(self.colors.text.opacity(RULE))
                .text_size(ui.px(size))
                .whitespace_nowrap()
                .children(meta)
                .children(reviewer),
        )
    }

    /// On a card to review, under its agent, the repository's controller as the agent is drawn:
    /// its kind and status, its name and status, and that it reviews. A click brings its pane to
    /// the front, as a click on its card in the sidebar does.
    #[allow(clippy::too_many_arguments)]
    fn reviewer(
        &self,
        card: &Card,
        place: Place,
        room: f32,
        ring: Hsla,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let c = self.colors;
        let agent = card.controller.as_ref()?;
        let look = card::look(agent.status);
        let (size, avatar, gap) = place.foot(ui);
        let words = "reviewer";
        let left = room
            - avatar
            - fit.normal(look.label, size)
            - fit.normal(words, REVIEWER_SIZE)
            - 4.0 * gap;
        let short = card::elide(short_name(&agent.name), &|cut| {
            fit.normal(cut, size) <= left
        });
        let name = agent.name.clone();
        Some(
            div()
                .id(SharedString::from(format!("kanban-reviewer-{}", card.file)))
                .flex()
                .items_center()
                .gap(px(gap))
                .min_w(px(0.0))
                .mx(ui.px(-4.0))
                .px(ui.px(4.0))
                .rounded(ui.px(5.0))
                .text_color(c.muted)
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.06)))
                .child(self.avatar(agent, ui, avatar, ring))
                .child(div().flex_shrink_0().text_color(c.bright).child(short))
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
                        .text_size(ui.px(REVIEWER_SIZE))
                        .text_color(c.dim)
                        .child(words),
                )
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    cx.emit(KanbanEvent::GoTo(name.clone()));
                })),
        )
    }

    /// A bar down a queued card's left edge, for high (red, as its High mark) or low (dim, as its
    /// Low mark); none for medium or in the other columns.
    fn stripe(&self, card: &Card, ui: &UiFont) -> Option<Div> {
        let color = match card.stripe()? {
            Priority::High => self.colors.red,
            _ => self.colors.dim,
        };
        Some(
            div()
                .absolute()
                .left(px(0.0))
                .top(ui.px(9.0))
                .bottom(ui.px(9.0))
                .w(px(3.0))
                .rounded_full()
                .bg(color),
        )
    }

    /// The id, for a card that has one, on a small tag.
    fn id(&self, card: &Card, ui: &UiFont) -> Option<Div> {
        (!card.id.is_empty()).then(|| {
            div()
                .flex_shrink_0()
                .px(ui.px(6.0))
                .rounded(ui.px(5.0))
                .bg(self.colors.text.opacity(SOFT))
                .font(self.mono.clone())
                .text_size(ui.px(ID_SIZE))
                .text_color(self.colors.muted)
                .child(card.id.clone())
        })
    }

    /// A warning sign beside the id that tells, when the mouse is on it, what in the card's task
    /// file could be read wrong (P5-60a).
    fn warning(&self, card: &Card, ui: &UiFont) -> Option<Stateful<Div>> {
        if card.problems.is_empty() {
            return None;
        }
        let said: Vec<String> = card.problems.iter().map(ToString::to_string).collect();
        let note = Note::new(said.join("\n").into(), &self.theme, ui);
        Some(
            div()
                .id(SharedString::from(format!("kanban-problems-{}", card.file)))
                .flex_shrink_0()
                .flex()
                .items_center()
                .child(footer_icon::icon(
                    Icon::Warning,
                    self.colors.yellow,
                    ui.scale(12.0 / footer_icon::SIZE),
                ))
                .tooltip(move |_, cx| cx.new(|_| note.clone()).into()),
        )
    }

    /// The marks under a card's title, each a pill: Draft, High or Low (medium is not marked),
    /// what a queued card waits for, and Dropped, which tells why when the mouse is on it.
    fn marks(&self, card: &Card, ui: &UiFont) -> Vec<AnyElement> {
        let c = self.colors;
        let quiet = || pill(ui).text_size(ui.px(11.0)).text_color(c.dim);
        let mut marks = Vec::new();
        if card.draft {
            marks.push(
                quiet()
                    .border_color(c.text.opacity(0.14))
                    .child("Draft")
                    .into_any_element(),
            );
        }
        match card.priority {
            Some(Priority::High) => marks.push(
                quiet()
                    .bg(c.red.opacity(0.14))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.red)
                    .child(Priority::High.label())
                    .into_any_element(),
            ),
            Some(Priority::Low) => marks.push(
                quiet()
                    .bg(c.text.opacity(SOFT))
                    .child(Priority::Low.label())
                    .into_any_element(),
            ),
            _ => {}
        }
        if card.column == Column::Queued
            && let Some((words, _)) = &card.state
        {
            marks.push(
                quiet()
                    .gap(ui.px(4.0))
                    .bg(c.text.opacity(SOFT))
                    .text_color(c.muted)
                    .child(svg().data(LINK).size(ui.px(10.0)).text_color(c.muted))
                    .child(words.clone())
                    .into_any_element(),
            );
        }
        if let Some(why) = &card.dropped {
            let note = Note::new(
                if why.is_empty() {
                    "Dropped".into()
                } else {
                    format!("Dropped: {why}").into()
                },
                &self.theme,
                ui,
            );
            marks.push(
                quiet()
                    .id(SharedString::from(format!("kanban-dropped-{}", card.file)))
                    .border_color(c.text.opacity(0.14))
                    .child("Dropped")
                    .tooltip(move |_, cx| cx.new(|_| note.clone()).into())
                    .into_any_element(),
            );
        }
        marks
    }

    /// On a card that needs the user, a pale amber block: Needs you, then what its task file says
    /// they are waited on for, on up to two lines and cut with `…`. One text, so GPUI wraps and
    /// cuts it alone on its lines.
    fn asks(&self, card: &Card, ui: &UiFont) -> Option<Div> {
        if !card.needs_you {
            return None;
        }
        let c = self.colors;
        let lead = "Needs you";
        let text = match card
            .asks
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
        {
            Some(asks) => format!("{lead}  {asks}"),
            None => lead.to_owned(),
        };
        let bold = HighlightStyle {
            color: Some(c.yellow),
            font_weight: Some(FontWeight::SEMIBOLD),
            ..HighlightStyle::default()
        };
        Some(
            div()
                .px(ui.px(9.0))
                .py(ui.px(7.0))
                .rounded(ui.px(7.0))
                .bg(c.yellow.opacity(0.08))
                .text_size(ui.px(12.0))
                .line_height(relative(1.45))
                .text_color(c.text)
                .line_clamp(2)
                .text_ellipsis()
                .child(StyledText::new(text).with_highlights([(0..lead.len(), bold)])),
        )
    }

    /// A card's agent line: the agent's kind and status, its name (in the list), the state, the
    /// branch (in the list), the lines added and deleted, and while the mouse is on the card, its
    /// age; the name and branch, or in a column the state, cut with `…` to the room left. `None`
    /// with nothing to say.
    #[allow(clippy::too_many_arguments)]
    fn meta(
        &self,
        card: &Card,
        place: Place,
        hovered: bool,
        room: f32,
        ring: Hsla,
        fit: &Fit,
        ui: &UiFont,
    ) -> Option<Div> {
        let c = self.colors;
        if card.agent.is_none() && card.state.is_none() && card.branch.is_none() {
            return None;
        }
        let full = place == Place::List;
        let (size, avatar, gap) = place.foot(ui);
        let name = card
            .agent
            .as_ref()
            .filter(|_| full)
            .map(|agent| short_name(&agent.name));
        let branch = card.branch.as_deref().filter(|_| full);
        let (added, deleted) = card.lines.map(kanban::line_words).unwrap_or_default();
        let age = (hovered && !card.age.is_empty()).then_some(card.age.as_str());
        // What keeps its width, and the room between each two things on the line.
        let mut fixed = 0.0;
        let mut things = 1; // The room that pushes the rest to the right.
        if card.agent.is_some() {
            fixed += avatar;
            things += 1;
        }
        let words = |text: &str, size: f32| fit.normal(text, size);
        let lines = [&added, &deleted]
            .into_iter()
            .flatten()
            .map(|l| words(l, size))
            .collect::<Vec<_>>();
        if !lines.is_empty() {
            fixed += lines.iter().sum::<f32>() + (lines.len() - 1) as f32 * ui.scale(4.0);
            things += 1;
        }
        if let Some(age) = age {
            fixed += words(age, AGE_SIZE);
            things += 1;
        }
        things += usize::from(name.is_some()) + usize::from(branch.is_some());
        let state = card.state.as_ref().map(|(words, _)| words.as_str());
        if state.is_some() {
            things += 1;
        }
        let left = room - fixed - (things - 1) as f32 * gap;
        let (state, name, branch) = if full {
            // The name and branch give way; the state only once they are down to `…` each.
            let floor = [name, branch]
                .into_iter()
                .flatten()
                .map(|_| words("…", size))
                .sum::<f32>();
            let state = state.map(|s| card::elide(s, &|cut| words(cut, size) <= left - floor));
            let pair = left - state.as_deref().map_or(0.0, |s| words(s, size));
            let (name, branch) = share(
                name.unwrap_or(""),
                branch.unwrap_or(""),
                pair,
                &|text| words(text, size),
                &|text| fit.mono(text, BRANCH_SIZE),
            );
            (
                state,
                Some(name).filter(|n| !n.is_empty()),
                Some(branch).filter(|b| !b.is_empty()),
            )
        } else {
            (
                state.map(|s| card::elide(s, &|cut| words(cut, size) <= left)),
                None,
                None,
            )
        };
        let mut meta = div()
            .flex()
            .items_center()
            .gap(px(gap))
            .min_w(px(0.0))
            .text_color(c.muted);
        if let Some(agent) = &card.agent {
            meta = meta.child(self.avatar(agent, ui, avatar, ring));
        }
        if let Some(name) = name {
            meta = meta.child(div().flex_shrink_0().text_color(c.bright).child(name));
        }
        if let (Some(words), Some((_, tone))) = (state, &card.state) {
            let color = match tone {
                Tone::Dim => c.dim,
                Tone::Agent => card.agent.as_ref().map_or(c.muted, |a| {
                    hsla(self.theme.fg(card::look(a.status).color), 1.0)
                }),
                Tone::Review => c.yellow,
                Tone::Merged => c.green,
            };
            meta = meta.child(div().flex_shrink_0().text_color(color).child(words));
        }
        meta = meta.child(div().flex_1());
        if let Some(branch) = branch {
            meta = meta.child(
                div()
                    .flex_shrink_0()
                    .font(self.mono.clone())
                    .text_size(ui.px(BRANCH_SIZE))
                    .text_color(c.dim)
                    .child(branch),
            );
        }
        if !lines.is_empty() {
            meta = meta.child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap(ui.px(4.0))
                    .children(added.map(|added| div().text_color(c.green).child(added)))
                    .children(deleted.map(|deleted| div().text_color(c.red).child(deleted))),
            );
        }
        if let Some(age) = age {
            meta = meta.child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(AGE_SIZE))
                    .text_color(c.dim)
                    .child(age.to_owned()),
            );
        }
        Some(meta)
    }

    /// The agent's kind on a square tinted in its colour, its status on the corner: an arc
    /// turning while it works, as on the left sidebar's cards, otherwise a dot in the status's
    /// colour; `ring` round either, the ground under the avatar. `size` in pixels.
    fn avatar(&self, agent: &Seen, ui: &UiFont, size: f32, ring: Hsla) -> Div {
        let brand = agent.kind.as_deref().map(card::brand);
        let color = hsla(
            self.theme
                .fg(brand.as_ref().map_or(|t| t.agents_dim, |b| b.color)),
            1.0,
        );
        let mark = match agent.kind.as_deref().and_then(kind_icon::of) {
            Some(icon) => icon.render(px(size * 0.6), color).into_any_element(),
            None => div()
                .text_size(px(size * 0.6))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(crate::sidebar::initial(short_name(&agent.name)))
                .into_any_element(),
        };
        let status = hsla(self.theme.fg(card::look(agent.status).color), 1.0);
        let badge: AnyElement = if agent.status == Status::Working {
            let spin = px(size * 0.5);
            div()
                .absolute()
                .right(ui.px(-3.0))
                .bottom(ui.px(-3.0))
                .size(spin)
                .rounded_full()
                .bg(ring)
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
                .size(px(dot) + ui.px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(ring)
                .child(div().size(px(dot)).rounded_full().bg(status))
                .into_any_element()
        };
        div()
            .relative()
            .flex_shrink_0()
            .size(px(size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(size * 0.28))
            .bg(color.opacity(0.14))
            .child(mark)
            .child(badge)
    }

    /// While the mouse is on the card, its ways out, `size` each and placed by the caller: the task
    /// file, and with an agent, its pane and its changes; on a card its task file says needs the
    /// user, Clear, which asks first; on a queued card, Set priority, which asks which.
    fn actions(&self, card: &Card, size: f32, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        if self.hovered.as_deref() != Some(card.file.as_str()) {
            return None;
        }
        let c = self.colors;
        let theme = self.theme.clone();
        let glyph = |id: &'static str, glyph: AnyElement, words: &'static str| {
            let tip = Tip::new(words, &theme, ui);
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(size))
                .rounded(px(5.0))
                .cursor_pointer()
                .hover(move |style| style.bg(c.text.opacity(0.09)))
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .child(glyph)
        };
        let button = |id: &'static str, icon: Icon, words: &'static str| {
            let icon = footer_icon::icon(icon, c.muted, ui.scale(13.0 / footer_icon::SIZE));
            glyph(id, icon.into_any_element(), words)
        };
        let path = self
            .board
            .as_ref()
            .map(|b| b.repo.join(kanban::TASKS).join(&card.file));
        let mut bar = div().flex().gap(ui.px(BAR_GAP)).child(
            button("kanban-open", Icon::Document, "Open task file").on_click(move |_, _, cx| {
                cx.stop_propagation();
                if let Some(path) = &path {
                    cx.open_with_system(path);
                }
            }),
        );
        if let Some(agent) = &card.agent {
            let (go, show) = (agent.name.clone(), agent.name.clone());
            bar = bar.child(
                button("kanban-agent", Icon::NewShell, "Go to agent").on_click(cx.listener(
                    move |_, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        cx.emit(KanbanEvent::GoTo(go.clone()));
                    },
                )),
            );
            // Changes shows an agent's own directory: the controller's is the main worktree, not
            // the task's (P5-67).
            if !agent.controller {
                bar = bar.child(
                    button("kanban-changes", Icon::Changes, "Show changes").on_click(cx.listener(
                        move |_, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            cx.emit(KanbanEvent::Changes(show.clone()));
                        },
                    )),
                );
            }
        }
        if self.offers_clear(card) {
            let file = card.file.clone();
            bar = bar.child(
                button(
                    "kanban-clear",
                    Icon::Check,
                    "Clear \u{201c}Needs you\u{201d}",
                )
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    view.ask_clear(&file, cx);
                })),
            );
        }
        if self.offers_priority(card) {
            let file = card.file.clone();
            bar = bar.child(
                button("kanban-priority", Icon::Sort, "Set priority").on_click(cx.listener(
                    move |view, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        view.ask_priority(&file, cx);
                    },
                )),
            );
        }
        if let Some(editing) = card.editing() {
            let words = match editing {
                kanban::Editing::Dialog => "Edit task",
                kanban::Editing::Editor => "Open file: its header can't be read here",
            };
            let pencil = svg()
                .data(PENCIL)
                .size(ui.px(13.0))
                .text_color(c.muted)
                .into_any_element();
            let card = card.clone();
            bar = bar.child(glyph("kanban-edit", pencil, words).on_click(cx.listener(
                move |view, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    view.edit(&card, cx);
                },
            )));
        }
        Some(bar)
    }

    /// Whether the buttons over `card` include Clear: its task file says it needs the user, and it
    /// is not being cleared or cleared already.
    fn offers_clear(&self, card: &Card) -> bool {
        let busy = matches!(
            self.clears.get(&card.file),
            Some(Clearing::Running | Clearing::Done)
        );
        card.clearable() && !busy
    }

    /// Whether the buttons over `card` include Set priority: it is queued, a draft or not, and its
    /// priority is not being changed or changed already.
    fn offers_priority(&self, card: &Card) -> bool {
        let busy = matches!(
            self.priorities.get(&card.file),
            Some(Clearing::Running | Clearing::Done)
        );
        card.priority.is_some() && !busy
    }

    /// Under a card being cleared: the question with Cancel and Clear, then that it is clearing,
    /// or why it was not.
    fn clearing(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        let row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(ui.px(6.0))
            .min_w(px(0.0));
        let words = |words: String, color: Hsla| {
            div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .whitespace_normal()
                .text_color(color)
                .child(words)
        };
        Some(match self.clears.get(&card.file)? {
            Clearing::Asking => {
                let (file, asks) = (card.file.clone(), card.asks.clone().unwrap_or_default());
                let leave = card.file.clone();
                let button = |id: &'static str, words: &'static str| {
                    div()
                        .id(id)
                        .flex_shrink_0()
                        .px(ui.px(8.0))
                        .h(ui.px(20.0))
                        .flex()
                        .items_center()
                        .rounded(px(5.0))
                        .border_1()
                        .cursor_pointer()
                        .child(words)
                };
                row.child(words(
                    "Clear \u{201c}Needs you\u{201d}? Commits to main.".into(),
                    c.text,
                ))
                .child(div().flex_1())
                // The two buttons stay together when the row wraps.
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .gap(ui.px(6.0))
                        .child(
                            button("kanban-clear-cancel", "Cancel")
                                .border_color(c.rule)
                                .text_color(c.muted)
                                .hover(move |style| style.bg(c.text.opacity(0.09)))
                                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    if view.clears.remove(&leave).is_some() {
                                        cx.notify();
                                    }
                                })),
                        )
                        .child(
                            button("kanban-clear-confirm", "Clear")
                                .bg(c.yellow.opacity(0.16))
                                .border_color(c.yellow.opacity(0.5))
                                .text_color(c.yellow)
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(move |style| style.bg(c.yellow.opacity(0.26)))
                                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    view.clear(&file, &asks, cx);
                                })),
                        ),
                )
            }
            Clearing::Running => row.child(words("Clearing\u{2026}".into(), c.dim)),
            Clearing::Done => row.child(words("Cleared".into(), c.dim)),
            Clearing::Refused(why) => row.child(words(format!("Not cleared: {why}"), c.red)),
        })
    }

    /// Under a card whose priority is asked for, as under one being cleared: High, Medium and Low,
    /// the one it has marked, and Cancel; then that it is changing, or why it was not.
    fn prioritizing(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        let row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(ui.px(6.0))
            .min_w(px(0.0));
        let words = |words: String, color: Hsla| {
            div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .whitespace_normal()
                .text_color(color)
                .child(words)
        };
        Some(match self.priorities.get(&card.file)? {
            Clearing::Asking => {
                let button = |id: (&'static str, usize), words: &'static str| {
                    div()
                        .id(id)
                        .flex_shrink_0()
                        .px(ui.px(8.0))
                        .h(ui.px(20.0))
                        .flex()
                        .items_center()
                        .rounded(px(5.0))
                        .border_1()
                        .cursor_pointer()
                        .child(words)
                };
                let mut buttons = div().flex_shrink_0().flex().gap(ui.px(6.0));
                for (n, priority) in Priority::ALL.into_iter().enumerate() {
                    let picked = card.clone();
                    let choice = button(("kanban-priority-pick", n), priority.label()).on_click(
                        cx.listener(move |view, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            view.set_priority(&picked, priority, cx);
                        }),
                    );
                    buttons = buttons.child(if card.priority == Some(priority) {
                        choice
                            .bg(c.yellow.opacity(0.16))
                            .border_color(c.yellow.opacity(0.5))
                            .text_color(c.yellow)
                            .font_weight(FontWeight::SEMIBOLD)
                            .hover(move |style| style.bg(c.yellow.opacity(0.26)))
                    } else {
                        choice
                            .border_color(c.rule)
                            .text_color(c.text)
                            .hover(move |style| style.bg(c.text.opacity(0.09)))
                    });
                }
                let leave = card.file.clone();
                buttons = buttons.child(
                    button(("kanban-priority-cancel", 0), "Cancel")
                        .border_color(c.rule)
                        .text_color(c.muted)
                        .hover(move |style| style.bg(c.text.opacity(0.09)))
                        .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            if view.priorities.remove(&leave).is_some() {
                                cx.notify();
                            }
                        })),
                );
                let question = if card.draft {
                    "Priority?"
                } else {
                    "Priority? Commits to main."
                };
                row.child(words(question.into(), c.text))
                    .child(div().flex_1())
                    // The buttons stay together when the row wraps.
                    .child(buttons)
            }
            Clearing::Running => row.child(words("Setting priority\u{2026}".into(), c.dim)),
            Clearing::Done => row.child(words("Priority set".into(), c.dim)),
            Clearing::Refused(why) => row.child(words(format!("Not changed: {why}"), c.red)),
        })
    }
}

/// The task dialog's width, the height of its body's box, the room it keeps from the window's
/// edges, in points at the base interface size, and how far down the window its top sits.
const DIALOG_WIDTH: f32 = 640.0;
const BODY_HEIGHT: f32 = 270.0;
const DIALOG_MARGIN: f32 = 16.0;
const DIALOG_TOP: f32 = 0.1;
/// Rows the Waits for list shows before it scrolls.
const WAITS_ROWS: usize = 8;

/// The task dialog over the dimmed window (DESIGN §13 P5-60b, P5-69), for a new task or a queued
/// one, a draft's too: the lines paddock writes at the head of the task file (its id, title,
/// priority, the task it waits for and who does it), and its body, written as Markdown, with a preview. ⌘↩ writes it: a new
/// draft with [`kanban::create_draft`], or the task file with [`kanban::save_task`] (committed alone
/// to main) or [`kanban::save_draft`]; when that is refused it says why and stays open. Esc, Cancel
/// and × close it. ↑↓, Tab and ↩ in a one-line field move between the fields.
pub struct TaskDialog {
    repo: PathBuf,
    theme: Rc<Theme>,
    mono: Font,
    focus: FocusHandle,
    /// The task file being edited; `None` for a new task.
    opened: Option<Opened>,
    id: Entity<TextInput>,
    title: Entity<TextInput>,
    body: Entity<TextInput>,
    priority: Priority,
    doer: Doer,
    depends: Option<String>,
    /// What Waits for offers.
    tasks: Vec<TaskChoice>,
    /// The Waits for list, while it is open.
    waits: Option<Waits>,
    /// Where a press outside the Waits for list just closed it, so the same press on its button
    /// does not open it again.
    dismissed: Option<Point<Pixels>>,
    /// Preview shows in place of Write.
    preview: bool,
    /// The file's head shows under the body.
    show_head: bool,
    /// Today, for a new task's head.
    date: String,
    /// Why the last write was refused, until something changes.
    refused: Option<String>,
    writing: bool,
    _subscriptions: Vec<Subscription>,
}

/// A task file the dialog edits.
struct Opened {
    file: String,
    draft: bool,
    /// Its text as read when the dialog opened, saved only over the same; `None` when its head
    /// cannot be read for the dialog.
    text: Option<String>,
}

/// The open Waits for list: a search field over the tasks that match.
struct Waits {
    search: Entity<TextInput>,
    /// The lit row: 0 for None, then the matches.
    index: usize,
    scroll: UniformListScrollHandle,
    _subscription: Subscription,
}

/// What the task dialog tells the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskDialogEvent {
    /// The task file was written: close, and read the board again.
    Written,
    Cancel,
}

impl EventEmitter<TaskDialogEvent> for TaskDialog {}

/// The colours of the dialog's text fields.
fn input_colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
    }
}

impl TaskDialog {
    /// For the main worktree `repo`: a new task, offered the next free id, its title taking the
    /// keys; or the task file `edit` names (and whether it is a draft), its body taking the keys.
    pub fn new(
        repo: PathBuf,
        edit: Option<(String, bool)>,
        tasks: Vec<TaskChoice>,
        theme: Rc<Theme>,
        mono: Font,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let colors = input_colors(&theme);
        let opened = edit.map(|(file, draft)| {
            let text = std::fs::read_to_string(repo.join(kanban::TASKS).join(&file))
                .ok()
                .filter(|text| kanban::editable(text).is_some());
            Opened { file, draft, text }
        });
        let fields = match &opened {
            Some(opened) => opened
                .text
                .as_deref()
                .and_then(kanban::editable)
                .unwrap_or_default(),
            None => kanban::Fields {
                body: kanban::BODY.to_owned(),
                ..kanban::Fields::default()
            },
        };
        let id = opened
            .as_ref()
            .and_then(|opened| kanban::task_id(&opened.file))
            .unwrap_or_default();
        let id = cx.new(|cx| TextInput::new(id, "ID", colors, cx));
        let title = cx.new(|cx| TextInput::new(fields.title, "Short title", colors, cx));
        let body = cx.new(|cx| TextInput::new(fields.body, "Markdown", colors, cx).wrapping(true));
        let subscriptions = [&id, &title, &body]
            .into_iter()
            .map(|input| {
                cx.subscribe(input, |this, _, _: &Changed, cx| {
                    this.refused = None;
                    cx.notify();
                })
            })
            .collect();
        let first = if opened.is_some() { &body } else { &title };
        window.focus(&first.focus_handle(cx), cx);
        if opened.is_none() {
            // The next free id, from main, the worktree and every branch; unless one is typed
            // first.
            let dir = repo.clone();
            let taken =
                cx.background_spawn(
                    async move { kanban::taken("git", &dir, &AtomicBool::new(false)) },
                );
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
        }
        let (year, month, day) = activity::date(activity::today());
        TaskDialog {
            repo,
            theme,
            mono,
            focus: cx.focus_handle(),
            opened,
            id,
            title,
            body,
            priority: fields.priority,
            doer: fields.doer,
            depends: fields.depends,
            tasks,
            waits: None,
            dismissed: None,
            preview: false,
            show_head: false,
            date: format!("{year:04}-{month:02}-{day:02}"),
            refused: None,
            writing: false,
            _subscriptions: subscriptions,
        }
    }

    /// What the fields say now.
    fn fields(&self, cx: &App) -> kanban::Fields {
        kanban::Fields {
            title: self.title.read(cx).text().to_owned(),
            depends: self.depends.clone(),
            doer: self.doer,
            priority: self.priority,
            body: self.body.read(cx).text().to_owned(),
        }
    }

    /// Why nothing can be written here: the task file's head could not be read.
    fn unreadable(&self) -> Option<String> {
        let opened = self
            .opened
            .as_ref()
            .filter(|opened| opened.text.is_none())?;
        Some(format!(
            "{}/{}'s first line isn't \u{201c}# 任务：<title>\u{201d}, or it can't be read: \
             open it in your editor",
            kanban::TASKS,
            opened.file
        ))
    }

    /// What paddock writes at the head of the file, as the fields say now.
    fn head(&self, cx: &App) -> Option<String> {
        let fields = self.fields(cx);
        let head = match &self.opened {
            None => kanban::new_head(&fields, &self.date),
            Some(opened) => {
                let text = opened.text.as_deref()?;
                kanban::head(&kanban::edited(text, &fields)).to_owned()
            }
        };
        Some(head.trim_end().to_owned())
    }

    /// ⌘↩: the draft created, or the task file saved, in the background; closed once written.
    fn write(&mut self, cx: &mut Context<Self>) {
        if self.writing {
            return;
        }
        let fields = self.fields(cx);
        let repo = self.repo.clone();
        let task = match &self.opened {
            None => {
                let (id, date) = (self.id.read(cx).text().to_owned(), self.date.clone());
                cx.background_spawn(async move {
                    let cancel = AtomicBool::new(false);
                    kanban::create_draft("git", &repo, &id, &fields, &date, &cancel)
                        .map(drop)
                        .map_err(|refused| refused.to_string())
                })
            }
            Some(Opened {
                text: Some(text),
                file,
                draft,
            }) => {
                let (text, file, draft) = (text.clone(), file.clone(), *draft);
                cx.background_spawn(async move {
                    let saved = if draft {
                        kanban::save_draft(&repo, &file, &text, &fields)
                    } else {
                        let cancel = AtomicBool::new(false);
                        kanban::save_task("git", &repo, &file, &text, &fields, &cancel)
                    };
                    saved.map_err(|why| why.to_string())
                })
            }
            Some(Opened { text: None, .. }) => return,
        };
        self.writing = true;
        self.refused = None;
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.writing = false;
                match result {
                    Ok(()) => cx.emit(TaskDialogEvent::Written),
                    Err(why) => this.refused = Some(why),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The field `step` on from the one with the keys: the id (for a new task), the title, the
    /// body, round again.
    fn step(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let mut fields = vec![&self.title, &self.body];
        if self.opened.is_none() {
            fields.insert(0, &self.id);
        }
        let at = fields
            .iter()
            .position(|input| input.focus_handle(cx).is_focused(window));
        let next = match at {
            Some(at) => (at as isize + step).rem_euclid(fields.len() as isize) as usize,
            None => 0,
        };
        window.focus(&fields[next].focus_handle(cx), cx);
        cx.notify();
    }

    /// The tasks Waits for offers that match what is typed in its search field, the task being
    /// edited left out.
    fn matches(&self, cx: &App) -> Vec<TaskChoice> {
        let query = self
            .waits
            .as_ref()
            .map(|waits| waits.search.read(cx).text().to_lowercase())
            .unwrap_or_default();
        let own = self
            .opened
            .as_ref()
            .and_then(|opened| kanban::task_id(&opened.file));
        self.tasks
            .iter()
            .filter(|task| Some(&task.id) != own.as_ref())
            .filter(|task| {
                let said = format!("{} {}", task.id, task.title).to_lowercase();
                query.split_whitespace().all(|word| said.contains(word))
            })
            .cloned()
            .collect()
    }

    /// Opens the Waits for list, its search field taking the keys and the task waited for lit; or
    /// closes it.
    fn toggle_waits(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.waits.is_some() {
            self.close_waits(window, cx);
            return;
        }
        let colors = input_colors(&self.theme);
        let search = cx.new(|cx| TextInput::new("", "Search tasks", colors, cx));
        let subscription = cx.subscribe(&search, |this, _, _: &Changed, cx| {
            if let Some(waits) = &mut this.waits {
                waits.index = 0;
                waits.scroll.scroll_to_item(0, ScrollStrategy::Top);
            }
            cx.notify();
        });
        window.focus(&search.read(cx).focus_handle(cx), cx);
        let index = self.depends.as_ref().map_or(0, |id| {
            self.matches(cx)
                .iter()
                .position(|task| task.id == *id)
                .map_or(0, |at| at + 1)
        });
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(index, ScrollStrategy::Center);
        self.waits = Some(Waits {
            search,
            index,
            scroll,
            _subscription: subscription,
        });
        cx.notify();
    }

    fn close_waits(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.waits.take().is_some() {
            window.focus(&self.title.focus_handle(cx), cx);
            cx.notify();
        }
    }

    /// ↑↓ in the open Waits for list.
    fn move_wait(&mut self, step: isize, cx: &mut Context<Self>) {
        let rows = self.matches(cx).len() + 1;
        if let Some(waits) = &mut self.waits {
            waits.index = (waits.index as isize + step).clamp(0, rows as isize - 1) as usize;
            waits
                .scroll
                .scroll_to_item(waits.index, ScrollStrategy::Nearest);
            cx.notify();
        }
    }

    /// The row at `index` of the open Waits for list picked: None, or a task to wait for.
    fn pick_wait(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.depends = match index.checked_sub(1) {
            None => None,
            Some(at) => match self.matches(cx).into_iter().nth(at) {
                Some(task) => Some(task.id),
                None => return,
            },
        };
        self.refused = None;
        self.close_waits(window, cx);
    }
}

impl Render for TaskDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let (text, bright, branch, dim, dimmer) = (
            fg(|t| t.agents_text),
            fg(|t| t.bright),
            fg(|t| t.agents_branch),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
        );
        let (rule, accent, red) = (
            fg(|t| t.agents_rule),
            fg(|t| t.agents_accent),
            fg(|t| t.agents_red),
        );
        let sunken = hsla(theme.bg(|t| t.agents_bg), 1.0);
        let viewport = window.viewport_size();
        let (room_x, room_y) = (f32::from(viewport.width), f32::from(viewport.height));
        let margin = ui.scale(DIALOG_MARGIN);
        let width = ui.scale(DIALOG_WIDTH).min(room_x - 2.0 * margin).max(0.0);
        let top = (room_y * DIALOG_TOP).round();
        let tallest = (room_y - top - margin).max(ui.scale(200.0));
        let editing = self.opened.is_some();
        let mono = self.mono.clone();

        // The heading, where the file is, and ×.
        let heading = match &self.opened {
            None => "New task".to_owned(),
            Some(opened) => match kanban::task_id(&opened.file) {
                Some(id) => format!("Edit {id}"),
                None => "Edit draft".to_owned(),
            },
        };
        let place = match &self.opened {
            None => {
                let id = self.id.read(cx).text().trim();
                let file = if kanban::task_id(id).as_deref() == Some(id) {
                    kanban::draft_file(id, self.title.read(cx).text())
                } else {
                    "\u{2026}".to_owned()
                };
                format!("{}/{file} · draft, not committed", kanban::TASKS)
            }
            Some(opened) => {
                let stands = if opened.draft { "draft" } else { "on main" };
                format!("{}/{} · {stands}", kanban::TASKS, opened.file)
            }
        };
        let close = div()
            .id("task-close")
            .flex_shrink_0()
            .size(ui.px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(8.0))
            .cursor_pointer()
            .hover(move |style| style.bg(text.opacity(0.08)))
            .child(footer_icon::icon(
                Icon::Close,
                dim,
                ui.scale(12.0 / footer_icon::SIZE),
            ))
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(TaskDialogEvent::Cancel)));
        let header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .gap(ui.px(12.0))
            .pt(ui.px(16.0))
            .pb(ui.px(10.0))
            .pl(ui.px(20.0))
            .pr(ui.px(18.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(ui.px(3.0))
                    .min_w(px(0.0))
                    .child(
                        div()
                            .text_size(ui.px(15.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(bright)
                            .child(heading),
                    )
                    .child(
                        div()
                            .font(mono.clone())
                            .text_size(ui.px(11.0))
                            .text_color(dimmer)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(place),
                    ),
            )
            .child(close);

        // A field's label over its control.
        let labelled = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(ui.px(5.0))
                .child(div().text_size(ui.px(11.0)).text_color(dim).child(label))
                .child(control)
        };
        let field = |input: &Entity<TextInput>| {
            let handle = input.focus_handle(cx);
            div()
                .h(ui.px(32.0))
                .px(ui.px(10.0))
                .flex()
                .items_center()
                .rounded(ui.px(8.0))
                .bg(sunken)
                .border_1()
                .border_color(if handle.is_focused(window) {
                    accent
                } else {
                    rule
                })
                .overflow_hidden()
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.focus(&handle, cx)
                })
                .child(input.clone())
        };
        let id = if editing {
            // Locked: the file is named by it.
            div()
                .h(ui.px(32.0))
                .px(ui.px(10.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(ui.px(8.0))
                .border_1()
                .border_color(rule)
                .font(mono.clone())
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child(self.id.read(cx).text().to_owned())
                .child(svg().data(LOCK).size(ui.px(11.0)).text_color(dim))
        } else {
            field(&self.id).font(mono.clone()).text_size(ui.px(12.5))
        };
        let names = div()
            .flex_shrink_0()
            .flex()
            .gap(ui.px(10.0))
            .pt(ui.px(4.0))
            .px(ui.px(20.0))
            .child(
                labelled("ID", id.into_any_element())
                    .w(ui.px(104.0))
                    .flex_shrink_0(),
            )
            .child(
                labelled("Title", field(&self.title).into_any_element())
                    .flex_1()
                    .min_w(px(0.0)),
            );

        // Priority, the task this one waits for, and who does it.
        let mut priorities = popover::choices(&theme, &ui).items_center().h(ui.px(32.0));
        for (n, priority) in Priority::ALL.into_iter().enumerate() {
            priorities = priorities.child(
                popover::choice(
                    &theme,
                    &ui,
                    ("task-priority", n),
                    priority.label(),
                    self.priority == priority,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.priority = priority;
                    this.refused = None;
                    cx.notify();
                })),
            );
        }
        let mut doers = popover::choices(&theme, &ui).items_center().h(ui.px(32.0));
        for (n, doer) in Doer::ALL.into_iter().enumerate() {
            doers = doers.child(
                popover::choice(
                    &theme,
                    &ui,
                    ("task-doer", n),
                    doer.label(),
                    self.doer == doer,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.doer = doer;
                    this.refused = None;
                    cx.notify();
                })),
            );
        }
        let open = self.waits.is_some();
        let waited = match &self.depends {
            None => "None".to_owned(),
            Some(id) => match self.tasks.iter().find(|task| task.id == *id) {
                Some(task) => format!("{id}  {}", task.title),
                None => id.clone(),
            },
        };
        let waits_button = div()
            .id("task-waits")
            .h(ui.px(32.0))
            .min_w(ui.px(140.0))
            .max_w(ui.px(300.0))
            .px(ui.px(10.0))
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .rounded(ui.px(8.0))
            .bg(sunken)
            .border_1()
            .border_color(if open { accent } else { rule })
            .text_size(ui.px(12.0))
            .text_color(if self.depends.is_some() { text } else { dim })
            .cursor_pointer()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(waited),
            )
            .child(footer_icon::icon(
                Icon::Down,
                dim,
                ui.scale(9.0 / footer_icon::SIZE),
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if this.dismissed.take() != Some(event.position) {
                        this.toggle_waits(window, cx);
                    }
                }),
            );
        let waits = div()
            .flex()
            .flex_col()
            .min_w(px(0.0))
            .child(waits_button)
            .when(open, |waits| {
                // Laid out right under the button, drawn above everything else.
                waits.child(
                    div().h(px(0.0)).child(deferred(
                        anchored()
                            .snap_to_window_with_margin(px(8.0))
                            .child(self.waits_list(&ui, cx)),
                    )),
                )
            });
        let choices = div()
            .flex_shrink_0()
            .flex()
            .items_end()
            .gap(ui.px(18.0))
            .pt(ui.px(14.0))
            .px(ui.px(20.0))
            .child(labelled("Priority", priorities.into_any_element()).flex_shrink_0())
            .child(labelled("Waits for", waits.into_any_element()).min_w(px(0.0)))
            .child(labelled("Done by", doers.into_any_element()).flex_shrink_0());

        // The body: Write or Preview.
        let tab = |id: &'static str, label: &'static str, on: bool, preview: bool| {
            div()
                .id(id)
                .px(ui.px(12.0))
                .pt(ui.px(6.0))
                .pb(ui.px(8.0))
                .border_b_2()
                .border_color(if on {
                    accent
                } else {
                    gpui::transparent_black()
                })
                .text_size(ui.px(12.0))
                .text_color(if on { bright } else { dim })
                .when(on, |tab| tab.font_weight(FontWeight::SEMIBOLD))
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.preview = preview;
                    cx.notify();
                }))
        };
        let tabs = div()
            .flex_shrink_0()
            .flex()
            .items_end()
            .justify_between()
            .pt(ui.px(6.0))
            .px(ui.px(8.0))
            .border_b_1()
            .border_color(rule)
            .child(
                div()
                    .flex()
                    .gap(ui.px(2.0))
                    .child(tab("task-write-tab", "Write", !self.preview, false))
                    .child(tab("task-preview-tab", "Preview", self.preview, true)),
            )
            .child(
                div()
                    .pb(ui.px(8.0))
                    .pr(ui.px(6.0))
                    .text_size(ui.px(11.0))
                    .text_color(dimmer)
                    .child(if editing {
                        "Markdown · the rest of the file, as it is"
                    } else {
                        "Markdown"
                    }),
            );
        let content = if self.preview {
            self.preview(&ui, cx).into_any_element()
        } else {
            div()
                .font(mono.clone())
                .text_size(ui.px(12.5))
                .line_height(relative(1.6))
                .text_color(branch)
                .child(self.body.clone())
                .into_any_element()
        };
        let body_focus = self.body.focus_handle(cx);
        let writing_body = !self.preview;
        let body = div()
            .flex_shrink(1.0)
            .min_h(ui.px(100.0))
            .mt(ui.px(16.0))
            .mx(ui.px(20.0))
            .flex()
            .flex_col()
            .rounded(ui.px(10.0))
            .bg(sunken)
            .border_1()
            .border_color(rule)
            .child(tabs)
            .child(
                div()
                    .id("task-body")
                    .h(ui.px(BODY_HEIGHT))
                    .flex_shrink(1.0)
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .px(ui.px(14.0))
                    .py(ui.px(12.0))
                    .when(writing_body, |body| body.cursor_text())
                    .on_click(move |_, window, cx| {
                        if writing_body {
                            window.focus(&body_focus, cx);
                        }
                    })
                    .child(content),
            );

        // What paddock writes at the head of the file.
        let head = self.show_head.then(|| self.head(cx)).flatten().map(|head| {
            div()
                .flex_shrink_0()
                .mt(ui.px(12.0))
                .mx(ui.px(20.0))
                .px(ui.px(14.0))
                .py(ui.px(10.0))
                .flex()
                .flex_col()
                .gap(ui.px(6.0))
                .rounded(ui.px(10.0))
                .border_1()
                .border_dashed()
                .border_color(rule)
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(ui.px(12.0))
                        .text_size(ui.px(11.0))
                        .text_color(dim)
                        .child("File header · written by paddock")
                        .child(
                            div()
                                .text_color(dimmer)
                                .child("edit the fields above to change it"),
                        ),
                )
                .child(
                    div()
                        .id("task-head")
                        .max_h(ui.px(140.0))
                        .overflow_y_scroll()
                        .font(mono.clone())
                        .text_size(ui.px(12.0))
                        .line_height(relative(1.6))
                        .text_color(branch)
                        .child(head),
                )
        });

        // Why it was not written, or what writing does.
        let note = match (
            self.refused.clone().or_else(|| self.unreadable()),
            &self.opened,
        ) {
            (Some(why), _) => Some((why, red)),
            (None, Some(opened)) if opened.draft => {
                Some(("Saves the draft · not committed".to_owned(), dim))
            }
            (None, Some(_)) => Some((
                "Saves and commits this file alone to main · not pushed".to_owned(),
                dim,
            )),
            (None, None) => None,
        };
        let note = note.map(|(words, color)| {
            div()
                .flex_shrink_0()
                .pt(ui.px(12.0))
                .px(ui.px(20.0))
                .text_size(ui.px(11.5))
                .text_color(color)
                .whitespace_normal()
                .child(words)
        });

        // Show file header; Cancel and Create draft or Save.
        let shown = self.show_head;
        let toggle = div()
            .id("task-head-toggle")
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .text_size(ui.px(12.0))
            .text_color(if shown { text } else { dim })
            .cursor_pointer()
            .hover(move |style| style.text_color(text))
            .child(svg().data(BRACKETS).size(ui.px(14.0)).text_color(if shown {
                text
            } else {
                dim
            }))
            .child(if shown {
                "Hide file header"
            } else {
                "Show file header"
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_head = !this.show_head;
                cx.notify();
            }));
        let cancel = div()
            .id("task-cancel")
            .h(ui.px(32.0))
            .px(ui.px(14.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_size(ui.px(12.5))
            .text_color(branch)
            .cursor_pointer()
            .hover(move |style| style.bg(text.opacity(0.08)))
            .child("Cancel")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(TaskDialogEvent::Cancel)));
        let primary = div()
            .id("task-write")
            .h(ui.px(32.0))
            .px(ui.px(16.0))
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .rounded_full()
            .bg(accent)
            .text_color(sunken)
            .text_size(ui.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .child(match (editing, self.writing) {
                (false, false) => "Create draft",
                (false, true) => "Creating\u{2026}",
                (true, false) => "Save",
                (true, true) => "Saving\u{2026}",
            })
            .child(
                div()
                    .font(mono.clone())
                    .text_size(ui.px(11.0))
                    .opacity(0.7)
                    .child("⌘↩"),
            );
        let primary = if self.writing || self.unreadable().is_some() {
            primary.opacity(0.5)
        } else {
            primary
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.9)))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.write(cx)))
        };
        let foot = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .pt(ui.px(14.0))
            .pb(ui.px(16.0))
            .pl(ui.px(20.0))
            .pr(ui.px(18.0))
            .child(toggle)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .child(cancel)
                    .child(primary),
            );

        let panel = div()
            .id("task-dialog")
            .w(px(width))
            .max_h(px(tallest))
            .flex()
            .flex_col()
            .rounded(ui.px(16.0))
            .border_1()
            .border_color(rule)
            .bg(popover::ground(&theme))
            .shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.55),
                offset: point(px(0.0), ui.px(24.0)),
                blur_radius: ui.px(60.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .text_size(ui.px(13.0))
            .text_color(text)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(header)
            .child(names)
            .child(choices)
            .child(body)
            .children(head)
            .children(note)
            .child(foot);
        // Only the panel takes clicks while it is open; a click on the dimmed window does nothing,
        // so what is typed is not lost to a stray click.
        ui.apply(div())
            .id("task-dialog-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::black().opacity(0.45))
            // Tab moves between the fields as it moves in the palette's list.
            .key_context(menu::PALETTE)
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &menu::SelectNext, window, cx| this.step(1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::SelectPrevious, window, cx| this.step(-1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::OpenSelected, window, cx| this.step(1, window, cx)),
            )
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| cx.emit(TaskDialogEvent::Cancel)))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let keys = &event.keystroke;
                let only_cmd = keys.modifiers.platform
                    && !keys.modifiers.control
                    && !keys.modifiers.alt
                    && !keys.modifiers.shift;
                if only_cmd && keys.key == "enter" {
                    cx.stop_propagation();
                    this.write(cx);
                }
            }))
            .flex()
            .flex_col()
            .items_center()
            .pt(px(top))
            .px(px(margin))
            .child(panel)
            // It dims the whole window, the Browser's page and all.
            .child(crate::browser::cover())
    }
}

impl TaskDialog {
    /// The open Waits for list: its search field, None, then the tasks that match, each with its
    /// title and where it stands.
    fn waits_list(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = &*self.theme;
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let (text, dim, rule, accent) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_rule),
            fg(|t| t.agents_accent),
        );
        let lit = popover::lit(theme);
        let Some(waits) = &self.waits else {
            return div().id("task-waits-list");
        };
        let rows = self.matches(cx).len() + 1;
        let row_height = ui.scale(28.0);
        let (index, depends, mono) = (waits.index, self.depends.clone(), self.mono.clone());
        let list = uniform_list(
            "task-waits-rows",
            rows,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                let matches = this.matches(cx);
                range
                    .map(|row| {
                        let task = row.checked_sub(1).and_then(|at| matches.get(at));
                        let picked = task.map(|task| &task.id) == depends.as_ref();
                        let line = div()
                            .id(("task-wait", row))
                            .w_full()
                            .h(px(row_height))
                            .px(px(8.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .hover(move |style| style.bg(lit.opacity(0.6)))
                            .when(row == index, |line| line.bg(lit))
                            .child(
                                div()
                                    .w(px(12.0))
                                    .flex_shrink_0()
                                    .text_color(accent)
                                    .child(if picked { "✓" } else { "" }),
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.pick_wait(row, window, cx)
                            }));
                        match task {
                            None => line.child(div().text_color(dim).child("None")),
                            Some(task) => {
                                let mut said = task.title.clone();
                                if !task.stands.is_empty() {
                                    said.push_str(" · ");
                                    said.push_str(task.stands);
                                }
                                line.child(
                                    div()
                                        .flex_shrink_0()
                                        .font(mono.clone())
                                        .text_size(px(11.5))
                                        .text_color(text)
                                        .child(task.id.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .text_color(dim)
                                        .child(said),
                                )
                            }
                        }
                    })
                    .collect()
            }),
        )
        .track_scroll(&waits.scroll)
        .h(px(row_height * rows.min(WAITS_ROWS) as f32));
        div()
            .id("task-waits-list")
            .key_context(menu::DIALOG)
            .occlude()
            .w(ui.px(320.0))
            .mt(px(4.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(6.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rule)
            .bg(popover::ground(theme))
            .shadow_lg()
            .text_size(ui.px(12.0))
            .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                this.dismissed = Some(event.position);
                this.close_waits(window, cx);
            }))
            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| this.move_wait(1, cx)))
            .on_action(cx.listener(|this, _: &menu::SelectPrevious, _, cx| this.move_wait(-1, cx)))
            .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                let index = this.waits.as_ref().map_or(0, |waits| waits.index);
                this.pick_wait(index, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_waits(window, cx)),
            )
            .child(
                div()
                    .h(ui.px(30.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(6.0))
                    .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
                    .child(footer_icon::icon(
                        Icon::Search,
                        dim,
                        ui.scale(12.0) / footer_icon::SIZE,
                    ))
                    .child(div().flex_1().min_w(px(0.0)).child(waits.search.clone())),
            )
            .child(list)
            .child(crate::browser::cover())
    }

    /// The body read as Markdown, as [`markdown::blocks`] has it.
    fn preview(&self, ui: &UiFont, cx: &App) -> Div {
        let theme = &*self.theme;
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let (bright, branch, dimmer, accent) = (
            fg(|t| t.bright),
            fg(|t| t.agents_branch),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_accent),
        );
        let code = popover::lit(theme);
        let mono = self.mono.family.clone();
        let words = |spans: &[markdown::Span]| styled(spans, &mono, code, accent);
        let blocks = markdown::blocks(self.body.read(cx).text());
        let mut out = div()
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .text_size(ui.px(13.0))
            .line_height(relative(1.55))
            .text_color(branch);
        if blocks.is_empty() {
            return out.child(div().text_color(dimmer).child("Nothing to preview"));
        }
        for (n, block) in blocks.iter().enumerate() {
            out = out.child(match block {
                markdown::Block::Heading(level, spans) => div()
                    .when(n > 0, |heading| heading.mt(ui.px(6.0)))
                    .text_size(ui.px(match level {
                        1 => 16.0,
                        2 => 14.5,
                        _ => 13.5,
                    }))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(bright)
                    .child(words(spans)),
                markdown::Block::Paragraph(spans) => div().child(words(spans)),
                markdown::Block::Item {
                    depth,
                    marker,
                    spans,
                } => {
                    let marker = match marker {
                        Some(markdown::Marker::Bullet) if *depth == 0 => "•".to_owned(),
                        Some(markdown::Marker::Bullet) => "◦".to_owned(),
                        Some(markdown::Marker::Number(n)) => format!("{n}."),
                        None => String::new(),
                    };
                    div()
                        .flex()
                        .gap(ui.px(8.0))
                        .pl(ui.px(18.0 * *depth as f32))
                        .child(
                            div()
                                .flex_shrink_0()
                                .min_w(ui.px(12.0))
                                .text_color(dimmer)
                                .child(marker),
                        )
                        .child(div().flex_1().min_w(px(0.0)).child(words(spans)))
                }
                markdown::Block::Code(text) => div()
                    .px(ui.px(10.0))
                    .py(ui.px(8.0))
                    .rounded(ui.px(6.0))
                    .bg(code)
                    .font(self.mono.clone())
                    .text_size(ui.px(12.0))
                    .child(text.clone()),
            });
        }
        out
    }
}

/// Markdown's words in their styles: bold, italic, inline code in `mono` on `code`, and links in
/// `link`, underlined.
fn styled(spans: &[markdown::Span], mono: &SharedString, code: Hsla, link: Hsla) -> StyledText {
    let mut text = String::new();
    let (mut highlights, mut fonts) = (Vec::new(), Vec::new());
    for span in spans {
        let start = text.len();
        text.push_str(&span.text);
        let range = start..text.len();
        let mut style = HighlightStyle::default();
        if span.bold {
            style.font_weight = Some(FontWeight::BOLD);
        }
        if span.italic {
            style.font_style = Some(FontStyle::Italic);
        }
        if span.link {
            style.color = Some(link);
            style.underline = Some(UnderlineStyle {
                thickness: px(1.0),
                color: Some(link),
                wavy: false,
            });
        }
        if span.code {
            style.background_color = Some(code);
            fonts.push((range.clone(), mono.clone()));
        }
        if style != HighlightStyle::default() {
            highlights.push((range, style));
        }
    }
    StyledText::new(text)
        .with_highlights(highlights)
        .with_font_family_overrides(fonts)
}

/// Takes back a question about `file` in `acts`, or why it was refused; not one under way.
/// Whether there was one.
fn take_back(acts: &mut HashMap<String, Clearing>, file: &str) -> bool {
    let shown = matches!(
        acts.get(file),
        Some(Clearing::Asking | Clearing::Refused(_))
    );
    if shown {
        acts.remove(file);
    }
    shown
}

/// How faded a card is: DONE some, one that needs the user not at all. A draft is told by its
/// fainter, dashed card instead.
fn fade(card: &Card) -> f32 {
    if card.needs_you {
        1.0
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

/// Whether a Clear or priority question is up for the user: one of `clears` asking about a card
/// still among the board's `files`, with the tab showing (`active`).
/// A question left about a card the board no longer has can be neither answered nor taken back
/// by leaving the card, so it does not count.
fn confirming(clears: &HashMap<String, Clearing>, active: bool, files: &HashSet<&str>) -> bool {
    active
        && clears
            .iter()
            .any(|(file, clearing)| *clearing == Clearing::Asking && files.contains(file.as_str()))
}

/// How many cards need the user, as the top says it.
fn need_you_words(n: usize) -> String {
    if n == 1 {
        format!("{n} needs you")
    } else {
        format!("{n} need you")
    }
}

/// A name and a branch sharing `room` on one line, as `name` and `branch` measure them: whole when
/// both fit; otherwise the branch is cut with `…` first, down to its share of the room, then the
/// name.
fn share(
    name: &str,
    branch: &str,
    room: f32,
    name_width: &dyn Fn(&str) -> f32,
    branch_width: &dyn Fn(&str) -> f32,
) -> (String, String) {
    let (whole, long) = (name_width(name), branch_width(branch));
    if whole + long <= room {
        return (name.to_owned(), branch.to_owned());
    }
    let branch_room = (room - whole).max(long.min(room * BRANCH_SHARE));
    let branch = card::elide(branch, &|cut| branch_width(cut) <= branch_room);
    let left = room - branch_width(&branch);
    (card::elide(name, &|cut| name_width(cut) <= left), branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_needs_you_and_more_need_you() {
        assert_eq!(need_you_words(1), "1 needs you");
        assert_eq!(need_you_words(2), "2 need you");
        assert_eq!(need_you_words(12), "12 need you");
    }

    #[test]
    fn a_name_and_branch_too_long_together_are_cut_with_an_ellipsis() {
        let width = |text: &str| text.chars().count() as f32;
        let (name, branch) = ("dev-kanbancards", "p5-71-kanban-cards");
        // Both fit: whole.
        assert_eq!(
            share(name, branch, 40.0, &width, &width),
            (name.to_owned(), branch.to_owned())
        );
        // The branch gives way first, the name stays whole.
        let (n, b) = share(name, branch, 28.0, &width, &width);
        assert_eq!(n, name);
        assert_eq!(b, "p5-71-kanban…");
        assert!(width(&n) + width(&b) <= 28.0);
        // Then the name too, once the branch is down to its share.
        let (n, b) = share(name, branch, 16.0, &width, &width);
        assert!(n.ends_with('…') && b.ends_with('…'), "{n:?} {b:?}");
        assert!(width(&n) + width(&b) <= 16.0, "{n:?} {b:?}");
        // Never `..`, and nothing at all left: just the ellipsis.
        let (n, b) = share(name, branch, 2.0, &width, &width);
        assert!(!n.contains("..") && !b.contains(".."));
        assert_eq!((n.as_str(), b.as_str()), ("…", "…"));
        // No branch: the name has the room.
        assert_eq!(
            share(name, "", 10.0, &width, &width),
            ("dev-kanba…".to_owned(), String::new())
        );
    }

    #[test]
    fn a_question_about_a_card_the_board_no_longer_has_is_not_up() {
        let clears = HashMap::from([("docs/任务/P1-a.md".to_owned(), Clearing::Asking)]);
        let before = HashSet::from(["docs/任务/P1-a.md", "docs/任务/P1-b.md"]);
        assert!(confirming(&clears, true, &before));
        // The task file went away while asking, and the board was read again without it.
        let after = HashSet::from(["docs/任务/P1-b.md"]);
        assert!(!confirming(&clears, true, &after));
        // Not on show: no one can answer it.
        assert!(!confirming(&clears, false, &before));
        // Being cleared, or refused: not a question.
        let running = HashMap::from([("docs/任务/P1-a.md".to_owned(), Clearing::Running)]);
        assert!(!confirming(&running, true, &before));
    }
}
