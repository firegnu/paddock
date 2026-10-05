//! The Settings window: pages on the left, settings on the right, Revert and Save below. Closing it
//! with unsaved edits asks Save / Don't Save / Cancel. The rules are in `settings.rs`; this file
//! draws them and keeps one text field per typed setting.
use crate::{
    config::Config,
    diagnostics::{self, Checks, Report, clock},
    fonts::{self, Installed, UiFont},
    menu,
    settings::{Conflict, Draft, Field, Kind, Page, Saved},
    text_input::{self, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Hsla, MouseButton, PromptLevel, Render, ScrollStrategy, SharedString, Stateful,
    Subscription, Task, UniformListScrollHandle, Window, anchored, deferred, div, prelude::*, px,
    uniform_list,
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
    /// Diagnostics as last collected, and its background checks once they are done.
    report: Option<Report>,
    checks: Option<Checks>,
    /// Installed families for the font lists.
    fonts: Installed,
    /// The open font list.
    picker: Option<Picker>,
    /// The font list a press just closed by landing outside it, so a press on its own button
    /// closes it rather than opening it again.
    dismissed: Option<String>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

/// A font list opened under its setting: a search field over the matching families.
struct Picker {
    key: String,
    /// Monospace families only, for the terminal.
    mono: bool,
    input: Entity<TextInput>,
    /// The highlighted row among the matches.
    index: usize,
    scroll: UniformListScrollHandle,
    _subscription: Subscription,
}

/// Rows a font list shows before it scrolls.
const PICKER_ROWS: usize = 10;

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
        "sidebar_width" | "ui_font_size" | "font_size" => Some("pt"),
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
            report: None,
            checks: None,
            fonts: cx.try_global::<Installed>().cloned().unwrap_or_default(),
            picker: None,
            dismissed: None,
            focus: cx.focus_handle(),
            _subscriptions: Vec::new(),
        };
        view.make_inputs(cx);
        view.load_fonts(cx);
        view
    }

    /// Reads the installed families off the UI thread, then measures which are monospace; both are
    /// kept for the next Settings window.
    fn load_fonts(&mut self, cx: &mut Context<Self>) {
        if self.fonts.mono.is_some() {
            return;
        }
        let text = cx.text_system().clone();
        cx.spawn(async move |this, cx| {
            let all = {
                let text = text.clone();
                cx.background_spawn(async move { fonts::visible(text.all_font_names()) })
                    .await
            };
            this.update(cx, |this, cx| {
                this.fonts.all = all.clone();
                cx.notify();
            })?;
            let mono = cx
                .background_spawn(async move { fonts::monospace_families(&text, &all) })
                .await;
            this.update(cx, |this, cx| {
                this.fonts.mono = Some(mono);
                cx.set_global(this.fonts.clone());
                cx.notify();
            })
        })
        .detach();
    }

    /// The families a font list offers, as shown: for the interface, the system's first.
    fn font_names(&self, mono: bool) -> Vec<String> {
        if mono {
            return self.fonts.mono.clone().unwrap_or_default();
        }
        let mut names = vec![fonts::SYSTEM.to_owned()];
        names.extend(self.fonts.all.iter().cloned());
        names
    }

    /// The open list's matches for what is typed in its field.
    fn picker_matches(&self, cx: &gpui::App) -> Vec<String> {
        let Some(picker) = &self.picker else {
            return Vec::new();
        };
        let names = self.font_names(picker.mono);
        let query = picker.input.read(cx).text().to_owned();
        fonts::matching(&names, &query)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// A setting's font as its list names it.
    fn font_label(&self, key: &str) -> String {
        let value = self.draft.value(key);
        if value.is_empty() && key == "ui_font" {
            fonts::SYSTEM.to_owned()
        } else {
            value
        }
    }

    /// Opens `key`'s font list with its field focused and the current font highlighted, or closes
    /// it when a press outside just did.
    fn toggle_picker(
        &mut self,
        key: &str,
        mono: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dismissed.take().as_deref() == Some(key) {
            return;
        }
        let colors = self.input_colors();
        let input = cx.new(|cx| TextInput::new("", "Search fonts", colors, cx));
        let subscription = cx.subscribe(&input, |this, _, _: &text_input::Changed, cx| {
            if let Some(picker) = &mut this.picker {
                picker.index = 0;
                picker.scroll.scroll_to_item(0, ScrollStrategy::Top);
            }
            cx.notify();
        });
        window.focus(&input.read(cx).focus_handle(cx), cx);
        let current = self.font_label(key);
        let index = self
            .font_names(mono)
            .iter()
            .position(|name| *name == current)
            .unwrap_or(0);
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(index, ScrollStrategy::Center);
        self.picker = Some(Picker {
            key: key.to_owned(),
            mono,
            input,
            index,
            scroll,
            _subscription: subscription,
        });
        cx.notify();
    }

    fn close_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.picker.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// ↑↓ in the open font list.
    fn move_pick(&mut self, step: isize, cx: &mut Context<Self>) {
        let count = self.picker_matches(cx).len();
        let Some(picker) = &mut self.picker else {
            return;
        };
        if count > 0 {
            let index = (picker.index.min(count - 1) as isize + step).clamp(0, count as isize - 1);
            picker.index = index as usize;
            picker
                .scroll
                .scroll_to_item(picker.index, ScrollStrategy::Nearest);
            cx.notify();
        }
    }

    /// Drafts the font at `index` of the open list's matches and closes it; Save applies it.
    fn pick(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.picker_matches(cx).into_iter().nth(index) else {
            return;
        };
        let Some(key) = self.picker.as_ref().map(|picker| picker.key.clone()) else {
            return;
        };
        let value = if key == "ui_font" && name == fonts::SYSTEM {
            ""
        } else {
            &name
        };
        self.set(&key, value, cx);
        self.close_picker(window, cx);
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
            .filter(|f| {
                !matches!(
                    f.kind,
                    Kind::Bool | Kind::Pet | Kind::Theme | Kind::MonoFont | Kind::UiFont
                )
            })
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

    /// Collects what Diagnostics shows: at once from the main window, then the command and file
    /// checks in the background.
    fn refresh_diagnostics(&mut self, cx: &mut Context<Self>) {
        let Some(report) = crate::windows::report(cx) else {
            return;
        };
        let (corral, shell, config) = (
            report.corral.clone(),
            report.shell.clone(),
            report.config_path.clone(),
        );
        self.report = Some(report);
        self.checks = None;
        let task = cx.background_spawn(async move {
            diagnostics::check(&corral, &shell, &config, std::time::Duration::from_secs(5))
        });
        cx.spawn(async move |this, cx| {
            let checks = task.await;
            let _ = this.update(cx, |this, cx| {
                this.checks = Some(checks);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// One Diagnostics line: a label and its value, coloured good, bad or unknown.
    fn diagnostic(&self, label: &'static str, value: impl Into<SharedString>, tone: Tone) -> Div {
        let color = match tone {
            Tone::Good => self.fg(|t| t.agents_green),
            Tone::Bad => self.fg(|t| t.agents_red),
            Tone::Plain => self.fg(|t| t.agents_text),
            Tone::Unknown => self.fg(|t| t.agents_dim),
        };
        div()
            .flex()
            .gap(px(12.0))
            .py(px(3.0))
            .child(
                div()
                    .w(px(150.0))
                    .flex_shrink_0()
                    .text_color(self.fg(|t| t.muted))
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_color(color)
                    .child(value.into()),
            )
    }

    fn diagnostics_page(&self, ui: &UiFont) -> Vec<AnyElement> {
        let Some(report) = &self.report else {
            return vec![
                self.diagnostic(
                    "Diagnostics",
                    "Unavailable: the main window is gone.",
                    Tone::Bad,
                )
                .into_any_element(),
            ];
        };
        let checks = self.checks.as_ref();
        let heading = |text: &'static str| {
            div()
                .pt(px(14.0))
                .pb(px(4.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(self.fg(|t| t.agents_accent))
                .child(text)
                .into_any_element()
        };
        let checking = || ("checking…".to_owned(), Tone::Unknown);
        let path = |found: &Result<std::path::PathBuf, String>| match found {
            Ok(path) => (path.display().to_string(), Tone::Good),
            Err(error) => (error.clone(), Tone::Bad),
        };
        let outcome = |result: &Result<String, String>| match result {
            Ok(text) => (text.clone(), Tone::Good),
            Err(error) => (error.clone(), Tone::Bad),
        };
        let row = |label, (value, tone): (String, Tone)| {
            self.diagnostic(label, value, tone).into_any_element()
        };
        let mut rows = vec![heading("Commands")];
        rows.push(row(
            "corral",
            checks.map_or_else(checking, |c| path(&c.corral.path)),
        ));
        rows.push(row(
            "corral version",
            checks.map_or_else(checking, |c| outcome(&c.corral.version)),
        ));
        rows.push(row(
            "git",
            checks.map_or_else(checking, |c| path(&c.git.path)),
        ));
        rows.push(row(
            "git version",
            checks.map_or_else(checking, |c| outcome(&c.git.version)),
        ));
        rows.push(row(
            "shell",
            checks.map_or_else(checking, |c| path(&c.shell)),
        ));

        rows.push(heading("Agents"));
        rows.push(row(
            "Latest corral ls",
            match &report.agents {
                None => ("not read yet".into(), Tone::Unknown),
                Some((time, Ok(text))) => (format!("{} · {text}", clock(*time)), Tone::Good),
                Some((time, Err(error))) => (format!("{} · {error}", clock(*time)), Tone::Bad),
            },
        ));

        rows.push(heading("Config"));
        rows.push(row("File", (shown_path(&report.config_path), Tone::Plain)));
        rows.push(row(
            "At start",
            if report.config_from_file {
                ("read from the file".into(), Tone::Good)
            } else {
                ("no file; defaults".into(), Tone::Unknown)
            },
        ));
        rows.push(row(
            "Now",
            match checks.map(|c| &c.config) {
                None => checking(),
                Some(Ok(true)) => ("the file reads fine".into(), Tone::Good),
                Some(Ok(false)) => ("no file; defaults".into(), Tone::Unknown),
                Some(Err(error)) => (error.clone(), Tone::Bad),
            },
        ));

        rows.push(heading("Layout"));
        rows.push(row(
            "File",
            match &report.layout_path {
                Some(path) => (shown_path(path), Tone::Plain),
                None => ("no state directory (HOME unavailable)".into(), Tone::Bad),
            },
        ));
        rows.push(row(
            "At start",
            match &report.restore {
                None => ("not read".into(), Tone::Unknown),
                Some((_, Ok(true))) => ("restored the saved layout".into(), Tone::Good),
                Some((_, Ok(false))) if report.save_off => (
                    "not used: started for one agent or program".into(),
                    Tone::Unknown,
                ),
                Some((_, Ok(false))) => ("nothing saved yet".into(), Tone::Unknown),
                Some((_, Err(error))) => (error.clone(), Tone::Bad),
            },
        ));
        rows.push(row(
            "Latest save",
            if report.save_off {
                (
                    "saving is off this time; the file is left as it is".into(),
                    Tone::Unknown,
                )
            } else {
                match &report.save {
                    None => ("not saved yet".into(), Tone::Unknown),
                    Some((time, Ok(()))) => (format!("{} · saved", clock(*time)), Tone::Good),
                    Some((time, Err(error))) => (format!("{} · {error}", clock(*time)), Tone::Bad),
                }
            },
        ));

        rows.push(heading("Start"));
        rows.push(row(
            "Started from",
            (
                if report.startup.desktop {
                    "Finder or the Dock"
                } else {
                    "a terminal"
                }
                .into(),
                Tone::Plain,
            ),
        ));
        rows.push(row(
            "PATH",
            match report.startup.login_path {
                None => ("as the terminal had it".into(), Tone::Plain),
                Some(true) => ("taken from the login shell".into(), Tone::Good),
                Some(false) => (
                    "the login shell did not answer; kept as started".into(),
                    Tone::Bad,
                ),
            },
        ));
        rows.push(
            div()
                .pt(px(14.0))
                .text_size(ui.px(11.0))
                .text_color(self.fg(|t| t.agents_dim))
                .child(format!(
                    "Checked at {}. Read-only; nothing here changes paddock or its files.",
                    clock(report.checked)
                ))
                .into_any_element(),
        );
        rows
    }

    /// Shows `page`, as when Go to Agent picked it.
    pub fn show_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        if page == Page::Diagnostics {
            self.refresh_diagnostics(cx);
        }
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
            .min_h(px(26.0))
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

    /// A font setting: its font in a box that opens the list of installed ones below it.
    fn font_control(&self, field: &Field, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let key = field.key.clone();
        let mono = field.kind == Kind::MonoFont;
        let open = self.picker.as_ref().is_some_and(|picker| picker.key == key);
        let button = div()
            .id(ElementId::Name(format!("{key}-picker").into()))
            .min_h(px(26.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if open {
                self.fg(|t| t.focus)
            } else {
                self.fg(|t| t.agents_rule)
            })
            .bg(hsla(self.theme.terminal().background, 1.0))
            .cursor_pointer()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(self.font_label(&key)),
            )
            .child(div().text_color(self.fg(|t| t.agents_dim)).child("▾"))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| this.toggle_picker(&key, mono, window, cx)),
            );
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(button)
            .when(open, |control| {
                // Laid out right under the box, drawn above everything else.
                control.child(
                    div().h(px(0.0)).child(deferred(
                        anchored()
                            .snap_to_window_with_margin(px(8.0))
                            .child(self.picker_card(ui, cx)),
                    )),
                )
            })
    }

    /// The open font list: a search field, then the matches, each in its own font.
    fn picker_card(&self, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let picker = self.picker.as_ref().expect("an open list");
        let matches = self.picker_matches(cx);
        let current = self.font_label(&picker.key);
        let selected = picker.index.min(matches.len().saturating_sub(1));
        let row_height = ui.scale(26.0);
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let (check, text) = (self.fg(|t| t.agents_accent), self.fg(|t| t.agents_text));
        let list = if matches.is_empty() {
            let note = if picker.mono && self.fonts.mono.is_none() {
                "Finding monospace fonts…"
            } else if self.font_names(picker.mono).is_empty() {
                "Reading installed fonts…"
            } else {
                "No font matches."
            };
            div()
                .px(px(8.0))
                .py(px(6.0))
                .text_color(self.fg(|t| t.agents_dim))
                .child(note)
                .into_any_element()
        } else {
            let rows = matches.len().min(PICKER_ROWS);
            uniform_list(
                "font-list",
                matches.len(),
                cx.processor(move |_, range: std::ops::Range<usize>, _, cx| {
                    range
                        .filter_map(|index| Some((index, matches.get(index)?.clone())))
                        .map(|(index, name)| {
                            let family = if name == fonts::SYSTEM {
                                ".SystemUIFont".to_owned()
                            } else {
                                name.clone()
                            };
                            div()
                                .id(("font", index))
                                .w_full()
                                .h(px(row_height))
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .rounded(px(5.0))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .cursor_pointer()
                                .hover(move |style| style.bg(highlight.opacity(0.6)))
                                .when(index == selected, |row| row.bg(highlight))
                                .child(
                                    div()
                                        .w(px(14.0))
                                        .flex_shrink_0()
                                        .text_color(check)
                                        .child(if name == current { "✓" } else { "" }),
                                )
                                .child(div().text_color(text).font_family(family).child(name))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.pick(index, window, cx)
                                }))
                        })
                        .collect()
                }),
            )
            .track_scroll(&picker.scroll)
            .h(px(row_height * rows as f32))
            .into_any_element()
        };
        div()
            .id("font-picker")
            .key_context(menu::DIALOG)
            .occlude()
            .w(ui.px(340.0))
            .mt(px(4.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(6.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(self.fg(|t| t.focus))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                this.dismissed = this.picker.as_ref().map(|picker| picker.key.clone());
                this.close_picker(window, cx);
            }))
            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| this.move_pick(1, cx)))
            .on_action(cx.listener(|this, _: &menu::SelectPrevious, _, cx| this.move_pick(-1, cx)))
            .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                let index = this.picker.as_ref().map_or(0, |picker| picker.index);
                this.pick(index, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_picker(window, cx)),
            )
            .child(
                div()
                    .min_h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(self.fg(|t| t.agents_rule))
                    .bg(hsla(self.theme.terminal().background, 1.0))
                    .child(div().text_color(self.fg(|t| t.agents_dim)).child("⌕"))
                    .child(div().flex_1().min_w(px(0.0)).child(picker.input.clone())),
            )
            .child(list)
    }

    fn row(&self, field: &Field, cx: &mut Context<Self>) -> AnyElement {
        let ui = UiFont::get(cx);
        let key = field.key.clone();
        let control: AnyElement = match field.kind {
            Kind::MonoFont | Kind::UiFont => self.font_control(field, &ui, cx).into_any_element(),
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
                        .w(ui.px(20.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(unit)
                }))
                .into_any_element(),
        };
        let mut tags = div().flex().items_center().gap(px(6.0)).flex_shrink_0();
        if self.draft.custom(&key) {
            tags = tags.child(
                div()
                    .text_size(ui.px(11.0))
                    .text_color(self.fg(|t| t.agents_accent))
                    .child("custom"),
            );
        }
        if field.restart && self.draft.changed(&key) {
            tags = tags.child(
                div()
                    .text_size(ui.px(11.0))
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
                .text_size(ui.px(11.0))
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
                        label.font_family("Menlo").text_size(ui.px(12.0))
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
        let ui = UiFont::get(cx);
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
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_page(page, cx)));
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
        if self.page == Page::Diagnostics {
            list = list.children(self.diagnostics_page(&ui));
        }

        let mut footer = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pt(px(10.0))
            .border_t_1()
            .border_color(self.fg(|t| t.agents_rule));
        footer = footer.child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(ui.px(12.0))
                .children(self.message.as_ref().map(|(text, problem)| {
                    div()
                        .text_color(if *problem {
                            self.fg(|t| t.agents_red)
                        } else {
                            self.fg(|t| t.agents_green)
                        })
                        .child(text.clone())
                })),
        );
        if self.page == Page::Diagnostics {
            footer =
                footer.child(self.button("refresh", "Refresh").on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.refresh_diagnostics(cx)),
                ));
        } else if self.conflict {
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

        ui.apply(div())
            .id("settings")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::save))
            // After a font list's own button has seen the press.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dismissed = None),
            )
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
            .text_size(ui.px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .child(
                div()
                    .text_size(ui.px(11.0))
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

#[derive(Clone, Copy)]
enum Tone {
    Good,
    Bad,
    Plain,
    Unknown,
}
