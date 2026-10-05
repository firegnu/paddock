//! What the Settings page edits and how it saves, apart from drawing. The rules follow Saddle's
//! Settings (`src/settings.rs` at commit `df1c727`): edits are a draft; Save writes only the keys
//! that changed, keeping the file's comments and everything else; Default resets one value (and is
//! written on Save); a colour set back to Default follows the theme again (its key is removed);
//! choosing another theme loads its colours and drops the colour overrides; a file changed on disk
//! since it was read is never overwritten.
use crate::{
    config::Config,
    pet::Pet,
    preset::Preset,
    theme::{self, Theme},
};
use anyhow::{Context as _, Result, bail};
use std::collections::BTreeMap;
use toml_edit::{DocumentMut, Item, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Colors,
    Advanced,
}

impl Page {
    pub const ALL: [Page; 3] = [Page::General, Page::Colors, Page::Advanced];

    pub fn label(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Colors => "Colors",
            Page::Advanced => "Advanced",
        }
    }
}

/// How a setting is edited and written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A whole number, written as an integer.
    Integer,
    /// A number, written as an integer when it is whole.
    Number,
    Bool,
    Pet,
    Theme,
    /// Free text, written as a string.
    Text,
    /// Comma-separated, written as an array of strings.
    List,
    /// A `[colors]` value.
    Color,
}

#[derive(Clone, Debug)]
pub struct Field {
    /// The config key; colours are `colors.<name>`.
    pub key: String,
    pub label: String,
    pub page: Page,
    /// The heading it sits under on its page, empty for none.
    pub group: &'static str,
    pub kind: Kind,
    /// Takes effect only after a restart.
    pub restart: bool,
}

impl Field {
    /// The `[colors]` name of a colour field.
    pub fn color(&self) -> Option<&str> {
        self.key.strip_prefix("colors.")
    }
}

/// Colour groups in display order: Saddle's, then the terminal's.
const GROUPS: [&str; 5] = [
    "Interface",
    "Agents panel",
    "Status",
    "Agent types",
    "Terminal",
];

fn group(name: &str) -> &'static str {
    match name {
        _ if name.starts_with("terminal_") => GROUPS[4],
        "agent_selected" => GROUPS[1],
        _ if name.starts_with("agents_") => GROUPS[1],
        _ if name.starts_with("agent_") => GROUPS[2],
        "claude" | "codex" | "pi" | "omp" => GROUPS[3],
        _ => GROUPS[0],
    }
}

/// Every setting, page by page, in display order.
pub fn fields() -> Vec<Field> {
    let field = |key: &str, label: &str, page, kind, restart| Field {
        key: key.into(),
        label: label.into(),
        page,
        group: "",
        kind,
        restart,
    };
    let mut list = vec![
        field(
            "sidebar_width",
            "Sidebar width",
            Page::General,
            Kind::Number,
            false,
        ),
        field(
            "refresh_ms",
            "Refresh interval",
            Page::General,
            Kind::Integer,
            true,
        ),
        field("mascot_enabled", "Mascot", Page::General, Kind::Bool, false),
        field("mascot", "Pet", Page::General, Kind::Pet, false),
        field("font", "Font", Page::General, Kind::Text, true),
        field(
            "font_fallbacks",
            "Fallback fonts",
            Page::General,
            Kind::List,
            true,
        ),
        field("font_size", "Font size", Page::General, Kind::Number, true),
        field(
            "line_height",
            "Line height",
            Page::General,
            Kind::Number,
            true,
        ),
        field("theme", "Theme", Page::Colors, Kind::Theme, false),
    ];
    let mut colors: Vec<Field> = theme::color_keys()
        .into_iter()
        .map(|name| Field {
            key: format!("colors.{name}"),
            label: name.into(),
            page: Page::Colors,
            group: group(name),
            kind: Kind::Color,
            restart: false,
        })
        .collect();
    colors.sort_by_key(|f| GROUPS.iter().position(|g| *g == f.group));
    list.extend(colors);
    list.push(field(
        "corral",
        "corral command",
        Page::Advanced,
        Kind::Text,
        true,
    ));
    list
}

/// A setting as the page shows it.
fn shown(config: &Config, key: &str) -> String {
    match key {
        "sidebar_width" => number(f64::from(config.sidebar_width)),
        "refresh_ms" => config.refresh_ms.to_string(),
        "mascot_enabled" => config.mascot_enabled.to_string(),
        "mascot" => config.mascot.name().into(),
        "font" => config.font.clone(),
        "font_fallbacks" => config.font_fallbacks.join(", "),
        "font_size" => number(f64::from(config.font_size)),
        "line_height" => number(f64::from(config.line_height)),
        "theme" => config.theme.clone().unwrap_or_else(|| "dune".into()),
        "corral" => config.corral.clone(),
        _ => String::new(),
    }
}

/// Whole numbers without a decimal point; the shortest form that reads back the same otherwise.
fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        // f32 values widened to f64 carry noise; round to what f32 holds.
        format!("{}", value as f32)
    }
}

/// An edit to one setting.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Set(String),
    /// Back to the default: written as the default value, or for a colour, following the theme.
    Default,
}

/// What Save did.
#[derive(Debug, PartialEq)]
pub enum Saved {
    /// The new file text and config, and the labels of saved settings that need a restart.
    Written {
        text: String,
        config: Box<Config>,
        restart: Vec<String>,
    },
    /// Nothing changed; nothing written.
    Unchanged,
}

/// The file changed on disk since Settings read it; nothing was saved.
#[derive(Debug, PartialEq)]
pub struct Conflict;

pub struct Draft {
    fields: Vec<Field>,
    /// The file as read, `None` when there was none.
    base: Option<String>,
    config: Config,
    changes: BTreeMap<String, Change>,
}

impl Draft {
    /// A draft over the config file's text as read (`None`: no file yet).
    pub fn new(base: Option<String>) -> Result<Self> {
        let config = match &base {
            Some(text) => Config::parse(text)?,
            None => Config::default(),
        };
        Ok(Self {
            fields: fields(),
            base,
            config,
            changes: BTreeMap::new(),
        })
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    fn field(&self, key: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.key == key)
    }

    pub fn edited(&self) -> bool {
        !self.changes.is_empty()
    }

    /// The theme as drafted.
    pub fn theme_name(&self) -> String {
        match self.changes.get("theme") {
            Some(Change::Set(name)) => name.clone(),
            Some(Change::Default) => "dune".into(),
            None => shown(&self.config, "theme"),
        }
    }

    /// The drafted theme's own colours, without any override.
    fn preset_theme(&self) -> Option<Theme> {
        Preset::parse(&self.theme_name()).ok()?;
        let config = Config {
            theme: Some(self.theme_name()),
            ..Config::default()
        };
        Theme::from_config(&config).ok()
    }

    /// A colour's override as drafted: `None` when it follows the theme.
    fn override_of(&self, key: &str) -> Option<String> {
        match self.changes.get(key) {
            Some(Change::Set(value)) => Some(value.clone()),
            Some(Change::Default) => None,
            None => {
                let name = key.strip_prefix("colors.")?;
                self.config.colors.get(name).cloned()
            }
        }
    }

    /// The value as the page shows it: drafted, saved, or for a colour following the theme, the
    /// theme's own.
    pub fn value(&self, key: &str) -> String {
        let Some(field) = self.field(key) else {
            return String::new();
        };
        if field.kind == Kind::Color {
            return self.override_of(key).unwrap_or_else(|| {
                self.preset_theme()
                    .and_then(|theme| theme.written(field.color().unwrap()))
                    .unwrap_or_default()
            });
        }
        match self.changes.get(key) {
            Some(Change::Set(value)) => value.clone(),
            Some(Change::Default) => shown(&Config::default(), key),
            None => shown(&self.config, key),
        }
    }

    /// A colour set by the user rather than by the theme.
    pub fn custom(&self, key: &str) -> bool {
        key.starts_with("colors.") && self.override_of(key).is_some()
    }

    /// Changed in this draft.
    pub fn changed(&self, key: &str) -> bool {
        self.changes.contains_key(key)
    }

    /// The drafted colours where they are valid, over the drafted theme, for swatches.
    pub fn theme(&self) -> Option<Theme> {
        let colors = self
            .fields
            .iter()
            .filter_map(|f| Some((f.color()?.to_owned(), self.override_of(&f.key)?)))
            .filter(|(name, value)| {
                let probe = Config {
                    theme: Some(self.theme_name()),
                    colors: BTreeMap::from([(name.clone(), value.clone())]),
                    ..Config::default()
                };
                Theme::from_config(&probe).is_ok()
            })
            .collect();
        let config = Config {
            theme: Some(self.theme_name()),
            colors,
            ..Config::default()
        };
        Theme::from_config(&config).ok()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if key == "theme" {
            self.choose_theme(value);
        } else {
            self.changes.insert(key.into(), Change::Set(value.into()));
        }
    }

    /// Another theme loads all its colours: every colour override goes.
    fn choose_theme(&mut self, name: &str) {
        self.changes
            .insert("theme".into(), Change::Set(name.into()));
        let colors: Vec<String> = self
            .fields
            .iter()
            .filter(|f| f.kind == Kind::Color)
            .map(|f| f.key.clone())
            .collect();
        for key in colors {
            if self.override_of(&key).is_some() {
                self.changes.insert(key, Change::Default);
            } else {
                self.changes.remove(&key);
            }
        }
    }

    pub fn reset(&mut self, key: &str) {
        if key == "theme" {
            self.choose_theme("dune");
            self.changes.insert("theme".into(), Change::Default);
        } else {
            self.changes.insert(key.into(), Change::Default);
        }
    }

    /// Takes the file as it is now as the base, keeping the draft's edits on top (Keep my edits).
    pub fn rebase(&mut self, disk: Option<String>) -> Result<()> {
        if let Some(text) = &disk {
            self.config = Config::parse(text)?;
        } else {
            self.config = Config::default();
        }
        self.base = disk;
        Ok(())
    }

    /// Drops the draft and takes the file as it is now (Discard my edits).
    pub fn discard(&mut self, disk: Option<String>) -> Result<()> {
        self.changes.clear();
        self.rebase(disk)
    }

    /// Checks the draft against the file as it is on `disk` and builds what to write.
    pub fn save(&self, disk: Option<&str>) -> Result<Result<Saved, Conflict>> {
        if disk != self.base.as_deref() {
            return Ok(Err(Conflict));
        }
        if self.changes.is_empty() {
            return Ok(Ok(Saved::Unchanged));
        }
        let mut document: DocumentMut = self
            .base
            .as_deref()
            .unwrap_or("")
            .parse()
            .context("the config file is not valid TOML")?;
        let defaults = Config::default();
        for (key, change) in &self.changes {
            let field = self.field(key).context("unknown setting")?;
            let text = match change {
                Change::Set(text) => Some(text.clone()),
                Change::Default if field.kind == Kind::Color => None,
                Change::Default => Some(shown(&defaults, key)),
            };
            let value = match text {
                Some(text) => Some(value(field, &text)?),
                None => None,
            };
            apply(&mut document, key, value)?;
        }
        let text = document.to_string();
        let config = Config::parse(&text)?;
        Theme::from_config(&config)?;
        let restart = self
            .changes
            .keys()
            .filter_map(|key| self.field(key))
            .filter(|field| field.restart)
            .map(|field| field.label.clone())
            .collect();
        Ok(Ok(Saved::Written {
            text,
            config: Box::new(config),
            restart,
        }))
    }
}

/// A setting's text as a TOML value of its kind; a wrong one names the setting.
fn value(field: &Field, text: &str) -> Result<Value> {
    let text = text.trim();
    let label = &field.label;
    Ok(match field.kind {
        Kind::Integer => Value::from(
            text.parse::<i64>()
                .with_context(|| format!("{label} must be a whole number"))?,
        ),
        Kind::Number => {
            let number: f64 = text
                .parse()
                .ok()
                .filter(|n: &f64| n.is_finite())
                .with_context(|| format!("{label} must be a number"))?;
            if number.fract() == 0.0 {
                Value::from(number as i64)
            } else {
                Value::from(number)
            }
        }
        Kind::Bool => Value::from(
            text.parse::<bool>()
                .with_context(|| format!("{label} must be true or false"))?,
        ),
        Kind::Pet => {
            Pet::parse(text).map_err(|e| anyhow::anyhow!(e))?;
            Value::from(text)
        }
        Kind::Theme => {
            if !matches!(text, "dune" | "tide" | "lagoon") {
                bail!("{label} must be dune, tide or lagoon");
            }
            Value::from(text)
        }
        Kind::Text | Kind::Color => Value::from(text),
        Kind::List => {
            let items: toml_edit::Array = text
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            Value::Array(items)
        }
    })
}

/// Sets one key in the document, keeping its place and its comment; `None` removes it. A
/// `colors.` key goes into the `[colors]` table, which is created when needed.
fn apply(document: &mut DocumentMut, key: &str, value: Option<Value>) -> Result<()> {
    let (table, key): (&mut dyn toml_edit::TableLike, &str) = match key.split_once('.') {
        Some((table, key)) => {
            let item = match document.as_table_mut().entry(table) {
                toml_edit::Entry::Vacant(_) if value.is_none() => return Ok(()),
                entry => entry.or_insert(toml_edit::table()),
            };
            let table = item
                .as_table_like_mut()
                .with_context(|| format!("[{table}] is not a table"))?;
            (table, key)
        }
        None => (document.as_table_mut(), key),
    };
    match (value, table.get_mut(key)) {
        (None, _) => {
            table.remove(key);
        }
        (Some(mut value), Some(item)) => {
            if let Some(old) = item.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            *item = Item::Value(value);
        }
        (Some(value), None) => {
            table.insert(key, Item::Value(value));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "# my paddock\ntheme = \"tide\"   # cool\nfont = \"Geist Mono\"\n\n[colors]\n# warm accent\nfocus = \"yellow\"\n";

    fn written(draft: &Draft) -> (String, Vec<String>) {
        match draft.save(draft.base.as_deref()).unwrap().unwrap() {
            Saved::Written { text, restart, .. } => (text, restart),
            Saved::Unchanged => panic!("nothing written"),
        }
    }

    #[test]
    fn only_changed_keys_are_written_and_comments_stay() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        draft.set("sidebar_width", "400");
        draft.set("font_size", "15.5");
        let (text, restart) = written(&draft);
        assert!(
            text.starts_with("# my paddock\ntheme = \"tide\"   # cool\n"),
            "{text}"
        );
        assert!(text.contains("# warm accent\nfocus = \"yellow\""), "{text}");
        assert!(text.contains("sidebar_width = 400\n"), "{text}");
        assert!(text.contains("font_size = 15.5\n"), "{text}");
        assert_eq!(restart, ["Font size"]);
        // A replaced value keeps its trailing comment.
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        draft.set("font", "Menlo");
        let (text, _) = written(&draft);
        assert!(text.contains("font = \"Menlo\"\n"), "{text}");
    }

    #[test]
    fn no_file_yet_is_created_and_nothing_changed_writes_nothing() {
        let mut draft = Draft::new(None).unwrap();
        assert_eq!(draft.save(None).unwrap(), Ok(Saved::Unchanged));
        draft.set("mascot", "cat");
        draft.set("mascot_enabled", "false");
        draft.set("font_fallbacks", "Sarasa Mono SC,  Maple Mono NF CN");
        let (text, _) = written(&draft);
        let config = Config::parse(&text).unwrap();
        assert_eq!(config.mascot, Pet::Cat);
        assert!(!config.mascot_enabled);
        assert_eq!(
            config.font_fallbacks,
            ["Sarasa Mono SC", "Maple Mono NF CN"]
        );
    }

    #[test]
    fn another_theme_drops_the_colour_overrides() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        assert!(draft.custom("colors.focus"));
        assert_eq!(draft.value("colors.focus"), "yellow");
        draft.set("theme", "lagoon");
        assert!(!draft.custom("colors.focus"));
        // The colour now shows the new theme's own value.
        let lagoon = Theme::from_config(&Config {
            theme: Some("lagoon".into()),
            ..Config::default()
        })
        .unwrap();
        assert_eq!(
            draft.value("colors.focus"),
            lagoon.written("focus").unwrap()
        );
        let (text, _) = written(&draft);
        let config = Config::parse(&text).unwrap();
        assert_eq!(config.theme.as_deref(), Some("lagoon"));
        assert!(config.colors.is_empty());
        // A colour edited afterwards is custom again.
        draft.set("colors.terminal_blue", "#7aa2f7");
        assert!(draft.custom("colors.terminal_blue"));
    }

    #[test]
    fn default_writes_the_default_and_a_colour_follows_the_theme() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        draft.reset("font");
        draft.reset("colors.focus");
        assert_eq!(draft.value("font"), "Menlo");
        assert!(!draft.custom("colors.focus"));
        let (text, _) = written(&draft);
        assert!(text.contains("font = \"Menlo\""), "{text}");
        let config = Config::parse(&text).unwrap();
        assert!(!config.colors.contains_key("focus"));
        assert_eq!(config.theme.as_deref(), Some("tide"));
    }

    #[test]
    fn wrong_values_name_the_setting_and_nothing_is_saved() {
        for (key, value, says) in [
            ("sidebar_width", "wide", "Sidebar width must be a number"),
            (
                "refresh_ms",
                "1.5",
                "Refresh interval must be a whole number",
            ),
            ("mascot", "dog", "unknown mascot"),
            ("colors.focus", "chartreuse", "focus"),
            ("font_size", "0", "font_size"),
        ] {
            let mut draft = Draft::new(Some(FILE.into())).unwrap();
            draft.set(key, value);
            let error = draft.save(Some(FILE)).unwrap_err();
            assert!(format!("{error:#}").contains(says), "{key}: {error:#}");
        }
    }

    #[test]
    fn a_file_changed_on_disk_is_not_overwritten() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        draft.set("sidebar_width", "400");
        let disk = format!("line_height = 1.4\n{FILE}");
        assert_eq!(draft.save(Some(&disk)).unwrap(), Err(Conflict));
        assert_eq!(draft.save(None).unwrap(), Err(Conflict));
        // Keep my edits: the edits go on top of the file as it is now.
        draft.rebase(Some(disk.clone())).unwrap();
        let (text, _) = written(&draft);
        assert!(text.contains("line_height = 1.4") && text.contains("sidebar_width = 400"));
        // Discard my edits: the file as it is, no edits left.
        draft.discard(Some(disk.clone())).unwrap();
        assert!(!draft.edited());
        assert_eq!(draft.value("sidebar_width"), "380");
    }

    #[test]
    fn every_colour_has_a_field_in_its_group() {
        let fields = fields();
        let colors: Vec<_> = fields.iter().filter(|f| f.kind == Kind::Color).collect();
        assert_eq!(colors.len(), 41 + 20);
        let groups: Vec<_> = colors.iter().map(|f| f.group).collect();
        let mut order = groups.clone();
        order.dedup();
        assert_eq!(order, GROUPS);
        let draft = Draft::new(None).unwrap();
        // Every colour shows a value the page can put back.
        for field in colors {
            assert!(!draft.value(&field.key).is_empty(), "{}", field.key);
        }
        assert_eq!(fields.iter().filter(|f| f.page == Page::General).count(), 8);
        assert!(
            fields
                .iter()
                .any(|f| f.key == "corral" && f.page == Page::Advanced)
        );
    }
}
