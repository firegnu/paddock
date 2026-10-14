//! The New Agent panel (P5-43, in place of P5-37's window): over the dimmed main window, a panel
//! with the presets along its top and the name corral is asked for at their right; under them a
//! large field for what the agent should work on first, which may stay empty; along its foot the
//! project, the kind with its model and effort (one picker for the three), where it opens and the
//! Controller switch, then `</>` for the command and the round start button (⌘↩). Under `</>` the
//! command can still be edited by hand. Starting runs `corral start` in the background: success
//! closes the panel and the main window opens the agent; a failure keeps everything typed, with
//! the reason over the foot. A preset picked stays lit after a change, marked edited, and can be
//! updated to the change.
use crate::{
    card,
    config::Config,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kind_icon,
    layout::Direction,
    menu, models,
    new_agent::{self, Form, Place, Preset, Started, Tool},
    popover::{self, Hang},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, BoxShadow, ClickEvent, Context, Div,
    DragMoveEvent, ElementId, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Hsla,
    MouseButton, MouseDownEvent, PathBuilder, PathPromptOptions, Pixels, Render, SharedString,
    Stateful, Subscription, TextRun, Window, canvas, div, point, prelude::*, px, relative,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::atomic::AtomicBool,
    time::Duration,
};

/// The key context of the New Agent panel: ⌘↩ starts, Esc steps back, ⌘W closes.
pub const CONTEXT: &str = "PaddockNewAgent";
/// The key context of its small fields (the name, a new preset's name): Return finishes.
pub const FIELD: &str = "PaddockNewAgentField";

/// Sizes, in points at the base interface size.
const WIDTH: f32 = 680.0;
/// Room kept from the window's sides and bottom.
const MARGIN: f32 = 16.0;
/// The panel's top edge, as a share of the window's height.
const TOP: f32 = 0.18;
/// The keys' line under the panel.
const HINTS: f32 = 30.0;
const PAD: f32 = 14.0;
const PRESET: f32 = 30.0;
const PRESET_GAP: f32 = 4.0;
const MORE: f32 = 34.0;
const NAMING: f32 = 170.0;
const RENAMING: f32 = 300.0;
const PROMPT: f32 = 96.0;
const PROMPT_MOST: f32 = 280.0;
const CHIP: f32 = 32.0;
const MENU: f32 = 280.0;
const PICKER: f32 = 318.0;

/// The efforts paddock knows, coolest first: one name is as hot for Claude as for Codex.
const EFFORT_RANKS: [&str; 6] = ["low", "medium", "high", "xhigh", "max", "ultra"];
/// Where on the way from cool to hot the colour is the theme's yellow.
const YELLOW_AT: f32 = 0.6;

/// How hot `effort` is, 0 to 1: by its name, so `max` is as hot for Claude as for Codex; a name
/// paddock does not know goes by its place, `index` of `count`.
fn heat(effort: &str, index: usize, count: usize) -> f32 {
    if let Some(rank) = EFFORT_RANKS.iter().position(|name| *name == effort) {
        return rank as f32 / (EFFORT_RANKS.len() - 1) as f32;
    }
    if count < 2 {
        return 0.0;
    }
    index as f32 / (count - 1) as f32
}

/// The stop nearest `x` on a track starting at `left`, `width` wide, with `count` stops along it.
fn stop_at(x: f32, left: f32, width: f32, count: usize) -> usize {
    if count < 2 || width <= 0.0 {
        return 0;
    }
    let along = ((x - left) / width).clamp(0.0, 1.0);
    (along * (count - 1) as f32).round() as usize
}

/// The colour at `heat`: the accent when cool, yellow at [`YELLOW_AT`], red when hottest, mixed
/// in between.
fn warmth(accent: Hsla, yellow: Hsla, red: Hsla, heat: f32) -> Hsla {
    let mix = |from: Hsla, to: Hsla, t: f32| {
        let (a, b) = (gpui::Rgba::from(from), gpui::Rgba::from(to));
        let at = |x: f32, y: f32| x + (y - x) * t;
        Hsla::from(gpui::Rgba {
            r: at(a.r, b.r),
            g: at(a.g, b.g),
            b: at(a.b, b.b),
            a: at(a.a, b.a),
        })
    };
    let heat = heat.clamp(0.0, 1.0);
    if heat <= YELLOW_AT {
        mix(accent, yellow, heat / YELLOW_AT)
    } else {
        mix(yellow, red, (heat - YELLOW_AT) / (1.0 - YELLOW_AT))
    }
}

/// The thumb's way to a new stop, 0 to 1 over the animation: past the stop a little, then back.
fn settle(t: f32) -> f32 {
    // An ease-out that overshoots by about 7%.
    const BACK: f32 = 1.4;
    let u = t.clamp(0.0, 1.0) - 1.0;
    1.0 + (BACK + 1.0) * u.powi(3) + BACK * u.powi(2)
}

/// How long the effort thumb takes to glide to a new stop.
const GLIDE: Duration = Duration::from_millis(280);

/// The effort thumb's latest move: from where (along the track, 0 to 1, and how hot) to where.
/// `moves` counts them, so each starts its own glide.
#[derive(Clone, Copy, PartialEq)]
struct Motion {
    from: (f32, f32),
    to: (f32, f32),
    moves: u64,
}

/// The effort thumb being dragged.
struct EffortDrag;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// What the main window needs to open the panel.
pub struct Seed {
    pub theme: Rc<Theme>,
    pub corral: String,
    /// The terminal's font, for the command.
    pub mono: SharedString,
    /// Directories to choose from: paddock's own and the agents'.
    pub projects: Vec<String>,
    /// The active pane's directory.
    pub project: String,
    /// The agents there are, by full name, so the name made up is a free one.
    pub names: Vec<String>,
    /// Opened from a workspace (P5-93): its name, for the agent to join it, and the directory to
    /// start in whatever was typed before.
    pub workspace: Option<String>,
    pub start_in: Option<String>,
}

pub enum NewAgentEvent {
    /// corral started it: open it where the form said.
    Started {
        started: Started,
        cwd: String,
        place: Place,
        /// The workspace it was opened from, for the agent to join (P5-93).
        workspace: Option<String>,
        /// The first message, to send once it is idle, for a kind that takes none at start (amp).
        first: Option<String>,
    },
    /// Esc, ⌘W or a click on the dimmed window: the panel goes, what was typed stays.
    Close,
}

/// What opens under the panel's parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Menu {
    Project,
    /// The kind.
    Agent,
    /// The kind's model and effort, for a kind that has them (P5-99).
    Model,
    Place,
    /// The presets with no room in the row, from its `+N`.
    Presets,
}

pub struct NewAgentView {
    theme: Rc<Theme>,
    corral: String,
    mono: SharedString,
    /// The agents' projects and paddock's own, then the recently used ones.
    projects: Vec<String>,
    recent: Vec<String>,
    recent_path: Option<PathBuf>,
    form: Form,
    presets: Vec<Preset>,
    /// `[agent_models]` in the config, to offer codex's models over when it answers.
    agent_models: models::Configured,
    config_path: PathBuf,
    prompt: Entity<TextInput>,
    command: Entity<TextInput>,
    prefix: Entity<TextInput>,
    name: Entity<TextInput>,
    preset_name: Entity<TextInput>,
    /// What is open under its part.
    open: Option<Menu>,
    /// Where each part was last drawn, for what opens to hang from.
    spots: Rc<RefCell<HashMap<Menu, Bounds<Pixels>>>>,
    /// Where the effort slider's track was last drawn, for the stop under the mouse.
    effort_track: Rc<Cell<Bounds<Pixels>>>,
    /// The effort thumb's latest move, which it glides along.
    effort_motion: Cell<Option<Motion>>,
    renaming: bool,
    naming_preset: bool,
    show_command: bool,
    busy: bool,
    /// How amp's plugin stood when last asked, while amp is chosen (P5-98), and whether it was
    /// asked since the panel opened.
    plugin: Option<String>,
    plugin_asked: bool,
    /// The workspace it was opened from (P5-93).
    workspace: Option<String>,
    error: Option<String>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NewAgentEvent> for NewAgentView {}

impl Focusable for NewAgentView {
    /// The field for the first message, so typing starts there.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.prompt.focus_handle(cx)
    }
}

/// A directory with the home directory written `~`.
fn shown_path(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && path.starts_with(&home) => {
            format!("~{}", &path[home.len()..])
        }
        _ => path.to_owned(),
    }
}

/// A directory's own name, its last part.
fn project_name(path: &str) -> String {
    let path = path.trim_end_matches('/');
    match path.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => shown_path(path),
    }
}

fn place_label(place: Place) -> &'static str {
    match place {
        Place::Current => "This pane",
        Place::Tab => "New tab",
        Place::Split(Direction::Right) => "Split right",
        Place::Split(Direction::Left) => "Split left",
        Place::Split(Direction::Up) => "Split up",
        Place::Split(Direction::Down) => "Split down",
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

/// The main window's commands that would change what lies under the panel or open another
/// panel over it: while the panel has the keys they do nothing.
fn hold_main_window(panel: Div) -> Div {
    macro_rules! hold {
        ($panel:expr, $($action:ident),*) => {
            $panel$(.on_action(|_: &menu::$action, _, _| {}))*
        };
    }
    hold!(
        panel,
        NewTab,
        NewShell,
        SplitRight,
        SplitDown,
        SplitLeft,
        SplitUp,
        ClosePane,
        CloseTab,
        StopAgent,
        ShowAttention,
        Search,
        CommandPalette,
        ZoomPane,
        NextTab,
        PreviousTab,
        Tab1,
        Tab2,
        Tab3,
        Tab4,
        Tab5,
        Tab6,
        Tab7,
        Tab8,
        Tab9,
        SelectNext,
        SelectPrevious
    )
}

impl NewAgentView {
    pub fn new(seed: Seed, place: Place, cx: &mut Context<Self>) -> Self {
        let config_path = crate::config::default_path();
        let config = Config::load(&config_path).unwrap_or_default();
        let presets = new_agent::presets(&config);
        let mut form = Form::new(seed.start_in.unwrap_or(seed.project), place);
        form.set_names(seed.names);
        form.set_lists(models::load(&config.agent_models));
        // Opens set as a regular Claude agent at its deepest model and normal effort.
        form.pick_tool(Tool::Claude);
        form.regular = true;
        let recent_path = new_agent::recent_path();
        let recent = recent_path
            .as_deref()
            .map(new_agent::load_recent)
            .unwrap_or_default();
        let colors = input_colors(&seed.theme);
        let field = |text: String, placeholder: &str, cx: &mut Context<Self>| {
            let placeholder = placeholder.to_owned();
            cx.new(|cx| TextInput::new(text, placeholder, colors, cx))
        };
        let prompt =
            cx.new(|cx| TextInput::new("", "What should it work on?", colors, cx).wrapping(true));
        let command = cx
            .new(|cx| TextInput::new(form.command.clone(), "command", colors, cx).wrapping(false));
        let prefix = field(form.prefix.clone(), "prefix", cx);
        let name = field(form.name.clone(), "name", cx);
        let preset_name = field(String::new(), "Preset name", cx);
        let subscriptions = vec![
            cx.subscribe(&prompt, |this, input, _: &Changed, cx| {
                this.form.prompt = input.read(cx).text().to_owned();
                this.error = None;
                cx.notify();
            }),
            cx.subscribe(&command, |this, input, _: &Changed, cx| {
                let text = input.read(cx).text().to_owned();
                this.form.set_command(text);
                this.sync_name(cx);
                this.error = None;
                this.check_plugin(cx);
                cx.notify();
            }),
            // Typing a prefix moves an untyped name along; show it.
            cx.subscribe(&prefix, |this, input, _: &Changed, cx| {
                let text = input.read(cx).text().to_owned();
                this.form.type_prefix(text);
                this.sync_name(cx);
                this.error = None;
                cx.notify();
            }),
            cx.subscribe(&name, |this, input, _: &Changed, cx| {
                let text = input.read(cx).text().to_owned();
                this.form.type_name(text);
                this.error = None;
                cx.notify();
            }),
        ];
        let mut projects = seed.projects;
        if !projects.contains(&form.project) {
            projects.insert(0, form.project.clone());
        }
        let mut view = Self {
            theme: seed.theme,
            corral: seed.corral,
            mono: seed.mono,
            projects,
            recent,
            recent_path,
            form,
            presets,
            agent_models: config.agent_models,
            config_path,
            prompt,
            command,
            prefix,
            name,
            preset_name,
            open: None,
            spots: Rc::default(),
            effort_track: Rc::default(),
            effort_motion: Cell::new(None),
            renaming: false,
            naming_preset: false,
            show_command: false,
            busy: false,
            plugin: None,
            plugin_asked: false,
            workspace: seed.workspace,
            error: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        view.ask_codex(cx);
        view
    }

    /// Asks codex for its models in the background when a month has passed since it last was,
    /// and offers them once it answers.
    fn ask_codex(&mut self, cx: &mut Context<Self>) {
        let task = cx.background_spawn(async { models::ask_codex(false) });
        cx.spawn(async move |this, cx| {
            if let Some(cache) = task.await {
                let _ = this.update(cx, |this, cx| {
                    let lists = models::Lists::new(&this.agent_models, Some(&cache));
                    this.form.set_lists(lists);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Opened again: the agents and their projects as they are now, and from `+` or the split
    /// button, open there. What was typed and chosen stays, but for the directory when a
    /// workspace says where to start.
    pub fn reopen(&mut self, seed: Seed, place: Place, cx: &mut Context<Self>) {
        self.form.set_names(seed.names);
        if let Some(project) = seed.start_in {
            self.set_project(project, cx);
        }
        self.workspace = seed.workspace;
        self.projects = seed.projects;
        if !self.projects.contains(&self.form.project) {
            self.projects.insert(0, self.form.project.clone());
        }
        if place != Place::Current {
            self.form.place = place;
        }
        self.sync_name(cx);
        // The config may have been edited by hand, and codex asked from Settings, meanwhile.
        self.agent_models = Config::load(&self.config_path)
            .unwrap_or_default()
            .agent_models;
        self.form.set_lists(models::load(&self.agent_models));
        self.ask_codex(cx);
        // The plugin may have been installed in Settings meanwhile.
        self.plugin = None;
        self.plugin_asked = false;
        self.check_plugin(cx);
        cx.notify();
    }

    /// `corral start` is running for Start.
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// Settings were saved: their colours, fonts and presets.
    pub fn restyle(&mut self, config: &Config, cx: &mut Context<Self>) {
        if let Ok(theme) = Theme::from_config(config) {
            self.theme = Rc::new(theme);
            let colors = input_colors(&self.theme);
            for input in [
                &self.prompt,
                &self.command,
                &self.prefix,
                &self.name,
                &self.preset_name,
            ] {
                input.update(cx, |input, cx| input.set_colors(colors, cx));
            }
        }
        self.mono = config.font.clone().into();
        self.presets = new_agent::presets(config);
        self.agent_models = config.agent_models.clone();
        self.form.set_lists(models::load(&self.agent_models));
        cx.notify();
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn bg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.bg(pick), 1.0)
    }

    /// The command and name fields show what the form now says.
    fn sync_command(&mut self, cx: &mut Context<Self>) {
        let command = self.form.command.clone();
        if self.command.read(cx).text() != command {
            self.command
                .update(cx, |input, cx| input.set_text(command, cx));
        }
        self.sync_name(cx);
    }

    fn sync_name(&mut self, cx: &mut Context<Self>) {
        for (input, text) in [
            (&self.prefix, self.form.prefix.clone()),
            (&self.name, self.form.name.clone()),
        ] {
            if input.read(cx).text() != text {
                input.update(cx, |input, cx| input.set_text(text, cx));
            }
        }
    }

    /// After a choice: the form changed, and the list it came from closes.
    fn chose(&mut self, cx: &mut Context<Self>) {
        self.open = None;
        self.changed(cx);
    }

    /// The form changed; the command follows. The picker stays open for the next choice.
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.sync_command(cx);
        self.check_plugin(cx);
        cx.notify();
    }

    /// With amp chosen, asks once how its plugin stands, to warn before starting an amp that would
    /// stay Starting (P5-98).
    fn check_plugin(&mut self, cx: &mut Context<Self>) {
        if self.form.tool != Tool::Amp || self.plugin_asked {
            return;
        }
        self.plugin_asked = true;
        let corral = self.corral.clone();
        let task = cx.background_spawn(async move {
            crate::integrations::plugin_status(&corral, Tool::Amp.kind(), &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let plugin = task.await;
            let _ = this.update(cx, |this, cx| {
                this.plugin = plugin;
                cx.notify();
            });
        })
        .detach();
    }

    fn set_project(&mut self, project: String, cx: &mut Context<Self>) {
        self.form.set_project(project);
        self.chose(cx);
    }

    fn pick(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(preset) = self.presets.get(index).cloned() {
            self.form.pick(&preset);
            self.chose(cx);
        }
    }

    /// Keeps the presets in the config file; a failure says why over the foot.
    fn save_presets(&mut self, presets: Vec<Preset>, cx: &mut Context<Self>) -> bool {
        let saved = match new_agent::save_presets(&self.config_path, &presets) {
            Ok(_) => {
                self.presets = presets;
                true
            }
            Err(error) => {
                self.error = Some(format!("Presets not saved: {error:#}"));
                false
            }
        };
        cx.notify();
        saved
    }

    fn delete_preset(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut presets = self.presets.clone();
        if index < presets.len() {
            presets.remove(index);
            self.save_presets(presets, cx);
        }
    }

    /// Update preset: the picked preset becomes what the form is set as now.
    fn update_preset(&mut self, cx: &mut Context<Self>) {
        if let Some(presets) = self.form.update_picked(&self.presets) {
            self.save_presets(presets, cx);
        }
    }

    /// The current kind, model, effort and role as a preset, under the name typed (replacing one
    /// of that name), or a name made from them; it is the one picked from then on.
    fn add_preset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.preset_name.read(cx).text().trim().to_owned();
        let name = if typed.is_empty() {
            self.summary()
        } else {
            typed
        };
        let preset = self.form.preset(name);
        let mut presets = self.presets.clone();
        match presets.iter_mut().find(|p| p.name == preset.name) {
            Some(known) => *known = preset.clone(),
            None => presets.push(preset.clone()),
        }
        if self.save_presets(presets, cx) {
            self.form.pick(&preset);
        }
        self.naming_preset = false;
        self.preset_name
            .update(cx, |input, cx| input.set_text("", cx));
        window.focus(&self.prompt.focus_handle(cx), cx);
    }

    /// The kind, and its model and effort when it has them: `Claude opus[1m] / high`.
    fn summary(&self) -> String {
        let tool = self.form.tool;
        match (&self.form.model, &self.form.effort) {
            (Some(model), Some(effort)) if tool.has_models() => {
                format!("{} {model} / {effort}", tool.label())
            }
            (Some(model), None) if tool.has_models() => format!("{} {model}", tool.label()),
            _ => tool.label().to_owned(),
        }
    }

    /// The system's folder chooser.
    fn browse(&mut self, cx: &mut Context<Self>) {
        self.open = None;
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update(cx, |this, cx| {
                    this.set_project(path.display().to_string(), cx)
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn create(&mut self, _: &menu::CreateAgent, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.open = None;
        let args = match self.form.args() {
            Ok(args) => args,
            Err(error) => {
                match self.form.problem() {
                    Some(("command" | "prompt", _)) => self.show_command = true,
                    Some(("prefix" | "name", _)) => self.renaming = true,
                    _ => {}
                }
                self.error = Some(format!("{error}"));
                cx.notify();
                return;
            }
        };
        self.busy = true;
        self.error = None;
        cx.notify();
        // As it is now: the panel may be opened again, from elsewhere, while corral starts it.
        let workspace = self.workspace.clone();
        let corral = self.corral.clone();
        let first = self.form.first_message();
        let task = cx.background_spawn(async move { new_agent::start(&corral, &args) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.busy = false;
                match result {
                    Ok(started) => {
                        this.remember_project(cx);
                        let cwd = this.form.args().map(|a| a[3].clone()).unwrap_or_default();
                        let place = this.form.place;
                        cx.emit(NewAgentEvent::Started {
                            started,
                            cwd,
                            place,
                            workspace,
                            first,
                        });
                    }
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Puts the project first among the recently used, kept beside the layout.
    fn remember_project(&mut self, cx: &mut Context<Self>) {
        new_agent::remember(&mut self.recent, self.form.project.trim());
        if let Some(path) = self.recent_path.clone() {
            let recent = self.recent.clone();
            cx.background_spawn(async move {
                let _ = new_agent::save_recent(&path, &recent);
            })
            .detach();
        }
    }

    /// Return in a small field: the name is done, or the new preset is saved.
    fn confirm(&mut self, _: &menu::OpenSelected, window: &mut Window, cx: &mut Context<Self>) {
        if self.naming_preset {
            self.add_preset(window, cx);
        } else if self.renaming {
            self.renaming = false;
            window.focus(&self.prompt.focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Esc: closes what is open (a small field, a list), and with nothing open, the panel.
    fn cancel(&mut self, _: &menu::Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if self.naming_preset || self.renaming {
            self.naming_preset = false;
            self.renaming = false;
            window.focus(&self.prompt.focus_handle(cx), cx);
        } else if self.open.is_some() {
            self.open = None;
        } else {
            cx.emit(NewAgentEvent::Close);
        }
        cx.notify();
    }

    /// How wide `text` is set at `size` points, in the interface font or, with `mono`, the
    /// terminal's.
    fn text_width(&self, text: &str, size: f32, mono: bool, window: &Window, ui: &UiFont) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let family = if mono {
            self.mono.clone()
        } else {
            ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into())
        };
        let run = TextRun {
            len: text.len(),
            font: gpui::font(family),
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
    }

    /// The kind's icon `size` points tall, in the kind's colour where it is a silhouette.
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

    /// An empty layer over its parent that notes where the parent is drawn, for `menu` to hang
    /// from; an open one follows when it moves.
    fn spot(&self, menu: Menu) -> impl IntoElement {
        let spots = self.spots.clone();
        let open = self.open == Some(menu);
        canvas(
            move |bounds, window, _| {
                let moved = spots.borrow_mut().insert(menu, bounds) != Some(bounds);
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

    fn toggle(&mut self, menu: Menu, cx: &mut Context<Self>) {
        self.open = if self.open == Some(menu) {
            None
        } else {
            Some(menu)
        };
        // The effort thumb starts where it is, not gliding in from where it was last time.
        self.effort_motion.set(None);
        self.naming_preset = false;
        cx.notify();
    }

    /// How wide preset `index` is drawn, lit as `lit` says.
    fn preset_width(&self, index: usize, edited: bool, window: &Window, ui: &UiFont) -> f32 {
        let name = &self.presets[index].name;
        let mut width =
            ui.scale(12.0 + 15.0 + 7.0 + 12.0) + self.text_width(name, 13.0, false, window, ui);
        if edited {
            width += ui.scale(7.0 + 6.0 + 6.0 + 8.0 + 18.0 + 4.0)
                + self.text_width("edited", 12.0, false, window, ui)
                + self.text_width("Update preset", 11.5, false, window, ui);
        }
        width
    }

    /// The presets along the panel's top, the lit one tinted (marked edited, with Update preset,
    /// once changed); each can be deleted under the mouse. Those with no room wait behind `+N`,
    /// and `+` keeps the current settings as a new one.
    fn presets_row(&self, room: f32, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let lit = self.form.lit(&self.presets);
        let accent = self.fg(|t| t.agents_accent);
        let text = self.fg(|t| t.agents_text);
        let branch = self.fg(|t| t.agents_branch);
        let dim = self.fg(|t| t.agents_dim);
        let hover = text.opacity(0.06);
        let add_width = if self.naming_preset {
            ui.scale(NAMING)
        } else if self.presets.is_empty() {
            ui.scale(PRESET + 6.0)
                + self.text_width("Save current as preset", 13.0, false, window, ui)
        } else {
            ui.scale(PRESET)
        };
        let widths: Vec<f32> = (0..self.presets.len())
            .map(|index| {
                let edited = lit.is_some_and(|lit| lit.index == index && lit.edited);
                self.preset_width(index, edited, window, ui)
            })
            .collect();
        let gap = ui.scale(PRESET_GAP);
        let shown = new_agent::fit_presets(
            &widths,
            lit.map(|lit| lit.index),
            room - add_width - gap,
            gap,
            ui.scale(MORE),
        );
        let hidden = self.presets.len() - shown.len();
        let mut row = div()
            .flex()
            .flex_1()
            .min_w(px(0.0))
            .items_center()
            .gap(px(gap))
            .text_size(ui.px(13.0));
        for index in shown {
            let preset = &self.presets[index];
            let (on, edited) = match lit {
                Some(lit) if lit.index == index => (true, lit.edited),
                _ => (false, false),
            };
            let group: SharedString = format!("preset-{index}").into();
            let delete = div()
                .id(ElementId::NamedInteger(
                    "preset-delete".into(),
                    index as u64,
                ))
                .absolute()
                .top(ui.px(-5.0))
                .right(ui.px(-4.0))
                .size(ui.px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(popover::ground(&self.theme))
                .border_1()
                .border_color(self.fg(|t| t.agents_rule))
                .text_size(ui.px(10.0))
                .text_color(dim)
                .invisible()
                .group_hover(group.clone(), |style| style.visible())
                .hover(move |style| style.text_color(text))
                .cursor_pointer()
                .child("×")
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.delete_preset(index, cx);
                }));
            let label = div()
                .id(ElementId::NamedInteger("preset".into(), index as u64))
                .h_full()
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .whitespace_nowrap()
                .cursor_pointer()
                .child(self.kind_icon(preset.kind, 15.0, ui))
                .child(preset.name.clone())
                .when(edited, |label| {
                    label
                        .child(div().size(ui.px(6.0)).rounded_full().bg(accent))
                        .child(
                            div()
                                .text_size(ui.px(12.0))
                                .text_color(accent.opacity(0.7))
                                .child("edited"),
                        )
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.pick(index, cx)));
            let pill = div()
                .group(group)
                .relative()
                .flex_shrink_0()
                .h(ui.px(PRESET))
                .pl(ui.px(12.0))
                .pr(ui.px(if edited { 4.0 } else { 12.0 }))
                .flex()
                .items_center()
                .gap(ui.px(8.0))
                .rounded(ui.px(PRESET / 2.0))
                .child(label)
                .when(edited, |pill| {
                    pill.child(
                        div()
                            .id("preset-update")
                            .flex_shrink_0()
                            .h(ui.px(22.0))
                            .px(ui.px(9.0))
                            .flex()
                            .items_center()
                            .rounded(ui.px(11.0))
                            .bg(accent.opacity(0.2))
                            .hover(move |style| style.bg(accent.opacity(0.3)))
                            .text_size(ui.px(11.5))
                            .text_color(accent)
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .child("Update preset")
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _, cx| this.update_preset(cx)),
                            ),
                    )
                })
                .child(delete);
            row = row.child(if on {
                pill.bg(accent.opacity(0.14)).text_color(accent)
            } else {
                pill.text_color(branch)
                    .hover(move |style| style.bg(hover).text_color(text))
            });
        }
        if hidden > 0 {
            let open = self.open == Some(Menu::Presets);
            row = row.child(
                div()
                    .id("presets-more")
                    .relative()
                    .flex_shrink_0()
                    .h(ui.px(PRESET))
                    .min_w(ui.px(MORE))
                    .px(ui.px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui.px(PRESET / 2.0))
                    .text_color(dim)
                    .cursor_pointer()
                    .when(open, |more| more.bg(hover).text_color(text))
                    .hover(move |style| style.bg(hover).text_color(text))
                    .child(format!("+{hidden}"))
                    .child(self.spot(Menu::Presets))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.toggle(Menu::Presets, cx)),
                    ),
            );
        }
        if self.naming_preset {
            row = row.child(
                div()
                    .key_context(FIELD)
                    .flex_shrink_0()
                    .w(ui.px(NAMING))
                    .h(ui.px(PRESET))
                    .px(ui.px(12.0))
                    .flex()
                    .items_center()
                    .rounded(ui.px(PRESET / 2.0))
                    .bg(text.opacity(0.06))
                    .text_size(ui.px(12.5))
                    .child(self.preset_name.clone()),
            );
        } else {
            row = row.child(
                div()
                    .id("preset-add")
                    .flex_shrink_0()
                    .h(ui.px(PRESET))
                    .min_w(ui.px(PRESET))
                    .px(ui.px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(ui.px(6.0))
                    .rounded(ui.px(PRESET / 2.0))
                    .text_color(dim)
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover).text_color(text))
                    .child(footer_icon::icon(Icon::Plus, dim, ui.scale(1.0)))
                    .when(self.presets.is_empty(), |add| {
                        add.child("Save current as preset")
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.naming_preset = true;
                        this.open = None;
                        window.focus(&this.preset_name.focus_handle(cx), cx);
                        cx.notify();
                    })),
            );
        }
        row
    }

    /// The name corral is asked for, `prefix/name` in small type with a pencil; a click opens it
    /// for editing (the prefix alone for a controller, always `main`).
    fn name_row(&self, ui: &UiFont, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let text = self.fg(|t| t.agents_text);
        let problem = self.error.as_ref().and(self.form.problem()).map(|(f, _)| f);
        let name = if self.form.regular {
            self.form.name.clone()
        } else {
            "main".into()
        };
        if !self.renaming {
            return div()
                .id("name")
                .flex_shrink_0()
                .h(ui.px(PRESET))
                .px(ui.px(10.0))
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .rounded(ui.px(8.0))
                .font_family(self.mono.clone())
                .text_size(ui.px(12.0))
                .text_color(dim)
                .cursor_pointer()
                .hover(move |style| style.bg(text.opacity(0.06)))
                .child(
                    div()
                        .flex()
                        .whitespace_nowrap()
                        .child(format!("{}/", self.form.prefix))
                        .child(div().text_color(self.fg(|t| t.agents_branch)).child(name)),
                )
                .child(stroke_icon(PENCIL, true, dim, ui.scale(12.0), 2.0))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.renaming = true;
                    this.open = None;
                    let field = if this.form.regular {
                        &this.name
                    } else {
                        &this.prefix
                    };
                    window.focus(&field.focus_handle(cx), cx);
                    cx.notify();
                }))
                .into_any_element();
        }
        let field = |input: &Entity<TextInput>, problem: bool| {
            let focused = input.focus_handle(cx).is_focused(window);
            div()
                .h(ui.px(26.0))
                .px(ui.px(8.0))
                .flex()
                .items_center()
                .rounded(ui.px(7.0))
                .bg(text.opacity(0.06))
                .border_1()
                .border_color(if problem {
                    self.fg(|t| t.agents_red)
                } else if focused {
                    self.fg(|t| t.agents_accent).opacity(0.5)
                } else {
                    gpui::transparent_black()
                })
                .overflow_hidden()
                .child(input.clone())
        };
        div()
            .key_context(FIELD)
            .flex_shrink_0()
            .w(ui.px(RENAMING))
            .flex()
            .items_center()
            .gap(ui.px(5.0))
            .font_family(self.mono.clone())
            .text_size(ui.px(12.0))
            .child(
                field(&self.prefix, problem == Some("prefix"))
                    .w(ui.px(110.0))
                    .flex_shrink_0(),
            )
            .child(div().text_color(dimmer).child("/"))
            .child(if self.form.regular {
                field(&self.name, problem == Some("name"))
                    .flex_1()
                    .min_w(px(0.0))
            } else {
                div().flex_1().text_color(dimmer).child("main")
            })
            .child(
                div()
                    .id("rename-done")
                    .flex_shrink_0()
                    .h(ui.px(26.0))
                    .px(ui.px(9.0))
                    .flex()
                    .items_center()
                    .rounded(ui.px(7.0))
                    .font_family(ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into()))
                    .text_color(self.fg(|t| t.agents_branch))
                    .cursor_pointer()
                    .hover(move |style| style.bg(text.opacity(0.06)))
                    .child("Done")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.confirm(&menu::OpenSelected, window, cx)
                    })),
            )
            .into_any_element()
    }

    /// A rounded part along the foot: what leads it and its label; lit while what it opens is
    /// open, and noting where it is drawn for that to hang from.
    fn chip(&self, id: &'static str, ui: &UiFont) -> Stateful<Div> {
        let text = self.fg(|t| t.agents_text);
        div()
            .id(id)
            .relative()
            .flex_shrink_0()
            .h(ui.px(CHIP))
            .px(ui.px(11.0))
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .rounded(ui.px(CHIP / 2.0))
            .border_1()
            .whitespace_nowrap()
            .cursor_pointer()
            .text_size(ui.px(13.0))
            .hover(move |style| style.text_color(text))
    }

    /// A chip that opens `menu`.
    fn menu_chip(
        &self,
        id: &'static str,
        menu: Menu,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let open = self.open == Some(menu);
        let ground = self.fg(|t| t.agents_text).opacity(0.05);
        let chip = self
            .chip(id, ui)
            .child(self.spot(menu))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle(menu, cx)));
        if open {
            chip.bg(ground.opacity(0.1))
                .border_color(self.fg(|t| t.agents_accent).opacity(0.45))
                .text_color(self.fg(|t| t.agents_text))
        } else {
            chip.bg(ground)
                .border_color(self.fg(|t| t.agents_rule).opacity(0.7))
                .text_color(self.fg(|t| t.agents_branch))
        }
    }

    /// The foot: project, kind, its model and effort, where it opens and the Controller switch;
    /// then `</>` and the start button.
    fn foot(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let tool = self.form.tool;
        let text = self.fg(|t| t.agents_text);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let accent = self.fg(|t| t.agents_accent);
        let s = ui.scale(1.0);
        let project = self
            .menu_chip("chip-project", Menu::Project, ui, cx)
            .child(footer_icon::icon(Icon::Folder, dim, s))
            .child(project_name(&self.form.project));
        let chevron = |menu: Menu| {
            stroke_icon(
                if self.open == Some(menu) {
                    CHEVRON_UP
                } else {
                    CHEVRON_DOWN
                },
                false,
                dim,
                ui.scale(12.0),
                2.0,
            )
        };
        // The kind, and beside it its model and effort when it has them to choose (P5-99).
        let agent = self
            .menu_chip("chip-agent", Menu::Agent, ui, cx)
            .pl(ui.px(9.0))
            .pr(ui.px(10.0))
            .child(self.kind_icon(tool, 16.0, ui))
            .child(tool.label())
            .child(chevron(Menu::Agent));
        let model = tool.has_models().then(|| {
            self.menu_chip("chip-model", Menu::Model, ui, cx)
                .pl(ui.px(12.0))
                .pr(ui.px(10.0))
                .child(self.form.model.clone().unwrap_or_else(|| "model".into()))
                .child(div().text_color(dimmer).child("·"))
                .child(
                    div()
                        .when_some(self.effort_spot(&self.form.efforts()), |effort, spot| {
                            effort.text_color(self.warm(spot.2))
                        })
                        .child(self.form.effort.clone().unwrap_or_else(|| "effort".into())),
                )
                .child(chevron(Menu::Model))
        });
        let place = self
            .menu_chip("chip-place", Menu::Place, ui, cx)
            .child(footer_icon::icon(Icon::Panes, dim, s))
            .child(place_label(self.form.place));
        let controller = !self.form.regular;
        let role = self
            .chip("chip-role", ui)
            .map(|chip| {
                if controller {
                    chip.bg(text.opacity(0.05))
                        .border_color(self.fg(|t| t.agents_rule).opacity(0.7))
                        .text_color(self.fg(|t| t.agents_branch))
                } else {
                    chip.border_dashed()
                        .border_color(self.fg(|t| t.agents_rule))
                        .text_color(dim)
                }
            })
            .when(controller, |chip| {
                chip.child(stroke_icon(CHECK, false, accent, ui.scale(12.0), 2.4))
            })
            .child("Controller")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.form.regular = !this.form.regular;
                this.chose(cx);
            }));
        let command = div()
            .id("show-command")
            .flex_shrink_0()
            .size(ui.px(34.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .cursor_pointer()
            .when(self.show_command, |button| button.bg(text.opacity(0.08)))
            .hover(move |style| style.bg(text.opacity(0.08)))
            .child(stroke_icon(
                CODE,
                false,
                if self.show_command { text } else { dim },
                ui.scale(16.0),
                2.0,
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_command = !this.show_command;
                cx.notify();
            }));
        let start = div()
            .id("start")
            .flex_shrink_0()
            .size(ui.px(36.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(accent)
            .child(stroke_icon(
                ARROW_UP,
                false,
                self.bg(|t| t.agents_bg),
                ui.scale(16.0),
                2.4,
            ));
        let start = if self.busy {
            start.opacity(0.5)
        } else {
            start
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.88)))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.create(&menu::CreateAgent, window, cx)
                }))
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .pl(ui.px(18.0))
            .pr(ui.px(12.0))
            .pt(ui.px(10.0))
            .pb(ui.px(12.0))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex_wrap()
                    .items_center()
                    .gap(ui.px(6.0))
                    .child(project)
                    .child(agent)
                    .children(model)
                    .child(place)
                    .child(role),
            )
            .child(command)
            .child(start)
    }

    /// A row of a list: a tick when it is the one in force, what leads it, the value (in the
    /// terminal's font for a path) and the line that says what it is.
    fn menu_row(
        &self,
        id: ElementId,
        lead: Option<AnyElement>,
        label: impl Into<SharedString>,
        note: impl Into<SharedString>,
        on: bool,
        ui: &UiFont,
    ) -> Stateful<Div> {
        let lit = popover::lit(&self.theme);
        div()
            .id(id)
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .px(ui.px(8.0))
            .py(ui.px(6.0))
            .rounded(ui.px(7.0))
            .cursor_pointer()
            .map(|row| {
                if on {
                    row.bg(lit)
                } else {
                    row.hover(move |style| style.bg(lit.opacity(0.7)))
                }
            })
            .child(popover::tick(&self.theme, ui, on))
            .children(lead)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(ui.px(1.0))
                    .child(
                        div()
                            .text_size(ui.px(12.5))
                            .text_color(self.fg(|t| t.agents_text))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(label.into()),
                    )
                    .child(
                        div()
                            .text_size(ui.px(11.0))
                            .text_color(self.fg(|t| t.agents_dim))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(note.into()),
                    ),
            )
    }

    /// The picker under the kind chip: the kinds. A pick closes it.
    fn kind_rows(&self, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let tool = self.form.tool;
        let text = self.fg(|t| t.agents_text);
        let branch = self.fg(|t| t.agents_branch);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let accent = self.fg(|t| t.agents_accent);
        let lit = popover::lit(&self.theme);
        Tool::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                let on = kind == tool;
                div()
                    .id(ElementId::NamedInteger("kind".into(), index as u64))
                    .flex_shrink_0()
                    .h(ui.px(38.0))
                    .px(ui.px(10.0))
                    .flex()
                    .items_center()
                    .gap(ui.px(10.0))
                    .rounded(ui.px(9.0))
                    .cursor_pointer()
                    .map(|row| {
                        if on {
                            row.bg(lit)
                        } else {
                            row.hover(move |style| style.bg(lit.opacity(0.7)))
                        }
                    })
                    .child(self.kind_icon(kind, 18.0, ui))
                    .child(
                        div()
                            .flex_1()
                            .text_size(ui.px(13.5))
                            .text_color(if on { text } else { branch })
                            .child(kind.label()),
                    )
                    .when(!kind.has_models(), |row| {
                        row.child(
                            div()
                                .text_size(ui.px(11.5))
                                .text_color(dimmer)
                                .child("own defaults"),
                        )
                    })
                    .when(on, |row| {
                        row.child(stroke_icon(CHECK, false, accent, ui.scale(14.0), 2.4))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if this.form.tool != kind {
                            this.form.pick_tool(kind);
                        }
                        this.open = None;
                        this.changed(cx);
                    }))
                    .into_any_element()
            })
            .collect()
    }

    /// The picker under the model chip: the kind's models one to a line, and its effort as a
    /// slider whose thumb glides from stop to stop, warming and pulsing the higher it goes. It
    /// stays open while choosing.
    fn model_rows(&self, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let text = self.fg(|t| t.agents_text);
        let branch = self.fg(|t| t.agents_branch);
        let accent = self.fg(|t| t.agents_accent);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let mut rows = Vec::new();
        let heading = |label: &'static str, aside: Option<String>| {
            div()
                .flex_shrink_0()
                .flex()
                .items_baseline()
                .justify_between()
                .px(ui.px(10.0))
                .pt(ui.px(8.0))
                .pb(ui.px(6.0))
                .child(
                    div()
                        .text_size(ui.px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(dim)
                        .child(label),
                )
                .children(aside)
        };
        let from = self.form.source().map(|source| {
            div()
                .text_size(ui.px(11.0))
                .text_color(dimmer)
                .child(source.label())
                .into_any_element()
        });
        rows.push(heading("Model", None).children(from).into_any_element());
        let default = self.form.default_model().map(str::to_owned);
        let mut models = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(1.0))
            .px(ui.px(6.0));
        for (index, model) in self.form.model_names().into_iter().enumerate() {
            let on = self.form.model.as_deref() == Some(model.as_str());
            let is_default = default.as_deref() == Some(model.as_str());
            let row = div()
                .id(ElementId::NamedInteger("model".into(), index as u64))
                .h(ui.px(30.0))
                .flex()
                .items_center()
                .gap(ui.px(8.0))
                .px(ui.px(10.0))
                .rounded(ui.px(8.0))
                .cursor_pointer()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .font_family(self.mono.clone())
                        .text_size(ui.px(12.5))
                        .child(model.clone()),
                )
                .when(is_default, |row| {
                    row.child(
                        div()
                            .flex_shrink_0()
                            .text_size(ui.px(10.5))
                            .text_color(dim)
                            .child("default"),
                    )
                })
                .when(on, |row| {
                    row.child(stroke_icon(CHECK, false, accent, ui.scale(13.0), 2.4))
                })
                .map(|row| {
                    if on {
                        row.bg(accent.opacity(0.13)).text_color(accent)
                    } else {
                        row.text_color(branch)
                            .hover(move |style| style.bg(text.opacity(0.05)).text_color(text))
                    }
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.form.set_model(&model);
                    this.changed(cx);
                }));
            models = models.child(row);
        }
        rows.push(models.into_any_element());
        rows.push(
            div()
                .flex_shrink_0()
                .h(px(1.0))
                .mx(ui.px(16.0))
                .my(ui.px(6.0))
                .bg(text.opacity(0.07))
                .into_any_element(),
        );
        rows.extend(self.effort_slider(ui, cx));
        rows
    }

    /// The colour of an effort `heat` hot, from the theme.
    fn warm(&self, heat: f32) -> Hsla {
        warmth(
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_red),
            heat,
        )
    }

    /// The chosen effort's place among `efforts`, along the track (0 to 1) and how hot it is;
    /// none when the command asks for one the model does not list.
    fn effort_spot(&self, efforts: &[String]) -> Option<(usize, f32, f32)> {
        let count = efforts.len();
        let index = efforts
            .iter()
            .position(|effort| self.form.effort.as_deref() == Some(effort.as_str()))?;
        let along = if count < 2 {
            0.0
        } else {
            index as f32 / (count - 1) as f32
        };
        Some((index, along, heat(&efforts[index], index, count)))
    }

    /// The effort heading, track and names. The thumb glides to a stop chosen by a press or a drag
    /// on the track or a click on a name.
    fn effort_slider(&self, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let efforts = self.form.efforts();
        let count = efforts.len().max(1);
        let spot = self.effort_spot(&efforts);
        let text = self.fg(|t| t.agents_text);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let thumb_ground = self.fg(|t| t.agents_text);
        // A new spot starts a glide from the old one.
        let motion = spot.map(|(_, along, heat)| {
            let to = (along, heat);
            match self.effort_motion.get() {
                Some(motion) if motion.to == to => motion,
                Some(motion) => Motion {
                    from: motion.to,
                    to,
                    moves: motion.moves + 1,
                },
                None => Motion {
                    from: to,
                    to,
                    moves: 0,
                },
            }
        });
        self.effort_motion.set(motion);
        let mut rows = Vec::new();
        let current = spot.map(|(index, _, heat)| {
            div()
                .text_size(ui.px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(self.warm(heat))
                .child(efforts[index].clone())
                .into_any_element()
        });
        rows.push(
            div()
                .flex_shrink_0()
                .flex()
                .items_baseline()
                .justify_between()
                .px(ui.px(10.0))
                .pt(ui.px(4.0))
                .pb(ui.px(2.0))
                .child(
                    div()
                        .text_size(ui.px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(dim)
                        .child("Effort"),
                )
                .children(current)
                .into_any_element(),
        );
        let colors = [
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_red),
        ];
        let rail = text.opacity(0.08);
        let unlit = text.opacity(0.22);
        let middle = ui.scale(15.0);
        let unit = ui.scale(1.0);
        let thumb_at = move |heat: f32| unit * (14.0 + 5.0 * heat);
        // How far past the thumb the ring spreads, as a share of the thumb.
        let ring_reach = 1.1;
        let moving = motion.map(|motion| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .with_animation(
                    ElementId::NamedInteger("effort-glide".into(), motion.moves),
                    Animation::new(GLIDE),
                    move |layer, delta| {
                        let at = |from: f32, to: f32, t: f32| from + (to - from) * t;
                        let along = at(motion.from.0, motion.to.0, settle(delta)).clamp(0.0, 1.0);
                        let heat = at(motion.from.1, motion.to.1, delta);
                        let warm = warmth(colors[0], colors[1], colors[2], heat);
                        let size = thumb_at(heat);
                        let mut layer = layer.child(
                            div()
                                .absolute()
                                .left_0()
                                .top(px(middle - 2.0))
                                .h(px(4.0))
                                .w(relative(along))
                                .rounded_full()
                                .bg(warm),
                        );
                        for stop in 0..count {
                            let place = if count < 2 {
                                0.0
                            } else {
                                stop as f32 / (count - 1) as f32
                            };
                            layer = layer.child(
                                div()
                                    .absolute()
                                    .left(relative(place))
                                    .top(px(middle - 3.0))
                                    .ml(px(-3.0))
                                    .size(px(6.0))
                                    .rounded_full()
                                    .bg(if place <= along + 0.001 { warm } else { unlit }),
                            );
                        }
                        // From `high` up, a ring spreads from the thumb, faster the hotter.
                        if motion.to.1 >= 0.4 {
                            let beat = Duration::from_secs_f32(2.6 - 1.9 * motion.to.1);
                            layer = layer.child(
                                div()
                                    .absolute()
                                    .left(relative(along))
                                    .top(px(middle))
                                    .with_animation(
                                        ElementId::NamedInteger(
                                            "effort-pulse".into(),
                                            (motion.to.1 * 100.0) as u64,
                                        ),
                                        Animation::new(beat).repeat().with_max_fps(30.0),
                                        move |ring, beat| {
                                            let out = (beat / 0.7).min(1.0);
                                            let ring_size = size * (1.0 + ring_reach * out);
                                            ring.ml(px(-ring_size / 2.0))
                                                .mt(px(-ring_size / 2.0))
                                                .size(px(ring_size))
                                                .rounded_full()
                                                .bg(warm.opacity(0.5 * (1.0 - out)))
                                        },
                                    ),
                            );
                        }
                        layer.child(
                            div()
                                .absolute()
                                .left(relative(along))
                                .top(px(middle - size / 2.0))
                                .ml(px(-size / 2.0))
                                .size(px(size))
                                .rounded_full()
                                .bg(thumb_ground)
                                .border(px(3.0))
                                .border_color(warm)
                                .shadow(vec![BoxShadow {
                                    color: warm.opacity(0.55),
                                    offset: point(px(0.0), px(0.0)),
                                    blur_radius: px(4.0 + 14.0 * heat),
                                    spread_radius: px(0.0),
                                    inset: false,
                                }]),
                        )
                    },
                )
        });
        let track_spot = self.effort_track.clone();
        let names = efforts.clone();
        let drag_names = efforts.clone();
        let track = div()
            .id("effort-track")
            .flex_shrink_0()
            .relative()
            .h(ui.px(30.0))
            // The track runs from the middle of the first name to the middle of the last.
            .mx(relative(0.5 / count as f32))
            .cursor_pointer()
            .child(
                canvas(move |bounds, _, _| track_spot.set(bounds), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(middle - 2.0))
                    .h(px(4.0))
                    .rounded_full()
                    .bg(rail),
            )
            .children(moving)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.slide_effort(&names, event.position.x, cx)
                }),
            )
            .on_drag(EffortDrag, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<EffortDrag>, _, cx| {
                    this.slide_effort(&drag_names, event.event.position.x, cx)
                }),
            );
        rows.push(div().px(ui.px(6.0)).child(track).into_any_element());
        let mut labels = div().flex_shrink_0().flex().px(ui.px(6.0)).pb(ui.px(8.0));
        for (index, effort) in efforts.into_iter().enumerate() {
            let on = spot.is_some_and(|(at, _, _)| at == index);
            let color = if on {
                self.warm(heat(&effort, index, count))
            } else {
                dimmer
            };
            labels = labels.child(
                div()
                    .id(ElementId::NamedInteger("effort".into(), index as u64))
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .justify_center()
                    .text_size(ui.px(11.0))
                    .text_color(color)
                    .when(on, |label| label.font_weight(FontWeight::SEMIBOLD))
                    .when(!on, |label| {
                        label.hover(move |style| style.text_color(text))
                    })
                    .cursor_pointer()
                    .child(effort.clone())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.form.set_effort(&effort);
                        this.changed(cx);
                    })),
            );
        }
        rows.push(labels.into_any_element());
        rows
    }

    /// The mouse pressed or dragged at `x` on the effort track: the stop under it.
    fn slide_effort(&mut self, efforts: &[String], x: Pixels, cx: &mut Context<Self>) {
        let track = self.effort_track.get();
        let index = stop_at(
            x.into(),
            track.left().into(),
            track.size.width.into(),
            efforts.len(),
        );
        let Some(effort) = efforts.get(index) else {
            return;
        };
        if self.form.effort.as_deref() != Some(effort.as_str()) {
            self.form.set_effort(effort);
            self.changed(cx);
        }
    }

    /// What is open, hung under its part (over it when there is more room there), over a layer
    /// that closes it when clicked.
    fn menu(&self, menu: Menu, ui: &UiFont, window: &Window, cx: &mut Context<Self>) -> Div {
        let anchor = self.spots.borrow().get(&menu).copied().unwrap_or_default();
        let (width, hang) = match menu {
            Menu::Model => (PICKER, Hang::BelowLeft),
            Menu::Presets => (MENU, Hang::BelowRight),
            Menu::Project | Menu::Agent | Menu::Place => (MENU, Hang::BelowLeft),
        };
        let placed = popover::hang(
            anchor,
            ui.scale(width),
            hang,
            window.viewport_size(),
            ui.scale(1.0),
        );
        let id = |name: &str, index: usize| {
            ElementId::NamedInteger(name.to_owned().into(), index as u64)
        };
        let (title, rows): (Option<&str>, Vec<AnyElement>) = match menu {
            Menu::Project => {
                let mut seen = Vec::new();
                let mut rows = Vec::new();
                for (index, project) in self.projects.iter().chain(&self.recent).enumerate() {
                    if seen.contains(project) {
                        continue;
                    }
                    seen.push(project.clone());
                    let path = project.clone();
                    rows.push(
                        self.menu_row(
                            id("project", index),
                            None,
                            project_name(project),
                            shown_path(project),
                            *project == self.form.project,
                            ui,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_project(path.clone(), cx)
                        }))
                        .into_any_element(),
                    );
                }
                rows.push(popover::rule(&self.theme, ui).into_any_element());
                rows.push(
                    self.menu_row(
                        "project-other".into(),
                        None,
                        "Other…",
                        "Choose a folder",
                        false,
                        ui,
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.browse(cx)))
                    .into_any_element(),
                );
                (Some("Project"), rows)
            }
            Menu::Agent => (None, self.kind_rows(ui, cx)),
            Menu::Model => (None, self.model_rows(ui, cx)),
            Menu::Place => {
                let split = match self.form.place {
                    Place::Split(direction) => Some(direction),
                    _ => None,
                };
                let places = [
                    (
                        Place::Split(split.unwrap_or(Direction::Right)),
                        "Beside the current pane",
                    ),
                    (Place::Tab, "In its own tab"),
                    (Place::Current, "In place of what the pane shows"),
                ];
                let mut rows: Vec<AnyElement> = places
                    .into_iter()
                    .enumerate()
                    .map(|(index, (place, note))| {
                        let on = match place {
                            Place::Split(_) => split.is_some(),
                            _ => self.form.place == place,
                        };
                        let label = if let Place::Split(_) = place {
                            "Split"
                        } else {
                            place_label(place)
                        };
                        self.menu_row(id("place", index), None, label, note, on, ui)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.form.place = place;
                                this.chose(cx);
                            }))
                            .into_any_element()
                    })
                    .collect();
                // Which side to split, as the window's split panel offers it.
                rows.insert(
                    1,
                    div()
                        .flex()
                        .pl(ui.px(8.0 + 10.0) + ui.px(footer_icon::SIZE))
                        .pb(ui.px(4.0))
                        .child(self.directions(split, ui, cx))
                        .into_any_element(),
                );
                (Some("Open in"), rows)
            }
            Menu::Presets => {
                let lit = self.form.lit(&self.presets).map(|lit| lit.index);
                let dim = self.fg(|t| t.agents_dim);
                let text = self.fg(|t| t.agents_text);
                let rows = self
                    .presets
                    .iter()
                    .enumerate()
                    .map(|(index, preset)| {
                        let models = preset.kind.has_models();
                        let note = [
                            preset.model.as_deref().filter(|_| models),
                            preset.effort.as_deref().filter(|_| models),
                            Some(match preset.role {
                                new_agent::Role::Regular => "regular",
                                new_agent::Role::Controller => "controller",
                            }),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · ");
                        let group: SharedString = format!("more-preset-{index}").into();
                        self.menu_row(
                            id("more-preset", index),
                            Some(self.kind_icon(preset.kind, 14.0, ui)),
                            preset.name.clone(),
                            note,
                            lit == Some(index),
                            ui,
                        )
                        .group(group.clone())
                        .child(
                            div()
                                .id(id("more-preset-delete", index))
                                .flex_shrink_0()
                                .size(ui.px(18.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .text_color(dim)
                                .invisible()
                                .group_hover(group, |style| style.visible())
                                .hover(move |style| style.text_color(text))
                                .child("×")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.delete_preset(index, cx);
                                })),
                        )
                        .on_click(
                            cx.listener(move |this, _: &ClickEvent, _, cx| this.pick(index, cx)),
                        )
                        .into_any_element()
                    })
                    .collect();
                (Some("Presets"), rows)
            }
        };
        let panel = popover::panel(&self.theme, ui)
            .id("menu")
            .absolute()
            .left(px(placed.left))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, Some(bottom)) => panel.bottom(px(bottom)),
                _ => panel,
            })
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .overflow_y_scroll()
            .occlude()
            .whitespace_normal()
            .when(matches!(menu, Menu::Agent | Menu::Model), |panel| {
                panel.p(ui.px(8.0)).gap(ui.px(2.0)).rounded(ui.px(14.0))
            })
            .children(title.map(|title| popover::heading(&self.theme, ui, title)))
            .children(rows);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                div()
                    .id("menu-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.open = None;
                            cx.notify();
                        }),
                    ),
            )
            .child(panel)
    }

    /// The four ways to split, each a small frame with that half lit.
    fn directions(&self, on: Option<Direction>, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let lit = self.bg(|t| t.agent_selected);
        let mut row = div().flex().flex_shrink_0().gap(ui.px(4.0));
        for (index, direction) in [
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ]
        .into_iter()
        .enumerate()
        {
            let chosen = on == Some(direction);
            let (stroke, fill) = if chosen {
                (
                    self.fg(|t| t.agents_branch),
                    self.fg(|t| t.agents_accent).opacity(0.6),
                )
            } else {
                let dimmer = self.fg(|t| t.agents_dimmer);
                (dimmer, dimmer.opacity(0.35))
            };
            let half = div().absolute().bg(fill);
            let half = match direction {
                Direction::Left => half.left_0().top_0().w(relative(0.5)).h_full(),
                Direction::Right => half.right_0().top_0().w(relative(0.5)).h_full(),
                Direction::Up => half.left_0().top_0().w_full().h(relative(0.5)),
                Direction::Down => half.left_0().bottom_0().w_full().h(relative(0.5)),
            };
            let frame = div()
                .relative()
                .w(ui.px(16.0))
                .h(ui.px(12.0))
                .rounded(px(2.0))
                .border_1()
                .border_color(stroke)
                .overflow_hidden()
                .child(half);
            let tile = div()
                .id(ElementId::NamedInteger("split".into(), index as u64))
                .size(ui.px(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(7.0))
                .border_1()
                .cursor_pointer()
                .child(frame)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.form.place = Place::Split(direction);
                    this.chose(cx);
                }));
            row = row.child(if chosen {
                tile.bg(lit)
                    .border_color(self.fg(|t| t.agents_border).opacity(0.75))
            } else {
                tile.border_color(self.fg(|t| t.agents_rule).opacity(0.7))
                    .hover(move |style| style.bg(lit.opacity(0.6)))
            });
        }
        row
    }

    /// The command as typed, and the whole call it makes, under `</>`.
    fn command_block(&self, ui: &UiFont, window: &Window, cx: &App) -> Stateful<Div> {
        let problem = self.error.as_ref().and(self.form.problem()).map(|(f, _)| f);
        let focused = self.command.focus_handle(cx).is_focused(window);
        let heading = |text: &'static str| {
            div()
                .mb(ui.px(5.0))
                .text_size(ui.px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(self.fg(|t| t.agents_dimmer))
                .child(text)
        };
        div()
            .id("command")
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px(ui.px(18.0))
            .pt(ui.px(12.0))
            .pb(ui.px(14.0))
            .border_t_1()
            .border_color(popover::edge_rule(&self.theme))
            .child(heading("Command"))
            .child(
                div()
                    .px(ui.px(9.0))
                    .py(ui.px(6.0))
                    .rounded(ui.px(8.0))
                    .bg(gpui::black().opacity(0.22))
                    .border_1()
                    .border_color(if problem == Some("command") {
                        self.fg(|t| t.agents_red)
                    } else if focused {
                        self.fg(|t| t.agents_accent).opacity(0.5)
                    } else {
                        gpui::transparent_black()
                    })
                    .font_family(self.mono.clone())
                    .text_size(ui.px(12.0))
                    .line_height(relative(1.45))
                    .child(self.command.clone()),
            )
            .child(heading("Will run").mt(ui.px(10.0)))
            .child(
                div()
                    .font_family(self.mono.clone())
                    .text_size(ui.px(11.5))
                    .line_height(relative(1.55))
                    .text_color(self.fg(|t| t.agents_branch))
                    .child(self.form.preview(&self.corral)),
            )
    }
}

/// Line drawings in a 24-point square: each path's corners, joined in order.
type Drawing = &'static [&'static [(f32, f32)]];
const PENCIL: Drawing = &[&[
    (4.0, 20.0),
    (8.0, 20.0),
    (19.0, 9.0),
    (15.0, 5.0),
    (4.0, 16.0),
]];
const ARROW_UP: Drawing = &[
    &[(12.0, 19.0), (12.0, 5.0)],
    &[(5.0, 12.0), (12.0, 5.0), (19.0, 12.0)],
];
const CODE: Drawing = &[
    &[(8.0, 7.0), (3.0, 12.0), (8.0, 17.0)],
    &[(16.0, 7.0), (21.0, 12.0), (16.0, 17.0)],
];
const CHEVRON_DOWN: Drawing = &[&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]];
const CHEVRON_UP: Drawing = &[&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)]];
const CHECK: Drawing = &[&[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)]];

/// `drawing` stroked in `color`, `size` points square, its lines `weight` of the 24 thick;
/// `closed` joins each path's last corner to its first.
fn stroke_icon(
    drawing: Drawing,
    closed: bool,
    color: Hsla,
    size: f32,
    weight: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let unit = size / 24.0;
            for corners in drawing {
                let at = |(x, y): (f32, f32)| bounds.origin + point(px(x * unit), px(y * unit));
                let mut path = PathBuilder::stroke(px(weight * unit));
                path.move_to(at(corners[0]));
                for &corner in &corners[1..] {
                    path.line_to(at(corner));
                }
                if closed {
                    path.close();
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .flex_shrink_0()
    .size(px(size))
}

impl Render for NewAgentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let viewport = window.viewport_size();
        let (room_x, room_y) = (f32::from(viewport.width), f32::from(viewport.height));
        let margin = ui.scale(MARGIN);
        let width = ui.scale(WIDTH).min(room_x - 2.0 * margin).max(0.0);
        let top = (room_y * TOP).round();
        let tallest = (room_y - top - margin - ui.scale(HINTS)).max(ui.scale(120.0));
        let text = self.fg(|t| t.agents_text);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);

        let name_width = if self.renaming {
            ui.scale(RENAMING)
        } else {
            let name = if self.form.regular {
                self.form.name.as_str()
            } else {
                "main"
            };
            let shown = format!("{}/{name}", self.form.prefix);
            ui.scale(10.0 + 6.0 + 12.0 + 10.0) + self.text_width(&shown, 12.0, true, window, &ui)
        };
        let room = width - 2.0 * ui.scale(PAD) - name_width - ui.scale(12.0);
        let header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(12.0))
            .px(ui.px(PAD))
            .pt(ui.px(PAD))
            .child(self.presets_row(room, window, &ui, cx))
            .child(self.name_row(&ui, window, cx));

        let prompt_focus = self.prompt.focus_handle(cx);
        let prompt = div()
            .id("prompt")
            .min_h(px(0.0))
            .overflow_y_scroll()
            .mt(ui.px(6.0))
            .px(ui.px(26.0))
            .py(ui.px(14.0))
            .cursor_text()
            .child(
                div()
                    .min_h(ui.px(PROMPT))
                    .max_h(ui.px(PROMPT_MOST))
                    .text_size(ui.px(16.0))
                    .line_height(relative(1.5))
                    .child(self.prompt.clone()),
            )
            .on_click(move |_, window, cx| window.focus(&prompt_focus, cx));

        // amp takes no first message at start: it is sent once amp is idle (P5-98).
        let later = (!self.form.tool.takes_prompt()).then(|| {
            div()
                .flex_shrink_0()
                .px(ui.px(26.0))
                .pb(ui.px(10.0))
                .text_size(ui.px(12.0))
                .text_color(dim)
                .child(format!(
                    "{} starts first; paddock sends this once it is idle.",
                    self.form.tool.label()
                ))
        });
        let unplugged = self
            .plugin
            .as_deref()
            .filter(|_| self.form.tool == Tool::Amp)
            .and_then(|status| match status {
                "same" => None,
                "overwrite" => {
                    Some("The corral plugin for amp has changed; update it in Settings.")
                }
                _ => Some("The corral plugin for amp isn't installed; amp will stay Starting."),
            })
            .map(|warning| {
                let yellow = self.fg(|t| t.agents_yellow);
                div()
                    .flex_shrink_0()
                    .mx(ui.px(18.0))
                    .mb(ui.px(4.0))
                    .px(ui.px(12.0))
                    .py(ui.px(8.0))
                    .flex()
                    .items_center()
                    .gap(ui.px(10.0))
                    .rounded(ui.px(10.0))
                    .bg(yellow.opacity(0.10))
                    .text_size(ui.px(12.5))
                    .text_color(yellow)
                    .child(div().flex_1().min_w(px(0.0)).child(warning))
                    .child(
                        div()
                            .id("open-integrations")
                            .flex_shrink_0()
                            .h(ui.px(26.0))
                            .px(ui.px(11.0))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .bg(yellow.opacity(0.18))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(ui.px(12.0))
                            .cursor_pointer()
                            .hover(move |style| style.bg(yellow.opacity(0.26)))
                            .child("Open Settings")
                            .on_click(|_, _, cx| {
                                let page = crate::settings::Page::Agents;
                                cx.defer(move |cx| crate::windows::open_settings_at(page, cx));
                            }),
                    )
            });

        let status: Option<AnyElement> = if let Some(error) = self.error.clone() {
            Some(
                div()
                    .text_color(self.fg(|t| t.agents_red))
                    .child(error)
                    .into_any_element(),
            )
        } else if self.busy {
            Some(
                div()
                    .text_color(dim)
                    .child(format!("Starting {}…", self.form.full_name()))
                    .into_any_element(),
            )
        } else {
            None
        };

        let panel = div()
            .id("new-agent")
            .w(px(width))
            .max_h(px(tallest))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(ui.px(16.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .bg(popover::ground(&self.theme))
            .shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.55),
                offset: point(px(0.0), ui.px(30.0)),
                blur_radius: ui.px(80.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .text_size(ui.px(13.0))
            .text_color(text)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(header)
            .child(prompt)
            .children(later)
            .children(unplugged)
            .children(status.map(|status| {
                div()
                    .flex_shrink_0()
                    .px(ui.px(26.0))
                    .text_size(ui.px(12.0))
                    .child(status)
            }))
            .child(self.foot(&ui, cx))
            .when(self.show_command, |panel| {
                panel.child(self.command_block(&ui, window, cx))
            });

        let key = |keys: &'static str, what: &'static str| {
            div()
                .flex()
                .gap(ui.px(5.0))
                .child(
                    div()
                        .font_family(self.mono.clone())
                        .text_color(dim)
                        .child(keys),
                )
                .child(what)
        };
        let hints = div()
            .flex_shrink_0()
            .h(ui.px(HINTS))
            .flex()
            .items_center()
            .justify_center()
            .gap(ui.px(18.0))
            .text_size(ui.px(12.0))
            .text_color(dimmer)
            .child(key("⌘↩", "start"))
            .child(key("esc", "close"))
            .child("empty task starts it idle");

        let menu = self.open.map(|menu| self.menu(menu, &ui, window, cx));
        let panel = hold_main_window(
            div()
                .key_context(CONTEXT)
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::create))
                .on_action(cx.listener(Self::confirm))
                .on_action(cx.listener(Self::cancel))
                .on_action(
                    cx.listener(|_, _: &menu::CloseWindow, _, cx| cx.emit(NewAgentEvent::Close)),
                ),
        )
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(top))
        .px(px(margin))
        .child(panel)
        .child(hints)
        .children(menu);
        // Only the panel takes clicks while it is open; a click on the dimmed window closes it.
        ui.apply(div())
            .id("new-agent-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::black().opacity(0.45))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.emit(NewAgentEvent::Close)),
            )
            .child(panel)
            // It dims the whole window, the Browser's page and all.
            .child(crate::browser::cover())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effort_is_as_hot_by_name_for_either_kind() {
        let claude = ["low", "medium", "high", "xhigh", "max"];
        let codex = ["low", "medium", "high", "xhigh", "max", "ultra"];
        for (index, name) in claude.iter().enumerate() {
            assert_eq!(heat(name, index, 5), heat(name, index, 6), "{name}");
        }
        assert_eq!(heat("low", 0, 6), 0.0);
        assert_eq!(heat("high", 2, 5), 0.4);
        assert_eq!(heat(codex[5], 5, 6), 1.0);
        // A name paddock does not know goes by its place.
        assert_eq!(heat("turbo", 1, 3), 0.5);
        assert_eq!(heat("only", 0, 1), 0.0);
    }

    #[test]
    fn the_stop_under_the_mouse_is_the_nearest() {
        assert_eq!(stop_at(0.0, 0.0, 100.0, 5), 0);
        assert_eq!(stop_at(12.4, 0.0, 100.0, 5), 0);
        assert_eq!(stop_at(12.6, 0.0, 100.0, 5), 1);
        assert_eq!(stop_at(260.0, 200.0, 100.0, 6), 3);
        assert_eq!(stop_at(-50.0, 0.0, 100.0, 5), 0);
        assert_eq!(stop_at(500.0, 0.0, 100.0, 5), 4);
        assert_eq!(stop_at(50.0, 0.0, 0.0, 5), 0);
        assert_eq!(stop_at(50.0, 0.0, 100.0, 0), 0);
    }

    #[test]
    fn the_colour_warms_from_the_accent_through_yellow_to_red() {
        let accent = gpui::rgb(0x7fd4bf).into();
        let yellow = gpui::rgb(0xdbbd6e).into();
        let red = gpui::rgb(0xe47a74).into();
        let close = |a: Hsla, b: Hsla| {
            let (a, b) = (gpui::Rgba::from(a), gpui::Rgba::from(b));
            (a.r - b.r).abs() < 0.01 && (a.g - b.g).abs() < 0.01 && (a.b - b.b).abs() < 0.01
        };
        assert!(close(warmth(accent, yellow, red, 0.0), accent));
        assert!(close(warmth(accent, yellow, red, YELLOW_AT), yellow));
        assert!(close(warmth(accent, yellow, red, 1.0), red));
        let between = gpui::Rgba::from(warmth(accent, yellow, red, YELLOW_AT / 2.0));
        let (a, y) = (gpui::Rgba::from(accent), gpui::Rgba::from(yellow));
        assert!((between.r - (a.r + y.r) / 2.0).abs() < 0.01);
    }

    #[test]
    fn the_thumb_overshoots_a_little_and_settles() {
        assert!(settle(0.0).abs() < 1e-6);
        assert!((settle(1.0) - 1.0).abs() < 1e-6);
        let peak = (1..100)
            .map(|i| settle(i as f32 / 100.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.0 && peak < 1.12, "{peak}");
    }
}
