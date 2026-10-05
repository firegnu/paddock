//! `~/.config/paddock/config.toml`. A missing file means every default; unknown keys are errors,
//! as in Saddle. Theme values are kept as written for the theme module to interpret.
use anyhow::{Context as _, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The preset name as written.
    pub theme: Option<String>,
    /// The `[colors]` overrides as written.
    pub colors: BTreeMap<String, String>,
    /// Width of the Agents sidebar, in points.
    pub sidebar_width: f32,
    pub font: String,
    pub font_fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: None,
            colors: BTreeMap::new(),
            sidebar_width: 380.0,
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
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text)?;
        ensure!(
            config.sidebar_width.is_finite() && config.sidebar_width > 0.0,
            "sidebar_width must be positive"
        );
        config.validate_fonts()?;
        Ok(config)
    }

    fn validate_fonts(&self) -> Result<()> {
        for (key, value) in [
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
    fn invalid_font_values_name_the_key() {
        for (key, values) in [
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
}
