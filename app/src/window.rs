//! The window's root view: the Agents sidebar on the left, the terminal pane under its title bar
//! on the right.
use crate::{
    config::Config,
    theme::Theme,
    view::{Options, TerminalView, hsla},
};
use gpui::{Context, Entity, Render, Window, div, prelude::*, px};
use std::rc::Rc;

pub struct PaddockWindow {
    theme: Rc<Theme>,
    sidebar_width: f32,
    terminal: Entity<TerminalView>,
}

impl PaddockWindow {
    pub fn new(
        config: &Config,
        theme: Theme,
        options: Options,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = Rc::new(theme);
        let terminal = {
            let theme = theme.clone();
            cx.new(|cx| TerminalView::new(options, theme, window, cx))
        };
        Self {
            theme,
            sidebar_width: config.sidebar_width,
            terminal,
        }
    }
}

impl Render for PaddockWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let subject = self.terminal.read(cx).subject().to_owned();
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(hsla(theme.bg(|t| t.bg), 1.0))
            .child(
                // Placeholder until the Agents list arrives.
                div()
                    .flex_shrink_0()
                    .w(px(self.sidebar_width))
                    .h_full()
                    .bg(hsla(theme.bg(|t| t.agents_bg), 1.0)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_shrink_0()
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_size(px(12.0))
                            .text_color(hsla(theme.fg(|t| t.muted), 1.0))
                            .bg(hsla(theme.bg(|t| t.bg), 1.0))
                            .border_b_1()
                            .border_color(hsla(theme.fg(|t| t.border), 1.0))
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(subject),
                    )
                    .child(
                        // The pane fills what the title bar leaves; its grid follows this size.
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .overflow_hidden()
                            .child(self.terminal.clone()),
                    ),
            )
    }
}
