//! The right sidebar's Kanban tab: the focused pane's repository's task files as cards, in five
//! groups one above another, each folding; widened, five columns side by side. The repository's
//! name at the top opens a menu that turns the board to another repository and keeps it there,
//! the focus staying where it is (`kanban_repos`, P5-85). Git is read in the
//! background every few seconds while the tab shows (`kanban::read`), the agents come from the
//! left sidebar each frame, and the board is drawn again only when it changed. A hovered card
//! offers its task file, its agent and the agent's changes, nothing that changes the work; the
//! things it writes are a task file, new or queued, from the task dialog (New task, or Edit on a
//! queued card), and, on a card its task file says needs the user, Clear: that line taken out and
//! committed once confirmed; on a queued card, its priority line, committed the same way or, in a
//! draft, only written (DESIGN §13 P5-29, P5-60b). A draft can be deleted, its file moved to the
//! Trash, and a queued task dropped, with the wrap-up commit that says so, a draft's task file
//! committed before it (P5-86). Over Queued, the Loop (P5-106): queued tasks put in it are handed
//! to the repository's controller one after another with `corral send`, read for in the background
//! whichever repository the tab shows, or none (`kanban_loop`).
use crate::{
    activity,
    agents::Status,
    card, changes,
    corral::Client,
    fonts::UiFont,
    footer_icon::{self, Icon},
    inbox,
    kanban::{self, Board, Cache, Card, Column, Doer, Priority, Read, Seen, Tone},
    kanban_loop::{self, Delivery, Doing, Loop, Loops, Stand, Timing, Top},
    kanban_repos::{self, Looking, Row},
    kind_icon, markdown, markdown_view, menu, popover, raised,
    right_panel::Tip,
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, BoxShadow, ClickEvent, Context, Div, Entity,
    EventEmitter, FocusHandle, Focusable, Font, FontWeight, Hsla, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, PathPromptOptions, Pixels, Point, Render, ScrollStrategy,
    SharedString, Stateful, Subscription, TextRun, Transformation, UniformListScrollHandle, Window,
    WindowTextSystem, anchored, deferred, div, percentage, point, prelude::*, px, relative, svg,
    uniform_list,
};
use objc2_foundation::{NSFileManager, NSString, NSURL};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
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
/// The tasks (P5-113, rows in a box since P5-71's cards): the narrow list's side padding; a widened
/// board's side padding, the room between its columns and inside each; a row's padding, a DONE
/// row's up and down, and its corner while the mouse is on it; the room between the id and the
/// title, and between the things at a row's right end.
const LIST_X: f32 = 8.0;
const COLUMNS_X: f32 = 12.0;
const COLUMN_GAP: f32 = 10.0;
const COLUMN_X: f32 = 6.0;
const CARD_LEFT: f32 = 12.0;
const CARD_RIGHT: f32 = 10.0;
const CARD_Y: f32 = 8.0;
const DONE_Y: f32 = 6.0;
const ROW_RADIUS: f32 = 8.0;
const ID_GAP: f32 = 8.0;
const END_GAP: f32 = 6.0;
/// A group's header and Show all, set in from the narrow list's side padding: the boxes reach
/// out to 8pt from the panel's edge as the Cairn tab's does, the words stay where they were
/// (P5-116).
const HEAD_X: f32 = 8.0;
/// A group's box in the narrow list, raised as the Cairn tab's (P5-117, `raised`): its corner and
/// the room inside its edge; a row's face while the mouse is on it.
const BOX_RADIUS: f32 = 10.0;
const BOX_PAD: f32 = 2.0;
/// A widened board's column: its face, in the faint colour.
const COLUMN_FACE: f32 = 0.12;
const ROW_HOVERED: f32 = 0.04;
/// A dropped DONE row's title, on its dim colour.
const DROPPED_TITLE: f32 = 0.7;
/// Room kept from the right sidebar's own edge when reckoning the width a text is cut to.
const SPARE: f32 = 2.0;
/// The ground of the small tags and pills, and the rule between two rows, in the text colour.
const SOFT: f32 = 0.06;
const RULE: f32 = 0.06;
/// Text sizes: the id, an age, a DONE line's title, the agent line in the list and in a column,
/// the branch, and the controller's `reviewer`; the line under a title (Needs you, Waits for); a
/// group's folding arrow; the ring before a task's id and in a group's header, and the room after
/// it (P5-114).
const ID_SIZE: f32 = 12.0;
const AGE_SIZE: f32 = 11.0;
const DONE_SIZE: f32 = 12.5;
const FOOT_SIZE: f32 = 12.0;
const TILE_FOOT_SIZE: f32 = 11.5;
const BRANCH_SIZE: f32 = 11.0;
const REVIEWER_SIZE: f32 = 10.5;
const UNDER_SIZE: f32 = 12.0;
const ARROW: f32 = 9.0;
const RING: f32 = 13.0;
const RING_GAP: f32 = 6.0;
/// A group's name, and how much an empty group's ring is faded.
const GROUP_SIZE: f32 = 13.0;
const EMPTY_RING: f32 = 0.45;
/// The line of empty groups in the list (P5-185): its rings' size and the room between groups.
const ZERO_RING: f32 = 11.0;
const ZERO_GAP: f32 = 14.0;
/// A title's line height, and so the height of the id and the end beside its first line.
const TITLE_LINE: f32 = 18.0;
/// The share of the room a branch keeps, cut, before the name beside it gives way.
const BRANCH_SHARE: f32 = 0.4;

const SPIN_TRACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><circle cx="6" cy="6" r="4.5" fill="none" stroke="#000" stroke-width="2"/></svg>"##;
const SPIN_ARC: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5a4.5 4.5 0 0 1 4.5 4.5" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></svg>"##;
/// Line drawings for a card's Edit and the task dialog: a pencil, a padlock (the id of a task being
/// edited), angle brackets (the file's head).
const PENCIL: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M7.5 2.5l2 2L4 10H2V8z" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const LOCK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><rect x="2.5" y="5.5" width="7" height="5" rx="1" fill="none" stroke="#000" stroke-width="1.3"/><path d="M4 5.5V4a2 2 0 0 1 4 0v1.5" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/></svg>"##;
const BRACKETS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><path d="M5 3.5L1.5 7 5 10.5M9 3.5L12.5 7 9 10.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// Queue task: an arrow entering the queue.
const QUEUE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M1.5 6h6m-2-2 2 2-2 2M9.5 2.5v7" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// Move to draft: an arrow leaving the queue.
const MOVE_TO_DRAFT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M7.5 6h-6m2-2-2 2 2 2M9.5 2.5v7" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// For a card's Delete draft and Drop task: a bin, and a circle struck through.
const BIN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M2.5 3.5h7M5 3.5v-1h2v1M3.5 3.5l.5 6.5h4l.5-6.5" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const STRUCK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><circle cx="6" cy="6" r="4" fill="none" stroke="#000" stroke-width="1.3"/><path d="M3.2 8.8l5.6-5.6" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/></svg>"##;
/// A link, on the mark of what a queued card waits for; chevrons on Show all and Show fewer.
const LINK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M4 6l2-2M3.2 4.6l-1 1a1.6 1.6 0 002.2 2.2l1-1M6.8 5.4l1-1a1.6 1.6 0 00-2.2-2.2l-1 1" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/></svg>"##;
/// The rings before a task's id and in a group's header (P5-114): open, dashed (a draft), half and
/// three quarters filled, filled with a tick cut out, struck through.
const RING_OPEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><circle cx="7" cy="7" r="5.2" fill="none" stroke="#000" stroke-width="1.6"/></svg>"##;
const RING_DASHED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><circle cx="7" cy="7" r="5.2" fill="none" stroke="#000" stroke-width="1.5" stroke-dasharray="2.4 1.9"/></svg>"##;
const RING_HALF: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><circle cx="7" cy="7" r="5.2" fill="none" stroke="#000" stroke-width="1.6"/><path d="M7 3.6a3.4 3.4 0 0 1 0 6.8z" fill="#000"/></svg>"##;
const RING_MOST: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><circle cx="7" cy="7" r="5.2" fill="none" stroke="#000" stroke-width="1.6"/><path d="M7 3.6a3.4 3.4 0 1 1-3.4 3.4H7z" fill="#000"/></svg>"##;
const RING_TICK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><mask id="m"><rect width="14" height="14" fill="#fff"/><path d="M4.3 7.2l1.8 1.8 3.6-3.8" fill="none" stroke="#000" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></mask><circle cx="7" cy="7" r="6" fill="#000" mask="url(#m)"/></svg>"##;
const RING_STRUCK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><circle cx="7" cy="7" r="5.2" fill="none" stroke="#000" stroke-width="1.5"/><path d="M3.4 10.6l7.2-7.2" fill="none" stroke="#000" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const CHEVRON_DOWN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 3.8L5 6.3l2.5-2.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const CHEVRON_UP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 6.2L5 3.7l2.5 2.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// A pin, on a repository the board stays on or keeps in its menu; an arrow turning back, on the
/// button that follows the focused pane again (P5-85).
const PIN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M6 2.5h4l-.6 4 2.1 2.2H4.5l2.1-2.2zM8 8.7v4.8" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const TURN_BACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M6.5 4L3 7.5 6.5 11M3.5 7.5H10a3 3 0 010 6H8.5" fill="none" stroke="#000" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
/// The menu under the repository's name: how wide.
const MENU_WIDTH: f32 = 276.0;
/// How often a repository whose loop is on is read for it, whichever the tab shows; and how long
/// after paddock opens the first is, so that nothing is handed over before the agents are read.
const LOOP_EVERY: Duration = Duration::from_secs(5);
/// Two arrows round a circle: the Loop's group and Add to loop.
const LOOP_ARROWS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M2.4 6a3.6 3.6 0 0 1 6.2-2.5M9.6 6a3.6 3.6 0 0 1-6.2 2.5" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round"/><path d="M8.9 1.5v2.3H6.6M3.1 10.5V8.2h2.3" fill="none" stroke="#000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// What the window tells the tab every time it draws.
pub struct Frame {
    /// The focused pane's directory: `None` with no pane in focus, `Some(None)` for a pane with
    /// no directory to read.
    pub cwd: Option<Option<String>>,
    /// The open shells' directories, sorted and without repeats.
    pub shells: Vec<String>,
    /// The right sidebar is open on this tab: read, and keep reading.
    pub active: bool,
    /// The sidebar's width, in points.
    pub width: f32,
    pub theme: Rc<Theme>,
    pub mono: Font,
    /// The agents as the left sidebar lists them.
    pub agents: Vec<Seen>,
    /// The `corral` command, which the Loop hands tasks over with.
    pub corral: String,
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
    /// What the Loop tells the Inbox changed ([`KanbanView::loop_notices`]).
    Loop,
}

impl EventEmitter<KanbanEvent> for KanbanView {}

struct Pending {
    cwd: String,
    cancel: Arc<AtomicBool>,
    out: Arc<Mutex<Option<Read>>>,
}

/// Where a card's Clear, a change of its priority, its draft's deletion or its task's dropping
/// stands.
#[derive(Clone, Debug, PartialEq)]
enum Clearing {
    /// Asking to confirm, which priority, or why it is dropped, while the mouse stays on the card.
    Asking,
    Running,
    /// Committed, a draft written, or its file in the Trash: until the board is read again.
    Done,
    /// Nothing was changed, and why; until the mouse leaves the card.
    Refused(String),
}

/// The field for why a task is dropped, under the card asked about.
struct Reason {
    file: String,
    input: Entity<TextInput>,
    /// What had the keys before it took them, to give them back to.
    back: Option<FocusHandle>,
}

/// The open menu under the repository's name.
struct RepoMenu {
    /// What had the keys before it took them, to give them back to.
    back: Option<FocusHandle>,
    /// Why the folder just chosen was not taken.
    note: Option<SharedString>,
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
    accent: Hsla,
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
            accent: fg(|t| t.agents_accent),
        }
    }
}

pub struct KanbanView {
    theme: Rc<Theme>,
    colors: Colors,
    mono: Font,
    /// The focused pane's directory, as the window gives it.
    focus: Option<Option<String>>,
    /// The directory the board reads: the focused pane's, or the repository it stays on.
    cwd: Option<Option<String>>,
    /// Which repository that is, and the ones kept in the menu.
    looking: Looking,
    /// The open shells' directories.
    shells: Vec<String>,
    /// The main worktree of the repository each directory asked about is in, `None` for one in
    /// none: the focused pane's, the agents' and the shells', read in the background.
    mains: HashMap<String, Option<PathBuf>>,
    /// Some are being read.
    resolving: bool,
    menu: Option<RepoMenu>,
    /// Has the keys while the menu is open, so that Esc closes it.
    menu_focus: FocusHandle,
    /// Where the press that closed the menu was: on its own button it does not open it again.
    dismissed: Option<Point<Pixels>>,
    _blur: Subscription,
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
    /// The drafts being deleted or asked to be, by task file.
    deletes: HashMap<String, Clearing>,
    /// The cards being dropped or asked to be, by task file.
    drops: HashMap<String, Clearing>,
    /// The drafts being queued or asked to be, by task file.
    queues: HashMap<String, Clearing>,
    /// Queued tasks being moved back to drafts, by task file.
    moves: HashMap<String, Clearing>,
    /// The field for why, while a card is asked whether to drop it.
    reason: Option<Reason>,
    /// The `corral` command, as the window gives it.
    corral: String,
    /// Every repository's Loop, as kept.
    loops: Loops,
    /// When the tab was made: no loop is read for before [`LOOP_EVERY`] after.
    began: Instant,
    /// Each repository whose loop is on, read for it.
    loop_reads: HashMap<PathBuf, LoopRead>,
    /// When a task the controller turned away is handed over again, by repository.
    loop_holds: HashMap<PathBuf, f64>,
    /// The repositories a task or a nudge is on its way to the controller of.
    loop_sending: HashSet<PathBuf>,
    /// Where each loop that is on stands, as last read: its repository's name, and how many wait.
    loop_stands: HashMap<PathBuf, (String, Stand, usize)>,
    /// What the loops have to tell that is only to be read, until it is.
    loop_notes: Vec<inbox::Notice>,
    /// What the Inbox was last told.
    loop_told: Vec<inbox::Notice>,
}

/// A read on its way: what stops it, and where it puts what it read.
type LoopPending = (Arc<AtomicBool>, Arc<Mutex<Option<Read>>>);

/// A repository read for its loop: the read on its way, and when the last came back.
#[derive(Default)]
struct LoopRead {
    pending: Option<LoopPending>,
    last: Option<Instant>,
}

/// Where a card stands in its repository's Loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// Queued in the LOOP group, at this place from 1.
    At(usize),
    /// Handed to the controller, not begun: Now, on top of the group.
    Sent,
    /// Under way, in its own column: marked Loop.
    Under,
}

/// The task handed over, as the Loop's box draws it on top (P5-158).
struct Now {
    /// It has begun: its agent's line goes under its title.
    agent: bool,
    /// It waits on the user, or is stuck: its mark is amber.
    needs: bool,
    /// What goes under its title otherwise.
    rows: Vec<AnyElement>,
}

impl KanbanView {
    pub fn new(
        theme: Rc<Theme>,
        mono: Font,
        folded: Vec<Column>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |view, cx| view.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let menu_focus = cx.focus_handle();
        // The keys went elsewhere, the window to the back or the tab out of sight: the menu goes.
        let blur = cx.on_blur(&menu_focus, window, |view, window, cx| {
            view.close_menu(window, cx)
        });
        KanbanView {
            colors: Colors::of(&theme),
            theme,
            mono,
            focus: None,
            cwd: None,
            looking: Looking::load(kanban_repos::kept_path()),
            shells: Vec::new(),
            mains: HashMap::new(),
            resolving: false,
            menu: None,
            menu_focus,
            dismissed: None,
            _blur: blur,
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
            deletes: HashMap::new(),
            drops: HashMap::new(),
            queues: HashMap::new(),
            moves: HashMap::new(),
            reason: None,
            corral: "corral".into(),
            loops: Loops::load(kanban_loop::kept_path()),
            began: Instant::now(),
            loop_reads: HashMap::new(),
            loop_holds: HashMap::new(),
            loop_sending: HashSet::new(),
            loop_stands: HashMap::new(),
            loop_notes: Vec::new(),
            loop_told: Vec::new(),
        }
    }

    /// The window's latest: redraws only when something here changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        if frame.cwd != self.focus {
            self.focus = frame.cwd;
            self.look();
            changed = true;
        }
        if frame.shells != self.shells {
            self.shells = frame.shells;
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
        self.corral = frame.corral;
        if frame.agents != self.agents {
            self.agents = frame.agents;
            // The open menu lists their repositories.
            changed |= self.rebuild() || self.menu.is_some();
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

    /// Turns to the directory the board now reads, the focused pane's or the repository it stays
    /// on, and reads it at once: whether it is another.
    fn look(&mut self) -> bool {
        let cwd = self.looking.dir(&self.focus);
        if cwd == self.cwd {
            return false;
        }
        self.cancel();
        self.last = None;
        self.cwd = cwd;
        true
    }

    /// The menu turned the board: nothing of the repository before stays up under the new name,
    /// where a card's buttons would still act on the old one.
    fn turn(&mut self, cx: &mut Context<Self>) {
        if self.look() {
            self.read = None;
            self.board = None;
        }
        cx.notify();
    }

    /// The focused pane's repository, once read.
    fn focus_repo(&self) -> Option<&PathBuf> {
        let cwd = self.focus.as_ref()?.as_ref()?;
        self.mains.get(cwd)?.as_ref()
    }

    /// The repositories of the agents and the open shells, as far as they are read: one for each,
    /// so with repeats.
    fn open_repos(&self) -> Vec<PathBuf> {
        self.agents
            .iter()
            .filter_map(|agent| agent.cwd.as_ref())
            .chain(&self.shells)
            .filter_map(|cwd| self.mains.get(cwd)?.clone())
            .collect()
    }

    /// Reads, in the background, which repository each directory not asked about yet is in: the
    /// focused pane's, the agents' and the open shells' (as the left sidebar's groups are read,
    /// P5-80).
    fn resolve(&mut self, cx: &mut Context<Self>) {
        if self.resolving {
            return;
        }
        let mut dirs: Vec<String> = self
            .focus
            .iter()
            .flatten()
            .chain(self.agents.iter().filter_map(|agent| agent.cwd.as_ref()))
            .chain(&self.shells)
            .filter(|cwd| !self.mains.contains_key(*cwd))
            .cloned()
            .collect();
        if dirs.is_empty() {
            return;
        }
        dirs.sort();
        dirs.dedup();
        self.resolving = true;
        let task = cx.background_spawn(async move {
            let cancel = AtomicBool::new(false);
            dirs.into_iter()
                .map(|cwd| {
                    let main = activity::main_repository("git", &cwd, &cancel);
                    (cwd, main)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |view, cx| {
                view.resolving = false;
                view.mains.extend(found);
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the menu under the repository's name, saying `note` in it. It takes the keys, for
    /// Esc, and gives them back as it closes.
    fn open_menu(
        &mut self,
        note: Option<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Another window may have changed the list, and a directory may have become a repository.
        self.looking.refresh();
        self.mains.retain(|_, main| main.is_some());
        self.menu = Some(RepoMenu {
            back: window.focused(cx),
            note,
        });
        window.focus(&self.menu_focus, cx);
        cx.notify();
    }

    fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        if self.menu_focus.is_focused(window) {
            match &menu.back {
                Some(back) => window.focus(back, cx),
                None => window.blur(cx),
            }
        }
        cx.notify();
    }

    /// A repository picked in the menu: the board stays on it, and it is kept.
    fn choose(&mut self, repo: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.looking.choose(repo);
        self.close_menu(window, cx);
        self.turn(cx);
    }

    /// `Follow focused pane`, and the button back: the board follows the focused pane again.
    fn follow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.looking.follow();
        self.close_menu(window, cx);
        self.turn(cx);
    }

    /// × on a kept repository: off the list, the menu staying open; the board follows the focused
    /// pane again when it stayed on that one.
    fn remove(&mut self, repo: &Path, cx: &mut Context<Self>) {
        self.looking.remove(repo);
        self.turn(cx);
    }

    /// `Choose folder…`: the system's folder chooser. The repository the folder is in is kept and
    /// shown; a folder in none is not, and the menu comes back saying so.
    fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_menu(window, cx);
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(folder) = paths.into_iter().next() else {
                return;
            };
            let dir = folder.to_string_lossy().into_owned();
            let repo = cx
                .background_executor()
                .spawn(
                    async move { activity::main_repository("git", &dir, &AtomicBool::new(false)) },
                )
                .await;
            let _ = this.update_in(cx, |view, window, cx| match repo {
                Some(repo) => view.choose(repo, window, cx),
                None => {
                    let name = kanban_repos::name(&folder);
                    let note = format!("{name} isn't in a Git repository");
                    view.open_menu(Some(note.into()), window, cx);
                }
            });
        })
        .detach();
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
                let mut settled = false;
                for acts in self.acts() {
                    let before = acts.len();
                    acts.retain(|_, clearing| *clearing != Clearing::Done);
                    settled |= acts.len() != before;
                }
                if settled {
                    cx.notify();
                }
                // A failed round keeps what is shown, unless there is nothing yet.
                let current = self.cwd.as_ref().and_then(Option::as_ref) == Some(&cwd);
                if current && (read != Read::Failed || self.read.is_none()) && self.accept(read) {
                    cx.notify();
                }
            }
        }
        // The loops go on whether the tab shows or not.
        self.tick_loops(cx);
        if !self.active {
            return;
        }
        self.resolve(cx);
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
        // Its loop as Git now has it, whether on or not.
        if let Some(board) = &board {
            self.settle_loop(board);
        }
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

    /// What the cards are asked or having done, each kind by task file.
    fn acts(&mut self) -> [&mut HashMap<String, Clearing>; 6] {
        [
            &mut self.clears,
            &mut self.priorities,
            &mut self.deletes,
            &mut self.drops,
            &mut self.queues,
            &mut self.moves,
        ]
    }

    /// Takes back whatever is asked about `file`'s card, or why it was refused, the field for a
    /// reason with it; not what is under way. Whether there was anything.
    fn take_back(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mut shown = false;
        for acts in self.acts() {
            shown |= take_back(acts, file);
        }
        if self.reason.as_ref().is_some_and(|r| r.file == file) {
            self.close_reason(window, cx);
        }
        shown
    }

    /// The field for a reason goes, the keys back to what had them before it.
    fn close_reason(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reason) = self.reason.take() else {
            return;
        };
        if reason.input.read(cx).focus_handle(cx).is_focused(window) {
            match &reason.back {
                Some(back) => window.focus(back, cx),
                None => window.blur(cx),
            }
        }
    }

    /// Whether a reason is typed in the field under `file`'s card.
    fn typed(&self, file: &str, cx: &App) -> bool {
        self.reason
            .as_ref()
            .is_some_and(|r| r.file == file && !r.input.read(cx).text().is_empty())
    }

    /// The card of the task file `file`, when the board has it to drop.
    fn to_drop(&self, file: &str) -> Option<&Card> {
        let board = self.board.as_ref()?;
        let mut cards = board.columns.iter().flatten();
        cards.find(|card| card.file == file && board.droppable(card))
    }

    fn hover(&mut self, file: &str, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        // Leaving takes back the question, or why it was not cleared, changed, deleted or dropped;
        // but a reason typed for dropping stays, with its question, until Cancel or Esc.
        if !hovered && !self.typed(file, cx) && self.take_back(file, window, cx) {
            cx.notify();
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
    fn ask_clear(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.clears.get(file),
            Some(Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        self.take_back(file, window, cx);
        self.clears.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// The priority button on a card: ask which, unless it is being changed already.
    fn ask_priority(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.priorities.get(file),
            Some(Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        self.take_back(file, window, cx);
        self.priorities.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// Delete draft on a card: ask first, unless it is being deleted already.
    fn ask_delete(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.deletes.get(file),
            Some(Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        self.take_back(file, window, cx);
        self.deletes.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// Drop task on a card: ask first, with a field for why that takes the keys, unless it is
    /// asked already, a reason typed perhaps, or being dropped.
    fn ask_drop(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.drops.get(file),
            Some(Clearing::Asking | Clearing::Running | Clearing::Done)
        ) {
            return;
        }
        // There is one field: a question still up on another card goes with it.
        if let Some(other) = self.reason.as_ref().map(|reason| reason.file.clone()) {
            self.take_back(&other, window, cx);
        }
        self.take_back(file, window, cx);
        let colors = input_colors(&self.theme);
        let input = cx.new(|cx| TextInput::new("", "Reason (optional)", colors, cx));
        let back = window.focused(cx);
        window.focus(&input.read(cx).focus_handle(cx), cx);
        self.reason = Some(Reason {
            file: file.to_owned(),
            input,
            back,
        });
        self.drops.insert(file.to_owned(), Clearing::Asking);
        cx.notify();
    }

    /// Confirmed: untrack the task alone, then refresh its state from Git.
    fn move_task_to_draft(&mut self, file: &str, cx: &mut Context<Self>) {
        if self.moves.get(file) != Some(&Clearing::Asking)
            || self.to_drop(file).is_none_or(|card| card.draft)
        {
            return;
        }
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        let file = file.to_owned();
        self.moves.insert(file.clone(), Clearing::Running);
        let name = file.clone();
        let task = cx.background_spawn(async move {
            kanban::move_to_draft("git", &repo, &name, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.moves.insert(file, Clearing::Done);
                        view.cancel();
                        view.last = None;
                    }
                    Err(why) => {
                        view.moves.insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Confirmed: commit the draft alone, then refresh its state from Git.
    fn queue_task(&mut self, file: &str, cx: &mut Context<Self>) {
        if self.queues.get(file) != Some(&Clearing::Asking)
            || !self.to_drop(file).is_some_and(|card| card.draft)
        {
            return;
        }
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        let file = file.to_owned();
        self.queues.insert(file.clone(), Clearing::Running);
        let name = file.clone();
        let task = cx.background_spawn(async move {
            kanban::queue_draft("git", &repo, &name, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.queues.insert(file, Clearing::Done);
                        view.cancel();
                        view.last = None;
                    }
                    Err(why) => {
                        view.queues.insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Confirmed: the draft's file moved to the Trash in the background, and the board read again
    /// once it is.
    fn delete(&mut self, file: &str, cx: &mut Context<Self>) {
        if self.deletes.get(file) != Some(&Clearing::Asking) {
            return;
        }
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        self.deletes.insert(file.to_owned(), Clearing::Running);
        let name = file.to_owned();
        let task = cx.background_spawn(async move {
            kanban::delete_draft("git", &repo, &name, &AtomicBool::new(false), trash)
        });
        let file = file.to_owned();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.deletes.insert(file, Clearing::Done);
                        // A read begun before would bring the card back for a while.
                        view.cancel();
                        view.last = None;
                    }
                    Err(why) => {
                        view.deletes
                            .insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Confirmed, with the reason in the field: the wrap-up commit made in the background, a
    /// draft's task file committed before it, and the board read again once it is. Not for a card
    /// the board, as it is now, no longer offers it on.
    fn drop_task(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.drops.get(file) != Some(&Clearing::Asking) {
            return;
        }
        let (Some(draft), Some(board)) = (self.to_drop(file).map(|card| card.draft), &self.board)
        else {
            return;
        };
        let repo = board.repo.clone();
        let why = match &self.reason {
            Some(reason) if reason.file == file => reason.input.read(cx).text().to_owned(),
            _ => String::new(),
        };
        self.close_reason(window, cx);
        let file = file.to_owned();
        self.drops.insert(file.clone(), Clearing::Running);
        let name = file.clone();
        let task = cx.background_spawn(async move {
            let cancel = AtomicBool::new(false);
            if draft {
                kanban::drop_draft("git", &repo, &name, &why, &cancel)
            } else {
                kanban::drop_task("git", &repo, &name, &why, &cancel)
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                // A read begun before a commit would keep the card as it was for a while: queued,
                // or a draft when only its own commit was made.
                if matches!(result, Ok(()) | Err(kanban::NotChanged::Half(_))) {
                    view.cancel();
                    view.last = None;
                }
                match result {
                    Ok(()) => {
                        view.drops.insert(file, Clearing::Done);
                    }
                    Err(why) => {
                        view.drops.insert(file, Clearing::Refused(why.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
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

    /// The board's loop settled against it, kept when that changed; a finish told to the Inbox.
    fn settle_loop(&mut self, board: &Board) {
        let mut lp = self.loops.get(&board.repo);
        let settled = kanban_loop::settle(&mut lp, board);
        if settled.changed {
            self.loops.put(&board.repo, lp);
        }
        if let Some(done) = settled.finished {
            let finished = Stand::Off {
                tasks: 0,
                finished: Some(done),
            };
            let text = finished.words(0, &Timing::DEFAULT);
            self.loop_note(&board.repo, &board.name, text);
        }
    }

    /// Something only to be read, for the Inbox.
    fn loop_note(&mut self, repo: &Path, name: &str, text: String) {
        let notice = inbox::Notice {
            repo: repo.to_path_buf(),
            name: name.to_owned(),
            text,
            needs: false,
        };
        if !self.loop_notes.contains(&notice) {
            self.loop_notes.push(notice);
        }
    }

    /// Reads each repository whose loop is on, every [`LOOP_EVERY`] in the background, and drives
    /// its loop with what comes back; the Inbox is told when what the loops say changes.
    fn tick_loops(&mut self, cx: &mut Context<Self>) {
        let on = self.loops.on();
        self.loop_reads.retain(|repo, read| {
            let keep = on.contains(repo);
            if !keep && let Some((cancel, _)) = &read.pending {
                cancel.store(true, Ordering::Relaxed);
            }
            keep
        });
        self.loop_stands.retain(|repo, _| on.contains(repo));
        let mut boards = Vec::new();
        if self.began.elapsed() >= LOOP_EVERY {
            for repo in on {
                let read = self.loop_reads.entry(repo.clone()).or_default();
                if let Some((_, out)) = &read.pending
                    && let Some(done) = out.lock().ok().and_then(|mut out| out.take())
                {
                    read.pending = None;
                    read.last = Some(Instant::now());
                    if let Read::Board(facts) = done {
                        boards.push((repo.clone(), facts));
                    }
                }
                if read.pending.is_some() || read.last.is_some_and(|l| l.elapsed() < LOOP_EVERY) {
                    continue;
                }
                let cancel = Arc::new(AtomicBool::new(false));
                let out = Arc::new(Mutex::new(None));
                let (stop, put, cache) = (cancel.clone(), out.clone(), self.cache.clone());
                let dir = repo.to_string_lossy().into_owned();
                let started = thread::Builder::new()
                    .name("paddock-kanban-loop".into())
                    .spawn(move || {
                        let read = kanban::read("git", &dir, &cache, &stop);
                        if let Ok(mut out) = put.lock() {
                            *out = Some(read);
                        }
                    });
                if started.is_ok() {
                    read.pending = Some((cancel, out));
                }
            }
        }
        for (repo, facts) in boards {
            let board = kanban::board(&facts, &self.agents, now());
            self.drive(&repo, &board, cx);
        }
        let told = self.loop_notices();
        if told != self.loop_told {
            self.loop_told = told;
            cx.emit(KanbanEvent::Loop);
        }
    }

    /// One turn of `repo`'s loop on `board` read for it: settled, where it stands kept, and what
    /// it says to hand over handed over.
    fn drive(&mut self, repo: &Path, board: &Board, cx: &mut Context<Self>) {
        self.settle_loop(board);
        let lp = self.loops.get(repo);
        let stand = self.stand(repo, &lp, board);
        if let Stand::Send {
            id,
            title,
            file,
            to,
        } = &stand
        {
            let words = kanban_loop::start_words(id, title, file);
            self.hand_over(repo, id, words, to, false, cx);
        }
        if lp.on {
            let next = lp.tasks.len();
            self.loop_stands
                .insert(repo.to_path_buf(), (board.name.clone(), stand, next));
        } else {
            self.loop_stands.remove(repo);
        }
        cx.notify();
    }

    /// Where `repo`'s loop `lp` stands on `board`, now.
    fn stand(&self, repo: &Path, lp: &Loop, board: &Board) -> Stand {
        kanban_loop::look(
            lp,
            board,
            &self.agents,
            now(),
            &Timing::DEFAULT,
            self.loop_holds.get(repo).copied(),
            self.loop_sending.contains(repo),
        )
    }

    /// Hands `words` about task `id` to the controller `to` in the background: its start, or
    /// (`nudge`) a nudge. Taken, it is kept as handed over (or nudged); turned away while the
    /// controller works or someone types in its pane, a start is tried again after a while;
    /// anything else waits for Retry.
    fn hand_over(
        &mut self,
        repo: &Path,
        id: &str,
        words: String,
        to: &str,
        nudge: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.loop_sending.insert(repo.to_path_buf()) {
            return;
        }
        let client = Client::local(self.corral.clone());
        let to = to.to_owned();
        let task = cx.background_spawn(async move {
            kanban_loop::deliver(&client, &to, &words, &AtomicBool::new(false))
        });
        let (repo, id, at) = (repo.to_path_buf(), id.to_owned(), now());
        cx.spawn(async move |this, cx| {
            let delivery = task.await;
            let _ = this.update(cx, |view, cx| {
                view.handed(&repo, id, nudge, at, delivery);
                cx.notify();
            });
        })
        .detach();
    }

    /// How handing over `id` at `at` went, kept in `repo`'s loop; the loop read again soon.
    fn handed(&mut self, repo: &Path, id: String, nudge: bool, at: f64, delivery: Delivery) {
        self.loop_sending.remove(repo);
        let mut lp = self.loops.get(repo);
        match delivery {
            Delivery::Taken { merged } => {
                self.loop_holds.remove(repo);
                if nudge {
                    if let Some(sent) = lp.sent.as_mut().filter(|sent| sent.id == id) {
                        sent.nudged = Some(at);
                    }
                } else {
                    lp.tasks.retain(|task| *task != id);
                    lp.sent = Some(kanban_loop::Sent {
                        id: id.clone(),
                        at,
                        nudged: None,
                    });
                }
                lp.failed = None;
                if merged {
                    let name = kanban_repos::name(repo);
                    let text = format!(
                        "Sent {id}; text already typed in the controller's box went in with it"
                    );
                    self.loop_note(repo, &name, text);
                }
            }
            // A controller at work is not stuck: a nudge it turns away is let be.
            Delivery::Later(_) if nudge => {}
            Delivery::Later(_) => {
                self.loop_holds
                    .insert(repo.to_path_buf(), now() + kanban_loop::RETRY);
            }
            Delivery::Failed(why) => lp.failed = Some(kanban_loop::Failed { id, nudge, why }),
        }
        self.loops.put(repo, lp);
        self.kick(repo);
    }

    /// `repo`'s loop read for again at once, and the board too when it shows that repository.
    fn kick(&mut self, repo: &Path) {
        if let Some(read) = self.loop_reads.get_mut(repo) {
            read.last = None;
        }
        if self.board.as_ref().is_some_and(|board| board.repo == repo) {
            self.read_again();
        }
    }

    /// The board's loop changed by `edit`, kept, and driven again at once when it changed.
    fn edit_loop(&mut self, edit: impl FnOnce(&mut Loop) -> bool, cx: &mut Context<Self>) {
        let Some(repo) = self.board.as_ref().map(|board| board.repo.clone()) else {
            return;
        };
        let mut lp = self.loops.get(&repo);
        if !edit(&mut lp) {
            return;
        }
        self.loops.put(&repo, lp);
        self.kick(&repo);
        cx.notify();
    }

    /// Retry: a start that failed is tried again on the loop's next turn, a nudge at once.
    fn loop_retry(&mut self, cx: &mut Context<Self>) {
        let Some(board) = self.board.clone() else {
            return;
        };
        let lp = self.loops.get(&board.repo);
        match lp.failed {
            Some(failed) if failed.nudge => self.nudge(cx),
            Some(_) => self.edit_loop(|lp| lp.failed.take().is_some(), cx),
            None => {}
        }
    }

    /// Nudge on a stuck task: one more line to the controller, to go on with it or say where it
    /// is stuck.
    fn nudge(&mut self, cx: &mut Context<Self>) {
        let Some(board) = self.board.clone() else {
            return;
        };
        let mut lp = self.loops.get(&board.repo);
        let controllers = kanban::controllers_for(&board.repo, &self.agents);
        let (Some(sent), [controller]) = (lp.sent.as_ref(), controllers.as_slice()) else {
            return;
        };
        let (id, to) = (sent.id.clone(), controller.name.clone());
        lp.failed = None;
        self.loops.put(&board.repo, lp);
        self.hand_over(
            &board.repo,
            &id,
            kanban_loop::nudge_words(&id),
            &to,
            true,
            cx,
        );
        cx.notify();
    }

    /// What the loops tell the Inbox: each that is on and waits on the user, by its status line,
    /// and what is only to be read.
    pub fn loop_notices(&self) -> Vec<inbox::Notice> {
        let mut needs: Vec<inbox::Notice> = self
            .loop_stands
            .iter()
            .filter(|(_, (_, stand, _))| stand.needs_you())
            .map(|(repo, (name, stand, next))| inbox::Notice {
                repo: repo.clone(),
                name: name.clone(),
                text: stand.words(*next, &Timing::DEFAULT),
                needs: true,
            })
            .collect();
        needs.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.repo.cmp(&b.repo)));
        needs.extend(self.loop_notes.iter().cloned());
        needs
    }

    /// The Inbox showed `notice`: one only to be read is read.
    pub fn read_notice(&mut self, notice: &inbox::Notice) {
        if !notice.needs {
            self.loop_notes.retain(|note| note != notice);
        }
    }

    /// The board turned to `repo` and kept there, as when picked in the menu: from the Inbox.
    pub fn show_repo(&mut self, repo: PathBuf, cx: &mut Context<Self>) {
        self.looking.choose(repo);
        self.turn(cx);
    }

    /// Where `card` stands in the board's loop: queued in it, at which place or handed over, or
    /// under way.
    fn slot(&self, card: &Card) -> Option<Slot> {
        let board = self.board.as_ref()?;
        if card.draft || card.id.is_empty() || card.column == Column::Done {
            return None;
        }
        let lp = self.loops.get(&board.repo);
        if !lp.has(&card.id) {
            return None;
        }
        if card.column != Column::Queued {
            return Some(Slot::Under);
        }
        if lp.sent.as_ref().is_some_and(|sent| sent.id == card.id) {
            return Some(Slot::Sent);
        }
        let queued = |id: &&String| {
            board
                .cards(Column::Queued)
                .iter()
                .any(|c| c.id == **id && !c.draft)
        };
        let at = lp
            .tasks
            .iter()
            .filter(queued)
            .position(|id| *id == card.id)?;
        Some(Slot::At(at + 1))
    }

    /// The queued cards in the board's loop, as its group lists them: the one handed over first,
    /// then the others in their order.
    fn in_loop<'a>(&self, board: &'a Board) -> Vec<&'a Card> {
        let lp = self.loops.get(&board.repo);
        let queued = |id: &str| {
            board
                .cards(Column::Queued)
                .iter()
                .find(|card| card.id == id && !card.draft)
        };
        lp.sent
            .iter()
            .map(|sent| sent.id.as_str())
            .chain(lp.tasks.iter().map(String::as_str))
            .filter_map(queued)
            .collect()
    }

    /// The task `lp` handed over, in whichever column it is until it is wrapped up.
    fn now_card<'a>(&self, board: &'a Board, lp: &Loop) -> Option<&'a Card> {
        let sent = lp.sent.as_ref()?;
        board
            .columns
            .iter()
            .flatten()
            .find(|card| card.id == sent.id && !card.draft && card.column != Column::Done)
    }

    /// The queued cards in `lp` not handed over, in their order: the Loop's Next.
    fn next_cards<'a>(&self, board: &'a Board, lp: &Loop) -> Vec<&'a Card> {
        lp.tasks
            .iter()
            .filter_map(|id| {
                board
                    .cards(Column::Queued)
                    .iter()
                    .find(|card| card.id == *id && !card.draft)
            })
            .collect()
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

    /// In the interface font made semibold.
    fn bold(&self, line: &str, size: f32) -> f32 {
        let font = Font {
            weight: FontWeight::SEMIBOLD,
            ..gpui::font(self.family.clone())
        };
        self.width(line, size, font)
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
        // A reason typed keeps its question up with the mouse elsewhere: it goes, the keys back
        // where they were, once the board no longer has the card to drop.
        if let Some(file) = self.reason.as_ref().map(|reason| reason.file.clone())
            && self.to_drop(&file).is_none()
        {
            cx.defer_in(window, move |view, window, cx| {
                if view.to_drop(&file).is_none() && view.take_back(&file, window, cx) {
                    cx.notify();
                }
            });
        }
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
        // With nothing to show, the repository's button is there all the same: its menu turns
        // the board to a kept repository with no pane in focus too.
        let Some(cwd) = &self.cwd else {
            return root.child(self.top(None, &ui, cx)).child(empty(
                "No pane in focus",
                "Kanban follows the focused pane's repository",
            ));
        };
        let message = match (cwd, &self.read) {
            (None, _) | (_, Some(Read::NotRepository)) => {
                Some(if self.looking.chosen().is_some() {
                    (
                        "Not a repository",
                        "This folder is no longer a Git repository",
                    )
                } else {
                    (
                        "Not in a repository",
                        "Kanban shows the task files of the focused pane's repository",
                    )
                })
            }
            (_, Some(Read::NoMain { .. })) => {
                Some(("No main branch", "Task files are read from main"))
            }
            (_, Some(Read::NoTasks { .. })) => {
                let sub = "Task files go in docs/任务 on main. New task writes a draft to \
                           docs/任务 in the main worktree, making the folder if it's missing.";
                return root
                    .child(self.top(None, &ui, cx))
                    .child(empty("No task files", sub).child(self.first_task(&ui, cx)));
            }
            (_, Some(Read::Failed)) => Some(("Git couldn't be read", "Trying again shortly")),
            _ => None,
        };
        if let Some((title, sub)) = message {
            return root.child(self.top(None, &ui, cx)).child(empty(title, sub));
        }
        let Some(board) = self.board.clone() else {
            // Still reading for the first time.
            return root.child(self.top(None, &ui, cx));
        };
        let wide = self.width >= self.wide_at;
        root.child(self.top(Some(&board), &ui, cx))
            .child(div().flex_shrink_0().h(px(1.0)).bg(c.text.opacity(RULE)))
            .child(if wide {
                self.columns(&board, &fit, &ui, cx).into_any_element()
            } else {
                self.list(&board, &fit, &ui, cx).into_any_element()
            })
    }
}

impl KanbanView {
    /// A card is asking whether to clear its task, which priority it gets, or whether to delete
    /// or drop it.
    pub fn confirming(&self) -> bool {
        let files: HashSet<&str> = self
            .board
            .iter()
            .flat_map(|board| board.columns.iter().flatten())
            .map(|card| card.file.as_str())
            .collect();
        [
            &self.clears,
            &self.priorities,
            &self.deletes,
            &self.drops,
            &self.queues,
            &self.moves,
        ]
        .into_iter()
        .any(|acts| confirming(acts, self.active, &files))
    }

    /// The repository's button and, over a board, main, how many need the user, are in progress
    /// and to review, and New task. While the board stays on a picked repository, the button back
    /// to the focused pane takes the numbers' place.
    fn top(&self, board: Option<&Board>, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let mut top = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .pt(ui.px(2.0))
            .pb(ui.px(10.0))
            .pl(ui.px(10.0))
            .pr(ui.px(14.0))
            .child(self.repo_button(ui, cx));
        if board.is_some() {
            top = top.child(
                pill(ui)
                    .px(ui.px(7.0))
                    .bg(c.text.opacity(SOFT))
                    .font(self.mono.clone())
                    .text_size(ui.px(11.0))
                    .text_color(c.muted)
                    .child(kanban::MAIN),
            );
        }
        top = top.child(div().flex_1());
        if let Some(chosen) = self.looking.chosen() {
            top = top.child(self.back(chosen, ui, cx));
        } else if let Some(board) = board {
            top = top.child(self.summary(board, ui));
        }
        if board.is_none() {
            return top;
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
        top.child(new_task)
    }

    /// The name on the repository's button: the picked one's; following, that of the repository
    /// whose board or empty state shows, or the focused pane's while it is still to be read.
    /// `None` outside any.
    fn shown_name(&self) -> Option<String> {
        if let Some(chosen) = self.looking.chosen() {
            return Some(kanban_repos::name(chosen));
        }
        match (&self.cwd, &self.read) {
            (None | Some(None), _) | (_, Some(Read::NotRepository)) => None,
            (_, Some(Read::NoMain { name } | Read::NoTasks { name, .. })) => Some(name.clone()),
            (_, Some(Read::Board(_))) => self.board.as_ref().map(|board| board.name.clone()),
            _ => self.focus_repo().map(|repo| kanban_repos::name(repo)),
        }
    }

    /// The repository's name as a button, its menu hanging under it when open: a folder before
    /// the name while the board follows the focused pane; a pin, and the accent's tint and edge,
    /// while it stays on a picked one.
    fn repo_button(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let (pinned, open) = (self.looking.chosen().is_some(), self.menu.is_some());
        let name = self
            .shown_name()
            .unwrap_or_else(|| "No repository".to_owned());
        let lead = if pinned {
            svg()
                .data(PIN)
                .flex_shrink_0()
                .size(ui.px(14.0))
                .text_color(c.accent)
                .into_any_element()
        } else {
            footer_icon::icon(Icon::Folder, c.muted, ui.scale(13.0 / footer_icon::SIZE))
                .into_any_element()
        };
        let button = div()
            .id("kanban-repo")
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .min_w(px(0.0))
            .h(ui.px(28.0))
            .pl(ui.px(6.0))
            .pr(ui.px(8.0))
            .rounded(ui.px(7.0))
            .border_1()
            .cursor_pointer()
            .map(|button| {
                if pinned {
                    button
                        .bg(c.accent.opacity(0.10))
                        .border_color(c.accent.opacity(0.45))
                        .hover(move |style| style.bg(c.accent.opacity(0.16)))
                } else {
                    button
                        .border_color(gpui::transparent_black())
                        .when(open, |button| button.bg(c.text.opacity(0.09)))
                        .hover(move |style| style.bg(c.text.opacity(0.09)))
                }
            })
            .child(lead)
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(name),
            )
            .child(
                svg()
                    .data(CHEVRON_DOWN)
                    .flex_shrink_0()
                    .size(ui.px(10.0))
                    .text_color(c.bright),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, cx| {
                    if view.dismissed.take() != Some(event.position) {
                        view.open_menu(None, window, cx);
                    }
                }),
            );
        div()
            .flex_shrink(1.0)
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(button)
            .when(open, |holder| {
                // Laid out right under the button, drawn above everything else.
                holder.child(
                    div().h(px(0.0)).child(deferred(
                        anchored()
                            .snap_to_window_with_margin(px(8.0))
                            .child(self.repo_menu(ui, cx)),
                    )),
                )
            })
    }

    /// The button back to the focused pane, where the numbers are while the board follows it.
    fn back(&self, chosen: &Path, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let focus = self.focus_repo().map(PathBuf::as_path);
        div()
            .id("kanban-back")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .h(ui.px(26.0))
            .px(ui.px(9.0))
            .rounded_full()
            .bg(c.text.opacity(SOFT))
            .text_size(ui.px(11.5))
            .text_color(c.bright)
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(move |style| style.bg(c.text.opacity(0.12)))
            .child(
                svg()
                    .data(TURN_BACK)
                    .flex_shrink_0()
                    .size(ui.px(11.0))
                    .text_color(c.bright),
            )
            .child(kanban_repos::back_label(focus, chosen))
            .on_click(cx.listener(|view, _: &ClickEvent, window, cx| view.follow(window, cx)))
    }

    /// The menu under the repository's name: following the focused pane, the repositories open
    /// in paddock now, the kept ones, and choosing a folder. A press outside it or Esc closes it.
    fn repo_menu(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let (theme, c) = (&*self.theme, self.colors);
        let rows = kanban_repos::menu(
            self.focus_repo().map(PathBuf::as_path),
            &self.open_repos(),
            &self.looking,
        );
        let label = || {
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
        };
        let mut panel = popover::panel(theme, ui)
            .id("kanban-repos")
            .track_focus(&self.menu_focus)
            .key_context(menu::DIALOG)
            .occlude()
            .w(ui.px(MENU_WIDTH))
            .mt(ui.px(4.0))
            .on_mouse_down_out(cx.listener(|view, event: &MouseDownEvent, window, cx| {
                view.dismissed = Some(event.position);
                view.close_menu(window, cx);
            }))
            .on_action(
                cx.listener(|view, _: &menu::Cancel, window, cx| view.close_menu(window, cx)),
            );
        for (index, row) in rows.into_iter().enumerate() {
            panel = match row {
                Row::Follow { name, on } => panel.child(
                    popover::lead_row(
                        theme,
                        ui,
                        "kanban-follow",
                        popover::tick(theme, ui, on),
                        false,
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Follow focused pane"),
                    )
                    .child(div().flex_1())
                    .children(name.map(|name| {
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_size(ui.px(11.5))
                            .text_color(c.dim)
                            .child(name)
                    }))
                    .on_click(
                        cx.listener(|view, _: &ClickEvent, window, cx| view.follow(window, cx)),
                    ),
                ),
                Row::Heading(heading) => panel
                    .child(popover::rule(theme, ui))
                    .child(popover::heading(theme, ui, heading)),
                Row::Repo { path, kept, on } => {
                    let lead = if kept {
                        svg()
                            .data(PIN)
                            .size(ui.px(13.0))
                            .text_color(c.accent)
                            .into_any_element()
                    } else {
                        footer_icon::icon(Icon::Folder, c.muted, ui.scale(13.0 / footer_icon::SIZE))
                            .into_any_element()
                    };
                    // The whole path: two repositories of one name are told apart by it.
                    let tip = Note::new(path.display().to_string().into(), theme, ui);
                    let line =
                        popover::lead_row(theme, ui, ("kanban-repo-row", index), lead, false)
                            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                            .child(label().child(kanban_repos::name(&path)))
                            .when(on, |line| line.child(popover::tick(theme, ui, true)))
                            .when(kept, |line| {
                                let tip = Tip::new("Remove from this list", theme, ui);
                                let repo = path.clone();
                                line.child(
                                    div()
                                        .id(("kanban-repo-remove", index))
                                        .flex_shrink_0()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size(ui.px(22.0))
                                        .rounded(px(5.0))
                                        .cursor_pointer()
                                        .invisible()
                                        .group_hover(popover::ROW_GROUP, |style| style.visible())
                                        .hover(move |style| style.bg(c.text.opacity(0.12)))
                                        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                                        .child(footer_icon::icon(
                                            Icon::Close,
                                            c.bright,
                                            ui.scale(10.0 / footer_icon::SIZE),
                                        ))
                                        .on_click(cx.listener(
                                            move |view, _: &ClickEvent, _, cx| {
                                                cx.stop_propagation();
                                                view.remove(&repo, cx);
                                            },
                                        )),
                                )
                            });
                    panel.child(line.on_click(cx.listener(
                        move |view, _: &ClickEvent, window, cx| {
                            view.choose(path.clone(), window, cx)
                        },
                    )))
                }
                Row::Choose => panel.child(popover::rule(theme, ui)).child(
                    popover::lead_row(theme, ui, "kanban-choose-folder", div(), false)
                        .child(label().child("Choose folder…"))
                        .on_click(cx.listener(|view, _: &ClickEvent, window, cx| {
                            view.choose_folder(window, cx)
                        })),
                ),
            };
        }
        // Why the folder just chosen was not taken, or why the list was not saved; the latter
        // stays until a save works.
        let note = self
            .menu
            .as_ref()
            .and_then(|menu| menu.note.clone())
            .or_else(|| self.looking.problem().map(SharedString::from));
        panel
            .children(note.map(|note| {
                div()
                    .px(ui.px(9.0))
                    .pt(ui.px(2.0))
                    .pb(ui.px(5.0))
                    .whitespace_normal()
                    .text_size(ui.px(11.5))
                    .text_color(c.yellow)
                    .child(note)
            }))
            .child(crate::browser::cover())
    }

    /// How many need the user, are in progress and to review.
    fn summary(&self, board: &Board, ui: &UiFont) -> Div {
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
        right
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
        let ids = self.id_width(board, fit, ui);
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
        // The Loop first, over Queued.
        list = list.child(self.loop_group(board, Place::List, ids, fit, ui, cx));
        for entry in entries(|column| self.listed(board, column).is_empty()) {
            let column = match entry {
                Entry::Group(column) => column,
                Entry::Zeros(columns) => {
                    list = list.child(self.zeros(&columns, ui));
                    continue;
                }
            };
            let open = !self.folded.contains(&column);
            list = list.child(self.group(board, column, open, false, ui, cx));
            // A folded group still shows the tasks that need the user.
            let shown: Vec<&Card> = self
                .listed(board, column)
                .into_iter()
                .filter(|card| open || card.needs_you)
                .collect();
            if !shown.is_empty() {
                // A group's rows in a box; DONE's bare, set in as far so their ids line up.
                let rows = self.rows(&shown, Place::List, ids, fit, ui, cx);
                let rows = if column == Column::Done {
                    rows.px(ui.px(BOX_PAD) + px(1.0))
                } else {
                    let raised = raised::raised(&self.theme, ui);
                    rows.p(ui.px(BOX_PAD))
                        .rounded(ui.px(BOX_RADIUS))
                        .bg(raised.face)
                        .border_1()
                        .border_color(raised.edge)
                        .shadow(raised.shadows)
                };
                let mut cards = div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .pt(ui.px(4.0))
                    .pb(ui.px(2.0))
                    .child(rows);
                if open && column == Column::Done {
                    cards = cards.children(
                        self.show_all(board, ui, cx)
                            .map(|more| div().flex().pt(ui.px(6.0)).pl(ui.px(HEAD_X)).child(more)),
                    );
                }
                list = list.child(cards);
            }
        }
        list
    }

    /// Tasks' rows one under another, a rule between each two but next to the one the mouse is on.
    fn rows(
        &self,
        cards: &[&Card],
        place: Place,
        ids: f32,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut rows = div().flex_shrink_0().flex().flex_col();
        let mut above = true;
        for card in cards {
            let hovered = self.hovered(card);
            rows = rows.child(self.card(card, place, ids, !above && !hovered, None, fit, ui, cx));
            above = hovered;
        }
        rows
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
        let empty = self.listed(board, column).is_empty();
        let mut head = div()
            .id(("kanban-group", column as usize))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(26.0))
            .mt(ui.px(if first { 2.0 } else { 8.0 }))
            .px(ui.px(HEAD_X))
            .rounded(ui.px(6.0))
            .text_size(ui.px(GROUP_SIZE));
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

    /// In the narrow list, the groups among In progress, To review and Merged that have nothing,
    /// on one line of small faint words (P5-185): each its small faded ring, its name and 0, set in
    /// as far as the groups' rings.
    fn zeros(&self, columns: &[Column], ui: &UiFont) -> Div {
        let c = self.colors;
        let mut line = div()
            .flex_shrink_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(ui.px(ZERO_GAP))
            .gap_y(ui.px(2.0))
            .min_h(ui.px(22.0))
            .mt(ui.px(8.0))
            .pl(ui.px(HEAD_X + ARROW + 8.0))
            .pr(ui.px(HEAD_X))
            .text_size(ui.px(12.0))
            .text_color(c.dim);
        for &column in columns {
            let mut zero = div()
                .id(("kanban-zero", column as usize))
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .whitespace_nowrap()
                .child(
                    div()
                        .flex_shrink_0()
                        .size(ui.px(RING))
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(EMPTY_RING)
                        .child(self.ring_icon(ring(column, false, None, false), ZERO_RING, ui)),
                )
                .child(format!("{} 0", column.label()));
            let note = column.note();
            if !note.is_empty() {
                let tip = Tip::new(note, &self.theme, ui);
                zero = zero.tooltip(move |_, cx| cx.new(|_| tip.clone()).into());
            }
            line = line.child(zero);
        }
        line
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
                .px(ui.px(COLUMN_X))
                .pb(ui.px(8.0));
            // The Loop at the top of Queued.
            if column == Column::Queued {
                body = body.child(self.loop_group(board, Place::Tile, 0.0, fit, ui, cx));
            }
            let cards = self.listed(board, column);
            body = body.child(self.rows(&cards, Place::Tile, 0.0, fit, ui, cx));
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
                    .rounded(ui.px(BOX_RADIUS))
                    .bg(c.faint.opacity(COLUMN_FACE))
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
                            .text_size(ui.px(GROUP_SIZE))
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

    /// A group's ring (P5-114), as a committed medium task in it is marked; faded for an empty
    /// group.
    fn dot(&self, column: Column, empty: bool, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .when(empty, |dot| dot.opacity(EMPTY_RING))
            .child(self.ring_icon(ring(column, false, None, false), RING, ui))
    }

    fn label(&self, column: Column) -> Div {
        div()
            .flex_shrink(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .child(column.label())
    }

    /// How many a group has, in a pill: `3 of 130` while DONE lists its last few; Queued's without
    /// those in the Loop.
    fn count(&self, board: &Board, column: Column, ui: &UiFont) -> Div {
        let count = if column == Column::Queued {
            self.listed(board, column).len().to_string()
        } else {
            board.count(column, self.all_done)
        };
        self.count_pill(count, ui)
    }

    fn count_pill(&self, count: String, ui: &UiFont) -> Div {
        let c = self.colors;
        pill(ui)
            .px(ui.px(7.0))
            .bg(c.text.opacity(SOFT))
            .text_size(ui.px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.muted)
            .child(count)
    }

    /// The cards a group lists ([`Board::listed`]), Queued's without those in the Loop, which has
    /// its own group over it.
    fn listed<'a>(&self, board: &'a Board, column: Column) -> Vec<&'a Card> {
        let mut cards = board.listed(column, self.all_done);
        if column == Column::Queued {
            let looped = self.in_loop(board);
            cards.retain(|card| !looped.iter().any(|l| std::ptr::eq(*l, *card)));
        }
        cards
    }

    /// The Loop's group over Queued (P5-106, P5-158): its header, with how many it has and the
    /// switch; and in a box as the other groups' are, on top the task handed over (Now), wherever
    /// it is on the board, with what the loop says of it under its title, then Next, the tasks
    /// waiting, numbered, under a rule. With nothing handed over, one line in Now's place says
    /// what the loop waits for, in amber when that is the user, with Retry when it can be.
    #[allow(clippy::too_many_arguments)]
    fn loop_group(
        &self,
        board: &Board,
        place: Place,
        ids: f32,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let lp = self.loops.get(&board.repo);
        let stand = self.stand(&board.repo, &lp, board);
        let sent = self.now_card(board, &lp);
        let next = self.next_cards(board, &lp);
        let face = kanban_loop::face(
            &stand,
            sent.map(|card| card.column != Column::Queued),
            next.len(),
            &Timing::DEFAULT,
        );
        let needs = stand.needs_you();
        let inline = place == Place::List;
        let head = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(26.0))
            .mt(ui.px(2.0))
            .px(ui.px(if inline { HEAD_X } else { 2.0 }))
            .text_size(ui.px(GROUP_SIZE))
            .when(inline, |head| {
                head.child(div().flex_shrink_0().w(ui.px(ARROW)))
            })
            .child(
                svg()
                    .data(LOOP_ARROWS)
                    .flex_shrink_0()
                    .size(ui.px(RING))
                    .text_color(if lp.on { c.accent } else { c.dim }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.bright)
                    .child("Loop"),
            )
            .child(self.count_pill(face.count.to_string(), ui))
            .child(div().flex_1())
            .child(self.loop_switch(&lp, ui, cx));
        let button = |id: &'static str, words: &'static str| {
            answer(id, words, ui)
                .border_color(c.rule)
                .text_color(c.text)
                .hover(move |style| style.bg(c.text.opacity(0.09)))
        };
        let retry = |view: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
            view.loop_retry(cx)
        };
        let nudge = |view: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
            view.nudge(cx)
        };
        let skip = |view: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
            view.edit_loop(Loop::skip, cx)
        };
        // Retry after a failed handing over; Nudge and Skip on a task that is stuck.
        let mut buttons = Vec::new();
        match &stand {
            Stand::Failed { .. } => {
                buttons.push(button("kanban-loop-retry", "Retry").on_click(cx.listener(retry)));
            }
            Stand::Stuck { .. } => {
                buttons.push(button("kanban-loop-nudge", "Nudge").on_click(cx.listener(nudge)));
                buttons.push(button("kanban-loop-skip", "Skip").on_click(cx.listener(skip)));
            }
            _ => {}
        }
        let stuck = matches!(stand, Stand::Stuck { .. });
        let top: Option<AnyElement> = match face.top {
            Top::Nothing => None,
            Top::Line { words, amber } => Some(
                asked(ui)
                    .pl(ui.px(CARD_LEFT))
                    .pr(ui.px(CARD_RIGHT))
                    .py(ui.px(CARD_Y))
                    .text_size(ui.px(UNDER_SIZE))
                    .child(said(
                        words,
                        if amber {
                            c.yellow
                        } else if lp.on {
                            c.muted
                        } else {
                            c.dim
                        },
                    ))
                    .children(buttons)
                    .into_any_element(),
            ),
            Top::Now(doing) => sent.map(|card| {
                let small = || asked(ui).text_size(ui.px(UNDER_SIZE));
                let (agent, needs, rows) = match doing {
                    Doing::Agent => (true, false, Vec::new()),
                    Doing::Sent => {
                        let to = kanban::controllers_for(&board.repo, &self.agents);
                        let to = match to.as_slice() {
                            [one] => one.name.as_str(),
                            _ => "the controller",
                        };
                        let ago = lp
                            .sent
                            .as_ref()
                            .map(|sent| {
                                format!(" \u{b7} {}", card::short_time(Some(now() - sent.at)))
                            })
                            .unwrap_or_default();
                        let words = format!("Sent to {to}{ago}");
                        (
                            false,
                            false,
                            vec![small().child(said(words, c.muted)).into_any_element()],
                        )
                    }
                    Doing::Says(words) => {
                        // Nudge and Skip on a row of their own; Retry after the words.
                        let (words_row, buttons_row) = if stuck {
                            (
                                small().child(said(words, c.yellow)),
                                small().children(buttons),
                            )
                        } else {
                            (
                                small().child(said(words, c.yellow)).children(buttons),
                                small(),
                            )
                        };
                        let mut rows = vec![words_row.into_any_element()];
                        if stuck {
                            rows.push(buttons_row.into_any_element());
                        }
                        (false, true, rows)
                    }
                };
                let row = Now { agent, needs, rows };
                self.card(card, place, ids, false, Some(row), fit, ui, cx)
                    .into_any_element()
            }),
        };
        let mut group = div().flex_shrink_0().flex().flex_col().child(head);
        if let Some(problem) = self.loops.problem() {
            group = group.child(
                div()
                    .px(ui.px(HEAD_X))
                    .text_size(ui.px(11.5))
                    .text_color(c.yellow)
                    .child(problem.to_owned()),
            );
        }
        if top.is_none() && next.is_empty() {
            return group;
        }
        let mut rows = div().flex_shrink_0().flex().flex_col().children(top);
        if face.rule {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .pl(ui.px(CARD_LEFT))
                    .pr(ui.px(CARD_RIGHT))
                    .pt(ui.px(4.0))
                    .pb(ui.px(2.0))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(ui.px(10.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c.dim)
                            .child("NEXT"),
                    )
                    .child(div().flex_1().h(px(1.0)).bg(c.text.opacity(RULE))),
            );
        }
        if !next.is_empty() {
            rows = rows.child(self.rows(&next, place, ids, fit, ui, cx));
        }
        let rows = if inline {
            let raised = raised::raised(&self.theme, ui);
            rows.p(ui.px(BOX_PAD))
                .rounded(ui.px(BOX_RADIUS))
                .bg(raised.face)
                .border_1()
                .border_color(if needs {
                    c.yellow.opacity(0.4)
                } else {
                    raised.edge
                })
                .shadow(raised.shadows)
        } else {
            rows.pb(ui.px(6.0))
                .mb(ui.px(4.0))
                .border_b_1()
                .border_color(c.rule.opacity(0.6))
        };
        group.child(
            div()
                .flex_shrink_0()
                .pt(ui.px(4.0))
                .pb(ui.px(2.0))
                .child(rows),
        )
    }

    /// The Loop's switch: on, the accent; it cannot be turned on with nothing in the loop.
    fn loop_switch(&self, lp: &Loop, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let on = lp.on;
        let can = on || lp.can_turn_on();
        let words = if on {
            "Turn the loop off: the task under way is still finished"
        } else if can {
            "Turn the loop on: hands the tasks to the controller one by one"
        } else {
            "Add queued tasks to the loop first"
        };
        let tip = Tip::new(words, &self.theme, ui);
        let knob = div()
            .absolute()
            .top(ui.px(2.0))
            .size(ui.px(11.0))
            .rounded_full()
            .bg(if on { c.ground } else { c.muted })
            .map(|knob| {
                if on {
                    knob.right(ui.px(2.0))
                } else {
                    knob.left(ui.px(2.0))
                }
            });
        div()
            .id("kanban-loop-switch")
            .relative()
            .flex_shrink_0()
            .w(ui.px(26.0))
            .h(ui.px(15.0))
            .rounded_full()
            .bg(if on { c.accent } else { c.text.opacity(0.14) })
            .when(!can, |track| track.opacity(0.5))
            .when(can, |track| track.cursor_pointer())
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(knob)
            .on_click(
                cx.listener(move |view, _: &ClickEvent, _, cx| {
                    view.edit_loop(|lp| lp.turn(!on), cx)
                }),
            )
    }

    /// The width inside a card's padding, in the narrow list or a widened column, that its texts
    /// sharing a line are cut to.
    fn room(&self, place: Place, ui: &UiFont) -> f32 {
        let card = match place {
            Place::List => self.width - 2.0 * ui.scale(LIST_X) - 2.0 * (ui.scale(BOX_PAD) + 1.0),
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
            .on_hover(cx.listener(move |view, hovered: &bool, window, cx| {
                view.hover(&file, *hovered, window, cx)
            }))
    }

    /// A task's row (P5-113), in a group's box in the narrow list or in a widened column, rows
    /// parted by a rule (`rule`) that is left out next to the one the mouse is on. In the list its
    /// id stands in a column `ids` wide left of the title, a draft's after a dashed ring; in a
    /// column, too narrow for that, on a line above the title. At the end of the title's first line
    /// any warning, Dropped and its age; the title on up to two lines, a DONE row's on one. Under
    /// the title, from its left edge: what it waits for, Needs you, its agent and the controller on
    /// one to review, and a question asked on it.
    #[allow(clippy::too_many_arguments)]
    fn card(
        &self,
        card: &Card,
        place: Place,
        ids: f32,
        rule: bool,
        now: Option<Now>,
        fit: &Fit,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let hovered = self.hovered(card);
        let done = card.column == Column::Done;
        let inline = place == Place::List;
        let room = self.room(place, ui);
        let under = if inline {
            (room - ids - ui.scale(ID_GAP)).max(0.0)
        } else {
            room
        };
        // A DONE row keeps its age with the mouse on it; another moves it to its agent's line, or
        // gives it up to the buttons.
        let age = (!card.age.is_empty() && (done || !hovered)).then(|| {
            div()
                .flex_shrink_0()
                .text_size(ui.px(AGE_SIZE))
                .text_color(c.dim)
                .child(card.age.clone())
        });
        // Under way from the Loop, in its own column: marked so (P5-106).
        let looped = (now.is_none() && self.slot(card) == Some(Slot::Under)).then(|| {
            pill(ui)
                .h(ui.px(16.0))
                .px(ui.px(6.0))
                .bg(c.accent.opacity(0.14))
                .text_size(ui.px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.accent)
                .child("Loop")
        });
        // Queued in the list, not in the Loop's box: Draft, High, Low (P5-185).
        let tags = if inline && now.is_none() && self.slot(card).is_none() {
            tags(card.column, card.draft, card.priority)
        } else {
            Vec::new()
        };
        let end = div()
            .flex_shrink_0()
            .h(ui.px(TITLE_LINE))
            .flex()
            .items_center()
            .gap(ui.px(END_GAP))
            .children(looped)
            .children(tags.into_iter().map(|t| self.tag(t, ui)))
            .children(self.warning(card, ui))
            .children(age);
        let title = if done {
            // Cut here, to what its line leaves it: in the list the end shares it, in a column the
            // end is on the id's line above.
            let mut left = room;
            if inline {
                let mut ends = Vec::new();
                if !card.problems.is_empty() {
                    ends.push(ui.scale(12.0));
                }
                if !card.age.is_empty() {
                    ends.push(fit.normal(&card.age, AGE_SIZE));
                }
                left = under;
                if !ends.is_empty() {
                    left -= ends.iter().sum::<f32>()
                        + ui.scale(ID_GAP)
                        + (ends.len() - 1) as f32 * ui.scale(END_GAP);
                }
            }
            let words = card::elide(&card.title, &|cut| fit.normal(cut, DONE_SIZE) <= left);
            div()
                .flex_1()
                .min_w(px(0.0))
                .whitespace_nowrap()
                .text_size(ui.px(DONE_SIZE))
                // Fainter than the other columns by its colour alone, its id left as bright
                // (P5-114a): one dropped fainter still, one that needs the user as the others.
                .text_color(if card.needs_you {
                    c.muted
                } else if card.dropped.is_some() {
                    c.dim.opacity(DROPPED_TITLE)
                } else {
                    c.dim
                })
                .child(words)
        } else {
            div()
                .flex_1()
                .min_w(px(0.0))
                .line_clamp(2)
                .text_ellipsis()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if card.draft { c.muted } else { c.text })
                .child(card.title.clone())
        }
        .line_height(ui.px(TITLE_LINE));
        let id = self.id(card, now.as_ref().map(|now| now.needs), ui);
        let head = if inline {
            div()
                .flex()
                .items_start()
                .gap(ui.px(ID_GAP))
                .min_w(px(0.0))
                .child(div().flex_shrink_0().w(px(ids)).child(id))
                .child(title)
                .child(end)
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(ui.px(2.0))
                .min_w(px(0.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(ui.px(END_GAP))
                        .child(id)
                        .child(div().flex_1())
                        .child(end),
                )
                .child(title)
                .into_any_element()
        };
        let ring = self.ring(place, hovered);
        let size = match place {
            Place::List => BAR_BUTTON,
            Place::Tile => TILE_BUTTON,
        };
        let bar = self.actions(card, size, ui, cx).map(|bar| {
            self.tray(bar, ui)
                .top(ui.px(if done { DONE_BAR_TOP } else { BAR_TOP }))
                .when(place == Place::Tile, |bar| {
                    bar.flex_wrap().justify_end().max_w(px(room))
                })
        });
        let small = |row: Div| row.text_size(ui.px(UNDER_SIZE));
        let below: Vec<AnyElement> = if let Some(now) = now {
            // Now, in the Loop's box: its agent's line once begun, else what the loop says of it.
            let agent = now
                .agent
                .then(|| self.footer(card, place, hovered, under, ring, fit, ui, cx))
                .flatten()
                .map(IntoElement::into_any_element);
            agent.into_iter().chain(now.rows).collect()
        } else {
            [
                self.waits(card, under, fit, ui)
                    .map(IntoElement::into_any_element),
                self.asks(card, under, fit, ui)
                    .map(IntoElement::into_any_element),
                self.footer(card, place, hovered, under, ring, fit, ui, cx)
                    .map(IntoElement::into_any_element),
                self.clearing(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
                self.prioritizing(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
                self.deleting(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
                self.dropping(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
                self.queuing(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
                self.moving_to_draft(card, ui, cx)
                    .map(|r| small(r).into_any_element()),
            ]
            .into_iter()
            .flatten()
            .collect()
        };
        let below = (!below.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap(ui.px(5.0))
                .min_w(px(0.0))
                .when(inline, |below| below.pl(px(ids) + ui.px(ID_GAP)))
                .children(below)
        });
        let y = if done { DONE_Y } else { CARD_Y };
        self.hoverable(card, cx)
            .flex()
            .flex_col()
            .gap(ui.px(4.0))
            .py(ui.px(y))
            .pl(ui.px(CARD_LEFT))
            .pr(ui.px(CARD_RIGHT))
            .rounded(ui.px(ROW_RADIUS))
            .when(hovered, |row| row.bg(c.text.opacity(ROW_HOVERED)))
            .text_size(ui.px(13.0))
            .when(rule, |row| {
                row.child(
                    div()
                        .absolute()
                        .top(px(0.0))
                        .left(ui.px(CARD_LEFT))
                        .right(ui.px(CARD_RIGHT))
                        .h(px(1.0))
                        .bg(c.text.opacity(RULE)),
                )
            })
            .child(head)
            .children(below)
            .children(bar)
    }

    /// A small tag at the end of a queued row in the list (P5-185): Draft in a dashed frame, High
    /// in red, Low faint.
    fn tag(&self, tag: Tag, ui: &UiFont) -> Div {
        let c = self.colors;
        let chip = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(ui.px(16.0))
            .px(ui.px(5.0))
            .rounded(ui.px(5.0))
            .border_1()
            .border_color(gpui::transparent_black())
            .whitespace_nowrap()
            .text_size(ui.px(10.5))
            .font_weight(FontWeight::MEDIUM);
        match tag {
            Tag::Draft => chip
                .border_dashed()
                .border_color(c.dim)
                .text_color(c.muted)
                .child("Draft"),
            Tag::High => chip
                .bg(c.red.opacity(0.14))
                .text_color(c.red)
                .child(Priority::High.label()),
            Tag::Low => chip
                .bg(c.text.opacity(SOFT))
                .text_color(c.muted)
                .child(Priority::Low.label()),
        }
    }

    /// The ground under a row's avatars, round their status dots: a group's box over the panel in
    /// the list, the column's face in a widened board, and the row's own face over either while
    /// the mouse is on it.
    fn ring(&self, place: Place, hovered: bool) -> Hsla {
        let c = self.colors;
        let ground = match place {
            Place::List => raised::face(&self.theme),
            Place::Tile => c.ground.blend(c.faint.opacity(COLUMN_FACE)),
        };
        if hovered {
            ground.blend(c.text.opacity(ROW_HOVERED))
        } else {
            ground
        }
    }

    /// How wide the column of ids is: the ring and the widest id the board lists.
    fn id_width(&self, board: &Board, fit: &Fit, ui: &UiFont) -> f32 {
        let id = Column::ALL
            .into_iter()
            .flat_map(|column| board.listed(column, self.all_done))
            .filter(|card| !card.id.is_empty())
            .map(|card| fit.mono(&card.id, ID_SIZE) + ui.scale(RING_GAP))
            .fold(0.0, f32::max);
        (ui.scale(RING) + id).ceil()
    }

    /// A ring (P5-114) `size` points wide, in its role's colour from the theme; its tick and stroke
    /// are cut out, so whatever is under it shows through in any theme.
    fn ring_icon(&self, (shape, role): (Shape, Role), size: f32, ui: &UiFont) -> gpui::Svg {
        let c = self.colors;
        let data = match shape {
            Shape::Open => RING_OPEN,
            Shape::Dashed => RING_DASHED,
            Shape::Half => RING_HALF,
            Shape::Most => RING_MOST,
            Shape::Tick => RING_TICK,
            Shape::Struck => RING_STRUCK,
        };
        let color = match role {
            Role::Red => c.red,
            Role::Muted => c.muted,
            Role::Dim => c.dim,
            Role::Blue => c.blue,
            Role::Yellow => c.yellow,
            Role::Green => c.green,
        };
        svg()
            .flex_shrink_0()
            .data(data)
            .size(ui.px(size))
            .text_color(color)
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

    /// Under the title of a task under way, to review or merged: its agent's line and, on one to
    /// review, the controller's. `ring` is the ground under the row, round the avatars' status
    /// dots. `None` with nothing to say.
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
                .gap(ui.px(5.0))
                .pt(ui.px(1.0))
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

    /// A task's ring (P5-114), then its id, if it has one, in small fixed-width type. The ring
    /// tells when the mouse is on it: Draft on a draft, why on one dropped, otherwise its state.
    fn id(&self, card: &Card, now: Option<bool>, ui: &UiFont) -> Div {
        let c = self.colors;
        let mark = ring(
            card.column,
            card.draft,
            card.stripe(),
            card.dropped.is_some(),
        );
        let words = if card.draft {
            "Draft".to_owned()
        } else if let Some(why) = &card.dropped {
            if why.is_empty() {
                "Dropped".to_owned()
            } else {
                format!("Dropped: {why}")
            }
        } else if let Some(priority) = card.stripe() {
            format!("{} \u{b7} {}", card.column.label(), priority.label())
        } else {
            card.column.label().to_owned()
        };
        // In the Loop's group, its place there instead, or that it was handed over (P5-106).
        let (marked, words) = match self.slot(card) {
            Some(Slot::At(n)) => (
                div()
                    .flex_shrink_0()
                    .size(ui.px(RING))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(c.accent)
                    .text_size(ui.px(8.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.accent)
                    .child(n.to_string())
                    .into_any_element(),
                format!("Loop \u{b7} number {n} in line"),
            ),
            _ => (self.ring_icon(mark, RING, ui).into_any_element(), words),
        };
        // Now, on top of the Loop's box: an arc turning, or an amber ! when it needs the user.
        let (marked, words) = match now {
            Some(true) => (
                div()
                    .flex_shrink_0()
                    .size(ui.px(RING))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(c.yellow)
                    .text_size(ui.px(8.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.yellow)
                    .child("!")
                    .into_any_element(),
                "Loop \u{b7} now \u{b7} needs you".to_owned(),
            ),
            Some(false) => (
                svg()
                    .data(SPIN_ARC)
                    .flex_shrink_0()
                    .size(ui.px(RING))
                    .text_color(c.accent)
                    .with_animation(
                        "loop-now",
                        Animation::new(Duration::from_millis(1100))
                            .repeat_synced()
                            .with_max_fps(30.0),
                        |arc, delta| {
                            arc.with_transformation(Transformation::rotate(percentage(delta)))
                        },
                    )
                    .into_any_element(),
                "Loop \u{b7} now".to_owned(),
            ),
            None => (marked, words),
        };
        let note = Note::new(words.into(), &self.theme, ui);
        div()
            .flex_shrink_0()
            .h(ui.px(TITLE_LINE))
            .flex()
            .items_center()
            .gap(ui.px(RING_GAP))
            .child(
                div()
                    .id(SharedString::from(format!("kanban-ring-{}", card.file)))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .child(marked)
                    .tooltip(move |_, cx| cx.new(|_| note.clone()).into()),
            )
            .children((!card.id.is_empty()).then(|| {
                div()
                    .font(self.mono.clone())
                    .text_size(ui.px(ID_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.muted)
                    .child(card.id.clone())
            }))
    }

    /// A warning sign at the end of the title's line that tells, when the mouse is on it, what in
    /// the task file could be read wrong (P5-60a).
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

    /// Under a queued task's title, what it waits for, after a link, cut with `…` to `room`.
    fn waits(&self, card: &Card, room: f32, fit: &Fit, ui: &UiFont) -> Option<Div> {
        if card.column != Column::Queued {
            return None;
        }
        let c = self.colors;
        let (words, _) = card.state.as_ref()?;
        let left = room - ui.scale(10.0 + 5.0) - SPARE;
        let words = card::elide(words, &|cut| fit.normal(cut, UNDER_SIZE) <= left);
        Some(
            div()
                .flex()
                .items_center()
                .gap(ui.px(5.0))
                .whitespace_nowrap()
                .text_size(ui.px(UNDER_SIZE))
                .text_color(c.muted)
                .child(svg().data(LINK).size(ui.px(10.0)).text_color(c.dim))
                .child(words),
        )
    }

    /// Under the title of a task that needs the user, one line: Needs you in amber, then what its
    /// task file says they are waited on for, cut here with `…` to `room` (GPUI's own cut left
    /// the text running out of its block, P5-113), the whole of it shown when the mouse is on the
    /// line. Only Needs you when just its agent waits.
    fn asks(&self, card: &Card, room: f32, fit: &Fit, ui: &UiFont) -> Option<Stateful<Div>> {
        if !card.needs_you {
            return None;
        }
        let c = self.colors;
        let lead = "Needs you";
        let gap = ui.scale(6.0);
        let asks = card
            .asks
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let left = room - fit.bold(lead, UNDER_SIZE) - gap - SPARE;
        let words = asks.map(|asks| asks_words(asks, left, &|cut| fit.normal(cut, UNDER_SIZE)));
        let mut line = div()
            .id(SharedString::from(format!("kanban-asks-{}", card.file)))
            .flex()
            .items_center()
            .gap(px(gap))
            .min_w(px(0.0))
            .whitespace_nowrap()
            .text_size(ui.px(UNDER_SIZE))
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.yellow)
                    .child(lead),
            )
            .children(words.map(|words| div().text_color(c.text).child(words)));
        if let Some(asks) = asks {
            let note = Note::new(format!("{lead}: {asks}").into(), &self.theme, ui);
            line = line.tooltip(move |_, cx| cx.new(|_| note.clone()).into());
        }
        Some(line)
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
    /// user, Clear, which asks first; on a queued card, Set priority, which asks which; on a draft,
    /// Delete draft, and on a queued card, a draft too, Drop task, which ask first.
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
                .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    view.ask_clear(&file, window, cx);
                })),
            );
        }
        if self.offers_priority(card) {
            let file = card.file.clone();
            bar = bar.child(
                button("kanban-priority", Icon::Sort, "Set priority").on_click(cx.listener(
                    move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        view.ask_priority(&file, window, cx);
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
        let drawn = |data: &'static [u8]| {
            svg()
                .data(data)
                .size(ui.px(13.0))
                .text_color(c.muted)
                .into_any_element()
        };
        // The Loop's (P5-106): a queued task on main is added to its end, unless it waits for
        // another; in it, moved and taken out, the one handed over taken out while not begun.
        let queued = card.column == Column::Queued && !card.draft && !card.id.is_empty();
        if queued && let Some(board) = &self.board {
            let id = card.id.clone();
            match self.slot(card) {
                None => {
                    if let Some(waits) = &card.waits {
                        let tip = Tip::new(format!("Waits for {waits}"), &theme, ui);
                        bar = bar.child(
                            div()
                                .id("kanban-loop-add")
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(ui.px(size))
                                .opacity(0.4)
                                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                                .child(drawn(LOOP_ARROWS)),
                        );
                    } else {
                        bar = bar.child(
                            glyph("kanban-loop-add", drawn(LOOP_ARROWS), "Add to loop").on_click(
                                cx.listener(move |view, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    view.edit_loop(|lp| lp.add(&id), cx);
                                }),
                            ),
                        );
                    }
                }
                Some(slot @ (Slot::At(_) | Slot::Sent)) => {
                    let places = self
                        .in_loop(board)
                        .iter()
                        .filter(|card| matches!(self.slot(card), Some(Slot::At(_))))
                        .count();
                    if let Slot::At(n) = slot {
                        for (up, show, name, words, data) in [
                            (true, n > 1, "kanban-loop-up", "Move up", CHEVRON_UP),
                            (
                                false,
                                n < places,
                                "kanban-loop-down",
                                "Move down",
                                CHEVRON_DOWN,
                            ),
                        ] {
                            if !show {
                                continue;
                            }
                            let id = id.clone();
                            bar = bar.child(glyph(name, drawn(data), words).on_click(cx.listener(
                                move |view, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    view.edit_loop(|lp| lp.shift(&id, up), cx);
                                },
                            )));
                        }
                    }
                    let icon =
                        footer_icon::icon(Icon::Close, c.muted, ui.scale(11.0 / footer_icon::SIZE));
                    bar = bar.child(
                        glyph(
                            "kanban-loop-remove",
                            icon.into_any_element(),
                            "Remove from loop",
                        )
                        .on_click(cx.listener(
                            move |view, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                view.edit_loop(|lp| lp.remove(&id), cx);
                            },
                        )),
                    );
                }
                Some(Slot::Under) => {}
            }
        }
        if self.offers_delete(card) {
            let file = card.file.clone();
            bar = bar.child(glyph("kanban-delete", drawn(BIN), "Delete draft").on_click(
                cx.listener(move |view, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    view.ask_delete(&file, window, cx);
                }),
            ));
        }
        if self.offers_drop(card) {
            if !card.draft
                && !matches!(
                    self.moves.get(&card.file),
                    Some(Clearing::Running | Clearing::Done)
                )
            {
                let file = card.file.clone();
                bar = bar.child(
                    glyph("kanban-move", drawn(MOVE_TO_DRAFT), "Move to draft").on_click(
                        cx.listener(move |view, _: &ClickEvent, window, cx| {
                            cx.stop_propagation();
                            view.take_back(&file, window, cx);
                            view.moves.insert(file.clone(), Clearing::Asking);
                            cx.notify();
                        }),
                    ),
                );
            }
            if card.draft
                && !matches!(
                    self.queues.get(&card.file),
                    Some(Clearing::Running | Clearing::Done)
                )
            {
                let file = card.file.clone();
                bar = bar.child(glyph("kanban-queue", drawn(QUEUE), "Queue task").on_click(
                    cx.listener(move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        view.take_back(&file, window, cx);
                        view.queues.insert(file.clone(), Clearing::Asking);
                        cx.notify();
                    }),
                ));
            }
            let file = card.file.clone();
            bar = bar.child(glyph("kanban-drop", drawn(STRUCK), "Drop task").on_click(
                cx.listener(move |view, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    view.ask_drop(&file, window, cx);
                }),
            ));
        }
        Some(bar)
    }

    /// Whether the buttons over `card` include Delete draft: it is a draft, and is not being
    /// deleted or deleted already.
    fn offers_delete(&self, card: &Card) -> bool {
        let busy = matches!(
            self.deletes.get(&card.file),
            Some(Clearing::Running | Clearing::Done)
        );
        card.deletable() && !busy
    }

    /// Whether the buttons over `card` include Drop task: the board has it to drop
    /// ([`Board::droppable`]), and it is not being dropped or dropped already.
    fn offers_drop(&self, card: &Card) -> bool {
        let busy = matches!(
            self.drops.get(&card.file),
            Some(Clearing::Running | Clearing::Done)
        );
        !busy
            && self
                .board
                .as_ref()
                .is_some_and(|board| board.droppable(card))
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

    /// Under a draft being deleted, as under a card being cleared: the question with Cancel and
    /// Delete, then that it is being deleted, or why it was not. The question goes once the card
    /// is no longer a draft.
    fn deleting(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        Some(match self.deletes.get(&card.file)? {
            Clearing::Asking => {
                if !card.deletable() {
                    return None;
                }
                let (file, leave) = (card.file.clone(), card.file.clone());
                let cancel = answer("kanban-delete-cancel", "Cancel", ui)
                    .border_color(c.rule)
                    .text_color(c.muted)
                    .hover(move |style| style.bg(c.text.opacity(0.09)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        if view.deletes.remove(&leave).is_some() {
                            cx.notify();
                        }
                    }));
                let confirm = answer("kanban-delete-confirm", "Delete", ui)
                    .bg(c.red.opacity(0.16))
                    .border_color(c.red.opacity(0.5))
                    .text_color(c.red)
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(c.red.opacity(0.26)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        view.delete(&file, cx);
                    }));
                asked(ui)
                    .child(said(
                        "Delete this draft? It goes to the Trash.".into(),
                        c.text,
                    ))
                    .child(div().flex_1())
                    // The two buttons stay together when the row wraps.
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .gap(ui.px(6.0))
                            .child(cancel)
                            .child(confirm),
                    )
            }
            Clearing::Running => asked(ui).child(said("Deleting\u{2026}".into(), c.dim)),
            Clearing::Done => asked(ui).child(said("Moved to the Trash".into(), c.dim)),
            Clearing::Refused(why) => asked(ui).child(said(format!("Not deleted: {why}"), c.red)),
        })
    }

    /// Under a queued task: confirm moving it to drafts, or show the result of its commit.
    fn moving_to_draft(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        Some(match self.moves.get(&card.file)? {
            Clearing::Asking => {
                if card.draft || !self.board.as_ref()?.droppable(card) {
                    return None;
                }
                let (file, leave) = (card.file.clone(), card.file.clone());
                let cancel = answer("kanban-move-cancel", "Cancel", ui)
                    .border_color(c.rule)
                    .text_color(c.muted)
                    .hover(move |style| style.bg(c.text.opacity(0.09)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if view.take_back(&leave, window, cx) {
                            cx.notify();
                        }
                    }));
                let confirm = answer("kanban-move-confirm", "Move", ui)
                    .bg(c.accent.opacity(0.16))
                    .border_color(c.accent.opacity(0.5))
                    .text_color(c.accent)
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(c.accent.opacity(0.26)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        view.move_task_to_draft(&file, cx);
                    }));
                asked(ui)
                    .child(said(
                        "Move this task back to drafts? Removes it from main; the file stays."
                            .into(),
                        c.text,
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .gap(ui.px(6.0))
                            .child(cancel)
                            .child(confirm),
                    )
            }
            Clearing::Running => asked(ui).child(said("Moving…".into(), c.dim)),
            Clearing::Done => asked(ui).child(said("Moved to drafts".into(), c.dim)),
            Clearing::Refused(why) => asked(ui).child(said(format!("Not moved: {why}"), c.red)),
        })
    }

    /// Under a draft: confirm queuing it, or show the result of its commit.
    fn queuing(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        Some(match self.queues.get(&card.file)? {
            Clearing::Asking => {
                if !card.draft || !self.board.as_ref()?.droppable(card) {
                    return None;
                }
                let (file, leave) = (card.file.clone(), card.file.clone());
                let cancel = answer("kanban-queue-cancel", "Cancel", ui)
                    .border_color(c.rule)
                    .text_color(c.muted)
                    .hover(move |style| style.bg(c.text.opacity(0.09)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if view.take_back(&leave, window, cx) {
                            cx.notify();
                        }
                    }));
                let confirm = answer("kanban-queue-confirm", "Queue", ui)
                    .bg(c.accent.opacity(0.16))
                    .border_color(c.accent.opacity(0.5))
                    .text_color(c.accent)
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(c.accent.opacity(0.26)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        view.queue_task(&file, cx);
                    }));
                asked(ui)
                    .child(said(
                        "Queue this task? Commits this draft to main.".into(),
                        c.text,
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .gap(ui.px(6.0))
                            .child(cancel)
                            .child(confirm),
                    )
            }
            Clearing::Running => asked(ui).child(said("Queuing…".into(), c.dim)),
            Clearing::Done => asked(ui).child(said("Queued".into(), c.dim)),
            Clearing::Refused(why) => asked(ui).child(said(format!("Not queued: {why}"), c.red)),
        })
    }

    /// Under a queued card being dropped: a field for why, which may stay empty, with Cancel and
    /// Drop (↩ in the field drops, Esc cancels), and on a draft that it is committed first; then
    /// that it is being dropped, or why it was not. The question goes once the board no longer
    /// has the card to drop.
    fn dropping(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> Option<Div> {
        let c = self.colors;
        Some(match self.drops.get(&card.file)? {
            Clearing::Asking => {
                let reason = self.reason.as_ref().filter(|r| r.file == card.file)?;
                if !self.board.as_ref()?.droppable(card) {
                    return None;
                }
                let (file, entered, leave, left) = (
                    card.file.clone(),
                    card.file.clone(),
                    card.file.clone(),
                    card.file.clone(),
                );
                let field = div()
                    .id("kanban-drop-reason")
                    .flex_1()
                    .min_w(ui.px(120.0))
                    .h(ui.px(20.0))
                    .flex()
                    .items_center()
                    .px(ui.px(6.0))
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(c.rule)
                    .overflow_hidden()
                    .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                        match event.keystroke.key.as_str() {
                            "enter" => view.drop_task(&entered, window, cx),
                            "escape" => {
                                view.take_back(&left, window, cx);
                                cx.notify();
                            }
                            _ => return,
                        }
                        cx.stop_propagation();
                    }))
                    .child(div().flex_1().min_w(px(0.0)).child(reason.input.clone()));
                let cancel = answer("kanban-drop-cancel", "Cancel", ui)
                    .border_color(c.rule)
                    .text_color(c.muted)
                    .hover(move |style| style.bg(c.text.opacity(0.09)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if view.take_back(&leave, window, cx) {
                            cx.notify();
                        }
                    }));
                let confirm = answer("kanban-drop-confirm", "Drop", ui)
                    .bg(c.red.opacity(0.16))
                    .border_color(c.red.opacity(0.5))
                    .text_color(c.red)
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(move |style| style.bg(c.red.opacity(0.26)))
                    .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        view.drop_task(&file, window, cx);
                    }));
                let first = card
                    .draft
                    .then(|| said("Commits this draft to main first.".into(), c.text));
                asked(ui)
                    .children(first)
                    .child(field)
                    // The two buttons stay together when the row wraps.
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .gap(ui.px(6.0))
                            .child(cancel)
                            .child(confirm),
                    )
            }
            Clearing::Running => asked(ui).child(said("Dropping\u{2026}".into(), c.dim)),
            Clearing::Done => asked(ui).child(said("Dropped".into(), c.dim)),
            Clearing::Refused(why) => asked(ui).child(said(format!("Not dropped: {why}"), c.red)),
        })
    }
}

/// The line under a card that asks about deleting or dropping it, or says how that went.
fn asked(ui: &UiFont) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(ui.px(6.0))
        .min_w(px(0.0))
}

/// Words on that line, wrapping where they must.
fn said(words: String, color: Hsla) -> Div {
    div()
        .flex_shrink(1.0)
        .min_w(px(0.0))
        .whitespace_normal()
        .text_color(color)
        .child(words)
}

/// A button on that line: an answer to what it asks.
fn answer(id: &'static str, words: &'static str, ui: &UiFont) -> Stateful<Div> {
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
}

/// Moves the file at `path` to the system's Trash, where it can be put back from; or says why it
/// could not be.
fn trash(path: &Path) -> Result<(), String> {
    let Some(path) = path.to_str() else {
        return Err("its path isn't valid UTF-8".to_owned());
    };
    objc2::rc::autoreleasepool(|_| {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        NSFileManager::defaultManager()
            .trashItemAtURL_resultingItemURL_error(&url, None)
            .map_err(|error| error.localizedDescription().to_string())
    })
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
    /// How far below its place the panel is drawn, as it comes in; the window says (P5-148a).
    rise: f32,
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
        faded: theme.is_faded(),
    }
}

impl TaskDialog {
    /// Drawn `rise` points below its place, as it comes in over the window (P5-148a).
    pub fn set_rise(&mut self, rise: f32) {
        self.rise = rise;
    }

    /// New colours: the window's, faded or not (P5-162).
    pub fn set_theme(&mut self, theme: Rc<Theme>, cx: &mut Context<Self>) {
        let colors = input_colors(&theme);
        for input in [&self.id, &self.title, &self.body] {
            input.update(cx, |input, cx| input.set_colors(colors, cx));
        }
        self.theme = theme;
        cx.notify();
    }

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
            rise: 0.0,
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
                color: gpui::black().opacity(theme.shade(0.55)),
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
            .bg(gpui::black().opacity(theme.shade(0.45)))
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
            .pt(px(top + self.rise))
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

    /// The body read as Markdown, as [`markdown::blocks`] has it and `markdown_view` draws it.
    fn preview(&self, ui: &UiFont, cx: &App) -> Div {
        let colors = markdown_view::Colors::of(&self.theme);
        let blocks = markdown::blocks(self.body.read(cx).text());
        let out = markdown_view::blocks(&blocks, &self.mono, colors, ui);
        if blocks.is_empty() {
            return out.child(div().text_color(colors.marker).child("Nothing to preview"));
        }
        out
    }
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

/// What a task waits on the user for, on one line after Needs you: whole if it fits in `room`, by
/// `width`, otherwise cut to it and ended with `…`.
fn asks_words(asks: &str, room: f32, width: &dyn Fn(&str) -> f32) -> String {
    card::elide(asks, &|cut| width(cut) <= room)
}

/// The ring before a task's id and in its group's header (P5-114): its shape follows the column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// Queued and committed.
    Open,
    /// Queued, a draft.
    Dashed,
    /// In progress: its right half filled.
    Half,
    /// To review: three quarters filled.
    Most,
    /// Merged or done: filled, a tick cut out of it.
    Tick,
    /// Dropped: struck through.
    Struck,
}

/// Which of the theme's colours a ring is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Red,
    Muted,
    Dim,
    Blue,
    Yellow,
    Green,
}

/// A task's ring, by its column, whether it is a draft, the priority it is marked for (high or low,
/// on a queued task, as [`Card::stripe`]) and whether it was dropped. A group's header is drawn as
/// a committed medium task in it.
fn ring(column: Column, draft: bool, priority: Option<Priority>, dropped: bool) -> (Shape, Role) {
    match column {
        Column::Queued => (
            if draft { Shape::Dashed } else { Shape::Open },
            match priority {
                Some(Priority::High) => Role::Red,
                Some(Priority::Low) => Role::Dim,
                _ => Role::Muted,
            },
        ),
        Column::InProgress => (Shape::Half, Role::Blue),
        Column::ToReview => (Shape::Most, Role::Yellow),
        Column::Merged => (Shape::Tick, Role::Green),
        Column::Done if dropped => (Shape::Struck, Role::Dim),
        Column::Done => (Shape::Tick, Role::Dim),
    }
}

/// What the narrow list shows, top to bottom (P5-185): each column as its group, but those of In
/// progress, To review and Merged that list nothing folded into one line, where the first of them
/// was. `empty` tells whether a column lists nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Group(Column),
    Zeros(Vec<Column>),
}

fn entries(empty: impl Fn(Column) -> bool) -> Vec<Entry> {
    let folds = |column: Column| !matches!(column, Column::Queued | Column::Done) && empty(column);
    let zeros: Vec<Column> = Column::ALL.into_iter().filter(|c| folds(*c)).collect();
    let mut entries = Vec::new();
    for column in Column::ALL {
        if !folds(column) {
            entries.push(Entry::Group(column));
        } else if zeros.first() == Some(&column) {
            entries.push(Entry::Zeros(zeros.clone()));
        }
    }
    entries
}

/// A small tag at the end of a queued task's row in the list (P5-185).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Draft,
    High,
    Low,
}

/// Draft on a draft, then High or Low; nothing for a committed medium task, nor in another column.
fn tags(column: Column, draft: bool, priority: Option<Priority>) -> Vec<Tag> {
    if column != Column::Queued {
        return Vec::new();
    }
    let priority = match priority {
        Some(Priority::High) => Some(Tag::High),
        Some(Priority::Low) => Some(Tag::Low),
        _ => None,
    };
    draft
        .then_some(Tag::Draft)
        .into_iter()
        .chain(priority)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_groups_with_nothing_fold_into_one_line_where_the_first_was() {
        use Column::*;
        use Entry::*;
        let with = |empty: &'static [Column]| entries(move |column| empty.contains(&column));
        // All three empty: one line between Queued and DONE.
        assert_eq!(
            with(&[InProgress, ToReview, Merged]),
            [
                Group(Queued),
                Zeros(vec![InProgress, ToReview, Merged]),
                Group(Done)
            ]
        );
        // Some empty: those in the line, in order, where the first of them stood.
        assert_eq!(
            with(&[ToReview, Merged]),
            [
                Group(Queued),
                Group(InProgress),
                Zeros(vec![ToReview, Merged]),
                Group(Done)
            ]
        );
        assert_eq!(
            with(&[InProgress, ToReview]),
            [
                Group(Queued),
                Zeros(vec![InProgress, ToReview]),
                Group(Merged),
                Group(Done)
            ]
        );
        assert_eq!(
            with(&[InProgress, Merged]),
            [
                Group(Queued),
                Zeros(vec![InProgress, Merged]),
                Group(ToReview),
                Group(Done)
            ]
        );
        // None empty: no line.
        assert_eq!(
            with(&[]),
            Column::ALL.into_iter().map(Group).collect::<Vec<_>>()
        );
        // Queued and DONE never fold in, empty or not.
        assert_eq!(
            with(&[Queued, InProgress, Done]),
            [
                Group(Queued),
                Zeros(vec![InProgress]),
                Group(ToReview),
                Group(Merged),
                Group(Done)
            ]
        );
    }

    #[test]
    fn a_queued_row_ends_with_draft_then_high_or_low_and_nothing_for_medium() {
        use Tag::*;
        let q = Column::Queued;
        for (draft, before) in [(false, vec![]), (true, vec![Draft])] {
            let and = |tag: Option<Tag>| before.iter().copied().chain(tag).collect::<Vec<_>>();
            assert_eq!(tags(q, draft, Some(Priority::High)), and(Some(High)));
            assert_eq!(tags(q, draft, Some(Priority::Medium)), and(None));
            assert_eq!(tags(q, draft, None), and(None));
            assert_eq!(tags(q, draft, Some(Priority::Low)), and(Some(Low)));
        }
        // Rows in the other groups: none.
        assert!(tags(Column::InProgress, false, Some(Priority::High)).is_empty());
        assert!(tags(Column::Done, false, Some(Priority::Low)).is_empty());
    }

    #[test]
    fn a_ring_shows_the_column_and_on_a_queued_task_its_priority() {
        use {Role::*, Shape::*};
        let q = Column::Queued;
        // Queued: open, dashed for a draft, in the priority's colour, medium muted.
        assert_eq!(ring(q, false, None, false), (Open, Muted));
        assert_eq!(ring(q, false, Some(Priority::High), false), (Open, Red));
        assert_eq!(ring(q, false, Some(Priority::Low), false), (Open, Dim));
        assert_eq!(ring(q, true, None, false), (Dashed, Muted));
        assert_eq!(ring(q, true, Some(Priority::High), false), (Dashed, Red));
        assert_eq!(ring(q, true, Some(Priority::Low), false), (Dashed, Dim));
        // The other columns, each its own.
        assert_eq!(ring(Column::InProgress, false, None, false), (Half, Blue));
        assert_eq!(ring(Column::ToReview, false, None, false), (Most, Yellow));
        assert_eq!(ring(Column::Merged, false, None, false), (Tick, Green));
        assert_eq!(ring(Column::Done, false, None, false), (Tick, Dim));
        // Dropped: struck through, grey.
        assert_eq!(ring(Column::Done, false, None, true), (Struck, Dim));
    }

    #[test]
    fn what_the_user_is_waited_on_for_is_cut_to_one_line_with_an_ellipsis() {
        let width = |text: &str| text.chars().count() as f32;
        let asks = "远端要先有能用的 corral，归 ranch 的 R10";
        // It fits: whole.
        assert_eq!(asks_words(asks, 40.0, &width), asks);
        // It does not: cut to the room and ended with the ellipsis, never `..`.
        let cut = asks_words(asks, 12.0, &width);
        assert!(cut.ends_with('…') && !cut.contains(".."), "{cut:?}");
        assert!(width(&cut) <= 12.0, "{cut:?}");
        assert!(asks.starts_with(cut.trim_end_matches('…')));
        // No room at all: just the ellipsis.
        assert_eq!(asks_words(asks, 0.5, &width), "…");
    }

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
