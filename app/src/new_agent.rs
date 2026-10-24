//! The New Agent form's rules, as in Saddle's form (`src/launch.rs` at commit `df1c727`): a
//! project directory, the kind of agent, a controller (always `main`) or a regular agent (named),
//! a prefix, the full command, a first message and where to open it; what it turns into is one
//! public `corral start` call. Starting and stopping run through `corral` here too. For Claude and
//! Codex a model and an effort can be chosen; they are written into the command the way P5-21
//! reads them back, and a command edited by hand is read back into the choices. Presets fill in
//! the kind, model, effort and role at once, and are kept in the config file.
use crate::{config::Config, corral::Client, layout::Direction, models::Lists};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub const CODEX_COMMAND: &str = "codex --yolo";
pub const CLAUDE_COMMAND: &str = "claude";
/// The prefix when the directory gives none, as in Saddle.
const FALLBACK_PREFIX: &str = "agents";
/// How long `corral start` and `corral stop` may take, as in Saddle.
const CORRAL_TIMEOUT: Duration = Duration::from_secs(120);
/// How many recently used projects are remembered.
const RECENT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Codex,
    Claude,
    Pi,
    Omp,
    Amp,
    Agy,
    Kimi,
}

impl Tool {
    /// In the order the form offers them.
    pub const ALL: [Tool; 7] = [
        Tool::Claude,
        Tool::Codex,
        Tool::Pi,
        Tool::Omp,
        Tool::Amp,
        Tool::Agy,
        Tool::Kimi,
    ];

    /// The kind as corral and the kind icons name it.
    pub fn kind(self) -> &'static str {
        match self {
            Tool::Claude => "claude",
            Tool::Codex => "codex",
            Tool::Pi => "pi",
            Tool::Omp => "omp",
            Tool::Amp => "amp",
            Tool::Agy => "agy",
            Tool::Kimi => "kimi",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Claude => "Claude",
            Tool::Codex => "Codex",
            Tool::Pi => "pi",
            Tool::Omp => "omp",
            Tool::Amp => "amp",
            Tool::Agy => "agy",
            Tool::Kimi => "kimi",
        }
    }

    /// The command it starts with, before a model or effort is asked for.
    pub fn command(self) -> &'static str {
        match self {
            Tool::Claude => CLAUDE_COMMAND,
            Tool::Codex => CODEX_COMMAND,
            Tool::Pi => "pi",
            Tool::Omp => "omp",
            Tool::Amp => "amp",
            Tool::Agy => "agy",
            Tool::Kimi => "kimi",
        }
    }

    /// Whether `corral start --prompt` takes its first message: not amp, whose plugin corral
    /// installs in its own configuration (ranch R6).
    pub fn takes_prompt(self) -> bool {
        self != Tool::Amp
    }

    /// Whether it cannot start without a first message: agy fires no hook until it has one, so
    /// it would stay Starting (ranch R8).
    pub fn needs_prompt(self) -> bool {
        self == Tool::Agy
    }

    /// Whether corral works with it through a plugin in its own configuration (P5-98, P5-88), or a
    /// block of hooks in a configuration file it shares with the user (kimi, P5-179).
    pub fn has_plugin(self) -> bool {
        matches!(self, Tool::Amp | Tool::Agy | Tool::Kimi)
    }

    /// Whether a model and effort are chosen for it; the others keep to their own settings. The
    /// lists to choose from are in [`crate::models`].
    pub fn has_models(self) -> bool {
        matches!(self, Tool::Claude | Tool::Codex)
    }

    fn from_program(program: &str) -> Option<Tool> {
        Tool::ALL.into_iter().find(|tool| tool.kind() == program)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The active pane; a running shell there is kept and a new tab used instead.
    Current,
    Tab,
    Split(Direction),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    Regular,
    Controller,
}

/// A named set of the kind, model, effort and role, as the config file keeps it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub name: String,
    pub kind: Tool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub role: Role,
    /// The fence profile a regular agent starts in (DESIGN §13 P5-120); none for no sandbox, and
    /// always for a controller.
    #[serde(default)]
    pub sandbox: Option<String>,
}

/// The preset the presets row lights: by its place in the list, and whether the form has been
/// changed from it since it was picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lit {
    pub index: usize,
    pub edited: bool,
}

/// Which presets the row shows, in order, when they are `widths` wide with `gap` between them
/// and the row has `room`: all when they fit; otherwise the first that fit beside a `more` wide
/// `+N`, the lit one (`keep`) always among them.
pub fn fit_presets(
    widths: &[f32],
    keep: Option<usize>,
    room: f32,
    gap: f32,
    more: f32,
) -> Vec<usize> {
    let total = |shown: &[usize]| {
        shown.iter().map(|&index| widths[index]).sum::<f32>()
            + gap * shown.len().saturating_sub(1) as f32
    };
    let all: Vec<usize> = (0..widths.len()).collect();
    if total(&all) <= room {
        return all;
    }
    let room = room - more - gap;
    let mut shown: Vec<usize> = Vec::new();
    for index in all {
        shown.push(index);
        if total(&shown) > room {
            shown.pop();
            break;
        }
    }
    if let Some(keep) = keep.filter(|keep| *keep < widths.len() && !shown.contains(keep)) {
        shown.push(keep);
        while shown.len() > 1 && total(&shown) > room {
            shown.remove(shown.len() - 2);
        }
    }
    shown
}

/// The presets a config file without any gets.
pub fn default_presets() -> Vec<Preset> {
    let preset = |name: &str, kind, model: &str, effort: &str, role| Preset {
        name: name.into(),
        kind,
        model: Some(model.into()),
        effort: Some(effort.into()),
        role,
        sandbox: None,
    };
    vec![
        preset(
            "Controller",
            Tool::Claude,
            "opus[1m]",
            "high",
            Role::Controller,
        ),
        preset("Developer", Tool::Claude, "opus[1m]", "high", Role::Regular),
        preset(
            "Codex builder",
            Tool::Codex,
            "gpt-6-astra",
            "high",
            Role::Regular,
        ),
        preset(
            "Quick look",
            Tool::Claude,
            "sonnet",
            "medium",
            Role::Regular,
        ),
    ]
}

/// The config's presets; the default four when it has none written (an empty list stays empty).
pub fn presets(config: &Config) -> Vec<Preset> {
    config.agent_presets.clone().unwrap_or_else(default_presets)
}

/// Writes `presets` into the config file at `path` as `agent_presets`, keeping everything else
/// in it as written; the file must still be a valid config afterwards. Returns the config read
/// back.
pub fn save_presets(path: &Path, presets: &[Preset]) -> Result<Config> {
    use toml_edit::{Array, DocumentMut, InlineTable, Value};
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let mut document: DocumentMut = text
        .parse()
        .with_context(|| format!("{} is not valid TOML", path.display()))?;
    let mut list = Array::new();
    for preset in presets {
        let mut table = InlineTable::new();
        table.insert("name", preset.name.as_str().into());
        table.insert("kind", preset.kind.kind().into());
        for (key, value) in [("model", &preset.model), ("effort", &preset.effort)] {
            if let Some(value) = value {
                table.insert(key, value.as_str().into());
            }
        }
        let role = match preset.role {
            Role::Regular => "regular",
            Role::Controller => "controller",
        };
        table.insert("role", role.into());
        if let Some(sandbox) = preset
            .sandbox
            .as_deref()
            .filter(|_| preset.role == Role::Regular)
        {
            table.insert("sandbox", sandbox.into());
        }
        let mut value = Value::InlineTable(table);
        value.decor_mut().set_prefix("\n  ");
        list.push_formatted(value);
    }
    if !presets.is_empty() {
        list.set_trailing_comma(true);
        list.set_trailing("\n");
    }
    document["agent_presets"] = toml_edit::value(list);
    let text = document.to_string();
    let config = Config::parse(&text).context("the presets would not make a valid config")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("toml.saving");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(config)
}

/// Where the recently used projects are kept, beside the layout.
pub fn recent_path() -> Option<PathBuf> {
    crate::layout_state::default_path().map(|path| path.with_file_name("recent-projects.json"))
}

/// The recently used projects, latest first; none when there is no list or it cannot be read.
pub fn load_recent(path: &Path) -> Vec<String> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Puts `project` first in `recent`, once, keeping the latest few.
pub fn remember(recent: &mut Vec<String>, project: &str) {
    recent.retain(|known| known != project);
    recent.insert(0, project.to_owned());
    recent.truncate(RECENT);
}

pub fn save_recent(path: &Path, recent: &[String]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(recent).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("json.saving");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

/// What a command's words say: the kind of agent its program is and, for `claude` and `codex`,
/// the model and effort it asks for (P5-21: `claude` `--model`, `--effort`; `codex` `-m` or
/// `--model`, `-c model_reasoning_effort=…`), the last valid one of each, and which words ask.
#[derive(Debug, Default, PartialEq)]
pub struct Reading<'a> {
    pub tool: Option<Tool>,
    pub model: Option<&'a str>,
    pub effort: Option<&'a str>,
    /// The words asking for a model or an effort, all before any `--`.
    asking: Vec<usize>,
}

pub fn read(words: &[String]) -> Reading<'_> {
    let mut reading = Reading::default();
    let Some(first) = words.first() else {
        return reading;
    };
    let program = first.rsplit('/').next().unwrap_or_default();
    reading.tool = Tool::from_program(program);
    let (claude, codex) = (program == "claude", program == "codex");
    if !claude && !codex {
        return reading;
    }
    let mut at = 1;
    while at < words.len() {
        let arg = words[at].as_str();
        let next = words.get(at + 1).map(String::as_str);
        let value = next.filter(|value| !value.is_empty());
        let pair = at..(at + 2).min(words.len());
        match arg {
            "--" => break,
            "--model" => {
                reading.model = value.or(reading.model);
                reading.asking.extend(pair);
                at += 2;
                continue;
            }
            "-m" if codex => {
                reading.model = value.or(reading.model);
                reading.asking.extend(pair);
                at += 2;
                continue;
            }
            "--effort" if claude => {
                reading.effort = value.or(reading.effort);
                reading.asking.extend(pair);
                at += 2;
                continue;
            }
            "-c" | "--config" if codex => {
                if let Some(value) =
                    next.and_then(|arg| arg.strip_prefix("model_reasoning_effort="))
                {
                    reading.effort = Some(value.trim_matches(['\'', '"']));
                    reading.asking.extend(pair);
                }
                at += 2;
                continue;
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--model=") {
                    reading.model = Some(value)
                        .filter(|value| !value.is_empty())
                        .or(reading.model);
                    reading.asking.push(at);
                } else if codex && let Some(value) = arg.strip_prefix("-m") {
                    let value = value.strip_prefix('=').unwrap_or(value);
                    reading.model = Some(value)
                        .filter(|value| !value.is_empty())
                        .or(reading.model);
                    reading.asking.push(at);
                } else if codex
                    && let Some(value) = arg.strip_prefix("--config=model_reasoning_effort=")
                {
                    reading.effort = Some(value.trim_matches(['\'', '"']));
                    reading.asking.push(at);
                } else if claude && let Some(value) = arg.strip_prefix("--effort=") {
                    reading.effort = Some(value)
                        .filter(|value| !value.is_empty())
                        .or(reading.effort);
                    reading.asking.push(at);
                }
            }
        }
        at += 1;
    }
    reading
}

#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub project: String,
    pub tool: Tool,
    /// The chosen model and effort, for a kind that has them.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// A regular agent, named; otherwise the project's controller, `main`.
    pub regular: bool,
    pub prefix: String,
    /// The regular agent's name; a controller is always `main`.
    pub name: String,
    pub command: String,
    pub prompt: String,
    pub place: Place,
    /// The agents there are, by full name, so a suggested name is a free one.
    pub names: Vec<String>,
    /// The preset last picked, by name: it stays lit after the form is changed.
    picked: Option<String>,
    /// The prefix was typed, so it no longer follows the project.
    prefix_typed: bool,
    /// The name was typed, so it no longer follows the kind.
    name_typed: bool,
    /// The models and efforts offered for Claude and Codex.
    models: Lists,
    /// The fence profile it starts in (DESIGN §13 P5-120); none for no sandbox. Only for a
    /// regular agent of a kind fence has data for, with a profile fence offers.
    pub sandbox: Option<String>,
    /// What fence offers; none while it is not known or fence is not there.
    offer: Option<Offer>,
    /// fence has not answered yet since the form was made: "not known", not "not there".
    looking: bool,
    /// A preset's sandbox, picked while fence was still being looked for: taken once fence
    /// answers and allows it, unless the role, the kind or the sandbox was changed meanwhile.
    pending: Option<String>,
}

/// What fence offers New Agent: its profiles' names and the kinds it has data for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Offer {
    pub profiles: Vec<String>,
    pub kinds: Vec<String>,
}

impl Offer {
    pub fn of(profiles: &crate::fence::Profiles) -> Self {
        Self {
            profiles: profiles.profiles.iter().map(|p| p.name.clone()).collect(),
            kinds: profiles.kinds.iter().map(|k| k.kind.clone()).collect(),
        }
    }

    /// Whether agents of kind `tool` can be sandboxed at all.
    pub fn supports(&self, tool: Tool) -> bool {
        self.kinds.iter().any(|kind| kind == tool.kind())
    }

    /// Whether `profile` can be used for agents of kind `tool`.
    pub fn allows(&self, profile: &str, tool: Tool) -> bool {
        self.supports(tool) && self.profiles.iter().any(|p| p == profile)
    }
}

/// A prefix from the project directory's name: its last part, with anything but letters, digits,
/// `-`, `_` and `.` turned into `-`.
pub fn suggest_prefix(project: &str) -> String {
    let last = project
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let prefix: String = last
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let prefix = prefix.trim_start_matches(['-', '.']).to_owned();
    if prefix.is_empty() {
        FALLBACK_PREFIX.into()
    } else {
        prefix
    }
}

/// `~` and `~/…` as the home directory.
fn expand_home(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{home}{rest}")
        }
        _ => path.to_owned(),
    }
}

impl Form {
    pub fn new(project: String, place: Place) -> Self {
        let mut form = Self {
            prefix: suggest_prefix(&project),
            project,
            tool: Tool::Codex,
            model: None,
            effort: None,
            regular: false,
            name: String::new(),
            command: CODEX_COMMAND.into(),
            prompt: String::new(),
            place,
            names: Vec::new(),
            picked: None,
            prefix_typed: false,
            name_typed: false,
            models: Lists::default(),
            sandbox: None,
            offer: None,
            looking: true,
            pending: None,
        };
        form.follow_name();
        form
    }

    /// What fence offers from now on; none when it is not there. A profile chosen that it no
    /// longer allows goes back to none.
    pub fn set_offer(&mut self, offer: Option<Offer>) {
        self.offer = offer;
        self.looking = false;
        // A preset picked while looking gets its sandbox now, if fence allows it.
        if let Some(pending) = self.pending.take()
            && self.sandbox.is_none()
        {
            self.sandbox = Some(pending);
        }
        self.refit_sandbox();
    }

    pub fn offer(&self) -> Option<&Offer> {
        self.offer.as_ref()
    }

    /// A preset's sandbox is waiting for fence to answer: Start waits for it too, rather than
    /// start without it.
    pub fn sandbox_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The command runs a kind paddock knows, and it is the kind chosen: a program typed by hand
    /// that paddock can't read as one has no sandbox, as fence's rules are by kind.
    pub fn runs_kind(&self) -> bool {
        shell_words::split(&self.command).is_ok_and(|words| read(&words).tool == Some(self.tool))
    }

    /// The profile it starts in, as the form is now: none for a controller, a command of no
    /// kind fence knows, or a profile fence doesn't offer.
    fn sandbox_now(&self) -> Option<&String> {
        self.sandbox.as_ref().filter(|profile| self.fits(profile))
    }

    /// `profile` can be had as the form is.
    fn fits(&self, profile: &str) -> bool {
        self.regular
            && self.runs_kind()
            && self
                .offer
                .as_ref()
                .is_some_and(|offer| offer.allows(profile, self.tool))
    }

    /// Whether a sandbox can be chosen as the form is: fence is there, the agent is regular and
    /// fence has data for its kind.
    pub fn can_sandbox(&self) -> bool {
        self.regular
            && self.runs_kind()
            && self.offer.as_ref().is_some_and(|o| o.supports(self.tool))
    }

    /// Starts it in `profile`, or in none.
    pub fn set_sandbox(&mut self, profile: Option<String>) {
        self.pending = None;
        self.sandbox = profile;
        self.refit_sandbox();
    }

    /// A controller, or a regular agent; a controller has no sandbox, as corral is blocked inside
    /// one and it could not hand out work (ranch's answer 1).
    pub fn set_regular(&mut self, regular: bool) {
        self.pending = None;
        self.regular = regular;
        self.refit_sandbox();
    }

    /// The profile `preset` would start in now: none when fence does not allow it for the kind;
    /// while fence is still being looked for, the preset's own (it is waited for).
    fn sandbox_of(&self, preset: &Preset) -> Option<String> {
        let regular = preset
            .sandbox
            .clone()
            .filter(|_| preset.role == Role::Regular);
        if self.looking {
            return regular;
        }
        let offer = self.offer.as_ref()?;
        regular.filter(|p| offer.allows(p, preset.kind))
    }

    /// Back to no sandbox when the one chosen can't be had as the form now is; a preset's waiting
    /// one goes too once the command is of no kind paddock knows.
    fn refit_sandbox(&mut self) {
        if self.sandbox_now().is_none() {
            self.sandbox = None;
        }
        if !self.regular || !self.runs_kind() {
            self.pending = None;
        }
    }

    /// Another project; the prefix follows it until it has been typed.
    pub fn set_project(&mut self, project: String) {
        if !self.prefix_typed {
            self.prefix = suggest_prefix(&project);
        }
        self.project = project;
        self.follow_name();
    }

    pub fn type_prefix(&mut self, prefix: String) {
        self.prefix_typed = true;
        self.prefix = prefix;
        self.follow_name();
    }

    pub fn type_name(&mut self, name: String) {
        self.name_typed = true;
        self.name = name;
    }

    /// The agents there are now; an untyped name moves to a free one.
    pub fn set_names(&mut self, names: Vec<String>) {
        self.names = names;
        self.follow_name();
    }

    /// Until it is typed, a regular agent's name is its kind and the first free number:
    /// `claude-1`, then `claude-2` when that is taken.
    fn follow_name(&mut self) {
        if self.name_typed {
            return;
        }
        let kind = self.tool.kind();
        self.name = (1..)
            .map(|n| format!("{kind}-{n}"))
            .find(|name| !self.names.contains(&format!("{}/{name}", self.prefix)))
            .unwrap_or_default();
    }

    /// Another kind; the command becomes that kind's own, asking for no model or effort.
    pub fn choose_tool(&mut self, tool: Tool) {
        self.pending = None;
        self.tool = tool;
        self.model = None;
        self.effort = None;
        self.command = tool.command().into();
        self.follow_name();
        self.refit_sandbox();
    }

    /// The models and efforts to offer from now on; what is chosen stays.
    pub fn set_lists(&mut self, lists: Lists) {
        self.models = lists;
    }

    /// The model the kind starts at.
    pub fn default_model(&self) -> Option<&str> {
        self.models.default_model(self.tool)
    }

    /// Where the kind's models came from; none for a kind without.
    pub fn source(&self) -> Option<&crate::models::Source> {
        self.models.of(self.tool).map(|list| &list.source)
    }

    /// The models the kind offers; none for a kind without.
    pub fn model_names(&self) -> Vec<String> {
        self.models
            .of(self.tool)
            .map(|list| list.models.iter().map(|model| model.name.clone()).collect())
            .unwrap_or_default()
    }

    /// The efforts the chosen model takes.
    pub fn efforts(&self) -> Vec<String> {
        self.models
            .of(self.tool)
            .map(|list| {
                list.efforts(self.model.as_deref())
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Another kind, at its first model and normal effort when it has them.
    pub fn pick_tool(&mut self, tool: Tool) {
        self.choose_tool(tool);
        if let Some(list) = self.models.of(tool) {
            self.model = self.models.default_model(tool).map(str::to_owned);
            self.effort = list.effort_for(self.model.as_deref(), None);
            self.ask();
        }
    }

    /// Another model; the effort stays when the model takes it.
    pub fn set_model(&mut self, model: &str) {
        self.model = Some(model.into());
        if let Some(list) = self.models.of(self.tool) {
            self.effort = list.effort_for(Some(model), self.effort.as_deref());
        }
        self.ask();
    }

    pub fn set_effort(&mut self, effort: &str) {
        self.effort = Some(effort.into());
        self.ask();
    }

    /// A preset's kind, model, effort, role and sandbox; the project, name and first message
    /// stay. A sandbox fence no longer allows for the kind is left out.
    pub fn apply(&mut self, preset: &Preset) {
        self.choose_tool(preset.kind);
        if preset.kind.has_models() {
            self.model = preset.model.clone();
            self.effort = preset.effort.clone();
            self.ask();
        }
        self.regular = preset.role == Role::Regular;
        if self.looking {
            // Not known yet: waited for, and taken once fence answers.
            self.sandbox = None;
            self.pending = self.sandbox_of(preset);
        } else {
            self.sandbox = self.sandbox_of(preset);
        }
    }

    /// How the form opens: as the first controller preset while the project has no `main` yet;
    /// otherwise a regular Claude agent at its deepest model and normal effort.
    pub fn open(&mut self, presets: &[Preset]) {
        self.pick_tool(Tool::Claude);
        self.regular = true;
        if self.names.contains(&format!("{}/main", self.prefix)) {
            return;
        }
        if let Some(preset) = presets
            .iter()
            .find(|preset| preset.role == Role::Controller)
        {
            self.apply(preset);
        }
    }

    /// Whether the form is set as `preset` sets it.
    pub fn matches(&self, preset: &Preset) -> bool {
        self.tool == preset.kind
            && self.regular == (preset.role == Role::Regular)
            && (!self.tool.has_models()
                || (self.model == preset.model && self.effort == preset.effort))
            && self.sandbox.as_ref().or(self.pending.as_ref()) == self.sandbox_of(preset).as_ref()
    }

    /// A preset clicked: its settings, and it is the one lit from now on.
    pub fn pick(&mut self, preset: &Preset) {
        self.apply(preset);
        self.picked = Some(preset.name.clone());
    }

    /// The picked preset's place in `presets`, while it is still there.
    fn picked_index(&self, presets: &[Preset]) -> Option<usize> {
        let name = self.picked.as_deref()?;
        presets.iter().position(|preset| preset.name == name)
    }

    /// The preset to light among `presets`: the one picked, edited when the form has been changed
    /// from it since; with none picked (or it is gone), the first the form is set as.
    pub fn lit(&self, presets: &[Preset]) -> Option<Lit> {
        if let Some(index) = self.picked_index(presets) {
            return Some(Lit {
                index,
                edited: !self.matches(&presets[index]),
            });
        }
        presets
            .iter()
            .position(|preset| self.matches(preset))
            .map(|index| Lit {
                index,
                edited: false,
            })
    }

    /// `presets` with the picked one set as the form now is; none when no preset was picked.
    pub fn update_picked(&self, presets: &[Preset]) -> Option<Vec<Preset>> {
        let index = self.picked_index(presets)?;
        let mut presets = presets.to_vec();
        let mut preset = self.preset(presets[index].name.clone());
        // Without fence, or before it answered, the form can't say: the preset's own stays.
        if self.offer.is_none() && preset.role == Role::Regular {
            preset.sandbox.clone_from(&presets[index].sandbox);
        }
        presets[index] = preset;
        Some(presets)
    }

    /// The form's kind, model, effort, role and sandbox as a preset called `name`.
    pub fn preset(&self, name: String) -> Preset {
        let models = self.tool.has_models();
        Preset {
            name,
            kind: self.tool,
            model: self.model.clone().filter(|_| models),
            effort: self.effort.clone().filter(|_| models),
            role: if self.regular {
                Role::Regular
            } else {
                Role::Controller
            },
            sandbox: self.sandbox_now().cloned(),
        }
    }

    /// A command typed by hand. The choices follow what it asks for; what it does not say is left
    /// as it was, except that a different kind starts from that kind's own defaults.
    pub fn set_command(&mut self, command: String) {
        self.command = command;
        let Ok(words) = shell_words::split(&self.command) else {
            return;
        };
        let reading = read(&words);
        if let Some(tool) = reading.tool
            && tool != self.tool
        {
            self.tool = tool;
            self.model = None;
            self.effort = None;
            self.pending = None;
            self.follow_name();
        }
        // Another kind, or a program of no kind paddock knows: the sandbox may go.
        self.refit_sandbox();
        if let Some(model) = reading.model.filter(|model| !model.is_empty()) {
            self.model = Some(model.into());
        }
        if let Some(effort) = reading.effort.filter(|effort| !effort.is_empty()) {
            self.effort = Some(effort.into());
        }
    }

    /// Writes the chosen model and effort into the command in place of what it asked for, the
    /// rest of the command as it was; a command that cannot be read starts again from the kind's.
    fn ask(&mut self) {
        let mut words = shell_words::split(&self.command)
            .ok()
            .filter(|words| !words.is_empty())
            .unwrap_or_else(|| {
                shell_words::split(self.tool.command()).expect("the kinds' commands are plain")
            });
        let asking = read(&words).asking;
        let mut index = 0;
        words.retain(|_| {
            index += 1;
            !asking.contains(&(index - 1))
        });
        let at = words
            .iter()
            .skip(1)
            .position(|word| word == "--")
            .map_or(words.len(), |at| at + 1);
        let mut asks = Vec::new();
        match self.tool {
            Tool::Claude => {
                if let Some(model) = &self.model {
                    asks.extend(["--model".into(), model.clone()]);
                }
                if let Some(effort) = &self.effort {
                    asks.extend(["--effort".into(), effort.clone()]);
                }
            }
            Tool::Codex => {
                if let Some(model) = &self.model {
                    asks.extend(["-m".into(), model.clone()]);
                }
                if let Some(effort) = &self.effort {
                    asks.extend(["-c".into(), format!("model_reasoning_effort=\"{effort}\"")]);
                }
            }
            Tool::Pi | Tool::Omp | Tool::Amp | Tool::Agy | Tool::Kimi => {}
        }
        words.splice(at..at, asks);
        self.command = shell_words::join(&words);
    }

    /// The directory it starts in, with `~` expanded.
    pub fn cwd(&self) -> String {
        expand_home(self.project.trim())
    }

    /// The name corral is asked for.
    pub fn full_name(&self) -> String {
        let name = if self.regular {
            self.name.as_str()
        } else {
            "main"
        };
        format!("{}/{name}", self.prefix)
    }

    /// The first problem, by field, as Saddle words it.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        let fields = [
            ("project", &self.project),
            ("prefix", &self.prefix),
            ("name", &self.name),
            ("command", &self.command),
            ("prompt", &self.prompt),
        ];
        if let Some((field, _)) = fields.iter().find(|(_, value)| value.contains('\0')) {
            return Some((field, "NUL bytes are not allowed".into()));
        }
        if self.project.trim().is_empty() {
            return Some(("project", "Directory is required".into()));
        }
        let bad = |value: &str| {
            value.trim().is_empty()
                || value.starts_with('-')
                || value.chars().any(char::is_whitespace)
        };
        if bad(&self.prefix) || self.prefix.contains('/') {
            return Some((
                "prefix",
                "Prefix needs text, no spaces, '/' or leading '-'".into(),
            ));
        }
        if self.regular && bad(&self.name) {
            return Some(("name", "Name needs text, no spaces or leading '-'".into()));
        }
        match shell_words::split(&self.command) {
            Err(_) => Some(("command", "Command has an unclosed quote".into())),
            Ok(words) if words.first().is_none_or(|w| w.is_empty()) => {
                Some(("command", "Command is required".into()))
            }
            Ok(_) => None,
        }
    }

    /// The arguments to `corral`.
    pub fn args(&self) -> Result<Vec<String>> {
        if let Some((_, problem)) = self.problem() {
            bail!("{problem}");
        }
        let words = shell_words::split(&self.command).context("Command has an unclosed quote")?;
        let role = if self.regular {
            "role=regular"
        } else {
            "role=controller"
        };
        let mut args = vec![
            "start".into(),
            self.full_name(),
            "--cwd".into(),
            expand_home(self.project.trim()),
            "--label".into(),
            role.into(),
        ];
        let reading = read(&words);
        for (key, value) in [("model", reading.model), ("effort", reading.effort)] {
            if let Some(value) = value.filter(|value| !value.is_empty()) {
                args.extend(["--label".into(), format!("{key}={value}")]);
            }
        }
        if let Some(profile) = self.sandbox_now() {
            args.extend(["--sandbox".into(), profile.clone()]);
        }
        if !self.prompt.is_empty() && self.tool.takes_prompt() {
            args.extend(["--prompt".into(), self.prompt.clone()]);
        }
        args.push("--".into());
        args.extend(words);
        Ok(args)
    }

    /// Whether Start can go: not for a kind that needs a first message without one (agy).
    pub fn can_start(&self) -> bool {
        !self.tool.needs_prompt() || !self.prompt.trim().is_empty()
    }

    /// The first message for a kind that cannot take it at start (amp): sent by [`send_first`]
    /// once the agent is idle.
    pub fn first_message(&self) -> Option<String> {
        (!self.tool.takes_prompt() && !self.prompt.is_empty()).then(|| self.prompt.clone())
    }

    /// The call as it will run, quoted for reading.
    pub fn preview(&self, corral: &str) -> String {
        match self.args() {
            Ok(args) => std::iter::once(corral.to_owned())
                .chain(args)
                .map(|word| shell_words::quote(&word).into_owned())
                .collect::<Vec<_>>()
                .join(" "),
            Err(error) => format!("{error}"),
        }
    }
}

/// What `corral start` reported: the agent's name and instance.
#[derive(Debug, PartialEq)]
pub struct Started {
    pub name: String,
    pub instance: Option<String>,
}

/// Runs `corral start …`; blocks, so call it off the UI thread.
pub fn start(corral: &str, args: &[String]) -> Result<Started> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = crate::command::run(corral, &args, None, CORRAL_TIMEOUT, &AtomicBool::new(false))?;
    if let Some(words) = sandbox_refusal(&output.stdout) {
        bail!("{words}");
    }
    let value = crate::corral::answer(&args, &output)?;
    let name = value["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .context("corral start returned no name; whether it started is unknown")?;
    Ok(Started {
        name: name.to_owned(),
        instance: value["instance"].as_str().map(str::to_owned),
    })
}

/// corral's words when it would not start an agent in its sandbox (`sandbox_unavailable`,
/// `sandbox_failed`, DESIGN §13 P5-120): its code and message, and for a failed check fence's own
/// code and message after them, as they were said.
fn sandbox_refusal(stdout: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(stdout).ok()?;
    let text = |value: &serde_json::Value, key| value.get(key)?.as_str().map(str::to_owned);
    let code = text(&value, "error").filter(|code| code.starts_with("sandbox_"))?;
    let mut words = match text(&value, "message") {
        Some(message) => format!("{code}: {message}"),
        None => code,
    };
    if let Some(fence) = value.get("fence") {
        let said: Vec<String> = [text(fence, "error"), text(fence, "message")]
            .into_iter()
            .flatten()
            .collect();
        if !said.is_empty() {
            words = format!("{words} ({})", said.join(": "));
        }
    }
    Some(words)
}

/// What New Agent warns of when `tool`'s plugin stands so that the agent would stay Starting: not
/// installed, out of date, or turned off in agy. For a block of hooks (kimi), without which corral
/// still sees the agent ready on screen but not its turns or replies (ranch R48c), that is said
/// instead. Starting is not held back.
pub fn plugin_warning(tool: Tool, install: &crate::integrations::Install) -> Option<String> {
    let kind = tool.kind();
    if install.block() {
        return match install.status.as_str() {
            "same" => None,
            "overwrite" => Some(format!(
                "corral's hooks for {kind} have changed; update them in Settings."
            )),
            _ => Some(format!(
                "corral's hooks for {kind} aren't installed; corral won't see {kind} working or \
                 replying."
            )),
        };
    }
    match install.status.as_str() {
        "same" if install.enabled == Some(false) => Some(format!(
            "The corral plugin is disabled in {kind}; run {kind} plugin enable corral."
        )),
        "same" => None,
        "overwrite" => Some(format!(
            "The corral plugin for {kind} has changed; update it in Settings."
        )),
        _ => Some(format!(
            "The corral plugin for {kind} isn't installed; {kind} will stay Starting."
        )),
    }
}

/// How long the first message waits for the agent to be ready.
const FIRST_WAIT: &str = "60";

/// Sends `text` to `name` once it is idle, as corral says to for a kind that takes no `--prompt`:
/// `corral wait`, then `corral send`. Blocks for up to a minute, so call it off the UI thread.
pub fn send_first(corral: &str, name: &str, text: &str) -> Result<()> {
    let cancel = AtomicBool::new(false);
    let output = crate::command::run(
        corral,
        &["wait", name, "--timeout", FIRST_WAIT],
        None,
        Duration::from_secs(75),
        &cancel,
    )?;
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("corral did not say whether it is ready")?;
    let state = value["state"].as_str().unwrap_or("unknown");
    match (value["result"].as_str(), state) {
        (Some("idle"), _) => {}
        (_, "starting") => {
            bail!("amp didn't get ready; is its plugin installed? See Settings › Agents")
        }
        (Some("blocked"), _) => bail!("it is waiting for you"),
        _ => bail!("it is {state}"),
    }
    let client = Client::local(corral);
    client
        .send(name, text, &cancel)
        .map_err(|refused| anyhow::anyhow!(refused.message))
}

/// Runs `corral stop NAME`; blocks, so call it off the UI thread.
pub fn stop(corral: &str, name: &str) -> Result<()> {
    stop_with(&Client::local(corral), name)
}

/// [`stop`] with any corral: this Mac's or another machine's (DESIGN §13 P5-90).
pub fn stop_with(client: &Client, name: &str) -> Result<()> {
    client.json(&["stop", name], CORRAL_TIMEOUT, &AtomicBool::new(false))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_codex_controller_named_after_the_project() {
        let form = Form::new("/tmp/demo".into(), Place::Current);
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/main",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=controller",
                "--",
                "codex",
                "--yolo"
            ]
        );
    }

    #[test]
    fn claude_a_regular_name_and_a_first_message() {
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.choose_tool(Tool::Claude);
        form.regular = true;
        form.name = "review".into();
        form.prompt = "先读 AGENTS.md".into();
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/review",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--prompt",
                "先读 AGENTS.md",
                "--",
                "claude"
            ]
        );
        // A controller is always main, whatever the regular name says.
        form.regular = false;
        assert_eq!(form.full_name(), "demo/main");
    }

    #[test]
    fn commands_supply_model_and_effort_labels() {
        for (tool, command, model, effort, words) in [
            (
                Tool::Codex,
                "/opt/tools/claude --model 'opus[1m]' --effort high",
                "model=opus[1m]",
                "effort=high",
                vec![
                    "/opt/tools/claude",
                    "--model",
                    "opus[1m]",
                    "--effort",
                    "high",
                ],
            ),
            (
                Tool::Claude,
                r#"/opt/tools/codex --yolo -m gpt-6-astra -c 'model_reasoning_effort="xhigh"'"#,
                "model=gpt-6-astra",
                "effort=xhigh",
                vec![
                    "/opt/tools/codex",
                    "--yolo",
                    "-m",
                    "gpt-6-astra",
                    "-c",
                    r#"model_reasoning_effort="xhigh""#,
                ],
            ),
        ] {
            let mut form = Form::new("/tmp/demo".into(), Place::Current);
            form.choose_tool(tool);
            form.command = command.into();
            form.prompt = "hello".into();
            let mut expected = vec![
                "start",
                "demo/main",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=controller",
                "--label",
                model,
                "--label",
                effort,
                "--prompt",
                "hello",
                "--",
            ];
            expected.extend(words);
            assert_eq!(form.args().unwrap(), expected, "{command}");
        }
    }

    #[test]
    fn missing_values_keep_the_last_valid_command_labels() {
        for (command, label) in [
            ("claude --model opus --model", "model=opus"),
            ("claude --effort high --effort", "effort=high"),
            ("codex -m gpt-6-astra -m", "model=gpt-6-astra"),
            ("codex --model gpt-6-astra --model", "model=gpt-6-astra"),
            ("claude --model opus --model=", "model=opus"),
            ("claude --effort high --effort ''", "effort=high"),
        ] {
            let mut form = Form::new("/tmp/demo".into(), Place::Current);
            form.command = command.into();
            let args = form.args().unwrap();
            let separator = args.iter().position(|arg| arg == "--").unwrap();
            assert!(
                args[..separator].iter().any(|arg| arg == label),
                "{command}: {args:?}"
            );
            assert_eq!(args[separator + 1..], shell_words::split(command).unwrap());
        }
    }

    #[test]
    fn attached_command_options_supply_labels() {
        for (command, labels) in [
            (
                r#"codex --config=model_reasoning_effort=\"high\""#,
                vec!["effort=high"],
            ),
            (
                r#"codex --config='model_reasoning_effort="xhigh"'"#,
                vec!["effort=xhigh"],
            ),
            ("codex -mgpt-6-astra", vec!["model=gpt-6-astra"]),
            ("codex -m=gpt-6-astra", vec!["model=gpt-6-astra"]),
            (
                "codex --model=gpt-6-astra --config=model_reasoning_effort=high",
                vec!["model=gpt-6-astra", "effort=high"],
            ),
            (
                "codex -c model_reasoning_effort=low --config=model_reasoning_effort=high --config=other=value",
                vec!["effort=high"],
            ),
            (
                "claude --model=opus --effort=high",
                vec!["model=opus", "effort=high"],
            ),
            ("claude --config=model_reasoning_effort=high", vec![]),
            (
                "codex -- --config=model_reasoning_effort=high -mgpt-6-astra",
                vec![],
            ),
        ] {
            let mut form = Form::new("/tmp/demo".into(), Place::Current);
            form.command = command.into();
            let args = form.args().unwrap();
            let separator = args.iter().position(|arg| arg == "--").unwrap();
            let actual: Vec<_> = args[..separator]
                .windows(2)
                .filter(|pair| pair[0] == "--label" && !pair[1].starts_with("role="))
                .map(|pair| pair[1].as_str())
                .collect();
            assert_eq!(actual, labels, "{command}");
            assert_eq!(args[separator + 1..], shell_words::split(command).unwrap());
        }
    }

    #[test]
    fn the_prefix_follows_the_project_until_typed() {
        let mut form = Form::new("/Users/me/My Project/".into(), Place::Current);
        assert_eq!(form.prefix, "My-Project");
        form.set_project("/tmp/paddock".into());
        assert_eq!(form.prefix, "paddock");
        form.type_prefix("work".into());
        form.set_project("/tmp/other".into());
        assert_eq!(form.prefix, "work");
        assert_eq!(suggest_prefix("/"), "agents");
        assert_eq!(suggest_prefix(".hidden"), "hidden");
    }

    #[test]
    fn problems_are_named_as_in_saddle() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.command = "codex 'unclosed".into();
        assert_eq!(form.problem().unwrap().0, "command");
        assert!(
            form.args()
                .unwrap_err()
                .to_string()
                .contains("unclosed quote")
        );
        form.choose_tool(Tool::Codex);
        form.prefix = "a/b".into();
        assert_eq!(form.problem().unwrap().0, "prefix");
        form.prefix = "demo".into();
        form.regular = true;
        form.name = "-x".into();
        assert_eq!(form.problem().unwrap().0, "name");
        form.name = "two words".into();
        assert_eq!(form.problem().unwrap().0, "name");
        form.name = "ok".into();
        form.project = "  ".into();
        assert_eq!(form.problem().unwrap().0, "project");
        form.project = "/tmp".into();
        form.command = String::new();
        assert_eq!(form.problem().unwrap().1, "Command is required");
    }

    #[test]
    fn the_preview_quotes_the_call() {
        let mut form = Form::new("/tmp/my demo".into(), Place::Current);
        form.prompt = "hello there".into();
        assert_eq!(
            form.preview("corral"),
            "corral start my-demo/main --cwd '/tmp/my demo' --label 'role=controller' \
             --prompt 'hello there' -- codex --yolo"
        );
    }

    #[test]
    fn home_is_expanded_in_the_directory() {
        let home = std::env::var("HOME").unwrap();
        let form = Form::new("~/code".into(), Place::Current);
        assert_eq!(form.args().unwrap()[3], format!("{home}/code"));
    }

    /// The arguments before `--` that are labels other than the role.
    fn labels(args: &[String]) -> Vec<&str> {
        let separator = args.iter().position(|arg| arg == "--").unwrap();
        args[..separator]
            .windows(2)
            .filter(|pair| pair[0] == "--label" && !pair[1].starts_with("role="))
            .map(|pair| pair[1].as_str())
            .collect()
    }

    fn command(args: &[String]) -> &[String] {
        let separator = args.iter().position(|arg| arg == "--").unwrap();
        &args[separator + 1..]
    }

    #[test]
    fn a_preset_fills_the_kind_model_effort_and_role_only() {
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.prompt = "读 HANDOFF.md".into();
        form.type_prefix("work".into());
        let presets = default_presets();
        assert_eq!(
            presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Controller", "Developer", "Codex builder", "Quick look"]
        );
        form.apply(&presets[0]);
        assert_eq!(
            (form.tool, form.model.as_deref(), form.effort.as_deref()),
            (Tool::Claude, Some("opus[1m]"), Some("high"))
        );
        assert!(!form.regular);
        assert_eq!(form.command, "claude --model 'opus[1m]' --effort high");
        assert!(form.matches(&presets[0]));
        assert!(!form.matches(&presets[1]));
        form.apply(&presets[2]);
        assert_eq!(
            (form.tool, form.model.as_deref(), form.effort.as_deref()),
            (Tool::Codex, Some("gpt-6-astra"), Some("high"))
        );
        assert!(form.regular);
        assert!(form.matches(&presets[2]));
        // Project, prefix, place and first message are left alone.
        assert_eq!(form.project, "/tmp/demo");
        assert_eq!(form.prefix, "work");
        assert_eq!(form.place, Place::Tab);
        assert_eq!(form.prompt, "读 HANDOFF.md");
        // The form's settings make a preset that matches it.
        let saved = form.preset("Mine".into());
        assert_eq!(saved.name, "Mine");
        assert!(form.matches(&saved));
    }

    #[test]
    fn it_opens_as_the_controller_preset_unless_the_project_has_a_main() {
        let presets = default_presets();
        let lit = |index| {
            Some(Lit {
                index,
                edited: false,
            })
        };
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.set_names(vec!["demo/claude-1".into(), "other/main".into()]);
        form.open(&presets);
        assert!(!form.regular);
        assert_eq!(form.lit(&presets), lit(0));

        // demo/main is taken: a regular Claude agent, as before, which Developer lights.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.set_names(vec!["demo/main".into()]);
        form.open(&presets);
        assert!(form.regular);
        assert_eq!(form.name, "claude-1");
        assert_eq!(form.lit(&presets), lit(1));

        // No controller preset: the same.
        let regular: Vec<Preset> = presets[1..].to_vec();
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.open(&regular);
        assert!(form.regular);
        assert_eq!(form.lit(&regular), lit(0));
    }

    #[test]
    fn a_picked_preset_stays_lit_when_changed_and_can_be_updated() {
        let presets = default_presets();
        let lit = |index, edited| Some(Lit { index, edited });
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        // Nothing picked: the preset the form is set as lights, as before.
        form.pick_tool(Tool::Claude);
        form.regular = true;
        assert_eq!(form.lit(&presets), lit(1, false));
        form.set_effort("low");
        assert_eq!(form.lit(&presets), None);

        // Developer, then another effort: still Developer, edited; back again, not edited.
        form.pick(&presets[1]);
        form.set_effort("xhigh");
        assert_eq!(form.lit(&presets), lit(1, true));
        form.set_effort("high");
        assert_eq!(form.lit(&presets), lit(1, false));
        // The kind, model and role count too.
        form.set_model("sonnet");
        assert_eq!(form.lit(&presets), lit(1, true));
        form.set_model("opus[1m]");
        form.regular = false;
        assert_eq!(form.lit(&presets), lit(1, true));
        form.regular = true;
        form.pick_tool(Tool::Pi);
        assert_eq!(form.lit(&presets), lit(1, true));
        form.pick_tool(Tool::Claude);
        assert_eq!(form.lit(&presets), lit(1, false));
        // So does what a command typed by hand moves.
        form.set_command("claude --model sonnet --effort high".into());
        assert_eq!(form.lit(&presets), lit(1, true));

        // Update preset: the picked one is written as the form is, the rest left; not edited.
        let updated = form.update_picked(&presets).unwrap();
        assert_eq!(
            updated[1],
            Preset {
                name: "Developer".into(),
                kind: Tool::Claude,
                model: Some("sonnet".into()),
                effort: Some("high".into()),
                role: Role::Regular,
                sandbox: None,
            }
        );
        assert_eq!(
            (&updated[..1], &updated[2..]),
            (&presets[..1], &presets[2..])
        );
        assert_eq!(form.lit(&updated), lit(1, false));

        // Another preset takes over.
        form.pick(&updated[2]);
        assert_eq!(form.lit(&updated), lit(2, false));
        form.set_effort("low");
        assert_eq!(form.lit(&updated), lit(2, true));
        // A picked preset deleted: back to the one the form is set as, if any.
        let mut fewer = updated.clone();
        fewer.remove(2);
        assert_eq!(form.lit(&fewer), None);
        assert_eq!(form.update_picked(&fewer), None);
        form.set_effort("high");
        fewer.push(form.preset("Mine".into()));
        assert_eq!(form.lit(&fewer), lit(3, false));
    }

    #[test]
    fn presets_that_do_not_fit_go_behind_more_the_lit_one_kept() {
        let widths = [100.0, 80.0, 120.0, 90.0];
        // All fit: 390 and three gaps of 4.
        assert_eq!(fit_presets(&widths, None, 402.0, 4.0, 30.0), [0, 1, 2, 3]);
        // Not all: as many as fit beside a 30-wide +N.
        assert_eq!(fit_presets(&widths, None, 400.0, 4.0, 30.0), [0, 1, 2]);
        assert_eq!(fit_presets(&widths, Some(1), 400.0, 4.0, 30.0), [0, 1, 2]);
        // The lit one stays, in place of the last that fit.
        assert_eq!(fit_presets(&widths, Some(3), 400.0, 4.0, 30.0), [0, 1, 3]);
        assert_eq!(fit_presets(&widths, Some(2), 300.0, 4.0, 30.0), [0, 2]);
        // No room at all: still the lit one.
        assert_eq!(fit_presets(&widths, Some(2), 10.0, 4.0, 30.0), [2]);
        assert!(fit_presets(&widths, None, 10.0, 4.0, 30.0).is_empty());
        assert!(fit_presets(&[], Some(0), 10.0, 4.0, 30.0).is_empty());
    }

    #[test]
    fn chosen_models_and_efforts_go_into_the_command_and_labels() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick_tool(Tool::Claude);
        form.set_model("sonnet");
        form.set_effort("max");
        let args = form.args().unwrap();
        assert_eq!(
            command(&args),
            ["claude", "--model", "sonnet", "--effort", "max"]
        );
        assert_eq!(labels(&args), ["model=sonnet", "effort=max"]);

        form.pick_tool(Tool::Codex);
        assert_eq!(form.model.as_deref(), Some("gpt-6-astra"));
        assert_eq!(form.effort.as_deref(), Some("high"));
        form.set_model("gpt-5.6-luna");
        form.set_effort("xhigh");
        let args = form.args().unwrap();
        assert_eq!(
            command(&args),
            [
                "codex",
                "--yolo",
                "-m",
                "gpt-5.6-luna",
                "-c",
                r#"model_reasoning_effort="xhigh""#
            ]
        );
        assert_eq!(labels(&args), ["model=gpt-5.6-luna", "effort=xhigh"]);
        assert_eq!(
            form.command,
            r#"codex --yolo -m gpt-5.6-luna -c 'model_reasoning_effort="xhigh"'"#
        );

        // pi and omp keep to their own settings: no model, no effort, no labels.
        for tool in [Tool::Pi, Tool::Omp] {
            form.pick_tool(tool);
            assert_eq!(
                (form.model.as_deref(), form.effort.as_deref()),
                (None, None)
            );
            let args = form.args().unwrap();
            assert_eq!(command(&args), [tool.kind()]);
            assert!(labels(&args).is_empty(), "{args:?}");
        }
        assert!(form.efforts().is_empty() && form.model_names().is_empty());
        assert!(!Tool::Pi.has_models() && Tool::Codex.has_models());
    }

    #[test]
    fn the_lists_come_from_the_form_and_the_effort_follows_the_model() {
        use crate::models::{List, Lists, Model, Source};
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        let model = |name: &str, efforts: &[&str]| Model {
            name: name.into(),
            efforts: efforts.iter().map(|e| (*e).into()).collect(),
        };
        form.set_lists(Lists {
            codex: List {
                models: vec![
                    model("fast", &["low", "medium"]),
                    model("deep", &["low", "high", "ultra"]),
                ],
                source: Source::Codex { at: 1 },
            },
            ..Lists::default()
        });
        form.pick_tool(Tool::Codex);
        assert_eq!(form.model_names(), ["fast", "deep"]);
        assert_eq!(form.default_model(), Some("fast"));
        assert_eq!(form.source(), Some(&Source::Codex { at: 1 }));
        // The first model, at the last effort it has when it has no `high`.
        assert_eq!(
            (form.model.as_deref(), form.effort.as_deref()),
            (Some("fast"), Some("medium"))
        );
        assert_eq!(form.efforts(), ["low", "medium"]);
        form.set_model("deep");
        assert_eq!(form.effort.as_deref(), Some("high"));
        assert_eq!(form.efforts(), ["low", "high", "ultra"]);
        form.set_effort("ultra");
        form.set_model("fast");
        assert_eq!(form.effort.as_deref(), Some("medium"));
        form.set_effort("low");
        form.set_model("deep");
        assert_eq!(form.effort.as_deref(), Some("low"));
        assert!(form.command.contains("-m deep"), "{}", form.command);
        assert!(
            form.command.contains(r#"model_reasoning_effort="low""#),
            "{}",
            form.command
        );
        // Claude keeps paddock's own, opus[1m] first.
        form.pick_tool(Tool::Claude);
        assert_eq!(form.model.as_deref(), Some("opus[1m]"));
        assert_eq!(form.efforts().last().map(String::as_str), Some("max"));
    }

    #[test]
    fn a_choice_replaces_what_the_command_asked_for_and_keeps_the_rest() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick_tool(Tool::Claude);
        form.set_command("claude --verbose --model=opus --effort low -- --model kept".into());
        form.set_model("sonnet");
        assert_eq!(
            form.command,
            "claude --verbose --model sonnet --effort low -- --model kept"
        );
        // A command that cannot be read starts again from the kind's own.
        form.command = "claude 'unclosed".into();
        form.set_effort("low");
        assert_eq!(form.command, "claude --model sonnet --effort low");
    }

    #[test]
    fn a_command_edited_by_hand_moves_the_choices_it_names() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick_tool(Tool::Claude);
        form.set_effort("xhigh");
        // A new model: the model follows, the effort it does not name stays.
        form.set_command("claude --model opus".into());
        assert_eq!(
            (form.tool, form.model.as_deref(), form.effort.as_deref()),
            (Tool::Claude, Some("opus"), Some("xhigh"))
        );
        // Nothing it can read: everything stays.
        form.set_command("claude --model".into());
        assert_eq!(form.model.as_deref(), Some("opus"));
        form.set_command("claude 'unclosed".into());
        assert_eq!(
            (form.tool, form.model.as_deref()),
            (Tool::Claude, Some("opus"))
        );
        // Another kind: its own defaults, then what the command names.
        form.set_command(
            r#"/opt/bin/codex -m gpt-5.6-luna -c model_reasoning_effort="low""#.into(),
        );
        assert_eq!(
            (form.tool, form.model.as_deref(), form.effort.as_deref()),
            (Tool::Codex, Some("gpt-5.6-luna"), Some("low"))
        );
        form.set_command("omp --resume".into());
        assert_eq!((form.tool, form.model.as_deref()), (Tool::Omp, None));
        // A program of no known kind leaves the kind as it was.
        form.set_command("my-agent --fast".into());
        assert_eq!(form.tool, Tool::Omp);
        assert_eq!(form.command, "my-agent --fast");
    }

    #[test]
    fn a_regular_name_follows_the_kind_to_a_free_number_until_typed() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.regular = true;
        assert_eq!(form.full_name(), "demo/codex-1");
        form.set_names(vec!["demo/claude-1".into(), "other/claude-2".into()]);
        form.pick_tool(Tool::Claude);
        assert_eq!(form.full_name(), "demo/claude-2");
        form.set_project("/tmp/other".into());
        assert_eq!(form.full_name(), "other/claude-1");
        form.type_name("review".into());
        form.pick_tool(Tool::Pi);
        assert_eq!(form.full_name(), "other/review");
        form.regular = false;
        assert_eq!(form.full_name(), "other/main");
    }

    #[test]
    fn presets_are_saved_read_and_deleted_in_the_config_file() {
        let dir = std::env::temp_dir().join(format!("paddock-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");
        // No file, or a file without presets: the default four.
        assert_eq!(presets(&Config::load(&path).unwrap()), default_presets());
        let before = "# my settings\ntheme = \"tide\"\n\n[colors]\nfocus = \"#ff0000\"\n";
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, before).unwrap();
        assert_eq!(presets(&Config::load(&path).unwrap()), default_presets());

        let mut list = default_presets();
        list.push(Preset {
            name: "pi helper".into(),
            kind: Tool::Pi,
            model: None,
            effort: None,
            role: Role::Regular,
            sandbox: None,
        });
        let saved = save_presets(&path, &list).unwrap();
        assert_eq!(saved.agent_presets.as_ref(), Some(&list));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("# my settings\ntheme = \"tide\"\n"),
            "{text}"
        );
        assert!(text.contains("focus = \"#ff0000\""), "{text}");
        let config = Config::load(&path).unwrap();
        assert_eq!(presets(&config), list);
        assert_eq!(config.theme.as_deref(), Some("tide"));

        // Deleting one, then all: an empty list stays empty.
        list.remove(1);
        save_presets(&path, &list).unwrap();
        assert_eq!(presets(&Config::load(&path).unwrap()), list);
        save_presets(&path, &[]).unwrap();
        assert!(presets(&Config::load(&path).unwrap()).is_empty());

        // Bad presets in the file are named.
        std::fs::write(
            &path,
            "agent_presets = [{ name = \"x\", kind = \"gemini\" }]\n",
        )
        .unwrap();
        let error = Config::load(&path).unwrap_err();
        assert!(format!("{error:#}").contains("gemini"), "{error:#}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn offer(profiles: &[&str]) -> Offer {
        Offer {
            profiles: profiles.iter().map(|p| p.to_string()).collect(),
            kinds: ["claude", "codex", "omp", "pi"].map(String::from).to_vec(),
        }
    }

    #[test]
    fn a_sandbox_is_only_for_a_regular_agent_of_a_kind_fence_knows() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick_tool(Tool::Claude);
        form.set_regular(true);
        // fence not there (or not known yet): nothing to choose.
        assert!(!form.can_sandbox());
        form.set_sandbox(Some("dev".into()));
        assert_eq!(form.sandbox, None);
        form.set_offer(Some(offer(&["dev"])));
        assert!(form.can_sandbox());
        form.set_sandbox(Some("dev".into()));
        assert_eq!(form.sandbox.as_deref(), Some("dev"));
        // `--sandbox` goes before `--`, the command after it as it was.
        let args = form.args().unwrap();
        let at = args.iter().position(|arg| arg == "--sandbox").unwrap();
        assert_eq!(args[at + 1], "dev");
        assert!(at < args.iter().position(|arg| arg == "--").unwrap());
        assert_eq!(command(&args)[0], "claude");
        assert!(form.preview("corral").contains(" --sandbox dev "));
        // Another kind fence knows keeps it; one it doesn't drops it, and can't have one.
        form.pick_tool(Tool::Codex);
        assert_eq!(form.sandbox.as_deref(), Some("dev"));
        form.pick_tool(Tool::Amp);
        assert_eq!(form.sandbox, None);
        assert!(!form.can_sandbox());
        form.pick_tool(Tool::Claude);
        assert_eq!(form.sandbox, None);
        // A command typed by hand that names another kind counts the same.
        form.set_sandbox(Some("dev".into()));
        form.set_command("agy".into());
        assert_eq!(form.sandbox, None);
        // A controller has none.
        form.pick_tool(Tool::Claude);
        form.set_sandbox(Some("dev".into()));
        form.set_regular(false);
        assert_eq!(form.sandbox, None);
        assert!(!form.can_sandbox());
        assert!(!form.args().unwrap().contains(&"--sandbox".to_owned()));
        // A profile fence doesn't have, or fence gone.
        form.set_regular(true);
        form.set_sandbox(Some("gone".into()));
        assert_eq!(form.sandbox, None);
        form.set_sandbox(Some("dev".into()));
        form.set_offer(None);
        assert_eq!(form.sandbox, None);
    }

    #[test]
    fn a_command_paddock_cant_read_as_a_kind_has_no_sandbox() {
        let sandboxed = |form: &Form| form.args().unwrap().contains(&"--sandbox".to_owned());
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick_tool(Tool::Claude);
        form.set_regular(true);
        form.set_offer(Some(offer(&["dev"])));
        form.set_sandbox(Some("dev".into()));
        assert!(sandboxed(&form));
        // Another program typed by hand: no sandbox, none to choose, and none in the call.
        form.set_command("my-agent --flag".into());
        assert_eq!(form.sandbox, None);
        assert!(!form.can_sandbox());
        form.set_sandbox(Some("dev".into()));
        assert_eq!(form.sandbox, None);
        let args = form.args().unwrap();
        assert!(!sandboxed(&form));
        assert_eq!(command(&args), ["my-agent", "--flag"]);
        // Even set straight on the form, the call leaves it out.
        form.sandbox = Some("dev".into());
        assert!(!sandboxed(&form));
        form.sandbox = None;
        // Back to Claude: one can be chosen again, but the old one doesn't come back by itself.
        form.set_command("claude".into());
        assert!(form.can_sandbox());
        assert_eq!(form.sandbox, None);
        assert!(!sandboxed(&form));
        // A controller's call never has one, however the form got it.
        form.set_regular(false);
        form.sandbox = Some("dev".into());
        assert!(!sandboxed(&form));
    }

    #[test]
    fn a_preset_picked_while_fence_is_looked_for_gets_its_sandbox_once_found() {
        let mut presets = default_presets();
        presets[1].sandbox = Some("dev".into());
        let lit = |index, edited| Some(Lit { index, edited });
        let sandboxed = |form: &Form| form.args().unwrap().contains(&"--sandbox".to_owned());
        // Picked before fence answered: it waits for it, and Start waits too.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick(&presets[1]);
        assert_eq!(form.sandbox, None);
        assert!(form.sandbox_pending());
        assert!(!sandboxed(&form));
        assert_eq!(form.lit(&presets), lit(1, false));
        form.set_offer(Some(offer(&["dev"])));
        assert_eq!(form.sandbox.as_deref(), Some("dev"));
        assert!(!form.sandbox_pending());
        assert!(sandboxed(&form));
        assert_eq!(form.lit(&presets), lit(1, false));
        let updated = form.update_picked(&presets).unwrap();
        assert_eq!(updated[1].sandbox.as_deref(), Some("dev"));

        // Changed meanwhile: a controller (and back), another kind, or no sandbox chosen.
        let changes: [fn(&mut Form); 3] = [
            |form| {
                form.set_regular(false);
                form.set_regular(true);
            },
            |form| form.pick_tool(Tool::Codex),
            |form| form.set_sandbox(None),
        ];
        for change in changes {
            let mut form = Form::new("/tmp/demo".into(), Place::Current);
            form.pick(&presets[1]);
            change(&mut form);
            assert!(!form.sandbox_pending());
            form.set_offer(Some(offer(&["dev"])));
            assert_eq!(form.sandbox, None);
        }

        // fence isn't there after all: none, and Update keeps the preset's own.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick(&presets[1]);
        form.set_offer(None);
        assert_eq!(form.sandbox, None);
        assert!(!form.sandbox_pending());
        assert_eq!(form.lit(&presets), lit(1, false));
        let updated = form.update_picked(&presets).unwrap();
        assert_eq!(updated[1].sandbox.as_deref(), Some("dev"));
        // Found, but without the profile: none.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick(&presets[1]);
        form.set_offer(Some(offer(&["other"])));
        assert_eq!(form.sandbox, None);
        assert!(!form.sandbox_pending());
    }

    #[test]
    fn a_preset_keeps_its_sandbox_unless_fence_no_longer_allows_it() {
        let mut presets = default_presets();
        presets[1].sandbox = Some("dev".into());
        // A controller preset's sandbox is never used.
        presets[0].sandbox = Some("dev".into());
        let lit = |index, edited| Some(Lit { index, edited });
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.set_offer(Some(offer(&["dev"])));
        form.pick(&presets[1]);
        assert_eq!(form.sandbox.as_deref(), Some("dev"));
        assert_eq!(form.lit(&presets), lit(1, false));
        // Changing the lock counts as changing the preset.
        form.set_sandbox(None);
        assert_eq!(form.lit(&presets), lit(1, true));
        let updated = form.update_picked(&presets).unwrap();
        assert_eq!(updated[1].sandbox, None);
        form.set_sandbox(Some("dev".into()));
        assert_eq!(form.lit(&presets), lit(1, false));
        assert_eq!(form.preset("Mine".into()).sandbox.as_deref(), Some("dev"));
        form.pick(&presets[0]);
        assert_eq!(form.sandbox, None);
        assert_eq!(form.lit(&presets), lit(0, false));
        assert_eq!(form.preset("Boss".into()).sandbox, None);

        // The profile is gone: none, and the preset still lit as it is.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.set_offer(Some(offer(&["other"])));
        form.pick(&presets[1]);
        assert_eq!(form.sandbox, None);
        assert_eq!(form.lit(&presets), lit(1, false));

        // fence not there: none, and Update preset leaves the preset's sandbox as it was.
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.pick(&presets[1]);
        assert_eq!(form.sandbox, None);
        form.set_effort("low");
        let updated = form.update_picked(&presets).unwrap();
        assert_eq!(updated[1].sandbox.as_deref(), Some("dev"));
        assert_eq!(updated[1].effort.as_deref(), Some("low"));
    }

    #[test]
    fn a_presets_sandbox_is_kept_in_the_config_file() {
        let dir = std::env::temp_dir().join(format!("paddock-sandbox-{}", std::process::id()));
        let path = dir.join("config.toml");
        std::fs::create_dir_all(&dir).unwrap();
        // An older config without the field reads as none.
        std::fs::write(
            &path,
            "agent_presets = [{ name = \"x\", kind = \"pi\", role = \"regular\" }]\n",
        )
        .unwrap();
        assert_eq!(presets(&Config::load(&path).unwrap())[0].sandbox, None);
        let mut list = default_presets();
        list[1].sandbox = Some("dev".into());
        list[0].sandbox = Some("dev".into());
        save_presets(&path, &list).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("sandbox = \"dev\"").count(), 1, "{text}");
        let read = presets(&Config::load(&path).unwrap());
        assert_eq!(read[1].sandbox.as_deref(), Some("dev"));
        assert_eq!(read[0].sandbox, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recent_projects_are_kept_latest_first() {
        let mut recent = vec!["/a".to_owned(), "/b".to_owned()];
        remember(&mut recent, "/b");
        assert_eq!(recent, ["/b", "/a"]);
        for n in 0..10 {
            remember(&mut recent, &format!("/p{n}"));
        }
        assert_eq!(recent.len(), RECENT);
        assert_eq!(recent[0], "/p9");
        let dir = std::env::temp_dir().join(format!("paddock-recent-{}", std::process::id()));
        let path = dir.join("recent-projects.json");
        assert!(load_recent(&path).is_empty());
        save_recent(&path, &recent).unwrap();
        assert_eq!(load_recent(&path), recent);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn amp_starts_bare_and_keeps_its_first_message_for_later() {
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.pick_tool(Tool::Amp);
        form.regular = true;
        assert_eq!(
            (form.command.as_str(), form.name.as_str()),
            ("amp", "amp-1")
        );
        assert_eq!(
            (form.model.as_deref(), form.effort.as_deref()),
            (None, None)
        );
        form.prompt = "读 HANDOFF.md".into();
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/amp-1",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--",
                "amp"
            ]
        );
        assert_eq!(form.first_message().as_deref(), Some("读 HANDOFF.md"));
        // A command naming amp by its path is amp too.
        form.pick_tool(Tool::Claude);
        assert_eq!(form.first_message(), None);
        form.set_command("/opt/homebrew/bin/amp".into());
        assert_eq!(form.tool, Tool::Amp);
        assert!(!form.args().unwrap().contains(&"--prompt".to_string()));
        form.prompt.clear();
        assert_eq!(form.first_message(), None);
    }

    #[test]
    fn agy_needs_its_first_message_and_starts_with_it() {
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.pick_tool(Tool::Agy);
        form.regular = true;
        assert_eq!(
            Tool::ALL[Tool::ALL.len() - 3..Tool::ALL.len() - 1],
            [Tool::Amp, Tool::Agy]
        );
        assert_eq!(
            (form.command.as_str(), form.name.as_str(), form.tool.label()),
            ("agy", "agy-1", "agy")
        );
        assert_eq!(
            (form.model.as_deref(), form.effort.as_deref()),
            (None, None)
        );
        assert!(form.tool.needs_prompt() && !Tool::Amp.needs_prompt());
        assert!(!form.can_start());
        form.prompt = "  ".into();
        assert!(!form.can_start());
        form.prompt = "读 HANDOFF.md".into();
        assert!(form.can_start());
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/agy-1",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--prompt",
                "读 HANDOFF.md",
                "--",
                "agy"
            ]
        );
        assert_eq!(form.first_message(), None);
        // Every other kind starts without one, as before.
        form.prompt.clear();
        for tool in [Tool::Claude, Tool::Codex, Tool::Pi, Tool::Omp, Tool::Amp] {
            form.pick_tool(tool);
            assert!(form.can_start(), "{tool:?}");
        }
        form.set_command("/Users/u/.local/bin/agy".into());
        assert_eq!(form.tool, Tool::Agy);
    }

    #[test]
    fn a_preset_can_be_agy() {
        let dir = std::env::temp_dir().join(format!(
            "paddock-agy-preset-{}",
            crate::control::random_id().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let list = vec![Preset {
            name: "agy".into(),
            kind: Tool::Agy,
            model: None,
            effort: None,
            role: Role::Regular,
            sandbox: None,
        }];
        save_presets(&path, &list).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("kind = \"agy\"")
        );
        assert_eq!(presets(&Config::load(&path).unwrap()), list);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_plugin_that_would_leave_the_agent_starting_is_warned_of() {
        use crate::integrations::Install;
        let install = |status: &str, enabled| Install {
            path: "/h/p".into(),
            status: status.into(),
            enabled,
            format: None,
        };
        let warning = |tool, status, enabled| plugin_warning(tool, &install(status, enabled));
        assert_eq!(warning(Tool::Agy, "same", Some(true)), None);
        assert_eq!(warning(Tool::Agy, "same", None), None);
        assert_eq!(
            warning(Tool::Agy, "same", Some(false)).as_deref(),
            Some("The corral plugin is disabled in agy; run agy plugin enable corral.")
        );
        assert_eq!(
            warning(Tool::Agy, "create", Some(true)).as_deref(),
            Some("The corral plugin for agy isn't installed; agy will stay Starting.")
        );
        assert_eq!(
            warning(Tool::Agy, "overwrite", Some(true)).as_deref(),
            Some("The corral plugin for agy has changed; update it in Settings.")
        );
        // amp's as before.
        assert_eq!(warning(Tool::Amp, "same", None), None);
        assert_eq!(
            warning(Tool::Amp, "foreign", None).as_deref(),
            Some("The corral plugin for amp isn't installed; amp will stay Starting.")
        );
        assert!(Tool::Amp.has_plugin() && Tool::Agy.has_plugin() && !Tool::Claude.has_plugin());
    }

    #[test]
    fn kimi_comes_after_agy_and_starts_like_claude_with_or_without_a_first_message() {
        assert_eq!(Tool::ALL[Tool::ALL.len() - 2..], [Tool::Agy, Tool::Kimi]);
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.pick_tool(Tool::Kimi);
        form.regular = true;
        assert_eq!(
            (form.command.as_str(), form.name.as_str(), form.tool.label()),
            ("kimi", "kimi-1", "kimi")
        );
        assert!(!form.tool.has_models());
        assert_eq!(
            (form.model.as_deref(), form.effort.as_deref()),
            (None, None)
        );
        // An empty first message starts it idle.
        assert!(form.can_start());
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/kimi-1",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--",
                "kimi"
            ]
        );
        form.prompt = "读 HANDOFF.md".into();
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/kimi-1",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--prompt",
                "读 HANDOFF.md",
                "--",
                "kimi"
            ]
        );
        assert_eq!(form.first_message(), None);
        form.pick_tool(Tool::Claude);
        form.set_command("/Users/u/.kimi-code/bin/kimi".into());
        assert_eq!(form.tool, Tool::Kimi);
    }

    #[test]
    fn a_preset_can_be_kimi() {
        let dir = std::env::temp_dir().join(format!(
            "paddock-kimi-preset-{}",
            crate::control::random_id().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let list = vec![Preset {
            name: "kimi".into(),
            kind: Tool::Kimi,
            model: None,
            effort: None,
            role: Role::Regular,
            sandbox: None,
        }];
        save_presets(&path, &list).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("kind = \"kimi\"")
        );
        assert_eq!(presets(&Config::load(&path).unwrap()), list);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kimi_s_hooks_not_in_place_are_warned_of_for_what_they_cost() {
        use crate::integrations::Install;
        let install = |status: &str| Install {
            path: "/h/.kimi-code/config.toml".into(),
            status: status.into(),
            enabled: None,
            format: Some("marked-block".into()),
        };
        let warning = |status| plugin_warning(Tool::Kimi, &install(status));
        assert!(Tool::Kimi.has_plugin());
        assert_eq!(warning("same"), None);
        for status in ["create", "foreign"] {
            assert_eq!(
                warning(status).as_deref(),
                Some(
                    "corral's hooks for kimi aren't installed; corral won't see kimi working \
                     or replying."
                ),
                "{status}"
            );
        }
        assert_eq!(
            warning("overwrite").as_deref(),
            Some("corral's hooks for kimi have changed; update them in Settings.")
        );
    }

    /// A corral whose `wait` answers `wait` (exiting `code`) and whose `send` answers `send`,
    /// logging each call.
    fn waiting_corral(wait: &str, code: u8, send: &str) -> (PathBuf, String, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "paddock-first-{}",
            crate::control::random_id().unwrap()
        ));
        std::fs::create_dir(&dir).unwrap();
        let log = dir.join("log");
        let corral = dir.join("corral");
        std::fs::write(
            &corral,
            format!(
                "#!/bin/sh\necho \"$*\" >> '{}'\ncase \"$1\" in\nwait) echo '{wait}'; exit {code};;\nsend) echo '{send}';;\nesac\n",
                log.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&corral, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, corral.display().to_string(), log)
    }

    #[test]
    fn a_sandbox_corral_refuses_says_why_in_its_and_fences_words() {
        let failed = r#"{"ok":false,"error":"sandbox_failed","message":"fence check failed","fence":{"ok":false,"error":"bad_cwd","message":"no directory: /x"}}"#;
        let (dir, corral, _) = waiting_corral("{}", 0, "{}");
        std::fs::write(&corral, format!("#!/bin/sh\necho '{failed}'\nexit 1\n")).unwrap();
        let args = ["start".to_owned(), "p/a".to_owned()];
        let error = start(&corral, &args).unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            "sandbox_failed: fence check failed (bad_cwd: no directory: /x)"
        );
        let unavailable =
            r#"{"ok":false,"error":"sandbox_unavailable","message":"fence not found"}"#;
        std::fs::write(
            &corral,
            format!("#!/bin/sh\necho '{unavailable}'\nexit 1\n"),
        )
        .unwrap();
        let error = start(&corral, &args).unwrap_err();
        assert_eq!(format!("{error:#}"), "sandbox_unavailable: fence not found");
        // Other refusals are said as before; a start that went well too.
        std::fs::write(
            &corral,
            "#!/bin/sh\necho '{\"ok\":false,\"error\":\"exists\"}'\nexit 1\n",
        )
        .unwrap();
        assert!(format!("{:#}", start(&corral, &args).unwrap_err()).ends_with(": exists"));
        let started = r#"{"ok":true,"name":"p/a","instance":"i1"}"#;
        std::fs::write(&corral, format!("#!/bin/sh\necho '{started}'\n")).unwrap();
        assert_eq!(
            start(&corral, &args).unwrap().instance.as_deref(),
            Some("i1")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_first_message_goes_once_the_agent_is_idle() {
        let (dir, corral, log) = waiting_corral(
            r#"{"ok":true,"state":"idle","result":"idle"}"#,
            0,
            r#"{"ok":true,"confirmed":true}"#,
        );
        send_first(&corral, "demo/amp-1", "读 HANDOFF.md").unwrap();
        assert_eq!(
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                "wait demo/amp-1 --timeout 60",
                "send demo/amp-1 读 HANDOFF.md"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_first_message_that_cannot_go_says_why_and_is_not_sent() {
        let cases = [
            (
                r#"{"ok":false,"error":"timeout","message":"demo/amp-1 still starting","state":"starting"}"#,
                4,
                "amp didn't get ready; is its plugin installed? See Settings › Agents",
            ),
            (
                r#"{"ok":true,"state":"blocked","result":"blocked"}"#,
                0,
                "it is waiting for you",
            ),
            (
                r#"{"ok":true,"state":"stopped","result":"unknown"}"#,
                0,
                "it is stopped",
            ),
        ];
        for (wait, code, why) in cases {
            let (dir, corral, log) = waiting_corral(wait, code, "{}");
            let error = send_first(&corral, "demo/amp-1", "hi").unwrap_err();
            assert_eq!(format!("{error:#}"), why);
            assert_eq!(std::fs::read_to_string(&log).unwrap().lines().count(), 1);
            std::fs::remove_dir_all(&dir).unwrap();
        }
        let (dir, corral, _) = waiting_corral(
            r#"{"ok":true,"state":"idle","result":"idle"}"#,
            0,
            r#"{"ok":false,"error":"human_active","message":"someone is typing"}"#,
        );
        let error = send_first(&corral, "demo/amp-1", "hi").unwrap_err();
        assert_eq!(format!("{error:#}"), "someone is typing");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
