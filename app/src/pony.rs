//! The `pony` command (DESIGN §13 P5-147): ranch's chat bot (R38), read through its public
//! commands only, `pony --version` and `pony status`, as ranch's `docs/CONTRACT.md` sets them
//! out. `status` prints one line of JSON from this Mac's files alone, in a few milliseconds, so it
//! may be read every few seconds. Unknown fields are left alone and unknown values shown as they
//! are; the chat apps come from the list, none is named here but to word it. Nothing here reads
//! or writes `~/.pony`, and no command that changes anything is run.
//!
//! pony is looked for on PATH, then in `~/.local/bin` (as corral is). Not in either, or its
//! `contract` not `"1"`: not installed. Every call blocks, so call them off the UI thread.
use crate::fence::Refusal;
use serde::Deserialize;
use serde_json::Value;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// The command's name.
pub const PROGRAM: &str = "pony";
/// The contract paddock reads.
pub const CONTRACT: &str = "1";
/// How long one call may take: `status` reads a few small files.
const TIMEOUT: Duration = Duration::from_secs(5);

/// `pony --version`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Version {
    pub version: String,
    pub contract: String,
}

/// Why pony last stopped.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Exit {
    /// When, in Unix seconds.
    pub at: Option<f64>,
    pub code: Option<String>,
    pub message: Option<String>,
}

/// One channel a chat app is bound to.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Channel {
    pub id: String,
    /// Known once pony has connected and looked it up.
    pub name: Option<String>,
    pub private: Option<bool>,
    pub repo: String,
    /// `not_member`, `not_found`, `archived`, or another pony names.
    pub problem: Option<String>,
}

impl Channel {
    /// `#name`, or the ID until pony has looked it up.
    pub fn shown(&self) -> String {
        match &self.name {
            Some(name) => format!("#{name}"),
            None => self.id.clone(),
        }
    }

    /// What is wrong with it, in words, and whether paddock knows the word (an unknown one is
    /// shown as its name). `bot` is the bot's name, when known.
    pub fn trouble(&self, bot: Option<&str>) -> Option<(String, bool)> {
        let problem = self.problem.as_deref()?;
        Some(match problem {
            "not_member" => (
                format!(
                    "Bot isn't in this channel. Invite {} there.",
                    bot.map_or("it".to_owned(), |bot| format!("@{bot}"))
                ),
                true,
            ),
            "not_found" => ("Slack can't find this channel.".to_owned(), true),
            "archived" => ("This channel is archived.".to_owned(), true),
            other => (other.to_owned(), false),
        })
    }
}

/// One chat app pony is set up for.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Chat {
    /// `slack`, or another adapter.
    pub kind: String,
    /// `connected`, `connecting`, `failed`, `off`, or another pony names.
    pub connection: String,
    /// Since when, in Unix seconds.
    pub since: Option<f64>,
    /// Why it failed: a code and pony's words.
    pub reason: Option<String>,
    pub message: Option<String>,
    pub workspace: Option<String>,
    pub bot: Option<String>,
    /// Other connections Slack sees for the bot: another machine may be running pony.
    pub other_connections: u64,
    pub channels: Vec<Channel>,
}

/// How a state is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Good,
    Busy,
    Bad,
    /// Grey: off, not set up, or a word paddock doesn't know.
    Quiet,
}

impl Chat {
    /// The card's title: `Slack`, or the kind as pony names it.
    pub fn title(&self) -> String {
        match self.kind.as_str() {
            "slack" => "Slack".to_owned(),
            other => other.to_owned(),
        }
    }

    /// The connection in words and its colour; a state paddock doesn't know as pony names it.
    pub fn state(&self) -> (String, Tone) {
        match self.connection.as_str() {
            "connected" => ("Connected".into(), Tone::Good),
            "connecting" => ("Connecting…".into(), Tone::Busy),
            "failed" => ("Can't connect".into(), Tone::Bad),
            "off" => ("Off".into(), Tone::Quiet),
            other => (other.to_owned(), Tone::Quiet),
        }
    }

    /// The bot's name, as `@` would call it.
    pub fn bot_name(&self) -> &str {
        self.bot.as_deref().unwrap_or(PROGRAM)
    }

    /// The warning when Slack sees other connections for the bot.
    pub fn others(&self) -> Option<String> {
        match self.other_connections {
            0 => None,
            n => Some(format!(
                "{} sees {n} other connection{} for this bot. Another machine may be running \
                 pony too, and some messages may go there.",
                self.title(),
                if n == 1 { "" } else { "s" }
            )),
        }
    }
}

/// `pony status`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Status {
    pub version: String,
    /// `ok`, `missing`, `invalid`, or another pony names.
    pub config: String,
    pub config_message: Option<String>,
    /// `installed`, `not_installed`, `unsupported`, or another pony names.
    pub service: String,
    pub running: bool,
    pub pid: Option<u64>,
    pub started: Option<f64>,
    pub last_exit: Option<Exit>,
    pub chats: Vec<Chat>,
}

impl Status {
    /// Not set up on this Mac.
    pub fn missing(&self) -> bool {
        self.config == "missing"
    }

    /// The pill on top: not set up, running or not running, and its colour.
    pub fn state(&self) -> (&'static str, Tone) {
        if self.missing() {
            ("not set up", Tone::Quiet)
        } else if self.running {
            ("running", Tone::Good)
        } else {
            ("not running", Tone::Quiet)
        }
    }

    /// The background service in words.
    pub fn service_words(&self) -> String {
        match self.service.as_str() {
            "installed" => "background service installed".into(),
            "not_installed" => "background service not installed".into(),
            "unsupported" => "no background service on this system".into(),
            other => format!("background service: {other}"),
        }
    }
}

/// pony as found: not installed (or another contract), or its version and status.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    Missing,
    Here(Version, Result<Box<Status>, Refusal>),
}

/// Where pony is: on `path` (a `PATH`), then in `home`'s `.local/bin`.
pub fn locate(path: Option<&OsStr>, home: Option<&Path>) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let runnable = |path: &Path| {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    path.into_iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(PROGRAM))
        .chain(home.map(|home| home.join(".local/bin").join(PROGRAM)))
        .find(|path| runnable(path))
}

/// pony's answer from `output`: its JSON when it said `ok:true` and exited 0; otherwise its
/// `error` and `message`, or what it wrote to stderr.
fn answer(output: &crate::command::Output) -> Result<Value, Refusal> {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let unreadable = || Refusal {
        code: String::new(),
        message: match (output.status.success(), stderr.is_empty()) {
            (_, false) => format!("pony's answer could not be read: {stderr}"),
            (true, true) => "pony's answer could not be read".to_owned(),
            (false, true) => format!("pony exited with {}", output.status),
        },
    };
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return Err(unreadable());
    };
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

/// Runs `program` with `args` and reads its answer as `T`.
fn call<T: serde::de::DeserializeOwned>(
    program: &str,
    args: &[&str],
    cancel: &AtomicBool,
) -> Result<T, Refusal> {
    let output =
        crate::command::run(program, args, None, TIMEOUT, cancel).map_err(|error| Refusal {
            code: String::new(),
            message: format!("pony did not run: {error:#}"),
        })?;
    serde_json::from_value(answer(&output)?).map_err(|error| Refusal {
        code: String::new(),
        message: format!("pony's answer could not be read: {error}"),
    })
}

/// pony's version, when it runs and keeps the contract paddock reads.
pub fn version(program: &str, cancel: &AtomicBool) -> Option<Version> {
    call::<Version>(program, &["--version"], cancel)
        .ok()
        .filter(|version| version.contract == CONTRACT)
}

/// `pony status`.
pub fn status(program: &str, cancel: &AtomicBool) -> Result<Status, Refusal> {
    call(program, &["status"], cancel)
}

/// Finds pony on `path` or in `home` and reads its status.
pub fn look_in(path: Option<&OsStr>, home: Option<&Path>, cancel: &AtomicBool) -> Found {
    let Some(program) = locate(path, home) else {
        return Found::Missing;
    };
    let program = program.display().to_string();
    match version(&program, cancel) {
        None => Found::Missing,
        Some(version) => Found::Here(version, status(&program, cancel).map(Box::new)),
    }
}

/// Finds pony on this process's PATH or in `~/.local/bin` and reads its status.
pub fn look(cancel: &AtomicBool) -> Found {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    look_in(std::env::var_os("PATH").as_deref(), home.as_deref(), cancel)
}

/// Now, in Unix seconds.
pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// A time in Unix seconds as the local clock shows it: `09:12` today, else `Oct 19 09:12`.
pub fn when(at: f64) -> String {
    use crate::activity::{MONTHS, date, day_of, local_offset, today};
    let at = at as i64;
    let offset = local_offset(at);
    let minutes = (at + offset).rem_euclid(86_400) / 60;
    let clock = format!("{:02}:{:02}", minutes / 60, minutes % 60);
    let day = day_of(at, offset);
    if day == today() {
        clock
    } else {
        let (_, month, of) = date(day);
        format!("{} {of} {clock}", MONTHS[month as usize - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Never cancelled.
    static NO: AtomicBool = AtomicBool::new(false);

    /// A fresh directory for a fake.
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "paddock-pony-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fake `pony` in `dir`: `script` is its shell body.
    fn fake_in(dir: &Path, script: &str) {
        let path = dir.join(PROGRAM);
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A fake `pony` that says contract 1 for `--version` and prints `status` for `status`.
    fn fake(name: &str, status: &str) -> PathBuf {
        let dir = temp(name);
        fake_in(
            &dir,
            &format!(
                r#"case "$1" in
  --version) echo '{{"ok":true,"version":"0.1.0","contract":"1"}}';;
  status) {status};;
  *) echo '{{"ok":false,"error":"usage","message":"no"}}'; exit 2;;
esac"#
            ),
        );
        dir
    }

    /// Reads the fake in `dir` as if `dir` were all of PATH and there were no home.
    fn read(dir: &Path) -> Found {
        look_in(Some(dir.as_os_str()), None, &NO)
    }

    fn status_of(dir: &Path) -> Status {
        match read(dir) {
            Found::Here(version, Ok(status)) => {
                assert_eq!(version.version, "0.1.0");
                *status
            }
            other => panic!("{other:?}"),
        }
    }

    const EXAMPLE: &str = r#"{"ok":true,"contract":"1","version":"0.1.0","config":"ok","config_message":null,"service":"installed","running":true,"pid":41234,"started":1792550000.5,"last_exit":null,"chats":[{"kind":"slack","connection":"connected","since":1792550003.1,"reason":null,"message":null,"workspace":"firegnu","bot":"pony","other_connections":0,"channels":[{"id":"C0AAAAAAAAA","name":"paddock","private":true,"repo":"paddock","problem":null},{"id":"C0BBBBBBBBB","name":null,"private":null,"repo":"ranch","problem":"not_member"}]}]}"#;

    #[test]
    fn the_contracts_example_is_read_as_it_is_written() {
        let dir = fake("example", &format!("echo '{EXAMPLE}'"));
        let status = status_of(&dir);
        assert_eq!(status.state(), ("running", Tone::Good));
        assert_eq!(status.pid, Some(41234));
        assert_eq!(status.started, Some(1792550000.5));
        assert_eq!(status.service_words(), "background service installed");
        let chat = &status.chats[0];
        assert_eq!(chat.title(), "Slack");
        assert_eq!(chat.state(), ("Connected".to_owned(), Tone::Good));
        assert_eq!(chat.workspace.as_deref(), Some("firegnu"));
        assert_eq!(chat.others(), None);
        let [paddock, ranch] = &chat.channels[..] else {
            panic!("{:?}", chat.channels)
        };
        assert_eq!(paddock.shown(), "#paddock");
        assert_eq!(paddock.private, Some(true));
        assert_eq!(paddock.trouble(chat.bot.as_deref()), None);
        // Not looked up yet: its ID; and the bot isn't in it.
        assert_eq!(ranch.shown(), "C0BBBBBBBBB");
        assert_eq!(ranch.repo, "ranch");
        assert_eq!(
            ranch.trouble(chat.bot.as_deref()),
            Some((
                "Bot isn't in this channel. Invite @pony there.".into(),
                true
            ))
        );
    }

    #[test]
    fn not_there_or_another_contract_is_not_installed() {
        // Neither on PATH nor in ~/.local/bin.
        let empty = temp("empty");
        assert_eq!(
            look_in(Some(empty.as_os_str()), Some(&empty), &NO),
            Found::Missing
        );
        assert_eq!(look_in(None, None, &NO), Found::Missing);
        // A pony that speaks another contract.
        let other = temp("contract");
        fake_in(
            &other,
            r#"echo '{"ok":true,"version":"9.0.0","contract":"2"}'"#,
        );
        assert_eq!(read(&other), Found::Missing);
        // One whose --version isn't JSON.
        let broken = temp("broken-version");
        fake_in(&broken, "echo pony 0.1.0");
        assert_eq!(read(&broken), Found::Missing);
    }

    #[test]
    fn pony_in_local_bin_is_found_when_path_has_none() {
        let home = temp("home");
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).unwrap();
        fake_in(
            &bin,
            r#"case "$1" in
  --version) echo '{"ok":true,"version":"0.2.0","contract":"1"}';;
  status) echo '{"ok":true,"contract":"1","version":"0.2.0","config":"missing","config_message":null,"service":"not_installed","running":false,"last_exit":null,"chats":[]}';;
esac"#,
        );
        let empty = temp("no-pony-on-path");
        assert_eq!(
            locate(Some(empty.as_os_str()), Some(&home)),
            Some(bin.join(PROGRAM))
        );
        // PATH comes first.
        let first = fake("first", "exit 0");
        assert_eq!(
            locate(Some(first.as_os_str()), Some(&home)),
            Some(first.join(PROGRAM))
        );
        match look_in(Some(empty.as_os_str()), Some(&home), &NO) {
            Found::Here(version, Ok(status)) => {
                assert_eq!(version.version, "0.2.0");
                assert!(status.missing());
                assert_eq!(status.state(), ("not set up", Tone::Quiet));
                assert!(status.chats.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn config_missing_is_not_set_up() {
        let dir = fake(
            "missing",
            r#"echo '{"ok":true,"contract":"1","version":"0.1.0","config":"missing","config_message":"no ~/.pony/config.toml","service":"not_installed","running":false,"last_exit":null,"chats":[]}'"#,
        );
        let status = status_of(&dir);
        assert!(status.missing());
        assert_eq!(status.state(), ("not set up", Tone::Quiet));
        assert!(status.chats.is_empty());
    }

    #[test]
    fn not_running_gives_why_it_stopped_and_the_service() {
        let dir = fake(
            "stopped",
            r#"echo '{"ok":true,"contract":"1","version":"0.1.0","config":"ok","config_message":null,"service":"not_installed","running":false,"last_exit":{"at":1792551000,"code":"secrets_too_open","message":"~/.pony/secrets.toml can be read by others: chmod 600 it"},"chats":[{"kind":"slack","connection":"off","since":1792551000,"reason":null,"message":null,"workspace":null,"bot":null,"other_connections":0,"channels":[]}]}'"#,
        );
        let status = status_of(&dir);
        assert_eq!(status.state(), ("not running", Tone::Quiet));
        assert_eq!(status.pid, None);
        let exit = status.last_exit.as_ref().unwrap();
        assert_eq!(exit.at, Some(1792551000.0));
        assert_eq!(
            exit.message.as_deref(),
            Some("~/.pony/secrets.toml can be read by others: chmod 600 it")
        );
        assert_eq!(status.service_words(), "background service not installed");
        assert_eq!(status.chats[0].state(), ("Off".to_owned(), Tone::Quiet));
        // On another system there is no service; a word paddock doesn't know is shown.
        let other = Status {
            service: "unsupported".into(),
            ..Status::default()
        };
        assert_eq!(
            other.service_words(),
            "no background service on this system"
        );
        let other = Status {
            service: "paused".into(),
            ..Status::default()
        };
        assert_eq!(other.service_words(), "background service: paused");
    }

    #[test]
    fn a_failed_connection_gives_its_code_and_message() {
        let dir = fake(
            "failed",
            r#"echo '{"ok":true,"contract":"1","version":"0.1.0","config":"invalid","config_message":"~/.pony/config.toml line 4: channel ID missing","service":"installed","running":true,"pid":7,"started":1792550000,"last_exit":null,"chats":[{"kind":"slack","connection":"failed","since":1792550100,"reason":"auth_failed","message":"invalid_auth: the bot token was revoked","workspace":"firegnu","bot":"pony","other_connections":1,"channels":[{"id":"C1","name":"paddock","private":false,"repo":"paddock","problem":"not_found"},{"id":"C2","name":"old","private":false,"repo":"cairn","problem":"archived"}]}]}'"#,
        );
        let status = status_of(&dir);
        assert_eq!(status.config, "invalid");
        assert_eq!(
            status.config_message.as_deref(),
            Some("~/.pony/config.toml line 4: channel ID missing")
        );
        let chat = &status.chats[0];
        assert_eq!(chat.state(), ("Can't connect".to_owned(), Tone::Bad));
        assert_eq!(chat.reason.as_deref(), Some("auth_failed"));
        assert_eq!(
            chat.message.as_deref(),
            Some("invalid_auth: the bot token was revoked")
        );
        assert_eq!(
            chat.others().as_deref(),
            Some(
                "Slack sees 1 other connection for this bot. Another machine may be running \
                 pony too, and some messages may go there."
            )
        );
        let troubles: Vec<_> = chat
            .channels
            .iter()
            .map(|channel| channel.trouble(chat.bot.as_deref()))
            .collect();
        assert_eq!(
            troubles,
            [
                Some(("Slack can't find this channel.".to_owned(), true)),
                Some(("This channel is archived.".to_owned(), true)),
            ]
        );
    }

    #[test]
    fn unknown_values_and_fields_are_shown_as_they_come() {
        let dir = fake(
            "unknown",
            r#"echo '{"ok":true,"contract":"1","version":"0.3.0","config":"ok","config_message":null,"service":"installed","running":true,"pid":9,"started":1792550000,"last_exit":null,"new_field":{"x":1},"chats":[{"kind":"discord","connection":"paused","since":1792550000,"reason":null,"message":null,"workspace":null,"bot":null,"other_connections":2,"channels":[{"id":"D1","name":"ranch","private":null,"repo":"ranch","problem":"renamed","extra":true}],"later":[]},{"kind":"slack","connection":"failed","since":1792550000,"reason":"brand_new_code","message":"something new","workspace":null,"bot":null,"other_connections":0,"channels":[]}]}'"#,
        );
        let status = status_of(&dir);
        let [discord, slack] = &status.chats[..] else {
            panic!("{:?}", status.chats)
        };
        assert_eq!(discord.title(), "discord");
        assert_eq!(discord.state(), ("paused".to_owned(), Tone::Quiet));
        assert_eq!(discord.bot_name(), "pony");
        assert!(
            discord
                .others()
                .unwrap()
                .starts_with("discord sees 2 other connections")
        );
        assert_eq!(
            discord.channels[0].trouble(None),
            Some(("renamed".to_owned(), false))
        );
        // A code paddock doesn't know is kept, with its message.
        assert_eq!(slack.reason.as_deref(), Some("brand_new_code"));
        assert_eq!(slack.message.as_deref(), Some("something new"));
        // Not a member, the bot's name not known yet.
        let channel = Channel {
            problem: Some("not_member".into()),
            ..Channel::default()
        };
        assert_eq!(
            channel.trouble(None).unwrap().0,
            "Bot isn't in this channel. Invite it there."
        );
    }

    #[test]
    fn a_status_that_fails_or_is_not_json_is_its_error() {
        let refused = fake(
            "refused",
            r#"echo '{"ok":false,"error":"usage","message":"unknown flag"}'; exit 2"#,
        );
        assert_eq!(
            read(&refused),
            Found::Here(
                Version {
                    version: "0.1.0".into(),
                    contract: "1".into()
                },
                Err(Refusal {
                    code: "usage".into(),
                    message: "unknown flag".into()
                })
            )
        );
        let garbage = fake("garbage", "echo 'not json'; echo 'lock is broken' >&2");
        let Found::Here(_, Err(refusal)) = read(&garbage) else {
            panic!()
        };
        assert_eq!(refusal.code, "");
        assert!(refusal.message.contains("lock is broken"), "{refusal}");
        let silent = fake("silent", "exit 3");
        let Found::Here(_, Err(refusal)) = read(&silent) else {
            panic!()
        };
        assert!(refusal.message.starts_with("pony exited with"), "{refusal}");
        // JSON of the wrong shape.
        let shape = fake("shape", r#"echo '{"ok":true,"running":"yes"}'"#);
        let Found::Here(_, Err(refusal)) = read(&shape) else {
            panic!()
        };
        assert!(refusal.message.contains("could not be read"), "{refusal}");
    }

    #[test]
    fn times_are_the_local_clock_with_the_day_when_not_today() {
        let now = now();
        let today = when(now);
        assert_eq!(today.len(), 5, "{today}");
        assert_eq!(&today[2..3], ":");
        let earlier = when(now - 3.0 * 86_400.0);
        assert!(
            crate::activity::MONTHS
                .iter()
                .any(|m| earlier.starts_with(m)),
            "{earlier}"
        );
    }
}
