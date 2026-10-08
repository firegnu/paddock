//! The Settings window: pages on the left, the page's settings in cards on the right, and a bar
//! that rises at the bottom while the draft differs from the file (Revert, Save) or Save has
//! something to say. Closing it with unsaved edits asks Save / Don't Save / Cancel. The rules are
//! in `settings.rs`; this file draws them and keeps one text field per typed setting.
use crate::{
    config::Config,
    diagnostics::{self, Checks, Report, clock},
    fonts::{self, Installed, UiFont},
    footer_icon::{self, Icon},
    menu,
    preset::Preset,
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
use std::{collections::HashMap, path::PathBuf, rc::Rc, time::Duration};

/// The key context of the Settings window: ⌘S saves, ⌘W closes.
pub const CONTEXT: &str = "PaddockSettings";

/// The top row at the base interface size: the traffic lights over the pages, the page's title
/// beside them.
pub const TITLE_BAR: f32 = 46.0;

/// How long a message that reports no problem stays in the bar.
const MESSAGE_TIME: Duration = Duration::from_secs(4);

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
    /// What Save last said, and whether it reports a problem; shown in the bar.
    message: Option<(String, bool)>,
    /// Counts messages, so one timing out leaves a newer one alone.
    said: usize,
    conflict: bool,
    /// The Save / Don't Save / Cancel question is showing.
    asking: bool,
    /// Appearance lists every colour, not only those set over the theme.
    all_colors: bool,
    /// The field a fallback font is being typed into.
    adding: Option<Entity<TextInput>>,
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

/// How − and + step a number setting: by how much, and the least they go to.
fn steps(key: &str) -> (f64, f64) {
    match key {
        "ui_font_size" | "font_size" => (1.0, 6.0),
        "sidebar_width" => (10.0, 10.0),
        "line_height" => (0.1, 0.5),
        "refresh_ms" => (250.0, 250.0),
        _ => (1.0, 1.0),
    }
}

/// `text` stepped `by` steps; text that is not a number steps from `default`.
fn stepped(text: &str, default: &str, by: f64, (step, least): (f64, f64)) -> String {
    let current = text
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .or_else(|| default.parse().ok())
        .unwrap_or(least);
    let next = (((current + by * step) * 100.0).round() / 100.0).max(least);
    if next.fract() == 0.0 {
        format!("{}", next as i64)
    } else {
        format!("{next}")
    }
}

/// General's settings by heading, in display order.
const GENERAL: [(&str, &[&str]); 3] = [
    ("Interface", &["ui_font", "ui_font_size", "sidebar_width"]),
    (
        "Terminal",
        &["font", "font_size", "line_height", "font_fallbacks"],
    ),
    ("Mascot", &["mascot_enabled", "mascot"]),
];

/// Agents' settings by heading.
const AGENTS: [(&str, &[&str]); 1] = [("corral", &["refresh_ms", "corral"])];

const PETS: [(&str, &str); 3] = [("clawd", "Clawd"), ("cat", "Cat"), ("capybara", "Capybara")];

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
    match key {
        "font_fallbacks" => Some("For characters the font lacks, in order"),
        "corral" => Some("--corral overrides it for one run"),
        _ => None,
    }
}

/// What a setting's Reset link says, given its default as the page shows it.
fn reset_label(field: &Field, default: &str) -> String {
    let default = match field.kind {
        Kind::Color => return "Reset".into(),
        Kind::List => "default",
        Kind::UiFont => "System",
        Kind::Bool if default == "true" => "on",
        Kind::Bool => "off",
        Kind::Pet => PETS
            .iter()
            .find(|(value, _)| *value == default)
            .map_or(default, |(_, label)| label),
        _ => default,
    };
    format!("Reset to {default}")
}

/// The fallback fonts in a list's text.
fn items(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The bar's count of unsaved settings, and which they are, the first few by name.
fn unsaved_summary(labels: &[String]) -> (String, String) {
    let count = labels.len();
    let head = format!(
        "{count} unsaved change{}",
        if count == 1 { "" } else { "s" }
    );
    let mut names = labels[..count.min(3)].join(", ");
    if count > 3 {
        names.push_str(&format!(" and {} more", count - 3));
    }
    (head, names)
}

/// How many theme cards sit side by side in `width`: up to four, each at least `least` wide.
fn columns(width: f32, least: f32, gap: f32) -> u16 {
    (((width + gap) / (least + gap)).floor() as u16).clamp(1, 4)
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
            said: 0,
            conflict: false,
            asking: false,
            all_colors: false,
            adding: None,
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
                text: self.fg(|t| t.agents_branch),
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
                matches!(
                    f.kind,
                    Kind::Integer | Kind::Number | Kind::Text | Kind::Color
                )
            })
            .cloned()
            .collect();
        for field in typed {
            let colors = self.colors_for(&field);
            let text = self.draft.value(&field.key);
            let placeholder = match field.kind {
                Kind::Color => "#rrggbb, an ANSI name, or default",
                _ => "",
            };
            let input = cx.new(|cx| TextInput::new(text, placeholder, colors, cx));
            let key = field.key.clone();
            let color = field.kind == Kind::Color;
            self._subscriptions.push(cx.subscribe(
                &input,
                move |this, input, _: &text_input::Changed, cx| {
                    let text = input.read(cx).text().to_owned();
                    // An emptied colour follows the theme again.
                    if color && text.trim().is_empty() {
                        this.draft.reset(&key);
                    } else {
                        this.draft.set(&key, &text);
                    }
                    this.message = None;
                    cx.notify();
                },
            ));
            self.inputs.insert(field.key, input);
        }
    }

    /// Puts the drafted values back into the fields after a Reset, a theme or a reload.
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

    /// − or + on a number setting.
    fn step(&mut self, key: &str, by: f64, cx: &mut Context<Self>) {
        let value = stepped(
            &self.draft.value(key),
            &self.draft.default_shown(key),
            by,
            steps(key),
        );
        self.set(key, &value, cx);
    }

    /// Opens a field after the fallback fonts for another one's name.
    fn start_adding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let colors = self.input_colors();
        let input = cx.new(|cx| TextInput::new("", "Font name", colors, cx));
        window.focus(&input.read(cx).focus_handle(cx), cx);
        self.adding = Some(input);
        cx.notify();
    }

    /// Closes the field for another fallback font, adding the name typed when `keep`.
    fn finish_adding(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.adding.take() else {
            return;
        };
        let name = input.read(cx).text().trim().to_owned();
        let mut list = items(&self.draft.value("font_fallbacks"));
        if keep && !name.is_empty() && !list.contains(&name) {
            list.push(name);
            self.set("font_fallbacks", &list.join(", "), cx);
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn remove_fallback(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut list = items(&self.draft.value("font_fallbacks"));
        if index < list.len() {
            list.remove(index);
            self.set("font_fallbacks", &list.join(", "), cx);
        }
    }

    /// Shows `text` in the bar; one that reports no problem goes after a few seconds.
    fn say(&mut self, text: impl Into<String>, problem: bool, cx: &mut Context<Self>) {
        self.message = Some((text.into(), problem));
        self.said += 1;
        if !problem {
            let said = self.said;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(MESSAGE_TIME).await;
                this.update(cx, |this, cx| {
                    if this.said == said {
                        this.message = None;
                        cx.notify();
                    }
                })
            })
            .detach();
        }
        cx.notify();
    }

    pub fn save(&mut self, _: &menu::SaveSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.save_draft(cx);
    }

    /// Saves the draft; false when nothing was written because of a problem, now shown below.
    fn save_draft(&mut self, cx: &mut Context<Self>) -> bool {
        let disk = match read(&self.path) {
            Ok(disk) => disk,
            Err(error) => {
                self.say(format!("Not saved: {error}"), true, cx);
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
                    self.say(format!("Not saved: {error}"), true, cx);
                } else {
                    self.draft = Draft::new(Some(text)).expect("just validated");
                    self.refresh_inputs(cx);
                    if restart.is_empty() {
                        self.say("Saved.", false, cx);
                    } else {
                        self.say(
                            format!("Saved. Restart paddock for: {}.", restart.join(", ")),
                            false,
                            cx,
                        );
                    }
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
                self.say("Nothing to save.", false, cx);
                saved = true;
            }
            Ok(Err(Conflict)) => {
                self.conflict = true;
                self.say(
                    "The config file changed on disk; nothing was saved.",
                    true,
                    cx,
                );
            }
            Err(error) => self.say(format!("Not saved: {error:#}"), true, cx),
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
        self.message = None;
        match result {
            Ok(()) if keep => self.say(
                "Your edits now sit on the file as it is; Save again.",
                false,
                cx,
            ),
            Ok(()) => {}
            Err(error) => self.say(
                format!("The config file has a problem: {error:#}"),
                true,
                cx,
            ),
        }
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

    /// One Diagnostics row: a dot for how it went, a label, and its value.
    fn diagnostic(
        &self,
        label: &'static str,
        value: impl Into<SharedString>,
        tone: Tone,
        ui: &UiFont,
    ) -> Div {
        let (dot, color) = match tone {
            Tone::Good => (self.fg(|t| t.agents_green), self.fg(|t| t.agents_text)),
            Tone::Bad => (self.fg(|t| t.agents_red), self.fg(|t| t.agents_red)),
            Tone::Plain => (self.fg(|t| t.agents_dimmer), self.fg(|t| t.agents_text)),
            Tone::Unknown => (self.fg(|t| t.agents_dimmer), self.fg(|t| t.agents_dim)),
        };
        div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(7.0))
            .text_size(ui.px(12.5))
            .child(
                div()
                    .w(px(12.0))
                    .h(ui.px(18.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .child(div().size(px(7.0)).rounded_full().bg(dot)),
            )
            .child(
                div()
                    .w(ui.px(108.0))
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

    /// Diagnostics' groups, side by side in two columns when `width` has room.
    fn diagnostics_page(&self, width: f32, ui: &UiFont) -> Vec<AnyElement> {
        let Some(report) = &self.report else {
            let row = self.diagnostic(
                "Diagnostics",
                "Unavailable: the main window is gone.",
                Tone::Bad,
                ui,
            );
            return vec![self.listing(vec![row]).into_any_element()];
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

        let section = |heading: &str, rows| self.section(heading, self.listing(rows), ui);
        let column = || {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(ui.px(18.0))
        };
        let left = [
            section("Commands", commands),
            section("Agents", agents),
            section("Start", start),
        ];
        let right = [section("Config", config), section("Layout", layout)];
        let body = if width >= ui.scale(560.0) {
            div()
                .flex()
                .items_start()
                .gap(ui.px(18.0))
                .child(column().children(left))
                .child(column().children(right))
        } else {
            column().children(left).children(right)
        };
        vec![
            body.into_any_element(),
            div()
                .px(px(4.0))
                .text_size(ui.px(11.5))
                .text_color(self.fg(|t| t.agents_dimmer))
                .child("Nothing here changes paddock or its files.")
                .into_any_element(),
        ]
    }

    pub fn show_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.adding = None;
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
        self.message = None;
        if let Err(error) = result {
            self.say(
                format!("The config file has a problem: {error:#}"),
                true,
                cx,
            );
        }
        self.refresh_inputs(cx);
    }

    /// Whether the window may close: yes without unsaved edits; otherwise as answered to Save /
    /// Don't Save / Cancel, where a Save that fails keeps the window open with the reason shown.
    pub fn confirm_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        if self.draft.unsaved().is_empty() {
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

    /// The window's ground.
    fn ground(&self) -> Hsla {
        hsla(self.theme.bg(|t| t.agents_bg), 1.0)
    }

    /// The ground lifted towards the text by `alpha`: cards at a little, fields on them at more.
    fn lifted(&self, alpha: f32) -> Hsla {
        self.ground()
            .blend(hsla(self.theme.fg(|t| t.agents_text), alpha))
    }

    fn card_ground(&self) -> Hsla {
        self.lifted(0.035)
    }

    fn field_ground(&self) -> Hsla {
        self.lifted(0.075)
    }

    /// The text colour at `alpha`: lines between rows, quiet edges.
    fn faint(&self, alpha: f32) -> Hsla {
        hsla(self.theme.fg(|t| t.agents_text), alpha)
    }

    /// A field's edge: red when its value can't be saved, the accent while typing, else none.
    fn edge(&self, problem: bool, focused: bool) -> Hsla {
        if problem {
            self.fg(|t| t.agents_red).opacity(0.8)
        } else if focused {
            self.fg(|t| t.agents_accent).opacity(0.5)
        } else {
            gpui::transparent_black()
        }
    }

    /// A heading over a card.
    fn heading(&self, text: impl Into<SharedString>, ui: &UiFont) -> Div {
        div()
            .pl(px(4.0))
            .text_size(ui.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.fg(|t| t.agents_dim))
            .child(text.into())
    }

    /// A heading and what it heads.
    fn section(&self, heading: &str, body: impl IntoElement, ui: &UiFont) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(ui.px(8.0))
            .child(self.heading(heading.to_owned(), ui))
            .child(body)
    }

    /// Rows on a card, `lead` above them; a faint line between rows from where their labels start.
    fn card(&self, lead: Option<AnyElement>, rows: Vec<Div>) -> Div {
        let line = self.faint(0.06);
        let mut card = div()
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .bg(self.card_ground())
            .children(lead);
        for (index, row) in rows.into_iter().enumerate() {
            if index > 0 {
                card = card.child(div().h(px(1.0)).ml(px(18.0)).bg(line));
            }
            card = card.child(row);
        }
        card
    }

    /// Rows on a card with no lines between them: Diagnostics.
    fn listing(&self, rows: Vec<Div>) -> Div {
        div()
            .flex()
            .flex_col()
            .py(px(6.0))
            .rounded(px(12.0))
            .bg(self.card_ground())
            .children(rows)
    }

    /// Room for the dot before a setting's label, and the dot when it differs from its default.
    fn dot_slot(&self, on: bool, ui: &UiFont) -> Div {
        div()
            .w(px(12.0))
            .h(ui.px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .when(on, |slot| {
                slot.child(
                    div()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(self.fg(|t| t.agents_accent)),
                )
            })
    }

    /// The yellow tag on a setting that waits for a restart.
    fn after_restart(&self, ui: &UiFont) -> Div {
        let yellow = self.fg(|t| t.agents_yellow);
        div()
            .flex_shrink_0()
            .ml(px(10.0))
            .h(ui.px(20.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded_full()
            .bg(yellow.opacity(0.14))
            .text_size(ui.px(11.0))
            .text_color(yellow)
            .child(footer_icon::icon(
                Icon::Reload,
                yellow,
                ui.scale(11.0) / footer_icon::SIZE,
            ))
            .child("after restart")
    }

    /// "Reset to <default>": puts the setting back to its default.
    fn reset_link(&self, field: &Field, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let key = field.key.clone();
        let text = self.fg(|t| t.agents_text);
        div()
            .id(ElementId::Name(format!("{key}-reset").into()))
            .flex_shrink_0()
            .ml(px(10.0))
            .text_size(ui.px(12.0))
            .text_color(self.fg(|t| t.agents_dim))
            .cursor_pointer()
            .hover(move |style| style.text_color(text))
            .child(reset_label(field, &self.draft.default_shown(&field.key)))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.reset(&key, cx)))
    }

    /// A line under a label: a note, or in red why the value can't be saved.
    fn under(&self, text: impl Into<SharedString>, color: Hsla, ui: &UiFont) -> Div {
        div()
            .pl(px(12.0))
            .mt(px(2.0))
            .text_size(ui.px(12.0))
            .text_color(color)
            .child(text.into())
    }

    /// A setting's label: the dot when it differs from its default, its name, "after restart"
    /// when it waits for one and Reset when it can go back; under it a note and any problem.
    fn label(&self, field: &Field, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let key = &field.key;
        let modified = self.draft.modified(key);
        let title = div()
            .flex()
            .items_center()
            .min_w(px(0.0))
            .child(self.dot_slot(modified, ui))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(13.5))
                    .child(row_label(field).to_owned()),
            )
            .when(field.restart, |title| title.child(self.after_restart(ui)))
            .when(modified, |title| {
                title.child(self.reset_link(field, ui, cx))
            });
        div()
            .flex_1()
            .min_w(ui.px(120.0))
            .flex()
            .flex_col()
            .child(title)
            .children(note(key).map(|note| self.under(note, self.fg(|t| t.agents_dim), ui)))
            .children(
                self.draft
                    .problem(key)
                    .map(|problem| self.under(problem, self.fg(|t| t.agents_red), ui)),
            )
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
        let raised = self.lifted(0.15);
        let (text, dim) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_dim));
        let mut row = div()
            .flex()
            .flex_shrink_0()
            .gap(px(2.0))
            .p(px(3.0))
            .rounded(px(9.0))
            .bg(self.field_ground());
        for &(value, label) in options {
            let on = current == value;
            let key = key.to_owned();
            let segment = div()
                .id(ElementId::Name(format!("{key}-{value}").into()))
                .h(ui.px(26.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .rounded(px(7.0))
                .text_size(ui.px(12.5))
                .cursor_pointer()
                .child(label)
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.set(&key, value, cx)),
                );
            row = row.child(if on {
                segment.bg(raised).text_color(text)
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
            self.lifted(0.16)
        };
        div()
            .id(ElementId::Name(format!("{key}-switch").into()))
            .flex_shrink_0()
            .w(ui.px(38.0))
            .h(ui.px(22.0))
            .p(ui.px(2.0))
            .rounded(ui.px(11.0))
            .bg(track)
            .flex()
            .items_center()
            .when(on, |track| track.justify_end())
            .cursor_pointer()
            .child(
                div()
                    .size(ui.px(18.0))
                    .rounded(ui.px(9.0))
                    .bg(self.fg(|t| t.agents_text))
                    .shadow_sm(),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let value = if on { "false" } else { "true" };
                this.set(&key, value, cx)
            }))
    }

    /// A setting's text field: free text, or a colour value, in the monospace family at `size`.
    #[allow(clippy::too_many_arguments)]
    fn text_box(
        &self,
        key: &str,
        width: Pixels,
        height: Pixels,
        size: f32,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let input = self.inputs.get(key).cloned();
        let focused = input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let mut field = div()
            .id(ElementId::Name(format!("{key}-field").into()))
            .flex_shrink_0()
            .w(width)
            .h(height)
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded(px(8.0))
            .bg(self.field_ground())
            .border_1()
            .border_color(self.edge(self.draft.problem(key).is_some(), focused))
            .overflow_hidden()
            .font_family(MONO)
            .text_size(ui.px(size));
        if let Some(input) = input {
            let handle = input.read(cx).focus_handle(cx);
            field = field
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.focus(&handle, cx)
                })
                .child(div().flex_1().min_w(px(0.0)).overflow_hidden().child(input));
        }
        field
    }

    /// A number setting: − and + either side of its value, which can be typed, and its unit.
    fn stepper(
        &self,
        key: &str,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let input = self.inputs.get(key).cloned();
        let focused = input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let (dim, text) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_text));
        let button = |id: &str, glyph: &'static str, by: f64| {
            let key = key.to_owned();
            div()
                .id(ElementId::Name(format!("{key}-{id}").into()))
                .flex_shrink_0()
                .w(ui.px(30.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(dim)
                .cursor_pointer()
                .hover(move |style| style.text_color(text))
                .child(glyph)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.step(&key, by, cx)))
        };
        let (less, more) = (button("less", "−", -1.0), button("more", "+", 1.0));
        let mut value = div()
            .id(ElementId::Name(format!("{key}-value").into()))
            .min_w(ui.px(if key == "refresh_ms" { 66.0 } else { 52.0 }))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0));
        if let Some(input) = input {
            let shown = input.read(cx).text().to_owned();
            let family = ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into());
            // As wide as its text, so the value and its unit sit together in the middle.
            let width = text_width(&shown, family, ui.px(13.0), window).max(ui.px(8.0)) + px(2.0);
            let handle = input.read(cx).focus_handle(cx);
            value = value
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.focus(&handle, cx)
                })
                .child(div().flex_shrink_0().w(width).child(input));
        }
        let value = value.children(
            unit(key).map(|unit| div().text_color(self.fg(|t| t.agents_dimmer)).child(unit)),
        );
        div()
            .id(ElementId::Name(format!("{key}-stepper").into()))
            .flex_shrink_0()
            .h(ui.px(32.0))
            .flex()
            .items_center()
            .rounded(px(8.0))
            .bg(self.field_ground())
            .border_1()
            .border_color(self.edge(self.draft.problem(key).is_some(), focused))
            .child(less)
            .child(value)
            .child(more)
    }

    /// The fallback fonts, each a tag with ×, then + Add or the field a new one is typed into.
    fn fallbacks(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let field = self.field_ground();
        let (dim, text) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_text));
        let close = self.faint(0.08);
        let pill = || {
            div()
                .flex_shrink_0()
                .h(ui.px(26.0))
                .flex()
                .items_center()
                .rounded_full()
                .font_family(MONO)
                .text_size(ui.px(11.5))
        };
        let mut area = div()
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(px(6.0))
            .min_w(px(0.0))
            .max_w(ui.px(400.0));
        for (index, name) in items(&self.draft.value("font_fallbacks"))
            .into_iter()
            .enumerate()
        {
            area = area.child(
                pill()
                    .pl(px(10.0))
                    .pr(px(4.0))
                    .gap(px(4.0))
                    .bg(field)
                    .text_color(self.fg(|t| t.agents_branch))
                    .child(name)
                    .child(
                        div()
                            .id(("fallback-remove", index))
                            .size(ui.px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .cursor_pointer()
                            .hover(move |style| style.bg(close))
                            .child(footer_icon::icon(
                                Icon::Close,
                                dim,
                                ui.scale(10.0) / footer_icon::SIZE,
                            ))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.remove_fallback(index, cx)
                            })),
                    ),
            );
        }
        match &self.adding {
            Some(input) => area.child(
                pill()
                    .id("fallback-adding")
                    .key_context(menu::DIALOG)
                    .w(ui.px(180.0))
                    .px(px(10.0))
                    .bg(field)
                    .border_1()
                    .border_color(self.edge(false, true))
                    .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                        this.finish_adding(true, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &menu::Cancel, window, cx| {
                        this.finish_adding(false, window, cx)
                    }))
                    .on_mouse_down_out(
                        cx.listener(|this, _, window, cx| this.finish_adding(true, window, cx)),
                    )
                    .child(div().flex_1().min_w(px(0.0)).child(input.clone())),
            ),
            None => area.child(
                pill()
                    .id("fallback-add")
                    .px(px(10.0))
                    .border_1()
                    .border_dashed()
                    .border_color(self.faint(0.16))
                    .font_family(ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into()))
                    .text_size(ui.px(12.0))
                    .text_color(dim)
                    .cursor_pointer()
                    .hover(move |style| style.text_color(text))
                    .child("+ Add")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.start_adding(window, cx)
                    })),
            ),
        }
    }

    /// A font setting: its font in a box that opens the list of installed ones below it.
    fn font_control(&self, field: &Field, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let key = field.key.clone();
        let mono = field.kind == Kind::MonoFont;
        let open = self.picker.as_ref().is_some_and(|picker| picker.key == key);
        let value = self.draft.value(&key);
        let button = div()
            .id(ElementId::Name(format!("{key}-picker").into()))
            .h(ui.px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .rounded(px(8.0))
            .bg(self.field_ground())
            .border_1()
            .border_color(self.edge(false, open))
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
                    .when(mono, |name| name.text_size(ui.px(12.5)))
                    .child(self.font_label(&key)),
            )
            .child(footer_icon::icon(
                Icon::Down,
                self.fg(|t| t.agents_dim),
                ui.scale(12.0) / footer_icon::SIZE,
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| this.toggle_picker(&key, mono, window, cx)),
            );
        div()
            .w(ui.px(220.0))
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
            .border_color(self.faint(0.1))
            .bg(self.ground())
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
                    .bg(self.field_ground())
                    .child(footer_icon::icon(
                        Icon::Search,
                        self.fg(|t| t.agents_dim),
                        ui.scale(12.0) / footer_icon::SIZE,
                    ))
                    .child(div().flex_1().min_w(px(0.0)).child(picker.input.clone())),
            )
            .child(list)
    }

    /// One setting on its card: its label on the left, its control on the right.
    fn row(&self, field: &Field, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let key = field.key.clone();
        let control: AnyElement = match field.kind {
            Kind::MonoFont | Kind::UiFont => self.font_control(field, ui, cx).into_any_element(),
            Kind::Bool => self.switch(&key, ui, cx).into_any_element(),
            Kind::Pet => self.segments(&key, &PETS, ui, cx).into_any_element(),
            Kind::Integer | Kind::Number => self.stepper(&key, window, ui, cx).into_any_element(),
            Kind::List => self.fallbacks(ui, cx).into_any_element(),
            Kind::Text | Kind::Color | Kind::Theme => self
                .text_box(&key, ui.px(260.0), ui.px(32.0), 12.5, window, ui, cx)
                .into_any_element(),
        };
        // The pet waits while the mascot is off.
        let resting = field.kind == Kind::Pet && self.draft.value("mascot_enabled") != "true";
        div()
            .flex()
            .items_center()
            .gap(px(16.0))
            .min_h(ui.px(50.0))
            .pl(px(6.0))
            .pr(px(14.0))
            .py(px(8.0))
            .when(resting, |row| row.opacity(0.45))
            .child(self.label(field, ui, cx))
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
                rows.push(self.row(&field, window, ui, cx));
            }
        }
        rows
    }

    /// A card per heading of `groups`; the terminal's has the preview on top.
    fn grouped(
        &self,
        groups: &[(&str, &[&str])],
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        groups
            .iter()
            .map(|(heading, keys)| {
                let rows = self.rows_for(keys, window, ui, cx);
                let lead = (*heading == "Terminal").then(|| self.preview().into_any_element());
                self.section(heading, self.card(lead, rows), ui)
                    .into_any_element()
            })
            .collect()
    }

    /// A few lines of a terminal in the drafted font, size, line height and colours; the
    /// terminal's own sizes, not the interface's.
    fn preview(&self) -> Div {
        let defaults = Config::default();
        let number = |key: &str, default: f32| {
            self.draft
                .value(key)
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|n| n.is_finite() && *n > 0.0)
                .unwrap_or(default)
        };
        let size = number("font_size", defaults.font_size);
        let height = number("line_height", defaults.line_height);
        let family = self.draft.value("font");
        let mut font = gpui::font(if family.trim().is_empty() {
            defaults.font
        } else {
            family
        });
        font.weight = FontWeight::NORMAL;
        let fallbacks = items(&self.draft.value("font_fallbacks"));
        if !fallbacks.is_empty() {
            font.fallbacks = Some(gpui::FontFallbacks::from_fonts(fallbacks));
        }
        let drafted = self.draft.theme();
        let terminal = drafted.as_ref().unwrap_or(&self.theme).terminal();
        let ansi = |index: usize| hsla(terminal.ansi[index], 1.0);
        let text = hsla(terminal.foreground, 1.0);
        let span = |words: &'static str, color: Hsla| div().text_color(color).child(words);
        let line = |spans: [Div; 4]| div().flex().whitespace_nowrap().children(spans);
        div()
            .mx(px(12.0))
            .mt(px(12.0))
            .mb(px(4.0))
            .px(px(14.0))
            .py(px(12.0))
            .rounded(px(8.0))
            .overflow_hidden()
            .bg(hsla(terminal.background, 1.0))
            .font(font)
            .text_size(px(size))
            .line_height(px(size * height))
            .text_color(text)
            .child(line([
                span("paddock", ansi(2)),
                span(" ~/code ", ansi(8)),
                span("❯ ", ansi(3)),
                span("cargo test", text),
            ]))
            .child(line([
                span("test result: ", ansi(8)),
                span("ok", ansi(2)),
                span(". 430 passed; ", ansi(8)),
                span("0 failed", text),
            ]))
            .child(line([
                span("⎇ main", ansi(4)),
                span("  ✓ ", ansi(2)),
                span("ready", text),
                span(" ▏", ansi(5)),
            ]))
    }

    /// The themes as small windows, each in its own ground, sidebar, accent and status colours,
    /// its name below; the drafted one ringed in the accent and ticked. A click drafts another.
    fn theme_cards(&self, columns: u16, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let current = self.draft.theme_name();
        let (accent, text) = (self.fg(|t| t.agents_accent), self.fg(|t| t.agents_text));
        let edge = self.faint(0.12);
        let mut grid = div().grid().grid_cols(columns).gap(px(12.0));
        for preset in Preset::ALL {
            let value = preset.name();
            let config = Config {
                theme: Some(value.into()),
                ..Config::default()
            };
            let Ok(theme) = Theme::from_config(&config) else {
                continue;
            };
            let on = current == value;
            let color = |pick: Pick| hsla(theme.fg(pick), 1.0);
            let bar = |width: f32, height: f32, fill: Hsla| {
                div()
                    .w(relative(width))
                    .h(ui.px(height))
                    .rounded(ui.px(height / 2.0))
                    .bg(fill)
            };
            let dot = |fill: Hsla| div().size(ui.px(8.0)).rounded_full().bg(fill);
            let window = div()
                .flex()
                .h(ui.px(70.0))
                .rounded(px(7.0))
                .overflow_hidden()
                .bg(hsla(theme.terminal().background, 1.0))
                .child(
                    div()
                        .w(ui.px(22.0))
                        .h_full()
                        .bg(hsla(theme.bg(|t| t.agents_bg), 1.0)),
                )
                .child(
                    div()
                        .flex_1()
                        .p(ui.px(10.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(5.0))
                        .child(bar(0.6, 5.0, color(|t| t.agents_accent)))
                        .child(bar(0.8, 4.0, color(|t| t.agents_rule)))
                        .child(bar(0.45, 4.0, color(|t| t.agents_rule)))
                        .child(div().mt(ui.px(4.0)).flex().gap(ui.px(4.0)).children([
                            dot(color(|t| t.agents_red)),
                            dot(color(|t| t.agents_green)),
                            dot(color(|t| t.agents_blue)),
                        ])),
                );
            let name = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(2.0))
                .text_size(ui.px(13.0))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(preset.label()),
                )
                .when(on, |name| {
                    name.child(footer_icon::icon(
                        Icon::Check,
                        accent,
                        ui.scale(14.0) / footer_icon::SIZE,
                    ))
                });
            let card = div()
                .id(ElementId::Name(format!("theme-{value}").into()))
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(6.0))
                .rounded(px(12.0))
                .bg(self.card_ground())
                .border_2()
                .cursor_pointer()
                .child(window)
                .child(name)
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.set("theme", value, cx)),
                );
            grid = grid.child(if on {
                card.border_color(accent).text_color(text)
            } else {
                card.border_color(gpui::transparent_black())
                    .text_color(text.opacity(0.8))
                    .hover(move |style| style.border_color(edge))
            });
        }
        grid
    }

    /// A colour: the dot when set over the theme (but not in the list of those that are), a
    /// swatch of it as drafted, its name and, when `listed`, its group; Reset when set, and its
    /// value to type.
    fn color_row(
        &self,
        field: &Field,
        drafted: Option<&Theme>,
        listed: bool,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let key = &field.key;
        let custom = self.draft.custom(key);
        let swatch = drafted
            .and_then(|theme| theme.color(field.color()?))
            .map(|rgb| hsla(rgb, 1.0))
            .unwrap_or(gpui::transparent_black());
        let name = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .min_w(px(0.0))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_family(MONO)
                    .text_size(ui.px(12.5))
                    .child(field.label.clone()),
            )
            .when(listed, |name| {
                name.child(
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(11.5))
                        .text_color(self.fg(|t| t.agents_dimmer))
                        .child(field.group),
                )
            });
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(ui.px(46.0))
            .pl(px(6.0))
            .pr(px(14.0))
            .py(px(6.0))
            .child(self.dot_slot(custom && !listed, ui))
            .child(
                div()
                    .flex_shrink_0()
                    .size(ui.px(18.0))
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(self.faint(0.12))
                    .bg(swatch),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(name)
                    .children(self.draft.problem(key).map(|problem| {
                        self.under(problem, self.fg(|t| t.agents_red), ui)
                            .pl(px(0.0))
                    })),
            )
            .when(custom, |row| row.child(self.reset_link(field, ui, cx)))
            .child(self.text_box(key, ui.px(108.0), ui.px(30.0), 12.0, window, ui, cx))
    }

    /// Custom colours: those set over the theme, or with Show all every one, by group.
    fn colors(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let fields: Vec<Field> = self
            .draft
            .fields()
            .iter()
            .filter(|f| f.kind == Kind::Color)
            .cloned()
            .collect();
        let custom: Vec<&Field> = fields
            .iter()
            .filter(|f| self.draft.custom(&f.key))
            .collect();
        let drafted = self.draft.theme();
        let accent = self.fg(|t| t.agents_accent);
        let toggle = if self.all_colors {
            "Show changed".to_owned()
        } else {
            format!("Show all {}", fields.len())
        };
        let header = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.heading("Custom colors", ui))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(if custom.is_empty() {
                        "On top of the theme".to_owned()
                    } else {
                        format!("On top of the theme · {} changed", custom.len())
                    }),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("show-all-colors")
                    .flex_shrink_0()
                    .text_size(ui.px(12.5))
                    .text_color(accent)
                    .cursor_pointer()
                    .hover(move |style| style.text_color(accent.opacity(0.8)))
                    .child(toggle)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.all_colors = !this.all_colors;
                        cx.notify();
                    })),
            );
        let mut section = div().flex().flex_col().gap(ui.px(8.0)).child(header);
        if self.all_colors {
            let mut groups: Vec<(&str, Vec<Div>)> = Vec::new();
            for field in &fields {
                let row = self.color_row(field, drafted.as_ref(), false, window, ui, cx);
                match groups.last_mut() {
                    Some((group, rows)) if *group == field.group => rows.push(row),
                    _ => groups.push((field.group, vec![row])),
                }
            }
            for (group, rows) in groups {
                section = section.child(
                    div()
                        .pt(ui.px(6.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(6.0))
                        .child(
                            div()
                                .pl(px(4.0))
                                .text_size(ui.px(12.0))
                                .text_color(self.fg(|t| t.agents_dimmer))
                                .child(group),
                        )
                        .child(self.card(None, rows)),
                );
            }
        } else if custom.is_empty() {
            section = section.child(
                div()
                    .pl(px(4.0))
                    .text_size(ui.px(12.5))
                    .text_color(self.fg(|t| t.agents_dimmer))
                    .child("Every color follows the theme."),
            );
        } else {
            let rows = custom
                .into_iter()
                .map(|field| self.color_row(field, drafted.as_ref(), true, window, ui, cx))
                .collect();
            section = section.child(self.card(None, rows));
        }
        section
    }

    fn page_items(
        &self,
        width: f32,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        match self.page {
            Page::General => self.grouped(&GENERAL, window, ui, cx),
            Page::Agents => self.grouped(&AGENTS, window, ui, cx),
            Page::Appearance => {
                let columns = columns(width, ui.scale(130.0), 12.0);
                vec![
                    self.section("Theme", self.theme_cards(columns, ui, cx), ui)
                        .into_any_element(),
                    self.colors(window, ui, cx).into_any_element(),
                ]
            }
            Page::Diagnostics => self.diagnostics_page(width, ui),
        }
    }

    /// The pages, each with its icon, the one showing raised; the config file's path at the foot.
    fn nav(&self, width: Pixels, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let (accent, text) = (self.fg(|t| t.agents_accent), self.fg(|t| t.agents_text));
        let (raised, hover) = (accent.opacity(0.13), self.faint(0.05));
        let mut nav = div()
            .w(width)
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(12.0))
            .pb(px(14.0))
            // Room for the traffic lights.
            .pt(px(crate::windows::title_bar(TITLE_BAR, ui)));
        for page in Page::ALL {
            let on = page == self.page;
            let icon = match page {
                Page::General => Icon::Settings,
                Page::Appearance => Icon::Palette,
                Page::Agents => Icon::Agent,
                Page::Diagnostics => Icon::Pulse,
            };
            let item = div()
                .id(page.label())
                .h(ui.px(34.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(8.0))
                .text_size(ui.px(13.5))
                .cursor_pointer()
                .child(footer_icon::icon(
                    icon,
                    if on {
                        accent
                    } else {
                        self.fg(|t| t.agents_dimmer)
                    },
                    ui.scale(16.0) / footer_icon::SIZE,
                ))
                .child(page.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_page(page, cx)));
            nav = nav.child(if on {
                item.bg(raised).text_color(text)
            } else {
                item.text_color(text.opacity(0.7))
                    .hover(move |style| style.bg(hover))
            });
        }
        let path = shown_path(&self.path);
        let (folder, file) = path.split_at(path.rfind('/').map_or(0, |at| at + 1));
        nav.child(div().flex_1()).child(
            div()
                .px(px(10.0))
                .font_family(MONO)
                .text_size(ui.px(11.0))
                .text_color(self.fg(|t| t.agents_dimmer))
                .when(!folder.is_empty(), |path| path.child(folder.to_owned()))
                .child(file.to_owned()),
        )
    }

    /// A quiet button in the bar.
    fn bar_button(
        &self,
        id: &'static str,
        label: &'static str,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: fn(&mut Self, &mut Window, &mut Context<Self>),
    ) -> Stateful<Div> {
        let hover = self.faint(0.07);
        div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(30.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_color(self.fg(|t| t.agents_branch))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| click(this, window, cx)))
    }

    /// The bar's accent button: Save ⌘S, or Keep my edits.
    fn bar_action(
        &self,
        id: &'static str,
        label: &'static str,
        keys: Option<&'static str>,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: fn(&mut Self, &mut Window, &mut Context<Self>),
    ) -> Stateful<Div> {
        let accent = self.fg(|t| t.agents_accent);
        div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(30.0))
            .px(px(14.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded_full()
            .bg(accent)
            .text_color(self.ground())
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(move |style| style.bg(accent.opacity(0.88)))
            .child(label)
            .children(keys.map(|keys| {
                div()
                    .opacity(0.7)
                    .font_family(MONO)
                    .text_size(ui.px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(keys)
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| click(this, window, cx)))
    }

    /// The bar at the bottom: the file changed on disk (Keep, Discard); or what Save last said; or
    /// how many settings differ from the file. Revert and Save while any do.
    fn save_bar(&self, ui: &UiFont, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let unsaved: Vec<String> = self
            .draft
            .unsaved()
            .into_iter()
            .map(|field| field.label.clone())
            .collect();
        let pill = |id: &'static str| {
            div()
                .id(id)
                .occlude()
                .min_w(px(0.0))
                .h(ui.px(46.0))
                .pl(px(16.0))
                .pr(px(8.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .rounded_full()
                .bg(self.lifted(0.08))
                .border_1()
                .shadow_lg()
                .whitespace_nowrap()
                .text_size(ui.px(13.0))
                .text_color(self.fg(|t| t.agents_branch))
        };
        let words = |text: String| {
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .child(text)
        };
        if self.conflict {
            let yellow = self.fg(|t| t.agents_yellow);
            let text = self
                .message
                .as_ref()
                .map(|(text, _)| text.clone())
                .unwrap_or_default();
            return Some(
                pill("conflict")
                    .border_color(yellow.opacity(0.35))
                    .child(footer_icon::icon(
                        Icon::Warning,
                        yellow,
                        ui.scale(15.0) / footer_icon::SIZE,
                    ))
                    .child(words(text))
                    .child(
                        self.bar_button("discard", "Discard my edits", ui, cx, |this, _, cx| {
                            this.resolve_conflict(false, cx)
                        }),
                    )
                    .child(self.bar_action(
                        "keep",
                        "Keep my edits",
                        None,
                        ui,
                        cx,
                        |this, _, cx| this.resolve_conflict(true, cx),
                    )),
            );
        }
        let (dot, text, detail, problem) = match &self.message {
            Some((text, problem)) => (
                if *problem {
                    self.fg(|t| t.agents_red)
                } else {
                    self.fg(|t| t.agents_green)
                },
                text.clone(),
                None,
                *problem,
            ),
            None if !unsaved.is_empty() => {
                let (head, names) = unsaved_summary(&unsaved);
                (self.fg(|t| t.agents_accent), head, Some(names), false)
            }
            None => return None,
        };
        let mut bar = pill("save-bar")
            .border_color(self.faint(0.08))
            .child(div().flex_shrink_0().size(px(7.0)).rounded_full().bg(dot))
            .child(
                div()
                    .flex()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .gap(px(6.0))
                    .child(div().flex_shrink_0().child(text))
                    .children(detail.map(|names| {
                        words(format!("· {names}")).text_color(self.fg(|t| t.agents_dim))
                    })),
            );
        if !unsaved.is_empty() {
            bar = bar
                .child(self.bar_button("revert", "Revert", ui, cx, |this, _, cx| this.revert(cx)))
                .child(self.bar_action(
                    "save",
                    "Save",
                    Some("⌘S"),
                    ui,
                    cx,
                    |this, window, cx| this.save(&menu::SaveSettings, window, cx),
                ));
        } else if problem {
            let (dim, text) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_text));
            bar = bar.child(
                div()
                    .id("dismiss")
                    .flex_shrink_0()
                    .size(ui.px(26.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .cursor_pointer()
                    .hover(move |style| style.bg(text.opacity(0.07)))
                    .child(footer_icon::icon(
                        Icon::Close,
                        dim,
                        ui.scale(12.0) / footer_icon::SIZE,
                    ))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.message = None;
                        cx.notify();
                    })),
            );
        } else {
            bar = bar.pr(px(18.0));
        }
        Some(bar)
    }

    /// The page's title, and on Diagnostics when it was checked and Refresh.
    fn title_row(&self, height: f32, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let diagnostics = self.page == Page::Diagnostics;
        let (dim, hover) = (self.fg(|t| t.agents_dim), self.lifted(0.13));
        div()
            .flex_shrink_0()
            .h(px(height))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(40.0))
            .child(
                div()
                    .text_size(ui.px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.page.label()),
            )
            .when(diagnostics, |row| {
                row.child(div().flex_1())
                    .children(self.report.as_ref().map(|report| {
                        div()
                            .text_size(ui.px(12.0))
                            .text_color(dim)
                            .child(format!("Checked at {} · read-only", clock(report.checked)))
                    }))
                    .child(
                        div()
                            .id("refresh")
                            .flex_shrink_0()
                            .size(ui.px(30.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(8.0))
                            .bg(self.field_ground())
                            .cursor_pointer()
                            .hover(move |style| style.bg(hover))
                            .child(footer_icon::icon(
                                Icon::Reload,
                                dim,
                                ui.scale(15.0) / footer_icon::SIZE,
                            ))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.refresh_diagnostics(cx)
                            })),
                    )
            })
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let bar = crate::windows::title_bar(TITLE_BAR, &ui);
        window.set_traffic_light_position(crate::window::traffic_lights(bar));
        let nav = ui.px(196.0);
        // The pages' width inside their margins.
        let width = f32::from(window.viewport_size().width - nav - px(1.0)) - 80.0;
        let items = self.page_items(width, window, &ui, cx);
        let content = div()
            .relative()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(self.title_row(bar, &ui, cx))
            .child(
                div()
                    .id("settings-list")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(ui.px(22.0))
                    .px(px(40.0))
                    .pt(px(10.0))
                    // Room to scroll the last row above the bar.
                    .pb(ui.px(90.0))
                    .children(items),
            )
            .children(self.save_bar(&ui, cx).map(|pill| {
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(ui.px(18.0))
                    .px(px(24.0))
                    .flex()
                    .justify_center()
                    .child(pill)
            }));

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
            .bg(self.ground())
            .text_size(ui.px(13.0))
            .text_color(self.fg(|t| t.agents_text))
            .child(self.nav(nav, &ui, cx))
            .child(div().w(px(1.0)).h_full().bg(self.faint(0.06)))
            .child(content)
    }
}

#[derive(Clone, Copy)]
enum Tone {
    Good,
    Bad,
    Plain,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::fields;

    #[test]
    fn steps_round_and_stop_at_the_least() {
        assert_eq!(stepped("1.3", "1.3", 1.0, steps("line_height")), "1.4");
        assert_eq!(stepped("14.5", "14", -1.0, steps("font_size")), "13.5");
        assert_eq!(stepped("6", "14", -1.0, steps("font_size")), "6");
        assert_eq!(stepped("wide", "380", 1.0, steps("sidebar_width")), "390");
        assert_eq!(stepped("1000", "1000", -1.0, steps("refresh_ms")), "750");
        assert_eq!(stepped("250", "1000", -1.0, steps("refresh_ms")), "250");
    }

    #[test]
    fn reset_names_the_default_as_the_page_shows_it() {
        let field = |key: &str| fields().into_iter().find(|f| f.key == key).unwrap();
        assert_eq!(reset_label(&field("font_size"), "14"), "Reset to 14");
        assert_eq!(reset_label(&field("corral"), "corral"), "Reset to corral");
        assert_eq!(reset_label(&field("ui_font"), ""), "Reset to System");
        assert_eq!(reset_label(&field("mascot"), "clawd"), "Reset to Clawd");
        assert_eq!(reset_label(&field("mascot_enabled"), "true"), "Reset to on");
        assert_eq!(
            reset_label(&field("font_fallbacks"), "a, b"),
            "Reset to default"
        );
        assert_eq!(reset_label(&field("colors.focus"), "yellow"), "Reset");
    }

    #[test]
    fn the_bar_counts_and_names_the_first_few() {
        let labels = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            unsaved_summary(&labels(&["Terminal size"])),
            ("1 unsaved change".into(), "Terminal size".into())
        );
        assert_eq!(
            unsaved_summary(&labels(&["Theme", "focus", "claude", "codex", "pi"])),
            (
                "5 unsaved changes".into(),
                "Theme, focus, claude and 2 more".into()
            )
        );
    }

    #[test]
    fn four_theme_cards_a_row_when_they_fit() {
        assert_eq!(columns(670.0, 130.0, 12.0), 4);
        assert_eq!(columns(420.0, 130.0, 12.0), 3);
        assert_eq!(columns(100.0, 130.0, 12.0), 1);
    }
}
