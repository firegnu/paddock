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
    FontWeight, Hsla, MouseButton, Pixels, PromptLevel, Render, ScrollStrategy, SharedString,
    Stateful, Subscription, Task, TextRun, UniformListScrollHandle, Window, anchored, deferred,
    div, prelude::*, px, relative, uniform_list,
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
        _ => None,
    }
}

/// General's settings by heading, in display order.
const GENERAL: [(&str, &[&str]); 4] = [
    ("Interface", &["ui_font", "ui_font_size", "sidebar_width"]),
    (
        "Terminal",
        &["font", "font_size", "line_height", "font_fallbacks"],
    ),
    ("Mascot", &["mascot_enabled", "mascot"]),
    ("Agents", &["refresh_ms"]),
];

/// A setting's label in its row, shorter where its heading already says what it is for.
fn row_label(field: &Field) -> &str {
    match field.key.as_str() {
        "ui_font" | "font" => "Font",
        "ui_font_size" | "font_size" => "Size",
        _ => &field.label,
    }
}

/// A faint line under a setting's label.
fn note(key: &str) -> Option<&'static str> {
    (key == "font_fallbacks").then_some("For characters the font lacks, in order")
}

/// The family for config values: paths, colour values, and monospace lists.
const MONO: &str = "Menlo";

/// How wide `text` is set in `family` at `size`.
fn text_width(text: &str, family: SharedString, size: Pixels, window: &Window) -> Pixels {
    if text.is_empty() {
        return px(0.0);
    }
    let run = TextRun {
        len: text.len(),
        font: gpui::font(family),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.to_owned().into(), size, &[run], None)
        .width
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

    /// A setting's field colours: colour values quieter than the rest.
    fn colors_for(&self, field: &Field) -> text_input::Colors {
        let colors = self.input_colors();
        if field.kind == Kind::Color {
            text_input::Colors {
                text: self.fg(|t| t.agents_dim),
                ..colors
            }
        } else {
            colors
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
        for field in typed {
            let colors = self.colors_for(&field);
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
                        let fields = self.draft.fields().to_vec();
                        for field in &fields {
                            let colors = self.colors_for(field);
                            if let Some(input) = self.inputs.get(&field.key) {
                                input.update(cx, |input, cx| input.set_colors(colors, cx));
                            }
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

    /// One Diagnostics row: a label and its value, coloured good, bad or unknown.
    fn diagnostic(
        &self,
        label: &'static str,
        value: impl Into<SharedString>,
        tone: Tone,
        ui: &UiFont,
    ) -> Div {
        let color = match tone {
            Tone::Good => self.fg(|t| t.agents_green),
            Tone::Bad => self.fg(|t| t.agents_red),
            Tone::Plain => self.fg(|t| t.agents_text),
            Tone::Unknown => self.fg(|t| t.agents_dim),
        };
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(ui.px(38.0))
            .px(px(16.0))
            .py(px(7.0))
            .child(
                div()
                    .w(ui.px(130.0))
                    .flex_shrink_0()
                    .text_color(self.fg(|t| t.agents_dim))
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
            let row = self.diagnostic(
                "Diagnostics",
                "Unavailable: the main window is gone.",
                Tone::Bad,
                ui,
            );
            return self.cards(vec![("", vec![row])], true, ui);
        };
        let checks = self.checks.as_ref();
        let checking = || ("checking…".to_owned(), Tone::Unknown);
        let path = |found: &Result<std::path::PathBuf, String>| match found {
            Ok(path) => (path.display().to_string(), Tone::Good),
            Err(error) => (error.clone(), Tone::Bad),
        };
        let outcome = |result: &Result<String, String>| match result {
            Ok(text) => (text.clone(), Tone::Good),
            Err(error) => (error.clone(), Tone::Bad),
        };
        let row = |label, (value, tone): (String, Tone)| self.diagnostic(label, value, tone, ui);
        let commands = vec![
            row(
                "corral",
                checks.map_or_else(checking, |c| path(&c.corral.path)),
            ),
            row(
                "corral version",
                checks.map_or_else(checking, |c| outcome(&c.corral.version)),
            ),
            row("git", checks.map_or_else(checking, |c| path(&c.git.path))),
            row(
                "git version",
                checks.map_or_else(checking, |c| outcome(&c.git.version)),
            ),
            row("shell", checks.map_or_else(checking, |c| path(&c.shell))),
        ];

        let agents = vec![row(
            "Latest corral ls",
            match &report.agents {
                None => ("not read yet".into(), Tone::Unknown),
                Some((time, Ok(text))) => (format!("{} · {text}", clock(*time)), Tone::Good),
                Some((time, Err(error))) => (format!("{} · {error}", clock(*time)), Tone::Bad),
            },
        )];

        let config = vec![
            row("File", (shown_path(&report.config_path), Tone::Plain)),
            row(
                "At start",
                if report.config_from_file {
                    ("read from the file".into(), Tone::Good)
                } else {
                    ("no file; defaults".into(), Tone::Unknown)
                },
            ),
            row(
                "Now",
                match checks.map(|c| &c.config) {
                    None => checking(),
                    Some(Ok(true)) => ("the file reads fine".into(), Tone::Good),
                    Some(Ok(false)) => ("no file; defaults".into(), Tone::Unknown),
                    Some(Err(error)) => (error.clone(), Tone::Bad),
                },
            ),
        ];

        let layout = vec![
            row(
                "File",
                match &report.layout_path {
                    Some(path) => (shown_path(path), Tone::Plain),
                    None => ("no state directory (HOME unavailable)".into(), Tone::Bad),
                },
            ),
            row(
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
            ),
            row(
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
                        Some((time, Err(error))) => {
                            (format!("{} · {error}", clock(*time)), Tone::Bad)
                        }
                    }
                },
            ),
        ];

        let start = vec![
            row(
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
            ),
            row(
                "PATH",
                match report.startup.login_path {
                    None => ("as the terminal had it".into(), Tone::Plain),
                    Some(true) => ("taken from the login shell".into(), Tone::Good),
                    Some(false) => (
                        "the login shell did not answer; kept as started".into(),
                        Tone::Bad,
                    ),
                },
            ),
        ];

        let mut items = self.cards(
            vec![
                ("Commands", commands),
                ("Agents", agents),
                ("Config", config),
                ("Layout", layout),
                ("Start", start),
            ],
            true,
            ui,
        );
        items.push(
            div()
                .pt(px(14.0))
                .px(px(4.0))
                .text_size(ui.px(11.0))
                .text_color(self.fg(|t| t.agents_dimmer))
                .child(format!(
                    "Checked at {}. Read-only; nothing here changes paddock or its files.",
                    clock(report.checked)
                ))
                .into_any_element(),
        );
        items
    }

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

    /// The theme's rule colour at `alpha`: lines between rows, around cards and fields.
    fn rule(&self, alpha: f32) -> Hsla {
        hsla(self.theme.fg(|t| t.agents_rule), alpha)
    }

    /// A heading over a card: small, upper case, faint.
    fn heading(&self, text: &str, first: bool, ui: &UiFont) -> Div {
        div()
            .pt(px(if first { 6.0 } else { 18.0 }))
            .pb(px(7.0))
            .px(px(4.0))
            .text_size(ui.px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.fg(|t| t.agents_dimmer))
            .child(text.to_uppercase())
    }

    /// Rows in a rounded card, a faint line between them.
    fn card(&self, rows: Vec<Div>) -> Div {
        let ground = hsla(self.theme.bg(|t| t.agents_bg), 1.0)
            .blend(hsla(self.theme.fg(|t| t.agents_text), 0.025));
        let line = self.rule(0.55);
        div()
            .flex()
            .flex_col()
            .rounded(px(10.0))
            .border_1()
            .border_color(line)
            .bg(ground)
            .children(
                rows.into_iter().enumerate().map(|(index, row)| {
                    row.when(index > 0, |row| row.border_t_1().border_color(line))
                }),
            )
    }

    /// A card per group of rows, each under its heading; `first` when they open the page.
    fn cards(&self, groups: Vec<(&str, Vec<Div>)>, first: bool, ui: &UiFont) -> Vec<AnyElement> {
        let mut items = Vec::new();
        for (index, (heading, rows)) in groups.into_iter().enumerate() {
            let first = first && index == 0;
            items.push(if heading.is_empty() {
                div()
                    .pt(px(if first { 6.0 } else { 18.0 }))
                    .into_any_element()
            } else {
                self.heading(heading, first, ui).into_any_element()
            });
            items.push(self.card(rows).into_any_element());
        }
        items
    }

    /// A small tinted label after a setting's name.
    fn tag(&self, text: &'static str, color: Hsla, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .px(px(6.0))
            .rounded(px(4.0))
            .text_size(ui.px(11.0))
            .text_color(color)
            .bg(color.opacity(0.12))
            .child(text)
    }

    /// A quiet button's shape: its text in a rounded box, no ground.
    fn button_shape(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        ui: &UiFont,
    ) -> Stateful<Div> {
        div()
            .id(id.into())
            .flex_shrink_0()
            .h(ui.px(28.0))
            .px(px(13.0))
            .flex()
            .items_center()
            .rounded(px(7.0))
            .child(label.into())
    }

    /// A quiet button: text, a faint ground on hover.
    fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        ui: &UiFont,
    ) -> Stateful<Div> {
        let hover = hsla(self.theme.fg(|t| t.agents_text), 0.07);
        self.button_shape(id, label, ui)
            .text_color(self.fg(|t| t.agents_branch))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
    }

    /// A row of choices, the drafted one raised.
    fn segments(
        &self,
        key: &str,
        options: &[(&'static str, &'static str)],
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let current = self.draft.value(key);
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let (text, dim) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_dim));
        let mut row = div()
            .flex()
            .flex_shrink_0()
            .p(px(3.0))
            .rounded(px(8.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .border_1()
            .border_color(self.rule(0.75));
        for &(value, label) in options {
            let on = current == value;
            let key = key.to_owned();
            let segment = div()
                .id(ElementId::Name(format!("{key}-{value}").into()))
                .h(ui.px(24.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .child(label)
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.set(&key, value, cx)),
                );
            row = row.child(if on {
                segment.bg(highlight).text_color(text)
            } else {
                segment
                    .text_color(dim)
                    .hover(move |style| style.text_color(text))
            });
        }
        row
    }

    /// A sliding switch: the accent when on.
    fn switch(&self, key: &str, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let on = self.draft.value(key) == "true";
        let key = key.to_owned();
        let track = if on {
            self.fg(|t| t.agents_accent)
        } else {
            self.fg(|t| t.agents_rule)
        };
        div()
            .id(ElementId::Name(format!("{key}-switch").into()))
            .flex_shrink_0()
            .w(ui.px(34.0))
            .h(ui.px(20.0))
            .p(ui.px(2.0))
            .rounded(ui.px(10.0))
            .bg(track)
            .flex()
            .items_center()
            .when(on, |track| track.justify_end())
            .cursor_pointer()
            .child(
                div()
                    .size(ui.px(16.0))
                    .rounded(ui.px(8.0))
                    .bg(self.fg(|t| t.agents_text))
                    .shadow_sm(),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let value = if on { "false" } else { "true" };
                this.set(&key, value, cx)
            }))
    }

    /// A setting's text field in its box. Numbers and colour values sit against the right edge; a
    /// colour's box shows only when its row is hovered or it is being typed in.
    fn field_box(
        &self,
        key: &str,
        boxed: Boxed,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let (width, mono, right) = match boxed {
            Boxed::Number => (ui.px(64.0), None, true),
            Boxed::Text => (px(220.0), None, false),
            Boxed::List => (px(220.0), Some(11.5), false),
            Boxed::Color(_) => (ui.px(120.0), Some(11.5), true),
        };
        let input = self.inputs.get(key).cloned();
        let focused = input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let ground = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let line = self.rule(0.85);
        let mut field = div()
            .id(ElementId::Name(format!("{key}-field").into()))
            .flex_shrink_0()
            .w(width)
            .h(ui.px(28.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded(px(7.0))
            .border_1()
            .overflow_hidden()
            .when(right, |field| field.justify_end())
            .when_some(mono, |field, size| {
                field.font_family(MONO).text_size(ui.px(size))
            });
        field = if focused {
            field
                .bg(ground)
                .border_color(self.fg(|t| t.agents_accent).opacity(0.7))
        } else if let Boxed::Color(group) = boxed {
            field
                .border_color(gpui::transparent_black())
                .group_hover(group.clone(), move |style| {
                    style.bg(ground).border_color(line)
                })
        } else {
            field.bg(ground).border_color(line)
        };
        let Some(input) = input else {
            return field;
        };
        let text = input.read(cx).text().to_owned();
        // The field is as wide as its text when it sits right, so the text ends at the edge.
        let inner = if right && !text.is_empty() {
            let family: SharedString = match mono {
                Some(_) => MONO.into(),
                None => ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into()),
            };
            let size = ui.px(mono.unwrap_or(13.0));
            div()
                .flex_shrink_0()
                .w(text_width(&text, family, size, window) + px(2.0))
        } else {
            div().flex_1().min_w(px(0.0)).overflow_hidden()
        };
        let handle = input.read(cx).focus_handle(cx);
        field
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&handle, cx)
            })
            .child(inner.child(input))
    }

    /// A number setting: a narrow box, its unit after it.
    fn number_control(
        &self,
        key: &str,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(6.0))
            .child(self.field_box(key, Boxed::Number, window, ui, cx))
            .child(
                div()
                    .w(ui.px(20.0))
                    .whitespace_nowrap()
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .children(unit(key)),
            )
    }

    /// A colour: its value, then a swatch of it as drafted.
    fn color_control(
        &self,
        field: &Field,
        drafted: Option<&Theme>,
        group: &SharedString,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let swatch = drafted
            .and_then(|theme| theme.color(field.color()?))
            .map(|rgb| hsla(rgb, 1.0))
            .unwrap_or(gpui::transparent_black());
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.0))
            .child(self.field_box(&field.key, Boxed::Color(group), window, ui, cx))
            .child(
                div()
                    .size(ui.px(20.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(hsla(self.theme.fg(|t| t.agents_text), 0.08))
                    .bg(swatch),
            )
    }

    /// A font setting: its font in a box that opens the list of installed ones below it.
    fn font_control(&self, field: &Field, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let key = field.key.clone();
        let mono = field.kind == Kind::MonoFont;
        let open = self.picker.as_ref().is_some_and(|picker| picker.key == key);
        let value = self.draft.value(&key);
        let button = div()
            .id(ElementId::Name(format!("{key}-picker").into()))
            .h(ui.px(28.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(if open {
                self.fg(|t| t.agents_accent).opacity(0.7)
            } else {
                self.rule(0.85)
            })
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .cursor_pointer()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    // The font shown in itself.
                    .when(!value.is_empty(), |name| name.font_family(value.clone()))
                    .when(mono, |name| name.text_size(ui.px(12.0)))
                    .child(self.font_label(&key)),
            )
            .child(
                div()
                    .text_size(ui.px(11.0))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child("▾"),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| this.toggle_picker(&key, mono, window, cx)),
            );
        div()
            .w(px(220.0))
            .flex_shrink_0()
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
                                .rounded(px(6.0))
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
            .rounded(px(10.0))
            .border_1()
            .border_color(self.rule(1.0))
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
                    .h(ui.px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(self.rule(0.85))
                    .bg(hsla(self.theme.terminal().background, 1.0))
                    .child(div().text_color(self.fg(|t| t.agents_dim)).child("⌕"))
                    .child(div().flex_1().min_w(px(0.0)).child(picker.input.clone())),
            )
            .child(list)
    }

    /// One setting in its card: the label on the left with its tags and note, Default on hover,
    /// the control on the right.
    fn row(
        &self,
        field: &Field,
        drafted: Option<&Theme>,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let key = field.key.clone();
        let group = SharedString::from(format!("row-{key}"));
        let color = field.kind == Kind::Color;
        let control: AnyElement = match field.kind {
            Kind::MonoFont | Kind::UiFont => self.font_control(field, ui, cx).into_any_element(),
            Kind::Bool => self.switch(&key, ui, cx).into_any_element(),
            Kind::Pet => self
                .segments(
                    &key,
                    &[("clawd", "Clawd"), ("cat", "Cat"), ("capybara", "Capybara")],
                    ui,
                    cx,
                )
                .into_any_element(),
            Kind::Theme => self
                .segments(
                    &key,
                    &[("dune", "Dune"), ("tide", "Tide"), ("lagoon", "Lagoon")],
                    ui,
                    cx,
                )
                .into_any_element(),
            Kind::Color => self
                .color_control(field, drafted, &group, window, ui, cx)
                .into_any_element(),
            Kind::Integer | Kind::Number => {
                self.number_control(&key, window, ui, cx).into_any_element()
            }
            Kind::Text => self
                .field_box(&key, Boxed::Text, window, ui, cx)
                .into_any_element(),
            Kind::List => self
                .field_box(&key, Boxed::List, window, ui, cx)
                .into_any_element(),
        };
        let mut title = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .min_w(px(0.0))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .when(color, |label| {
                        label.font_family(MONO).text_size(ui.px(12.0))
                    })
                    .child(row_label(field).to_owned()),
            );
        if self.draft.custom(&key) {
            title = title.child(self.tag("custom", self.fg(|t| t.agents_accent), ui));
        }
        if field.restart && self.draft.changed(&key) {
            title = title.child(self.tag("Restart required", self.fg(|t| t.agents_yellow), ui));
        }
        let label = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(title)
            .children(note(&key).map(|note| {
                div()
                    .mt(px(1.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(11.5))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child(note)
            }));
        let (dim, text) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_text));
        let reset_key = key.clone();
        let default = div()
            .id(ElementId::Name(format!("{key}-default").into()))
            .flex_shrink_0()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(5.0))
            .text_size(ui.px(11.0))
            .text_color(dim)
            .cursor_pointer()
            .invisible()
            .group_hover(group.clone(), |style| style.visible())
            .hover(move |style| style.text_color(text))
            .child("Default")
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.reset(&reset_key, cx)));
        // The pet waits while the mascot is off.
        let resting = field.kind == Kind::Pet && self.draft.value("mascot_enabled") != "true";
        div()
            .group(group)
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(ui.px(if color { 38.0 } else { 44.0 }))
            .px(px(16.0))
            .py(px(6.0))
            .when(resting, |row| row.opacity(0.45))
            .child(label)
            .child(default)
            .child(control)
    }

    /// The rows for the settings with `keys`, in that order.
    fn rows_for(
        &self,
        keys: &[&str],
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<Div> {
        let mut rows = Vec::new();
        for key in keys {
            if let Some(field) = self.draft.fields().iter().find(|f| f.key == *key) {
                let field = field.clone();
                rows.push(self.row(&field, None, window, ui, cx));
            }
        }
        rows
    }

    /// Rows for `fields` in a card per group, as they are grouped.
    fn grouped(
        &self,
        fields: &[Field],
        drafted: Option<&Theme>,
        first: bool,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut groups: Vec<(&str, Vec<Div>)> = Vec::new();
        for field in fields {
            let row = self.row(field, drafted, window, ui, cx);
            match groups.last_mut() {
                Some((group, rows)) if *group == field.group => rows.push(row),
                _ => groups.push((field.group, vec![row])),
            }
        }
        self.cards(groups, first, ui)
    }

    /// The three themes as small pictures of the window in their own colours.
    fn theme_picker(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let current = self.draft.theme_name();
        let accent = self.fg(|t| t.agents_accent);
        let mut choices = div().flex().gap(px(12.0));
        for (value, label) in [("dune", "Dune"), ("tide", "Tide"), ("lagoon", "Lagoon")] {
            let config = Config {
                theme: Some(value.into()),
                ..Config::default()
            };
            let Ok(theme) = Theme::from_config(&config) else {
                continue;
            };
            let on = current == value;
            let color = |pick: Pick| hsla(theme.fg(pick), 1.0);
            let (text, own_accent) = (color(|t| t.agents_text), color(|t| t.agents_accent));
            let side = hsla(theme.bg(|t| t.agents_bg), 1.0).blend(text.opacity(0.05));
            let bar = |color: Hsla, width: f32, height: f32| {
                div()
                    .h(px(height))
                    .w(relative(width))
                    .rounded(px(height / 2.0))
                    .bg(color)
            };
            let picture = div()
                .h(ui.px(74.0))
                .flex()
                .rounded(px(7.0))
                .overflow_hidden()
                .bg(hsla(theme.terminal().background, 1.0))
                .child(
                    div()
                        .w(relative(0.34))
                        .h_full()
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
                        .px(px(7.0))
                        .py(px(9.0))
                        .bg(side)
                        .child(bar(own_accent, 0.8, 5.0))
                        .child(bar(color(|t| t.agent_blocked), 0.6, 5.0))
                        .child(bar(color(|t| t.agent_working), 0.7, 5.0)),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
                        .px(px(8.0))
                        .py(px(9.0))
                        .child(bar(text.opacity(0.7), 0.7, 4.0))
                        .child(bar(text.opacity(0.4), 0.45, 4.0))
                        .child(bar(own_accent.opacity(0.8), 0.55, 4.0)),
                );
            let rule = self.rule(1.0);
            let choice = div()
                .id(ElementId::Name(format!("theme-{value}").into()))
                .flex_1()
                .min_w(px(0.0))
                .p(px(6.0))
                .rounded(px(10.0))
                .border_1()
                .cursor_pointer()
                .child(picture)
                .child(
                    div()
                        .pt(px(7.0))
                        .px(px(3.0))
                        .text_size(ui.px(12.5))
                        .text_color(if on {
                            self.fg(|t| t.agents_text)
                        } else {
                            hsla(self.theme.fg(|t| t.agents_text), 0.7)
                        })
                        .child(label),
                )
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.set("theme", value, cx)),
                );
            choices = choices.child(if on {
                choice.border_color(accent)
            } else {
                choice
                    .border_color(self.rule(0.6))
                    .hover(move |style| style.border_color(rule))
            });
        }
        choices
    }

    fn page_items(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match self.page {
            Page::General => {
                let mut groups = Vec::new();
                for (heading, keys) in GENERAL {
                    groups.push((heading, self.rows_for(keys, window, ui, cx)));
                }
                self.cards(groups, true, ui)
            }
            Page::Colors => {
                let colors: Vec<Field> = self
                    .draft
                    .fields()
                    .iter()
                    .filter(|f| f.page == Page::Colors && f.kind != Kind::Theme)
                    .cloned()
                    .collect();
                let drafted = self.draft.theme();
                let mut items = vec![
                    self.heading("Theme", true, ui).into_any_element(),
                    self.theme_picker(ui, cx).into_any_element(),
                ];
                items.extend(self.grouped(&colors, drafted.as_ref(), false, window, ui, cx));
                items
            }
            Page::Advanced => {
                let fields: Vec<Field> = self
                    .draft
                    .fields()
                    .iter()
                    .filter(|f| f.page == Page::Advanced)
                    .cloned()
                    .collect();
                self.grouped(&fields, None, true, window, ui, cx)
            }
            Page::Diagnostics => self.diagnostics_page(ui),
        }
    }

    /// The pages, each with its glyph; the one showing raised.
    fn nav(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let text = self.fg(|t| t.agents_text);
        let mut nav = div()
            .w(px(184.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(10.0))
            .pt(px(12.0));
        for page in Page::ALL {
            let on = page == self.page;
            let glyph = match page {
                Page::General => "◐",
                Page::Colors => "◑",
                Page::Advanced => "⋯",
                Page::Diagnostics => "ⓘ",
            };
            let item = div()
                .id(page.label())
                .h(ui.px(32.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(7.0))
                .cursor_pointer()
                .child(
                    div()
                        .w(ui.px(16.0))
                        .flex()
                        .justify_center()
                        .text_color(if on {
                            self.fg(|t| t.agents_accent)
                        } else {
                            self.fg(|t| t.agents_dimmer)
                        })
                        .child(glyph),
                )
                .child(page.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_page(page, cx)));
            nav = nav.child(if on {
                item.bg(highlight)
                    .text_color(text)
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                item.text_color(text.opacity(0.7))
                    .hover(move |style| style.bg(highlight.opacity(0.6)))
            });
        }
        nav
    }

    /// The config file's path, what Save last said, and the buttons.
    fn footer(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let mut footer = div()
            .flex_shrink_0()
            .h(ui.px(52.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .pl(px(20.0))
            .pr(px(16.0))
            .border_t_1()
            .border_color(self.rule(0.75))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_family(MONO)
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child(shown_path(&self.path)),
            )
            .child(div().flex_1())
            .children(self.message.as_ref().map(|(text, problem)| {
                div()
                    .min_w(px(0.0))
                    .mr(px(6.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(12.0))
                    .text_color(if *problem {
                        self.fg(|t| t.agents_red)
                    } else {
                        self.fg(|t| t.agents_green)
                    })
                    .child(text.clone())
            }));
        if self.page == Page::Diagnostics {
            footer =
                footer.child(self.button("refresh", "Refresh", ui).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.refresh_diagnostics(cx)),
                ));
        } else if self.conflict {
            footer = footer
                .child(self.button("keep", "Keep my edits", ui).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.resolve_conflict(true, cx)),
                ))
                .child(self.button("discard", "Discard my edits", ui).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.resolve_conflict(false, cx)),
                ));
        } else {
            let revert = if self.draft.edited() {
                self.button("revert", "Revert", ui)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.revert(cx)))
            } else {
                // Nothing to revert: shown, but not clickable.
                // (GPUI allows one hover style per element, so not `button` with its hover undone.)
                self.button_shape("revert", "Revert", ui)
                    .text_color(self.fg(|t| t.agents_dimmer))
            };
            let accent = self.fg(|t| t.agents_accent);
            footer = footer.child(revert).child(
                div()
                    .id("save")
                    .flex_shrink_0()
                    .h(ui.px(28.0))
                    .px(px(13.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .rounded(px(7.0))
                    .bg(accent)
                    .text_color(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(move |style| style.bg(accent.opacity(0.88)))
                    .child("Save")
                    .child(
                        div()
                            .opacity(0.6)
                            .font_weight(FontWeight::MEDIUM)
                            .child("⌘S"),
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.save(&menu::SaveSettings, window, cx)
                    })),
            );
        }
        footer
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let items = self.page_items(window, &ui, cx);
        let content = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_shrink_0()
                    .h(ui.px(46.0))
                    .flex()
                    .items_center()
                    .px(px(28.0))
                    .text_size(ui.px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.page.label()),
            )
            .child(
                div()
                    .id("settings-list")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .px(px(28.0))
                    .pt(px(2.0))
                    .pb(px(20.0))
                    .children(items),
            );

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
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .child(self.nav(&ui, cx))
                    .child(div().w(px(1.0)).h_full().bg(self.rule(0.75)))
                    .child(content),
            )
            .child(self.footer(&ui, cx))
    }
}

/// How a text field's box is drawn.
enum Boxed<'a> {
    Number,
    Text,
    List,
    /// A colour value, its box shown when the row named here is hovered.
    Color(&'a SharedString),
}

#[derive(Clone, Copy)]
enum Tone {
    Good,
    Bad,
    Plain,
    Unknown,
}
