//! The right sidebar's Changes tab: the focused pane's worktree as Git sees it, read in the
//! background every moment or two while the tab is showing, and drawn as a file index followed by
//! every file's diff in one scrolling list; widened, a file tree beside it and a side-by-side
//! layout to choose. Only Git's read-only commands are run (`diff.rs`).
use crate::{
    diff::{self, File, Kind, Line, Note, Read, Scope, Status},
    fonts::UiFont,
    highlight::{self, Span, Stream, Token},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, EventEmitter, Font, FontStyle, FontWeight,
    HighlightStyle, Hsla, IntoElement, ListAlignment, ListOffset, ListState, PathBuilder, Pixels,
    Point, Render, ScrollWheelEvent, Stateful, StyledText, Window, canvas, div, list, point,
    prelude::*, px,
};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// How often the worktree is read again while the tab shows.
const EVERY: Duration = Duration::from_millis(1500);
/// How often a finished read or a due one is looked for.
const TICK: Duration = Duration::from_millis(120);
/// Past this many changed lines a file's diff waits to be asked for.
pub const LARGE: u32 = 1500;
/// From this width, at the base interface size, the tab shows a file tree and offers Split.
pub const WIDE: f32 = 680.0;
/// The file tree's width when widened, at the base interface size.
const TREE: f32 = 230.0;
/// Lines highlighted ahead of the one drawn, so scrolling finds them done.
const AHEAD: usize = 48;

/// Whose changes to show: the focused pane's name, its status colour and its directory.
#[derive(Clone, Debug, PartialEq)]
pub struct Follow {
    pub name: String,
    pub dot: Hsla,
    /// `None` for a pane with no directory to read.
    pub cwd: Option<String>,
}

/// What the window tells the tab every time it draws.
pub struct Frame {
    pub follow: Option<Follow>,
    /// The right sidebar is open on this tab: read, and keep reading.
    pub active: bool,
    /// The sidebar's width, in points.
    pub width: f32,
    pub theme: Rc<Theme>,
    pub mono: Font,
}

/// The scope or the layout changed: the window saves them with the layout.
pub struct Changed;

impl EventEmitter<Changed> for ChangesView {}

type Key = (String, Scope);

struct Pending {
    key: Key,
    cancel: Arc<AtomicBool>,
    out: Arc<Mutex<Option<Option<Read>>>>,
}

/// One row of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// A file in the index above the diffs.
    Index(usize),
    Head(usize),
    /// What a file shows instead of its lines.
    Note(usize),
    Hunk(usize, usize),
    /// Unchanged lines not shown, before a hunk or after the last.
    Gap(usize, usize),
    Line(usize, usize, usize),
    /// Side by side: the old line and the new.
    Pair(usize, usize, Option<usize>, Option<usize>),
    /// A line of a gap opened up.
    Extra(usize, usize, usize),
    /// The space after a file's last line.
    Foot(usize),
    End,
}

impl Row {
    fn file(self) -> Option<usize> {
        match self {
            Row::Head(f)
            | Row::Note(f)
            | Row::Hunk(f, _)
            | Row::Gap(f, _)
            | Row::Line(f, ..)
            | Row::Pair(f, ..)
            | Row::Extra(f, ..)
            | Row::Foot(f) => Some(f),
            Row::Index(_) | Row::End => None,
        }
    }
}

/// Unchanged lines between hunks: before hunk `at`, or after the last when `at` is past them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    pub at: usize,
    /// The new side's lines, first and one past the last.
    pub new: (u32, u32),
    /// The old side's first.
    pub old: u32,
}

/// The gaps around a file's hunks; the last needs the file's length now.
pub fn gaps(file: &File) -> Vec<Gap> {
    let mut gaps = Vec::new();
    let mut next = (1, 1);
    for (at, hunk) in file.hunks.iter().enumerate() {
        let (first, end) = hunk.new_lines();
        if first > next.1 {
            gaps.push(Gap {
                at,
                new: (next.1, first),
                old: next.0,
            });
        }
        next = (hunk.old_lines().1, end);
    }
    if let Some(total) = file.lines_now
        && !file.hunks.is_empty()
        && total + 1 > next.1
    {
        gaps.push(Gap {
            at: file.hunks.len(),
            new: (next.1, total + 1),
            old: next.0,
        });
    }
    gaps
}

/// A block of lines being coloured, a hunk's or an opened gap's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Block {
    Hunk(usize),
    Gap(usize),
}

/// A block's colours, worked out as far as has been drawn.
struct Paint {
    old: Stream,
    new: Stream,
    spans: Vec<Vec<Span>>,
    /// The changed words of lines that face another, by line.
    words: HashMap<usize, Vec<Range<usize>>>,
    facing: HashMap<usize, usize>,
}

impl Paint {
    fn new(file: &File, lines: &[Line]) -> Self {
        let first = (file.hunks.first().is_some_and(|h| h.new_start <= 1))
            .then(|| lines.first().map(|l| l.text.as_str()))
            .flatten();
        let syntax = highlight::syntax_for(&file.path, first);
        let mut facing = HashMap::new();
        for (a, b) in highlight::pairs(lines) {
            facing.insert(a, b);
            facing.insert(b, a);
        }
        Paint {
            old: Stream::new(syntax),
            new: Stream::new(syntax),
            spans: Vec::new(),
            words: HashMap::new(),
            facing,
        }
    }

    /// Colours up to line `to` of `lines`, and its changed words.
    fn reach(&mut self, lines: &[Line], to: usize) {
        let to = (to + AHEAD).min(lines.len().saturating_sub(1));
        while self.spans.len() <= to && self.spans.len() < lines.len() {
            let line = &lines[self.spans.len()];
            let spans = match line.kind {
                Kind::Added => self.new.line(&line.text),
                Kind::Deleted => self.old.line(&line.text),
                Kind::Context => {
                    self.old.line(&line.text);
                    self.new.line(&line.text)
                }
            };
            self.spans.push(spans);
        }
    }

    fn words(&mut self, lines: &[Line], i: usize) -> &[Range<usize>] {
        if !self.words.contains_key(&i) {
            let (gone, came) = match self.facing.get(&i) {
                Some(&j) => {
                    let (old, new) = if lines[i].kind == Kind::Deleted {
                        (i, j)
                    } else {
                        (j, i)
                    };
                    let words = highlight::changed_words(&lines[old].text, &lines[new].text)
                        .unwrap_or_default();
                    if lines[i].kind == Kind::Deleted {
                        (words.0, words.1)
                    } else {
                        (words.1, words.0)
                    }
                }
                None => (Vec::new(), Vec::new()),
            };
            self.words.insert(i, gone);
            if let Some(&j) = self.facing.get(&i) {
                self.words.insert(j, came);
            }
        }
        &self.words[&i]
    }
}

/// The theme's colours the tab uses.
#[derive(Clone, Copy)]
struct Colors {
    ground: Hsla,
    text: Hsla,
    code: Hsla,
    muted: Hsla,
    dim: Hsla,
    faint: Hsla,
    rule: Hsla,
    selected: Hsla,
    green: Hsla,
    red: Hsla,
    yellow: Hsla,
    blue: Hsla,
    accent: Hsla,
    purple: Hsla,
    orange: Hsla,
}

impl Colors {
    fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        Colors {
            ground: hsla(theme.terminal().background, 1.0),
            text: fg(|t| t.agents_text),
            code: fg(|t| t.agents_branch),
            muted: fg(|t| t.agents_dim),
            dim: fg(|t| t.agents_dimmer),
            faint: fg(|t| t.agents_faint),
            rule: fg(|t| t.agents_rule),
            selected: hsla(theme.bg(|t| t.agent_selected), 1.0),
            green: fg(|t| t.agents_green),
            red: fg(|t| t.agents_red),
            yellow: fg(|t| t.agents_yellow),
            blue: fg(|t| t.agents_blue),
            accent: fg(|t| t.agents_accent),
            purple: fg(|t| t.agents_purple),
            orange: fg(|t| t.agent_stalled),
        }
    }

    fn token(&self, token: Token) -> Hsla {
        match token {
            Token::Plain => self.code,
            Token::Keyword => self.purple,
            Token::Function => self.blue,
            Token::Type => self.accent,
            Token::String => self.yellow,
            Token::Number => self.orange,
            Token::Comment => self.dim,
            Token::Punctuation => self.muted,
        }
    }

    fn status(&self, status: Status) -> Hsla {
        match status {
            Status::Modified => self.yellow,
            Status::Added => self.green,
            Status::Deleted => self.red,
            Status::Renamed => self.blue,
        }
    }
}

pub struct ChangesView {
    program: String,
    theme: Rc<Theme>,
    colors: Colors,
    mono: Font,
    follow: Option<Follow>,
    active: bool,
    width: f32,
    pub scope: Scope,
    /// Side by side when widened; the narrow tab is always unified.
    pub split: bool,
    shown: Option<(Key, Rc<Read>)>,
    /// The branch and its base, as last read, by directory.
    place: Option<(String, String, Option<String>)>,
    pending: Option<Pending>,
    /// When the last read finished, or `None` to read at once.
    last: Option<Instant>,
    /// Files folded or unfolded by hand, by path; unfolded otherwise.
    folded: HashMap<String, bool>,
    /// Large and deleted files whose diffs were asked for.
    revealed: HashSet<String>,
    /// Gaps opened, by path and their first new line.
    opened: HashSet<(String, u32)>,
    /// How far each file is scrolled sideways, by path.
    scroll_x: HashMap<String, f32>,
    // Worked out from `shown` and the layout:
    rows: Vec<Row>,
    gaps: Vec<Vec<Gap>>,
    extras: HashMap<(usize, usize), Rc<Vec<Line>>>,
    widest: Vec<usize>,
    paints: HashMap<(usize, Block), Paint>,
    list: ListState,
    /// The layout the rows were made for: widened, side by side.
    built: (bool, bool),
    /// The code font's advance at the code size, in pixels.
    advance: f32,
    /// The width from which it is widened, at this interface size.
    wide_at: f32,
}

impl ChangesView {
    pub fn new(
        program: String,
        theme: Rc<Theme>,
        mono: Font,
        scope: Scope,
        split: bool,
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
        ChangesView {
            program,
            colors: Colors::of(&theme),
            theme,
            mono,
            follow: None,
            active: false,
            width: 0.0,
            scope,
            split,
            shown: None,
            place: None,
            pending: None,
            last: None,
            folded: HashMap::new(),
            revealed: HashSet::new(),
            opened: HashSet::new(),
            scroll_x: HashMap::new(),
            rows: Vec::new(),
            gaps: Vec::new(),
            extras: HashMap::new(),
            widest: Vec::new(),
            paints: HashMap::new(),
            list: ListState::new(0, ListAlignment::Top, px(400.0)),
            built: (false, false),
            advance: 7.0,
            wide_at: WIDE,
        }
    }

    /// The window's latest: redraws only when something here changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        if frame.follow != self.follow {
            let old_cwd = self.follow.as_ref().and_then(|f| f.cwd.clone());
            let new_cwd = frame.follow.as_ref().and_then(|f| f.cwd.clone());
            if old_cwd != new_cwd {
                self.cancel();
                self.last = None;
                // Another worktree: its files are folded and scrolled afresh.
                self.folded.clear();
                self.revealed.clear();
                self.opened.clear();
                self.scroll_x.clear();
            }
            self.follow = frame.follow;
            changed = true;
        }
        if frame.active != self.active {
            self.active = frame.active;
            if !self.active {
                self.cancel();
            } else {
                self.last = None;
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
            self.list.remeasure();
            changed = true;
        }
        let wide_at = UiFont::get(cx).scale(WIDE).max(WIDE);
        if wide_at != self.wide_at {
            // Another interface size: the rows are another height.
            self.wide_at = wide_at;
            self.list.remeasure();
            changed = true;
        }
        if self.layout() != self.built {
            let anchor = self.anchor();
            self.rebuild();
            if let Some(anchor) = anchor {
                self.restore(anchor);
            }
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Widened, and side by side.
    fn layout(&self) -> (bool, bool) {
        let wide = self.width >= self.wide_at;
        (wide, wide && self.split)
    }

    fn key(&self) -> Option<Key> {
        let cwd = self.follow.as_ref()?.cwd.clone()?;
        Some((cwd, self.scope))
    }

    fn cancel(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Takes a finished read and starts one when due.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = &self.pending {
            let done = pending.out.lock().ok().and_then(|mut out| out.take());
            if let Some(read) = done {
                let key = pending.key.clone();
                self.pending = None;
                self.last = Some(Instant::now());
                if let Some(read) = read
                    && Some(&key) == self.key().as_ref()
                {
                    self.accept(key, read, cx);
                }
            }
        }
        if self.pending.is_some() || !self.active {
            return;
        }
        let Some(key) = self.key() else {
            return;
        };
        let shown = self.shown.as_ref().map(|(k, _)| k);
        let due = shown != Some(&key) || self.last.is_none_or(|last| last.elapsed() >= EVERY);
        if !due {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let out = Arc::new(Mutex::new(None));
        let (program, (cwd, scope)) = (self.program.clone(), key.clone());
        let (stop, put) = (cancel.clone(), out.clone());
        let started = thread::Builder::new()
            .name("paddock-changes".into())
            .spawn(move || {
                let read = diff::read(&program, &cwd, scope, &stop);
                // The grammars load here, off the interface's thread, before the first lines are drawn.
                highlight::syntaxes();
                if let Ok(mut out) = put.lock() {
                    *out = Some(read);
                }
            });
        if started.is_ok() {
            self.pending = Some(Pending { key, cancel, out });
        }
    }

    /// A read came back: shown if it differs from what is.
    fn accept(&mut self, key: Key, read: Read, cx: &mut Context<Self>) {
        if let Some((shown, old)) = &self.shown
            && *shown == key
            && **old == read
        {
            return;
        }
        match &read {
            Read::Changes(changes) => {
                self.place = Some((key.0.clone(), changes.head.clone(), changes.base.clone()))
            }
            Read::NoBase { head } => self.place = Some((key.0.clone(), head.clone(), None)),
            Read::NotRepository => self.place = None,
            Read::Failed(_) => {}
        }
        let same_place = self.shown.as_ref().is_some_and(|(k, _)| *k == key);
        let anchor = same_place.then(|| self.anchor()).flatten();
        self.shown = Some((key, Rc::new(read)));
        self.rebuild();
        match anchor {
            Some(anchor) => self.restore(anchor),
            None => self.list.scroll_to(ListOffset::default()),
        }
        cx.notify();
    }

    /// The files shown, if any.
    fn changes(&self) -> Option<Rc<Read>> {
        let (key, read) = self.shown.as_ref()?;
        (Some(key) == self.key().as_ref()).then(|| read.clone())
    }

    /// Where the list is scrolled to: the file at the top, how many rows into it, and how far
    /// into that row.
    fn anchor(&self) -> Option<(String, usize, Pixels)> {
        let top = self.list.logical_scroll_top();
        let read = self.changes()?;
        let Read::Changes(changes) = &*read else {
            return None;
        };
        let file = self.rows.get(top.item_ix)?.file()?;
        let head = self.rows.iter().position(|r| *r == Row::Head(file))?;
        Some((
            changes.files[file].path.clone(),
            top.item_ix - head,
            top.offset_in_item,
        ))
    }

    fn restore(&mut self, (path, rows_in, offset): (String, usize, Pixels)) {
        let Some(read) = self.changes() else { return };
        let Read::Changes(changes) = &*read else {
            return;
        };
        let Some(file) = changes.files.iter().position(|f| f.path == path) else {
            return;
        };
        let Some(head) = self.rows.iter().position(|r| *r == Row::Head(file)) else {
            return;
        };
        let end = self.rows[head..]
            .iter()
            .position(|r| r.file() != Some(file))
            .map_or(self.rows.len(), |n| head + n);
        let item_ix = (head + rows_in).min(end.saturating_sub(1));
        self.list.scroll_to(ListOffset {
            item_ix,
            offset_in_item: if item_ix == head + rows_in {
                offset
            } else {
                px(0.0)
            },
        });
    }

    fn is_folded(&self, file: &File) -> bool {
        self.folded.get(&file.path).copied().unwrap_or(false)
    }

    /// Its lines wait to be asked for: deleted, large or generated.
    fn waits(&self, file: &File) -> bool {
        !file.hunks.is_empty()
            && !self.revealed.contains(&file.path)
            && (file.status == Status::Deleted || file.changed() > LARGE || file.generated())
    }

    /// The rows for what is shown, in the layout the tab has now.
    fn rebuild(&mut self) {
        let old = self.rows.len();
        self.rows.clear();
        self.gaps.clear();
        self.extras.clear();
        self.widest.clear();
        self.paints.clear();
        let (wide, split) = self.layout();
        self.built = (wide, split);
        if let Some(read) = self.changes()
            && let Read::Changes(changes) = &*read
            && !changes.files.is_empty()
        {
            if !wide {
                self.rows.extend((0..changes.files.len()).map(Row::Index));
            }
            for (f, file) in changes.files.iter().enumerate() {
                let gaps = gaps(file);
                self.widest.push(
                    file.hunks
                        .iter()
                        .flat_map(|h| &h.lines)
                        .map(|l| l.text.chars().count())
                        .max()
                        .unwrap_or(0),
                );
                self.rows.push(Row::Head(f));
                if self.is_folded(file) {
                    self.gaps.push(gaps);
                    continue;
                }
                if self.waits(file) || file.hunks.is_empty() {
                    self.rows.push(Row::Note(f));
                    self.gaps.push(gaps);
                    continue;
                }
                for (h, hunk) in file.hunks.iter().enumerate() {
                    self.gap_rows(&changes.top, file, f, &gaps, h);
                    self.rows.push(Row::Hunk(f, h));
                    if split {
                        self.rows.extend(
                            sides(&hunk.lines)
                                .into_iter()
                                .map(|(a, b)| Row::Pair(f, h, a, b)),
                        );
                    } else {
                        self.rows
                            .extend((0..hunk.lines.len()).map(|i| Row::Line(f, h, i)));
                    }
                }
                self.gap_rows(&changes.top, file, f, &gaps, file.hunks.len());
                self.rows.push(Row::Foot(f));
                self.gaps.push(gaps);
            }
            self.rows.push(Row::End);
        }
        self.list.splice(0..old, self.rows.len());
    }

    /// The unchanged lines before hunk `at` (or after the last): a row to open them, or the
    /// lines themselves once opened and readable.
    fn gap_rows(&mut self, top: &str, file: &File, f: usize, gaps: &[Gap], at: usize) {
        let Some(g) = gaps.iter().position(|gap| gap.at == at) else {
            return;
        };
        let gap = gaps[g];
        if self.opened.contains(&(file.path.clone(), gap.new.0))
            && let Some(lines) = unchanged(top, file, gap)
        {
            self.rows
                .extend((0..lines.len()).map(|i| Row::Extra(f, g, i)));
            self.extras.insert((f, g), Rc::new(lines));
        } else {
            self.rows.push(Row::Gap(f, g));
        }
    }

    /// The rows made again for a change of folding or layout, keeping the place.
    fn relayout(&mut self, cx: &mut Context<Self>) {
        let anchor = self.anchor();
        self.rebuild();
        if let Some(anchor) = anchor {
            self.restore(anchor);
        }
        cx.notify();
    }

    fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        if scope == self.scope {
            return;
        }
        self.scope = scope;
        self.cancel();
        self.last = None;
        self.rebuild();
        self.list.scroll_to(ListOffset::default());
        cx.emit(Changed);
        cx.notify();
    }

    fn set_split(&mut self, split: bool, cx: &mut Context<Self>) {
        if split == self.split {
            return;
        }
        self.split = split;
        cx.emit(Changed);
        self.relayout(cx);
    }

    fn files(&self) -> Vec<String> {
        match self.changes().as_deref() {
            Some(Read::Changes(changes)) => changes.files.iter().map(|f| f.path.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Folds every file, or unfolds them all when every one is folded.
    fn fold_all(&mut self, cx: &mut Context<Self>) {
        let paths = self.files();
        let any_open = paths
            .iter()
            .any(|p| !self.folded.get(p).copied().unwrap_or(false));
        for path in paths {
            self.folded.insert(path, any_open);
        }
        self.rebuild();
        self.list.scroll_to(ListOffset::default());
        cx.notify();
    }

    fn any_open(&self) -> bool {
        match self.changes().as_deref() {
            Some(Read::Changes(changes)) => changes.files.iter().any(|f| !self.is_folded(f)),
            _ => false,
        }
    }

    /// Scrolls to file `f`, unfolded.
    fn jump(&mut self, f: usize, cx: &mut Context<Self>) {
        let Some(path) = self.files().get(f).cloned() else {
            return;
        };
        if self.folded.insert(path, false) == Some(true) {
            self.rebuild();
        }
        if let Some(head) = self.rows.iter().position(|r| *r == Row::Head(f)) {
            self.list.scroll_to(ListOffset {
                item_ix: head,
                offset_in_item: px(0.0),
            });
        }
        cx.notify();
    }

    fn toggle(&mut self, f: usize, cx: &mut Context<Self>) {
        let Some(path) = self.files().get(f).cloned() else {
            return;
        };
        let folded = self.folded.get(&path).copied().unwrap_or(false);
        self.folded.insert(path, !folded);
        let top = self.list.logical_scroll_top();
        let head = self.rows.iter().position(|r| *r == Row::Head(f));
        self.relayout(cx);
        // Folded from its header pinned at the top: the files after it come up under it.
        if let Some(head) = head
            && head < top.item_ix
        {
            self.list.scroll_to(ListOffset {
                item_ix: head,
                offset_in_item: px(0.0),
            });
        }
    }

    fn reveal(&mut self, f: usize, cx: &mut Context<Self>) {
        if let Some(path) = self.files().get(f).cloned() {
            self.revealed.insert(path);
            self.relayout(cx);
        }
    }

    fn open_gap(&mut self, f: usize, g: usize, cx: &mut Context<Self>) {
        let Some(path) = self.files().get(f).cloned() else {
            return;
        };
        if let Some(gap) = self.gaps.get(f).and_then(|gaps| gaps.get(g)) {
            self.opened.insert((path, gap.new.0));
            self.relayout(cx);
        }
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        self.last = None;
        self.tick(cx);
    }

    /// The file at the top of the list, whose header stays pinned there, and how far the next
    /// header pushes it up.
    fn pinned(&self, head: f32) -> Option<(usize, f32)> {
        let top = self.list.logical_scroll_top();
        let row = *self.rows.get(top.item_ix)?;
        let file = row.file()?;
        if row == Row::Head(file) && top.offset_in_item <= px(0.0) {
            return None;
        }
        let next = self.rows[top.item_ix + 1..]
            .iter()
            .position(|r| matches!(r, Row::Head(_)))
            .map(|n| top.item_ix + 1 + n);
        let shift = next
            .and_then(|next| self.list.bounds_for_item(next))
            .map(|bounds| {
                let below = f32::from(bounds.top() - self.list.viewport_bounds().top());
                (below - head).min(0.0)
            })
            .unwrap_or(0.0);
        Some((file, shift))
    }

    /// How far file `f` can scroll sideways.
    fn reach_x(&self, f: usize, ui: &UiFont, digits: usize) -> f32 {
        let (wide, split) = self.layout();
        let mut width = self.width - 2.0;
        if wide {
            width -= ui.scale(TREE);
        }
        let number = self.number_width(ui, digits);
        let code = if split {
            width / 2.0 - number - 1.0
        } else {
            width - 2.0 * number - ui.scale(SIGN)
        };
        let content =
            self.widest.get(f).copied().unwrap_or(0) as f32 * self.advance + ui.scale(16.0);
        (content - code).max(0.0)
    }

    fn number_width(&self, ui: &UiFont, digits: usize) -> f32 {
        digits.max(3) as f32 * self.advance * 11.0 / 12.0 + ui.scale(10.0)
    }
}

/// What a segment of a segmented control does when clicked.
type Choose = Box<dyn Fn(&mut ChangesView, &mut Context<ChangesView>)>;

/// The sign column's width.
const SIGN: f32 = 12.0;

/// A hunk's lines side by side: context on both sides, each run of deletions beside the
/// additions after it, the longer run against blanks.
pub fn sides(lines: &[Line]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut rows = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind == Kind::Context {
            rows.push((Some(i), Some(i)));
            i += 1;
            continue;
        }
        let deleted = i;
        while i < lines.len() && lines[i].kind == Kind::Deleted {
            i += 1;
        }
        let added = i;
        while i < lines.len() && lines[i].kind == Kind::Added {
            i += 1;
        }
        let (dels, adds) = (added - deleted, i - added);
        for k in 0..dels.max(adds) {
            rows.push((
                (k < dels).then_some(deleted + k),
                (k < adds).then_some(added + k),
            ));
        }
    }
    rows
}

/// A gap's lines from the file as it is now, numbered on both sides.
fn unchanged(top: &str, file: &File, gap: Gap) -> Option<Vec<Line>> {
    let bytes = std::fs::read(std::path::Path::new(top).join(&file.path)).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let all: Vec<&str> = text.lines().collect();
    let (first, end) = (gap.new.0 as usize, gap.new.1 as usize);
    let lines = all.get(first.checked_sub(1)?..end.checked_sub(1)?)?;
    Some(
        lines
            .iter()
            .enumerate()
            .map(|(k, text)| Line {
                kind: Kind::Context,
                old: gap.old + k as u32,
                new: gap.new.0 + k as u32,
                text: diff::untab(text.trim_end_matches('\r')),
            })
            .collect(),
    )
}

/// A file's widest line number.
fn digits(file: &File) -> usize {
    let last = file
        .hunks
        .iter()
        .map(|h| (h.old_start + h.old_len).max(h.new_start + h.new_len))
        .chain(file.lines_now)
        .max()
        .unwrap_or(1);
    last.to_string().len()
}

/// `1,234`.
pub fn count(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `12 KB`, `3.1 MB`.
pub fn size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{} KB", (bytes as f64 / 1024.0).round()),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

/// What a file shows instead of its lines, and the action offered, if any.
fn note_text(file: &File, waits: bool) -> (String, Option<&'static str>) {
    if waits {
        let text = if file.changed() > LARGE {
            format!("Large diff · {} lines", count(file.changed()))
        } else if file.generated() {
            "Generated file".into()
        } else {
            "Deleted file".into()
        };
        return (text, Some("Show diff"));
    }
    let text = match &file.note {
        Some(Note::Binary(before, after)) => match (before, after) {
            (Some(b), Some(a)) => format!("Binary file · {} → {}", size(*b), size(*a)),
            (None, Some(a)) if file.status == Status::Added => {
                format!("Binary file · {}", size(*a))
            }
            (Some(b), None) if file.status == Status::Deleted => {
                format!("Binary file · {}", size(*b))
            }
            _ => "Binary file".into(),
        },
        Some(Note::TooLarge(bytes)) => format!("Large file · {}", size(*bytes)),
        Some(Note::Mode(old, new)) => format!("Mode changed · {old} → {new}"),
        Some(Note::Empty) if file.status == Status::Added => "Empty file".into(),
        Some(Note::Empty) if file.status == Status::Deleted => "Deleted empty file".into(),
        _ if file.status == Status::Renamed => "Renamed without changes".into(),
        _ => "No changes to show".into(),
    };
    (text, None)
}

/// A file's name as listed: its directory, faint, and its name, or both names when renamed.
fn names(file: &File) -> (String, String) {
    let (dir, name) = file.dir_and_name();
    match &file.old_path {
        Some(old) => {
            let (old_dir, old_name) = match old.rfind('/') {
                Some(i) => old.split_at(i + 1),
                None => ("", old.as_str()),
            };
            if old_dir == dir {
                (dir.to_owned(), format!("{old_name} → {name}"))
            } else {
                (String::new(), format!("{old} → {}", file.path))
            }
        }
        None => (dir.to_owned(), name.to_owned()),
    }
}

/// The text's colours: each piece's, with the changed words on `word`.
fn styles(
    spans: &[Span],
    words: &[Range<usize>],
    colors: &Colors,
    word: Hsla,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut out = Vec::new();
    for span in spans {
        let style = HighlightStyle {
            color: Some(colors.token(span.token)),
            font_style: (span.token == Token::Comment).then_some(FontStyle::Italic),
            ..HighlightStyle::default()
        };
        let mut cuts: Vec<usize> = words
            .iter()
            .flat_map(|w| [w.start, w.end])
            .filter(|&c| c > span.range.start && c < span.range.end)
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts.push(span.range.end);
        let mut at = span.range.start;
        for cut in cuts {
            let inside = words.iter().any(|w| w.start <= at && cut <= w.end);
            out.push((
                at..cut,
                if inside {
                    HighlightStyle {
                        background_color: Some(word),
                        ..style
                    }
                } else {
                    style
                },
            ));
            at = cut;
        }
    }
    out
}

/// One side of a line: whose lines, which block, which line, and how it reads.
struct Side {
    f: usize,
    block: Block,
    i: usize,
}

impl ChangesView {
    /// Line `i` of a block, coloured.
    fn code(&mut self, read: &diff::Changes, side: &Side) -> (Line, StyledText) {
        let file = &read.files[side.f];
        let extra: Rc<Vec<Line>>;
        let lines: &[Line] = match side.block {
            Block::Hunk(h) => &file.hunks[h].lines,
            Block::Gap(g) => {
                extra = self.extras.get(&(side.f, g)).cloned().unwrap_or_default();
                &extra
            }
        };
        let colors = self.colors;
        let paint = self
            .paints
            .entry((side.f, side.block))
            .or_insert_with(|| Paint::new(file, lines));
        paint.reach(lines, side.i);
        let line = lines[side.i].clone();
        let spans = paint.spans.get(side.i).cloned().unwrap_or_default();
        let (words, word) = match line.kind {
            Kind::Context => (Vec::new(), colors.code),
            Kind::Added => (
                paint.words(lines, side.i).to_vec(),
                colors.green.opacity(0.28),
            ),
            Kind::Deleted => (
                paint.words(lines, side.i).to_vec(),
                colors.red.opacity(0.30),
            ),
        };
        let text = StyledText::new(line.text.clone())
            .with_highlights(styles(&spans, &words, &colors, word));
        (line, text)
    }

    fn row(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = UiFont::get(cx);
        let Some(&row) = self.rows.get(ix) else {
            return div().into_any_element();
        };
        let Some(read) = self.changes() else {
            return div().into_any_element();
        };
        let Read::Changes(changes) = &*read else {
            return div().into_any_element();
        };
        let c = self.colors;
        match row {
            Row::Index(f) => {
                let file = &changes.files[f];
                let last = f + 1 == changes.files.len();
                div()
                    .w_full()
                    .px(ui.px(8.0))
                    .when(f == 0, |d| d.pt(ui.px(6.0)))
                    .when(last, |d| d.pb(ui.px(8.0)))
                    .child(
                        self.file_item(file, ui.px(8.0), false, &ui)
                            .id(("changes-index", f))
                            .on_click(
                                cx.listener(move |this, _: &ClickEvent, _, cx| this.jump(f, cx)),
                            ),
                    )
                    .into_any_element()
            }
            Row::Head(f) => self
                .head(&changes.files[f], f, false, &ui, cx)
                .into_any_element(),
            Row::Note(f) => {
                let file = &changes.files[f];
                let waits = self.waits(file);
                let (text, action) = note_text(file, waits);
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .h(ui.px(30.0))
                    .mb(ui.px(6.0))
                    .pl(ui.px(40.0))
                    .pr(ui.px(14.0))
                    .text_size(ui.px(12.0))
                    .text_color(c.muted)
                    .whitespace_nowrap()
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(text),
                    )
                    .when_some(action, |d, action| {
                        d.child(div().text_color(c.faint).child("·")).child(
                            div()
                                .id(("changes-reveal", f))
                                .flex_shrink_0()
                                .cursor_pointer()
                                .text_color(c.accent)
                                .hover(|s| s.opacity(0.8))
                                .child(action)
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.reveal(f, cx)
                                })),
                        )
                    })
                    .into_any_element()
            }
            Row::Hunk(f, h) => {
                let file = &changes.files[f];
                let hunk = &file.hunks[h];
                let gutter = self.gutter(&ui, digits(file));
                self.sideways(f, &ui, digits(file), cx)
                    .when(h > 0, |d| d.mt(ui.px(6.0)))
                    .flex()
                    .items_center()
                    .h(ui.px(20.0))
                    .bg(c.blue.opacity(0.07))
                    .font(self.mono.clone())
                    .text_size(ui.px(11.5))
                    .child(div().flex_shrink_0().w(px(gutter)))
                    .child(
                        self.shifted(
                            f,
                            div()
                                .flex()
                                .whitespace_nowrap()
                                .child(div().text_color(c.blue).child(hunk.header()))
                                .when(!hunk.context.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .pl(px(self.advance * 2.0))
                                            .text_color(c.dim)
                                            .child(hunk.context.clone()),
                                    )
                                }),
                        ),
                    )
                    .into_any_element()
            }
            Row::Gap(f, g) => {
                let file = &changes.files[f];
                let gap = self.gaps[f][g];
                let n = gap.new.1 - gap.new.0;
                let label = if gap.at == file.hunks.len() {
                    format!(
                        "{} more {}",
                        count(n),
                        if n == 1 { "line" } else { "lines" }
                    )
                } else {
                    format!(
                        "{} unchanged {}",
                        count(n),
                        if n == 1 { "line" } else { "lines" }
                    )
                };
                let gutter = self.gutter(&ui, digits(file));
                div()
                    .id(("changes-gap", ix))
                    .w_full()
                    .flex()
                    .items_center()
                    .h(ui.px(24.0))
                    .cursor_pointer()
                    .text_size(ui.px(11.5))
                    .text_color(c.dim)
                    .hover(move |s| s.bg(c.selected.opacity(0.6)))
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(px(gutter))
                            .flex()
                            .justify_center()
                            .child(glyph(Glyph::Unfold, c.dim, ui.scale(10.0))),
                    )
                    .child(label)
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.open_gap(f, g, cx)),
                    )
                    .into_any_element()
            }
            Row::Line(f, h, i) => self.unified(
                changes,
                Side {
                    f,
                    block: Block::Hunk(h),
                    i,
                },
                &ui,
                cx,
            ),
            Row::Extra(f, g, i) => {
                if self.layout().1 {
                    self.pair(
                        changes,
                        Some(Side {
                            f,
                            block: Block::Gap(g),
                            i,
                        }),
                        Some(Side {
                            f,
                            block: Block::Gap(g),
                            i,
                        }),
                        &ui,
                        cx,
                    )
                } else {
                    self.unified(
                        changes,
                        Side {
                            f,
                            block: Block::Gap(g),
                            i,
                        },
                        &ui,
                        cx,
                    )
                }
            }
            Row::Pair(f, h, a, b) => {
                let side = |i: Option<usize>| {
                    i.map(|i| Side {
                        f,
                        block: Block::Hunk(h),
                        i,
                    })
                };
                self.pair(changes, side(a), side(b), &ui, cx)
            }
            Row::Foot(_) => div().h(ui.px(6.0)).into_any_element(),
            Row::End => div().h(ui.px(24.0)).into_any_element(),
        }
    }

    /// The width before the code: two line numbers and the sign, or one number when split.
    fn gutter(&self, ui: &UiFont, digits: usize) -> f32 {
        if self.layout().1 {
            self.number_width(ui, digits)
        } else {
            2.0 * self.number_width(ui, digits) + ui.scale(SIGN)
        }
    }

    /// A row of file `f`, as wide as the list, that scrolls the file sideways under a sideways
    /// swipe.
    fn sideways(&self, f: usize, ui: &UiFont, digits: usize, cx: &mut Context<Self>) -> Div {
        let reach = self.reach_x(f, ui, digits);
        // A list row is only as wide as its content unless told otherwise: full width, its ground
        // reaches the edge and side by side its halves are half each.
        div()
            .w_full()
            .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(20.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                // Mostly sideways only: a vertical swipe that wanders leaves the code where it is.
                if dx.abs() <= dy.abs() {
                    return;
                }
                let Some(path) = this.files().get(f).cloned() else {
                    return;
                };
                let x = this.scroll_x.entry(path).or_default();
                let to = (*x - dx).clamp(0.0, reach);
                if to != *x {
                    *x = to;
                    cx.notify();
                }
            }))
    }

    /// Code of file `f`, moved by how far it is scrolled sideways, cut at its column's edges.
    fn shifted(&self, f: usize, code: Div) -> Div {
        let x = match self.shown.as_ref().map(|(_, read)| &**read) {
            Some(Read::Changes(changes)) => changes
                .files
                .get(f)
                .and_then(|file| self.scroll_x.get(&file.path))
                .copied()
                .unwrap_or(0.0),
            _ => 0.0,
        };
        div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .items_center()
            .overflow_hidden()
            .child(code.flex_shrink_0().ml(px(-x)))
    }

    fn number(&self, n: u32, width: f32, color: Hsla, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .w(px(width))
            .flex()
            .justify_end()
            .pr(ui.px(6.0))
            .text_size(ui.px(11.0))
            .text_color(color)
            .when(n > 0, |d| d.child(n.to_string()))
    }

    fn unified(
        &mut self,
        changes: &diff::Changes,
        side: Side,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = self.colors;
        let f = side.f;
        let file_digits = digits(&changes.files[f]);
        let (line, text) = self.code(changes, &side);
        let (bg, sign, sign_color, number) = match line.kind {
            Kind::Added => (c.green.opacity(0.10), "+", c.green, c.green.opacity(0.55)),
            Kind::Deleted => (c.red.opacity(0.10), "−", c.red, c.red.opacity(0.55)),
            Kind::Context => (gpui::transparent_black(), "", c.dim, c.faint),
        };
        let width = self.number_width(ui, file_digits);
        self.sideways(f, ui, file_digits, cx)
            .flex()
            .items_center()
            .h(ui.px(20.0))
            .bg(bg)
            .font(self.mono.clone())
            .text_size(ui.px(12.0))
            .line_height(ui.px(20.0))
            .child(self.number(line.old, width, number, ui))
            .child(self.number(line.new, width, number, ui))
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(SIGN))
                    .text_color(sign_color)
                    .child(sign),
            )
            .child(self.shifted(f, div().whitespace_nowrap().pr(ui.px(16.0)).child(text)))
            .into_any_element()
    }

    fn pair(
        &mut self,
        changes: &diff::Changes,
        old: Option<Side>,
        new: Option<Side>,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = self.colors;
        let f = old.as_ref().or(new.as_ref()).map_or(0, |s| s.f);
        let file_digits = digits(&changes.files[f]);
        let width = self.number_width(ui, file_digits);
        let mut half = |side: Option<Side>, left: bool| -> Div {
            let base = div().flex_1().min_w(px(0.0)).h_full().flex().items_center();
            let base = if left {
                base.border_r_1().border_color(c.rule)
            } else {
                base
            };
            let Some(side) = side else {
                return base.bg(c.text.opacity(0.018));
            };
            let (line, text) = self.code(changes, &side);
            let (bg, number) = match line.kind {
                Kind::Added => (c.green.opacity(0.10), c.green.opacity(0.55)),
                Kind::Deleted => (c.red.opacity(0.10), c.red.opacity(0.55)),
                Kind::Context => (gpui::transparent_black(), c.faint),
            };
            let n = if left { line.old } else { line.new };
            base.bg(bg).child(self.number(n, width, number, ui)).child(
                self.shifted(
                    f,
                    div()
                        .whitespace_nowrap()
                        .pl(ui.px(4.0))
                        .pr(ui.px(16.0))
                        .child(text),
                ),
            )
        };
        let left = half(old, true);
        let right = half(new, false);
        self.sideways(f, ui, file_digits, cx)
            .flex()
            .h(ui.px(20.0))
            .font(self.mono.clone())
            .text_size(ui.px(12.0))
            .line_height(ui.px(20.0))
            .child(left)
            .child(right)
            .into_any_element()
    }

    /// A file as the index and the tree list it: status letter, directory and name, counts.
    fn file_item(&self, file: &File, pad: Pixels, selected: bool, ui: &UiFont) -> Div {
        let c = self.colors;
        let (dir, name) = names(file);
        div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(26.0))
            .px(pad)
            .rounded(px(6.0))
            .cursor_pointer()
            .text_size(ui.px(12.5))
            .when(selected, |d| {
                d.bg(c.selected).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!selected, |d| {
                d.hover(move |s| s.bg(c.selected.opacity(0.6)))
            })
            .child(letter(file.status, &c, ui))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(c.text)
                    .child(StyledText::new(format!("{dir}{name}")).with_highlights([(
                        0..dir.len(),
                        HighlightStyle {
                            color: Some(c.dim),
                            ..HighlightStyle::default()
                        },
                    )])),
            )
            .child(counts(file.added, file.deleted, ui.px(11.5), &c))
    }

    /// A file's header: fold chevron, status, names, counts; `pinned` over the list's top.
    fn head(
        &self,
        file: &File,
        f: usize,
        pinned: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let open = !self.is_folded(file);
        let (dir, name) = names(file);
        div()
            .id(if pinned {
                ("changes-pinned", f)
            } else {
                ("changes-head", f)
            })
            .flex()
            .items_center()
            .w_full()
            .gap(ui.px(8.0))
            .h(ui.px(34.0))
            .pl(ui.px(12.0))
            .pr(ui.px(14.0))
            .bg(c.ground)
            .when(!pinned, |d| d.border_t_1().border_color(c.rule))
            .when(open || pinned, |d| d.border_b_1().border_color(c.rule))
            .cursor_pointer()
            .text_size(ui.px(12.5))
            .child(glyph(
                if open { Glyph::Open } else { Glyph::Closed },
                c.dim,
                ui.scale(9.0),
            ))
            .child(letter(file.status, &c, ui))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(c.text)
                    .child(StyledText::new(format!("{dir}{name}")).with_highlights([
                        (
                            0..dir.len(),
                            HighlightStyle {
                                color: Some(c.dim),
                                ..HighlightStyle::default()
                            },
                        ),
                        (
                            dir.len()..dir.len() + name.len(),
                            HighlightStyle {
                                font_weight: Some(FontWeight::SEMIBOLD),
                                ..HighlightStyle::default()
                            },
                        ),
                    ])),
            )
            .child(counts(file.added, file.deleted, ui.px(11.5), &c))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle(f, cx)))
    }
}

fn letter(status: Status, c: &Colors, ui: &UiFont) -> Div {
    div()
        .flex_shrink_0()
        .w(ui.px(12.0))
        .flex()
        .justify_center()
        .text_size(ui.px(10.5))
        .font_weight(FontWeight::BOLD)
        .text_color(c.status(status))
        .child(status.letter())
}

fn counts(added: u32, deleted: u32, size: Pixels, c: &Colors) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .gap(px(4.0))
        .text_size(size)
        .whitespace_nowrap()
        .when(added > 0, |d| {
            d.child(
                div()
                    .text_color(c.green)
                    .child(format!("+{}", count(added))),
            )
        })
        .when(deleted > 0, |d| {
            d.child(
                div()
                    .text_color(c.red)
                    .child(format!("−{}", count(deleted))),
            )
        })
}

impl Render for ChangesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        let font = window.text_system().resolve_font(&self.mono);
        if let Ok(advance) = window.text_system().advance(font, ui.px(12.0), 'm') {
            self.advance = f32::from(advance.width);
        }
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(c.text)
            .text_size(ui.px(13.0));
        let Some(follow) = self.follow.clone() else {
            return root.child(empty(
                Glyph::Pane,
                "No pane in focus",
                "Changes follow the focused pane",
                None,
                &c,
                &ui,
            ));
        };
        let (wide, split) = self.layout();
        let read = self.changes();
        let place = self
            .place
            .as_ref()
            .filter(|(cwd, ..)| Some(cwd) == follow.cwd.as_ref());
        let base = place.and_then(|(_, _, base)| base.clone());
        let files = match read.as_deref() {
            Some(Read::Changes(changes)) => Some(changes),
            _ => None,
        };
        // Not a repository, or no directory to read: no scopes to choose between.
        let repo = follow.cwd.is_some()
            && match read.as_deref() {
                Some(Read::NotRepository) => false,
                Some(_) => true,
                // Still reading: a repository if the last read here found one.
                None => place.is_some(),
            };
        // No files: nothing to count or fold.
        let listed = files.filter(|changes| !changes.files.is_empty());
        // Who, on what branch, and how much changed.
        let who = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .when(!wide, |d| d.w_full())
            .child(
                div()
                    .flex_shrink_0()
                    .size(ui.px(7.0))
                    .rounded_full()
                    .bg(follow.dot),
            )
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(follow.name.clone()),
            )
            .when_some(place, |d, (_, head, _)| {
                d.child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(ui.px(12.0))
                        .text_color(c.dim)
                        .child(format!("⎇ {head}")),
                )
            })
            .when(!wide, |d| d.child(div().flex_1()))
            .when_some(files, |d, changes| {
                d.child(counts(changes.added(), changes.deleted(), ui.px(12.0), &c))
            });
        let segment = |items: Vec<(AnyElement, bool, Choose)>,
                       id: &'static str,
                       cx: &mut Context<Self>| {
            // Short of room, the labels end in an ellipsis rather than run out of the row.
            let mut group = div()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .p(px(2.0))
                .rounded(px(7.0))
                .bg(c.faint.opacity(0.35))
                .text_size(ui.px(12.0));
            for (n, (label, on, pick)) in items.into_iter().enumerate() {
                group = group.child(
                    div()
                        .id((id, n))
                        .flex()
                        .items_center()
                        .justify_center()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .h(ui.px(20.0))
                        .px(ui.px(9.0))
                        .rounded(px(5.0))
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .cursor_pointer()
                        .when(on, |d| {
                            d.bg(c.rule)
                                .text_color(c.text)
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(!on, |d| {
                            d.text_color(c.muted).hover(move |s| s.text_color(c.text))
                        })
                        .child(label)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| pick(this, cx))),
                );
            }
            group
        };
        let branch_label = match &base {
            Some(base) => format!("Branch vs {base}"),
            None => "Branch".into(),
        };
        let scopes = segment(
            vec![
                (
                    "Uncommitted".into_any_element(),
                    self.scope == Scope::Uncommitted,
                    Box::new(|this, cx| this.set_scope(Scope::Uncommitted, cx)),
                ),
                (
                    branch_label.into_any_element(),
                    self.scope == Scope::Branch,
                    Box::new(|this, cx| this.set_scope(Scope::Branch, cx)),
                ),
            ],
            "changes-scope",
            cx,
        );
        let any_open = self.any_open();
        let fold = div()
            .id("changes-fold")
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(24.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |s| s.bg(c.selected))
            .child(glyph(
                if any_open {
                    Glyph::FoldAll
                } else {
                    Glyph::UnfoldAll
                },
                c.muted,
                ui.scale(12.0),
            ))
            .tooltip(tip(
                if any_open {
                    "Collapse all"
                } else {
                    "Expand all"
                },
                &c,
                &ui,
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.fold_all(cx)));
        let file_count = listed.map(|changes| {
            let n = changes.files.len();
            format!(
                "{} {}",
                count(n as u32),
                if n == 1 { "file" } else { "files" }
            )
        });
        let top = if wide {
            let layouts = segment(
                vec![
                    (
                        glyph(
                            Glyph::Unified,
                            if split { c.muted } else { c.text },
                            ui.scale(12.0),
                        )
                        .into_any_element(),
                        !split,
                        Box::new(|this, cx| this.set_split(false, cx)),
                    ),
                    (
                        glyph(
                            Glyph::Split,
                            if split { c.text } else { c.muted },
                            ui.scale(12.0),
                        )
                        .into_any_element(),
                        split,
                        Box::new(|this, cx| this.set_split(true, cx)),
                    ),
                ],
                "changes-layout",
                cx,
            );
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(10.0))
                .pt(ui.px(2.0))
                .pb(ui.px(10.0))
                .pl(ui.px(16.0))
                .pr(ui.px(14.0))
                .child(who)
                .child(div().flex_1())
                .when(repo, |d| d.child(scopes))
                // No files to lay out: no layouts to choose between.
                .when(listed.is_some(), |d| d.child(layouts))
                .when(listed.is_some(), |d| d.child(fold))
        } else {
            div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap(ui.px(8.0))
                .pt(ui.px(2.0))
                .pb(ui.px(10.0))
                .pl(ui.px(16.0))
                .pr(ui.px(14.0))
                .child(who)
                .when(repo, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui.px(8.0))
                            .child(scopes)
                            .child(div().flex_1())
                            .when_some(file_count, |d, n| {
                                d.child(
                                    div()
                                        .min_w(px(0.0))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(ui.px(12.0))
                                        .text_color(c.dim)
                                        .child(n),
                                )
                            })
                            .when(listed.is_some(), |d| d.child(fold)),
                    )
                })
        };
        let rule = div().flex_shrink_0().h(px(1.0)).bg(c.rule);
        let body: AnyElement = match (follow.cwd.as_ref(), read.as_deref()) {
            (None, _) => empty(
                Glyph::Folder,
                "No directory to read",
                "This pane has no working directory",
                None,
                &c,
                &ui,
            )
            .into_any_element(),
            (Some(_), None) => div().flex_1().into_any_element(),
            (Some(_), Some(Read::NotRepository)) => empty(
                Glyph::Folder,
                "Not a git repository",
                "Changes show for panes inside a repository",
                None,
                &c,
                &ui,
            )
            .into_any_element(),
            (Some(_), Some(Read::NoBase { head })) => empty(
                Glyph::Branch,
                "No base branch",
                if head == "main" {
                    "main has no upstream to compare with"
                } else {
                    "There is no main branch to compare with"
                },
                None,
                &c,
                &ui,
            )
            .into_any_element(),
            (Some(_), Some(Read::Failed(reason))) => {
                let retry = div()
                    .id("changes-retry")
                    .mt(ui.px(4.0))
                    .px(ui.px(12.0))
                    .py(ui.px(4.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(c.rule)
                    .cursor_pointer()
                    .text_size(ui.px(12.0))
                    .text_color(c.text)
                    .hover(move |s| s.bg(c.selected.opacity(0.6)))
                    .child("Retry")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.retry(cx)));
                empty(
                    Glyph::Warning,
                    "Couldn’t read changes",
                    reason,
                    Some(retry.into_any_element()),
                    &c,
                    &ui,
                )
                .into_any_element()
            }
            (Some(_), Some(Read::Changes(changes))) if changes.files.is_empty() => {
                let (title, sub) = match self.scope {
                    Scope::Uncommitted => (
                        "No uncommitted changes",
                        "Working tree matches HEAD".to_owned(),
                    ),
                    Scope::Branch => (
                        "No changes on this branch",
                        format!(
                            "Nothing since it left {}",
                            base.as_deref().unwrap_or("its base")
                        ),
                    ),
                };
                empty(Glyph::Check, title, &sub, None, &c, &ui).into_any_element()
            }
            (Some(_), Some(Read::Changes(changes))) => {
                let head = ui.scale(34.0);
                let pinned = self.pinned(head).map(|(f, shift)| {
                    div()
                        .absolute()
                        .top(px(shift))
                        .left_0()
                        .right_0()
                        .child(self.head(&changes.files[f], f, true, &ui, cx))
                });
                let diffs = div()
                    .relative()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_hidden()
                    .child(
                        list(
                            self.list.clone(),
                            cx.processor(|this, ix, window, cx| this.row(ix, window, cx)),
                        )
                        .size_full(),
                    )
                    .children(pinned);
                if wide {
                    div()
                        .flex_1()
                        .min_h(px(0.0))
                        .flex()
                        .child(self.tree(changes, &ui, cx))
                        .child(diffs)
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_h(px(0.0))
                        .flex()
                        .child(diffs)
                        .into_any_element()
                }
            }
        };
        root.child(top).child(rule).child(body)
    }
}

impl ChangesView {
    /// The widened tab's file tree: directories, and under them the files, the one at the top of
    /// the diffs selected.
    fn tree(&self, changes: &diff::Changes, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let current = self
            .rows
            .get(self.list.logical_scroll_top().item_ix)
            .and_then(|r| r.file());
        let n = changes.files.len();
        let mut tree = div()
            .id("changes-tree")
            .flex_shrink_0()
            .w(ui.px(TREE))
            .h_full()
            .overflow_y_scroll()
            .p(ui.px(8.0))
            .border_r_1()
            .border_color(c.rule)
            .text_size(ui.px(12.5))
            .child(
                div()
                    .px(ui.px(8.0))
                    .pt(ui.px(4.0))
                    .pb(ui.px(6.0))
                    .text_size(ui.px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.muted)
                    .child(format!(
                        "{} {}",
                        count(n as u32),
                        if n == 1 { "FILE" } else { "FILES" }
                    )),
            );
        let mut open: Vec<&str> = Vec::new();
        for (f, file) in changes.files.iter().enumerate() {
            let dirs: Vec<&str> = file.path.split('/').collect();
            let dirs = &dirs[..dirs.len() - 1];
            let same = open.iter().zip(dirs).take_while(|(a, b)| a == b).count();
            open.truncate(same);
            for dir in &dirs[same..] {
                let depth = open.len();
                tree = tree.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(ui.px(7.0))
                        .h(ui.px(26.0))
                        .pl(ui.px(8.0 + 14.0 * depth as f32))
                        .pr(ui.px(8.0))
                        .text_color(c.muted)
                        .whitespace_nowrap()
                        .child(glyph(Glyph::Open, c.dim, ui.scale(9.0)))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(dir.to_string()),
                        ),
                );
                open.push(dir);
            }
            let depth = open.len();
            let mut item = file.clone();
            if file.old_path.is_none() {
                item.path = file.dir_and_name().1.to_owned();
            }
            tree = tree.child(
                div().pl(ui.px(14.0 * depth as f32)).child(
                    self.file_item(&item, ui.px(8.0), current == Some(f), ui)
                        .id(("changes-tree-file", f))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.jump(f, cx))),
                ),
            );
        }
        tree
    }
}

/// A quiet state in the middle: a faint icon, a line, a fainter line, and maybe a button.
fn empty(
    glyph_kind: Glyph,
    title: &str,
    sub: &str,
    action: Option<AnyElement>,
    c: &Colors,
    ui: &UiFont,
) -> Div {
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
        .child(glyph(
            glyph_kind,
            c.faint.blend(c.dim.opacity(0.5)),
            ui.scale(28.0),
        ))
        .child(
            div()
                .text_size(ui.px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.code)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_size(ui.px(12.0))
                .text_color(c.dim)
                .text_center()
                .child(sub.to_owned()),
        )
        .children(action)
}

/// A button's hover text.
fn tip(
    text: &'static str,
    c: &Colors,
    ui: &UiFont,
) -> impl Fn(&mut Window, &mut gpui::App) -> gpui::AnyView + 'static {
    let tip = Tip {
        text,
        size: ui.px(11.5),
        color: c.text,
        background: c.ground,
        border: c.rule,
    };
    move |_, cx| cx.new(|_| tip.clone()).into()
}

#[derive(Clone)]
struct Tip {
    text: &'static str,
    size: Pixels,
    color: Hsla,
    background: Hsla,
    border: Hsla,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(7.0))
            .py(px(3.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(self.border)
            .bg(self.background)
            .shadow_md()
            .text_size(self.size)
            .text_color(self.color)
            .child(self.text)
    }
}

/// The tab's own small drawings, as lines like the sidebar's icons.
#[derive(Clone, Copy)]
enum Glyph {
    /// A chevron down: a file shown.
    Open,
    /// A chevron right: a file folded.
    Closed,
    /// Arrows from above and below meeting at a bar: fold everything.
    FoldAll,
    /// Arrows leaving a bar up and down: unfold everything.
    UnfoldAll,
    /// Arrows up and down: show the lines between.
    Unfold,
    /// A frame with lines across: one column.
    Unified,
    /// A frame cut down the middle: side by side.
    Split,
    Check,
    Folder,
    Warning,
    /// A branch leaving its line.
    Branch,
    /// A pane's frame.
    Pane,
}

/// `kind` in `color`, `size` points square.
fn glyph(kind: Glyph, color: Hsla, size: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            for path in glyph_paths(kind, bounds.origin, size) {
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .flex_shrink_0()
    .size(px(size))
}

fn glyph_paths(kind: Glyph, origin: Point<Pixels>, size: f32) -> Vec<PathBuilder> {
    // Small ones are drawn on a 12-unit square, the large states' on a 28-unit one.
    let units = match kind {
        Glyph::Check | Glyph::Folder | Glyph::Warning | Glyph::Branch | Glyph::Pane => 28.0,
        _ => 12.0,
    };
    let k = size / units;
    let at = |x: f32, y: f32| origin + point(px(x * k), px(y * k));
    let stroke = || PathBuilder::stroke(px(if units > 12.0 { 1.5 * k } else { 1.3 * k }));
    let line = |points: &[(f32, f32)]| {
        let mut path = stroke();
        path.move_to(at(points[0].0, points[0].1));
        for &(x, y) in &points[1..] {
            path.line_to(at(x, y));
        }
        path
    };
    match kind {
        Glyph::Open => vec![line(&[(2.5, 4.25), (6.0, 7.75), (9.5, 4.25)])],
        Glyph::Closed => vec![line(&[(4.25, 2.5), (7.75, 6.0), (4.25, 9.5)])],
        Glyph::FoldAll => vec![
            line(&[(1.5, 6.0), (10.5, 6.0)]),
            line(&[(6.0, 0.5), (6.0, 4.0)]),
            line(&[(4.0, 2.25), (6.0, 4.25), (8.0, 2.25)]),
            line(&[(6.0, 11.5), (6.0, 8.0)]),
            line(&[(4.0, 9.75), (6.0, 7.75), (8.0, 9.75)]),
        ],
        Glyph::UnfoldAll => vec![
            line(&[(1.5, 6.0), (10.5, 6.0)]),
            line(&[(6.0, 4.0), (6.0, 0.75)]),
            line(&[(4.0, 2.75), (6.0, 0.75), (8.0, 2.75)]),
            line(&[(6.0, 8.0), (6.0, 11.25)]),
            line(&[(4.0, 9.25), (6.0, 11.25), (8.0, 9.25)]),
        ],
        Glyph::Unfold => vec![
            line(&[(3.6, 4.2), (6.0, 1.8), (8.4, 4.2)]),
            line(&[(3.6, 7.8), (6.0, 10.2), (8.4, 7.8)]),
        ],
        Glyph::Unified => vec![
            frame(at, stroke(), (1.5, 1.5), (10.5, 10.5), 1.5 * k),
            line(&[(4.0, 4.5), (8.0, 4.5)]),
            line(&[(4.0, 7.5), (8.0, 7.5)]),
        ],
        Glyph::Split => vec![
            frame(at, stroke(), (1.5, 1.5), (10.5, 10.5), 1.5 * k),
            line(&[(6.0, 1.5), (6.0, 10.5)]),
        ],
        Glyph::Check => vec![line(&[(6.0, 14.5), (11.0, 19.5), (22.0, 8.5)])],
        Glyph::Folder => {
            let mut outline = line(&[
                (5.5, 6.5),
                (10.5, 6.5),
                (13.0, 9.0),
                (22.5, 9.0),
                (24.5, 11.0),
                (24.5, 20.5),
                (22.5, 22.5),
                (5.5, 22.5),
                (3.5, 20.5),
                (3.5, 8.5),
            ]);
            outline.close();
            vec![outline, line(&[(10.0, 15.5), (18.0, 15.5)])]
        }
        Glyph::Warning => {
            let mut outline = line(&[(14.0, 4.5), (24.5, 23.0), (3.5, 23.0)]);
            outline.close();
            vec![
                outline,
                line(&[(14.0, 11.5), (14.0, 16.5)]),
                line(&[(14.0, 19.5), (14.0, 20.0)]),
            ]
        }
        Glyph::Branch => vec![
            line(&[(9.0, 4.5), (9.0, 23.5)]),
            line(&[(19.0, 4.5), (19.0, 10.5), (9.0, 18.0)]),
        ],
        Glyph::Pane => vec![
            frame(at, stroke(), (3.5, 5.5), (24.5, 22.5), 2.5 * k),
            line(&[(8.0, 11.0), (11.0, 14.0), (8.0, 17.0)]),
            line(&[(13.5, 17.0), (19.0, 17.0)]),
        ],
    }
}

fn frame(
    at: impl Fn(f32, f32) -> Point<Pixels>,
    mut path: PathBuilder,
    min: (f32, f32),
    max: (f32, f32),
    r: f32,
) -> PathBuilder {
    let (min, max) = (at(min.0, min.1), at(max.0, max.1));
    let r = px(r);
    let radii = point(r, r);
    path.move_to(point(min.x + r, min.y));
    path.line_to(point(max.x - r, min.y));
    path.arc_to(radii, px(0.0), false, true, point(max.x, min.y + r));
    path.line_to(point(max.x, max.y - r));
    path.arc_to(radii, px(0.0), false, true, point(max.x - r, max.y));
    path.line_to(point(min.x + r, max.y));
    path.arc_to(radii, px(0.0), false, true, point(min.x, max.y - r));
    path.line_to(point(min.x, min.y + r));
    path.arc_to(radii, px(0.0), false, true, point(min.x + r, min.y));
    path.close();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{Hunk, parse};

    #[test]
    fn side_by_side_puts_deletions_beside_the_additions_after_them() {
        let lines: Vec<Line> = " --+ +++- "
            .chars()
            .map(|c| Line {
                kind: match c {
                    '+' => Kind::Added,
                    '-' => Kind::Deleted,
                    _ => Kind::Context,
                },
                old: 0,
                new: 0,
                text: String::new(),
            })
            .collect();
        assert_eq!(
            sides(&lines),
            [
                (Some(0), Some(0)),
                (Some(1), Some(3)),
                (Some(2), None),
                (Some(4), Some(4)),
                (None, Some(5)),
                (None, Some(6)),
                (None, Some(7)),
                (Some(8), None),
                (Some(9), Some(9)),
            ]
        );
    }

    #[test]
    fn side_by_side_puts_pure_additions_right_and_pure_deletions_left() {
        let files = parse(
            "\
diff --git a/Cargo.toml b/Cargo.toml
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,2 +1,4 @@
 [dependencies]
+similar = \"2\"
+syntect = \"5\"
 toml = \"1\"
diff --git a/new.rs b/new.rs
new file mode 100644
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+fn a() {}
+fn b() {}
diff --git a/old.rs b/old.rs
deleted file mode 100644
--- a/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn a() {}
-fn b() {}
",
        );
        let pairs = |f: usize| sides(&files[f].hunks[0].lines);
        assert_eq!(
            pairs(0),
            [
                (Some(0), Some(0)),
                (None, Some(1)),
                (None, Some(2)),
                (Some(3), Some(3)),
            ]
        );
        assert_eq!(pairs(1), [(None, Some(0)), (None, Some(1))]);
        assert_eq!(pairs(2), [(Some(0), None), (Some(1), None)]);
    }

    #[test]
    fn unchanged_lines_between_and_after_hunks() {
        let mut file = parse(
            "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -3,2 +3,3 @@ fn a
 c
+n
 d
@@ -20 +21 @@
-t
+T
",
        )
        .remove(0);
        // Lines 1-2 before, 6-20 between (old 5-19), and 22-30 after once the length is known.
        assert_eq!(
            gaps(&file),
            [
                Gap {
                    at: 0,
                    new: (1, 3),
                    old: 1
                },
                Gap {
                    at: 1,
                    new: (6, 21),
                    old: 5
                },
            ]
        );
        file.lines_now = Some(30);
        assert_eq!(
            gaps(&file).last(),
            Some(&Gap {
                at: 2,
                new: (22, 31),
                old: 21
            })
        );
        // A hunk at the very top, and one reaching the end: nothing around them.
        file.hunks = vec![Hunk {
            old_start: 1,
            old_len: 2,
            new_start: 1,
            new_len: 3,
            context: String::new(),
            lines: Vec::new(),
        }];
        file.lines_now = Some(3);
        assert_eq!(gaps(&file), []);
    }

    #[test]
    fn counts_and_sizes_read_well() {
        assert_eq!(count(7), "7");
        assert_eq!(count(2431), "2,431");
        assert_eq!(count(1234567), "1,234,567");
        assert_eq!(size(900), "900 B");
        assert_eq!(size(12 * 1024), "12 KB");
        assert_eq!(size(3_250_000), "3.1 MB");
    }

    #[test]
    fn special_files_say_what_they_are() {
        let files = parse(
            "\
diff --git a/src/icon.rs b/src/app_icon.rs
similarity index 100%
rename from src/icon.rs
rename to src/app_icon.rs
diff --git a/old-notes.md b/old-notes.md
deleted file mode 100644
--- a/old-notes.md
+++ /dev/null
@@ -1,2 +0,0 @@
-a
-b
diff --git a/icon.png b/icon.png
Binary files a/icon.png and b/icon.png differ
",
        );
        assert_eq!(
            note_text(&files[0], false),
            ("Renamed without changes".into(), None)
        );
        assert_eq!(
            names(&files[0]),
            ("src/".into(), "icon.rs → app_icon.rs".into())
        );
        assert_eq!(
            note_text(&files[1], true),
            ("Deleted file".into(), Some("Show diff"))
        );
        let mut binary = files[2].clone();
        binary.note = Some(Note::Binary(Some(12 * 1024), Some(14 * 1024)));
        assert_eq!(note_text(&binary, false).0, "Binary file · 12 KB → 14 KB");
        let mut large = files[1].clone();
        large.status = Status::Modified;
        large.added = 2431;
        assert_eq!(
            note_text(&large, true),
            ("Large diff · 2,433 lines".into(), Some("Show diff"))
        );
        let mut lock = files[1].clone();
        (lock.path, lock.status) = ("app/Cargo.lock".into(), Status::Modified);
        assert_eq!(
            note_text(&lock, true),
            ("Generated file".into(), Some("Show diff"))
        );
    }
}
