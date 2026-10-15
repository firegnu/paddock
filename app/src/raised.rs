//! A box raised off the right sidebar's card (P5-117): the Kanban tab's groups and the Cairn tab's
//! notes the next session starts from share it. Its face is solid, a shade lighter than the card,
//! so the shadows under it do not show through; a light line inside its top edge and a dark one
//! inside its bottom edge, as the left sidebar's cards (P5-51b); outside, a near small shadow and
//! a far soft one. Every colour comes from the theme.
use crate::{fonts::UiFont, theme::Theme, view::hsla};
use gpui::{BoxShadow, Hsla, px};

/// The face, the edge and the light line, in the text colour over the card's ground; the dark line,
/// in black.
const FACE: f32 = 0.055;
const EDGE: f32 = 0.12;
const LIGHT: f32 = 0.12;
const DARK: f32 = 0.40;
/// The two shadows outside: how far down, how soft and how dark.
const NEAR: (f32, f32, f32) = (2.0, 3.0, 0.45);
const FAR: (f32, f32, f32) = (8.0, 20.0, 0.35);

pub struct Raised {
    pub face: Hsla,
    pub edge: Hsla,
    pub shadows: Vec<BoxShadow>,
}

/// The face alone, for what is drawn on it.
pub fn face(theme: &Theme) -> Hsla {
    hsla(theme.terminal().background, 1.0).blend(text(theme).opacity(FACE))
}

pub fn raised(theme: &Theme, ui: &UiFont) -> Raised {
    let text = text(theme);
    let black = gpui::black();
    let outside = |(down, blur, dark): (f32, f32, f32)| {
        BoxShadow::new(px(0.0), ui.px(down), black.opacity(dark)).blur_radius(ui.px(blur))
    };
    Raised {
        face: face(theme),
        edge: text.opacity(EDGE),
        shadows: vec![
            BoxShadow::new(px(0.0), px(1.0), text.opacity(LIGHT)).inset(),
            BoxShadow::new(px(0.0), px(-1.0), black.opacity(DARK)).inset(),
            outside(NEAR),
            outside(FAR),
        ],
    }
}

fn text(theme: &Theme) -> Hsla {
    hsla(theme.fg(|t| t.agents_text), 1.0)
}
