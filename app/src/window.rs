//! The window's root view: the title bar, where the tabs share the traffic lights' row; below it
//! the Agents sidebar on the left and the active tab's panes on the right, each a card, a split
//! pane's header at its top. `layout.rs` holds the rules; this file draws them and keeps one
//! terminal view per pane.
use crate::{
    agents::Panel,
    attention::Kind as AttentionKind,
    card,
    config::Config,
    corral::{Agent, Role},
    diagnostics::{Report, Startup},
    find,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kind_icon,
    layout::{Axis, Direction, Node, PaneId, Placement, Shown, Workspace},
    layout_state::{Content, Layout, Store},
    menu,
    new_agent::{self, Place, Started},
    new_agent_view::Seed,
    pet::PetView,
    popover::{self, Hang, Placed, Tone},
    right_panel::{self, RightPanel, Room, Tab as RightTab},
    search::{self, Lead, Mode, Target},
    settings::{Conflict, Draft, Saved},
    sidebar::{self, Sidebar, SidebarEvent, status_dot},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::{Launch, Options, TerminalView, hsla},
    viewer::AgentMetadata,
    windows,
};
use gpui::{
    AnyElement, Bounds, BoxShadow, ClickEvent, Context, Div, DragMoveEvent, Entity, ExternalPaths,
    FocusHandle, Focusable, FontWeight, HighlightStyle, Hsla, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, Point, PromptLevel, Render, ScrollHandle, SharedString, Stateful,
    StyledText, Task, Window, canvas, div, point, prelude::*, px, relative,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

/// What "New shell" and the `Shell` choice start.
pub struct NewShell {
    pub program: String,
    pub cwd: String,
}

/// The small panel open over the window.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Popup {
    /// Choose what the new tab shows, from the `+`.
    NewTab,
    /// Choose the side and what the new pane shows in one go, from the pane's split button or
    /// the title bar's split icon.
    Split(Direction),
    /// The Attention list.
    Attention,
    /// The command palette.
    Palette,
    /// The sidebar's menu of actions, over its button.
    Actions,
}

/// What dragging the divider carries: nothing, only that it is the divider.
struct SidebarDrag;

/// The released sidebar width, formatted for the settings draft.
fn finish_sidebar_width(width: &mut f32) -> String {
    *width = width.round();
    width.to_string()
}

/// What dragging the right sidebar's divider carries.
struct RightDrag;

/// The command palette while it is open.
struct Palette {
    input: Entity<TextInput>,
    /// The selected row, counting every group's rows in order.
    index: usize,
    list: ScrollHandle,
    /// The menu bar's commands.
    commands: Vec<search::Row>,
    /// What the last text search found, and the one waiting for typing to pause.
    text: Option<TextResults>,
    searching: Option<Task<()>>,
}

/// A text search's matches in the open panes, newest first, for the text it looked for.
struct TextResults {
    needle: String,
    /// The panes with matches, the active one first: its matches, and whether it has more.
    panes: Vec<(PaneId, Vec<find::Hit>, bool)>,
}

/// How long typing pauses before the palette searches the panes' text.
const PAUSE: Duration = Duration::from_millis(120);

/// A close waiting on an answer, by a pane in it, so it still finds its target if the layout
/// changes meanwhile.
#[derive(Clone, Copy)]
enum Closing {
    Pane(PaneId),
    /// The tab holding this pane.
    Tab(PaneId),
}

/// The system prompt before live shells end.
#[derive(Debug, PartialEq)]
pub struct Question {
    pub message: String,
    pub detail: String,
    pub confirm: &'static str,
}

impl Question {
    /// Shows it; resolves to whether the confirming button was chosen.
    fn ask(
        &self,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> impl std::future::Future<Output = bool> + use<> {
        let answer = window.prompt(
            PromptLevel::Warning,
            &self.message,
            Some(&self.detail),
            &[self.confirm, "Cancel"],
            cx,
        );
        async move { matches!(answer.await, Ok(0)) }
    }
}

/// What to ask before closing ends `shells` (their titles) and detaches `agents` views, or quitting
/// does: `None` when no live shell would end, so nothing needs asking. As in Saddle.
pub fn close_question(shells: &[String], agents: usize, quit: bool) -> Option<Question> {
    if shells.is_empty() {
        return None;
    }
    let (them, it, button) = if shells.len() == 1 {
        ("shell", "it", "End Shell")
    } else {
        ("shells", "them", "End Shells")
    };
    let message = if quit {
        format!("Quit paddock and end the running {them}?")
    } else {
        format!("End the running {them}?")
    };
    let mut detail = shells.join("\n");
    detail.push_str(&format!(
        "\n\nWhat runs in {it} in the foreground ends too."
    ));
    if agents > 0 {
        detail.push_str(" Agent views only detach; the agents keep running.");
    }
    Some(Question {
        message,
        detail,
        confirm: if quit { "Quit" } else { button },
    })
}

/// Something to open in a new tab or pane.
#[derive(Clone, Debug, PartialEq)]
enum Choice {
    Shell,
    /// The New Agent window, to start one there.
    NewAgent,
    Agent(String),
}

/// The new tab and split panels while open: the field the new tab panel filters its agents with,
/// and the selected row.
#[derive(Default)]
struct Chooser {
    input: Option<Entity<TextInput>>,
    index: usize,
    list: ScrollHandle,
}

/// The child of the chooser's list that shows row `index`: after the first heading, and in the
/// new tab panel after the agents' heading too.
fn chooser_child(new_tab: bool, index: usize) -> usize {
    if new_tab && index >= 2 {
        index + 2
    } else {
        index + 1
    }
}

/// What a panel can hang from, as last drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Spot {
    /// The title bar's `+`.
    NewTab,
    /// The title bar's split icon.
    Split,
    /// The Attention bell, in the title bar while the sidebar is expanded.
    Bell,
    Pane(PaneId),
}

/// Every word of `query` in the agent's full name (its project, then its own), ignoring case.
fn agent_matches(name: &str, query: &str) -> bool {
    let name = name.to_lowercase();
    query
        .split_whitespace()
        .all(|word| name.contains(&word.to_lowercase()))
}

/// The new tab and split panels' rows: Shell and Agent… always, then the agents (`names`, sorted)
/// that match what is typed.
fn choices(names: &[String], query: &str) -> Vec<Choice> {
    let agents = names
        .iter()
        .filter(|name| agent_matches(name, query))
        .map(|name| Choice::Agent(name.clone()));
    [Choice::Shell, Choice::NewAgent]
        .into_iter()
        .chain(agents)
        .collect()
}

/// The row selected as the list changes: Shell before anything is typed, then the first agent
/// that matches, as that is what typing looks for.
fn first_choice(query: &str, choices: &[Choice]) -> usize {
    if query.trim().is_empty() {
        return 0;
    }
    choices
        .iter()
        .position(|choice| matches!(choice, Choice::Agent(_)))
        .unwrap_or(0)
}

/// How the split panel was asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SplitAsk {
    /// The pane's split button.
    Button,
    /// The title bar's split icon.
    Bar,
    /// A shortcut or menu item for that side.
    Side(Direction),
}

/// The popup after asking for the split panel while `open` is: the button opens it on Right; a
/// side's shortcut opens it on that side, switches an open one to it, and closes it when it is
/// already there.
fn split_popup(open: Option<Popup>, ask: SplitAsk) -> Option<Popup> {
    match ask {
        SplitAsk::Button | SplitAsk::Bar => Some(Popup::Split(Direction::Right)),
        SplitAsk::Side(direction) if open == Some(Popup::Split(direction)) => None,
        SplitAsk::Side(direction) => Some(Popup::Split(direction)),
    }
}

/// What the split panel hangs from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SplitFrom {
    /// The title bar's split icon.
    Bar,
    /// The active pane's split button, in its header.
    Pane,
}

/// What the split panel hangs from after asking for it, `hanging` from where it is when already
/// open, with or without pane headers: each button from itself; a shortcut or the menu from the
/// active pane's button, or from the title bar's icon when no pane has a header. Switching the
/// side of an open panel leaves it where it hangs.
fn split_from(ask: SplitAsk, hanging: Option<SplitFrom>, header: bool) -> SplitFrom {
    match ask {
        SplitAsk::Bar => SplitFrom::Bar,
        SplitAsk::Button => SplitFrom::Pane,
        SplitAsk::Side(_) => hanging.unwrap_or(if header {
            SplitFrom::Pane
        } else {
            SplitFrom::Bar
        }),
    }
}

/// The split panel's heading over what to open, for the side chosen.
fn open_where(direction: Direction) -> &'static str {
    match direction {
        Direction::Left => "OPEN ON THE LEFT",
        Direction::Right => "OPEN ON THE RIGHT",
        Direction::Up => "OPEN ABOVE",
        Direction::Down => "OPEN BELOW",
    }
}

/// How long a tab's × takes to fade in or out.
const FADE: Duration = Duration::from_millis(150);
/// A tab's × at its brightest.
const CLOSE_SHOWN: f32 = 0.75;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

const GAP: f32 = 6.0;
/// The title bar's height at the base interface size: the traffic lights' row, which holds the
/// tabs. It grows with larger interface sizes, never shrinks below this.
pub const TITLE_BAR: f32 = 40.0;
/// The title bar with the pet shown: room for it at twice its pixel size, standing on the
/// terminal's top edge.
pub const PET_TITLE_BAR: f32 = 48.0;
/// The room between the cards, and between them and the window's right and bottom edges (and the
/// strip); their corners' radius.
const CARD_GAP: f32 = 8.0;
const CARD_RADIUS: f32 = 10.0;
/// How opaque a card's rim is, drawn in the text's colour; the active pane's among several, a
/// step brighter.
const RIM: f32 = 0.05;
const RIM_ACTIVE: f32 = 0.16;
/// The line a grip lights up under the mouse and while dragged, and the room on each side of it
/// the mouse can take it by.
const DIVIDER: f32 = 1.0;
const GRIP: f32 = 3.0;
/// What the dimmed panes are covered with: the terminal's background, this opaque.
const DIM: f32 = 0.42;
/// A split pane's header, its buttons, and the room after the last.
const PANE_HEADER: f32 = 34.0;
const PANE_BUTTON: f32 = 28.0;
const PANE_HEADER_END: f32 = 6.0;
/// The kind icon in a tab or a pane's header, and the status dot on its corner.
const BADGE_ICON: f32 = 13.0;
const BADGE_DOT: f32 = 6.0;
/// The title bar's `+` and split icon; the search field's width, and the room between it and the
/// split icon, besides the bar's own gap.
const BAR_BUTTON: f32 = 28.0;
const SEARCH: f32 = 220.0;
const SPLIT_GAP: f32 = 4.0;
/// How far under the title bar the strip draws the Attention bell, for the list to hang from it.
const RAIL_BELL: f32 = 10.0;
/// The room the title bar keeps after the sidebar's header row, or after the expand button.
const HEAD_END: f32 = 10.0;

/// Where the traffic lights go in a title bar `height` points tall: in from the left edge, and
/// centred on the row (AppKit's buttons are 14 points tall).
pub fn traffic_lights(height: f32) -> Point<Pixels> {
    point(px(14.0), px(((height - 14.0) / 2.0).max(0.0)))
}

fn title_bar_height(ui: &UiFont, pet: bool) -> f32 {
    let base = if pet { PET_TITLE_BAR } else { TITLE_BAR };
    base.max(ui.scale(base))
}

/// How wide the title bar's left part is, before the divider's line or the tabs: the sidebar's
/// width, with its header row in it; collapsed to the strip, the traffic lights (none in full
/// screen) and the expand button.
fn bar_left(collapsed: bool, full_screen: bool, sidebar_width: f32, ui: &UiFont) -> f32 {
    if collapsed {
        sidebar::head_start(full_screen) + sidebar::toggle_room(ui) + HEAD_END
    } else {
        sidebar_width
    }
}

/// Where the cards sit across the window, in points from its left edge.
#[derive(Debug, PartialEq)]
struct Across {
    /// Where the panes' card starts: right against the sidebar, which keeps its own room, or a
    /// seam after the strip.
    panes: f32,
    /// The middle of the seam the sidebar's grip takes; none beside the strip.
    left_grip: Option<f32>,
    /// The middle of the seam before the right sidebar's card, when it is open.
    right_grip: Option<f32>,
}

/// The cards in a window `window` wide, the sidebar `sidebar` wide (the strip's width when
/// `collapsed`), and the right sidebar `right` wide when it is open.
fn across(window: f32, sidebar: f32, collapsed: bool, right: Option<f32>) -> Across {
    let half = CARD_GAP / 2.0;
    Across {
        panes: if collapsed {
            sidebar + CARD_GAP
        } else {
            sidebar
        },
        left_grip: (!collapsed).then_some(sidebar - half),
        right_grip: right.map(|width| window - CARD_GAP - width - half),
    }
}

/// What a pane shows, as far as its tab and its header name it.
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    Agent(String),
    /// A shell: the program and the directory it started in.
    Shell {
        program: String,
        cwd: String,
    },
    /// A program started with `-- PROGRAM ARG…`, as typed.
    Command(String),
    Empty,
}

/// The last part of a path: `paddock` for `/Users/me/paddock`, `/` for the root.
fn last_part(path: &str) -> &str {
    match path.trim_end_matches('/').rsplit('/').next() {
        Some(part) if !part.is_empty() => part,
        _ => "/",
    }
}

/// A tab's title, for the pane it shows: an agent by its name without the group prefix, unless
/// another agent open in the window (`open`) would read the same; a shell as `zsh · paddock`.
pub fn tab_title(subject: &Subject, open: &[String]) -> String {
    match subject {
        Subject::Agent(name) => {
            let short = name.rsplit('/').next().unwrap_or(name);
            let clash = open
                .iter()
                .any(|other| other != name && other.rsplit('/').next() == Some(short));
            if clash || short.is_empty() {
                name.clone()
            } else {
                short.to_owned()
            }
        }
        Subject::Shell { program, cwd } => format!("{} · {}", last_part(program), last_part(cwd)),
        Subject::Command(command) => command.clone(),
        Subject::Empty => "empty".into(),
    }
}

/// The faint count after a tab's title for the panes it holds besides the one named.
pub fn more_panes(panes: usize) -> Option<String> {
    (panes > 1).then(|| format!("+{}", panes - 1))
}

/// A directory, short: `~` for home, and only the last part of anything deeper, as `~/…/paddock`.
pub fn short_dir(path: &str, home: Option<&str>) -> String {
    let path = path.trim_end_matches('/');
    let (root, rest) = match home.map(|home| home.trim_end_matches('/')) {
        Some(home) if !home.is_empty() && path == home => return "~".into(),
        Some(home) if !home.is_empty() && path.starts_with(&format!("{home}/")) => {
            ("~/", &path[home.len() + 1..])
        }
        _ => ("/", path.trim_start_matches('/')),
    };
    match rest.split('/').count() {
        _ if rest.is_empty() => root.into(),
        1 => format!("{root}{rest}"),
        _ => format!("{root}…/{}", last_part(rest)),
    }
}

/// A split pane's header: the name, in full, and the directory, short, when there is one.
pub fn pane_name(
    subject: &Subject,
    agent_cwd: Option<&str>,
    home: Option<&str>,
) -> (String, Option<String>) {
    match subject {
        Subject::Agent(name) => (name.clone(), agent_cwd.map(|cwd| short_dir(cwd, home))),
        Subject::Shell { program, cwd } => {
            (last_part(program).to_owned(), Some(short_dir(cwd, home)))
        }
        Subject::Command(command) => (command.clone(), None),
        Subject::Empty => ("empty".into(), None),
    }
}

/// Panes get a header only when their tab is split: alone, the terminal is all there is. A zoomed
/// pane keeps its header, for Restore.
pub fn header_shown(panes_in_tab: usize) -> bool {
    panes_in_tab > 1
}

/// The panes on screen besides the active one are dimmed; one alone is never.
pub fn dimmed(panes_shown: usize, active: bool) -> bool {
    panes_shown > 1 && !active
}

pub struct PaddockWindow {
    theme: Rc<Theme>,
    sidebar: Entity<Sidebar>,
    workspace: Workspace,
    panes: HashMap<PaneId, Entity<TerminalView>>,
    /// Font, size and corral program for new panes; its launch is replaced each time.
    template: Options,
    new_shell: NewShell,
    popup: Option<Popup>,
    /// What the split panel hangs from while it is open.
    split_from: SplitFrom,
    tabs: ScrollHandle,
    /// The pet in the tab strip's spare room, unless turned off.
    pet: Option<Entity<PetView>>,
    /// The View menu as last set: folded, sorted by name, sidebar collapsed, right sidebar open.
    menu_state: Option<(bool, bool, bool, bool)>,
    /// The pet as configured: shown, and which.
    pet_setting: (bool, crate::pet::Pet),
    /// A close or quit question is showing.
    asking: bool,
    /// The selected row of the Attention list.
    attention_index: usize,
    /// The new tab or split panel's field and selected row.
    chooser: Chooser,
    /// Where the `+`, the split icon and the panes were last drawn, for the panels that hang from
    /// them.
    spots: Rc<RefCell<HashMap<Spot, Bounds<Pixels>>>>,
    /// The tab under the mouse and since when, and the one it last left and when: their ×
    /// fades in and out.
    tab_hover: Option<(usize, Instant)>,
    tab_left: Option<(usize, Instant)>,
    /// Where the layout is saved.
    store: Store,
    /// The config came from its file at startup, rather than defaults.
    config_from_file: bool,
    /// The command palette while it is open.
    palette: Option<Palette>,
    /// The sidebar's width when expanded, which the title bar leaves empty before the tabs.
    sidebar_width: f32,
    /// The sidebar is collapsed to its strip; saved with the layout.
    collapsed: bool,
    /// The divider is being dragged: the width and the mouse's x when it was pressed.
    resizing: Option<(f32, f32)>,
    /// The right sidebar: open or not, its width, its tab; saved with the layout.
    right: RightPanel,
    /// A press on the title bar's empty part: moving now drags the window.
    dragging: bool,
    /// The title bar height the traffic lights were last centred on.
    lights: Option<f32>,
}

impl PaddockWindow {
    pub fn new(
        config: &Config,
        theme: Theme,
        options: Options,
        new_shell: NewShell,
        // Where the layout is saved, and the saved layout to open.
        (store, saved): (Store, Option<Layout>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = Rc::new(theme);
        let mono = mono_font(&options);
        let sidebar = {
            let theme = theme.clone();
            let (width, corral) = (config.sidebar_width, options.corral.clone());
            let refresh = std::time::Duration::from_millis(config.refresh_ms);
            cx.new(|cx| Sidebar::new(theme, width, mono, corral, refresh, cx))
        };
        cx.subscribe_in(&sidebar, window, Self::on_sidebar).detach();
        let collapsed = saved
            .as_ref()
            .is_some_and(|layout| layout.sidebar_collapsed);
        sidebar.update(cx, |sidebar, cx| sidebar.set_collapsed(collapsed, cx));
        let right = RightPanel::new(
            &saved
                .as_ref()
                .map(|layout| layout.right_sidebar.clone())
                .unwrap_or_default(),
        );
        let shown = match &options.launch {
            Launch::Empty => Shown::Empty,
            Launch::Agent { name } => Shown::Agent(name.clone()),
            Launch::Shell { .. } | Launch::Command { .. } => Shown::Shell,
        };
        let (workspace, restored) = match &saved {
            Some(layout) => {
                let (workspace, contents) = layout.workspace();
                (workspace, Some(contents))
            }
            None => (Workspace::new(shown).0, None),
        };
        let mut this = Self {
            theme,
            sidebar,
            workspace,
            panes: HashMap::new(),
            template: Options {
                launch: Launch::Empty,
                ..options.clone()
            },
            new_shell,
            popup: None,
            split_from: SplitFrom::Pane,
            tabs: ScrollHandle::new(),
            pet: config
                .mascot_enabled
                .then(|| cx.new(|cx| PetView::new(config.mascot, cx))),
            menu_state: None,
            pet_setting: (config.mascot_enabled, config.mascot),
            asking: false,
            attention_index: 0,
            chooser: Chooser::default(),
            spots: Rc::default(),
            tab_hover: None,
            tab_left: None,
            palette: None,
            sidebar_width: config.sidebar_width,
            collapsed,
            resizing: None,
            right,
            dragging: false,
            lights: None,
            store,
            config_from_file: crate::config::default_path().exists(),
        };
        match restored {
            Some(contents) => this.restore(contents, window, cx),
            None => {
                let first = this.workspace.active_pane();
                let view = this.view(options.launch, window, cx);
                this.panes.insert(first, view);
            }
        }
        if let Some(problem) = this.store.problem() {
            this.sidebar
                .update(cx, |sidebar, cx| sidebar.note(problem, true, cx));
        }
        this.sync(cx);
        this
    }

    /// Opens the saved panes: shells start afresh in their directories, agents attach again
    /// (saying so when they are gone or were restarted), empty panes stay empty.
    fn restore(
        &mut self,
        contents: Vec<(PaneId, Content)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for (pane, content) in contents {
            let launch = match &content {
                Content::Shell { cwd } => Launch::Shell {
                    program: self.new_shell.program.clone(),
                    cwd: cwd.clone(),
                },
                Content::Empty | Content::Agent { .. } => Launch::Empty,
            };
            let view = self.view(launch, window, cx);
            self.panes.insert(pane, view);
            if let Content::Agent {
                name,
                cwd,
                instance,
            } = content
            {
                let metadata = AgentMetadata { cwd, instance };
                self.attach(pane, &name, metadata, cx);
            }
        }
        self.focus_active(window, cx);
    }

    /// What a pane shows, in full, for the saved layout.
    fn content(&self, pane: PaneId, cx: &gpui::App) -> Content {
        match self.workspace.shown(pane) {
            Shown::Empty => Content::Empty,
            Shown::Shell => {
                let cwd = self.panes.get(&pane).and_then(|view| view.read(cx).cwd());
                let cwd = cwd
                    .filter(|cwd| std::path::Path::new(cwd).is_absolute())
                    .unwrap_or(&self.new_shell.cwd);
                Content::Shell {
                    cwd: cwd.to_owned(),
                }
            }
            Shown::Agent(name) => {
                let mut metadata = self
                    .panes
                    .get(&pane)
                    .map(|view| view.read(cx).agent_metadata())
                    .unwrap_or_default();
                let listed = self.sidebar.read(cx).metadata(name);
                metadata.cwd = metadata.cwd.or(listed.cwd);
                metadata.instance = metadata.instance.or(listed.instance);
                Content::Agent {
                    name: name.clone(),
                    cwd: metadata
                        .cwd
                        .filter(|cwd| std::path::Path::new(cwd).is_absolute()),
                    instance: metadata.instance,
                }
            }
        }
    }

    /// What Diagnostics shows from this window: the commands it runs and how its reads and saves
    /// went.
    pub fn report(&self, cx: &gpui::App) -> Report {
        Report {
            corral: self.template.corral.clone(),
            shell: self.new_shell.program.clone(),
            agents: self.sidebar.read(cx).last_read(),
            config_path: crate::config::default_path(),
            config_from_file: self.config_from_file,
            layout_path: self.store.path().map(std::path::Path::to_path_buf),
            restore: self.store.restored.clone(),
            save: self.store.saved.clone(),
            save_off: self.store.protected(),
            startup: cx.try_global::<Startup>().cloned().unwrap_or_default(),
            checked: std::time::SystemTime::now(),
        }
    }

    /// Saves the layout when it changed since the last save.
    pub fn save_layout(&mut self, cx: &gpui::App) {
        let layout = Layout {
            sidebar_collapsed: self.collapsed,
            right_sidebar: self.right.saved(),
            ..Layout::of(&self.workspace, |pane| self.content(pane, cx))
        };
        self.store.save(&layout);
    }

    /// ⌘B, and the sidebar's own buttons: collapse it to the strip or expand it again.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.collapsed = !self.collapsed;
        if self.popup == Some(Popup::Actions) {
            self.popup = None;
        }
        let collapsed = self.collapsed;
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_collapsed(collapsed, cx));
        self.save_layout(cx);
        cx.notify();
    }

    /// ⌥⌘B, the title bar's button and the right sidebar's ×: open or close it.
    fn toggle_right(&mut self, cx: &mut Context<Self>) {
        self.right.open = !self.right.open;
        self.save_layout(cx);
        cx.notify();
    }

    /// The room the right sidebar has in the window as it is now.
    fn right_room(&self, window: &Window, cx: &gpui::App) -> Room {
        let ui = UiFont::get(cx);
        Room {
            window: f32::from(window.viewport_size().width),
            // Besides the cards: the sidebar (and the seam after the strip), and the seams before
            // and after this card.
            others: across(0.0, self.sidebar_shown(&ui), self.collapsed, None).panes
                + CARD_GAP
                + CARD_GAP,
            min: ui.scale(right_panel::MIN_WIDTH).max(right_panel::MIN_WIDTH),
        }
    }

    /// Where the right sidebar is taken by to resize it: the seam before its card, centred on `x`,
    /// as the left one's.
    fn right_grip(&self, x: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let bright = self.fg(|t| t.agents_border);
        let bar = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(GRIP - 1.0))
            .w(px(DIVIDER + 2.0))
            .when(self.right.resizing(), |bar| bar.bg(bright))
            .group_hover("right-divider", move |style| style.bg(bright));
        div()
            .id("right-divider")
            .group("right-divider")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(x - GRIP - DIVIDER / 2.0))
            .w(px(GRIP + DIVIDER + GRIP))
            .cursor_col_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    let room = this.right_room(window, cx);
                    this.right.press(room, f32::from(event.position.x));
                    cx.notify();
                }),
            )
            .on_drag(RightDrag, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
            .child(bar)
    }

    /// The right sidebar's divider was let go: its width is saved with the layout.
    fn finish_right_resize(&mut self, cx: &mut Context<Self>) {
        if self.right.release() {
            self.save_layout(cx);
            cx.notify();
        }
    }

    /// How wide the sidebar is drawn now: its width, or the strip's.
    fn sidebar_shown(&self, ui: &UiFont) -> f32 {
        if self.collapsed {
            ui.scale(sidebar::RAIL)
        } else {
            self.sidebar_width
        }
    }

    /// Where the sidebar is taken by to resize it: a few points either side of a line in the seam
    /// before the panes, centred on `x`, which lights up under the mouse and while dragged.
    fn grip(&self, x: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let bright = self.fg(|t| t.agents_border);
        let bar = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(GRIP - 1.0))
            .w(px(DIVIDER + 2.0))
            .when(self.resizing.is_some(), |bar| bar.bg(bright))
            .group_hover("divider", move |style| style.bg(bright));
        div()
            .id("divider")
            .group("divider")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(x - GRIP - DIVIDER / 2.0))
            .w(px(GRIP + DIVIDER + GRIP))
            .cursor_col_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.resizing = Some((this.sidebar_width, f32::from(event.position.x)));
                    cx.notify();
                }),
            )
            .on_drag(SidebarDrag, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
            .child(bar)
    }

    /// The divider moved: the sidebar follows at once.
    fn resize_sidebar(&mut self, x: f32, cx: &mut Context<Self>) {
        let Some((width, from)) = self.resizing else {
            return;
        };
        let width = sidebar::resize(width, from, x, sidebar::min_width(&UiFont::get(cx)));
        if width != self.sidebar_width {
            self.sidebar_width = width;
            let theme = self.theme.clone();
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.restyle(theme, width, cx));
            cx.notify();
        }
    }

    /// The divider was let go: the width goes into the config file as `sidebar_width`, through
    /// the same draft Settings saves with, so the rest of the file stays as it was.
    fn finish_resize(&mut self, cx: &mut Context<Self>) {
        let Some((from, _)) = self.resizing.take() else {
            return;
        };
        cx.notify();
        if from == self.sidebar_width {
            return;
        }
        let path = crate::config::default_path();
        let width = finish_sidebar_width(&mut self.sidebar_width);
        let theme = self.theme.clone();
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.restyle(theme, self.sidebar_width, cx)
        });
        let result = (|| -> anyhow::Result<()> {
            let disk = match std::fs::read_to_string(&path) {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            let mut draft = Draft::new(disk.clone())?;
            draft.set("sidebar_width", &width);
            match draft.save(disk.as_deref())? {
                Ok(Saved::Written { text, .. }) => {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    let temporary = path.with_extension("toml.saving");
                    std::fs::write(&temporary, text)?;
                    std::fs::rename(&temporary, &path)?;
                    Ok(())
                }
                Ok(Saved::Unchanged) => Ok(()),
                Err(Conflict) => anyhow::bail!("the config file changed while it was read"),
            }
        })();
        if let Err(error) = result {
            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.note(format!("Sidebar width not saved: {error:#}"), true, cx)
            });
        }
    }

    fn view(
        &self,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        let options = Options {
            launch,
            ..self.template.clone()
        };
        let theme = self.theme.clone();
        cx.new(|cx| TerminalView::new(options, theme, window, cx))
    }

    fn shell(&self) -> Launch {
        Launch::Shell {
            program: self.new_shell.program.clone(),
            cwd: self.new_shell.cwd.clone(),
        }
    }

    /// Tells the sidebar what is shown: the active pane's agent and every agent open here.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let selected = self.workspace.active_agent().map(str::to_owned);
        let here = self.workspace.agents();
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_view(selected, here, cx));
        self.save_layout(cx);
        cx.notify();
    }

    /// Views for panes the layout made on its own (the empty pane left after closing the last tab).
    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let missing: Vec<PaneId> = self
            .workspace
            .tabs
            .iter()
            .flat_map(|tab| tab.panes())
            .filter(|pane| !self.panes.contains_key(pane))
            .collect();
        for pane in missing {
            let view = self.view(Launch::Empty, window, cx);
            self.panes.insert(pane, view);
        }
    }

    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.scroll_to_item(self.workspace.active_tab);
        if let Some(view) = self.panes.get(&self.workspace.active_pane()) {
            let focus: FocusHandle = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
        self.sync(cx);
    }

    fn focus_pane(&mut self, pane: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.active_pane() != pane {
            self.workspace.focus(pane);
            self.focus_active(window, cx);
        }
    }

    fn on_sidebar(
        &mut self,
        _: &Entity<Sidebar>,
        event: &SidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SidebarEvent::Attach { name, metadata } => {
                self.show_agent(name, metadata.clone(), window, cx)
            }
            // After this update: opening the window reads this one.
            SidebarEvent::NewAgent => cx.defer(|cx| windows::open_new_agent(Place::Current, cx)),
            SidebarEvent::Attention => self.toggle_attention(window, cx),
            SidebarEvent::Actions => {
                self.popup = match self.popup {
                    Some(Popup::Actions) => None,
                    _ => Some(Popup::Actions),
                };
                cx.notify();
            }
            SidebarEvent::ToggleCollapse => self.toggle_sidebar(cx),
            SidebarEvent::Stop(name) => self.stop_agent(Some(name.clone()), window, cx),
            SidebarEvent::Alive(names) => {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                for view in self.panes.values() {
                    view.update(cx, |v, _| v.disappeared(&names));
                }
            }
        }
    }

    /// What the New Agent window starts from: the directories to offer and the active pane's.
    pub fn seed(&self, cx: &gpui::App) -> Seed {
        let sidebar = self.sidebar.read(cx);
        let active = self.workspace.active_pane();
        let project = match self.workspace.shown(active) {
            Shown::Agent(name) => sidebar.metadata(name).cwd,
            Shown::Shell => self.panes[&active].read(cx).cwd().map(str::to_owned),
            Shown::Empty => None,
        }
        .unwrap_or_else(|| self.new_shell.cwd.clone());
        let mut projects = sidebar.projects();
        projects.push(self.new_shell.cwd.clone());
        projects.sort();
        projects.dedup();
        Seed {
            theme: self.theme.clone(),
            corral: self.template.corral.clone(),
            mono: self.template.font_family.clone().into(),
            projects,
            project,
        }
    }

    /// An agent the New Agent window started, opened where it said. A running shell in the
    /// active pane is kept: the agent gets a new tab instead.
    pub fn open_started(
        &mut self,
        started: &Started,
        cwd: &str,
        place: Place,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = started.name.clone();
        let shown = Shown::Agent(name.clone());
        let active = self.workspace.active_pane();
        let pane = match place {
            Place::Current if !self.panes[&active].read(cx).shell_live() => {
                self.workspace.set_shown(active, shown);
                active
            }
            Place::Current | Place::Tab => self.workspace.new_tab(shown),
            Place::Split(direction) => self.workspace.split(direction, shown),
        };
        if !self.panes.contains_key(&pane) {
            let view = self.view(Launch::Empty, window, cx);
            self.panes.insert(pane, view);
        }
        let metadata = AgentMetadata {
            cwd: Some(cwd.to_owned()),
            instance: started.instance.clone(),
        };
        self.attach(pane, &name, metadata, cx);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.note(format!("Started {name}"), false, cx);
            sidebar.refresh();
        });
        self.focus_active(window, cx);
    }

    /// Stops `name`, or else the active pane's agent, with `corral stop`, after asking.
    fn stop_agent(&mut self, name: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = name.or_else(|| self.workspace.active_agent().map(str::to_owned)) else {
            self.sidebar.update(cx, |sidebar, cx| {
                let text = "Stop acts on the agent in the active pane; open one first.";
                sidebar.note(text.into(), true, cx)
            });
            return;
        };
        if self.asking {
            return;
        }
        self.asking = true;
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!("Stop {name}?"),
            Some("corral stop ends the agent and its session. Panes showing it stay, saying it has gone."),
            &["Stop", "Cancel"],
            cx,
        );
        let corral = self.template.corral.clone();
        cx.spawn_in(window, async move |this, cx| {
            let stop = matches!(answer.await, Ok(0));
            let _ = this.update(cx, |this, cx| {
                this.asking = false;
                if stop {
                    this.sidebar.update(cx, |sidebar, cx| {
                        sidebar.note(format!("Stopping {name}…"), false, cx)
                    });
                }
            });
            if !stop {
                return;
            }
            let target = name.clone();
            let result = cx
                .background_spawn(async move { new_agent::stop(&corral, &target) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |sidebar, cx| {
                    match result {
                        Ok(()) => sidebar.note(format!("Stopped {name}"), false, cx),
                        Err(error) => sidebar.note(format!("{error:#}"), true, cx),
                    }
                    sidebar.refresh();
                })
            });
        })
        .detach();
    }

    /// The colours in use, for the Settings and About windows.
    pub fn theme(&self) -> Rc<Theme> {
        self.theme.clone()
    }

    /// What Settings saved that takes effect at once: colours, sidebar width, the pet, the
    /// interface font and the terminal font, which open terminals take up as if resized. The rest
    /// (refresh interval, corral command) waits for a restart.
    pub fn apply(&mut self, config: &Config, cx: &mut Context<Self>) {
        let ui = UiFont::from_config(config);
        if UiFont::get(cx) != ui {
            cx.set_global(ui);
            cx.refresh_windows();
        }
        let template = &self.template;
        if (
            &template.font_family,
            &template.fallbacks,
            template.font_size,
            template.line_height,
        ) != (
            &config.font,
            &config.font_fallbacks,
            config.font_size,
            config.line_height,
        ) {
            self.template.font_family = config.font.clone();
            self.template.fallbacks = config.font_fallbacks.clone();
            self.template.font_size = config.font_size;
            self.template.line_height = config.line_height;
            for view in self.panes.values() {
                view.update(cx, |view, cx| view.set_font(&self.template, cx));
            }
            let mono = mono_font(&self.template);
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.set_mono(mono, cx));
        }
        if let Ok(theme) = Theme::from_config(config) {
            let theme = Rc::new(theme);
            self.theme = theme.clone();
            let width = config.sidebar_width;
            self.sidebar_width = width;
            let sidebar_theme = theme.clone();
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.restyle(sidebar_theme, width, cx));
            for view in self.panes.values() {
                let theme = theme.clone();
                view.update(cx, |view, cx| view.set_theme(theme, cx));
            }
        }
        let pet = (config.mascot_enabled, config.mascot);
        if pet != self.pet_setting {
            self.pet_setting = pet;
            self.pet = pet.0.then(|| cx.new(|cx| PetView::new(pet.1, cx)));
        }
        cx.notify();
    }

    /// A shell: in the active pane when it is empty, else in a new tab.
    fn new_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.workspace.active_pane();
        if self.workspace.shown(active) == &Shown::Empty {
            let NewShell { program, cwd } = &self.new_shell;
            let (program, cwd) = (program.clone(), cwd.clone());
            self.panes[&active].update(cx, |v, cx| v.start_shell(program, cwd, cx));
            self.workspace.set_shown(active, Shown::Shell);
        } else {
            let pane = self.workspace.new_tab(Shown::Shell);
            let view = self.view(self.shell(), window, cx);
            self.panes.insert(pane, view);
        }
        self.focus_active(window, cx);
    }

    /// ⌘T and the `+`: the new tab panel, its field taking the keys; again, closed.
    fn toggle_new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::NewTab) {
            self.close_popup(window, cx);
            return;
        }
        let colors = text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        };
        let input = cx.new(|cx| TextInput::new("", "Open in a new tab…", colors, cx));
        cx.subscribe(&input, |this, input, _: &Changed, cx| {
            let query = input.read(cx).text().to_owned();
            let rows = choices(&this.sidebar.read(cx).agent_names(), &query);
            this.chooser.index = first_choice(&query, &rows);
            cx.notify();
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.palette = None;
        self.chooser = Chooser {
            input: Some(input),
            ..Chooser::default()
        };
        self.popup = Some(Popup::NewTab);
        cx.notify();
    }

    /// The split buttons, ⌘D and the other sides' shortcuts: the split panel on the side asked
    /// for (see [`split_popup`]), hanging from the button asked from or the active pane's (see
    /// [`split_from`]).
    fn ask_split(&mut self, ask: SplitAsk, window: &mut Window, cx: &mut Context<Self>) {
        let popup = split_popup(self.popup, ask);
        if popup.is_none() {
            self.close_popup(window, cx);
            return;
        }
        let splitting = matches!(self.popup, Some(Popup::Split(_)));
        let header = header_shown(self.workspace.tab().panes().len());
        self.split_from = split_from(ask, splitting.then_some(self.split_from), header);
        if !splitting {
            // Another panel's field gives the keys back.
            if self.chooser.input.is_some() || self.palette.is_some() {
                self.focus_active(window, cx);
            }
            self.palette = None;
            self.chooser = Chooser::default();
        }
        self.popup = popup;
        cx.notify();
    }

    /// What the open split panel hangs from: the title bar's icon once no pane has a header to
    /// hang it from.
    fn split_hanging(&self) -> Option<SplitFrom> {
        if !matches!(self.popup, Some(Popup::Split(_))) {
            return None;
        }
        Some(if header_shown(self.workspace.tab().panes().len()) {
            self.split_from
        } else {
            SplitFrom::Bar
        })
    }

    /// The text typed in the new tab panel's field; nothing for the split panel.
    fn chooser_query(&self, cx: &gpui::App) -> String {
        match &self.chooser.input {
            Some(input) => input.read(cx).text().to_owned(),
            None => String::new(),
        }
    }

    /// The open new tab or split panel's rows.
    fn chooser_rows(&self, cx: &gpui::App) -> Vec<Choice> {
        choices(
            &self.sidebar.read(cx).agent_names(),
            &self.chooser_query(cx),
        )
    }

    /// An empty layer over its parent that notes where the parent is drawn, for a panel to hang
    /// from; an open panel follows when it moves.
    fn spot(&self, spot: Spot) -> impl IntoElement {
        let spots = self.spots.clone();
        let open = matches!(
            self.popup,
            Some(Popup::NewTab | Popup::Split(_) | Popup::Attention)
        );
        canvas(
            move |bounds, window, _| {
                let moved = spots.borrow_mut().insert(spot, bounds) != Some(bounds);
                if moved && open {
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }

    /// The tab after or before the active one, going round.
    fn cycle_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.workspace.tabs.len();
        let index = if forward {
            (self.workspace.active_tab + 1) % count
        } else {
            (self.workspace.active_tab + count - 1) % count
        };
        self.select_tab(index, window, cx);
    }

    /// ⌘1…⌘8 go to that tab if there is one; ⌘9 to the last, as in other macOS apps.
    fn nth_tab(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.workspace.tabs.len();
        let index = if n == 9 { count - 1 } else { n - 1 };
        if index < count {
            self.select_tab(index, window, cx);
        }
    }

    /// An agent chosen in the sidebar: where it already is, else by the layout rules.
    fn show_agent(
        &mut self,
        name: &str,
        metadata: AgentMetadata,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panes = &self.panes;
        let placement = self
            .workspace
            .place(name, |pane| panes[&pane].read(cx).shell_live());
        match placement {
            Placement::Existing { pane, .. } => {
                self.workspace.focus(pane);
                // Its attach may have ended since; open it again in the same place.
                if self.panes[&pane].read(cx).target() != Some(name) {
                    self.attach(pane, name, metadata, cx);
                }
            }
            Placement::Pane(pane) => {
                self.attach(pane, name, metadata, cx);
                self.workspace
                    .set_shown(pane, Shown::Agent(name.to_owned()));
            }
            Placement::NewTab => {
                let pane = self.workspace.new_tab(Shown::Agent(name.to_owned()));
                let view = self.view(Launch::Empty, window, cx);
                self.panes.insert(pane, view);
                self.attach(pane, name, metadata, cx);
            }
        }
        self.focus_active(window, cx);
    }

    fn attach(&self, pane: PaneId, name: &str, metadata: AgentMetadata, cx: &mut Context<Self>) {
        let name = name.to_owned();
        self.panes[&pane].update(cx, |v, cx| v.attach(name, metadata, cx));
    }

    /// A choice from the new tab or split panel. An agent already open elsewhere moves here.
    fn choose(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let direction = match self.popup {
            Some(Popup::NewTab) => None,
            Some(Popup::Split(direction)) => Some(direction),
            Some(Popup::Attention | Popup::Palette | Popup::Actions) | None => return,
        };
        self.popup = None;
        self.chooser = Chooser::default();
        let (shown, launch) = match &choice {
            Choice::Shell => (Shown::Shell, self.shell()),
            Choice::Agent(name) => (Shown::Agent(name.clone()), Launch::Empty),
            // After this update: opening the window reads this one.
            Choice::NewAgent => {
                let place = direction.map_or(Place::Tab, Place::Split);
                self.focus_active(window, cx);
                cx.defer(move |cx| windows::open_new_agent(place, cx));
                return;
            }
        };
        if let Choice::Agent(name) = &choice
            && let Some(old) = self.workspace.find(name)
        {
            self.panes[&old].update(cx, |v, cx| v.close(cx));
            self.workspace.set_shown(old, Shown::Empty);
        }
        let pane = match direction {
            None => self.workspace.new_tab(shown),
            Some(direction) => self.workspace.split(direction, shown),
        };
        let view = self.view(launch, window, cx);
        self.panes.insert(pane, view);
        if let Choice::Agent(name) = &choice {
            let metadata = self.sidebar.read(cx).metadata(name);
            self.attach(pane, name, metadata, cx);
        }
        self.focus_active(window, cx);
    }

    fn close(&mut self, gone: Vec<PaneId>, window: &mut Window, cx: &mut Context<Self>) {
        for pane in gone {
            if let Some(view) = self.panes.remove(&pane) {
                view.update(cx, |v, cx| v.close(cx));
            }
        }
        self.fill(window, cx);
        self.focus_active(window, cx);
    }

    fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pane = self.workspace.active_pane();
        self.request_close(Closing::Pane(pane), window, cx);
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.workspace.tabs.get(index) {
            let pane = tab.active;
            self.request_close(Closing::Tab(pane), window, cx);
        }
    }

    /// The panes `closing` would close, as the layout is now.
    fn closing_panes(&self, closing: Closing) -> Vec<PaneId> {
        let (Closing::Pane(pane) | Closing::Tab(pane)) = closing;
        let Some(tab) = self
            .workspace
            .tabs
            .iter()
            .find(|t| t.panes().contains(&pane))
        else {
            return Vec::new();
        };
        match closing {
            Closing::Pane(_) => vec![pane],
            Closing::Tab(_) => tab.panes(),
        }
    }

    /// What to ask before closing `panes`: `None` when no live shell is among them.
    fn question(&self, panes: &[PaneId], quit: bool, cx: &Context<Self>) -> Option<Question> {
        let shells: Vec<String> = panes
            .iter()
            .filter(|pane| self.panes[pane].read(cx).shell_live())
            .map(|pane| self.title(*pane, cx))
            .collect();
        let agents = panes
            .iter()
            .filter(|pane| matches!(self.workspace.shown(**pane), Shown::Agent(_)))
            .count();
        close_question(&shells, agents, quit)
    }

    /// Closes at once, or after End Shells when live shells would end. A pane gone meanwhile is
    /// simply not closed.
    fn request_close(&mut self, closing: Closing, window: &mut Window, cx: &mut Context<Self>) {
        let panes = self.closing_panes(closing);
        let Some(question) = self.question(&panes, false, cx) else {
            self.finish_close(closing, window, cx);
            return;
        };
        if self.asking {
            return;
        }
        self.asking = true;
        let answer = question.ask(window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let end = answer.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.asking = false;
                if end {
                    this.finish_close(closing, window, cx);
                }
            });
        })
        .detach();
    }

    fn finish_close(&mut self, closing: Closing, window: &mut Window, cx: &mut Context<Self>) {
        let gone = match closing {
            Closing::Pane(pane) => self.workspace.close_pane(pane),
            Closing::Tab(pane) => {
                match self
                    .workspace
                    .tabs
                    .iter()
                    .position(|t| t.panes().contains(&pane))
                {
                    Some(index) => self.workspace.close_tab(index),
                    None => Vec::new(),
                }
            }
        };
        self.close(gone, window, cx);
    }

    /// Whether paddock may quit: yes without live shells, else as answered.
    pub fn confirm_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        let panes: Vec<PaneId> = self
            .workspace
            .tabs
            .iter()
            .flat_map(|tab| tab.panes())
            .collect();
        let Some(question) = self.question(&panes, true, cx) else {
            return Task::ready(true);
        };
        if self.asking {
            return Task::ready(false);
        }
        self.asking = true;
        let answer = question.ask(window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let quit = answer.await;
            let _ = this.update(cx, |this, _| this.asking = false);
            quit
        })
    }

    fn toggle_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.toggle_zoom();
        self.focus_active(window, cx);
    }

    /// The mouse came onto a tab or left it: its × fades in or out from where it is.
    fn hover_tab(&mut self, index: usize, hovered: bool, cx: &mut Context<Self>) {
        let now = Instant::now();
        // Started as long ago as the fade would have taken to get this far.
        let from = |this: &Self, tab: usize, rising: bool| {
            let part = this.close_shown(tab, now) / CLOSE_SHOWN;
            let part = if rising { part } else { 1.0 - part };
            now.checked_sub(FADE.mul_f32(part)).unwrap_or(now)
        };
        if hovered {
            if let Some((other, _)) = self.tab_hover
                && other != index
            {
                self.tab_left = Some((other, from(self, other, false)));
            }
            self.tab_hover = Some((index, from(self, index, true)));
        } else if self.tab_hover.is_some_and(|(tab, _)| tab == index) {
            self.tab_left = Some((index, from(self, index, false)));
            self.tab_hover = None;
        }
        cx.notify();
    }

    /// How far a tab's × has faded in at `now`.
    fn close_shown(&self, index: usize, now: Instant) -> f32 {
        let part = |since: Instant| {
            (now.saturating_duration_since(since).as_secs_f32() / FADE.as_secs_f32()).min(1.0)
        };
        match (self.tab_hover, self.tab_left) {
            (Some((tab, since)), _) if tab == index => CLOSE_SHOWN * part(since),
            (_, Some((tab, since))) if tab == index => CLOSE_SHOWN * (1.0 - part(since)),
            _ => 0.0,
        }
    }

    /// A tab's × is still fading: draw again next frame.
    fn fading(&self) -> bool {
        let now = Instant::now();
        [self.tab_hover, self.tab_left]
            .into_iter()
            .flatten()
            .any(|(_, since)| now.saturating_duration_since(since) < FADE)
    }

    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.select_tab(index);
        self.focus_active(window, cx);
    }

    /// `Role · name` for an agent (`Agent` without a role label), `shell · <cwd>` for a shell,
    /// `Viewer` for an empty pane, as Saddle titles its panes.
    fn title(&self, pane: PaneId, cx: &Context<Self>) -> String {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => {
                let role = match self.sidebar.read(cx).role(name) {
                    Some(Role::Controller) => "Controller",
                    Some(Role::Regular) => "Regular",
                    Some(Role::Implementer) => "Implementer",
                    Some(Role::Reviewer) => "Reviewer",
                    None => "Agent",
                };
                format!("{role} · {name}")
            }
            Shown::Shell => self.panes[&pane].read(cx).subject().to_owned(),
            Shown::Empty => "Viewer".into(),
        }
    }

    /// What a pane shows, for its tab and its header.
    fn subject(&self, pane: PaneId, cx: &Context<Self>) -> Subject {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => Subject::Agent(name.clone()),
            Shown::Shell => {
                let view = self.panes[&pane].read(cx);
                match view.cwd() {
                    Some(cwd) => Subject::Shell {
                        program: self.new_shell.program.clone(),
                        cwd: cwd.to_owned(),
                    },
                    None => Subject::Command(view.subject().to_owned()),
                }
            }
            Shown::Empty => Subject::Empty,
        }
    }

    /// A pane's status dot: an agent's status colour as the sidebar shows it, a neutral one for
    /// a shell, a faint one for an empty pane or an agent no longer listed.
    fn dot(&self, pane: PaneId, agents: &[Agent], now: f64) -> Hsla {
        self.fg(dot(self.workspace.shown(pane), agents, now))
    }

    /// A pane's mark in its tab and its header: the icon of an agent's kind, or a terminal for a
    /// shell, with the status dot on its lower right corner, ringed in `ground` (`hovered` while
    /// the group is hovered) to stand off the icon; the dot alone where there is no icon.
    fn badge(
        &self,
        pane: PaneId,
        agents: &[Agent],
        now: f64,
        ground: Hsla,
        hovered: Option<(SharedString, Hsla)>,
        ui: &UiFont,
    ) -> AnyElement {
        let dot = self.dot(pane, agents, now);
        let icon = match self.workspace.shown(pane) {
            Shown::Agent(name) => agents
                .iter()
                .find(|a| &a.name == name)
                .and_then(|a| a.kind.as_deref())
                .and_then(|kind| Some((kind_icon::of(kind)?, card::brand(kind).color)))
                .map(|(icon, color)| {
                    icon.render(ui.px(BADGE_ICON), self.fg(color).opacity(0.85))
                        .into_any_element()
                }),
            Shown::Shell => Some(
                footer_icon::icon(
                    Icon::NewShell,
                    self.fg(|t| t.muted),
                    ui.scale(BADGE_ICON / footer_icon::SIZE),
                )
                .into_any_element(),
            ),
            Shown::Empty => None,
        };
        let Some(icon) = icon else {
            return div()
                .flex_shrink_0()
                .size(ui.px(7.0))
                .rounded_full()
                .bg(dot)
                .into_any_element();
        };
        let ring = 2.0;
        let mark = div()
            .absolute()
            .right(px(-(ui.scale(2.0) + ring)))
            .bottom(px(-(ui.scale(1.0) + ring)))
            .size(px(ui.scale(BADGE_DOT) + 2.0 * ring))
            .rounded_full()
            .border_2()
            .border_color(ground)
            .bg(dot);
        let mark = match hovered {
            Some((group, color)) => mark.group_hover(group, move |style| style.border_color(color)),
            None => mark,
        };
        div()
            .relative()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(BADGE_ICON + 2.0))
            .child(icon)
            .child(mark)
            .into_any_element()
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    /// A card on the frame: the terminal's ground, rounded, with a faint rim, a step brighter for
    /// the active pane among several.
    fn card(&self, bright: bool) -> Div {
        div()
            .rounded(px(CARD_RADIUS))
            .bg(hsla(self.theme.terminal().background, 1.0))
            .border_1()
            .border_color(
                self.fg(|t| t.agents_text)
                    .opacity(if bright { RIM_ACTIVE } else { RIM }),
            )
    }

    fn highlight(&self) -> Hsla {
        hsla(self.theme.bg(|t| t.agent_selected), 1.0)
    }

    /// The title bar: over the sidebar the traffic lights (none in full screen) and the sidebar's
    /// header row, then the tabs from the terminal's left edge, the `+` after the last, and the
    /// pet in the room left. What is not a tab or a button drags the window, and a double click
    /// there zooms or minimises it as the system is set.
    fn title_bar(
        &self,
        agents: &[Agent],
        now: f64,
        full_screen: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let scale = ui.scale(1.0);
        let highlight = self.highlight();
        // A hovered tab's faint ground, solid, so the × drawn over its title can hide the text.
        let ground = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let hovered = ground.blend(highlight.opacity(0.6));
        let muted = self.theme.fg(|t| t.muted);
        let open = self.workspace.agents();
        let shown_at = Instant::now();
        // A press on a tab or a button is theirs, not the start of a drag.
        let keep = |_: &MouseDownEvent, _: &mut Window, cx: &mut gpui::App| cx.stop_propagation();
        let mut tabs = div()
            .id("tabs")
            .flex()
            .items_center()
            .gap(px(4.0))
            .min_w(px(0.0))
            .flex_shrink(1.0)
            .h_full()
            .overflow_x_scroll()
            .track_scroll(&self.tabs);
        for (index, tab) in self.workspace.tabs.iter().enumerate() {
            let active = index == self.workspace.active_tab;
            // The active tab keeps its × in line; the others have it over the end of their
            // title, so a narrow tab gives the title all its room. Either fades in only while
            // the mouse is on the tab.
            let under = if active { highlight } else { hovered };
            let close = div()
                .id(("close-tab", index))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(16.0))
                .rounded(px(4.0))
                .bg(under)
                .opacity(self.close_shown(index, shown_at))
                .hover(move |style| style.bg(under.blend(hsla(muted, 0.18))))
                .when(!active, |close| {
                    close.absolute().top(ui.px(6.0)).right(ui.px(6.0))
                })
                .child(footer_icon::icon(
                    Icon::Close,
                    self.fg(|t| t.muted),
                    scale * 0.85,
                ))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    this.close_tab(index, window, cx);
                }));
            let mut item = div()
                .id(("tab", index))
                .group(SharedString::from(format!("tab-{index}")))
                .relative()
                .max_w(ui.px(220.0))
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .h(ui.px(28.0))
                .pl(ui.px(9.0))
                .rounded(px(7.0))
                .text_size(ui.px(12.5))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, keep)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    this.hover_tab(index, *hovered, cx)
                }))
                .child(if active {
                    self.badge(tab.active, agents, now, highlight, None, &ui)
                } else {
                    let group = SharedString::from(format!("tab-{index}"));
                    self.badge(tab.active, agents, now, ground, Some((group, hovered)), &ui)
                })
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(tab_title(&self.subject(tab.active, cx), &open)),
                )
                .children(more_panes(tab.panes().len()).map(|more| {
                    div()
                        .flex_shrink_0()
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(more)
                }));
            // Short of room, the other tabs narrow first; the active one keeps its title.
            item = if active {
                item.flex_shrink_0()
                    .pr(ui.px(6.0))
                    .bg(highlight)
                    .text_color(self.fg(|t| t.agents_text))
            } else {
                item.flex_shrink(1.0)
                    .min_w(ui.px(56.0))
                    .pr(ui.px(11.0))
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(hovered))
            };
            tabs = tabs.child(item.child(close).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| this.select_tab(index, window, cx),
            )));
        }
        // Lit while its panel is open, which hangs from it.
        let choosing = self.popup == Some(Popup::NewTab);
        let new_tab = div()
            .id("new-tab")
            .relative()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(BAR_BUTTON))
            .rounded(px(6.0))
            .cursor_pointer()
            .when(choosing, |button| button.bg(highlight))
            .hover(move |style| style.bg(highlight))
            .on_mouse_down(MouseButton::Left, keep)
            .child(footer_icon::icon(
                Icon::Plus,
                if choosing {
                    self.fg(|t| t.agents_text)
                } else {
                    self.fg(|t| t.muted)
                },
                scale,
            ))
            .child(self.spot(Spot::NewTab))
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_new_tab(window, cx)),
            );
        // Over the sidebar: the traffic lights, then the sidebar's header row (the expand button
        // alone over the strip), with room to drag by.
        let compact = sidebar::compact_bell(self.sidebar_width, full_screen, &ui);
        let spot = self.spot(Spot::Bell).into_any_element();
        let head = self
            .sidebar
            .update(cx, |sidebar, cx| sidebar.head(compact, spot, cx));
        let left = div()
            .flex_shrink_0()
            .w(px(bar_left(
                self.collapsed,
                full_screen,
                self.sidebar_width,
                &ui,
            )))
            .h_full()
            .flex()
            .items_center()
            .overflow_hidden()
            .pl(px(sidebar::head_start(full_screen)))
            .when(!self.collapsed, |left| left.pr(px(HEAD_END)))
            .child(head);
        div()
            .id("title-bar")
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(px(title_bar_height(&ui, self.pet.is_some())))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, _| {
                    this.dragging = event.click_count < 2;
                    if event.click_count == 2 {
                        window.titlebar_double_click();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, _| {
                if this.dragging && event.pressed_button == Some(MouseButton::Left) {
                    this.dragging = false;
                    window.start_window_move();
                }
            }))
            .child(left)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .pl(px(10.0))
                    .pr(px(10.0))
                    .child(tabs)
                    .child(new_tab)
                    // Spare room, where the pet walks along the terminal's top edge.
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .ml(px(GAP))
                            .children(self.pet.clone()),
                    )
                    .child(self.split_button(cx))
                    .child(self.search_button(cx))
                    .child(self.right_button(cx)),
            )
    }

    /// The right sidebar's switch, after the search field at the title bar's right end; lit while
    /// the sidebar is open.
    fn right_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        let lit = self.right.open;
        let tip = BarTip {
            text: "Toggle right sidebar",
            keys: menu::keys(&menu::ToggleRightSidebar).concat(),
            size: ui.px(11.5),
            color: self.fg(|t| t.agents_text),
            dim: self.fg(|t| t.agents_dim),
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: self.fg(|t| t.agents_rule),
        };
        div()
            .id("right-sidebar")
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(BAR_BUTTON))
            .ml(ui.px(SPLIT_GAP))
            .rounded(px(6.0))
            .cursor_pointer()
            .when(lit, |button| button.bg(highlight))
            .hover(move |style| style.bg(highlight))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(footer_icon::icon(
                Icon::RightSidebar,
                if lit {
                    self.fg(|t| t.agents_text)
                } else {
                    self.fg(|t| t.muted)
                },
                ui.scale(1.0),
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_right(cx)))
    }

    /// The way into the split panel for the active pane, always before the search field, as a
    /// lone pane has no header to carry its split button. Quiet until hovered; lit while the
    /// panel hangs from it.
    fn split_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        let lit = self.split_hanging() == Some(SplitFrom::Bar);
        let tip = BarTip {
            text: "Split pane",
            keys: menu::keys(&menu::SplitRight).concat(),
            size: ui.px(11.5),
            color: self.fg(|t| t.agents_text),
            dim: self.fg(|t| t.agents_dim),
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: self.fg(|t| t.agents_rule),
        };
        div()
            .id("split-pane")
            .relative()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(BAR_BUTTON))
            .mr(ui.px(SPLIT_GAP))
            .rounded(px(6.0))
            .cursor_pointer()
            .when(lit, |button| button.bg(highlight))
            .hover(move |style| style.bg(highlight))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(footer_icon::icon(
                Icon::Split,
                if lit {
                    self.fg(|t| t.agents_text)
                } else {
                    self.fg(|t| t.muted)
                },
                ui.scale(1.0),
            ))
            .child(self.spot(Spot::Split))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.ask_split(SplitAsk::Bar, window, cx)
            }))
    }

    /// The way into the command palette, always at the title bar's right end: the tabs narrow
    /// before it does.
    fn search_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        div()
            .id("search")
            .flex_shrink_0()
            .w(ui.px(SEARCH))
            .h(ui.px(28.0))
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .pl(ui.px(10.0))
            .pr(ui.px(6.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(hsla(self.theme.fg(|t| t.agents_rule), 0.6))
            .bg(highlight.opacity(0.5))
            .hover(move |style| style.bg(highlight))
            .text_size(ui.px(12.5))
            .text_color(self.fg(|t| t.agents_dim))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(footer_icon::icon(
                Icon::Search,
                self.fg(|t| t.agents_dim),
                ui.scale(13.0 / footer_icon::SIZE),
            ))
            .child(div().flex_1().child("Search"))
            .child(self.keycaps(&menu::keys(&menu::Search), &ui, 18.0))
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_palette("", window, cx)),
            )
    }

    /// A shortcut's keys, each on a small cap `height` points tall.
    fn keycaps(&self, keys: &[String], ui: &UiFont, height: f32) -> Div {
        let (pad, radius) = if height < 20.0 {
            (4.0, 4.0)
        } else {
            (5.0, 5.0)
        };
        div()
            .flex_shrink_0()
            .flex()
            .gap(px(2.0))
            .children(keys.iter().map(|key| {
                div()
                    .min_w(ui.px(height - 2.0))
                    .h(ui.px(height))
                    .px(ui.px(pad))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(radius))
                    .bg(self.highlight())
                    .text_size(ui.px(11.5))
                    .text_color(self.fg(|t| t.muted))
                    .child(key.clone())
            }))
    }

    /// `shown` is how many panes are on screen, for dimming all but the active one.
    fn node(
        &self,
        node: &Node,
        shown: usize,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Node::Pane(pane) => self.pane(*pane, shown, agents, cx),
            Node::Split {
                axis,
                first,
                second,
            } => {
                // Each pane its own card, a seam of the frame between.
                let split = div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .gap(px(CARD_GAP));
                let split = match axis {
                    Axis::Row => split.flex_row(),
                    Axis::Column => split.flex_col(),
                };
                split
                    .child(self.node(first, shown, agents, cx))
                    .child(self.node(second, shown, agents, cx))
                    .into_any_element()
            }
        }
    }

    fn pane(
        &self,
        pane: PaneId,
        shown: usize,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active = pane == self.workspace.active_pane();
        let header = header_shown(self.workspace.tab().panes().len());
        let background = hsla(self.theme.terminal().background, 1.0);
        let dim = dimmed(shown, active);
        // Around the terminal, keeping it well off the card's corners (the view adds 6 itself):
        // roomier alone, tighter under a header, whose own height leaves room above.
        let (top, bottom, side) = if header {
            (0.0, 6.0, 10.0)
        } else {
            (8.0, 8.0, 12.0)
        };
        self.card(shown > 1 && active)
            .id(("pane", pane as usize))
            .group(SharedString::from(format!("pane-{pane}")))
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .overflow_hidden()
            .capture_any_mouse_down(
                cx.listener(move |this, _, window, cx| this.focus_pane(pane, window, cx)),
            )
            .on_drop::<ExternalPaths>(cx.listener(move |this, paths, window, cx| {
                this.focus_pane(pane, window, cx);
                this.panes[&pane].update(cx, |view, cx| view.drop_files(paths, window, cx));
            }))
            .when(header, |this| {
                this.child(self.pane_header(pane, active, agents, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .pt(px(top))
                    .pb(px(bottom))
                    .px(px(side))
                    .child(self.panes[&pane].clone()),
            )
            .child(self.spot(Spot::Pane(pane)))
            // A veil rather than a frame, rounded as the card: it takes no clicks, so they reach the
            // terminal.
            .when(dim, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(CARD_RADIUS))
                        .bg(background.opacity(DIM)),
                )
            })
            .into_any_element()
    }

    /// A split pane's header, at the top of its card: kind icon with the status dot, name, short
    /// directory, and icons to split, zoom and close, always there on the active pane and on hover
    /// on the others.
    fn pane_header(
        &self,
        pane: PaneId,
        active: bool,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> Div {
        let ui = UiFont::get(cx);
        let scale = ui.scale(1.0);
        let highlight = self.highlight();
        let agent_cwd = match self.workspace.shown(pane) {
            Shown::Agent(name) => agents
                .iter()
                .find(|a| &a.name == name)
                .and_then(|a| a.cwd.clone()),
            _ => None,
        };
        let home = std::env::var("HOME").ok();
        let (name, dir) = pane_name(
            &self.subject(pane, cx),
            agent_cwd.as_deref(),
            home.as_deref(),
        );
        let now = now();
        let splitting = active && self.split_hanging() == Some(SplitFrom::Pane);
        let button = |id: &str, icon: Icon, lit: bool| {
            let group = SharedString::from(format!("{id}-{pane}"));
            div()
                .id(SharedString::from(format!("{id}-{pane}")))
                .group(group.clone())
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(28.0))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(ui.px(22.0))
                        .rounded(px(6.0))
                        .when(lit, |button| button.bg(highlight))
                        .group_hover(group, move |style| style.bg(highlight))
                        .child(footer_icon::icon(
                            icon,
                            if lit {
                                self.fg(|t| t.agents_text)
                            } else {
                                self.fg(|t| t.muted)
                            },
                            scale,
                        )),
                )
        };
        let zoomed = self.workspace.zoomed().is_some();
        let buttons = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .when(!active, |buttons| {
                buttons
                    .invisible()
                    .group_hover(format!("pane-{pane}"), |style| style.visible())
            })
            // Lit while its panel is open, which hangs from it.
            .child(
                button("split", Icon::Split, splitting).on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| this.ask_split(SplitAsk::Button, window, cx),
                )),
            )
            .child(
                button(
                    "zoom",
                    if zoomed { Icon::Restore } else { Icon::Zoom },
                    false,
                )
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_zoom(window, cx)),
                ),
            )
            .child(
                button("close-pane", Icon::Close, false).on_click(cx.listener(
                    move |this, _: &ClickEvent, window, cx| {
                        this.request_close(Closing::Pane(pane), window, cx)
                    },
                )),
            );
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(PANE_HEADER))
            .pl(ui.px(14.0))
            .pr(ui.px(PANE_HEADER_END))
            .text_size(ui.px(12.0))
            .child(self.badge(
                pane,
                agents,
                now,
                hsla(self.theme.terminal().background, 1.0),
                None,
                &ui,
            ))
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if active {
                        self.fg(|t| t.agents_text)
                    } else {
                        self.fg(|t| t.muted)
                    })
                    .child(name),
            )
            .children(dir.map(|dir| {
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(format!("· {dir}"))
            }))
            .child(div().flex_1())
            .child(buttons)
    }

    fn toggle_attention(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Attention) {
            self.popup = None;
        } else {
            // The new tab panel's field gives the keys back.
            if self.chooser.input.is_some() {
                self.chooser = Chooser::default();
                self.focus_active(window, cx);
            }
            self.popup = Some(Popup::Attention);
            self.attention_index = 0;
        }
        cx.notify();
    }

    /// Opens an Attention item's agent, where it is or by the layout rules; a failed read has
    /// nothing to open.
    fn open_attention(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup != Some(Popup::Attention) {
            return;
        }
        let items = self.sidebar.read(cx).attention();
        let Some(name) = items.get(index).and_then(|item| item.agent.clone()) else {
            return;
        };
        self.popup = None;
        let metadata = self.sidebar.read(cx).metadata(&name);
        self.show_agent(&name, metadata, window, cx);
    }

    /// The Attention list, hanging from the bell: `Needs you` and how many, then each item with
    /// its mark, name, project and age, why, and the start of its text.
    fn attention_panel(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let items = self.sidebar.read(cx).attention();
        let agents = self.sidebar.read(cx).agents();
        let now = now();
        let selected = self.attention_index.min(items.len().saturating_sub(1));
        let placed = self.placed(Popup::Attention, 360.0, window, cx);
        let lit = popover::lit(theme);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let header = div()
            .flex_shrink_0()
            .flex()
            .items_baseline()
            .gap(ui.px(8.0))
            .px(ui.px(9.0))
            .pt(ui.px(7.0))
            .pb(ui.px(8.0))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Needs you"))
            .when(!items.is_empty(), |header| {
                header.child(
                    div()
                        .text_size(ui.px(12.0))
                        .text_color(dimmer)
                        .child(items.len().to_string()),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_size(ui.px(11.5))
                    .text_color(dimmer.opacity(0.8))
                    .child(menu::keys(&menu::ShowAttention).concat()),
            );
        let mut list = div()
            .id("attention-rows")
            .flex()
            .flex_col()
            .min_h(px(0.0))
            .overflow_y_scroll();
        if items.is_empty() {
            list = list.child(
                div()
                    .px(ui.px(9.0))
                    .pb(ui.px(8.0))
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .child("Nothing needs attention."),
            );
        }
        for (index, item) in items.iter().enumerate() {
            let mark = match item.kind {
                AttentionKind::Waiting => self.fg(|t| t.agents_yellow),
                AttentionKind::Error | AttentionKind::ReadFailed => self.fg(|t| t.agents_red),
                AttentionKind::Reply => self.fg(|t| t.agents_accent),
            };
            let short = item
                .label
                .rsplit('/')
                .next()
                .unwrap_or(&item.label)
                .to_owned();
            let project = crate::agents::group(&item.label)
                .trim_end_matches('/')
                .to_owned();
            // Since the agent came to this state, when corral says.
            let age = item
                .agent
                .as_ref()
                .and_then(|name| agents.iter().find(|a| &a.name == name))
                .and_then(|agent| agent.state_started)
                .map(|since| card::short_time(Some(now - since)));
            let first = div()
                .flex()
                .items_baseline()
                .gap(ui.px(6.0))
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(short),
                )
                .when(!project.is_empty(), |line| {
                    line.child(
                        div()
                            .flex_shrink_0()
                            .text_size(ui.px(12.0))
                            .text_color(dimmer)
                            .child(project),
                    )
                })
                .child(div().flex_1())
                .children(age.map(|age| {
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(11.5))
                        .text_color(dimmer)
                        .child(age)
                }));
            let mut row = div()
                .id(("attention-item", index))
                .flex_shrink_0()
                .flex()
                .items_start()
                .gap(ui.px(11.0))
                .p(ui.px(9.0))
                .rounded(ui.px(7.0))
                .child(
                    div()
                        .flex_shrink_0()
                        .mt(ui.px(1.0))
                        .size(ui.px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(mark.opacity(0.16))
                        .text_color(mark)
                        .text_size(ui.px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .child(item.mark()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(2.0))
                        .child(first)
                        .child(
                            div()
                                .text_size(ui.px(12.0))
                                .text_color(mark)
                                .child(item.reason()),
                        )
                        .when(!item.note.is_empty(), |text| {
                            text.child(
                                div()
                                    .min_w(px(0.0))
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .text_size(ui.px(12.0))
                                    .text_color(dim)
                                    .child(item.note.clone()),
                            )
                        }),
                );
            if index == selected {
                row = row.bg(lit);
            }
            if item.agent.is_some() {
                row = row
                    .cursor_pointer()
                    .hover(move |style| style.bg(lit.opacity(0.6)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open_attention(index, window, cx)
                    }));
            }
            list = list.child(row);
        }
        let panel = popover::panel(theme, &ui)
            .id("attention-list")
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(header)
            .child(list);
        // A click outside closes it; nothing is dimmed, as for a menu.
        div()
            .id("attention-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(panel)
    }

    /// The sidebar's menu of actions, opening upward from its button: new agent and shell, the
    /// list's order and folding, stopping the active pane's agent, and Settings. Choosing closes
    /// it, as do Esc and a click outside.
    fn actions_menu(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let (fold, by_name) = self.sidebar.read(cx).view_state();
        let (stop, can_stop) = sidebar::stop_item(self.workspace.active_agent());
        let keys = |action: &dyn gpui::Action| popover::keys(theme, &ui, &menu::keys(action));
        let sort = |this: &mut Self, by_name: bool, window: &mut Window, cx: &mut Context<Self>| {
            this.close_popup(window, cx);
            this.sidebar.update(cx, |s, cx| s.set_sort(by_name, cx));
        };
        let mut stop = popover::row(
            theme,
            &ui,
            "menu-stop",
            Icon::Stop,
            stop,
            if can_stop { Tone::Danger } else { Tone::Off },
        );
        if can_stop {
            stop = stop.on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.close_popup(window, cx);
                this.stop_agent(None, window, cx);
            }));
        }
        let (left, bottom) = sidebar::menu_anchor(self.collapsed, &ui);
        let panel = popover::panel(theme, &ui)
            .id("actions-menu")
            .absolute()
            .left(left)
            .bottom(bottom)
            .w(ui.px(252.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                popover::row(
                    theme,
                    &ui,
                    "menu-new-agent",
                    Icon::NewAgent,
                    "New Agent…",
                    Tone::Plain,
                )
                .child(keys(&menu::NewAgent))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.close_popup(window, cx);
                    cx.defer(|cx| windows::open_new_agent(Place::Current, cx));
                })),
            )
            .child(
                popover::row(
                    theme,
                    &ui,
                    "menu-new-shell",
                    Icon::NewShell,
                    "New Shell",
                    Tone::Plain,
                )
                .child(keys(&menu::NewShell))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.close_popup(window, cx);
                    this.new_shell(window, cx);
                })),
            )
            .child(popover::rule(theme, &ui))
            .child(
                popover::line(theme, &ui, Icon::Sort, "Sort", false).child(
                    popover::choices(theme, &ui)
                        .child(
                            popover::choice(theme, &ui, "sort-status", "Status", !by_name)
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    sort(this, false, window, cx)
                                })),
                        )
                        .child(
                            popover::choice(theme, &ui, "sort-name", "Name", by_name).on_click(
                                cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    sort(this, true, window, cx)
                                }),
                            ),
                        ),
                ),
            )
            .child(
                popover::row(
                    theme,
                    &ui,
                    "menu-fold",
                    Icon::Fold,
                    "Fold Agents",
                    Tone::Plain,
                )
                .child(popover::tick(theme, &ui, fold))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.close_popup(window, cx);
                    this.sidebar.update(cx, |s, cx| s.toggle_fold(cx));
                })),
            )
            .child(popover::rule(theme, &ui))
            .child(stop)
            .child(popover::rule(theme, &ui))
            .child(
                popover::row(
                    theme,
                    &ui,
                    "menu-settings",
                    Icon::Gear,
                    "Settings…",
                    Tone::Plain,
                )
                .child(keys(&menu::OpenSettings))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.close_popup(window, cx);
                    cx.defer(windows::open_settings);
                })),
            );
        // A click outside closes it; nothing is dimmed, as for a menu.
        div()
            .id("actions-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(panel)
    }

    /// ⌘P (`query` empty) or ⌘⇧P (`>`): the command palette over the window, its field focused.
    /// The same shortcut again closes it; the other one switches to its mode.
    fn toggle_palette(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Palette)
            && let Some(palette) = &self.palette
        {
            let commands = search::mode(palette.input.read(cx).text()).0 == Mode::Commands;
            if commands == query.starts_with('>') {
                self.close_popup(window, cx);
            } else {
                self.set_palette_query(query, window, cx);
            }
            return;
        }
        let colors = text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        };
        let placeholder = "Search agents, tabs, settings and commands";
        let input = cx.new(|cx| TextInput::new(query.to_owned(), placeholder, colors, cx));
        cx.subscribe(&input, |this, _, _: &Changed, cx| this.palette_typed(cx))
            .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.palette = Some(Palette {
            input,
            index: 0,
            list: ScrollHandle::new(),
            commands: search::commands(&menu::commands()),
            text: None,
            searching: None,
        });
        self.chooser = Chooser::default();
        self.popup = Some(Popup::Palette);
        cx.notify();
    }

    /// `> Commands` and `/ Search text` under the list, or the other shortcut: the field starts
    /// again from `query`.
    fn set_palette_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(palette) = &self.palette else {
            return;
        };
        let input = palette.input.clone();
        input.update(cx, |input, cx| input.set_text(query.to_owned(), cx));
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.palette_typed(cx);
    }

    /// The query changed: the first row selected again, and after `/` a new search of the open
    /// panes once typing pauses, off the UI thread.
    fn palette_typed(&mut self, cx: &mut Context<Self>) {
        let screens: Vec<_> = self
            .text_panes()
            .into_iter()
            .filter_map(|pane| Some((pane, self.panes.get(&pane)?.read(cx).screen()?)))
            .collect();
        let Some(palette) = &mut self.palette else {
            return;
        };
        palette.index = 0;
        palette.list.set_offset(point(px(0.0), px(0.0)));
        let query = palette.input.read(cx).text().to_owned();
        let (mode, needle) = search::mode(&query);
        if mode != Mode::Text || needle.is_empty() {
            palette.text = None;
            palette.searching = None;
            cx.notify();
            return;
        }
        let needle = needle.to_owned();
        palette.searching = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE).await;
            let looked = needle.clone();
            let panes = cx
                .background_spawn(async move {
                    screens
                        .into_iter()
                        .map(|(pane, screen)| {
                            let screen = screen.lock().unwrap();
                            let (hits, more) = find::hits(&screen.term, &looked, search::PER_PANE);
                            (pane, hits, more)
                        })
                        .filter(|(_, hits, _)| !hits.is_empty())
                        .collect()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Some(palette) = &mut this.palette {
                    palette.text = Some(TextResults { needle, panes });
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    /// The panes a text search looks through: every one in the window, the active one first.
    fn text_panes(&self) -> Vec<PaneId> {
        let active = self.workspace.active_pane();
        let mut panes = vec![active];
        panes.extend(
            self.workspace
                .tabs
                .iter()
                .flat_map(|tab| tab.panes())
                .filter(|pane| *pane != active),
        );
        panes
    }

    /// A pane as the text search's group names it: an agent in full, anything else as its tab.
    fn pane_label(&self, pane: PaneId, cx: &Context<Self>) -> String {
        match self.subject(pane, cx) {
            Subject::Agent(name) => name,
            subject => tab_title(&subject, &[]),
        }
    }

    /// The tabs, as the palette lists them: dot, title and count, number and agents, shortcut.
    fn tab_rows(&self, agents: &[Agent], now: f64, cx: &Context<Self>) -> Vec<search::Row> {
        let open = self.workspace.agents();
        let count = self.workspace.tabs.len();
        let tabs = self.workspace.tabs.iter().enumerate();
        tabs.map(|(index, tab)| {
            let panes = tab.panes();
            let mut title = tab_title(&self.subject(tab.active, cx), &open);
            if let Some(more) = more_panes(panes.len()) {
                title = format!("{title} {more}");
            }
            let names: Vec<String> = panes
                .iter()
                .filter_map(|pane| match self.workspace.shown(*pane) {
                    Shown::Agent(name) => Some(name.clone()),
                    _ => None,
                })
                .collect();
            let keys = search::tab_shortcut(index, count)
                .and_then(menu::tab)
                .map(|action| menu::keys(action.as_ref()))
                .unwrap_or_default();
            search::Row {
                target: Target::Tab(index),
                lead: Lead::Dot {
                    color: dot(self.workspace.shown(tab.active), agents, now),
                    breathing: false,
                },
                title,
                detail: search::tab_detail(index, &names),
                keys,
                hit: None,
            }
        })
        .collect()
    }

    /// The palette's groups for what is typed, its mode, and what the list says when it is empty.
    fn palette_groups(
        &self,
        palette: &Palette,
        cx: &Context<Self>,
    ) -> (Mode, Vec<search::Group>, Option<String>) {
        let query = palette.input.read(cx).text().to_owned();
        let (mode, needle) = search::mode(&query);
        match mode {
            Mode::Everything => {
                let agents = self.sidebar.read(cx).agents();
                let now = now();
                let home = std::env::var("HOME").ok();
                let groups = search::everything(
                    &search::agents(&agents, now, home.as_deref()),
                    &self.tab_rows(&agents, now, cx),
                    &search::settings(),
                    &palette.commands,
                    needle,
                );
                let empty = search::empty(&groups, needle);
                (mode, groups, empty)
            }
            Mode::Commands => {
                let groups = search::only_commands(&palette.commands, needle);
                let empty = search::empty(&groups, needle);
                (mode, groups, empty)
            }
            // The last search's results until the next one is done, saying what they are for.
            Mode::Text => {
                let Some(text) = &palette.text else {
                    return (mode, Vec::new(), None);
                };
                let groups: Vec<_> = text
                    .panes
                    .iter()
                    .filter(|(pane, ..)| self.panes.contains_key(pane))
                    .filter_map(|(pane, hits, more)| {
                        let rows = hits.iter().enumerate();
                        let rows = rows
                            .map(|(index, hit)| {
                                search::line(*pane, index, &hit.line, hit.range.clone())
                            })
                            .collect();
                        search::pane(&self.pane_label(*pane, cx), rows, *more)
                    })
                    .collect();
                let empty = search::empty(&groups, &text.needle);
                (mode, groups, empty)
            }
        }
    }

    /// Opens a palette row: the agent where it is or by the layout rules, the tab, Settings at the
    /// page, the command as its menu item runs it, or the match in its pane.
    fn run_palette_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(palette) = &self.palette else {
            return;
        };
        let (_, groups, _) = self.palette_groups(palette, cx);
        let Some((_, row)) = search::selected(&groups, index) else {
            return;
        };
        let target = row.target.clone();
        let found = match &target {
            Target::Match { pane, index } => palette.text.as_ref().and_then(|text| {
                let (_, hits, _) = text.panes.iter().find(|(p, ..)| p == pane)?;
                Some((text.needle.clone(), hits.get(*index)?.clone()))
            }),
            _ => None,
        };
        self.close_popup(window, cx);
        match target {
            Target::Agent(name) => {
                let metadata = self.sidebar.read(cx).metadata(&name);
                self.show_agent(&name, metadata, window, cx);
            }
            Target::Tab(index) => self.select_tab(index, window, cx),
            // After this update: opening Settings reads this window.
            Target::Settings(page) => cx.defer(move |cx| windows::open_settings_at(page, cx)),
            // From the focused pane, as a click on the menu item goes.
            Target::Command(name) => {
                let command = menu::commands()
                    .into_iter()
                    .find(|command| command.action.name() == name);
                if let Some(command) = command {
                    window.dispatch_action(command.action, cx);
                }
            }
            Target::Match { pane, .. } => {
                if let (Some((needle, hit)), Some(view)) = (found, self.panes.get(&pane).cloned()) {
                    self.focus_pane(pane, window, cx);
                    view.update(cx, |view, cx| view.show_match(&needle, &hit, window, cx));
                }
            }
        }
    }

    /// Closes the open chooser, list or palette and gives the keys back to the active pane.
    fn close_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.palette = None;
        self.chooser = Chooser::default();
        self.focus_active(window, cx);
    }

    /// ↑↓ in the new tab or split panel, the Attention list or the palette.
    fn move_selection(&mut self, step: isize, cx: &mut Context<Self>) {
        match self.popup {
            Some(popup @ (Popup::NewTab | Popup::Split(_))) => {
                let count = self.chooser_rows(cx).len();
                let index = search::step(self.chooser.index, count, step);
                self.chooser.index = index;
                let child = chooser_child(popup == Popup::NewTab, index);
                self.chooser.list.scroll_to_item(child);
            }
            Some(Popup::Attention) => {
                let count = self.sidebar.read(cx).attention().len();
                self.attention_index = search::step(self.attention_index, count, step);
            }
            Some(Popup::Palette) => {
                let Some(palette) = &self.palette else {
                    return;
                };
                let (_, groups, _) = self.palette_groups(palette, cx);
                let index = search::step(palette.index, search::count(&groups), step);
                let child = search::child(&groups, index);
                if let Some(palette) = &mut self.palette {
                    palette.index = index;
                    palette.list.scroll_to_item(child);
                }
            }
            _ => return,
        }
        cx.notify();
    }

    /// ⏎ in the new tab or split panel, the Attention list or the palette.
    fn open_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.popup {
            Some(Popup::NewTab | Popup::Split(_)) => {
                let rows = self.chooser_rows(cx);
                let index = self.chooser.index.min(rows.len() - 1);
                self.choose(rows[index].clone(), window, cx)
            }
            Some(Popup::Attention) => {
                let index = self.attention_index;
                self.open_attention(index, window, cx)
            }
            Some(Popup::Palette) => {
                if let Some(index) = self.palette.as_ref().map(|palette| palette.index) {
                    self.run_palette_row(index, window, cx)
                }
            }
            _ => {}
        }
    }

    /// The command palette, under the title bar over the dimmed window: the field, the rows in
    /// their groups, and the hints.
    fn palette_panel(
        &self,
        palette: &Palette,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let (mode, groups, empty) = self.palette_groups(palette, cx);
        let selected = search::selected(&groups, palette.index).map(|(index, _)| index);
        // Nothing to list yet (only `/` typed): the field sits on the hints.
        let listed = !groups.is_empty() || empty.is_some();
        let highlight = self.highlight();
        let rule = hsla(self.theme.fg(|t| t.agents_rule), 0.6);
        let top = title_bar_height(&ui, self.pet.is_some()) + ui.scale(44.0);
        // The list takes what the window has under the field and the hints, up to 470 points.
        let room = f32::from(window.viewport_size().height) - top - ui.scale(48.0 + 34.0 + 24.0);
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .max_h(px(ui.scale(470.0).min(room).max(ui.scale(68.0))))
            .overflow_y_scroll()
            .track_scroll(&palette.list)
            .px(ui.px(6.0))
            .pt(ui.px(6.0))
            .pb(ui.px(8.0));
        if let Some(empty) = empty {
            list = list.child(
                div()
                    .flex()
                    .justify_center()
                    .px(ui.px(16.0))
                    .py(ui.px(28.0))
                    .text_size(ui.px(13.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(empty),
            );
        }
        let mut index = 0;
        for group in groups {
            list = list.child(
                div()
                    .flex()
                    .gap(ui.px(6.0))
                    .px(ui.px(10.0))
                    .pt(ui.px(10.0))
                    .pb(ui.px(4.0))
                    .text_size(ui.px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child(group.label)
                    .child(
                        div()
                            .text_color(self.fg(|t| t.agents_border))
                            .child(group.note),
                    ),
            );
            for row in group.rows {
                list = list.child(self.palette_row(row, index, selected == Some(index), &ui, cx));
                index += 1;
            }
        }
        let field = div()
            .flex_shrink_0()
            .h(ui.px(48.0))
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .pl(ui.px(16.0))
            .pr(ui.px(14.0))
            .border_b_1()
            .border_color(rule)
            .child(footer_icon::icon(
                Icon::Search,
                self.fg(|t| t.agents_dim),
                ui.scale(15.0 / footer_icon::SIZE),
            ))
            .children(mode.chip().map(|chip| {
                div()
                    .flex_shrink_0()
                    .h(ui.px(22.0))
                    .px(ui.px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(highlight)
                    .text_size(ui.px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.fg(|t| t.agents_branch))
                    .whitespace_nowrap()
                    .child(chip)
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_size(ui.px(15.0))
                    .child(palette.input.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(11.5))
                    .text_color(self.fg(|t| t.agents_border))
                    .child("esc"),
            );
        let key = |key: &'static str| div().text_color(self.fg(|t| t.muted)).child(key);
        let hint = |keys: &'static str, label: &'static str| {
            div().flex().gap(ui.px(4.0)).child(key(keys)).child(label)
        };
        let text = self.fg(|t| t.agents_text);
        let switch = |id: &'static str, keys: &'static str, label: &'static str| {
            hint(keys, label)
                .id(id)
                .cursor_pointer()
                .hover(move |style| style.text_color(text))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.set_palette_query(keys, window, cx)
                }))
        };
        let footer = div()
            .flex_shrink_0()
            .h(ui.px(34.0))
            .flex()
            .items_center()
            .gap(ui.px(16.0))
            .px(ui.px(16.0))
            .when(listed, |footer| footer.border_t_1().border_color(rule))
            .text_size(ui.px(11.5))
            .text_color(self.fg(|t| t.agents_dimmer))
            .child(hint("↑↓", "Select"))
            .child(hint("↵", mode.enter()))
            .child(div().flex_1())
            .child(switch("palette-commands", ">", "Commands"))
            .child(switch("palette-text", "/", "Search text"));
        let card = div()
            .id("palette")
            .key_context(menu::PALETTE)
            .w_full()
            .max_w(ui.px(600.0))
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.5),
                offset: point(px(0.0), px(24.0)),
                blur_radius: px(60.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .text_color(text)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(field)
            .when(listed, |card| card.child(list))
            .child(footer);
        // Only the palette takes clicks while it is open; a click outside closes it.
        div()
            .id("palette-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(top))
            .px(ui.px(16.0))
            .bg(gpui::black().opacity(0.35))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(card)
    }

    /// A palette row: dot or icon, title and detail (or a line of text, its match marked), and
    /// the shortcut; the selected one on the selection's ground, the others on it faintly when
    /// hovered.
    fn palette_row(
        &self,
        row: search::Row,
        index: usize,
        selected: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let highlight = self.highlight();
        let scale = ui.scale(1.0);
        let lead = match row.lead {
            Lead::Dot { color, breathing } => Some(status_dot(self.fg(color), breathing, ui)),
            Lead::Settings => Some(
                footer_icon::icon(Icon::Settings, self.fg(|t| t.agents_dim), scale)
                    .into_any_element(),
            ),
            Lead::Command => Some(
                footer_icon::icon(Icon::Command, self.fg(|t| t.agents_dim), scale)
                    .into_any_element(),
            ),
            Lead::Nothing => None,
        };
        let body = match row.hit {
            Some(hit) => div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .font(mono_font(&self.template))
                .text_size(ui.px(12.5))
                .text_color(self.fg(|t| t.agents_branch))
                .child(StyledText::new(row.title).with_highlights([(
                    hit,
                    HighlightStyle {
                        color: Some(self.fg(|t| t.agents_text)),
                        background_color: Some(hsla(self.theme.fg(|t| t.agents_accent), 0.28)),
                        ..Default::default()
                    },
                )])),
            None => div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_baseline()
                .gap(ui.px(8.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(13.0))
                        .text_color(self.fg(|t| t.agents_text))
                        .child(row.title),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(ui.px(12.0))
                        .text_color(self.fg(|t| t.agents_dimmer))
                        .child(row.detail),
                ),
        };
        div()
            .id(("palette-row", index))
            .flex_shrink_0()
            .min_h(ui.px(34.0))
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .px(ui.px(10.0))
            .py(ui.px(5.0))
            .rounded(px(7.0))
            .cursor_pointer()
            .map(|row| {
                if selected {
                    row.bg(highlight)
                } else {
                    row.hover(move |style| style.bg(highlight.opacity(0.5)))
                }
            })
            .children(lead.map(|lead| {
                div()
                    .flex_shrink_0()
                    .w(ui.px(16.0))
                    .flex()
                    .justify_center()
                    .child(lead)
            }))
            .child(body)
            .child(self.keycaps(&row.keys, ui, 20.0))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.run_palette_row(index, window, cx)
            }))
    }

    /// Where a panel hangs: from the `+`; from the active pane's split button or the title bar's
    /// split icon, as [`split_from`] says; from the Attention bell, under it in the title bar or
    /// beside the strip.
    fn placed(&self, popup: Popup, width: f32, window: &Window, cx: &Context<Self>) -> Placed {
        let ui = UiFont::get(cx);
        let s = ui.scale(1.0);
        let title = title_bar_height(&ui, self.pet.is_some());
        let side = self.sidebar_shown(&ui);
        let viewport = window.viewport_size();
        let rect = |x: f32, y: f32, w: f32, h: f32| Bounds {
            origin: point(px(x), px(y)),
            size: gpui::size(px(w), px(h)),
        };
        let spots = self.spots.borrow();
        let (anchor, how) = match popup {
            Popup::NewTab => (
                // Not drawn yet: where the first tab starts.
                spots
                    .get(&Spot::NewTab)
                    .copied()
                    .unwrap_or_else(|| rect(side + 10.0, 0.0, 0.0, title)),
                Hang::BelowLeft,
            ),
            Popup::Split(_) if self.split_hanging() == Some(SplitFrom::Pane) => {
                let pane = spots
                    .get(&Spot::Pane(self.workspace.active_pane()))
                    .copied()
                    .unwrap_or_else(|| {
                        let width = f32::from(viewport.width) - side;
                        rect(side, title, width, f32::from(viewport.height) - title)
                    });
                let (right, top) = (f32::from(pane.right()), f32::from(pane.top()));
                // The header's split button, first of its three at its right end, centred on the
                // header's height.
                let button = PANE_BUTTON * s;
                (
                    rect(
                        right - PANE_HEADER_END * s - 3.0 * button,
                        top + (PANE_HEADER - PANE_BUTTON) * s / 2.0,
                        button,
                        button,
                    ),
                    Hang::BelowRight,
                )
            }
            Popup::Split(_) => (
                // Not drawn yet: before the search field at the title bar's right end.
                spots.get(&Spot::Split).copied().unwrap_or_else(|| {
                    let button = BAR_BUTTON * s;
                    let right = f32::from(viewport.width) - 10.0 - SEARCH * s - 4.0 - SPLIT_GAP * s;
                    rect(right - button, (title - button) / 2.0, button, button)
                }),
                Hang::BelowRight,
            ),
            _ => {
                if self.collapsed {
                    (
                        rect(0.0, title + RAIL_BELL * s, side, 30.0 * s),
                        Hang::Beside,
                    )
                } else {
                    // Not drawn yet: at the right end of the header row in the title bar.
                    let anchor = spots.get(&Spot::Bell).copied().unwrap_or_else(|| {
                        let bell = 28.0 * s;
                        let right = self.sidebar_width - HEAD_END;
                        rect(right - bell, (title - bell) / 2.0, bell, bell)
                    });
                    (anchor, Hang::BelowLeft)
                }
            }
        };
        popover::hang(anchor, ui.scale(width), how, viewport, s)
    }

    /// The new tab panel (under the `+`: a field, then Shell and Agent…, then the agents that match)
    /// or the split panel (under the split button it hangs from: the side, then the same rows).
    /// The selected row is lit; Esc or a click outside closes it.
    fn chooser_panel(
        &self,
        popup: Popup,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let s = ui.scale(1.0);
        let theme = &*self.theme;
        let side = match popup {
            Popup::Split(direction) => Some(direction),
            _ => None,
        };
        let placed = self.placed(
            popup,
            if side.is_some() { 316.0 } else { 340.0 },
            window,
            cx,
        );
        let query = self.chooser_query(cx);
        let rows = self.chooser_rows(cx);
        let selected = self.chooser.index.min(rows.len() - 1);
        let agents = self.sidebar.read(cx).agents();
        let none = agents.is_empty();
        let now = now();
        let accent = self.fg(|t| t.agents_accent);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let keys = |action: &dyn gpui::Action| popover::keys(theme, &ui, &menu::keys(action));
        let note = |text: String| {
            div()
                .flex_shrink_0()
                .px(ui.px(9.0))
                .py(ui.px(5.0))
                .text_size(ui.px(12.0))
                .text_color(self.fg(|t| t.agents_dim))
                .child(text)
        };
        let mut list = div()
            .id("chooser-list")
            .flex()
            .flex_col()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&self.chooser.list)
            .child(popover::heading(theme, &ui, side.map_or("NEW", open_where)));
        for (index, choice) in rows.iter().enumerate() {
            let lit = index == selected;
            let id = ("chooser-row", index);
            let row = match choice {
                Choice::Shell => popover::lead_row(
                    theme,
                    &ui,
                    id,
                    footer_icon::icon(Icon::NewShell, accent, s),
                    lit,
                )
                .child(div().flex_1().child("Shell"))
                .when(side.is_none(), |row| row.child(keys(&menu::NewShell))),
                Choice::NewAgent => popover::lead_row(
                    theme,
                    &ui,
                    id,
                    footer_icon::icon(Icon::NewAgent, accent, s),
                    lit,
                )
                .child(div().flex_1().child("Agent…"))
                .when(side.is_none(), |row| row.child(keys(&menu::NewAgent))),
                Choice::Agent(name) => {
                    let dot = div().size(ui.px(7.0)).rounded_full().bg(self.fg(dot(
                        &Shown::Agent(name.clone()),
                        &agents,
                        now,
                    )));
                    let short = name.rsplit('/').next().unwrap_or(name).to_owned();
                    let project = crate::agents::group(name).trim_end_matches('/').to_owned();
                    popover::lead_row(theme, &ui, id, dot, lit)
                        .child(
                            div()
                                .flex_shrink(1.0)
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(short),
                        )
                        .when(!project.is_empty(), |row| {
                            row.child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(ui.px(12.0))
                                    .text_color(dimmer)
                                    .child(project),
                            )
                        })
                        .child(div().flex_1())
                        .when(self.workspace.find(name).is_some(), |row| {
                            row.child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(ui.px(12.0))
                                    .text_color(dimmer)
                                    .child("Move here"),
                            )
                        })
                }
            };
            let choice = choice.clone();
            list = list.child(row.on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| this.choose(choice.clone(), window, cx),
            )));
            // The new tab panel names the agents' group; the split panel lists them straight on.
            if index == 1 && side.is_none() {
                list = list.child(popover::heading(theme, &ui, "AGENTS").pt(ui.px(10.0)));
            }
        }
        if none {
            list = list.child(note("No agents to open here.".into()));
        } else if rows.len() == 2 {
            list = list.child(note(format!("No agents match “{}”", query.trim())));
        }
        let mut panel = popover::panel(theme, &ui)
            .id("chooser")
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        if let Some(input) = &self.chooser.input {
            panel = panel.child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .h(ui.px(34.0))
                    .px(ui.px(10.0))
                    .mb(ui.px(4.0))
                    .border_b_1()
                    .border_color(popover::edge_rule(theme))
                    .child(footer_icon::icon(
                        Icon::Search,
                        dimmer,
                        ui.scale(13.0 / footer_icon::SIZE),
                    ))
                    .child(div().flex_1().min_w(px(0.0)).child(input.clone())),
            );
        }
        if let Some(direction) = side {
            panel = panel
                .child(popover::heading(theme, &ui, "SPLIT").pb(ui.px(6.0)))
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .gap(ui.px(6.0))
                        .px(ui.px(4.0))
                        .pb(ui.px(8.0))
                        .children(
                            [
                                Direction::Left,
                                Direction::Right,
                                Direction::Up,
                                Direction::Down,
                            ]
                            .into_iter()
                            .map(|tile| self.split_tile(tile, tile == direction, &ui, cx)),
                        ),
                )
                .child(popover::rule(theme, &ui));
        }
        panel = panel.child(list).when(side.is_none(), |panel| {
            panel.child(popover::hints(
                theme,
                &ui,
                &[("↑↓", "move"), ("↩", "open"), ("esc", "close")],
            ))
        });
        // A click outside closes it; nothing is dimmed, as for a menu.
        div()
            .id("chooser-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(panel)
    }

    /// One of the split panel's four sides: a small frame with that half lit, its name and its
    /// shortcut when it has one.
    fn split_tile(
        &self,
        direction: Direction,
        on: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let (label, action): (&str, &dyn gpui::Action) = match direction {
            Direction::Left => ("Left", &menu::SplitLeft),
            Direction::Right => ("Right", &menu::SplitRight),
            Direction::Up => ("Up", &menu::SplitUp),
            Direction::Down => ("Down", &menu::SplitDown),
        };
        let lit = popover::lit(&self.theme);
        let border = self.fg(|t| t.agents_border);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let (edge, stroke, fill, text) = if on {
            (
                border,
                self.fg(|t| t.agents_branch),
                self.fg(|t| t.agents_accent).opacity(0.55),
                self.fg(|t| t.agents_text),
            )
        } else {
            (
                popover::edge_rule(&self.theme),
                dimmer,
                dimmer.opacity(0.35),
                self.fg(|t| t.agents_dim),
            )
        };
        // The lit half, its outer corners rounded inside the frame's.
        let half = div().absolute().bg(fill);
        let radius = ui.px(3.0);
        let half = match direction {
            Direction::Left => half
                .left_0()
                .top_0()
                .h_full()
                .w(relative(0.5))
                .rounded_l(radius),
            Direction::Right => half
                .right_0()
                .top_0()
                .h_full()
                .w(relative(0.5))
                .rounded_r(radius),
            Direction::Up => half
                .left_0()
                .top_0()
                .w_full()
                .h(relative(0.5))
                .rounded_t(radius),
            Direction::Down => half
                .left_0()
                .bottom_0()
                .w_full()
                .h(relative(0.5))
                .rounded_b(radius),
        };
        div()
            .id(SharedString::from(format!("split-{label}")))
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(ui.px(5.0))
            .pt(ui.px(8.0))
            .pb(ui.px(6.0))
            .rounded(ui.px(8.0))
            .border_1()
            .border_color(edge)
            .cursor_pointer()
            .when(on, |tile| tile.bg(lit))
            .hover(move |style| style.border_color(border))
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .w(ui.px(34.0))
                    .h(ui.px(24.0))
                    .rounded(ui.px(4.0))
                    .border_1()
                    .border_color(stroke)
                    .child(half),
            )
            .child(div().text_size(ui.px(11.5)).text_color(text).child(label))
            .child(
                div()
                    .h(ui.px(12.0))
                    .text_size(ui.px(10.5))
                    .text_color(dimmer.opacity(0.8))
                    .child(menu::keys(action).concat()),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.popup = Some(Popup::Split(direction));
                cx.notify();
            }))
    }
}

/// A title bar button's hover text, with its shortcut after it.
#[derive(Clone)]
struct BarTip {
    text: &'static str,
    keys: String,
    size: Pixels,
    color: Hsla,
    dim: Hsla,
    background: Hsla,
    border: Hsla,
}

impl Render for BarTip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(px(8.0))
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
            .child(div().text_color(self.dim).child(self.keys.clone()))
    }
}

/// The status dot's colour for what a pane shows.
fn dot(shown: &Shown, agents: &[Agent], now: f64) -> Pick {
    match shown {
        Shown::Agent(name) => match agents.iter().find(|a| &a.name == name) {
            Some(agent) => card::look(Panel::default().status(agent, now)).color,
            None => |t| t.agents_faint,
        },
        Shown::Shell => |t| t.muted,
        Shown::Empty => |t| t.agents_faint,
    }
}

/// Seconds since the epoch, as the agent statuses count them.
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// The sidebar's technical lines use the terminal's font.
fn mono_font(options: &Options) -> gpui::Font {
    let mut mono = gpui::font(options.font_family.clone());
    if !options.fallbacks.is_empty() {
        mono.fallbacks = Some(gpui::FontFallbacks::from_fonts(options.fallbacks.clone()));
    }
    mono
}

impl Render for PaddockWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fill(window, cx);
        if self.fading() {
            window.request_animation_frame();
        }
        // The View menu follows the sidebar.
        let (fold, by_name) = self.sidebar.read(cx).view_state();
        let view_state = (fold, by_name, self.collapsed, self.right.open);
        if self.menu_state != Some(view_state) {
            self.menu_state = Some(view_state);
            cx.set_menus(menu::menus(fold, by_name, self.collapsed, self.right.open));
        }
        // The menu button stays lit while its menu is open.
        let menu_open = self.popup == Some(Popup::Actions);
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_menu_open(menu_open, cx));
        let ui = UiFont::get(cx);
        // Larger interface sizes make the title bar taller; the traffic lights stay centred on it.
        let height = title_bar_height(&ui, self.pet.is_some());
        if self.lights != Some(height) {
            self.lights = Some(height);
            window.set_traffic_light_position(traffic_lights(height));
        }
        // A zoomed pane fills the tab; the split waits underneath.
        let (root, shown) = match self.workspace.zoomed() {
            Some(pane) => (Node::Pane(pane), 1),
            None => {
                let tab = self.workspace.tab();
                (tab.root.clone(), tab.panes().len())
            }
        };
        let agents = self.sidebar.read(cx).agents();
        let now = now();
        // The cards stand on the frame, their tops on the title bar's bottom edge, where the pet
        // walks; seams between them and along the right and bottom edges, and after the strip.
        let mut cards = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .gap(px(CARD_GAP))
            .pr(px(CARD_GAP))
            .pb(px(CARD_GAP))
            .when(self.collapsed, |cards| cards.pl(px(CARD_GAP)))
            .child(self.node(&root, shown, &agents, cx));
        let right = self
            .right
            .open
            .then(|| self.right.shown(self.right_room(window, cx)));
        let seams = across(
            f32::from(window.viewport_size().width),
            self.sidebar_shown(&ui),
            self.collapsed,
            right,
        );
        // The right sidebar, a card pushed out from the right edge: the terminal narrows for it.
        if let Some(width) = right {
            let panel = self.right.render(
                &self.theme,
                &ui,
                cx.listener(|this, tab: &RightTab, _, cx| {
                    this.right.tab = *tab;
                    this.save_layout(cx);
                    cx.notify();
                }),
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.right.wide = !this.right.wide;
                    cx.notify();
                }),
                cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_right(cx)),
            );
            cards = cards.child(
                self.card(false)
                    .flex_shrink_0()
                    .w(px(width))
                    .h_full()
                    .overflow_hidden()
                    .child(panel),
            );
        }
        let body = div()
            .relative()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_row()
            .child(self.sidebar.clone())
            .child(cards)
            .children(seams.left_grip.map(|x| self.grip(x, cx)))
            .children(seams.right_grip.map(|x| self.right_grip(x, cx)));
        let dialog = match self.popup {
            Some(Popup::Actions) => Some(self.actions_menu(cx)),
            Some(Popup::Attention) => Some(self.attention_panel(window, cx)),
            Some(Popup::Palette) => self
                .palette
                .as_ref()
                .map(|palette| self.palette_panel(palette, window, cx)),
            Some(popup) => Some(self.chooser_panel(popup, window, cx)),
            None => None,
        };
        ui.apply(div())
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // GPUI's default size, scaled, for text nothing else sizes.
            .text_size(ui.px(16.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .when(self.popup.is_some(), |root| root.key_context(menu::DIALOG))
            .on_action(
                cx.listener(|this, _: &menu::NewTab, window, cx| this.toggle_new_tab(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::NewShell, window, cx| this.new_shell(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::SplitRight, window, cx| {
                this.ask_split(SplitAsk::Side(Direction::Right), window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitDown, window, cx| {
                this.ask_split(SplitAsk::Side(Direction::Down), window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitLeft, window, cx| {
                this.ask_split(SplitAsk::Side(Direction::Left), window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitUp, window, cx| {
                this.ask_split(SplitAsk::Side(Direction::Up), window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &menu::ClosePane, window, cx| this.close_pane(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::StopAgent, window, cx| {
                this.stop_agent(None, window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::ShowAttention, window, cx| {
                this.toggle_attention(window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &menu::Search, window, cx| {
                    this.toggle_palette("", window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &menu::CommandPalette, window, cx| {
                this.toggle_palette(">", window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| this.move_selection(1, cx)))
            .on_action(
                cx.listener(|this, _: &menu::SelectPrevious, _, cx| this.move_selection(-1, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                this.open_selected(window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::CloseTab, window, cx| {
                let index = this.workspace.active_tab;
                this.close_tab(index, window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::ToggleFold, _, cx| {
                this.sidebar.update(cx, |s, cx| s.toggle_fold(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &menu::ToggleSortByName, _, cx| {
                this.sidebar.update(cx, |s, cx| s.toggle_sort(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &menu::ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(
                cx.listener(|this, _: &menu::ToggleRightSidebar, _, cx| this.toggle_right(cx)),
            )
            // Dragging the dividers: followed wherever the mouse goes, and let go anywhere.
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<SidebarDrag>, _, cx| {
                    this.resize_sidebar(f32::from(event.event.position.x), cx)
                }),
            )
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<RightDrag>, window, cx| {
                    let room = this.right_room(window, cx);
                    if this.right.drag(room, f32::from(event.event.position.x)) {
                        cx.notify();
                    }
                }),
            )
            .capture_any_mouse_up(cx.listener(|this, _, _, cx| {
                this.finish_resize(cx);
                this.finish_right_resize(cx);
            }))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.finish_resize(cx);
                    this.finish_right_resize(cx);
                }),
            )
            .on_action(cx.listener(|_, _: &menu::Minimize, window, _| window.minimize_window()))
            .on_action(cx.listener(|_, _: &menu::Zoom, window, _| window.zoom_window()))
            .on_action(
                cx.listener(|this, _: &menu::ZoomPane, window, cx| this.toggle_zoom(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::NextTab, window, cx| this.cycle_tab(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::PreviousTab, window, cx| {
                this.cycle_tab(false, window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::Tab1, w, cx| this.nth_tab(1, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab2, w, cx| this.nth_tab(2, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab3, w, cx| this.nth_tab(3, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab4, w, cx| this.nth_tab(4, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab5, w, cx| this.nth_tab(5, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab6, w, cx| this.nth_tab(6, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab7, w, cx| this.nth_tab(7, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab8, w, cx| this.nth_tab(8, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab9, w, cx| this.nth_tab(9, w, cx)))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_popup(window, cx)),
            )
            .child(self.title_bar(&agents, now, window.is_fullscreen(), cx))
            .child(body)
            .children(dialog)
            // While a divider is dragged the cursor keeps its shape, and nothing under it
            // reacts.
            .when(self.resizing.is_some() || self.right.resizing(), |root| {
                root.child(
                    div()
                        .id("resizing")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .cursor_col_resize(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_sidebar_width_is_the_same_integer_in_config_and_view() {
        let disk = "# sidebar\nsidebar_width = 380\ntheme = \"tide\"\n";
        for (dragged, expected) in [(312.734_38, 313.0), (312.25, 312.0), (312.5, 313.0)] {
            let mut width = dragged;
            let value = finish_sidebar_width(&mut width);
            let mut draft = Draft::new(Some(disk.into())).unwrap();
            draft.set("sidebar_width", &value);
            let Saved::Written { text, .. } = draft.save(Some(disk)).unwrap().unwrap() else {
                panic!("width was not written");
            };
            assert_eq!(
                text,
                format!("# sidebar\nsidebar_width = {expected}\ntheme = \"tide\"\n")
            );
            assert_eq!(width, expected);
            assert_eq!(Config::parse(&text).unwrap().sidebar_width, width);
        }
    }

    fn shell(program: &str, cwd: &str) -> Subject {
        Subject::Shell {
            program: program.into(),
            cwd: cwd.into(),
        }
    }

    #[test]
    fn tabs_name_agents_short_unless_that_reads_the_same_as_another() {
        let agent = Subject::Agent("paddock/main".into());
        let open = vec!["paddock/main".to_owned(), "paddock/dev-fonts".to_owned()];
        assert_eq!(tab_title(&agent, &open), "main");
        let clash = vec!["paddock/main".to_owned(), "saddle/main".to_owned()];
        assert_eq!(tab_title(&agent, &clash), "paddock/main");
        // Not open anywhere else, or without a group.
        assert_eq!(tab_title(&agent, &[]), "main");
        assert_eq!(tab_title(&Subject::Agent("solo".into()), &[]), "solo");
    }

    #[test]
    fn tabs_name_shells_by_program_and_directory() {
        let open = [];
        assert_eq!(
            tab_title(&shell("/bin/zsh", "/Users/me/paddock"), &open),
            "zsh · paddock"
        );
        assert_eq!(tab_title(&shell("/bin/zsh", "/tmp/x/"), &open), "zsh · x");
        assert_eq!(tab_title(&shell("fish", "/"), &open), "fish · /");
        assert_eq!(
            tab_title(&Subject::Command("htop -d 5".into()), &open),
            "htop -d 5"
        );
        assert_eq!(tab_title(&Subject::Empty, &open), "empty");
    }

    #[test]
    fn a_split_tab_counts_its_other_panes() {
        assert_eq!(more_panes(1), None);
        assert_eq!(more_panes(2).as_deref(), Some("+1"));
        assert_eq!(more_panes(4).as_deref(), Some("+3"));
    }

    #[test]
    fn directories_shorten_to_their_last_part() {
        let home = Some("/Users/me");
        assert_eq!(short_dir("/Users/me", home), "~");
        assert_eq!(short_dir("/Users/me/", home), "~");
        assert_eq!(short_dir("/Users/me/code", home), "~/code");
        assert_eq!(short_dir("/Users/me/code/paddock", home), "~/…/paddock");
        assert_eq!(short_dir("/Users/meow/x", home), "/…/x");
        assert_eq!(short_dir("/tmp", home), "/tmp");
        assert_eq!(short_dir("/", home), "/");
        assert_eq!(short_dir("/Users/me/code", None), "/…/code");
    }

    #[test]
    fn pane_headers_name_in_full_with_the_short_directory() {
        let home = Some("/Users/me");
        let agent = Subject::Agent("paddock/main".into());
        assert_eq!(
            pane_name(&agent, Some("/Users/me/code/paddock"), home),
            ("paddock/main".into(), Some("~/…/paddock".into()))
        );
        assert_eq!(pane_name(&agent, None, home), ("paddock/main".into(), None));
        assert_eq!(
            pane_name(&shell("/bin/zsh", "/Users/me/code"), None, home),
            ("zsh".into(), Some("~/code".into()))
        );
        assert_eq!(
            pane_name(&Subject::Empty, None, home),
            ("empty".into(), None)
        );
    }

    #[test]
    fn only_split_tabs_have_pane_headers() {
        assert!(!header_shown(1));
        assert!(header_shown(2));
        assert!(header_shown(3));
    }

    #[test]
    fn panes_other_than_the_active_one_dim_when_several_show() {
        assert!(!dimmed(1, true));
        // A zoomed pane shows alone: nothing to tell it from.
        assert!(!dimmed(1, false));
        assert!(!dimmed(2, true));
        assert!(dimmed(2, false));
        assert!(dimmed(4, false));
    }

    #[test]
    fn traffic_lights_centre_on_the_title_bar() {
        assert_eq!(traffic_lights(40.0), point(px(14.0), px(13.0)));
        assert_eq!(traffic_lights(54.0), point(px(14.0), px(20.0)));
        assert_eq!(title_bar_height(&UiFont::default(), false), TITLE_BAR);
        // The pet walks at twice its pixel size on the terminal's top edge: the bar makes room.
        assert_eq!(title_bar_height(&UiFont::default(), true), PET_TITLE_BAR);
        let large = UiFont {
            family: None,
            size: 18.0,
        };
        assert!(title_bar_height(&large, false) > TITLE_BAR);
        assert!(title_bar_height(&large, true) > PET_TITLE_BAR);
        let small = UiFont {
            family: None,
            size: 11.0,
        };
        assert_eq!(title_bar_height(&small, false), TITLE_BAR);
        assert_eq!(title_bar_height(&small, true), PET_TITLE_BAR);
    }

    #[test]
    fn the_title_bar_left_part_is_the_sidebar_or_the_lights_and_the_expand_button() {
        let base = UiFont::default();
        // Expanded: as wide as the sidebar, the divider following it, full screen or not.
        assert_eq!(bar_left(false, false, 300.0, &base), 300.0);
        assert_eq!(bar_left(false, true, 300.0, &base), 300.0);
        assert_eq!(bar_left(false, false, 220.0, &base), 220.0);
        // Collapsed: the traffic lights, the expand button and room after it, whatever the
        // sidebar's width.
        assert_eq!(bar_left(true, false, 300.0, &base), 118.0);
        assert_eq!(bar_left(true, false, 220.0, &base), 118.0);
        // In full screen there are no traffic lights: the button is 12 points in.
        assert_eq!(bar_left(true, true, 300.0, &base), 54.0);
        // The button grows with the interface size; the lights do not.
        let large = UiFont {
            family: None,
            size: 19.5,
        };
        assert_eq!(bar_left(true, false, 300.0, &large), 76.0 + 48.0 + 10.0);
        assert_eq!(bar_left(true, true, 300.0, &large), 12.0 + 48.0 + 10.0);
    }

    #[test]
    fn the_cards_sit_against_the_sidebar_or_a_seam_in_and_the_grips_take_the_seams() {
        // Expanded: the panes' card starts at the sidebar, which keeps its own room; the grips
        // take the middle of the seam before each card.
        assert_eq!(
            across(1200.0, 300.0, false, Some(400.0)),
            Across {
                panes: 300.0,
                left_grip: Some(296.0),
                right_grip: Some(788.0),
            }
        );
        assert_eq!(
            across(1200.0, 300.0, false, None),
            Across {
                panes: 300.0,
                left_grip: Some(296.0),
                right_grip: None,
            }
        );
        // Beside the strip: a seam before the card too, and no grip there.
        assert_eq!(
            across(1200.0, 52.0, true, Some(400.0)),
            Across {
                panes: 60.0,
                left_grip: None,
                right_grip: Some(788.0),
            }
        );
    }

    #[test]
    fn only_live_shells_make_closing_ask() {
        assert_eq!(close_question(&[], 2, false), None);
        assert_eq!(close_question(&[], 0, true), None);
        let one = close_question(&["shell · /tmp".into()], 0, false).unwrap();
        assert_eq!(one.message, "End the running shell?");
        assert_eq!(one.confirm, "End Shell");
        assert!(
            one.detail
                .contains("What runs in it in the foreground ends too.")
        );
        assert!(one.detail.starts_with("shell · /tmp\n\n"));
        assert!(!one.detail.contains("Agent"));
    }

    #[test]
    fn the_question_lists_every_shell_and_says_agents_only_detach() {
        let shells = vec![
            "shell · /tmp".to_owned(),
            "shell · /Users/me/code".to_owned(),
        ];
        let quit = close_question(&shells, 1, true).unwrap();
        assert_eq!(quit.message, "Quit paddock and end the running shells?");
        assert_eq!(quit.confirm, "Quit");
        assert!(quit.detail.contains("What runs in them"));
        assert_eq!(
            close_question(&shells, 0, false).unwrap().confirm,
            "End Shells"
        );
        assert!(
            quit.detail
                .starts_with("shell · /tmp\nshell · /Users/me/code\n\n")
        );
        assert!(
            quit.detail
                .contains("Agent views only detach; the agents keep running.")
        );
    }

    fn names() -> Vec<String> {
        ["paddock/dev-fonts", "paddock/main", "ranch/test-m2", "solo"]
            .map(String::from)
            .to_vec()
    }

    fn agents_listed(rows: &[Choice]) -> Vec<&str> {
        rows.iter()
            .filter_map(|row| match row {
                Choice::Agent(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn typing_in_the_new_tab_panel_narrows_the_agents_by_name_and_project() {
        let names = names();
        // Nothing typed: Shell and Agent… first, then every agent, Shell selected.
        let all = choices(&names, "");
        assert_eq!(all[..2], [Choice::Shell, Choice::NewAgent]);
        assert_eq!(
            agents_listed(&all),
            names.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(first_choice("", &all), 0);
        // By the project, by the name, ignoring case, every word in any order.
        assert_eq!(
            agents_listed(&choices(&names, "PADDOCK")),
            ["paddock/dev-fonts", "paddock/main"]
        );
        assert_eq!(agents_listed(&choices(&names, "m2")), ["ranch/test-m2"]);
        assert_eq!(
            agents_listed(&choices(&names, " main  paddock ")),
            ["paddock/main"]
        );
        // Typing selects the first agent that matches; Shell and Agent… stay.
        let typed = choices(&names, "fonts");
        assert_eq!(typed[..2], [Choice::Shell, Choice::NewAgent]);
        assert_eq!(first_choice("fonts", &typed), 2);
        // Nothing matches: only Shell and Agent…, Shell selected.
        let none = choices(&names, "zzz");
        assert_eq!(none.len(), 2);
        assert_eq!(first_choice("zzz", &none), 0);
        // Blanks alone are nothing typed.
        assert_eq!(first_choice("  ", &choices(&names, "  ")), 0);
    }

    #[test]
    fn the_split_panel_opens_on_the_side_asked_for_and_says_where() {
        use Direction::*;
        // The split button opens it on Right; each side's shortcut on its own side.
        assert_eq!(
            split_popup(None, SplitAsk::Button),
            Some(Popup::Split(Right))
        );
        for side in [Right, Down, Left, Up] {
            assert_eq!(
                split_popup(None, SplitAsk::Side(side)),
                Some(Popup::Split(side))
            );
        }
        // Open on another side, a shortcut switches it; on its own side, it closes it.
        assert_eq!(
            split_popup(Some(Popup::Split(Right)), SplitAsk::Side(Down)),
            Some(Popup::Split(Down))
        );
        assert_eq!(
            split_popup(Some(Popup::Split(Down)), SplitAsk::Side(Down)),
            None
        );
        // From another panel, it takes over.
        assert_eq!(
            split_popup(Some(Popup::NewTab), SplitAsk::Side(Right)),
            Some(Popup::Split(Right))
        );
        // The heading over what to open follows the side.
        assert_eq!(open_where(Right), "OPEN ON THE RIGHT");
        assert_eq!(open_where(Left), "OPEN ON THE LEFT");
        assert_eq!(open_where(Up), "OPEN ABOVE");
        assert_eq!(open_where(Down), "OPEN BELOW");
    }

    #[test]
    fn the_split_panel_hangs_from_where_it_was_asked_for() {
        use Direction::*;
        use SplitFrom::*;
        // Each button: its own, with headers or without.
        for header in [false, true] {
            assert_eq!(split_from(SplitAsk::Bar, None, header), Bar);
            assert_eq!(split_from(SplitAsk::Bar, Some(Pane), header), Bar);
        }
        assert_eq!(split_from(SplitAsk::Button, None, true), Pane);
        assert_eq!(split_from(SplitAsk::Button, Some(Bar), true), Pane);
        // A shortcut or the menu: the active pane's button when panes have headers, else the
        // title bar's icon.
        for side in [Right, Down, Left, Up] {
            assert_eq!(split_from(SplitAsk::Side(side), None, true), Pane);
            assert_eq!(split_from(SplitAsk::Side(side), None, false), Bar);
        }
        // Switching the side of an open panel leaves it where it hangs.
        assert_eq!(split_from(SplitAsk::Side(Down), Some(Bar), true), Bar);
        assert_eq!(split_from(SplitAsk::Side(Down), Some(Pane), true), Pane);
        // The title bar's icon opens it on Right, as the pane's button does.
        assert_eq!(
            split_popup(Some(Popup::Split(Down)), SplitAsk::Bar),
            Some(Popup::Split(Right))
        );
    }

    #[test]
    fn the_keys_scroll_to_the_selected_row_past_the_headings() {
        // New tab: NEW, Shell, Agent…, AGENTS, then the agents.
        assert_eq!(chooser_child(true, 0), 1);
        assert_eq!(chooser_child(true, 1), 2);
        assert_eq!(chooser_child(true, 2), 4);
        // Split: OPEN …, then every row.
        assert_eq!(chooser_child(false, 2), 3);
    }
}
