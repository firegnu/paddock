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
/// The two shadows outside: how far down, how soft and how dark; light, so the box sits just off
/// the card (P5-186).
const NEAR: (f32, f32, f32) = (1.0, 2.0, 0.20);
const FAR: (f32, f32, f32) = (3.0, 8.0, 0.12);
/// On a light theme (P5-175) the face and the light line are white, as the dark text would sink
/// the box: the face this much white over the card, the line this much.
const LIGHT_THEME_FACE: f32 = 0.6;
const LIGHT_THEME_LINE: f32 = 0.9;

pub struct Raised {
    pub face: Hsla,
    pub edge: Hsla,
    pub shadows: Vec<BoxShadow>,
}

/// The face alone, for what is drawn on it.
pub fn face(theme: &Theme) -> Hsla {
    let lift = if theme.is_light() {
        gpui::white().opacity(LIGHT_THEME_FACE)
    } else {
        text(theme).opacity(FACE)
    };
    hsla(theme.terminal().background, 1.0).blend(lift)
}

pub fn raised(theme: &Theme, ui: &UiFont) -> Raised {
    let text = text(theme);
    let black = gpui::black();
    let outside = |(down, blur, dark): (f32, f32, f32)| {
        BoxShadow::new(px(0.0), ui.px(down), black.opacity(theme.shade(dark)))
            .blur_radius(ui.px(blur))
    };
    let line = if theme.is_light() {
        gpui::white().opacity(LIGHT_THEME_LINE)
    } else {
        text.opacity(LIGHT)
    };
    Raised {
        face: face(theme),
        edge: text.opacity(EDGE),
        shadows: vec![
            BoxShadow::new(px(0.0), px(1.0), line).inset(),
            BoxShadow::new(px(0.0), px(-1.0), black.opacity(theme.shade(DARK))).inset(),
            outside(NEAR),
            outside(FAR),
        ],
    }
}

fn text(theme: &Theme) -> Hsla {
    hsla(theme.fg(|t| t.agents_text), 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, preset::Preset};

    /// A dark theme's box is drawn as before the light theme came; the light one's face is lighter
    /// than its card, its top line white and its shadows softer (P5-175).
    #[test]
    fn a_box_lifts_in_the_text_colour_on_a_dark_theme_and_in_white_on_a_light_one() {
        let ui = UiFont {
            family: None,
            size: 13.0,
        };
        for preset in Preset::ALL {
            let theme = Theme::from_config(&Config {
                theme: Some(preset.name().into()),
                ..Config::default()
            })
            .unwrap();
            let card = hsla(theme.terminal().background, 1.0);
            let text = text(&theme);
            let raised = raised(&theme, &ui);
            if theme.is_light() {
                assert!(raised.face.l > card.l, "{}", preset.name());
                assert_eq!(
                    raised.shadows[0].color,
                    gpui::white().opacity(LIGHT_THEME_LINE)
                );
                assert!(raised.shadows[2].color.a < NEAR.2 && raised.shadows[3].color.a < FAR.2);
            } else {
                assert_eq!(
                    raised.face,
                    card.blend(text.opacity(FACE)),
                    "{}",
                    preset.name()
                );
                assert_eq!(raised.shadows[0].color, text.opacity(LIGHT));
                assert_eq!(raised.shadows[1].color, gpui::black().opacity(DARK));
                assert_eq!(raised.shadows[2].color, gpui::black().opacity(NEAR.2));
                assert_eq!(raised.shadows[3].color, gpui::black().opacity(FAR.2));
            }
        }
    }

    /// The box sits just off the card, not floating over it (P5-186, the mockup's “light”): a near
    /// shadow 1pt down, 2pt soft; a far one 3pt down, 8pt soft; both faint.
    #[test]
    fn a_box_sits_just_off_the_card() {
        let ui = UiFont {
            family: None,
            size: 13.0,
        };
        let theme = Theme::from_config(&Config::default()).unwrap();
        let raised = raised(&theme, &ui);
        let outside: Vec<_> = raised.shadows[2..]
            .iter()
            .map(|s| {
                (
                    f32::from(s.offset.y),
                    f32::from(s.blur_radius),
                    (s.color.a * 100.0).round(),
                )
            })
            .collect();
        assert_eq!(outside, [(1.0, 2.0, 20.0), (3.0, 8.0, 12.0)]);
    }
}
