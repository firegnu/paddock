//! The New Agent window: the form from `new_agent.rs`, Create to run `corral start` in the
//! background. Success closes the window and the main window opens the agent; a failure keeps
//! the window and everything typed, with the reason beside the buttons.
use crate::{
    fonts::UiFont,
    layout::Direction,
    menu,
    new_agent::{self, Form, Place, Started, Tool},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, ClickEvent, Context, Div, ElementId, Entity,
    EventEmitter, FocusHandle, Focusable, FontWeight, Hsla, PathBuilder, PathPromptOptions, Render,
    SharedString, Stateful, Subscription, Window, canvas, div, ease_out_quint, point, prelude::*,
    px, relative,
};
use std::{rc::Rc, time::Duration};

/// The key context of the New Agent window: ⌘↩ creates, ⌘W closes.
pub const CONTEXT: &str = "PaddockNewAgent";

/// The top row at the base interface size: the traffic lights and the window's title.
pub const TITLE_BAR: f32 = 38.0;

/// The labels' column and the height of a field or button, in points at the base interface size.
const LABEL: f32 = 96.0;
const CONTROL: f32 = 30.0;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// What the main window needs to open the window.
pub struct Seed {
    pub theme: Rc<Theme>,
    pub corral: String,
    /// The terminal's font, for the call preview.
    pub mono: SharedString,
    /// Directories to choose from: paddock's own and the agents'.
    pub projects: Vec<String>,
    /// The active pane's directory.
    pub project: String,
}

pub enum NewAgentEvent {
    /// corral started it: open it where the form said.
    Started {
        started: Started,
        cwd: String,
        place: Place,
    },
}

pub struct NewAgentView {
    theme: Rc<Theme>,
    corral: String,
    mono: SharedString,
    projects: Vec<String>,
    form: Form,
    project: Entity<TextInput>,
    prefix: Entity<TextInput>,
    name: Entity<TextInput>,
    command: Entity<TextInput>,
    prompt: Entity<TextInput>,
    advanced: bool,
    /// Advanced has been opened or closed by hand, so its chevron turns.
    advanced_turned: bool,
    busy: bool,
    error: Option<String>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NewAgentEvent> for NewAgentView {}

impl Focusable for NewAgentView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
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

impl NewAgentView {
    pub fn new(seed: Seed, place: Place, cx: &mut Context<Self>) -> Self {
        let form = Form::new(seed.project, place);
        let colors = text_input::Colors {
            text: hsla(seed.theme.fg(|t| t.agents_text), 1.0),
            placeholder: hsla(seed.theme.fg(|t| t.agents_dimmer), 1.0),
            cursor: hsla(seed.theme.fg(|t| t.focus), 1.0),
            selection: hsla(seed.theme.fg(|t| t.focus), 0.3),
        };
        let mut subscriptions = Vec::new();
        let mut input = |text: &str, placeholder: &str, set: fn(&mut Form, String)| {
            let input =
                cx.new(|cx| TextInput::new(text.to_owned(), placeholder.to_owned(), colors, cx));
            subscriptions.push(cx.subscribe(&input, move |this, input, _: &Changed, cx| {
                let text = input.read(cx).text().to_owned();
                set(&mut this.form, text);
                this.error = None;
                cx.notify();
            }));
            input
        };
        let project = input(&form.project, "/path/to/project", |f, t| f.set_project(t));
        let prefix = input(&form.prefix, "prefix", Form::type_prefix);
        let name = input(&form.name, "name", |f, t| f.name = t);
        let command = input(&form.command, "command", |f, t| f.command = t);
        let prompt = input("", "Optional: the agent's first message", |f, t| {
            f.prompt = t
        });
        let view = Self {
            theme: seed.theme,
            corral: seed.corral,
            mono: seed.mono,
            projects: seed.projects,
            form,
            project,
            prefix,
            name,
            command,
            prompt,
            advanced: false,
            advanced_turned: false,
            busy: false,
            error: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        // Typing a project moves an untyped prefix along; show it.
        let follow = cx.observe(&view.project, |this, _, cx| this.sync_prefix(cx));
        let mut view = view;
        view._subscriptions.push(follow);
        view
    }

    /// Opened again from `+` or `Split ▾`: open there.
    pub fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        self.form.place = place;
        cx.notify();
    }

    fn sync_prefix(&mut self, cx: &mut Context<Self>) {
        let prefix = self.form.prefix.clone();
        if self.prefix.read(cx).text() != prefix {
            self.prefix
                .update(cx, |input, cx| input.set_text(prefix, cx));
        }
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn set_project(&mut self, project: String, cx: &mut Context<Self>) {
        self.form.set_project(project.clone());
        self.project
            .update(cx, |input, cx| input.set_text(project, cx));
        self.sync_prefix(cx);
        self.error = None;
        cx.notify();
    }

    fn choose_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        self.form.choose_tool(tool);
        let command = self.form.command.clone();
        self.command
            .update(cx, |input, cx| input.set_text(command, cx));
        cx.notify();
    }

    /// The system's folder chooser.
    fn browse(&mut self, cx: &mut Context<Self>) {
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
    }

    fn create(&mut self, _: &menu::CreateAgent, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let args = match self.form.args() {
            Ok(args) => args,
            Err(error) => {
                if matches!(self.form.problem(), Some(("command" | "prompt", _))) {
                    self.advanced = true;
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

    fn bg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.bg(pick), 1.0)
    }

    /// A quiet button: no fill until hovered; `outlined` gives it a faint edge.
    fn ghost(
        &self,
        id: &'static str,
        label: &'static str,
        outlined: bool,
        ui: &UiFont,
    ) -> Stateful<Div> {
        let hover = self.bg(|t| t.agent_selected);
        div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(CONTROL))
            .px(ui.px(13.0))
            .flex()
            .items_center()
            .rounded(ui.px(7.0))
            .when(outlined, |button| {
                button.border_1().border_color(self.fg(|t| t.agents_rule))
            })
            .whitespace_nowrap()
            .text_color(self.fg(|t| t.agents_branch))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(label)
    }

    /// A faint box of choices with `on` lit; a colour puts a dot before its label.
    fn segments<T: Copy + PartialEq + 'static>(
        &self,
        id: &'static str,
        options: &[(T, &'static str, Option<Hsla>)],
        on: T,
        pick: fn(&mut Self, T, &mut Context<Self>),
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let lit = self.bg(|t| t.agent_selected);
        let text = self.fg(|t| t.agents_text);
        let mut row = div()
            .flex()
            .flex_shrink_0()
            .p(ui.px(3.0))
            .rounded(ui.px(8.0))
            .bg(well(0.3))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule).opacity(0.7));
        for (index, &(value, label, dot)) in options.iter().enumerate() {
            let mut segment = div()
                .id(ElementId::NamedInteger(id.into(), index as u64))
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .h(ui.px(26.0))
                .px(ui.px(14.0))
                .rounded(ui.px(6.0))
                .whitespace_nowrap()
                .cursor_pointer()
                .children(dot.map(|color| {
                    div()
                        .flex_shrink_0()
                        .size(ui.px(7.0))
                        .rounded_full()
                        .bg(color)
                }))
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| pick(this, value, cx)));
            segment = if value == on {
                segment.bg(lit).text_color(text)
            } else {
                segment
                    .text_color(self.fg(|t| t.agents_dim))
                    .hover(move |style| style.text_color(text))
            };
            row = row.child(segment);
        }
        row
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
                .size(ui.px(CONTROL))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(7.0))
                .border_1()
                .cursor_pointer()
                .child(frame)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_place(Place::Split(direction), cx)
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

    /// A field's box, red when its value is the problem, lit while typing in it.
    fn frame(&self, problem: bool, focused: bool, ui: &UiFont) -> Div {
        div()
            .flex_1()
            .min_w(px(0.0))
            .h(ui.px(CONTROL))
            .px(ui.px(10.0))
            .flex()
            .items_center()
            .rounded(ui.px(7.0))
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
    }

    fn field(
        &self,
        input: &Entity<TextInput>,
        problem: bool,
        ui: &UiFont,
        window: &Window,
        cx: &App,
    ) -> Div {
        let focused = input.focus_handle(cx).is_focused(window);
        self.frame(problem, focused, ui).child(input.clone())
    }

    /// A label on the left, set right, and its control.
    fn row(&self, label: &'static str, control: impl IntoElement, ui: &UiFont) -> Div {
        div()
            .flex()
            .items_center()
            .gap(ui.px(16.0))
            .child(
                div()
                    .w(ui.px(LABEL))
                    .flex_shrink_0()
                    .text_right()
                    .whitespace_nowrap()
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .child(control),
            )
    }

    /// Advanced's chevron, turning a quarter when it opens or closes.
    fn chevron(&self, ui: &UiFont) -> AnyElement {
        let color = self.fg(|t| t.agents_dim);
        let size = ui.scale(9.0);
        let (from, to) = if self.advanced {
            (0.0, 90.0)
        } else {
            (90.0, 0.0)
        };
        if !self.advanced_turned {
            return chevron(color, to, size).into_any_element();
        }
        div()
            .flex_shrink_0()
            .size(px(size))
            .with_animation(
                ElementId::NamedInteger("chevron".into(), self.advanced as u64),
                Animation::new(Duration::from_millis(150)).with_easing(ease_out_quint()),
                move |turning, delta| {
                    turning.child(chevron(color, from + (to - from) * delta, size))
                },
            )
            .into_any_element()
    }
}

/// The darker ground under fields, choices and the call: a shade over the panel colour, which
/// the terminal's background matches in every preset. `depth` is how dark, 0 to 1.
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
}

impl Render for NewAgentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let bar = crate::windows::title_bar(TITLE_BAR, &ui);
        window.set_traffic_light_position(crate::window::traffic_lights(bar));
        let problem = self.error.as_ref().and(self.form.problem()).map(|(f, _)| f);
        let lit = self.bg(|t| t.agent_selected);
        let text = self.fg(|t| t.agents_text);

        let mut chips = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_wrap()
            .gap(ui.px(4.0))
            .font_family(self.mono.clone())
            .text_size(ui.px(11.5));
        for project in &self.projects {
            let path = project.clone();
            let chip = div()
                .id(ElementId::Name(format!("project-{project}").into()))
                .h(ui.px(22.0))
                .px(ui.px(8.0))
                .flex()
                .items_center()
                .rounded(ui.px(6.0))
                .min_w(px(0.0))
                .max_w_full()
                .cursor_pointer()
                // A long path loses its start, so the project's own name stays.
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis_start()
                        .child(shown_path(project)),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_project(path.clone(), cx)
                }));
            chips = chips.child(if *project == self.form.project {
                chip.bg(lit).text_color(text)
            } else {
                chip.text_color(self.fg(|t| t.agents_dim))
                    .hover(move |style| style.text_color(text))
            });
        }
        let project = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .child(
                self.field(&self.project, problem == Some("project"), &ui, window, cx)
                    .font_family(self.mono.clone())
                    .text_size(ui.px(12.0)),
            )
            .child(
                self.ghost("browse", "Choose…", true, &ui)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.browse(cx))),
            );

        let name = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .child(
                div()
                    .w(ui.px(120.0))
                    .flex_shrink_0()
                    .flex()
                    .child(self.field(&self.prefix, problem == Some("prefix"), &ui, window, cx)),
            )
            .child(
                div()
                    .text_color(self.fg(|t| t.agents_dimmer).opacity(0.8))
                    .child("/"),
            )
            .child(if self.form.regular {
                self.field(&self.name, problem == Some("name"), &ui, window, cx)
            } else {
                // A controller is always main.
                self.frame(false, false, &ui)
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child("main")
            });

        // Split keeps its direction; from elsewhere it starts to the right.
        let split = match self.form.place {
            Place::Split(direction) => Place::Split(direction),
            _ => Place::Split(Direction::Right),
        };
        let mut place = div()
            .flex()
            .items_center()
            .gap(ui.px(10.0))
            .child(self.segments(
                "place",
                &[
                    (Place::Current, "Current pane", None),
                    (Place::Tab, "New tab", None),
                    (split, "Split", None),
                ],
                self.form.place,
                |this, place, cx| this.set_place(place, cx),
                &ui,
                cx,
            ));
        if let Place::Split(direction) = self.form.place {
            place = place.child(self.directions(Some(direction), &ui, cx));
        }

        let mut advanced = div()
            .mt(ui.px(16.0))
            .pt(ui.px(10.0))
            .border_t_1()
            .border_color(self.fg(|t| t.agents_rule).opacity(0.7))
            .flex()
            .flex_col()
            .items_start()
            .child(
                div()
                    .id("advanced")
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .py(ui.px(2.0))
                    .cursor_pointer()
                    .text_size(ui.px(12.5))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(self.chevron(&ui))
                    .child("Advanced")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.advanced = !this.advanced;
                        this.advanced_turned = true;
                        cx.notify();
                    })),
            );
        if self.advanced {
            advanced = advanced
                .child(
                    self.row(
                        "Command",
                        self.field(&self.command, problem == Some("command"), &ui, window, cx)
                            .font_family(self.mono.clone())
                            .text_size(ui.px(12.0)),
                        &ui,
                    )
                    .w_full()
                    .mt(ui.px(12.0)),
                )
                .child(
                    self.row(
                        "First message",
                        self.field(&self.prompt, problem == Some("prompt"), &ui, window, cx),
                        &ui,
                    )
                    .w_full()
                    .mt(ui.px(12.0)),
                );
        }

        let will_run = div()
            .mt(ui.px(16.0))
            .px(ui.px(12.0))
            .py(ui.px(10.0))
            .rounded(ui.px(8.0))
            .bg(well(0.22))
            .child(
                div()
                    .mb(ui.px(5.0))
                    .text_size(ui.px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child("WILL RUN"),
            )
            .child(
                div()
                    .font_family(self.mono.clone())
                    .text_size(ui.px(11.5))
                    .line_height(relative(1.55))
                    .text_color(self.fg(|t| t.agents_branch))
                    .child(self.form.preview(&self.corral)),
            );

        let body = div()
            .id("new-agent-body")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(ui.px(28.0))
            .pt(ui.px(10.0))
            .pb(ui.px(4.0))
            .flex()
            .flex_col()
            .child(self.row("Project", project, &ui))
            .child(self.row("", chips, &ui).mt(ui.px(8.0)))
            .child(
                self.row(
                    "Agent",
                    self.segments(
                        "tool",
                        &[
                            (Tool::Claude, "Claude", Some(self.fg(|t| t.claude))),
                            (Tool::Codex, "Codex", Some(self.fg(|t| t.codex))),
                        ],
                        self.form.tool,
                        |this, tool, cx| this.choose_tool(tool, cx),
                        &ui,
                        cx,
                    ),
                    &ui,
                )
                .mt(ui.px(14.0)),
            )
            .child(
                self.row(
                    "Role",
                    self.segments(
                        "role",
                        &[(false, "Controller", None), (true, "Regular", None)],
                        self.form.regular,
                        |this, regular, cx| {
                            this.form.regular = regular;
                            this.error = None;
                            cx.notify();
                        },
                        &ui,
                        cx,
                    ),
                    &ui,
                )
                .mt(ui.px(14.0)),
            )
            .child(self.row("Name", name, &ui).mt(ui.px(14.0)))
            .child(self.row("Open in", place, &ui).mt(ui.px(14.0)))
            .child(advanced)
            .child(will_run);

        let accent = self.fg(|t| t.agents_accent);
        let create = div()
            .id("create")
            .flex_shrink_0()
            .h(ui.px(CONTROL))
            .px(ui.px(14.0))
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .rounded(ui.px(7.0))
            .bg(accent)
            .whitespace_nowrap()
            .text_color(self.bg(|t| t.agents_bg))
            .font_weight(FontWeight::SEMIBOLD)
            .child("Create")
            .child(
                div()
                    .opacity(0.6)
                    .font_weight(FontWeight::MEDIUM)
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
        let footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .pl(ui.px(28.0))
            .pr(ui.px(20.0))
            .py(ui.px(16.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_size(ui.px(12.0))
                    .children(
                        self.error
                            .clone()
                            .map(|error| div().text_color(self.fg(|t| t.agents_red)).child(error)),
                    )
                    .when(self.busy, |status| {
                        status
                            .text_color(self.fg(|t| t.agents_dim))
                            .child(format!("Starting {}…", self.form.full_name()))
                    }),
            )
            .child(
                self.ghost("cancel", "Cancel", false, &ui)
                    .on_click(cx.listener(|_, _: &ClickEvent, window, _| window.remove_window())),
            )
            .child(create);

        ui.apply(div())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::create))
            .on_action(cx.listener(|_, _: &menu::CloseWindow, window, _| window.remove_window()))
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
            .child(footer)
    }
}
