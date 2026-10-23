//! The Agents sidebar: `corral ls` and the agents' Git summaries through the pollers taken from
//! Saddle, ordered and judged by its Agents panel model, and each idle agent's last reply, read
//! once a spell. Each agent is a card read like a conversation (`card.rs` decides what
//! it says): its kind's avatar with the status on its corner, then the name, a preview and where it
//! works. Clicking a card asks the window to show that agent, and clicking the one shown opens its
//! details. Under the list the activity grid (`activity_view.rs`) shows the agents' repositories'
//! commits a day. The footer's one button opens the window's menu of actions; the header row, which
//! the window puts in its title bar, starts with the button that collapses the sidebar to a narrow
//! strip of one tile per agent, and before its bell has a button that pauses or resumes every agent
//! (P5-33).
use crate::{
    activity_view::{self, ActivityView, Folded},
    agents::{Folds, Panel, Spell, Status},
    attention,
    card::{self, Card, Click, Ink, Line, Pick, Preview, Tone},
    corral::{Agent, Client, Poller, Replies, Role},
    fonts::UiFont,
    footer_icon::{self, Icon, Pose},
    git, inbox, kind_icon,
    local_only::Only,
    motion::{self, Hover, HoverMotion},
    pause, reduce_motion, sunk,
    theme::Theme,
    view::hsla,
    viewer::AgentMetadata,
};
use anyhow::Result;
use gpui::{
    Animation, AnimationExt, AnyElement, App, BoxShadow, ClickEvent, ClipboardItem, Context, Div,
    ElementId, Entity, EventEmitter, Font, FontFeatures, FontWeight, HighlightStyle, Hsla,
    MouseButton, MouseDownEvent, Pixels, Render, RenderOnce, Rgba, SharedString, StyledText,
    TextRun, Transformation, Window, div, ease_in_out, percentage, point, prelude::*, px, relative,
    svg,
};
use std::{
    collections::HashMap,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

#[path = "sidebar_workspaces.rs"]
mod workspaces_view;
use workspaces_view::Floating;

#[path = "sidebar_sandbox.rs"]
mod sandbox_view;

#[path = "sidebar_delegates.rs"]
mod delegates_view;

#[path = "sidebar_motion.rs"]
mod cards_motion;
use cards_motion::{Anchor, CardMotion, Slot};

/// One machine in the switcher over the list (DESIGN §13 P5-90).
#[derive(Clone, Debug, PartialEq)]
pub struct MachineTab {
    /// Who the machine is, its config name; `None` for This Mac.
    pub id: Option<String>,
    /// The name people see: its label, else its name (P5-123).
    pub name: String,
    /// How many agents its corral holds, as last read, written after its name: another
    /// machine's, when it has any (see [`agents_written`]).
    pub agents: Option<usize>,
    /// What wants the user on it, as its bell counts it (P5-130).
    pub attention: Option<attention::Capsule>,
    /// How its connection stands while it does not answer.
    pub status: Option<String>,
    /// The status tells of something wrong.
    pub problem: bool,
    /// The hover note: where it is, or why it does not answer and what to do.
    pub tip: String,
    /// While it is down, the banner over its list (P5-122): what stands, with its name, and what
    /// to do.
    pub trouble: Option<(String, String)>,
    /// On show.
    pub shown: bool,
}

/// How many agents the switcher writes after machine `seat`'s name, of the `agents` its corral
/// last listed (P5-122 rework 1): another machine's, when it has any; none for This Mac.
pub fn agents_written(seat: usize, agents: usize) -> Option<usize> {
    (seat > 0 && agents > 0).then_some(agents)
}

/// What the sidebar asks of the window.
pub enum SidebarEvent {
    /// Show this agent: where it already is, or by the layout rules.
    Attach {
        name: String,
        metadata: AgentMetadata,
    },
    /// Open the New Agent panel.
    NewAgent,
    /// Open the New Agent panel from a workspace, starting in `start_in` when it says: the
    /// repository of the agent it starts joins the workspace when in none (P5-93, P5-105).
    NewAgentIn {
        workspace: String,
        start_in: Option<String>,
    },
    /// Open or close the Inbox, the bell's panel.
    Attention,
    /// Open or close the menu of actions.
    Actions,
    /// Collapse to the strip, or expand again.
    ToggleCollapse,
    /// Stop this agent, after asking (the open card's Stop…).
    Stop(String),
    /// Pause or resume agents with corral, asking first when it says: a card's Pause or Resume,
    /// or the button for every agent.
    Pause(pause::Request),
    /// The agents corral still lists, for panes to let go of one that disappeared.
    Alive(Vec<String>),
    /// The activity panel was folded (`true`) or opened: save it with the layout.
    ActivityFolded(bool),
    /// A group was folded to its heading or opened: save them with the layout.
    Folds(Folds),
    /// The switcher's machine at this place: show it, and try it again when it could not be
    /// read (DESIGN §13 P5-90).
    Machine(usize),
    /// `Switch to This Mac` on the hint about what starts there alone: show This Mac, then do it
    /// (P5-122).
    ThisMac(Only),
    /// A choice in another machine's capsule menu (P5-123), for the machine by its name.
    MachineMenu { machine: String, ask: MachineAsk },
}

/// What a machine's capsule menu offers (P5-123).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineAsk {
    /// Its New Agent dialog, the machine on show (P5-124).
    NewAgent,
    /// Settings › Machines, its name as a field.
    Rename,
    Hide,
    /// Settings › Machines, asking first.
    Remove,
    /// Settings › Machines.
    Settings,
}

/// How often the agents' worktrees are summarised, as in Saddle.
const GIT_REFRESH: Duration = Duration::from_secs(5);

/// How often fence is asked what the wall stopped for each sandboxed agent (P5-120).
const FENCE_REFRESH: Duration = Duration::from_secs(3);

// Sizes, in points at the base interface size.
/// The list's side padding.
const PAD: f32 = 10.0;
/// A card's padding (left, right, top and bottom, inside its 1-point edge) and corners.
const CARD_LEFT: f32 = 10.0;
const CARD_RIGHT: f32 = 12.0;
const CARD_Y: f32 = 10.0;
const CARD_RADIUS: f32 = 12.0;
/// The avatar: its side, its corners, the room after it, and the kind's icon or letter in it.
const AVATAR: f32 = 34.0;
const AVATAR_RADIUS: f32 = 10.0;
const AVATAR_GAP: f32 = 11.0;
const AVATAR_ICON: f32 = 16.0;
/// An original without a ground of its own (pi's logo) in the avatar, leaving a margin of tint.
const AVATAR_PICTURE: f32 = 24.0;
/// How strongly the kind's colour tints the avatar, and waiting amber the card.
const AVATAR_TINT: f32 = 0.13;
const WAITING_TINT: f32 = 0.07;
/// The room between two cards (P5-51).
const CARD_GAP: f32 = 8.0;
/// A card lifted off the column (P5-51): how much of the text colour its face and edge take
/// resting, hovered and paused; how much of the accent a selected card's face (over the material)
/// and edge take, and of amber a waiting card's edge.
const FACE: f32 = 0.055;
const FACE_HOVERED: f32 = 0.085;
const FACE_PAUSED: f32 = 0.03;
const EDGE: f32 = 0.07;
const EDGE_HOVERED: f32 = 0.11;
const EDGE_PAUSED: f32 = 0.05;
const SELECTED_FACE: f32 = 0.11;
const SELECTED_EDGE: f32 = 0.42;
const WAITING_EDGE: f32 = 0.34;
/// Over the material a card's see-through face takes this much more of its colour, to stand brighter
/// than the material around it; and how dark the line inside its bottom edge is (P5-51b).
const FROSTED_FACE: f32 = 1.45;
const FROSTED_BOTTOM: f32 = 0.28;
/// On a light theme (P5-175) a card's face, and the light along its top edge, are white this many
/// times as strong as the text colour would be on a dark one.
const LIGHT_FACE: f32 = 6.0;
const LIGHT_EDGE: f32 = 8.0;
const DOT: f32 = 8.0;
/// Room kept before the age.
const TIME_PAD: f32 = 4.0;
/// The share of a card's width a name keeps before the effort gives way.
const NAME_SHARE: f32 = 0.4;
/// The marks after the name: the gap before each, the unread dot, and the open-here ring and its
/// stroke, each in a box to hover.
const MARK_GAP: f32 = 6.0;
const UNREAD: f32 = 6.0;
const UNREAD_BOX: f32 = 10.0;
const HERE: f32 = 5.0;
const HERE_RING: f32 = 1.5;
const HERE_BOX: f32 = 9.0;
/// The lock in a controller's delegates mark and menu (P5-141).
const LOCK: f32 = 12.0;
/// The open card: between its cells' columns, and a chip's sides.
const CELL_GAP: f32 = 12.0;
const CHIP_X: f32 = 7.0;
/// A text button's sides.
const BUTTON_X: f32 = 9.0;
/// A group's status bar: each agent's share, and the most it grows to.
const BAR_SHARE: f32 = 12.0;
const BAR_MAX: f32 = 96.0;
// The type scale (DESIGN §13).
const TITLE_SIZE: f32 = 13.0;
const CARD_NAME_SIZE: f32 = 14.0;
const NAME_SIZE: f32 = 13.0;
const PREVIEW_SIZE: f32 = 12.5;
const SECOND_SIZE: f32 = 12.0;
const NOTE_SIZE: f32 = 11.5;
const INSTANCE_SIZE: f32 = 10.5;
/// A group's heading (a repository's line): the name, and the count after it (P5-129, P5-129a:
/// the name sits with the card's folder and time (`NOTE_SIZE`)).
const GROUP_NAME_SIZE: f32 = 11.5;
const GROUP_COUNT_SIZE: f32 = 11.0;
const CELL_LABEL_SIZE: f32 = 9.5;
const BADGE_SIZE: f32 = 9.0;
const TILE_SIZE: f32 = 12.5;
/// The preview's lines, folded and open.
const PREVIEW_LINES: usize = 2;
const PREVIEW_OPEN_LINES: usize = 6;
/// The arrow after a group's count: down while its cards show, right while it is folded.
const CHEVRON_DOWN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 3.8L5 6.3l2.5-2.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const CHEVRON_RIGHT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M3.8 2.5L6.3 5 3.8 7.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const CHEVRON: f32 = 9.0;
/// How far a group's heading takes clicks, and lights under the pointer, above and below its
/// words.
const HEADING_Y: f32 = 4.0;
/// A working agent's spinner, drawn as a mask: the faint track and the arc that turns.
const SPIN_TRACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><circle cx="6" cy="6" r="4.5" fill="none" stroke="#000" stroke-width="2"/></svg>"##;
const SPIN_ARC: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5a4.5 4.5 0 0 1 4.5 4.5" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></svg>"##;

/// The collapsed strip's width, in points at the base interface size.
pub const RAIL: f32 = 52.0;
/// The widths dragging the divider keeps to, at the base interface size; larger sizes need more
/// room for the header row (see [`min_width`]).
pub const MIN_WIDTH: f32 = 220.0;
pub const MAX_WIDTH: f32 = 560.0;
/// Where the header row starts in the title bar, from the window's left edge: past the traffic
/// lights (at 14 points, ending at 68), or in from the edge in full screen, where there are none.
/// Neither scales.
const LIGHTS: f32 = 76.0;
const FULL_SCREEN: f32 = 12.0;
/// The collapse (or expand) button, the room after it collapsed to the strip, and the room before
/// `Agents` past it.
const TOGGLE: f32 = 26.0;
const TOGGLE_GAP: f32 = 6.0;
const TITLE_GAP: f32 = 10.0;
/// What the header row needs for its words, about as wide as the system font draws them: `Agents`,
/// the room before its count, and a digit of the bell's count.
const TITLE_WIDTH: f32 = 44.0;
const COUNT_GAP: f32 = 6.0;
const DIGIT: f32 = 7.8;
/// The count after `Agents`, in a small pill: its height (the ends round off), sides, type size and
/// a digit as wide as it draws.
const COUNT_HEIGHT: f32 = 18.0;
const COUNT_X: f32 = 5.0;
const COUNT_SIZE: f32 = 11.0;
const COUNT_DIGIT: f32 = 6.8;
/// The bell as a pill (its sides, and the room between the icon and the count) and compact.
const PILL_X: f32 = 8.0;
const PILL_GAP: f32 = 5.0;
const COMPACT_BELL: f32 = 28.0;
/// The button that pauses or resumes every agent, before the bell, and the room between them.
const PAUSE_ALL: f32 = 28.0;
const PAUSE_GAP: f32 = 2.0;
/// The button that opens `New Workspace…`, before that one, as big and as far from it.
const NEW_WORKSPACE: f32 = PAUSE_ALL;
/// The footer, and its menu button at the bottom left and the icon in it; in the strip the button
/// is centred.
const FOOTER: f32 = 50.0;
const BUTTON: f32 = 32.0;
const BUTTON_ICON: f32 = 17.0;
/// Under the list's last card, before the activity panel: the list's own end and the panel's gap
/// above it, as when the panel lay under the list.
const LIST_END: f32 = 10.0;
const ACTIVITY_GAP: f32 = 4.0;
/// The cards' avatars' left edge, unscaled: past the list's side, a card's edge and its padding.
const AVATAR_LEFT: f32 = PAD + 1.0 + CARD_LEFT;
/// How long a note of a start or stop that went well stays whole, and then how long it fades.
const NOTE_HOLD: Duration = Duration::from_secs(4);
const NOTE_FADE: Duration = Duration::from_millis(600);
/// A card's icon buttons, Copy, Pause (or Resume) and Stop…, and their corners; Copy's icon is a
/// little smaller.
const CARD_BUTTON: f32 = 24.0;
const CARD_BUTTON_RADIUS: f32 = 7.0;
const COPY_ICON: f32 = 0.9;
/// The arrow after a card's time that opens or closes its details, its corners and its icon's
/// scale: no taller than the name's line (P5-74).
const ARROW_BOX: f32 = 18.0;
const ARROW_RADIUS: f32 = 5.0;
const ARROW_ICON: f32 = 0.8;
/// How strongly a paused agent's card and tile show.
const PAUSED_OPACITY: f32 = 0.55;
/// How bright another machine's cards stay while it cannot be read: as last read, dimmed
/// (DESIGN §13 P5-90).
const STALE: f32 = 0.5;
/// A machine's cell in the switcher (P5-139): its height and corners.
const SWITCH_CELL: f32 = 24.0;
const SWITCH_RADIUS: f32 = 7.0;
/// The widest a machine's name is in its capsule before it is cut short (P5-123).
const CAPSULE_NAME: f32 = 120.0;
/// How wide a machine's capsule menu is (P5-123): at least, and at most for a long name.
const MACHINE_MENU: (f32, f32) = (210.0, 320.0);
/// How strongly the grey cards show while another machine is not read yet, one after another.
const PLACEHOLDERS: [f32; 3] = [1.0, 0.6, 0.3];
/// What a card asks before pausing an agent in a turn.
const PAUSE_WORKING: &str = "Working \u{2014} pause anyway?";
/// An agent's tile in the strip: its side, corners and the room above and below it, and how far
/// the plate showing its state reaches round a tile its original covers (two plates, a shown
/// tile's and a hovered neighbour's, just miss).
const TILE: f32 = 34.0;
const TILE_RADIUS: f32 = 9.0;
const TILE_GAP: f32 = 3.5;
const TILE_PLATE: f32 = 3.0;
/// The strip's bell: its height (and least width), its sides, and the room between the icon and
/// the count.
const RAIL_BELL: f32 = 28.0;
const RAIL_BELL_X: f32 = 6.0;
const RAIL_BELL_GAP: f32 = 3.0;
/// Name prefixes that say what kind of work an agent does, not which it is: a tile's letter comes
/// after them.
const ROLE_PREFIXES: [&str; 3] = ["dev-", "test-", "review-"];

/// The sidebar's width while the divider is dragged: the width when it was pressed, moved as far
/// as the mouse has, kept within reach and to whole points.
pub fn resize(width_at_press: f32, press_x: f32, x: f32, min: f32) -> f32 {
    (width_at_press + x - press_x)
        .clamp(min, MAX_WIDTH.max(min))
        .round()
}

/// The room under the list's last card (P5-70): the activity panel and the footer under it lie over
/// the list's end (P5-70b), so the list ends that far below its last card, which then scrolls clear
/// above the panel. On the frosted sidebar the list stops at the footer ([`list_bottom`]), which
/// is then left out.
pub fn list_end(width: f32, ui: &UiFont, folded: bool, frosted: bool) -> f32 {
    let footer = if frosted { 0.0 } else { ui.scale(FOOTER) };
    LIST_END + ui.scale(ACTIVITY_GAP) + activity_view::height(width, ui, folded) + footer
}

/// The room kept below the list itself: on the frosted sidebar the footer has no ground of its own
/// and shows the material, as the column above it does, so no card goes on under it (P5-75); on the
/// sidebar's own colour, none (P5-70b).
pub fn list_bottom(ui: &UiFont, frosted: bool) -> f32 {
    if frosted { ui.scale(FOOTER) } else { 0.0 }
}

/// Where the header row starts in the title bar.
pub fn head_start(full_screen: bool) -> f32 {
    if full_screen { FULL_SCREEN } else { LIGHTS }
}

/// The expand button and the room after it, collapsed to the strip, past [`head_start`].
pub fn toggle_room(ui: &UiFont) -> f32 {
    ui.scale(TOGGLE + TOGGLE_GAP)
}

/// What the header row holds, as wide as it is set: `Agents`, the number in its count's pill (none
/// without agents), whether the button for every agent is there, and the bell's number (none when
/// nothing rings it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadWords {
    pub title: f32,
    pub count: Option<f32>,
    pub pause_all: bool,
    pub bell: Option<f32>,
}

impl HeadWords {
    /// The row the narrowest sidebar is kept for, about as wide as the system font draws it: a
    /// two-digit count, and the button for every agent.
    fn reckoned(ui: &UiFont) -> Self {
        Self {
            title: ui.scale(TITLE_WIDTH),
            count: Some(ui.scale(2.0 * COUNT_DIGIT)),
            pause_all: true,
            bell: Some(ui.scale(2.0 * DIGIT)),
        }
    }
}

/// How wide a sidebar the header row with `words` needs, starting at `start`, with the bell
/// compact or as a pill; the `New Workspace…` button is always there.
fn head_width(start: f32, compact: bool, ui: &UiFont, words: &HeadWords) -> f32 {
    let bell = if compact {
        ui.scale(COMPACT_BELL)
    } else {
        ui.scale(PILL_X + footer_icon::SIZE + PILL_X)
            + words.bell.map_or(0.0, |number| ui.scale(PILL_GAP) + number)
    };
    let count = words.count.map_or(0.0, |number| {
        ui.scale(COUNT_GAP) + (ui.scale(2.0 * COUNT_X) + number).max(ui.scale(COUNT_HEIGHT))
    });
    let pause_all = if words.pause_all {
        ui.scale(PAUSE_ALL + PAUSE_GAP)
    } else {
        0.0
    };
    let new_workspace = ui.scale(NEW_WORKSPACE + PAUSE_GAP);
    start
        + ui.scale(TOGGLE + TITLE_GAP)
        + words.title
        + count
        + new_workspace
        + pause_all
        + bell
        + PAD
}

/// The narrowest the sidebar is dragged to: 220, or wider when the header row with the compact
/// bell needs it, in whole points so that a dragged width, rounded, still keeps to it.
pub fn min_width(ui: &UiFont) -> f32 {
    MIN_WIDTH.max(head_width(LIGHTS, true, ui, &HeadWords::reckoned(ui)).ceil())
}

/// The sidebar's width as drawn for `width` from the config file: one narrower than
/// [`min_width`], written by hand or before the header row needed it, comes up to it, which the
/// next drag saves.
pub fn fit_width(width: f32, ui: &UiFont) -> f32 {
    width.max(min_width(ui))
}

/// Whether the bell goes compact in a sidebar `width` wide: when the row, with `words` as wide as
/// they are set, has no room for it as a pill.
pub fn compact_bell(width: f32, full_screen: bool, ui: &UiFont, words: &HeadWords) -> bool {
    width < head_width(head_start(full_screen), false, ui, words)
}

/// A tile's letter: the agent's short name's first letter or digit, upper case, after a prefix
/// such as `dev-`.
pub fn initial(short: &str) -> String {
    let core = ROLE_PREFIXES
        .iter()
        .find_map(|prefix| short.strip_prefix(prefix))
        .filter(|rest| rest.chars().any(char::is_alphanumeric))
        .unwrap_or(short);
    core.chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "?".to_owned(), |c| c.to_uppercase().collect())
}

/// The menu's Stop item for the active pane's agent: what it says, and whether it can be chosen.
pub fn stop_item(selected: Option<&str>) -> (String, bool) {
    match selected {
        Some(name) => (format!("Stop {name}…"), true),
        None => ("Stop Agent…".to_owned(), false),
    }
}

/// The menu's Pause item for the active pane's agent, `paused` or not: what it says, and whether
/// it can be chosen.
pub fn pause_item(selected: Option<&str>, paused: bool) -> (String, bool) {
    match selected {
        Some(name) if paused => (format!("Resume {name}"), true),
        Some(name) => (format!("Pause {name}"), true),
        None => ("Pause Agent".to_owned(), false),
    }
}

/// Where the menu of actions opens: its left edge, in line with the button's, and how far its
/// bottom edge sits above the window's, just over the button.
pub fn menu_anchor(collapsed: bool, ui: &UiFont) -> (Pixels, Pixels) {
    let left = if collapsed {
        ui.px((RAIL - BUTTON) / 2.0)
    } else {
        px(button_left(ui))
    };
    (left, ui.px((FOOTER - BUTTON) / 2.0 + BUTTON + 6.0))
}

/// The menu button's left edge in the expanded sidebar: where its icon's drawing starts, centred
/// in the button, falls in line with the Activity card's outer edge just above it, as the button
/// sits outside the card (P5-57). The box never starts left of the sidebar, so at large interface
/// sizes the icon may sit a little in.
fn button_left(ui: &UiFont) -> f32 {
    (activity_view::card_left()
        - ui.scale(
            (BUTTON - BUTTON_ICON) / 2.0
                + footer_icon::ACTIONS_LEFT * BUTTON_ICON / footer_icon::SIZE,
        ))
    .max(0.0)
}

/// The agent the user sees in a machine's active pane, whose new reply is read at once: none while
/// that machine waits for another on show (DESIGN §13 P5-90).
pub fn looking_at(selected: Option<&str>, on_show: bool) -> Option<&str> {
    selected.filter(|_| on_show)
}

/// The list model: the last good `corral ls`, the last error if the latest read failed, and the
/// Git summaries by directory.
#[derive(Default)]
pub struct Listing {
    panel: Panel,
    error: Option<String>,
    /// corral has answered at least once, well or not.
    loaded: bool,
    /// The latest `corral ls`, for Diagnostics.
    last: crate::diagnostics::Last,
}

impl Listing {
    /// Takes one corral result. On success returns the agents still alive, for the pane to let go
    /// of one that disappeared; an error keeps the previous list.
    pub fn absorb(
        &mut self,
        update: Result<Vec<Agent>>,
        here: Option<&str>,
        now: f64,
    ) -> Option<Vec<String>> {
        self.loaded = true;
        let now_time = std::time::SystemTime::now();
        self.last = Some((
            now_time,
            match &update {
                Ok(agents) => Ok(format!("{} agents", agents.len())),
                Err(error) => Err(format!("{error:#}")),
            },
        ));
        match update {
            Ok(agents) => {
                self.error = None;
                let alive = agents
                    .iter()
                    .filter(|a| a.state.as_deref() != Some("exited"))
                    .map(|a| a.name.clone())
                    .collect();
                self.panel.absorb(agents, here, now);
                Some(alive)
            }
            Err(error) => {
                self.error = Some(format!("corral: {error:#}"));
                None
            }
        }
    }

    pub fn absorb_git(&mut self, batch: git::Batch) {
        self.panel.absorb_git(batch);
    }

    /// Asks for the last reply of each agent that has just gone idle; never twice in one spell.
    pub fn ask_replies(&mut self, replies: &Replies<Spell>, now: f64) {
        for spell in self.panel.replies_to_read(now) {
            let name = spell.name.clone();
            replies.ask(spell, name);
        }
    }

    /// Takes the replies read since; whether there were any.
    pub fn absorb_replies(&mut self, replies: &Replies<Spell>) -> bool {
        let mut any = false;
        for (spell, text) in replies.updates.try_iter() {
            self.panel.absorb_reply(spell, text);
            any = true;
        }
        any
    }

    /// The directories whose Git state the panel shows.
    pub fn cwds(&self) -> Vec<String> {
        let mut cwds: Vec<String> = self
            .panel
            .agents
            .iter()
            .filter_map(|a| a.cwd.clone())
            .collect();
        cwds.sort();
        cwds.dedup();
        cwds
    }

    pub fn lines(
        &self,
        selected: Option<&str>,
        here: &[String],
        home: Option<&str>,
        now: f64,
    ) -> Vec<Line> {
        card::lines(
            &self.panel,
            self.error.as_deref(),
            selected,
            here,
            home,
            now,
        )
    }

    /// The lines with every card, whatever is folded: the strip has a tile for each agent.
    fn unfolded(
        &self,
        selected: Option<&str>,
        here: &[String],
        home: Option<&str>,
        now: f64,
    ) -> Vec<Line> {
        card::unfolded(
            &self.panel,
            self.error.as_deref(),
            selected,
            here,
            home,
            now,
        )
    }

    /// An agent is working, as the cards show it: a paused one is not, for the spinners and the
    /// activity glow alike.
    fn animating(&self, now: f64) -> bool {
        self.panel
            .agents
            .iter()
            .any(|a| self.panel.shown(a, now) == Status::Working)
    }
}

pub struct Sidebar {
    /// The colours from Settings.
    given: Rc<Theme>,
    /// The colours the column is drawn in: the given ones, their quiet text lifted while frosted.
    theme: Rc<Theme>,
    width: f32,
    /// The terminal's font, for the instance id in the details.
    mono: Font,
    listing: Listing,
    poller: Poller,
    /// Another machine's agents, rather than this Mac's (DESIGN §13 P5-90): read over ssh, with
    /// nothing read from this Mac's directories (no Git, Activity or workspaces).
    remote: Option<Arc<crate::remote::Link>>,
    /// When its agents were last read, for saying how long ago while it cannot be reached.
    read_at: Option<f64>,
    /// When the read of the agents listed now began: a pane checks a count it reads against when
    /// it attached (DESIGN §13 P5-90).
    listed_at: Option<std::time::Instant>,
    /// What rings the bell on the other machines, which the Inbox lists too.
    elsewhere: Vec<attention::Item>,
    /// The switcher's machines; none without other machines.
    machines: Vec<MachineTab>,
    /// None for another machine's agents.
    git: Option<git::Poller>,
    /// Reads what fence's wall stopped for the sandboxed agents; none for another machine's.
    fence: Option<crate::fence::Watcher>,
    /// The Blocked popover lists every block, not only the network and file-write ones.
    denials_all: bool,
    /// The `corral` program, for setting a controller's Delegates (P5-141).
    corral: String,
    /// fence's profiles as last read for the Delegates menu, and the read still running.
    profiles: delegates_view::Read,
    finding_profiles: Option<crate::fence::Cancel>,
    /// Reads last replies for the cards' previews.
    replies: Replies<Spell>,
    /// The active pane's agent, in this machine's layout.
    selected: Option<String>,
    /// This machine is the one on show: its active pane's agent is what the user sees, and a
    /// new reply there is read. Another machine's, waiting, reads nothing (DESIGN §13 P5-90).
    on_show: bool,
    /// Every agent open in this window's panes.
    here: Vec<String>,
    /// Written `~` in directories.
    home: Option<String>,
    /// The result of the last start or stop, until it fades.
    note: Option<Note>,
    /// Collapsed to the narrow strip.
    collapsed: bool,
    /// Sliding to the strip or back, how far folded (P5-148a): the window says each frame.
    fold: Option<f32>,
    /// The menu of actions is open: its button stays lit.
    menu_open: bool,
    /// The card asking whether to pause its agent in a turn, until the mouse leaves it.
    asking: Option<String>,
    /// Its column shows the system's sidebar material: its grounds are tints over it.
    frosted: bool,
    /// The activity grid over the footer (P5-32), lying over the list's end (P5-70).
    activity: Entity<ActivityView>,
    /// Where the workspaces are kept (P5-89); `None` keeps them only in memory.
    workspaces_path: Option<std::path::PathBuf>,
    /// The workspace menu, panel or field open over the list: one at a time.
    floating: Option<Floating>,
    /// Has the keys while one is open, for Esc and Return.
    floating_focus: gpui::FocusHandle,
    /// What had the keys before it opened, to give them back.
    floating_back: Option<gpui::FocusHandle>,
    /// The press that closed a menu, and whose it was: on its own button it does not open it
    /// again.
    dismissed: Option<(gpui::Point<Pixels>, String)>,
    /// The cards sliding, fading in or out and ringing (P5-154), shared with what draws them.
    cards: Rc<std::cell::RefCell<CardMotion>>,
    _floating_out: gpui::Subscription,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        theme: Rc<Theme>,
        width: f32,
        mono: Font,
        corral: String,
        refresh: Duration,
        remote: Option<Arc<crate::remote::Link>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let floating_focus = cx.focus_handle();
        let floating_out = cx.on_focus_out(&floating_focus, window, |sidebar, _, window, cx| {
            sidebar.floating_left(window, cx)
        });
        // The window opens on this Mac.
        let on_show = remote.is_none();
        let workspaces_path = crate::workspaces::path().filter(|_| remote.is_none());
        let mut listing = Listing::default();
        if let Some(path) = &workspaces_path {
            listing.panel.workspaces = crate::workspaces::Workspaces::load(path);
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |sidebar, cx| sidebar.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let activity = {
            let (theme, mono) = (theme.clone(), mono.clone());
            cx.new(|cx| ActivityView::new(theme, mono, width, cx))
        };
        cx.subscribe(&activity, |_, _, folded: &Folded, cx| {
            // The list's end follows the panel's height.
            cx.notify();
            cx.emit(SidebarEvent::ActivityFolded(folded.0))
        })
        .detach();
        Self {
            given: theme.clone(),
            theme,
            width,
            mono,
            listing,
            replies: Replies::start(Client {
                program: corral.clone(),
                remote: remote.clone(),
            }),
            poller: match &remote {
                None => Poller::start(Client::local(corral.clone()), refresh),
                Some(link) => Poller::remote(link.clone()),
            },
            git: remote
                .is_none()
                .then(|| git::Poller::start("git".into(), GIT_REFRESH)),
            fence: remote
                .is_none()
                .then(|| crate::fence::Watcher::start(crate::fence::PROGRAM.into(), FENCE_REFRESH)),
            denials_all: false,
            corral,
            profiles: None,
            finding_profiles: None,
            // Another machine's home is known once it answers.
            home: match &remote {
                None => std::env::var("HOME").ok(),
                Some(_) => None,
            },
            remote,
            read_at: None,
            listed_at: None,
            elsewhere: Vec::new(),
            machines: Vec::new(),
            selected: None,
            on_show,
            here: Vec::new(),
            note: None,
            collapsed: false,
            fold: None,
            menu_open: false,
            asking: None,
            frosted: false,
            activity,
            workspaces_path,
            floating: None,
            floating_focus,
            floating_back: None,
            dismissed: None,
            cards: Rc::default(),
            _floating_out: floating_out,
        }
    }

    /// New colours or width from Settings, at once.
    pub fn restyle(&mut self, theme: Rc<Theme>, width: f32, cx: &mut Context<Self>) {
        self.theme = column_theme(&theme, self.frosted);
        self.given = theme;
        self.width = width;
        cx.notify();
    }

    /// The system's sidebar material shows through the column, or it is gone (full screen).
    pub fn set_frosted(&mut self, frosted: bool, cx: &mut Context<Self>) {
        self.frosted = frosted;
        self.theme = column_theme(&self.given, frosted);
        cx.notify();
    }

    /// The grounds laid on the column.
    fn grounds(&self) -> Grounds {
        Grounds::of(&self.theme, self.frosted)
    }

    /// A new terminal font from Settings, for the instance id.
    pub fn set_mono(&mut self, mono: Font, cx: &mut Context<Self>) {
        self.mono = mono;
        cx.notify();
    }

    /// What the window shows: the active pane's agent, and every agent open in it.
    pub fn set_view(
        &mut self,
        selected: Option<String>,
        here: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if self.selected != selected || self.here != here {
            self.selected = selected;
            self.here = here;
            cx.notify();
        }
    }

    /// This machine on show in the window, or waiting while another is.
    pub fn set_on_show(&mut self, on_show: bool) {
        self.on_show = on_show;
        // Shown again, its list is there as it is then.
        if !on_show {
            self.cards.borrow_mut().forget();
        }
        // The offer to switch to This Mac was for while this one showed.
        if !on_show && self.note.as_ref().is_some_and(|note| note.offer.is_some()) {
            self.note = None;
        }
    }

    /// Collapsed to the strip, or expanded.
    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.collapsed != collapsed {
            self.collapsed = collapsed;
            cx.notify();
        }
    }

    /// Sliding between the list and the strip, `fold` of the way to the strip, or still (`None`)
    /// (P5-148a). Drawn with the window, which says it each frame.
    pub fn set_fold(&mut self, fold: Option<f32>) {
        self.fold = fold;
    }

    /// The activity panel folded to one line, as the layout saved it.
    pub fn set_activity_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        self.activity
            .update(cx, |activity, cx| activity.set_folded(folded, cx));
        cx.notify();
    }

    /// The groups folded to their heading, as the layout saved them.
    pub fn set_folds(&mut self, folds: Folds, cx: &mut Context<Self>) {
        self.listing.panel.folded = folds;
        cx.notify();
    }

    /// Folds a group to its heading, or opens it again: the one in `workspace`, or in none.
    fn fold_group(&mut self, workspace: Option<&str>, group: &str, cx: &mut Context<Self>) {
        self.listing.panel.folded.toggle_group(workspace, group);
        self.folds_changed(cx);
    }

    /// Folds a workspace to its title, or opens it again.
    fn fold_workspace(&mut self, name: &str, cx: &mut Context<Self>) {
        self.listing.panel.folded.toggle_workspace(name);
        self.folds_changed(cx);
    }

    /// Folds the `Idle` shelf to its title, or opens it again (P5-134, P5-164).
    fn fold_idle(&mut self, cx: &mut Context<Self>) {
        let folded = &mut self.listing.panel.folded;
        folded.idle = !folded.idle;
        self.folds_changed(cx);
    }

    /// Opens out a workspace's one line of repositories with no agent, or puts them back in it.
    fn fold_vacant(&mut self, workspace: &str, cx: &mut Context<Self>) {
        self.listing.panel.folded.toggle_vacant(workspace);
        self.folds_changed(cx);
    }

    /// What is folded changed: the window saves it with the layout.
    fn folds_changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(SidebarEvent::Folds(self.listing.panel.folded.clone()));
        cx.notify();
    }

    /// Whether the menu of actions is open, for its button.
    pub fn set_menu_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.menu_open != open {
            self.menu_open = open;
            cx.notify();
        }
    }

    /// A line beside the footer's button about the last start or stop, in place of the one
    /// before. One that went well fades after a few seconds; a problem stays.
    pub fn note(&mut self, text: String, problem: bool, cx: &mut Context<Self>) {
        self.note = Some(Note::new(
            text,
            problem,
            Instant::now(),
            reduce_motion::on(),
        ));
        if !problem {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(NOTE_HOLD).await;
                this.update(cx, |_, cx| cx.notify()).ok();
            })
            .detach();
        }
        cx.notify();
    }

    /// What starts on This Mac alone was asked for while this machine is on show (P5-122): in
    /// place of the footer's note, the hint in neutral words over the footer with `Switch to This
    /// Mac`. It stays until the next note, its button, or another machine on show.
    pub fn offer(&mut self, only: Only, cx: &mut Context<Self>) {
        let mut note = Note::new(
            only.hint().into(),
            false,
            Instant::now(),
            reduce_motion::on(),
        );
        note.offer = Some(only);
        self.note = Some(note);
        cx.notify();
    }

    /// Lists the agents again now rather than at the next interval.
    pub fn refresh(&self) {
        self.poller.refresh();
    }

    /// The header row, in the title bar after the traffic lights (none in `full_screen`): the
    /// collapse button, `Agents` and how many, and at the right end `New Workspace…`, the button for
    /// every agent and the bell, compact when the row, measured as it is set, has no room for it
    /// as a pill; collapsed to the strip, only the expand button, where the collapse button was.
    /// `spot` goes in the bell, for the window to hang the Inbox from. A press on a button is not
    /// the start of a drag.
    pub fn head(
        &mut self,
        full_screen: bool,
        spot: AnyElement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let ui = UiFont::get(cx);
        let keep = |_: &MouseDownEvent, _: &mut Window, cx: &mut App| cx.stop_propagation();
        // Sliding to the strip or back, it keeps the expanded row, the rest of it fading (P5-148a).
        let strip = self.collapsed && self.fold.is_none();
        let gap = ui.px(if strip { TOGGLE_GAP } else { TITLE_GAP });
        let row = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .items_center()
            .child(
                self.collapse_button(self.collapsed, cx)
                    .map(move |button| button.mr(gap).on_mouse_down(MouseButton::Left, keep)),
            );
        if strip {
            return row;
        }
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        // Every agent, those of folded groups too.
        let agents = self.listing.panel.agents.len();
        let pause_all = self.pause_all_button(cx);
        let family = ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into());
        let set = |words: String, size: f32, weight: FontWeight, features: FontFeatures| {
            let run = TextRun {
                len: words.len(),
                font: Font {
                    weight,
                    features,
                    ..gpui::font(family.clone())
                },
                color: Hsla::default(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line = window
                .text_system()
                .shape_line(words.into(), ui.px(size), &[run], None);
            f32::from(line.width)
        };
        let (count, _) = self.bell();
        let words = HeadWords {
            title: set(
                "Agents".into(),
                TITLE_SIZE,
                FontWeight::SEMIBOLD,
                FontFeatures::default(),
            ),
            count: (agents > 0).then(|| {
                set(
                    agents.to_string(),
                    COUNT_SIZE,
                    FontWeight::NORMAL,
                    tabular(),
                )
            }),
            pause_all: pause_all.is_some(),
            bell: (count > 0).then(|| {
                set(
                    count.to_string(),
                    NOTE_SIZE,
                    FontWeight::SEMIBOLD,
                    FontFeatures::default(),
                )
            }),
        };
        let compact = compact_bell(self.width, full_screen, &ui, &words);
        let bell = if compact {
            self.small_bell((COMPACT_BELL, COMPACT_BELL), Some(spot), cx)
        } else {
            self.badge(spot, cx)
        };
        let pause_gap = ui.px(PAUSE_GAP);
        // Another machine's agents are in no workspace.
        let new_workspace = self.remote.is_none().then(|| {
            self.new_workspace_button(cx)
                .mr(pause_gap)
                .on_mouse_down(MouseButton::Left, keep)
        });
        let pause_all = pause_all.map(|button| {
            button.map(move |button| button.mr(pause_gap).on_mouse_down(MouseButton::Left, keep))
        });
        // All after the button, put in `into`; sliding, it fades together as one.
        let fill = move |into: Div| {
            into.child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui.px(COUNT_GAP))
                    .child(
                        div()
                            .text_size(ui.px(TITLE_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg(|t| t.agents_text))
                            .child("Agents"),
                    )
                    .when(agents > 0, |title| {
                        title.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h(ui.px(COUNT_HEIGHT))
                                .min_w(ui.px(COUNT_HEIGHT))
                                .px(ui.px(COUNT_X))
                                .rounded(ui.px(COUNT_HEIGHT / 2.0))
                                .bg(fg(|t| t.agents_text).opacity(0.08))
                                .text_size(ui.px(COUNT_SIZE))
                                .line_height(ui.px(COUNT_HEIGHT))
                                .font_features(tabular())
                                .text_color(fg(|t| t.agents_dim))
                                .child(agents.to_string()),
                        )
                    }),
            )
            .child(div().flex_1())
            .children(new_workspace)
            .children(pause_all)
            .child(bell.map(move |bell| bell.on_mouse_down(MouseButton::Left, keep)))
        };
        match self.fold {
            None => fill(row),
            Some(folded) => row.child(
                fill(div().flex_1().min_w(px(0.0)).h_full().flex().items_center())
                    .opacity(1.0 - folded),
            ),
        }
    }

    /// The button for every agent, before the bell and in the strip under it: it pauses them all
    /// while any live agent is not paused, and resumes them once all are; `None` with no live
    /// agent. Its bars dip, or its triangle slides on, as the pointer comes in.
    fn pause_all_button(&self, cx: &mut Context<Self>) -> Option<HoverMotion> {
        let pause = pause::all(&self.listing.panel.agents)?;
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let selected = self.grounds().selected;
        let on_click = cx.listener(|this, _: &ClickEvent, _, cx| this.ask_pause_all(cx));
        let size = (PAUSE_ALL, PAUSE_ALL, 7.0);
        Some(if pause {
            motion::hover_motion("pause-all-motion", motion::DIP, move |hover| {
                button(
                    (&theme, selected, |t| t.agents_dim),
                    &ui,
                    "pause-all",
                    (Icon::Pause, motion::dip(hover.play), footer_icon::SIZE),
                    "Pause all agents",
                    size,
                    false,
                )
                .on_click(on_click)
            })
        } else {
            motion::hover_motion("resume-all-motion", motion::SHIFT, move |hover| {
                button(
                    (&theme, selected, |t| t.agents_dim),
                    &ui,
                    "resume-all",
                    (
                        Icon::Resume,
                        motion::shift(hover.play, true),
                        footer_icon::SIZE,
                    ),
                    "Resume all agents",
                    size,
                    false,
                )
                .on_click(on_click)
            })
        })
    }

    /// The button that opens the `New Workspace…` panel, nothing ticked, as the View menu does:
    /// always there, before the button for every agent and in the strip under it (P5-92).
    fn new_workspace_button(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let ui = UiFont::get(cx);
        let selected = self.grounds().selected;
        button(
            (&self.theme, selected, |t| t.agents_dim),
            &ui,
            "new-workspace",
            (Icon::NewWorkspace, Pose::Rest, footer_icon::SIZE),
            "New Workspace…",
            (NEW_WORKSPACE, NEW_WORKSPACE, 7.0),
            false,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
            this.new_workspace(Default::default(), window, cx)
        }))
    }

    /// The button for every agent was clicked (see [`pause::every`]).
    fn ask_pause_all(&mut self, cx: &mut Context<Self>) {
        if let Some(request) = pause::every(&self.listing.panel.agents) {
            cx.emit(SidebarEvent::Pause(request));
        }
    }

    /// Whether corral last said `name` is paused.
    pub fn paused(&self, name: &str) -> bool {
        self.listing
            .panel
            .agents
            .iter()
            .any(|a| a.name == name && a.paused)
    }

    /// The header's bell, always there so the Inbox has a way in: how many rows its first two
    /// groups have, in amber when an agent needs a person, in the accent for new replies only,
    /// quiet and without a number when there is nothing. A click opens the Inbox.
    fn badge(&self, spot: AnyElement, cx: &mut Context<Self>) -> HoverMotion {
        let (count, color) = self.bell();
        let ui = UiFont::get(cx);
        let on_click = cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Attention));
        motion::hover_motion("attention-motion", motion::RING, move |hover| {
            let mut pill = div()
                .flex()
                .items_center()
                .gap(ui.px(5.0))
                .h(ui.px(24.0))
                .px(ui.px(8.0))
                .rounded(ui.px(12.0))
                .text_color(color)
                .text_size(ui.px(NOTE_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(bell_icon(&ui, color, hover));
            if count > 0 {
                pill = pill.bg(color.opacity(0.15)).child(count.to_string());
            }
            div()
                .id("attention")
                .relative()
                .flex_shrink_0()
                .h(ui.px(28.0))
                .flex()
                .items_center()
                .cursor_pointer()
                .hover(move |style| style.opacity(0.85))
                .child(pill)
                .child(spot)
                .on_click(on_click)
        })
    }

    /// How many rows the Inbox has under `Needs you` and `New replies`, and the bell's colour for
    /// them.
    fn bell(&self) -> (usize, Hsla) {
        let mut ringing = self.ringing();
        ringing.extend(self.elsewhere.iter().cloned());
        let (count, urgency) = attention::bell(&ringing);
        let color = hsla(
            match urgency {
                attention::Urgency::Needs => self.theme.fg(|t| t.agents_yellow),
                attention::Urgency::Replies => self.theme.fg(|t| t.agents_accent),
                attention::Urgency::Quiet => self.theme.fg(|t| t.agents_dim),
            },
            1.0,
        );
        (count, color)
    }

    /// The strip's bell: the icon, and when the Inbox has rows that ring it, their count beside it
    /// in a small pill tinted in the bell's colour, never over the icon.
    fn rail_bell(&self, cx: &mut Context<Self>) -> HoverMotion {
        let (count, color) = self.bell();
        let ui = UiFont::get(cx);
        let selected = self.grounds().selected;
        let on_click = cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Attention));
        motion::hover_motion("attention-motion", motion::RING, move |hover| {
            div()
                .id("attention")
                .flex_shrink_0()
                .h(ui.px(RAIL_BELL))
                .min_w(ui.px(RAIL_BELL))
                .px(ui.px(RAIL_BELL_X))
                .flex()
                .items_center()
                .justify_center()
                .gap(ui.px(RAIL_BELL_GAP))
                .rounded(ui.px(8.0))
                .cursor_pointer()
                .text_color(color)
                .text_size(ui.px(NOTE_SIZE))
                .font_weight(FontWeight::BOLD)
                .font_features(tabular())
                .child(bell_icon(&ui, color, hover))
                .map(|bell| {
                    if count > 0 {
                        bell.bg(color.opacity(0.14))
                            .hover(move |style| style.bg(color.opacity(0.22)))
                            .child(count.to_string())
                    } else {
                        bell.hover(move |style| style.bg(selected))
                    }
                })
                .on_click(on_click)
        })
    }

    /// The bell as an icon with the count in a small disc on its corner, `(width, height)` in
    /// points: the header's in a narrow sidebar. `spot` as in [`Self::head`].
    fn small_bell(
        &self,
        (width, height): (f32, f32),
        spot: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> HoverMotion {
        let (count, color) = self.bell();
        let ui = UiFont::get(cx);
        let ground = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let selected = self.grounds().selected;
        let on_click = cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Attention));
        motion::hover_motion("attention-motion", motion::RING, move |hover| {
            div()
                .id("attention")
                .relative()
                .flex_shrink_0()
                .w(ui.px(width))
                .h(ui.px(height))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(7.0))
                .cursor_pointer()
                .hover(move |style| style.bg(selected))
                .child(bell_icon(&ui, color, hover))
                .when(count > 0, |bell| {
                    bell.child(
                        div()
                            .absolute()
                            .top(ui.px(3.0))
                            .right(ui.px(3.0))
                            .min_w(ui.px(13.0))
                            .h(ui.px(13.0))
                            .px(ui.px(3.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(color)
                            .text_color(ground)
                            .text_size(ui.px(BADGE_SIZE))
                            .font_weight(FontWeight::BOLD)
                            .child(count.to_string()),
                    )
                })
                .children(spot)
                .on_click(on_click)
        })
    }

    /// The footer's button for the menu of actions, lit while the menu is open; its knobs slide
    /// as the pointer comes in.
    fn actions_button(&self, cx: &mut Context<Self>) -> HoverMotion {
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let selected = self.grounds().selected;
        let lit = self.menu_open;
        let on_click = cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Actions));
        motion::hover_motion("actions-motion", motion::SLIDE, move |hover| {
            button(
                (&theme, selected, |t| t.agents_dim),
                &ui,
                "actions",
                (Icon::Actions, motion::slide(hover.play), BUTTON_ICON),
                "Agent actions and settings",
                (BUTTON, BUTTON, 8.0),
                lit,
            )
            .on_click(on_click)
        })
    }

    /// The button that collapses the sidebar (`expand` false) or expands the strip, one size
    /// either way so it stays put in the title bar; as the pointer comes in its edge slides the
    /// way the click goes, out to collapse and in to expand.
    fn collapse_button(&self, expand: bool, cx: &mut Context<Self>) -> HoverMotion {
        let ui = UiFont::get(cx);
        let tip = if expand {
            "Expand sidebar (⌘B)"
        } else {
            "Collapse sidebar (⌘B)"
        };
        // Expanding, it stands on the title bar's opaque row, in its buttons' colours.
        let (theme, selected, ink) = if expand {
            (
                self.given.clone(),
                Grounds::of(&self.given, false).selected,
                (|t| t.muted) as Pick,
            )
        } else {
            (
                self.theme.clone(),
                self.grounds().selected,
                (|t| t.agents_dim) as Pick,
            )
        };
        let on_click =
            cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::ToggleCollapse));
        motion::hover_motion("collapse-motion", motion::SHIFT, move |hover| {
            button(
                (&theme, selected, ink),
                &ui,
                "collapse",
                (
                    Icon::LeftSidebar,
                    motion::shift(hover.play, expand),
                    footer_icon::SIZE,
                ),
                tip,
                (TOGGLE, TOGGLE, 6.0),
                false,
            )
            .on_click(on_click)
        })
    }

    /// Sliding between the list and the strip (P5-148a), `folded` of the way to the strip: as wide
    /// as it has got, the list fading out over the strip fading in as it folds, and back as it
    /// unfolds, each laid out as when still and cut where the sidebar ends.
    fn folding(&mut self, folded: f32, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let rail = UiFont::get(cx).scale(RAIL);
        let list = self.expanded(window, cx);
        let strip = self.rail(window, cx);
        let layer = |id: &'static str, width: f32, shown: f32, child: AnyElement| {
            div()
                .id(id)
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .w(px(width))
                .opacity(shown)
                .child(child)
        };
        div()
            .flex_shrink_0()
            .relative()
            .h_full()
            .w(px(self.width + (rail - self.width) * folded))
            .overflow_hidden()
            .child(layer("folding-list", self.width, 1.0 - folded, list))
            .child(layer("folding-strip", rail, folded, strip))
            .into_any_element()
    }

    /// The collapsed strip: the bell, the button for every agent and `New Workspace…`, then a tile
    /// for each agent, the projects set apart by short rules, and the menu button at the bottom.
    fn rail(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let grounds = self.grounds();
        let rule = |width: f32, y: f32| {
            div()
                .flex_shrink_0()
                .w(ui.px(width))
                .h(px(1.0))
                .my(ui.px(y))
                .bg(grounds.rule)
        };
        let edge_rule = hsla(self.theme.fg(|t| t.agents_dimmer), 0.7);
        let lines = self.listing.unfolded(
            self.selected.as_deref(),
            &self.here,
            self.home.as_deref(),
            now(),
        );
        let mut tiles = div()
            .id("rail")
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .pt(ui.px(4.0))
            .pb(ui.px(4.0));
        // Between groups a short rule; a longer, brighter one where a workspace starts or the
        // last one ends.
        let mut first = true;
        let mut last: Option<Option<String>> = None;
        for line in lines {
            match line {
                // Workspaces with no agent and repositories with none take no room (P5-134).
                Line::Error(_)
                | Line::Workspace(_)
                | Line::Vacant { .. }
                | Line::Vacancies { .. }
                | Line::Empty(_)
                | Line::Idle { .. }
                | Line::IdleWorkspace { .. } => {}
                Line::Group(heading) => {
                    let edge = last.as_ref().is_some_and(|last| *last != heading.workspace);
                    if !first && edge {
                        tiles = tiles.child(rule(28.0, 7.0).h(px(2.0)).bg(edge_rule));
                    } else if !first {
                        tiles = tiles.child(rule(16.0, 6.0));
                    }
                    last = Some(heading.workspace);
                }
                Line::Agent(card) => {
                    first = false;
                    let on_click = {
                        let card = card.clone();
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.click(&card, cx))
                    };
                    tiles = tiles.child(Tile {
                        // Its hover note lies on its own opaque ground: the given colours.
                        tip: RailTip::of(&card, &self.given, &ui),
                        card: *card,
                        theme: theme.clone(),
                        on_click: Box::new(on_click),
                        grounds,
                    });
                }
            }
        }
        div()
            .flex_shrink_0()
            .w(ui.px(RAIL))
            .h_full()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .line_height(relative(1.3))
            .pt(ui.px(8.0))
            // `New Workspace…` from the View menu or the palette opens over the strip too.
            .children(self.editor(window, cx))
            .children(self.switcher(true, cx))
            .child(self.rail_bell(cx).map({
                let top = ui.px(2.0);
                move |bell| bell.mt(top)
            }))
            .children(self.pause_all_button(cx).map(|button| {
                let top = ui.px(4.0);
                button.map(move |button| button.mt(top))
            }))
            .when(self.remote.is_none(), |rail| {
                rail.child(self.new_workspace_button(cx).mt(ui.px(4.0)))
            })
            .child(rule(24.0, 0.0).mt(ui.px(8.0)).mb(ui.px(4.0)))
            .child(tiles)
            .child(
                div()
                    .flex_shrink_0()
                    .h(ui.px(FOOTER))
                    .flex()
                    .items_center()
                    .child(self.actions_button(cx)),
            )
            .into_any_element()
    }

    /// The latest `corral ls`: when, and how many agents or why it failed.
    pub fn last_read(&self) -> crate::diagnostics::Last {
        self.listing.last.clone()
    }

    /// The agents as corral last listed them.
    pub fn agents(&self) -> Vec<Agent> {
        self.listing.panel.agents.clone()
    }

    /// When the read of [`Self::agents`] began; none before the first.
    pub fn listed_at(&self) -> Option<std::time::Instant> {
        self.listed_at
    }

    pub fn panel(&self) -> &Panel {
        &self.listing.panel
    }

    /// The agents as the Kanban tab sees them: with their status as the cards show it and, while
    /// idle, the last reply read.
    pub fn seen(&self) -> Vec<crate::kanban::Seen> {
        let (panel, now) = (&self.listing.panel, now());
        panel
            .agents
            .iter()
            .map(|a| crate::kanban::Seen::of(a, panel.status(a, now), panel.reply(a, now)))
            .collect()
    }

    /// What needs looking at now, for the tabs' marks.
    pub fn attention(&self) -> Vec<attention::Item> {
        attention::items(&self.listing.panel, self.listing.error.as_deref(), now())
    }

    /// Why the latest corral read failed, for the Inbox's row about it.
    pub fn read_error(&self) -> Option<&str> {
        self.listing.error.as_deref()
    }

    /// What rings the bell from these agents: the Inbox's first two groups.
    pub fn ringing(&self) -> Vec<attention::Item> {
        inbox::ringing(&self.listing.panel, self.listing.error.as_deref(), now())
    }

    /// What rings the bell on the other machines (DESIGN §13 P5-90): the bell counts it too.
    pub fn set_elsewhere(&mut self, items: Vec<attention::Item>, cx: &mut Context<Self>) {
        if self.elsewhere != items {
            self.elsewhere = items;
            cx.notify();
        }
    }

    /// The machine these agents run on, when not this Mac.
    pub fn remote(&self) -> Option<&Arc<crate::remote::Link>> {
        self.remote.as_ref()
    }

    /// The switcher's machines, as the window has them now.
    pub fn set_machines(&mut self, machines: Vec<MachineTab>, cx: &mut Context<Self>) {
        if self.machines != machines {
            self.machines = machines;
            cx.notify();
        }
    }

    /// The switcher over the list, a cell per machine, as wide as its words and from the left,
    /// in line with the workspaces and cards below (P5-139): its name, how many of its agents want
    /// the user, and how its connection stands, why on hover; the machine on show on a faint
    /// accent, no more. Too many for one row, they go on in the next. A click shows that machine,
    /// and tries one that could not be read again. Narrow, on the strip, a column of each
    /// machine's first letters. None without other machines.
    fn switcher(&self, narrow: bool, cx: &mut Context<Self>) -> Option<Div> {
        if self.machines.is_empty() {
            return None;
        }
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let (text, dim, dimmer, amber, red, green) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_yellow),
            fg(|t| t.agents_red),
            fg(|t| t.agents_green),
        );
        let (bright, accent) = (fg(|t| t.bright), fg(|t| t.agents_accent));
        let grounds = self.grounds();
        let selected = grounds.selected;
        let mut bar = div().flex_shrink_0().flex();
        bar = if narrow {
            bar.items_center().gap(ui.px(2.0)).flex_col().pb(ui.px(4.0))
        } else {
            bar.flex_wrap()
                .items_center()
                .gap(ui.px(4.0))
                .text_size(ui.px(12.5))
        };
        for (index, machine) in self.machines.iter().enumerate() {
            let tip = Tip {
                text: machine.tip.clone().into(),
                size: ui.px(NOTE_SIZE),
                color: text,
                background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
                border: fg(|t| t.agents_rule),
            };
            let on_click =
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Machine(index)));
            // Another machine's capsule, right-clicked: its menu, hanging from it (P5-123).
            let menu = machine.id.clone().map(|id| {
                let open = matches!(&self.floating, Some(Floating::Machine(open)) if *open == id);
                let hanging =
                    open.then(|| Self::hung(self.machine_menu(&id, &machine.name, &ui, cx), true));
                let press = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.press_menu(Floating::Machine(id.clone()), event, window, cx);
                });
                (hanging, press)
            });
            let (hanging, press) = menu.unzip();
            let hanging = hanging.flatten();
            let attention = machine.attention.as_ref().map(|attention| {
                let color = match attention.kind {
                    attention::Kind::Error | attention::Kind::ReadFailed => red,
                    attention::Kind::Waiting => amber,
                    attention::Kind::Reply => accent,
                };
                div()
                    .text_size(ui.px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .font_features(tabular())
                    .text_color(color)
                    .child(format!("{}{}", attention.kind.mark(), attention.count))
            });
            if narrow {
                bar = bar.child(
                    div()
                        .id(("machine", index))
                        .flex()
                        .items_center()
                        .gap(ui.px(5.0))
                        .h(ui.px(24.0))
                        .px(ui.px(8.0))
                        .rounded(ui.px(6.0))
                        .cursor_pointer()
                        .whitespace_nowrap()
                        .text_size(ui.px(12.5))
                        .text_color(if machine.shown { text } else { dim })
                        .when(machine.shown, |tab| {
                            tab.bg(selected).font_weight(FontWeight::SEMIBOLD)
                        })
                        .hover(move |style| style.bg(selected))
                        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                        .on_click(on_click)
                        .when_some(press, |tab, press| {
                            tab.relative().on_mouse_down(MouseButton::Right, press)
                        })
                        .children(hanging)
                        .child(machine.name.chars().take(2).collect::<String>())
                        // Another machine answering: a dot; not answering, a dot for what stands.
                        .when(index > 0, |tab| {
                            let color = match machine.status {
                                None => green,
                                Some(_) if machine.problem => red,
                                Some(_) => dim,
                            };
                            tab.child(div().size(ui.px(6.0)).rounded_full().bg(color))
                        })
                        .children(attention),
                );
                continue;
            }
            let on = machine.shown;
            let status_color = if machine.problem { red } else { dimmer };
            bar = bar.child(
                div()
                    .id(("machine", index))
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .h(ui.px(SWITCH_CELL))
                    .px(ui.px(10.0))
                    .rounded(ui.px(SWITCH_RADIUS))
                    .cursor_pointer()
                    .whitespace_nowrap()
                    .text_color(if on { bright } else { dim })
                    .when(on, |cell| {
                        cell.bg(accent.opacity(sunk::SHOWN_FILL))
                            .font_weight(FontWeight::MEDIUM)
                    })
                    .hover(move |style| style.text_color(bright))
                    .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                    .on_click(on_click)
                    .when_some(press, |cell, press| {
                        cell.relative().on_mouse_down(MouseButton::Right, press)
                    })
                    .children(hanging)
                    // A long name cut short; the hover note has it whole (P5-123).
                    .child(
                        div()
                            .min_w(px(0.0))
                            .max_w(ui.px(CAPSULE_NAME))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(machine.name.clone()),
                    )
                    // Another machine answering: a dot; not answering, what stands.
                    .when(index > 0 && machine.status.is_none(), |pill| {
                        pill.child(div().size(ui.px(6.0)).rounded_full().bg(green))
                    })
                    .children(machine.status.clone().map(|status| {
                        div()
                            .text_size(ui.px(11.0))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(status_color)
                            .child(status)
                    }))
                    // Another machine's agents after that, on a count's tag as the Cairn switch
                    // has them (P5-122 rework 1, 3).
                    .children(machine.agents.map(|agents| sunk::count(agents, text, &ui)))
                    .children(attention),
            );
        }
        Some(if narrow {
            bar
        } else {
            div()
                .flex_shrink_0()
                .px(px(PAD))
                .pt(ui.px(8.0))
                .pb(ui.px(8.0))
                .child(bar)
        })
    }

    /// Another machine's capsule menu (P5-123): New Agent on it first (P5-124), which shows it and
    /// opens its New Agent dialog; Rename… and Remove… go on in Settings › Machines, Hide hides it
    /// at once; Machines Settings… opens that page.
    fn machine_menu(
        &self,
        id: &str,
        name: &str,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let (new_agent, rename, hide, remove, settings) = (
            Self::machine_ask(id, MachineAsk::NewAgent, cx),
            Self::machine_ask(id, MachineAsk::Rename, cx),
            Self::machine_ask(id, MachineAsk::Hide, cx),
            Self::machine_ask(id, MachineAsk::Remove, cx),
            Self::machine_ask(id, MachineAsk::Settings, cx),
        );
        let theme = &*self.theme;
        let row = |id: &'static str, label: String, tone| {
            crate::popover::text_row(theme, ui, id, label, tone)
        };
        self.floated(crate::popover::panel(theme, ui), "machine-menu", cx)
            .min_w(ui.px(MACHINE_MENU.0))
            .max_w(ui.px(MACHINE_MENU.1))
            .mt(ui.px(4.0))
            .child(
                row(
                    "machine-new-agent",
                    format!("New Agent on {name}…"),
                    crate::popover::Tone::Plain,
                )
                .on_click(new_agent),
            )
            .child(crate::popover::rule(theme, ui))
            .child(
                row(
                    "machine-rename",
                    "Rename…".into(),
                    crate::popover::Tone::Plain,
                )
                .on_click(rename),
            )
            .child(
                row(
                    "machine-hide",
                    format!("Hide {name}"),
                    crate::popover::Tone::Plain,
                )
                .on_click(hide),
            )
            .child(
                row(
                    "machine-remove",
                    format!("Remove {name}…"),
                    crate::popover::Tone::Danger,
                )
                .on_click(remove),
            )
            .child(crate::popover::rule(theme, ui))
            .child(
                row(
                    "machine-settings",
                    "Machines Settings…".into(),
                    crate::popover::Tone::Plain,
                )
                .on_click(settings),
            )
    }

    /// A click on a capsule menu's row: the menu closes and the window is asked.
    fn machine_ask(
        machine: &str,
        ask: MachineAsk,
        cx: &mut Context<Self>,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static {
        let machine = machine.to_owned();
        cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.close_floating(window, cx);
            cx.emit(SidebarEvent::MachineMenu {
                machine: machine.clone(),
                ask,
            });
        })
    }

    /// Over the list of another machine's agents while it cannot be read (P5-122): what stands,
    /// with its name; how long ago its agents were read, and what to do; `Try again`, as a click on
    /// it in the switcher. The cards under it are dimmed.
    fn banner(&self, error: &str, now: f64, cx: &mut Context<Self>) -> Div {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let (red, dim, text) = (
            fg(|t| t.agents_red),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_text),
        );
        let shown = self.machines.iter().position(|machine| machine.shown);
        let name = self
            .remote
            .as_ref()
            .map(|link| link.label())
            .unwrap_or_default();
        // Down, as the switcher says; else corral there failed: why.
        let (headline, step) = match shown.and_then(|index| self.machines[index].trouble.clone()) {
            Some(trouble) => trouble,
            None => (
                format!("Can't read {name}"),
                error.strip_prefix("corral: ").unwrap_or(error).to_owned(),
            ),
        };
        let read = self.read_at.map(|at| {
            format!(
                "Showing what was read {} ago.",
                card::short_time(Some(now - at))
            )
        });
        let body = read.into_iter().chain([step]).collect::<Vec<_>>().join(" ");
        let again = shown.map(|index| {
            div()
                .id("machine-try-again")
                .flex_shrink_0()
                .flex()
                .items_center()
                .h(ui.px(26.0))
                .px(ui.px(12.0))
                .rounded(ui.px(7.0))
                .bg(text.opacity(0.09))
                .border_1()
                .border_color(text.opacity(0.14))
                .text_color(text)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(move |style| style.bg(text.opacity(0.14)))
                .child("Try again")
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    cx.emit(SidebarEvent::Machine(index))
                }))
        });
        div()
            .flex_shrink_0()
            .flex()
            .items_start()
            .gap(ui.px(8.0))
            .px(ui.px(10.0))
            .py(ui.px(8.0))
            .rounded(ui.px(9.0))
            .bg(red.opacity(0.09))
            .border_1()
            .border_color(red.opacity(0.32))
            .text_size(ui.px(SECOND_SIZE))
            .line_height(relative(1.45))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_color(red)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(headline),
                    )
                    .child(div().text_color(dim).child(body)),
            )
            .children(again)
    }

    /// The offer over the footer (P5-122): the hint about what starts on This Mac alone, and
    /// `Switch to This Mac`, which shows This Mac and then does it.
    fn offer_card(&self, only: Only, cx: &mut Context<Self>) -> Div {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.given.fg(pick), 1.0);
        let (text, accent) = (fg(|t| t.agents_text), fg(|t| t.agents_accent));
        let switch = div()
            .id("switch-to-this-mac")
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(ui.px(26.0))
            .px(ui.px(12.0))
            .rounded(ui.px(7.0))
            .bg(accent.opacity(0.14))
            .border_1()
            .border_color(accent.opacity(0.4))
            .text_color(accent)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(move |style| style.bg(accent.opacity(0.22)))
            .child("Switch to This Mac")
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.note = None;
                cx.emit(SidebarEvent::ThisMac(only));
                cx.notify();
            }));
        // On its own opaque ground: it lies over the list's end.
        div()
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .mx(px(PAD))
            .mb(ui.px(6.0))
            .px(ui.px(10.0))
            .py(ui.px(8.0))
            .rounded(ui.px(9.0))
            .bg(hsla(self.given.bg(|t| t.agents_bg), 1.0).blend(text.opacity(0.06)))
            .border_1()
            .border_color(text.opacity(0.12))
            .shadow_md()
            .text_size(ui.px(SECOND_SIZE))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_color(fg(|t| t.agents_dim))
                    .child(only.hint()),
            )
            .child(switch)
    }

    /// Another machine not read yet (P5-122): `Connecting to <machine>…` and grey cards in the
    /// place of its agents.
    fn connecting(&self, cx: &mut Context<Self>) -> Div {
        let ui = UiFont::get(cx);
        let name = self
            .remote
            .as_ref()
            .map(|link| link.label())
            .unwrap_or_default();
        let card = self.grounds().card;
        let mut list = div().flex().flex_col().gap(px(CARD_GAP)).child(
            div()
                .px(px(8.0))
                .pt(px(4.0))
                .pb(px(2.0))
                .text_size(ui.px(SECOND_SIZE))
                .text_color(hsla(self.theme.fg(|t| t.agents_dim), 1.0))
                .child(format!("Connecting to {name}…")),
        );
        for opacity in PLACEHOLDERS {
            list = list.child(
                div()
                    .h(ui.px(AVATAR) + px(2.0 * (CARD_Y + 1.0)))
                    .rounded(px(CARD_RADIUS))
                    .bg(card)
                    .opacity(opacity),
            );
        }
        list
    }

    /// The Inbox shows `row`'s reply: it is read, for the bell, its card and its tab.
    pub fn read(&mut self, row: &inbox::Row, cx: &mut Context<Self>) {
        if inbox::read(&mut self.listing.panel, row) {
            cx.notify();
        }
    }

    /// The agents' directories, sorted and without repeats.
    pub fn projects(&self) -> Vec<String> {
        self.listing.cwds()
    }

    /// Whether every card is folded, and sorted by name, for the View menu's ticks.
    pub fn view_state(&self) -> (bool, bool) {
        (
            self.listing.panel.expanded.is_empty(),
            self.listing.panel.by_name,
        )
    }

    /// Closes every card's details (View ▸ Fold Agents, and the footer).
    pub fn toggle_fold(&mut self, cx: &mut Context<Self>) {
        self.listing.panel.collapse_all();
        cx.notify();
    }

    pub fn toggle_sort(&mut self, cx: &mut Context<Self>) {
        self.listing.panel.by_name = !self.listing.panel.by_name;
        cx.notify();
    }

    /// Sorts by name, or by status (the menu's choice).
    pub fn set_sort(&mut self, by_name: bool, cx: &mut Context<Self>) {
        self.listing.panel.by_name = by_name;
        cx.notify();
    }

    /// Every agent corral lists, by name.
    pub fn agent_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .listing
            .panel
            .agents
            .iter()
            .map(|a| a.name.clone())
            .collect();
        names.sort();
        names
    }

    /// The agent's public role label, for its pane's title.
    pub fn role(&self, name: &str) -> Option<Role> {
        self.listing
            .panel
            .agents
            .iter()
            .find(|a| a.name == name)
            .and_then(Agent::role)
    }

    /// What attaching to `name` needs to check it is still the same agent.
    pub fn metadata(&self, name: &str) -> AgentMetadata {
        let agent = self.listing.panel.agents.iter().find(|a| a.name == name);
        AgentMetadata {
            cwd: agent.and_then(|a| a.cwd.clone()),
            instance: agent.and_then(|a| a.instance.clone()),
        }
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let now = now();
        let mut changed = false;
        for (began, update) in self.poller.updates.try_iter().collect::<Vec<_>>() {
            changed = true;
            let read = update.is_ok();
            if read {
                self.read_at = Some(now);
                self.listed_at = Some(began);
            }
            let looking = looking_at(self.selected.as_deref(), self.on_show);
            if let Some(alive) = self.listing.absorb(update, looking, now) {
                cx.emit(SidebarEvent::Alive(alive));
            }
            if read {
                // A Delegates pick saved after this read began stands over what it says.
                let panel = &mut self.listing.panel;
                panel.delegates.settle(&mut panel.agents, began);
            }
        }
        if let Some(link) = &self.remote {
            // Another machine: its paths written `~` by its own home, nothing read here.
            if changed {
                self.home = link.snapshot().home;
                self.listing.ask_replies(&self.replies, now);
            }
        } else if changed {
            // As often as corral is read: another window may have changed the workspaces.
            self.reload_workspaces();
            if let Some(git) = &mut self.git {
                git.watch(self.listing.cwds());
            }
            if let Some(fence) = &mut self.fence {
                let runs = sandbox_view::runs(&self.listing.panel.agents);
                self.listing.panel.fence.keep(&runs);
                fence.watch(runs);
            }
            let cwds = self.listing.cwds();
            self.activity.update(cx, |activity, _| activity.watch(cwds));
            self.listing.ask_replies(&self.replies, now);
        }
        if let Some(git) = &self.git {
            for batch in git.updates.try_iter().collect::<Vec<_>>() {
                changed = true;
                self.listing.absorb_git(batch);
            }
        }
        if let Some(fence) = &self.fence {
            for update in fence.updates.try_iter().collect::<Vec<_>>() {
                changed = true;
                self.listing.panel.fence.absorb(update);
            }
        }
        if changed && self.remote.is_none() {
            self.adopt_named(cx);
        }
        changed |= self.listing.absorb_replies(&self.replies);
        // Working agents' ages tick; their dots breathe by themselves.
        if changed || self.listing.animating(now) {
            cx.notify();
        }
    }

    /// A group's heading, the `index`th line of the list: its name, how many agents and, at the
    /// right, how they are doing. A click folds the group to this line, or opens it again
    /// (P5-89): the arrow after the count says which, shown while the group is folded or the
    /// pointer is over the heading, its room always kept. Before the bar, while the pointer is
    /// over it, the button that puts its repository in a workspace, or moves it to another or out.
    /// `above` is the room wanted over its words, past the gap between lines.
    fn heading(
        &self,
        heading: card::Heading,
        index: usize,
        above: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let hover: SharedString = format!("group-{index}").into();
        let (button, menu) = match self.remote {
            None => self.group_button(&heading, &hover, cx).unzip(),
            Some(_) => (None, None),
        };
        let card::Heading {
            group,
            title,
            count,
            bar,
            folded,
            showing,
            stack,
            workspace,
            ..
        } = heading;
        // In a workspace's tray its sides are the tray's.
        let side = if workspace.is_some() { 4.0 } else { 8.0 };
        let lit = fg(|t| t.agents_text).opacity(0.06);
        let (dim, dimmer) = (fg(|t| t.agents_dim), fg(|t| t.agents_dimmer));
        // The agent the active pane shows is folded away in it.
        let showing = folded && showing;
        // Folded to this line, its agents' cards stacked before its name, as a folded workspace's
        // title has them (P5-128); open, no icon.
        let stack = (folded && !stack.is_empty()).then(|| self.stack_mark(&stack, &ui));
        let on_click = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.fold_group(workspace.as_deref(), &group, cx)
        });
        let row = div()
            .id(("group", index))
            .group(hover.clone())
            .relative()
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .px(px(side))
            .py(px(HEADING_Y))
            .rounded(px(7.0))
            .cursor_pointer()
            .hover(move |style| style.bg(lit))
            .on_click(on_click)
            .children(stack)
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(ui.px(6.0))
                    .text_size(ui.px(GROUP_NAME_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(
                        div()
                            .text_color(if showing { dim } else { dimmer })
                            .group_hover(hover.clone(), move |style| style.text_color(dim))
                            .child(title.trim_end_matches('/').to_owned()),
                    )
                    .child(
                        div()
                            .text_size(ui.px(GROUP_COUNT_SIZE))
                            .text_color(dimmer.opacity(0.6))
                            .child(count.to_string()),
                    ),
            )
            .child(
                svg()
                    .data(if folded { CHEVRON_RIGHT } else { CHEVRON_DOWN })
                    .flex_shrink_0()
                    .size(ui.px(CHEVRON))
                    .text_color(dimmer)
                    .when(!folded, |arrow| {
                        arrow
                            .opacity(0.0)
                            .group_hover(hover, |style| style.opacity(1.0))
                    }),
            )
            // Hollow, as the mark on the card of an agent open in this window.
            .when(showing, |row| {
                row.child(
                    div()
                        .flex_shrink_0()
                        .size(ui.px(HERE))
                        .rounded_full()
                        .border(ui.px(HERE_RING))
                        .border_color(fg(|t| t.agents_accent)),
                )
            })
            .child(div().flex_1())
            .children(button)
            // How the group's agents are doing, a short stroke each.
            .child(workspaces_view::bar_of(&self.theme, &ui, bar))
            .children(menu.flatten());
        // `above` under the line over it, the gap between lines apart; down to its own first
        // card, only that gap. The row's own reach above and below its words is taken out of both.
        div()
            .pt(px((above - HEADING_Y).max(0.0)))
            .mb(px(-HEADING_Y))
            .child(row)
            .into_any_element()
    }

    /// A click on a card: show the agent, or, when it is already the one shown, open or close its
    /// details.
    fn click(&mut self, card: &Card, cx: &mut Context<Self>) {
        match card::click(card) {
            Click::Open => show(card, cx),
            Click::Details => {
                self.listing.panel.toggle_details(&card.name);
                cx.notify();
            }
        }
    }

    /// What a card's buttons do: Reply shows the agent, Copy copies its instance, Stop… asks the
    /// window to stop it. Each keeps the click from the card.
    fn card_actions(&self, card: &Card, cx: &mut Context<Self>) -> CardActions {
        let reply = {
            let card = card.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                show(&card, cx);
            })
        };
        let copy = card.instance.clone().map(|instance| -> OnClick {
            Box::new(move |_, _, cx: &mut App| {
                cx.stop_propagation();
                cx.write_to_clipboard(ClipboardItem::new_string(instance.clone()));
            })
        });
        let stop = {
            let name = card.name.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                cx.emit(SidebarEvent::Stop(name.clone()));
            })
        };
        // Pausing an agent in a turn asks on the card first; resuming never asks.
        let pause = {
            let (name, paused, working) = (card.name.clone(), card.paused, card.working);
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                if !paused && working {
                    this.asking = Some(name.clone());
                    cx.notify();
                } else {
                    this.asking = None;
                    cx.emit(SidebarEvent::Pause(pause::Request {
                        names: vec![name.clone()],
                        pause: !paused,
                        ask: None,
                    }));
                }
            })
        };
        let confirm = {
            let name = card.name.clone();
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.asking = None;
                cx.emit(SidebarEvent::Pause(pause::Request {
                    names: vec![name.clone()],
                    pause: true,
                    ask: None,
                }));
                cx.notify();
            })
        };
        let cancel = cx.listener(|this, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
            this.asking = None;
            cx.notify();
        });
        // Leaving the card takes the question back.
        let leave = {
            let name = card.name.clone();
            cx.listener(move |this, hovered: &bool, _, cx| {
                if !hovered && this.asking.as_ref() == Some(&name) {
                    this.asking = None;
                    cx.notify();
                }
            })
        };
        let details = {
            let name = card.name.clone();
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.listing.panel.toggle_details(&name);
                cx.notify();
            })
        };
        CardActions {
            reply: Box::new(reply),
            copy,
            pause: Box::new(pause),
            confirm: Box::new(confirm),
            cancel: Box::new(cancel),
            leave: Box::new(leave),
            stop: Box::new(stop),
            details: Box::new(details),
        }
    }
}

/// Asks the window to show the agent; it puts the keyboard in its pane.
fn show(card: &Card, cx: &mut Context<Sidebar>) {
    cx.emit(SidebarEvent::Attach {
        name: card.name.clone(),
        metadata: AgentMetadata {
            cwd: card.cwd.clone(),
            instance: card.instance.clone(),
        },
    });
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tidy_delegates(window, cx);
        match self.fold {
            Some(folded) => self.folding(folded, window, cx),
            None if self.collapsed => {
                // Drawn open again, the list is there as it is then.
                self.cards.borrow_mut().forget();
                self.rail(window, cx)
            }
            None => self.expanded(window, cx),
        }
    }
}

impl Sidebar {
    /// Expanded: the switcher, the list of agents, and at its foot the activity panel and the
    /// footer.
    fn expanded(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let grounds = self.grounds();
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let now = now();
        // The note while it shows, drawn again each frame as it fades and dropped once gone.
        let instant = Instant::now();
        let shown = self.note.as_ref().and_then(|note| note.opacity(instant));
        if shown.is_none() {
            self.note = None;
        } else if self.note.as_ref().is_some_and(|note| note.fading(instant)) {
            window.request_animation_frame();
        }
        let (lines, mut places, note) = {
            // Measured in the interface font: GPUI's own ellipsis misjudges text that shares a
            // row, so every one-line text of a card is cut here to the room it has, and never
            // wraps. Only the preview wraps, and GPUI cuts it, alone on its lines.
            let family = ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into());
            let text = window.text_system();
            let measure = |line: &str, size: f32, font: Font| {
                let run = TextRun {
                    len: line.len(),
                    font,
                    color: Hsla::default(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let size = ui.px(size);
                f32::from(
                    text.shape_line(SharedString::from(line.to_owned()), size, &[run], None)
                        .width,
                )
            };
            let width = |line: &str, size: f32, weight: FontWeight, features: &FontFeatures| {
                let font = Font {
                    weight,
                    features: features.clone(),
                    ..gpui::font(family.clone())
                };
                measure(line, size, font)
            };
            let plain = FontFeatures::default();
            let normal = |line: &str, size: f32| width(line, size, FontWeight::NORMAL, &plain);
            let mono = |line: &str| measure(line, INSTANCE_SIZE, self.mono.clone());
            // The words beside the avatar.
            let full = words_width(self.width, &ui);
            let mut lines = self.listing.lines(
                self.selected.as_deref(),
                &self.here,
                self.home.as_deref(),
                now,
            );
            // What each open card's `Workspace` cell says, cut to its button.
            let mut places = HashMap::new();
            for line in &mut lines {
                let Line::Agent(card) = line else { continue };
                // A workspace's tray takes its sides and its edges off its cards.
                let room = if card.workspace.is_some() {
                    full - 2.0 * (workspaces_view::TRAY_X + 1.0)
                } else {
                    full
                };
                // The room the first line leaves the name.
                let name_room = |card: &Card| {
                    let mut left = room
                        - ui.scale(MARK_GAP)
                        - TIME_PAD
                        - width(&card.time, NOTE_SIZE, FontWeight::NORMAL, &tabular())
                        - ui.scale(MARK_GAP + ARROW_BOX);
                    if card.unread {
                        left -= ui.scale(MARK_GAP + UNREAD_BOX);
                    }
                    if card.here {
                        left -= ui.scale(MARK_GAP + HERE_BOX);
                    }
                    if let Some(effort) = &card.effort {
                        left -= ui.scale(MARK_GAP) + normal(effort, NOTE_SIZE);
                    }
                    if let Some(sandbox) = &card.sandbox {
                        let semibold =
                            |text: &str| width(text, NOTE_SIZE, FontWeight::SEMIBOLD, &plain);
                        left -= ui.scale(MARK_GAP)
                            + sandbox_view::pill_width(
                                &ui,
                                &sandbox.profile,
                                sandbox.count().as_deref(),
                                &semibold,
                            );
                    }
                    if delegates_mark(card) {
                        left -= ui.scale(MARK_GAP) + delegates_view::mark_width(&ui);
                    }
                    left
                };
                // The effort gives way before the name gets short; then the name is cut: the age
                // always shows.
                let weight = name_weight(card.selected);
                let floor =
                    width(&card.short, CARD_NAME_SIZE, weight, &plain).min(room * NAME_SHARE);
                card::yield_to_name(card, &name_room, floor);
                let left = name_room(card);
                card.short = card::elide(&card.short, &|name| {
                    width(name, CARD_NAME_SIZE, weight, &plain) <= left
                });
                card.place = card.place.fit(&|line| normal(line, NOTE_SIZE) <= room);
                if !card.expanded {
                    continue;
                }
                // A chip too long for the line gives up the end of its branch.
                let chip_room = room - ui.scale(2.0 * CHIP_X);
                for chip in &mut card.details.chips {
                    let others: f32 = chip
                        .iter()
                        .filter(|(_, ink)| *ink != Ink::Branch)
                        .map(|(text, _)| normal(text, NOTE_SIZE))
                        .sum();
                    for (text, _) in chip.iter_mut().filter(|(_, ink)| *ink == Ink::Branch) {
                        *text =
                            card::elide(text, &|cut| others + normal(cut, NOTE_SIZE) <= chip_room);
                    }
                }
                let column = (room - ui.scale(CELL_GAP)) / 2.0;
                for cell in &mut card.details.cells {
                    cell.value =
                        card::elide(&cell.value, &|value| normal(value, SECOND_SIZE) <= column);
                }
                // A button for one whose repository it moves; words alone for the rest (P5-105).
                let button = match card.repo {
                    Some(_) => column - ui.scale(CHIP_X + MARK_GAP + CHEVRON),
                    None => column,
                };
                // Another machine's agents are in no workspace: no cell.
                if self.remote.is_none() {
                    let place = card.workspace.as_deref().unwrap_or("None");
                    places.insert(
                        card.name.clone(),
                        card::elide(place, &|name| normal(name, SECOND_SIZE) <= button),
                    );
                }
                if let Some(path) = &card.details.path {
                    card.details.path =
                        Some(card::shorten(path, &|path| normal(path, NOTE_SIZE) <= room));
                }
                // The instance shares its line with Copy, Pause and Stop….
                let buttons = ["Copy", "Stop…"]
                    .iter()
                    .map(|label| normal(label, SECOND_SIZE) + ui.scale(2.0 * BUTTON_X))
                    .sum::<f32>()
                    + ui.scale(3.0 * MARK_GAP)
                    + ui.scale(CARD_BUTTON + MARK_GAP);
                if let Some(instance) = &card.details.instance {
                    card.details.instance =
                        Some(card::elide(instance, &|id| mono(id) <= room - buttons));
                }
            }
            // The note beside the footer's button, in line with the words beside the avatars.
            let note = self
                .note
                .as_ref()
                .zip(shown)
                .filter(|(note, _)| note.offer.is_none());
            let note = note.map(|(note, opacity)| {
                let text = card::elide(&note.text, &|cut| normal(cut, NOTE_SIZE) <= full);
                // Cut short (agents that could not be paused, say), it shows whole on hover.
                let whole = (text != note.text).then(|| note.text.clone());
                (text, whole, note.problem, opacity)
            });
            (lines, places, note)
        };
        // The cards as drawn now, against last frame: which slide, come, go or ring (P5-154).
        {
            let cards: Vec<&Card> = (lines.iter())
                .filter_map(|line| match line {
                    Line::Agent(card) => Some(&**card),
                    _ => None,
                })
                .collect();
            let panel = &self.listing.panel;
            let listed = self.listing.loaded.then(|| {
                (panel.agents.iter())
                    .map(|a| (a.name.clone(), panel.shown(a, now)))
                    .collect()
            });
            let mut motion = self.cards.borrow_mut();
            motion.frame(listed, &cards, panel.by_name, reduce_motion::on(), instant);
            if motion.fading(instant) {
                window.request_animation_frame();
            }
        }
        // Every agent, those of folded groups too.
        let agents = self.listing.panel.agents.len();
        // Workspaces stay when their agents have all stopped (P5-89): once corral has been read,
        // one opened is listed even with no agent at all; the rest lie on the `Idle` shelf at the
        // foot, under `No agents yet` while there is no agent (P5-134, P5-164).
        let loaded = self.listing.loaded;
        let listed =
            agents > 0 || loaded && lines.iter().any(|line| matches!(line, Line::Workspace(_)));
        let idle = |line: &Line| matches!(line, Line::Idle { .. } | Line::IdleWorkspace { .. });
        let shelf = loaded && lines.iter().any(idle);

        // The list runs on under the activity panel, and ends the panel's height below its last
        // card (P5-70).
        let folded = self.activity.read(cx).folded();
        let end = match self.remote {
            None => list_end(self.width, &ui, folded, self.frosted),
            // No activity panel for another machine: only the footer.
            Some(_) => LIST_END + if self.frosted { 0.0 } else { ui.scale(FOOTER) },
        };
        // Another machine that cannot be read: its agents as last read, dimmed.
        let stale = self.remote.is_some() && self.listing.error.is_some();
        // The cards in an open workspace's well.
        let in_well = grounds.in_well(
            workspaces_view::well(&theme, grounds.base, false),
            hsla(theme.bg(|t| t.agents_bg), 1.0),
        );
        let mut list = div()
            .id("agents")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(CARD_GAP))
            .px(px(PAD))
            .pt(px(2.0))
            .pb(px(end))
            // The cards whose agents have gone, fading out where they were, under the rest.
            .children(self.leaving_cards(stale, grounds, in_well));
        // Another machine that cannot be read: the banner over its list, in place of the error's
        // line; not read yet, grey cards (P5-122).
        if let (Some(error), Some(_)) = (&self.listing.error, &self.remote) {
            list = list.child(self.banner(error, now, cx));
        } else if self.remote.is_some() && !self.listing.loaded {
            list = list.child(self.connecting(cx));
        }
        if !listed && self.listing.loaded && !stale {
            // Nothing listed: why, or how to start one, in the middle of the list.
            let quiet = match (lines.first(), &self.remote) {
                (Some(Line::Error(text)), None) => quiet(
                    &theme,
                    &ui,
                    "Can't read corral",
                    text,
                    None,
                    None,
                    grounds.selected,
                ),
                // Another machine's starts there, in its own dialog (P5-124, P5-124a).
                (_, remote) => quiet(
                    &theme,
                    &ui,
                    &remote.as_ref().map_or("No agents yet".into(), |link| {
                        format!("No agents on {}", link.label())
                    }),
                    "Start one to see its status and replies here.",
                    Some(Icon::NewAgent),
                    Some(
                        div()
                            .id("empty-new-agent")
                            .mt(px(4.0))
                            .h(ui.px(30.0))
                            .px(ui.px(14.0))
                            .flex()
                            .items_center()
                            .rounded(px(8.0))
                            .bg(fg(|t| t.agents_accent))
                            .text_color(hsla(theme.bg(|t| t.agents_bg), 1.0))
                            .text_size(ui.px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.9))
                            .child("New Agent")
                            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                cx.emit(SidebarEvent::NewAgent)
                            })),
                    ),
                    grounds.selected,
                ),
            };
            list = if shelf {
                // In the middle of the room the idle workspaces leave above them.
                list.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .child(quiet),
                )
            } else {
                list.justify_center().child(quiet)
            };
        }
        // A workspace's lines follow its own, and go in its tray; the rest lie in the list.
        let mut lines = (lines.into_iter())
            .filter(|line| listed || shelf && idle(line))
            .enumerate()
            .peekable();
        while let Some((index, line)) = lines.next() {
            let mut inside = Vec::new();
            let (tray, mut line) = match line {
                Line::Workspace(tray) => (Some(tray), None),
                line => (None, Some(line)),
            };
            // Its own line outside a tray; in one, every line up to the next workspace's or the
            // first that is in none.
            loop {
                let next = match line.take() {
                    Some(line) => Some((index, line)),
                    None if tray.is_some() => lines.next_if(|(_, line)| match line {
                        Line::Group(heading) => heading.workspace.is_some(),
                        Line::Agent(card) => card.workspace.is_some(),
                        Line::Vacant { .. } | Line::Vacancies { .. } | Line::Empty(_) => true,
                        Line::Error(_)
                        | Line::Workspace(_)
                        | Line::Idle { .. }
                        | Line::IdleWorkspace { .. } => false,
                    }),
                    None => None,
                };
                let Some((index, line)) = next else { break };
                let first = inside.is_empty();
                inside.push(match line {
                    // Another machine's says it in the banner over the list.
                    Line::Error(_) if self.remote.is_some() => continue,
                    // The list corral last gave stays below this.
                    Line::Error(text) => div()
                        .px(px(8.0))
                        .pt(px(6.0))
                        .text_size(ui.px(NOTE_SIZE))
                        .text_color(fg(|t| t.agents_red))
                        .child(text)
                        .into_any_element(),
                    // Sixteen points below the card above, the gap between lines included
                    // (closer to the top for the first: the window's title bar is over it); under
                    // a workspace's title, close.
                    Line::Group(heading) => {
                        let above = match (tray.is_some(), first, index) {
                            (true, true, _) => 2.0,
                            (false, _, 0) => 10.0,
                            _ => 16.0 - CARD_GAP,
                        };
                        self.heading(heading, index, above, cx)
                    }
                    Line::Workspace(_) => continue,
                    Line::Vacant {
                        workspace,
                        repo,
                        grouped,
                    } => self.vacant(&workspace, Some(repo), grouped, cx),
                    Line::Vacancies {
                        workspace,
                        repos,
                        open,
                    } => self.vacancies(&workspace, &repos, open, cx),
                    Line::Empty(workspace) => self.vacant(&workspace, None, false, cx),
                    // The shelf draws its own lines, every workspace with no agent in it
                    // (P5-164).
                    Line::Idle { count, folded } => {
                        let mut rows = Vec::new();
                        while let Some((_, Line::IdleWorkspace { name, repos })) =
                            lines.next_if(|(_, line)| matches!(line, Line::IdleWorkspace { .. }))
                        {
                            rows.push(self.idle_workspace(&name, &repos, cx));
                        }
                        self.idle_shelf(count, folded, rows, index == 0, cx)
                    }
                    Line::IdleWorkspace { name, repos } => self.idle_workspace(&name, &repos, cx),
                    Line::Agent(card) => {
                        let on_click = {
                            let card = card.clone();
                            cx.listener(move |this, _: &ClickEvent, _, cx| this.click(&card, cx))
                        };
                        let actions = self.card_actions(&card, cx);
                        // An open card's `Workspace` cell, and its menu while that is open.
                        let place = places
                            .remove(&card.name)
                            .map(|place| self.workspace_cell(&card, place, cx));
                        let sandbox = card
                            .expanded
                            .then(|| self.sandbox_cell(&card, cx))
                            .flatten();
                        let delegates = card
                            .expanded
                            .then(|| self.delegates_cell(&card, window, cx))
                            .flatten();
                        let card_grounds = if card.workspace.is_some() {
                            in_well
                        } else {
                            grounds
                        };
                        let name = card.name.clone();
                        let (shown, ripple) = {
                            let motion = self.cards.borrow();
                            (motion.shown(&name), motion.ripple(&name))
                        };
                        let drawn = AgentCard {
                            asking: self.asking.as_ref() == Some(&card.name),
                            ripple,
                            card: *card,
                            theme: theme.clone(),
                            mono: self.mono.clone(),
                            on_click: Box::new(on_click),
                            actions,
                            workspace: place,
                            sandbox,
                            delegates,
                            grounds: card_grounds,
                        }
                        .into_any_element();
                        // Coming in, it fades in where it is laid out.
                        let drawn = if stale || shown < 1.0 {
                            let dim = if stale { STALE } else { 1.0 };
                            div().opacity(dim * shown).child(drawn).into_any_element()
                        } else {
                            drawn
                        };
                        Slot {
                            motion: self.cards.clone(),
                            name,
                            card: drawn,
                        }
                        .into_any_element()
                    }
                });
                if tray.is_none() {
                    break;
                }
            }
            match tray {
                Some(tray) => list = list.child(self.tray(tray, inside, index == 0, cx)),
                None => list = list.children(inside),
            }
        }
        let editor = self.editor(window, cx);

        let note = note.map(|(text, whole, problem, opacity)| {
            let tip = whole.map(|whole| Tip {
                text: whole.into(),
                size: ui.px(NOTE_SIZE),
                color: fg(|t| t.agents_text),
                background: hsla(theme.bg(|t| t.agents_bg), 1.0),
                border: fg(|t| t.agents_rule),
            });
            div()
                .id("footer-note")
                .when_some(tip, |note, tip| {
                    note.tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                })
                .min_w(px(0.0))
                // In line with the cards' names, past their avatars.
                .ml(px(
                    AVATAR_LEFT + ui.scale(AVATAR + AVATAR_GAP - BUTTON) - button_left(&ui)
                ))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_size(ui.px(NOTE_SIZE))
                .text_color(if problem {
                    fg(|t| t.agents_red)
                } else {
                    fg(|t| t.agents_dim)
                })
                .opacity(opacity)
                .child(text)
        });
        let offer = self
            .note
            .as_ref()
            .and_then(|note| note.offer)
            .map(|only| self.offer_card(only, cx));
        let footer = div()
            .flex_shrink_0()
            .h(ui.px(FOOTER))
            .flex()
            .items_center()
            .pl(px(button_left(&ui)))
            .child(self.actions_button(cx))
            .children(note);
        let activity = activity_view::Frame {
            width: self.width,
            theme: self.theme.clone(),
            frosted: self.frosted,
            mono: self.mono.clone(),
            // The window's activation redraws it, and a listing that changes who works too.
            active: window.is_window_active(),
            working: self.listing.animating(now),
        };
        self.activity
            .update(cx, |view, cx| view.frame(activity, cx));

        let switcher = self.switcher(false, cx);
        div()
            .flex_shrink_0()
            .w(px(self.width))
            .h_full()
            .flex()
            .flex_col()
            // Closer than GPUI's default, as in the design.
            .line_height(relative(1.3))
            .children(switcher)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .relative()
                    .flex()
                    .flex_col()
                    .children(editor)
                    // Frosted, the list stops at the footer, which shows the material (P5-75).
                    .child(Anchor {
                        motion: self.cards.clone(),
                        list: list
                            .mb(px(list_bottom(&ui, self.frosted)))
                            .into_any_element(),
                    })
                    // Over the end of the list, which scrolls on its own, down to the sidebar's foot: the
                    // panel, and the footer under it in the panel's see-through ground (P5-70b), or
                    // none on the frosted sidebar (P5-75); the strip has none.
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            // Another machine's agents work in its directories: no Activity here.
                            .when(self.remote.is_none(), |foot| {
                                foot.child(self.activity.clone())
                            })
                            .children(offer)
                            .child(if self.frosted {
                                footer
                            } else {
                                footer.bg(self.activity.read(cx).ground())
                            }),
                    ),
            )
            .into_any_element()
    }
}

/// The footer's note about the last start or stop.
struct Note {
    text: String,
    /// A problem: it stays, in red, until the next note.
    problem: bool,
    /// The offer to switch to This Mac for what starts there alone (P5-122): it stays, in neutral
    /// words, with its button.
    offer: Option<Only>,
    /// When it came.
    at: Instant,
    /// The system's Reduce Motion then: it goes at once rather than fading.
    still: bool,
}

impl Note {
    fn new(text: String, problem: bool, at: Instant, still: bool) -> Self {
        Self {
            text,
            problem,
            offer: None,
            at,
            still,
        }
    }

    /// It stays until the next note rather than fading.
    fn stays(&self) -> bool {
        self.problem || self.offer.is_some()
    }

    /// How much of it shows `now`: all of it while it holds, less as it fades, `None` once gone.
    fn opacity(&self, now: Instant) -> Option<f32> {
        if self.stays() {
            return Some(1.0);
        }
        let Some(fading) = now
            .saturating_duration_since(self.at)
            .checked_sub(NOTE_HOLD)
        else {
            return Some(1.0);
        };
        if self.still || fading >= NOTE_FADE {
            return None;
        }
        Some(1.0 - fading.as_secs_f32() / NOTE_FADE.as_secs_f32())
    }

    /// Whether it is fading `now`, and needs drawing every frame.
    fn fading(&self, now: Instant) -> bool {
        !self.stays() && now >= self.at + NOTE_HOLD && self.opacity(now).is_some()
    }
}

/// The grounds laid on the sidebar's column. On the sidebar's own colour they are whole colours;
/// over the system's sidebar material, tints of the text colour (amber for waiting) that light it
/// and let it through, as much as the preset says (`preset::Frost`).
#[derive(Clone, Copy)]
struct Grounds {
    /// The column itself: the sidebar's colour, or nothing over the material.
    base: Hsla,
    /// A selected tile, a lit button; a hovered one takes half of it.
    selected: Hsla,
    /// What a waiting card adds.
    waiting: Hsla,
    /// The strip's short rules.
    rule: Hsla,
    /// A card's own face resting, hovered, paused and paused hovered, and a selected card's (P5-51).
    card: Hsla,
    card_hovered: Hsla,
    card_paused: Hsla,
    card_paused_hovered: Hsla,
    card_selected: Hsla,
    /// Over the system's material: card faces are see-through.
    frosted: bool,
}

impl Grounds {
    fn of(theme: &Theme, frosted: bool) -> Self {
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let text = fg(|t| t.agents_text);
        // A card stands up off the column in the text colour on a dark theme. On a light one
        // (P5-175) the dark text would sink it instead, so its face is white, the more of it as
        // white has less room over a light ground.
        let face = |alpha: f32| {
            if theme.is_light() {
                gpui::white().opacity((alpha * LIGHT_FACE).min(1.0))
            } else {
                text.opacity(alpha)
            }
        };
        if frosted {
            let frost = theme.frost();
            let accent = fg(|t| t.agents_accent).opacity(SELECTED_FACE * FROSTED_FACE);
            if theme.is_light() {
                // GPUI adds a see-through layer's alpha to the alpha under it, so a light tint
                // over the sidebar's see-through wash hides more of the material than it lets
                // through and comes out grey. On a light theme (P5-175) the faces are whole
                // colours over the sidebar's colour instead; a paused card, drawn see-through as
                // a whole, has no face of its own.
                let base = hsla(theme.bg(|t| t.agents_bg), 1.0);
                let clear = gpui::transparent_black();
                return Self {
                    base: clear,
                    selected: text.opacity(frost.lit),
                    waiting: fg(|t| t.agents_yellow).opacity(frost.waiting),
                    rule: text.opacity(frost.lit),
                    card: over(face(FACE * FROSTED_FACE), base),
                    card_hovered: over(face(FACE_HOVERED * FROSTED_FACE), base),
                    card_paused: clear,
                    card_paused_hovered: clear,
                    card_selected: over(accent, base),
                    frosted,
                };
            }
            Self {
                base: gpui::transparent_black(),
                selected: text.opacity(frost.lit),
                waiting: fg(|t| t.agents_yellow).opacity(frost.waiting),
                rule: text.opacity(frost.lit),
                card: face(FACE * FROSTED_FACE),
                card_hovered: face(FACE_HOVERED * FROSTED_FACE),
                card_paused: face(FACE_PAUSED * FROSTED_FACE),
                card_paused_hovered: face(FACE_HOVERED * FROSTED_FACE),
                card_selected: accent,
                frosted,
            }
        } else {
            let base = hsla(theme.bg(|t| t.agents_bg), 1.0);
            // The preset's own selected colour, as the strip's tile shows it.
            let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);
            Self {
                base,
                selected,
                waiting: fg(|t| t.agents_yellow).opacity(WAITING_TINT),
                rule: fg(|t| t.agents_rule).opacity(0.8),
                card: over(face(FACE), base),
                card_hovered: over(face(FACE_HOVERED), base),
                card_paused: over(face(FACE_PAUSED), base),
                card_paused_hovered: over(face(FACE_HOVERED), base),
                card_selected: selected,
                frosted,
            }
        }
    }

    /// These grounds for the cards in an open workspace's `well` (P5-91), `column` the sidebar's
    /// colour: a whole face hides the well; a see-through one gives back what the well darkens,
    /// so the cards there look as those outside it. A card with no face (paused on a light theme)
    /// lies flat in the well as well.
    fn in_well(self, well: Hsla, column: Hsla) -> Self {
        if !self.frosted {
            return self;
        }
        let through = |face: Hsla| {
            if face.a == 0.0 {
                face
            } else {
                through(face, well, column)
            }
        };
        Self {
            card: through(self.card),
            card_hovered: through(self.card_hovered),
            card_paused: through(self.card_paused),
            card_paused_hovered: through(self.card_paused_hovered),
            card_selected: through(self.card_selected),
            ..self
        }
    }
}

/// `face`, a see-through tint over the whole colour `column`, as it must be laid over `well` there
/// to look the same: as see-through as it was where that will do, less where the well takes
/// away more than its colour can give back.
fn through(face: Hsla, well: Hsla, column: Hsla) -> Hsla {
    let (seen, under) = (
        Rgba::from(over(face, column)),
        Rgba::from(over(well, column)),
    );
    let channels = [(seen.r, under.r), (seen.g, under.g), (seen.b, under.b)];
    let alpha = channels
        .iter()
        .fold(face.a, |alpha, &(seen, under)| {
            let room = if seen > under { 1.0 - under } else { under };
            if room > 0.0 {
                alpha.max((seen - under).abs() / room)
            } else {
                alpha
            }
        })
        .min(1.0);
    let color = |(seen, under): (f32, f32)| (under + (seen - under) / alpha).clamp(0.0, 1.0);
    Rgba {
        r: color(channels[0]),
        g: color(channels[1]),
        b: color(channels[2]),
        a: alpha,
    }
    .into()
}

/// How high a card lies (P5-51): the light along its top edge, in the text colour, then a tight
/// shadow and a soft one under it, each its drop, blur and darkness. Over the material its face is
/// see-through, and GPUI lays a shadow under the whole card, which would show through and darken
/// it: there a dark line inside the bottom edge takes the shadows' place (P5-51b).
struct Lift {
    light: f32,
    near: (f32, f32, f32),
    far: (f32, f32, f32),
}

const RESTING: Lift = Lift {
    light: 0.07,
    near: (1.0, 2.0, 0.25),
    far: (6.0, 14.0, 0.18),
};
/// Hovered, it rises a little: the shadows reach further, as the card itself stays put.
const HOVERED: Lift = Lift {
    light: 0.09,
    near: (2.0, 4.0, 0.28),
    far: (10.0, 22.0, 0.28),
};
const SELECTED: Lift = Lift {
    light: 0.10,
    near: (1.0, 2.0, 0.30),
    far: (8.0, 20.0, 0.30),
};

impl Lift {
    /// The card's shadows in `theme`: the light along its top edge in its text colour, white on a
    /// light theme (P5-175), and the dark ones as dark as [`Theme::shade`] has them there.
    fn shadows(&self, theme: &Theme, frosted: bool) -> Vec<BoxShadow> {
        let edge = if theme.is_light() {
            gpui::white().opacity((self.light * LIGHT_EDGE).min(1.0))
        } else {
            hsla(theme.fg(|t| t.agents_text), 1.0).opacity(self.light)
        };
        // Inside the card, under its top edge, so that edge reads lighter than the rest.
        let light = BoxShadow::new(px(0.0), px(1.0), edge).inset();
        if frosted {
            let dark = BoxShadow::new(
                px(0.0),
                px(-1.0),
                gpui::black().opacity(theme.shade(FROSTED_BOTTOM)),
            );
            return vec![light, dark.inset()];
        }
        let drop = |(y, blur, alpha): (f32, f32, f32)| {
            BoxShadow::new(px(0.0), px(y), gpui::black().opacity(theme.shade(alpha)))
                .blur_radius(px(blur))
        };
        vec![light, drop(self.near), drop(self.far)]
    }
}

/// The colours the column is drawn in: over the system's sidebar material the quiet text is lifted
/// to read on that brighter ground (`Theme::frosted`); on the sidebar's own colour, as given.
fn column_theme(given: &Rc<Theme>, frosted: bool) -> Rc<Theme> {
    if frosted {
        Rc::new(given.frosted())
    } else {
        given.clone()
    }
}

/// `top` laid over `bottom`, either of them see-through; over a whole colour, as `Hsla::blend`.
fn over(top: Hsla, bottom: Hsla) -> Hsla {
    let alpha = top.a + bottom.a * (1.0 - top.a);
    if alpha <= 0.0 {
        return gpui::transparent_black();
    }
    let (t, b) = (Rgba::from(top), Rgba::from(bottom));
    let mix = |t: f32, b: f32| (t * top.a + b * bottom.a * (1.0 - top.a)) / alpha;
    Rgba {
        r: mix(t.r, b.r),
        g: mix(t.g, b.g),
        b: mix(t.b, b.b),
        a: alpha,
    }
    .into()
}

/// The empty list's message: an icon on `ground`, a title, a sentence and an action, centred and
/// quiet.
fn quiet(
    theme: &Theme,
    ui: &UiFont,
    title: &str,
    text: &str,
    icon: Option<Icon>,
    action: Option<gpui::Stateful<Div>>,
    ground: Hsla,
) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .pb(px(40.0))
        .text_center()
        .children(icon.map(|icon| {
            div()
                .size(ui.px(36.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(10.0))
                .bg(ground)
                .child(footer_icon::icon(
                    icon,
                    fg(|t| t.agents_dim),
                    ui.scale(18.0 / footer_icon::SIZE),
                ))
        }))
        .child(
            div()
                .text_size(ui.px(NAME_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(fg(|t| t.agents_text))
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_size(ui.px(SECOND_SIZE))
                .text_color(fg(|t| t.agents_dim))
                .child(text.to_owned()),
        )
        .children(action)
}

/// An icon button, `(width, height, corner radius)` in points, its icon `glyph` points square, with
/// what it does shown on hover; `lit` while what it opens is open. Lit and hovered, it takes
/// `ground`; unlit, its icon is in `quiet`.
fn button(
    (theme, ground, quiet): (&Theme, Hsla, Pick),
    ui: &UiFont,
    id: &'static str,
    (icon, pose, glyph): (Icon, Pose, f32),
    tip: &'static str,
    (width, height, radius): (f32, f32, f32),
    lit: bool,
) -> gpui::Stateful<Div> {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let tip = Tip {
        text: tip.into(),
        size: ui.px(NOTE_SIZE),
        color: fg(|t| t.agents_text),
        background: hsla(theme.bg(|t| t.agents_bg), 1.0),
        border: fg(|t| t.agents_rule),
    };
    let ink = if lit {
        fg(|t| t.agents_text)
    } else {
        fg(quiet)
    };
    div()
        .id(id)
        .flex_shrink_0()
        .w(ui.px(width))
        .h(ui.px(height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui.px(radius))
        .cursor_pointer()
        .when(lit, |button| button.bg(ground))
        .hover(move |style| style.bg(ground))
        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        .child(footer_icon::posed(
            icon,
            pose,
            ink,
            ui.scale(glyph / footer_icon::SIZE),
        ))
}

/// The Attention bell in `color`, swinging as `hover` says.
fn bell_icon(ui: &UiFont, color: Hsla, hover: Hover) -> impl IntoElement {
    footer_icon::posed(Icon::Bell, motion::ring(hover.play), color, ui.scale(1.0))
}

/// A button's hover text.
#[derive(Clone)]
struct Tip {
    text: SharedString,
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
            .child(self.text.clone())
            .child(crate::browser::cover())
    }
}

type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// Told whether the pointer is over the element.
type OnHover = Box<dyn Fn(&bool, &mut Window, &mut App)>;

/// What a card's buttons do (see [`Sidebar::card_actions`]).
struct CardActions {
    reply: OnClick,
    /// `None` without an instance to copy.
    copy: Option<OnClick>,
    /// Pause or Resume; for an agent in a turn, the question first.
    pause: OnClick,
    /// The question's two answers.
    confirm: OnClick,
    cancel: OnClick,
    /// The pointer came over the card or left it.
    leave: OnHover,
    stop: OnClick,
    /// The arrow: open or close the details, without showing the agent (P5-74).
    details: OnClick,
}

/// One agent, read like a conversation: its kind's avatar with the status on the corner; the name,
/// its marks, the effort and how long; the preview; where it works; Reply while it waits; the
/// details when open. Faded while paused. Its own element, so the list never assumes a height.
#[derive(IntoElement)]
struct AgentCard {
    card: Card,
    theme: Rc<Theme>,
    mono: Font,
    on_click: OnClick,
    actions: CardActions,
    /// The open card's `Workspace` cell (P5-89).
    workspace: Option<workspaces_view::WorkspaceCell>,
    /// The open card's Sandbox cell (P5-120).
    sandbox: Option<sandbox_view::SandboxCell>,
    /// The open controller card's Delegates cell (P5-141).
    delegates: Option<delegates_view::DelegatesCell>,
    grounds: Grounds,
    /// It asks whether to pause its agent in a turn.
    asking: bool,
    /// How far the ring its status dot sends out has gone, while it goes (P5-154).
    ripple: Option<f32>,
}

impl RenderOnce for AgentCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let AgentCard {
            card,
            theme,
            mono,
            on_click,
            actions,
            workspace,
            sandbox,
            delegates,
            grounds,
            asking,
            ripple,
        } = self;
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let tip = |text: SharedString| Tip {
            text,
            size: ui.px(NOTE_SIZE),
            color: fg(|t| t.agents_text),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: fg(|t| t.agents_rule),
        };
        let waiting = card.status == Status::Waiting;
        // Idle and not the one shown: the name and preview a step quieter.
        let resting = card.status == Status::Idle && !card.selected;
        let paused = card.status == Status::Paused;
        // The card's face, on the sidebar's own colour a whole colour, so the ring round the
        // status badge matches it; over the system's material a tint, which only nears that.
        // Waiting adds amber.
        let (base, lit) = if card.selected {
            (grounds.card_selected, grounds.card_selected)
        } else if paused {
            (grounds.card_paused, grounds.card_paused_hovered)
        } else {
            (grounds.card, grounds.card_hovered)
        };
        let amber = |face| {
            if waiting {
                over(grounds.waiting, face)
            } else {
                face
            }
        };
        let (ground, hovered) = (amber(base), amber(lit));
        let text = fg(|t| t.agents_text);
        let (edge, edge_hovered) = if waiting {
            let amber = fg(|t| t.agents_yellow).opacity(WAITING_EDGE);
            (amber, amber)
        } else if card.selected {
            let accent = fg(|t| t.agents_accent).opacity(SELECTED_EDGE);
            (accent, accent)
        } else if paused {
            (text.opacity(EDGE_PAUSED), text.opacity(EDGE_HOVERED))
        } else {
            (text.opacity(EDGE), text.opacity(EDGE_HOVERED))
        };
        // Lifted off the column, the selected card highest; a paused one lies flat.
        let frosted = grounds.frosted;
        let lift = |lift: &Lift| {
            if paused {
                Vec::new()
            } else {
                lift.shadows(&theme, frosted)
            }
        };
        let (shadows, shadows_hovered) = if card.selected {
            (lift(&SELECTED), lift(&SELECTED))
        } else {
            (lift(&RESTING), lift(&HOVERED))
        };
        let group = SharedString::from(format!("agent-card-{}", card.name));

        // Every one-line text comes already cut to its room (see `Sidebar::render`).
        let unread = card.unread.then(|| {
            let tip = tip("Finished a turn you haven't seen".into());
            div()
                .id("unread")
                .flex_shrink_0()
                .size(ui.px(UNREAD_BOX))
                .flex()
                .items_center()
                .justify_center()
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .child(
                    div()
                        .size(ui.px(UNREAD))
                        .rounded_full()
                        .bg(fg(|t| t.agents_accent)),
                )
        });
        let here = card.here.then(|| {
            let tip = tip("Open in this window".into());
            div()
                .id("here")
                .flex_shrink_0()
                .size(ui.px(HERE_BOX))
                .flex()
                .items_center()
                .justify_center()
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                // Hollow, so it reads apart from the unread dot.
                .child(
                    div()
                        .size(ui.px(HERE))
                        .rounded_full()
                        .border(ui.px(HERE_RING))
                        .border_color(fg(|t| t.agents_accent)),
                )
        });
        let effort = card.effort.clone().map(|effort| {
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(ui.px(NOTE_SIZE))
                .text_color(fg(|t| t.agents_dimmer))
                .child(effort)
        });
        // Runs in a sandbox: a pill with the profile, and a yellow number once something was
        // blocked (P5-145).
        let lock = card.sandbox.as_ref().map(|sandbox| {
            let tip = tip(sandbox.tip().into());
            sandbox_view::pill(&theme, &ui, sandbox.profile.clone(), sandbox.count())
                .id("sandbox")
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        });
        // A collapsed controller whose agents start in a sandbox: an arrow into a blue lock.
        let delegated = delegates_mark(&card)
            .then(|| card.delegates.as_ref().and_then(|d| d.tip()))
            .flatten()
            .map(|words| delegates_view::mark(&theme, &ui, tip(words.into())));
        let age = card
            .time_color
            .map_or(fg(|t| t.agents_dimmer), |pick| fg(pick).opacity(0.8));
        let first = div()
            .flex()
            .items_center()
            .gap(ui.px(MARK_GAP))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(CARD_NAME_SIZE))
                    .font_weight(name_weight(card.selected))
                    .text_color(fg(if card.status == Status::Exited {
                        |t| t.agents_dim
                    } else if resting {
                        |t| t.agents_branch
                    } else {
                        |t| t.agents_text
                    }))
                    .child(card.short.clone()),
            )
            .children(unread)
            .children(here)
            .children(effort)
            .children(lock)
            .children(delegated)
            .child(
                div()
                    .flex_shrink_0()
                    .ml_auto()
                    .pl(px(TIME_PAD))
                    .whitespace_nowrap()
                    .text_size(ui.px(NOTE_SIZE))
                    .font_features(tabular())
                    .text_color(age)
                    .child(card.time.clone()),
            )
            .child({
                // Its room is always kept, so the time never moves; closed, it shows only while
                // the pointer is over the card.
                let arrow = arrow(card.expanded);
                let tip = tip(arrow.tip.into());
                let lit = text.opacity(0.08);
                div()
                    .id("details")
                    .flex_shrink_0()
                    .size(ui.px(ARROW_BOX))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui.px(ARROW_RADIUS))
                    .cursor_pointer()
                    .hover(move |style| style.bg(lit))
                    .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                    .on_click(actions.details)
                    .when(!arrow.always, |button| {
                        button
                            .opacity(0.0)
                            .group_hover(group.clone(), |style| style.opacity(1.0))
                    })
                    .child(footer_icon::icon(
                        arrow.icon,
                        fg(|t| t.agents_dim),
                        ui.scale(ARROW_ICON),
                    ))
            });

        // Up to two lines, six when open, cut by GPUI with an ellipsis.
        let preview = card.preview.as_ref().map(|preview| {
            let quiet = fg(if resting {
                |t| t.agents_dim
            } else {
                |t| t.agents_branch
            });
            let (text, color, lead) = match preview {
                Preview::Busy { tool, title } => {
                    let tool = tool.clone().unwrap_or_default();
                    let lead = tool.len();
                    let text = match title {
                        Some(title) if lead > 0 => format!("{tool}  {title}"),
                        Some(title) => title.clone(),
                        None => tool,
                    };
                    (text, quiet, lead)
                }
                Preview::Note(text, tone) => (text.clone(), fg(tone_color(*tone)), 0),
                Preview::Reply(text) => (text.clone(), quiet, 0),
            };
            // A working agent's tool leads, in the status's colour.
            let highlights = (lead > 0).then(|| {
                (
                    0..lead,
                    HighlightStyle {
                        color: Some(fg(card.look.color)),
                        font_weight: Some(FontWeight::SEMIBOLD),
                        ..HighlightStyle::default()
                    },
                )
            });
            div()
                .min_w(px(0.0))
                .text_size(ui.px(PREVIEW_SIZE))
                .line_height(relative(1.42))
                .text_color(color)
                .line_clamp(if card.expanded {
                    PREVIEW_OPEN_LINES
                } else {
                    PREVIEW_LINES
                })
                .text_ellipsis()
                .child(StyledText::new(text).with_highlights(highlights))
        });

        let place = (!card.expanded).then(|| {
            let (text, colors) = pieces(&theme, &card.place.spans());
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(ui.px(NOTE_SIZE))
                .child(styled(text, &colors))
        });

        // Waiting: answering is one click away.
        let reply = waiting.then(|| {
            div().flex().mt(px(6.0)).child(
                div()
                    .id("reply")
                    .flex_shrink_0()
                    .h(ui.px(24.0))
                    .px(ui.px(10.0))
                    .flex()
                    .items_center()
                    .rounded(ui.px(7.0))
                    .bg(fg(|t| t.agents_yellow))
                    .text_color(hsla(theme.bg(|t| t.agents_bg), 1.0))
                    .text_size(ui.px(SECOND_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(actions.reply)
                    .child("Reply"),
            )
        });
        let details = card.expanded.then(|| {
            let buttons = Buttons {
                copy: actions.copy,
                pause: actions.pause,
                stop: actions.stop,
                asking: asking.then_some((actions.confirm, actions.cancel)),
                workspace,
                sandbox,
                delegates,
            };
            details(&theme, &mono, &ui, &card, buttons)
        });

        let badge = badge(&theme, &ui, &card, (ground, hovered), &group, ripple);
        let avatar = avatar(&theme, &ui, &card, badge).when_some(
            card.brand
                .as_ref()
                .map(|brand| tip(brand.kind.clone().into())),
            |avatar, tip| avatar.tooltip(move |_, cx| cx.new(|_| tip.clone()).into()),
        );
        let words = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(first)
            .children(preview)
            .children(place)
            .children(reply)
            .children(details);

        div()
            .id(ElementId::Name(SharedString::from(card.name.clone())))
            .group(group)
            .flex()
            .items_start()
            .gap(ui.px(AVATAR_GAP))
            .w_full()
            .pl(px(CARD_LEFT))
            .pr(px(CARD_RIGHT))
            .py(px(CARD_Y))
            .rounded(px(CARD_RADIUS))
            .border_1()
            .border_color(edge)
            .bg(ground)
            .shadow(shadows)
            .when(!card.selected, |body| {
                body.hover(move |style| {
                    style
                        .bg(hovered)
                        .border_color(edge_hovered)
                        .shadow(shadows_hovered)
                })
            })
            .cursor_pointer()
            .on_click(on_click)
            .on_hover(actions.leave)
            .when(paused, |body| body.opacity(PAUSED_OPACITY))
            .child(avatar)
            .child(words)
    }
}

/// The avatar: the kind's icon, or the name's letter for a kind without one, on a square tinted in
/// the kind's colour, with `badge` on its corner. An original that brings its own ground fills the
/// square instead of the tint; one without (pi's logo) is drawn larger than a silhouette.
fn avatar(theme: &Theme, ui: &UiFont, card: &Card, badge: AnyElement) -> gpui::Stateful<Div> {
    let color = kind_color(theme, card);
    let (mark, tile) = original_mark(ui, card, color, (AVATAR, AVATAR_RADIUS))
        .unwrap_or_else(|| (kind_mark(ui, card, color, CARD_NAME_SIZE), false));
    div()
        .id("avatar")
        .relative()
        .flex_shrink_0()
        .size(ui.px(AVATAR))
        .mt(px(1.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui.px(AVATAR_RADIUS))
        .when(!tile, |avatar| avatar.bg(color.opacity(AVATAR_TINT)))
        .child(mark)
        .child(badge)
}

/// The kind's original from this machine in a square `(side, radius)`, as the avatar and the
/// strip's tile draw it, and whether it fills the square: one that brings its own ground does,
/// one without (pi's logo) is drawn `AVATAR_PICTURE` tall for the square's tint to frame it.
/// `None` without an original.
fn original_mark(
    ui: &UiFont,
    card: &Card,
    color: Hsla,
    (side, radius): (f32, f32),
) -> Option<(AnyElement, bool)> {
    let icon = card
        .brand
        .as_ref()
        .and_then(|brand| kind_icon::of(&brand.kind))
        .filter(|icon| icon.original())?;
    Some(if icon.tile() {
        (icon.render_tile(ui.px(side), ui.px(radius)), true)
    } else {
        (icon.render(ui.px(AVATAR_PICTURE), color), false)
    })
}

/// The colour of a card's kind, dim for an agent of no known kind.
fn kind_color(theme: &Theme, card: &Card) -> Hsla {
    let kind: Pick = card
        .brand
        .as_ref()
        .map_or(|t| t.agents_dim, |brand| brand.color);
    hsla(theme.fg(kind), 1.0)
}

/// The kind's icon in `color`, as the avatar and the strip's tile draw it, or for a kind without
/// one the name's letter, `letter` points.
fn kind_mark(ui: &UiFont, card: &Card, color: Hsla, letter: f32) -> AnyElement {
    match card
        .brand
        .as_ref()
        .and_then(|brand| kind_icon::of(&brand.kind))
    {
        Some(icon) => icon.render(ui.px(AVATAR_ICON), color).into_any_element(),
        None => {
            // The short name before it was cut to fit.
            let short = card
                .name
                .split_once('/')
                .map_or(card.name.as_str(), |(_, s)| s);
            div()
                .text_size(ui.px(letter))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(initial(short))
                .into_any_element()
        }
    }
}

/// The status on the avatar's corner, ringed in the card's ground (the second while the card is
/// hovered): an arc turning while it works, an amber dot softly pulsing while it waits, an orange
/// `!` when stalled, otherwise a small dot in the status's colour. Its agent just stopped or come
/// to need you, the dot sends one ring out, `ripple` of the way (P5-154).
fn badge(
    theme: &Theme,
    ui: &UiFont,
    card: &Card,
    (ground, hovered): (Hsla, Hsla),
    group: &SharedString,
    ripple: Option<f32>,
) -> AnyElement {
    let color = hsla(theme.fg(card.look.color), 1.0);
    let ring_out = |dot: f32| ripple.map(|went| cards_motion::ripple(color, ui.scale(dot), went));
    // The ring: `inner` points across, `ring` round it, its edge `inset` from the avatar's corner.
    let ring = |inner: f32, ring: f32, inset: f32| {
        div()
            .absolute()
            .right(ui.px(inset))
            .bottom(ui.px(inset))
            .size(ui.px(inner + 2.0 * ring))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(ground)
            .group_hover(group.clone(), move |style| style.bg(hovered))
    };
    match card.status {
        Status::Working => {
            let spin = ui.px(11.0);
            let arc = svg()
                .data(SPIN_ARC)
                .absolute()
                .size(spin)
                .text_color(color)
                .with_animation(
                    "spin",
                    Animation::new(Duration::from_millis(1100))
                        .repeat_synced()
                        .with_max_fps(30.0),
                    |arc, delta| arc.with_transformation(Transformation::rotate(percentage(delta))),
                );
            ring(15.0, 0.0, -4.0)
                .child(
                    div()
                        .relative()
                        .size(spin)
                        .child(
                            svg()
                                .data(SPIN_TRACK)
                                .absolute()
                                .size(spin)
                                .text_color(color.opacity(0.25)),
                        )
                        .child(arc),
                )
                .into_any_element()
        }
        Status::Waiting => {
            let reach = ui.scale(5.0);
            // Come to need you just now, the ring it sends out once goes over the beat.
            let rung = ring_out(11.0);
            let dot = div()
                .size(ui.px(11.0))
                .rounded_full()
                .bg(color)
                .with_animation(
                    "pulse",
                    Animation::new(Duration::from_millis(1800))
                        .repeat_synced()
                        .with_max_fps(30.0),
                    move |dot, delta| {
                        // A ring widens and fades over most of the beat, then rests.
                        let out = ease_in_out((delta / 0.7).min(1.0));
                        let pulse = BoxShadow {
                            color: color.opacity(0.6 * (1.0 - out)),
                            offset: point(px(0.0), px(0.0)),
                            blur_radius: px(0.0),
                            spread_radius: px(reach * out),
                            inset: false,
                        };
                        dot.shadow(std::iter::once(pulse).chain(rung.clone()).collect())
                    },
                );
            ring(11.0, 2.5, -5.5).child(dot).into_any_element()
        }
        Status::Stalled => ring(14.0, 2.0, -6.0)
            .child(
                div()
                    .size(ui.px(14.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(color)
                    .text_size(ui.px(10.0))
                    .line_height(relative(1.0))
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .text_color(hsla(theme.bg(|t| t.agents_bg), 1.0))
                    .child("!"),
            )
            .into_any_element(),
        _ => ring(8.0, 2.5, -4.5)
            .child(
                div()
                    .size(ui.px(8.0))
                    .rounded_full()
                    .bg(color)
                    .when_some(ring_out(8.0), |dot, ring| dot.shadow(vec![ring])),
            )
            .into_any_element(),
    }
}

/// The open card's buttons, and while it asks whether to pause its agent in a turn, the answers.
struct Buttons {
    copy: Option<OnClick>,
    /// Pause, or Resume for a paused agent.
    pause: OnClick,
    stop: OnClick,
    /// Pause anyway, and Cancel.
    asking: Option<(OnClick, OnClick)>,
    /// The `Workspace` cell: where the agent is, and the button that opens its menu.
    workspace: Option<workspaces_view::WorkspaceCell>,
    /// The Sandbox cell, for a sandboxed agent: the button that opens what was blocked.
    sandbox: Option<sandbox_view::SandboxCell>,
    /// The Delegates cell, for a controller: the button that opens its menu (P5-141).
    delegates: Option<delegates_view::DelegatesCell>,
}

/// The collapsed card shows the Delegates mark: a controller whose agents start in a sandbox.
fn delegates_mark(card: &Card) -> bool {
    !card.expanded && card.delegates.as_ref().is_some_and(|d| d.profile.is_some())
}

/// The open card under the header: the full path over the branch and changes as chips; the rest in
/// two columns of cells, a small uppercase label over each value; then the instance with Copy, and
/// Pause (or Resume) and Stop… at the right; under them the question before pausing an agent in a
/// turn.
fn details(theme: &Theme, mono: &Font, ui: &UiFont, card: &Card, buttons: Buttons) -> Div {
    let Buttons {
        copy,
        pause,
        stop,
        asking,
        workspace: place,
        sandbox,
        delegates,
    } = buttons;
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let (text, dim, dimmer, rule) = (
        fg(|t| t.agents_text),
        fg(|t| t.agents_dim),
        fg(|t| t.agents_dimmer),
        fg(|t| t.agents_rule),
    );
    let details = &card.details;
    let chips = details.chips.iter().map(|chip| {
        let (text, colors) = pieces(theme, chip);
        div()
            .flex_shrink_0()
            .h(ui.px(21.0))
            .px(ui.px(CHIP_X))
            .flex()
            .items_center()
            .rounded(ui.px(6.0))
            .bg(fg(|t| t.agents_text).opacity(0.055))
            .whitespace_nowrap()
            .text_size(ui.px(NOTE_SIZE))
            .child(styled(text, &colors))
    });
    let location = (details.path.is_some() || !details.chips.is_empty()).then(|| {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .children(details.path.clone().map(|path| {
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(NOTE_SIZE))
                    .text_color(dim)
                    .child(path)
            }))
            .when(!details.chips.is_empty(), |body| {
                body.child(div().flex().flex_wrap().gap(ui.px(5.0)).children(chips))
            })
    });
    let label = |name: &str| {
        div()
            .whitespace_nowrap()
            .text_size(ui.px(CELL_LABEL_SIZE))
            .text_color(dimmer.opacity(0.8))
            .child(name.to_ascii_uppercase())
    };
    let cell = |cell: &card::Cell| {
        // What follows `·` (how long, the effort, `open here`) is quieter than the main word.
        let tail = cell
            .value
            .find(" · ")
            .or_else(|| cell.value.find('·'))
            .map(|from| {
                (
                    from..cell.value.len(),
                    HighlightStyle {
                        color: Some(dim),
                        ..HighlightStyle::default()
                    },
                )
            });
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(label(cell.label))
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(SECOND_SIZE))
                    .text_color(text)
                    .child(StyledText::new(cell.value.clone()).with_highlights(tail)),
            )
    };
    // The last cell: the workspace, as a button that opens the menu of them; words alone for an
    // agent that goes by its name's prefix, which has no repository to move (P5-105).
    let place = place.map(|place| {
        let Some(on_press) = place.on_press else {
            let words = card::Cell {
                label: "Workspace",
                value: place.value,
            };
            return cell(&words).into_any_element();
        };
        let (ground, lit) = (text.opacity(0.12), text.opacity(0.18));
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(label("Workspace"))
            .child(
                div()
                    .relative()
                    .flex()
                    .child(
                        div()
                            .id("card-workspace")
                            .mx(-ui.px(CHIP_X))
                            .px(ui.px(CHIP_X))
                            .my(-px(2.0))
                            .py(px(2.0))
                            .flex()
                            .items_center()
                            .gap(ui.px(MARK_GAP))
                            .rounded(ui.px(6.0))
                            .when(place.open, |button| button.bg(lit))
                            .cursor_pointer()
                            .hover(move |style| style.bg(if place.open { lit } else { ground }))
                            .on_mouse_down(MouseButton::Left, on_press)
                            // The press opened the menu: its click is not one on the card.
                            .on_click(|_, _, cx| cx.stop_propagation())
                            .child(
                                div()
                                    .whitespace_nowrap()
                                    .text_size(ui.px(SECOND_SIZE))
                                    .text_color(text)
                                    .child(place.value),
                            )
                            .child(
                                svg()
                                    .data(CHEVRON_DOWN)
                                    .flex_shrink_0()
                                    .size(ui.px(CHEVRON))
                                    .text_color(dim),
                            ),
                    )
                    .children(place.menu),
            )
            .into_any_element()
    });
    let mut all = (details.cells.iter())
        .map(|each| cell(each).into_any_element())
        .chain(place);
    let mut cells = div().flex().flex_col().gap(px(10.0));
    while let Some(left) = all.next() {
        let right = all
            .next()
            .unwrap_or_else(|| div().flex_1().into_any_element());
        cells = cells.child(div().flex().gap(ui.px(CELL_GAP)).child(left).child(right));
    }
    let cells = cells
        .children(delegates.map(|delegates| delegates_view::cell(theme, ui, delegates)))
        .children(sandbox.map(|sandbox| sandbox_view::cell(theme, ui, sandbox)));
    // An icon button with its hover text, quiet until hovered; what it looks like then is the
    // caller's. Made each frame from its motion's state.
    let action = |id: &'static str, tip: &'static str| {
        let tip = Tip {
            text: tip.into(),
            size: ui.px(NOTE_SIZE),
            color: text,
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: rule,
        };
        let (size, radius) = (ui.px(CARD_BUTTON), ui.px(CARD_BUTTON_RADIUS));
        move |on_click: OnClick| {
            div()
                .id(id)
                .flex_shrink_0()
                .size(size)
                .flex()
                .items_center()
                .justify_center()
                .rounded(radius)
                .cursor_pointer()
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .on_click(on_click)
        }
    };
    let scale = ui.scale(1.0);
    let copy_scale = ui.scale(COPY_ICON);
    let instance = details.instance.clone().map(|instance| {
        div()
            .flex_shrink_0()
            .whitespace_nowrap()
            .font(mono.clone())
            .text_size(ui.px(INSTANCE_SIZE))
            .text_color(dimmer)
            .child(instance)
    });
    let last = div()
        .pt(px(8.0))
        .border_t_1()
        .border_color(rule)
        .flex()
        .items_center()
        .gap(ui.px(MARK_GAP))
        .children(instance)
        .children(copy.map(|copy| {
            // It nudges as the pointer comes in, and turns to a green tick for a moment once it
            // has copied.
            let button = action("copy", "Copy instance ID");
            let (ground, green) = (text.opacity(0.08), fg(|t| t.agents_green));
            motion::hover_motion("copy-motion", motion::COPY, move |hover| {
                let (icon, pose, opacity) = motion::copy(hover);
                let ink = if icon == Icon::Copied { green } else { dimmer };
                button(copy)
                    .hover(move |style| style.bg(ground))
                    .child(footer_icon::posed(
                        icon,
                        pose,
                        ink.opacity(opacity),
                        copy_scale,
                    ))
            })
        }))
        .child(div().flex_1())
        .child({
            // Its bars dip together as the pointer comes in; a paused agent's triangle slides the
            // way it plays.
            let lit = text.opacity(0.08);
            if card.paused {
                let button = action("resume", "Resume agent");
                motion::hover_motion("resume-motion", motion::SHIFT, move |hover| {
                    button(pause)
                        .hover(move |style| style.bg(lit))
                        .child(footer_icon::posed(
                            Icon::Resume,
                            motion::shift(hover.play, true),
                            dim,
                            scale,
                        ))
                })
            } else {
                let button = action("pause", "Pause agent");
                motion::hover_motion("pause-motion", motion::DIP, move |hover| {
                    button(pause)
                        .hover(move |style| style.bg(lit))
                        .child(footer_icon::posed(
                            Icon::Pause,
                            motion::dip(hover.play),
                            dim,
                            scale,
                        ))
                })
            }
        })
        .child({
            // It presses in and reddens on a faint red ground while the pointer is over it.
            let button = action("stop", "Stop agent…");
            let red = fg(|t| t.agents_red);
            motion::hover_motion("stop-motion", motion::PRESS, move |hover| {
                button(stop)
                    .bg(red.opacity(motion::PRESSED_GROUND * hover.lit))
                    .child(footer_icon::posed(
                        Icon::StopAgent,
                        motion::press(hover.lit, hover.still),
                        motion::mix(dim, red, hover.lit),
                        scale,
                    ))
            })
        });
    // Pausing an agent in a turn: the question, and its two answers kept together when it wraps.
    let question = asking.map(|(confirm, cancel)| {
        let answer = |id: &'static str, label: &'static str| {
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
                .child(label)
        };
        let yellow = fg(|t| t.agents_yellow);
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(ui.px(6.0))
            .text_size(ui.px(SECOND_SIZE))
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .text_color(text)
                    .child(PAUSE_WORKING),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap(ui.px(6.0))
                    .child(
                        answer("pause-cancel", "Cancel")
                            .border_color(rule)
                            .text_color(dim)
                            .hover(move |style| style.bg(text.opacity(0.09)))
                            .on_click(cancel),
                    )
                    .child(
                        answer("pause-confirm", "Pause")
                            .bg(yellow.opacity(0.16))
                            .border_color(yellow.opacity(0.5))
                            .text_color(yellow)
                            .font_weight(FontWeight::SEMIBOLD)
                            .hover(move |style| style.bg(yellow.opacity(0.26)))
                            .on_click(confirm),
                    ),
            )
    });
    div()
        .mt(px(8.0))
        .pt(px(10.0))
        .border_t_1()
        .border_color(rule)
        .flex()
        .flex_col()
        .gap(px(12.0))
        .children(location)
        .child(cells)
        .child(last)
        .children(question)
}

/// An agent in the collapsed strip: its kind's icon (or letter) as on the card's avatar, with the
/// card's status mark on the corner; a waiting one on faint amber with a thin amber edge, the one
/// in the active pane on a lit square with a faint edge, a paused one faded. Hovering shows who it
/// is.
#[derive(IntoElement)]
struct Tile {
    card: Card,
    theme: Rc<Theme>,
    tip: RailTip,
    on_click: OnClick,
    grounds: Grounds,
}

impl RenderOnce for Tile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let theme = &self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let card = &self.card;
        let grounds = self.grounds;
        let tip = self.tip;
        let waiting = card.status == Status::Waiting;
        // The tile's ground, as the card's: lit when shown, amber added while waiting.
        let base = if card.selected {
            over(grounds.selected, grounds.base)
        } else {
            grounds.base
        };
        let ground = if waiting {
            over(grounds.waiting, base)
        } else {
            base
        };
        let hovered = if card.selected {
            ground
        } else {
            over(grounds.selected.opacity(0.6), ground)
        };
        let group = SharedString::from(format!("rail-tile-{}", card.name));
        let edge = if waiting {
            fg(|t| t.agents_yellow).opacity(0.22)
        } else if card.selected {
            fg(|t| t.agents_border).opacity(0.6)
        } else {
            gpui::transparent_black()
        };
        let color = kind_color(theme, card);
        let original = original_mark(&ui, card, color, (TILE, TILE_RADIUS));
        // An original covers the tile, so its ground and edge go on a plate round it instead.
        let plate = original.is_some().then(|| {
            let selected = card.selected;
            div()
                .absolute()
                .top(ui.px(-TILE_PLATE))
                .left(ui.px(-TILE_PLATE))
                .size(ui.px(TILE + 2.0 * TILE_PLATE))
                .rounded(ui.px(TILE_RADIUS + TILE_PLATE))
                .bg(ground)
                .border_1()
                .border_color(edge)
                .when(!selected, |plate| {
                    plate.group_hover(group.clone(), move |style| style.bg(hovered))
                })
        });
        let (mark, fills) = match original {
            Some((mark, fills)) => (mark, Some(fills)),
            None => (kind_mark(&ui, card, color, TILE_SIZE), None),
        };
        div()
            .id(ElementId::Name(SharedString::from(card.name.clone())))
            .group(group.clone())
            .relative()
            .flex_shrink_0()
            .size(ui.px(TILE))
            .my(ui.px(TILE_GAP))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(TILE_RADIUS))
            .cursor_pointer()
            .map(|tile| match fills {
                None => tile
                    .bg(ground)
                    .border_1()
                    .border_color(edge)
                    .when(!card.selected, |tile| {
                        tile.hover(move |style| style.bg(hovered))
                    }),
                Some(false) => tile.bg(color.opacity(AVATAR_TINT)),
                Some(true) => tile,
            })
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .on_click(self.on_click)
            .when(card.status == Status::Paused, |tile| {
                tile.opacity(PAUSED_OPACITY)
            })
            .children(plate)
            .child(mark)
            // The mark hangs over the strip: ringed in the strip's colour.
            .child(badge(
                theme,
                &ui,
                card,
                (grounds.base, grounds.base),
                &group,
                None,
            ))
    }
}

/// A tile's hover note: the group, faint, before the short name, and the effort; under them the
/// status and how long.
#[derive(Clone)]
struct RailTip {
    group: Option<String>,
    name: String,
    effort: Option<String>,
    status: (String, Hsla),
    size: Pixels,
    text: Hsla,
    dim: Hsla,
    background: Hsla,
    border: Hsla,
}

impl RailTip {
    fn of(card: &Card, theme: &Theme, ui: &UiFont) -> Self {
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let (state, color) = match &card.preview {
            Some(Preview::Note(text, tone)) => (text.clone(), fg(tone_color(*tone))),
            _ => (card.look.label.to_owned(), fg(card.look.color)),
        };
        let status = if card.time.is_empty() {
            state
        } else {
            format!("{state} · {}", card.time)
        };
        Self {
            group: workspaces_view::tip_group(card),
            name: card.short.clone(),
            effort: card.effort.clone(),
            status: (status, color),
            size: ui.px(SECOND_SIZE),
            text: fg(|t| t.agents_text),
            dim: fg(|t| t.agents_dim),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: fg(|t| t.agents_rule),
        }
    }
}

impl Render for RailTip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(self.border)
            .bg(self.background)
            .shadow_md()
            .whitespace_nowrap()
            .text_size(self.size)
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .children(
                        self.group
                            .clone()
                            .map(|group| div().text_color(self.dim).child(group)),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(self.text)
                            .child(self.name.clone()),
                    )
                    .children(
                        self.effort
                            .clone()
                            .map(|effort| div().text_color(self.dim).child(format!(" · {effort}"))),
                    ),
            )
            .child(div().text_color(self.status.1).child(self.status.0.clone()))
            .child(crate::browser::cover())
    }
}

/// `text` with each range in its colour; ranges may run past the end.
fn styled(text: String, colors: &[(std::ops::Range<usize>, Hsla)]) -> StyledText {
    let len = text.len();
    let highlights: Vec<_> = colors
        .iter()
        .map(|(range, color)| (range.start.min(len)..range.end.min(len), *color))
        .filter(|(range, _)| !range.is_empty())
        .map(|(range, color)| {
            (
                range,
                HighlightStyle {
                    color: Some(color),
                    ..HighlightStyle::default()
                },
            )
        })
        .collect();
    StyledText::new(text).with_highlights(highlights)
}

/// Coloured pieces, of the place line or a chip, as one text and the range of each colour.
fn pieces(
    theme: &Theme,
    pieces: &[(String, Ink)],
) -> (String, Vec<(std::ops::Range<usize>, Hsla)>) {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let mut text = String::new();
    let mut colors = Vec::new();
    for (piece, ink) in pieces {
        let color = match ink {
            Ink::Dim => fg(|t| t.agents_dim),
            Ink::Dimmer => fg(|t| t.agents_dimmer),
            Ink::Branch => fg(|t| t.agents_branch).opacity(0.8),
            Ink::Added => fg(|t| t.agents_green).opacity(0.9),
            Ink::Deleted => fg(|t| t.agents_red).opacity(0.9),
            Ink::Ahead => fg(|t| t.agents_yellow).opacity(0.9),
        };
        colors.push((text.len()..text.len() + piece.len(), color));
        text += piece;
    }
    (text, colors)
}

/// A note's colour: dim while starting, amber while waiting, red for errors and exits.
fn tone_color(tone: Tone) -> Pick {
    match tone {
        Tone::Quiet => |t| t.agents_dim,
        Tone::Waiting => |t| t.agents_yellow,
        Tone::Problem => |t| t.agents_red,
    }
}

/// How wide a card's words are, beside the avatar, in a sidebar `width` wide.
fn words_width(width: f32, ui: &UiFont) -> f32 {
    width - 2.0 * PAD - CARD_LEFT - CARD_RIGHT - 2.0 - ui.scale(AVATAR + AVATAR_GAP)
}

/// The name's weight: heavier on the card shown in the active pane.
fn name_weight(selected: bool) -> FontWeight {
    if selected {
        FontWeight::SEMIBOLD
    } else {
        FontWeight::MEDIUM
    }
}

/// The arrow after a card's time, which opens or closes its details in one click whether or not
/// the card is the one shown (P5-74).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Arrow {
    icon: Icon,
    tip: &'static str,
    /// Shown all the time, not only while the pointer is over the card.
    always: bool,
}

fn arrow(expanded: bool) -> Arrow {
    if expanded {
        Arrow {
            icon: Icon::Up,
            tip: "Hide details",
            always: true,
        }
    } else {
        Arrow {
            icon: Icon::Down,
            tip: "Show details",
            always: false,
        }
    }
}

/// Digits of equal width, so ages line up.
fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// A status dot, here and in the command palette; a working agent's breathes, a ring widening
/// as it fades, every 1.6 s.
pub(crate) fn status_dot(color: Hsla, breathing: bool, ui: &UiFont) -> AnyElement {
    let dot = div()
        .flex_shrink_0()
        .size(ui.px(DOT))
        .rounded_full()
        .bg(color);
    if !breathing {
        return dot.into_any_element();
    }
    let reach = ui.scale(4.0);
    dot.with_animation(
        "breath",
        Animation::new(Duration::from_millis(1600))
            .repeat_synced()
            .with_max_fps(30.0),
        move |dot, delta| {
            let out = ease_in_out(if delta < 0.5 {
                delta * 2.0
            } else {
                (1.0 - delta) * 2.0
            });
            dot.shadow(vec![BoxShadow {
                color: color.opacity(0.55 * (1.0 - out)),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(reach * out),
                inset: false,
            }])
        },
    )
    .into_any_element()
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_card_shows_its_arrow_down_on_hover_and_an_open_one_up_always() {
        assert_eq!(
            arrow(false),
            Arrow {
                icon: Icon::Down,
                tip: "Show details",
                always: false
            }
        );
        assert_eq!(
            arrow(true),
            Arrow {
                icon: Icon::Up,
                tip: "Hide details",
                always: true
            }
        );
    }

    #[test]
    fn frosted_cards_cast_no_shadow_under_their_see_through_face() {
        for name in crate::preset::Preset::ALL.map(crate::preset::Preset::name) {
            let theme = Theme::from_config(&crate::config::Config {
                theme: Some(name.into()),
                ..crate::config::Config::default()
            })
            .unwrap();
            for lift in [&RESTING, &HOVERED, &SELECTED] {
                // Over the material the face is see-through: only lines inside it, light on top
                // and dark at the bottom, nothing laid under it to show through (P5-51b).
                let frosted = lift.shadows(&theme, true);
                assert!(frosted.iter().all(|s| s.inset), "{name}: {frosted:?}");
                assert!(frosted.iter().any(|s| s.offset.y > px(0.0)));
                assert!(frosted.iter().any(|s| s.offset.y < px(0.0)));
                // On the sidebar's own colour the face is whole: the shadows stay.
                assert!(lift.shadows(&theme, false).iter().any(|s| !s.inset));
            }
        }
    }

    /// A dark theme's cards are drawn as before the light theme came; the light one's faces and
    /// top edges are white and its shadows softer (P5-175).
    #[test]
    fn cards_lift_in_the_text_colour_on_a_dark_theme_and_in_white_on_a_light_one() {
        for name in crate::preset::Preset::ALL.map(crate::preset::Preset::name) {
            let theme = Theme::from_config(&crate::config::Config {
                theme: Some(name.into()),
                ..crate::config::Config::default()
            })
            .unwrap();
            let text = hsla(theme.fg(|t| t.agents_text), 1.0);
            let base = hsla(theme.bg(|t| t.agents_bg), 1.0);
            let (whole, frosted) = (Grounds::of(&theme, false), Grounds::of(&theme, true));
            let lifts = RESTING.shadows(&theme, false);
            if theme.is_light() {
                let white = gpui::white();
                // Over the material too, a whole face over the sidebar's colour; a paused card
                // has none there.
                let face = over(white.opacity(FACE * FROSTED_FACE * LIGHT_FACE), base);
                assert_eq!(frosted.card, face, "{name}");
                assert_eq!(frosted.card.a, 1.0, "{name}");
                assert_eq!(frosted.card_hovered.a, 1.0, "{name}");
                assert_eq!(frosted.card_selected.a, 1.0, "{name}");
                assert_eq!(frosted.card_paused, gpui::transparent_black());
                assert_eq!(frosted.card_paused_hovered, gpui::transparent_black());
                assert_eq!(whole.card, over(white.opacity(FACE * LIGHT_FACE), base));
                assert!(Rgba::from(whole.card).r > Rgba::from(base).r, "{name}");
                assert_eq!(lifts[0].color, white.opacity(RESTING.light * LIGHT_EDGE));
                assert!(lifts[1].color.a < RESTING.near.2, "{name}");
            } else {
                assert_eq!(frosted.card, text.opacity(FACE * FROSTED_FACE), "{name}");
                assert_eq!(
                    frosted.card_hovered,
                    text.opacity(FACE_HOVERED * FROSTED_FACE)
                );
                assert_eq!(
                    frosted.card_paused,
                    text.opacity(FACE_PAUSED * FROSTED_FACE)
                );
                assert_eq!(frosted.card_paused_hovered, frosted.card_hovered, "{name}");
                assert_eq!(whole.card_paused_hovered, whole.card_hovered, "{name}");
                let accent = hsla(theme.fg(|t| t.agents_accent), 1.0);
                let selected = accent.opacity(SELECTED_FACE * FROSTED_FACE);
                assert_eq!(frosted.card_selected, selected, "{name}");
                assert_eq!(whole.card, over(text.opacity(FACE), base), "{name}");
                assert_eq!(whole.card_hovered, over(text.opacity(FACE_HOVERED), base));
                assert_eq!(whole.card_paused, over(text.opacity(FACE_PAUSED), base));
                assert_eq!(lifts[0].color, text.opacity(RESTING.light), "{name}");
                assert_eq!(lifts[1].color, gpui::black().opacity(RESTING.near.2));
                assert_eq!(lifts[2].color, gpui::black().opacity(RESTING.far.2));
                let bottom = SELECTED.shadows(&theme, true)[1].color;
                assert_eq!(bottom, gpui::black().opacity(FROSTED_BOTTOM), "{name}");
            }
        }
    }

    #[test]
    fn the_list_ends_the_activity_panel_its_gap_and_the_footer_below_its_last_card() {
        let width = 300.0;
        let mut heights = Vec::new();
        for size in [13.0, 16.0] {
            let ui = UiFont { family: None, size };
            for folded in [false, true] {
                let panel = activity_view::height(width, &ui, folded);
                heights.push(panel);
                // The list's end and the panel's gap, as when the panel lay under the list, and the
                // footer, which lies over the list too (P5-70b).
                let gap = LIST_END + ui.scale(ACTIVITY_GAP);
                let footer = ui.scale(FOOTER);
                assert_eq!(
                    list_end(width, &ui, folded, false),
                    panel + gap + footer,
                    "{size} {folded}"
                );
                assert_eq!(list_bottom(&ui, false), 0.0);
                // On the frosted sidebar the footer shows the material, so the list stops at its
                // top (P5-75): the room under the last card leaves it out.
                assert_eq!(
                    list_end(width, &ui, folded, true),
                    panel + gap,
                    "{size} {folded} frosted"
                );
                assert_eq!(list_bottom(&ui, true), footer);
            }
        }
        // The room follows the panel: folding it, or a larger interface size, changes it.
        let [open, folded, open_large, folded_large] = heights[..] else {
            unreachable!()
        };
        assert!(folded > 0.0 && folded < open, "{heights:?}");
        assert!(open_large > open && folded_large > folded, "{heights:?}");
    }

    #[test]
    fn the_last_card_scrolls_clear_above_the_activity_panel() {
        // A list of cards taller than the column, scrolled to its end.
        let (column, cards) = (600.0, 1400.0);
        for size in [13.0, 16.0] {
            let ui = UiFont { family: None, size };
            for (folded, frosted) in [(false, false), (true, false), (false, true), (true, true)] {
                let panel = activity_view::height(300.0, &ui, folded);
                // The list's own height: the column, less what is kept under the footer.
                let shown = column - list_bottom(&ui, frosted);
                let scrolled = cards + list_end(300.0, &ui, folded, frosted) - shown;
                let last_bottom = cards - scrolled;
                // The footer lies at the column's foot and the panel on it; the last card ends above
                // the panel, by the gap.
                let panel_top = column - ui.scale(FOOTER) - panel;
                assert!(
                    last_bottom + LIST_END <= panel_top,
                    "{size} {folded} {frosted}: {last_bottom} {panel_top}"
                );
            }
        }
    }

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            cwd: Some(format!("/work/{name}")),
            instance: Some(format!("i-{name}")),
            ..Agent::default()
        }
    }

    /// A machine waiting while another is on show: the reply its active pane's agent just gave is
    /// new for the bell and the inbox, and read only once the user looks (P5-90 cross review 4).
    #[test]
    fn a_reply_on_a_machine_not_on_show_stays_unread_until_it_is() {
        let mut listing = Listing::default();
        let active = Some("p/main");
        let _ = listing.absorb(Ok(vec![agent("p/main", "working")]), None, 1000.0);
        let hidden = looking_at(active, false);
        let _ = listing.absorb(Ok(vec![agent("p/main", "idle")]), hidden, 1001.0);
        assert!(listing.panel.unread.contains("p/main"));
        // Still waiting, the next read leaves it so.
        let _ = listing.absorb(Ok(vec![agent("p/main", "idle")]), hidden, 1002.0);
        assert!(listing.panel.unread.contains("p/main"));
        // On show again: what the active pane shows is seen.
        let _ = listing.absorb(
            Ok(vec![agent("p/main", "idle")]),
            looking_at(active, true),
            1003.0,
        );
        assert!(!listing.panel.unread.contains("p/main"));
    }

    #[test]
    fn a_paused_working_agent_is_not_working_for_spinners_or_the_activity_glow() {
        let mut listing = Listing::default();
        let paused = Agent {
            paused: true,
            ..agent("p/a", "working")
        };
        let _ = listing.absorb(Ok(vec![paused]), None, 1000.0);
        assert!(!listing.animating(1000.0));
        let _ = listing.absorb(Ok(vec![agent("p/a", "working")]), None, 1000.0);
        assert!(listing.animating(1000.0));
    }

    fn shown(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| match line {
                Line::Error(text) => format!("! {text}"),
                Line::Group(heading) => format!("# {} ({})", heading.title, heading.count),
                Line::Agent(card) => format!("{} {}", card.short, card.look.label),
                other => panic!("no workspaces here: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn notes_that_went_well_fade_and_problems_stay() {
        let start = Instant::now();
        let secs = |s: f64| start + Duration::from_millis((s * 1000.0).round() as u64);
        let started = Note::new("Started a".into(), false, start, false);
        // Whole for the hold, then fading, then gone.
        assert_eq!(started.opacity(secs(3.9)), Some(1.0));
        assert!(!started.fading(secs(3.9)));
        let half = started.opacity(secs(4.3)).unwrap();
        assert!(half > 0.0 && half < 1.0, "{half}");
        assert!(started.fading(secs(4.3)));
        assert_eq!(started.opacity(secs(4.6)), None);
        assert!(!started.fading(secs(4.6)));
        // With Reduce Motion it goes at once when the hold ends.
        let still = Note::new("Stopped a".into(), false, start, true);
        assert_eq!(still.opacity(secs(3.9)), Some(1.0));
        assert_eq!(still.opacity(secs(4.0)), None);
        // A problem stays until the next note replaces it.
        let failed = Note::new("no such agent".into(), true, start, false);
        assert_eq!(failed.opacity(secs(600.0)), Some(1.0));
        assert!(!failed.fading(secs(600.0)));
        // A new note counts from when it came: still whole after the first one has gone.
        let next = Note::new("Stopping a…".into(), false, secs(3.0), false);
        assert_eq!(next.opacity(secs(5.0)), Some(1.0));
        assert_eq!(next.opacity(secs(7.6)), None);
    }

    #[test]
    fn groups_order_and_short_names() {
        let mut listing = Listing::default();
        let alive = listing.absorb(
            Ok(vec![
                agent("paddock/main", "idle"),
                agent("solo", "working"),
                agent("paddock/dev-agents", "working"),
                agent("paddock/dev-theme", "blocked"),
                agent("saddle/main", "exited"),
            ]),
            None,
            1000.0,
        );
        assert_eq!(
            alive.unwrap(),
            [
                "paddock/main",
                "solo",
                "paddock/dev-agents",
                "paddock/dev-theme"
            ]
        );
        let lines = listing.lines(None, &[], None, 1000.0);
        // Groups in name order, the ungrouped first; within a group, those needing a person first.
        assert_eq!(
            shown(&lines),
            [
                "# agents/ (1)",
                "solo Working",
                "# paddock/ (3)",
                "dev-theme Waiting",
                "dev-agents Working",
                "main Idle",
                "# saddle/ (1)",
                "main Exited",
            ]
        );
        let Line::Agent(card) = &lines[3] else {
            panic!()
        };
        assert_eq!(card.name, "paddock/dev-theme");
        assert_eq!(card.cwd.as_deref(), Some("/work/paddock/dev-theme"));
        assert_eq!(card.instance.as_deref(), Some("i-paddock/dev-theme"));
        assert_eq!(
            listing.cwds(),
            [
                "/work/paddock/dev-agents",
                "/work/paddock/dev-theme",
                "/work/paddock/main",
                "/work/saddle/main",
                "/work/solo"
            ]
        );
    }

    #[test]
    fn dragging_the_divider_follows_the_mouse_within_reach() {
        // Pressed at x 300 on a 300-point sidebar: the width moves as far as the mouse.
        assert_eq!(resize(300.0, 300.0, 340.0, MIN_WIDTH), 340.0);
        assert_eq!(resize(300.0, 302.0, 250.0, MIN_WIDTH), 248.0);
        // Never narrower than 220 nor wider than 560, however far the mouse goes.
        assert_eq!(resize(300.0, 300.0, 100.0, MIN_WIDTH), MIN_WIDTH);
        assert_eq!(resize(300.0, 300.0, -50.0, MIN_WIDTH), 220.0);
        assert_eq!(resize(300.0, 300.0, 900.0, MIN_WIDTH), MAX_WIDTH);
        assert_eq!(resize(300.0, 300.0, 560.5, MIN_WIDTH), 560.0);
        // Whole points, for the config file.
        assert_eq!(resize(300.0, 300.0, 333.4, MIN_WIDTH), 333.0);
        // A width set outside the range in Settings comes back into it once dragged.
        assert_eq!(resize(180.0, 180.0, 181.0, MIN_WIDTH), 220.0);
    }

    #[test]
    fn the_narrowest_sidebar_still_holds_the_header_row_in_the_title_bar() {
        // At the base interface size the bell goes compact before the row runs out of room, and
        // the compact row, with its two-digit count in a pill, `New Workspace…` and the button for
        // every agent before the bell, needs more than the usual narrowest sidebar.
        let base = UiFont::default();
        assert_eq!(min_width(&base).round(), 284.0);
        assert!(compact_bell(
            min_width(&base),
            false,
            &base,
            &HeadWords::reckoned(&base)
        ));
        assert!(!compact_bell(
            330.0,
            false,
            &base,
            &HeadWords::reckoned(&base)
        ));
        // Larger interface sizes need a wider sidebar, and dragging keeps to it.
        let large = UiFont {
            family: None,
            size: 16.0,
        };
        let larger = UiFont {
            family: None,
            size: 20.0,
        };
        assert!(min_width(&large) > MIN_WIDTH);
        assert!(min_width(&larger) > min_width(&large));
        assert!(compact_bell(
            min_width(&large),
            false,
            &large,
            &HeadWords::reckoned(&large)
        ));
        assert_eq!(
            resize(300.0, 300.0, 100.0, min_width(&large)),
            min_width(&large).round()
        );
        // Smaller ones need less, down to 220.
        let small = UiFont {
            family: None,
            size: 11.0,
        };
        assert!(min_width(&small) < min_width(&base));
        let smaller = UiFont {
            family: None,
            size: 8.0,
        };
        assert_eq!(min_width(&smaller), MIN_WIDTH);
        // In full screen there are no traffic lights: the row has more room.
        assert!(compact_bell(
            265.0,
            false,
            &base,
            &HeadWords::reckoned(&base)
        ));
        assert!(!compact_bell(
            265.0,
            true,
            &base,
            &HeadWords::reckoned(&base)
        ));
    }

    #[test]
    fn a_config_width_narrower_than_the_header_row_shows_at_the_narrowest() {
        let base = UiFont::default();
        let large = UiFont {
            family: None,
            size: 18.0,
        };
        // Narrower than the row needs: drawn at the narrowest, in whole points.
        assert_eq!(fit_width(200.0, &base), 284.0);
        assert_eq!(fit_width(283.0, &base), 284.0);
        assert_eq!(fit_width(284.0, &large), min_width(&large));
        assert!(fit_width(284.0, &large) > 284.0);
        // Wide enough: as written.
        assert_eq!(fit_width(284.0, &base), 284.0);
        assert_eq!(fit_width(300.5, &base), 300.5);
        // A drag from there keeps to the range, and what it saves is no narrower.
        for ui in [&base, &large] {
            let fitted = fit_width(100.0, ui);
            let dragged = resize(fitted, 400.0, 399.0, min_width(ui));
            assert!(dragged >= min_width(ui), "{dragged}");
            assert_eq!(dragged, dragged.round());
        }
    }

    #[test]
    fn the_header_row_keeps_room_for_the_new_workspace_button_without_agents() {
        for size in [13.0, 18.0] {
            let ui = UiFont { family: None, size };
            let words = HeadWords {
                title: ui.scale(43.0),
                count: None,
                pause_all: false,
                bell: None,
            };
            let bell = ui.scale(PILL_X + footer_icon::SIZE + PILL_X);
            let rest = LIGHTS + ui.scale(TOGGLE + TITLE_GAP) + words.title + bell + PAD;
            let need = head_width(LIGHTS, false, &ui, &words);
            assert!(
                (need - rest - ui.scale(NEW_WORKSPACE + PAUSE_GAP)).abs() < 1e-3,
                "{size}"
            );
        }
    }

    #[test]
    fn the_bell_goes_compact_by_its_words_as_set() {
        for size in [13.0, 18.0] {
            let ui = UiFont { family: None, size };
            // Digits about as wide as the system font sets them, at this size.
            let digits = |n: usize| ui.scale(7.0) * n as f32;
            let words = |count: usize, bell: usize, pause_all: bool| HeadWords {
                title: ui.scale(43.0),
                count: (count > 0).then(|| digits(count)),
                pause_all,
                bell: (bell > 0).then(|| digits(bell)),
            };
            let need = |words: &HeadWords| head_width(LIGHTS, false, &ui, words);
            // The bell is a pill exactly while the row has room for everything in it.
            for words in [
                words(1, 1, true),
                words(2, 2, true),
                words(3, 3, true),
                words(0, 0, false),
            ] {
                let width = need(&words);
                assert!(!compact_bell(width, false, &ui, &words), "{size} {words:?}");
                assert!(
                    compact_bell(width - 0.5, false, &ui, &words),
                    "{size} {words:?}"
                );
            }
            // A longer number needs as much more room as it is wider.
            let (one, three) = (need(&words(1, 1, true)), need(&words(1, 3, true)));
            assert!((three - one - digits(2)).abs() < 1e-3, "{size}");
            // With one digit each the pill fits where two were reckoned for, and with three it no
            // longer fits where the reckoning said it would.
            let reckoned = HeadWords::reckoned(&ui);
            let at = need(&reckoned);
            assert!(!compact_bell(at - 1.0, false, &ui, &words(1, 1, true)));
            assert!(compact_bell(at, false, &ui, &words(2, 3, true)));
            // Without agents there is no count and no button for every agent: more room still.
            assert!(need(&words(0, 0, false)) < need(&words(1, 0, true)));
            assert!(
                (need(&words(1, 0, true))
                    - need(&words(1, 0, false))
                    - ui.scale(PAUSE_ALL + PAUSE_GAP))
                .abs()
                    < 1e-3
            );
        }
    }

    #[test]
    fn a_tile_shows_the_first_letter_after_a_role_prefix() {
        assert_eq!(initial("main"), "M");
        assert_eq!(initial("dev-fonts"), "F");
        assert_eq!(initial("test-m2"), "M");
        assert_eq!(initial("review-p5"), "P");
        // Only a prefix at the start goes, and only one.
        assert_eq!(initial("fonts-dev"), "F");
        assert_eq!(initial("dev-test-a"), "T");
        // Digits count; marks before the first letter do not.
        assert_eq!(initial("2fa"), "2");
        assert_eq!(initial("_x"), "X");
        assert_eq!(initial("ärger"), "Ä");
        // Nothing after the prefix: the name as it is.
        assert_eq!(initial("dev-"), "D");
        assert_eq!(initial("--"), "?");
        assert_eq!(initial(""), "?");
    }

    #[test]
    fn stop_names_the_active_agent_and_is_off_without_one() {
        assert_eq!(
            stop_item(Some("paddock/main")),
            ("Stop paddock/main…".to_owned(), true)
        );
        assert_eq!(stop_item(None), ("Stop Agent…".to_owned(), false));
    }

    #[test]
    fn pause_names_the_active_agent_and_resumes_a_paused_one() {
        assert_eq!(
            pause_item(Some("paddock/main"), false),
            ("Pause paddock/main".to_owned(), true)
        );
        assert_eq!(
            pause_item(Some("paddock/main"), true),
            ("Resume paddock/main".to_owned(), true)
        );
        assert_eq!(pause_item(None, false), ("Pause Agent".to_owned(), false));
    }

    #[test]
    fn the_menu_opens_over_its_button_in_both_shapes() {
        let base = UiFont::default();
        // Expanded: in line with the button, whose icon's ink starts at the Activity card's outer
        // edge (10 in), its box 7.5 into the button and the rails 1.5 of 14 into the box, the
        // button never starting left of the sidebar; collapsed: the button centred in the strip.
        let near = |(left, bottom): (Pixels, Pixels), want: (f32, f32)| {
            assert!(
                (f32::from(left) - want.0).abs() < 0.01,
                "{left:?}, {want:?}"
            );
            assert_eq!(f32::from(bottom), want.1);
        };
        near(
            menu_anchor(false, &base),
            (10.0 - 7.5 - 1.5 * 17.0 / 14.0, 47.0),
        );
        assert_eq!(menu_anchor(true, &base), (px(10.0), px(47.0)));
        // A larger interface size: the bigger button would start left of the sidebar, so it starts
        // at its edge.
        let large = UiFont {
            size: 26.0,
            ..UiFont::default()
        };
        near(menu_anchor(false, &large), (0.0, 94.0));
        assert_eq!(menu_anchor(true, &large), (px(20.0), px(94.0)));
    }

    #[test]
    fn status_refreshes_with_each_listing() {
        let mut listing = Listing::default();
        listing.absorb(Ok(vec![agent("p/a", "working")]), None, 1000.0);
        assert_eq!(
            shown(&listing.lines(None, &[], None, 1000.0))[1],
            "a Working"
        );
        listing.absorb(Ok(vec![agent("p/a", "idle")]), None, 1001.0);
        assert_eq!(shown(&listing.lines(None, &[], None, 1001.0))[1], "a Idle");
        listing.absorb(Ok(vec![]), None, 1002.0);
        assert!(listing.lines(None, &[], None, 1002.0).is_empty());
    }

    #[test]
    fn errors_and_stalls_come_from_the_panel() {
        // Errors and stalls come from the panel's judgement, not just corral's state.
        let mut listing = Listing::default();
        let mut broken = agent("p/broken", "idle");
        broken.error = Some("status failed".into());
        let mut quiet = agent("p/quiet", "working");
        quiet.last_output = Some(0.0);
        listing.absorb(Ok(vec![broken, quiet]), None, 1000.0);
        assert_eq!(
            shown(&listing.lines(None, &[], None, 1000.0)),
            ["# p/ (2)", "broken Error", "quiet Stalled"]
        );
    }

    /// A fake `corral` in a temporary directory; never the real one.
    fn fake_corral(name: &str, script: &str) -> (std::path::PathBuf, String) {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "paddock-sidebar-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corral");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, path.display().to_string())
    }

    fn first_update(program: String) -> Result<Vec<Agent>> {
        let poller = Poller::start(Client::local(program), Duration::from_secs(60));
        poller
            .updates
            .recv_timeout(Duration::from_secs(20))
            .expect("poller result")
            .1
    }

    #[test]
    fn a_failed_read_ends_no_agent() {
        use crate::window::{AgentPane, ended_panes};
        let (_, failing) = fake_corral(
            "failing-ends",
            r#"echo '{"ok": false, "error": "daemon unreachable"}'; exit 1"#,
        );
        let mut listing = Listing::default();
        listing.absorb(Ok(vec![agent("p/a", "idle")]), None, 1000.0);
        // No listing goes to the window, which checks its panes only against one it is given.
        assert!(
            listing
                .absorb(first_update(failing), None, 1001.0)
                .is_none()
        );
        // And the list kept still has the agent running.
        let pane = AgentPane {
            pane: 1,
            name: "p/a".into(),
            instance: Some("i-p/a".into()),
            attached: false,
        };
        assert!(ended_panes(&[pane], &listing.panel.agents).is_empty());
    }

    #[test]
    fn corral_errors_become_a_line_and_keep_the_list() {
        let (dir, failing) = fake_corral(
            "failing",
            r#"echo '{"ok": false, "error": "daemon unreachable"}'; exit 1"#,
        );
        let (_, garbage) = fake_corral("garbage", "echo not json");
        let mut listing = Listing::default();
        listing.absorb(Ok(vec![agent("p/a", "idle")]), None, 1000.0);

        assert!(
            listing
                .absorb(first_update(failing), None, 1001.0)
                .is_none()
        );
        let lines = shown(&listing.lines(None, &[], None, 1001.0));
        assert!(lines[0].starts_with("! corral: "), "{lines:?}");
        assert!(lines[0].contains("daemon unreachable"), "{lines:?}");
        assert_eq!(lines[1..], ["# p/ (1)", "a Idle"]);

        assert!(
            listing
                .absorb(first_update(garbage), None, 1002.0)
                .is_none()
        );
        let lines = shown(&listing.lines(None, &[], None, 1002.0));
        assert!(lines[0].contains("invalid JSON"), "{lines:?}");

        let missing = dir.join("no-such-corral").display().to_string();
        assert!(
            listing
                .absorb(first_update(missing), None, 1003.0)
                .is_none()
        );
        assert!(shown(&listing.lines(None, &[], None, 1003.0))[0].starts_with("! corral: "));

        // The next good listing clears the error.
        listing.absorb(Ok(vec![agent("p/a", "working")]), None, 1004.0);
        assert_eq!(
            shown(&listing.lines(None, &[], None, 1004.0)),
            ["# p/ (1)", "a Working"]
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(dir.with_file_name(format!(
            "paddock-sidebar-test-{}-garbage",
            std::process::id()
        )))
        .unwrap();
    }

    #[test]
    fn a_reply_is_read_once_for_each_spell_of_idling_or_waiting() {
        // Each call is logged and numbered, so a repeated call shows in the next reply.
        let (dir, program) = fake_corral(
            "replies",
            r#"calls="$(dirname "$0")/calls"
case "$1" in
reply) echo "$2" >> "$calls"
  n=$(wc -l < "$calls" | tr -d ' ')
  printf '{"ok": true, "name": "%s", "text": "Reply %s.\\n\\nAll green.", "at": 1}\n' "$2" "$n" ;;
esac"#,
        );
        let replies = Replies::start(Client::local(program));
        let mut listing = Listing::default();
        let idle = |since: f64| Agent {
            state_started: Some(since),
            ..agent("p/a", "idle")
        };
        // Listed with it all along: an agent waiting since before, whose card shows its question
        // and whose last reply is read once too, for the Inbox.
        let with_asking = |a: Agent| {
            let asking = Agent {
                state_started: Some(980.0),
                ..agent("q/ask", "blocked")
            };
            Ok(vec![a, asking])
        };
        // Waits until the card shows the reply read for its current spell.
        let shown = |listing: &mut Listing, now: f64| -> String {
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            loop {
                listing.absorb_replies(&replies);
                if let Line::Agent(card) = &listing.lines(None, &[], None, now)[1]
                    && let Some(card::Preview::Reply(text)) = &card.preview
                {
                    return text.clone();
                }
                assert!(std::time::Instant::now() < deadline, "no reply shown");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        // Listed again and again while it idles: corral is asked once.
        for now in [1000.0, 1001.0, 1002.0] {
            listing.absorb(with_asking(idle(990.0)), None, now);
            listing.ask_replies(&replies, now);
        }
        assert_eq!(shown(&mut listing, 1002.0), "Reply 1. All green.");
        for now in [1003.0, 1004.0] {
            listing.absorb(with_asking(idle(990.0)), None, now);
            listing.ask_replies(&replies, now);
        }
        // Another turn ends in a new spell: asked once more, as the second call.
        listing.absorb(with_asking(agent("p/a", "working")), None, 1005.0);
        listing.ask_replies(&replies, 1005.0);
        for now in [1010.0, 1011.0] {
            listing.absorb(with_asking(idle(1009.0)), None, now);
            listing.ask_replies(&replies, now);
        }
        assert_eq!(shown(&mut listing, 1011.0), "Reply 3. All green.");
        // The idle agent once for each spell, the waiting one once in all.
        let calls = std::fs::read_to_string(dir.join("calls")).unwrap();
        assert_eq!(calls.lines().collect::<Vec<_>>(), ["p/a", "q/ask", "p/a"]);
        drop(replies);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fake_corral_listing_reaches_the_cards() {
        let (dir, program) = fake_corral(
            "listing",
            r#"case "$1" in
ls) echo '{"agents": [{"name": "p/a", "cwd": "/tmp/a", "instance": "i1"}]}' ;;
status) echo '{"name": "p/a", "state": "working", "instance": "i1"}' ;;
esac"#,
        );
        let mut listing = Listing::default();
        let alive = listing.absorb(first_update(program), None, 1000.0).unwrap();
        assert_eq!(alive, ["p/a"]);
        let lines = listing.lines(None, &[], None, 1000.0);
        assert_eq!(shown(&lines), ["# p/ (1)", "a Working"]);
        let Line::Agent(card) = &lines[1] else {
            panic!()
        };
        assert_eq!(card.cwd.as_deref(), Some("/tmp/a"));
        assert_eq!(card.instance.as_deref(), Some("i1"));
        // Before the first Git round the place line is only the directory.
        assert_eq!(
            card.place,
            card::Place {
                dir: "/tmp/a".into(),
                branch: None,
                changes: None,
                untracked: 0,
                ahead: 0,
            }
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn quiet_text_reads_on_the_frost_and_stays_in_full_screen() {
        // WCAG relative luminance: linearize sRGB before weighting its channels.
        fn luminance((r, g, b): crate::palette::Rgb) -> f64 {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }
        fn contrast(a: crate::palette::Rgb, b: crate::palette::Rgb) -> f64 {
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }
        let rgb = |color: Hsla| {
            let color = Rgba::from(color);
            let channel = |value: f32| (value * 255.0).round() as u8;
            (channel(color.r), channel(color.g), channel(color.b))
        };

        for name in crate::preset::Preset::ALL.map(crate::preset::Preset::name) {
            let given = Rc::new(
                Theme::from_config(&crate::config::Config {
                    theme: Some(name.into()),
                    ..crate::config::Config::default()
                })
                .unwrap(),
            );
            let frosted = column_theme(&given, true);
            // What the quiet text lies on over the material: on a dark theme, the material
            // under a selected card, as the user's screenshot shows it; on a light one
            // (P5-175), the column, the material's light look as a screenshot shows it washed
            // with the sidebar's colour, and the cards' whole faces on it.
            let grounds = if given.is_light() {
                let base = hsla(given.bg(|t| t.agents_bg), 1.0);
                let material = gpui::rgb(0xe6e6e6).into();
                let column = over(base.opacity(given.frost().wash), material);
                let faces = Grounds::of(&frosted, true);
                let waiting = over(faces.waiting, faces.card);
                [column, faces.card, faces.card_selected, waiting]
                    .map(rgb)
                    .to_vec()
            } else {
                vec![(0x45, 0x49, 0x4a)]
            };
            for ground in grounds {
                let dim = contrast(frosted.fg(|t| t.agents_dim), ground);
                assert!(
                    dim >= 4.5,
                    "{name} agents_dim: {dim:.2}:1 < 4.5:1 on {ground:?}"
                );
                let dimmer = contrast(frosted.fg(|t| t.agents_dimmer), ground);
                assert!(
                    dimmer >= 3.5,
                    "{name} agents_dimmer: {dimmer:.2}:1 < 3.5:1 on {ground:?}"
                );
            }

            let full_screen = column_theme(&given, false);
            for key in crate::theme::color_keys() {
                assert_eq!(full_screen.color(key), given.color(key), "{name} {key}");
            }
        }
    }

    /// The switcher writes another machine's agents after its name, all its corral holds, and
    /// nothing when it holds none; never on This Mac (P5-122 rework 1).
    #[test]
    fn another_machines_capsule_says_how_many_agents_it_holds() {
        assert_eq!(agents_written(1, 3), Some(3));
        assert_eq!(agents_written(2, 1), Some(1));
        assert_eq!(agents_written(1, 0), None);
        assert_eq!(agents_written(0, 5), None);
    }
}
