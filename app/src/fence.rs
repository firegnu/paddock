//! The `fence` command (DESIGN §13 P5-120): the sandbox wall ranch keeps (R17b), read through its
//! public front commands only, `--version`, `profiles`, `check` and `denials`, as ranch's
//! `docs/CONTRACT.md` sets them out. Each prints one line of JSON: `ok:true` and exit 0, or
//! `ok:false` with `error` and `message` and exit 1. Unknown fields are left alone and unknown
//! values shown as they are. Nothing here writes fence's files or reads them directly.
//!
//! fence is looked for on PATH (paddock takes the login shell's at start). When it is not there,
//! or its `contract` is not `"1"`, paddock offers no sandbox anywhere. Every call blocks, so call
//! them off the UI thread.
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
};

/// The command, found on PATH.
pub const PROGRAM: &str = "fence";
/// The contract paddock reads.
pub const CONTRACT: &str = "1";
/// How long one call may take: `check` takes about 50 ms, `denials` well under that.
const TIMEOUT: Duration = Duration::from_secs(10);
/// How many records a count asks for: fence gives no count by itself (ranch's answer 3).
pub const LIMIT: usize = 5000;

/// `fence --version`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Version {
    pub version: String,
    pub contract: String,
    pub srt: Option<String>,
    pub node: Option<String>,
}

/// One profile from `fence profiles`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    /// `builtin` or `user`; another value is shown as it is.
    pub source: String,
    pub path: Option<String>,
    pub overrides_builtin: bool,
    /// Present when fence could not read it; the profile is still listed.
    pub error: Option<Value>,
}

impl Profile {
    /// Where it comes from, in words: `built in`, `yours`, or fence's word.
    pub fn origin(&self) -> String {
        origin(&self.source)
    }

    /// Why fence could not read it, as it said.
    pub fn problem(&self) -> Option<String> {
        self.error.as_ref().map(said)
    }
}

/// One web search server `fence profiles` lists (ranch R30): built-in data or this Mac's.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Server {
    pub name: String,
    /// `builtin` or `user`; another value is shown as it is.
    pub source: String,
    /// Present when fence could not read it; the server is still listed.
    pub error: Option<Value>,
}

impl Server {
    /// Where it comes from, in words: `built in`, `yours`, or fence's word.
    pub fn origin(&self) -> String {
        origin(&self.source)
    }

    /// Why fence could not read it, as it said.
    pub fn problem(&self) -> Option<String> {
        self.error.as_ref().map(said)
    }
}

/// `builtin` or `user` in words; another value as it is.
pub fn origin(source: &str) -> String {
    match source {
        "builtin" => "built in".into(),
        "user" => "yours".into(),
        other => other.into(),
    }
}

/// A kind of agent fence has data for.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Kind {
    pub kind: String,
    pub source: String,
    pub services: Vec<String>,
}

/// This Mac's own additions to the profiles.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Local {
    pub path: String,
    pub exists: bool,
}

/// `fence profiles`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Profiles {
    pub profiles: Vec<Profile>,
    pub kinds: Vec<Kind>,
    pub local: Option<Local>,
    /// The web search servers fence can give an agent; none when fence is too old to list them
    /// (`mcp` is a newer field, ranch R30).
    pub mcp: Option<Vec<Server>>,
}

impl Profiles {
    /// Whether fence has data for agents of `kind` (`claude`, `codex`…).
    pub fn supports(&self, kind: &str) -> bool {
        self.kinds.iter().any(|known| known.kind == kind)
    }

    /// Whether there is a profile called `name`.
    pub fn has(&self, name: &str) -> bool {
        self.profiles.iter().any(|profile| profile.name == name)
    }
}

/// One layer `fence check` merged, in order.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layer {
    /// `profile`, `agent`, `service`, `repo`, or another fence names.
    pub source: String,
    pub name: Option<String>,
    pub origin: Option<String>,
    pub path: Option<String>,
}

impl Layer {
    /// As a small tag: `profile dev`, `agent claude`, `repo .fence.toml`.
    pub fn tag(&self) -> String {
        let what = self
            .name
            .clone()
            .or_else(|| {
                self.path
                    .as_deref()
                    .map(|path| path.rsplit('/').next().unwrap_or(path).to_owned())
            })
            .unwrap_or_default();
        if what.is_empty() {
            self.source.clone()
        } else {
            format!("{} {what}", self.source)
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Network {
    pub allowed_domains: Vec<String>,
    pub denied_domains: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Filesystem {
    pub deny_read: Vec<String>,
    pub allow_read: Vec<String>,
    pub allow_write: Vec<String>,
    pub deny_write: Vec<String>,
}

/// The rules `fence check` merged.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub network: Network,
    pub filesystem: Filesystem,
}

/// A rule fence left out, and why.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Skipped {
    pub entry: String,
    pub reason: String,
}

/// `fence check`: what a profile comes to for one kind in one directory.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Check {
    pub profile: String,
    pub kind: Option<String>,
    pub cwd: Option<String>,
    pub layers: Vec<Layer>,
    pub settings: Settings,
    pub skipped: Vec<Skipped>,
    pub warnings: Vec<String>,
}

/// One web search server a check gives the agent.
#[derive(Clone, Debug, PartialEq)]
pub struct Search {
    pub name: String,
    /// Where it comes from in words; none when its layer doesn't say.
    pub origin: Option<String>,
}

impl Search {
    /// As words: `exa (built in)`, or the name alone when the layer gave no origin.
    pub fn words(&self) -> String {
        match &self.origin {
            Some(origin) => format!("{} ({origin})", self.name),
            None => self.name.clone(),
        }
    }
}

impl Check {
    /// The web search servers this check gives the agent, in the order fence merged them; empty
    /// when the profile and kind connect none.
    pub fn search(&self) -> Vec<Search> {
        self.layers
            .iter()
            .filter(|layer| layer.source == "mcp")
            .map(|layer| Search {
                name: layer.name.clone().unwrap_or_else(|| layer.tag()),
                origin: layer.origin.as_deref().map(origin),
            })
            .collect()
    }
}

/// One thing the wall stopped.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Denial {
    /// When, in seconds since the epoch.
    pub t: f64,
    /// `network`, `file-read`, `file-write`, `other`, or another fence names.
    #[serde(rename = "type")]
    pub kind: String,
    /// What it was after; none when fence did not know.
    pub target: Option<String>,
    pub detail: Option<String>,
}

impl Denial {
    /// Counted on the card: a network or file-write block. Reads and the rest are mostly noise
    /// (ranch's answer 3).
    pub fn counted(&self) -> bool {
        matches!(self.kind.as_str(), "network" | "file-write")
    }

    /// Its type as a tag: Network, Write, Read, Other; another type as fence gave it.
    pub fn tag(&self) -> String {
        match self.kind.as_str() {
            "network" => "Network".into(),
            "file-write" => "Write".into(),
            "file-read" => "Read".into(),
            "other" => "Other".into(),
            other => other.into(),
        }
    }
}

/// `fence denials`: what the wall stopped in one run, oldest first.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Denials {
    pub profile: Option<String>,
    pub running: bool,
    pub denials: Vec<Denial>,
    /// The log was full and fence stopped writing it.
    pub truncated: bool,
}

impl Denials {
    /// How many count on the card (network and file-write).
    pub fn blocked(&self) -> usize {
        self.denials.iter().filter(|d| d.counted()).count()
    }

    /// As many as [`LIMIT`] came back: there may be more than counted.
    pub fn capped(&self) -> bool {
        self.denials.len() >= LIMIT
    }
}

/// Why a call did not answer: fence's `error` and `message`; an empty code when fence could not
/// be run or its answer could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.code.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

/// Stops the fence calls it was handed to once dropped: whoever asked no longer wants the answer,
/// so a slow fence is killed and reaped rather than left to run out its time.
#[derive(Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    /// The flag to hand the calls.
    pub fn flag(&self) -> Arc<AtomicBool> {
        self.0.clone()
    }
}

impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// A value fence said, in words: a string as it is, an object by its `message` (after its code
/// when it has one), anything else as JSON.
fn said(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_owned();
    }
    let text = |key| value.get(key).and_then(Value::as_str);
    match (text("error").or(text("code")), text("message")) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (None, Some(message)) => message.to_owned(),
        _ => value.to_string(),
    }
}

/// fence's answer to `args` from `output`: its JSON when it said `ok:true` and exited 0;
/// otherwise its `error` and `message`.
fn answer(output: &crate::command::Output) -> Result<Value, Refusal> {
    let unreadable = || Refusal {
        code: String::new(),
        message: format!(
            "fence's answer could not be read{}",
            match String::from_utf8_lossy(&output.stderr).trim() {
                "" => String::new(),
                error => format!(": {error}"),
            }
        ),
    };
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|_| unreadable())?;
    if output.status.success() && value.get("ok") == Some(&Value::Bool(true)) {
        return Ok(value);
    }
    let text = |key| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match (text("error"), text("message")) {
        (None, None) => Err(unreadable()),
        (code, message) => Err(Refusal {
            message: message.or_else(|| code.clone()).unwrap_or_default(),
            code: code.unwrap_or_default(),
        }),
    }
}

/// Runs `program` with `args` and reads its answer as `T`; `cancel` stops it and reaps it.
fn call<T: serde::de::DeserializeOwned>(
    program: &str,
    args: &[&str],
    cancel: &AtomicBool,
) -> Result<T, Refusal> {
    let output =
        crate::command::run(program, args, None, TIMEOUT, cancel).map_err(|error| Refusal {
            code: String::new(),
            message: format!("fence did not run: {error:#}"),
        })?;
    serde_json::from_value(answer(&output)?).map_err(|error| Refusal {
        code: String::new(),
        message: format!("fence's answer could not be read: {error}"),
    })
}

/// fence's version, when it runs and keeps the contract paddock reads; otherwise none, and
/// paddock offers no sandbox.
pub fn version(program: &str, cancel: &AtomicBool) -> Option<Version> {
    call::<Version>(program, &["--version"], cancel)
        .ok()
        .filter(|version| version.contract == CONTRACT)
}

/// The profiles, and the kinds fence has data for.
pub fn profiles(program: &str, cancel: &AtomicBool) -> Result<Profiles, Refusal> {
    call(program, &["profiles"], cancel)
}

/// What `profile` comes to for `kind` in `cwd`; refused when it can't be used there.
pub fn check(
    program: &str,
    profile: &str,
    kind: Option<&str>,
    cwd: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Check, Refusal> {
    let mut args = vec!["check", "--profile", profile];
    if let Some(kind) = kind {
        args.extend(["--kind", kind]);
    }
    if let Some(cwd) = cwd {
        args.extend(["--cwd", cwd]);
    }
    call(program, &args, cancel)
}

/// What the wall stopped in `agent`'s run `instance`, the last [`LIMIT`]; none while fence has no
/// record of it yet (`not_found`, also for a moment after `corral start`).
pub fn denials(
    program: &str,
    agent: &str,
    instance: &str,
    cancel: &AtomicBool,
) -> Result<Option<Denials>, Refusal> {
    let limit = LIMIT.to_string();
    let args = [
        "denials",
        "--agent",
        agent,
        "--instance",
        instance,
        "--limit",
        &limit,
    ];
    match call(program, &args, cancel) {
        Ok(denials) => Ok(Some(denials)),
        Err(refusal) if refusal.code == "not_found" => Ok(None),
        Err(refusal) => Err(refusal),
    }
}

/// What the watcher found.
#[derive(Clone, Debug, PartialEq)]
pub enum Update {
    /// fence as found, or none: not there, or another contract.
    Found(Option<Version>),
    /// One agent's run: what was stopped, none before fence has a record of it.
    Denials {
        agent: String,
        instance: String,
        result: Result<Option<Denials>, Refusal>,
    },
}

/// How often fence itself is looked for again, so one installed later is found.
const LOOK_AGAIN: Duration = Duration::from_secs(60);

/// Looks for fence, then every `every` reads what the wall stopped for each sandboxed agent it is
/// told of, by name and instance; on its own thread.
pub struct Watcher {
    pub updates: mpsc::Receiver<Update>,
    watched: Vec<(String, String)>,
    wake: mpsc::Sender<Vec<(String, String)>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Watcher {
    pub fn start(program: String, every: Duration) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel::<Vec<(String, String)>>();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            let mut runs: Vec<(String, String)> = Vec::new();
            let mut found = None;
            let mut looked: Option<Instant> = None;
            loop {
                if quitting.load(Ordering::Relaxed) {
                    break;
                }
                if looked.is_none_or(|at| at.elapsed() >= LOOK_AGAIN) {
                    looked = Some(Instant::now());
                    let now = version(&program, &quitting);
                    if now != found {
                        found = now.clone();
                        if tx.send(Update::Found(now)).is_err() {
                            break;
                        }
                    }
                }
                if found.is_some() {
                    for (agent, instance) in &runs {
                        if quitting.load(Ordering::Relaxed) {
                            return;
                        }
                        let result = denials(&program, agent, instance, &quitting);
                        let update = Update::Denials {
                            agent: agent.clone(),
                            instance: instance.clone(),
                            result,
                        };
                        if tx.send(update).is_err() {
                            return;
                        }
                    }
                }
                match wait.recv_timeout(every) {
                    Ok(next) => runs = next,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                if let Some(latest) = wait.try_iter().last() {
                    runs = latest;
                }
            }
        });
        Self {
            updates,
            watched: Vec::new(),
            wake,
            cancel,
            worker: Some(worker),
        }
    }

    /// The runs to read from now on, by agent name and instance; new ones are read at once.
    pub fn watch(&mut self, runs: Vec<(String, String)>) {
        if runs != self.watched {
            self.watched.clone_from(&runs);
            let _ = self.wake.send(runs);
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.wake.send(Vec::new());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// What the watcher has told, kept for drawing: whether fence is there, and each sandboxed run's
/// latest read, by agent name.
#[derive(Clone, Debug, Default)]
pub struct Seen {
    pub found: Option<Version>,
    /// By agent name: the instance read and what it gave.
    runs: HashMap<String, (String, Result<Option<Denials>, Refusal>)>,
}

impl Seen {
    pub fn absorb(&mut self, update: Update) {
        match update {
            Update::Found(found) => {
                if found.is_none() {
                    self.runs.clear();
                }
                self.found = found;
            }
            Update::Denials {
                agent,
                instance,
                result,
            } => {
                self.runs.insert(agent, (instance, result));
            }
        }
    }

    /// The latest read of `agent`'s run `instance`; none before one, or for another run.
    pub fn of(&self, agent: &str, instance: &str) -> Option<&Result<Option<Denials>, Refusal>> {
        self.runs
            .get(agent)
            .filter(|(seen, _)| seen == instance)
            .map(|(_, result)| result)
    }

    /// Forgets the runs no longer watched.
    pub fn keep(&mut self, runs: &[(String, String)]) {
        self.runs
            .retain(|agent, (instance, _)| runs.iter().any(|(a, i)| a == agent && i == instance));
    }
}

/// How many records the card counts for a run: `3`, `5000+` when more may be past the limit; none
/// before fence has a record or when it could not be read.
pub fn count(read: Option<&Result<Option<Denials>, Refusal>>) -> Option<(usize, bool)> {
    match read {
        Some(Ok(Some(denials))) => Some((denials.blocked(), denials.capped())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};

    /// Never cancelled.
    static NO: AtomicBool = AtomicBool::new(false);

    /// A fake `fence` in a fresh directory: `script` is its shell body.
    fn fake(name: &str, script: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!(
            "paddock-fence-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fence");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let program = path.display().to_string();
        (dir, program)
    }

    const PROFILES: &str = r#"{"kinds":[{"kind":"claude","services":["anthropic"],"source":"builtin"},{"kind":"codex","services":["openai"],"source":"builtin"},{"kind":"omp","services":[],"source":"builtin"},{"kind":"pi","services":[],"source":"builtin"}],"local":{"exists":false,"path":"/Users/u/.fence/local.toml"},"ok":true,"profiles":[{"description":"在工作目录里改代码","name":"dev","overrides_builtin":false,"path":null,"source":"builtin"},{"name":"broken","source":"user","path":"/Users/u/.fence/profiles/broken.toml","overrides_builtin":false,"error":{"error":"invalid_data","message":"bad toml"}}],"services":[],"new_field":1}"#;

    #[test]
    fn the_version_counts_only_with_contract_one() {
        let (_dir, program) = fake(
            "version",
            r#"echo '{"contract":"1","node":"v24.13.1","ok":true,"srt":"0.0.80","version":"0.1.0"}'"#,
        );
        let version = version(&program, &NO).unwrap();
        assert_eq!(version.version, "0.1.0");
        assert_eq!(version.srt.as_deref(), Some("0.0.80"));
        let (_dir, other) = fake(
            "contract",
            r#"echo '{"contract":"2","ok":true,"version":"9"}'"#,
        );
        assert_eq!(super::version(&other, &NO), None);
        // Not there at all.
        assert_eq!(super::version("/nonexistent/fence", &NO), None);
    }

    #[test]
    fn profiles_are_read_with_the_kinds_and_unknown_fields_left_alone() {
        let (_dir, program) = fake("profiles", &format!("echo '{PROFILES}'"));
        let profiles = profiles(&program, &NO).unwrap();
        assert_eq!(profiles.profiles.len(), 2);
        assert!(profiles.supports("claude") && profiles.supports("pi"));
        assert!(!profiles.supports("amp") && !profiles.supports("agy"));
        assert!(profiles.has("dev") && !profiles.has("nope"));
        let dev = &profiles.profiles[0];
        assert_eq!(dev.origin(), "built in");
        assert_eq!(dev.problem(), None);
        let broken = &profiles.profiles[1];
        assert_eq!(broken.origin(), "yours");
        assert_eq!(broken.problem().as_deref(), Some("invalid_data: bad toml"));
        assert_eq!(origin("system"), "system");
        let local = profiles.local.unwrap();
        assert!(!local.exists);
        assert_eq!(local.path, "/Users/u/.fence/local.toml");
    }

    #[test]
    fn web_search_servers_are_listed_and_a_check_says_which_one_it_gives() {
        const SERVERS: &str = r#"{"kinds":[],"mcp":[{"name":"exa","source":"builtin"},{"name":"mine","source":"user"},{"name":"broken","source":"user","error":{"error":"invalid_data","message":"bad toml"}}],"ok":true,"profiles":[]}"#;
        const CHECK: &str = r#"{"ok":true,"profile":"dev","layers":[{"source":"profile","name":"dev","origin":"builtin"},{"source":"mcp","name":"exa","origin":"builtin"}],"settings":{}}"#;
        let script = format!(
            r#"case "$1" in
  profiles) echo '{SERVERS}';;
  check) echo '{CHECK}';;
  *) exit 2;;
esac"#
        );
        let (_dir, program) = fake("mcp", &script);
        let profiles = super::profiles(&program, &NO).unwrap();
        let servers = profiles.mcp.as_ref().unwrap();
        assert_eq!(servers.len(), 3);
        assert_eq!(servers[0].name, "exa");
        assert_eq!(servers[0].origin(), "built in");
        assert_eq!(servers[0].problem(), None);
        assert_eq!(servers[1].origin(), "yours");
        assert_eq!(
            servers[2].problem().as_deref(),
            Some("invalid_data: bad toml")
        );
        let check = super::check(&program, "dev", Some("claude"), None, &NO).unwrap();
        let search: Vec<String> = check.search().iter().map(Search::words).collect();
        assert_eq!(search, ["exa (built in)"]);
    }

    #[test]
    fn an_old_fence_has_no_web_search_field_and_a_check_without_one_says_none() {
        // Before R30 fence doesn't list the servers at all; the new fence says `[]` for none.
        let old: Profiles =
            serde_json::from_str(r#"{"kinds":[],"ok":true,"profiles":[]}"#).unwrap();
        assert_eq!(old.mcp, None);
        let new: Profiles =
            serde_json::from_str(r#"{"kinds":[],"mcp":[],"ok":true,"profiles":[]}"#).unwrap();
        assert_eq!(new.mcp, Some(Vec::new()));
        let check: Check = serde_json::from_str(
            r#"{"layers":[{"name":"dev","origin":"builtin","source":"profile"}],"ok":true,"profile":"dev"}"#,
        )
        .unwrap();
        assert!(check.search().is_empty());
    }

    #[test]
    fn a_check_gives_the_layers_rules_and_warnings_or_fences_refusal() {
        let ok = r#"{"ok":true,"profile":"dev","kind":"codex","cwd":"/w/x","layers":[{"source":"profile","name":"dev","origin":"builtin"},{"source":"repo","path":"/w/x/.fence.toml"}],"settings":{"network":{"allowedDomains":["chatgpt.com"],"deniedDomains":[]},"filesystem":{"denyRead":["/Users/u"],"allowRead":["/w/x"],"allowWrite":["/w/x"],"denyWrite":["/w/x/.fence.toml"]},"allowPty":true},"skipped":[{"entry":"${target}","reason":"no value"}],"warnings":["weaker_network_isolation: 削弱网络隔离"]}"#;
        // The arguments come back as given, so the test sees them.
        let script = format!(
            r#"case "$*" in
  "check --profile dev --kind codex --cwd /w/x") echo '{ok}';;
  *bad*) echo '{{"error":"bad_cwd","message":"no directory: /bad","ok":false}}'; exit 1;;
  *) echo "unexpected $*" >&2; exit 2;;
esac"#
        );
        let (_dir, program) = fake("check", &script);
        let check = check(&program, "dev", Some("codex"), Some("/w/x"), &NO).unwrap();
        assert_eq!(check.settings.network.allowed_domains, ["chatgpt.com"]);
        assert_eq!(check.settings.filesystem.deny_write, ["/w/x/.fence.toml"]);
        assert_eq!(check.warnings, ["weaker_network_isolation: 削弱网络隔离"]);
        assert_eq!(check.skipped[0].entry, "${target}");
        let tags: Vec<String> = check.layers.iter().map(Layer::tag).collect();
        assert_eq!(tags, ["profile dev", "repo .fence.toml"]);
        let refusal = super::check(&program, "dev", Some("claude"), Some("/bad"), &NO).unwrap_err();
        assert_eq!(refusal.code, "bad_cwd");
        assert_eq!(refusal.message, "no directory: /bad");
        assert_eq!(refusal.to_string(), "bad_cwd: no directory: /bad");
        // Something that is not fence's JSON.
        let refusal = super::check(&program, "x", None, None, &NO).unwrap_err();
        assert_eq!(refusal.code, "");
        assert!(refusal.message.contains("unexpected"), "{refusal}");
    }

    #[test]
    fn denials_count_network_and_writes_and_not_found_is_no_data_yet() {
        let ok = r#"{"ok":true,"agent":"p/a","instance":"i1","profile":"dev","running":true,"denials":[{"t":1760760123.4,"type":"network","target":"example.com:443","detail":"deny network-outbound"},{"t":1760760124.0,"type":"file-read","target":"/Users/u/.ssh","detail":"deny file-read-data"},{"t":1760760125.0,"type":"file-write","target":null,"detail":"deny file-write-data x"},{"t":1760760126.0,"type":"other","target":null,"detail":"mach-lookup"},{"t":1760760127.0,"type":"strange","target":"z"}],"truncated":true}"#;
        let script = format!(
            r#"case "$*" in
  "denials --agent p/a --instance i1 --limit 5000") echo '{ok}';;
  *i2*) echo '{{"error":"not_found","message":"no run","ok":false}}'; exit 1;;
  *) echo '{{"error":"usage","message":"what","ok":false}}'; exit 1;;
esac"#
        );
        let (_dir, program) = fake("denials", &script);
        let read = denials(&program, "p/a", "i1", &NO).unwrap().unwrap();
        assert_eq!(read.denials.len(), 5);
        assert_eq!(read.blocked(), 2);
        assert!(read.truncated && read.running && !read.capped());
        let tags: Vec<String> = read.denials.iter().map(Denial::tag).collect();
        assert_eq!(tags, ["Network", "Read", "Write", "Other", "strange"]);
        assert_eq!(super::denials(&program, "p/a", "i2", &NO), Ok(None));
        let refusal = super::denials(&program, "p/b", "i3", &NO).unwrap_err();
        assert_eq!(refusal.code, "usage");
    }

    #[test]
    fn what_is_seen_is_kept_by_run_and_forgotten_when_fence_goes() {
        let mut seen = Seen::default();
        let version = Version {
            version: "0.1.0".into(),
            contract: "1".into(),
            ..Version::default()
        };
        seen.absorb(Update::Found(Some(version)));
        let read = Denials {
            denials: vec![Denial {
                kind: "network".into(),
                ..Denial::default()
            }],
            ..Denials::default()
        };
        seen.absorb(Update::Denials {
            agent: "p/a".into(),
            instance: "i1".into(),
            result: Ok(Some(read)),
        });
        assert_eq!(count(seen.of("p/a", "i1")), Some((1, false)));
        // Another run of the same name has nothing yet.
        assert_eq!(count(seen.of("p/a", "i2")), None);
        seen.keep(&[("p/a".into(), "i2".into())]);
        assert_eq!(seen.of("p/a", "i1"), None);
        seen.absorb(Update::Denials {
            agent: "p/a".into(),
            instance: "i2".into(),
            result: Ok(None),
        });
        assert_eq!(count(seen.of("p/a", "i2")), None);
        seen.absorb(Update::Found(None));
        assert!(seen.found.is_none());
        assert_eq!(seen.of("p/a", "i2"), None);
    }

    #[test]
    fn the_watcher_finds_fence_and_reads_each_run_it_is_told_of() {
        let script = r#"case "$1" in
  --version) echo '{"contract":"1","ok":true,"version":"0.1.0"}';;
  denials) echo '{"ok":true,"denials":[{"t":1,"type":"file-write","target":"/x"}],"truncated":false}';;
esac"#;
        let (_dir, program) = fake("watcher", script);
        let mut watcher = Watcher::start(program, Duration::from_millis(50));
        watcher.watch(vec![("p/a".into(), "i1".into())]);
        let mut seen = Seen::default();
        let deadline = Instant::now() + Duration::from_secs(10);
        while count(seen.of("p/a", "i1")).is_none() && Instant::now() < deadline {
            if let Ok(update) = watcher.updates.recv_timeout(Duration::from_millis(100)) {
                seen.absorb(update);
            }
        }
        assert_eq!(
            seen.found.as_ref().map(|v| v.version.as_str()),
            Some("0.1.0")
        );
        assert_eq!(count(seen.of("p/a", "i1")), Some((1, false)));
    }

    /// A fake fence that writes its process number next to itself and then hangs.
    fn stuck(name: &str) -> (PathBuf, String) {
        fake(name, "echo $$ > \"$(dirname \"$0\")/pid\"\nexec sleep 30")
    }

    /// The process number `stuck` wrote, once it has.
    fn stuck_pid(dir: &std::path::Path) -> String {
        let path = dir.join("pid");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let pid = std::fs::read_to_string(&path).unwrap_or_default();
            if !pid.trim().is_empty() || Instant::now() >= deadline {
                return pid.trim().to_owned();
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Whether a process numbered `pid` still exists (killed and reaped ones don't).
    fn alive(pid: &str) -> bool {
        crate::command::run(
            "kill",
            &["-0", pid],
            None,
            Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap()
        .status
        .success()
    }

    #[test]
    fn a_check_no_longer_wanted_stops_at_once() {
        let (dir, program) = stuck("stuck-check");
        let cancel = Cancel::default();
        let flag = cancel.flag();
        let call = thread::spawn(move || check(&program, "dev", None, None, &flag));
        let pid = stuck_pid(&dir);
        assert!(!pid.is_empty());
        let began = Instant::now();
        drop(cancel);
        let answer = call.join().unwrap();
        assert!(
            began.elapsed() < Duration::from_secs(3),
            "{:?}",
            began.elapsed()
        );
        assert!(answer.is_err());
        assert!(!alive(&pid));
    }

    #[test]
    fn dropping_the_watcher_stops_the_fence_it_is_waiting_on() {
        let (dir, program) = stuck("stuck-watcher");
        let watcher = Watcher::start(program, Duration::from_millis(50));
        let pid = stuck_pid(&dir);
        assert!(!pid.is_empty());
        let began = Instant::now();
        drop(watcher);
        assert!(
            began.elapsed() < Duration::from_secs(3),
            "{:?}",
            began.elapsed()
        );
        assert!(!alive(&pid));
    }
}
