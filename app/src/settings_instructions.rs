//! Settings › Instructions (DESIGN §13 P5-119): each agent's global instructions file from
//! `crate::instructions`. A row of agents, then the one chosen on a card: its file read as
//! Markdown, or edited as text, or its changes before a save; when there is no file yet, which
//! file the agent reads and Create file.
use super::*;
use crate::{
    instructions::{self, DiffRow, File, SaveError, Sign, State},
    markdown::{self, Block},
    markdown_view,
};
use gpui::{ClipboardItem, StyledText};
use std::time::SystemTime;

/// The page's state in the Settings window.
#[derive(Default)]
pub(super) struct Instructions {
    /// Each agent's file as last read, in `instructions::KINDS` order.
    files: Vec<File>,
    /// The agent shown, an index into `files`.
    shown: usize,
    /// The shown file's text read as Markdown.
    blocks: Vec<Block>,
    edit: Option<Edit>,
}

/// A file being edited.
struct Edit {
    /// The agent, an index into `files`.
    index: usize,
    /// Where the agent reads its file, looked at again before each check and the save.
    path: PathBuf,
    /// Where `path` led when editing began: the file written, as long as it still leads there.
    target: PathBuf,
    /// The text read when editing began: the file must still hold it to be saved over.
    opened: String,
    input: Entity<TextInput>,
    /// The changes shown before a save, when they are.
    review: Option<Review>,
    /// Changed somewhere else since editing began: the file no longer holds `opened`, or `path`
    /// no longer leads to `target`.
    changed: bool,
    _subscription: Subscription,
}

struct Review {
    rows: Vec<DiffRow>,
    /// Where the file as it is goes first.
    backup: PathBuf,
}

/// Under the card.
const TIMING: &str =
    "Changes apply to sessions started after you save; agents already running keep what they read.";

const CHANGED: &str = "This file changed on disk after you opened it.";

/// What else is worth knowing about an agent's file.
fn also(kind: &str) -> Option<&'static str> {
    match kind {
        "omp" => Some(
            "It also reads other agents' global files, such as ~/.claude/CLAUDE.md and \
             ~/.codex/AGENTS.md.",
        ),
        "agy" => Some("Gemini CLI reads this file too."),
        _ => None,
    }
}

/// `text` with each path in it (a word from `~/` or `/`) in the monospace family.
fn with_paths(text: String) -> StyledText {
    let mut fonts = Vec::new();
    let mut at = 0;
    for word in text.split(' ') {
        let path = word.trim_end_matches(['.', ',']);
        if path.starts_with("~/") || path.starts_with('/') {
            fonts.push((at..at + path.len(), SharedString::from(MONO)));
        }
        at += word.len() + 1;
    }
    StyledText::new(text).with_font_family_overrides(fonts)
}

/// How a button on the page looks.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    /// The accent, outlined: Edit, Create file, Review changes….
    Go,
    /// Filled with the accent: Save.
    Solid,
    /// A faint ground: Cancel, Back to editing, Reload, Copy my text.
    Quiet,
}

impl SettingsView {
    /// Reads every agent's file again: on showing the page and on coming back to the window. What
    /// is being edited stays; whether its file changed meanwhile is looked at again.
    pub(super) fn refresh_instructions(&mut self) {
        let state = &mut self.instructions;
        state.files = instructions::read_all();
        state.shown = state.shown.min(state.files.len().saturating_sub(1));
        if let Some(edit) = &mut state.edit {
            edit.changed = !instructions::unchanged(&edit.path, &edit.target, &edit.opened);
        }
        self.parse_instructions();
    }

    /// The shown file's Markdown, after it was read.
    fn parse_instructions(&mut self) {
        let state = &mut self.instructions;
        state.blocks = match state.files.get(state.shown).map(|file| &file.state) {
            Some(State::Text { text, .. }) => markdown::blocks(text),
            _ => Vec::new(),
        };
    }

    /// The agent at `index` read again; none while editing, so the edit can't be dropped.
    fn read_instructions(&mut self, index: usize) -> Option<File> {
        let file = self.instructions.files.get(index)?;
        let file = instructions::read(file.kind, file.path.clone());
        self.instructions.files[index] = file.clone();
        if index == self.instructions.shown {
            self.parse_instructions();
        }
        Some(file)
    }

    fn show_agent(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.instructions.edit.is_some() {
            return;
        }
        self.instructions.shown = index;
        self.read_instructions(index);
        cx.notify();
    }

    /// Edit: the file as it is now, as text in the field.
    fn edit_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.instructions.shown;
        let Some(file) = self.read_instructions(index) else {
            return;
        };
        let State::Text { text, .. } = file.state else {
            cx.notify();
            return;
        };
        let colors = self.input_colors();
        let input =
            cx.new(|cx| TextInput::new(text.clone(), "Markdown", colors, cx).wrapping(true));
        let subscription = cx.subscribe(&input, |_, _, _: &text_input::Changed, cx| cx.notify());
        window.focus(&input.read(cx).focus_handle(cx), cx);
        self.instructions.edit = Some(Edit {
            index,
            path: file.path,
            target: file.target,
            opened: text,
            input,
            review: None,
            changed: false,
            _subscription: subscription,
        });
        cx.notify();
    }

    /// Create file: an empty file where the agent reads it, then editing it.
    fn create_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = self.instructions.files.get(self.instructions.shown) else {
            return;
        };
        let target = file.target.clone();
        match instructions::create(&target) {
            Ok(()) => self.edit_instructions(window, cx),
            Err(error) => {
                self.read_instructions(self.instructions.shown);
                self.say(
                    format!("Couldn't create {}: {error}", shown_path(&target)),
                    true,
                    cx,
                );
            }
        }
    }

    /// Cancel: the edit dropped, the file as it is shown again.
    fn cancel_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.instructions.edit = None;
        self.read_instructions(self.instructions.shown);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Review changes…: what would change, and whether the file changed meanwhile.
    fn review_instructions(&mut self, cx: &mut Context<Self>) {
        let state = &mut self.instructions;
        let Some(edit) = &mut state.edit else {
            return;
        };
        let text = edit.input.read(cx).text().to_owned();
        if text == edit.opened {
            return;
        }
        let kind = state
            .files
            .get(edit.index)
            .map_or("agent", |file| file.kind);
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let backup = instructions::backups(&home).join(instructions::backup_name(
            kind,
            &edit.target,
            SystemTime::now(),
        ));
        edit.changed = !instructions::unchanged(&edit.path, &edit.target, &edit.opened);
        edit.review = Some(Review {
            rows: instructions::diff(&edit.opened, &text),
            backup,
        });
        cx.notify();
    }

    fn back_to_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(edit) = &mut self.instructions.edit {
            edit.review = None;
            window.focus(&edit.input.read(cx).focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Save: the old file backed up, then the text written in its place; refused when the file
    /// changed meanwhile, and when it can't be written the text stays and the bar says why.
    fn save_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = &mut self.instructions.edit else {
            return;
        };
        let Some(review) = &edit.review else {
            return;
        };
        if edit.changed {
            return;
        }
        let text = edit.input.read(cx).text().to_owned();
        match instructions::save(
            &edit.path,
            &edit.target,
            &text,
            &edit.opened,
            &review.backup,
        ) {
            Ok(backup) => {
                let index = edit.index;
                self.instructions.edit = None;
                let path = self
                    .read_instructions(index)
                    .map_or_else(String::new, |file| shown_path(&file.path));
                window.focus(&self.focus, cx);
                self.say(
                    format!("Saved {path}; the old file is in {}.", shown_path(&backup)),
                    false,
                    cx,
                );
            }
            Err(SaveError::Changed) => {
                edit.changed = true;
                cx.notify();
            }
            Err(SaveError::Failed(why)) => {
                self.say(format!("Not saved. {why}. Your text is kept."), true, cx)
            }
        }
    }

    /// Reload: the edit dropped for the file as it is now, edited again; when it is gone, shown.
    fn reload_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.instructions.edit.as_ref().map(|edit| edit.index) else {
            return;
        };
        let file = self.read_instructions(index);
        match (file, &mut self.instructions.edit) {
            (
                Some(File {
                    path,
                    target,
                    state: State::Text { text, .. },
                    ..
                }),
                Some(edit),
            ) => {
                edit.input
                    .update(cx, |input, cx| input.set_text(text.clone(), cx));
                edit.opened = text;
                edit.path = path;
                edit.target = target;
                edit.changed = false;
                edit.review = None;
                window.focus(&edit.input.read(cx).focus_handle(cx), cx);
            }
            _ => {
                self.instructions.edit = None;
                window.focus(&self.focus, cx);
            }
        }
        cx.notify();
    }

    /// Copy my text: the whole field to the clipboard.
    fn copy_instructions(&mut self, cx: &mut Context<Self>) {
        if let Some(edit) = &self.instructions.edit {
            let text = edit.input.read(cx).text().to_owned();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.say("Copied your text.", false, cx);
        }
    }

    /// A button: `enabled` or faded and inert.
    #[allow(clippy::too_many_arguments)]
    fn page_button(
        &self,
        id: &'static str,
        label: &'static str,
        look: Look,
        enabled: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
        click: fn(&mut Self, &mut Window, &mut Context<Self>),
    ) -> Stateful<Div> {
        let accent = self.fg(|t| t.agents_accent);
        let button = div()
            .id(id)
            .flex_shrink_0()
            .h(ui.px(28.0))
            .px(px(13.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_size(ui.px(12.0))
            .child(label);
        let (button, hover) = match look {
            Look::Go => (
                button
                    .bg(accent.opacity(0.16))
                    .border_1()
                    .border_color(accent.opacity(0.45))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(accent),
                accent.opacity(0.24),
            ),
            Look::Solid => (
                button
                    .bg(accent)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.ground()),
                accent.opacity(0.85),
            ),
            Look::Quiet => (
                button
                    .bg(self.faint(0.07))
                    .text_color(self.fg(|t| t.agents_text)),
                self.faint(0.12),
            ),
        };
        if enabled {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, window, cx| click(this, window, cx)),
                )
        } else {
            button.opacity(0.4)
        }
    }

    /// A small tag in the card's head: `Editing`, `Review` in yellow, `Not created` quiet.
    fn page_tag(&self, text: &'static str, yellow: bool, ui: &UiFont) -> Div {
        let (ground, color) = if yellow {
            let yellow = self.fg(|t| t.agents_yellow);
            (yellow.opacity(0.13), yellow)
        } else {
            (self.faint(0.07), self.fg(|t| t.agents_dim))
        };
        div()
            .flex_shrink_0()
            .h(ui.px(20.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded_full()
            .bg(ground)
            .text_size(ui.px(11.0))
            .text_color(color)
            .child(text)
    }

    /// The agents side by side: a dot when its file is there, a ring when not; the one shown
    /// raised. While a file is edited the others wait.
    fn agent_strip(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let state = &self.instructions;
        let (accent, text, dim) = (
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
        );
        let (green, ring) = (self.fg(|t| t.agents_green), self.fg(|t| t.agents_dimmer));
        let editing = state.edit.is_some();
        let mut strip = div().flex().flex_wrap().gap(px(6.0));
        for (index, file) in state.files.iter().enumerate() {
            let on = index == state.shown;
            let dot = div().size(px(6.0)).rounded_full();
            let dot = if file.exists() {
                dot.bg(green)
            } else {
                dot.border_1().border_color(ring)
            };
            let pill = div()
                .id(("instructions-agent", index))
                .h(ui.px(30.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(7.0))
                .rounded_full()
                .border_1()
                .text_size(ui.px(12.5))
                .child(dot)
                .child(crate::integrations::name(file.kind));
            strip = strip.child(if on {
                pill.bg(accent.opacity(0.13))
                    .border_color(accent.opacity(0.4))
                    .text_color(text)
            } else if editing {
                pill.bg(self.card_ground())
                    .border_color(gpui::transparent_black())
                    .text_color(dim)
                    .opacity(0.5)
            } else {
                pill.bg(self.card_ground())
                    .border_color(gpui::transparent_black())
                    .text_color(dim)
                    .cursor_pointer()
                    .hover(move |style| style.text_color(text))
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.show_agent(index, cx)),
                    )
            });
        }
        strip
    }

    /// The card's head: the path, where it leads when it is a link, and what is going on.
    fn file_head(&self, file: &File, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let edit = self.instructions.edit.as_ref();
        let dim = self.fg(|t| t.agents_dim);
        let mono = |text: String| {
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .font_family(MONO)
                .text_size(ui.px(12.0))
                .child(text)
        };
        let link = (file.target != file.path)
            .then(|| mono(format!("→ {}", shown_path(&file.target))).text_color(dim));
        let mut head = div()
            .flex_shrink_0()
            .min_h(ui.px(52.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .pl(px(16.0))
            .pr(px(12.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(self.faint(0.06))
            .child(
                mono(shown_path(&file.path))
                    .flex_shrink_0()
                    .max_w(relative(0.6)),
            )
            .children(link);
        head = match (&file.state, edit) {
            (_, Some(edit)) => head.child(self.page_tag(
                if edit.review.is_some() {
                    "Review"
                } else {
                    "Editing"
                },
                true,
                ui,
            )),
            (State::Missing, None) => head.child(self.page_tag("Not created", false, ui)),
            (State::Text { size, modified, .. }, None) => {
                let mut meta = instructions::size_label(*size);
                if let Some(modified) = modified {
                    meta = format!("{meta} · {}", instructions::changed_label(*modified));
                }
                head.child(
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(12.0))
                        .text_color(self.fg(|t| t.agents_dimmer))
                        .child(meta),
                )
            }
            (State::Unreadable(_), None) => head,
        };
        let edit_button = (edit.is_none() && matches!(file.state, State::Text { .. })).then(|| {
            self.page_button(
                "instructions-edit",
                "Edit",
                Look::Go,
                true,
                ui,
                cx,
                |this, w, cx| this.edit_instructions(w, cx),
            )
        });
        head.child(div().flex_1()).children(edit_button)
    }

    /// The card's foot: a line on the left, buttons on the right.
    fn file_foot(&self, hint: impl IntoElement, buttons: Vec<Stateful<Div>>, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pl(px(16.0))
            .pr(px(12.0))
            .pt(px(10.0))
            .pb(px(12.0))
            .border_t_1()
            .border_color(self.faint(0.06))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(hint),
            )
            .children(buttons)
    }

    /// The yellow line over an edit whose file changed meanwhile: copy the text, or take the new
    /// file instead.
    fn changed_banner(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let yellow = self.fg(|t| t.agents_yellow);
        div()
            .flex_shrink_0()
            .mx(px(12.0))
            .mt(px(10.0))
            .px(px(12.0))
            .py(px(7.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .rounded(px(8.0))
            .bg(yellow.opacity(0.10))
            .border_1()
            .border_color(yellow.opacity(0.30))
            .text_size(ui.px(12.5))
            .text_color(yellow)
            .child(div().flex_1().min_w(px(0.0)).child(CHANGED))
            .child(self.page_button(
                "instructions-copy",
                "Copy my text",
                Look::Quiet,
                true,
                ui,
                cx,
                |this, _, cx| this.copy_instructions(cx),
            ))
            .child(self.page_button(
                "instructions-reload",
                "Reload",
                Look::Quiet,
                true,
                ui,
                cx,
                |this, window, cx| this.reload_instructions(window, cx),
            ))
    }

    /// The file read as Markdown, to scroll through; when there is none, `file_missing`.
    fn file_view(&self, file: &File, ui: &UiFont, cx: &mut Context<Self>) -> AnyElement {
        let dim = self.fg(|t| t.agents_dim);
        let centered = |lines: Vec<AnyElement>| {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .px(ui.px(60.0))
                .text_center()
                .children(lines)
                .into_any_element()
        };
        let quiet = |text: String| {
            div()
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child(with_paths(text))
                .into_any_element()
        };
        match &file.state {
            State::Text { text, .. } if text.trim().is_empty() => {
                centered(vec![quiet("This file is empty.".into())])
            }
            State::Text { .. } => div()
                .id("instructions-body")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .px(px(18.0))
                .pt(px(12.0))
                .pb(px(14.0))
                .child(markdown_view::blocks(
                    &self.instructions.blocks,
                    &gpui::font(MONO),
                    markdown_view::Colors::of(&self.theme),
                    ui,
                ))
                .into_any_element(),
            State::Unreadable(why) => centered(vec![
                div()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("This file can't be shown")
                    .into_any_element(),
                quiet(format!("Couldn't read it: {why}.")),
            ]),
            State::Missing => self.file_missing(file, ui, cx).into_any_element(),
        }
    }

    /// No file yet: which one the agent reads, and Create file.
    fn file_missing(&self, file: &File, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let name = crate::integrations::name(file.kind);
        let mut says = format!(
            "{name} reads {} when a session starts.",
            shown_path(&file.path)
        );
        if let Some(also) = also(file.kind) {
            says = format!("{says} {also}");
        }
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .px(ui.px(60.0))
            .text_center()
            .child(
                div()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{name} has no global instructions file yet")),
            )
            .child(
                div()
                    .text_size(ui.px(12.5))
                    .line_height(relative(1.55))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(with_paths(says)),
            )
            .child(div().mt(px(8.0)).child(self.page_button(
                "instructions-create",
                "Create file",
                Look::Go,
                true,
                ui,
                cx,
                |this, window, cx| this.create_instructions(window, cx),
            )))
    }

    /// The text being edited, in the monospace family, and Cancel and Review changes….
    fn file_editor(&self, edit: &Edit, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let text = edit.input.read(cx).text().to_owned();
        let edited = text != edit.opened;
        let handle = edit.input.read(cx).focus_handle(cx);
        let field = div()
            .id("instructions-editor")
            .flex_1()
            .min_h(px(0.0))
            .mx(px(12.0))
            .my(px(10.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(8.0))
            .bg(self.field_ground())
            .border_1()
            .border_color(self.fg(|t| t.agents_accent).opacity(0.35))
            .overflow_y_scroll()
            .cursor_text()
            .font_family(MONO)
            .text_size(ui.px(12.0))
            .line_height(relative(1.6))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&handle, cx)
            })
            .child(edit.input.clone());
        let foot = self.file_foot(
            instructions::summary(&edit.opened, &text),
            vec![
                self.page_button(
                    "instructions-cancel",
                    "Cancel",
                    Look::Quiet,
                    true,
                    ui,
                    cx,
                    |this, window, cx| this.cancel_instructions(window, cx),
                ),
                self.page_button(
                    "instructions-review",
                    "Review changes…",
                    Look::Go,
                    edited,
                    ui,
                    cx,
                    |this, _, cx| this.review_instructions(cx),
                ),
            ],
            ui,
        );
        vec![field.into_any_element(), foot.into_any_element()]
    }

    /// The changes before a save: added lines green, removed red, unchanged ones folded; where
    /// the old file goes, and Back to editing and Save.
    fn file_review(
        &self,
        edit: &Edit,
        review: &Review,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (dim, dimmer) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_dimmer));
        let (green, red) = (self.fg(|t| t.agents_green), self.fg(|t| t.agents_red));
        let line = ui.px(12.0 * 1.65);
        let rows = review.rows.iter().map(|row| match row {
            DiffRow::Gap => div().pl(px(76.0)).min_h(line).text_color(dimmer).child("⋯"),
            DiffRow::Line { number, sign, text } => {
                let (mark, color, ground) = match sign {
                    Sign::Same => (" ", dim, None),
                    Sign::Added => ("+", green, Some(green.opacity(0.13))),
                    Sign::Removed => ("−", red, Some(red.opacity(0.12))),
                };
                div()
                    .flex()
                    .px(px(16.0))
                    .min_h(line)
                    .when_some(ground, |row, ground| row.bg(ground))
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(ui.px(34.0))
                            .pr(px(10.0))
                            .text_right()
                            .text_color(dimmer)
                            .child(number.to_string()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(ui.px(14.0))
                            .text_color(color)
                            .child(mark),
                    )
                    .child(div().flex_1().min_w(px(0.0)).child(text.clone()))
            }
        });
        let changes = div()
            .id("instructions-diff")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .py(px(8.0))
            .font_family(MONO)
            .text_size(ui.px(12.0))
            .line_height(relative(1.65))
            .children(rows);
        let hint = with_paths(format!(
            "The current file is copied to {} first.",
            shown_path(&review.backup)
        ));
        let foot = self.file_foot(
            hint,
            vec![
                self.page_button(
                    "instructions-back",
                    "Back to editing",
                    Look::Quiet,
                    true,
                    ui,
                    cx,
                    |this, window, cx| this.back_to_editing(window, cx),
                ),
                self.page_button(
                    "instructions-save",
                    "Save",
                    Look::Solid,
                    !edit.changed,
                    ui,
                    cx,
                    |this, window, cx| this.save_instructions(window, cx),
                ),
            ],
            ui,
        );
        vec![changes.into_any_element(), foot.into_any_element()]
    }

    /// The page: the agents, the card for the one shown, and when changes apply. It fills the
    /// window's height and scrolls inside the card; `room` is kept under it for the bar.
    pub(super) fn instructions_page(
        &self,
        room: Pixels,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let page = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .gap(ui.px(14.0))
            .px(px(40.0))
            .pt(px(10.0))
            .pb(room);
        let note = |text: String| {
            div()
                .flex_shrink_0()
                .pl(px(4.0))
                .text_size(ui.px(12.0))
                .line_height(relative(1.5))
                .text_color(self.fg(|t| t.agents_dim))
                .child(with_paths(text))
        };
        let state = &self.instructions;
        let Some(file) = state.files.get(state.shown) else {
            return page.child(note(
                "No home folder, so no agent's files can be found.".into(),
            ));
        };
        let mut card = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .overflow_hidden()
            .bg(self.card_ground())
            .child(self.file_head(file, ui, cx));
        // While editing, and with no file (where the card says who reads what), only when
        // changes apply.
        let says = match &state.edit {
            Some(edit) => {
                if edit.changed {
                    card = card.child(self.changed_banner(ui, cx));
                }
                card = card.children(match &edit.review {
                    Some(review) => self.file_review(edit, review, ui, cx),
                    None => self.file_editor(edit, ui, cx),
                });
                TIMING.to_owned()
            }
            None => {
                card = card.child(self.file_view(file, ui, cx));
                let name = crate::integrations::name(file.kind);
                match (&file.state, also(file.kind)) {
                    (State::Missing, _) => TIMING.to_owned(),
                    (_, Some(also)) => {
                        format!("{name} reads this file when a session starts. {also} {TIMING}")
                    }
                    (_, None) => format!("{name} reads this file when a session starts. {TIMING}"),
                }
            }
        };
        page.child(self.agent_strip(ui, cx))
            .child(card)
            .child(note(says))
    }
}
