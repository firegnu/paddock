//! A well sunk into a ground, and the capsule of a switch set in it (P5-121a): the Cairn tab's
//! `Notes | Ledger | Lessons` and the left sidebar's machine switcher (P5-122) both have them, and
//! the capsule shown is drawn the same on either (10-17). The well is the ground with black over
//! it, less on a light ground; inside, a shadow under its top edge and a dark rim; outside, a
//! light lip under it and a faint edge in the text colour around it. The capsule shown is filled
//! faintly with the accent and ringed in it, its words the bright text colour and heavy; one not
//! shown carries no ring, its words in the text colour, brighter under the pointer. A count in a
//! capsule goes on a small round tag.
use crate::{
    fonts::UiFont,
    window::{RING_ACTIVE, TAB_RING},
};
use gpui::{BoxShadow, Div, FontWeight, Hsla, Stateful, div, prelude::*, px};

/// How much black the well takes, on a dark ground and on a light one; its shadow, rim, lip (dark,
/// light) and edge.
const WELL: (f32, f32) = (0.38, 0.07);
const WELL_TOP: f32 = 0.65;
const WELL_RIM: f32 = 0.45;
const WELL_LIP: (f32, f32) = (0.07, 0.70);
const WELL_EDGE: f32 = 0.06;
/// How much accent fills the capsule shown (10-17: a solid accent was too heavy in the sidebar).
const SHOWN_FILL: f32 = 0.18;
/// A count's ground in the text colour.
const COUNT: f32 = 0.12;

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

/// The capsule of a switch, `on` for the one shown: its ground a faint fill of `accent` ringed in
/// the accent, its words in `bright` and heavy; one not shown with no ring, its words in `text`,
/// brighter under the pointer. Both switches draw theirs with this, the sidebar's machine switcher
/// and the Cairn tab's pages alike (10-17).
pub fn chosen(
    pill: Stateful<Div>,
    on: bool,
    accent: Hsla,
    text: Hsla,
    bright: Hsla,
) -> Stateful<Div> {
    let pill = pill.rounded_full().border(px(TAB_RING));
    if on {
        pill.bg(accent.opacity(SHOWN_FILL))
            .border_color(accent.opacity(RING_ACTIVE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(bright)
    } else {
        pill.border_color(gpui::transparent_black())
            .text_color(text)
            .hover(move |style| style.text_color(bright))
    }
}

/// How many there are, on a small round tag in a capsule (the Cairn switch's pages, another
/// machine's agents): words in `text` on a faint ground of it.
pub fn count(count: usize, text: Hsla, ui: &UiFont) -> Div {
    div()
        .px(ui.px(6.0))
        .rounded_full()
        .text_size(ui.px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .bg(text.opacity(COUNT))
        .text_color(text)
        .child(count.to_string())
}
