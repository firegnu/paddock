//! A well sunk into a ground, and the solid capsule of the accent set in it (P5-121a): the Cairn
//! tab's `Notes | Ledger | Lessons` and the left sidebar's machine switcher (P5-122) share it. The
//! well is the ground with black over it, less on a light ground; inside, a shadow under its top
//! edge and a dark rim; outside, a light lip under it and a faint edge in the text colour around
//! it. The capsule has a light line inside its top edge and a dark one inside its bottom edge, in
//! white and black; under it a small shadow and a faint glow of the accent.
use gpui::{BoxShadow, Hsla, px};

/// How much black the well takes, on a dark ground and on a light one; its shadow, rim, lip (dark,
/// light) and edge.
const WELL: (f32, f32) = (0.38, 0.07);
const WELL_TOP: f32 = 0.65;
const WELL_RIM: f32 = 0.45;
const WELL_LIP: (f32, f32) = (0.07, 0.70);
const WELL_EDGE: f32 = 0.06;
/// The capsule's light and dark lines, its shadow and its glow.
const PILL_LIGHT: f32 = 0.30;
const PILL_DARK: f32 = 0.18;
const PILL_SHADOW: f32 = 0.45;
const PILL_GLOW: f32 = 0.18;

pub struct Well {
    pub ground: Hsla,
    pub shadows: Vec<BoxShadow>,
}

fn shadow(y: f32, color: Hsla) -> BoxShadow {
    BoxShadow::new(px(0.0), px(y), color)
}

/// The well sunk into `under`, `light` or dark, its edge in `text`. Over a see-through ground (the
/// sidebar's material) it is the black alone, as see-through.
pub fn well(under: Hsla, light: bool, text: Hsla) -> Well {
    let (black, white) = (gpui::black(), gpui::white());
    let pick = |(dark, lit): (f32, f32)| if light { lit } else { dark };
    let dark = black.opacity(pick(WELL));
    Well {
        ground: if under.a >= 1.0 {
            under.blend(dark)
        } else {
            dark
        },
        shadows: vec![
            shadow(1.0, black.opacity(WELL_TOP))
                .blur_radius(px(2.0))
                .inset(),
            shadow(0.0, black.opacity(WELL_RIM))
                .spread_radius(px(1.0))
                .inset(),
            shadow(1.0, white.opacity(pick(WELL_LIP))),
            shadow(0.0, text.opacity(WELL_EDGE)).spread_radius(px(1.0)),
        ],
    }
}

/// The shadows of the capsule of `accent` set in the well.
pub fn capsule(accent: Hsla) -> Vec<BoxShadow> {
    let (black, white) = (gpui::black(), gpui::white());
    vec![
        shadow(1.0, white.opacity(PILL_LIGHT)).inset(),
        shadow(-1.0, black.opacity(PILL_DARK)).inset(),
        shadow(1.0, black.opacity(PILL_SHADOW)).blur_radius(px(2.0)),
        shadow(2.0, accent.opacity(PILL_GLOW)).blur_radius(px(6.0)),
    ]
}
