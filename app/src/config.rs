//! `~/.config/paddock/config.toml`. A missing file means every default; unknown keys are errors,
//! as in Saddle. Theme values are kept as written for the theme module to interpret.
use anyhow::{Context as _, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// How opaque, in percent, the darker frame lies over the system's sidebar material when the
/// config leaves it out: not at all, the material showing through whole (P5-95c; P5-166, was 65).
pub const FRAME_OPACITY: u8 = 0;

/// How opaque, in percent, the system's sidebar material itself is when the config leaves it out:
/// whole, as before it could be thinned (P5-169).
pub const MATERIAL_OPACITY: u8 = 100;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The preset name as written.
    pub theme: Option<String>,
    /// The `[colors]` overrides as written.
    pub colors: BTreeMap<String, String>,
    /// Width of the Agents sidebar, in points.
    pub sidebar_width: f32,
    /// The interface font; `None` is the system's.
    pub ui_font: Option<String>,
    /// The interface size every other interface size scales from, in points.
    pub ui_font_size: f32,
    /// The terminal font, its fallbacks, size and line height.
    pub font: String,
    pub font_fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
    /// Whether the pet walks in the tab strip.
    pub mascot_enabled: bool,
    /// Which pet: `clawd`, `cat` or `capybara`.
    pub mascot: crate::pet::Pet,
    /// How often corral is asked about the agents, in milliseconds.
    pub refresh_ms: u64,
    /// The corral program; `--corral` overrides it for one run.
    pub corral: String,
    /// What a file ⌘-clicked in a terminal opens with (P5-143b): empty for the app macOS picks
    /// (`open`), else a command such as `zed` or `code -g`, split at spaces and given
    /// `path:line:column`.
    pub open_files_with: String,
    /// Whether the terminal cursor blinks, fading out and in, unless the program asks for a
    /// steady one; and whether it slides to where it goes (P5-143c1).
    pub blink_cursor: bool,
    pub animate_cursor: bool,
    /// Whether the history scrolls by the pixel, a wheel's notch sliding into place, rather than by
    /// the line (P5-143c2).
    pub smooth_scroll: bool,
    /// Copy a nonempty mouse selection to the clipboard when released (P5-152).
    pub copy_on_select: bool,
    /// Whether text a program drew too close to its background is moved just far enough from it
    /// to read (P5-143e).
    pub minimum_contrast: bool,
    /// The New Agent panel's presets; `None` (not written) is the default four.
    pub agent_presets: Option<Vec<crate::new_agent::Preset>>,
    /// `[agent_models]`: the models and efforts New Agent offers for Claude and Codex (P5-100).
    pub agent_models: crate::models::Configured,
    /// How opaque, in percent, the sidebar's colour lies over its column while the system's
    /// sidebar material shows; `None` (not written) is the theme's own.
    pub sidebar_opacity: Option<u8>,
    /// How opaque, in percent, the darker frame lies over the material beside the sidebar; `None`
    /// (not written) is [`FRAME_OPACITY`].
    pub frame_opacity: Option<u8>,
    /// How opaque, in percent, the system's sidebar material under the whole window is: thinner,
    /// the desktop behind shows clearer; `None` (not written) is [`MATERIAL_OPACITY`].
    pub material_opacity: Option<u8>,
    /// `[[machines]]` as written; read into `machines` by [`Config::parse`].
    #[serde(rename = "machines")]
    pub machines_written: Vec<toml::Value>,
    /// The other machines whose corral agents paddock shows, each in a view of its own (DESIGN
    /// §13 P5-90), written by hand and only read. An entry that is wrong is left out and said in
    /// `machines_problem`: it never keeps paddock from opening on this Mac.
    #[serde(skip)]
    pub machines: Vec<Machine>,
    #[serde(skip)]
    pub machines_problem: Option<String>,
}

/// One `[[machines]]` entry: what paddock calls the machine, what it hands `ssh`, and where corral
/// is there when `ssh` cannot find it; the name it is shown by, and whether it is shown at all
/// (P5-123).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    /// Lower-case letters, digits and `-`; not `local`. Who the machine is: its saved panes and
    /// its connection go by it, never by `label`.
    pub name: String,
    /// An alias from `~/.ssh/config` or `user@host`, given to `ssh` as it is.
    pub ssh: String,
    /// corral's path there; a leading `~` is that machine's `$HOME`.
    #[serde(default)]
    pub corral: Option<String>,
    /// The name people see; `None` (not written) is `name`. Read without the spaces at its ends.
    #[serde(default)]
    pub label: Option<String>,
    /// `false`: hidden, as if not configured: not connected, not read, its saved panes kept.
    #[serde(default = "shown")]
    pub show: bool,
}

/// The longest a machine's label may be, in characters: room for a host's own name.
pub const LABEL_MAX: usize = 40;

fn shown() -> bool {
    true
}

/// The keys P5-123 added are of their kinds, named when not: serde's own words lose the key.
fn typed(written: &toml::Value) -> Result<()> {
    let name = written
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or("a machine");
    if written.get("label").is_some_and(|label| !label.is_str()) {
        anyhow::bail!("machines: {name}'s label must be text");
    }
    if written.get("show").is_some_and(|show| !show.is_bool()) {
        anyhow::bail!("machines: {name}'s show must be true or false");
    }
    Ok(())
}

/// `text` as a machine's label: without the spaces at its ends, not empty, no control characters,
/// at most [`LABEL_MAX`] characters.
pub fn label(text: &str) -> Result<String> {
    let text = text.trim();
    ensure!(!text.is_empty(), "the name shown must not be empty");
    ensure!(
        !text.chars().any(char::is_control),
        "the name shown must not have control characters"
    );
    ensure!(
        text.chars().count() <= LABEL_MAX,
        "the name shown must be at most {LABEL_MAX} characters"
    );
    Ok(text.to_owned())
}

impl Machine {
    /// The name people see: its label, else its name.
    pub fn shown_name(&self) -> &str {
        self.label.as_deref().map_or(&self.name, str::trim)
    }

    fn validate(&self) -> Result<()> {
        let name = &self.name;
        ensure!(
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "machines: name {name:?} may have only lower-case letters, digits and -"
        );
        ensure!(
            name != "local",
            "machines: name \"local\" is taken by this Mac"
        );
        let ssh = &self.ssh;
        ensure!(
            !ssh.is_empty(),
            "machines: {name} needs ssh, a host from ~/.ssh/config or user@host"
        );
        ensure!(
            !ssh.chars().any(|c| c.is_whitespace() || c.is_control()),
            "machines: {name}'s ssh {ssh:?} must not have spaces or control characters"
        );
        ensure!(
            !ssh.starts_with('-'),
            "machines: {name}'s ssh {ssh:?} must not start with -"
        );
        if let Some(corral) = &self.corral {
            ensure!(
                (corral.starts_with('/') || corral == "~" || corral.starts_with("~/"))
                    && !corral.chars().any(char::is_control),
                "machines: {name}'s corral {corral:?} must be a path, from / or ~/"
            );
        }
        if let Some(text) = &self.label {
            label(text).map_err(|error| anyhow::anyhow!("machines: {name}'s label: {error}"))?;
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: None,
            colors: BTreeMap::new(),
            sidebar_width: 380.0,
            ui_font: None,
            ui_font_size: crate::fonts::BASE_SIZE,
            font: "Menlo".into(),
            // Common Nerd Font families for prompt icons; missing ones are skipped.
            font_fallbacks: [
                "Symbols Nerd Font Mono",
                "FiraCode Nerd Font Mono",
                "FiraCode Nerd Font",
            ]
            .map(str::to_owned)
            .into(),
            font_size: 14.0,
            line_height: 1.3,
            mascot_enabled: true,
            mascot: crate::pet::Pet::Clawd,
            refresh_ms: 1000,
            corral: "corral".into(),
            open_files_with: String::new(),
            blink_cursor: true,
            animate_cursor: true,
            smooth_scroll: true,
            copy_on_select: false,
            minimum_contrast: true,
            agent_presets: None,
            agent_models: Default::default(),
            sidebar_opacity: None,
            frame_opacity: None,
            material_opacity: None,
            machines_written: Vec::new(),
            machines: Vec::new(),
            machines_problem: None,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(text)?;
        ensure!(
            config.sidebar_width.is_finite() && config.sidebar_width > 0.0,
            "sidebar_width must be positive"
        );
        config.validate_fonts()?;
        config.agent_models.validate()?;
        ensure!(
            config.refresh_ms > 0,
            "refresh_ms must be a positive number of milliseconds"
        );
        ensure!(
            !config.corral.trim().is_empty(),
            "corral must name a program"
        );
        for (key, value) in [
            ("sidebar_opacity", config.sidebar_opacity),
            ("frame_opacity", config.frame_opacity),
            ("material_opacity", config.material_opacity),
        ] {
            ensure!(
                value.is_none_or(|percent| percent <= 100),
                "{key} must be from 0 to 100"
            );
        }
        let mut problems = Vec::new();
        for written in &config.machines_written {
            let machine = typed(written)
                .and_then(|()| {
                    written
                        .clone()
                        .try_into::<Machine>()
                        .map_err(|error| anyhow::anyhow!("machines: {}", error.message()))
                })
                .and_then(|machine| {
                    machine.validate()?;
                    ensure!(
                        !config.machines.iter().any(|m| m.name == machine.name),
                        "machines: {} is named twice",
                        machine.name
                    );
                    Ok(machine)
                });
            match machine {
                Ok(machine) => config.machines.push(machine),
                Err(error) => problems.push(format!("{error:#}")),
            }
        }
        if !problems.is_empty() {
            config.machines_problem = Some(format!(
                "{}; left out of paddock until fixed",
                problems.join("; ")
            ));
        }
        Ok(config)
    }

    fn validate_fonts(&self) -> Result<()> {
        for (key, value) in [
            ("ui_font_size", self.ui_font_size),
            ("font_size", self.font_size),
            ("line_height", self.line_height),
        ] {
            ensure!(
                value.is_finite() && value > 0.0,
                "{key} must be positive and finite"
            );
        }
        Ok(())
    }

    /// Apply explicit command-line values; repeated fallbacks replace the whole configured list.
    pub fn apply_font_overrides(
        &mut self,
        font: Option<String>,
        fallbacks: Vec<String>,
        font_size: Option<f32>,
        line_height: Option<f32>,
    ) -> Result<()> {
        if let Some(font) = font {
            self.font = font;
        }
        if !fallbacks.is_empty() {
            self.font_fallbacks = fallbacks;
        }
        if let Some(font_size) = font_size {
            self.font_size = font_size;
        }
        if let Some(line_height) = line_height {
            self.line_height = line_height;
        }
        self.validate_fonts()
    }

    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                Self::parse(&text).with_context(|| format!("invalid config {}", path.display()))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
        }
    }
}

pub fn default_path() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/paddock/config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_settings_are_accepted() {
        let config = Config::parse(
            r#"
font = "Geist Mono"
font_fallbacks = ["Sarasa Mono SC", "Maple Mono NF CN"]
font_size = 14.5
line_height = 1.4
"#,
        )
        .unwrap();
        assert_eq!(config.font, "Geist Mono");
        assert_eq!(
            config.font_fallbacks,
            ["Sarasa Mono SC", "Maple Mono NF CN"]
        );
        assert_eq!(config.font_size, 14.5);
        assert_eq!(config.line_height, 1.4);
        let integers = Config::parse("font_size = 15\nline_height = 2").unwrap();
        assert_eq!(integers.font_size, 15.0);
        assert_eq!(integers.line_height, 2.0);
    }

    #[test]
    fn interface_font_is_read_apart_from_the_terminal_font() {
        let config = Config::parse(
            r#"
ui_font = "Avenir Next"
ui_font_size = 14.5
font = "Geist Mono"
font_size = 15
"#,
        )
        .unwrap();
        assert_eq!(config.ui_font.as_deref(), Some("Avenir Next"));
        assert_eq!(config.ui_font_size, 14.5);
        assert_eq!(config.font, "Geist Mono");
        assert_eq!(config.font_size, 15.0);
        // An older file without the interface keys: the system font at the size used so far.
        let old = Config::parse(
            "font = \"Geist Mono\"\nfont_fallbacks = [\"Sarasa Mono SC\"]\nfont_size = 14.5\nline_height = 1.4\n",
        )
        .unwrap();
        assert_eq!(old.ui_font, None);
        assert_eq!(old.ui_font_size, 13.0);
        assert_eq!(
            (old.font.as_str(), old.font_size, old.line_height),
            ("Geist Mono", 14.5, 1.4)
        );
        assert_eq!(old.font_fallbacks, ["Sarasa Mono SC"]);
    }

    #[test]
    fn invalid_font_values_name_the_key() {
        for (key, values) in [
            ("ui_font", vec!["14", "[]"]),
            ("ui_font_size", vec!["\"13\"", "0", "-2", "nan", "inf"]),
            ("font", vec!["14", "[]"]),
            ("font_fallbacks", vec!["\"Menlo\"", "[1]", "[\"Menlo\", 1]"]),
            (
                "font_size",
                vec!["\"14\"", "true", "0", "-1", "nan", "inf", "-inf"],
            ),
            (
                "line_height",
                vec!["\"1.3\"", "[]", "0", "-1.3", "nan", "inf", "-inf"],
            ),
        ] {
            for value in values {
                let error = Config::parse(&format!("{key} = {value}")).unwrap_err();
                assert!(
                    format!("{error:#}").contains(key),
                    "{key} = {value}: {error:#}"
                );
            }
        }
    }

    #[test]
    fn refresh_and_corral_are_read_and_checked() {
        let config = Config::default();
        assert_eq!(
            (config.refresh_ms, config.corral.as_str()),
            (1000, "corral")
        );
        let config = Config::parse("refresh_ms = 2500\ncorral = \"/opt/bin/corral\"").unwrap();
        assert_eq!(
            (config.refresh_ms, config.corral.as_str()),
            (2500, "/opt/bin/corral")
        );
        for bad in [
            "refresh_ms = 0",
            "refresh_ms = -5",
            "refresh_ms = 1.5",
            "corral = \" \"",
        ] {
            assert!(Config::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn mascot_settings_are_read_and_checked() {
        let config = Config::default();
        assert!(config.mascot_enabled);
        assert_eq!(config.mascot, crate::pet::Pet::Clawd);
        let config = Config::parse("mascot_enabled = false\nmascot = \"capybara\"").unwrap();
        assert!(!config.mascot_enabled);
        assert_eq!(config.mascot, crate::pet::Pet::Capybara);
        let error = Config::parse("mascot = \"dog\"").unwrap_err();
        assert!(format!("{error:#}").contains("unknown mascot"), "{error:#}");
        assert!(Config::parse("mascot_enabled = \"yes\"").is_err());
    }

    #[test]
    fn opacities_are_whole_percentages_left_out_by_default() {
        let config = Config::default();
        assert_eq!((config.sidebar_opacity, config.frame_opacity), (None, None));
        assert_eq!(config.material_opacity, None);
        let config = Config::parse("sidebar_opacity = 30\nframe_opacity = 100").unwrap();
        assert_eq!(
            (config.sidebar_opacity, config.frame_opacity),
            (Some(30), Some(100))
        );
        assert_eq!(
            Config::parse("frame_opacity = 0").unwrap().frame_opacity,
            Some(0)
        );
        assert_eq!(
            Config::parse("material_opacity = 0")
                .unwrap()
                .material_opacity,
            Some(0)
        );
        for key in ["sidebar_opacity", "frame_opacity", "material_opacity"] {
            for value in ["101", "300", "-1", "45.5", "\"45\""] {
                let error = Config::parse(&format!("{key} = {value}")).unwrap_err();
                assert!(
                    format!("{error:#}").contains(key),
                    "{key} = {value}: {error:#}"
                );
            }
        }
    }

    #[test]
    fn missing_file_is_all_defaults() {
        let path = std::env::temp_dir().join("paddock-config-test-missing/config.toml");
        let config = Config::load(&path).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.sidebar_width, 380.0);
        assert_eq!(config.theme, None);
        assert!(config.colors.is_empty());
        assert_eq!(config.font, "Menlo");
        assert_eq!(
            config.font_fallbacks,
            [
                "Symbols Nerd Font Mono",
                "FiraCode Nerd Font Mono",
                "FiraCode Nerd Font",
            ]
        );
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.line_height, 1.3);
        assert_eq!(config.ui_font, None);
        assert_eq!(config.ui_font_size, 13.0);
    }

    #[test]
    fn font_precedence_is_cli_then_config_then_defaults() {
        for text in [
            "",
            r#"
font = "Geist Mono"
font_fallbacks = ["Sarasa Mono SC", "Maple Mono NF CN"]
font_size = 14.5
line_height = 1.4
"#,
        ] {
            let mut config = Config::parse(text).unwrap();
            let original = config.clone();
            config
                .apply_font_overrides(None, vec![], None, None)
                .unwrap();
            assert_eq!(config, original);

            config
                .apply_font_overrides(Some("CLI Font".into()), vec![], None, None)
                .unwrap();
            assert_eq!(config.font, "CLI Font");
            assert_eq!(config.font_fallbacks, original.font_fallbacks);
            assert_eq!(config.font_size, original.font_size);
            assert_eq!(config.line_height, original.line_height);

            config
                .apply_font_overrides(
                    None,
                    vec!["CLI Fallback A".into(), "CLI Fallback B".into()],
                    Some(16.5),
                    Some(1.6),
                )
                .unwrap();
            assert_eq!(config.font, "CLI Font");
            assert_eq!(config.font_fallbacks, ["CLI Fallback A", "CLI Fallback B"]);
            assert_eq!(config.font_size, 16.5);
            assert_eq!(config.line_height, 1.6);
        }
        let mut config = Config::parse("font_fallbacks = []\nfont_size = 15").unwrap();
        config
            .apply_font_overrides(None, vec![], None, None)
            .unwrap();
        assert!(config.font_fallbacks.is_empty());
        assert_eq!(config.font, "Menlo");
        assert_eq!(config.font_size, 15.0);
        assert_eq!(config.line_height, 1.3);
    }

    #[test]
    fn invalid_numeric_font_overrides_name_the_key() {
        for value in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for (size, height, key) in [
                (Some(value), None, "font_size"),
                (None, Some(value), "line_height"),
            ] {
                let error = Config::default()
                    .apply_font_overrides(None, vec![], size, height)
                    .unwrap_err();
                assert!(format!("{error:#}").contains(key), "{error:#}");
            }
        }
    }

    #[test]
    fn values_are_kept_as_written() {
        let config = Config::parse(
            r##"
theme = "not-a-preset"
sidebar_width = 300

[colors]
agents_bg = "#123456"
whatever = "anything"
"##,
        )
        .unwrap();
        assert_eq!(config.theme.as_deref(), Some("not-a-preset"));
        assert_eq!(config.sidebar_width, 300.0);
        assert_eq!(config.colors["agents_bg"], "#123456");
        assert_eq!(config.colors["whatever"], "anything");
        assert_eq!(
            Config::parse("sidebar_width = 180.5")
                .unwrap()
                .sidebar_width,
            180.5
        );
    }

    #[test]
    fn unknown_keys_are_errors() {
        let error = Config::parse("left_width = 52").unwrap_err();
        assert!(format!("{error:#}").contains("left_width"), "{error:#}");
        assert!(Config::parse("[sidebar]\nwidth = 3").is_err());
    }

    #[test]
    fn invalid_values_are_errors() {
        assert!(Config::parse("theme = 3").is_err());
        assert!(Config::parse("[colors]\nagents_bg = 3").is_err());
        assert!(Config::parse("sidebar_width = \"wide\"").is_err());
        assert!(Config::parse("sidebar_width = 0").is_err());
    }

    #[test]
    fn load_reports_the_path() {
        let dir = std::env::temp_dir().join(format!("paddock-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "sidebar_width = 200\n").unwrap();
        assert_eq!(Config::load(&path).unwrap().sidebar_width, 200.0);
        std::fs::write(&path, "bogus = 1\n").unwrap();
        let error = Config::load(&path).unwrap_err();
        assert!(format!("{error:#}").contains("config.toml"), "{error:#}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn agent_models_are_read_and_checked() {
        let config = Config::parse(
            r#"
[agent_models.claude]
models = ["opus[1m]", "sonnet"]
efforts = ["low", "high"]
"#,
        )
        .unwrap();
        let claude = config.agent_models.claude.unwrap();
        assert_eq!(claude.models, ["opus[1m]", "sonnet"]);
        assert!(config.agent_models.codex.is_none());
        assert_eq!(Config::default().agent_models, Default::default());
        for bad in [
            "[agent_models.codex]\nmodels = []\nefforts = [\"low\"]",
            "[agent_models.codex]\nmodels = [3]\nefforts = [\"low\"]",
            "[agent_models.codex]\nmodels = [\"x\"]",
            "[agent_models.pi]\nmodels = [\"x\"]\nefforts = [\"low\"]",
        ] {
            assert!(Config::parse(bad).is_err(), "{bad}");
        }
    }
}

#[cfg(test)]
mod machine_tests {
    use super::*;

    #[test]
    fn machines_are_read_with_an_optional_corral() {
        let config = Config::parse(
            r#"
[[machines]]
name = "studio"
ssh = "me@studio.local"

[[machines]]
name = "vps-2"
ssh = "vps"
corral = "~/.local/bin/corral"
"#,
        )
        .unwrap();
        assert_eq!(config.machines_problem, None);
        assert_eq!(
            config.machines,
            [
                Machine {
                    name: "studio".into(),
                    ssh: "me@studio.local".into(),
                    corral: None,
                    label: None,
                    show: true,
                },
                Machine {
                    name: "vps-2".into(),
                    ssh: "vps".into(),
                    corral: Some("~/.local/bin/corral".into()),
                    label: None,
                    show: true,
                },
            ]
        );
        // None written: none, and nothing to say.
        let config = Config::parse("").unwrap();
        assert!(config.machines.is_empty() && config.machines_problem.is_none());
    }

    #[test]
    fn a_wrong_machine_is_left_out_and_said_without_failing_the_config() {
        for (entry, says) in [
            (
                "name = \"x\"\nssh = \"-oProxyCommand=evil\"",
                "must not start with -",
            ),
            ("name = \"x\"\nssh = \"me@host extra\"", "spaces or control"),
            ("name = \"x\"\nssh = \"me@host\\n\"", "spaces or control"),
            ("name = \"x\"\nssh = \"\"", "needs ssh"),
            ("name = \"local\"\nssh = \"host\"", "taken by this Mac"),
            ("name = \"Studio\"\nssh = \"host\"", "lower-case"),
            ("name = \"a b\"\nssh = \"host\"", "lower-case"),
            ("name = \"\"\nssh = \"host\"", "lower-case"),
            (
                "name = \"x\"\nssh = \"host\"\ncorral = \"corral\"",
                "must be a path",
            ),
            ("name = \"x\"\nssh = \"host\"\nport = 22", "port"),
            ("name = \"x\"", "ssh"),
        ] {
            let text = format!(
                "font_size = 15\n[[machines]]\nname = \"good\"\nssh = \"h\"\n[[machines]]\n{entry}\n"
            );
            let config = Config::parse(&text).unwrap_or_else(|e| panic!("{entry}: {e:#}"));
            // The rest of the config, and the good machine, stand.
            assert_eq!(config.font_size, 15.0);
            assert_eq!(config.machines.len(), 1, "{entry}");
            assert_eq!(config.machines[0].name, "good");
            let problem = config.machines_problem.unwrap();
            assert!(problem.contains(says), "{entry}: {problem}");
        }
    }

    #[test]
    fn a_machine_named_twice_keeps_the_first() {
        let config = Config::parse(
            "[[machines]]\nname = \"a\"\nssh = \"one\"\n[[machines]]\nname = \"a\"\nssh = \"two\"\n",
        )
        .unwrap();
        assert_eq!(config.machines.len(), 1);
        assert_eq!(config.machines[0].ssh, "one");
        assert!(
            config
                .machines_problem
                .unwrap()
                .contains("a is named twice")
        );
    }

    /// P5-123: the name shown and whether the machine is shown, both left out by default.
    #[test]
    fn a_machine_has_an_optional_label_and_can_be_hidden() {
        let config = Config::parse(
            "[[machines]]\nname = \"5090\"\nssh = \"gpu\"\nlabel = \"  RTX 5090 \"\nshow = false\n\
             [[machines]]\nname = \"vps\"\nssh = \"v\"\nshow = true\n\
             [[machines]]\nname = \"pi\"\nssh = \"p\"\n",
        )
        .unwrap();
        assert_eq!(config.machines_problem, None);
        let [gpu, vps, pi] = &config.machines[..] else {
            panic!("{:?}", config.machines);
        };
        // Shown without the spaces at its ends; the name stays who it is.
        assert_eq!((gpu.shown_name(), gpu.name.as_str()), ("RTX 5090", "5090"));
        assert!(!gpu.show);
        assert_eq!((vps.shown_name(), vps.show), ("vps", true));
        assert_eq!((pi.shown_name(), pi.show, &pi.label), ("pi", true, &None));
    }

    #[test]
    fn a_wrong_label_or_show_leaves_the_machine_out_and_says_why() {
        let long = "x".repeat(LABEL_MAX + 1);
        for (entry, says) in [
            ("label = \"\"", "must not be empty"),
            ("label = \"   \"", "must not be empty"),
            ("label = \"a\\tb\"", "control characters"),
            ("label = \"a\\u0007\"", "control characters"),
            (&format!("label = \"{long}\"")[..], "at most 40 characters"),
            ("label = 5090", "label"),
            ("show = \"no\"", "show"),
            ("show = 0", "show"),
        ] {
            let text = format!(
                "[[machines]]\nname = \"good\"\nssh = \"h\"\n[[machines]]\nname = \"x\"\nssh = \"h\"\n{entry}\n"
            );
            let config = Config::parse(&text).unwrap_or_else(|e| panic!("{entry}: {e:#}"));
            assert_eq!(config.machines.len(), 1, "{entry}");
            assert_eq!(config.machines[0].name, "good");
            let problem = config.machines_problem.unwrap();
            assert!(problem.contains(says), "{entry}: {problem}");
        }
        // As long as allowed, and in any script, is fine.
        let most = "名".repeat(LABEL_MAX);
        assert_eq!(label(&format!(" {most} ")).unwrap(), most);
    }
}
