//! The colours in effect: a preset plus `[colors]` overrides. Interface colours come from the
//! presets taken from Saddle (`preset.rs`); the terminal palette is paddock's own, since paddock has
//! no outer terminal to defer to. Named and default colours resolve to concrete RGB through that
//! palette.
use crate::preset::Color;
use crate::preset::{Frost, Preset, parse_color};
use crate::{
    config::Config,
    palette::{self, Rgb},
};
use anyhow::{Result, anyhow, bail};

/// How much of the terminal's background the window's ground around the panes keeps.
const FRAME: f32 = 0.55;

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

/// Catppuccin Mocha (https://github.com/catppuccin/alacritty, MIT, Copyright (c) 2021
/// Catppuccin): the terminal colours as published, with bright black (surface2, #585b70)
/// lightened to #676a7d to reach 3:1 on base, and the rosewater cursor. Catppuccin's terminals
/// select in rosewater with the text turned base; paddock keeps the text's own colours, so the
/// selection is Catppuccin's style-guide one instead, overlay2 at 30% over base.
const CATPPUCCIN: Terminal = Terminal {
    ansi: [
        rgb(0x45475a),
        rgb(0xf38ba8),
        rgb(0xa6e3a1),
        rgb(0xf9e2af),
        rgb(0x89b4fa),
        rgb(0xf5c2e7),
        rgb(0x94e2d5),
        rgb(0xbac2de),
        rgb(0x676a7d),
        rgb(0xf38ba8),
        rgb(0xa6e3a1),
        rgb(0xf9e2af),
        rgb(0x89b4fa),
        rgb(0xf5c2e7),
        rgb(0x94e2d5),
        rgb(0xa6adc8),
    ],
    cursor: rgb(0xf5e0dc),
    selection: rgb(0x414356),
};

/// Tokyo Night, night style (https://github.com/folke/tokyonight.nvim `extras/kitty`, Apache-2.0,
/// by Folke Lemaitre; after https://github.com/enkia/tokyo-night-vscode-theme, MIT, Copyright (c)
/// 2018-present Enkia): the terminal colours, cursor and selection as published, with bright
/// black (#414868) lightened to #616782 to reach 3:1 on its background.
const TOKYO_NIGHT: Terminal = Terminal {
    ansi: [
        rgb(0x15161e),
        rgb(0xf7768e),
        rgb(0x9ece6a),
        rgb(0xe0af68),
        rgb(0x7aa2f7),
        rgb(0xbb9af7),
        rgb(0x7dcfff),
        rgb(0xa9b1d6),
        rgb(0x616782),
        rgb(0xff899d),
        rgb(0x9fe044),
        rgb(0xfaba4a),
        rgb(0x8db0ff),
        rgb(0xc7a9ff),
        rgb(0xa4daff),
        rgb(0xc0caf5),
    ],
    cursor: rgb(0xc0caf5),
    selection: rgb(0x283457),
};

/// Rosé Pine, main variant (https://github.com/rose-pine/alacritty, MIT, Copyright (c) Rosé
/// Pine): the terminal colours, cursor (highlight high) and selection (highlight med) as
/// published; its bright colours repeat the normal ones, and every one reaches 3:1 on base.
const ROSE_PINE: Terminal = Terminal {
    ansi: [
        rgb(0x26233a),
        rgb(0xeb6f92),
        rgb(0x31748f),
        rgb(0xf6c177),
        rgb(0x9ccfd8),
        rgb(0xc4a7e7),
        rgb(0xebbcba),
        rgb(0xe0def4),
        rgb(0x6e6a86),
        rgb(0xeb6f92),
        rgb(0x31748f),
        rgb(0xf6c177),
        rgb(0x9ccfd8),
        rgb(0xc4a7e7),
        rgb(0xebbcba),
        rgb(0xe0def4),
    ],
    cursor: rgb(0x524f67),
    selection: rgb(0x403d52),
};

/// Kanagawa, wave variant (https://github.com/rebelot/kanagawa.nvim `extras/alacritty`, MIT,
/// Copyright (c) 2021 Tommaso Laurenzi): the terminal colours and selection as published, the
/// cursor oldWhite as its kitty colours have it; every one reaches 3:1 on sumiInk3.
const KANAGAWA: Terminal = Terminal {
    ansi: [
        rgb(0x090618),
        rgb(0xc34043),
        rgb(0x76946a),
        rgb(0xc0a36e),
        rgb(0x7e9cd8),
        rgb(0x957fb8),
        rgb(0x6a9589),
        rgb(0xc8c093),
        rgb(0x727169),
        rgb(0xe82424),
        rgb(0x98bb6c),
        rgb(0xe6c384),
        rgb(0x7fb4ca),
        rgb(0x938aa9),
        rgb(0x7aa89f),
        rgb(0xdcd7ba),
    ],
    cursor: rgb(0xc8c093),
    selection: rgb(0x2d4f67),
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
    if name == "terminal" {
        bail!(
            "theme \"terminal\" is not supported: it follows the outer terminal's colours, and \
             paddock has none; use {}",
            Preset::expected()
        );
    }
    Preset::parse(name).map_err(|message| anyhow!(message))
}

pub struct Theme {
    saddle: crate::preset::Theme,
    terminal: palette::Theme,
    frost: Frost,
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
            Preset::Dune => &DUNE,
            Preset::Tide => &TIDE,
            Preset::Lagoon => &LAGOON,
            Preset::Catppuccin => &CATPPUCCIN,
            Preset::TokyoNight => &TOKYO_NIGHT,
            Preset::RosePine => &ROSE_PINE,
            Preset::Kanagawa => &KANAGAWA,
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
        Ok(Theme {
            saddle,
            terminal,
            frost: preset.frost(),
        })
    }

    /// A Saddle interface colour used as text; `Reset` is the terminal's default foreground.
    pub fn fg(&self, pick: fn(&crate::preset::Theme) -> Color) -> palette::Rgb {
        resolve(&self.terminal, pick(&self.saddle), self.terminal.foreground)
    }

    /// A Saddle interface colour used as a background; `Reset` is the terminal's default
    /// background.
    pub fn bg(&self, pick: fn(&crate::preset::Theme) -> Color) -> palette::Rgb {
        resolve(&self.terminal, pick(&self.saddle), self.terminal.background)
    }

    /// How the sidebar's column lies over the system's sidebar material: the preset's, whatever
    /// `[colors]` sets.
    pub fn frost(&self) -> Frost {
        self.frost
    }

    /// These colours for the sidebar's column over the system's sidebar material: `agents_dim`
    /// and `agents_dimmer`, as `[colors]` leaves them, stepped towards `agents_text` as far as
    /// [`Self::frost`] says; every other colour as it is.
    pub fn frosted(&self) -> Theme {
        let text = self.fg(|t| t.agents_text);
        let toward = |(r, g, b): Rgb, share: f32| {
            let step = |from: u8, to: u8| {
                (f32::from(from) + (f32::from(to) - f32::from(from)) * share).round() as u8
            };
            Color::Rgb(step(r, text.0), step(g, text.1), step(b, text.2))
        };
        let mut saddle = self.saddle.clone();
        saddle.agents_dim = toward(self.fg(|t| t.agents_dim), self.frost.dim);
        saddle.agents_dimmer = toward(self.fg(|t| t.agents_dimmer), self.frost.dimmer);
        Theme {
            saddle,
            terminal: self.terminal,
            frost: self.frost,
        }
    }

    /// The window's ground around and between the panes' panels: the terminal's background a
    /// step darker, so each panel stands off it (DESIGN §13 P5-59).
    pub fn frame(&self) -> Rgb {
        let darker = |channel: u8| (f32::from(channel) * FRAME).round() as u8;
        let (r, g, b) = self.terminal.background;
        (darker(r), darker(g), darker(b))
    }

    /// The terminal pane's colours.
    pub fn terminal(&self) -> &palette::Theme {
        &self.terminal
    }

    /// A `[colors]` key's colour in this theme, as RGB, for swatches.
    pub fn color(&self, key: &str) -> Option<Rgb> {
        if let Some(i) = TERMINAL_KEYS.iter().position(|k| *k == key) {
            let mut terminal = self.terminal;
            return Some(*terminal_slots(&mut terminal)[i]);
        }
        let mut saddle = self.saddle.clone();
        let color = *saddle
            .named_mut()
            .into_iter()
            .find(|(name, _)| *name == key)?
            .1;
        // The terminal's default stands in for an unset colour: its background for backgrounds.
        let background = key.ends_with("bg") || key.ends_with("selected") || key == "overlay";
        let reset = if background {
            self.terminal.background
        } else {
            self.terminal.foreground
        };
        Some(resolve(&self.terminal, color, reset))
    }

    /// A `[colors]` key's value in this theme as the config file writes it.
    pub fn written(&self, key: &str) -> Option<String> {
        if let Some(i) = TERMINAL_KEYS.iter().position(|k| *k == key) {
            let mut terminal = self.terminal;
            let (r, g, b) = *terminal_slots(&mut terminal)[i];
            return Some(format!("#{r:02x}{g:02x}{b:02x}"));
        }
        let mut saddle = self.saddle.clone();
        let color = *saddle
            .named_mut()
            .into_iter()
            .find(|(name, _)| *name == key)?
            .1;
        Some(crate::preset::color_name(color))
    }
}

/// Every `[colors]` key: Saddle's interface colours, then the terminal's.
pub fn color_keys() -> Vec<&'static str> {
    let mut saddle = crate::preset::Theme::default();
    let mut keys: Vec<&'static str> = saddle
        .named_mut()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    keys.extend(TERMINAL_KEYS);
    keys
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

    /// WCAG contrast: relative luminance from linearized sRGB.
    fn contrast(a: Rgb, b: Rgb) -> f64 {
        fn luminance((r, g, b): Rgb) -> f64 {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn preset_ansi_colors_have_at_least_three_to_one_contrast() {
        for preset in Preset::ALL {
            let name = preset.name();
            let theme = theme(&format!("theme = \"{name}\"")).unwrap();
            let terminal = theme.terminal();
            for (index, &color) in terminal.ansi.iter().enumerate().skip(1) {
                let contrast = contrast(color, terminal.background);
                assert!(
                    contrast >= 3.0,
                    "{name} ANSI {index}: {contrast:.4}:1 < 3:1"
                );
            }
        }
    }

    #[test]
    fn quiet_text_and_status_colors_read_on_their_grounds() {
        type Pick = fn(&crate::preset::Theme) -> Color;
        for preset in Preset::ALL {
            let name = preset.name();
            let theme = theme(&format!("theme = \"{name}\"")).unwrap();
            let (ground, sidebar) = (theme.bg(|t| t.bg), theme.bg(|t| t.agents_bg));
            let muted = contrast(theme.fg(|t| t.muted), ground);
            assert!(muted >= 4.5, "{name} muted: {muted:.2}:1 < 4.5:1");
            let quiet: [(&str, Pick, f64); 8] = [
                ("agents_dim", |t| t.agents_dim, 4.5),
                ("agents_dimmer", |t| t.agents_dimmer, 3.0),
                ("agents_green", |t| t.agents_green, 4.5),
                ("agents_red", |t| t.agents_red, 4.5),
                ("agents_blue", |t| t.agents_blue, 4.5),
                ("agents_yellow", |t| t.agents_yellow, 4.5),
                ("agent_stalled", |t| t.agent_stalled, 4.5),
                ("agent_starting", |t| t.agent_starting, 4.5),
            ];
            for (key, pick, least) in quiet {
                let contrast = contrast(theme.fg(pick), sidebar);
                assert!(
                    contrast >= least,
                    "{name} {key}: {contrast:.2}:1 < {least}:1"
                );
            }
        }
    }

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

    fn theme(config: &str) -> Result<Theme> {
        Theme::from_config(&Config::parse(config).unwrap())
    }

    fn error(config: &str) -> String {
        format!("{:#}", theme(config).err().expect("an error"))
    }

    #[test]
    fn presets_by_name() {
        let dune = theme("").unwrap();
        assert_eq!(dune.terminal().ansi, DUNE.ansi);
        assert_eq!(
            theme("theme = \"dune\"").unwrap().terminal().ansi,
            DUNE.ansi
        );
        // Dune's text and background are the outer terminal's; its Agents panel colours stand in.
        let saddle = Preset::Dune.theme();
        assert_eq!(
            dune.terminal().foreground,
            resolve(dune.terminal(), saddle.agents_text, (0, 0, 0))
        );
        assert_eq!(
            dune.terminal().background,
            resolve(dune.terminal(), saddle.agents_bg, (0, 0, 0))
        );

        for (name, preset, own) in [
            ("tide", Preset::Tide, &TIDE),
            ("lagoon", Preset::Lagoon, &LAGOON),
            ("catppuccin", Preset::Catppuccin, &CATPPUCCIN),
            ("tokyonight", Preset::TokyoNight, &TOKYO_NIGHT),
            ("rosepine", Preset::RosePine, &ROSE_PINE),
            ("kanagawa", Preset::Kanagawa, &KANAGAWA),
        ] {
            let theme = theme(&format!("theme = \"{name}\"")).unwrap();
            let saddle = preset.theme();
            let terminal = theme.terminal();
            assert_eq!(terminal.ansi, own.ansi, "{name}");
            assert_eq!(
                (terminal.cursor, terminal.selection),
                (own.cursor, own.selection)
            );
            assert_eq!(Color::from(terminal.foreground), saddle.text);
            assert_eq!(Color::from(terminal.background), saddle.bg);
            assert_eq!(
                Color::from(theme.bg(|t| t.agents_bg)),
                saddle.agents_bg,
                "{name}"
            );
        }

        // The sidebar and title bar colours change with the theme.
        type Pick = fn(&crate::preset::Theme) -> Color;
        let picks: [(&str, Pick); 3] = [
            ("agents_bg", |t| t.agents_bg),
            ("agents_text", |t| t.agents_text),
            ("border", |t| t.border),
        ];
        for (key, pick) in picks {
            let colors = Preset::ALL.map(|p| {
                theme(&format!("theme = \"{}\"", p.name()))
                    .unwrap()
                    .fg(pick)
            });
            for (i, a) in colors.iter().enumerate() {
                for b in &colors[i + 1..] {
                    assert_ne!(a, b, "{key}: {colors:?}");
                }
            }
        }
    }

    #[test]
    fn the_ground_under_the_panes_is_a_step_darker_than_the_terminal_in_every_theme() {
        let sum = |(r, g, b): Rgb| u32::from(r) + u32::from(g) + u32::from(b);
        for preset in Preset::ALL {
            let theme = theme(&format!("theme = \"{}\"", preset.name())).unwrap();
            let (background, frame) = (theme.terminal().background, theme.frame());
            assert!(
                frame.0 <= background.0 && frame.1 <= background.1 && frame.2 <= background.2,
                "{}: {frame:?} over {background:?}",
                preset.name()
            );
            assert!(
                sum(frame) * 10 <= sum(background) * 7,
                "{}: {frame:?} over {background:?}",
                preset.name()
            );
        }
    }

    #[test]
    fn terminal_and_unknown_themes_are_errors() {
        let message = error("theme = \"terminal\"");
        assert!(
            message.contains("\"terminal\" is not supported"),
            "{message}"
        );
        for name in ["solarized", "Dune", "", "rose-pine", "Tokyo Night"] {
            let message = error(&format!("theme = \"{name}\""));
            assert!(
                message.contains(&format!("unknown theme {name:?}")),
                "{message}"
            );
            assert!(
                message.contains(
                    "expected dune, tide, lagoon, catppuccin, tokyonight, rosepine or kanagawa"
                ),
                "{message}"
            );
        }
    }

    #[test]
    fn overrides_merge_onto_the_preset() {
        let theme = theme(
            r##"
theme = "tide"
[colors]
agents_bg = "#010203"
border = "red"
text = "#aabbcc"
terminal_blue = "#0a0b0c"
terminal_cursor = "light_red"
terminal_selection = "default"
"##,
        )
        .unwrap();
        let terminal = theme.terminal();
        // Interface colours: overridden ones as written, names through the terminal palette.
        assert_eq!(theme.bg(|t| t.agents_bg), (1, 2, 3));
        assert_eq!(theme.fg(|t| t.border), TIDE.ansi[1]);
        assert_eq!(
            Color::from(theme.fg(|t| t.muted)),
            Preset::Tide.theme().muted
        );
        // Terminal colours: RGB as written, a name takes that entry, default keeps the theme's.
        assert_eq!(terminal.ansi[4], (10, 11, 12));
        assert_eq!(terminal.ansi[5], TIDE.ansi[5]);
        assert_eq!(terminal.cursor, TIDE.ansi[9]);
        assert_eq!(terminal.selection, TIDE.selection);
        // The terminal's text follows Saddle's `text`; its background still follows `bg`.
        assert_eq!(terminal.foreground, (0xaa, 0xbb, 0xcc));
        assert_eq!(Color::from(terminal.background), Preset::Tide.theme().bg);
    }

    #[test]
    fn names_and_indexes_follow_the_overridden_palette() {
        let theme = theme(
            r##"
[colors]
terminal_dark_gray = "#123456"
terminal_blue = "#0a0b0c"
terminal_cursor = "blue"
"##,
        )
        .unwrap();
        // Dune's border is dark gray.
        assert_eq!(theme.fg(|t| t.border), (0x12, 0x34, 0x56));
        assert_eq!(theme.terminal().cursor, (10, 11, 12));
        assert_eq!(theme.fg(|_| Color::Indexed(4)), (10, 11, 12));
        assert_eq!(theme.fg(|_| Color::Indexed(21)), (0, 0, 255));
        assert_eq!(theme.fg(|_| Color::Indexed(232)), (8, 8, 8));
    }

    #[test]
    fn terminal_text_and_bg_win_over_saddle_text_and_bg() {
        let theme = theme(
            r##"
theme = "lagoon"
[colors]
bg = "#ffffff"
terminal_bg = "#000001"
terminal_text = "white"
"##,
        )
        .unwrap();
        assert_eq!(theme.terminal().background, (0, 0, 1));
        assert_eq!(theme.terminal().foreground, LAGOON.ansi[15]);
        assert_eq!(theme.bg(|t| t.bg), (255, 255, 255));
        // Reset in the interface still means the terminal's own default.
        assert_eq!(theme.bg(|_| Color::Reset), (0, 0, 1));
    }

    #[test]
    fn unknown_keys_and_bad_values_name_the_key() {
        for (config, key) in [
            (
                "[colors]\nagent_bg = \"#000000\"",
                "unknown color \"agent_bg\"",
            ),
            (
                "[colors]\nterminal_foreground = \"#000000\"",
                "unknown color \"terminal_foreground\"",
            ),
            (
                "[colors]\nterminal_red = \"#12345\"",
                "terminal_red: invalid color",
            ),
            ("[colors]\nfocus = \"purple\"", "focus: invalid color"),
            ("[colors]\nterminal_bg = \"\"", "terminal_bg: invalid color"),
        ] {
            let message = error(config);
            assert!(message.contains(key), "{message}");
        }
    }
}
