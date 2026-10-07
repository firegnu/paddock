//! The look the window's small floating panels share: a panel a shade lighter than the sidebar
//! with a faint edge and a soft shadow, rows of an icon, a label and a quiet shortcut that light
//! up under the mouse, faint rules between groups, and an inline choice of two or three; and where
//! a panel hangs from the button that opened it. The sidebar's menu, the new tab and split panels
//! and the Attention list use it.
use crate::{
    fonts::UiFont,
    footer_icon::{self, Icon},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Bounds, BoxShadow, Div, ElementId, FontWeight, Hsla, Pixels, SharedString, Size, Stateful, div,
    point, prelude::*, px,
};

/// Sizes, in points at the base interface size.
const RADIUS: f32 = 10.0;
const INSET: f32 = 5.0;
const ROW: f32 = 30.0;
const ROW_RADIUS: f32 = 6.0;
const ROW_X: f32 = 9.0;
const ROW_GAP: f32 = 10.0;
const TEXT: f32 = 13.0;
const KEYS: f32 = 12.0;
const CHOICE: f32 = 11.5;

/// What a row's hover brightens: this name on each row, for its shortcut.
pub const ROW_GROUP: &str = "popover-row";

/// The panel's colours, all from the theme.
struct Colors {
    ground: Hsla,
    edge: Hsla,
    rule: Hsla,
    hover: Hsla,
    text: Hsla,
    icon: Hsla,
    keys: Hsla,
    danger: Hsla,
}

impl Colors {
    fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        let sidebar = hsla(theme.bg(|t| t.agents_bg), 1.0);
        let selected = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let rule = fg(|t| t.agents_rule);
        let ground = sidebar.blend(selected.opacity(0.5));
        Self {
            ground,
            edge: rule,
            rule: rule.opacity(0.5),
            hover: ground.blend(rule.opacity(0.55)),
            text: fg(|t| t.agents_text),
            icon: fg(|t| t.agents_dim),
            keys: fg(|t| t.agents_dimmer),
            danger: fg(|t| t.agents_red),
        }
    }
}

/// The floating panel; place it, size it and fill it.
pub fn panel(theme: &Theme, ui: &UiFont) -> Div {
    let colors = Colors::of(theme);
    div()
        .flex()
        .flex_col()
        .p(ui.px(INSET))
        .rounded(ui.px(RADIUS))
        .border_1()
        .border_color(colors.edge)
        .bg(colors.ground)
        .shadow(vec![BoxShadow {
            color: gpui::black().opacity(0.45),
            offset: point(px(0.0), ui.px(12.0)),
            blur_radius: ui.px(32.0),
            spread_radius: px(0.0),
            inset: false,
        }])
        .text_size(ui.px(TEXT))
        .text_color(colors.text)
        .whitespace_nowrap()
}

/// How a row reads and answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    /// It ends something: red, and redder under the mouse.
    Danger,
    /// Not available now: faded, and the mouse leaves it alone.
    Off,
}

/// A row: the icon, the label, and room on the right for `trailing` (a shortcut, a tick, a
/// choice). Give it a click unless it is `Off`.
pub fn row(
    theme: &Theme,
    ui: &UiFont,
    id: impl Into<ElementId>,
    icon: Icon,
    label: impl Into<SharedString>,
    tone: Tone,
) -> Stateful<Div> {
    let colors = Colors::of(theme);
    let row = line(theme, ui, icon, label, tone == Tone::Danger).id(id);
    match tone {
        Tone::Plain => {
            let hover = colors.hover;
            row.cursor_pointer().hover(move |style| style.bg(hover))
        }
        Tone::Danger => {
            let hover = colors.danger.opacity(0.16);
            row.cursor_pointer().hover(move |style| style.bg(hover))
        }
        Tone::Off => row.opacity(0.4),
    }
}

/// A row that only holds what is on its right, such as a choice: it does not light up itself.
pub fn line(
    theme: &Theme,
    ui: &UiFont,
    icon: Icon,
    label: impl Into<SharedString>,
    danger: bool,
) -> Div {
    let colors = Colors::of(theme);
    let (ink, glyph) = if danger {
        (colors.danger, colors.danger)
    } else {
        (colors.text, colors.icon)
    };
    div()
        .group(ROW_GROUP)
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(ui.px(ROW_GAP))
        .h(ui.px(ROW))
        .px(ui.px(ROW_X))
        .rounded(ui.px(ROW_RADIUS))
        .text_color(ink)
        .child(footer_icon::icon(icon, glyph, ui.scale(1.0)))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .child(label.into()),
        )
}

/// A shortcut at a row's right, as keycaps run together (`⌘⇧N`); brighter under the mouse.
pub fn keys(theme: &Theme, ui: &UiFont, caps: &[String]) -> Div {
    let colors = Colors::of(theme);
    let bright = colors.icon;
    div()
        .flex_shrink_0()
        .text_size(ui.px(KEYS))
        .text_color(colors.keys)
        .group_hover(ROW_GROUP, move |style| style.text_color(bright))
        .child(caps.concat())
}

/// A tick at a row's right when it is on; the same room left empty when not, so the label never
/// moves.
pub fn tick(theme: &Theme, ui: &UiFont, on: bool) -> Div {
    let accent = hsla(theme.fg(|t| t.agents_accent), 1.0);
    div()
        .flex_shrink_0()
        .size(ui.px(footer_icon::SIZE))
        .when(on, |tick| {
            tick.child(footer_icon::icon(Icon::Check, accent, ui.scale(1.0)))
        })
}

/// The faint rule between groups of rows.
pub fn rule(theme: &Theme, ui: &UiFont) -> Div {
    div()
        .flex_shrink_0()
        .h(px(1.0))
        .mx(ui.px(4.0))
        .my(ui.px(5.0))
        .bg(Colors::of(theme).rule)
}

/// Where an inline choice's options sit: a sunken strip.
pub fn choices(theme: &Theme, ui: &UiFont) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .p(ui.px(2.0))
        .rounded(ui.px(ROW_RADIUS))
        .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
        .text_size(ui.px(CHOICE))
}

/// One option of an inline choice, raised when it is the one in force.
pub fn choice(
    theme: &Theme,
    ui: &UiFont,
    id: impl Into<ElementId>,
    label: &'static str,
    on: bool,
) -> Stateful<Div> {
    let colors = Colors::of(theme);
    let raised = colors.edge;
    let option = div()
        .id(id)
        .px(ui.px(8.0))
        .py(ui.px(3.0))
        .rounded(ui.px(4.0))
        .cursor_pointer()
        .child(label);
    if on {
        option
            .bg(raised)
            .text_color(colors.text)
            .font_weight(FontWeight::MEDIUM)
    } else {
        option
            .text_color(colors.icon)
            .hover(move |style| style.bg(raised.opacity(0.5)))
    }
}

/// Room a panel keeps from what opened it, and from the window's edges.
const HANG_GAP: f32 = 6.0;
const HANG_MARGIN: f32 = 8.0;
/// Under this much room below what opened it, a panel opens upward when there is more room there.
const HANG_LOW: f32 = 200.0;
const HEADING: f32 = 10.5;
const HINTS: f32 = 11.5;

/// Where a panel hangs from what opened it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hang {
    /// Under it, left edges in line.
    BelowLeft,
    /// Under it, right edges in line.
    BelowRight,
    /// Beside it on the right, tops in line.
    Beside,
}

/// A panel's place in the window, in points: its left edge and width, its top edge or (when it
/// opens upward) how far its bottom edge sits above the window's, and the most it may be tall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub left: f32,
    pub width: f32,
    pub top: Option<f32>,
    pub bottom: Option<f32>,
    pub max_height: f32,
}

/// Places a panel `width` points wide by `anchor`, what opened it, inside a window `window` big,
/// at `scale` times the base interface size: it keeps a margin from every edge, narrowing in a
/// narrow window, and is never taller than the room it has. Hung below something low in the
/// window, it opens upward instead when there is more room there.
pub fn hang(
    anchor: Bounds<Pixels>,
    width: f32,
    hang: Hang,
    window: Size<Pixels>,
    scale: f32,
) -> Placed {
    let (gap, margin, low) = (HANG_GAP * scale, HANG_MARGIN * scale, HANG_LOW * scale);
    let (x, y) = (f32::from(anchor.origin.x), f32::from(anchor.origin.y));
    let (w, h) = (f32::from(anchor.size.width), f32::from(anchor.size.height));
    let (room_x, room_y) = (f32::from(window.width), f32::from(window.height));
    let width = width.min(room_x - 2.0 * margin).max(0.0);
    let left = match hang {
        Hang::BelowLeft => x,
        Hang::BelowRight => x + w - width,
        Hang::Beside => x + w + gap,
    };
    let left = left.min(room_x - margin - width).max(margin);
    let below = |top: f32| Placed {
        left,
        width,
        top: Some(top),
        bottom: None,
        max_height: (room_y - margin - top).max(0.0),
    };
    match hang {
        Hang::Beside => below(y.min(room_y - margin - low).max(margin)),
        Hang::BelowLeft | Hang::BelowRight => {
            let top = y + h + gap;
            let under = room_y - margin - top;
            let over = y - gap - margin;
            if under >= low || under >= over {
                below(top)
            } else {
                Placed {
                    left,
                    width,
                    top: None,
                    bottom: Some(room_y - (y - gap)),
                    max_height: over.max(0.0),
                }
            }
        }
    }
}

/// A group's name over its rows: small capitals, faint.
pub fn heading(theme: &Theme, ui: &UiFont, text: impl Into<SharedString>) -> Div {
    div()
        .flex_shrink_0()
        .px(ui.px(ROW_X))
        .pt(ui.px(6.0))
        .pb(ui.px(3.0))
        .text_size(ui.px(HEADING))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(Colors::of(theme).keys)
        .child(text.into())
}

/// A row led by `lead` (an icon in a colour of its own, a status dot), lit when `selected` and
/// faintly under the mouse; the caller adds the rest.
pub fn lead_row(
    theme: &Theme,
    ui: &UiFont,
    id: impl Into<ElementId>,
    lead: impl IntoElement,
    selected: bool,
) -> Stateful<Div> {
    let hover = Colors::of(theme).hover;
    div()
        .id(id)
        .group(ROW_GROUP)
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(ui.px(ROW_GAP))
        .h(ui.px(ROW))
        .px(ui.px(ROW_X))
        .rounded(ui.px(ROW_RADIUS))
        .cursor_pointer()
        .map(|row| {
            if selected {
                row.bg(hover)
            } else {
                row.hover(move |style| style.bg(hover.opacity(0.6)))
            }
        })
        .child(
            div()
                .flex_shrink_0()
                .w(ui.px(footer_icon::SIZE))
                .flex()
                .justify_center()
                .child(lead),
        )
}

/// The panel's ground.
pub fn ground(theme: &Theme) -> Hsla {
    Colors::of(theme).ground
}

/// The ground of a row that is selected, or under the mouse.
pub fn lit(theme: &Theme) -> Hsla {
    Colors::of(theme).hover
}

/// The faint rule under a panel's field or over its hints, edge to edge.
pub fn edge_rule(theme: &Theme) -> Hsla {
    Colors::of(theme).rule
}

/// The keys a panel answers to, along its foot under a faint rule: `↑↓ move` and so on.
pub fn hints(theme: &Theme, ui: &UiFont, hints: &[(&'static str, &'static str)]) -> Div {
    let colors = Colors::of(theme);
    div()
        .flex_shrink_0()
        .flex()
        .gap(ui.px(14.0))
        .mt(ui.px(4.0))
        .px(ui.px(ROW_X))
        .pt(ui.px(8.0))
        .pb(ui.px(4.0))
        .border_t_1()
        .border_color(colors.rule)
        .text_size(ui.px(HINTS))
        .text_color(colors.keys.opacity(0.8))
        .children(hints.iter().map(|(keys, what)| {
            div()
                .flex()
                .gap(ui.px(4.0))
                .child(keys.to_string())
                .child(what.to_string())
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    fn at(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    const WINDOW: (f32, f32) = (1280.0, 800.0);

    fn place(anchor: Bounds<Pixels>, width: f32, how: Hang, window: (f32, f32)) -> Placed {
        hang(anchor, width, how, size(px(window.0), px(window.1)), 1.0)
    }

    #[test]
    fn panels_hang_under_what_opened_them() {
        // The `+`: left edges in line, just under it, as tall as the window has room for.
        let new_tab = place(at(400.0, 6.0, 28.0, 28.0), 340.0, Hang::BelowLeft, WINDOW);
        assert_eq!(
            new_tab,
            Placed {
                left: 400.0,
                width: 340.0,
                top: Some(40.0),
                bottom: None,
                max_height: 800.0 - 8.0 - 40.0,
            }
        );
        // The split button: right edges in line.
        let split = place(
            at(1000.0, 40.0, 28.0, 28.0),
            316.0,
            Hang::BelowRight,
            WINDOW,
        );
        assert_eq!((split.left, split.top), (1028.0 - 316.0, Some(74.0)));
        // The strip's bell: beside the strip, tops in line.
        let attention = place(at(0.0, 80.0, 52.0, 30.0), 360.0, Hang::Beside, WINDOW);
        assert_eq!((attention.left, attention.top), (58.0, Some(80.0)));
    }

    #[test]
    fn panels_stay_inside_the_window() {
        let inside = |placed: Placed, window: (f32, f32)| {
            let top = placed
                .top
                .unwrap_or_else(|| window.1 - placed.bottom.unwrap() - placed.max_height);
            placed.left >= 8.0
                && placed.left + placed.width <= window.0 - 8.0
                && top >= 8.0
                && top + placed.max_height <= window.1 - 8.0
        };
        // Near the right edge, a left-aligned panel moves left to keep its margin…
        let placed = place(at(1200.0, 6.0, 28.0, 28.0), 340.0, Hang::BelowLeft, WINDOW);
        assert_eq!(placed.left, 1280.0 - 8.0 - 340.0);
        assert!(inside(placed, WINDOW));
        // …and near the left edge a right-aligned one moves right.
        let placed = place(at(20.0, 40.0, 28.0, 28.0), 316.0, Hang::BelowRight, WINDOW);
        assert_eq!(placed.left, 8.0);
        assert!(inside(placed, WINDOW));
        // A window narrower than the panel narrows it.
        let narrow = (300.0, 500.0);
        let placed = place(at(100.0, 6.0, 28.0, 28.0), 340.0, Hang::BelowLeft, narrow);
        assert_eq!((placed.left, placed.width), (8.0, 284.0));
        assert!(inside(placed, narrow));
        // Low in the window, it opens upward, no taller than the room above.
        let placed = place(
            at(1000.0, 700.0, 28.0, 28.0),
            316.0,
            Hang::BelowRight,
            WINDOW,
        );
        assert_eq!(placed.top, None);
        assert_eq!(placed.bottom, Some(800.0 - 694.0));
        assert_eq!(placed.max_height, 694.0 - 8.0);
        assert!(inside(placed, WINDOW));
        // Beside something near the bottom, it rises to keep some room.
        let placed = place(at(0.0, 760.0, 52.0, 30.0), 360.0, Hang::Beside, WINDOW);
        assert_eq!(placed.top, Some(800.0 - 8.0 - 200.0));
        assert!(inside(placed, WINDOW));
        // Larger interface sizes keep larger margins.
        let placed = hang(
            at(1200.0, 6.0, 28.0, 28.0),
            340.0,
            Hang::BelowLeft,
            size(px(1280.0), px(800.0)),
            2.0,
        );
        assert_eq!(placed.left, 1280.0 - 16.0 - 340.0);
    }
}
