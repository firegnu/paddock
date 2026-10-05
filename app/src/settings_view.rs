//! The Settings window: pages on the left, settings on the right, Revert and Save below. Closing it
//! with unsaved edits asks Save / Don't Save / Cancel. The rules are in `settings.rs`; this file
//! draws them and keeps one text field per typed setting.
use crate::{
    config::Config,
    menu,
    settings::{Conflict, Draft, Field, Kind, Page, Saved},
    text_input::{self, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Hsla, PromptLevel, Render, SharedString, Stateful, Subscription, Task, Window, div,
    prelude::*, px,
};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

/// The key context of the Settings window: ⌘S saves, ⌘W closes.
pub const CONTEXT: &str = "PaddockSettings";

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

pub enum SettingsEvent {
    /// Saved: the config as written, and the settings that wait for a restart.
    Saved {
        config: Box<Config>,
        restart: Vec<String>,
    },
}

pub struct SettingsView {
    theme: Rc<Theme>,
    path: PathBuf,
    draft: Draft,
    page: Page,
    inputs: HashMap<String, Entity<TextInput>>,
    /// A message for the footer, and whether it reports a problem.
    message: Option<(String, bool)>,
    conflict: bool,
    /// The Save / Don't Save / Cancel question is showing.
    asking: bool,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

fn read(path: &PathBuf) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The path with the home directory written `~`.
fn shown_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && text.starts_with(&home) => {
            format!("~{}", &text[home.len()..])
        }
        _ => text,
    }
}

/// The unit after a number setting.
fn unit(key: &str) -> Option<&'static str> {
    match key {
        "sidebar_width" | "font_size" => Some("pt"),
        "refresh_ms" => Some("ms"),
        "line_height" => Some("×"),
        _ => None,
    }
}

/// Writes the whole file at once: to a temporary file beside it, then renamed over it.
fn write(path: &PathBuf, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("toml.saving");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path)
}

impl SettingsView {
    pub fn new(theme: Rc<Theme>, path: PathBuf, cx: &mut Context<Self>) -> Self {
        let (draft, message) = match read(&path)
            .map_err(anyhow::Error::from)
            .and_then(Draft::new)
        {
            Ok(draft) => (draft, None),
            // A file that can't be read or parsed: show the defaults, and why; Save will refuse.
            Err(error) => (
                Draft::new(None).expect("defaults parse"),
                Some((format!("The config file has a problem: {error:#}"), true)),
            ),
        };
        let mut view = Self {
            theme,
            path,
            draft,
            page: Page::General,
            inputs: HashMap::new(),
            message,
            conflict: false,
            asking: false,
            focus: cx.focus_handle(),
            _subscriptions: Vec::new(),
        };
        view.make_inputs(cx);
        view
    }

    fn input_colors(&self) -> text_input::Colors {
        text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        }
    }

    fn make_inputs(&mut self, cx: &mut Context<Self>) {
        let typed: Vec<Field> = self
            .draft
            .fields()
            .iter()
            .filter(|f| !matches!(f.kind, Kind::Bool | Kind::Pet | Kind::Theme))
            .cloned()
            .collect();
        let colors = self.input_colors();
        for field in typed {
            let text = self.draft.value(&field.key);
            let placeholder = match field.kind {
                Kind::List => "Font, Font, …",
                Kind::Color => "#rrggbb, an ANSI name, or default",
                _ => "",
            };
            let input = cx.new(|cx| TextInput::new(text, placeholder, colors, cx));
            let key = field.key.clone();
            self._subscriptions.push(cx.subscribe(
                &input,
                move |this, input, _: &text_input::Changed, cx| {
                    let text = input.read(cx).text().to_owned();
                    this.draft.set(&key, &text);
                    this.message = None;
                    cx.notify();
                },
            ));
            self.inputs.insert(field.key, input);
        }
    }

    /// Puts the drafted values back into the fields after a Default, a theme or a reload.
    fn refresh_inputs(&mut self, cx: &mut Context<Self>) {
        for (key, input) in &self.inputs {
            let value = self.draft.value(key);
            if input.read(cx).text() != value {
                input.update(cx, |input, cx| input.set_text(value, cx));
            }
        }
        cx.notify();
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn set(&mut self, key: &str, value: &str, cx: &mut Context<Self>) {
        self.draft.set(key, value);
        self.message = None;
        self.refresh_inputs(cx);
    }

    fn reset(&mut self, key: &str, cx: &mut Context<Self>) {
        self.draft.reset(key);
        self.message = None;
        self.refresh_inputs(cx);
    }

    pub fn save(&mut self, _: &menu::SaveSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.save_draft(cx);
    }

    /// Saves the draft; false when nothing was written because of a problem, now shown below.
    fn save_draft(&mut self, cx: &mut Context<Self>) -> bool {
        let disk = match read(&self.path) {
            Ok(disk) => disk,
            Err(error) => {
                self.message = Some((format!("Not saved: {error}"), true));
                cx.notify();
                return false;
            }
        };
        let mut saved = false;
        match self.draft.save(disk.as_deref()) {
            Ok(Ok(Saved::Written {
                text,
                config,
                restart,
            })) => {
                if let Err(error) = write(&self.path, &text) {
                    self.message = Some((format!("Not saved: {error}"), true));
                } else {
                    self.draft = Draft::new(Some(text)).expect("just validated");
                    self.refresh_inputs(cx);
                    self.message = Some(if restart.is_empty() {
                        ("Saved.".into(), false)
                    } else {
                        (
                            format!("Saved. Restart paddock for: {}.", restart.join(", ")),
                            false,
                        )
                    });
                    if let Ok(theme) = Theme::from_config(&config) {
                        self.theme = Rc::new(theme);
                        let colors = self.input_colors();
                        for input in self.inputs.values() {
                            input.update(cx, |input, cx| input.set_colors(colors, cx));
                        }
                    }
                    cx.emit(SettingsEvent::Saved { config, restart });
                    saved = true;
                }
            }
            Ok(Ok(Saved::Unchanged)) => {
                self.message = Some(("Nothing to save.".into(), false));
                saved = true;
            }
            Ok(Err(Conflict)) => {
                self.conflict = true;
                self.message = Some((
                    "The config file changed on disk; nothing was saved.".into(),
                    true,
                ));
            }
            Err(error) => self.message = Some((format!("Not saved: {error:#}"), true)),
        }
        cx.notify();
        saved
    }

    fn resolve_conflict(&mut self, keep: bool, cx: &mut Context<Self>) {
        let result = read(&self.path)
            .map_err(anyhow::Error::from)
            .and_then(|disk| {
                if keep {
                    self.draft.rebase(disk)
                } else {
                    self.draft.discard(disk)
                }
            });
        self.conflict = false;
        self.message = match result {
            Ok(()) if keep => Some((
                "Your edits now sit on the file as it is; Save again.".into(),
                false,
            )),
            Ok(()) => None,
            Err(error) => Some((format!("The config file has a problem: {error:#}"), true)),
        };
        self.refresh_inputs(cx);
    }

    /// Shows `page`, as when Go to Agent picked it.
    pub fn show_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    /// Drops the draft; the window stays open.
    fn revert(&mut self, cx: &mut Context<Self>) {
        let result = read(&self.path)
            .map_err(anyhow::Error::from)
            .and_then(|disk| self.draft.discard(disk));
        self.conflict = false;
        self.message = result
            .err()
            .map(|error| (format!("The config file has a problem: {error:#}"), true));
        self.refresh_inputs(cx);
    }

    /// Whether the window may close: yes without unsaved edits; otherwise as answered to Save /
    /// Don't Save / Cancel, where a Save that fails keeps the window open with the reason shown.
    pub fn confirm_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        if !self.draft.edited() {
            return Task::ready(true);
        }
        if self.asking {
            return Task::ready(false);
        }
        self.asking = true;
        let answer = window.prompt(
            PromptLevel::Warning,
            "Save your changes to Settings?",
            Some("If you don't save them, your edits are lost."),
            &["Save", "Don't Save", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            this.update(cx, |this, cx| {
                this.asking = false;
                match answer {
                    Ok(0) => this.save_draft(cx),
                    Ok(1) => true,
                    _ => false,
                }
            })
            .unwrap_or(false)
        })
    }

    /// The red button or ⌘W.
    pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let close = self.confirm_close(window, cx);
        cx.spawn_in(window, async move |_, cx| {
            if close.await {
                let _ = cx.update(|window, _| window.remove_window());
            }
        })
        .detach();
    }

    fn button(&self, id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        div()
            .id(id.into())
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .text_color(self.fg(|t| t.agents_text))
            .cursor_pointer()
            .hover(move |style| style.bg(highlight))
            .child(label.into())
    }

    /// A row of choices, the drafted one highlighted.
    fn segments(
        &self,
        key: &str,
        options: &[(&'static str, &'static str)],
        cx: &mut Context<Self>,
    ) -> Div {
        let current = self.draft.value(key);
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let mut row = div()
            .flex()
            .p(px(2.0))
            .gap(px(2.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule));
        for &(value, label) in options {
            let on = current == value;
            let key = key.to_owned();
            let mut segment = div()
                .id(ElementId::Name(format!("{key}-{value}").into()))
                .px(px(12.0))
                .py(px(3.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .child(label)
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.set(&key, value, cx)),
                );
            segment = if on {
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

    fn switch(&self, key: &str, cx: &mut Context<Self>) -> Stateful<Div> {
        let on = self.draft.value(key) == "true";
        let key = key.to_owned();
        let track = if on {
            self.fg(|t| t.agents_green)
        } else {
            self.fg(|t| t.agents_faint)
        };
        div()
            .id(ElementId::Name(format!("{key}-switch").into()))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .child(
                div()
                    .w(px(34.0))
                    .h(px(18.0))
                    .p(px(2.0))
                    .rounded(px(9.0))
                    .bg(track)
                    .flex()
                    .when(on, |track| track.justify_end())
                    .child(div().size(px(14.0)).rounded(px(7.0)).bg(gpui::white())),
            )
            .child(
                div()
                    .text_color(self.fg(|t| t.muted))
                    .child(if on { "On" } else { "Off" }),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let value = if on { "false" } else { "true" };
                this.set(&key, value, cx)
            }))
    }

    fn field_box(&self, key: &str) -> Div {
        div()
            .flex_1()
            .min_w(px(0.0))
            .h(px(26.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded(px(6.0))
            .border_1()
            .border_color(self.fg(|t| t.agents_rule))
            .bg(hsla(self.theme.terminal().background, 1.0))
            .overflow_hidden()
            .children(self.inputs.get(key).cloned())
    }

    fn row(&self, field: &Field, cx: &mut Context<Self>) -> AnyElement {
        let key = field.key.clone();
        let control: AnyElement = match field.kind {
            Kind::Bool => self.switch(&key, cx).into_any_element(),
            Kind::Pet => self
                .segments(
                    &key,
                    &[("clawd", "Clawd"), ("cat", "Cat"), ("capybara", "Capybara")],
                    cx,
                )
                .into_any_element(),
            Kind::Theme => self
                .segments(
                    &key,
                    &[("dune", "Dune"), ("tide", "Tide"), ("lagoon", "Lagoon")],
                    cx,
                )
                .into_any_element(),
            Kind::Color => {
                let swatch = self
                    .draft
                    .theme()
                    .and_then(|theme| theme.color(field.color().unwrap()))
                    .map(|rgb| hsla(rgb, 1.0))
                    .unwrap_or(gpui::transparent_black());
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(self.fg(|t| t.agents_rule))
                            .bg(swatch),
                    )
                    .child(self.field_box(&key).font_family("Menlo"))
                    .into_any_element()
            }
            _ => div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(self.field_box(&key))
                .children(unit(&key).map(|unit| {
                    div()
                        .w(px(20.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(unit)
                }))
                .into_any_element(),
        };
        let mut tags = div().flex().items_center().gap(px(6.0)).flex_shrink_0();
        if self.draft.custom(&key) {
            tags = tags.child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.fg(|t| t.agents_accent))
                    .child("custom"),
            );
        }
        if field.restart && self.draft.changed(&key) {
            tags = tags.child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.fg(|t| t.agents_yellow))
                    .child("Restart required"),
            );
        }
        let reset_key = key.clone();
        tags = tags.child(
            div()
                .id(ElementId::Name(format!("{key}-default").into()))
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(5.0))
                .text_size(px(11.0))
                .text_color(self.fg(|t| t.agents_dim))
                .cursor_pointer()
                .hover(|style| style.text_color(gpui::white()))
                .child("Default")
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.reset(&reset_key, cx)),
                ),
        );
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .py(px(4.0))
            .child(
                div()
                    .w(px(170.0))
                    .flex_shrink_0()
                    .text_color(self.fg(|t| t.agents_text))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .when(field.kind == Kind::Color, |label| {
                        label.font_family("Menlo").text_size(px(12.0))
                    })
                    .child(field.label.clone()),
            )
            .child(div().flex_1().min_w(px(0.0)).flex().child(control))
            .child(tags)
            .into_any_element()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let mut nav = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .w(px(150.0))
            .flex_shrink_0();
        for page in Page::ALL {
            let on = page == self.page;
            let mut item = div()
                .id(page.label())
                .px(px(12.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .child(page.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.page = page;
                    cx.notify();
                }));
            item = if on {
                item.bg(highlight)
                    .text_color(self.fg(|t| t.agents_text))
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                item.text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(highlight.opacity(0.6)))
            };
            nav = nav.child(item);
        }

        let fields: Vec<Field> = self
            .draft
            .fields()
            .iter()
            .filter(|f| f.page == self.page)
            .cloned()
            .collect();
        let mut list = div()
            .id("settings-list")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pr(px(8.0));
        let mut heading = "";
        for field in &fields {
            if field.group != heading {
                heading = field.group;
                list = list.child(
                    div()
                        .pt(px(14.0))
                        .pb(px(4.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(self.fg(|t| t.agents_accent))
                        .child(heading),
                );
            }
            list = list.child(self.row(field, cx));
        }

        let mut footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pt(px(10.0))
            .border_t_1()
            .border_color(self.fg(|t| t.agents_rule));
        footer = footer.child(div().flex_1().min_w(px(0.0)).text_size(px(12.0)).children(
            self.message.as_ref().map(|(text, problem)| {
                div()
                    .text_color(if *problem {
                        self.fg(|t| t.agents_red)
                    } else {
                        self.fg(|t| t.agents_green)
                    })
                    .child(text.clone())
            }),
        ));
        if self.conflict {
            footer = footer
                .child(self.button("keep", "Keep my edits").on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.resolve_conflict(true, cx)),
                ))
                .child(self.button("discard", "Discard my edits").on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.resolve_conflict(false, cx)),
                ));
        } else {
            let edited = self.draft.edited();
            let revert = if edited {
                self.button("revert", "Revert")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.revert(cx)))
            } else {
                // Nothing to revert: shown, but not clickable.
                div()
                    .id("revert")
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(self.fg(|t| t.agents_rule))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child("Revert")
            };
            footer = footer.child(revert).child(
                self.button("save", "Save  ⌘S")
                    .bg(hsla(self.theme.bg(|t| t.agent_selected), 1.0))
                    .border_color(self.fg(|t| t.focus))
                    .font_weight(FontWeight::SEMIBOLD)
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.save(&menu::SaveSettings, window, cx)
                    })),
            );
        }

        div()
            .id("settings")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::save))
            .on_action(
                cx.listener(|this, _: &menu::CloseWindow, window, cx| {
                    this.request_close(window, cx)
                }),
            )
            .size_full()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .p(px(16.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(format!("Config file: {}", shown_path(&self.path))),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .gap(px(16.0))
                    .child(nav)
                    .child(list),
            )
            .child(footer)
    }
}
