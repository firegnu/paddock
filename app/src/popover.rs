//! The look the window's small floating panels share: a panel a shade lighter than the sidebar
//! with a faint edge and a soft shadow, rows of an icon, a label and a quiet shortcut that light
//! up under the mouse, faint rules between groups, and an inline choice of two or three. Only the
//! sidebar's menu uses it so far; the new tab, split and Attention panels are to follow.
use crate::{
    fonts::UiFont,
    footer_icon::{self, Icon},
    theme::Theme,
    view::hsla,
};
use gpui::{
    BoxShadow, Div, ElementId, FontWeight, Hsla, SharedString, Stateful, div, point, prelude::*, px,
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
const ROW_GROUP: &str = "popover-row";

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
