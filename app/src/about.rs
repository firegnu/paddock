//! The About window: what paddock is and where its parts come from.
use crate::{
    fonts::UiFont,
    menu,
    pet::{Pet, Rgb},
    theme::Theme,
    view::hsla,
};
use gpui::{
    BoxShadow, Context, FocusHandle, Focusable, FontWeight, Render, Window, canvas, div, point,
    prelude::*, px, size,
};
use std::rc::Rc;

/// The key context of the About window: ⌘W closes it.
pub const CONTEXT: &str = "PaddockAbout";

/// The app icon's square, in points at the base interface size.
const ICON: f32 = 84.0;

pub struct AboutView {
    theme: Rc<Theme>,
    /// The cat's standing pose cut to its own bounds: width, height and its pixels row by row.
    cat: (usize, usize, Vec<Option<Rgb>>),
    focus: FocusHandle,
}

impl AboutView {
    pub fn new(theme: Rc<Theme>, cx: &mut Context<Self>) -> Self {
        Self {
            theme,
            cat: cat(),
            focus: cx.focus_handle(),
        }
    }
}

/// The cat as `icon.rs` puts it on the app icon: its standing pose without the empty margin.
fn cat() -> (usize, usize, Vec<Option<Rgb>>) {
    let pack = Pet::Cat.pack();
    let image = pack.image("stand").expect("the cat stands");
    let (width, height) = (pack.width, pack.height);
    let solid = |x: usize, y: usize| image[y * width + x].is_some();
    let xs: Vec<usize> = (0..width)
        .filter(|&x| (0..height).any(|y| solid(x, y)))
        .collect();
    let ys: Vec<usize> = (0..height)
        .filter(|&y| (0..width).any(|x| solid(x, y)))
        .collect();
    let (left, top) = (xs[0], ys[0]);
    let (w, h) = (xs[xs.len() - 1] - left + 1, ys[ys.len() - 1] - top + 1);
    let pixels = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| image[(top + y) * width + left + x])
        .collect();
    (w, h, pixels)
}

impl Focusable for AboutView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AboutView {
    /// The app icon, drawn as `icon.rs` draws it: the cat in the middle of a rounded square, at a
    /// whole number of points per picture pixel, a little below centre.
    fn icon(&self, ui: &UiFont) -> impl IntoElement {
        let square = ui.scale(ICON);
        let (w, h, pixels) = self.cat.clone();
        let fit = ((square * 0.72) / w as f32).min((square * 0.62) / h as f32);
        let scale = fit.floor().max(1.0);
        let drawn = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let (room_w, room_h) =
                    (f32::from(bounds.size.width), f32::from(bounds.size.height));
                let left = bounds.origin.x + px(((room_w - w as f32 * scale) / 2.0).round());
                let top = bounds.origin.y
                    + px(
                        ((room_h - h as f32 * scale) / 2.0 + square * 1024.0 / 824.0 / 40.0)
                            .round(),
                    );
                for y in 0..h {
                    let mut x = 0;
                    while x < w {
                        let Some(rgb) = pixels[y * w + x] else {
                            x += 1;
                            continue;
                        };
                        let start = x;
                        while x < w && pixels[y * w + x] == Some(rgb) {
                            x += 1;
                        }
                        let (r, g, b) = rgb;
                        window.paint_quad(gpui::fill(
                            gpui::Bounds::new(
                                point(left + px(start as f32 * scale), top + px(y as f32 * scale)),
                                size(px((x - start) as f32 * scale), px(scale)),
                            ),
                            gpui::Rgba {
                                r: f32::from(r) / 255.0,
                                g: f32::from(g) / 255.0,
                                b: f32::from(b) / 255.0,
                                a: 1.0,
                            },
                        ));
                    }
                }
            },
        )
        .size_full();
        div()
            .flex_shrink_0()
            .size(px(square))
            .rounded(px(square * 0.225))
            .border_1()
            .border_color(hsla(self.theme.fg(|t| t.agents_rule), 1.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.4),
                offset: point(px(0.0), ui.px(6.0)),
                blur_radius: ui.px(18.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .child(drawn)
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
            .text_center()
            .px(ui.px(36.0))
            .pt(ui.px(22.0))
            .pb(ui.px(26.0))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(13.0))
            .text_color(fg(|t| t.agents_text))
            .child(self.icon(&ui))
            .child(
                div()
                    .mt(ui.px(16.0))
                    .text_size(ui.px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("paddock"),
            )
            .child(
                div()
                    .mt(ui.px(3.0))
                    .text_size(ui.px(12.0))
                    .text_color(fg(|t| t.agents_dim))
                    .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
            )
            .child(
                div()
                    .mt(ui.px(14.0))
                    .line_height(gpui::relative(1.5))
                    .text_color(fg(|t| t.agents_branch))
                    .child("A desktop home for corral agents, built with GPUI."),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(40.0))
                    .h(px(1.0))
                    .mt(ui.px(16.0))
                    .mb(ui.px(12.0))
                    .bg(fg(|t| t.agents_rule)),
            )
            .child(
                div()
                    .text_size(ui.px(11.5))
                    .line_height(gpui::relative(1.55))
                    .text_color(fg(|t| t.agents_dimmer))
                    .child(
                        "Terminal and agent code, themes and pets come from Saddle. \
                         Clawd is Claude Code's mascot, from Anthropic; the cat and the \
                         capybara are Saddle originals.",
                    ),
            )
    }
}
