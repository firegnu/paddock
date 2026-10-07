//! The left sidebar's activity grid (DESIGN §13 P5-32): a small rounded panel under the agents'
//! list, one cell a day and a column a week, as many weeks as the sidebar is wide for, each cell
//! darker for more commits that day (`activity.rs`). The first time it shows its cells pop in
//! along the diagonal, today's cell breathes, a cell whose commits rose at a refresh swells once,
//! and a hovered day shows its card; clicking the header folds it to one line. With the system's
//! Reduce Motion on, nothing moves. Colours are the theme's: the accent at five strengths.
use crate::{
    activity::{self, Ask, Counts, Day, Poller},
    fonts::UiFont,
    motion, popover, reduce_motion,
    theme::Theme,
    view::hsla,
};
use gpui::{
    Anchor, Animation, AnimationExt, AnyElement, BoxShadow, ClickEvent, Context, Div, EventEmitter,
    Font, FontWeight, Hsla, IntoElement, Render, Stateful, Window, anchored, deferred, div, point,
    prelude::*, px,
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

/// What the sidebar tells the panel every time it draws.
pub struct Frame {
    /// The sidebar's width, in points.
    pub width: f32,
    /// The column's colours.
    pub theme: Rc<Theme>,
    /// The system's sidebar material shows through the column.
    pub frosted: bool,
    /// The terminal's font, for the numbers.
    pub mono: Font,
}

/// The panel was folded (`true`) or opened: save it with the layout.
pub struct Folded(pub bool);

impl EventEmitter<Folded> for ActivityView {}

// Sizes, in points at the base interface size.
/// Apart from the sidebar's edges, as the cards are.
const MARGIN_X: f32 = 10.0;
const PAD_X: f32 = 12.0;
const PAD_TOP: f32 = 11.0;
const PAD_BOTTOM: f32 = 10.0;
const FOLDED_Y: f32 = 9.0;
const RADIUS: f32 = 12.0;
const ROW_GAP: f32 = 8.0;
const CELL: f32 = 11.0;
const GAP: f32 = 3.0;
/// A cell's corners, as a share of its size.
const CELL_ROUND: f32 = 0.27;
const MONTHS_HEIGHT: f32 = 12.0;
const LABEL_SIZE: f32 = 10.5;
const SUM_SIZE: f32 = 11.5;
const SMALL_SIZE: f32 = 10.0;
const SWATCH: f32 = 9.0;
/// The folded line's week of cells, and the panel width from which it has room.
const MINI: f32 = 7.0;
const MINI_FROM: f32 = 250.0;
/// The room inside the panel from which the header and foot say everything.
const ROOMY: f32 = 230.0;
/// How far above its cell a day's card hangs.
const CARD_LIFT: f32 = 7.0;
/// The accent's strength at each level above none.
const LEVELS: [f32; 4] = [0.24, 0.45, 0.7, 1.0];
/// How often a finished read is looked for.
const TICK: Duration = Duration::from_millis(250);
/// The repositories a day's card names before the rest are put together.
const NAMED: usize = 3;
/// Today's glow without motion, part way through a breath.
const STILL_GLOW: f32 = 0.4;
/// How often today's breath is drawn: it never stops, and a slow glow needs few frames.
const GLOW_FPS: f32 = 15.0;
/// Today's ring: how far from its cell, and how wide.
const HALO_GAP: f32 = 1.0;
const HALO_WIDTH: f32 = 1.25;

/// The theme's colours the panel uses.
#[derive(Clone, Copy)]
struct Colors {
    ground: Hsla,
    edge: Hsla,
    /// A day without commits.
    empty: Hsla,
    accent: Hsla,
    text: Hsla,
    dim: Hsla,
    dimmer: Hsla,
}

impl Colors {
    fn of(theme: &Theme, frosted: bool) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let text = fg(|t| t.agents_text);
        // Over the material, tints of the text as the sidebar's grounds are; on the sidebar's
        // own colour, its rule for an edge.
        let (ground, edge, empty) = if frosted {
            let lit = theme.frost().lit;
            (
                text.opacity(lit * 0.35),
                text.opacity(lit),
                text.opacity(lit * 0.8),
            )
        } else {
            (
                text.opacity(0.025),
                fg(|t| t.agents_rule),
                text.opacity(0.065),
            )
        };
        Self {
            ground,
            edge,
            empty,
            accent: fg(|t| t.agents_accent),
            text,
            dim: fg(|t| t.agents_dim),
            dimmer: fg(|t| t.agents_dimmer),
        }
    }

    /// A cell at `level`, 0 (none) to 4.
    fn level(&self, level: usize) -> Hsla {
        match level {
            0 => self.empty,
            n => self.accent.opacity(LEVELS[n.min(4) - 1]),
        }
    }
}

/// The cells' sizes for this width and interface size.
#[derive(Clone, Copy)]
struct Sizes {
    weeks: usize,
    cell: f32,
    gap: f32,
}

impl Sizes {
    fn step(&self) -> f32 {
        self.cell + self.gap
    }
}

pub struct ActivityView {
    poller: Poller,
    /// The latest reading, once there is one.
    counts: Option<Rc<Counts>>,
    theme: Rc<Theme>,
    colors: Colors,
    frosted: bool,
    mono: Font,
    width: f32,
    weeks: usize,
    /// The agents' directories, from the sidebar's listing.
    cwds: Vec<String>,
    folded: bool,
    hovered: Option<Day>,
    /// When the cells began popping in: the first reading, or opening the panel again.
    shown_at: Option<Instant>,
    /// The days whose commits rose at the latest refresh, and when.
    rose: Option<(Instant, Vec<Day>)>,
}

impl ActivityView {
    /// `width` is the sidebar's; the repositories already listed are read at once.
    pub fn new(theme: Rc<Theme>, mono: Font, width: f32, cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |view, cx| view.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let mut view = Self {
            poller: Poller::start("git".into(), activity::repos_path(), activity::EVERY),
            counts: None,
            colors: Colors::of(&theme, false),
            theme,
            frosted: false,
            mono,
            width,
            weeks: 0,
            cwds: Vec::new(),
            folded: false,
            hovered: None,
            shown_at: None,
            rose: None,
        };
        view.weeks = view.sizes(&UiFont::get(cx)).weeks;
        view.ask();
        view
    }

    /// The agents' directories as the sidebar last listed them: their repositories join the list.
    pub fn watch(&mut self, cwds: Vec<String>) {
        self.cwds = cwds;
        self.ask();
    }

    fn ask(&mut self) {
        self.poller.ask(Ask {
            cwds: self.cwds.clone(),
            weeks: self.weeks,
        });
    }

    /// Folded to one line, as the layout saved it.
    pub fn set_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        if self.folded != folded {
            self.folded = folded;
            cx.notify();
        }
    }

    /// The sidebar's latest: asks for the weeks that fit, redraws only when something here
    /// changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        if !Rc::ptr_eq(&frame.theme, &self.theme) || frame.frosted != self.frosted {
            self.colors = Colors::of(&frame.theme, frame.frosted);
            self.theme = frame.theme;
            self.frosted = frame.frosted;
            changed = true;
        }
        if frame.mono != self.mono {
            self.mono = frame.mono;
            changed = true;
        }
        if frame.width != self.width {
            self.width = frame.width;
            changed = true;
        }
        let weeks = self.sizes(&UiFont::get(cx)).weeks;
        if weeks != self.weeks {
            self.weeks = weeks;
            self.ask();
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Takes a finished reading; draws again only when it differs.
    fn tick(&mut self, cx: &mut Context<Self>) {
        let Some(counts) = self.poller.updates.try_iter().last() else {
            return;
        };
        if self.counts.as_deref() == Some(&counts) {
            return;
        }
        let now = Instant::now();
        match &self.counts {
            // Days already read whose commits rose since swell once.
            Some(old) => {
                let rose: Vec<Day> = counts
                    .days
                    .keys()
                    .copied()
                    .filter(|&day| {
                        day >= old.first
                            && day <= old.today
                            && counts.commits(day) > old.commits(day)
                    })
                    .collect();
                if !rose.is_empty() {
                    self.rose = Some((now, rose));
                }
            }
            None => self.shown_at = Some(now),
        }
        self.counts = Some(Rc::new(counts));
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        self.folded = !self.folded;
        self.hovered = None;
        if !self.folded && self.counts.is_some() {
            // Opened again: the cells pop in again.
            self.shown_at = Some(Instant::now());
        }
        cx.emit(Folded(self.folded));
        cx.notify();
    }

    fn hover(&mut self, day: Day, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(day)
        } else if self.hovered == Some(day) {
            None
        } else {
            return;
        };
        if now != self.hovered {
            self.hovered = now;
            cx.notify();
        }
    }

    /// The room inside the panel.
    fn inner(&self, ui: &UiFont) -> f32 {
        self.width - 2.0 * MARGIN_X - 2.0 * ui.scale(PAD_X) - 2.0
    }

    /// Whether the header and foot have room for all their words, or keep to the numbers.
    fn roomy(&self, ui: &UiFont) -> bool {
        self.inner(ui) >= ui.scale(ROOMY)
    }

    /// As many weeks as fit, the cells grown a little to fill the width exactly.
    fn sizes(&self, ui: &UiFont) -> Sizes {
        let (cell, gap) = (ui.scale(CELL), ui.scale(GAP));
        let inner = self.inner(ui).max(cell);
        let weeks = activity::weeks_for(inner, cell, gap);
        Sizes {
            weeks,
            cell: ((inner + gap) / weeks as f32 - gap).min(cell * 1.25),
            gap,
        }
    }

    /// The header: `ACTIVITY`, then the commits and weeks shown, or why there are none.
    /// `total` is the count shown now, counting up as the cells pop in, and the whole count.
    fn head(
        &self,
        ui: &UiFont,
        total: Option<(u32, u32)>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.colors;
        let summary = match total {
            None => div(),
            Some((_, 0)) => div().text_color(c.dim).child("No activity yet"),
            Some((total, _)) => div()
                .flex()
                .items_baseline()
                .text_color(c.dim)
                .child(
                    div()
                        .font(self.mono.clone())
                        .text_color(c.text)
                        .child(thousands(total)),
                )
                .child(format!(" {}", plural(total, "commit")))
                .when(self.roomy(ui), |line| {
                    line.child(format!(" · {} weeks", self.weeks))
                }),
        };
        header(ui, c)
            .justify_between()
            .gap(ui.px(8.0))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle(cx)))
            .child(summary.text_size(ui.px(SUM_SIZE)).whitespace_nowrap())
    }

    /// Folded: one line, `ACTIVITY · 29 today · 12-day streak`, and the last seven days' cells when
    /// there is room.
    fn folded_line(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.colors;
        let words = match &self.counts {
            None => div(),
            Some(k) if k.total(k.first) == 0 => div().child("· No activity yet"),
            Some(k) => {
                let streak = k.streak();
                div()
                    .flex()
                    .items_baseline()
                    .child("· ")
                    .child(
                        div()
                            .font(self.mono.clone())
                            .text_color(c.text)
                            .child(k.commits(k.today).to_string()),
                    )
                    .child(" today")
                    .when(streak > 0 && self.roomy(ui), |line| {
                        line.child(" · ")
                            .child(
                                div()
                                    .font(self.mono.clone())
                                    .text_color(c.text)
                                    .child(streak.to_string()),
                            )
                            .child("-day streak")
                    })
            }
        };
        let week = self
            .counts
            .as_ref()
            .filter(|_| self.inner(ui) + 2.0 * ui.scale(PAD_X) >= ui.scale(MINI_FROM))
            .map(|k| {
                let first = k.today - 6;
                let busiest = k.busiest(activity::first_day(k.today, self.weeks));
                div()
                    .flex_shrink_0()
                    .self_center()
                    .flex()
                    .gap(ui.px(2.0))
                    .children((first..first + 7).map(|day| {
                        let cell = div()
                            .size(ui.px(MINI))
                            .rounded(ui.px(MINI * CELL_ROUND))
                            .bg(c.level(activity::level(k.commits(day), busiest)));
                        if day == k.today {
                            cell.shadow(vec![ring(c.accent, ui.scale(1.0))])
                        } else {
                            cell
                        }
                    }))
            });
        header(ui, c)
            .gap(ui.px(6.0))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle(cx)))
            .child(
                words
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(ui.px(SUM_SIZE))
                    .text_color(c.dim),
            )
            .children(week)
    }

    /// One day's cell: `col` weeks in, `row` days into its week.
    #[allow(clippy::too_many_arguments)]
    fn cell(
        &self,
        counts: Option<&Counts>,
        day: Day,
        (col, row): (usize, usize),
        sizes: Sizes,
        (today, busiest): (Day, u32),
        (now, still, glowing): (Instant, bool, bool),
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = self.colors;
        let slot = div()
            .id(("activity-day", day as u64))
            .flex_shrink_0()
            .size(px(sizes.cell))
            .relative()
            .flex()
            .items_center()
            .justify_center();
        if day > today {
            return slot.into_any_element();
        }
        let commits = counts.map_or(0, |k| k.commits(day));
        let color = c.level(activity::level(commits, busiest));
        let (mut scale, opacity) = match self.shown_at {
            _ if still => (1.0, 1.0),
            None => motion::cell_in(None),
            Some(at) => {
                let turn = at + motion::CELL_STAGGER * (col + row) as u32;
                motion::cell_in(now.checked_duration_since(turn))
            }
        };
        if !still
            && let Some((at, days)) = &self.rose
            && days.contains(&day)
        {
            scale *= motion::bump(now - *at);
        }
        let size = sizes.cell * scale;
        let hovered = self.hovered == Some(day);
        let mut shadows = Vec::new();
        if hovered {
            shadows.push(ring(c.text.opacity(0.85), ui.scale(1.0)));
        }
        let inner = div()
            .size(px(size))
            .rounded(px(size * CELL_ROUND))
            .bg(Hsla {
                a: color.a * opacity,
                ..color
            })
            .shadow(shadows);
        // Today's ring, a little apart from the cell, and its glow: laid first, under the cell.
        let halo = (day == today && opacity >= 1.0).then(|| {
            let reach = ui.scale(HALO_GAP + HALO_WIDTH);
            let ring = div()
                .absolute()
                .top(px(-reach))
                .left(px(-reach))
                .size(px(sizes.cell + 2.0 * reach))
                .rounded(px(sizes.cell * CELL_ROUND + reach))
                .border(ui.px(HALO_WIDTH));
            let accent = c.accent;
            if glowing {
                let ui = ui.clone();
                ring.with_animation(
                    "activity-today",
                    Animation::new(motion::GLOW).repeat().with_max_fps(GLOW_FPS),
                    move |ring, t| halo(ring, accent, motion::glow(t), &ui),
                )
                .into_any_element()
            } else {
                halo(ring, accent, STILL_GLOW, ui).into_any_element()
            }
        });
        let card = (hovered && counts.is_some()).then(|| {
            div()
                .absolute()
                .top_0()
                .left(px(sizes.cell / 2.0))
                .size(px(0.0))
                .child(deferred(
                    anchored()
                        .anchor(Anchor::BottomCenter)
                        .offset(point(px(0.0), -ui.px(CARD_LIFT)))
                        .snap_to_window_with_margin(px(8.0))
                        .child(self.card(counts.expect("read"), day, ui)),
                ))
        });
        let on_hover =
            cx.listener(move |this, hovered: &bool, _, cx| this.hover(day, *hovered, cx));
        slot.on_hover(on_hover)
            .children(halo)
            .child(inner)
            .children(card)
            .into_any_element()
    }

    /// A day's card: its date and commits, its busiest repositories, and the tasks wrapped up.
    fn card(&self, counts: &Counts, day: Day, ui: &UiFont) -> Div {
        let c = self.colors;
        let commits = counts.commits(day);
        let number = |n: u32| {
            div()
                .font(self.mono.clone())
                .text_color(c.text)
                .child(n.to_string())
        };
        let repos = counts.repos_on(day);
        let repos = activity::grouped(&repos, NAMED);
        let wrapped = counts.wrapped(day);
        let line = || div().flex().items_baseline().text_color(c.dim);
        popover::panel(&self.theme, ui)
            .px(ui.px(9.0))
            .py(ui.px(6.0))
            .gap(ui.px(2.0))
            .text_size(ui.px(SUM_SIZE))
            .child(
                line()
                    .child(div().text_color(c.text).child(activity::label(day)))
                    .child(" · ")
                    .child(number(commits))
                    .child(format!(" {}", plural(commits, "commit"))),
            )
            .when(!repos.is_empty(), |card| {
                card.child(
                    line()
                        .gap(ui.px(8.0))
                        .children(repos.into_iter().map(|(name, n)| {
                            div()
                                .flex()
                                .items_baseline()
                                .gap(ui.px(4.0))
                                .child(name.unwrap_or("others").to_owned())
                                .child(number(n))
                        })),
                )
            })
            .when(wrapped > 0, |card| {
                card.child(
                    line()
                        .gap(ui.px(5.0))
                        .child(
                            div()
                                .size(ui.px(6.0))
                                .rounded_full()
                                .bg(c.accent)
                                .flex_shrink_0(),
                        )
                        .child(number(wrapped))
                        .child(format!(
                            "{} wrapped up",
                            if wrapped == 1 { "task" } else { "tasks" }
                        )),
                )
            })
    }
}

impl Render for ActivityView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        let panel = div()
            .id("activity")
            .flex_shrink_0()
            .mx(px(MARGIN_X))
            .mt(ui.px(4.0))
            .rounded(ui.px(RADIUS))
            .border_1()
            .border_color(c.edge)
            .bg(c.ground)
            .flex()
            .flex_col()
            .px(ui.px(PAD_X));
        if self.folded {
            return panel.py(ui.px(FOLDED_Y)).child(self.folded_line(&ui, cx));
        }
        let still = reduce_motion::on();
        let now = Instant::now();
        let sizes = self.sizes(&ui);
        let today = self
            .counts
            .as_ref()
            .map_or_else(activity::today, |k| k.today);
        let first = activity::first_day(today, sizes.weeks);
        let counts = self.counts.clone();
        let busiest = counts.as_ref().map_or(0, |k| k.busiest(first));
        // The cells pop in, then today's starts breathing; a risen cell swells.
        let popped = self
            .shown_at
            .map(|at| at + motion::CELL_STAGGER * (sizes.weeks + 6) as u32 + motion::CELL_IN);
        let popping = !still && popped.is_some_and(|end| now < end);
        let swelling = !still
            && self
                .rose
                .as_ref()
                .is_some_and(|(at, _)| now < *at + motion::BUMP);
        if !swelling {
            self.rose = None;
        }
        if popping || swelling {
            window.request_animation_frame();
        }
        let glowing = !still && !popping && self.shown_at.is_some();
        // The total counts up as the cells pop in, on their curve.
        let total = counts.as_ref().map(|k| {
            let total = k.total(first);
            let shown = match (self.shown_at, popped) {
                (Some(at), Some(end)) if popping => {
                    let t = (now - at).as_secs_f32() / (end - at).as_secs_f32();
                    let (_, eased) = motion::cell_in(Some(motion::CELL_IN.mul_f32(t)));
                    (total as f32 * eased).round() as u32
                }
                _ => total,
            };
            (shown, total)
        });

        let months = div()
            .relative()
            .h(ui.px(MONTHS_HEIGHT))
            .text_size(ui.px(SMALL_SIZE))
            .text_color(c.dimmer)
            .whitespace_nowrap()
            .children(
                activity::months(first, sizes.weeks)
                    .into_iter()
                    .map(|(col, name)| {
                        div()
                            .absolute()
                            .top_0()
                            .left(px(col as f32 * sizes.step()))
                            .child(name)
                    }),
            );
        let mut grid = div().flex().gap(px(sizes.gap));
        for col in 0..sizes.weeks {
            let mut column = div().flex().flex_col().gap(px(sizes.gap));
            for row in 0..7 {
                let day = first + (col * 7 + row) as i64;
                column = column.child(self.cell(
                    counts.as_deref(),
                    day,
                    (col, row),
                    sizes,
                    (today, busiest),
                    (now, still, glowing),
                    &ui,
                    cx,
                ));
            }
            grid = grid.child(column);
        }
        let streak = counts.as_deref().map_or(0, Counts::streak);
        let roomy = self.roomy(&ui);
        let foot = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(ui.px(8.0))
            .text_size(ui.px(SMALL_SIZE))
            .text_color(c.dimmer)
            .whitespace_nowrap()
            .child(if streak > 0 {
                div()
                    .flex()
                    .items_baseline()
                    .child(
                        div()
                            .font(self.mono.clone())
                            .text_color(c.dim)
                            .child(streak.to_string()),
                    )
                    .child("-day streak")
            } else {
                div().child(if counts.is_some() {
                    "No streak yet"
                } else {
                    ""
                })
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(3.0))
                    .when(roomy, |legend| {
                        legend.child(div().mr(ui.px(2.0)).child("Less"))
                    })
                    .children((0..5).map(|level| {
                        div()
                            .size(ui.px(SWATCH))
                            .rounded(ui.px(SWATCH * CELL_ROUND))
                            .bg(c.level(level))
                    }))
                    .when(roomy, |legend| {
                        legend.child(div().ml(ui.px(2.0)).child("More"))
                    }),
            );
        panel
            .pt(ui.px(PAD_TOP))
            .pb(ui.px(PAD_BOTTOM))
            .gap(ui.px(ROW_GAP))
            .child(self.head(&ui, total, cx))
            .child(months)
            .child(grid)
            .child(foot)
    }
}

/// The clickable header row both shapes start with: `ACTIVITY`, brighter under the mouse.
fn header(ui: &UiFont, c: Colors) -> Stateful<Div> {
    div()
        .id("activity-head")
        .group("activity-head")
        .flex()
        .items_baseline()
        .cursor_pointer()
        .child(
            div()
                .flex_shrink_0()
                .text_size(ui.px(LABEL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.dimmer)
                .group_hover("activity-head", |s| s.text_color(c.dim))
                .child("ACTIVITY"),
        )
}

/// A thin ring around a cell.
fn ring(color: Hsla, width: f32) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(width),
        inset: false,
    }
}

/// Today's ring, `swell` (0 to 1) of the way through a breath: brighter, and its glow wider.
fn halo(ring: Div, accent: Hsla, swell: f32, ui: &UiFont) -> Div {
    ring.border_color(accent.opacity(0.7 + 0.3 * swell))
        .shadow(vec![BoxShadow {
            color: accent.opacity(0.25 + 0.35 * swell),
            offset: point(px(0.0), px(0.0)),
            blur_radius: ui.px(3.0 + 8.0 * swell),
            spread_radius: px(0.0),
            inset: false,
        }])
}

/// `1,234`.
fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `commit` or `commits`, for `n`.
fn plural(n: u32, word: &str) -> String {
    if n == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_written() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1234), "1,234");
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(plural(1, "commit"), "commit");
        assert_eq!(plural(0, "commit"), "commits");
    }
}
