//! Starting an agent on another machine (DESIGN §13 P5-124), the simple way: the kind, its
//! command, the directory and the name, nothing else. What runs there goes through the user's own
//! login shell there (`$SHELL -l -i -c`), as an agent installed under nvm or the like is found only
//! there; every word the user typed is quoted as [`crate::remote::quote`] has it. `~` is that
//! machine's home, as it said on connecting, never this Mac's. The command is the kind's own or
//! one the user wrote (P5-124c); what started is kept for that machine in [`Commands`]. Nothing
//! here runs on the UI thread.
use crate::{
    corral_colors::Colors,
    new_agent::{Started, Tool, suggest_prefix},
    remote::{Link, command_line, quote},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

/// The kinds offered there, in this order: amp needs its plugin and agy a first message, which the
/// simple dialog has neither of. kimi starts without its hooks too, shown ready on screen; whether
/// they are installed there is not asked (P5-179).
pub const KINDS: [Tool; 5] = [Tool::Claude, Tool::Codex, Tool::Pi, Tool::Omp, Tool::Kimi];
/// How long looking for the kinds may take.
const LOOK: Duration = Duration::from_secs(20);
/// How long `corral start` may take there, as on this Mac.
const START: Duration = Duration::from_secs(120);
/// How many directories are remembered for each machine.
const RECENT: usize = 5;

/// `inner` run by the shell of the user there, the one `$SHELL` names (`sh` when unset), as an
/// interactive login shell, its standard input on nothing: what its profile or its rc puts on the
/// PATH is found, as in a terminal there. zsh reads `~/.zshrc`, where nvm is set up, only when
/// interactive (P5-124b).
pub fn login(inner: &str) -> String {
    format!(
        "exec \"${{SHELL:-/bin/sh}}\" -l -i -c {} </dev/null",
        quote(inner)
    )
}

/// The command that says which of [`KINDS`] the login shell there finds: `command -v` for each,
/// the kind's name on a line of its own for each found.
pub fn look_command() -> String {
    let kinds: Vec<&str> = KINDS.iter().map(|tool| tool.kind()).collect();
    login(&format!(
        "for kind in {}; do if command -v \"$kind\" >/dev/null 2>&1; then printf '%s\\n' \"$kind\"; fi; done",
        kinds.join(" ")
    ))
}

/// The kinds [`look_command`] found, in the order of [`KINDS`]; what else the login shell printed
/// (a profile's greeting) is passed over.
pub fn found(stdout: &[u8]) -> Vec<Tool> {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    KINDS
        .into_iter()
        .filter(|tool| lines.contains(&tool.kind()))
        .collect()
}

/// Looks for the kinds there; blocks, so call it off the UI thread.
pub fn look(link: &Link) -> Result<Vec<Tool>, String> {
    let output = link
        .run(&look_command(), LOOK, &AtomicBool::new(false))
        .map_err(|problem| problem.to_string())?;
    Ok(found(&output.stdout))
}

/// Which field a problem is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Command,
    Dir,
    Name,
}

/// What is started: one of the kinds found there, its command as written; or one of the user's
/// own, known by its command alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agent {
    Kind(Tool),
    Custom,
}

/// The words of a command, split as a shell would (quotes and all), as on this Mac's New Agent;
/// else why not.
pub fn check_command(command: &str) -> Result<Vec<String>, String> {
    if command.chars().any(char::is_control) {
        return Err("Command can't hold control characters".into());
    }
    match shell_words::split(command) {
        Err(_) => Err("Command has an unclosed quote".into()),
        Ok(words) if words.first().is_none_or(|word| word.is_empty()) => {
            Err("Command is required".into())
        }
        Ok(words) => Ok(words),
    }
}

/// The name a command of the user's own goes by: its first word's last part, as a name's prefix
/// is made (`~/bin/my-agent --fast` is `my-agent`); none for a command that cannot be read.
pub fn custom_name(command: &str) -> Option<String> {
    let words = check_command(command).ok()?;
    Some(suggest_prefix(&words[0]))
}

/// The kind among those `found` there that a command of the user's own called `name` is.
pub fn found_kind(name: &str, found: &[Tool]) -> Option<Tool> {
    found.iter().copied().find(|tool| tool.kind() == name)
}

/// Whether `command` says something else than `tool`'s own, word for word.
pub fn edited(command: &str, tool: Tool) -> bool {
    shell_words::split(command).ok() != shell_words::split(tool.command()).ok()
}

/// The directory as typed: a full path, `~` or `~/…`, else why not.
pub fn check_dir(dir: &str) -> Result<(), String> {
    let dir = dir.trim();
    if dir.is_empty() {
        return Err("Directory is required".into());
    }
    if dir.chars().any(char::is_control) {
        return Err("Directory can't hold control characters".into());
    }
    if dir.starts_with('/') || dir == "~" || dir.starts_with("~/") {
        Ok(())
    } else if dir.starts_with('~') {
        Err("Only your own ~ works here; write ~/… or a full path".into())
    } else {
        Err("Write a full path, ~ or ~/…".into())
    }
}

/// The directory there: `~` and `~/…` under `home`, that machine's home.
pub fn expand(dir: &str, home: &str) -> String {
    tilde(dir.trim(), home)
}

/// `word` with a leading `~` or `~/` as `home`, as a shell has it.
fn tilde(word: &str, home: &str) -> String {
    match word.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
        _ => word.to_owned(),
    }
}

/// The name used when none is typed: the directory's last part (`home` for `~`), then the kind or
/// the name of the user's own command (`agent`). corral is asked for a free one like it
/// (`--unique`), and says which.
pub fn default_name(dir: &str, agent: &str) -> String {
    format!("{}/{agent}", name_prefix(dir))
}

/// The part of [`default_name`] before the `/`.
pub fn name_prefix(dir: &str) -> String {
    let dir = dir.trim().trim_end_matches('/');
    if dir == "~" {
        "home".into()
    } else {
        suggest_prefix(dir)
    }
}

/// A typed name, as on this Mac: `prefix/name`, no spaces, neither part starting with `-`.
pub fn check_name(name: &str) -> Result<(), String> {
    let problem = || Err("Name needs prefix/name, no spaces or leading '-'".into());
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return problem();
    }
    match name.split_once('/') {
        Some((prefix, rest))
            if !prefix.is_empty()
                && !rest.is_empty()
                && !prefix.starts_with('-')
                && !rest.starts_with('-') =>
        {
            Ok(())
        }
        _ => problem(),
    }
}

/// What the dialog asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub agent: Agent,
    /// As typed, split as [`check_command`] does.
    pub command: String,
    /// As typed: `~` not yet that machine's home.
    pub dir: String,
    /// As typed; empty for [`default_name`].
    pub name: String,
}

impl Request {
    /// The first problem, by field.
    pub fn problem(&self) -> Option<(Field, String)> {
        if let Err(problem) = check_command(&self.command) {
            return Some((Field::Command, problem));
        }
        if let Err(problem) = check_dir(&self.dir) {
            return Some((Field::Dir, problem));
        }
        if !self.name.trim().is_empty()
            && let Err(problem) = check_name(&self.name)
        {
            return Some((Field::Name, problem));
        }
        None
    }

    /// What the agent goes by in [`default_name`]: the kind, or the name of the user's own command;
    /// none while that command cannot be read.
    pub fn agent_name(&self) -> Option<String> {
        match self.agent {
            Agent::Kind(tool) => Some(tool.kind().to_owned()),
            Agent::Custom => custom_name(&self.command),
        }
    }

    /// The name corral is asked for.
    pub fn name(&self) -> String {
        match self.name.trim() {
            "" => default_name(&self.dir, &self.agent_name().unwrap_or_default()),
            _ => self.name.clone(),
        }
    }

    /// The command for ssh: `corral start` through the login shell there, its directory under
    /// `home`, that machine's home, then each word of the command as it was split, a word's
    /// leading `~` that home too, as corral runs the words as they come, with no shell. Only a
    /// request with no [`Request::problem`] is started. First tells the same corral the terminal
    /// colours; an old corral's error and output are ignored without another SSH round trip.
    pub fn command(&self, corral: &str, home: &str, colors: &Colors) -> String {
        let mut words = vec![
            corral.to_owned(),
            "start".into(),
            self.name(),
            "--unique".into(),
            "--cwd".into(),
            expand(&self.dir, home),
            "--label".into(),
            "role=regular".into(),
            "--".into(),
        ];
        let typed = check_command(&self.command).unwrap_or_default();
        words.extend(typed.iter().map(|word| tilde(word, home)));
        let set_colors = format!("{} {}", quote(corral), command_line(&colors.args()));
        login(&format!(
            "{set_colors} >/dev/null 2>&1 || :; {}",
            command_line(&words)
        ))
    }
}

/// Why it did not start: what came back, as it came, and the field it is about when corral says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub text: String,
    pub field: Option<Field>,
}

impl Failure {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            field: None,
        }
    }
}

/// Runs [`Request::command`] there with the corral and home it found on connecting; blocks, so
/// call it off the UI thread.
pub fn start(link: &Link, request: &Request, colors: &Colors) -> Result<Started, Failure> {
    if let Some((field, text)) = request.problem() {
        return Err(Failure {
            text,
            field: Some(field),
        });
    }
    let snapshot = link.snapshot();
    let label = link.label();
    let corral = snapshot
        .corral
        .ok_or_else(|| Failure::plain(format!("{label} is not connected")))?;
    let home = snapshot
        .home
        .ok_or_else(|| Failure::plain(format!("{label} did not say where its home is")))?;
    let command = request.command(&corral, &home, colors);
    let output = link
        .run(&command, START, &AtomicBool::new(false))
        .map_err(|problem| Failure::plain(problem.to_string()))?;
    answer(&output)
}

/// What `corral start` answered there: the agent it started, or what it said, as it said it.
pub fn answer(output: &crate::command::Output) -> Result<Started, Failure> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (said, value) = match json_in(&stdout) {
        Some((at, value)) => (Some(stdout[at..].trim()), Some(value)),
        None => (None, None),
    };
    let refused = value
        .as_ref()
        .is_none_or(|value| value.get("ok") == Some(&Value::Bool(false)));
    if output.status.success() && !refused {
        let value = value.unwrap_or_default();
        let name = value["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                Failure::plain("corral start returned no name; whether it started is unknown")
            })?;
        return Ok(Started {
            name: name.to_owned(),
            instance: value["instance"].as_str().map(str::to_owned),
        });
    }
    // corral's own answer when there is one; else all that came back.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let said: Vec<&str> = match said {
        Some(json) => vec![json],
        None => [stdout.trim(), stderr.trim()]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect(),
    };
    let text = match said.is_empty() {
        true => match output.status.code() {
            Some(code) => format!("corral start exited {code} and said nothing"),
            None => "corral start was stopped and said nothing".into(),
        },
        false => said.join("\n"),
    };
    let field = value
        .filter(|value| value["error"] == "bad_cwd")
        .map(|_| Field::Dir);
    Err(Failure { text, field })
}

/// corral's JSON in what came back, and where it starts: all of it, or from a line on that starts
/// with `{`, after whatever a login profile printed first.
fn json_in(text: &str) -> Option<(usize, Value)> {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with('{')
            && let Ok(value) = serde_json::from_str::<Value>(&text[at..])
        {
            return Some((at, value));
        }
        at += line.len();
    }
    None
}

/// The directories recently started in, latest first, each machine's apart (by its `name`), and
/// apart from this Mac's New Agent's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecentDirs(BTreeMap<String, Vec<String>>);

impl RecentDirs {
    /// As kept at `path`; none when there is no list or it cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .map(Self)
            .unwrap_or_default()
    }

    pub fn of(&self, machine: &str) -> &[String] {
        self.0.get(machine).map_or(&[], Vec::as_slice)
    }

    /// Puts `dir` first among `machine`'s, once, keeping the latest few.
    pub fn remember(&mut self, machine: &str, dir: &str) {
        let dir = dir.trim();
        let dirs = self.0.entry(machine.to_owned()).or_default();
        dirs.retain(|known| known != dir);
        dirs.insert(0, dir.to_owned());
        dirs.truncate(RECENT);
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        save_json(&self.0, path)
    }
}

/// Writes `value` to `path` whole: to a file beside it first, then in its place.
fn save_json(value: &impl Serialize, path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("json.saving");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

/// Where they are kept, beside the layout and this Mac's recent projects.
pub fn recent_path() -> Option<PathBuf> {
    crate::layout_state::default_path().map(|path| path.with_file_name("recent-remote-dirs.json"))
}

/// One of the user's own commands kept for a machine, a capsule of its own in the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Custom {
    /// As [`custom_name`] has it.
    pub name: String,
    pub command: String,
}

impl Custom {
    /// The kind whose icon it wears: the one its name is (`amp`), if any.
    pub fn kind(&self) -> Option<Tool> {
        Tool::ALL.into_iter().find(|tool| tool.kind() == self.name)
    }
}

/// What is kept for one machine.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Kept {
    /// The commands of the kinds found there as edited, by kind (`pi`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    edited: BTreeMap<String, String>,
    /// The user's own commands, in the order first started.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    custom: Vec<String>,
}

impl Kept {
    fn is_empty(&self) -> bool {
        self.edited.is_empty() && self.custom.is_empty()
    }
}

/// The commands that started there, each machine's apart (by its `name`): a found kind's as
/// edited, and the user's own. Only what corral started is kept ([`Commands::remember`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Commands(BTreeMap<String, Kept>);

impl Commands {
    /// As kept at `path`; none when there are none or they cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .map(Self)
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        save_json(&self.0, path)
    }

    /// `tool`'s command on `machine`: as edited there, else its own.
    pub fn command(&self, machine: &str, tool: Tool) -> String {
        self.0
            .get(machine)
            .and_then(|kept| kept.edited.get(tool.kind()))
            .cloned()
            .unwrap_or_else(|| tool.command().to_owned())
    }

    /// The user's own commands on `machine`, in order; of two by the same name, the later in the
    /// earlier's place.
    pub fn customs(&self, machine: &str) -> Vec<Custom> {
        let mut customs: Vec<Custom> = Vec::new();
        for command in self.0.get(machine).map_or(&[][..], |kept| &kept.custom) {
            let Some(name) = custom_name(command) else {
                continue;
            };
            let custom = Custom {
                name,
                command: command.clone(),
            };
            match customs.iter_mut().find(|known| known.name == custom.name) {
                Some(known) => *known = custom,
                None => customs.push(custom),
            }
        }
        customs
    }

    /// The user's own commands on `machine` that get a capsule beside the kinds `found` there: not
    /// one named as a kind found, which is kept all the same for when that kind is not found.
    pub fn shown(&self, machine: &str, found: &[Tool]) -> Vec<Custom> {
        let mut customs = self.customs(machine);
        customs.retain(|custom| found_kind(&custom.name, found).is_none());
        customs
    }

    /// What `request` started on `machine`: a kind's command as edited, or forgotten when it is
    /// the kind's own again; or the user's own, in place of one by the same name, else last. The
    /// user's own named as a kind `found` there this time is that kind's, as if written on its
    /// capsule.
    pub fn remember(&mut self, machine: &str, request: &Request, found: &[Tool]) {
        let command = request.command.trim().to_owned();
        let agent = match request.agent {
            Agent::Custom => custom_name(&command)
                .and_then(|name| found_kind(&name, found))
                .map_or(Agent::Custom, Agent::Kind),
            kind => kind,
        };
        let kept = self.0.entry(machine.to_owned()).or_default();
        match agent {
            Agent::Kind(tool) if edited(&command, tool) => {
                kept.edited.insert(tool.kind().to_owned(), command);
            }
            Agent::Kind(tool) => {
                kept.edited.remove(tool.kind());
            }
            Agent::Custom => {
                if let Some(name) = custom_name(&command) {
                    let same = |known: &String| custom_name(known).as_deref() == Some(&*name);
                    match kept.custom.iter().position(same) {
                        Some(at) => kept.custom[at] = command,
                        None => kept.custom.push(command),
                    }
                }
            }
        }
        self.tidy(machine);
    }

    /// Forgets the user's own command called `name` on `machine`.
    pub fn forget(&mut self, machine: &str, name: &str) {
        if let Some(kept) = self.0.get_mut(machine) {
            kept.custom
                .retain(|command| custom_name(command).as_deref() != Some(name));
        }
        self.tidy(machine);
    }

    fn tidy(&mut self, machine: &str) {
        if self.0.get(machine).is_some_and(Kept::is_empty) {
            self.0.remove(machine);
        }
    }
}

/// What a start leaves for `machine`: when corral started it, its directory first among the recent
/// ones and its command kept; after a failure, nothing. Whether anything was kept.
pub fn keep(
    outcome: &Result<Started, Failure>,
    machine: &str,
    request: &Request,
    found: &[Tool],
    recent: &mut RecentDirs,
    commands: &mut Commands,
) -> bool {
    if outcome.is_err() {
        return false;
    }
    recent.remember(machine, &request.dir);
    commands.remember(machine, request, found);
    true
}

/// Where they are kept: beside the recent directories, in a file of their own.
pub fn commands_path() -> Option<PathBuf> {
    crate::layout_state::default_path()
        .map(|path| path.with_file_name("remote-agent-commands.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn output(code: i32, stdout: &str, stderr: &str) -> crate::command::Output {
        crate::command::Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// The words the login shell there is given, as `sh` reads the command ssh hands it.
    fn as_run(command: &str) -> Vec<String> {
        let words = shell_words::split(command).unwrap();
        assert_eq!(words[..5], ["exec", "${SHELL:-/bin/sh}", "-l", "-i", "-c"]);
        assert_eq!(words.len(), 7, "{command}");
        assert_eq!(words[6], "</dev/null");
        let (_, start) = words[5].split_once(" || :; ").unwrap();
        shell_words::split(start).unwrap()
    }

    fn colors() -> Colors {
        Colors::of(
            &crate::theme::Theme::from_config(
                &crate::config::Config::parse("theme = 'latte'").unwrap(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn terminal_colors_precede_start_and_usage_does_not_change_its_answer() {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("paddock-remote-colors-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let corral = dir.join("corral it's $x `literal`");
        let log = dir.join("args");
        std::fs::write(&corral, format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nif [ \"$1\" = terminal-colors ]; then\n echo '{{\"ok\":false,\"error\":\"usage\"}}'\n echo usage >&2\n exit 1\nfi\necho '{{\"ok\":true,\"name\":\"test/pi\",\"instance\":\"new\"}}'\n",
            quote(log.to_str().unwrap()),
        )).unwrap();
        std::fs::set_permissions(&corral, std::fs::Permissions::from_mode(0o755)).unwrap();
        let request = Request {
            agent: Agent::Kind(Tool::Pi),
            command: "pi".into(),
            dir: "~/it's $x".into(),
            name: "test/pi".into(),
        };
        let command = request.command(corral.to_str().unwrap(), "/remote/home", &colors());
        // Run only the inner command locally: no SSH and no user login files.
        let shell = shell_words::split(&command).unwrap();
        let output = crate::command::run(
            "/bin/sh",
            &["-c", &shell[5]],
            None,
            Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(output.stderr.is_empty());
        let started = answer(&output).unwrap();
        assert_eq!(started.name, "test/pi");
        assert_eq!(started.instance.as_deref(), Some("new"));
        assert_eq!(
            std::fs::read_to_string(&log).unwrap(),
            "terminal-colors --fg #4c4f69 --bg #eff1f5\nstart test/pi --unique --cwd /remote/home/it's $x --label role=regular -- pi\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn it_starts_through_the_login_shell_there_every_typed_word_as_written() {
        let request = Request {
            agent: Agent::Kind(Tool::Codex),
            command: Tool::Codex.command().into(),
            dir: "~/my dir/it's $(rm -rf ~); `id` \"q\"".into(),
            name: "p/it's;$x".into(),
        };
        let command = request.command("/home/me/.local/bin/corral", "/home/me", &colors());
        assert!(command.starts_with("exec \"${SHELL:-/bin/sh}\" -l -i -c '"));
        assert_eq!(
            as_run(&command),
            [
                "/home/me/.local/bin/corral",
                "start",
                "p/it's;$x",
                "--unique",
                "--cwd",
                "/home/me/my dir/it's $(rm -rf ~); `id` \"q\"",
                "--label",
                "role=regular",
                "--",
                "codex",
                "--yolo",
            ]
        );
    }

    #[test]
    fn tilde_is_that_machines_home_and_a_full_path_stays() {
        assert_eq!(expand("~", "/home/me"), "/home/me");
        assert_eq!(expand(" ~/x/y ", "/home/me"), "/home/me/x/y");
        assert_eq!(expand("/srv/a", "/home/me"), "/srv/a");
        let request = |dir: &str| Request {
            agent: Agent::Kind(Tool::Pi),
            command: Tool::Pi.command().into(),
            dir: dir.into(),
            name: String::new(),
        };
        // That machine's home, whatever this Mac's is.
        let there = "/remote/home/of/someone";
        assert_ne!(std::env::var("HOME").ok().as_deref(), Some(there));
        let words = as_run(&request("~").command("/c", there, &colors()));
        assert_eq!(words[5], there);
        let words = as_run(&request("~/w").command("/c", there, &colors()));
        assert_eq!(words[5], format!("{there}/w"));
    }

    #[test]
    fn an_empty_name_is_the_directorys_last_part_and_the_kind() {
        let named = |dir: &str, tool, name: &str| {
            Request {
                agent: Agent::Kind(tool),
                command: tool.command().into(),
                dir: dir.into(),
                name: name.into(),
            }
            .name()
        };
        assert_eq!(named("~", Tool::Pi, ""), "home/pi");
        assert_eq!(named("~/", Tool::Omp, ""), "home/omp");
        assert_eq!(named("~/Develop/ranch", Tool::Pi, ""), "ranch/pi");
        assert_eq!(named("/srv/my proj/", Tool::Claude, "  "), "my-proj/claude");
        assert_eq!(named("~/x", Tool::Codex, "a/b"), "a/b");
        // Asked for with --unique either way: corral says which name it took.
        for name in ["", "a/b"] {
            let words = as_run(
                &Request {
                    agent: Agent::Kind(Tool::Pi),
                    command: Tool::Pi.command().into(),
                    dir: "~".into(),
                    name: name.into(),
                }
                .command("/c", "/h", &colors()),
            );
            assert_eq!(words[3], "--unique");
        }
    }

    #[test]
    fn only_a_full_path_or_your_own_tilde_is_taken() {
        for good in ["/", "/srv/a b", "~", "~/", "~/x", " ~/x "] {
            assert_eq!(check_dir(good), Ok(()), "{good}");
        }
        assert_eq!(check_dir(""), Err("Directory is required".into()));
        assert_eq!(check_dir("   "), Err("Directory is required".into()));
        assert_eq!(
            check_dir("src/app"),
            Err("Write a full path, ~ or ~/…".into())
        );
        assert_eq!(check_dir("./x"), Err("Write a full path, ~ or ~/…".into()));
        assert!(check_dir("~root").unwrap_err().contains("own ~"));
        assert!(check_dir("~root/x").is_err());
        assert!(check_dir("/a\nb").is_err());
        let request = Request {
            agent: Agent::Kind(Tool::Pi),
            command: Tool::Pi.command().into(),
            dir: "rel".into(),
            name: String::new(),
        };
        assert_eq!(request.problem().map(|p| p.0), Some(Field::Dir));
    }

    #[test]
    fn a_typed_name_keeps_this_macs_rules() {
        for good in ["a/b", "ranch/pi-2", "a/b/c"] {
            assert_eq!(check_name(good), Ok(()), "{good}");
        }
        for bad in ["ab", "a b/c", "-a/b", "a/-b", "/b", "a/", "a/\tb"] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
        let request = Request {
            agent: Agent::Kind(Tool::Pi),
            command: Tool::Pi.command().into(),
            dir: "~".into(),
            name: "no slash".into(),
        };
        assert_eq!(request.problem().map(|p| p.0), Some(Field::Name));
    }

    #[test]
    fn the_kinds_found_are_read_in_order_past_whatever_the_profile_printed() {
        let command = look_command();
        let inner = &shell_words::split(&command).unwrap()[5];
        assert!(command.starts_with("exec \"${SHELL:-/bin/sh}\" -l -i -c '"));
        assert!(inner.contains("for kind in claude codex pi omp kimi; do"));
        assert!(inner.contains("command -v \"$kind\""));
        assert_eq!(found(b"Welcome to 5090\nomp\npi\n"), [Tool::Pi, Tool::Omp]);
        assert_eq!(found(b"codex\nclaude\n"), [Tool::Claude, Tool::Codex]);
        // kimi comes after omp (P5-179).
        assert_eq!(
            found(b"kimi\nomp\nclaude\n"),
            [Tool::Claude, Tool::Omp, Tool::Kimi]
        );
        // A path or an alias is not a kind's own line; amp and agy are never offered.
        assert!(found(b"/usr/bin/pi\npip\namp\nagy\n").is_empty());
        assert!(found(b"").is_empty());
    }

    #[test]
    fn corrals_answer_is_the_agent_or_what_it_said_as_it_said_it() {
        let started = answer(&output(
            0,
            "hello from .profile\n{\"ok\":true,\"name\":\"ranch/pi-2\",\"instance\":\"i-9\"}\n",
            "",
        ));
        assert_eq!(
            started,
            Ok(Started {
                name: "ranch/pi-2".into(),
                instance: Some("i-9".into()),
            })
        );
        let bad_cwd =
            r#"{"cwd":"/home/me/nope","error":"bad_cwd","message":"no such directory","ok":false}"#;
        assert_eq!(
            answer(&output(1, &format!("Welcome\n{bad_cwd}\n"), "a warning\n")),
            Err(Failure {
                text: bad_cwd.into(),
                field: Some(Field::Dir),
            })
        );
        let usage = r#"{"error":"usage","message":"bad name","ok":false}"#;
        assert_eq!(answer(&output(0, usage, "")).unwrap_err().field, None);
        assert_eq!(
            answer(&output(127, "", "zsh:1: no such file or directory: /c\n")),
            Err(Failure::plain("zsh:1: no such file or directory: /c"))
        );
        assert!(
            answer(&output(0, "{\"ok\":true}", ""))
                .unwrap_err()
                .text
                .contains("returned no name")
        );
        assert_eq!(
            answer(&output(3, "", "")).unwrap_err().text,
            "corral start exited 3 and said nothing"
        );
    }

    #[test]
    fn recent_directories_are_kept_for_each_machine_apart() {
        let mut recent = RecentDirs::default();
        recent.remember("studio", "~/a");
        recent.remember("5090", "~");
        recent.remember("studio", " ~/b ");
        recent.remember("studio", "~/a");
        assert_eq!(recent.of("studio"), ["~/a", "~/b"]);
        assert_eq!(recent.of("5090"), ["~"]);
        assert!(recent.of("other").is_empty());
        for n in 0..9 {
            recent.remember("5090", &format!("/p{n}"));
        }
        assert_eq!(recent.of("5090").len(), RECENT);
        assert_eq!(recent.of("5090")[0], "/p8");
        assert_eq!(recent.of("studio"), ["~/a", "~/b"]);

        let dir = std::env::temp_dir().join(format!("paddock-recent-dirs-{}", std::process::id()));
        let path = dir.join("recent-remote-dirs.json");
        assert_eq!(RecentDirs::load(&path), RecentDirs::default());
        recent.save(&path).unwrap();
        assert_eq!(RecentDirs::load(&path), recent);
        // This Mac's list is another file, and stays as it is.
        let local = dir.join("recent-projects.json");
        crate::new_agent::save_recent(&local, &["/mine".to_owned()]).unwrap();
        recent.save(&path).unwrap();
        assert_eq!(crate::new_agent::load_recent(&local), ["/mine"]);
        assert_ne!(
            recent_path().map(|p| p.file_name().map(ToOwned::to_owned)),
            crate::new_agent::recent_path().map(|p| p.file_name().map(ToOwned::to_owned))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn custom(command: &str) -> Request {
        Request {
            agent: Agent::Custom,
            command: command.into(),
            dir: "~".into(),
            name: String::new(),
        }
    }

    fn kind(tool: Tool, command: &str) -> Request {
        Request {
            agent: Agent::Kind(tool),
            ..custom(command)
        }
    }

    #[test]
    fn a_command_is_split_as_a_shell_would_then_each_word_quoted_for_there() {
        let typed = r#"~/bin/my-agent "two words" 'it''s $HOME' a\ b ';' '`id`' "$(touch x)" --cfg ~/c.toml x~/y"#;
        let words = as_run(&custom(typed).command("/c", "/home/me", &colors()));
        assert_eq!(
            words[9..],
            [
                // A word's leading ~ is that machine's home: corral runs the words with no shell.
                "/home/me/bin/my-agent",
                "two words",
                "its $HOME",
                "a b",
                ";",
                "`id`",
                "$(touch x)",
                "--cfg",
                "/home/me/c.toml",
                "x~/y",
            ]
        );
        assert_eq!(words[8], "--");
        // A kind's command as edited goes the same way.
        let words = as_run(&kind(Tool::Pi, "pi --model 'son net'").command("/c", "/h", &colors()));
        assert_eq!(words[9..], ["pi", "--model", "son net"]);
    }

    #[test]
    fn a_command_that_cannot_be_split_or_is_empty_is_a_problem_of_its_own() {
        for (typed, problem) in [
            ("amp 'oops", "Command has an unclosed quote"),
            ("amp \"oops", "Command has an unclosed quote"),
            ("", "Command is required"),
            ("   ", "Command is required"),
            ("''", "Command is required"),
            ("amp\n--x", "Command can't hold control characters"),
        ] {
            assert_eq!(check_command(typed), Err(problem.into()), "{typed:?}");
            assert_eq!(
                custom(typed).problem(),
                Some((Field::Command, problem.into())),
                "{typed:?}"
            );
        }
        // The command comes first: its row is above the directory's.
        let request = Request {
            dir: "rel".into(),
            ..custom("amp '")
        };
        assert_eq!(request.problem().map(|p| p.0), Some(Field::Command));
        assert_eq!(custom("amp").problem(), None);
    }

    #[test]
    fn a_command_with_a_problem_is_not_sent() {
        let machine = crate::config::Machine {
            name: "studio".into(),
            ssh: "me@studio".into(),
            corral: None,
            label: None,
            show: true,
        };
        // An ssh that is never run: the problem is found first.
        let link = Link::with(
            &machine,
            "/nonexistent/ssh".into(),
            Ok("/nonexistent/s".into()),
        );
        assert_eq!(
            start(&link, &custom(""), &colors()),
            Err(Failure {
                text: "Command is required".into(),
                field: Some(Field::Command),
            })
        );
    }

    #[test]
    fn a_custom_command_goes_by_its_first_words_last_part() {
        for (typed, name) in [
            ("~/bin/my-agent --fast", "my-agent"),
            ("amp", "amp"),
            ("/usr/local/bin/amp --x", "amp"),
            ("'my agent' --x", "my-agent"),
            ("./run.sh", "run.sh"),
            ("tools/", "tools"),
        ] {
            assert_eq!(custom_name(typed).as_deref(), Some(name), "{typed}");
        }
        assert_eq!(custom_name(""), None);
        assert_eq!(custom_name("a 'b"), None);
        // An empty name takes it, or the kind, after the directory.
        assert_eq!(custom("~/bin/my-agent --fast").name(), "home/my-agent");
        assert_eq!(
            Request {
                dir: "~/Develop/ranch".into(),
                ..kind(Tool::Pi, "pi --model x")
            }
            .name(),
            "ranch/pi"
        );
        // The kind's icon when it is a kind's name; none of its own otherwise.
        let kept = |command: &str| Custom {
            name: custom_name(command).unwrap(),
            command: command.into(),
        };
        assert_eq!(kept("amp --x").kind(), Some(Tool::Amp));
        assert_eq!(kept("~/bin/claude").kind(), Some(Tool::Claude));
        assert_eq!(kept("~/bin/my-agent").kind(), None);
    }

    #[test]
    fn a_kinds_command_is_edited_only_when_its_words_differ() {
        assert!(!edited("pi", Tool::Pi));
        assert!(!edited("  pi  ", Tool::Pi));
        assert!(!edited("codex   '--yolo'", Tool::Codex));
        assert!(edited("codex", Tool::Codex));
        assert!(edited("pi --model sonnet", Tool::Pi));
        assert!(edited("", Tool::Pi));
        assert!(edited("pi '", Tool::Pi));
    }

    #[test]
    fn what_started_is_kept_for_that_machine_and_a_kinds_own_again_forgets_the_edit() {
        let mut kept = Commands::default();
        assert_eq!(kept.command("5090", Tool::Pi), "pi");
        assert_eq!(kept.command("5090", Tool::Codex), "codex --yolo");
        kept.remember("5090", &kind(Tool::Pi, " pi --model sonnet "), &[]);
        assert_eq!(kept.command("5090", Tool::Pi), "pi --model sonnet");
        // Each machine apart, each kind apart.
        assert_eq!(kept.command("studio", Tool::Pi), "pi");
        assert_eq!(kept.command("5090", Tool::Omp), "omp");
        // Started with the kind's own again (Reset): the edit is forgotten.
        kept.remember("5090", &kind(Tool::Pi, "pi"), &[]);
        assert_eq!(kept.command("5090", Tool::Pi), "pi");
        assert_eq!(kept, Commands::default());
    }

    #[test]
    fn the_users_own_become_capsules_a_later_one_by_the_same_name_in_its_place() {
        let mut kept = Commands::default();
        kept.remember("5090", &custom("~/bin/my-agent --fast"), &[]);
        kept.remember("5090", &custom("amp"), &[]);
        kept.remember("studio", &custom("goose"), &[]);
        let names = |kept: &Commands, machine: &str| -> Vec<String> {
            kept.customs(machine).into_iter().map(|c| c.name).collect()
        };
        assert_eq!(names(&kept, "5090"), ["my-agent", "amp"]);
        assert_eq!(names(&kept, "studio"), ["goose"]);
        kept.remember("5090", &custom("/opt/my-agent --slow"), &[]);
        assert_eq!(
            kept.customs("5090"),
            [
                Custom {
                    name: "my-agent".into(),
                    command: "/opt/my-agent --slow".into(),
                },
                Custom {
                    name: "amp".into(),
                    command: "amp".into(),
                },
            ]
        );
        // Removed from that machine alone, at once.
        kept.forget("5090", "my-agent");
        assert_eq!(names(&kept, "5090"), ["amp"]);
        assert_eq!(names(&kept, "studio"), ["goose"]);
        kept.forget("5090", "amp");
        kept.forget("5090", "nothing");
        assert!(kept.customs("5090").is_empty());
        // A kind's edit and the user's own sit side by side.
        kept.remember("studio", &kind(Tool::Omp, "omp --x"), &[]);
        assert_eq!(names(&kept, "studio"), ["goose"]);
        assert_eq!(kept.command("studio", Tool::Omp), "omp --x");
    }

    #[test]
    fn a_custom_named_as_a_kind_found_there_is_kept_as_that_kinds_edit() {
        let found = [Tool::Claude, Tool::Pi];
        let mut kept = Commands::default();
        kept.remember("5090", &custom("pi --model sonnet"), &found);
        assert_eq!(kept.command("5090", Tool::Pi), "pi --model sonnet");
        assert!(kept.customs("5090").is_empty());
        // A full path is the same kind by name.
        kept.remember("5090", &custom("/opt/bin/claude --x"), &found);
        assert_eq!(kept.command("5090", Tool::Claude), "/opt/bin/claude --x");
        assert!(kept.customs("5090").is_empty());
        // Its own command again: the edit is forgotten.
        kept.remember("5090", &custom(" pi "), &found);
        assert_eq!(kept.command("5090", Tool::Pi), "pi");
        // Not found this time: a capsule of its own; on another machine, the same.
        kept.remember("5090", &custom("~/bin/omp --x"), &found);
        kept.remember("studio", &custom("~/x/pi"), &[]);
        assert_eq!(kept.command("5090", Tool::Omp), "omp");
        assert_eq!(kept.customs("5090").len(), 1);
        assert_eq!(kept.command("studio", Tool::Pi), "pi");
        assert_eq!(kept.customs("studio")[0].name, "pi");
    }

    #[test]
    fn a_capsule_named_as_a_kind_found_there_is_not_drawn_beside_it() {
        let mut kept = Commands::default();
        kept.remember("5090", &custom("~/x/pi --fast"), &[]);
        kept.remember("5090", &custom("~/bin/my-agent"), &[]);
        let shown = |found: &[Tool]| -> Vec<String> {
            kept.shown("5090", found)
                .into_iter()
                .map(|c| c.name)
                .collect()
        };
        assert_eq!(shown(&[Tool::Pi, Tool::Omp]), ["my-agent"]);
        // Kept all the same: when pi is not found it is back.
        assert_eq!(shown(&[Tool::Omp]), ["pi", "my-agent"]);
        assert_eq!(shown(&[]), ["pi", "my-agent"]);
        assert_eq!(kept.customs("5090").len(), 2);
    }

    #[test]
    fn the_commands_are_kept_in_a_file_of_their_own_and_one_that_cannot_be_read_is_none() {
        let dir = std::env::temp_dir().join(format!("paddock-commands-{}", std::process::id()));
        let path = dir.join("remote-agent-commands.json");
        assert_eq!(Commands::load(&path), Commands::default());
        let mut kept = Commands::default();
        kept.remember("5090", &kind(Tool::Pi, "pi --model sonnet"), &[]);
        kept.remember("5090", &custom("~/bin/my-agent --fast"), &[]);
        kept.save(&path).unwrap();
        assert_eq!(Commands::load(&path), kept);
        // Beside the recent directories, and neither touches the other.
        let recent_file = dir.join("recent-remote-dirs.json");
        let mut recent = RecentDirs::default();
        recent.remember("5090", "~/x");
        recent.save(&recent_file).unwrap();
        assert_eq!(Commands::load(&path), kept);
        assert_eq!(RecentDirs::load(&recent_file), recent);
        assert_eq!(
            commands_path().and_then(|p| p.parent().map(ToOwned::to_owned)),
            recent_path().and_then(|p| p.parent().map(ToOwned::to_owned))
        );
        assert_ne!(commands_path(), recent_path());
        // Broken, or not what is kept: none.
        for broken in ["{", "[1, 2]", r#"{"5090": {"custom": "not a list"}}"#, ""] {
            std::fs::write(&path, broken).unwrap();
            assert_eq!(Commands::load(&path), Commands::default(), "{broken}");
        }
        // Written by hand: what cannot be read is passed over, and of two by one name the later.
        std::fs::write(
            &path,
            r#"{"5090": {"custom": ["a 'b", "x --one", "amp", "/bin/x --two"]}}"#,
        )
        .unwrap();
        let customs = Commands::load(&path).customs("5090");
        let commands: Vec<&str> = customs.iter().map(|c| c.command.as_str()).collect();
        assert_eq!(commands, ["/bin/x --two", "amp"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
