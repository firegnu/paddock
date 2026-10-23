//! The New Agent dialog for another machine (DESIGN §13 P5-124): over the dimmed window, `New
//! Agent on <machine>` and four things to fill in, the kind of agent among those that machine's
//! login shell finds and the user's own kept for it, the command it runs (the kind's, as edited
//! there, or one the user writes after `+ Custom…`, P5-124c), the directory there (`~` its home)
//! and the name, which may stay empty. Its rules are [`crate::remote_start`]. Start (⌘↩) runs
//! `corral start` there in the background: success keeps the command for that machine, closes the
//! dialog, and the window opens the agent in a pane of that machine; a failure keeps everything
//! typed, with what came back in the dialog. Tab and ⇧Tab go round the fields, ← and → change the
//! kind, ⌫ forgets the user's own one chosen, Esc closes.
use crate::{
    card,
    fonts::UiFont,
    kind_icon, menu,
    new_agent::{Place, Started, Tool},
    new_agent_view::hold_main_window,
    popover,
    remote::Link,
    remote_start::{self, Agent, Commands, Custom, Failure, Field, RecentDirs, Request},
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
/// The key context of the kinds while they have the keys: ← and → choose another, ⌫ forgets the
/// user's own one chosen.
const KINDS: &str = "PaddockRemoteAgentKinds";
/// The key context of `Reset to …` while it has the keys: Return or Space click it.
const RESET: &str = "PaddockRemoteAgentReset";

actions!(
    remote_agent,
    [
        /// The next of the kind, the command, the directory and the name.
        NextField,
        PreviousField,
        NextKind,
        PreviousKind,
        /// Forgets the user's own command chosen among the kinds.
        RemoveCustom,
        /// The command back to the one kept for the kind chosen.
        ResetCommand
    ]
);

/// The dialog's keys. An input method composing text gets them first, Return and Tab included.
pub fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("tab", NextField, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousField, Some(CONTEXT)),
        KeyBinding::new("right", NextKind, Some(KINDS)),
        KeyBinding::new("left", PreviousKind, Some(KINDS)),
        KeyBinding::new("backspace", RemoveCustom, Some(KINDS)),
        KeyBinding::new("delete", RemoveCustom, Some(KINDS)),
        KeyBinding::new("enter", ResetCommand, Some(RESET)),
        KeyBinding::new("space", ResetCommand, Some(RESET)),
        KeyBinding::new("cmd-enter", menu::CreateAgent, Some(CONTEXT)),
        KeyBinding::new("escape", menu::Cancel, Some(CONTEXT)),
        KeyBinding::new("cmd-w", menu::CloseWindow, Some(CONTEXT)),
    ]
}

/// Sizes, in points at the base interface size.
const WIDTH: f32 = 560.0;
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
/// The widest a capsule's name gets before it is cut short.
const CAPSULE_NAME: f32 = 160.0;
/// The `×` on the user's own capsule.
const REMOVE: f32 = 14.0;
/// Start's width: the same for `Start ⌘↩` and `Starting…`, so Cancel never moves.
const START: f32 = 108.0;
/// The keyboard focus ring around a field or the kinds.
const RING: f32 = 3.0;
const RING_ALPHA: f32 = 0.22;
/// The tallest the start error gets before it scrolls.
const ERROR_MOST: f32 = 140.0;
/// How faint what cannot be clicked is.
const OFF: f32 = 0.45;
/// What `Command` says when empty for the user's own.
const CUSTOM_PLACEHOLDER: &str = "amp, or ~/bin/my-agent --flag";

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

/// A capsule among the kinds: one found there, one of the user's own kept for it (by its name),
/// or `+ Custom…`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Choice {
    Kind(Tool),
    Custom(String),
    New,
}

/// One of those the keyboard goes round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Kinds,
    Command,
    Reset,
    Dir,
    Name,
}

pub struct RemoteAgentView {
    theme: Rc<Theme>,
    mono: SharedString,
    link: Arc<Link>,
    place: Place,
    kinds: Kinds,
    /// None until the kinds are found or one is clicked.
    choice: Option<Choice>,
    kinds_focus: FocusHandle,
    command: Entity<TextInput>,
    reset_focus: FocusHandle,
    dir: Entity<TextInput>,
    name: Entity<TextInput>,
    recent: RecentDirs,
    recent_path: Option<PathBuf>,
    commands: Commands,
    commands_path: Option<PathBuf>,
    /// This machine's own commands with a capsule: those not named as a kind found there.
    customs: Vec<Custom>,
    /// What was typed in `Command` for each capsule left, to come back to while the dialog is open.
    drafts: Vec<(Choice, String)>,
    /// Why the last Start failed, until the command, the directory or the name changes.
    failure: Option<Failure>,
    starting: bool,
    _subscriptions: Vec<Subscription>,
    /// How far below its place the panel is drawn, as it comes in; the window says (P5-148a).
    rise: f32,
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
        faded: theme.is_faded(),
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
    /// Drawn `rise` points below its place, as it comes in over the window (P5-148a).
    pub fn set_rise(&mut self, rise: f32) {
        self.rise = rise;
    }

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
        let command = cx.new(|cx| TextInput::new("", "", colors, cx).wrapping(false));
        let dir = cx.new(|cx| TextInput::new("~", "~/…", colors, cx).wrapping(false));
        let name = cx.new(|cx| TextInput::new("", "", colors, cx));
        let subscriptions = vec![
            cx.subscribe(&command, |this, _, _: &Changed, cx| {
                // Typed before the kinds came: the user's own.
                if this.choice.is_none() {
                    this.choice = Some(Choice::New);
                }
                this.typed(cx)
            }),
            cx.subscribe(&dir, |this, _, _: &Changed, cx| this.typed(cx)),
            cx.subscribe(&name, |this, _, _: &Changed, cx| this.typed(cx)),
        ];
        let recent_path = remote_start::recent_path();
        let recent = recent_path
            .as_deref()
            .map(RecentDirs::load)
            .unwrap_or_default();
        let commands_path = remote_start::commands_path();
        let commands = commands_path
            .as_deref()
            .map(Commands::load)
            .unwrap_or_default();
        let customs = commands.shown(&link.name, &[]);
        let mut view = Self {
            theme,
            mono,
            link,
            place,
            kinds: Kinds::Checking,
            choice: None,
            kinds_focus: cx.focus_handle(),
            command,
            reset_focus: cx.focus_handle(),
            dir,
            name,
            recent,
            recent_path,
            commands,
            commands_path,
            customs,
            drafts: Vec::new(),
            failure: None,
            starting: false,
            _subscriptions: subscriptions,
            rise: 0.0,
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
        self.mono = mono;
        self.set_theme(theme, cx);
    }

    /// New colours: the window's, faded or not (P5-162).
    pub fn set_theme(&mut self, theme: Rc<Theme>, cx: &mut Context<Self>) {
        let colors = input_colors(&theme);
        for input in [&self.command, &self.dir, &self.name] {
            input.update(cx, |input, cx| input.set_colors(colors, cx));
        }
        self.theme = theme;
        cx.notify();
    }

    /// Asks that machine's login shell which kinds it finds, in the background; then, if none was
    /// clicked meanwhile, the first capsule is chosen.
    fn look(&mut self, cx: &mut Context<Self>) {
        let link = self.link.clone();
        let task = cx.background_spawn(async move { remote_start::look(&link) });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                this.kinds = match found {
                    Ok(kinds) => Kinds::Found(kinds),
                    Err(why) => Kinds::Failed(why),
                };
                this.customs = this.commands.shown(&this.link.name, this.found());
                // One of the user's own clicked meanwhile that is a kind found now: that kind.
                let hidden = match &this.choice {
                    Some(Choice::Custom(name)) if this.custom(name).is_none() => {
                        remote_start::found_kind(name, this.found())
                    }
                    _ => None,
                };
                if let Some(tool) = hidden {
                    this.choose(Choice::Kind(tool), cx);
                } else if this.choice.is_none() {
                    this.choose(this.capsules()[0].clone(), cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    /// The kinds found there this time; none while looking or when it could not be asked.
    fn found(&self) -> &[Tool] {
        match &self.kinds {
            Kinds::Found(kinds) => kinds,
            _ => &[],
        }
    }

    /// The capsules, in order: the kinds found there, the user's own, `+ Custom…`.
    fn capsules(&self) -> Vec<Choice> {
        self.found()
            .iter()
            .map(|tool| Choice::Kind(*tool))
            .chain(
                self.customs
                    .iter()
                    .map(|custom| Choice::Custom(custom.name.clone())),
            )
            .chain([Choice::New])
            .collect()
    }

    fn custom(&self, name: &str) -> Option<&Custom> {
        self.customs.iter().find(|custom| custom.name == name)
    }

    /// The command kept for a capsule: a kind's as edited on this machine, the user's own as
    /// started; none for `+ Custom…`.
    fn kept_command(&self, choice: &Choice) -> String {
        match choice {
            Choice::Kind(tool) => self.commands.command(&self.link.name, *tool),
            Choice::Custom(name) => self
                .custom(name)
                .map(|custom| custom.command.clone())
                .unwrap_or_default(),
            Choice::New => String::new(),
        }
    }

    fn command_text(&self, cx: &App) -> String {
        self.command.read(cx).text().to_owned()
    }

    fn request(&self, cx: &App) -> Option<Request> {
        let agent = match self.choice.as_ref()? {
            Choice::Kind(tool) => Agent::Kind(*tool),
            Choice::Custom(_) | Choice::New => Agent::Custom,
        };
        Some(Request {
            agent,
            command: self.command_text(cx),
            dir: self.dir.read(cx).text().to_owned(),
            name: self.name.read(cx).text().to_owned(),
        })
    }

    /// The command, the directory or the name changed: what failed before no longer stands.
    fn typed(&mut self, cx: &mut Context<Self>) {
        self.failure = None;
        self.follow_name(cx);
        cx.notify();
    }

    /// The name used when none is typed, or none while it cannot be told (no kind yet, or a
    /// command of the user's own that cannot be read).
    fn default_name(&self, cx: &App) -> Option<String> {
        let agent = self.request(cx)?.agent_name()?;
        Some(remote_start::default_name(self.dir.read(cx).text(), &agent))
    }

    /// The empty name field shows the name it would get, or the directory's part alone.
    fn follow_name(&mut self, cx: &mut Context<Self>) {
        let shown = self.default_name(cx).unwrap_or_else(|| {
            format!("{}/…", remote_start::name_prefix(self.dir.read(cx).text()))
        });
        self.name
            .update(cx, |name, cx| name.set_placeholder(shown, cx));
    }

    /// Another capsule: what was typed for the one left is kept for coming back to, and the
    /// command becomes what was typed for this one before, else what is kept for it.
    fn choose(&mut self, choice: Choice, cx: &mut Context<Self>) {
        if let Some(left) = self.choice.take() {
            let typed = self.command_text(cx);
            self.drafts.retain(|(known, _)| *known != left);
            self.drafts.push((left, typed));
        }
        let text = self
            .drafts
            .iter()
            .find(|(known, _)| *known == choice)
            .map(|(_, typed)| typed.clone())
            .unwrap_or_else(|| self.kept_command(&choice));
        let placeholder = match &choice {
            Choice::Kind(tool) => tool.command().to_owned(),
            Choice::Custom(_) => self.kept_command(&choice),
            Choice::New => CUSTOM_PLACEHOLDER.to_owned(),
        };
        self.command.update(cx, |input, cx| {
            input.set_text(text, cx);
            input.set_placeholder(placeholder, cx);
        });
        self.choice = Some(choice);
        self.typed(cx);
    }

    /// A capsule clicked or stepped to: chosen, the keys staying on the kinds, or on `Command`
    /// for `+ Custom…`, which is to be written.
    fn pick(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let focus = match choice {
            Choice::New => self.command.focus_handle(cx),
            _ => self.kinds_focus.clone(),
        };
        if self.choice.as_ref() != Some(&choice) {
            self.choose(choice, cx);
        }
        window.focus(&focus, cx);
        cx.notify();
    }

    /// ← and →: the capsule before or after, stopping at either end.
    fn step_kind(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.starting {
            return;
        }
        let capsules = self.capsules();
        let next = match self
            .choice
            .as_ref()
            .and_then(|choice| capsules.iter().position(|known| known == choice))
        {
            Some(at) => (at as isize + by).clamp(0, capsules.len() as isize - 1) as usize,
            None => 0,
        };
        self.pick(capsules[next].clone(), window, cx);
    }

    /// Forgets the user's own command `name` on this machine at once; when it was the one chosen,
    /// the first capsule is chosen instead.
    fn remove(&mut self, name: String, cx: &mut Context<Self>) {
        if self.starting {
            return;
        }
        self.commands.forget(&self.link.name, &name);
        self.customs = self.commands.shown(&self.link.name, self.found());
        let gone = Choice::Custom(name);
        self.drafts.retain(|(known, _)| *known != gone);
        if self.choice.as_ref() == Some(&gone) {
            self.choice = None;
            self.choose(self.capsules()[0].clone(), cx);
        }
        self.save_commands(cx);
        cx.notify();
    }

    /// What `Reset to …` puts back and is called by: the command kept for the capsule chosen,
    /// when the one typed says something else and is still that capsule's.
    fn reset_target(&self, cx: &App) -> Option<(String, String)> {
        let typed = self.command_text(cx);
        match self.choice.as_ref()? {
            Choice::Kind(tool) => remote_start::edited(&typed, *tool)
                .then(|| (tool.command().to_owned(), tool.label().to_owned())),
            Choice::Custom(name) => {
                let kept = &self.custom(name)?.command;
                let same = shell_words::split(&typed).ok() == shell_words::split(kept).ok();
                (!same && remote_start::custom_name(&typed).as_ref() == Some(name))
                    .then(|| (kept.clone(), name.clone()))
            }
            Choice::New => None,
        }
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((command, _)) = self.reset_target(cx).filter(|_| !self.starting) else {
            return;
        };
        self.command
            .update(cx, |input, cx| input.set_text(command, cx));
        // The link goes with the edit: the keys go on to the command.
        if self.reset_focus.is_focused(window) {
            window.focus(&self.command.focus_handle(cx), cx);
        }
        self.typed(cx);
    }

    /// What Tab goes round: the kinds, the command, `Reset to …` while it is there, the directory
    /// and the name.
    fn stops(&self, cx: &App) -> Vec<Stop> {
        let mut stops = vec![Stop::Kinds, Stop::Command];
        if !self.starting && self.reset_target(cx).is_some() {
            stops.push(Stop::Reset);
        }
        stops.extend([Stop::Dir, Stop::Name]);
        stops
    }

    fn handle(&self, stop: Stop, cx: &App) -> FocusHandle {
        match stop {
            Stop::Kinds => self.kinds_focus.clone(),
            Stop::Command => self.command.focus_handle(cx),
            Stop::Reset => self.reset_focus.clone(),
            Stop::Dir => self.dir.focus_handle(cx),
            Stop::Name => self.name.focus_handle(cx),
        }
    }

    /// Tab (`by` 1) and ⇧Tab (-1): the next or the one before, round.
    fn step_field(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.stops(cx);
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

    /// Whether Start can go: a kind, a command, a directory and a name taken, and not already
    /// going.
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
                let found = this.found().to_vec();
                if remote_start::keep(
                    &result,
                    &this.link.name,
                    &request,
                    &found,
                    &mut this.recent,
                    &mut this.commands,
                ) {
                    this.save_kept(cx);
                }
                match result {
                    Ok(started) => {
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

    /// What corral started was kept ([`remote_start::keep`]): the recent directories and the
    /// commands written beside the layout.
    fn save_kept(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.recent_path.clone() {
            let recent = self.recent.clone();
            cx.background_spawn(async move {
                let _ = recent.save(&path);
            })
            .detach();
        }
        self.customs = self.commands.shown(&self.link.name, self.found());
        self.save_commands(cx);
    }

    fn save_commands(&self, cx: &mut Context<Self>) {
        if let Some(path) = self.commands_path.clone() {
            let commands = self.commands.clone();
            cx.background_spawn(async move {
                let _ = commands.save(&path);
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

    /// The user's own command's icon: the kind's when it is one (`amp`), else a dashed square in
    /// `color`, in a slot as wide as a kind's icon.
    fn custom_icon(&self, custom: &Custom, size: f32, color: Hsla, ui: &UiFont) -> AnyElement {
        if let Some(tool) = custom.kind() {
            return self.kind_icon(tool, size, ui);
        }
        div()
            .flex_shrink_0()
            .size(ui.px(size))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(ui.px(size - 2.0))
                    .rounded(ui.px(3.0))
                    .border(ui.px(1.5))
                    .border_dashed()
                    .border_color(color),
            )
            .into_any_element()
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

    /// One capsule among the kinds; the user's own with room for its `×`, shown under the pointer
    /// or while the kinds have the keys and it is the one chosen.
    fn capsule(
        &self,
        index: usize,
        choice: Choice,
        kinds_focused: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let (text, bright, accent, dim) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.bright),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_dim),
        );
        let on = self.choice.as_ref() == Some(&choice);
        let words = match choice {
            Choice::New => dim,
            _ => text,
        };
        let group: SharedString = format!("remote-kind-{index}").into();
        let custom = match &choice {
            Choice::Custom(name) => self.custom(name).cloned(),
            _ => None,
        };
        let pill = div()
            .id(("remote-kind", index))
            .group(group.clone())
            .h(ui.px(CAPSULE))
            .pl(ui.px(12.0))
            .pr(ui.px(if custom.is_some() { 6.0 } else { 12.0 }))
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(ui.px(6.0))
            .whitespace_nowrap();
        let mut pill = sunk::chosen(pill, on, accent, words, bright);
        let name = |name: String| {
            div()
                .min_w(px(0.0))
                .max_w(ui.px(CAPSULE_NAME))
                .overflow_hidden()
                .text_ellipsis()
                .child(name)
        };
        pill = match (&choice, &custom) {
            (Choice::Kind(tool), _) => pill
                .child(self.kind_icon(*tool, 14.0, ui))
                .child(tool.label()),
            (Choice::Custom(_), Some(custom)) => {
                let shown = !self.starting && on && kinds_focused;
                let gone = custom.name.clone();
                let remove = div()
                    .id(("remote-kind-remove", index))
                    .flex_shrink_0()
                    .size(ui.px(REMOVE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui.px(4.0))
                    .text_size(ui.px(11.0))
                    .line_height(ui.px(REMOVE))
                    .font_weight(FontWeight::NORMAL)
                    .child("×");
                let remove = if self.starting {
                    remove.invisible()
                } else {
                    remove
                        .bg(text.opacity(0.10))
                        .text_color(dim)
                        .cursor_pointer()
                        .hover(move |style| style.bg(text.opacity(0.18)).text_color(bright))
                        .active(move |style| style.bg(text.opacity(0.26)))
                        .map(|remove| {
                            if shown {
                                // The kinds have the keys: ⌫ is this `×`.
                                remove.bg(accent.opacity(0.28)).text_color(bright)
                            } else {
                                remove
                                    .invisible()
                                    .group_hover(group.clone(), |style| style.visible())
                            }
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.remove(gone.clone(), cx);
                        }))
                };
                let color = if on { bright } else { text };
                pill.child(self.custom_icon(custom, 14.0, color, ui))
                    .child(name(custom.name.clone()))
                    .child(remove)
            }
            _ => pill.child("+ Custom…"),
        };
        if self.starting {
            pill.opacity(OFF).hover(|style| style)
        } else {
            pill.cursor_pointer()
                .active(|style| style.opacity(0.8))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.pick(choice.clone(), window, cx)
                }))
        }
    }

    /// The capsules in a well, wrapping onto more lines when they do not fit; while the kinds are
    /// looked for, `Checking…` where they will be.
    fn kinds(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let (text, accent, dim) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_dim),
        );
        let ground = popover::ground(&self.theme);
        let well = sunk::well(ground, ground.l >= 0.5, text);
        let focused = self.kinds_focus.is_focused(window);
        let mut shadows = well.shadows;
        if focused {
            shadows.push(ring(accent, ui));
        }
        let mut capsules = div()
            .id("remote-kinds")
            .key_context(KINDS)
            .track_focus(&self.kinds_focus)
            .on_action(cx.listener(|this, _: &NextKind, window, cx| this.step_kind(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousKind, window, cx| this.step_kind(-1, window, cx)),
            )
            .on_action(cx.listener(|this, _: &RemoveCustom, _, cx| {
                if let Some(Choice::Custom(name)) = this.choice.clone() {
                    this.remove(name, cx);
                }
            }))
            .flex()
            .flex_wrap()
            .min_w(px(0.0))
            .gap(ui.px(2.0))
            .p(ui.px(WELL_PAD))
            .rounded(ui.px(CAPSULE / 2.0 + WELL_PAD))
            .bg(well.ground)
            .shadow(shadows)
            .text_size(ui.px(12.5));
        if matches!(self.kinds, Kinds::Checking) {
            capsules = capsules.child(
                div()
                    .h(ui.px(CAPSULE))
                    .px(ui.px(12.0))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .text_color(dim)
                    .child("Checking…"),
            );
        }
        for (index, choice) in self.capsules().into_iter().enumerate() {
            capsules = capsules.child(self.capsule(index, choice, focused, ui, cx));
        }
        div().flex().min_w(px(0.0)).child(capsules)
    }

    /// What is said under the kinds: where they were found, or why none are there.
    fn kinds_note(&self, ui: &UiFont) -> Div {
        let label = self.link.label();
        let (dimmer, branch, red) = (
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_branch),
            self.fg(|t| t.agents_red),
        );
        let (words, color) = match &self.kinds {
            Kinds::Checking => (None, dimmer),
            Kinds::Failed(why) => (
                Some(format!("Couldn't look for agents on {label}: {why}")),
                red,
            ),
            Kinds::Found(kinds) if kinds.is_empty() => (
                Some(format!("No Claude, Codex, pi or omp found on {label}.")),
                branch,
            ),
            Kinds::Found(_) if self.customs.is_empty() => {
                (Some(format!("Found on {label}.")), dimmer)
            }
            Kinds::Found(_) => (
                Some(format!("Found on {label}, and the ones you added.")),
                dimmer,
            ),
        };
        self.note(words, color, ui)
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

    /// The command, and under it two lines that keep their room as the capsule changes: that it
    /// runs through the login shell there, then what is wrong with it as typed, that it is edited
    /// (with `Reset to …`), or what it will be kept as.
    fn command_control(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let label = self.link.label();
        let (dimmer, accent, red) = (
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_red),
        );
        let typed = self.command_text(cx);
        // Empty is not wrong, only not yet startable.
        let problem = remote_start::check_command(&typed)
            .err()
            .filter(|_| !typed.trim().is_empty());
        let failed = self
            .failure
            .as_ref()
            .is_some_and(|failure| failure.field == Some(Field::Command));
        let second: Option<AnyElement> = if let Some(problem) = problem.clone() {
            Some(self.note(Some(problem), red, ui).into_any_element())
        } else if let Some((_, called)) = self.reset_target(cx) {
            let focused = self.reset_focus.is_focused(window);
            let link = div()
                .id("remote-agent-reset")
                .track_focus(&self.reset_focus)
                .key_context(RESET)
                .on_action(cx.listener(|this, _: &ResetCommand, window, cx| this.reset(window, cx)))
                .px(ui.px(3.0))
                .mx(ui.px(-3.0))
                .rounded(ui.px(4.0))
                .text_size(ui.px(11.5))
                .line_height(ui.px(NOTE))
                .text_color(accent)
                .whitespace_nowrap()
                .when(focused, |link| link.shadow(vec![ring(accent, ui)]))
                .child(format!("Reset to {called}"));
            let link = if self.starting {
                link.opacity(OFF)
            } else {
                link.cursor_pointer()
                    .hover(|style| style.underline())
                    .active(|style| style.opacity(0.7))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.reset(window, cx)),
                    )
            };
            Some(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(ui.px(6.0))
                    .child(self.note(Some(format!("Edited, kept for {label} ·")), dimmer, ui))
                    .child(link)
                    .into_any_element(),
            )
        } else {
            let kept_as = match &self.choice {
                Some(Choice::New) => Some(remote_start::custom_name(&typed)),
                Some(Choice::Custom(name)) => remote_start::custom_name(&typed)
                    .filter(|called| called != name)
                    .map(Some),
                _ => None,
            };
            kept_as.map(|called| {
                // Named as a kind found there: kept as that kind's command, not a capsule.
                let found = called
                    .as_deref()
                    .and_then(|called| remote_start::found_kind(called, self.found()));
                let words = match (found, called) {
                    (Some(tool), _) => format!(
                        "Once it starts, it is kept as {}'s command for {label}.",
                        tool.label()
                    ),
                    (None, called) => format!(
                        "Once it starts, {} is kept for {label} next to the others.",
                        called.as_deref().unwrap_or("it")
                    ),
                };
                self.note(Some(words), dimmer, ui).into_any_element()
            })
        };
        div()
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .child(self.field(&self.command, problem.is_some() || failed, window, ui, cx))
            .child(
                div()
                    .min_h(ui.px(2.0 * NOTE))
                    .flex()
                    .flex_col()
                    .child(self.note(
                        Some(format!("Runs through your login shell on {label}.")),
                        dimmer,
                        ui,
                    ))
                    .children(second),
            )
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
            start.opacity(OFF)
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

        let agent = div()
            .flex()
            .flex_col()
            .gap(ui.px(6.0))
            .child(self.kinds(window, &ui, cx))
            .child(self.kinds_note(&ui));
        let command = self.command_control(window, &ui, cx);

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
            (None, Some(name)) => (format!("Leave empty to use {name}."), dimmer),
            (None, None) => (
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
                color: gpui::black().opacity(self.theme.shade(0.5)),
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
            .child(self.row("Command", command, &ui))
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
        .pt(px(top + self.rise))
        .px(px(margin))
        .child(panel);
        // Only the dialog takes clicks while it is open; a click on the dimmed window closes it.
        ui.apply(div())
            .id("remote-agent-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::black().opacity(self.theme.shade(0.45)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.emit(RemoteAgentEvent::Close)),
            )
            .child(panel)
            // It dims the whole window, the Browser's page and all.
            .child(crate::browser::cover())
    }
}
