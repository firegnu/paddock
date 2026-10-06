//! Saddle's interface colours for the Dune, Tide and Lagoon presets, and how a configured colour is
//! written. From Saddle `src/theme.rs` at commit `df1c727`, kept apart from Saddle since then
//! (DESIGN §8). Not taken: the Terminal preset (paddock has no outer terminal), the 256-colour
//! fallback, `[colors]` deserialisation, and the ratatui block and layout helpers. Since M1 colours
//! are paddock's own `Color` below instead of ratatui's, with the same values.

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
    Tide,
    Lagoon,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::Dune, Preset::Tide, Preset::Lagoon];
    /// How the theme is written in the config file.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Dune => "dune",
            Preset::Tide => "tide",
            Preset::Lagoon => "lagoon",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == value)
            .ok_or_else(|| format!("unknown theme {value:?}: expected dune, tide or lagoon"))
    }
    /// How the sidebar's column lies over the system's sidebar material. The material is already
    /// dark and frosted; the wash only leans it to the preset's colour, and the grounds on it step
    /// up about half again as much as the preset's own selected colour does over its sidebar, as
    /// they lie on a busier ground. The quiet text steps up until, on a grey like the material's
    /// under a selected card (#45494a), `agents_dim` reads at about 4.6:1 and `agents_dimmer` at
    /// about 3.6:1, still apart from each other and from the text (about 7:1).
    pub fn frost(self) -> Frost {
        match self {
            // A warm brown, the nearest to the material's grey: the least wash. Its quiet text is
            // the darkest of the three, so it steps the furthest.
            Preset::Dune => Frost {
                wash: 0.30,
                lit: 0.10,
                waiting: 0.11,
                dim: 0.56,
                dimmer: 0.48,
            },
            // Deeper, bluer and greener grounds, a little more of each to keep them; their own
            // selected colours step further too.
            Preset::Tide => Frost {
                wash: 0.34,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.46,
                dimmer: 0.40,
            },
            Preset::Lagoon => Frost {
                wash: 0.38,
                lit: 0.12,
                waiting: 0.12,
                dim: 0.42,
                dimmer: 0.36,
            },
        }
    }
    /// Every color of the theme.
    pub fn theme(self) -> Theme {
        match self {
            Preset::Dune => Theme::default(),
            Preset::Tide => tide(),
            Preset::Lagoon => lagoon(),
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
        claude: rgb(0xe48c66),
        codex: rgb(0x6fd3c2),
        pi: rgb(0xeef2f7),
        omp: rgb(0xb07cf2),
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
        claude: rgb(0xe48c66),
        codex: rgb(0x6fd3c2),
        pi: rgb(0xeef5f2),
        omp: rgb(0xb07cf2),
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
    pub claude: Color,
    pub codex: Color,
    pub pi: Color,
    pub omp: Color,
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
            claude: Color::Rgb(0xe2, 0x83, 0x5e),
            codex: Color::Rgb(0x79, 0xd4, 0xb4),
            pi: Color::Rgb(0xff, 0xff, 0xff),
            omp: Color::Rgb(0xa8, 0x55, 0xf7),
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
    pub fn named_mut(&mut self) -> [(&'static str, &mut Color); 41] {
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
            ("claude", &mut self.claude),
            ("codex", &mut self.codex),
            ("pi", &mut self.pi),
            ("omp", &mut self.omp),
        ]
    }
}
