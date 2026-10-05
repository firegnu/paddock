//! The Agents sidebar: `corral ls` and the agents' Git summaries through the pollers taken from
//! Saddle, ordered and judged by its Agents panel model, shown as native two-line cards (`card.rs`
//! decides what each card says). Clicking a card asks the window to show that agent, and clicking
//! the one shown opens its details; the footer can sort, close the details and open a shell.
use crate::{
    agents::{Panel, Status},
    attention,
    card::{self, Card, Click, Face, Ink, Line, Pick, Second, Tone},
    corral::{Agent, Client, Poller, Role},
    fonts::UiFont,
    footer_icon::{self, Icon},
    git,
    theme::Theme,
    view::hsla,
    viewer::AgentMetadata,
};
use anyhow::Result;
use gpui::{
    Animation, AnimationExt, AnyElement, App, BoxShadow, ClickEvent, Context, Div, ElementId,
    EventEmitter, Font, FontFeatures, FontWeight, HighlightStyle, Hsla, Pixels, Render, RenderOnce,
    SharedString, StyledText, TextRun, Window, div, ease_in_out, point, prelude::*, px, relative,
};
use std::{rc::Rc, sync::Arc, time::Duration};

/// What the sidebar asks of the window.
pub enum SidebarEvent {
    /// Show this agent: where it already is, or by the layout rules.
    Attach {
        name: String,
        metadata: AgentMetadata,
    },
    /// Open a shell.
    NewShell,
    /// Open the New Agent window.
    NewAgent,
    /// Stop the active pane's agent, after asking.
    Stop,
    /// Open or close the Attention list.
    Attention,
    /// The agents corral still lists, for panes to let go of one that disappeared.
    Alive(Vec<String>),
}

/// How often the agents' worktrees are summarised, as in Saddle.
const GIT_REFRESH: Duration = Duration::from_secs(5);

// Sizes, in points at the base interface size.
/// The list's side padding, and a card's own.
const PAD: f32 = 10.0;
const CARD_X: f32 = 10.0;
const CARD_Y: f32 = 8.0;
const DOT: f32 = 8.0;
/// The second line and the details start under the name: the dot and the gap after it.
const INDENT: f32 = 16.0;
/// Room kept before the age.
const TIME_PAD: f32 = 4.0;
/// Between a detail's label column and its value.
const LABEL_GAP: f32 = 14.0;
/// The most of a card's width the working tool takes before it is cut.
const TOOL_SHARE: f32 = 0.3;
/// The share of a card's width a name keeps before the effort, then the tool, give way.
const NAME_SHARE: f32 = 0.4;
/// The marks after the name: the gap before each, the unread dot and its hover box, and the
/// open-here icon.
const MARK_GAP: f32 = 5.0;
const UNREAD: f32 = 6.0;
const UNREAD_BOX: f32 = 10.0;
const HERE_BOX: f32 = 12.0;
// The type scale (DESIGN §13).
const TITLE_SIZE: f32 = 15.0;
const NAME_SIZE: f32 = 13.0;
const SECOND_SIZE: f32 = 12.0;
const NOTE_SIZE: f32 = 11.5;
const KIND_SIZE: f32 = 11.0;
const LABEL_SIZE: f32 = 10.5;

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

    fn animating(&self, now: f64) -> bool {
        self.panel
            .agents
            .iter()
            .any(|a| self.panel.status(a, now) == Status::Working)
    }
}

pub struct Sidebar {
    theme: Rc<Theme>,
    width: f32,
    /// The terminal's font, for the instance id in the details.
    mono: Font,
    listing: Listing,
    poller: Poller,
    git: git::Poller,
    /// The active pane's agent.
    selected: Option<String>,
    /// Every agent open in this window's panes.
    here: Vec<String>,
    /// Written `~` in directories.
    home: Option<String>,
    /// The result of the last start or stop, and whether it is a problem.
    note: Option<(String, bool)>,
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
        Self {
            theme,
            width,
            mono,
            listing: Listing::default(),
            poller: Poller::start(Client { program: corral }, refresh),
            git: git::Poller::start("git".into(), GIT_REFRESH),
            selected: None,
            here: Vec::new(),
            home: std::env::var("HOME").ok(),
            note: None,
        }
    }

    /// New colours or width from Settings, at once.
    pub fn restyle(&mut self, theme: Rc<Theme>, width: f32, cx: &mut Context<Self>) {
        self.theme = theme;
        self.width = width;
        cx.notify();
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

    /// A line above the footer about the last start or stop.
    pub fn note(&mut self, text: String, problem: bool, cx: &mut Context<Self>) {
        self.note = Some((text, problem));
        cx.notify();
    }

    /// Lists the agents again now rather than at the next interval.
    pub fn refresh(&self) {
        self.poller.refresh();
    }

    /// The header's bell, always there so the Attention list has a way in: how many rows the list
    /// has, in amber when an agent needs a person, in the accent for new replies only, quiet and
    /// without a number when there is nothing. A click opens the Attention list.
    fn badge(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let (count, urgency) = attention::bell(&self.attention());
        let ui = UiFont::get(cx);
        let color = hsla(
            match urgency {
                attention::Urgency::Needs => self.theme.fg(|t| t.agents_yellow),
                attention::Urgency::Replies => self.theme.fg(|t| t.agents_accent),
                attention::Urgency::Quiet => self.theme.fg(|t| t.agents_dim),
            },
            1.0,
        );
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
            .child(footer_icon::icon(Icon::Bell, color, ui.scale(1.0)));
        if count > 0 {
            pill = pill.bg(color.opacity(0.15)).child(count.to_string());
        }
        div()
            .id("attention")
            .flex_shrink_0()
            .h(ui.px(28.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .hover(move |style| style.opacity(0.85))
            .child(pill)
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Attention)))
    }

    /// The latest `corral ls`: when, and how many agents or why it failed.
    pub fn last_read(&self) -> crate::diagnostics::Last {
        self.listing.last.clone()
    }

    /// The agents as corral last listed them.
    pub fn agents(&self) -> Vec<Agent> {
        self.listing.panel.agents.clone()
    }

    /// What needs looking at now, for the Attention list.
    pub fn attention(&self) -> Vec<attention::Item> {
        attention::items(&self.listing.panel, self.listing.error.as_deref(), now())
    }

    /// The agents' directories, sorted and without repeats.
    pub fn projects(&self) -> Vec<String> {
        self.listing.cwds()
    }

    /// Whether every card is folded to two lines, and sorted by name, for the View menu's ticks.
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
        }
        for batch in self.git.updates.try_iter().collect::<Vec<_>>() {
            changed = true;
            self.listing.absorb_git(batch);
        }
        // Working agents' ages tick; their dots breathe by themselves.
        if changed || self.listing.animating(now) {
            cx.notify();
        }
    }

    /// A click on a card: show the agent, or, when it is already the one shown, open or close its
    /// details.
    fn click(&mut self, card: &Card, cx: &mut Context<Self>) {
        match card::click(card) {
            Click::Open => cx.emit(SidebarEvent::Attach {
                name: card.name.clone(),
                metadata: AgentMetadata {
                    cwd: card.cwd.clone(),
                    instance: card.instance.clone(),
                },
            }),
            Click::Details => {
                self.listing.panel.toggle_details(&card.name);
                cx.notify();
            }
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let theme = self.theme.clone();
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let now = now();
        let mut label_width = 0.0f32;
        let lines = {
            // Measured in the interface font: GPUI's own ellipsis misjudges text that shares a
            // row, so every line of a card is cut here to the room it has, and never wraps.
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
            let card_width = self.width - 2.0 * PAD - 2.0 * CARD_X;
            let gap = ui.scale(INDENT - DOT);
            let room = card_width - ui.scale(INDENT);
            let fits = |line: &str| width(line, SECOND_SIZE, FontWeight::NORMAL, &plain) <= room;
            let mut lines = self.listing.lines(
                self.selected.as_deref(),
                &self.here,
                self.home.as_deref(),
                now,
            );
            for line in &lines {
                let Line::Agent(card) = line else { continue };
                for detail in card.details.iter().filter(|_| card.expanded) {
                    label_width =
                        label_width.max(width(detail.label, NOTE_SIZE, FontWeight::NORMAL, &plain));
                }
            }
            let value_room = room - label_width - ui.scale(LABEL_GAP);
            for line in &mut lines {
                let Line::Agent(card) = line else { continue };
                // The tool takes at most a share of the line, so the name keeps some room.
                if let Some(tool) = &card.tool {
                    card.tool = Some(card::elide(tool, &|tool| {
                        width(tool, NOTE_SIZE, FontWeight::NORMAL, &plain)
                            <= card_width * TOOL_SHARE
                    }));
                }
                // The room the first line leaves the name.
                let name_room = |card: &Card| {
                    let mut room = card_width
                        - ui.scale(DOT)
                        - gap
                        - gap
                        - TIME_PAD
                        - width(&trailer(card).0, NOTE_SIZE, FontWeight::NORMAL, &tabular());
                    if let Some((program, _, _)) = program(card) {
                        room -= gap + width(&program, KIND_SIZE, FontWeight::NORMAL, &plain);
                    }
                    if card.unread {
                        room -= ui.scale(MARK_GAP + UNREAD_BOX);
                    }
                    if card.here {
                        room -= ui.scale(MARK_GAP + HERE_BOX);
                    }
                    room
                };
                // The effort and the tool give way before the name gets short; then the name is
                // cut: the program and the age always show.
                let weight = name_weight(card.selected);
                let floor =
                    width(&card.short, NAME_SIZE, weight, &plain).min(card_width * NAME_SHARE);
                card::yield_to_name(card, &name_room, floor);
                let room = name_room(card);
                card.short = card::elide(&card.short, &|name| {
                    width(name, NAME_SIZE, weight, &plain) <= room
                });
                card.second = match &card.second {
                    Second::Note(text, tone) => Second::Note(card::elide(text, &fits), *tone),
                    Second::Place(place) => Second::Place(place.fit(&fits)),
                };
                for detail in card.details.iter_mut().filter(|_| card.expanded) {
                    let fits = |value: &str| {
                        let used = match detail.face {
                            Face::Mono => measure(value, KIND_SIZE, self.mono.clone()),
                            _ => width(value, NOTE_SIZE, FontWeight::NORMAL, &plain),
                        };
                        used <= value_room
                    };
                    detail.value = match detail.face {
                        Face::Path => card::shorten(&detail.value, &fits),
                        _ => card::elide(&detail.value, &fits),
                    };
                }
            }
            lines
        };
        let agents = lines
            .iter()
            .filter(|line| matches!(line, Line::Agent(_)))
            .count();

        let header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .min_h(ui.px(44.0))
            .pl(px(18.0))
            .pr(px(14.0))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(ui.px(7.0))
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
                                .text_size(ui.px(SECOND_SIZE))
                                .text_color(fg(|t| t.agents_dimmer))
                                .child(agents.to_string()),
                        )
                    }),
            )
            .child(div().flex_1())
            .child(self.badge(cx));

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
                Some(Line::Error(text)) => {
                    quiet(&theme, &ui, "Can't read corral", text, None, None)
                }
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
                ),
            };
            list = list.justify_center().child(quiet);
        }
        for line in lines.into_iter().filter(|_| agents > 0) {
            list = list.child(match line {
                // The list corral last gave stays below this.
                Line::Error(text) => div()
                    .px(px(8.0))
                    .pt(px(6.0))
                    .text_size(ui.px(NOTE_SIZE))
                    .text_color(fg(|t| t.agents_red))
                    .child(text)
                    .into_any_element(),
                Line::Group(title, count) => div()
                    .flex()
                    .items_baseline()
                    .gap(ui.px(6.0))
                    .px(px(8.0))
                    .pt(px(14.0))
                    .pb(px(6.0))
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
                    )
                    .into_any_element(),
                Line::Agent(card) => {
                    let on_click = {
                        let card = card.clone();
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.click(&card, cx))
                    };
                    AgentCard {
                        card: *card,
                        theme: theme.clone(),
                        mono: self.mono.clone(),
                        label_width,
                        on_click: Box::new(on_click),
                    }
                    .into_any_element()
                }
            });
        }

        let by_name = self.listing.panel.by_name;
        let footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(2.0))
            .px(px(PAD))
            .py(px(6.0))
            .child(
                chip(
                    &theme,
                    &ui,
                    "sort",
                    Icon::Sort,
                    if by_name {
                        "Sort by name"
                    } else {
                        "Sort by status"
                    },
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.listing.panel.by_name = !this.listing.panel.by_name;
                    cx.notify();
                })),
            )
            .child(
                chip(&theme, &ui, "fold", Icon::Fold, "Close all details")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_fold(cx))),
            )
            .child(div().flex_1())
            .child(
                chip(&theme, &ui, "new-agent", Icon::NewAgent, "New agent").on_click(
                    cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::NewAgent)),
                ),
            )
            .child(
                chip(&theme, &ui, "new-shell", Icon::NewShell, "New shell").on_click(
                    cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::NewShell)),
                ),
            )
            .child(if self.selected.is_some() {
                chip(&theme, &ui, "stop", Icon::Stop, "Stop")
                    .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Stop)))
            } else {
                // Stop acts on the active pane's agent; there is none.
                chip(&theme, &ui, "stop", Icon::Stop, "Stop")
                    .opacity(0.4)
                    .cursor_default()
            });
        let note = self.note.as_ref().map(|(text, problem)| {
            div()
                .flex_shrink_0()
                .px(px(18.0))
                .pt(px(6.0))
                .text_size(ui.px(NOTE_SIZE))
                .text_color(if *problem {
                    fg(|t| t.agents_red)
                } else {
                    fg(|t| t.agents_dim)
                })
                .child(text.clone())
        });

        div()
            .flex_shrink_0()
            .w(px(self.width))
            .h_full()
            .flex()
            .flex_col()
            .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
            // Closer than GPUI's default, as in the design.
            .line_height(relative(1.3))
            .child(header)
            .child(list)
            .children(note)
            .child(footer)
    }
}

/// The empty list's message: an icon, a title, a sentence and an action, centred and quiet.
fn quiet(
    theme: &Theme,
    ui: &UiFont,
    title: &'static str,
    text: &str,
    icon: Option<Icon>,
    action: Option<gpui::Stateful<Div>>,
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
                .bg(hsla(theme.bg(|t| t.agent_selected), 1.0))
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

/// A footer control: an icon, with what it does shown on hover.
fn chip(
    theme: &Theme,
    ui: &UiFont,
    id: &'static str,
    icon: Icon,
    tip: &'static str,
) -> gpui::Stateful<Div> {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);
    let tip = Tip {
        text: tip,
        size: ui.px(NOTE_SIZE),
        color: fg(|t| t.agents_text),
        background: hsla(theme.bg(|t| t.agents_bg), 1.0),
        border: fg(|t| t.agents_rule),
    };
    div()
        .id(id)
        .size(ui.px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(selected))
        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        .child(footer_icon::icon(icon, fg(|t| t.agents_dim), ui.scale(1.0)))
}

/// A footer control's hover text.
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

type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One agent: the status dot, name, program and age; the second line; the details when open. Its
/// own element, so the list never assumes a height.
#[derive(IntoElement)]
struct AgentCard {
    card: Card,
    theme: Rc<Theme>,
    mono: Font,
    /// The details' label column, as wide as the widest label.
    label_width: f32,
    on_click: OnClick,
}

impl RenderOnce for AgentCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let theme = &self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let card = &self.card;
        let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let tip = |text: &'static str| Tip {
            text,
            size: ui.px(NOTE_SIZE),
            color: fg(|t| t.agents_text),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: fg(|t| t.agents_rule),
        };

        // Every text comes already cut to its room (see `Sidebar::render`).
        let unread = card.unread.then(|| {
            let tip = tip("Finished a turn you haven't seen");
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
            let tip = tip("Open in this window");
            div()
                .id("here")
                .flex_shrink_0()
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .child(footer_icon::icon(
                    Icon::Here,
                    fg(|t| t.agents_dimmer),
                    ui.scale(HERE_BOX / footer_icon::SIZE),
                ))
        });
        let name = div()
            .flex()
            .items_center()
            .min_w(px(0.0))
            .gap(ui.px(MARK_GAP))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(NAME_SIZE))
                    .font_weight(name_weight(card.selected))
                    .text_color(fg(if card.status == Status::Exited {
                        |t| t.agents_dim
                    } else {
                        |t| t.agents_text
                    }))
                    .child(card.short.clone()),
            )
            .children(unread)
            .children(here);
        let program = program(card).map(|(text, split, kind)| {
            let dimmer = fg(|t| t.agents_dimmer);
            let kind = kind.map_or(dimmer, |pick| fg(pick).opacity(0.85));
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(ui.px(KIND_SIZE))
                .child(styled(
                    text,
                    &[(0..split, kind), (split..usize::MAX, dimmer)],
                ))
        });
        let (trailer_text, split) = trailer(card);
        let age = card
            .time_color
            .map_or(fg(|t| t.agents_dimmer), |pick| fg(pick).opacity(0.8));
        let first = div()
            .flex()
            .items_center()
            .gap(ui.px(INDENT - DOT))
            .child(dot(card.look, fg(card.look.color), &ui))
            .child(name)
            .children(program)
            .child(
                div()
                    .flex_shrink_0()
                    .ml_auto()
                    .pl(px(TIME_PAD))
                    .whitespace_nowrap()
                    .text_size(ui.px(NOTE_SIZE))
                    .font_features(tabular())
                    .child(styled(
                        trailer_text,
                        &[
                            (0..split, fg(|t| t.agents_dimmer)),
                            (split..usize::MAX, age),
                        ],
                    )),
            );

        let (text, colors): (String, Vec<(std::ops::Range<usize>, Hsla)>) = match &card.second {
            Second::Note(text, tone) => {
                let color = fg(match tone {
                    Tone::Quiet => |t| t.agents_dim,
                    Tone::Waiting => |t| t.agents_yellow,
                    Tone::Problem => |t| t.agents_red,
                });
                (text.clone(), vec![(0..text.len(), color)])
            }
            Second::Place(place) => {
                let mut text = String::new();
                let mut colors = Vec::new();
                for (piece, ink) in place.spans() {
                    let color = match ink {
                        Ink::Dim => fg(|t| t.agents_dim),
                        Ink::Dimmer => fg(|t| t.agents_dimmer),
                        Ink::Branch => fg(|t| t.agents_branch).opacity(0.8),
                        Ink::Added => fg(|t| t.agents_green).opacity(0.9),
                        Ink::Deleted => fg(|t| t.agents_red).opacity(0.9),
                        Ink::Ahead => fg(|t| t.agents_yellow).opacity(0.9),
                    };
                    colors.push((text.len()..text.len() + piece.len(), color));
                    text += &piece;
                }
                (text, colors)
            }
        };
        let second = div()
            .mt(px(3.0))
            .pl(ui.px(INDENT))
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(ui.px(SECOND_SIZE))
            .child(styled(text, &colors));

        div()
            .id(ElementId::Name(SharedString::from(card.name.clone())))
            .flex()
            .flex_col()
            .w_full()
            .px(px(CARD_X))
            .py(px(CARD_Y))
            .rounded(px(8.0))
            .when(card.selected, |card| card.bg(selected))
            .when(!card.selected, |card| {
                card.hover(move |style| style.bg(selected.opacity(0.5)))
            })
            .cursor_pointer()
            .on_click(self.on_click)
            .child(first)
            .child(second)
            .when(card.expanded, |body| {
                body.child(details(theme, &self.mono, &ui, self.label_width, card))
            })
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

/// The program and the effort after it, `claude · high`, where the program ends, and its colour;
/// `None` when there is neither.
fn program(card: &Card) -> Option<(String, usize, Option<Pick>)> {
    match (&card.brand, &card.effort) {
        (Some(brand), Some(effort)) => Some((
            format!("{} · {effort}", brand.kind),
            brand.kind.len(),
            Some(brand.color),
        )),
        (Some(brand), None) => Some((brand.kind.clone(), brand.kind.len(), Some(brand.color))),
        (None, Some(effort)) => Some((effort.clone(), 0, None)),
        (None, None) => None,
    }
}

/// The tool a working agent uses, then its age, `Bash · 2m`, and where the age starts.
fn trailer(card: &Card) -> (String, usize) {
    match &card.tool {
        Some(tool) => {
            let lead = format!("{tool} · ");
            let at = lead.len();
            (lead + &card.time, at)
        }
        None => (card.time.clone(), 0),
    }
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

/// The status dot; a working agent's breathes, a ring widening as it fades, every 1.6 s.
fn dot(look: card::Look, color: Hsla, ui: &UiFont) -> AnyElement {
    let dot = div()
        .flex_shrink_0()
        .size(ui.px(DOT))
        .rounded_full()
        .bg(color);
    if !look.breathing {
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

/// The open details: labels and values in two columns under a faint rule, each on one line.
fn details(theme: &Theme, mono: &Font, ui: &UiFont, label_width: f32, card: &Card) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    div()
        .mt(px(10.0))
        .mb(px(2.0))
        .ml(ui.px(INDENT))
        .pt(px(10.0))
        .border_t_1()
        .border_color(fg(|t| t.agents_rule))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .text_size(ui.px(NOTE_SIZE))
        .children(card.details.iter().map(|detail| {
            div()
                .flex()
                .gap(ui.px(LABEL_GAP))
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(label_width))
                        .whitespace_nowrap()
                        .text_color(fg(|t| t.agents_dimmer))
                        .child(detail.label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_color(fg(|t| t.agents_branch))
                        .when(detail.face == Face::Mono, |value| {
                            value.font(mono.clone()).text_size(ui.px(KIND_SIZE))
                        })
                        .child(detail.value.clone()),
                )
        }))
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

    fn shown(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| match line {
                Line::Error(text) => format!("! {text}"),
                Line::Group(title, count) => format!("# {title} ({count})"),
                Line::Agent(card) => format!("{} {}", card.short, card.look.label),
            })
            .collect()
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
        // Before the first Git round the second line is only the directory.
        assert_eq!(
            card.second,
            card::Second::Place(card::Place {
                dir: "/tmp/a".into(),
                branch: None,
                changes: None,
                untracked: 0,
                ahead: 0,
            })
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
