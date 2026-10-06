//! The small icon a card shows after an agent's name for its kind, compiled in from
//! `assets/kinds/` (where the README says where each comes from). GPUI draws an SVG as a mask in
//! one colour, so each takes the theme's colour for its kind.
use gpui::{Hsla, IntoElement, Pixels, Styled, svg};

/// A kind's icon and how wide it is drawn for its height. The official files are used as they
/// are, so their widths even out their weight: pi's solid badge is drawn smaller, omp's wide,
/// thin-legged π larger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KindIcon {
    pub data: &'static [u8],
    pub width: f32,
}

/// The icon for `kind`, matched as the kind colours are; `None` for a kind without one.
pub fn of(kind: &str) -> Option<KindIcon> {
    let (data, width): (&'static [u8], _) = match kind.to_ascii_lowercase().as_str() {
        "claude" => (include_bytes!("../assets/kinds/claude.svg"), 1.0),
        "codex" => (include_bytes!("../assets/kinds/codex.svg"), 1.0),
        "pi" => (include_bytes!("../assets/kinds/pi.svg"), 0.8),
        "omp" => (include_bytes!("../assets/kinds/omp.svg"), 1.15),
        _ => return None,
    };
    Some(KindIcon { data, width })
}

impl KindIcon {
    /// The icon `height` tall in `color`, centred in its width.
    pub fn render(self, height: Pixels, color: Hsla) -> impl IntoElement {
        svg()
            .data(self.data)
            .flex_shrink_0()
            .w(height * self.width)
            .h(height)
            .text_color(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_known_kind_has_its_own_icon_and_others_none() {
        let file = |kind: &str| {
            let icon = of(kind)?;
            ["claude", "codex", "pi", "omp"].into_iter().find(|name| {
                let path = format!("{}/assets/kinds/{name}.svg", env!("CARGO_MANIFEST_DIR"));
                std::fs::read(path).unwrap() == icon.data
            })
        };
        assert_eq!(file("claude"), Some("claude"));
        assert_eq!(file("Codex"), Some("codex"));
        assert_eq!(file("pi"), Some("pi"));
        assert_eq!(file("OMP"), Some("omp"));
        assert_eq!(file("gemini"), None);
        assert_eq!(file(""), None);
    }
}
