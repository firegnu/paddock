//! The colours in effect: a preset plus `[colors]` overrides. Interface colours come from Saddle's
//! public theme; the terminal palette is paddock's own, since paddock has no outer terminal to
//! defer to. Named and default colours resolve to concrete RGB through that palette.
use crate::{
    config::Config,
    palette::{self, Rgb},
};
use anyhow::{Result, anyhow, bail};
use ratatui::style::Color;
use saddle::theme::{Preset, parse_color};

/// A preset's own terminal colours; its default text and background follow Saddle's `text`/`bg`.
struct Terminal {
    ansi: [Rgb; 16],
    cursor: Rgb,
    selection: Rgb,
}

const fn rgb(v: u32) -> Rgb {
    ((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Gruvbox dark (https://github.com/morhetz/gruvbox, MIT/X11, by morhetz): the terminal colours
/// as published, with dark0 as black, light4/gray as the two grays, light1 as white, cursor and
/// selection dark2.
const DUNE: Terminal = Terminal {
    ansi: [
        rgb(0x282828),
        rgb(0xcc241d),
        rgb(0x98971a),
        rgb(0xd79921),
        rgb(0x458588),
        rgb(0xb16286),
        rgb(0x689d6a),
        rgb(0xa89984),
        rgb(0x928374),
        rgb(0xfb4934),
        rgb(0xb8bb26),
        rgb(0xfabd2f),
        rgb(0x83a598),
        rgb(0xd3869b),
        rgb(0x8ec07c),
        rgb(0xebdbb2),
    ],
    cursor: rgb(0xebdbb2),
    selection: rgb(0x504945),
};

/// Nord (https://www.nordtheme.com, MIT, Copyright (c) 2016-present Sven Greb). Nord's bright
/// colours repeat the normal ones, so here they are the normal ones mixed 25% toward white, and
/// bright black (nord3, #4c566a) is lightened to #616e88 to reach 3:1 on Tide's background.
const TIDE: Terminal = Terminal {
    ansi: [
        rgb(0x3b4252),
        rgb(0xbf616a),
        rgb(0xa3be8c),
        rgb(0xebcb8b),
        rgb(0x81a1c1),
        rgb(0xb48ead),
        rgb(0x88c0d0),
        rgb(0xe5e9f0),
        rgb(0x616e88),
        rgb(0xcf898f),
        rgb(0xbacea9),
        rgb(0xf0d8a8),
        rgb(0xa1b9d1),
        rgb(0xc7aac2),
        rgb(0xabcdcc),
        rgb(0xeceff4),
    ],
    cursor: rgb(0xd8dee9),
    selection: rgb(0x434c5e),
};

/// Everforest dark (https://github.com/sainnhe/everforest, MIT, Copyright (c) 2019 sainnhe): its
/// foreground colours, with hard bg3 as black, grey2/grey1 as the two grays and fg as white. It has
/// no bright variants, so those are the normal ones mixed 25% toward white; the selection is
/// paddock's own deep teal.
const LAGOON: Terminal = Terminal {
    ansi: [
        rgb(0x374145),
        rgb(0xe67e80),
        rgb(0xa7c080),
        rgb(0xdbbc7f),
        rgb(0x7fbbb3),
        rgb(0xd699b6),
        rgb(0x83c092),
        rgb(0x9da9a0),
        rgb(0x859289),
        rgb(0xec9ea0),
        rgb(0xbdd0a0),
        rgb(0xe4cd9f),
        rgb(0x9fccc6),
        rgb(0xe0b3c8),
        rgb(0xa2d0ad),
        rgb(0xd3c6aa),
    ],
    cursor: rgb(0xd3c6aa),
    selection: rgb(0x284440),
};

/// The `[colors]` keys of the terminal colours: the 16 under Saddle's ANSI names, then text,
/// background, cursor and selection.
const TERMINAL_KEYS: [&str; 20] = [
    "terminal_black",
    "terminal_red",
    "terminal_green",
    "terminal_yellow",
    "terminal_blue",
    "terminal_magenta",
    "terminal_cyan",
    "terminal_gray",
    "terminal_dark_gray",
    "terminal_light_red",
    "terminal_light_green",
    "terminal_light_yellow",
    "terminal_light_blue",
    "terminal_light_magenta",
    "terminal_light_cyan",
    "terminal_white",
    "terminal_text",
    "terminal_bg",
    "terminal_cursor",
    "terminal_selection",
];

fn terminal_slots(t: &mut palette::Theme) -> [&mut Rgb; 20] {
    let [
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        a8,
        a9,
        a10,
        a11,
        a12,
        a13,
        a14,
        a15,
    ] = &mut t.ansi;
    [
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        a8,
        a9,
        a10,
        a11,
        a12,
        a13,
        a14,
        a15,
        &mut t.foreground,
        &mut t.background,
        &mut t.cursor,
        &mut t.selection,
    ]
}

fn preset(name: &str) -> Result<Preset> {
    match Preset::parse(name) {
        Ok(Preset::Terminal) => bail!(
            "theme \"terminal\" is not supported: it follows the outer terminal's colours, and \
             paddock has none; use dune, tide or lagoon"
        ),
        Ok(preset) => Ok(preset),
        Err(_) => bail!("unknown theme {name:?}: expected dune, tide or lagoon"),
    }
}

pub struct Theme {
    saddle: saddle::theme::Theme,
    terminal: palette::Theme,
}

impl Theme {
    /// The `theme` preset (Dune when not written) with the `[colors]` overrides applied.
    pub fn from_config(config: &Config) -> Result<Theme> {
        let preset = preset(config.theme.as_deref().unwrap_or("dune"))?;
        let base = preset.theme();
        let mut saddle = base.clone();
        let mut terminal_overrides = Vec::new();
        for (key, value) in &config.colors {
            let color = parse_color(value).map_err(|e| anyhow!("[colors] {key}: {e}"))?;
            if let Some((_, slot)) = saddle.named_mut().into_iter().find(|(name, _)| name == key) {
                *slot = color;
            } else if let Some(i) = TERMINAL_KEYS.iter().position(|name| name == key) {
                terminal_overrides.push((i, color));
            } else {
                bail!("[colors] unknown color {key:?}");
            }
        }

        let own = match preset {
            Preset::Tide => &TIDE,
            Preset::Lagoon => &LAGOON,
            _ => &DUNE,
        };
        let mut terminal = palette::Theme {
            ansi: own.ansi,
            foreground: (0, 0, 0),
            background: (0, 0, 0),
            cursor: own.cursor,
            selection: own.selection,
        };
        for &(i, color) in &terminal_overrides {
            if let Color::Rgb(r, g, b) = color {
                *terminal_slots(&mut terminal)[i] = (r, g, b);
            }
        }
        // Text and background follow Saddle's `text`/`bg`; where those are the outer terminal's
        // default (Dune), the preset's Agents panel colours stand in.
        let written = |i: usize| terminal_overrides.iter().any(|&(j, _)| j == i);
        if !written(16) {
            let fallback = resolve(&terminal, base.agents_text, (0, 0, 0));
            terminal.foreground = resolve(&terminal, saddle.text, fallback);
        }
        if !written(17) {
            let fallback = resolve(&terminal, base.agents_bg, (0, 0, 0));
            terminal.background = resolve(&terminal, saddle.bg, fallback);
        }
        // An ANSI name takes that colour after the RGB overrides; `default` keeps the theme's.
        let settled = terminal;
        for &(i, color) in &terminal_overrides {
            if !matches!(color, Color::Rgb(..) | Color::Reset) {
                *terminal_slots(&mut terminal)[i] = resolve(&settled, color, (0, 0, 0));
            }
        }
        Ok(Theme { saddle, terminal })
    }

    /// A Saddle interface colour used as text; `Reset` is the terminal's default foreground.
    pub fn fg(&self, pick: fn(&saddle::theme::Theme) -> Color) -> palette::Rgb {
        resolve(&self.terminal, pick(&self.saddle), self.terminal.foreground)
    }

    /// A Saddle interface colour used as a background; `Reset` is the terminal's default
    /// background.
    pub fn bg(&self, pick: fn(&saddle::theme::Theme) -> Color) -> palette::Rgb {
        resolve(&self.terminal, pick(&self.saddle), self.terminal.background)
    }

    /// The terminal pane's colours.
    pub fn terminal(&self) -> &palette::Theme {
        &self.terminal
    }
}

/// Named and indexed colours mean what the terminal palette says they do.
fn resolve(terminal: &palette::Theme, color: Color, reset: Rgb) -> Rgb {
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
    terminal.indexed(index)
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
