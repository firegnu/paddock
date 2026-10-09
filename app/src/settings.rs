//! What the Settings page edits and how it writes, apart from drawing. The rules follow Saddle's
//! Settings (`src/settings.rs` at commit `df1c727`), with every edit put into effect at once
//! (P5-56): an edit is drafted, then committed over the file as it is at that moment, writing only
//! the keys that changed and keeping the file's comments and everything else; a value that can't
//! be written is dropped rather than written; Default resets one value; a colour set back to
//! Default follows the theme again (its key is removed); choosing another theme loads its colours
//! and drops the colour overrides, which can be drafted back; a file that isn't a valid config is
//! never written over. Numbers stay inside a range.
use crate::{
    config::Config,
    footer_icon::Icon,
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
    /// The theme and the colours over it.
    Appearance,
    /// How paddock reads its agents: the refresh interval and the corral command.
    Agents,
    /// Read-only: what paddock runs and how its reads and saves went.
    Diagnostics,
}

impl Page {
    pub const ALL: [Page; 4] = [
        Page::General,
        Page::Appearance,
        Page::Agents,
        Page::Diagnostics,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Appearance => "Appearance",
            Page::Agents => "Agents",
            Page::Diagnostics => "Diagnostics",
        }
    }

    /// The page's icon, in Settings' sidebar and the command palette.
    pub fn icon(self) -> Icon {
        match self {
            Page::General => Icon::Settings,
            Page::Appearance => Icon::Palette,
            Page::Agents => Icon::Agent,
            Page::Diagnostics => Icon::Pulse,
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
    /// An installed monospace family, chosen from a list; written as a string.
    MonoFont,
    /// An installed family, chosen from a list; empty for the system's, which removes the key.
    UiFont,
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
            Page::Agents,
            Kind::Integer,
            true,
        ),
        field(
            "mascot_enabled",
            "Show the mascot",
            Page::General,
            Kind::Bool,
            false,
        ),
        field("mascot", "Pet", Page::General, Kind::Pet, false),
        field(
            "ui_font",
            "Interface font",
            Page::General,
            Kind::UiFont,
            false,
        ),
        field(
            "ui_font_size",
            "Interface size",
            Page::General,
            Kind::Number,
            false,
        ),
        field(
            "font",
            "Terminal font",
            Page::General,
            Kind::MonoFont,
            false,
        ),
        field(
            "font_size",
            "Terminal size",
            Page::General,
            Kind::Number,
            false,
        ),
        field(
            "line_height",
            "Line height",
            Page::General,
            Kind::Number,
            false,
        ),
        field(
            "font_fallbacks",
            "Fallback fonts",
            Page::General,
            Kind::List,
            false,
        ),
        field("theme", "Theme", Page::Appearance, Kind::Theme, false),
    ];
    let mut colors: Vec<Field> = theme::color_keys()
        .into_iter()
        .map(|name| Field {
            key: format!("colors.{name}"),
            label: name.into(),
            page: Page::Appearance,
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
        Page::Agents,
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
        "ui_font" => config.ui_font.clone().unwrap_or_default(),
        "ui_font_size" => number(f64::from(config.ui_font_size)),
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

/// The theme and the colour overrides as the file has them, to put back after another theme.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSetting {
    pub theme: Option<String>,
    pub colors: BTreeMap<String, String>,
}

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

    /// Unlike its default, as drafted: a colour set by the user, or another setting that reads
    /// differently from its default (numbers by value, lists item by item).
    pub fn modified(&self, key: &str) -> bool {
        match self.field(key) {
            Some(field) if field.kind == Kind::Color => self.custom(key),
            Some(field) => !same(field, &self.value(key), &self.default_shown(key)),
            None => false,
        }
    }

    /// The default as the page shows it; empty for the system's interface font.
    pub fn default_shown(&self, key: &str) -> String {
        shown(&Config::default(), key)
    }

    /// Why the drafted value of `key` can't be saved, as Save would say; `None` when it can or
    /// is not drafted.
    pub fn problem(&self, key: &str) -> Option<String> {
        let (Some(field), Some(Change::Set(text))) = (self.field(key), self.changes.get(key))
        else {
            return None;
        };
        if field.kind == Kind::UiFont && text.trim().is_empty() {
            return None;
        }
        let checked = value(field, text).and_then(|value| {
            let mut document = DocumentMut::new();
            apply(&mut document, key, Some(value))?;
            Theme::from_config(&Config::parse(&document.to_string())?)?;
            Ok(())
        });
        checked.err().map(|error| format!("{error:#}"))
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

    /// Takes the file as it is now as the base, keeping the draft's edits on top.
    pub fn rebase(&mut self, disk: Option<String>) -> Result<()> {
        if let Some(text) = &disk {
            self.config = Config::parse(text)?;
        } else {
            self.config = Config::default();
        }
        self.base = disk;
        Ok(())
    }

    /// Drops every drafted value: the file as read again.
    pub fn discard(&mut self) {
        self.changes.clear();
    }

    /// Drops the drafted values that can't be written, which then read as the file has them;
    /// gives their labels in display order.
    pub fn drop_invalid(&mut self) -> Vec<String> {
        let wrong: Vec<Field> = self
            .fields
            .iter()
            .filter(|field| self.problem(&field.key).is_some())
            .cloned()
            .collect();
        for field in &wrong {
            self.changes.remove(&field.key);
        }
        wrong.into_iter().map(|field| field.label).collect()
    }

    /// Builds everything drafted over the file as it is now on `disk`, which then becomes the
    /// base with nothing left drafted; the caller writes the text. A file that isn't a valid
    /// config, or a drafted value that can't be written, gives an error and leaves the draft.
    pub fn commit(&mut self, disk: Option<String>) -> Result<Saved> {
        let mut over = Draft::new(disk.clone())?;
        over.changes = self.changes.clone();
        let Ok(saved) = over.save(disk.as_deref())? else {
            unreachable!("built over the same text it is checked against");
        };
        let text = match &saved {
            Saved::Written { text, .. } => Some(text.clone()),
            Saved::Unchanged => disk,
        };
        self.rebase(text)?;
        self.changes.clear();
        Ok(saved)
    }

    /// The theme and colour overrides as the file has them.
    pub fn theme_setting(&self) -> ThemeSetting {
        ThemeSetting {
            theme: self.config.theme.clone(),
            colors: self.config.colors.clone(),
        }
    }

    /// Drafts `setting` back: its theme, its colour overrides, and no other.
    pub fn restore_theme(&mut self, setting: ThemeSetting) {
        let theme = match setting.theme {
            Some(name) => Change::Set(name),
            None => Change::Default,
        };
        self.changes.insert("theme".into(), theme);
        let colors: Vec<(String, String)> = self
            .fields
            .iter()
            .filter_map(|f| Some((f.key.clone(), f.color()?.to_owned())))
            .collect();
        for (key, name) in colors {
            match setting.colors.get(&name) {
                Some(value) => {
                    self.changes.insert(key, Change::Set(value.clone()));
                }
                None if self.override_of(&key).is_some() => {
                    self.changes.insert(key, Change::Default);
                }
                None => {
                    self.changes.remove(&key);
                }
            }
        }
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
                Change::Set(text) if field.kind == Kind::UiFont && text.trim().is_empty() => None,
                Change::Set(text) => Some(text.clone()),
                Change::Default if matches!(field.kind, Kind::Color | Kind::UiFont) => None,
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

/// The least a number setting takes and, where there is one, the most: the steppers stop there,
/// and a typed value outside can't be written.
pub fn range(key: &str) -> Option<(f64, Option<f64>)> {
    Some(match key {
        "ui_font_size" => (6.0, Some(24.0)),
        "font_size" => (6.0, Some(72.0)),
        "line_height" => (0.5, Some(3.0)),
        // As wide as a drag can make it.
        "sidebar_width" => (10.0, Some(f64::from(crate::sidebar::MAX_WIDTH))),
        "refresh_ms" => (250.0, None),
        _ => return None,
    })
}

/// A number of `field` inside its [`range`].
fn within(field: &Field, value: f64) -> Result<()> {
    let label = &field.label;
    match range(&field.key) {
        Some((least, Some(most))) if !(least..=most).contains(&value) => {
            bail!("{label} must be from {} to {}", number(least), number(most))
        }
        Some((least, None)) if value < least => {
            bail!("{label} must be at least {}", number(least))
        }
        _ => Ok(()),
    }
}

/// Two texts of `field` that save the same: numbers by value, lists item by item.
fn same(field: &Field, a: &str, b: &str) -> bool {
    match field.kind {
        Kind::Integer | Kind::Number => match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
            (Ok(a), Ok(b)) => a == b,
            _ => a.trim() == b.trim(),
        },
        Kind::List => {
            let items = |text: &str| -> Vec<String> {
                text.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            };
            items(a) == items(b)
        }
        _ => a.trim() == b.trim(),
    }
}

/// A setting's text as a TOML value of its kind; a wrong one names the setting.
fn value(field: &Field, text: &str) -> Result<Value> {
    let text = text.trim();
    let label = &field.label;
    Ok(match field.kind {
        Kind::Integer => {
            let number = text
                .parse::<i64>()
                .with_context(|| format!("{label} must be a whole number"))?;
            within(field, number as f64)?;
            Value::from(number)
        }
        Kind::Number => {
            let number: f64 = text
                .parse()
                .ok()
                .filter(|n: &f64| n.is_finite())
                .with_context(|| format!("{label} must be a number"))?;
            within(field, number)?;
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
            if Preset::parse(text).is_err() {
                bail!("{label} must be {}", Preset::expected());
            }
            Value::from(text)
        }
        Kind::Text | Kind::Color | Kind::MonoFont | Kind::UiFont => Value::from(text),
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
        draft.set("refresh_ms", "2000");
        let (text, restart) = written(&draft);
        assert!(
            text.starts_with("# my paddock\ntheme = \"tide\"   # cool\n"),
            "{text}"
        );
        assert!(text.contains("# warm accent\nfocus = \"yellow\""), "{text}");
        assert!(text.contains("sidebar_width = 400\n"), "{text}");
        assert!(text.contains("font_size = 15.5\n"), "{text}");
        // The terminal font takes effect at once; the refresh interval still waits.
        assert_eq!(restart, ["Refresh interval"]);
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
    fn interface_font_is_saved_apart_and_system_removes_the_key() {
        let file = "# fonts\nfont = \"Geist Mono\"\nui_font = \"Avenir Next\"   # mine\n";
        let draft = Draft::new(Some(file.into())).unwrap();
        assert_eq!(draft.value("ui_font"), "Avenir Next");
        assert_eq!(draft.value("ui_font_size"), "13");
        assert_eq!(draft.value("font"), "Geist Mono");

        let mut draft = Draft::new(Some(file.into())).unwrap();
        draft.set("ui_font", "Helvetica Neue");
        draft.set("ui_font_size", "15");
        let (text, restart) = written(&draft);
        assert!(
            text.contains("ui_font = \"Helvetica Neue\"   # mine\n"),
            "{text}"
        );
        assert!(text.contains("ui_font_size = 15\n"), "{text}");
        assert!(text.contains("font = \"Geist Mono\"\n"), "{text}");
        assert!(restart.is_empty(), "{restart:?}");

        // Choosing System, or Default, leaves the key out: the system font.
        for system in [Some(""), None] {
            let mut draft = Draft::new(Some(file.into())).unwrap();
            match system {
                Some(value) => draft.set("ui_font", value),
                None => draft.reset("ui_font"),
            }
            assert_eq!(draft.value("ui_font"), "");
            let (text, _) = written(&draft);
            assert!(!text.contains("ui_font"), "{text}");
            assert!(
                text.starts_with("# fonts\nfont = \"Geist Mono\"\n"),
                "{text}"
            );
            assert_eq!(Config::parse(&text).unwrap().ui_font, None);
        }

        // No interface key yet: an older file, with the terminal font kept as it is.
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        assert_eq!(draft.value("ui_font"), "");
        draft.set("ui_font_size", "12.5");
        let (text, _) = written(&draft);
        let config = Config::parse(&text).unwrap();
        assert_eq!(config.ui_font_size, 12.5);
        assert_eq!(config.font, "Geist Mono");
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
            ("font_size", "0", "Terminal size must be from 6 to 72"),
            ("ui_font_size", "-1", "Interface size must be from 6 to 24"),
            ("ui_font_size", "big", "Interface size must be a number"),
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
        // Dropped: the file as it is, no edits left.
        draft.discard();
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
        for field in &colors {
            assert!(!draft.value(&field.key).is_empty(), "{}", field.key);
        }
        let page = |key: &str| fields.iter().find(|f| f.key == key).unwrap().page;
        assert_eq!(page("theme"), Page::Appearance);
        assert!(colors.iter().all(|f| f.page == Page::Appearance));
    }

    #[test]
    fn the_refresh_interval_sits_with_the_corral_command_on_agents() {
        let fields = fields();
        let on = |page| -> Vec<&str> {
            fields
                .iter()
                .filter(|f| f.page == page)
                .map(|f| f.key.as_str())
                .collect()
        };
        assert_eq!(on(Page::Agents), ["refresh_ms", "corral"]);
        assert_eq!(on(Page::General).len(), 9);
        assert!(!on(Page::General).contains(&"refresh_ms"));
        assert_eq!(
            Page::ALL.map(Page::label),
            ["General", "Appearance", "Agents", "Diagnostics"]
        );
    }

    #[test]
    fn a_setting_unlike_its_default_offers_the_default() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        // Saved unlike the default, or drafted so.
        assert!(draft.modified("font"));
        assert_eq!(draft.default_shown("font"), "Menlo");
        assert!(draft.modified("colors.focus"));
        assert!(!draft.modified("font_size"));
        assert!(!draft.modified("colors.claude"));
        assert!(!draft.modified("ui_font"));
        draft.set("font_size", "15");
        assert!(draft.modified("font_size"));
        assert_eq!(draft.default_shown("font_size"), "14");
        // The same number written another way is still the default.
        draft.set("font_size", "14.0");
        assert!(!draft.modified("font_size"));
        draft.set(
            "font_fallbacks",
            "Symbols Nerd Font Mono,FiraCode Nerd Font Mono, FiraCode Nerd Font",
        );
        assert!(!draft.modified("font_fallbacks"));
        draft.reset("font");
        assert!(!draft.modified("font"));
        draft.set("ui_font", "Avenir Next");
        assert!(draft.modified("ui_font"));
        assert_eq!(draft.default_shown("ui_font"), "");
        assert_eq!(draft.default_shown("mascot"), "clawd");
    }

    /// What `commit` wrote, or `None` when it wrote nothing.
    fn committed(draft: &mut Draft, disk: Option<&str>) -> Option<String> {
        match draft.commit(disk.map(str::to_owned)).unwrap() {
            Saved::Written { text, .. } => Some(text),
            Saved::Unchanged => None,
        }
    }

    #[test]
    fn numbers_stay_between_the_steppers_least_and_a_most() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        for (key, value, says) in [
            ("font_size", "1", "Terminal size must be from 6 to 72"),
            ("font_size", "73", "Terminal size must be from 6 to 72"),
            ("ui_font_size", "5", "Interface size must be from 6 to 24"),
            ("ui_font_size", "30", "Interface size must be from 6 to 24"),
            ("line_height", "0.3", "Line height must be from 0.5 to 3"),
            ("line_height", "3.5", "Line height must be from 0.5 to 3"),
            ("sidebar_width", "5", "Sidebar width must be from 10 to 560"),
            (
                "sidebar_width",
                "561",
                "Sidebar width must be from 10 to 560",
            ),
            (
                "sidebar_width",
                "900",
                "Sidebar width must be from 10 to 560",
            ),
            ("refresh_ms", "100", "Refresh interval must be at least 250"),
        ] {
            draft.set(key, value);
            let problem = draft.problem(key).unwrap_or_default();
            assert!(problem.contains(says), "{key} {value}: {problem}");
        }
        for (key, value) in [
            ("font_size", "6"),
            ("font_size", "72"),
            ("ui_font_size", "24"),
            ("line_height", "0.5"),
            ("line_height", "3"),
            ("sidebar_width", "560"),
            ("refresh_ms", "250"),
            ("refresh_ms", "60000"),
        ] {
            draft.set(key, value);
            assert_eq!(draft.problem(key), None, "{key} {value}");
        }
        assert_eq!(range("font_size"), Some((6.0, Some(72.0))));
        assert_eq!(range("refresh_ms"), Some((250.0, None)));
        assert_eq!(range("font"), None);
    }

    #[test]
    fn the_sidebar_fits_its_most_at_the_largest_interface_size() {
        // Settings writes the least the header needs when a width is narrower; it must be one it
        // can write.
        let (_, most) = range("ui_font_size").unwrap();
        let config = Config {
            ui_font_size: most.unwrap() as f32,
            ..Config::default()
        };
        let least = crate::sidebar::min_width(&crate::fonts::UiFont::from_config(&config));
        assert!(
            f64::from(least) <= range("sidebar_width").unwrap().1.unwrap(),
            "{least}"
        );
    }

    #[test]
    fn a_commit_writes_over_the_file_as_it_is_now() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        // Changed elsewhere since Settings read it: the edit goes on top, the rest stays.
        let disk = format!("line_height = 1.4\n{FILE}");
        draft.set("sidebar_width", "400");
        let text = committed(&mut draft, Some(&disk)).unwrap();
        assert!(
            text.starts_with("line_height = 1.4\n# my paddock\n"),
            "{text}"
        );
        assert!(text.contains("sidebar_width = 400\n"), "{text}");
        // What was written is now the base: nothing left drafted.
        assert!(!draft.edited());
        assert_eq!(draft.value("line_height"), "1.4");
        assert_eq!(draft.value("sidebar_width"), "400");
        // Nothing drafted writes nothing.
        assert_eq!(committed(&mut draft, Some(&text)), None);
        // No file yet: one is made.
        let mut draft = Draft::new(None).unwrap();
        draft.set("mascot", "cat");
        let text = committed(&mut draft, None).unwrap();
        assert_eq!(Config::parse(&text).unwrap().mascot, Pet::Cat);
    }

    #[test]
    fn a_broken_file_is_never_written_over() {
        for broken in ["theme = ", "no_such_key = 1\n", "sidebar_width = -3\n"] {
            let mut draft = Draft::new(Some(FILE.into())).unwrap();
            draft.set("font_size", "15");
            assert!(draft.commit(Some(broken.into())).is_err(), "{broken}");
            // The edit stays drafted, for the page to drop.
            assert!(draft.edited());
        }
    }

    #[test]
    fn values_that_cant_be_written_are_dropped_and_named() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        draft.set("font_size", "1");
        draft.set("colors.focus", "#ff");
        draft.set("sidebar_width", "400");
        assert_eq!(draft.drop_invalid(), ["Terminal size", "focus"]);
        // Back to what the file says.
        assert_eq!(draft.value("font_size"), "14");
        assert_eq!(draft.value("colors.focus"), "yellow");
        // The valid one is still there to be written.
        let text = committed(&mut draft, Some(FILE)).unwrap();
        assert!(text.contains("sidebar_width = 400\n"), "{text}");
        assert!(text.contains("focus = \"yellow\""), "{text}");
        assert!(draft.drop_invalid().is_empty());
    }

    #[test]
    fn another_theme_can_be_undone_with_its_colours() {
        let file = format!("{FILE}claude = \"#d97757\"\n");
        let mut draft = Draft::new(Some(file.clone())).unwrap();
        let before = draft.theme_setting();
        assert_eq!(before.theme.as_deref(), Some("tide"));
        assert_eq!(before.colors.len(), 2);
        draft.set("theme", "lagoon");
        let text = committed(&mut draft, Some(&file)).unwrap();
        assert!(Config::parse(&text).unwrap().colors.is_empty(), "{text}");
        // Undo: the theme and every colour come back.
        draft.restore_theme(before);
        let text = committed(&mut draft, Some(&text)).unwrap();
        let config = Config::parse(&text).unwrap();
        assert_eq!(config.theme.as_deref(), Some("tide"));
        assert_eq!(
            config.colors,
            BTreeMap::from([
                ("claude".to_owned(), "#d97757".to_owned()),
                ("focus".to_owned(), "yellow".to_owned()),
            ])
        );
        // A colour set after the switch goes again on undo; no theme written means the default.
        let mut draft = Draft::new(Some("font = \"Menlo\"\n".into())).unwrap();
        let before = draft.theme_setting();
        draft.set("theme", "lagoon");
        draft.set("colors.focus", "#cfc27a");
        let text = committed(&mut draft, Some("font = \"Menlo\"\n")).unwrap();
        draft.restore_theme(before);
        let text = committed(&mut draft, Some(&text)).unwrap();
        let config = Config::parse(&text).unwrap();
        assert!(config.colors.is_empty(), "{text}");
        assert_eq!(draft.theme_name(), "dune");
    }

    #[test]
    fn a_wrong_value_names_its_problem_before_save() {
        let mut draft = Draft::new(Some(FILE.into())).unwrap();
        assert_eq!(draft.problem("sidebar_width"), None);
        for (key, value, says) in [
            ("sidebar_width", "wide", "Sidebar width must be a number"),
            (
                "refresh_ms",
                "1.5",
                "Refresh interval must be a whole number",
            ),
            ("refresh_ms", "0", "Refresh interval must be at least 250"),
            ("font_size", "0", "Terminal size must be from 6 to 72"),
            ("colors.focus", "chartreuse", "focus"),
            ("corral", " ", "corral"),
        ] {
            draft.set(key, value);
            let problem = draft.problem(key).unwrap_or_default();
            assert!(problem.contains(says), "{key}: {problem}");
        }
        for (key, value) in [
            ("sidebar_width", "400"),
            ("font_size", "15.5"),
            ("colors.focus", "#cfc27a"),
            ("ui_font", ""),
        ] {
            draft.set(key, value);
            assert_eq!(draft.problem(key), None, "{key}");
        }
    }
}
