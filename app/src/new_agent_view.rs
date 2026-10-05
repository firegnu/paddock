//! The New Agent window: the form from `new_agent.rs`, Create to run `corral start` in the
//! background. Success closes the window and the main window opens the agent; a failure keeps
//! the window and everything typed, with the reason below.
use crate::{
    fonts::UiFont,
    menu,
    new_agent::{self, Form, Place, Started, Tool},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Hsla, PathPromptOptions, Render, SharedString, Stateful, Subscription, Window, div,
    prelude::*, px,
};
use std::rc::Rc;

/// The key context of the New Agent window: ⌘↩ creates, ⌘W closes.
pub const CONTEXT: &str = "PaddockNewAgent";

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

    fn button(&self, id: &'static str, label: impl Into<SharedString>) -> Stateful<Div> {
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        div()
            .id(id)
            .px(px(12.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .text_color(self.fg(|t| t.agents_text))
            .cursor_pointer()
            .hover(move |style| style.bg(highlight))
            .child(label.into())
    }

    /// A row of choices, `on` highlighted.
    fn segments<T: Copy + PartialEq + 'static>(
        &self,
        id: &'static str,
        options: &[(T, &'static str)],
        on: T,
        pick: fn(&mut Self, T, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) -> Div {
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let mut row = div()
            .flex()
            .flex_wrap()
            .p(px(2.0))
            .gap(px(2.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule));
        for (index, &(value, label)) in options.iter().enumerate() {
            let mut segment = div()
                .id(ElementId::NamedInteger(id.into(), index as u64))
                .px(px(12.0))
                .py(px(3.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| pick(this, value, cx)));
            segment = if value == on {
                segment
                    .bg(highlight)
                    .text_color(self.fg(|t| t.agents_text))
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                segment
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(highlight.opacity(0.6)))
            };
            row = row.child(segment);
        }
        row
    }

    fn field(&self, input: &Entity<TextInput>, problem: bool) -> Div {
        div()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(26.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded(px(6.0))
            .border_1()
            .border_color(if problem {
                self.fg(|t| t.agents_red)
            } else {
                self.fg(|t| t.agents_rule)
            })
            .bg(hsla(self.theme.terminal().background, 1.0))
            .overflow_hidden()
            .child(input.clone())
    }

    fn row(&self, label: &'static str, control: impl IntoElement) -> Div {
        div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(104.0))
                    .flex_shrink_0()
                    .pt(px(4.0))
                    .text_color(self.fg(|t| t.muted))
                    .child(label),
            )
            .child(div().flex_1().min_w(px(0.0)).child(control))
    }
}

impl Render for NewAgentView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let problem = self.error.as_ref().and(self.form.problem()).map(|(f, _)| f);
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);

        let mut chips = div().flex().flex_wrap().gap(px(4.0)).pt(px(6.0));
        for project in &self.projects {
            let on = *project == self.form.project;
            let path = project.clone();
            let mut chip = div()
                .id(ElementId::Name(format!("project-{project}").into()))
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(5.0))
                .border_1()
                .text_size(ui.px(11.0))
                .cursor_pointer()
                .child(shown_path(project))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_project(path.clone(), cx)
                }));
            chip = if on {
                chip.bg(highlight)
                    .border_color(self.fg(|t| t.focus))
                    .text_color(self.fg(|t| t.agents_text))
            } else {
                chip.border_color(self.fg(|t| t.agents_rule))
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(highlight.opacity(0.6)))
            };
            chips = chips.child(chip);
        }
        let project = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(self.field(&self.project, problem == Some("project")))
                    .child(
                        self.button("browse", "Choose…")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.browse(cx))),
                    ),
            )
            .child(chips);

        let name: AnyElement = {
            let mut row = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .w(px(180.0))
                        .flex()
                        .child(self.field(&self.prefix, problem == Some("prefix"))),
                )
                .child(div().text_color(self.fg(|t| t.agents_dim)).child("/"));
            row = if self.form.regular {
                row.child(self.field(&self.name, problem == Some("name")))
            } else {
                row.child(
                    div()
                        .px(px(4.0))
                        .text_color(self.fg(|t| t.agents_text))
                        .child("main"),
                )
                .child(
                    div()
                        .text_size(ui.px(11.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child("a controller is always main"),
                )
            };
            row.into_any_element()
        };

        let mut body = div()
            .id("new-agent-body")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(self.row("Project", project))
            .child(self.row(
                "Agent",
                div().flex().child(self.segments(
                    "tool",
                    &[(Tool::Codex, "Codex"), (Tool::Claude, "Claude")],
                    self.form.tool,
                    |this, tool, cx| this.choose_tool(tool, cx),
                    cx,
                )),
            ))
            .child(self.row(
                "Role",
                div().flex().child(self.segments(
                    "role",
                    &[(false, "Controller"), (true, "Regular")],
                    self.form.regular,
                    |this, regular, cx| {
                        this.form.regular = regular;
                        this.error = None;
                        cx.notify();
                    },
                    cx,
                )),
            ))
            .child(self.row("Name", name))
            .child(self.row(
                "Open in",
                div().flex().child(self.segments(
                    "place",
                    &Place::ALL.map(|p| (p, p.label())),
                    self.form.place,
                    |this, place, cx| this.set_place(place, cx),
                    cx,
                )),
            ))
            .child(
                div()
                    .id("advanced")
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .pt(px(4.0))
                    .cursor_pointer()
                    .text_color(self.fg(|t| t.agents_accent))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(if self.advanced {
                        "▾ Advanced"
                    } else {
                        "▸ Advanced"
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.advanced = !this.advanced;
                        cx.notify();
                    })),
            );
        if self.advanced {
            body = body
                .child(
                    self.row(
                        "Command",
                        div()
                            .flex()
                            .child(self.field(&self.command, problem == Some("command"))),
                    ),
                )
                .child(
                    self.row(
                        "First message",
                        div()
                            .flex()
                            .child(self.field(&self.prompt, problem == Some("prompt"))),
                    ),
                )
                .child(
                    self.row(
                        "Will run",
                        div()
                            .p(px(8.0))
                            .rounded(px(6.0))
                            .bg(hsla(self.theme.terminal().background, 1.0))
                            .font_family(self.mono.clone())
                            .text_size(ui.px(11.5))
                            .text_color(self.fg(|t| t.agents_text))
                            .child(self.form.preview(&self.corral)),
                    ),
                );
        }

        let mut footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pt(px(10.0))
            .border_t_1()
            .border_color(self.fg(|t| t.agents_rule))
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
                            .text_color(self.fg(|t| t.muted))
                            .child(format!("Starting {}…", self.form.full_name()))
                    }),
            )
            .child(
                self.button("cancel", "Cancel")
                    .on_click(cx.listener(|_, _: &ClickEvent, window, _| window.remove_window())),
            );
        let create = self
            .button("create", "Create  ⌘↩")
            .bg(highlight)
            .border_color(self.fg(|t| t.focus))
            .font_weight(FontWeight::SEMIBOLD);
        footer = if self.busy {
            footer.child(create.opacity(0.5).cursor_default())
        } else {
            footer.child(
                create.on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.create(&menu::CreateAgent, window, cx)
                })),
            )
        };

        ui.apply(div())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::create))
            .on_action(cx.listener(|_, _: &menu::CloseWindow, window, _| window.remove_window()))
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(18.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .child(body)
            .child(footer)
    }
}
