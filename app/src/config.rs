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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: None,
            colors: BTreeMap::new(),
            sidebar_width: 240.0,
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
        Ok(config)
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
    fn missing_file_is_all_defaults() {
        let path = std::env::temp_dir().join("paddock-config-test-missing/config.toml");
        let config = Config::load(&path).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.sidebar_width, 240.0);
        assert_eq!(config.theme, None);
        assert!(config.colors.is_empty());
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
