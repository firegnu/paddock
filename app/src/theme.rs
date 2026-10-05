//! The colours in effect: Saddle's interface colours, read from its public theme, resolved to
//! concrete RGB through the terminal palette, since paddock has no outer terminal to defer to.
use crate::{config::Config, palette};
use anyhow::Result;
use ratatui::style::Color;

/// The pane colours the prototype shipped with; the xterm 16 colours live in `palette`.
const TERMINAL: palette::Theme = palette::Theme {
    foreground: (220, 220, 220),
    background: (24, 24, 27),
    cursor: (220, 220, 220),
};

pub struct Theme {
    saddle: saddle::theme::Theme,
    terminal: palette::Theme,
}

impl Theme {
    /// Built from the config; for now always Dune, `theme` and `[colors]` are not applied yet.
    pub fn from_config(_config: &Config) -> Result<Theme> {
        Ok(Theme {
            saddle: saddle::theme::Preset::Dune.theme(),
            terminal: TERMINAL,
        })
    }

    /// A Saddle interface colour used as text; `Reset` is the terminal's default foreground.
    pub fn fg(&self, pick: fn(&saddle::theme::Theme) -> Color) -> palette::Rgb {
        self.resolve(pick(&self.saddle), self.terminal.foreground)
    }

    /// A Saddle interface colour used as a background; `Reset` is the terminal's default
    /// background.
    pub fn bg(&self, pick: fn(&saddle::theme::Theme) -> Color) -> palette::Rgb {
        self.resolve(pick(&self.saddle), self.terminal.background)
    }

    /// The terminal pane's colours.
    pub fn terminal(&self) -> &palette::Theme {
        &self.terminal
    }

    /// Named and indexed colours mean what the terminal palette says they do.
    fn resolve(&self, color: Color, reset: palette::Rgb) -> palette::Rgb {
        let index = match color {
            Color::Reset => return reset,
            Color::Rgb(r, g, b) => return (r, g, b),
            Color::Indexed(index) => index,
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Magenta => 5,
            Color::Cyan => 6,
            Color::Gray => 7,
            Color::DarkGray => 8,
            Color::LightRed => 9,
            Color::LightGreen => 10,
            Color::LightYellow => 11,
            Color::LightBlue => 12,
            Color::LightMagenta => 13,
            Color::LightCyan => 14,
            Color::White => 15,
        };
        palette::indexed(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_is_the_terminal_default() {
        let theme = Theme::from_config(&Config::default()).unwrap();
        let terminal = theme.terminal();
        assert_ne!(terminal.foreground, terminal.background);
        assert_eq!(theme.fg(|_| Color::Reset), terminal.foreground);
        assert_eq!(theme.bg(|_| Color::Reset), terminal.background);
        // Dune leaves its text and background to the terminal.
        assert_eq!(theme.fg(|t| t.text), terminal.foreground);
        assert_eq!(theme.bg(|t| t.bg), terminal.background);
    }
}
