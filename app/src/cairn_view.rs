//! The right sidebar's Cairn tab: whether cairn is at work in the focused pane's directory, the
//! handover notes saved there, and what the next session would be given, as `cairn.rs` reads them.
//! cairn is read in the background while the tab shows: at once when it comes up or the pane's
//! directory changes, then every few seconds. Under the repository's name and whether it has
//! adopted cairn, a line of which agents have their hooks and how many saves wait to be taken in,
//! then the notes one line for each session that saved them, those the next session starts from in
//! a box of their own and the others by day; a session opens in place to its note in full, read
//! when it is first opened, and to its earlier notes, and at the foot the exact text the next
//! session receives unfolds, drawn as Markdown. Where the notes are read, the project's ledger is
//! too, on a page of its own the switch under the hooks turns to: its entries by kind, one line
//! each, each kind in a raised box that folds, one opening in place to its text, and a filter over
//! them. A repository that has not adopted cairn offers Adopt, which asks first; nothing else here
//! changes anything (DESIGN §13 P5-55, P5-79, P5-110, P5-118, P5-133).
use crate::{
    activity::local_offset,
    cairn::{
        self, Adopting, Body, Entry, Fired, Folds, Found, Full, Group, LESSONS, Ledger, Note,
        Notes, Read, Records, Session,
    },
    card,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kind_icon, markdown, menu, raised, sunk,
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, Entity, FocusHandle, Focusable, Font, FontStyle,
    FontWeight, HighlightStyle, Hsla, IntoElement, MouseButton, Render, Stateful, StyledText,
    Subscription, Task, UnderlineStyle, Window, div, prelude::*, px, relative,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

/// How often cairn is read again while the tab shows.
const EVERY: Duration = Duration::from_secs(5);
/// The ground of the small tags and boxes, the rule under the status line, and the ground of
/// words written as code and the rule after a section's label, in the text colour.
const SOFT: f32 = 0.06;
const RULE: f32 = 0.06;
const CODE: f32 = 0.07;
/// The confirmation card's face in the text colour, and its edge in the accent.
const CARD_FACE: f32 = 0.045;
const CARD_EDGE: f32 = 0.34;
/// The ground of an open note's facts, in the ground colour on the note the next session starts
/// from (raised as the Kanban tab's groups, P5-117) and in the text colour on the others; and the
/// rule between two notes.
const FACTS_ON_NEXT: f32 = 0.55;
const FACTS: f32 = 0.04;
const BETWEEN: f32 = 0.05;
/// The rule between two sessions in the box of what the next session is given, in the accent.
const NEXT_RULE: f32 = 0.16;
/// How tall the icon of the agent that saved a note is drawn.
const MARK: f32 = 13.0;
/// How many of an open session's earlier notes are listed before Show all.
const SHOWN_EARLIER: usize = 3;
/// How far in a note's words start: its arrow and the room after it.
const INDENT: f32 = 18.0;
/// How wide a ledger entry's `#no` is drawn, and how far in its words start: its arrow, its
/// number and the room after each.
const NO: f32 = 28.0;
const ENTRY_INDENT: f32 = INDENT + NO + 8.0;
/// The arrows that fold a note and the exact text.
const ARROW: f32 = 10.0;
/// The filter's edge while nothing is typed and the ground of its clear button, in the text
/// colour; the ground of the words it finds in a title, in the yellow.
const FILTER_EDGE: f32 = 0.12;
const CLEAR: f32 = 0.12;
const LIT: f32 = 0.28;

/// Which of its pages the tab shows where cairn keeps a ledger.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Notes,
    Ledger,
    Lessons,
}

/// The ledger's groups apart from its lessons, and the lessons, if any.
fn split(groups: &[Group]) -> (Vec<&Group>, Option<&Group>) {
    let (lessons, book): (Vec<&Group>, Vec<&Group>) =
        groups.iter().partition(|group| group.name == LESSONS);
    (book, lessons.into_iter().next())
}

/// What the window tells the tab every time it draws.
pub struct Frame {
    /// The focused pane's directory: `None` with no pane in focus, `Some(None)` for a pane with
    /// no directory to read.
    pub cwd: Option<Option<String>>,
    /// The right sidebar is open on this tab: read, and keep reading.
    pub active: bool,
    pub theme: Rc<Theme>,
    pub mono: Font,
}

/// The theme's colours the tab uses.
#[derive(Clone, Copy)]
struct Colors {
    ground: Hsla,
    text: Hsla,
    title: Hsla,
    bright: Hsla,
    muted: Hsla,
    dim: Hsla,
    accent: Hsla,
    green: Hsla,
    red: Hsla,
    yellow: Hsla,
}

impl Colors {
    fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        Colors {
            ground: hsla(theme.terminal().background, 1.0),
            text: fg(|t| t.agents_text),
            title: fg(|t| t.bright),
            bright: fg(|t| t.agents_branch),
            muted: fg(|t| t.agents_dim),
            dim: fg(|t| t.agents_dimmer),
            accent: fg(|t| t.agents_accent),
            green: fg(|t| t.agents_green),
            red: fg(|t| t.agents_red),
            yellow: fg(|t| t.agents_yellow),
        }
    }
}

pub struct CairnView {
    theme: Rc<Theme>,
    colors: Colors,
    mono: Font,
    cwd: Option<Option<String>>,
    active: bool,
    /// The latest read, for whichever directory it was.
    read: Option<Read>,
    /// The reading while the tab shows, and what stops the command it has running.
    watching: Option<(Task<()>, Arc<AtomicBool>)>,
    adopting: Adopting,
    home: Option<PathBuf>,
    /// The repository the latest read found: what is below is about its notes, and starts over
    /// when the pane is in another.
    repo: Option<PathBuf>,
    /// How many records cairn is asked for: a page, and one more with each Show older.
    limit: usize,
    /// The session that is open, by the id of the note it is shown by.
    open: Option<String>,
    /// One of the open session's earlier notes that is open too, by its id.
    inner: Option<String>,
    /// The open session lists all its earlier notes, not just the first few.
    all: bool,
    /// The notes read in full since the tab came up, or why one could not be; `None` while one is
    /// being read.
    full: HashMap<String, Option<Result<Full, String>>>,
    /// The exact text the next session receives is unfolded.
    exact: bool,
    /// The page last turned to, kept from one repository to the next.
    page: Page,
    /// The ledger entry that is open, by its id.
    entry: Option<String>,
    /// The tab's own hold on the keyboard, which a click in it takes: ⌘F then goes to the filter
    /// of the page shown, not to a pane.
    focus: FocusHandle,
    /// The filters over the ledger's page and the lessons', each kept while the pane stays in the
    /// repository (P5-133).
    ledger_filter: Entity<TextInput>,
    lessons_filter: Entity<TextInput>,
    _typed: [Subscription; 2],
    /// Which of the ledger's groups are open in each repository, and the file they are kept in.
    folds: Folds,
    folds_path: Option<PathBuf>,
}

/// A filter's colours.
fn input_colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
    }
}

impl CairnView {
    pub fn new(theme: Rc<Theme>, mono: Font, cx: &mut Context<Self>) -> Self {
        let colors = input_colors(&theme);
        let ledger_filter = cx.new(|cx| TextInput::new("", "Filter entries", colors, cx));
        let lessons_filter = cx.new(|cx| TextInput::new("", "Filter lessons", colors, cx));
        let _typed = [&ledger_filter, &lessons_filter]
            .map(|input| cx.subscribe(input, |_, _, _: &Changed, cx| cx.notify()));
        let folds_path = cairn::folds_path();
        CairnView {
            focus: cx.focus_handle(),
            ledger_filter,
            lessons_filter,
            _typed,
            folds: folds_path.as_deref().map(Folds::load).unwrap_or_default(),
            folds_path,
            colors: Colors::of(&theme),
            theme,
            mono,
            cwd: None,
            active: false,
            read: None,
            watching: None,
            adopting: Adopting::default(),
            home: std::env::var_os("HOME").map(PathBuf::from),
            repo: None,
            limit: cairn::PAGE,
            open: None,
            inner: None,
            all: false,
            full: HashMap::new(),
            exact: false,
            page: Page::Notes,
            entry: None,
        }
    }

    /// The window's latest: redraws only when something here changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        let moved = frame.cwd != self.cwd;
        if moved || frame.active != self.active {
            self.cwd = frame.cwd;
            self.active = frame.active;
            // Left for another tab or closed: a question still up is taken back, and the notes
            // read in full are let go, the open one to be read again on coming back.
            if !self.active {
                changed |= self.adopting.follow(None);
                self.full.clear();
            }
            self.watch(cx);
            self.load(cx);
            changed |= moved;
        }
        if !Rc::ptr_eq(&frame.theme, &self.theme) {
            self.colors = Colors::of(&frame.theme);
            let colors = input_colors(&frame.theme);
            for input in [&self.ledger_filter, &self.lessons_filter] {
                input.update(cx, |input, cx| input.set_colors(colors, cx));
            }
            self.theme = frame.theme;
            changed = true;
        }
        if frame.mono != self.mono {
            self.mono = frame.mono;
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Reads the pane's directory now and every few seconds after, in the background, while the
    /// tab shows; any reading under way is dropped first.
    fn watch(&mut self, cx: &mut Context<Self>) {
        if let Some((_, cancel)) = self.watching.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        let (true, Some(Some(cwd))) = (self.active, self.cwd.clone()) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                let Ok(limit) = this.update(cx, |view, _| view.limit) else {
                    break;
                };
                let (dir, stop) = (cwd.clone(), stop.clone());
                let read = cx
                    .background_executor()
                    .spawn(async move {
                        cairn::read(
                            cairn::PROGRAM,
                            "git",
                            &dir,
                            limit,
                            SystemTime::now(),
                            &local_offset,
                            &stop,
                        )
                    })
                    .await;
                if this.update(cx, |view, cx| view.accept(read, cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(EVERY).await;
            }
        });
        self.watching = Some((task, cancel));
    }

    /// A read came back: drawn when it differs, and a confirmation card that is not about what
    /// it shows closes. In another repository than the last, what was open folds, the list is
    /// a page again and the filters are emptied.
    fn accept(&mut self, read: Read, cx: &mut Context<Self>) {
        if let Read::Found(found) = &read
            && found.repo != self.repo
        {
            self.repo = found.repo.clone();
            self.limit = cairn::PAGE;
            self.open = None;
            self.inner = None;
            self.all = false;
            self.full.clear();
            self.exact = false;
            self.entry = None;
            for input in [&self.ledger_filter, &self.lessons_filter] {
                input.update(cx, |input, cx| input.set_text("", cx));
            }
        }
        let repo = match &read {
            Read::Found(Found {
                body: Body::NotAdopted(repo),
                ..
            }) => Some(repo.as_path()),
            _ => None,
        };
        let closed = self.adopting.follow(repo);
        if closed || self.read.as_ref() != Some(&read) {
            self.read = Some(read);
            self.placeholders(cx);
            cx.notify();
        }
    }

    /// What the empty filters say: how many entries and lessons there are to filter.
    fn placeholders(&mut self, cx: &mut Context<Self>) {
        let Some(Read::Found(Found {
            ledger: Some(Ledger::Groups(groups)),
            ..
        })) = &self.read
        else {
            return;
        };
        let (book, lessons) = split(groups);
        let say = |n: usize, one: &str, many: &str| {
            format!("Filter {n} {}", if n == 1 { one } else { many })
        };
        let entries = book.iter().map(|group| group.entries.len()).sum();
        let ledger = say(entries, "entry", "entries");
        let lessons = say(
            lessons.map_or(0, |group| group.entries.len()),
            "lesson",
            "lessons",
        );
        self.ledger_filter
            .update(cx, |input, cx| input.set_placeholder(ledger, cx));
        self.lessons_filter
            .update(cx, |input, cx| input.set_placeholder(lessons, cx));
    }

    /// The filter over `page`, the ledger's or the lessons'.
    fn filter_of(&self, page: Page) -> Option<&Entity<TextInput>> {
        match page {
            Page::Notes => None,
            Page::Ledger => Some(&self.ledger_filter),
            Page::Lessons => Some(&self.lessons_filter),
        }
    }

    /// The filter of the page shown, where it is drawn: on a page with entries to filter.
    fn shown_filter(&self) -> Option<&Entity<TextInput>> {
        let Some(Read::Found(Found {
            ledger: Some(Ledger::Groups(groups)),
            ..
        })) = &self.read
        else {
            return None;
        };
        let (book, lessons) = split(groups);
        let any = match self.page {
            Page::Notes => false,
            Page::Ledger => !book.is_empty(),
            Page::Lessons => lessons.is_some(),
        };
        any.then(|| self.filter_of(self.page)).flatten()
    }

    /// ⌘F while the keyboard is in the tab: the filter of the page shown takes it, what it holds
    /// selected.
    fn find(&mut self, _: &menu::Find, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.shown_filter() else {
            return;
        };
        window.focus(&input.read(cx).focus_handle(cx), cx);
        cx.defer_in(window, |_, window, cx| {
            window.dispatch_action(Box::new(text_input::SelectAll), cx)
        });
    }

    /// Esc while the keyboard is in the tab, or the filter's ×: the filter of the page shown is
    /// emptied.
    fn clear_filter(&mut self, _: &menu::CloseFind, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = self.filter_of(self.page)
            && !input.read(cx).text().is_empty()
        {
            input.update(cx, |input, cx| input.set_text("", cx));
            cx.notify();
        }
    }

    /// One of the ledger's groups was clicked: it folds, or opens, in this repository, and stays
    /// so when paddock opens again.
    fn fold(&mut self, group: &'static str, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let open = self.folds.open(&repo, group);
        self.folds.set(&repo, group, !open);
        // A small file, written at once so that two clicks in a row cannot land out of order.
        if let Some(path) = &self.folds_path {
            let _ = self.folds.save(path);
        }
        cx.notify();
    }

    /// Adopt on the confirmation card: `cairn adopt` in the directory shown, in the background,
    /// and whatever comes of it, cairn read again at once.
    fn adopt(&mut self, cx: &mut Context<Self>) {
        let Some(Read::Found(Found {
            cwd,
            body: Body::NotAdopted(repo),
            ..
        })) = &self.read
        else {
            return;
        };
        let (cwd, repo) = (cwd.clone(), repo.clone());
        if !self.adopting.confirm() {
            return;
        }
        let task = cx.background_spawn(async move {
            cairn::adopt(cairn::PROGRAM, &cwd, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.adopting.finish(&repo, result);
                view.watch(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A session was clicked, by the note it is shown by: it opens, any other folding with what
    /// was open in it, and the note is read in full if it has not been; the open one folds.
    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        self.inner = None;
        self.all = false;
        if self.open.as_deref() == Some(id) {
            self.open = None;
        } else {
            self.open = Some(id.to_owned());
            self.load(cx);
        }
        cx.notify();
    }

    /// One of the open session's earlier notes was clicked: it opens, any other of them folding,
    /// and is read in full if it has not been; the open one folds.
    fn toggle_inner(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.inner.as_deref() == Some(id) {
            self.inner = None;
        } else {
            self.inner = Some(id.to_owned());
            self.load(cx);
        }
        cx.notify();
    }

    /// Reads the open notes in full, in the background, in the directory shown, unless they have
    /// been read or are being read; one that failed is tried again.
    fn load(&mut self, cx: &mut Context<Self>) {
        for id in [self.open.clone(), self.inner.clone()]
            .into_iter()
            .flatten()
        {
            self.load_one(id, cx);
        }
    }

    fn load_one(&mut self, id: String, cx: &mut Context<Self>) {
        let (true, Some(Read::Found(found))) = (self.active, &self.read) else {
            return;
        };
        if matches!(self.full.get(&id), Some(None | Some(Ok(_)))) {
            return;
        }
        let cwd = found.cwd.clone();
        self.full.insert(id.clone(), None);
        let reading = id.clone();
        let task = cx.background_spawn(async move {
            cairn::note(
                cairn::PROGRAM,
                &cwd,
                &reading,
                &local_offset,
                &AtomicBool::new(false),
            )
        });
        cx.spawn(async move |this, cx| {
            let full = task.await;
            let _ = this.update(cx, |view, cx| {
                // Let go since, with the tab left or the pane in another repository: not kept.
                if let Some(slot) = view.full.get_mut(&id) {
                    *slot = Some(full);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The card Adopt has up for `repo`, if any.
    fn asking(&self, repo: &Path) -> Option<&Adopting> {
        match &self.adopting {
            Adopting::Asking(of)
            | Adopting::Running(of)
            | Adopting::Done(of)
            | Adopting::Refused(of, _)
                if of == repo =>
            {
                Some(&self.adopting)
            }
            _ => None,
        }
    }
}

impl Render for CairnView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        // A click anywhere in the tab gives it the keyboard; ⌘F and Esc are its filter's there.
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .key_context(menu::FIND)
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::clear_filter))
            .text_color(c.text)
            .text_size(ui.px(13.0));
        let rule = || {
            div()
                .flex_shrink_0()
                .mt(ui.px(6.0))
                .h(px(1.0))
                .bg(c.text.opacity(RULE))
        };
        let follows = "Cairn follows the focused pane's directory";
        match (&self.cwd, &self.read) {
            (None, _) => root.child(
                self.middle(&ui)
                    .child(self.title("No pane in focus", &ui))
                    .child(self.sub(follows, &ui)),
            ),
            (Some(None), _) => root.child(
                self.middle(&ui)
                    .child(self.title("No directory to read", &ui))
                    .child(self.sub(follows, &ui)),
            ),
            // Still reading for the first time.
            (_, None) => root,
            (_, Some(Read::NotInstalled)) => root.child(rule()).child(
                self.middle(&ui)
                    .child(self.title("cairn is not installed", &ui))
                    .child(self.sub(
                        "This tab shows cairn's handover records once the `cairn` command is on \
                         this Mac.",
                        &ui,
                    )),
            ),
            (_, Some(Read::Failed(why))) => root.child(rule()).child(
                self.middle(&ui)
                    .gap(ui.px(10.0))
                    .child(self.title("Could not read cairn", &ui))
                    .child(self.refusal(why, &ui)),
            ),
            (_, Some(Read::Found(found))) => {
                let root = root
                    .child(self.header(found, &ui))
                    .child(self.hooks(found, &ui));
                match &found.ledger {
                    Some(ledger) => {
                        let root = root.child(self.switch(ledger, &ui, cx));
                        match self.page {
                            Page::Notes => root.child(self.body(found, &ui, cx)),
                            Page::Ledger => root.child(self.ledger(ledger, &ui, cx)),
                            Page::Lessons => root.child(self.lessons(ledger, &ui, cx)),
                        }
                    }
                    None => root.child(self.body(found, &ui, cx)),
                }
            }
        }
    }
}

impl CairnView {
    /// The repository's name, or the folder's, its branch, and whether it has adopted cairn.
    fn header(&self, found: &Found, ui: &UiFont) -> Div {
        let c = self.colors;
        let (words, color, ground, dot) = if found.adopted {
            ("Adopted", c.green, c.green.opacity(0.12), c.green)
        } else {
            ("Not adopted", c.muted, c.text.opacity(SOFT), c.dim)
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .pt(ui.px(6.0))
            .pb(ui.px(10.0))
            .px(ui.px(14.0))
            .child(footer_icon::icon(
                Icon::Folder,
                c.muted,
                ui.scale(15.0 / footer_icon::SIZE),
            ))
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(found.name.clone()),
            )
            .children(found.branch.clone().map(|branch| {
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .px(ui.px(7.0))
                    .py(ui.px(1.0))
                    .rounded_full()
                    .bg(c.text.opacity(SOFT))
                    .font(self.mono.clone())
                    .text_size(ui.px(11.0))
                    .text_color(c.muted)
                    .child(branch)
            }))
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .px(ui.px(10.0))
                    .py(ui.px(3.0))
                    .rounded_full()
                    .bg(ground)
                    .whitespace_nowrap()
                    .text_size(ui.px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(div().size(ui.px(6.0)).rounded_full().bg(dot))
                    .child(words),
            )
    }

    /// Which agents have cairn's hooks, and in an adopted repository how long ago each one's last
    /// ran there and how many saves cairn has not yet taken in.
    fn hooks(&self, found: &Found, ui: &UiFont) -> Div {
        let c = self.colors;
        let agent = |name: &'static str, installed: bool, fired: &Fired| {
            // Installed and never run is the one worth a look: say so where the check would be.
            let mark = if *fired == Fired::Never {
                div()
                    .flex_shrink_0()
                    .text_color(c.yellow)
                    .child("never")
                    .into_any_element()
            } else if installed {
                footer_icon::icon(Icon::Check, c.green, ui.scale(11.0 / footer_icon::SIZE))
                    .into_any_element()
            } else {
                div()
                    .flex_shrink_0()
                    .w(ui.px(6.0))
                    .h(ui.px(1.5))
                    .rounded_full()
                    .bg(c.dim)
                    .into_any_element()
            };
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(5.0))
                .text_color(if installed { c.muted } else { c.dim })
                .child(name)
                .child(mark)
                .children(match fired {
                    Fired::Ago(ago) => {
                        Some(div().flex_shrink_0().text_color(c.dim).child(ago.clone()))
                    }
                    Fired::Unknown | Fired::Never => None,
                })
        };
        let waiting = found.adopted.then(|| {
            let words = format!("{} uncollected", found.uncollected);
            let words = div().flex_shrink_0().whitespace_nowrap().child(words);
            if found.uncollected == 0 {
                words.text_color(c.dim)
            } else {
                words
                    .px(ui.px(8.0))
                    .py(ui.px(1.0))
                    .rounded_full()
                    .bg(c.yellow.opacity(0.14))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.yellow)
            }
        });
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(12.0))
            .px(ui.px(14.0))
            .pb(ui.px(11.0))
            .border_b_1()
            .border_color(c.text.opacity(RULE))
            .text_size(ui.px(12.0))
            .child(div().flex_shrink_0().text_color(c.dim).child("Hooks"))
            .child(agent("Claude", found.claude, &found.claude_fired))
            .child(agent("Codex", found.codex, &found.codex_fired))
            .child(div().flex_1())
            .children(waiting)
    }

    /// Under the header: what cairn has for the next session, or why there is nothing.
    fn body(&self, found: &Found, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        match &found.body {
            Body::NoHooks => self
                .middle(ui)
                .child(self.title("cairn's hooks are not installed", ui))
                .child(self.sub(
                    "Nothing is recorded for Claude Code or Codex until they are. To install \
                     them, run in a terminal:",
                    ui,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(ui.px(4.0))
                        .max_w_full()
                        .px(ui.px(12.0))
                        .py(ui.px(7.0))
                        .rounded(ui.px(7.0))
                        .bg(c.text.opacity(SOFT))
                        .font(self.mono.clone())
                        .text_size(ui.px(11.5))
                        .line_height(relative(1.5))
                        .text_color(c.bright)
                        .child("cairn install --agent claude")
                        .child("cairn install --agent codex"),
                ),
            Body::Outside => self
                .middle(ui)
                .child(self.title("This folder has not adopted cairn", ui))
                .child(self.sub("Adopt is offered inside Git repositories only.", ui)),
            Body::NoRecords => self
                .middle(ui)
                .child(self.title("No records yet", ui))
                .child(self.sub(
                    "The first one is saved when an agent you talk to here finishes a turn.",
                    ui,
                )),
            Body::NotAdopted(repo) => match self.asking(repo) {
                Some(adopting) => self.confirm(found, repo, adopting, ui, cx),
                None => self.offer(repo, ui, cx),
            },
            Body::Records(records) => {
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(match &records.notes {
                        Some(notes) => self.notes(records, notes, found.branch.as_deref(), ui, cx),
                        // A cairn that does not list them: the text alone.
                        None => self.text(&records.text, ui),
                    })
            }
        }
    }

    /// The handover notes, scrolling: what they are, then the sessions the next session is given in
    /// one box, the others under the day of their note, Show older when cairn has more, and at the
    /// foot the exact text the next session receives, folded (DESIGN §13 P5-110). A branch tag is
    /// drawn only where it is not `branch`, the one the header shows.
    fn notes(
        &self,
        records: &Records,
        notes: &Notes,
        branch: Option<&str>,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let head = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(ui.px(4.0))
            .pt(ui.px(16.0))
            .px(ui.px(16.0))
            .pb(ui.px(4.0))
            .child(
                div()
                    .text_size(ui.px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.title)
                    .child("Handover notes"),
            )
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .line_height(relative(1.5))
                    .text_color(c.muted)
                    .child(
                        "cairn summarizes sessions in the background once they have been quiet \
                         for 3 minutes or have ended, skipping ones with nothing to carry over. \
                         A new session in this repository starts from the most recent \
                         summaries.",
                    ),
            );
        let label = |words: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .pt(ui.px(16.0))
                .px(ui.px(8.0))
                .pb(ui.px(7.0))
                .text_size(ui.px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(words)
        };
        let mut list = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .px(ui.px(8.0))
            .pb(ui.px(10.0));
        let mut nth = 0;
        if !notes.next.is_empty() {
            let raised = raised::raised(&self.theme, ui);
            let mut group = div()
                .flex()
                .flex_col()
                .rounded(ui.px(10.0))
                .border_1()
                .border_color(raised.edge)
                .bg(raised.face)
                .shadow(raised.shadows)
                .overflow_hidden();
            for (at, session) in notes.next.iter().enumerate() {
                let row = self.session(session, nth, true, branch, ui, cx);
                group = group.child(if at == 0 {
                    row
                } else {
                    row.border_t_1().border_color(c.accent.opacity(NEXT_RULE))
                });
                nth += 1;
            }
            list = list
                .child(
                    label("\u{2192} Next session starts from these")
                        .pt(ui.px(14.0))
                        .text_color(c.accent),
                )
                .child(group);
        }
        if !notes.days.is_empty() {
            list = list.child(label("Earlier").text_color(c.muted));
        }
        for day in &notes.days {
            let date = div()
                .font_weight(FontWeight::NORMAL)
                .text_color(c.dim)
                .child(day.date.clone());
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .pt(ui.px(8.0))
                    .px(ui.px(8.0))
                    .pb(ui.px(4.0))
                    .text_size(ui.px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.muted)
                    .map(|head| match day.name {
                        Some(name) => head.child(name).child(date),
                        None => head.child(day.date.clone()),
                    }),
            );
            for (at, session) in day.sessions.iter().enumerate() {
                let row = self.session(session, nth, false, branch, ui, cx);
                list = list.child(if at + 1 == day.sessions.len() {
                    row
                } else {
                    row.border_b_1().border_color(c.text.opacity(BETWEEN))
                });
                nth += 1;
            }
        }
        if notes.older {
            list = list.child(
                div().flex().mt(ui.px(10.0)).mx(ui.px(8.0)).child(
                    self.button("cairn-older", "Show older", false, ui)
                        .px(ui.px(12.0))
                        .rounded(ui.px(7.0))
                        .text_size(ui.px(12.0))
                        .text_color(c.bright)
                        .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                            view.limit += cairn::PAGE;
                            view.watch(cx);
                        })),
                ),
            );
        }
        let exact = div()
            .id("cairn-exact")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .px(ui.px(19.0))
            .py(ui.px(13.0))
            .border_t_1()
            .border_color(c.text.opacity(RULE))
            .text_size(ui.px(12.0))
            .text_color(c.muted)
            .cursor_pointer()
            .child(self.arrow(self.exact, c.muted, ui))
            .child("Exact text the next session receives")
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                view.exact = !view.exact;
                cx.notify();
            }));
        div()
            .id("cairn-notes")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .child(head)
            .child(list)
            // The fold sits at the foot of the tab while the notes leave room above it.
            .child(div().flex_1())
            .child(exact)
            .children(self.exact.then(|| {
                let text = div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .px(ui.px(16.0))
                    .pb(ui.px(20.0))
                    .line_height(relative(1.55));
                self.written(text, &records.text, false, ui)
            }))
    }

    /// One session, by its note: when that was saved, how long ago for one of today's, how many
    /// earlier ones it has, who saved it and on which branch, and its line; open, what `details`
    /// has in place of the line. One the next session is given (`next`) has its line under the
    /// time, wrapped in full (P5-137a), and its day before the time when that is not today; the
    /// others have one line of it beside the time, cut short.
    fn session(
        &self,
        session: &Session,
        nth: usize,
        next: bool,
        branch: Option<&str>,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let note = &session.note;
        let open = self.open.as_deref() == Some(note.id.as_str());
        let id = note.id.clone();
        let summary = || {
            (!open && !note.summary.is_empty()).then(|| {
                div()
                    .text_size(ui.px(12.5))
                    .line_height(relative(1.5))
                    .text_color(if next { c.text } else { c.muted })
                    .child(note.summary.clone())
            })
        };
        let time = div()
            .flex_shrink_0()
            .font(self.mono.clone())
            .text_size(ui.px(if next { 12.5 } else { 12.0 }))
            .when(next, |time| {
                time.font_weight(FontWeight::MEDIUM).text_color(c.title)
            })
            .when(!next, |time| time.text_color(c.muted))
            .child(match (&note.day, next) {
                (Some(day), true) => format!("{day} {}", note.time),
                _ => note.time.clone(),
            });
        let earlier = (!session.earlier.is_empty()).then(|| {
            let more = if session.more { "+" } else { "" };
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .px(ui.px(7.0))
                .py(ui.px(1.0))
                .rounded_full()
                .bg(c.text.opacity(SOFT))
                .text_size(ui.px(11.0))
                .text_color(c.muted)
                .child(format!("{}{more} earlier", session.earlier.len()))
        });
        let line = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .child(self.arrow(
                open,
                match (open, next) {
                    (true, _) => c.text,
                    (false, true) => c.muted,
                    (false, false) => c.dim,
                },
                ui,
            ))
            .child(time)
            .when(next, |line| {
                line.children(note.ago.clone().map(|ago| {
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .text_size(ui.px(11.5))
                        .text_color(c.dim)
                        .child(ago)
                }))
                .child(div().flex_1())
            })
            .when(!next, |line| {
                line.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .children(summary()),
                )
            })
            .children(earlier)
            .child(self.mark(note, ui))
            .children(self.branch_tag(note.branch.as_deref(), branch, ui));
        let head = div()
            .id(("cairn-session", nth))
            .flex()
            .flex_col()
            .gap(ui.px(5.0))
            .px(ui.px(if next { 10.0 } else { 8.0 }))
            .py(ui.px(if next { 10.0 } else { 8.0 }))
            .cursor_pointer()
            .child(line)
            .when(next, |head| {
                head.children(summary().map(|summary| summary.pl(ui.px(INDENT))))
            })
            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.toggle(&id, cx)));
        div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .child(head)
            .children(open.then(|| self.details(session, next, ui, cx)))
    }

    /// Which agent saved `note`: its icon, or with none its name.
    fn mark(&self, note: &Note, ui: &UiFont) -> AnyElement {
        let size = ui.px(MARK);
        match kind_icon::of(&note.agent) {
            Some(icon) if icon.tile() => icon.render_tile(size, ui.px(3.0)),
            Some(icon) => {
                let color = hsla(self.theme.fg(card::brand(&note.agent).color), 1.0);
                icon.render(size, color)
            }
            None => div()
                .flex_shrink_0()
                .text_size(ui.px(11.5))
                .text_color(self.colors.muted)
                .child(note.by.clone())
                .into_any_element(),
        }
    }

    /// The tag of the branch a note was saved on, unless it is `shown`, the header's.
    fn branch_tag(&self, branch: Option<&str>, shown: Option<&str>, ui: &UiFont) -> Option<Div> {
        let branch = branch.filter(|branch| Some(*branch) != shown)?;
        let c = self.colors;
        Some(
            div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .px(ui.px(7.0))
                .py(ui.px(1.0))
                .rounded_full()
                .bg(c.text.opacity(SOFT))
                .font(self.mono.clone())
                .text_size(ui.px(10.5))
                .text_color(c.muted)
                .child(branch.to_owned()),
        )
    }

    /// An open session, under its line: its note in full, then the session's earlier notes, the
    /// first few or all of them, each opening in place to its own.
    fn details(&self, session: &Session, next: bool, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let under = div()
            .flex()
            .flex_col()
            .pl(ui.px(if next { 10.0 } else { 8.0 } + INDENT))
            .pr(ui.px(12.0))
            .pb(ui.px(14.0))
            .child(self.full_note(&session.note.id, next, ui));
        if session.earlier.is_empty() {
            return under;
        }
        let count = format!(
            "{}{}",
            session.earlier.len(),
            if session.more { "+" } else { "" }
        );
        let shown = if self.all {
            session.earlier.len()
        } else {
            SHOWN_EARLIER
        };
        let mut earlier = div()
            .flex()
            .flex_col()
            .mt(ui.px(14.0))
            .pt(ui.px(9.0))
            .border_t_1()
            .border_color(if next {
                c.accent.opacity(NEXT_RULE)
            } else {
                c.text.opacity(RULE)
            })
            .child(
                div()
                    .pb(ui.px(4.0))
                    .text_size(ui.px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.muted)
                    .child(format!("Earlier in this session \u{b7} {count}")),
            );
        for (at, note) in session.earlier.iter().take(shown).enumerate() {
            let open = self.inner.as_deref() == Some(note.id.as_str());
            let id = note.id.clone();
            let time = match &note.day {
                Some(day) => format!("{day} {}", note.time),
                None => note.time.clone(),
            };
            earlier = earlier
                .child(
                    div()
                        .id(("cairn-earlier", at))
                        .flex()
                        .items_center()
                        .gap(ui.px(8.0))
                        .min_w(px(0.0))
                        .py(ui.px(4.0))
                        .cursor_pointer()
                        .child(self.arrow(open, if open { c.text } else { c.dim }, ui))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font(self.mono.clone())
                                .text_size(ui.px(11.5))
                                .text_color(c.dim)
                                .child(time),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(ui.px(12.0))
                                .text_color(c.muted)
                                .child(note.summary.clone()),
                        )
                        .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                            view.toggle_inner(&id, cx)
                        })),
                )
                .children(open.then(|| {
                    div()
                        .pl(ui.px(INDENT))
                        .pt(ui.px(4.0))
                        .pb(ui.px(10.0))
                        .child(self.full_note(&note.id, next, ui))
                }));
        }
        if shown < session.earlier.len() {
            earlier = earlier.child(
                div()
                    .id("cairn-all-earlier")
                    .pl(ui.px(INDENT))
                    .py(ui.px(4.0))
                    .text_size(ui.px(12.0))
                    .text_color(c.muted)
                    .cursor_pointer()
                    .child(format!("Show all {count}"))
                    .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                        view.all = true;
                        cx.notify();
                    })),
            );
        }
        under.child(earlier)
    }

    /// A note in full: when it was saved, who by, on which branch and as which record, then its
    /// text, and after a rule the correction written for it; before that is read, that it is
    /// being, and when it could not be, why. `next` when it sits in the box of what the next
    /// session is given.
    fn full_note(&self, id: &str, next: bool, ui: &UiFont) -> Div {
        let c = self.colors;
        let under = div().flex().flex_col();
        let full = match self.full.get(id) {
            Some(Some(Ok(full))) => full,
            Some(Some(Err(why))) => return under.child(self.refusal(why, ui)),
            _ => {
                return under
                    .text_size(ui.px(12.0))
                    .text_color(c.dim)
                    .child("Loading\u{2026}");
            }
        };
        let code = || div().font(self.mono.clone()).text_size(ui.px(11.0));
        let mut rows = vec![
            ("Saved", div().child(full.saved.clone())),
            ("By", self.by(&full.by, full.session.as_deref())),
        ];
        rows.extend(
            full.branch
                .clone()
                .map(|branch| ("Branch", code().child(branch))),
        );
        rows.push(("Record", code().text_color(c.muted).child(id.to_owned())));
        let facts = self.facts(rows, next, ui);
        let text = |text: &str| {
            let words = div()
                .flex()
                .flex_col()
                .text_size(ui.px(12.5))
                .line_height(relative(1.55));
            self.written(words, text, true, ui)
        };
        under
            .child(facts)
            .children(full.body.as_deref().map(|body| text(body).pt(ui.px(2.0))))
            .children(full.correction.as_ref().map(|(saved, body)| {
                div()
                    .flex()
                    .flex_col()
                    .child(div().mt(ui.px(14.0)).h(px(1.0)).bg(c.text.opacity(RULE)))
                    .child(
                        div()
                            .mt(ui.px(10.0))
                            .mb(ui.px(6.0))
                            .text_size(ui.px(11.5))
                            .text_color(c.dim)
                            .child(format!("Corrected {saved}")),
                    )
                    .children(body.as_deref().map(&text))
            }))
    }

    /// The facts about a note or a ledger entry, each after its label, on a ground of their own:
    /// for one on a raised box's face (`raised`), the panel's ground through it.
    fn facts(&self, rows: Vec<(&'static str, Div)>, raised: bool, ui: &UiFont) -> Div {
        let c = self.colors;
        let mut facts = div()
            .flex()
            .flex_col()
            .gap(ui.px(4.0))
            .px(ui.px(11.0))
            .py(ui.px(9.0))
            .rounded(ui.px(7.0))
            .bg(if raised {
                c.ground.opacity(FACTS_ON_NEXT)
            } else {
                c.text.opacity(FACTS)
            })
            .text_size(ui.px(11.5))
            .line_height(relative(1.45));
        for (label, value) in rows {
            facts = facts.child(
                div()
                    .flex()
                    .gap(ui.px(8.0))
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(ui.px(50.0))
                            .text_color(c.muted)
                            .child(label),
                    )
                    .child(value.flex_1().min_w(px(0.0))),
            );
        }
        facts
    }

    /// Who wrote a note or a ledger entry, and for an agent which of its sessions, in the
    /// monospace font.
    fn by(&self, by: &str, session: Option<&str>) -> Div {
        match session {
            Some(session) => {
                let words = format!("{by}, session {session}");
                let session = words.len() - session.len()..words.len();
                div().child(
                    StyledText::new(words)
                        .with_font_family_overrides(vec![(session, self.mono.family.clone())]),
                )
            }
            None => div().child(by.to_owned()),
        }
    }

    /// Under the hooks where cairn keeps a ledger: the switch between the notes, the ledger and the
    /// lessons, a well at the left as wide as its words, the page shown the chosen capsule of the
    /// accent set in it, as the sidebar's machine switcher draws it, the ledger's and the lessons'
    /// with how many entries each has; on those two pages, what it is to the right (P5-118a,
    /// P5-121, P5-121a).
    fn switch(&self, ledger: &Ledger, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let well = sunk::well(c.ground, c.ground.l >= 0.5, c.text);
        let (count, lessons) = match ledger {
            Ledger::Groups(groups) => {
                let (book, lessons) = split(groups);
                (
                    book.iter().map(|group| group.entries.len()).sum(),
                    lessons.map_or(0, |group| group.entries.len()),
                )
            }
            Ledger::Unsupported | Ledger::Failed(_) => (0, 0),
        };
        let count = (count > 0).then_some(count);
        let lessons = (lessons > 0).then_some(lessons);
        let half = |id: &'static str, page: Page, words: &'static str, count: Option<usize>| {
            let on = self.page == page;
            sunk::chosen(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .px(ui.px(14.0))
                    .py(ui.px(4.0))
                    .cursor_pointer(),
                on,
                c.accent,
                c.text,
                c.title,
            )
            .child(words)
            .children(count.map(|count| sunk::count(count, c.text, ui)))
            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                if view.page != page {
                    view.page = page;
                    cx.notify();
                }
            }))
        };
        let capsule = div()
            .flex_shrink_0()
            .flex()
            .p(ui.px(3.0))
            .rounded_full()
            .bg(well.ground)
            .shadow(well.shadows)
            .text_size(ui.px(12.5))
            .child(half("cairn-page-notes", Page::Notes, "Notes", None))
            .child(half("cairn-page-ledger", Page::Ledger, "Ledger", count))
            .child(half(
                "cairn-page-lessons",
                Page::Lessons,
                "Lessons",
                lessons,
            ));
        let hint = match self.page {
            Page::Notes => None,
            Page::Ledger => Some("Kept across sessions by cairn"),
            Page::Lessons => Some("Learned across sessions by cairn"),
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .min_w(px(0.0))
            .mx(ui.px(16.0))
            .mt(ui.px(12.0))
            .child(capsule)
            .child(div().flex_1())
            .children(hint.map(|hint| {
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(11.5))
                    .text_color(c.dim)
                    .child(hint)
            }))
    }

    /// The ledger's page: its entries, or that it has none yet, or why there are none to show.
    fn ledger(&self, ledger: &Ledger, ui: &UiFont, cx: &mut Context<Self>) -> AnyElement {
        match ledger {
            Ledger::Unsupported => self
                .middle(ui)
                .child(self.title("This cairn keeps no ledger", ui))
                .child(self.sub("The project ledger comes with cairn 0.5.0.", ui))
                .into_any_element(),
            Ledger::Failed(why) => self
                .middle(ui)
                .gap(ui.px(10.0))
                .child(self.title("Could not read the ledger", ui))
                .child(self.refusal(why, ui))
                .into_any_element(),
            Ledger::Groups(groups) => match split(groups).0 {
                book if book.is_empty() => self
                    .middle(ui)
                    .child(self.title("No ledger entries yet", ui))
                    .child(self.sub(
                        "cairn adds the first ones after a session here ends. You can also add \
                         entries with `cairn ledger add`.",
                        ui,
                    ))
                    .into_any_element(),
                book => self.entries(&book, false, ui, cx).into_any_element(),
            },
        }
    }

    /// The lessons' page: the lessons cairn has drawn from the sessions here (0.6.0), kept apart
    /// from the ledger, or that there are none yet, or why there are none to show (P5-121).
    fn lessons(&self, ledger: &Ledger, ui: &UiFont, cx: &mut Context<Self>) -> AnyElement {
        match ledger {
            Ledger::Unsupported => self
                .middle(ui)
                .child(self.title("This cairn keeps no lessons", ui))
                .child(self.sub("Lessons come with cairn 0.6.0.", ui))
                .into_any_element(),
            Ledger::Failed(why) => self
                .middle(ui)
                .gap(ui.px(10.0))
                .child(self.title("Could not read the lessons", ui))
                .child(self.refusal(why, ui))
                .into_any_element(),
            Ledger::Groups(groups) => match split(groups).1 {
                Some(lessons) => self.entries(&[lessons], true, ui, cx).into_any_element(),
                None => self
                    .middle(ui)
                    .child(self.title("No lessons yet", ui))
                    .child(self.sub(
                        "cairn keeps a lesson when a session here learns something worth \
                         remembering (cairn 0.6.0). You can also add one with `cairn ledger add \
                         lesson`.",
                        ui,
                    ))
                    .into_any_element(),
            },
        }
    }

    /// The filter over a page's entries, under the switch: a well with a magnifier, the field, and
    /// `⌘F`; with something typed, its edge in the accent, how many of `total` it finds, and ×
    /// to clear it. A click anywhere in it puts the keyboard in the field.
    fn filter(
        &self,
        input: &Entity<TextInput>,
        found: usize,
        total: usize,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let typed = !input.read(cx).text().trim().is_empty();
        let well = sunk::well(c.ground, c.ground.l >= 0.5, c.text);
        let field = input.read(cx).focus_handle(cx);
        let small = || {
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(ui.px(11.0))
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .mx(ui.px(16.0))
            .mt(ui.px(10.0))
            .h(ui.px(28.0))
            .px(ui.px(9.0))
            .rounded(ui.px(7.0))
            .bg(well.ground)
            .border_1()
            .border_color(if typed {
                c.accent
            } else {
                c.text.opacity(FILTER_EDGE)
            })
            .text_size(ui.px(12.0))
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&field, cx);
                // Not the tab's own hold, which would take the keyboard back.
                window.prevent_default();
            })
            .child(footer_icon::icon(
                Icon::Search,
                if typed { c.muted } else { c.dim },
                ui.scale(12.0 / footer_icon::SIZE),
            ))
            .child(div().flex_1().min_w(px(0.0)).child(input.clone()))
            .map(|row| {
                if typed {
                    row.child(
                        small()
                            .text_color(c.muted)
                            .child(format!("{found} of {total}")),
                    )
                    .child(
                        div()
                            .id("cairn-filter-clear")
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(ui.px(16.0))
                            .rounded_full()
                            .bg(c.text.opacity(CLEAR))
                            .text_size(ui.px(11.0))
                            .text_color(c.muted)
                            .cursor_pointer()
                            .hover(move |style| style.text_color(c.title))
                            .child("\u{d7}")
                            .on_click(cx.listener(|view, _: &ClickEvent, window, cx| {
                                view.clear_filter(&menu::CloseFind, window, cx)
                            })),
                    )
                } else {
                    row.child(small().text_color(c.dim).child("\u{2318}F"))
                }
            })
    }

    /// A page's entries under their filter: how many the next session is not given if any, then
    /// each kind under its name, one that folds, in a raised box; the lessons, one kind with the
    /// switch to name it, in the box alone. While a filter is typed, only what it finds, in the
    /// kinds it finds them in, each of those open, or that it finds nothing.
    fn entries(
        &self,
        groups: &[&Group],
        lessons: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let input = if lessons {
            &self.lessons_filter
        } else {
            &self.ledger_filter
        };
        let query = input.read(cx).text().trim().to_owned();
        let typed = !query.is_empty();
        let shown = cairn::filter(groups, &query);
        let total = groups.iter().map(|group| group.entries.len()).sum();
        let found = shown.iter().map(|(_, hits)| hits.len()).sum();
        let filter = self.filter(input, found, total, ui, cx);
        let left_out = groups
            .iter()
            .flat_map(|group| &group.entries)
            .filter(|entry| !entry.given)
            .count();
        let (one, many) = if lessons {
            ("lesson is", "lessons are")
        } else {
            ("entry is", "entries are")
        };
        let head = (left_out > 0).then(|| {
            div()
                .flex_shrink_0()
                .pt(ui.px(10.0))
                .px(ui.px(16.0))
                .text_size(ui.px(11.5))
                .line_height(relative(1.5))
                .text_color(c.dim)
                .child(match left_out {
                    1 => format!("1 {one} left out of what the next session receives."),
                    _ => format!("{left_out} {many} left out of what the next session receives."),
                })
        });
        let raised = raised::raised(&self.theme, ui);
        let mut list = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .px(ui.px(8.0))
            .pb(ui.px(16.0))
            .when(lessons, |list| list.pt(ui.px(12.0)));
        let mut nth = 0;
        for (at, (group, hits)) in shown.iter().enumerate() {
            let name = group.name;
            // The lessons are one group, always open; a kind of the ledger is open as the user
            // left it in this repository, and while filtering whenever it has what was found.
            let open = lessons
                || typed
                || self
                    .repo
                    .as_deref()
                    .is_none_or(|repo| self.folds.open(repo, name));
            if !lessons {
                let count = if typed {
                    format!("{} of {}", hits.len(), group.entries.len())
                } else {
                    group.entries.len().to_string()
                };
                list = list.child(
                    div()
                        .id(("cairn-group", at))
                        .flex()
                        .items_center()
                        .gap(ui.px(7.0))
                        .pt(ui.px(14.0))
                        .px(ui.px(8.0))
                        .pb(ui.px(if open { 7.0 } else { 0.0 }))
                        .text_size(ui.px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if open { c.title } else { c.text })
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(ui.px(10.0))
                                .text_size(ui.px(9.0))
                                .text_color(c.muted)
                                .child(if open { "\u{25be}" } else { "\u{25b8}" }),
                        )
                        .child(name)
                        .child(
                            div()
                                .px(ui.px(7.0))
                                .rounded_full()
                                .bg(c.text.opacity(SOFT))
                                .font_weight(FontWeight::NORMAL)
                                .text_size(ui.px(11.5))
                                .text_color(c.muted)
                                .child(count),
                        )
                        // While filtering the groups stay as they were left; clearing it shows them so.
                        .when(!typed, |head| {
                            head.cursor_pointer()
                                .hover(move |style| style.text_color(c.title))
                                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                    view.fold(name, cx)
                                }))
                        }),
                );
            }
            if !open {
                continue;
            }
            let mut rows = div()
                .flex()
                .flex_col()
                .rounded(ui.px(10.0))
                .border_1()
                .border_color(raised.edge)
                .bg(raised.face)
                .shadow(raised.shadows.clone())
                .overflow_hidden();
            for (at, entry) in hits.iter().enumerate() {
                let row = self.entry(entry, nth, &query, ui, cx);
                rows = rows.child(if at == 0 {
                    row
                } else {
                    row.border_t_1().border_color(c.text.opacity(BETWEEN))
                });
                nth += 1;
            }
            list = list.child(rows);
        }
        if typed && found == 0 {
            let what = if lessons { "lessons" } else { "entries" };
            list = list.child(
                div()
                    .pt(ui.px(14.0))
                    .px(ui.px(8.0))
                    .text_size(ui.px(12.5))
                    .line_height(relative(1.5))
                    .text_color(c.muted)
                    .child(format!("No {what} match \u{201c}{query}\u{201d}")),
            );
        }
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(filter)
            .child(
                div()
                    .id(if lessons {
                        "cairn-lessons"
                    } else {
                        "cairn-ledger"
                    })
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(head)
                    .child(list),
            )
    }

    /// One ledger entry, one line: its number, its title cut short to fit, what `query` found in
    /// it lit, and the day it was added; open, its text in full and the facts about it. The title
    /// of one the next session is not given is quiet.
    fn entry(
        &self,
        entry: &Entry,
        nth: usize,
        query: &str,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let open = self.entry.as_deref() == Some(entry.id.as_str());
        let body = entry.body.as_deref().filter(|body| !body.trim().is_empty());
        let id = entry.id.clone();
        let lit = HighlightStyle {
            color: Some(c.title),
            background_color: Some(c.yellow.opacity(LIT)),
            ..HighlightStyle::default()
        };
        let title = StyledText::new(entry.title.clone()).with_highlights(
            cairn::found(&entry.title, query)
                .into_iter()
                .map(|range| (range, lit)),
        );
        let line = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .child(
                div()
                    .flex_shrink_0()
                    .h(ui.px(18.0))
                    .flex()
                    .items_center()
                    .child(self.arrow(open, if open { c.text } else { c.dim }, ui)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(NO))
                    .flex()
                    .justify_end()
                    .font(self.mono.clone())
                    .text_size(ui.px(11.5))
                    .line_height(ui.px(18.0))
                    .text_color(c.dim)
                    .child(format!("#{}", entry.no)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(12.5))
                    .line_height(ui.px(18.0))
                    .map(|title| match (open, entry.given) {
                        (true, _) => title.font_weight(FontWeight::MEDIUM).text_color(c.title),
                        (false, true) => title.text_color(c.text),
                        (false, false) => title.text_color(c.muted),
                    })
                    .child(title),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .text_size(ui.px(11.0))
                    .text_color(c.dim)
                    .child(entry.day.clone()),
            );
        let head = div()
            .id(("cairn-entry", nth))
            .flex()
            .flex_col()
            .px(ui.px(10.0))
            .py(ui.px(6.0))
            .cursor_pointer()
            .child(line)
            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                view.entry = if view.entry.as_deref() == Some(id.as_str()) {
                    None
                } else {
                    Some(id.clone())
                };
                cx.notify();
            }));
        div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .child(head)
            .children(open.then(|| {
                let code = || div().font(self.mono.clone()).text_size(ui.px(11.0));
                let mut rows = vec![("Added", div().child(entry.added.clone()))];
                rows.extend(
                    entry
                        .updated
                        .clone()
                        .map(|updated| ("Updated", div().child(updated))),
                );
                rows.push(("By", self.by(&entry.by, entry.session.as_deref())));
                rows.push(("Entry", code().text_color(c.muted).child(entry.id.clone())));
                let text = body.map(|body| {
                    let words = div()
                        .flex()
                        .flex_col()
                        .pb(ui.px(10.0))
                        .text_size(ui.px(12.5))
                        .line_height(relative(1.55));
                    self.written(words, body, true, ui)
                });
                div()
                    .flex()
                    .flex_col()
                    .pl(ui.px(10.0 + ENTRY_INDENT))
                    .pr(ui.px(12.0))
                    .pb(ui.px(12.0))
                    .children(text)
                    .child(self.facts(rows, true, ui))
                    .children((!entry.given).then(|| {
                        div()
                            .pt(ui.px(8.0))
                            .text_size(ui.px(11.5))
                            .text_color(c.dim)
                            .child("Left out of what the next session receives.")
                    }))
            }))
    }

    /// The arrow before what folds: down when it is open.
    fn arrow(&self, open: bool, color: Hsla, ui: &UiFont) -> impl IntoElement + use<> {
        footer_icon::icon(
            if open { Icon::Down } else { Icon::Forward },
            color,
            ui.scale(ARROW / footer_icon::SIZE),
        )
    }

    /// A repository that has not adopted cairn: what adopting does, where, and Adopt….
    fn offer(&self, repo: &Path, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let ask = repo.to_owned();
        self.middle(ui)
            .gap(ui.px(12.0))
            .pb(ui.px(48.0))
            .child(footer_icon::icon(
                Icon::Cairn,
                c.dim,
                ui.scale(34.0 / footer_icon::SIZE),
            ))
            .child(self.title("This repository has not adopted cairn", ui))
            .child(self.sub(
                "Once adopted, cairn summarizes the sessions you have here in the background, and \
                 each new session starts from the most recent ones.",
                ui,
            ))
            .child(self.place(repo, ui))
            .child(
                self.button("cairn-adopt-ask", "Adopt\u{2026}", true, ui)
                    .mt(ui.px(6.0))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        view.adopting.ask(&ask);
                        cx.notify();
                    })),
            )
    }

    /// Adopt… was pressed: what it does and how to undo it, then Cancel and Adopt; while it runs
    /// and once it has, that, and after it failed, why.
    fn confirm(
        &self,
        found: &Found,
        repo: &Path,
        adopting: &Adopting,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let words = |text: &str, color: Hsla| {
            div()
                .text_size(ui.px(12.5))
                .line_height(relative(1.55))
                .text_color(color)
                .child(self.words(text))
        };
        let foot = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(ui.px(8.0))
            .mt(ui.px(4.0));
        let under_way = match adopting {
            Adopting::Running(_) => Some("Adopting\u{2026}"),
            Adopting::Done(_) => Some("Adopted"),
            _ => None,
        };
        let foot = if let Some(words) = under_way {
            foot.h(ui.px(30.0))
                .text_size(ui.px(12.5))
                .text_color(c.dim)
                .child(words)
        } else {
            foot.child(
                self.button("cairn-adopt-cancel", "Cancel", false, ui)
                    .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                        if view.adopting.cancel() {
                            cx.notify();
                        }
                    })),
            )
            .child(
                self.button("cairn-adopt-confirm", "Adopt", true, ui)
                    .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.adopt(cx))),
            )
        };
        let card = div()
            .flex()
            .flex_col()
            .gap(ui.px(10.0))
            .pt(ui.px(16.0))
            .px(ui.px(16.0))
            .pb(ui.px(14.0))
            .rounded(ui.px(12.0))
            .border_1()
            .border_color(c.accent.opacity(CARD_EDGE))
            .bg(c.text.opacity(CARD_FACE))
            .child(
                div()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.title)
                    .child(format!("Adopt cairn in {}?", found.name)),
            )
            .child(div().flex().child(self.place(repo, ui)))
            .child(words(
                "This takes effect right away. cairn summarizes the Claude and Codex sessions you \
                 talk to in this repository in the background, the open ones included from their \
                 next turn, and new sessions start from those summaries.",
                c.bright,
            ))
            .child(words(
                "Dispatched agents are not affected. To undo, run `cairn unadopt` in this \
                 repository.",
                c.muted,
            ))
            .children(match adopting {
                Adopting::Refused(_, why) => Some(self.refusal(why, ui)),
                _ => None,
            })
            .child(foot);
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .justify_center()
            .px(ui.px(20.0))
            .pb(ui.px(48.0))
            .child(card)
    }

    /// The text the next session would be given, as Markdown, scrolling.
    fn text(&self, text: &str, ui: &UiFont) -> Stateful<Div> {
        let out = div()
            .id("cairn-text")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pt(ui.px(4.0))
            .px(ui.px(16.0))
            .pb(ui.px(20.0))
            .line_height(relative(1.55));
        self.written(out, text, false, ui)
    }

    /// `text` as Markdown, its blocks added to `out`. cairn writes its sections as third-level
    /// headings and the parts of a record as second-level ones: a section is a quiet label with a
    /// rule after it, a part a small bold heading, no larger than the words in an open `note`.
    fn written<E: ParentElement>(&self, mut out: E, text: &str, note: bool, ui: &UiFont) -> E {
        /// What the block before was, for the room between the two.
        #[derive(Clone, Copy, PartialEq)]
        enum Before {
            Nothing,
            Heading,
            Words,
            Item,
        }
        let c = self.colors;
        let mut before = Before::Nothing;
        for block in markdown::blocks_by_line(text) {
            let apart = |after_words: f32, after_item: f32| match before {
                Before::Nothing | Before::Heading => 0.0,
                Before::Words => after_words,
                Before::Item => after_item,
            };
            let (element, now) = match &block {
                markdown::Block::Heading(3, spans) => (
                    div()
                        .flex()
                        .items_center()
                        .gap(ui.px(10.0))
                        .mt(ui.px(if before == Before::Nothing {
                            14.0
                        } else {
                            20.0
                        }))
                        .mb(ui.px(8.0))
                        .text_size(ui.px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.muted)
                        .child(
                            div()
                                .flex_shrink(1.0)
                                .min_w(px(0.0))
                                .child(self.styled(spans)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(ui.px(12.0))
                                .h(px(1.0))
                                .bg(c.text.opacity(CODE)),
                        ),
                    Before::Heading,
                ),
                markdown::Block::Heading(_, spans) => (
                    div()
                        .mt(ui.px(if note { 14.0 } else { 16.0 }))
                        .mb(ui.px(4.0))
                        .text_size(ui.px(if note { 12.5 } else { 13.5 }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.title)
                        .child(self.styled(spans)),
                    Before::Heading,
                ),
                markdown::Block::Paragraph(spans) => (
                    div().mt(ui.px(apart(12.0, 12.0))).child(self.styled(spans)),
                    Before::Words,
                ),
                markdown::Block::Item {
                    depth,
                    marker,
                    spans,
                } => {
                    let mark = match marker {
                        Some(markdown::Marker::Bullet) => div()
                            .mt(ui.px(8.0))
                            .size(ui.px(4.0))
                            .rounded_full()
                            .bg(c.dim),
                        Some(markdown::Marker::Number(n)) => {
                            div().text_color(c.dim).child(format!("{n}."))
                        }
                        // A further paragraph of the item above, under its words.
                        None => div().w(ui.px(4.0)),
                    };
                    (
                        div()
                            .flex()
                            .gap(ui.px(8.0))
                            .mt(ui.px(apart(6.0, 4.0)))
                            .pl(ui.px(18.0 * *depth as f32))
                            .child(mark.flex_shrink_0())
                            .child(div().flex_1().min_w(px(0.0)).child(self.styled(spans))),
                        Before::Item,
                    )
                }
                markdown::Block::Code(code) => (
                    div()
                        .mt(ui.px(apart(8.0, 8.0)))
                        .px(ui.px(10.0))
                        .py(ui.px(8.0))
                        .rounded(ui.px(6.0))
                        .bg(c.text.opacity(SOFT))
                        .font(self.mono.clone())
                        .text_size(ui.px(11.5))
                        .child(code.clone()),
                    Before::Words,
                ),
                markdown::Block::Table { head, rows } => {
                    let colors = crate::markdown_view::Colors {
                        bright: c.title,
                        text: c.text,
                        marker: c.dim,
                        link: c.accent,
                        code: c.text.opacity(CODE),
                        rule: c.text.opacity(CODE),
                    };
                    let mono = &self.mono.family;
                    (
                        crate::markdown_view::table(head, rows, mono, colors, ui)
                            .mt(ui.px(apart(8.0, 8.0))),
                        Before::Words,
                    )
                }
            };
            out = out.child(element.flex_shrink_0());
            before = now;
        }
        out
    }

    /// The room under the header, what is in it in the middle.
    fn middle(&self, ui: &UiFont) -> Div {
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(ui.px(8.0))
            .px(ui.px(36.0))
            .pb(ui.px(20.0))
    }

    fn title(&self, text: &'static str, ui: &UiFont) -> Div {
        div()
            .text_size(ui.px(14.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.colors.title)
            .text_center()
            .child(text)
    }

    /// A quieter line or two under a title.
    fn sub(&self, text: &str, ui: &UiFont) -> Div {
        div()
            .text_size(ui.px(12.5))
            .line_height(relative(1.55))
            .text_color(self.colors.muted)
            .text_center()
            .child(self.words(text))
    }

    /// A repository's path, the home directory written `~`, as a small tag that breaks anywhere.
    fn place(&self, repo: &Path, ui: &UiFont) -> Div {
        let c = self.colors;
        div()
            .max_w_full()
            .min_w(px(0.0))
            .px(ui.px(8.0))
            .py(ui.px(2.0))
            .rounded(ui.px(6.0))
            .bg(c.text.opacity(SOFT))
            .font(self.mono.clone())
            .text_size(ui.px(11.0))
            .text_color(c.muted)
            .child(cairn::tilde(repo, self.home.as_deref()))
    }

    /// Why cairn could not be read, or would not adopt: the first line of what it said.
    fn refusal(&self, why: &str, ui: &UiFont) -> Div {
        let c = self.colors;
        div()
            .max_w_full()
            .min_w(px(0.0))
            .px(ui.px(10.0))
            .py(ui.px(6.0))
            .rounded(ui.px(7.0))
            .bg(c.red.opacity(0.1))
            .font(self.mono.clone())
            .text_size(ui.px(11.5))
            .line_height(relative(1.5))
            .text_color(c.red)
            .child(why.to_owned())
    }

    /// A button: the one that goes on in the accent, the other quiet.
    fn button(
        &self,
        id: &'static str,
        words: &'static str,
        goes_on: bool,
        ui: &UiFont,
    ) -> Stateful<Div> {
        let c = self.colors;
        let button = div()
            .id(id)
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(ui.px(30.0))
            .rounded(ui.px(8.0))
            .whitespace_nowrap()
            .text_size(ui.px(12.5))
            .cursor_pointer()
            .child(words);
        if goes_on {
            button
                .px(ui.px(18.0))
                .bg(c.accent)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.ground)
                .hover(move |style| style.bg(c.accent.opacity(0.88)))
        } else {
            button
                .px(ui.px(14.0))
                .bg(c.text.opacity(SOFT))
                .text_color(c.text)
                .hover(move |style| style.bg(c.text.opacity(0.1)))
        }
    }

    /// A line of the tab's own words, what is between backticks written as code.
    fn words(&self, text: &str) -> StyledText {
        match markdown::blocks(text).into_iter().next() {
            Some(markdown::Block::Paragraph(spans)) => self.styled(&spans),
            _ => StyledText::new(text.to_owned()),
        }
    }

    /// Markdown's words in their styles, as the Kanban task dialog's Preview draws them: bold,
    /// italic, code in the monospace font on a faint ground, and links underlined in the accent.
    fn styled(&self, spans: &[markdown::Span]) -> StyledText {
        let c = self.colors;
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
                style.color = Some(c.accent);
                style.underline = Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(c.accent),
                    wavy: false,
                });
            }
            if span.code {
                style.background_color = Some(c.text.opacity(CODE));
                fonts.push((range.clone(), self.mono.family.clone()));
            }
            if style != HighlightStyle::default() {
                highlights.push((range, style));
            }
        }
        StyledText::new(text)
            .with_highlights(highlights)
            .with_font_family_overrides(fonts)
    }
}
