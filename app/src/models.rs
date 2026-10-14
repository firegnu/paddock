//! Where New Agent's models and efforts come from (DESIGN §13, P5-100). For Codex, first the
//! catalog `codex debug models` gives, asked at most once a month (or on Settings' Refresh) and
//! kept in paddock's own file beside the layout; then `[agent_models.codex]` in the config; then
//! paddock's own list. For Claude, which has no way to list its models, the config and then
//! paddock's own. The config is only read here, never written.
use crate::{activity, new_agent::Tool};
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

/// How long what codex gave is good for before it is asked again.
pub const MONTH: u64 = 30 * 24 * 60 * 60;
/// How long `codex debug models` may take; it asks the network.
const ASK_TIMEOUT: Duration = Duration::from_secs(60);
/// The efforts of a kind that names no others, and of a model not in its list.
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
/// The effort chosen with a model when it has it.
pub const DEFAULT_EFFORT: &str = "high";

const CLAUDE: [&str; 5] = ["opus[1m]", "fable", "opus", "sonnet", "haiku"];
/// The model Codex starts at, wherever its list came from, while the list has it (user 10-14).
const CODEX_DEFAULT: &str = "gpt-6-astra";
/// What codex listed on 10-14, in its order.
const CODEX: [&str; 7] = [
    "gpt-6.1-sol",
    "gpt-6-astra",
    "gpt-6-sol",
    "gpt-6-luna",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
];

/// A model and the efforts it takes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub efforts: Vec<String>,
}

/// `[agent_models]` in the config: a list for each kind that has models, either left out.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configured {
    pub claude: Option<KindModels>,
    pub codex: Option<KindModels>,
}

/// One kind's lists in the config: the first model is the one chosen with the kind.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindModels {
    pub models: Vec<String>,
    pub efforts: Vec<String>,
}

impl Configured {
    /// Each list has something in it, and nothing blank.
    pub fn validate(&self) -> Result<()> {
        for (key, kind) in [("claude", &self.claude), ("codex", &self.codex)] {
            let Some(kind) = kind else { continue };
            for (field, names) in [("models", &kind.models), ("efforts", &kind.efforts)] {
                if names.is_empty() || names.iter().any(|name| name.trim().is_empty()) {
                    bail!("agent_models.{key}.{field} must name at least one, none blank");
                }
            }
        }
        Ok(())
    }

    fn of(&self, tool: Tool) -> Option<&KindModels> {
        match tool {
            Tool::Claude => self.claude.as_ref(),
            Tool::Codex => self.codex.as_ref(),
            _ => None,
        }
    }
}

/// Where a kind's list came from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// From codex, got at this time (seconds since the epoch).
    Codex {
        at: u64,
    },
    Config,
    Own,
}

impl Source {
    /// As Settings says it: `codex, Oct 14`, `config.toml`, `paddock's list`.
    pub fn label(&self) -> String {
        match *self {
            Source::Codex { at } => {
                let at = at as i64;
                let (year, month, day) =
                    activity::date(activity::day_of(at, activity::local_offset(at)));
                let (this_year, _, _) = activity::date(activity::today());
                let month = activity::MONTHS[month as usize - 1];
                if year == this_year {
                    format!("codex, {month} {day}")
                } else {
                    format!("codex, {month} {day}, {year}")
                }
            }
            Source::Config => "config.toml".into(),
            Source::Own => "paddock's list".into(),
        }
    }
}

/// A kind's models and where they came from.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub models: Vec<Model>,
    pub source: Source,
}

impl List {
    /// The model chosen with the kind.
    pub fn first(&self) -> Option<&str> {
        self.models.first().map(|model| model.name.as_str())
    }

    /// The models on one line in Settings: all of up to five, else the first three and how many
    /// more.
    pub fn shown(&self) -> String {
        let names: Vec<&str> = self
            .models
            .iter()
            .map(|model| model.name.as_str())
            .collect();
        if names.len() <= 5 {
            return names.join(" · ");
        }
        format!("{} · +{}", names[..3].join(" · "), names.len() - 3)
    }

    /// The efforts `model` takes; a model not in the list (typed into the command) takes
    /// paddock's usual ones.
    pub fn efforts(&self, model: Option<&str>) -> Vec<&str> {
        match self.models.iter().find(|m| Some(m.name.as_str()) == model) {
            Some(model) => model.efforts.iter().map(String::as_str).collect(),
            None => EFFORTS.to_vec(),
        }
    }

    /// The effort to have with `model`: `keep` when it takes it, else the usual one, else its last.
    pub fn effort_for(&self, model: Option<&str>, keep: Option<&str>) -> Option<String> {
        let efforts = self.efforts(model);
        [keep, Some(DEFAULT_EFFORT)]
            .into_iter()
            .flatten()
            .find(|effort| efforts.contains(effort))
            .or(efforts.last().copied())
            .map(str::to_owned)
    }
}

/// Both kinds' lists, as New Agent offers them.
#[derive(Clone, Debug, PartialEq)]
pub struct Lists {
    pub claude: List,
    pub codex: List,
}

impl Default for Lists {
    /// paddock's own, before anything is read.
    fn default() -> Self {
        Self::new(&Configured::default(), None)
    }
}

impl Lists {
    pub fn new(configured: &Configured, cache: Option<&Cache>) -> Self {
        let list = |tool: Tool, own: &[&str]| {
            if tool == Tool::Codex
                && let Some(cache) = cache.filter(|cache| !cache.models.is_empty())
            {
                return List {
                    models: cache.models.clone(),
                    source: Source::Codex {
                        at: cache.fetched_at.unwrap_or(cache.asked_at),
                    },
                };
            }
            let (names, efforts, source) = match configured.of(tool) {
                Some(kind) => (kind.models.clone(), kind.efforts.clone(), Source::Config),
                None => (
                    own.iter().map(|name| (*name).to_owned()).collect(),
                    EFFORTS.map(str::to_owned).to_vec(),
                    Source::Own,
                ),
            };
            List {
                models: names
                    .into_iter()
                    .map(|name| Model {
                        name,
                        efforts: efforts.clone(),
                    })
                    .collect(),
                source,
            }
        };
        Self {
            claude: list(Tool::Claude, &CLAUDE),
            codex: list(Tool::Codex, &CODEX),
        }
    }

    /// The model chosen with `tool`: for Codex [`CODEX_DEFAULT`] while its list has it, else the
    /// list's first.
    pub fn default_model(&self, tool: Tool) -> Option<&str> {
        let list = self.of(tool)?;
        if tool == Tool::Codex && list.models.iter().any(|model| model.name == CODEX_DEFAULT) {
            return Some(CODEX_DEFAULT);
        }
        list.first()
    }

    /// `tool`'s list; none for a kind without models.
    pub fn of(&self, tool: Tool) -> Option<&List> {
        match tool {
            Tool::Claude => Some(&self.claude),
            Tool::Codex => Some(&self.codex),
            _ => None,
        }
    }
}

/// What paddock keeps of asking codex.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    /// When codex was last asked, whether or not it answered (seconds since the epoch).
    pub asked_at: u64,
    /// When it last answered, and what.
    pub fetched_at: Option<u64>,
    pub models: Vec<Model>,
    /// Why the last asking failed; none when it answered.
    pub error: Option<String>,
}

/// Whether codex should be asked again: never asked, or a month has passed.
pub fn due(cache: Option<&Cache>, now: u64) -> bool {
    cache.is_none_or(|cache| now >= cache.asked_at.saturating_add(MONTH))
}

/// Where the cache is kept: beside the layout.
pub fn cache_path() -> Option<PathBuf> {
    crate::layout_state::default_path().map(|path| path.with_file_name("codex-models.json"))
}

/// The cache at `path`; none when it is missing or cannot be read.
pub fn load_cache(path: &Path) -> Option<Cache> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Writes beside it first, so a crash never leaves half a file.
pub fn save_cache(path: &Path, cache: &Cache) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let saving = path.with_extension("json.saving");
    fs::write(&saving, serde_json::to_vec_pretty(cache)?)?;
    fs::rename(saving, path)
}

#[derive(Deserialize)]
struct Catalog {
    models: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    slug: String,
    visibility: String,
    #[serde(default)]
    priority: i64,
    #[serde(default)]
    supported_reasoning_levels: Vec<Level>,
}

#[derive(Deserialize)]
struct Level {
    effort: String,
}

/// The models codex lists for people to choose, in its order, each with its efforts.
pub fn parse_catalog(bytes: &[u8]) -> Result<Vec<Model>> {
    let catalog: Catalog = serde_json::from_slice(bytes).context("reading codex's catalog")?;
    let mut entries: Vec<_> = catalog
        .models
        .into_iter()
        .filter(|entry| entry.visibility == "list")
        .collect();
    entries.sort_by_key(|entry| entry.priority);
    let models: Vec<_> = entries
        .into_iter()
        .map(|entry| Model {
            name: entry.slug,
            efforts: entry
                .supported_reasoning_levels
                .into_iter()
                .map(|level| level.effort)
                .collect(),
        })
        .collect();
    if models.is_empty() {
        bail!("codex's catalog lists no models");
    }
    Ok(models)
}

/// Asks `program` (codex) for its catalog. The error is a short reason for Settings.
pub fn ask(program: &str, cancel: &AtomicBool) -> Result<Vec<Model>, String> {
    let output = crate::command::run(program, &["debug", "models"], None, ASK_TIMEOUT, cancel)
        .map_err(|error| {
            let missing = error
                .chain()
                .filter_map(|cause| cause.downcast_ref::<io::Error>())
                .any(|cause| cause.kind() == io::ErrorKind::NotFound);
            if missing {
                "codex not found".to_owned()
            } else {
                error.to_string()
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(
                || format!("codex exited with {}", output.status),
                str::to_owned,
            ));
    }
    parse_catalog(&output.stdout).map_err(|_| "unexpected output".to_owned())
}

/// Asks codex now and keeps what it says at `path`: its models when it answers; otherwise what
/// was kept before, with why it failed. Either way codex is not asked again for a month.
pub fn refresh(program: &str, path: &Path, now: u64, cancel: &AtomicBool) -> Cache {
    let before = load_cache(path);
    let cache = match ask(program, cancel) {
        Ok(models) => Cache {
            asked_at: now,
            fetched_at: Some(now),
            models,
            error: None,
        },
        Err(error) => Cache {
            asked_at: now,
            fetched_at: before.as_ref().and_then(|cache| cache.fetched_at),
            models: before.map(|cache| cache.models).unwrap_or_default(),
            error: Some(error),
        },
    };
    // Unsaved, it is asked again next time New Agent opens; nothing worse.
    let _ = save_cache(path, &cache);
    cache
}

/// Codex is being asked, from New Agent or Settings: the other does not ask as well.
static ASKING: AtomicBool = AtomicBool::new(false);

pub fn asking() -> bool {
    ASKING.load(Ordering::SeqCst)
}

/// The lists as New Agent offers them now: what paddock kept from codex, then `configured`.
pub fn load(configured: &Configured) -> Lists {
    Lists::new(configured, kept().as_ref())
}

/// What paddock kept from codex, if anything.
pub fn kept() -> Option<Cache> {
    cache_path().and_then(|path| load_cache(&path))
}

/// Asks codex, with `force` (Refresh) now, else only when a month has passed since it was last
/// asked; none when it was not asked, or is being asked already. Blocks: run it in the background.
pub fn ask_codex(force: bool) -> Option<Cache> {
    let path = cache_path()?;
    if !force && !due(load_cache(&path).as_ref(), now()) {
        return None;
    }
    if ASKING.swap(true, Ordering::SeqCst) {
        return None;
    }
    let cache = refresh("codex", &path, now(), &AtomicBool::new(false));
    ASKING.store(false, Ordering::SeqCst);
    Some(cache)
}

/// Seconds since the epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(name: &str, efforts: &[&str]) -> Model {
        Model {
            name: name.into(),
            efforts: efforts.iter().map(|e| (*e).into()).collect(),
        }
    }

    fn kind(models: &[&str], efforts: &[&str]) -> KindModels {
        KindModels {
            models: models.iter().map(|m| (*m).into()).collect(),
            efforts: efforts.iter().map(|e| (*e).into()).collect(),
        }
    }

    fn fetched(at: u64, models: Vec<Model>) -> Cache {
        Cache {
            asked_at: at,
            fetched_at: Some(at),
            models,
            error: None,
        }
    }

    const CATALOG: &str = r#"{"models":[
        {"slug":"hidden","visibility":"hide","priority":0,"supported_reasoning_levels":[{"effort":"low"}]},
        {"slug":"second","visibility":"list","priority":2,"supported_reasoning_levels":[{"effort":"low","description":"x"},{"effort":"ultra"}]},
        {"slug":"first","visibility":"list","priority":1,"supported_reasoning_levels":[{"effort":"medium"},{"effort":"max"}]}
    ]}"#;

    #[test]
    fn own_lists_keep_opus_1m_first_and_codex_as_listed() {
        let lists = Lists::default();
        assert_eq!(lists.claude.source, Source::Own);
        assert_eq!(lists.claude.first(), Some("opus[1m]"));
        let claude: Vec<_> = lists
            .claude
            .models
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(claude, ["opus[1m]", "fable", "opus", "sonnet", "haiku"]);
        assert_eq!(lists.codex.first(), Some("gpt-6.1-sol"));
        assert_eq!(lists.codex.models.len(), 7);
        assert!(!lists.codex.models.iter().any(|m| m.name == "gpt-5.5"));
        assert_eq!(lists.codex.efforts(Some("gpt-5.5")), EFFORTS);
        // Codex starts at gpt-6-astra wherever the list came from, while the list has it.
        assert_eq!(lists.default_model(Tool::Codex), Some("gpt-6-astra"));
        assert_eq!(lists.default_model(Tool::Claude), Some("opus[1m]"));
        assert_eq!(lists.default_model(Tool::Pi), None);
        let got = Lists::new(
            &Configured::default(),
            Some(&fetched(
                1,
                vec![model("new", &["low"]), model("gpt-6-astra", &["high"])],
            )),
        );
        assert_eq!(got.default_model(Tool::Codex), Some("gpt-6-astra"));
        let without = Lists::new(
            &Configured::default(),
            Some(&fetched(1, vec![model("new", &["low"])])),
        );
        assert_eq!(without.default_model(Tool::Codex), Some("new"));
        assert!(lists.of(Tool::Pi).is_none() && lists.of(Tool::Amp).is_none());
        assert_eq!(lists.of(Tool::Codex), Some(&lists.codex));
    }

    #[test]
    fn the_config_comes_before_paddocks_own() {
        let configured = Configured {
            claude: Some(kind(&["sonnet", "opus"], &["low", "high"])),
            codex: None,
        };
        let lists = Lists::new(&configured, None);
        assert_eq!(lists.claude.source, Source::Config);
        assert_eq!(lists.claude.models[1], model("opus", &["low", "high"]));
        assert_eq!(lists.codex.source, Source::Own);
    }

    #[test]
    fn what_codex_gave_comes_first_however_old() {
        let configured = Configured {
            claude: None,
            codex: Some(kind(&["mine"], &["low"])),
        };
        let cache = fetched(5, vec![model("first", &["medium", "max"])]);
        let lists = Lists::new(&configured, Some(&cache));
        assert_eq!(lists.codex.source, Source::Codex { at: 5 });
        assert_eq!(lists.codex.first(), Some("first"));
        // Never answered, or answered nothing: the config.
        let failed = Cache {
            asked_at: 9,
            fetched_at: None,
            models: vec![],
            error: Some("codex not found".into()),
        };
        let lists = Lists::new(&configured, Some(&failed));
        assert_eq!(lists.codex.source, Source::Config);
        assert_eq!(lists.codex.first(), Some("mine"));
        // Claude never takes codex's.
        assert_eq!(
            Lists::new(&configured, Some(&cache)).claude.source,
            Source::Own
        );
    }

    #[test]
    fn efforts_follow_the_model() {
        let list = List {
            models: vec![
                model("a", &["low", "high", "ultra"]),
                model("b", &["low", "medium"]),
            ],
            source: Source::Codex { at: 1 },
        };
        assert_eq!(list.efforts(Some("a")), ["low", "high", "ultra"]);
        assert_eq!(list.efforts(Some("typed")), EFFORTS);
        assert_eq!(list.efforts(None), EFFORTS);
        // Kept when the model takes it; else high; else its last.
        assert_eq!(
            list.effort_for(Some("a"), Some("ultra")).as_deref(),
            Some("ultra")
        );
        assert_eq!(
            list.effort_for(Some("b"), Some("ultra")).as_deref(),
            Some("medium")
        );
        assert_eq!(
            list.effort_for(Some("a"), Some("max")).as_deref(),
            Some("high")
        );
        assert_eq!(list.effort_for(Some("a"), None).as_deref(), Some("high"));
    }

    #[test]
    fn the_config_must_name_something() {
        assert!(Configured::default().validate().is_ok());
        let ok = Configured {
            claude: Some(kind(&["opus"], &["high"])),
            codex: Some(kind(&["x"], &["low"])),
        };
        assert!(ok.validate().is_ok());
        for bad in [
            kind(&[], &["low"]),
            kind(&["x"], &[]),
            kind(&["x", " "], &["low"]),
        ] {
            let configured = Configured {
                claude: None,
                codex: Some(bad),
            };
            let error = configured.validate().unwrap_err().to_string();
            assert!(error.contains("agent_models.codex"), "{error}");
        }
    }

    #[test]
    fn settings_says_the_list_and_where_it_came_from() {
        let lists = Lists::default();
        assert_eq!(
            lists.claude.shown(),
            "opus[1m] · fable · opus · sonnet · haiku"
        );
        assert_eq!(
            lists.codex.shown(),
            "gpt-6.1-sol · gpt-6-astra · gpt-6-sol · +4"
        );
        assert_eq!(Source::Own.label(), "paddock's list");
        assert_eq!(Source::Config.label(), "config.toml");
        // 2026-10-14 12:00 UTC: the day is this machine's, the year left out when it is this one.
        let at = 1_791_979_200;
        let label = Source::Codex { at }.label();
        assert!(
            label == "codex, Oct 14" || label.starts_with("codex, Oct 1"),
            "{label}"
        );
        let old = Source::Codex {
            at: 20 * 86_400 + 43_200,
        }
        .label();
        assert_eq!(old, "codex, Jan 21, 1970");
    }

    #[test]
    fn asked_once_a_month() {
        assert!(due(None, 100));
        let cache = Cache {
            asked_at: 100,
            fetched_at: None,
            models: vec![],
            error: Some("x".into()),
        };
        assert!(!due(Some(&cache), 100 + MONTH - 1));
        assert!(due(Some(&cache), 100 + MONTH));
    }

    #[test]
    fn the_catalog_lists_what_codex_offers_in_its_order() {
        let models = parse_catalog(CATALOG.as_bytes()).unwrap();
        assert_eq!(
            models,
            [
                model("first", &["medium", "max"]),
                model("second", &["low", "ultra"])
            ]
        );
        assert!(parse_catalog(b"not json").is_err());
        assert!(parse_catalog(br#"{"models":[]}"#).is_err());
        assert!(parse_catalog(br#"{"models":[{"slug":"h","visibility":"hide"}]}"#).is_err());
    }

    #[test]
    fn the_cache_reads_back_and_a_bad_one_is_none() {
        let dir = std::env::temp_dir().join(format!("paddock-models-{}", std::process::id()));
        let path = dir.join("state/codex-models.json");
        let cache = fetched(7, vec![model("a", &["low"])]);
        save_cache(&path, &cache).unwrap();
        assert_eq!(load_cache(&path), Some(cache));
        fs::write(&path, "{").unwrap();
        assert_eq!(load_cache(&path), None);
        assert_eq!(load_cache(&dir.join("missing.json")), None);
        let _ = fs::remove_dir_all(dir);
    }

    /// A stand-in for codex: a script printing `output` and leaving with `status`.
    fn fake_codex(dir: &Path, output: &str, status: i32) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("codex");
        fs::create_dir_all(dir).unwrap();
        fs::write(
            &path,
            format!(
                "#!/bin/sh\n[ \"$1 $2\" = 'debug models' ] || exit 9\ncat <<'EOF'\n{output}\nEOF\nexit {status}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path.display().to_string()
    }

    #[test]
    fn refreshing_keeps_what_codex_says_or_why_it_could_not() {
        let dir = std::env::temp_dir().join(format!("paddock-refresh-{}", std::process::id()));
        let path = dir.join("codex-models.json");
        let cancel = AtomicBool::new(false);

        let good = fake_codex(&dir.join("good"), CATALOG, 0);
        let cache = refresh(&good, &path, 1000, &cancel);
        assert_eq!(cache.asked_at, 1000);
        assert_eq!(cache.fetched_at, Some(1000));
        assert_eq!(cache.models.len(), 2);
        assert_eq!(cache.error, None);
        assert_eq!(load_cache(&path).as_ref(), Some(&cache));

        // A failure keeps the models got before and says why.
        let bad = fake_codex(&dir.join("bad"), "garbage", 0);
        let cache = refresh(&bad, &path, 2000, &cancel);
        assert_eq!((cache.asked_at, cache.fetched_at), (2000, Some(1000)));
        assert_eq!(cache.models.len(), 2);
        assert_eq!(cache.error.as_deref(), Some("unexpected output"));

        let failing = fake_codex(&dir.join("failing"), "", 1);
        let cache = refresh(&failing, &path, 3000, &cancel);
        assert!(cache.error.is_some());
        assert_eq!(cache.fetched_at, Some(1000));

        let missing = dir.join("nowhere/codex").display().to_string();
        let cache = refresh(&missing, &path, 4000, &cancel);
        assert_eq!(cache.error.as_deref(), Some("codex not found"));
        assert_eq!(load_cache(&path).unwrap().asked_at, 4000);
        let _ = fs::remove_dir_all(dir);
    }
}
