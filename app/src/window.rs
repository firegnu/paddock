//! The window's root view: the title bar, where the tabs share the traffic lights' row; below it
//! the Agents sidebar on the left and the active tab's panes on the right, each a card, a split
//! pane's header at its top. `layout.rs` holds the rules; this file draws them and keeps one
//! terminal view per pane.
use crate::{
    agents::Panel,
    attention::Kind as AttentionKind,
    browser::{self, Owner, cover},
    browser_view::{BrowserView, Handoff, Visited},
    card,
    changes::{self, ChangesView, Follow, Frame},
    config::Config,
    corral::{Agent, Role},
    diagnostics::{Report, Startup},
    find,
    fonts::UiFont,
    footer_icon::{self, Icon},
    frost::{Frost, Shape},
    kanban_view::{self, KanbanEvent, KanbanView, NewTask, NewTaskEvent},
    kind_icon,
    layout::{self, Axis, Direction, Node, PaneId, Placement, Shown, Workspace},
    layout_state::{Content, Layout, Store, WindowSize},
    menu,
    motion::{self, HoverMotion},
    new_agent::{self, Place, Started},
    new_agent_view::{NewAgentEvent, NewAgentView, Seed},
    pause,
    pet::PetView,
    popover::{self, Hang, Placed, Tone},
    right_panel::{self, RightPanel, Room, Tab as RightTab},
    search::{self, Lead, Mode, Target},
    settings::{Conflict, Draft, Saved},
    sidebar::{self, Sidebar, SidebarEvent, status_dot},
    tab_fit::{self, Bar, TabSize, Title},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::{AttachEnded, Launch, Options, TerminalView, hsla},
    viewer::AgentMetadata,
    windows,
};
use gpui::{
    Animation, AnimationExt, AnyElement, AnyView, Bounds, BoxShadow, ClickEvent, Context, Div,
    DragMoveEvent, Entity, ExternalPaths, FocusHandle, Focusable, Font, FontWeight, HighlightStyle,
    Hsla, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, PromptLevel, Render,
    ScrollHandle, SharedString, Size, Stateful, StyleRefinement, StyledText, Task, TextRun, Window,
    WindowBackgroundAppearance, canvas, div, ease_in_out, point, prelude::*, px, relative, size,
};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSView, NSWindow};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

/// What "New shell" and the `Shell` choice start, and the `paddock ctl` instance their
/// `PADDOCK_INSTANCE` names.
pub struct NewShell {
    pub program: String,
    pub cwd: String,
    pub instance: String,
    /// Why `paddock ctl` did not start (then `instance` is empty), shown once at startup.
    pub ctl_problem: Option<String>,
}

/// When the latest normal-window resize is ready to save.
const WINDOW_SIZE_SAVE_PAUSE: Duration = Duration::from_millis(500);

#[derive(Default)]
struct WindowSizeSave {
    due: Option<Instant>,
}

impl WindowSizeSave {
    fn changed(&mut self, now: Instant) {
        self.due = Some(now + WINDOW_SIZE_SAVE_PAUSE);
    }

    fn take_due(&mut self, now: Instant) -> bool {
        if self.due.is_some_and(|due| now >= due) {
            self.due = None;
            true
        } else {
            false
        }
    }
}

/// GPUI does not expose the native minimum size or miniaturized state.
fn native_window(window: &Window) -> Option<Retained<NSWindow>> {
    MainThreadMarker::new()?;
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI's live NSView, read on the main thread while `window` is borrowed.
    let view = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
    view.window()
}

/// Restores only the main window's size, centred in its screen's usable area. Explicit
/// `--bounds` bypasses this. Keep AppKit's existing minimum rather than setting a new one.
pub fn restore_size(window: &mut Window, saved: WindowSize, cx: &mut gpui::App) {
    let Some(native) = native_window(window) else {
        return;
    };
    let Some(screen) = native.screen() else {
        return;
    };
    let available = screen.visibleFrame();
    let minimum = native.minSize();
    let restored = saved.for_startup(
        WindowSize {
            width: available.size.width as f32,
            height: available.size.height as f32,
        },
        WindowSize {
            width: minimum.width as f32,
            height: minimum.height as f32,
        },
    );
    let mut frame = native.frame();
    frame.size.width = restored.width as f64;
    frame.size.height = restored.height as f64;
    frame.origin.x = available.origin.x + (available.size.width - frame.size.width) / 2.0;
    frame.origin.y = available.origin.y + (available.size.height - frame.size.height) / 2.0;
    native.setFrame_display(frame, false);
    // The native resize happened during construction, before GPUI can dispatch to this window.
    window.bounds_changed(cx);
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
    /// The Kanban tab's New task panel, under its button.
    NewTask,
    /// The tabs with no room in the title bar, from its `+N`.
    Overflow,
    /// The New Agent panel over the dimmed window.
    NewAgent,
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

/// What dragging the seam between split panes carries.
struct SplitDrag;

/// A seam between split panes being dragged: its split, named by the pane after it.
struct Splitting {
    after: PaneId,
    axis: Axis,
    /// The first part's share and the mouse along the axis when it was pressed.
    from: f32,
    grab: f32,
    /// The split's length along the axis, and the least each part may keep.
    length: f32,
    least: (f32, f32),
}

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
    /// The New Agent panel, to start one there.
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

/// What a panel can hang from, or a seam is dragged across, as last drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Spot {
    /// The title bar's `+`.
    NewTab,
    /// The title bar's split icon.
    Split,
    /// The Attention bell, in the title bar while the sidebar is expanded.
    Bell,
    /// The Kanban tab's New task.
    NewTask,
    /// The title bar's `+N`.
    Overflow,
    Pane(PaneId),
    /// The split whose seam comes just before this pane.
    Seam(PaneId),
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
        Direction::Left => "Open on the left",
        Direction::Right => "Open on the right",
        Direction::Up => "Open above",
        Direction::Down => "Open below",
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
/// How far down a paused agent's pane its Paused panel hangs, as a share of the pane.
const PAUSED_AT: f32 = 0.58;
/// A split pane's header, its buttons, and the room after the last.
const PANE_HEADER: f32 = 34.0;
const PANE_BUTTON: f32 = 28.0;
const PANE_HEADER_END: f32 = 6.0;
/// The kind icon in a tab or a pane's header, and the status dot on its corner.
const BADGE_ICON: f32 = 13.0;
const BADGE_DOT: f32 = 6.0;
/// What a command palette row's hover brightens: this name on each row.
const PALETTE_ROW: &str = "palette-row";
/// What an Attention row's hover brightens: this name on each row.
const ATTENTION_ROW: &str = "attention-row";
/// The title bar's `+` and split icon; the search field's width, and the room between it and the
/// split icon, besides the bar's own gap.
const BAR_BUTTON: f32 = 28.0;
const SEARCH: f32 = 220.0;
const SPLIT_GAP: f32 = 4.0;
/// The room between the title bar's tabs and buttons, and at either end of the part after the
/// sidebar; unscaled.
const BAR_GAP: f32 = 4.0;
const BAR_END: f32 = 10.0;
/// A tab's capsule: its height; its title's size; the room before its badge, after it, and after
/// the title (an inactive tab's, its × drawn over the title's end; the active tab's, after its
/// ×); its ×; its ring, drawn clear where it has none so every title sits alike, and the room
/// between capsules, both unscaled; the widest a tab grows, and the least one shortened for room
/// keeps.
const TAB_HEIGHT: f32 = 28.0;
const TAB_TEXT: f32 = 13.0;
const TAB_START: f32 = 6.0;
const TAB_GAP: f32 = 7.0;
const TAB_END: f32 = 12.0;
const ACTIVE_TAB_END: f32 = 6.0;
const TAB_CLOSE: f32 = 16.0;
const TAB_RING: f32 = 1.0;
const TAB_SPACING: f32 = 6.0;
const TAB_WIDEST: f32 = 220.0;
const TAB_LEAST: f32 = 78.0;
/// How strongly the active tab's accent ring, and the amber ring of one with an agent waiting for
/// a person, show.
const RING_ACTIVE: f32 = 0.5;
const RING_WAITING: f32 = 0.6;
/// A paused agent's tab: how strongly it shows, and its pause mark's scale.
const PAUSED_TAB: f32 = 0.55;
const PAUSE_MARK: f32 = 0.7;
/// A split tab's pane count, in a small pill after its title: the pill's height and sides, the
/// room between its icon and count, the count's size and the icon's scale.
const PANES_HEIGHT: f32 = 18.0;
const PANES_X: f32 = 6.0;
const PANES_GAP: f32 = 3.0;
const PANES_TEXT: f32 = 11.0;
const PANES_ICON: f32 = 0.75;
/// The `+N` capsule for the tabs with no room in the bar: its count's size, the room at its ends
/// and before its amber dot and chevron, the dot, and the chevron's scale.
const MORE_TEXT: f32 = 12.5;
const MORE_START: f32 = 12.0;
const MORE_GAP: f32 = 5.0;
const MORE_END: f32 = 10.0;
const MORE_DOT: f32 = 6.0;
const MORE_ICON: f32 = 0.7;
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

/// Where the system's sidebar material shows in a window `window` big under a title bar
/// `bar_height` tall: the left column as wide as the sidebar `sidebar_width`, from the top edge,
/// title bar and all, to the bottom; collapsed, the strip from the title bar's bottom edge, the
/// title bar's row opaque all along; none in full screen, where the window is opaque.
fn frost_column(
    collapsed: bool,
    full_screen: bool,
    sidebar_width: f32,
    bar_height: f32,
    window: Size<Pixels>,
    ui: &UiFont,
) -> Option<Bounds<Pixels>> {
    let (top, width) = if collapsed {
        (bar_height, ui.scale(sidebar::RAIL))
    } else {
        (0.0, sidebar_width)
    };
    (!full_screen).then(|| {
        Bounds::new(
            point(px(0.0), px(top)),
            size(px(width), window.height - px(top)),
        )
    })
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

/// A tab as the title bar draws it: which, its width when the bar is short of room, its title
/// (the group to draw faint, and the rest), and its whole title to show on hover when that is
/// shortened.
struct ShownTab {
    index: usize,
    width: Option<f32>,
    title: (Option<String>, String),
    whole: Option<String>,
}

/// The tabs in the title bar, in order; the ones waiting behind its `+N`, and that button's width;
/// whether one of those has an agent waiting for a person; and each tab's marks.
struct TabsFit {
    shown: Vec<ShownTab>,
    hidden: Vec<usize>,
    more: f32,
    waiting: bool,
    marks: Vec<TabMarks>,
}

/// The last part of a path: `paddock` for `/Users/me/paddock`, `/` for the root.
pub(crate) fn last_part(path: &str) -> &str {
    match path.trim_end_matches('/').rsplit('/').next() {
        Some(part) if !part.is_empty() => part,
        _ => "/",
    }
}

/// An agent's name as the tabs, the split panes' headers and the Changes header show it: the
/// group before the short name (with its `/`, for drawing faint) only when another agent open in
/// the window (`open`) has the same short name, then the short name. A name without a group, or
/// with nothing after it, is shown whole.
pub fn agent_name(name: &str, open: &[String]) -> (Option<String>, String) {
    match name.rsplit_once('/') {
        Some((group, short)) if !group.is_empty() && !short.is_empty() => {
            let clash = open
                .iter()
                .any(|other| other != name && other.rsplit('/').next() == Some(short));
            (clash.then(|| format!("{group}/")), short.to_owned())
        }
        _ => (None, name.to_owned()),
    }
}

/// A tab's title, for the pane it shows, as the group to draw faint and the rest: an agent as
/// [`agent_name`] has it; a shell as `zsh · paddock`.
pub fn tab_label(subject: &Subject, open: &[String]) -> (Option<String>, String) {
    match subject {
        Subject::Agent(name) => agent_name(name, open),
        Subject::Shell { program, cwd } => {
            (None, format!("{} · {}", last_part(program), last_part(cwd)))
        }
        Subject::Command(command) => (None, command.clone()),
        Subject::Empty => (None, "empty".into()),
    }
}

/// A tab's title in one piece, as the command palette lists it.
pub fn tab_title(subject: &Subject, open: &[String]) -> String {
    let (group, name) = tab_label(subject, open);
    group.unwrap_or_default() + &name
}

/// The faint count after a tab's title in the command palette for the panes it holds besides the
/// one named.
pub fn more_panes(panes: usize) -> Option<String> {
    (panes > 1).then(|| format!("+{}", panes - 1))
}

/// The hover text of the small split icon after a split tab's title: how many panes it holds.
pub fn panes_tip(panes: usize) -> Option<String> {
    (panes > 1).then(|| format!("{panes} panes"))
}

/// A name as [`agent_name`] gives it: the group, when there is one, in `faint` and the regular
/// weight, then the name as the text around it is drawn.
pub fn grouped((group, name): (Option<String>, String), faint: Hsla) -> StyledText {
    let lead = group.as_ref().map_or(0, String::len);
    let highlight = (lead > 0).then(|| {
        (
            0..lead,
            HighlightStyle {
                color: Some(faint),
                font_weight: Some(FontWeight::NORMAL),
                ..HighlightStyle::default()
            },
        )
    });
    StyledText::new(group.unwrap_or_default() + &name).with_highlights(highlight)
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

/// A split pane's header, and the Changes header: the name, an agent's as [`agent_name`] has it
/// among the agents `open` in the window, and the directory, short, when there is one.
pub fn pane_name(
    subject: &Subject,
    agent_cwd: Option<&str>,
    home: Option<&str>,
    open: &[String],
) -> ((Option<String>, String), Option<String>) {
    match subject {
        Subject::Agent(name) => (
            agent_name(name, open),
            agent_cwd.map(|cwd| short_dir(cwd, home)),
        ),
        Subject::Shell { program, cwd } => (
            (None, last_part(program).to_owned()),
            Some(short_dir(cwd, home)),
        ),
        Subject::Command(command) => ((None, command.clone()), None),
        Subject::Empty => ((None, "empty".into()), None),
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

/// What a title bar tab's capsule shows of its agents: one of them needs a person (Attention's
/// Needs you), or the agent it shows is paused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TabMarks {
    pub waiting: bool,
    pub paused: bool,
}

/// [`TabMarks`] for each of `workspace`'s tabs: waiting when any of its panes shows an agent
/// `needs` names; paused when the pane it is named after shows an agent `agents` lists paused.
pub fn tab_marks(workspace: &Workspace, agents: &[Agent], needs: &[String]) -> Vec<TabMarks> {
    let agent = |pane: PaneId| match workspace.shown(pane) {
        Shown::Agent(name) => Some(name),
        _ => None,
    };
    workspace
        .tabs
        .iter()
        .map(|tab| TabMarks {
            waiting: tab
                .panes()
                .into_iter()
                .filter_map(agent)
                .any(|name| needs.contains(name)),
            paused: agent(tab.active)
                .is_some_and(|name| agents.iter().any(|a| &a.name == name && a.paused)),
        })
        .collect()
}

/// A pane showing an agent, as checked against corral's listing.
pub struct AgentPane {
    pub pane: PaneId,
    pub name: String,
    /// The instance the pane attached to, when known.
    pub instance: Option<String>,
    /// Its attach is under way or running.
    pub attached: bool,
}

/// The panes to close because their agent has ended, by `agents`, a listing corral has just given:
/// it lists the agent as exited, no longer lists it, or lists another instance under its name
/// (stopped and started again). Only panes whose attach has ended count, so a listing read just
/// before an agent started never closes its new pane. A paused agent has not ended.
pub fn ended_panes(panes: &[AgentPane], agents: &[Agent]) -> Vec<PaneId> {
    panes
        .iter()
        .filter(|p| !p.attached)
        .filter(|p| match agents.iter().find(|a| a.name == p.name) {
            None => true,
            Some(a) => {
                a.state.as_deref() == Some("exited")
                    || p.instance
                        .as_ref()
                        .zip(a.instance.as_ref())
                        .is_some_and(|(shown, listed)| shown != listed)
            }
        })
        .map(|p| p.pane)
        .collect()
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
    /// The New task panel while it is open.
    new_task: Option<Entity<NewTask>>,
    /// The New Agent panel, kept with what was typed while it is closed, until it starts an agent.
    new_agent: Option<Entity<NewAgentView>>,
    /// Where the `+`, the split icon and the panes were last drawn, for the panels that hang from
    /// them.
    spots: Rc<RefCell<HashMap<Spot, Bounds<Pixels>>>>,
    /// The tab under the mouse and since when, and the one it last left and when: their ×
    /// fades in and out.
    tab_hover: Option<(usize, Instant)>,
    tab_left: Option<(usize, Instant)>,
    /// Where the layout is saved.
    store: Store,
    /// Last normal window size, kept while fullscreen or miniaturized.
    window_size: WindowSize,
    window_size_save: WindowSizeSave,
    window_size_save_task: Option<Task<()>>,
    /// The config came from its file at startup, rather than defaults.
    config_from_file: bool,
    /// The command palette while it is open.
    palette: Option<Palette>,
    /// The sidebar's width when expanded, which the title bar leaves empty before the tabs.
    sidebar_width: f32,
    /// The sidebar is collapsed to its strip; saved with the layout.
    collapsed: bool,
    /// The sidebar's activity panel is folded to one line; saved with the layout.
    activity_folded: bool,
    /// The divider is being dragged: the width and the mouse's x when it was pressed.
    resizing: Option<(f32, f32)>,
    /// A seam between split panes is being dragged.
    splitting: Option<Splitting>,
    /// The right sidebar: open or not, its width, its tab; saved with the layout.
    right: RightPanel,
    /// The right sidebar's Changes tab, following the focused pane.
    changes: Entity<ChangesView>,
    /// The right sidebar's Browser tab, this window's web page.
    browser: Entity<BrowserView>,
    /// The right sidebar's Kanban tab, the focused pane's repository's task files.
    kanban: Entity<KanbanView>,
    /// A press on the title bar's empty part: moving now drags the window.
    dragging: bool,
    /// The title bar height the traffic lights were last centred on.
    lights: Option<f32>,
    /// The window is in full screen, where it is opaque: there is nothing behind it to see.
    full_screen: bool,
    /// The system's sidebar material under the left column; none when it could not be put there,
    /// and the window stays opaque.
    frost: Option<Frost>,
    /// What `paddock ctl` asked for and is still under way.
    ctl: ctl::Ctl,
}

#[path = "window_ctl.rs"]
mod ctl;

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
        let activity_folded = saved.as_ref().is_some_and(|layout| layout.activity_folded);
        sidebar.update(cx, |sidebar, cx| {
            sidebar.set_activity_folded(activity_folded, cx)
        });
        let right = RightPanel::new(
            &saved
                .as_ref()
                .map(|layout| layout.right_sidebar.clone())
                .unwrap_or_default(),
        );
        let changes = {
            let (theme, mono) = (theme.clone(), mono_font(&options));
            let (scope, split) = (right.scope, right.split);
            cx.new(|cx| ChangesView::new("git".into(), theme, mono, scope, split, cx))
        };
        cx.subscribe(&changes, |this, changes, _: &changes::Changed, cx| {
            let changes = changes.read(cx);
            (this.right.scope, this.right.split) = (changes.scope, changes.split);
            this.save_layout(cx);
        })
        .detach();
        let browser = {
            let (theme, url) = (theme.clone(), right.url.clone());
            cx.new(|cx| BrowserView::new(theme, url, cx))
        };
        cx.subscribe(&browser, |this, _, visited: &Visited, cx| {
            this.right.url = visited.0.clone();
            this.save_layout(cx);
        })
        .detach();
        cx.subscribe_in(
            &browser,
            window,
            |this, _, handoff: &Handoff, window, cx| match handoff {
                Handoff::Dismiss => {
                    if this.popup.is_some() {
                        this.close_popup(window, cx);
                    }
                }
                Handoff::ToPane => {
                    browser::take_keys(window);
                    this.focus_active(window, cx);
                }
            },
        )
        .detach();
        let kanban = {
            let (theme, mono) = (theme.clone(), mono_font(&options));
            let folded = right.kanban_folded.clone();
            cx.new(|cx| KanbanView::new(theme, mono, folded, cx))
        };
        cx.subscribe_in(&kanban, window, Self::on_kanban).detach();
        let shown = match &options.launch {
            Launch::Empty => Shown::Empty,
            Launch::Agent { name, .. } => Shown::Agent(name.clone()),
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
            new_task: None,
            new_agent: None,
            spots: Rc::default(),
            tab_hover: None,
            tab_left: None,
            palette: None,
            sidebar_width: config.sidebar_width,
            collapsed,
            activity_folded,
            resizing: None,
            splitting: None,
            right,
            changes,
            browser,
            kanban,
            dragging: false,
            lights: None,
            store,
            window_size: WindowSize {
                width: window.bounds().size.width.into(),
                height: window.bounds().size.height.into(),
            },
            window_size_save: WindowSizeSave::default(),
            window_size_save_task: None,
            // It opens windowed (`main.rs`).
            full_screen: false,
            config_from_file: crate::config::default_path().exists(),
            frost: Frost::install(window),
            ctl: ctl::Ctl::default(),
        };
        // The column shows the material through the window, which is see-through for it.
        if this.frost.is_some() {
            window.set_background_appearance(WindowBackgroundAppearance::Transparent);
            this.sidebar
                .update(cx, |sidebar, cx| sidebar.set_frosted(true, cx));
        }
        // Full screen has nothing behind it to show: the window turns opaque there, and back after.
        cx.observe_window_bounds(window, |this, window, cx| {
            let full_screen = window.is_fullscreen();
            if !full_screen
                && !window.is_simple_fullscreen()
                && native_window(window).is_some_and(|native| !native.isMiniaturized())
            {
                let size = window.bounds().size;
                let size = WindowSize {
                    width: size.width.into(),
                    height: size.height.into(),
                };
                if size != this.window_size {
                    this.window_size = size;
                    this.window_size_save.changed(Instant::now());
                    // Replacing the task cancels the previous wait; only the last resize saves.
                    this.window_size_save_task = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(WINDOW_SIZE_SAVE_PAUSE).await;
                        let _ = this.update(cx, |this, cx| {
                            if this.window_size_save.take_due(Instant::now()) {
                                this.save_layout(cx);
                            }
                        });
                    }));
                }
            }
            if full_screen != this.full_screen {
                this.full_screen = full_screen;
                if this.frost.is_some() {
                    window.set_background_appearance(if full_screen {
                        WindowBackgroundAppearance::Opaque
                    } else {
                        WindowBackgroundAppearance::Transparent
                    });
                }
                let frosted = this.frosted();
                this.sidebar
                    .update(cx, |sidebar, cx| sidebar.set_frosted(frosted, cx));
                cx.notify();
            }
        })
        .detach();
        match restored {
            Some(contents) => this.restore(contents, window, cx),
            None => {
                let first = this.workspace.active_pane();
                let view = this.view(first, options.launch, window, cx);
                this.panes.insert(first, view);
            }
        }
        if let Some(problem) = this.store.problem() {
            this.sidebar
                .update(cx, |sidebar, cx| sidebar.note(problem, true, cx));
        }
        if let Some(problem) = this.new_shell.ctl_problem.clone() {
            this.sidebar
                .update(cx, |sidebar, cx| sidebar.note(problem, true, cx));
        }
        this.sync(cx);
        this
    }

    /// Opens the saved panes: shells start afresh in their directories, agents attach again
    /// (panes of agents gone or restarted meanwhile then close), empty panes stay empty.
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
                    env: Vec::new(),
                },
                Content::Empty | Content::Agent { .. } => Launch::Empty,
            };
            let view = self.view(pane, launch, window, cx);
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
            window_size: self.window_size,
            sidebar_collapsed: self.collapsed,
            activity_folded: self.activity_folded,
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

    /// The terminal for `pane`; a shell gets the pane's identity.
    fn view(
        &self,
        pane: PaneId,
        mut launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        if let Launch::Shell { env, .. } = &mut launch {
            *env = self.identity(pane);
        }
        let options = Options {
            launch,
            ..self.template.clone()
        };
        let theme = self.theme.clone();
        let view = cx.new(|cx| TerminalView::new(options, theme, window, cx));
        // corral is asked at once whether the agent ended with it, to close its panes.
        cx.subscribe(&view, |this, _, _: &AttachEnded, cx| {
            this.sidebar.read(cx).refresh()
        })
        .detach();
        view
    }

    fn shell(&self) -> Launch {
        Launch::Shell {
            program: self.new_shell.program.clone(),
            cwd: self.new_shell.cwd.clone(),
            env: Vec::new(),
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

    /// Views for panes the layout made on its own (the shell opened after closing the last tab),
    /// a shell starting where ⌘N starts one.
    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let missing: Vec<PaneId> = self
            .workspace
            .tabs
            .iter()
            .flat_map(|tab| tab.panes())
            .filter(|pane| !self.panes.contains_key(pane))
            .collect();
        for pane in missing {
            let launch = match self.workspace.shown(pane) {
                Shown::Shell => self.shell(),
                Shown::Empty | Shown::Agent(_) => Launch::Empty,
            };
            let view = self.view(pane, launch, window, cx);
            self.panes.insert(pane, view);
        }
    }

    /// The agent panes whose agent has ended, by the listing corral has just given the sidebar.
    fn ended(&self, cx: &gpui::App) -> Vec<PaneId> {
        let panes: Vec<AgentPane> = self
            .workspace
            .agent_panes()
            .into_iter()
            .filter_map(|(pane, name)| {
                let view = self.panes.get(&pane)?.read(cx);
                Some(AgentPane {
                    pane,
                    name,
                    instance: view.agent_metadata().instance,
                    attached: view.attaching(),
                })
            })
            .collect();
        ended_panes(&panes, &self.sidebar.read(cx).agents())
    }

    /// Closes the panes of agents that have ended, as closing them by hand does, and says so in the
    /// footer.
    fn close_ended(&mut self, ended: Vec<PaneId>, window: &mut Window, cx: &mut Context<Self>) {
        let mut names: Vec<String> = ended
            .iter()
            .filter_map(|pane| match self.workspace.shown(*pane) {
                Shown::Agent(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        if names.is_empty() {
            return;
        }
        names.sort();
        names.dedup();
        let gone = ended
            .into_iter()
            .flat_map(|pane| self.workspace.close_pane(pane))
            .collect();
        self.close_quietly(gone, window, cx);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.note(format!("{} ended", names.join(", ")), false, cx)
        });
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
        } else if let Some(view) = self.panes.get(&pane) {
            // The active pane clicked while the Browser has the keyboard (its header too): back
            // to the pane.
            let focus: FocusHandle = view.read(cx).focus_handle(cx);
            if !focus.contains_focused(window, cx) {
                window.focus(&focus, cx);
            }
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
            SidebarEvent::NewAgent => self.open_new_agent(Place::Current, window, cx),
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
            SidebarEvent::Pause(request) => self.pause_agents(request.clone(), window, cx),
            SidebarEvent::Alive(names) => {
                // Asked before the panes let go of agents this listing leaves out: a pane still
                // attaching then is not closed by it (its agent may have only just started).
                let ended = self.ended(cx);
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                for view in self.panes.values() {
                    view.update(cx, |v, _| v.disappeared(&names));
                }
                self.sync_paused(cx);
                self.close_ended(ended, window, cx);
            }
            SidebarEvent::ActivityFolded(folded) => {
                self.activity_folded = *folded;
                self.save_layout(cx);
            }
        }
    }

    /// The Kanban tab's asks: its folds saved with the layout; an agent's pane brought to the
    /// front as a click on its card in the sidebar does, and for its changes the right sidebar
    /// turned to Changes, which follows that pane.
    fn on_kanban(
        &mut self,
        _: &Entity<KanbanView>,
        event: &KanbanEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            KanbanEvent::Folded(folded) => {
                self.right.kanban_folded = folded.clone();
                self.save_layout(cx);
            }
            KanbanEvent::GoTo(name) => {
                let metadata = self.sidebar.read(cx).metadata(name);
                self.show_agent(name, metadata, window, cx);
            }
            KanbanEvent::Changes(name) => {
                let metadata = self.sidebar.read(cx).metadata(name);
                self.show_agent(name, metadata, window, cx);
                self.right.tab = RightTab::Changes;
                self.save_layout(cx);
                cx.notify();
            }
            KanbanEvent::NewTask { repo, anchor } => {
                if let Some(anchor) = anchor {
                    self.spots.borrow_mut().insert(Spot::NewTask, *anchor);
                }
                self.open_new_task(repo.clone(), window, cx);
            }
        }
    }

    /// The New task panel, its title field taking the keys; once the draft is written, it is
    /// opened with the default program, as Open task file opens a task file.
    fn open_new_task(&mut self, repo: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let theme = self.theme.clone();
        let panel = cx.new(|cx| NewTask::new(repo, theme, window, cx));
        cx.subscribe_in(
            &panel,
            window,
            |this, _, event: &NewTaskEvent, window, cx| {
                if let NewTaskEvent::Created(path) = event {
                    cx.open_with_system(path);
                }
                this.close_popup(window, cx);
            },
        )
        .detach();
        self.palette = None;
        self.chooser = Chooser::default();
        self.new_task = Some(panel);
        self.popup = Some(Popup::NewTask);
        cx.notify();
    }

    /// ⇧⌘N and the other ways to a new agent: the New Agent panel over the dimmed window, its
    /// field taking the keys. One closed earlier comes back with what was typed, and takes `place`
    /// unless that is the active pane; one open already only takes the keys back.
    pub fn open_new_agent(&mut self, place: Place, window: &mut Window, cx: &mut Context<Self>) {
        let seed = self.seed(cx);
        let view = match &self.new_agent {
            Some(view) => {
                view.update(cx, |view, cx| view.reopen(seed, place, cx));
                view.clone()
            }
            None => {
                let view = cx.new(|cx| NewAgentView::new(seed, place, cx));
                cx.subscribe_in(
                    &view,
                    window,
                    |this, _, event: &NewAgentEvent, window, cx| {
                        let open = this.popup == Some(Popup::NewAgent);
                        match event {
                            NewAgentEvent::Started {
                                started,
                                cwd,
                                place,
                            } => {
                                this.new_agent = None;
                                if open {
                                    this.popup = None;
                                }
                                this.open_started(started, cwd, *place, window, cx);
                            }
                            NewAgentEvent::Close if open => this.close_popup(window, cx),
                            NewAgentEvent::Close => {}
                        }
                    },
                )
                .detach();
                self.new_agent = Some(view.clone());
                view
            }
        };
        self.palette = None;
        self.chooser = Chooser::default();
        self.new_task = None;
        self.popup = Some(Popup::NewAgent);
        let focus = view.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// The New Agent panel is starting an agent, open or closed.
    fn new_agent_busy(&self, cx: &gpui::App) -> bool {
        self.new_agent
            .as_ref()
            .is_some_and(|view| view.read(cx).busy())
    }

    /// What the New Agent panel starts from: the directories to offer and the active pane's.
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
            names: sidebar.agent_names(),
        }
    }

    /// An agent the New Agent panel started, opened where it said. A running shell in the
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
            let view = self.view(pane, Launch::Empty, window, cx);
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
            Some("corral stop ends the agent and its session. Panes showing it close."),
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

    /// Pauses or resumes the agents with corral in the background, first asking when the request
    /// says, and says in the footer how it went; the list is read again at once.
    fn pause_agents(
        &mut self,
        request: pause::Request,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pause::Request { names, pause, ask } = request;
        if names.is_empty() || self.asking {
            return;
        }
        let answer = ask.map(|ask| {
            self.asking = true;
            window.prompt(PromptLevel::Warning, &ask, None, &["Pause", "Cancel"], cx)
        });
        let corral = self.template.corral.clone();
        cx.spawn_in(window, async move |this, cx| {
            if let Some(answer) = answer {
                let go = matches!(answer.await, Ok(0));
                let _ = this.update(cx, |this, _| this.asking = false);
                if !go {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |sidebar, cx| {
                    sidebar.note(pause::doing(&names, pause), false, cx)
                })
            });
            let results = {
                let names = names.clone();
                cx.background_spawn(async move { pause::run(&corral, &names, pause) })
                    .await
            };
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |sidebar, cx| {
                    let (text, problem) = pause::summary(&results, pause);
                    sidebar.note(text, problem, cx);
                    sidebar.refresh();
                })
            });
        })
        .detach();
    }

    /// The menu's Pause or Resume for the active pane's agent; pausing one in a turn asks first.
    fn pause_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.workspace.active_agent().map(str::to_owned) else {
            return;
        };
        let agents = self.sidebar.read(cx).agents();
        let agent = agents.iter().find(|a| a.name == name);
        let pause = !agent.is_some_and(|a| a.paused);
        let working = agent.is_some_and(|a| a.state.as_deref() == Some("working"));
        let names = vec![name];
        let ask = if pause && working {
            pause::question(&agents, &names)
        } else {
            None
        };
        self.pause_agents(pause::Request { names, pause, ask }, window, cx);
    }

    /// Over a paused agent's pane, a little below the middle: that it is paused, and Resume. The
    /// rest of the pane still takes the mouse, to select and scroll back.
    fn paused_overlay(&self, pane: PaneId, cx: &mut Context<Self>) -> Option<Div> {
        let view = self.panes[&pane].read(cx);
        if !view.paused() {
            return None;
        }
        let name = view.target()?.to_owned();
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let on_resume = cx.listener(move |this, _: &ClickEvent, window, cx| {
            let request = pause::Request {
                names: vec![name.clone()],
                pause: false,
                ask: None,
            };
            this.pause_agents(request, window, cx)
        });
        let accent = self.fg(|t| t.agents_accent);
        let panel = popover::panel(theme, &ui)
            .id(("paused", pane as usize))
            .occlude()
            .flex_row()
            .items_center()
            .gap(ui.px(10.0))
            .pl(ui.px(12.0))
            .child(footer_icon::icon(
                Icon::Pause,
                self.fg(|t| t.agents_dim),
                ui.scale(1.0),
            ))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Paused"))
            .child(
                div()
                    .id(("resume", pane as usize))
                    .h(ui.px(26.0))
                    .px(ui.px(12.0))
                    .flex()
                    .items_center()
                    .rounded(ui.px(6.0))
                    .bg(accent)
                    .text_color(hsla(theme.bg(|t| t.agents_bg), 1.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .child("Resume")
                    .on_click(on_resume),
            );
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .child(div().h(relative(PAUSED_AT)))
                .child(panel),
        )
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
            self.browser
                .update(cx, |browser, cx| browser.set_theme(theme, cx));
        }
        let pet = (config.mascot_enabled, config.mascot);
        if pet != self.pet_setting {
            self.pet_setting = pet;
            self.pet = pet.0.then(|| cx.new(|cx| PetView::new(pet.1, cx)));
        }
        // The New Agent panel follows too, open or closed.
        if let Some(view) = &self.new_agent {
            view.update(cx, |view, cx| view.restyle(config, cx));
        }
        cx.notify();
    }

    /// A shell: in the active pane when it is empty, else in a new tab.
    fn new_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.workspace.active_pane();
        if self.workspace.shown(active) == &Shown::Empty {
            // A fresh pane ID: a process left from a shell that once ran here keeps the old one.
            let pane = self.workspace.renew(active, Shown::Shell);
            let view = self
                .panes
                .remove(&active)
                .expect("the active pane has a view");
            self.panes.insert(pane, view.clone());
            let NewShell { program, cwd, .. } = &self.new_shell;
            let (program, cwd, env) = (program.clone(), cwd.clone(), self.identity(pane));
            view.update(cx, |v, cx| v.start_shell(program, cwd, env, cx));
        } else {
            let pane = self.workspace.new_tab(Shown::Shell);
            let view = self.view(pane, self.shell(), window, cx);
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
            Some(Popup::NewTab | Popup::Split(_) | Popup::Attention | Popup::Overflow)
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
                let view = self.view(pane, Launch::Empty, window, cx);
                self.panes.insert(pane, view);
                self.attach(pane, name, metadata, cx);
            }
        }
        self.focus_active(window, cx);
    }

    /// Attaches `pane` to `name`: a new attach is new content, so the pane's revision rises and
    /// `paddock ctl` requests and close confirmations about what it showed before end.
    fn attach(
        &mut self,
        pane: PaneId,
        name: &str,
        metadata: AgentMetadata,
        cx: &mut Context<Self>,
    ) {
        self.workspace.touch(pane);
        let paused = self.sidebar.read(cx).paused(name);
        let name = name.to_owned();
        self.panes[&pane].update(cx, |v, cx| {
            v.attach(name, metadata, cx);
            v.set_paused(paused, cx);
        });
    }

    /// Tells each pane whether its agent is paused, as corral last said: a paused one takes no
    /// input and offers Resume.
    fn sync_paused(&mut self, cx: &mut Context<Self>) {
        let sidebar = self.sidebar.read(cx);
        let changes: Vec<(Entity<TerminalView>, bool)> = self
            .panes
            .values()
            .filter_map(|view| {
                let pane = view.read(cx);
                let paused = pane.target().is_some_and(|name| sidebar.paused(name));
                (paused != pane.paused()).then(|| (view.clone(), paused))
            })
            .collect();
        if changes.is_empty() {
            return;
        }
        for (view, paused) in changes {
            view.update(cx, |v, cx| v.set_paused(paused, cx));
        }
        cx.notify();
    }

    /// A choice from the new tab or split panel. An agent already open elsewhere moves here.
    fn choose(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let direction = match self.popup {
            Some(Popup::NewTab) => None,
            Some(Popup::Split(direction)) => Some(direction),
            Some(
                Popup::Attention
                | Popup::Palette
                | Popup::Actions
                | Popup::NewTask
                | Popup::Overflow
                | Popup::NewAgent,
            )
            | None => {
                return;
            }
        };
        self.popup = None;
        self.chooser = Chooser::default();
        let (shown, launch) = match &choice {
            Choice::Shell => (Shown::Shell, self.shell()),
            Choice::Agent(name) => (Shown::Agent(name.clone()), Launch::Empty),
            Choice::NewAgent => {
                let place = direction.map_or(Place::Tab, Place::Split);
                self.open_new_agent(place, window, cx);
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
        let view = self.view(pane, launch, window, cx);
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

    /// Whose changes the Changes tab shows: the focused pane's name and dot, and its directory: an
    /// agent's as it was attached with (else as listed), a shell's as it opened in.
    fn follow(&self, agents: &[Agent], now: f64, cx: &Context<Self>) -> Option<Follow> {
        let pane = self.workspace.active_pane();
        if matches!(self.workspace.shown(pane), Shown::Empty) {
            return None;
        }
        let subject = self.subject(pane, cx);
        let cwd = match &subject {
            Subject::Agent(name) => self
                .panes
                .get(&pane)
                .and_then(|view| view.read(cx).agent_metadata().cwd)
                .or_else(|| self.sidebar.read(cx).metadata(name).cwd),
            Subject::Shell { cwd, .. } => Some(cwd.clone()),
            Subject::Command(_) | Subject::Empty => None,
        };
        let ((group, name), _) = pane_name(&subject, None, None, &self.workspace.agents());
        Some(Follow {
            group,
            name,
            dot: self.dot(pane, agents, now),
            cwd,
        })
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
                .and_then(|a| self.kind_icon(a.kind.as_deref()?, ui)),
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
        match icon {
            Some(icon) => self.dotted(icon, (dot, false), ground, hovered, ui),
            None => plain_dot(dot, ui),
        }
    }

    /// An agent's mark in a list's row: the icon of its `kind` with the status dot on the corner,
    /// as [`Self::badge`] draws it; the dot alone for an agent of no known kind.
    fn agent_mark(
        &self,
        kind: Option<&str>,
        dot: Hsla,
        ground: Hsla,
        hovered: Option<(SharedString, Hsla)>,
        ui: &UiFont,
    ) -> AnyElement {
        match kind.and_then(|kind| self.kind_icon(kind, ui)) {
            Some(icon) => self.dotted(icon, (dot, false), ground, hovered, ui),
            None => plain_dot(dot, ui),
        }
    }

    /// The icon of an agent's `kind`, a badge's size: a silhouette in the kind's colour, an
    /// original in its own; `None` for a kind without one.
    fn kind_icon(&self, kind: &str, ui: &UiFont) -> Option<AnyElement> {
        let icon = kind_icon::of(kind)?;
        let color = self.fg(card::brand(kind).color).opacity(0.85);
        Some(icon.render(ui.px(BADGE_ICON), color))
    }

    /// `icon` with the status dot `(colour, breathing)` on its lower right corner, ringed in
    /// `ground` (`hovered` while the group is hovered) to stand off the icon.
    fn dotted(
        &self,
        icon: AnyElement,
        (dot, breathing): (Hsla, bool),
        ground: Hsla,
        hovered: Option<(SharedString, Hsla)>,
        ui: &UiFont,
    ) -> AnyElement {
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
        let mark = if breathing {
            let reach = ui.scale(3.0);
            mark.with_animation(
                "breath",
                Animation::new(Duration::from_millis(1600))
                    .repeat_synced()
                    .with_max_fps(30.0),
                move |mark, delta| {
                    let out = ease_in_out(if delta < 0.5 {
                        delta * 2.0
                    } else {
                        (1.0 - delta) * 2.0
                    });
                    mark.shadow(vec![BoxShadow {
                        color: dot.opacity(0.55 * (1.0 - out)),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(reach * out),
                        inset: false,
                    }])
                },
            )
            .into_any_element()
        } else {
            mark.into_any_element()
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

    /// The left column shows the system's sidebar material: it is there, and not in full screen.
    fn frosted(&self) -> bool {
        self.frost.is_some() && !self.full_screen
    }

    /// What lies under everything while the column shows the material: the sidebar's colour,
    /// faintly, over the column, and opaque elsewhere, as under the cards and the rest of the
    /// title bar; over the strip the title bar's whole row is opaque, one piece.
    fn frosted_ground(&self, column: Bounds<Pixels>) -> Div {
        let ground = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .absolute()
                    .left(column.left())
                    .top(column.top())
                    .w(column.size.width)
                    .h(column.size.height)
                    .bg(ground.opacity(self.theme.frost().wash)),
            )
            .when(column.top() > px(0.0), |under| {
                under.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .right_0()
                        .h(column.top())
                        .bg(ground),
                )
            })
            .child(
                div()
                    .absolute()
                    .left(column.right())
                    .top(column.top())
                    .right_0()
                    .bottom_0()
                    .bg(ground),
            )
    }

    /// Puts the material under the column GPUI is drawing now, or takes it away; what it gives up
    /// waits for the next frame, once this one is drawn (see [`Frost::fit`]). Its look follows the
    /// theme: dark under a dark sidebar.
    fn fit_frost(
        &mut self,
        column: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dark = hsla(self.theme.bg(|t| t.agents_bg), 1.0).l < 0.5;
        let Some(frost) = &mut self.frost else { return };
        frost.set_dark(dark);
        let shape = column.map(|column| Shape {
            top: f32::from(column.top()),
            width: f32::from(column.size.width),
        });
        if frost.fit(shape) {
            cx.on_next_frame(window, |this, _, _| {
                if let Some(frost) = &mut this.frost {
                    frost.settle();
                }
            });
        }
    }

    /// An empty layer that puts the Browser's page where this frame leaves room for it, `open`
    /// saying whether the right sidebar is, `dragging` whether a divider is being dragged. It does
    /// so as it is painted: by then everything drawn over the window, hover texts last, has noted
    /// itself with [`cover`].
    fn place_page(&self, open: bool, dragging: bool) -> impl IntoElement {
        let browser = self.browser.clone();
        let on_browser = self.right.tab == RightTab::Browser;
        let dialog = self.popup.is_some();
        canvas(
            |_, _, _| {},
            move |_, _, window, cx| {
                browser.update(cx, |browser, cx| {
                    browser.place(open, on_browser, dragging, dialog, window, cx)
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }

    /// How the tabs fit the title bar as the window is now (see `tab_fit`), with the `+N` button's
    /// width when some wait behind it.
    fn fit_tabs(&self, full_screen: bool, window: &Window, cx: &Context<Self>) -> TabsFit {
        let ui = UiFont::get(cx);
        let s = ui.scale(1.0);
        let open = self.workspace.agents();
        let subjects: Vec<Subject> = self
            .workspace
            .tabs
            .iter()
            .map(|tab| self.subject(tab.active, cx))
            .collect();
        let family = ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into());
        let width = |text: &str, size: f32, weight: FontWeight| {
            if text.is_empty() {
                return 0.0;
            }
            let run = TextRun {
                len: text.len(),
                font: Font {
                    weight,
                    ..gpui::font(family.clone())
                },
                color: Hsla::default(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line =
                window
                    .text_system()
                    .shape_line(text.to_owned().into(), ui.px(size), &[run], None);
            f32::from(line.width)
        };
        let titles: Vec<(Option<String>, String)> = subjects
            .iter()
            .map(|subject| tab_label(subject, &open))
            .collect();
        let short = tab_fit::distinct(&subjects, &open);
        let marks = self.tab_marks(cx);
        let active = self.workspace.active_tab;
        let sizes: Vec<TabSize> = self
            .workspace
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let (group, name) = &titles[index];
                let weight = if index == active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                };
                let badge = match self.workspace.shown(tab.active) {
                    Shown::Empty => 7.0,
                    _ => BADGE_ICON + 2.0,
                };
                let panes = match tab.panes().len() {
                    1 => 0.0,
                    count => {
                        width(&count.to_string(), PANES_TEXT, FontWeight::NORMAL)
                            + (TAB_GAP + 2.0 * PANES_X + footer_icon::SIZE * PANES_ICON + PANES_GAP)
                                * s
                    }
                };
                let pause = if marks[index].paused {
                    TAB_GAP + footer_icon::SIZE * PAUSE_MARK
                } else {
                    0.0
                };
                let end = if index == active {
                    TAB_GAP + TAB_CLOSE + ACTIVE_TAB_END
                } else {
                    TAB_END
                };
                let group = group.as_deref().unwrap_or_default();
                TabSize {
                    chrome: (TAB_START + badge + TAB_GAP + pause + end) * s
                        + panes
                        + 2.0 * TAB_RING,
                    full: width(group, TAB_TEXT, FontWeight::NORMAL)
                        + width(name, TAB_TEXT, weight),
                    short: width(&short[index], TAB_TEXT, weight),
                }
            })
            .collect();
        let count = format!("+{}", sizes.len().saturating_sub(1));
        let more = width(&count, MORE_TEXT, FontWeight::NORMAL)
            + (MORE_START + MORE_GAP + footer_icon::SIZE * MORE_ICON + MORE_END) * s;
        // Room for the amber dot, kept while any tab that may go behind the `+N` waits.
        let dot = (MORE_GAP + MORE_DOT) * s;
        let waiting = |tab: &usize| marks[*tab].waiting;
        let may_wait = (0..sizes.len())
            .filter(|&tab| tab != active)
            .any(|tab| waiting(&tab));
        let room = f32::from(window.viewport_size().width)
            - bar_left(self.collapsed, full_screen, self.sidebar_width, &ui)
            - 2.0 * BAR_END
            // Between the tabs, the `+`, the pet's room, the split icon, Search and the right
            // sidebar's switch; the pet's room keeps a little more.
            - 5.0 * BAR_GAP
            - GAP
            - (3.0 * BAR_BUTTON + 2.0 * SPLIT_GAP + SEARCH) * s;
        let bar = Bar {
            room,
            gap: TAB_SPACING,
            widest: TAB_WIDEST * s,
            least: TAB_LEAST * s,
            more: if may_wait { more + dot } else { more },
        };
        let fit = tab_fit::fit(&sizes, active, &bar);
        let shown = fit
            .shown
            .iter()
            .map(|slot| {
                let index = slot.tab;
                let size = &sizes[index];
                let (title, cut) = match slot.title {
                    Title::Full => {
                        let cut = size.chrome + size.full > slot.width + 0.5;
                        (titles[index].clone(), cut)
                    }
                    Title::Short => ((None, short[index].clone()), true),
                    Title::Squeezed(room) => {
                        let text = tab_fit::middle(&short[index], room, |text| {
                            width(text, TAB_TEXT, FontWeight::NORMAL)
                        });
                        ((None, text), true)
                    }
                };
                ShownTab {
                    index,
                    width: (!fit.roomy).then_some(slot.width),
                    title,
                    whole: cut.then(|| tab_title(&subjects[index], &open)),
                }
            })
            .collect();
        let waiting = fit.hidden.iter().any(waiting);
        TabsFit {
            shown,
            hidden: fit.hidden,
            more: if waiting { more + dot } else { more },
            waiting,
            marks,
        }
    }

    /// Each tab's marks, as corral last listed the agents and Attention has them.
    fn tab_marks(&self, cx: &Context<Self>) -> Vec<TabMarks> {
        let sidebar = self.sidebar.read(cx);
        let needs: Vec<String> = sidebar
            .attention()
            .into_iter()
            .filter(|item| item.needs())
            .filter_map(|item| item.agent)
            .collect();
        tab_marks(&self.workspace, &sidebar.agents(), &needs)
    }

    /// A title bar capsule's grounds, each a step brighter than the bar's: at rest, under the
    /// mouse (or lit), and the active tab's.
    fn capsule_grounds(&self) -> (Hsla, Hsla, Hsla) {
        let bar = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let highlight = self.highlight();
        (
            bar.blend(highlight.opacity(0.7)),
            highlight,
            highlight.blend(self.fg(|t| t.agents_text).opacity(0.07)),
        )
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let scale = ui.scale(1.0);
        let highlight = self.highlight();
        // Each capsule's ground, solid, so the × drawn over an inactive one's title can hide the
        // text: at rest, under the mouse, and the active tab's.
        let (rest, lit, chosen) = self.capsule_grounds();
        let muted = self.theme.fg(|t| t.muted);
        let dim = self.fg(|t| t.agents_dim);
        let shown_at = Instant::now();
        let fit = self.fit_tabs(full_screen, window, cx);
        // A press on a tab or a button is theirs, not the start of a drag.
        let keep = |_: &MouseDownEvent, _: &mut Window, cx: &mut gpui::App| cx.stop_propagation();
        let mut tabs = div()
            .id("tabs")
            .flex()
            .items_center()
            .gap(px(TAB_SPACING))
            .min_w(px(0.0))
            .flex_shrink(1.0)
            .h_full()
            .overflow_x_scroll()
            .track_scroll(&self.tabs);
        for shown in fit.shown {
            let index = shown.index;
            let tab = &self.workspace.tabs[index];
            let active = index == self.workspace.active_tab;
            let marks = fit.marks[index];
            // The active tab keeps its × in line; the others have it over the end of their
            // title, so a narrow tab gives the title all its room. The active one's always shows;
            // the others' fade in only while the mouse is on the tab.
            let under = if active { chosen } else { lit };
            let close = div()
                .id(("close-tab", index))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(TAB_CLOSE))
                .rounded_full()
                .bg(under)
                .opacity(if active {
                    CLOSE_SHOWN
                } else {
                    self.close_shown(index, shown_at)
                })
                .hover(move |style| style.bg(under.blend(hsla(muted, 0.18))))
                .when(!active, |close| {
                    // Centred in the capsule, inside its ring.
                    let inset = px(ui.scale((TAB_HEIGHT - TAB_CLOSE) / 2.0) - TAB_RING);
                    close.absolute().top(inset).right(inset)
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
            // The accent round the active tab; amber round another with an agent waiting for a
            // person, the active one's dot and its panes' headers telling that instead.
            let ring = if active {
                self.fg(|t| t.agents_accent).opacity(RING_ACTIVE)
            } else if marks.waiting {
                self.fg(|t| t.agents_yellow).opacity(RING_WAITING)
            } else {
                gpui::transparent_black()
            };
            let mut item = div()
                .id(("tab", index))
                .group(SharedString::from(format!("tab-{index}")))
                .relative()
                .max_w(ui.px(TAB_WIDEST))
                .flex()
                .items_center()
                .gap(ui.px(TAB_GAP))
                .h(ui.px(TAB_HEIGHT))
                .pl(ui.px(TAB_START))
                .rounded_full()
                .border(px(TAB_RING))
                .border_color(ring)
                .text_size(ui.px(TAB_TEXT))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, keep)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    this.hover_tab(index, *hovered, cx)
                }))
                .child(if active {
                    self.badge(tab.active, agents, now, chosen, None, &ui)
                } else {
                    let group = SharedString::from(format!("tab-{index}"));
                    self.badge(tab.active, agents, now, rest, Some((group, lit)), &ui)
                })
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(grouped(shown.title, self.fg(|t| t.agents_dimmer))),
                )
                // A paused agent's tab, faint, with the pause mark after its title.
                .when(marks.paused, |item| {
                    item.opacity(PAUSED_TAB).child(footer_icon::icon(
                        Icon::Pause,
                        dim,
                        scale * PAUSE_MARK,
                    ))
                })
                .children(panes_tip(tab.panes().len()).map(|text| {
                    let tip = BarTip {
                        text: text.into(),
                        keys: String::new(),
                        size: ui.px(11.5),
                        color: self.fg(|t| t.agents_text),
                        dim,
                        background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
                        border: self.fg(|t| t.agents_rule),
                    };
                    div()
                        .id(("tab-panes", index))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(ui.px(PANES_GAP))
                        .h(ui.px(PANES_HEIGHT))
                        .px(ui.px(PANES_X))
                        .rounded_full()
                        .bg(self.fg(|t| t.agents_text).opacity(0.08))
                        .text_size(ui.px(PANES_TEXT))
                        .text_color(dim)
                        .font_weight(FontWeight::NORMAL)
                        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                        .child(footer_icon::icon(Icon::Panes, dim, scale * PANES_ICON))
                        .child(tab.panes().len().to_string())
                }));
            // Short of room, the other tabs narrow first; the active one keeps its title.
            item = if active {
                item.flex_shrink_0()
                    .pr(ui.px(ACTIVE_TAB_END))
                    .bg(chosen)
                    .text_color(self.fg(|t| t.agents_text))
                    // The short name heavier; a faint group before it stays regular.
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                item.flex_shrink(1.0)
                    .min_w(ui.px(56.0))
                    .pr(ui.px(TAB_END))
                    .bg(rest)
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(lit))
            };
            // Fitted to the room left, as `fit_tabs` worked it out; the whole title on hover
            // when it is shortened.
            if let Some(width) = shown.width {
                item = item.flex_shrink_0().w(px(width));
            }
            if let Some(whole) = shown.whole {
                let tip = BarTip {
                    text: whole.into(),
                    keys: String::new(),
                    size: ui.px(11.5),
                    color: self.fg(|t| t.agents_text),
                    dim,
                    background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
                    border: self.fg(|t| t.agents_rule),
                };
                item = item.tooltip(move |_, cx| cx.new(|_| tip.clone()).into());
            }
            tabs = tabs.child(item.child(close).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| this.select_tab(index, window, cx),
            )));
        }
        if !fit.hidden.is_empty() {
            tabs = tabs.child(self.more_button(fit.hidden.len(), fit.more, fit.waiting, cx));
        }
        // A round button, lit while its panel is open, which hangs from it; the `+` turns as the
        // pointer comes in.
        let choosing = self.popup == Some(Popup::NewTab);
        let color = if choosing {
            self.fg(|t| t.agents_text)
        } else {
            self.fg(|t| t.muted)
        };
        let spot = self.spot(Spot::NewTab).into_any_element();
        let on_click =
            cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_new_tab(window, cx));
        let button_size = ui.px(BAR_BUTTON);
        let new_tab = motion::hover_motion("new-tab-motion", motion::SPIN, move |hover| {
            div()
                .id("new-tab")
                .relative()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(button_size)
                .rounded_full()
                .cursor_pointer()
                .when(choosing, |button| button.bg(highlight))
                .hover(move |style| style.bg(highlight))
                .on_mouse_down(MouseButton::Left, keep)
                .child(footer_icon::posed(
                    Icon::Plus,
                    motion::spin(hover.play),
                    color,
                    scale,
                ))
                .child(spot)
                .on_click(on_click)
        });
        // Over the sidebar: the traffic lights, then the sidebar's header row (the expand button
        // alone over the strip), with room to drag by.
        let spot = self.spot(Spot::Bell).into_any_element();
        let head = self.sidebar.update(cx, |sidebar, cx| {
            sidebar.head(full_screen, spot, window, cx)
        });
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
                    .gap(px(BAR_GAP))
                    .pl(px(BAR_END))
                    .pr(px(BAR_END))
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

    /// The `+N` capsule after the tabs for the `count` with no room in the title bar, `width`
    /// wide, with an amber dot when one of them has an agent `waiting` for a person; lit while
    /// their menu, hanging from it, is open.
    fn more_button(
        &self,
        count: usize,
        width: f32,
        waiting: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let (rest, lit_ground, _) = self.capsule_grounds();
        let lit = self.popup == Some(Popup::Overflow);
        let color = if lit {
            self.fg(|t| t.agents_text)
        } else {
            self.fg(|t| t.muted)
        };
        div()
            .id("tabs-more")
            .relative()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .gap(ui.px(MORE_GAP))
            .w(px(width))
            .h(ui.px(TAB_HEIGHT))
            .rounded_full()
            .bg(if lit { lit_ground } else { rest })
            .text_size(ui.px(MORE_TEXT))
            .text_color(color)
            .cursor_pointer()
            .hover(move |style| style.bg(lit_ground))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(format!("+{count}"))
            .when(waiting, |button| {
                button.child(
                    div()
                        .flex_shrink_0()
                        .size(ui.px(MORE_DOT))
                        .rounded_full()
                        .bg(self.fg(|t| t.agents_yellow)),
                )
            })
            .child(footer_icon::icon(Icon::Down, color, ui.scale(MORE_ICON)))
            .child(self.spot(Spot::Overflow))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.popup = match this.popup {
                    Some(Popup::Overflow) => None,
                    _ => Some(Popup::Overflow),
                };
                cx.notify();
            }))
    }

    /// The `+N` menu, under it: the tabs with no room in the title bar (`hidden`), each its badge
    /// and its whole title; a click goes to that tab, and a click outside closes it.
    fn overflow_menu(
        &self,
        hidden: &[usize],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let placed = self.placed(Popup::Overflow, 240.0, window, cx);
        let agents = self.sidebar.read(cx).agents();
        let now = now();
        let open = self.workspace.agents();
        // The badge's dot ringed in the row's ground, as the row lights under the mouse.
        let ground = popover::ground(theme);
        let hovered = ground.blend(popover::lit(theme).opacity(0.6));
        let mut panel = popover::panel(theme, &ui)
            .id("overflow-menu")
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .overflow_y_scroll()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        for &index in hidden {
            let tab = &self.workspace.tabs[index];
            let group = SharedString::from(popover::ROW_GROUP);
            let badge = self.badge(
                tab.active,
                &agents,
                now,
                ground,
                Some((group, hovered)),
                &ui,
            );
            let title = tab_label(&self.subject(tab.active, cx), &open);
            panel = panel.child(
                popover::lead_row(theme, &ui, ("overflow-tab", index), badge, false)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(grouped(title, self.fg(|t| t.agents_dimmer))),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.close_popup(window, cx);
                        this.select_tab(index, window, cx);
                    })),
            );
        }
        // A click outside closes it; nothing is dimmed, as for a menu.
        div()
            .id("overflow-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(panel.child(cover()))
    }

    /// The right sidebar's switch, after the search field at the title bar's right end; lit while
    /// the sidebar is open. As the pointer comes in its edge slides the way the click goes, out
    /// to close and in to open.
    fn right_button(&self, cx: &mut Context<Self>) -> HoverMotion {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        let lit = self.right.open;
        let tip = BarTip {
            text: "Toggle right sidebar".into(),
            keys: menu::keys(&menu::ToggleRightSidebar).concat(),
            size: ui.px(11.5),
            color: self.fg(|t| t.agents_text),
            dim: self.fg(|t| t.agents_dim),
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: self.fg(|t| t.agents_rule),
        };
        let color = if lit {
            self.fg(|t| t.agents_text)
        } else {
            self.fg(|t| t.muted)
        };
        let on_click = cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_right(cx));
        motion::hover_motion("right-sidebar-motion", motion::SHIFT, move |hover| {
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
                .child(footer_icon::posed(
                    Icon::RightSidebar,
                    // Open, the click closes it: out, to the right.
                    motion::shift(hover.play, lit),
                    color,
                    ui.scale(1.0),
                ))
                .on_click(on_click)
        })
    }

    /// The way into the split panel for the active pane, always before the search field, as a
    /// lone pane has no header to carry its split button. Quiet until hovered; lit while the
    /// panel hangs from it. Its halves part as the pointer comes in.
    fn split_button(&self, cx: &mut Context<Self>) -> HoverMotion {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        let lit = self.split_hanging() == Some(SplitFrom::Bar);
        let tip = BarTip {
            text: "Split pane".into(),
            keys: menu::keys(&menu::SplitRight).concat(),
            size: ui.px(11.5),
            color: self.fg(|t| t.agents_text),
            dim: self.fg(|t| t.agents_dim),
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: self.fg(|t| t.agents_rule),
        };
        let color = if lit {
            self.fg(|t| t.agents_text)
        } else {
            self.fg(|t| t.muted)
        };
        let spot = self.spot(Spot::Split).into_any_element();
        let on_click = cx
            .listener(|this, _: &ClickEvent, window, cx| this.ask_split(SplitAsk::Bar, window, cx));
        motion::hover_motion("split-pane-motion", motion::PART, move |hover| {
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
                .child(footer_icon::posed(
                    Icon::Split,
                    motion::part(hover.play),
                    color,
                    ui.scale(1.0),
                ))
                .child(spot)
                .on_click(on_click)
        })
    }

    /// The way into the command palette, always at the title bar's right end: the tabs narrow
    /// before it does. Its magnifier tilts as the pointer comes in.
    fn search_button(&self, cx: &mut Context<Self>) -> HoverMotion {
        let ui = UiFont::get(cx);
        let highlight = self.highlight();
        let border = hsla(self.theme.fg(|t| t.agents_rule), 0.6);
        let dim = self.fg(|t| t.agents_dim);
        let keycaps = self.keycaps(&menu::keys(&menu::Search), &ui, 18.0);
        let on_click =
            cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_palette("", window, cx));
        motion::hover_motion("search-motion", motion::TILT, move |hover| {
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
                .border_color(border)
                .bg(highlight.opacity(0.5))
                .hover(move |style| style.bg(highlight))
                .text_size(ui.px(12.5))
                .text_color(dim)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(footer_icon::posed(
                    Icon::Search,
                    motion::tilt(hover.play),
                    dim,
                    ui.scale(13.0 / footer_icon::SIZE),
                ))
                .child(div().flex_1().child("Search"))
                .child(keycaps)
                .on_click(on_click)
        })
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
                ratio,
                first,
                second,
            } => {
                // Each pane its own card, a seam of the frame between, which shares the split out
                // by its ratio.
                let axis = *axis;
                let after = second.first_pane();
                let part = |node: AnyElement, share: f32| {
                    let part = div()
                        .flex()
                        .flex_basis(px(0.0))
                        .flex_grow(share)
                        .min_w(px(0.0))
                        .min_h(px(0.0));
                    match axis {
                        Axis::Row => part.flex_row(),
                        Axis::Column => part.flex_col(),
                    }
                    .child(node)
                };
                let split = div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    // Where the split is, for its seam to know how far the mouse moves it.
                    .child(self.spot(Spot::Seam(after)));
                match axis {
                    Axis::Row => split.flex_row(),
                    Axis::Column => split.flex_col(),
                }
                .child(part(self.node(first, shown, agents, cx), *ratio))
                .child(self.seam(after, axis, cx))
                .child(part(self.node(second, shown, agents, cx), 1.0 - ratio))
                .into_any_element()
            }
        }
    }

    /// The seam before the pane `after` in a split along `axis`: taken by the mouse it shares the
    /// split out, and a line in it lights up under the mouse and while dragged; a double click
    /// splits it in half again.
    fn seam(&self, after: PaneId, axis: Axis, cx: &mut Context<Self>) -> Stateful<Div> {
        let accent = self.fg(|t| t.agents_accent);
        let group = SharedString::from(format!("seam-{after}"));
        let held = self.splitting.as_ref().is_some_and(|s| s.after == after);
        let across = (CARD_GAP - DIVIDER) / 2.0 - 1.0;
        let line = div().absolute();
        let line = match axis {
            Axis::Row => line
                .top_0()
                .bottom_0()
                .left(px(across))
                .w(px(DIVIDER + 2.0)),
            Axis::Column => line.left_0().right_0().top(px(across)).h(px(DIVIDER + 2.0)),
        }
        .when(held, |line| line.bg(accent))
        .group_hover(group.clone(), move |style| style.bg(accent));
        let seam = div()
            .id(("seam", after as usize))
            .group(group)
            .relative()
            .flex_shrink_0();
        match axis {
            Axis::Row => seam.w(px(CARD_GAP)).cursor_col_resize(),
            Axis::Column => seam.h(px(CARD_GAP)).cursor_row_resize(),
        }
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                if event.click_count >= 2 {
                    this.splitting = None;
                    if this.workspace.even(after) {
                        this.save_layout(cx);
                    }
                } else {
                    this.press_seam(after, axis, event.position, window, cx);
                }
                cx.notify();
            }),
        )
        .on_drag(SplitDrag, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
        .child(line)
    }

    /// A seam was pressed: how far it may go is worked out now, from where its split was painted.
    fn press_seam(
        &mut self,
        after: PaneId,
        axis: Axis,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.least_pane(window, cx);
        let Some(bounds) = self.spots.borrow().get(&Spot::Seam(after)).copied() else {
            return;
        };
        let Some(Node::Split {
            ratio,
            first,
            second,
            ..
        }) = self.workspace.tab().root.split_before(after)
        else {
            return;
        };
        let (grab, length) = match axis {
            Axis::Row => (at.x, bounds.size.width),
            Axis::Column => (at.y, bounds.size.height),
        };
        self.splitting = Some(Splitting {
            after,
            axis,
            from: *ratio,
            grab: f32::from(grab),
            length: f32::from(length),
            least: (
                first.least(axis, pane, CARD_GAP),
                second.least(axis, pane, CARD_GAP),
            ),
        });
    }

    /// The seam moved: its split follows at once, each part keeping its least size.
    fn drag_seam(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(s) = &self.splitting else {
            return;
        };
        let along = f32::from(match s.axis {
            Axis::Row => at.x,
            Axis::Column => at.y,
        });
        let wanted = s.from + (along - s.grab) / (s.length - CARD_GAP);
        let ratio = layout::clamp_ratio(wanted, s.length, CARD_GAP, s.least);
        if self.workspace.set_ratio(s.after, ratio) {
            cx.notify();
        }
    }

    /// The seam was let go: the split's share is saved with the layout.
    fn finish_seam(&mut self, cx: &mut Context<Self>) {
        if self.splitting.take().is_some() {
            self.save_layout(cx);
            cx.notify();
        }
    }

    /// The least width and height of a split pane's card: 20 columns by 5 rows of the terminal
    /// font, with the room `pane` leaves around the terminal under a header.
    fn least_pane(&self, window: &mut Window, cx: &mut Context<Self>) -> (f32, f32) {
        let ui = UiFont::get(cx);
        let font_size = self.template.font_size;
        let text = window.text_system();
        let cell = text
            .advance(
                text.resolve_font(&mono_font(&self.template)),
                px(font_size),
                'm',
            )
            .map_or(font_size * 0.6, |advance| f32::from(advance.width));
        let line = (font_size * self.template.line_height).round();
        // Beside the terminal, the card's 10 and the view's 6 on each side; above and below it,
        // the header, the card's 6 under it and the view's 6 on each side.
        (
            20.0 * cell + 2.0 * (10.0 + 6.0),
            5.0 * line + ui.scale(PANE_HEADER) + 6.0 + 2.0 * 6.0,
        )
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
            .children(self.paused_overlay(pane, cx))
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
            &self.workspace.agents(),
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
                    .child(grouped(name, self.fg(|t| t.agents_dimmer))),
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
            let agent = item
                .agent
                .as_ref()
                .and_then(|name| agents.iter().find(|a| &a.name == name));
            // Since the agent came to this state, when corral says.
            let age = agent
                .and_then(|agent| agent.state_started)
                .map(|since| card::short_time(Some(now - since)));
            // Its kind and status before its name, ringed in the row's ground.
            let (ground, hovered) = if index == selected {
                (lit, None)
            } else {
                let ground = popover::ground(theme);
                let hovered = ground.blend(lit.opacity(0.6));
                (ground, Some((SharedString::from(ATTENTION_ROW), hovered)))
            };
            let badge = agent.map(|agent| {
                let color = self.fg(dot(&Shown::Agent(agent.name.clone()), &agents, now));
                let mark = self.agent_mark(agent.kind.as_deref(), color, ground, hovered, &ui);
                div().flex_shrink_0().self_center().child(mark)
            });
            let first = div()
                .flex()
                .items_baseline()
                .gap(ui.px(6.0))
                .children(badge)
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
                .group(ATTENTION_ROW)
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
            .child(list)
            .child(cover());
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
    /// list's order and folding, pausing (or resuming) and stopping the active pane's agent, and
    /// Settings. Choosing closes it, as do Esc and a click outside.
    fn actions_menu(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let (fold, by_name) = self.sidebar.read(cx).view_state();
        let (stop, can_stop) = sidebar::stop_item(self.workspace.active_agent());
        let paused = self
            .workspace
            .active_agent()
            .is_some_and(|name| self.sidebar.read(cx).paused(name));
        let (pause, can_pause) = sidebar::pause_item(self.workspace.active_agent(), paused);
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
        let mut pause = popover::row(
            theme,
            &ui,
            "menu-pause",
            if paused { Icon::Resume } else { Icon::Pause },
            pause,
            if can_pause { Tone::Plain } else { Tone::Off },
        );
        if can_pause {
            pause = pause.on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.close_popup(window, cx);
                this.pause_active(window, cx);
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
                    this.open_new_agent(Place::Current, window, cx);
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
            .child(pause)
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
            )
            .child(cover());
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
                    kind: None,
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

    /// Closes the open chooser, list or palette and gives the keys back to the active pane, or to
    /// the Browser's page when it had them as the popup opened.
    fn close_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.palette = None;
        self.chooser = Chooser::default();
        self.new_task = None;
        if self.browser.read(cx).owner() == Owner::Page {
            self.browser
                .update(cx, |browser, cx| browser.give_back(window, cx));
            cx.notify();
        } else {
            self.focus_active(window, cx);
        }
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
            // It dims the whole window, the Browser's page and all.
            .child(cover())
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
        // An agent's dot sits on its kind's icon, ringed in the row's ground.
        let ground = if selected {
            highlight
        } else {
            hsla(self.theme.bg(|t| t.agents_bg), 1.0)
        };
        let hovered = (!selected).then(|| {
            let hovered = ground.blend(highlight.opacity(0.5));
            (SharedString::from(PALETTE_ROW), hovered)
        });
        let lead = match row.lead {
            Lead::Dot {
                color,
                breathing,
                kind,
            } => Some(match kind.and_then(|kind| self.kind_icon(&kind, ui)) {
                Some(icon) => self.dotted(icon, (self.fg(color), breathing), ground, hovered, ui),
                None => status_dot(self.fg(color), breathing, ui),
            }),
            Lead::Settings(page) => Some(
                footer_icon::icon(page.icon(), self.fg(|t| t.agents_dim), scale).into_any_element(),
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
            .group(PALETTE_ROW)
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
            Popup::Overflow => (
                // Not drawn yet: where the first tab starts.
                spots
                    .get(&Spot::Overflow)
                    .copied()
                    .unwrap_or_else(|| rect(side + BAR_END, 0.0, 0.0, title)),
                Hang::BelowLeft,
            ),
            Popup::NewTask => (
                // Not drawn yet: at the right end of the right sidebar's first row.
                spots.get(&Spot::NewTask).copied().unwrap_or_else(|| {
                    let button = 22.0 * s;
                    let right = f32::from(viewport.width) - 14.0 * s;
                    rect(right - button, title + 40.0 * s, button, button)
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
            .child(popover::heading(theme, &ui, side.map_or("New", open_where)));
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
                    let color = self.fg(dot(&Shown::Agent(name.clone()), &agents, now));
                    let kind = agents
                        .iter()
                        .find(|a| &a.name == name)
                        .and_then(|a| a.kind.as_deref());
                    let (ground, hovered) = if lit {
                        (popover::lit(theme), None)
                    } else {
                        let ground = popover::ground(theme);
                        let hovered = ground.blend(popover::lit(theme).opacity(0.6));
                        (ground, Some((popover::ROW_GROUP.into(), hovered)))
                    };
                    let dot = self.agent_mark(kind, color, ground, hovered, &ui);
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
                list = list.child(popover::heading(theme, &ui, "Agents").pt(ui.px(10.0)));
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
                .child(popover::heading(theme, &ui, "Split").pb(ui.px(6.0)))
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
        panel = panel
            .child(list)
            .when(side.is_none(), |panel| {
                panel.child(popover::hints(
                    theme,
                    &ui,
                    &[("↑↓", "move"), ("↩", "open"), ("esc", "close")],
                ))
            })
            .child(cover());
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

    /// The Kanban tab's New task panel, under its button; Esc or a click outside closes it.
    fn new_task_panel(
        &self,
        panel: Entity<NewTask>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let placed = self.placed(Popup::NewTask, kanban_view::NEW_TASK_WIDTH, window, cx);
        let panel = div()
            .id("new-task-place")
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(panel)
            .child(cover());
        // A click outside closes it; nothing is dimmed, as for the other panels.
        div()
            .id("new-task-backdrop")
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

/// A title bar hover text, with the shortcut after it when there is one.
#[derive(Clone)]
struct BarTip {
    text: SharedString,
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
            .relative()
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
            .child(self.text.clone())
            .when(!self.keys.is_empty(), |tip| {
                tip.child(div().text_color(self.dim).child(self.keys.clone()))
            })
            .child(cover())
    }
}

/// The status dot's colour for what a pane shows.
fn dot(shown: &Shown, agents: &[Agent], now: f64) -> Pick {
    match shown {
        Shown::Agent(name) => match agents.iter().find(|a| &a.name == name) {
            Some(agent) => card::look(Panel::default().shown(agent, now)).color,
            None => |t| t.agents_faint,
        },
        Shown::Shell => |t| t.muted,
        Shown::Empty => |t| t.agents_faint,
    }
}

/// A status dot on its own, where there is no icon to put it on.
fn plain_dot(color: Hsla, ui: &UiFont) -> AnyElement {
    div()
        .flex_shrink_0()
        .size(ui.px(7.0))
        .rounded_full()
        .bg(color)
        .into_any_element()
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
        // The left column shows the system's sidebar material while it is there: top to bottom
        // beside the sidebar, under the title bar beside the strip.
        let column = if self.frost.is_some() {
            frost_column(
                self.collapsed,
                self.full_screen,
                self.sidebar_width,
                height,
                window.viewport_size(),
                &ui,
            )
        } else {
            None
        };
        self.fit_frost(column, window, cx);
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
        let dragging = self.resizing.is_some() || self.right.resizing() || self.splitting.is_some();
        let seams = across(
            f32::from(window.viewport_size().width),
            self.sidebar_shown(&ui),
            self.collapsed,
            right,
        );
        // The Changes tab follows the focused pane, and reads only while it shows.
        let follow = self.follow(&agents, now, cx);
        let kanban = kanban_view::Frame {
            cwd: follow.as_ref().map(|f| f.cwd.clone()),
            active: right.is_some() && self.right.tab == RightTab::Kanban,
            width: right.unwrap_or(0.0),
            theme: self.theme.clone(),
            mono: mono_font(&self.template),
            agents: self.sidebar.read(cx).seen(),
        };
        let frame = Frame {
            follow,
            active: right.is_some() && self.right.tab == RightTab::Changes,
            width: right.unwrap_or(0.0),
            theme: self.theme.clone(),
            mono: mono_font(&self.template),
        };
        self.changes
            .update(cx, |changes, cx| changes.frame(frame, cx));
        // The Kanban tab follows the same pane's repository, and reads only while it shows.
        self.kanban.update(cx, |view, cx| view.frame(kanban, cx));
        // The right sidebar, a card pushed out from the right edge: the terminal narrows for it.
        if let Some(width) = right {
            let content = match self.right.tab {
                RightTab::Changes => AnyView::from(self.changes.clone())
                    .cached(StyleRefinement::default().size_full())
                    .into_any_element(),
                RightTab::Browser => self.browser.clone().into_any_element(),
                RightTab::Kanban => AnyView::from(self.kanban.clone())
                    .cached(StyleRefinement::default().size_full())
                    .into_any_element(),
            };
            let panel = self.right.render(
                &self.theme,
                &ui,
                content,
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
        // The `+N` menu goes once every tab has room again.
        let hidden = match self.popup {
            Some(Popup::Overflow) => self.fit_tabs(window.is_fullscreen(), window, cx).hidden,
            _ => Vec::new(),
        };
        if self.popup == Some(Popup::Overflow) && hidden.is_empty() {
            self.popup = None;
        }
        let dialog: Option<AnyElement> = match self.popup {
            Some(Popup::Actions) => Some(self.actions_menu(cx).into_any_element()),
            Some(Popup::Overflow) => {
                Some(self.overflow_menu(&hidden, window, cx).into_any_element())
            }
            Some(Popup::Attention) => Some(self.attention_panel(window, cx).into_any_element()),
            Some(Popup::Palette) => self
                .palette
                .as_ref()
                .map(|palette| self.palette_panel(palette, window, cx).into_any_element()),
            Some(Popup::NewTask) => self
                .new_task
                .clone()
                .map(|panel| self.new_task_panel(panel, window, cx).into_any_element()),
            Some(Popup::NewAgent) => self.new_agent.clone().map(IntoElement::into_any_element),
            Some(popup) => Some(self.chooser_panel(popup, window, cx).into_any_element()),
            None => None,
        };
        ui.apply(div())
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // GPUI's default size, scaled, for text nothing else sizes.
            .text_size(ui.px(16.0))
            .when(column.is_none(), |root| {
                root.bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            })
            // First, so it lies under the rest.
            .children(column.map(|column| self.frosted_ground(column)))
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
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<SplitDrag>, _, cx| {
                    this.drag_seam(event.event.position, cx)
                }),
            )
            .capture_any_mouse_up(cx.listener(|this, _, _, cx| {
                this.finish_resize(cx);
                this.finish_right_resize(cx);
                this.finish_seam(cx);
            }))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.finish_resize(cx);
                    this.finish_right_resize(cx);
                    this.finish_seam(cx);
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
            .child(self.title_bar(&agents, now, window.is_fullscreen(), window, cx))
            .child(body)
            .children(dialog)
            // While a divider is dragged the cursor keeps its shape, and nothing under it
            // reacts.
            .when(dragging, |root| {
                root.child(
                    div()
                        .id("resizing")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .map(|cover| match self.splitting.as_ref().map(|s| s.axis) {
                            Some(Axis::Column) => cover.cursor_row_resize(),
                            _ => cover.cursor_col_resize(),
                        }),
                )
            })
            .child(self.place_page(right.is_some(), dragging))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(name: &str, state: &str, instance: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            instance: Some(instance.into()),
            ..Agent::default()
        }
    }

    /// Every agent pane of `w`, its attach ended, as attached to `instance` of its agent.
    fn detached(w: &Workspace, instance: Option<&str>) -> Vec<AgentPane> {
        w.agent_panes()
            .into_iter()
            .map(|(pane, name)| AgentPane {
                pane,
                name,
                instance: instance.map(str::to_owned),
                attached: false,
            })
            .collect()
    }

    #[test]
    fn a_shown_agent_that_stopped_closes_its_pane_and_a_shell_takes_its_place() {
        for agents in [vec![], vec![listed("p/a", "exited", "i1")]] {
            let (mut w, only) = Workspace::new(Shown::Agent("p/a".into()));
            let ended = ended_panes(&detached(&w, Some("i1")), &agents);
            assert_eq!(ended, [only]);
            for pane in ended {
                w.close_pane(pane);
            }
            assert_eq!(w.tabs.len(), 1);
            assert_eq!(w.shown(w.active_pane()), &Shown::Shell);
        }
    }

    #[test]
    fn an_ended_agents_panes_close_in_every_tab_and_empty_tabs_with_them() {
        let (mut w, shell) = Workspace::new(Shown::Shell);
        let a = w.split(Direction::Right, Shown::Agent("p/a".into()));
        let again = w.new_tab(Shown::Agent("p/a".into()));
        let b = w.new_tab(Shown::Agent("p/b".into()));
        // p/a has left the list; p/b, in the tab shown, still runs.
        let ended = ended_panes(&detached(&w, None), &[listed("p/b", "idle", "i2")]);
        assert_eq!(ended, [a, again]);
        for pane in ended {
            w.close_pane(pane);
        }
        assert_eq!(w.tabs.len(), 2);
        assert_eq!(w.tabs[0].panes(), [shell]);
        assert_eq!(w.active_pane(), b);
    }

    #[test]
    fn a_restarted_agent_ended_the_instance_its_pane_showed() {
        let (w, pane) = Workspace::new(Shown::Agent("p/a".into()));
        let restarted = [listed("p/a", "starting", "i2")];
        assert_eq!(ended_panes(&detached(&w, Some("i1")), &restarted), [pane]);
        // Without an instance to tell them apart, the name still runs.
        assert!(ended_panes(&detached(&w, None), &restarted).is_empty());
        assert!(ended_panes(&detached(&w, Some("i2")), &restarted).is_empty());
    }

    #[test]
    fn a_paused_agent_has_not_ended() {
        let (w, _) = Workspace::new(Shown::Agent("p/a".into()));
        let paused = Agent {
            paused: true,
            ..listed("p/a", "idle", "i1")
        };
        assert!(ended_panes(&detached(&w, Some("i1")), &[paused]).is_empty());
    }

    #[test]
    fn a_pane_still_attaching_or_attached_is_not_taken_for_ended() {
        // A listing read before its agent started (New Agent, `paddock ctl`) leaves it out.
        let (w, _) = Workspace::new(Shown::Agent("p/new".into()));
        let mut panes = detached(&w, Some("i1"));
        panes[0].attached = true;
        assert!(ended_panes(&panes, &[]).is_empty());
    }

    #[test]
    fn a_restored_pane_whose_agent_is_gone_is_not_kept() {
        // Restoring attaches each saved agent; for one gone the attach ends at once, and the
        // listing then closes the pane rather than leave it saying the agent is gone.
        let (mut w, shell) = Workspace::new(Shown::Shell);
        let gone = w.split(Direction::Right, Shown::Agent("p/gone".into()));
        let kept = w.new_tab(Shown::Agent("p/kept".into()));
        let mut panes = detached(&w, Some("i1"));
        panes[1].attached = true;
        let ended = ended_panes(&panes, &[listed("p/kept", "idle", "i1")]);
        assert_eq!(ended, [gone]);
        w.close_pane(gone);
        assert_eq!(w.tabs[0].panes(), [shell]);
        assert_eq!(w.agents(), ["p/kept"]);
        assert_eq!(w.active_pane(), kept);
    }

    #[test]
    fn a_tab_waits_with_any_of_its_agents_and_is_paused_with_the_one_it_shows() {
        let (mut w, _) = Workspace::new(Shown::Agent("p/a".into()));
        // A split tab: its other pane's agent needs a person; the one it is named after is paused.
        w.split(Direction::Right, Shown::Agent("p/b".into()));
        let shown = w.active_pane();
        w.set_shown(shown, Shown::Agent("p/b".into()));
        w.new_tab(Shown::Shell);
        w.new_tab(Shown::Agent("p/c".into()));
        let paused = |name: &str| Agent {
            paused: true,
            ..listed(name, "idle", "i1")
        };
        let agents = [listed("p/a", "waiting", "i1"), paused("p/b"), paused("p/c")];
        let marks = tab_marks(&w, &agents, &["p/a".into()]);
        assert_eq!(
            marks,
            [
                TabMarks {
                    waiting: true,
                    paused: true
                },
                TabMarks::default(),
                TabMarks {
                    waiting: false,
                    paused: true
                },
            ]
        );
        // Another pane of the tab focused, the tab is named after it, and it is not paused.
        w.select_tab(0);
        let other = w.tabs[0].panes()[0];
        w.focus(other);
        assert!(!tab_marks(&w, &agents, &[])[0].paused);
    }

    #[test]
    fn a_paused_agents_dot_is_the_cards_paused_colour() {
        let agent = Agent {
            name: "p/a".into(),
            state: Some("idle".into()),
            paused: true,
            ..Default::default()
        };
        let theme = crate::preset::Theme::default();
        let pick = dot(&Shown::Agent("p/a".into()), &[agent], 0.0);
        let paused = card::look(crate::agents::Status::Paused).color;
        assert_eq!(pick(&theme), paused(&theme));
    }

    #[test]
    fn continuous_window_size_changes_save_once_after_the_last_change() {
        let start = Instant::now();
        let mut pending = WindowSizeSave::default();
        let mut saves = Vec::new();
        for millis in [0, 100, 300, 600, 1099, 1100, 1600] {
            let now = start + Duration::from_millis(millis);
            if millis <= 600 {
                pending.changed(now);
            }
            if pending.take_due(now) {
                saves.push(millis);
            }
        }
        assert_eq!(saves, [1100]);
    }

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
    fn agents_show_the_short_name_and_the_group_only_when_another_open_agent_shares_it() {
        let open = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let short = |s: &str| (None, s.to_owned());
        let grouped = |g: &str, s: &str| (Some(g.to_owned()), s.to_owned());
        // Short whenever no other open agent reads the same, including not being open at all.
        let same_group = open(&["paddock/main", "paddock/dev-fonts"]);
        assert_eq!(agent_name("paddock/main", &same_group), short("main"));
        assert_eq!(agent_name("paddock/main", &[]), short("main"));
        // The same agent in two panes is no clash.
        let twice = open(&["paddock/main", "paddock/main"]);
        assert_eq!(agent_name("paddock/main", &twice), short("main"));
        // Both clashing names get their group, and only they do.
        let clash = open(&["paddock/main", "ranch/main", "ranch/dev-route"]);
        assert_eq!(
            agent_name("paddock/main", &clash),
            grouped("paddock/", "main")
        );
        assert_eq!(agent_name("ranch/main", &clash), grouped("ranch/", "main"));
        assert_eq!(agent_name("ranch/dev-route", &clash), short("dev-route"));
        // The group is everything before the last `/`.
        let deep = open(&["a/b/c", "x/c"]);
        assert_eq!(agent_name("a/b/c", &deep), grouped("a/b/", "c"));
        // Without a group, or with nothing after it, the name is shown whole.
        let solo = open(&["solo", "p/solo"]);
        assert_eq!(agent_name("solo", &solo), short("solo"));
        assert_eq!(agent_name("p/solo", &solo), grouped("p/", "solo"));
        assert_eq!(agent_name("p/", &open(&["p/", "q/"])), short("p/"));
        // The tab keeps the group apart, to draw it faint.
        let agent = Subject::Agent("paddock/main".into());
        assert_eq!(tab_label(&agent, &clash), grouped("paddock/", "main"));
        assert_eq!(tab_label(&agent, &same_group), short("main"));
        assert_eq!(
            tab_label(&shell("/bin/zsh", "/Users/me/paddock"), &clash),
            short("zsh · paddock")
        );
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
    fn a_split_tab_shows_the_panes_icon_with_how_many_panes_it_holds() {
        assert_eq!(panes_tip(1), None);
        assert_eq!(panes_tip(2).as_deref(), Some("2 panes"));
        assert_eq!(panes_tip(4).as_deref(), Some("4 panes"));
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
    fn pane_headers_name_agents_as_tabs_do_with_the_short_directory() {
        let home = Some("/Users/me");
        let agent = Subject::Agent("paddock/main".into());
        let open = vec!["paddock/main".to_owned()];
        assert_eq!(
            pane_name(&agent, Some("/Users/me/code/paddock"), home, &open),
            ((None, "main".into()), Some("~/…/paddock".into()))
        );
        let clash = vec!["paddock/main".to_owned(), "ranch/main".to_owned()];
        assert_eq!(
            pane_name(&agent, None, home, &clash),
            ((Some("paddock/".into()), "main".into()), None)
        );
        assert_eq!(
            pane_name(&shell("/bin/zsh", "/Users/me/code"), None, home, &clash),
            ((None, "zsh".into()), Some("~/code".into()))
        );
        assert_eq!(
            pane_name(&Subject::Empty, None, home, &open),
            ((None, "empty".into()), None)
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
    fn the_frost_is_the_column_or_the_strip_under_the_title_bar_and_gone_in_full_screen() {
        let base = UiFont::default();
        let window = size(px(1200.0), px(800.0));
        let column = |top: f32, width: f32| {
            Some(Bounds::new(
                point(px(0.0), px(top)),
                size(px(width), px(800.0 - top)),
            ))
        };
        // Expanded: one column, the sidebar with the title bar over it, from the window's top edge
        // to its bottom; it follows the divider.
        assert_eq!(
            frost_column(false, false, 300.0, TITLE_BAR, window, &base),
            column(0.0, 300.0)
        );
        assert_eq!(
            frost_column(false, false, 412.0, PET_TITLE_BAR, window, &base),
            column(0.0, 412.0)
        );
        // Collapsed: the strip, whatever the sidebar's width, from the title bar's bottom edge to
        // the window's; the title bar's row is opaque all along.
        assert_eq!(
            frost_column(true, false, 300.0, TITLE_BAR, window, &base),
            column(TITLE_BAR, 52.0)
        );
        assert_eq!(
            frost_column(true, false, 412.0, PET_TITLE_BAR, window, &base),
            column(PET_TITLE_BAR, 52.0)
        );
        // It grows with the interface size and starts under the taller title bar.
        let large = UiFont {
            family: None,
            size: 19.5,
        };
        let tall = title_bar_height(&large, false);
        assert_eq!(
            frost_column(true, false, 300.0, tall, window, &large),
            column(tall, 78.0)
        );
        // Full screen: none, expanded or collapsed.
        assert_eq!(
            frost_column(false, true, 300.0, TITLE_BAR, window, &base),
            None
        );
        assert_eq!(
            frost_column(true, true, 300.0, TITLE_BAR, window, &base),
            None
        );
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
        assert_eq!(open_where(Right), "Open on the right");
        assert_eq!(open_where(Left), "Open on the left");
        assert_eq!(open_where(Up), "Open above");
        assert_eq!(open_where(Down), "Open below");
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
