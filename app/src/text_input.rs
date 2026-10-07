//! A single-line text field: typing (with input methods), selection by mouse and keyboard, copy,
//! cut, paste, undo and redo. Adapted from GPUI's `examples/input.rs` (gpui-pre 0.3.8, Copyright 2022–2025
//! Zed Industries, Inc., Apache-2.0, like GPUI itself): themed colours, a change event, paddock's
//! Copy and Paste menu actions, and the GPUI example's own window and quit code left out.
use crate::menu;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, Hsla, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, actions, div,
    fill, point, prelude::*, px, relative, size,
};
use std::{
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Cut,
        Undo,
        Redo
    ]
);

/// The key context of a focused field.
const CONTEXT: &str = "TextInput";

/// Editing keys, only while a field has the keyboard. Copy and paste are the menu's.
pub fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("left", Left, Some(CONTEXT)),
        KeyBinding::new("right", Right, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
        KeyBinding::new("cmd-left", Home, Some(CONTEXT)),
        KeyBinding::new("cmd-right", End, Some(CONTEXT)),
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        KeyBinding::new("cmd-z", Undo, Some(CONTEXT)),
        KeyBinding::new("cmd-shift-z", Redo, Some(CONTEXT)),
    ]
}

/// The field's colours.
#[derive(Clone, Copy)]
pub struct Colors {
    pub text: Hsla,
    pub placeholder: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
}

/// Sent whenever the text changes.
pub struct Changed;

/// Editing state shared by input callbacks and history actions, independent of a window.
struct EditState {
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_typing: Option<(Instant, usize)>,
    composition_before: Option<Snapshot>,
}

struct Snapshot {
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

impl EditState {
    fn new(content: SharedString) -> Self {
        let end = content.len();
        Self {
            content,
            selected_range: end..end,
            selection_reversed: false,
            marked_range: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_typing: None,
            composition_before: None,
        }
    }

    fn set_text(&mut self, text: SharedString) {
        *self = Self::new(text);
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.content = snapshot.content;
        self.selected_range = snapshot.selected_range;
        self.selection_reversed = snapshot.selection_reversed;
    }

    fn replace(&mut self, range: Range<usize>, new_text: &str, now: Instant) {
        let composing = self.composition_before.is_some();
        let before = self
            .composition_before
            .take()
            .unwrap_or_else(|| self.snapshot());
        let typing = !composing && range.is_empty() && !new_text.is_empty();
        let merge = typing
            && self.last_typing.is_some_and(|(time, cursor)| {
                now.duration_since(time) < Duration::from_secs(1) && cursor == range.start
            });
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        let end = range.start + new_text.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_typing = typing.then_some((now, end));
        if self.content != before.content {
            if !merge {
                self.undo.push(before);
            }
            self.redo.clear();
        }
    }

    fn begin_composition(&mut self) {
        // Keep preedit updates out of history; commit the whole composition together.
        self.last_typing = None;
        if self.composition_before.is_none() {
            self.composition_before = Some(self.snapshot());
        }
    }

    fn finish_composition(&mut self) {
        self.marked_range = None;
        if let Some(before) = self.composition_before.take()
            && self.content != before.content
        {
            self.undo.push(before);
            self.redo.clear();
        }
    }

    fn undo(&mut self) -> bool {
        if self.marked_range.is_some() {
            return false;
        }
        self.last_typing = None;
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(before);
        true
    }

    fn paste(&mut self, range: Range<usize>, text: &str, now: Instant) {
        self.last_typing = None;
        self.replace(range, &text.replace('\n', " "), now);
        self.last_typing = None;
    }

    fn redo(&mut self) -> bool {
        if self.marked_range.is_some() {
            return false;
        }
        self.last_typing = None;
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(after);
        true
    }
}

pub struct TextInput {
    focus_handle: FocusHandle,
    edit: EditState,
    placeholder: SharedString,
    colors: Colors,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
}

impl EventEmitter<Changed> for TextInput {}

impl TextInput {
    pub fn new(
        content: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        colors: Colors,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            edit: EditState::new(content.into()),
            placeholder: placeholder.into(),
            colors,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.edit.content
    }

    /// Replaces the text without telling listeners: for loading a value, not for typing.
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.edit.set_text(text.into());
        cx.notify();
    }

    /// New colours, after the theme changed.
    pub fn set_colors(&mut self, colors: Colors, cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if self.edit.undo() {
            cx.emit(Changed);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if self.edit.redo() {
            cx.emit(Changed);
            cx.notify();
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.edit.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.edit.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.edit.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.edit.selected_range.end), cx);
        } else {
            self.move_to(self.edit.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.edit.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.edit.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        self.edit.last_typing = None;
        let mut range = self.edit.selected_range.clone();
        if range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            range.start = prev;
        }
        let range = self.edit.marked_range.clone().unwrap_or(range);
        self.replace_text_in_range(Some(self.range_to_utf16(&range)), "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        self.edit.last_typing = None;
        let mut range = self.edit.selected_range.clone();
        if range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            range.end = next;
        }
        let range = self.edit.marked_range.clone().unwrap_or(range);
        self.replace_text_in_range(Some(self.range_to_utf16(&range)), "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn paste(&mut self, _: &menu::Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let range = self
                .edit
                .marked_range
                .clone()
                .unwrap_or(self.edit.selected_range.clone());
            self.edit.paste(range, &text, Instant::now());
            cx.emit(Changed);
            cx.notify();
        }
    }

    fn copy(&mut self, _: &menu::Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.edit.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.edit.content[self.edit.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.edit.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.edit.content[self.edit.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.edit.last_typing = None;
        self.edit.selected_range = offset..offset;
        self.edit.selection_reversed = false;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.edit.selection_reversed {
            self.edit.selected_range.start
        } else {
            self.edit.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.edit.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.edit.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.edit.last_typing = None;
        if self.edit.selection_reversed {
            self.edit.selected_range.start = offset
        } else {
            self.edit.selected_range.end = offset
        };
        if self.edit.selected_range.end < self.edit.selected_range.start {
            self.edit.selection_reversed = !self.edit.selection_reversed;
            self.edit.selected_range = self.edit.selected_range.end..self.edit.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.edit.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.edit.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.edit
            .content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.edit
            .content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.edit.content.len())
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.edit.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.edit.selected_range),
            reversed: self.edit.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.edit
            .marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.edit.finish_composition();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.edit.marked_range.clone())
            .unwrap_or(self.edit.selected_range.clone());
        self.edit.replace(range, new_text, Instant::now());
        cx.emit(Changed);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.edit.marked_range.clone())
            .unwrap_or(self.edit.selected_range.clone());
        self.edit.begin_composition();
        self.edit.content = (self.edit.content[0..range.start].to_owned()
            + new_text
            + &self.edit.content[range.end..])
            .into();
        if !new_text.is_empty() {
            self.edit.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.edit.marked_range = None;
        }
        self.edit.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        if self.edit.marked_range.is_none() {
            self.edit.finish_composition();
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.edit.content.clone();
        let selected_range = input.edit.selected_range.clone();
        let cursor = input.cursor_offset();
        let colors = input.colors;
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), colors.placeholder)
        } else {
            (content, colors.text)
        };
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.edit.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(1.5), bounds.bottom() - bounds.top()),
                    ),
                    colors.cursor,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    colors.selection,
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        let _ = line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        );
        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_typing_undoes_and_redoes_in_one_step() {
        let mut edit = EditState::new("".into());
        let now = Instant::now();
        for ch in ["a", "你", "🙂"] {
            edit.replace(edit.selected_range.clone(), ch, now);
        }
        assert_eq!(edit.content.as_ref(), "a你🙂");
        // A pause ends the group without sleeping in the test.
        edit.replace(8..8, "b", now + Duration::from_secs(2));
        edit.replace(9..9, "c", now + Duration::from_millis(2100));
        edit.undo();
        assert_eq!(edit.content.as_ref(), "a你🙂");
        edit.undo();
        assert_eq!(edit.content.as_ref(), "");
        edit.redo();
        assert_eq!(edit.content.as_ref(), "a你🙂");
        assert_eq!(edit.selected_range, 8..8);

        edit.redo();
        assert_eq!(edit.content.as_ref(), "a你🙂bc");
    }

    #[test]
    fn paste_is_one_step_separate_from_typing() {
        let mut edit = EditState::new("".into());
        let now = Instant::now();
        edit.replace(0..0, "a", now);
        edit.paste(1..1, "你\n🙂", now);
        assert_eq!(edit.content.as_ref(), "a你 🙂");
        edit.replace(edit.selected_range.clone(), "b", now);
        edit.undo();
        assert_eq!(edit.content.as_ref(), "a你 🙂");
        edit.undo();
        assert_eq!(edit.content.as_ref(), "a");
        edit.redo();
        assert_eq!(edit.content.as_ref(), "a你 🙂");
    }

    #[test]
    fn typing_after_undo_clears_redo() {
        let mut edit = EditState::new("prefix".into());
        let now = Instant::now();
        edit.replace(6..6, " old", now);
        edit.undo();
        assert_eq!(edit.content.as_ref(), "prefix");
        edit.replace(6..6, " new", now);
        assert!(!edit.redo());
        assert_eq!(edit.content.as_ref(), "prefix new");
        edit.undo();
        assert_eq!(edit.content.as_ref(), "prefix");
    }

    #[test]
    fn programmatic_prefill_is_not_undoable_and_resets_history() {
        let mut edit = EditState::new("initial".into());
        let now = Instant::now();
        assert!(!edit.undo());
        edit.replace(7..7, " first", now);
        edit.paste(edit.selected_range.clone(), " second", now);
        edit.undo();
        edit.set_text("预填🙂".into());
        assert!(!edit.undo());
        assert!(!edit.redo());
        assert_eq!(edit.content.as_ref(), "预填🙂");
        assert_eq!(edit.selected_range, 10..10);
        edit.replace(10..10, " typed", now);
        edit.undo();
        assert_eq!(edit.content.as_ref(), "预填🙂");
        assert!(!edit.undo());
    }
}
