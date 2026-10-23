//! Saddle's interface colours for the Dune, Tide and Lagoon presets, and how a configured colour is
//! written. From Saddle `src/theme.rs` at commit `df1c727`, kept apart from Saddle since then
//! (DESIGN §8). Not taken: the Terminal preset (paddock has no outer terminal), the 256-colour
//! fallback, `[colors]` deserialisation, and the ratatui block and layout helpers. Since M1 colours
//! are paddock's own `Color` below instead of ratatui's, with the same values. The Catppuccin
//! Mocha, Tokyo Night, Rosé Pine and Kanagawa presets are paddock's own (P5-40), as are their
//! light versions (P5-175, P5-176), from those palettes' published colours; each names its source.
//! So are Dune's, Tide's and Lagoon's light versions (P5-177), from a sample and the light
//! versions of the palettes their terminal colours come from.

/// A configured interface colour: the terminal's default, one of the 16 basic colours, an entry of
/// the 256-colour palette, or RGB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Color {
    #[default]
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}
impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Color::Rgb(r, g, b)
    }
}

pub const BG: Color = Color::Reset;
pub const OVERLAY: Color = Color::Reset;
pub const SELECTED: Color = Color::DarkGray;
// Subtle warm tint matched to the current Terminal theme.
pub const AGENT_SELECTED: Color = Color::Rgb(0x2b, 0x26, 0x21);
pub const AGENT_WORKING: Color = Color::Rgb(0x7f, 0xb4, 0xee);
pub const AGENT_IDLE: Color = Color::Rgb(0x9c, 0xbd, 0x80);
pub const AGENT_BLOCKED: Color = Color::Rgb(0xe6, 0xb5, 0x66);
pub const AGENT_STALLED: Color = Color::Rgb(0xe7, 0x9b, 0x65);
pub const AGENT_ERROR: Color = Color::Rgb(0xef, 0x81, 0x74);
pub const AGENT_STARTING: Color = Color::Rgb(0xb0, 0xa1, 0xd8);
pub const BORDER: Color = Color::DarkGray;
pub const TEXT: Color = Color::Reset;
pub const BRIGHT: Color = Color::White;
pub const MUTED: Color = Color::Gray;
pub const DIM: Color = Color::DarkGray;
pub const FOCUS: Color = Color::Yellow;
pub const CONNECTED: Color = Color::Cyan;
pub const WORKING: Color = Color::Blue;
pub const DANGER: Color = Color::Red;
pub const UNREAD: Color = Color::Magenta;
// The Agents panel's own palette (design 3a); other areas keep the shared colors above.
pub const AGENTS_BG: Color = Color::Rgb(0x1d, 0x1a, 0x16);
pub const AGENTS_BORDER: Color = Color::Rgb(0x5a, 0x52, 0x47);
pub const AGENTS_RULE: Color = Color::Rgb(0x3a, 0x35, 0x2e);
pub const AGENTS_FAINT: Color = Color::Rgb(0x3d, 0x38, 0x30);
pub const AGENTS_TEXT: Color = Color::Rgb(0xe8, 0xdf, 0xcc);
pub const AGENTS_BRANCH: Color = Color::Rgb(0xcf, 0xc6, 0xb2);
pub const AGENTS_DIM: Color = Color::Rgb(0x8c, 0x83, 0x74);
pub const AGENTS_DIMMER: Color = Color::Rgb(0x75, 0x6c, 0x5e);
pub const AGENTS_ACCENT: Color = Color::Rgb(0xbd, 0xb8, 0x6a);
pub const AGENTS_GREEN: Color = Color::Rgb(0xa8, 0xc4, 0x7c);
pub const AGENTS_RED: Color = Color::Rgb(0xe3, 0x72, 0x64);
pub const AGENTS_BLUE: Color = Color::Rgb(0x7f, 0xa9, 0xea);
pub const AGENTS_YELLOW: Color = Color::Rgb(0xd9, 0xb2, 0x5f);
pub const AGENTS_PURPLE: Color = Color::Rgb(0xa5, 0x8b, 0xdc);
pub const AGENTS_SANDBOX: Color = Color::Rgb(0x6c, 0xc4, 0xad);

/// How a preset's sidebar column lies over the system's sidebar material (P5-19b): `wash`, how
/// much of the sidebar's own colour is laid over the material; `lit`, how much of the text colour
/// lights a selected card, tile or button there (a hovered one takes half of it, the strip's rules
/// as much); `waiting`, how much amber a waiting card takes; `dim` and `dimmer`, how far the two
/// quiet text colours step towards the text colour there, to read on the brighter ground (P5-19c).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frost {
    pub wash: f32,
    pub lit: f32,
    pub waiting: f32,
    pub dim: f32,
    pub dimmer: f32,
}

/// A built-in palette that `[colors]` overrides; `dune` is the original look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preset {
    #[default]
    Dune,
    DuneLight,
    Tide,
    TideLight,
    Lagoon,
    LagoonLight,
    Catppuccin,
    Latte,
    TokyoNight,
    TokyoNightDay,
    RosePine,
    RosePineDawn,
    Kanagawa,
    KanagawaLotus,
}

impl Preset {
    pub const ALL: [Preset; 14] = [
        Preset::Dune,
        Preset::DuneLight,
        Preset::Tide,
        Preset::TideLight,
        Preset::Lagoon,
        Preset::LagoonLight,
        Preset::Catppuccin,
        Preset::Latte,
        Preset::TokyoNight,
        Preset::TokyoNightDay,
        Preset::RosePine,
        Preset::RosePineDawn,
        Preset::Kanagawa,
        Preset::KanagawaLotus,
    ];
    /// The light version of a dark preset, where it has one (P5-176, P5-177).
    pub fn light(self) -> Option<Preset> {
        match self {
            Preset::Dune => Some(Preset::DuneLight),
            Preset::Tide => Some(Preset::TideLight),
            Preset::Lagoon => Some(Preset::LagoonLight),
            Preset::Catppuccin => Some(Preset::Latte),
            Preset::TokyoNight => Some(Preset::TokyoNightDay),
            Preset::RosePine => Some(Preset::RosePineDawn),
            Preset::Kanagawa => Some(Preset::KanagawaLotus),
            _ => None,
        }
    }
    /// The presets as Settings groups them (P5-176): the dark ones in [`Self::ALL`]'s order, then
    /// the light ones in their dark versions' order, so a light version lines up under its dark one
    /// once each dark preset has one.
    pub fn groups() -> (Vec<Preset>, Vec<Preset>) {
        let dark: Vec<Preset> = Self::ALL
            .into_iter()
            .filter(|p| !Self::ALL.iter().any(|d| d.light() == Some(*p)))
            .collect();
        let light = dark.iter().filter_map(|p| p.light()).collect();
        (dark, light)
    }
    /// How the theme is written in the config file.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Dune => "dune",
            Preset::DuneLight => "dune-light",
            Preset::Tide => "tide",
            Preset::TideLight => "tide-light",
            Preset::Lagoon => "lagoon",
            Preset::LagoonLight => "lagoon-light",
            Preset::Catppuccin => "catppuccin",
            Preset::Latte => "latte",
            Preset::TokyoNight => "tokyonight",
            Preset::TokyoNightDay => "tokyonight-day",
            Preset::RosePine => "rosepine",
            Preset::RosePineDawn => "rosepine-dawn",
            Preset::Kanagawa => "kanagawa",
            Preset::KanagawaLotus => "kanagawa-lotus",
        }
    }
    /// How the theme is shown: the palette's own name for the borrowed ones.
    pub fn label(self) -> &'static str {
        match self {
            Preset::Dune => "Dune",
            Preset::DuneLight => "Dune Light",
            Preset::Tide => "Tide",
            Preset::TideLight => "Tide Light",
            Preset::Lagoon => "Lagoon",
            Preset::LagoonLight => "Lagoon Light",
            Preset::Catppuccin => "Catppuccin Mocha",
            Preset::Latte => "Catppuccin Latte",
            Preset::TokyoNight => "Tokyo Night",
            Preset::TokyoNightDay => "Tokyo Night Day",
            Preset::RosePine => "Rosé Pine",
            Preset::RosePineDawn => "Rosé Pine Dawn",
            Preset::Kanagawa => "Kanagawa",
            Preset::KanagawaLotus => "Kanagawa Lotus",
        }
    }
    /// The names as an error message lists them: "dune, tide, … or kanagawa".
    pub fn expected() -> String {
        let names = Self::ALL.map(Preset::name);
        let (last, rest) = names.split_last().expect("presets");
        format!("{} or {last}", rest.join(", "))
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == value)
            .ok_or_else(|| format!("unknown theme {value:?}: expected {}", Self::expected()))
    }
    /// How the sidebar's column lies over the system's sidebar material. The material is already
    /// dark and frosted; the wash only leans it to the preset's colour, and the grounds on it step
    /// up about half again as much as the preset's own selected colour does over its sidebar, as
    /// they lie on a busier ground. The quiet text steps up until, on a grey like the material's
    /// under a selected card (#45494a), `agents_dim` reads at about 4.6:1 and `agents_dimmer` at
    /// about 3.6:1, still apart from each other and from the text (about 7:1).
    pub fn frost(self) -> Frost {
        match self {
            // Every preset is washed alike, under half, so the frost still shows through while the
            // column keeps the terminal's hue (P5-65; P5-24b had most washed most of the way,
            // which read as a haze, and a quarter of the way, P5-64, read as torn from the
            // terminal). A warm brown, the nearest to the material's grey; its quiet text is the
            // darkest of the three, so it steps the furthest.
            Preset::Dune => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.11,
                dim: 0.56,
                dimmer: 0.48,
            },
            // Deeper, bluer and greener grounds, far from the material's grey. Their own selected
            // colours step further too. The darker ground keeps the quiet text at least as readable
            // as on the grey.
            Preset::Tide => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.46,
                dimmer: 0.40,
            },
            Preset::Lagoon => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.42,
                dimmer: 0.36,
            },
            // The four borrowed palettes lie on deep blue-violet grounds like Tide's, washed as
            // far, Kanagawa's nearly neutral ink too. Their quiet text is darker
            // than Tide's against their own text, so it steps further: Tokyo Night's, the
            // darkest, the furthest.
            Preset::Catppuccin => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.62,
                dimmer: 0.48,
            },
            // The light one lies over the material's light look (P5-175), washed as far; its grounds
            // step darker, in its dark text colour. Its cards are whole faces there, the selected
            // one the darkest (#d7cdef), and its quiet text steps towards the text until it reads
            // at about 4.6:1 and 3.6:1 on that.
            Preset::Latte => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.14,
                dim: 0.64,
                dimmer: 0.53,
            },
            Preset::TokyoNight => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.72,
                dimmer: 0.60,
            },
            Preset::RosePine => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.50,
                dimmer: 0.46,
            },
            Preset::Kanagawa => Frost {
                wash: 0.45,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.48,
                dimmer: 0.44,
            },
            // The other light ones (P5-176) lie as Latte does, their quiet text stepping as far,
            // Tokyo Night Day's `dim` a little further: on their darkest card face it reads at
            // 4.5:1 and 3.5:1 at least.
            Preset::TokyoNightDay => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.14,
                dim: 0.66,
                dimmer: 0.53,
            },
            Preset::RosePineDawn => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.14,
                dim: 0.64,
                dimmer: 0.53,
            },
            Preset::KanagawaLotus => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.14,
                dim: 0.64,
                dimmer: 0.53,
            },
            // Dune's, Tide's and Lagoon's light versions (P5-177) lie as Latte does.
            Preset::DuneLight | Preset::TideLight | Preset::LagoonLight => Frost {
                wash: 0.45,
                lit: 0.10,
                waiting: 0.14,
                dim: 0.64,
                dimmer: 0.53,
            },
        }
    }
    /// Every color of the theme.
    pub fn theme(self) -> Theme {
        match self {
            Preset::Dune => Theme::default(),
            Preset::DuneLight => dune_light(),
            Preset::Tide => tide(),
            Preset::TideLight => tide_light(),
            Preset::Lagoon => lagoon(),
            Preset::LagoonLight => lagoon_light(),
            Preset::Catppuccin => catppuccin(),
            Preset::Latte => latte(),
            Preset::TokyoNight => tokyo_night(),
            Preset::TokyoNightDay => tokyo_night_day(),
            Preset::RosePine => rose_pine(),
            Preset::RosePineDawn => rose_pine_dawn(),
            Preset::Kanagawa => kanagawa(),
            Preset::KanagawaLotus => kanagawa_lotus(),
        }
    }
}

/// Cool blue-gray neutrals on a cool background, which the terminal's default colors share; the
/// status, danger and agent-type hues keep their Dune roles.
fn tide() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x151b23),
        text: rgb(0xdce4ee),
        selected: rgb(0x243244),
        agent_selected: rgb(0x212b38),
        agent_working: rgb(0x7fa8f0),
        agent_idle: rgb(0x93c79a),
        agent_blocked: rgb(0xe0bd70),
        agent_stalled: rgb(0xe39e72),
        agent_error: rgb(0xec7f7a),
        agent_starting: rgb(0xaa9fe2),
        border: rgb(0x435166),
        bright: rgb(0xe8eef6),
        muted: rgb(0x8d9bb0),
        dim: rgb(0x56657a),
        focus: rgb(0x86c1e6),
        connected: rgb(0x6ccfb4),
        working: rgb(0x7a9cf0),
        danger: rgb(0xe06c75),
        unread: rgb(0xc38ae8),
        input_text: rgb(0x0e141b),
        reply_code: rgb(0xe2c98a),
        reply_heading: rgb(0x7cc4e8),
        agents_bg: rgb(0x151b23),
        agents_border: rgb(0x4a586b),
        agents_rule: rgb(0x2b3542),
        agents_faint: rgb(0x323d4b),
        agents_text: rgb(0xdce4ee),
        agents_branch: rgb(0xbfcad8),
        agents_dim: rgb(0x8595a9),
        agents_dimmer: rgb(0x6b7a8f),
        agents_accent: rgb(0x86c1e6),
        agents_green: rgb(0x9cc98e),
        agents_red: rgb(0xe47a74),
        agents_blue: rgb(0x7ea2ee),
        agents_yellow: rgb(0xdbbd6e),
        agents_purple: rgb(0xa99ae6),
        agents_sandbox: rgb(0x67c9a5),
        claude: rgb(0xe48c66),
        codex: rgb(0x6fd3c2),
        pi: rgb(0xeef2f7),
        omp: rgb(0xb07cf2),
        amp: rgb(0xe3cb73),
        agy: rgb(0x6aa6f2),
        ..Theme::default()
    }
}

/// Deep green-teal neutrals on the #0c1616 background, which the terminal's default colors
/// share; the status, danger and agent-type hues keep their Dune roles.
fn lagoon() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x0c1616),
        text: rgb(0xd6e6e1),
        selected: rgb(0x1d3833),
        agent_selected: rgb(0x172b2a),
        agent_working: rgb(0x7fa8f0),
        agent_idle: rgb(0x93c79a),
        agent_blocked: rgb(0xe0bd70),
        agent_stalled: rgb(0xe39e72),
        agent_error: rgb(0xec7f7a),
        agent_starting: rgb(0xaa9fe2),
        border: rgb(0x36524c),
        bright: rgb(0xe8f3ef),
        muted: rgb(0x8aa59e),
        dim: rgb(0x52706a),
        focus: rgb(0x7fd4bf),
        connected: rgb(0x6cc6dc),
        working: rgb(0x7a9cf0),
        danger: rgb(0xe06c75),
        unread: rgb(0xc38ae8),
        input_text: rgb(0x0a1414),
        reply_code: rgb(0xe2c98a),
        reply_heading: rgb(0x82cbd6),
        agents_bg: rgb(0x0c1616),
        agents_border: rgb(0x3a5751),
        agents_rule: rgb(0x1f3633),
        agents_faint: rgb(0x263f3b),
        agents_text: rgb(0xd6e6e1),
        agents_branch: rgb(0xb8cdc6),
        agents_dim: rgb(0x83a098),
        agents_dimmer: rgb(0x668580),
        agents_accent: rgb(0x7fd4bf),
        agents_green: rgb(0x9cc98e),
        agents_red: rgb(0xe47a74),
        agents_blue: rgb(0x7ea2ee),
        agents_yellow: rgb(0xdbbd6e),
        agents_purple: rgb(0xa99ae6),
        agents_sandbox: rgb(0x8fd18a),
        claude: rgb(0xe48c66),
        codex: rgb(0x6fd3c2),
        pi: rgb(0xeef5f2),
        omp: rgb(0xb07cf2),
        amp: rgb(0xe3cb73),
        agy: rgb(0x6aa6f2),
        ..Theme::default()
    }
}

/// Dune's light version (P5-177), paddock's own like Dune: the grounds, text, quiet text and rules
/// are the sample's warm sand (`docs/设计稿/P5-177-Dune-Tide-Lagoon浅色版/`): #f7f2e6 for the
/// terminal, #efe7d6 for the sidebar, #3c3836 text, #665c54 and #928374 quiet text, #d5c4a1 rules.
/// The selected ground is the sample's selected card over the sidebar (#e1d4b9). The rest come from
/// Gruvbox light (https://github.com/morhetz/gruvbox, MIT/X11, by morhetz) by Dune's roles, where
/// Dune names the terminal's colours: its faded hues for blue working, green idle, yellow waiting
/// and focus, orange stalled, red error, purple starting, aqua sandbox; light3 for edges, dark2 for
/// branches, its "white" (dark4) for muted, the faded green as the olive-gold accent. Those read as
/// text under 4.5:1 (3:1 for `agents_dimmer`) on their grounds are darkened just enough, towards
/// black: muted #7c6f64 to #796d62 on the terminal's ground; on the sidebar, gray #928374 to #918274
/// for the quieter text, green #79740e to #6f6b0d, yellow #b57614 to #8f5d10 and aqua #427b58 to
/// #3e7353. The keys nothing reads as text keep Gruvbox's own hues.
fn dune_light() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xf7f2e6),
        text: rgb(0x3c3836),
        selected: rgb(0xe1d4b9),
        agent_selected: rgb(0xe1d4b9),
        agent_working: rgb(0x076678),
        agent_idle: rgb(0x79740e),
        agent_blocked: rgb(0xb57614),
        agent_stalled: rgb(0xaf3a03),
        agent_error: rgb(0x9d0006),
        agent_starting: rgb(0x8f3f71),
        border: rgb(0xbdae93),
        bright: rgb(0x3c3836),
        muted: rgb(0x796d62),
        dim: rgb(0x928374),
        focus: rgb(0xb57614),
        connected: rgb(0x427b58),
        working: rgb(0x076678),
        danger: rgb(0x9d0006),
        unread: rgb(0x8f3f71),
        input_text: rgb(0xfbf1c7),
        reply_code: rgb(0xb57614),
        reply_heading: rgb(0x427b58),
        agents_bg: rgb(0xefe7d6),
        agents_border: rgb(0xbdae93),
        agents_rule: rgb(0xd5c4a1),
        agents_faint: rgb(0xd5c4a1),
        agents_text: rgb(0x3c3836),
        agents_branch: rgb(0x504945),
        agents_dim: rgb(0x665c54),
        agents_dimmer: rgb(0x918274),
        agents_accent: rgb(0x79740e),
        agents_green: rgb(0x6f6b0d),
        agents_red: rgb(0x9d0006),
        agents_blue: rgb(0x076678),
        agents_yellow: rgb(0x8f5d10),
        agents_purple: rgb(0x8f3f71),
        agents_sandbox: rgb(0x3e7353),
        claude: rgb(0xaf3a03),
        codex: rgb(0x427b58),
        pi: rgb(0x3c3836),
        omp: rgb(0x8f3f71),
        amp: rgb(0xb57614),
        agy: rgb(0x076678),
        ..Theme::default()
    }
}

/// Tide's light version (P5-177), paddock's own like Tide, in Nord (https://www.nordtheme.com, MIT,
/// Copyright (c) 2016-present Sven Greb): the sample's grounds, text, quiet text and rules
/// (`docs/设计稿/P5-177-Dune-Tide-Lagoon浅色版/`), Snow Storm nord6 #eceff4 for the terminal and nord5
/// #e5e9f0 for the sidebar, Polar Night nord0 #2e3440 for text, nord3 #4c566a and #7b88a1 for the
/// quiet text, nord4 #d8dee9 for rules. The selected ground is the sample's selected card over the
/// sidebar (#dae0ea); edges lie halfway from the rules to the quieter text (#aab3c5), branches are
/// nord2. Status hues by Tide's roles, as the sample deepens Nord's for a light ground: frost
/// nord10 working, green #5f7f4a idle, yellow #9a7a2e waiting, orange #c0704a stalled, aurora red
/// nord11 error, purple #9a6e93 starting; the accent is the deepened frost nord8 (#3b8ba3), and the
/// sandbox halfway from nord7 to nord14 (#99bda4), Tide's green-teal. Those read as text under
/// 4.5:1 (3:1 for `agents_dimmer`) on their grounds are darkened just enough, towards black: muted
/// #7b88a1 to #636d82 on the terminal's ground; on the sidebar, #7b88a1 to #7a869f for the quieter
/// text, green to #557142, red #bf616a to #a05159, blue #5e81ac to #4e6b8f, yellow to #806526, the
/// sandbox to #596e60, orange to #98593b (stalled) and purple to #835d7d (starting); and the
/// accent, #3b8ba3 to #33788d, so the selected right tab's icon reads at 3:1 on its capsule and
/// each level of the activity panel stands apart from the last.
fn tide_light() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xeceff4),
        text: rgb(0x2e3440),
        selected: rgb(0xdae0ea),
        agent_selected: rgb(0xdae0ea),
        agent_working: rgb(0x5e81ac),
        agent_idle: rgb(0x5f7f4a),
        agent_blocked: rgb(0x9a7a2e),
        agent_stalled: rgb(0x98593b),
        agent_error: rgb(0xbf616a),
        agent_starting: rgb(0x835d7d),
        border: rgb(0xaab3c5),
        bright: rgb(0x2e3440),
        muted: rgb(0x636d82),
        dim: rgb(0x7b88a1),
        focus: rgb(0x33788d),
        connected: rgb(0x3b8ba3),
        working: rgb(0x5e81ac),
        danger: rgb(0xbf616a),
        unread: rgb(0x9a6e93),
        input_text: rgb(0xd8dee9),
        reply_code: rgb(0x9a7a2e),
        reply_heading: rgb(0x3b8ba3),
        agents_bg: rgb(0xe5e9f0),
        agents_border: rgb(0xaab3c5),
        agents_rule: rgb(0xd8dee9),
        agents_faint: rgb(0xd8dee9),
        agents_text: rgb(0x2e3440),
        agents_branch: rgb(0x434c5e),
        agents_dim: rgb(0x4c566a),
        agents_dimmer: rgb(0x7a869f),
        agents_accent: rgb(0x33788d),
        agents_green: rgb(0x557142),
        agents_red: rgb(0xa05159),
        agents_blue: rgb(0x4e6b8f),
        agents_yellow: rgb(0x806526),
        agents_purple: rgb(0x9a6e93),
        agents_sandbox: rgb(0x596e60),
        claude: rgb(0xc0704a),
        codex: rgb(0x3b8ba3),
        pi: rgb(0x2e3440),
        omp: rgb(0x9a6e93),
        amp: rgb(0x9a7a2e),
        agy: rgb(0x5e81ac),
        ..Theme::default()
    }
}

/// Lagoon's light version (P5-177), paddock's own like Lagoon: the sample's pale mint grounds and
/// deep teal text (`docs/设计稿/P5-177-Dune-Tide-Lagoon浅色版/`), #eef4f0 for the terminal, #e0ece6
/// for the sidebar, #3a524c text, #cfdcd6 rules. The selected ground is the sample's selected card
/// over the sidebar (#cce0d7); edges lie halfway from the rules to the quieter text (#a9bbb4),
/// branches a third of the way from the text to the quiet text (#465f59). Status hues by Lagoon's
/// roles from Everforest light (https://github.com/sainnhe/everforest, MIT, Copyright (c) 2019
/// sainnhe), deepened as the sample has them: blue #3a94c5 working, green #5f8a00 idle and sandbox
/// (Lagoon's sandbox is its green), yellow #b88a00 waiting, orange #e06f22 stalled, red #d94a47
/// error, purple #b8569a starting; the accent is the deepened aqua #2f9474, Lagoon's teal. Those
/// read as text under 4.5:1 (3:1 for `agents_dimmer`) on their grounds are darkened just enough,
/// towards black: muted, the sample's #829a93, to #61736d on the terminal's ground; on the sidebar,
/// the sample's quiet text #5f7a73 to #566e68 and #829a93 to #758a84, green to #507400, red to
/// #b93f3d, blue to #2c6f94, yellow to #866400, orange to #a65219 (stalled) and purple to #a04b86
/// (starting); the terminal's yellow too, to #b28500 (see `theme.rs`); and the accent, #2f9474
/// to #2a8367, so the selected right tab's icon reads at 3:1 on its capsule and each level of the
/// activity panel stands apart from the last.
fn lagoon_light() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xeef4f0),
        text: rgb(0x3a524c),
        selected: rgb(0xcce0d7),
        agent_selected: rgb(0xcce0d7),
        agent_working: rgb(0x3a94c5),
        agent_idle: rgb(0x5f8a00),
        agent_blocked: rgb(0xb28500),
        agent_stalled: rgb(0xa65219),
        agent_error: rgb(0xd94a47),
        agent_starting: rgb(0xa04b86),
        border: rgb(0xa9bbb4),
        bright: rgb(0x3a524c),
        muted: rgb(0x61736d),
        dim: rgb(0x829a93),
        focus: rgb(0x2a8367),
        connected: rgb(0x3a94c5),
        working: rgb(0x3a94c5),
        danger: rgb(0xd94a47),
        unread: rgb(0xb8569a),
        input_text: rgb(0xe6e2cc),
        reply_code: rgb(0xb28500),
        reply_heading: rgb(0x2f9474),
        agents_bg: rgb(0xe0ece6),
        agents_border: rgb(0xa9bbb4),
        agents_rule: rgb(0xcfdcd6),
        agents_faint: rgb(0xcfdcd6),
        agents_text: rgb(0x3a524c),
        agents_branch: rgb(0x465f59),
        agents_dim: rgb(0x566e68),
        agents_dimmer: rgb(0x758a84),
        agents_accent: rgb(0x2a8367),
        agents_green: rgb(0x507400),
        agents_red: rgb(0xb93f3d),
        agents_blue: rgb(0x2c6f94),
        agents_yellow: rgb(0x866400),
        agents_purple: rgb(0xb8569a),
        agents_sandbox: rgb(0x507400),
        claude: rgb(0xe06f22),
        codex: rgb(0x2f9474),
        pi: rgb(0x3a524c),
        omp: rgb(0xb8569a),
        amp: rgb(0xb28500),
        agy: rgb(0x3a94c5),
        ..Theme::default()
    }
}

/// Catppuccin Mocha (https://github.com/catppuccin/catppuccin, MIT, Copyright (c) 2021
/// Catppuccin) by its roles: base for the terminal, mantle for the sidebar, surface0–2 for the
/// selected ground, rules and edges, overlay0–2 for the quiet text, subtext1 for branches, mauve
/// for the accent. Status hues: blue working, green idle, yellow waiting, peach stalled, red
/// error, lavender starting.
fn catppuccin() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x1e1e2e),
        text: rgb(0xcdd6f4),
        selected: rgb(0x45475a),
        agent_selected: rgb(0x313244),
        agent_working: rgb(0x89b4fa),
        agent_idle: rgb(0xa6e3a1),
        agent_blocked: rgb(0xf9e2af),
        agent_stalled: rgb(0xfab387),
        agent_error: rgb(0xf38ba8),
        agent_starting: rgb(0xb4befe),
        border: rgb(0x45475a),
        bright: rgb(0xcdd6f4),
        muted: rgb(0x9399b2),
        dim: rgb(0x585b70),
        focus: rgb(0xcba6f7),
        connected: rgb(0x94e2d5),
        working: rgb(0x89b4fa),
        danger: rgb(0xf38ba8),
        unread: rgb(0xf5c2e7),
        input_text: rgb(0x11111b),
        reply_code: rgb(0xf9e2af),
        reply_heading: rgb(0x89dceb),
        agents_bg: rgb(0x181825),
        agents_border: rgb(0x585b70),
        agents_rule: rgb(0x313244),
        agents_faint: rgb(0x45475a),
        agents_text: rgb(0xcdd6f4),
        agents_branch: rgb(0xbac2de),
        agents_dim: rgb(0x7f849c),
        agents_dimmer: rgb(0x6c7086),
        agents_accent: rgb(0xcba6f7),
        agents_green: rgb(0xa6e3a1),
        agents_red: rgb(0xf38ba8),
        agents_blue: rgb(0x89b4fa),
        agents_yellow: rgb(0xf9e2af),
        agents_purple: rgb(0xb4befe),
        agents_sandbox: rgb(0x94e2d5),
        claude: rgb(0xfab387),
        codex: rgb(0x94e2d5),
        pi: rgb(0xcdd6f4),
        omp: rgb(0xcba6f7),
        amp: rgb(0xf9e2af),
        agy: rgb(0x89b4fa),
        ..Theme::default()
    }
}

/// Catppuccin Latte (https://github.com/catppuccin/catppuccin, MIT, Copyright (c) 2021
/// Catppuccin), paddock's first light preset (P5-175), by Mocha's roles: base for the terminal,
/// mantle for the sidebar, surface0–2 for the selected ground, rules and edges, overlay0–2 for the
/// quiet text, subtext1 for branches, mauve for the accent. Status hues: blue working, green idle,
/// yellow waiting, peach stalled, red error, lavender starting. Latte's quiet text and most of its
/// hues read under 4.5:1 (3:1 for `agents_dimmer`) on its light grounds, so those that must are
/// darkened just enough, towards black: muted overlay2 #7c7f93 to #6b6d7f on base; on mantle,
/// overlay1 #8c8fa1 to #666876 and overlay0 #9ca0b0 to #828593 for the quiet text, green
/// #40a02b to #307820, red #d20f39 to #d00f39, blue #1e66f5 to #1c5fe5, yellow #df8e1d to
/// #935d13, teal #179299 to #12747a, peach #fe640b to #b34708 and lavender #7287fd to #5362b8.
/// The keys nothing reads as text keep Latte's own hues.
fn latte() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xeff1f5),
        text: rgb(0x4c4f69),
        selected: rgb(0xbcc0cc),
        agent_selected: rgb(0xccd0da),
        agent_working: rgb(0x1e66f5),
        agent_idle: rgb(0x40a02b),
        agent_blocked: rgb(0xdf8e1d),
        agent_stalled: rgb(0xb34708),
        agent_error: rgb(0xd20f39),
        agent_starting: rgb(0x5362b8),
        border: rgb(0xbcc0cc),
        bright: rgb(0x4c4f69),
        muted: rgb(0x6b6d7f),
        dim: rgb(0xacb0be),
        focus: rgb(0x8839ef),
        connected: rgb(0x179299),
        working: rgb(0x1e66f5),
        danger: rgb(0xd20f39),
        unread: rgb(0xea76cb),
        input_text: rgb(0xdce0e8),
        reply_code: rgb(0xdf8e1d),
        reply_heading: rgb(0x04a5e5),
        agents_bg: rgb(0xe6e9ef),
        agents_border: rgb(0xacb0be),
        agents_rule: rgb(0xccd0da),
        agents_faint: rgb(0xbcc0cc),
        agents_text: rgb(0x4c4f69),
        agents_branch: rgb(0x5c5f77),
        agents_dim: rgb(0x666876),
        agents_dimmer: rgb(0x828593),
        agents_accent: rgb(0x8839ef),
        agents_green: rgb(0x307820),
        agents_red: rgb(0xd00f39),
        agents_blue: rgb(0x1c5fe5),
        agents_yellow: rgb(0x935d13),
        agents_purple: rgb(0x7287fd),
        agents_sandbox: rgb(0x12747a),
        claude: rgb(0xfe640b),
        codex: rgb(0x179299),
        pi: rgb(0x4c4f69),
        omp: rgb(0x8839ef),
        amp: rgb(0xdf8e1d),
        agy: rgb(0x1e66f5),
        ..Theme::default()
    }
}

/// Tokyo Night, night style (https://github.com/folke/tokyonight.nvim, Apache-2.0, by Folke
/// Lemaitre; after Tokyo Night, https://github.com/enkia/tokyo-night-vscode-theme, MIT, Copyright
/// (c) 2018-present Enkia): bg for the terminal, bg_dark for the sidebar, bg_visual for the
/// selected ground, bg_highlight, fg_gutter, terminal_black and dark3 for rules and edges, fg_dark
/// for branches, blue for the accent. Its quiet text (dark5, comment) reads under 4.5:1 and 3:1
/// on its grounds, so it is lightened just enough: dark5 #737aa2 to #7b82a8, comment #565f89 to
/// #59628b. Status hues: blue working, green idle, yellow waiting, orange stalled, red error,
/// magenta starting.
fn tokyo_night() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x1a1b26),
        text: rgb(0xc0caf5),
        selected: rgb(0x283457),
        agent_selected: rgb(0x283457),
        agent_working: rgb(0x7aa2f7),
        agent_idle: rgb(0x9ece6a),
        agent_blocked: rgb(0xe0af68),
        agent_stalled: rgb(0xff9e64),
        agent_error: rgb(0xf7768e),
        agent_starting: rgb(0xbb9af7),
        border: rgb(0x3b4261),
        bright: rgb(0xc0caf5),
        muted: rgb(0x7b82a8),
        dim: rgb(0x545c7e),
        focus: rgb(0x7aa2f7),
        connected: rgb(0x7dcfff),
        working: rgb(0x7aa2f7),
        danger: rgb(0xf7768e),
        unread: rgb(0xbb9af7),
        input_text: rgb(0x15161e),
        reply_code: rgb(0xe0af68),
        reply_heading: rgb(0x7dcfff),
        agents_bg: rgb(0x16161e),
        agents_border: rgb(0x414868),
        agents_rule: rgb(0x292e42),
        agents_faint: rgb(0x3b4261),
        agents_text: rgb(0xc0caf5),
        agents_branch: rgb(0xa9b1d6),
        agents_dim: rgb(0x7b82a8),
        agents_dimmer: rgb(0x59628b),
        agents_accent: rgb(0x7aa2f7),
        agents_green: rgb(0x9ece6a),
        agents_red: rgb(0xf7768e),
        agents_blue: rgb(0x7aa2f7),
        agents_yellow: rgb(0xe0af68),
        agents_purple: rgb(0xbb9af7),
        agents_sandbox: rgb(0x73daca),
        claude: rgb(0xff9e64),
        codex: rgb(0x73daca),
        pi: rgb(0xc0caf5),
        omp: rgb(0x9d7cd8),
        amp: rgb(0xe0af68),
        agy: rgb(0x7aa2f7),
        ..Theme::default()
    }
}

/// Rosé Pine, main variant (https://github.com/rose-pine/rose-pine-theme, MIT, Copyright (c) 2023
/// Rosé Pine): base for the terminal, surface for the sidebar as Rosé Pine lays out panels,
/// highlight med for the selected ground and rules, highlight high for edges, subtle and muted for
/// the quiet text, rose for the accent. It has three neutrals, so branches sit a third of the way
/// from text to subtle (#c8c5de). Status hues: foam working, gold waiting, love error, iris
/// starting; idle takes pine as Rosé Pine's green, but main's #31748f is too dark to read as
/// small text on the frosted sidebar, so Moon's pine (#3e8fb0) stands in, and stalled takes
/// Moon's deeper rose (#ea9a97), apart from the accent.
fn rose_pine() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x191724),
        text: rgb(0xe0def4),
        selected: rgb(0x403d52),
        agent_selected: rgb(0x403d52),
        agent_working: rgb(0x9ccfd8),
        agent_idle: rgb(0x3e8fb0),
        agent_blocked: rgb(0xf6c177),
        agent_stalled: rgb(0xea9a97),
        agent_error: rgb(0xeb6f92),
        agent_starting: rgb(0xc4a7e7),
        border: rgb(0x403d52),
        bright: rgb(0xe0def4),
        muted: rgb(0x908caa),
        dim: rgb(0x6e6a86),
        focus: rgb(0xebbcba),
        connected: rgb(0x9ccfd8),
        working: rgb(0x9ccfd8),
        danger: rgb(0xeb6f92),
        unread: rgb(0xc4a7e7),
        input_text: rgb(0x16141f),
        reply_code: rgb(0xf6c177),
        reply_heading: rgb(0x9ccfd8),
        agents_bg: rgb(0x1f1d2e),
        agents_border: rgb(0x524f67),
        agents_rule: rgb(0x403d52),
        agents_faint: rgb(0x524f67),
        agents_text: rgb(0xe0def4),
        agents_branch: rgb(0xc8c5de),
        agents_dim: rgb(0x908caa),
        agents_dimmer: rgb(0x6e6a86),
        agents_accent: rgb(0xebbcba),
        agents_green: rgb(0x3e8fb0),
        agents_red: rgb(0xeb6f92),
        agents_blue: rgb(0x9ccfd8),
        agents_yellow: rgb(0xf6c177),
        agents_purple: rgb(0xc4a7e7),
        agents_sandbox: rgb(0x9ccfd8),
        claude: rgb(0xebbcba),
        codex: rgb(0x9ccfd8),
        pi: rgb(0xe0def4),
        omp: rgb(0xc4a7e7),
        amp: rgb(0xf6c177),
        agy: rgb(0x3e8fb0),
        ..Theme::default()
    }
}

/// Kanagawa, wave variant (https://github.com/rebelot/kanagawa.nvim, MIT, Copyright (c) 2021
/// Tommaso Laurenzi): sumiInk3 for the terminal, sumiInk0 for the sidebar, sumiInk4–6 for the
/// selected ground, rules and edges, fujiWhite and oldWhite for text and branches, carpYellow for
/// the accent. Its comment grey (fujiGray) reads under 4.5:1, so the quiet text takes the
/// palette's dragonGray2 and katanaGray. Status hues: crystalBlue working, springGreen idle,
/// autumnYellow waiting (deeper than the accent), surimiOrange stalled, waveRed error, oniViolet
/// starting.
fn kanagawa() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0x1f1f28),
        text: rgb(0xdcd7ba),
        selected: rgb(0x223249),
        agent_selected: rgb(0x2a2a37),
        agent_working: rgb(0x7e9cd8),
        agent_idle: rgb(0x98bb6c),
        agent_blocked: rgb(0xdca561),
        agent_stalled: rgb(0xffa066),
        agent_error: rgb(0xe46876),
        agent_starting: rgb(0x957fb8),
        border: rgb(0x363646),
        bright: rgb(0xdcd7ba),
        muted: rgb(0x9e9b93),
        dim: rgb(0x727169),
        focus: rgb(0xe6c384),
        connected: rgb(0x7fb4ca),
        working: rgb(0x7e9cd8),
        danger: rgb(0xe46876),
        unread: rgb(0x957fb8),
        input_text: rgb(0x16161d),
        reply_code: rgb(0xe6c384),
        reply_heading: rgb(0x7fb4ca),
        agents_bg: rgb(0x16161d),
        agents_border: rgb(0x54546d),
        agents_rule: rgb(0x2a2a37),
        agents_faint: rgb(0x363646),
        agents_text: rgb(0xdcd7ba),
        agents_branch: rgb(0xc8c093),
        agents_dim: rgb(0x9e9b93),
        agents_dimmer: rgb(0x717c7c),
        agents_accent: rgb(0xe6c384),
        agents_green: rgb(0x98bb6c),
        agents_red: rgb(0xe46876),
        agents_blue: rgb(0x7e9cd8),
        agents_yellow: rgb(0xdca561),
        agents_purple: rgb(0x957fb8),
        agents_sandbox: rgb(0x7aa89f),
        claude: rgb(0xffa066),
        codex: rgb(0x7aa89f),
        pi: rgb(0xdcd7ba),
        omp: rgb(0x957fb8),
        amp: rgb(0xdca561),
        agy: rgb(0x7e9cd8),
        ..Theme::default()
    }
}

/// Tokyo Night, day style (https://github.com/folke/tokyonight.nvim `extras/lua/tokyonight_day.lua`,
/// Apache-2.0, by Folke Lemaitre; after Tokyo Night, https://github.com/enkia/tokyo-night-vscode-theme,
/// MIT, Copyright (c) 2018-present Enkia), a light preset (P5-176), by the night style's roles:
/// bg for the terminal, bg_dark for the sidebar, bg_visual for the selected ground, bg_highlight,
/// fg_gutter, terminal_black and dark3 for rules and edges, fg_dark for branches, blue for the
/// accent. Status hues: blue working, green idle, yellow waiting, orange stalled, red error,
/// magenta starting. Those read as text under 4.5:1 (3:1 for `agents_dimmer`) on its grounds are
/// darkened just enough, towards black: muted dark5 #68709a to #5c6389 on bg; on bg_dark, dark5
/// #68709a to #555b7d and comment #848cb5 to #707699 for the quiet text, green #587539 to
/// #4c6431, red #f52a65 to #b21e49, blue #2e7de9 to #225bab, yellow #8c6c3e to #735833, teal
/// #118c74 to #0d6957, orange #b15c00 to #8f4b00 and magenta #9854f1 to #7440b9; and the
/// sidebar's text, fg #3760bf to #243f7e, so the tabs' names read at 4.5:1 on their capsules
/// over bg_dark; the accent, blue #2e7de9 to #2668c3, so the selected right tab's icon reads at
/// 3:1 on its capsule. The keys nothing reads as text keep the day style's own hues.
fn tokyo_night_day() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xe1e2e7),
        text: rgb(0x3760bf),
        selected: rgb(0xb7c1e3),
        agent_selected: rgb(0xb7c1e3),
        agent_working: rgb(0x2e7de9),
        agent_idle: rgb(0x587539),
        agent_blocked: rgb(0x8c6c3e),
        agent_stalled: rgb(0x8f4b00),
        agent_error: rgb(0xf52a65),
        agent_starting: rgb(0x7440b9),
        border: rgb(0xa8aecb),
        bright: rgb(0x3760bf),
        muted: rgb(0x5c6389),
        dim: rgb(0x8990b3),
        focus: rgb(0x2e7de9),
        connected: rgb(0x007197),
        working: rgb(0x2e7de9),
        danger: rgb(0xf52a65),
        unread: rgb(0x9854f1),
        input_text: rgb(0xb4b5b9),
        reply_code: rgb(0x8c6c3e),
        reply_heading: rgb(0x007197),
        agents_bg: rgb(0xd0d5e3),
        agents_border: rgb(0xa1a6c5),
        agents_rule: rgb(0xc4c8da),
        agents_faint: rgb(0xa8aecb),
        agents_text: rgb(0x243f7e),
        agents_branch: rgb(0x6172b0),
        agents_dim: rgb(0x555b7d),
        agents_dimmer: rgb(0x707699),
        agents_accent: rgb(0x2668c3),
        agents_green: rgb(0x4c6431),
        agents_red: rgb(0xb21e49),
        agents_blue: rgb(0x225bab),
        agents_yellow: rgb(0x735833),
        agents_purple: rgb(0x9854f1),
        agents_sandbox: rgb(0x0d6957),
        claude: rgb(0xb15c00),
        codex: rgb(0x118c74),
        pi: rgb(0x3760bf),
        omp: rgb(0x7847bd),
        amp: rgb(0x8c6c3e),
        agy: rgb(0x2e7de9),
        ..Theme::default()
    }
}

/// Rosé Pine, dawn variant (https://github.com/rose-pine/rose-pine-theme, MIT, Copyright (c) 2023
/// Rosé Pine), a light preset (P5-176), by main's roles: base for the terminal, highlight med for
/// the selected ground and rules, highlight high for edges, subtle and muted for the quiet text,
/// rose for the accent, branches a third of the way from text to subtle (#625e82). The sidebar
/// is overlay, not surface as on main: dawn's surface (#fffaf3) is nearly white, and the cards'
/// white faces would not stand off it. Status hues: foam working, pine idle (dawn's own pine
/// reads on its light grounds), gold waiting, love error, iris starting, and rose deepened for
/// stalled, apart from the accent. Those read as text under 4.5:1 (3:1 for `agents_dimmer`) on
/// its grounds are darkened just enough, towards black: muted subtle #797593 to #716d89 on base;
/// on overlay, subtle #797593 to #6b6782 and muted #9893a5 to #898495 for the quiet text, love
/// #b4637a to #9a5569, foam #56949f to #42717a (blue and sandbox), gold #ea9d34 to #906020, rose
/// #d7827e to #955a58 (stalled) and iris #907aa9 to #746389 (starting); and the sidebar's text,
/// #575279 to #514d71, so the tabs' names read at 4.5:1 on their capsules over overlay; the
/// accent, rose #d7827e to #a86562, so the selected right tab's icon reads at 3:1. The keys
/// nothing reads as text keep dawn's own hues.
fn rose_pine_dawn() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xfaf4ed),
        text: rgb(0x575279),
        selected: rgb(0xdfdad9),
        agent_selected: rgb(0xdfdad9),
        agent_working: rgb(0x56949f),
        agent_idle: rgb(0x286983),
        agent_blocked: rgb(0xea9d34),
        agent_stalled: rgb(0x955a58),
        agent_error: rgb(0xb4637a),
        agent_starting: rgb(0x746389),
        border: rgb(0xdfdad9),
        bright: rgb(0x575279),
        muted: rgb(0x716d89),
        dim: rgb(0x9893a5),
        focus: rgb(0xd7827e),
        connected: rgb(0x56949f),
        working: rgb(0x56949f),
        danger: rgb(0xb4637a),
        unread: rgb(0x907aa9),
        input_text: rgb(0xf2e9e1),
        reply_code: rgb(0xea9d34),
        reply_heading: rgb(0x56949f),
        agents_bg: rgb(0xf2e9e1),
        agents_border: rgb(0xcecacd),
        agents_rule: rgb(0xdfdad9),
        agents_faint: rgb(0xcecacd),
        agents_text: rgb(0x514d71),
        agents_branch: rgb(0x625e82),
        agents_dim: rgb(0x6b6782),
        agents_dimmer: rgb(0x898495),
        agents_accent: rgb(0xa86562),
        agents_green: rgb(0x286983),
        agents_red: rgb(0x9a5569),
        agents_blue: rgb(0x42717a),
        agents_yellow: rgb(0x906020),
        agents_purple: rgb(0x907aa9),
        agents_sandbox: rgb(0x42717a),
        claude: rgb(0xd7827e),
        codex: rgb(0x56949f),
        pi: rgb(0x575279),
        omp: rgb(0x907aa9),
        amp: rgb(0xea9d34),
        agy: rgb(0x286983),
        ..Theme::default()
    }
}

/// Kanagawa, lotus variant (https://github.com/rebelot/kanagawa.nvim, MIT, Copyright (c) 2021
/// Tommaso Laurenzi), a light preset (P5-176), by wave's roles: bg (lotusWhite3) for the
/// terminal, bg_m3 (lotusWhite0) for the sidebar, bg_visual (lotusViolet3), bg_p1–2 (lotusWhite4–5)
/// and nontext (lotusViolet1) for the selected ground, rules and edges, fg and fg_dim (lotusInk1–2)
/// for text and branches, the identifier yellow (lotusYellow) for the accent. The quiet text takes
/// the float border's lotusGray2 and the comment's lotusGray3. Status hues: lotusBlue4 working,
/// lotusGreen idle, lotusYellow3 waiting, lotusOrange stalled, lotusRed error, lotusViolet4
/// starting. Those read as text under 4.5:1 (3:1 for `agents_dimmer`) on its grounds are darkened
/// just enough, towards black: muted lotusGray2 #716e61 to #6d6a5e on bg; on bg_m3, lotusGray2
/// #716e61 to #5a574d and lotusGray3 #8a8980 to #73736b for the quiet text, lotusGreen #6f894e
/// to #4c5d35, lotusRed #c84053 to #9b3240, lotusBlue4 #4d699b to #405781, lotusYellow3 #de9800
/// to #755000, lotusAqua #597b75 to #435c58 and lotusOrange #cc6d00 to #864700; and the
/// sidebar's text, lotusInk1 #545464 to #3b3b46, so the tabs' names read at 4.5:1 on their
/// capsules over bg_m3; the accent, lotusYellow #77713f to #6c6739, so the selected right tab's
/// icon reads at 3:1 and each level of the activity panel stands apart from the last. The keys nothing reads as text keep lotus's own hues.
fn kanagawa_lotus() -> Theme {
    let rgb = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    Theme {
        bg: rgb(0xf2ecbc),
        text: rgb(0x545464),
        selected: rgb(0xc9cbd1),
        agent_selected: rgb(0xe7dba0),
        agent_working: rgb(0x4d699b),
        agent_idle: rgb(0x6f894e),
        agent_blocked: rgb(0xde9800),
        agent_stalled: rgb(0x864700),
        agent_error: rgb(0xc84053),
        agent_starting: rgb(0x624c83),
        border: rgb(0xe4d794),
        bright: rgb(0x545464),
        muted: rgb(0x6d6a5e),
        dim: rgb(0x8a8980),
        focus: rgb(0x77713f),
        connected: rgb(0x6693bf),
        working: rgb(0x4d699b),
        danger: rgb(0xc84053),
        unread: rgb(0x624c83),
        input_text: rgb(0xd5cea3),
        reply_code: rgb(0x77713f),
        reply_heading: rgb(0x6693bf),
        agents_bg: rgb(0xd5cea3),
        agents_border: rgb(0xa09cac),
        agents_rule: rgb(0xe7dba0),
        agents_faint: rgb(0xe4d794),
        agents_text: rgb(0x3b3b46),
        agents_branch: rgb(0x43436c),
        agents_dim: rgb(0x5a574d),
        agents_dimmer: rgb(0x73736b),
        agents_accent: rgb(0x6c6739),
        agents_green: rgb(0x4c5d35),
        agents_red: rgb(0x9b3240),
        agents_blue: rgb(0x405781),
        agents_yellow: rgb(0x755000),
        agents_purple: rgb(0x624c83),
        agents_sandbox: rgb(0x435c58),
        claude: rgb(0xcc6d00),
        codex: rgb(0x597b75),
        pi: rgb(0x545464),
        omp: rgb(0x624c83),
        amp: rgb(0xde9800),
        agy: rgb(0x4d699b),
        ..Theme::default()
    }
}

/// The colors in effect. Queue deliberately shares the Agents status accents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color,
    pub overlay: Color,
    pub selected: Color,
    pub agent_selected: Color,
    pub agent_working: Color,
    pub agent_idle: Color,
    pub agent_blocked: Color,
    pub agent_stalled: Color,
    pub agent_error: Color,
    pub agent_starting: Color,
    pub border: Color,
    pub text: Color,
    pub bright: Color,
    pub muted: Color,
    pub dim: Color,
    pub focus: Color,
    pub connected: Color,
    pub working: Color,
    pub danger: Color,
    pub unread: Color,
    pub input_text: Color,
    pub reply_code: Color,
    pub reply_heading: Color,
    pub agents_bg: Color,
    pub agents_border: Color,
    pub agents_rule: Color,
    pub agents_faint: Color,
    pub agents_text: Color,
    pub agents_branch: Color,
    pub agents_dim: Color,
    pub agents_dimmer: Color,
    pub agents_accent: Color,
    pub agents_green: Color,
    pub agents_red: Color,
    pub agents_blue: Color,
    pub agents_yellow: Color,
    pub agents_purple: Color,
    /// A sandboxed agent's own lock and pill (P5-145), set apart from the accent and yellow.
    pub agents_sandbox: Color,
    pub claude: Color,
    pub codex: Color,
    pub pi: Color,
    pub omp: Color,
    pub amp: Color,
    pub agy: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: BG,
            overlay: OVERLAY,
            selected: SELECTED,
            agent_selected: AGENT_SELECTED,
            agent_working: AGENT_WORKING,
            agent_idle: AGENT_IDLE,
            agent_blocked: AGENT_BLOCKED,
            agent_stalled: AGENT_STALLED,
            agent_error: AGENT_ERROR,
            agent_starting: AGENT_STARTING,
            border: BORDER,
            text: TEXT,
            bright: BRIGHT,
            muted: MUTED,
            dim: DIM,
            focus: FOCUS,
            connected: CONNECTED,
            working: WORKING,
            danger: DANGER,
            unread: UNREAD,
            input_text: Color::Black,
            reply_code: Color::Yellow,
            reply_heading: Color::Cyan,
            agents_bg: AGENTS_BG,
            agents_border: AGENTS_BORDER,
            agents_rule: AGENTS_RULE,
            agents_faint: AGENTS_FAINT,
            agents_text: AGENTS_TEXT,
            agents_branch: AGENTS_BRANCH,
            agents_dim: AGENTS_DIM,
            agents_dimmer: AGENTS_DIMMER,
            agents_accent: AGENTS_ACCENT,
            agents_green: AGENTS_GREEN,
            agents_red: AGENTS_RED,
            agents_blue: AGENTS_BLUE,
            agents_yellow: AGENTS_YELLOW,
            agents_purple: AGENTS_PURPLE,
            agents_sandbox: AGENTS_SANDBOX,
            claude: Color::Rgb(0xe2, 0x83, 0x5e),
            codex: Color::Rgb(0x79, 0xd4, 0xb4),
            pi: Color::Rgb(0xff, 0xff, 0xff),
            omp: Color::Rgb(0xa8, 0x55, 0xf7),
            amp: Color::Rgb(0xe5, 0xcf, 0x6f),
            agy: Color::Rgb(0x5b, 0x9c, 0xf2),
        }
    }
}

const NAMES: [(&str, Color); 16] = [
    ("black", Color::Black),
    ("red", Color::Red),
    ("green", Color::Green),
    ("yellow", Color::Yellow),
    ("blue", Color::Blue),
    ("magenta", Color::Magenta),
    ("cyan", Color::Cyan),
    ("gray", Color::Gray),
    ("dark_gray", Color::DarkGray),
    ("light_red", Color::LightRed),
    ("light_green", Color::LightGreen),
    ("light_yellow", Color::LightYellow),
    ("light_blue", Color::LightBlue),
    ("light_magenta", Color::LightMagenta),
    ("light_cyan", Color::LightCyan),
    ("white", Color::White),
];

/// How a colour is written in the config file (Saddle's `color_name`): `#rrggbb`, an ANSI name,
/// or `default`.
pub fn color_name(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Reset => "default".into(),
        Color::Indexed(index) => format!("{index}"),
        other => NAMES
            .iter()
            .find(|(_, color)| *color == other)
            .map_or_else(String::new, |(name, _)| (*name).into()),
    }
}

/// A configured color: `default`/`reset`, an ANSI name or `#RRGGBB`.
pub fn parse_color(value: &str) -> Result<Color, String> {
    if matches!(value, "default" | "reset") {
        return Ok(Color::Reset);
    }
    if let Some((_, color)) = NAMES.iter().find(|(name, _)| *name == value) {
        return Ok(*color);
    }
    if let Some(hex) = value.strip_prefix('#')
        && hex.len() == 6
        && hex.bytes().all(|b| b.is_ascii_hexdigit())
        && let Ok(rgb) = u32::from_str_radix(hex, 16)
    {
        return Ok(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
    }
    Err(format!(
        "invalid color {value:?}: expected default, an ANSI color name, or #RRGGBB"
    ))
}

impl Theme {
    /// Every configurable color by its `[colors]` key.
    pub fn named_mut(&mut self) -> [(&'static str, &mut Color); 44] {
        [
            ("bg", &mut self.bg),
            ("overlay", &mut self.overlay),
            ("selected", &mut self.selected),
            ("agent_selected", &mut self.agent_selected),
            ("agent_working", &mut self.agent_working),
            ("agent_idle", &mut self.agent_idle),
            ("agent_blocked", &mut self.agent_blocked),
            ("agent_stalled", &mut self.agent_stalled),
            ("agent_error", &mut self.agent_error),
            ("agent_starting", &mut self.agent_starting),
            ("border", &mut self.border),
            ("text", &mut self.text),
            ("bright", &mut self.bright),
            ("muted", &mut self.muted),
            ("dim", &mut self.dim),
            ("focus", &mut self.focus),
            ("connected", &mut self.connected),
            ("working", &mut self.working),
            ("danger", &mut self.danger),
            ("unread", &mut self.unread),
            ("input_text", &mut self.input_text),
            ("reply_code", &mut self.reply_code),
            ("reply_heading", &mut self.reply_heading),
            ("agents_bg", &mut self.agents_bg),
            ("agents_border", &mut self.agents_border),
            ("agents_rule", &mut self.agents_rule),
            ("agents_faint", &mut self.agents_faint),
            ("agents_text", &mut self.agents_text),
            ("agents_branch", &mut self.agents_branch),
            ("agents_dim", &mut self.agents_dim),
            ("agents_dimmer", &mut self.agents_dimmer),
            ("agents_accent", &mut self.agents_accent),
            ("agents_green", &mut self.agents_green),
            ("agents_red", &mut self.agents_red),
            ("agents_blue", &mut self.agents_blue),
            ("agents_yellow", &mut self.agents_yellow),
            ("agents_purple", &mut self.agents_purple),
            ("agents_sandbox", &mut self.agents_sandbox),
            ("claude", &mut self.claude),
            ("codex", &mut self.codex),
            ("pi", &mut self.pi),
            ("omp", &mut self.omp),
            ("amp", &mut self.amp),
            ("agy", &mut self.agy),
        ]
    }
}

#[cfg(test)]
mod groups {
    use super::*;

    /// Settings shows the dark presets, then the light ones in their dark versions' order
    /// (P5-176); each group holds only themes of its kind, and every preset is in one.
    #[test]
    fn the_light_group_follows_the_dark_ones_order() {
        let (dark, light) = Preset::groups();
        assert_eq!(
            dark,
            [
                Preset::Dune,
                Preset::Tide,
                Preset::Lagoon,
                Preset::Catppuccin,
                Preset::TokyoNight,
                Preset::RosePine,
                Preset::Kanagawa,
            ]
        );
        assert_eq!(
            light,
            [
                Preset::DuneLight,
                Preset::TideLight,
                Preset::LagoonLight,
                Preset::Latte,
                Preset::TokyoNightDay,
                Preset::RosePineDawn,
                Preset::KanagawaLotus,
            ]
        );
        // Seven over seven (P5-177): each light version sits right under its dark one.
        assert_eq!(dark.len(), light.len());
        for (dark, light) in dark.iter().zip(&light) {
            assert_eq!(dark.light(), Some(*light), "{}", dark.name());
        }
        assert_eq!(dark.len() + light.len(), Preset::ALL.len());
        for (group, is_light) in [(&dark, false), (&light, true)] {
            for preset in group {
                let config = crate::config::Config {
                    theme: Some(preset.name().into()),
                    ..crate::config::Config::default()
                };
                let theme = crate::theme::Theme::from_config(&config).unwrap();
                assert_eq!(theme.is_light(), is_light, "{}", preset.name());
            }
        }
    }
}

#[cfg(test)]
mod kind_colours {
    use super::*;

    #[test]
    fn every_theme_tells_the_agent_kinds_apart() {
        for preset in Preset::ALL {
            let mut theme = preset.theme();
            let kinds = ["claude", "codex", "pi", "omp", "amp", "agy"];
            let colours: Vec<Color> = theme
                .named_mut()
                .into_iter()
                .filter(|(name, _)| kinds.contains(name))
                .map(|(_, colour)| *colour)
                .collect();
            assert_eq!(colours.len(), kinds.len(), "{}", preset.name());
            for (i, colour) in colours.iter().enumerate() {
                assert!(
                    !colours[..i].contains(colour),
                    "{}: {} shares a colour",
                    preset.name(),
                    kinds[i]
                );
            }
        }
    }
}
