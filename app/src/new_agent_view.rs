//! The New Agent window (P5-37): what the agent should work on first, in a large field that may
//! stay empty; under it a row of choices that each open a short list (project, kind, model,
//! effort, where it opens, role); over it the presets, which set the kind, model, effort and role
//! at once, and the name, which is made up and can be changed. The raw `corral start` call is
//! kept under Show command, where the command can still be edited by hand. Create (⌘↩) runs
//! `corral start` in the background: success closes the window and the main window opens the
//! agent; a failure keeps the window and everything typed, with the reason under the field.
use crate::{
    card,
    config::Config,
    fonts::UiFont,
    kind_icon,
    layout::Direction,
    menu,
    new_agent::{self, Form, Place, Preset, Started, Tool},
    popover::{self, Hang},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, App, Bounds, BoxShadow, ClickEvent, Context, Div, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, FontWeight, Hsla, MouseButton, MouseDownEvent, PathBuilder,
    PathPromptOptions, Pixels, Render, SharedString, Stateful, Subscription, Window, canvas, div,
    point, prelude::*, px, relative,
};
use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

/// The key context of the New Agent window: ⌘↩ creates, Esc steps back, ⌘W closes.
pub const CONTEXT: &str = "PaddockNewAgent";
/// The key context of its small fields (the name, a new preset's name): Return finishes.
pub const FIELD: &str = "PaddockNewAgentField";

/// The top row at the base interface size: the traffic lights and the window's title.
pub const TITLE_BAR: f32 = 38.0;

/// Sizes, in points at the base interface size.
const CHIP: f32 = 28.0;
const BUTTON: f32 = 30.0;
const PROMPT: f32 = 150.0;
const PROMPT_MOST: f32 = 280.0;
const MENU: f32 = 280.0;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// What the main window needs to open the window.
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
}

pub enum NewAgentEvent {
    /// corral started it: open it where the form said.
    Started {
        started: Started,
        cwd: String,
        place: Place,
    },
}

/// The choices under the field, each opening its list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Menu {
    Project,
    Kind,
    Model,
    Effort,
    Place,
    Role,
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
    config_path: PathBuf,
    prompt: Entity<TextInput>,
    command: Entity<TextInput>,
    prefix: Entity<TextInput>,
    name: Entity<TextInput>,
    preset_name: Entity<TextInput>,
    /// The list open under its choice.
    open: Option<Menu>,
    /// Where each choice was last drawn, for its list to hang from.
    spots: Rc<RefCell<HashMap<Menu, Bounds<Pixels>>>>,
    renaming: bool,
    naming_preset: bool,
    show_command: bool,
    busy: bool,
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

impl NewAgentView {
    pub fn new(seed: Seed, place: Place, cx: &mut Context<Self>) -> Self {
        let config_path = crate::config::default_path();
        let presets = new_agent::presets(&Config::load(&config_path).unwrap_or_default());
        let mut form = Form::new(seed.project, place);
        form.set_names(seed.names);
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
        Self {
            theme: seed.theme,
            corral: seed.corral,
            mono: seed.mono,
            projects,
            recent,
            recent_path,
            form,
            presets,
            config_path,
            prompt,
            command,
            prefix,
            name,
            preset_name,
            open: None,
            spots: Rc::default(),
            renaming: false,
            naming_preset: false,
            show_command: false,
            busy: false,
            error: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    /// Opened again from `+` or the split button: open there.
    pub fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        self.form.place = place;
        cx.notify();
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

    /// After a choice: the form changed, the list closes.
    fn chose(&mut self, cx: &mut Context<Self>) {
        self.open = None;
        self.error = None;
        self.sync_command(cx);
        cx.notify();
    }

    fn set_project(&mut self, project: String, cx: &mut Context<Self>) {
        self.form.set_project(project);
        self.chose(cx);
    }

    fn apply(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(preset) = self.presets.get(index).cloned() {
            self.form.apply(&preset);
            self.chose(cx);
        }
    }

    /// Keeps the presets in the config file; a failure says why under the field.
    fn save_presets(&mut self, presets: Vec<Preset>, cx: &mut Context<Self>) {
        match new_agent::save_presets(&self.config_path, &presets) {
            Ok(_) => self.presets = presets,
            Err(error) => self.error = Some(format!("Presets not saved: {error:#}")),
        }
        cx.notify();
    }

    fn delete_preset(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut presets = self.presets.clone();
        if index < presets.len() {
            presets.remove(index);
            self.save_presets(presets, cx);
        }
    }

    /// The current kind, model, effort and role as a preset, under the name typed (replacing one
    /// of that name), or a name made from them.
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
            Some(known) => *known = preset,
            None => presets.push(preset),
        }
        self.save_presets(presets, cx);
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
        let corral = self.corral.clone();
        let task = cx.background_spawn(async move { new_agent::start(&corral, &args) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
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
                        });
                        window.remove_window();
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

    /// Esc: closes what is open (a small field, a list), and with nothing open, the window.
    fn cancel(&mut self, _: &menu::Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if self.naming_preset || self.renaming {
            self.naming_preset = false;
            self.renaming = false;
            window.focus(&self.prompt.focus_handle(cx), cx);
        } else if self.open.is_some() {
            self.open = None;
        } else {
            window.remove_window();
        }
        cx.notify();
    }

    /// A quiet button: no fill until hovered.
    fn ghost(&self, id: &'static str, label: &'static str, ui: &UiFont) -> Stateful<Div> {
        let hover = self.bg(|t| t.agent_selected);
        div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(26.0))
            .px(ui.px(10.0))
            .flex()
            .items_center()
            .rounded(ui.px(6.0))
            .whitespace_nowrap()
            .text_color(self.fg(|t| t.agents_branch))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(label)
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

    /// A small square or round mark in `color`, leading a choice.
    fn dot(&self, color: Hsla, round: bool, ui: &UiFont) -> AnyElement {
        div()
            .flex_shrink_0()
            .size(ui.px(8.0))
            .rounded(if round { ui.px(4.0) } else { ui.px(2.0) })
            .bg(color)
            .into_any_element()
    }

    /// The presets over the field, the one in force lit; each can be deleted under the mouse,
    /// and `+` keeps the current settings as a new one.
    fn presets_row(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let lit = popover::lit(&self.theme);
        let rule = self.fg(|t| t.agents_rule);
        let accent = self.fg(|t| t.agents_accent);
        let text = self.fg(|t| t.agents_text);
        let dim = self.fg(|t| t.agents_dim);
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(ui.px(6.0))
            .text_size(ui.px(12.0));
        for (index, preset) in self.presets.iter().enumerate() {
            let on = self.form.matches(preset);
            let group: SharedString = format!("preset-{index}").into();
            let delete = div()
                .id(ElementId::NamedInteger(
                    "preset-delete".into(),
                    index as u64,
                ))
                .flex_shrink_0()
                .size(ui.px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .text_size(ui.px(11.0))
                .text_color(dim)
                .invisible()
                .group_hover(group.clone(), |style| style.visible())
                .hover(move |style| style.bg(rule).text_color(text))
                .cursor_pointer()
                .child("×")
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.delete_preset(index, cx);
                }));
            let pill = div()
                .id(ElementId::NamedInteger("preset".into(), index as u64))
                .group(group)
                .h(ui.px(28.0))
                .pl(ui.px(9.0))
                .pr(ui.px(4.0))
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .rounded(ui.px(8.0))
                .border_1()
                .cursor_pointer()
                .child(self.kind_icon(preset.kind, 13.0, ui))
                .child(
                    div()
                        .whitespace_nowrap()
                        .font_weight(FontWeight::MEDIUM)
                        .child(preset.name.clone()),
                )
                .child(delete)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.apply(index, cx)));
            row = row.child(if on {
                pill.bg(lit)
                    .border_color(accent.opacity(0.55))
                    .text_color(text)
            } else {
                pill.border_color(rule.opacity(0.8))
                    .text_color(self.fg(|t| t.agents_branch))
                    .hover(move |style| style.bg(lit.opacity(0.6)))
            });
        }
        if self.naming_preset {
            row = row.child(
                div()
                    .key_context(FIELD)
                    .w(ui.px(170.0))
                    .h(ui.px(28.0))
                    .px(ui.px(9.0))
                    .flex()
                    .items_center()
                    .rounded(ui.px(8.0))
                    .bg(well(0.3))
                    .border_1()
                    .border_color(accent.opacity(0.6))
                    .child(self.preset_name.clone()),
            );
        } else {
            row = row.child(
                div()
                    .id("preset-add")
                    .h(ui.px(28.0))
                    .px(ui.px(9.0))
                    .flex()
                    .items_center()
                    .gap(ui.px(5.0))
                    .rounded(ui.px(8.0))
                    .text_color(dim)
                    .cursor_pointer()
                    .hover(move |style| style.bg(lit.opacity(0.6)).text_color(text))
                    .child("+")
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

    /// The name corral is asked for, `prefix/name`; a click opens it for editing.
    fn name_row(&self, ui: &UiFont, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let problem = self.error.as_ref().and(self.form.problem()).map(|(f, _)| f);
        if !self.renaming {
            let name = if self.form.regular {
                self.form.name.clone()
            } else {
                "main".into()
            };
            return div()
                .id("name")
                .flex()
                .items_baseline()
                .gap(ui.px(8.0))
                .px(ui.px(4.0))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .font_family(self.mono.clone())
                        .text_size(ui.px(13.0))
                        .child(
                            div()
                                .text_color(dim)
                                .child(format!("{}/", self.form.prefix)),
                        )
                        .child(name),
                )
                .child(div().text_size(ui.px(11.0)).text_color(dimmer).child(
                    if self.form.regular {
                        "name · click to rename"
                    } else {
                        "a controller is always main · click to change the prefix"
                    },
                ))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.renaming = true;
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
                .rounded(ui.px(6.0))
                .bg(well(0.3))
                .border_1()
                .border_color(if problem {
                    self.fg(|t| t.agents_red)
                } else if focused {
                    self.fg(|t| t.agents_accent).opacity(0.6)
                } else {
                    self.fg(|t| t.agents_rule)
                })
                .overflow_hidden()
                .font_family(self.mono.clone())
                .child(input.clone())
        };
        div()
            .key_context(FIELD)
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .text_size(ui.px(12.5))
            .child(
                field(&self.prefix, problem == Some("prefix"))
                    .w(ui.px(140.0))
                    .flex_shrink_0(),
            )
            .child(div().text_color(dimmer).child("/"))
            .child(if self.form.regular {
                field(&self.name, problem == Some("name")).flex_1()
            } else {
                div()
                    .flex_1()
                    .font_family(self.mono.clone())
                    .text_color(dimmer)
                    .child("main")
            })
            .child(self.ghost("rename-done", "Done", ui).on_click(cx.listener(
                |this, _: &ClickEvent, window, cx| this.confirm(&menu::OpenSelected, window, cx),
            )))
            .into_any_element()
    }

    /// One of the choices under the field: its mark, its value and a small chevron; lit while
    /// its list is open. It notes where it is drawn for the list to hang from.
    fn chip(
        &self,
        menu: Menu,
        lead: AnyElement,
        label: impl Into<SharedString>,
        mono: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let open = self.open == Some(menu);
        let lit = popover::lit(&self.theme);
        let text = self.fg(|t| t.agents_text);
        let spots = self.spots.clone();
        div()
            .id(ElementId::Name(format!("chip-{menu:?}").into()))
            .relative()
            .flex_shrink_0()
            .h(ui.px(CHIP))
            .px(ui.px(8.0))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .rounded(ui.px(7.0))
            .whitespace_nowrap()
            .cursor_pointer()
            .text_size(ui.px(12.0))
            .when(mono, |chip| chip.font_family(self.mono.clone()))
            .map(|chip| {
                if open {
                    chip.bg(lit).text_color(text)
                } else {
                    chip.text_color(self.fg(|t| t.agents_branch))
                        .hover(move |style| style.bg(lit.opacity(0.7)).text_color(text))
                }
            })
            .child(lead)
            .child(label.into())
            .child(chevron(self.fg(|t| t.agents_dim), 90.0, ui.scale(8.0)))
            .child(
                canvas(
                    move |bounds, window, _| {
                        // An open list follows its choice when it moves.
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
                .size_full(),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.open = if this.open == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
                this.naming_preset = false;
                cx.notify();
            }))
    }

    /// The choices, in a row along the field's foot.
    fn chips(&self, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let tool = self.form.tool;
        let dim = self.fg(|t| t.agents_dim);
        let accent = self.fg(|t| t.agents_accent);
        let mut chips = vec![
            self.chip(
                Menu::Project,
                self.dot(dim, false, ui),
                project_name(&self.form.project),
                false,
                ui,
                cx,
            )
            .into_any_element(),
            self.chip(
                Menu::Kind,
                self.kind_icon(tool, 13.0, ui),
                tool.label(),
                false,
                ui,
                cx,
            )
            .into_any_element(),
        ];
        if tool.has_models() {
            chips.push(
                self.chip(
                    Menu::Model,
                    self.dot(accent, true, ui),
                    self.form.model.clone().unwrap_or_else(|| "Model".into()),
                    self.form.model.is_some(),
                    ui,
                    cx,
                )
                .into_any_element(),
            );
            chips.push(
                self.chip(
                    Menu::Effort,
                    self.dot(accent.opacity(0.7), false, ui),
                    self.form.effort.clone().unwrap_or_else(|| "Effort".into()),
                    false,
                    ui,
                    cx,
                )
                .into_any_element(),
            );
        }
        let quiet = self.fg(|t| t.agents_dimmer);
        chips.push(
            self.chip(
                Menu::Place,
                self.dot(quiet, false, ui),
                place_label(self.form.place),
                false,
                ui,
                cx,
            )
            .into_any_element(),
        );
        chips.push(
            self.chip(
                Menu::Role,
                self.dot(quiet, false, ui),
                if self.form.regular {
                    "Regular"
                } else {
                    "Controller"
                },
                false,
                ui,
                cx,
            )
            .into_any_element(),
        );
        chips
    }

    /// A row of an open list: a tick when it is the one in force, the value (in the terminal's
    /// font for a model), and the line that says what it is for.
    #[allow(clippy::too_many_arguments)]
    fn menu_row(
        &self,
        id: ElementId,
        lead: Option<AnyElement>,
        label: impl Into<SharedString>,
        note: impl Into<SharedString>,
        on: bool,
        mono: bool,
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
                            .when(mono, |label| label.font_family(self.mono.clone()))
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

    /// The open choice's list, hung under it (over it when there is more room there), over a
    /// layer that closes it when clicked.
    fn menu(&self, menu: Menu, ui: &UiFont, window: &Window, cx: &mut Context<Self>) -> Div {
        let tool = self.form.tool;
        let anchor = self.spots.borrow().get(&menu).copied().unwrap_or_default();
        let placed = popover::hang(
            anchor,
            ui.scale(MENU),
            Hang::BelowLeft,
            window.viewport_size(),
            ui.scale(1.0),
        );
        let id = |name: &str, index: usize| {
            ElementId::NamedInteger(name.to_owned().into(), index as u64)
        };
        let (title, rows): (&str, Vec<AnyElement>) = match menu {
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
                            false,
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
                        false,
                        ui,
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.browse(cx)))
                    .into_any_element(),
                );
                ("PROJECT", rows)
            }
            Menu::Kind => (
                "AGENT",
                Tool::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, kind)| {
                        self.menu_row(
                            id("kind", index),
                            Some(self.kind_icon(kind, 14.0, ui)),
                            kind.label(),
                            if kind.has_models() {
                                "Model and effort to choose"
                            } else {
                                "Its own model and effort"
                            },
                            kind == tool,
                            false,
                            ui,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if this.form.tool != kind {
                                this.form.pick_tool(kind);
                            }
                            this.chose(cx);
                        }))
                        .into_any_element()
                    })
                    .collect(),
            ),
            Menu::Model => (
                "MODEL",
                tool.models()
                    .iter()
                    .enumerate()
                    .map(|(index, &(model, note))| {
                        self.menu_row(
                            id("model", index),
                            None,
                            model,
                            note,
                            self.form.model.as_deref() == Some(model),
                            true,
                            ui,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.form.set_model(model);
                            this.chose(cx);
                        }))
                        .into_any_element()
                    })
                    .collect(),
            ),
            Menu::Effort => (
                "EFFORT",
                tool.efforts()
                    .iter()
                    .enumerate()
                    .map(|(index, &(effort, note))| {
                        self.menu_row(
                            id("effort", index),
                            None,
                            effort,
                            note,
                            self.form.effort.as_deref() == Some(effort),
                            false,
                            ui,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.form.set_effort(effort);
                            this.chose(cx);
                        }))
                        .into_any_element()
                    })
                    .collect(),
            ),
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
                        self.menu_row(id("place", index), None, label, note, on, false, ui)
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
                        .pl(ui.px(8.0 + 10.0) + ui.px(crate::footer_icon::SIZE))
                        .pb(ui.px(4.0))
                        .child(self.directions(split, ui, cx))
                        .into_any_element(),
                );
                ("OPEN IN", rows)
            }
            Menu::Role => (
                "ROLE",
                [
                    (true, "Regular", "Works on a task"),
                    (
                        false,
                        "Controller",
                        "Splits work and hands it out; always main",
                    ),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (regular, label, note))| {
                    self.menu_row(
                        id("role", index),
                        None,
                        label,
                        note,
                        self.form.regular == regular,
                        false,
                        ui,
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.form.regular = regular;
                        this.chose(cx);
                    }))
                    .into_any_element()
                })
                .collect(),
            ),
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
            .child(popover::heading(&self.theme, ui, title))
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
                .size(ui.px(BUTTON))
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

    /// The command as typed, and the whole call it makes, under Show command.
    fn command_block(&self, ui: &UiFont, window: &Window, cx: &App) -> Div {
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
            .flex()
            .flex_col()
            .px(ui.px(12.0))
            .py(ui.px(10.0))
            .rounded(ui.px(8.0))
            .bg(well(0.22))
            .child(heading("COMMAND"))
            .child(
                div()
                    .px(ui.px(9.0))
                    .py(ui.px(6.0))
                    .rounded(ui.px(6.0))
                    .bg(well(0.3))
                    .border_1()
                    .border_color(if problem == Some("command") {
                        self.fg(|t| t.agents_red)
                    } else if focused {
                        self.fg(|t| t.agents_accent).opacity(0.6)
                    } else {
                        self.fg(|t| t.agents_rule)
                    })
                    .font_family(self.mono.clone())
                    .text_size(ui.px(12.0))
                    .line_height(relative(1.45))
                    .child(self.command.clone()),
            )
            .child(heading("WILL RUN").mt(ui.px(10.0)))
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

/// The darker ground under fields and the command: a shade over the panel colour, which the
/// terminal's background matches in every preset. `depth` is how dark, 0 to 1.
fn well(depth: f32) -> Hsla {
    gpui::black().opacity(depth)
}

/// A `>` in a 9-point square, turned `angle` degrees clockwise, `size` points across.
fn chevron(color: Hsla, angle: f32, size: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (sin, cos) = angle.to_radians().sin_cos();
            let unit = size / 9.0;
            let at = |x: f32, y: f32| {
                let (dx, dy) = (x - 4.5, y - 4.5);
                bounds.origin
                    + point(
                        px((4.5 + dx * cos - dy * sin) * unit),
                        px((4.5 + dx * sin + dy * cos) * unit),
                    )
            };
            let mut path = PathBuilder::stroke(px(1.3 * unit));
            path.move_to(at(3.0, 1.5));
            path.line_to(at(6.0, 4.5));
            path.line_to(at(3.0, 7.5));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .flex_shrink_0()
    .size(px(size))
    .opacity(0.7)
}

impl Render for NewAgentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let bar = crate::windows::title_bar(TITLE_BAR, &ui);
        window.set_traffic_light_position(crate::window::traffic_lights(bar));
        let text = self.fg(|t| t.agents_text);
        let accent = self.fg(|t| t.agents_accent);
        let rule = self.fg(|t| t.agents_rule);
        let prompt_focused = self.prompt.focus_handle(cx).is_focused(window);

        let create = div()
            .id("create")
            .flex_shrink_0()
            .h(ui.px(BUTTON))
            .px(ui.px(12.0))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .rounded(ui.px(8.0))
            .bg(accent)
            .whitespace_nowrap()
            .text_size(ui.px(13.0))
            .text_color(self.bg(|t| t.agents_bg))
            .font_weight(FontWeight::SEMIBOLD)
            .child("Create")
            .child(
                div()
                    .opacity(0.6)
                    .font_weight(FontWeight::NORMAL)
                    .child("⌘↩"),
            );
        let create = if self.busy {
            create.opacity(0.5)
        } else {
            create
                .cursor_pointer()
                .hover(move |style| style.bg(accent.opacity(0.9)))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.create(&menu::CreateAgent, window, cx)
                }))
        };

        let prompt_focus = self.prompt.focus_handle(cx);
        let composer = div()
            .flex()
            .flex_col()
            .rounded(ui.px(12.0))
            .bg(well(0.22))
            .border_1()
            .border_color(if prompt_focused {
                accent.opacity(0.45)
            } else {
                rule
            })
            .when(prompt_focused, |composer| {
                composer.shadow(vec![BoxShadow {
                    color: accent.opacity(0.08),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: ui.px(4.0),
                    inset: false,
                }])
            })
            .child(
                div()
                    .id("prompt")
                    .min_h(ui.px(PROMPT))
                    .max_h(ui.px(PROMPT_MOST))
                    .overflow_y_scroll()
                    .px(ui.px(16.0))
                    .pt(ui.px(14.0))
                    .pb(ui.px(10.0))
                    .flex()
                    .flex_col()
                    .cursor_text()
                    .text_size(ui.px(14.0))
                    .line_height(relative(1.55))
                    .child(self.prompt.clone())
                    .child(
                        div()
                            .mt(ui.px(10.0))
                            .text_size(ui.px(12.0))
                            .text_color(self.fg(|t| t.agents_dimmer))
                            .child("Leave empty to start it idle · ⌘↩ to create"),
                    )
                    .on_click(move |_, window, cx| window.focus(&prompt_focus, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(ui.px(4.0))
                    .px(ui.px(8.0))
                    .py(ui.px(8.0))
                    .border_t_1()
                    .border_color(rule.opacity(0.6))
                    .children(self.chips(&ui, cx)),
            );

        let status: AnyElement = if let Some(error) = self.error.clone() {
            div()
                .text_color(self.fg(|t| t.agents_red))
                .child(error)
                .into_any_element()
        } else if self.busy {
            div()
                .child(format!("Starting {}…", self.form.full_name()))
                .into_any_element()
        } else {
            div()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(format!(
                    "Opens as {} · {}",
                    place_label(self.form.place).to_lowercase(),
                    self.summary()
                ))
                .into_any_element()
        };
        let footer = div()
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .px(ui.px(4.0))
            .text_size(ui.px(11.5))
            .text_color(self.fg(|t| t.agents_dim))
            .child(div().flex_1().min_w(px(0.0)).child(status))
            .child(
                div()
                    .id("show-command")
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .text_color(accent)
                    .hover(move |style| style.text_color(text))
                    .child(if self.show_command {
                        "Hide command"
                    } else {
                        "Show command"
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.show_command = !this.show_command;
                        cx.notify();
                    })),
            )
            .child(
                self.ghost("cancel", "Cancel", &ui)
                    .h(ui.px(BUTTON))
                    .px(ui.px(12.0))
                    .text_size(ui.px(13.0))
                    .on_click(cx.listener(|_, _: &ClickEvent, window, _| window.remove_window())),
            )
            .child(create);

        let body = div()
            .id("new-agent-body")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(ui.px(20.0))
            .pt(ui.px(4.0))
            .pb(ui.px(18.0))
            .flex()
            .flex_col()
            .gap(ui.px(12.0))
            .child(self.presets_row(&ui, cx))
            .child(self.name_row(&ui, window, cx))
            .child(composer)
            .child(footer)
            .when(self.show_command, |body| {
                body.child(self.command_block(&ui, window, cx))
            });

        let menu = self.open.map(|menu| self.menu(menu, &ui, window, cx));
        ui.apply(div())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::create))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(|_, _: &menu::CloseWindow, window, _| window.remove_window()))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(self.bg(|t| t.agents_bg))
            .text_size(ui.px(13.0))
            .text_color(text)
            // The title bar: the traffic lights on the left, the title in the middle.
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(bar))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(ui.px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.fg(|t| t.agents_dim))
                    .child("New Agent"),
            )
            .child(body)
            .children(menu)
    }
}
