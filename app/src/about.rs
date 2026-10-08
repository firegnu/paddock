//! The About window: what paddock is and where its parts come from.
use crate::{fonts::UiFont, menu, theme::Theme, view::hsla};
use gpui::{
    BoxShadow, Context, Corners, FocusHandle, Focusable, FontWeight, Render, RenderImage, Window,
    canvas, div, point, prelude::*, px,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

/// The key context of the About window: ⌘W closes it.
pub const CONTEXT: &str = "PaddockAbout";

/// The top row at the base interface size: the traffic lights alone.
pub const TITLE_BAR: f32 = 34.0;

/// The app icon's square, in points at the base interface size.
const ICON: f32 = 84.0;

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

/// The app icon `pixels` device pixels square, as `icon.rs` draws it into the bundle, margin and
/// all; drawn once per size and kept.
fn picture(pixels: u32) -> Arc<RenderImage> {
    thread_local! {
        static PICTURES: RefCell<HashMap<u32, Arc<RenderImage>>> = RefCell::default();
    }
    PICTURES.with_borrow_mut(|pictures| {
        pictures
            .entry(pixels)
            .or_insert_with(|| {
                let mut pixel_data = crate::icon::icon(pixels);
                // GPUI keeps images as BGRA.
                for pixel in pixel_data.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
                let buffer = image::ImageBuffer::from_raw(pixels, pixels, pixel_data)
                    .expect("icon() gives a square of RGBA pixels");
                Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
            })
            .clone()
    })
}

impl Focusable for AboutView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AboutView {
    /// The app icon as the Dock shows it, drawn by `icon.rs` at this screen's pixels: its rounded
    /// square fills `ICON`, and the transparent margin round it spills over.
    fn icon(&self, ui: &UiFont) -> impl IntoElement {
        let square = ui.scale(ICON);
        let drawn = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let whole = bounds.size.width * (1024.0 / 824.0);
                let margin = (whole - bounds.size.width) / 2.0;
                let area = gpui::Bounds::new(
                    point(bounds.origin.x - margin, bounds.origin.y - margin),
                    gpui::size(whole, whole),
                );
                let pixels = (f32::from(whole) * window.scale_factor()).round().max(1.0);
                let image = picture(pixels as u32);
                let _ = window.paint_image(area, area, Corners::default(), image, 0, false);
            },
        )
        .size_full();
        div()
            .flex_shrink_0()
            .size(px(square))
            .rounded(px(square * 0.225))
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let bar = crate::windows::title_bar(TITLE_BAR, &ui);
        window.set_traffic_light_position(crate::window::traffic_lights(bar));
        let fg = |pick: fn(&crate::preset::Theme) -> crate::preset::Color| {
            hsla(self.theme.fg(pick), 1.0)
        };
        // Under the title bar, scrolling when the window is too small for it, as at large
        // interface sizes: a little room above the icon, then the rest.
        let body = div()
            .id("about-body")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .text_center()
            .px(ui.px(36.0))
            .pt(ui.px(8.0))
            .pb(ui.px(26.0))
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
            );
        ui.apply(div())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &menu::CloseWindow, window, _| window.remove_window()))
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(13.0))
            .text_color(fg(|t| t.agents_text))
            .child(div().flex_shrink_0().h(px(bar)))
            .child(body)
    }
}
