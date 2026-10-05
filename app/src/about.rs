//! The About window: what paddock is and where its parts come from.
use crate::{fonts::UiFont, menu, theme::Theme, view::hsla};
use gpui::{Context, FocusHandle, Focusable, FontWeight, Render, Window, div, prelude::*, px};
use std::rc::Rc;

/// The key context of the About window: ⌘W closes it.
pub const CONTEXT: &str = "PaddockAbout";

pub struct AboutView {
    theme: Rc<Theme>,
    focus: FocusHandle,
}

impl AboutView {
    pub fn new(theme: Rc<Theme>, cx: &mut Context<Self>) -> Self {
        Self {
            theme,
            focus: cx.focus_handle(),
        }
    }
}

impl Focusable for AboutView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AboutView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let fg = |pick: fn(&crate::preset::Theme) -> crate::preset::Color| {
            hsla(self.theme.fg(pick), 1.0)
        };
        ui.apply(div())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &menu::CloseWindow, window, _| window.remove_window()))
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .px(px(28.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(12.0))
            .text_color(fg(|t| t.agents_text))
            .child(
                div()
                    .text_size(ui.px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("paddock"),
            )
            .child(
                div()
                    .text_color(fg(|t| t.muted))
                    .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
            )
            .child(
                div()
                    .pt(px(6.0))
                    .child("A desktop home for corral agents, built with GPUI."),
            )
            .child(
                div()
                    .text_center()
                    .text_size(ui.px(11.0))
                    .text_color(fg(|t| t.agents_dim))
                    .child(
                        "Terminal and agent code, themes and pets come from Saddle. \
                         Clawd is Claude Code's mascot, from Anthropic; the cat and the \
                         capybara are Saddle originals.",
                    ),
            )
    }
}
