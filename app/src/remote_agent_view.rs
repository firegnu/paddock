//! The New Agent dialog for another machine (DESIGN §13 P5-124): over the dimmed window, `New
//! Agent on <machine>` and three things to fill in, the kind of agent among those that machine's
//! login shell finds, the directory there (`~` its home) and the name, which may stay empty. Its
//! rules are [`crate::remote_start`]. Start (⌘↩) runs `corral start` there in the background: success
//! closes the dialog and the window opens the agent in a pane of that machine; a failure keeps
//! everything typed, with what came back in the dialog. Tab and ⇧Tab go round the three, ← and →
//! change the kind, Esc closes.
use crate::{
    card,
    fonts::UiFont,
    kind_icon, menu,
    new_agent::{Place, Started, Tool},
    new_agent_view::hold_main_window,
    popover,
    remote::Link,
    remote_start::{self, Failure, Field, RecentDirs, Request},
    sunk,
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle,
    Focusable, FontWeight, Hsla, KeyBinding, MouseButton, MouseDownEvent, Render, SharedString,
    Stateful, Subscription, Window, actions, div, point, prelude::*, px,
};
use std::{path::PathBuf, rc::Rc, sync::Arc};

/// The key context of the dialog: ⌘↩ starts, Esc closes, Tab and ⇧Tab go round its fields.
pub const CONTEXT: &str = "PaddockRemoteAgent";
/// The key context of the kinds while they have the keys: ← and → choose another.
const KINDS: &str = "PaddockRemoteAgentKinds";

actions!(
    remote_agent,
    [
        /// The next of the kind, the directory and the name.
        NextField,
        PreviousField,
        NextKind,
        PreviousKind
    ]
);

/// The dialog's keys. An input method composing text gets them first, Return and Tab included.
pub fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("tab", NextField, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousField, Some(CONTEXT)),
        KeyBinding::new("right", NextKind, Some(KINDS)),
        KeyBinding::new("left", PreviousKind, Some(KINDS)),
        KeyBinding::new("cmd-enter", menu::CreateAgent, Some(CONTEXT)),
        KeyBinding::new("escape", menu::Cancel, Some(CONTEXT)),
        KeyBinding::new("cmd-w", menu::CloseWindow, Some(CONTEXT)),
    ]
}

/// Sizes, in points at the base interface size.
const WIDTH: f32 = 480.0;
const MARGIN: f32 = 16.0;
/// The dialog's top edge, as a share of the window's height.
const TOP: f32 = 0.18;
/// The labels' column, and the height of a control's first line, which each label is centred on.
const LABEL: f32 = 76.0;
const LINE: f32 = 30.0;
/// A field's text line, and the room over and under it inside the field's edge.
const TEXT_LINE: f32 = 17.0;
const FIELD_Y: f32 = 5.5;
/// The note under a control: always there, so the dialog keeps its height as notes come and go.
const NOTE: f32 = 16.0;
/// The room a capsule's well keeps around them, and a capsule's height.
const WELL_PAD: f32 = 2.0;
const CAPSULE: f32 = 26.0;
/// Start's width: the same for `Start ⌘↩` and `Starting…`, so Cancel never moves.
const START: f32 = 108.0;
/// The keyboard focus ring around a field or the kinds.
const RING: f32 = 3.0;
const RING_ALPHA: f32 = 0.22;
/// The tallest the start error gets before it scrolls.
const ERROR_MOST: f32 = 140.0;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// What the dialog tells the window.
pub enum RemoteAgentEvent {
    /// corral started it on `machine` (its `name`): open it there.
    Started {
        machine: String,
        started: Started,
        /// Its directory there, `~` that machine's home.
        cwd: String,
        place: Place,
    },
    /// Esc, ⌘W, Cancel or a click on the dimmed window.
    Close,
}

/// The kinds that machine has, as far as is known.
enum Kinds {
    Checking,
    Found(Vec<Tool>),
    /// It could not be asked: why.
    Failed(String),
}

/// One of the three the keyboard goes round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Kinds,
    Dir,
    Name,
}

pub struct RemoteAgentView {
    theme: Rc<Theme>,
    mono: SharedString,
    link: Arc<Link>,
    place: Place,
    kinds: Kinds,
    tool: Option<Tool>,
    kinds_focus: FocusHandle,
    dir: Entity<TextInput>,
    name: Entity<TextInput>,
    recent: RecentDirs,
    recent_path: Option<PathBuf>,
    /// Why the last Start failed, until the directory or the name changes.
    failure: Option<Failure>,
    starting: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<RemoteAgentEvent> for RemoteAgentView {}

impl Focusable for RemoteAgentView {
    /// The directory, so the dialog opens with it taking the keys.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.dir.focus_handle(cx)
    }
}

fn input_colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
    }
}

/// The ring around what has the keys.
fn ring(accent: Hsla, ui: &UiFont) -> BoxShadow {
    BoxShadow {
        color: accent.opacity(RING_ALPHA),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: ui.px(RING),
        inset: false,
    }
}

impl RemoteAgentView {
    /// For machine `link`, opening the agent at `place`: its home as the directory, and the kinds
    /// looked for there at once.
    pub fn new(
        link: Arc<Link>,
        place: Place,
        theme: Rc<Theme>,
        mono: SharedString,
        cx: &mut Context<Self>,
    ) -> Self {
        let colors = input_colors(&theme);
        let dir = cx.new(|cx| TextInput::new("~", "~/…", colors, cx).wrapping(false));
        let name = cx.new(|cx| TextInput::new("", "", colors, cx));
        let subscriptions = vec![
            cx.subscribe(&dir, |this, _, _: &Changed, cx| this.typed(cx)),
            cx.subscribe(&name, |this, _, _: &Changed, cx| this.typed(cx)),
        ];
        let recent_path = remote_start::recent_path();
        let recent = recent_path
            .as_deref()
            .map(RecentDirs::load)
            .unwrap_or_default();
        let mut view = Self {
            theme,
            mono,
            link,
            place,
            kinds: Kinds::Checking,
            tool: None,
            kinds_focus: cx.focus_handle(),
            dir,
            name,
            recent,
            recent_path,
            failure: None,
            starting: false,
            _subscriptions: subscriptions,
        };
        view.look(cx);
        view.follow_name(cx);
        view
    }

    /// The machine it starts on, by its `name`.
    pub fn machine(&self) -> &str {
        &self.link.name
    }

    /// `corral start` is running there.
    pub fn starting(&self) -> bool {
        self.starting
    }

    /// Settings were saved: their colours and the terminal's font.
    pub fn restyle(&mut self, theme: Rc<Theme>, mono: SharedString, cx: &mut Context<Self>) {
        let colors = input_colors(&theme);
        for input in [&self.dir, &self.name] {
            input.update(cx, |input, cx| input.set_colors(colors, cx));
        }
        (self.theme, self.mono) = (theme, mono);
        cx.notify();
    }

    /// Asks that machine's login shell which kinds it finds, in the background.
    fn look(&mut self, cx: &mut Context<Self>) {
        let link = self.link.clone();
        let task = cx.background_spawn(async move { remote_start::look(&link) });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                this.kinds = match found {
                    Ok(kinds) => {
                        this.tool = kinds.first().copied();
                        Kinds::Found(kinds)
                    }
                    Err(why) => Kinds::Failed(why),
                };
                this.follow_name(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn request(&self, cx: &App) -> Option<Request> {
        Some(Request {
            tool: self.tool?,
            dir: self.dir.read(cx).text().to_owned(),
            name: self.name.read(cx).text().to_owned(),
        })
    }

    /// The directory or the name changed: what failed before no longer stands.
    fn typed(&mut self, cx: &mut Context<Self>) {
        self.failure = None;
        self.follow_name(cx);
        cx.notify();
    }

    /// The name used when none is typed, as the empty field shows it: the directory's and the
    /// kind's, or the directory's alone while the kinds are looked for.
    fn default_name(&self, cx: &App) -> Option<String> {
        let dir = self.dir.read(cx).text();
        match (&self.kinds, self.tool) {
            (_, Some(tool)) => Some(remote_start::default_name(dir, tool)),
            (Kinds::Checking, None) => Some(format!("{}/…", remote_start::name_prefix(dir))),
            _ => None,
        }
    }

    fn follow_name(&mut self, cx: &mut Context<Self>) {
        let shown = self.default_name(cx).unwrap_or_default();
        self.name
            .update(cx, |name, cx| name.set_placeholder(shown, cx));
    }

    fn choose(&mut self, tool: Tool, window: &mut Window, cx: &mut Context<Self>) {
        self.tool = Some(tool);
        window.focus(&self.kinds_focus, cx);
        self.follow_name(cx);
        cx.notify();
    }

    /// ← and →: the kind before or after, stopping at either end.
    fn step_kind(&mut self, by: isize, cx: &mut Context<Self>) {
        let (Kinds::Found(kinds), Some(tool)) = (&self.kinds, self.tool) else {
            return;
        };
        let at = kinds.iter().position(|kind| *kind == tool).unwrap_or(0) as isize;
        let next = (at + by).clamp(0, kinds.len() as isize - 1) as usize;
        self.tool = Some(kinds[next]);
        self.follow_name(cx);
        cx.notify();
    }

    /// What Tab goes round: the kinds once there is one to choose, the directory and the name.
    fn stops(&self) -> Vec<Stop> {
        let kinds = matches!(&self.kinds, Kinds::Found(kinds) if !kinds.is_empty());
        let mut stops = vec![Stop::Dir, Stop::Name];
        if kinds {
            stops.insert(0, Stop::Kinds);
        }
        stops
    }

    fn handle(&self, stop: Stop, cx: &App) -> FocusHandle {
        match stop {
            Stop::Kinds => self.kinds_focus.clone(),
            Stop::Dir => self.dir.focus_handle(cx),
            Stop::Name => self.name.focus_handle(cx),
        }
    }

    /// Tab (`by` 1) and ⇧Tab (-1): the next or the one before, round.
    fn step_field(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.stops();
        let at = stops
            .iter()
            .position(|stop| self.handle(*stop, cx).is_focused(window));
        let count = stops.len() as isize;
        let next = match at {
            Some(at) => (at as isize + by).rem_euclid(count) as usize,
            None => 0,
        };
        window.focus(&self.handle(stops[next], cx), cx);
        cx.notify();
    }

    /// Whether Start can go: a kind, a directory taken and a name taken, and not already going.
    fn can_start(&self, cx: &App) -> bool {
        !self.starting
            && self
                .request(cx)
                .is_some_and(|request| request.problem().is_none())
    }

    fn create(&mut self, _: &menu::CreateAgent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.can_start(cx) {
            return;
        }
        let Some(request) = self.request(cx) else {
            return;
        };
        self.starting = true;
        self.failure = None;
        cx.notify();
        let link = self.link.clone();
        let asked = request.clone();
        let task = cx.background_spawn(async move { remote_start::start(&link, &asked) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.starting = false;
                match result {
                    Ok(started) => {
                        this.remember(&request.dir, cx);
                        let home = this.link.snapshot().home.unwrap_or_default();
                        cx.emit(RemoteAgentEvent::Started {
                            machine: this.link.name.clone(),
                            started,
                            cwd: remote_start::expand(&request.dir, &home),
                            place: this.place,
                        });
                    }
                    Err(failure) => this.failure = Some(failure),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Puts `dir` first among this machine's recent directories, kept beside the layout.
    fn remember(&mut self, dir: &str, cx: &mut Context<Self>) {
        self.recent.remember(&self.link.name, dir);
        if let Some(path) = self.recent_path.clone() {
            let recent = self.recent.clone();
            cx.background_spawn(async move {
                let _ = recent.save(&path);
            })
            .detach();
        }
    }

    fn cancel(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(RemoteAgentEvent::Close);
    }

    /// A recent directory clicked: it goes in the field, which takes the keys.
    fn pick_dir(&mut self, dir: String, window: &mut Window, cx: &mut Context<Self>) {
        self.dir.update(cx, |input, cx| input.set_text(dir, cx));
        window.focus(&self.dir.focus_handle(cx), cx);
        self.typed(cx);
    }

    /// The kind's icon `size` points tall, in its own colour where it is a silhouette.
    fn kind_icon(&self, tool: Tool, size: f32, ui: &UiFont) -> AnyElement {
        let color = self.fg(card::brand(tool.kind()).color);
        match kind_icon::of(tool.kind()) {
            Some(icon) => icon.render(ui.px(size), color),
            None => div()
                .size(ui.px(size * 0.6))
                .rounded_full()
                .bg(color)
                .into_any_element(),
        }
    }

    /// A row: its label in the labels' column, centred on the control's first line, and the
    /// control with what goes under it.
    fn row(&self, label: &'static str, control: impl IntoElement, ui: &UiFont) -> Div {
        div()
            .flex()
            .items_start()
            .gap(ui.px(10.0))
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(LABEL))
                    .h(ui.px(LINE))
                    .flex()
                    .items_center()
                    .text_size(ui.px(12.5))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(ui.px(6.0))
                    .child(control),
            )
    }

    /// A line of words in place of a control, its first line where a control's is.
    fn words(&self, words: String, color: Hsla, ui: &UiFont) -> Div {
        div()
            .min_h(ui.px(LINE))
            .py(ui.px((LINE - TEXT_LINE) / 2.0))
            .line_height(ui.px(TEXT_LINE))
            .text_size(ui.px(12.5))
            .text_color(color)
            .whitespace_normal()
            .child(words)
    }

    /// The note under a control, in `color`; its room kept when there is none.
    fn note(&self, words: Option<String>, color: Hsla, ui: &UiFont) -> Div {
        div()
            .min_h(ui.px(NOTE))
            .text_size(ui.px(11.5))
            .line_height(ui.px(NOTE))
            .text_color(color)
            .whitespace_normal()
            .children(words)
    }

    /// The kinds that machine has, as capsules in a well; while they are looked for, or when there
    /// are none, what stands.
    fn kinds(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let label = self.link.label();
        let (text, bright, accent) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.bright),
            self.fg(|t| t.agents_accent),
        );
        let (dim, branch, red) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_branch),
            self.fg(|t| t.agents_red),
        );
        let found = match &self.kinds {
            Kinds::Checking => return div().child(self.words("Checking…".into(), dim, ui)),
            Kinds::Failed(why) => {
                return div().child(self.words(
                    format!("Couldn't look for agents on {label}: {why}"),
                    red,
                    ui,
                ));
            }
            Kinds::Found(kinds) if kinds.is_empty() => {
                return div().child(self.words(
                    format!("No Claude, Codex, pi or omp found on {label}."),
                    branch,
                    ui,
                ));
            }
            Kinds::Found(kinds) => kinds.clone(),
        };
        let ground = popover::ground(&self.theme);
        let well = sunk::well(ground, ground.l >= 0.5, text);
        let mut shadows = well.shadows;
        if self.kinds_focus.is_focused(window) {
            shadows.push(ring(accent, ui));
        }
        let mut capsules = div()
            .id("remote-kinds")
            .key_context(KINDS)
            .track_focus(&self.kinds_focus)
            .on_action(cx.listener(|this, _: &NextKind, _, cx| this.step_kind(1, cx)))
            .on_action(cx.listener(|this, _: &PreviousKind, _, cx| this.step_kind(-1, cx)))
            .flex()
            .gap(ui.px(2.0))
            .p(ui.px(WELL_PAD))
            .rounded_full()
            .bg(well.ground)
            .shadow(shadows)
            .text_size(ui.px(12.5));
        for (index, tool) in found.into_iter().enumerate() {
            let on = self.tool == Some(tool);
            capsules = capsules.child(
                sunk::chosen(
                    div()
                        .id(("remote-kind", index))
                        .h(ui.px(CAPSULE))
                        .px(ui.px(12.0))
                        .flex()
                        .items_center()
                        .gap(ui.px(6.0))
                        .cursor_pointer()
                        .whitespace_nowrap(),
                    on,
                    accent,
                    text,
                    bright,
                )
                .active(|style| style.opacity(0.8))
                .child(self.kind_icon(tool, 14.0, ui))
                .child(tool.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.choose(tool, window, cx)
                })),
            );
        }
        div().flex().child(capsules)
    }

    /// A field in its frame: the ring while it has the keys, red when it is what failed.
    fn field(
        &self,
        input: &Entity<TextInput>,
        bad: bool,
        window: &Window,
        ui: &UiFont,
        cx: &App,
    ) -> Stateful<Div> {
        let (text, rule, accent, red) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_rule),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_red),
        );
        let handle = input.focus_handle(cx);
        let focused = handle.is_focused(window);
        let edge = match (bad, focused) {
            (true, _) => red.opacity(0.7),
            (false, true) => accent,
            (false, false) => rule,
        };
        let id: SharedString = format!("remote-field-{}", input.entity_id()).into();
        div()
            .id(id)
            .min_h(ui.px(LINE))
            .px(ui.px(10.0))
            .py(ui.px(FIELD_Y))
            .flex()
            .items_center()
            .rounded(ui.px(7.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .border_1()
            .border_color(edge)
            .when(focused, |field| {
                field.shadow(vec![ring(if bad { red } else { accent }, ui)])
            })
            .when(!focused && !bad, |field| {
                field.hover(move |style| style.border_color(text.opacity(0.28)))
            })
            .line_height(ui.px(TEXT_LINE))
            .font_family(self.mono.clone())
            .text_size(ui.px(12.5))
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&handle, cx)
            })
            .child(div().flex_1().min_w(px(0.0)).child(input.clone()))
    }

    /// The directory there, what is wrong with it as typed, and the recent ones to click.
    fn dir_control(
        &self,
        problem: Option<String>,
        bad: bool,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let (text, branch, dimmer, accent, red) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_branch),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_red),
        );
        let typed = self.dir.read(cx).text().trim().to_owned();
        let recent = self.recent.of(&self.link.name);
        let chips = (!recent.is_empty()).then(|| {
            let mut row = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(ui.px(5.0))
                .child(
                    div()
                        .mr(ui.px(2.0))
                        .text_size(ui.px(11.0))
                        .text_color(dimmer)
                        .child("Recent"),
                );
            for (index, dir) in recent.iter().enumerate() {
                let on = *dir == typed;
                let picked = dir.clone();
                row = row.child(
                    div()
                        .id(("remote-recent", index))
                        .max_w_full()
                        .h(ui.px(22.0))
                        .px(ui.px(8.0))
                        .flex()
                        .items_center()
                        .rounded(ui.px(6.0))
                        .bg(text.opacity(0.06))
                        .border_1()
                        .border_color(if on {
                            accent.opacity(0.5)
                        } else {
                            text.opacity(0.10)
                        })
                        .font_family(self.mono.clone())
                        .text_size(ui.px(11.5))
                        .text_color(if on { text } else { branch })
                        .cursor_pointer()
                        .hover(move |style| style.bg(text.opacity(0.10)).text_color(text))
                        .active(move |style| style.bg(text.opacity(0.15)))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(dir.clone()),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.pick_dir(picked.clone(), window, cx)
                        })),
                );
            }
            row
        });
        div()
            .flex()
            .flex_col()
            .gap(ui.px(7.0))
            .child(self.field(&self.dir, bad, window, ui, cx))
            .children(problem.map(|problem| self.note(Some(problem), red, ui)))
            .children(chips)
    }

    /// Why Start failed, as it came back, in the dialog, which stays.
    fn failure(&self, failure: &Failure, ui: &UiFont) -> Div {
        let (text, red) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_red));
        div()
            .flex()
            .gap(ui.px(8.0))
            .px(ui.px(10.0))
            .py(ui.px(9.0))
            .rounded(ui.px(8.0))
            .bg(red.opacity(0.08))
            .border_1()
            .border_color(red.opacity(0.28))
            .text_size(ui.px(11.5))
            .line_height(ui.px(17.0))
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::BOLD)
                    .text_color(red)
                    .child("!"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(ui.px(2.0))
                    .child(
                        div()
                            .text_color(red)
                            .child(format!("Couldn't start on {}:", self.link.label())),
                    )
                    .child(
                        div()
                            .id("remote-agent-error")
                            .max_h(ui.px(ERROR_MOST))
                            .overflow_y_scroll()
                            .font_family(self.mono.clone())
                            .text_color(text)
                            .whitespace_normal()
                            .child(failure.text.clone()),
                    ),
            )
    }

    /// Cancel, and Start with its keys; `Starting…` and not to be clicked while corral starts it.
    fn foot(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let (text, accent) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_accent));
        let sunken = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let cancel = div()
            .id("remote-agent-cancel")
            .h(ui.px(LINE))
            .px(ui.px(14.0))
            .flex()
            .items_center()
            .rounded(ui.px(8.0))
            .bg(text.opacity(0.07))
            .border_1()
            .border_color(text.opacity(0.14))
            .text_color(text)
            .cursor_pointer()
            .hover(move |style| style.bg(text.opacity(0.11)))
            .active(move |style| style.bg(text.opacity(0.16)))
            .child("Cancel")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(RemoteAgentEvent::Close)));
        let can = self.can_start(cx);
        let start = div()
            .id("remote-agent-start")
            .w(ui.px(START))
            .h(ui.px(LINE))
            .flex()
            .items_center()
            .justify_center()
            .gap(ui.px(8.0))
            .rounded(ui.px(8.0))
            .bg(accent)
            .border_1()
            .border_color(accent)
            .text_color(sunken)
            .font_weight(FontWeight::SEMIBOLD)
            .whitespace_nowrap()
            .map(|start| {
                if self.starting {
                    start.child("Starting…")
                } else {
                    start.child("Start").child(
                        div()
                            .font_weight(FontWeight::NORMAL)
                            .text_size(ui.px(11.0))
                            .opacity(0.7)
                            .child("⌘↩"),
                    )
                }
            });
        let start = if can {
            start
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.9)))
                .active(move |style| style.bg(accent.opacity(0.78)))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.create(&menu::CreateAgent, window, cx)
                }))
        } else {
            start.opacity(0.45)
        };
        div()
            .flex()
            .justify_end()
            .gap(ui.px(8.0))
            .pt(ui.px(4.0))
            .text_size(ui.px(12.5))
            .child(cancel)
            .child(start)
    }
}

impl Render for RemoteAgentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let viewport = window.viewport_size();
        let (room_x, room_y) = (f32::from(viewport.width), f32::from(viewport.height));
        let margin = ui.scale(MARGIN);
        let width = ui.scale(WIDTH).min(room_x - 2.0 * margin).max(0.0);
        let top = (room_y * TOP).round();
        let tallest = (room_y - top - margin).max(ui.scale(160.0));
        let (bright, dim, dimmer, red, green) = (
            self.fg(|t| t.bright),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_red),
            self.fg(|t| t.agents_green),
        );
        let ink = crate::window::machine_ink(
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_blue),
            self.fg(|t| t.agents_purple),
        );
        let label = self.link.label();

        let header = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(bright)
                    .child("New Agent"),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(12.5))
                    .text_color(dim)
                    .child("on"),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .h(ui.px(20.0))
                    .px(ui.px(8.0))
                    .flex()
                    .items_center()
                    .gap(ui.px(5.0))
                    .rounded_full()
                    .bg(ink.opacity(0.14))
                    .text_size(ui.px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(
                        div()
                            .flex_shrink_0()
                            .size(ui.px(6.0))
                            .rounded_full()
                            .bg(green),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(label.clone()),
                    ),
            );

        let found = matches!(&self.kinds, Kinds::Found(kinds) if !kinds.is_empty());
        let agent = div()
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .child(self.kinds(window, &ui, cx))
            .child(self.note(
                found.then(|| format!("Agents found on {label}.")),
                dimmer,
                &ui,
            ));

        let dir_text = self.dir.read(cx).text().to_owned();
        let dir_problem = remote_start::check_dir(&dir_text).err();
        let failed_on = self.failure.as_ref().and_then(|failure| failure.field);
        let dir_bad = dir_problem.is_some() || failed_on == Some(Field::Dir);
        let directory = self.dir_control(dir_problem, dir_bad, window, &ui, cx);

        let typed_name = self.name.read(cx).text().to_owned();
        let name_problem = (!typed_name.trim().is_empty())
            .then(|| remote_start::check_name(&typed_name).err())
            .flatten();
        let name_note = match (&name_problem, self.default_name(cx)) {
            (Some(problem), _) => (problem.clone(), red),
            (None, Some(name)) if self.tool.is_some() => {
                (format!("Leave empty to use {name}."), dimmer)
            }
            (None, _) => (
                "Leave empty to name it after the directory and the agent.".to_owned(),
                dimmer,
            ),
        };
        let name = div()
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .child(self.field(
                &self.name,
                name_problem.is_some() || failed_on == Some(Field::Name),
                window,
                &ui,
                cx,
            ))
            .child(self.note(Some(name_note.0), name_note.1, &ui));

        let panel = div()
            .id("remote-agent")
            .w(px(width))
            .max_h(px(tallest))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui.px(16.0))
            .px(ui.px(22.0))
            .py(ui.px(20.0))
            .rounded(ui.px(12.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .bg(popover::ground(&self.theme))
            .shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.5),
                offset: point(px(0.0), ui.px(20.0)),
                blur_radius: ui.px(50.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .text_size(ui.px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(header)
            .child(self.row("Agent", agent, &ui))
            .child(self.row("Directory", directory, &ui))
            .child(self.row("Name", name, &ui))
            .children(
                self.failure
                    .as_ref()
                    .map(|failure| self.failure(failure, &ui)),
            )
            .child(self.foot(&ui, cx));

        let panel = hold_main_window(
            div()
                .key_context(CONTEXT)
                .on_action(cx.listener(Self::create))
                .on_action(cx.listener(Self::cancel))
                .on_action(
                    cx.listener(|this, _: &NextField, window, cx| this.step_field(1, window, cx)),
                )
                .on_action(cx.listener(|this, _: &PreviousField, window, cx| {
                    this.step_field(-1, window, cx)
                }))
                // Return goes to an input method composing, else nowhere: not to the window.
                .on_action(|_: &menu::OpenSelected, _, _| {})
                .on_action(
                    cx.listener(|_, _: &menu::CloseWindow, _, cx| cx.emit(RemoteAgentEvent::Close)),
                ),
        )
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(top))
        .px(px(margin))
        .child(panel);
        // Only the dialog takes clicks while it is open; a click on the dimmed window closes it.
        ui.apply(div())
            .id("remote-agent-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::black().opacity(0.45))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.emit(RemoteAgentEvent::Close)),
            )
            .child(panel)
            // It dims the whole window, the Browser's page and all.
            .child(crate::browser::cover())
    }
}
