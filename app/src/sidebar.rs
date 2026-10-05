//! The Agents sidebar: `corral ls` and the agents' Git summaries through the pollers taken from
//! Saddle, ordered and judged by its Agents panel model, shown as native cards (`card.rs` decides
//! what each card says). Clicking a card asks the window to show that agent; the footer can sort,
//! fold and open a shell instead.
use crate::{
    agents::{Panel, Status},
    attention,
    card::{self, Card, GitLine, Line, Pick},
    corral::{Agent, Client, Poller, Role},
    footer_icon::{self, Icon},
    git,
    theme::Theme,
    view::hsla,
    viewer::AgentMetadata,
};
use anyhow::Result;
use gpui::{
    AnyElement, App, ClickEvent, Context, Div, ElementId, EventEmitter, Font, FontWeight, Hsla,
    Render, RenderOnce, SharedString, Window, div, prelude::*, px,
};
use std::{rc::Rc, time::Duration};

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

// Sizes, in points.
const PAD: f32 = 10.0;
const NAME_SIZE: f32 = 13.0;
const DETAIL_SIZE: f32 = 12.0;
const MONO_SIZE: f32 = 11.5;
/// The card's left bar plus its inner padding, and the detail lines' indent under the name.
const CARD_INSET: f32 = 2.0 + 10.0 + 8.0;
const INDENT: f32 = 16.0;

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
        columns: usize,
        now: f64,
    ) -> Vec<Line> {
        card::lines(
            &self.panel,
            self.error.as_deref(),
            selected,
            here,
            columns,
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
    /// The terminal's font, for the panel's technical lines.
    mono: Font,
    listing: Listing,
    poller: Poller,
    git: git::Poller,
    /// The active pane's agent, and every agent open in this window.
    selected: Option<String>,
    here: Vec<String>,
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
            note: None,
        }
    }

    /// New colours or width from Settings, at once.
    pub fn restyle(&mut self, theme: Rc<Theme>, width: f32, cx: &mut Context<Self>) {
        self.theme = theme;
        self.width = width;
        cx.notify();
    }

    /// What the window shows: the active pane's agent and every agent open in it.
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

    /// `Attention · N` in the header: yellow when something needs a person, the unread colour for
    /// replies only, faint at zero, `…` before corral first answers. A click opens the list.
    fn attention_entry(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let theme = &self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let items = self.attention();
        let color = if items.iter().any(attention::Item::needs) {
            fg(|t| t.agents_yellow)
        } else if !items.is_empty() {
            fg(|t| t.unread)
        } else {
            fg(|t| t.agents_dimmer)
        };
        let count = if self.listing.loaded {
            items.len().to_string()
        } else {
            "…".into()
        };
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        div()
            .id("attention")
            .flex()
            .items_baseline()
            .gap(px(4.0))
            .px(px(6.0))
            .py(px(1.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .hover(move |style| style.bg(highlight))
            .text_size(px(DETAIL_SIZE))
            .child(
                div()
                    .text_color(if items.is_empty() {
                        fg(|t| t.agents_dim)
                    } else {
                        color
                    })
                    .child("Attention"),
            )
            .child(div().text_color(fg(|t| t.agents_dim)).child("·"))
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(color)
                    .child(count),
            )
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

    /// Whether the list is folded, and sorted by name, for the View menu's ticks.
    pub fn view_state(&self) -> (bool, bool) {
        (self.listing.panel.folded(), self.listing.panel.by_name)
    }

    pub fn toggle_fold(&mut self, cx: &mut Context<Self>) {
        self.listing.panel.toggle_fold();
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
        // Working agents' dots and spinners turn, and ages tick.
        if changed || self.listing.animating(now) {
            cx.notify();
        }
    }

    fn attach(&mut self, card: &Card, cx: &mut Context<Self>) {
        cx.emit(SidebarEvent::Attach {
            name: card.name.clone(),
            metadata: AgentMetadata {
                cwd: card.cwd.clone(),
                instance: card.instance.clone(),
            },
        });
    }

    /// Monospace cells that fit a detail line.
    fn columns(&self, window: &Window) -> usize {
        let text = window.text_system();
        let cell = text
            .advance(text.resolve_font(&self.mono), px(MONO_SIZE), 'm')
            .map_or(MONO_SIZE * 0.6, |advance| f32::from(advance.width));
        ((self.width - 2.0 * PAD - CARD_INSET - INDENT) / cell).max(8.0) as usize
    }
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.clone();
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let now = now();
        let lines = self.listing.lines(
            self.selected.as_deref(),
            &self.here,
            self.columns(window),
            now,
        );
        let agents = lines
            .iter()
            .filter(|line| matches!(line, Line::Agent(_)))
            .count();
        let effort_column = lines
            .iter()
            .any(|line| matches!(line, Line::Agent(card) if card.effort.is_some()));
        let compact = self.width < 320.0;

        let header = div()
            .flex_shrink_0()
            .flex()
            .items_baseline()
            .gap(px(6.0))
            .px(px(PAD + 2.0))
            .pt(px(10.0))
            .pb(px(8.0))
            .border_b_1()
            .border_color(fg(|t| t.agents_rule))
            .child(
                div()
                    .text_size(px(NAME_SIZE + 1.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg(|t| t.agents_text))
                    .child("Agents"),
            )
            .child(
                div()
                    .text_size(px(DETAIL_SIZE))
                    .text_color(fg(|t| t.agents_dim))
                    .child(agents.to_string()),
            )
            .child(div().flex_1())
            .child(self.attention_entry(cx));

        let mut list = div()
            .id("agents")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(PAD))
            .pb(px(8.0));
        let mut first_group = true;
        for line in lines {
            list = list.child(match line {
                Line::Error(text) => div()
                    .mt(px(8.0))
                    .px(px(4.0))
                    .text_size(px(DETAIL_SIZE))
                    .text_color(fg(|t| t.agents_red))
                    .child(text)
                    .into_any_element(),
                Line::Group(title, count) => {
                    let top = if first_group { 10.0 } else { 16.0 };
                    first_group = false;
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(2.0))
                        .pt(px(top))
                        .pb(px(4.0))
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(DETAIL_SIZE))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg(|t| t.agents_accent))
                                .child(title),
                        )
                        .child(div().flex_1().h(px(1.0)).bg(fg(|t| t.agents_faint)))
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(DETAIL_SIZE - 1.0))
                                .text_color(fg(|t| t.agents_dim))
                                .child(count.to_string()),
                        )
                        .into_any_element()
                }
                Line::Agent(card) => {
                    let on_click = {
                        let card = card.clone();
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.attach(&card, cx))
                    };
                    AgentCard {
                        card: *card,
                        theme: theme.clone(),
                        mono: self.mono.clone(),
                        effort_column,
                        compact,
                        now,
                        on_click: Box::new(on_click),
                    }
                    .into_any_element()
                }
            });
        }

        let folded = self.listing.panel.folded();
        let by_name = self.listing.panel.by_name;
        let footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(2.0))
            .px(px(PAD - 2.0))
            .py(px(5.0))
            .border_t_1()
            .border_color(fg(|t| t.agents_rule))
            .child(
                chip(
                    &theme,
                    "sort",
                    Icon::Sort,
                    if by_name {
                        "Sort by name"
                    } else {
                        "Sort by status"
                    },
                    false,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.listing.panel.by_name = !this.listing.panel.by_name;
                    cx.notify();
                })),
            )
            .child(
                chip(&theme, "fold", Icon::Fold, "Fold all", folded).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.listing.panel.toggle_fold();
                        cx.notify();
                    },
                )),
            )
            .child(div().flex_1())
            .child(
                chip(&theme, "new-agent", Icon::NewAgent, "New agent", false).on_click(
                    cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::NewAgent)),
                ),
            )
            .child(
                chip(&theme, "new-shell", Icon::NewShell, "New shell", false).on_click(
                    cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::NewShell)),
                ),
            )
            .child(if self.selected.is_some() {
                chip(&theme, "stop", Icon::Stop, "Stop", false)
                    .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(SidebarEvent::Stop)))
            } else {
                // Stop acts on the active pane's agent; there is none.
                chip(&theme, "stop", Icon::Stop, "Stop", false)
                    .opacity(0.4)
                    .cursor_default()
            });
        let note = self.note.as_ref().map(|(text, problem)| {
            div()
                .flex_shrink_0()
                .px(px(PAD + 2.0))
                .py(px(5.0))
                .border_t_1()
                .border_color(fg(|t| t.agents_rule))
                .text_size(px(DETAIL_SIZE))
                .text_color(if *problem {
                    fg(|t| t.agents_red)
                } else {
                    fg(|t| t.muted)
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
            .border_r_1()
            .border_color(fg(|t| t.agents_rule))
            .child(header)
            .child(list)
            .children(note)
            .child(footer)
    }
}

/// A footer control: an icon, with what it does shown on hover. `on` marks a toggle that is in
/// effect.
fn chip(
    theme: &Theme,
    id: &'static str,
    icon: Icon,
    tip: &'static str,
    on: bool,
) -> gpui::Stateful<Div> {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);
    let tip = Tip {
        text: tip,
        color: fg(|t| t.agents_text),
        background: hsla(theme.bg(|t| t.agents_bg), 1.0),
        border: fg(|t| t.agents_rule),
    };
    let mut chip = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .p(px(5.0))
        .rounded(px(5.0))
        .cursor_pointer()
        .hover(move |style| style.bg(selected))
        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        .child(footer_icon::icon(icon, fg(|t| t.agents_text)));
    if on {
        chip = chip.bg(selected);
    }
    chip
}

/// A footer control's hover text.
#[derive(Clone)]
struct Tip {
    text: &'static str,
    color: Hsla,
    background: Hsla,
    border: Hsla,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(7.0))
            .py(px(3.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(self.border)
            .bg(self.background)
            .shadow_md()
            .text_size(px(DETAIL_SIZE))
            .text_color(self.color)
            .child(self.text)
    }
}

type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One agent: a first line that always shows, and the details when expanded. Its own element, so
/// the list never assumes a height.
#[derive(IntoElement)]
struct AgentCard {
    card: Card,
    theme: Rc<Theme>,
    mono: Font,
    effort_column: bool,
    compact: bool,
    now: f64,
    on_click: OnClick,
}

impl RenderOnce for AgentCard {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let theme = &self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let card = &self.card;
        let exited = card.status == Status::Exited;
        let state_color = fg(card.look.color);
        let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);

        // First line: dot, name, program, effort, state and age, in columns shared by all cards.
        let mut first = div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .text_size(px(NAME_SIZE))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(INDENT - 6.0))
                    .text_color(state_color)
                    .child(card.look.dot),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg(if exited {
                        |t| t.agents_faint
                    } else {
                        |t| t.agents_text
                    }))
                    .child(card.short.clone()),
            );
        let brand_width = if self.compact { 18.0 } else { 60.0 };
        let brand = match &card.brand {
            Some(brand) => {
                let text = match (brand.mark, self.compact) {
                    (Some(mark), true) => mark.to_owned(),
                    (Some(mark), false) => format!("{mark} {}", brand.kind),
                    (None, _) => brand.kind.clone(),
                };
                div()
                    .text_color(fg(brand.color))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(text)
            }
            None => div(),
        };
        first = first.child(
            brand
                .flex_shrink_0()
                .w(px(brand_width))
                .text_size(px(DETAIL_SIZE))
                .overflow_hidden()
                .whitespace_nowrap(),
        );
        if self.effort_column {
            first = first.child(effort_icon(theme, card.effort));
        }
        let mut state = div()
            .flex_shrink_0()
            .w(px(if self.compact { 18.0 } else { 66.0 }))
            .flex()
            .gap(px(3.0))
            .text_size(px(DETAIL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .whitespace_nowrap()
            .overflow_hidden();
        if card.status == Status::Working {
            state = state.child(
                div()
                    .text_color(fg(|t| t.agents_purple))
                    .child(card::spinner(self.now)),
            );
        }
        if !self.compact {
            state = state.child(div().text_color(state_color).child(card.look.label));
        }
        first = first.child(state);
        let (mark, mark_color): (&str, Pick) = if card.here {
            ("⦿ ", |t| t.agents_green)
        } else if card.unread {
            ("• ", |t| t.unread)
        } else {
            ("", |t| t.agents_text)
        };
        first = first.child(
            div()
                .flex_shrink_0()
                .w(px(42.0))
                .flex()
                .justify_end()
                .font(self.mono.clone())
                .text_size(px(MONO_SIZE))
                .child(div().text_color(fg(mark_color)).child(mark))
                .child(
                    div()
                        .text_color(fg(card.time_color))
                        .child(card.time.clone()),
                ),
        );

        let mut body = div().flex().flex_col().gap(px(2.0)).w_full().child(first);
        if card.expanded {
            body = body.children(details(&self.theme, &self.mono, card, state_color));
        }

        div()
            .id(ElementId::Name(SharedString::from(card.name.clone())))
            .flex()
            .w_full()
            .pl(px(10.0))
            .pr(px(8.0))
            .py(px(if card.expanded { 6.0 } else { 3.0 }))
            .rounded(px(6.0))
            .border_l_2()
            .border_color(fg(if card.selected {
                |t| t.agents_accent
            } else {
                |t| t.agents_faint
            }))
            .when(card.selected, |card| card.bg(selected))
            .hover(move |style| style.bg(selected.opacity(0.6)))
            .cursor_pointer()
            .on_click(self.on_click)
            .child(body)
    }
}

/// The expanded lines: title, activity, Git, directory, then identity and connections.
fn details(theme: &Theme, mono: &Font, card: &Card, state_color: Hsla) -> Vec<AnyElement> {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let line = || {
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(INDENT))
            .min_w(px(0.0))
            .whitespace_nowrap()
            .overflow_hidden()
    };
    let mono_line = || line().font(mono.clone()).text_size(px(MONO_SIZE));
    let mut lines = Vec::new();
    if let Some(title) = &card.title {
        lines.push(
            line()
                .text_size(px(DETAIL_SIZE))
                .text_color(fg(|t| t.agents_text))
                .child(div().overflow_hidden().text_ellipsis().child(title.clone()))
                .into_any_element(),
        );
    }
    for activity in &card.activity {
        let mut row = line()
            .text_size(px(DETAIL_SIZE))
            .text_color(state_color)
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(activity.label),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(activity.text.clone()),
            );
        if let Some(duration) = &activity.duration {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .text_color(fg(|t| t.agents_dim))
                    .child(format!("· {duration}")),
            );
        }
        lines.push(row.into_any_element());
    }
    if let Some(git) = &card.git {
        let changes = || changes(theme, git);
        let mut left = mono_line().child(git_left(theme, git));
        match git {
            GitLine::Known {
                changes_below: false,
                ..
            } => {
                left = left.child(div().flex_1()).child(changes());
                lines.push(left.into_any_element());
            }
            GitLine::Known { .. } => {
                lines.push(left.into_any_element());
                lines.push(
                    mono_line()
                        .justify_end()
                        .child(changes())
                        .into_any_element(),
                );
            }
            _ => lines.push(left.into_any_element()),
        }
    }
    lines.push(
        mono_line()
            .text_color(fg(|t| t.agents_dim))
            .child(card.path.clone())
            .into_any_element(),
    );
    let meta: Pick = if card.selected {
        |t| t.agents_text
    } else {
        |t| t.agents_dimmer
    };
    let instance: String = card
        .instance
        .as_deref()
        .unwrap_or("—")
        .chars()
        .take(6)
        .collect();
    lines.push(
        mono_line()
            .gap(px(0.0))
            .text_color(fg(meta))
            .child(format!("{instance} · "))
            .child(
                div()
                    .text_color(fg(if card.attached > 0 {
                        |t| t.agents_green
                    } else {
                        meta
                    }))
                    .child(format!("ATT {}", card.attached)),
            )
            .child(format!(" · VIA {}", card.via))
            .into_any_element(),
    );
    lines
}

/// `⎇ branch ↑n base`, with the count yellow when HEAD is ahead of its base.
fn git_left(theme: &Theme, git: &GitLine) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let dim = fg(|t| t.agents_dim);
    let GitLine::Known { head, ahead, .. } = git else {
        return div().text_color(dim).child(git.left_text());
    };
    let mut left = div()
        .flex()
        .gap(px(6.0))
        .min_w(px(0.0))
        .overflow_hidden()
        .child(div().text_color(dim).child("⎇"));
    if !head.is_empty() {
        left = left.child(
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .text_color(fg(|t| t.agents_branch))
                .child(head.clone()),
        );
    }
    let (count, color): (String, Pick) = match ahead {
        Some((n, _)) if *n > 0 => (format!("↑{n}"), |t| t.agents_yellow),
        Some((n, _)) => (format!("↑{n}"), |t| t.agents_faint),
        None => ("↑—".into(), |t| t.agents_faint),
    };
    left = left.child(div().flex_shrink_0().text_color(fg(color)).child(count));
    if let Some((_, base)) = ahead {
        left = left.child(div().flex_shrink_0().text_color(dim).child(base.clone()));
    }
    left
}

/// `+added -deleted [n binary] ?untracked`, green, red and dim.
fn changes(theme: &Theme, git: &GitLine) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let dim = fg(|t| t.agents_dim);
    let GitLine::Known {
        changes, untracked, ..
    } = git
    else {
        return div();
    };
    let mut row = div().flex_shrink_0().flex().gap(px(6.0));
    match changes {
        Some((added, deleted, binary)) => {
            row = row
                .child(
                    div()
                        .text_color(fg(|t| t.agents_green))
                        .child(format!("+{added}")),
                )
                .child(
                    div()
                        .text_color(fg(|t| t.agents_red))
                        .child(format!("-{deleted}")),
                );
            if *binary > 0 {
                row = row.child(div().text_color(dim).child(format!("{binary} binary")));
            }
        }
        None => row = row.child(div().text_color(dim).child("+— -—")),
    }
    let untracked = untracked.map_or("—".into(), |n| n.to_string());
    row.child(div().text_color(dim).child(format!("?{untracked}")))
}

/// Three rising bars, lit by the delegated effort; empty space when there is no effort label.
fn effort_icon(theme: &Theme, effort: Option<crate::corral::Effort>) -> Div {
    let icon = div()
        .flex_shrink_0()
        .w(px(14.0))
        .h(px(10.0))
        .flex()
        .items_end()
        .gap(px(1.5));
    let Some(effort) = effort else {
        return icon;
    };
    let (lit, color) = card::effort(effort);
    let lit_color = hsla(theme.fg(color), 1.0);
    let unlit = hsla(theme.fg(|t| t.agents_faint), 1.0);
    icon.children((0..3).map(|i| {
        div()
            .w(px(3.0))
            .h(px(4.0 + 3.0 * i as f32))
            .rounded(px(1.0))
            .bg(if i < lit { lit_color } else { unlit })
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
        let lines = listing.lines(None, &[], 40, 1000.0);
        // Groups in name order, the ungrouped first; within a group, those needing a person first.
        assert_eq!(
            shown(&lines),
            [
                "# agents/ (1)",
                "solo working",
                "# paddock/ (3)",
                "dev-theme waiting",
                "dev-agents working",
                "main idle",
                "# saddle/ (1)",
                "main exited",
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
        assert_eq!(shown(&listing.lines(None, &[], 40, 1000.0))[1], "a working");
        listing.absorb(Ok(vec![agent("p/a", "idle")]), None, 1001.0);
        assert_eq!(shown(&listing.lines(None, &[], 40, 1001.0))[1], "a idle");
        listing.absorb(Ok(vec![]), None, 1002.0);
        assert!(listing.lines(None, &[], 40, 1002.0).is_empty());
    }

    #[test]
    fn statuses_use_the_agents_panel_colours() {
        let dune = crate::preset::Preset::Dune.theme();
        let expected: [(Status, &str, crate::preset::Color); 8] = [
            (Status::Waiting, "waiting", dune.agents_yellow),
            (Status::Error, "error", dune.agents_red),
            (Status::Stalled, "stalled", dune.agent_stalled),
            (Status::Working, "working", dune.agents_blue),
            (Status::Starting, "starting", dune.agent_starting),
            (Status::Unknown, "unknown", dune.agents_dim),
            (Status::Idle, "idle", dune.agents_green),
            (Status::Exited, "exited", dune.agents_faint),
        ];
        for (status, label, color) in expected {
            let look = card::look(status, 0.0);
            assert_eq!(look.label, label);
            assert_eq!((look.color)(&dune), color, "{status:?}");
        }
        // Errors and stalls come from the panel's judgement, not just corral's state.
        let mut listing = Listing::default();
        let mut broken = agent("p/broken", "idle");
        broken.error = Some("status failed".into());
        let mut quiet = agent("p/quiet", "working");
        quiet.last_output = Some(0.0);
        listing.absorb(Ok(vec![broken, quiet]), None, 1000.0);
        assert_eq!(
            shown(&listing.lines(None, &[], 40, 1000.0)),
            ["# p/ (2)", "broken error", "quiet stalled"]
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
        let lines = shown(&listing.lines(None, &[], 40, 1001.0));
        assert!(lines[0].starts_with("! corral: "), "{lines:?}");
        assert!(lines[0].contains("daemon unreachable"), "{lines:?}");
        assert_eq!(lines[1..], ["# p/ (1)", "a idle"]);

        assert!(
            listing
                .absorb(first_update(garbage), None, 1002.0)
                .is_none()
        );
        let lines = shown(&listing.lines(None, &[], 40, 1002.0));
        assert!(lines[0].contains("invalid JSON"), "{lines:?}");

        let missing = dir.join("no-such-corral").display().to_string();
        assert!(
            listing
                .absorb(first_update(missing), None, 1003.0)
                .is_none()
        );
        assert!(shown(&listing.lines(None, &[], 40, 1003.0))[0].starts_with("! corral: "));

        // The next good listing clears the error.
        listing.absorb(Ok(vec![agent("p/a", "working")]), None, 1004.0);
        assert_eq!(
            shown(&listing.lines(None, &[], 40, 1004.0)),
            ["# p/ (1)", "a working"]
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
        let lines = listing.lines(None, &[], 40, 1000.0);
        assert_eq!(shown(&lines), ["# p/ (1)", "a working"]);
        let Line::Agent(card) = &lines[1] else {
            panic!()
        };
        assert_eq!(card.cwd.as_deref(), Some("/tmp/a"));
        assert_eq!(card.instance.as_deref(), Some("i1"));
        // Before the first Git round the line says it is still loading.
        assert_eq!(card.git, Some(GitLine::Loading));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
