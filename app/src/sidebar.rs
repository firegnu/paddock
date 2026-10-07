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
    agents::{Panel, Spell, Status},
    attention,
    card::{self, Card, Click, Ink, Line, Pick, Preview, Tone},
    corral::{Agent, Client, Poller, Replies, Role},
    fonts::UiFont,
    footer_icon::{self, Icon, Pose},
    git, kind_icon,
    motion::{self, Hover, HoverMotion},
    pause, reduce_motion,
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
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

/// What the sidebar asks of the window.
pub enum SidebarEvent {
    /// Show this agent: where it already is, or by the layout rules.
    Attach {
        name: String,
        metadata: AgentMetadata,
    },
    /// Open the New Agent window.
    NewAgent,
    /// Open or close the Attention list.
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
}

/// How often the agents' worktrees are summarised, as in Saddle.
const GIT_REFRESH: Duration = Duration::from_secs(5);

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
/// How strongly the kind's colour tints the avatar, and waiting amber the card.
const AVATAR_TINT: f32 = 0.13;
const WAITING_TINT: f32 = 0.07;
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
const KIND_SIZE: f32 = 11.0;
const LABEL_SIZE: f32 = 10.5;
const CELL_LABEL_SIZE: f32 = 9.5;
const BADGE_SIZE: f32 = 9.0;
const TILE_SIZE: f32 = 12.5;
/// The preview's lines, folded and open.
const PREVIEW_LINES: usize = 2;
const PREVIEW_OPEN_LINES: usize = 6;
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
/// The footer, and its menu button at the bottom left and the icon in it; in the strip the button
/// is centred.
const FOOTER: f32 = 50.0;
const BUTTON: f32 = 32.0;
const BUTTON_ICON: f32 = 17.0;
/// The menu button's left edge, unscaled: in line with the cards' avatars, past the list's side, a
/// card's edge and its padding.
const BUTTON_LEFT: f32 = PAD + 1.0 + CARD_LEFT;
/// How long a note of a start or stop that went well stays whole, and then how long it fades.
const NOTE_HOLD: Duration = Duration::from_secs(4);
const NOTE_FADE: Duration = Duration::from_millis(600);
/// A card's icon buttons, Copy, Pause (or Resume) and Stop…, and their corners.
const CARD_BUTTON: f32 = 24.0;
const CARD_BUTTON_RADIUS: f32 = 7.0;
/// How strongly a paused agent's card and tile show.
const PAUSED_OPACITY: f32 = 0.55;
/// What a card asks before pausing an agent in a turn.
const PAUSE_WORKING: &str = "Working \u{2014} pause anyway?";
/// An agent's tile in the strip.
const TILE: f32 = 34.0;
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
/// the Attention list is empty).
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
/// compact or as a pill.
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
    start + ui.scale(TOGGLE + TITLE_GAP) + words.title + count + pause_all + bell + PAD
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
        px(BUTTON_LEFT)
    };
    (left, ui.px((FOOTER - BUTTON) / 2.0 + BUTTON + 6.0))
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
    git: git::Poller,
    /// Reads last replies for the cards' previews.
    replies: Replies<Spell>,
    /// The active pane's agent.
    selected: Option<String>,
    /// Every agent open in this window's panes.
    here: Vec<String>,
    /// Written `~` in directories.
    home: Option<String>,
    /// The result of the last start or stop, until it fades.
    note: Option<Note>,
    /// Collapsed to the narrow strip.
    collapsed: bool,
    /// The menu of actions is open: its button stays lit.
    menu_open: bool,
    /// The card asking whether to pause its agent in a turn, until the mouse leaves it.
    asking: Option<String>,
    /// Its column shows the system's sidebar material: its grounds are tints over it.
    frosted: bool,
    /// The activity grid over the footer (P5-32).
    activity: Entity<ActivityView>,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    pub fn new(
        theme: Rc<Theme>,
        width: f32,
        mono: Font,
        corral: String,
        refresh: Duration,
        cx: &mut Context<Self>,
    ) -> Self {
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
            cx.emit(SidebarEvent::ActivityFolded(folded.0))
        })
        .detach();
        Self {
            given: theme.clone(),
            theme,
            width,
            mono,
            listing: Listing::default(),
            replies: Replies::start(Client {
                program: corral.clone(),
            }),
            poller: Poller::start(Client { program: corral }, refresh),
            git: git::Poller::start("git".into(), GIT_REFRESH),
            selected: None,
            here: Vec::new(),
            home: std::env::var("HOME").ok(),
            note: None,
            collapsed: false,
            menu_open: false,
            asking: None,
            frosted: false,
            activity,
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

    /// Collapsed to the strip, or expanded.
    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.collapsed != collapsed {
            self.collapsed = collapsed;
            cx.notify();
        }
    }

    /// The activity panel folded to one line, as the layout saved it.
    pub fn set_activity_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        self.activity
            .update(cx, |activity, cx| activity.set_folded(folded, cx));
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

    /// Lists the agents again now rather than at the next interval.
    pub fn refresh(&self) {
        self.poller.refresh();
    }

    /// The header row, in the title bar after the traffic lights (none in `full_screen`): the
    /// collapse button, `Agents` and how many, and at the right end the button for every agent and
    /// the bell, compact when the row, measured as it is set, has no room for it as a pill;
    /// collapsed to the strip, only the expand button, where the collapse button was. `spot` goes
    /// in the bell, for the window to hang the Attention list from. A press on a button is not the
    /// start of a drag.
    pub fn head(
        &mut self,
        full_screen: bool,
        spot: AnyElement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let ui = UiFont::get(cx);
        let keep = |_: &MouseDownEvent, _: &mut Window, cx: &mut App| cx.stop_propagation();
        let gap = ui.px(if self.collapsed {
            TOGGLE_GAP
        } else {
            TITLE_GAP
        });
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
        if self.collapsed {
            return row;
        }
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let agents = self
            .listing
            .lines(
                self.selected.as_deref(),
                &self.here,
                self.home.as_deref(),
                now(),
            )
            .iter()
            .filter(|line| matches!(line, Line::Agent(_)))
            .count();
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
        let pause_all = pause_all.map(|button| {
            button.map(move |button| button.mr(pause_gap).on_mouse_down(MouseButton::Left, keep))
        });
        row.child(
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
        .children(pause_all)
        .child(bell.map(move |bell| bell.on_mouse_down(MouseButton::Left, keep)))
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

    /// The header's bell, always there so the Attention list has a way in: how many rows the list
    /// has, in amber when an agent needs a person, in the accent for new replies only, quiet and
    /// without a number when there is nothing. A click opens the Attention list.
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

    /// How many rows the Attention list has, and the bell's colour for them.
    fn bell(&self) -> (usize, Hsla) {
        let (count, urgency) = attention::bell(&self.attention());
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

    /// The strip's bell: the icon, and when the Attention list has rows, their count beside it in
    /// a small pill tinted in the bell's colour, never over the icon.
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

    /// The collapsed strip: the bell and the button for every agent, then a tile for each agent,
    /// the projects set apart by short rules, and the menu button at the bottom.
    fn rail(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        let lines = self.listing.lines(
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
        let mut first = true;
        for line in lines {
            match line {
                Line::Error(_) => {}
                Line::Group(..) => {
                    if !first {
                        tiles = tiles.child(rule(16.0, 6.0));
                    }
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
            .flex()
            .flex_col()
            .items_center()
            .line_height(relative(1.3))
            .pt(ui.px(8.0))
            .child(self.rail_bell(cx).map({
                let top = ui.px(2.0);
                move |bell| bell.mt(top)
            }))
            .children(self.pause_all_button(cx).map(|button| {
                let top = ui.px(4.0);
                button.map(move |button| button.mt(top))
            }))
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

    /// What needs looking at now, for the Attention list.
    pub fn attention(&self) -> Vec<attention::Item> {
        attention::items(&self.listing.panel, self.listing.error.as_deref(), now())
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
        for update in self.poller.updates.try_iter().collect::<Vec<_>>() {
            changed = true;
            if let Some(alive) = self.listing.absorb(update, self.selected.as_deref(), now) {
                cx.emit(SidebarEvent::Alive(alive));
            }
        }
        if changed {
            self.git.watch(self.listing.cwds());
            let cwds = self.listing.cwds();
            self.activity.update(cx, |activity, _| activity.watch(cwds));
            self.listing.ask_replies(&self.replies, now);
        }
        for batch in self.git.updates.try_iter().collect::<Vec<_>>() {
            changed = true;
            self.listing.absorb_git(batch);
        }
        changed |= self.listing.absorb_replies(&self.replies);
        // Working agents' ages tick; their dots breathe by themselves.
        if changed || self.listing.animating(now) {
            cx.notify();
        }
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
        CardActions {
            reply: Box::new(reply),
            copy,
            pause: Box::new(pause),
            confirm: Box::new(confirm),
            cancel: Box::new(cancel),
            leave: Box::new(leave),
            stop: Box::new(stop),
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
        if self.collapsed {
            return self.rail(cx);
        }
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
        let (lines, note) = {
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
            let mono = |line: &str| measure(line, KIND_SIZE, self.mono.clone());
            // The words beside the avatar.
            let room = words_width(self.width, &ui);
            let mut lines = self.listing.lines(
                self.selected.as_deref(),
                &self.here,
                self.home.as_deref(),
                now,
            );
            for line in &mut lines {
                let Line::Agent(card) = line else { continue };
                // The room the first line leaves the name.
                let name_room = |card: &Card| {
                    let mut left = room
                        - ui.scale(MARK_GAP)
                        - TIME_PAD
                        - width(&card.time, NOTE_SIZE, FontWeight::NORMAL, &tabular());
                    if card.unread {
                        left -= ui.scale(MARK_GAP + UNREAD_BOX);
                    }
                    if card.here {
                        left -= ui.scale(MARK_GAP + HERE_BOX);
                    }
                    if let Some(effort) = &card.effort {
                        left -= ui.scale(MARK_GAP) + normal(effort, NOTE_SIZE);
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
                if let Some(path) = &card.details.path {
                    card.details.path = Some(card::shorten(path, &|path| mono(path) <= room));
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
            let note = self.note.as_ref().zip(shown).map(|(note, opacity)| {
                let text = card::elide(&note.text, &|cut| normal(cut, NOTE_SIZE) <= room);
                // Cut short (agents that could not be paused, say), it shows whole on hover.
                let whole = (text != note.text).then(|| note.text.clone());
                (text, whole, note.problem, opacity)
            });
            (lines, note)
        };
        let agents = lines
            .iter()
            .filter(|line| matches!(line, Line::Agent(_)))
            .count();

        let mut list = div()
            .id("agents")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(PAD))
            .pt(px(2.0))
            .pb(px(10.0));
        if agents == 0 && self.listing.loaded {
            // Nothing listed: why, or how to start one, in the middle of the list.
            let quiet = match lines.first() {
                Some(Line::Error(text)) => quiet(
                    &theme,
                    &ui,
                    "Can't read corral",
                    text,
                    None,
                    None,
                    grounds.selected,
                ),
                _ => quiet(
                    &theme,
                    &ui,
                    "No agents yet",
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
            list = list.justify_center().child(quiet);
        }
        for (index, line) in lines.into_iter().filter(|_| agents > 0).enumerate() {
            list = list.child(match line {
                // The list corral last gave stays below this.
                Line::Error(text) => div()
                    .px(px(8.0))
                    .pt(px(6.0))
                    .text_size(ui.px(NOTE_SIZE))
                    .text_color(fg(|t| t.agents_red))
                    .child(text)
                    .into_any_element(),
                Line::Group(title, count, bar) => div()
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .px(px(8.0))
                    // Closer to the top for the first: the window's title bar is over it.
                    .pt(px(if index == 0 { 10.0 } else { 14.0 }))
                    .pb(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(ui.px(6.0))
                            .text_size(ui.px(LABEL_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                div()
                                    .text_color(fg(|t| t.agents_dimmer))
                                    .child(title.trim_end_matches('/').to_uppercase()),
                            )
                            .child(
                                div()
                                    .text_color(fg(|t| t.agents_dimmer).opacity(0.6))
                                    .child(count.to_string()),
                            ),
                    )
                    .child(div().flex_1())
                    // How the group's agents are doing, a short stroke each.
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .gap(px(2.0))
                            .w(ui.px((BAR_SHARE * bar.len() as f32).min(BAR_MAX)))
                            .children(bar.into_iter().map(|status| {
                                div()
                                    .flex_1()
                                    .h(px(3.0))
                                    .rounded(px(2.0))
                                    .bg(fg(card::look(status).color))
                            })),
                    )
                    .into_any_element(),
                Line::Agent(card) => {
                    let on_click = {
                        let card = card.clone();
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.click(&card, cx))
                    };
                    let actions = self.card_actions(&card, cx);
                    AgentCard {
                        asking: self.asking.as_ref() == Some(&card.name),
                        card: *card,
                        theme: theme.clone(),
                        mono: self.mono.clone(),
                        on_click: Box::new(on_click),
                        actions,
                        grounds,
                    }
                    .into_any_element()
                }
            });
        }

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
                .ml(ui.px(AVATAR + AVATAR_GAP - BUTTON))
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
        let footer = div()
            .flex_shrink_0()
            .h(ui.px(FOOTER))
            .flex()
            .items_center()
            .pl(px(BUTTON_LEFT))
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

        div()
            .flex_shrink_0()
            .w(px(self.width))
            .h_full()
            .flex()
            .flex_col()
            // Closer than GPUI's default, as in the design.
            .line_height(relative(1.3))
            .child(list)
            // Under the list, which scrolls on its own; the strip has none.
            .child(self.activity.clone())
            .child(footer)
            .into_any_element()
    }
}

/// The footer's note about the last start or stop.
struct Note {
    text: String,
    /// A problem: it stays, in red, until the next note.
    problem: bool,
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
            at,
            still,
        }
    }

    /// How much of it shows `now`: all of it while it holds, less as it fades, `None` once gone.
    fn opacity(&self, now: Instant) -> Option<f32> {
        if self.problem {
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
        !self.problem && now >= self.at + NOTE_HOLD && self.opacity(now).is_some()
    }
}

/// The grounds laid on the sidebar's column. On the sidebar's own colour they are whole colours;
/// over the system's sidebar material, tints of the text colour (amber for waiting) that light it
/// and let it through, as much as the preset says (`preset::Frost`).
#[derive(Clone, Copy)]
struct Grounds {
    /// The column itself: the sidebar's colour, or nothing over the material.
    base: Hsla,
    /// A selected card or tile, a lit button; a hovered one takes half of it.
    selected: Hsla,
    /// What a waiting card adds.
    waiting: Hsla,
    /// The strip's short rules.
    rule: Hsla,
}

impl Grounds {
    fn of(theme: &Theme, frosted: bool) -> Self {
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        if frosted {
            let frost = theme.frost();
            Self {
                base: gpui::transparent_black(),
                selected: fg(|t| t.agents_text).opacity(frost.lit),
                waiting: fg(|t| t.agents_yellow).opacity(frost.waiting),
                rule: fg(|t| t.agents_text).opacity(frost.lit),
            }
        } else {
            Self {
                base: hsla(theme.bg(|t| t.agents_bg), 1.0),
                selected: hsla(theme.bg(|t| t.agent_selected), 1.0),
                waiting: fg(|t| t.agents_yellow).opacity(WAITING_TINT),
                rule: fg(|t| t.agents_rule).opacity(0.8),
            }
        }
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
    title: &'static str,
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
                .child(title),
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
    grounds: Grounds,
    /// It asks whether to pause its agent in a turn.
    asking: bool,
}

impl RenderOnce for AgentCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let AgentCard {
            card,
            theme,
            mono,
            on_click,
            actions,
            grounds,
            asking,
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
        // The card's ground, on the sidebar's own colour a whole colour, so the ring round the
        // status badge matches it; over the system's material a tint, which only nears that.
        // Waiting adds amber.
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
            over(grounds.selected.opacity(0.5), ground)
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
            );

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

        let (text, colors) = pieces(&theme, &card.place.spans());
        let place = div()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(ui.px(NOTE_SIZE))
            .child(styled(text, &colors));

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
            };
            details(&theme, &mono, &ui, &card, buttons)
        });

        let badge = badge(&theme, &ui, &card, (ground, hovered), &group);
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
            .child(place)
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
            .border_color(if waiting {
                fg(|t| t.agents_yellow).opacity(0.22)
            } else {
                gpui::transparent_black()
            })
            .bg(ground)
            .when(!card.selected, |body| {
                body.hover(move |style| style.bg(hovered))
            })
            .cursor_pointer()
            .on_click(on_click)
            .on_hover(actions.leave)
            .when(card.status == Status::Paused, |body| {
                body.opacity(PAUSED_OPACITY)
            })
            .child(avatar)
            .child(words)
    }
}

/// The avatar: the kind's icon, or the name's letter for a kind without one, on a square tinted in
/// the kind's colour, with `badge` on its corner.
fn avatar(theme: &Theme, ui: &UiFont, card: &Card, badge: AnyElement) -> gpui::Stateful<Div> {
    let color = kind_color(theme, card);
    let mark = kind_mark(ui, card, color, CARD_NAME_SIZE);
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
        .bg(color.opacity(AVATAR_TINT))
        .child(mark)
        .child(badge)
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
/// `!` when stalled, otherwise a small dot in the status's colour.
fn badge(
    theme: &Theme,
    ui: &UiFont,
    card: &Card,
    (ground, hovered): (Hsla, Hsla),
    group: &SharedString,
) -> AnyElement {
    let color = hsla(theme.fg(card.look.color), 1.0);
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
                        dot.shadow(vec![BoxShadow {
                            color: color.opacity(0.6 * (1.0 - out)),
                            offset: point(px(0.0), px(0.0)),
                            blur_radius: px(0.0),
                            spread_radius: px(reach * out),
                            inset: false,
                        }])
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
            .child(div().size(ui.px(8.0)).rounded_full().bg(color))
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
}

/// The open card under the place line: the branch and changes as chips; the rest in two columns of
/// cells, a small label over each value; the full path; then the instance with Copy, and Pause (or
/// Resume) and Stop… at the right; under them the question before pausing an agent in a turn.
fn details(theme: &Theme, mono: &Font, ui: &UiFont, card: &Card, buttons: Buttons) -> Div {
    let Buttons {
        copy,
        pause,
        stop,
        asking,
    } = buttons;
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
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
    let cell = |cell: &card::Cell| {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(
                div()
                    .whitespace_nowrap()
                    .text_size(ui.px(CELL_LABEL_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg(|t| t.agents_dimmer))
                    .child(cell.label.to_uppercase()),
            )
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(SECOND_SIZE))
                    .text_color(fg(|t| t.agents_text))
                    .child(cell.value.clone()),
            )
    };
    let cells = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .children(details.cells.chunks(2).map(|pair| {
            div()
                .flex()
                .gap(ui.px(CELL_GAP))
                .children(pair.iter().map(cell))
                .when(pair.len() == 1, |row| row.child(div().flex_1()))
        }));
    let path = details.path.clone().map(|path| {
        div()
            .overflow_hidden()
            .whitespace_nowrap()
            .font(mono.clone())
            .text_size(ui.px(KIND_SIZE))
            .text_color(fg(|t| t.agents_dimmer))
            .child(path)
    });
    // An icon button with its hover text, quiet until hovered; what it looks like then is the
    // caller's. Made each frame from its motion's state.
    let action = |id: &'static str, tip: &'static str| {
        let tip = Tip {
            text: tip.into(),
            size: ui.px(NOTE_SIZE),
            color: fg(|t| t.agents_text),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: fg(|t| t.agents_rule),
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
    let dim = fg(|t| t.agents_dim);
    let instance = details.instance.clone().map(|instance| {
        div()
            .flex_shrink_0()
            .whitespace_nowrap()
            .font(mono.clone())
            .text_size(ui.px(KIND_SIZE))
            .text_color(fg(|t| t.agents_dim))
            .child(instance)
    });
    let last = div()
        .flex()
        .items_center()
        .gap(ui.px(MARK_GAP))
        .children(instance)
        .children(copy.map(|copy| {
            // It nudges as the pointer comes in, and turns to a green tick for a moment once it
            // has copied.
            let button = action("copy", "Copy instance ID");
            let (ground, green) = (fg(|t| t.agents_text).opacity(0.08), fg(|t| t.agents_green));
            motion::hover_motion("copy-motion", motion::COPY, move |hover| {
                let (icon, pose, opacity) = motion::copy(hover);
                let ink = if icon == Icon::Copied { green } else { dim };
                button(copy)
                    .hover(move |style| style.bg(ground))
                    .child(footer_icon::posed(icon, pose, ink.opacity(opacity), scale))
            })
        }))
        .child(div().flex_1())
        .child({
            // Its bars dip together as the pointer comes in; a paused agent's triangle slides the
            // way it plays.
            let lit = fg(|t| t.agents_text).opacity(0.08);
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
        let (text, rule, yellow) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_rule),
            fg(|t| t.agents_yellow),
        );
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
        .flex()
        .flex_col()
        .gap(px(10.0))
        .when(!details.chips.is_empty(), |body| {
            body.child(div().flex().flex_wrap().gap(ui.px(5.0)).children(chips))
        })
        .child(cells)
        .children(path)
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
        div()
            .id(ElementId::Name(SharedString::from(card.name.clone())))
            .group(group.clone())
            .relative()
            .flex_shrink_0()
            .size(ui.px(TILE))
            .my(ui.px(2.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(9.0))
            .cursor_pointer()
            .bg(ground)
            .border_1()
            .border_color(edge)
            .when(!card.selected, |tile| {
                tile.hover(move |style| style.bg(hovered))
            })
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .on_click(self.on_click)
            .when(card.status == Status::Paused, |tile| {
                tile.opacity(PAUSED_OPACITY)
            })
            .child(kind_mark(&ui, card, kind_color(theme, card), TILE_SIZE))
            // The mark hangs over the strip: ringed in the strip's colour.
            .child(badge(
                theme,
                &ui,
                card,
                (grounds.base, grounds.base),
                &group,
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
        let group = card
            .name
            .strip_suffix(&card.short)
            .filter(|prefix| !prefix.is_empty())
            .map(str::to_owned);
        Self {
            group,
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

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            cwd: Some(format!("/work/{name}")),
            instance: Some(format!("i-{name}")),
            ..Agent::default()
        }
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
                Line::Group(title, count, _) => format!("# {title} ({count})"),
                Line::Agent(card) => format!("{} {}", card.short, card.look.label),
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
        // the compact row, with its two-digit count in a pill and the button for every agent
        // before the bell, needs more than the usual narrowest sidebar.
        let base = UiFont::default();
        assert_eq!(min_width(&base).round(), 254.0);
        assert!(compact_bell(
            min_width(&base),
            false,
            &base,
            &HeadWords::reckoned(&base)
        ));
        assert!(!compact_bell(
            300.0,
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
            size: 10.0,
        };
        assert_eq!(min_width(&smaller), MIN_WIDTH);
        // In full screen there are no traffic lights: the row has more room.
        assert!(compact_bell(
            235.0,
            false,
            &base,
            &HeadWords::reckoned(&base)
        ));
        assert!(!compact_bell(
            235.0,
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
        assert_eq!(fit_width(200.0, &base), 254.0);
        assert_eq!(fit_width(253.0, &base), 254.0);
        assert_eq!(fit_width(254.0, &large), min_width(&large));
        assert!(fit_width(254.0, &large) > 254.0);
        // Wide enough: as written.
        assert_eq!(fit_width(254.0, &base), 254.0);
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
        // Expanded: in line with the cards' avatars; collapsed: the button centred in the strip.
        assert_eq!(menu_anchor(false, &base), (px(21.0), px(47.0)));
        assert_eq!(menu_anchor(true, &base), (px(10.0), px(47.0)));
        // A larger interface size moves it with the bigger button.
        let large = UiFont {
            size: 26.0,
            ..UiFont::default()
        };
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
        let poller = Poller::start(Client { program }, Duration::from_secs(60));
        poller
            .updates
            .recv_timeout(Duration::from_secs(20))
            .expect("poller result")
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
    fn a_reply_is_read_once_for_each_spell_of_idling_and_never_while_waiting() {
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
        let replies = Replies::start(Client { program });
        let mut listing = Listing::default();
        let idle = |since: f64| Agent {
            state_started: Some(since),
            ..agent("p/a", "idle")
        };
        // Listed with it all along: an agent waiting since before, whose card shows its question.
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
        assert_eq!(shown(&mut listing, 1011.0), "Reply 2. All green.");
        // Only the idle agent was ever asked.
        let calls = std::fs::read_to_string(dir.join("calls")).unwrap();
        assert_eq!(calls.lines().collect::<Vec<_>>(), ["p/a", "p/a"]);
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
        // The material under a selected card, as the user's screenshot shows it.
        let frost = luminance((0x45, 0x49, 0x4a));
        let contrast = |color| (luminance(color) + 0.05) / (frost + 0.05);

        for name in crate::preset::Preset::ALL.map(crate::preset::Preset::name) {
            let given = Rc::new(
                Theme::from_config(&crate::config::Config {
                    theme: Some(name.into()),
                    ..crate::config::Config::default()
                })
                .unwrap(),
            );
            let frosted = column_theme(&given, true);
            let dim = contrast(frosted.fg(|t| t.agents_dim));
            assert!(dim >= 4.5, "{name} agents_dim: {dim:.2}:1 < 4.5:1");
            let dimmer = contrast(frosted.fg(|t| t.agents_dimmer));
            assert!(dimmer >= 3.5, "{name} agents_dimmer: {dimmer:.2}:1 < 3.5:1");

            let full_screen = column_theme(&given, false);
            for key in crate::theme::color_keys() {
                assert_eq!(full_screen.color(key), given.color(key), "{name} {key}");
            }
        }
    }
}
