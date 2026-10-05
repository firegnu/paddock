//! Fonts: the interface font, which every window uses for everything but the terminal grid, and
//! the installed families the Settings font pickers offer.
use crate::config::Config;
use gpui::{App, Global, Pixels, SharedString, Styled, TextSystem, px};

/// The interface size the existing sizes were drawn for; every interface size scales from it, so
/// headings stay larger than text and notes smaller.
pub const BASE_SIZE: f32 = 13.0;
/// How Settings names the interface font when none is set: the system's own.
pub const SYSTEM: &str = "System (SF Pro)";

/// The interface font as configured, shared by every window.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFont {
    /// `None`: the system interface font.
    pub family: Option<SharedString>,
    pub size: f32,
}

impl Default for UiFont {
    fn default() -> Self {
        Self {
            family: None,
            size: BASE_SIZE,
        }
    }
}

impl Global for UiFont {}

impl UiFont {
    pub fn from_config(config: &Config) -> Self {
        Self {
            family: config
                .ui_font
                .as_deref()
                .map(str::trim)
                .filter(|family| !family.is_empty())
                .map(|family| family.to_owned().into()),
            size: config.ui_font_size,
        }
    }

    /// The one in use; the default before any is set.
    pub fn get(cx: &App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    /// A size drawn for [`BASE_SIZE`], at the configured size.
    pub fn scale(&self, points: f32) -> f32 {
        points * self.size / BASE_SIZE
    }

    pub fn px(&self, points: f32) -> Pixels {
        px(self.scale(points))
    }

    /// Sets the family on a window's root, for everything inside to inherit.
    pub fn apply<E: Styled>(&self, element: E) -> E {
        match &self.family {
            Some(family) => element.font_family(family.clone()),
            None => element,
        }
    }
}

/// The installed families, kept once read: all of them, and the monospace ones once measured.
#[derive(Clone, Debug, Default)]
pub struct Installed {
    pub all: Vec<String>,
    pub mono: Option<Vec<String>>,
}

impl Global for Installed {}

/// Installed families as Settings offers them: the hidden ones (named with a leading dot) left out.
pub fn visible(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| !name.starts_with('.') && !name.trim().is_empty())
        .collect()
}

/// Characters that differ in width in any proportional font.
const PROBE: [char; 4] = ['i', 'm', 'W', '.'];

/// Monospace when every probe character advances the same; one the font lacks means not.
pub fn monospace(advances: &[Option<f32>]) -> bool {
    let Some(first) = advances.first().copied().flatten() else {
        return false;
    };
    first > 0.0
        && advances
            .iter()
            .all(|advance| advance.is_some_and(|width| (width - first).abs() <= first * 0.01))
}

/// The monospace families among `names`, measured by the text system. Loads every family, so it
/// runs off the UI thread.
pub fn monospace_families(text: &TextSystem, names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            let id = text.resolve_font(&gpui::font(name.to_string()));
            let advances: Vec<Option<f32>> = PROBE
                .iter()
                .map(|&c| {
                    text.advance(id, px(16.0), c)
                        .ok()
                        .map(|size| f32::from(size.width))
                })
                .collect();
            monospace(&advances)
        })
        .cloned()
        .collect()
}

/// The names matching what was typed: every word of it, ignoring case, in their order.
pub fn matching<'a>(names: &'a [String], query: &str) -> Vec<&'a str> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    names
        .iter()
        .filter(|name| {
            let name = name.to_lowercase();
            words.iter().all(|word| name.contains(word.as_str()))
        })
        .map(String::as_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_advances_are_monospace() {
        assert!(monospace(&[Some(7.2), Some(7.2), Some(7.2), Some(7.2)]));
        // Rounding in the font's units is not a difference.
        assert!(monospace(&[Some(9.6), Some(9.6), Some(9.63), Some(9.6)]));
        assert!(!monospace(&[Some(3.5), Some(12.0), Some(14.0), Some(4.0)]));
        // A glyph the font lacks, or nothing measured.
        assert!(!monospace(&[Some(7.2), None, Some(7.2), Some(7.2)]));
        assert!(!monospace(&[None, Some(7.2)]));
        assert!(!monospace(&[]));
        assert!(!monospace(&[Some(0.0), Some(0.0)]));
    }

    #[test]
    fn hidden_families_are_not_offered() {
        let names = [".AppleSystemUIFont", "Menlo", "", "SF Pro", ".ZedMono"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(visible(names), ["Menlo", "SF Pro"]);
    }

    #[test]
    fn typing_narrows_by_every_word_ignoring_case() {
        let names = [
            "JetBrains Mono",
            "Menlo",
            "SF Mono",
            "Sarasa Mono SC",
            "Helvetica",
        ]
        .map(str::to_owned);
        assert_eq!(matching(&names, "").len(), 5);
        assert_eq!(
            matching(&names, "mono"),
            ["JetBrains Mono", "SF Mono", "Sarasa Mono SC"]
        );
        assert_eq!(matching(&names, "  MONO  sc "), ["Sarasa Mono SC"]);
        assert_eq!(matching(&names, "men"), ["Menlo"]);
        assert!(matching(&names, "courier").is_empty());
    }

    #[test]
    fn sizes_scale_from_the_base() {
        let default = UiFont::default();
        assert_eq!(default.scale(12.0), 12.0);
        assert_eq!(default.family, None);
        let config = Config {
            ui_font: Some(" Avenir Next ".into()),
            ui_font_size: 15.6,
            ..Config::default()
        };
        let larger = UiFont::from_config(&config);
        assert_eq!(larger.family.as_deref(), Some("Avenir Next"));
        assert!((larger.scale(13.0) - 15.6).abs() < 1e-4);
        assert!((larger.scale(11.0) - 13.2).abs() < 1e-4);
        assert!(larger.scale(20.0) > larger.scale(13.0));
        let blank = Config {
            ui_font: Some("  ".into()),
            ..Config::default()
        };
        assert_eq!(UiFont::from_config(&blank), UiFont::default());
    }
}
