//! Another machine's corral, over the system's `ssh` (DESIGN §13 P5-90). Signing in is left to the
//! user's own ssh setup: paddock reads nothing under `~/.ssh`, keeps no credentials and never
//! prompts (`BatchMode=yes`). Each machine has a connection of paddock's own that every call goes
//! through (`ControlMaster=auto` on a socket in a short private directory), so a listing costs two
//! round trips, not a handshake each. On connecting, paddock checks that the login shell there
//! hands words through as written, finds corral (where `ssh` runs commands, then ranch's
//! `~/.local/bin/corral`) and that it speaks contract 1. Nothing here runs on the UI thread.
use crate::corral::Agent;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

/// How often a machine's agents are read while it answers.
pub const EVERY: Duration = Duration::from_secs(2);
/// How often the poller looks at the clock: for a retry that is due, and for the Mac waking.
pub const TICK: Duration = Duration::from_secs(1);
/// What `ssh` exits with when it fails itself, rather than the command it ran.
pub const SSH_FAILED: u32 = 255;
/// The contract paddock speaks (`corral --version`).
pub const CONTRACT: &str = "1";
/// How long one call there may take.
const CALL: Duration = Duration::from_secs(20);
/// The retries after a failure: from the first wait, doubling, up to the last.
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LAST_RETRY: Duration = Duration::from_secs(60);
/// The longest a Unix socket's path may be on macOS, its closing nul included, and what OpenSSH
/// adds to the control path while it binds it.
const SOCKET_ROOM: usize = 104;
const SOCKET_SUFFIX: usize = 17;
/// Sent through the login shell on connecting; it must come back as it went.
const PROBE: &str = "it's $(echo no) `echo no` \"q\" * ~ {a,b} \\ $HOME\n第二行 ;|&";
/// Between the answers of one call that asks corral several things.
const SEPARATOR: &str = "\u{1e}";

/// A word for a POSIX shell, as it is: always in single quotes, a quote inside written `'\''`.
pub fn quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// A command line for the shell `ssh` runs commands in: each word quoted.
pub fn command_line<S: AsRef<str>>(words: &[S]) -> String {
    words
        .iter()
        .map(|word| quote(word.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why a machine cannot be read, as the switcher says it and what the user can do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// ssh could not sign in without asking.
    SignIn(String),
    /// The machine's host key is not known, or changed.
    HostKey(String),
    /// No connection, or no answer.
    Unreachable(String),
    /// Neither where `ssh` runs commands nor in `~/.local/bin`; or not where the config says.
    NoCorral(String),
    /// corral speaks another contract.
    Contract(String),
    /// The login shell there changed the test words: not a POSIX shell.
    Quoting,
    /// Anything else on this side: ssh missing, the control socket's directory.
    Local(String),
}

impl Problem {
    /// The switcher's few words.
    pub fn label(&self) -> String {
        match self {
            Problem::SignIn(_) => "Can't sign in".into(),
            Problem::HostKey(_) => "Host key unknown".into(),
            Problem::Unreachable(_) => "Can't connect".into(),
            Problem::NoCorral(_) => "corral not found".into(),
            Problem::Contract(contract) => format!("corral contract {contract}, needs {CONTRACT}"),
            Problem::Quoting => "Shell not supported".into(),
            Problem::Local(_) => "Can't start ssh".into(),
        }
    }

    /// What went wrong, and what to do about it, for the switcher's hover note.
    pub fn advice(&self, target: &str) -> String {
        match self {
            Problem::SignIn(why) => format!(
                "{why}\nRun ssh {target} once in Terminal; a key with a passphrase needs ssh-add first."
            ),
            Problem::HostKey(why) => {
                format!("{why}\nRun ssh {target} once in Terminal to check and accept its host key.")
            }
            Problem::Unreachable(why) => {
                format!("{why}\nTrying again by itself; click to try now.")
            }
            Problem::NoCorral(why) => format!(
                "{why}\nInstall corral there with ranch, or write its path as corral = \"…\" under this [[machines]] entry."
            ),
            Problem::Contract(contract) => format!(
                "corral there speaks contract {contract}; paddock needs {CONTRACT}. Upgrade corral on that machine."
            ),
            Problem::Quoting => "Its login shell does not keep quoted words as written; paddock needs sh, bash or zsh there.".into(),
            Problem::Local(why) => why.clone(),
        }
    }

    /// Worth retrying soon by itself: the network, not something the user must fix.
    pub fn transient(&self) -> bool {
        matches!(self, Problem::Unreachable(_))
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::SignIn(why)
            | Problem::HostKey(why)
            | Problem::Unreachable(why)
            | Problem::NoCorral(why)
            | Problem::Local(why) => f.write_str(why),
            Problem::Contract(_) | Problem::Quoting => f.write_str(&self.label()),
        }
    }
}

/// What ssh's own failure (exit 255) was, from what it wrote.
pub fn classify(stderr: &str) -> Problem {
    let said = stderr
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("ssh failed")
        .to_owned();
    let has = |words: &str| stderr.contains(words);
    if has("Host key verification failed")
        || has("host key is known")
        || has("REMOTE HOST IDENTIFICATION HAS CHANGED")
    {
        Problem::HostKey(said)
    } else if has("Permission denied") || has("Too many authentication failures") {
        Problem::SignIn(said)
    } else {
        Problem::Unreachable(said)
    }
}

/// Where a machine stands, as the switcher shows it.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Not answered yet.
    Connecting,
    Online,
    /// Down, and when it is tried again.
    Down {
        problem: Problem,
        retry: Instant,
    },
}

/// What a machine's connection knows, for the switcher and Diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub status: Status,
    /// corral there, as found.
    pub corral: Option<String>,
    /// `corral --version`: the version and the contract.
    pub version: Option<(String, String)>,
    /// `$HOME` there, for writing `~` in its paths.
    pub home: Option<String>,
    /// The latest error, and when.
    pub error: Option<(SystemTime, String)>,
}

/// One machine: its ssh target and connection, and what is known of corral there.
pub struct Link {
    /// What paddock calls it.
    pub name: String,
    /// What `ssh` is given.
    pub target: String,
    /// corral's path as configured, `~` not yet replaced.
    configured: Option<String>,
    /// The `ssh` program; another only in tests.
    ssh: String,
    /// The control socket, or why there is none.
    socket: Result<PathBuf, String>,
    state: Mutex<State>,
}

struct State {
    snapshot: Snapshot,
    /// Failures in a row, for the retries' waits.
    failures: u32,
}

impl Link {
    /// A machine as configured, connecting through paddock's own socket in `/tmp`.
    pub fn new(machine: &crate::config::Machine) -> Self {
        let uid = unsafe { libc::getuid() };
        let socket = control_dir(Path::new("/tmp"), uid)
            .and_then(|dir| socket_path(&dir, &machine.name, &machine.ssh))
            .map_err(|error| format!("{error:#}"));
        Self::with(machine, "ssh".into(), socket)
    }

    /// With another `ssh` program and socket: for tests.
    pub fn with(
        machine: &crate::config::Machine,
        ssh: String,
        socket: Result<PathBuf, String>,
    ) -> Self {
        Self {
            name: machine.name.clone(),
            target: machine.ssh.clone(),
            configured: machine.corral.clone(),
            ssh,
            socket,
            state: Mutex::new(State {
                snapshot: Snapshot {
                    status: Status::Connecting,
                    corral: None,
                    version: None,
                    home: None,
                    error: None,
                },
                failures: 0,
            }),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap().snapshot.clone()
    }

    pub fn online(&self) -> bool {
        self.snapshot().status == Status::Online
    }

    /// The options every ssh here takes: paddock's own connection, never a prompt, and a dead
    /// connection noticed in about fifteen seconds. Before the target, which `--` sets apart.
    fn options(&self) -> Result<Vec<String>, Problem> {
        let socket = self
            .socket
            .as_ref()
            .map_err(|why| Problem::Local(why.clone()))?;
        let mut options = vec!["-S".to_owned(), socket.display().to_string()];
        for option in [
            "ControlMaster=auto",
            "ControlPersist=600",
            "BatchMode=yes",
            "ConnectTimeout=10",
            "ServerAliveInterval=5",
            "ServerAliveCountMax=3",
        ] {
            options.push("-o".into());
            options.push(option.into());
        }
        Ok(options)
    }

    /// `ssh`'s arguments to run `command` there: the options, `--`, the target, the command.
    pub fn args(&self, command: &str) -> Result<Vec<String>, Problem> {
        let mut args = self.options()?;
        args.extend(["--".to_owned(), self.target.clone(), command.to_owned()]);
        Ok(args)
    }

    /// The command line of a pane attached to `name` there: `ssh -tt -e none`, through the same
    /// connection, its escape character off so that nothing typed ends it.
    pub fn attach_argv(&self, name: &str) -> Result<Vec<String>> {
        let corral = self
            .snapshot()
            .corral
            .ok_or_else(|| anyhow!("{} is not connected", self.name))?;
        let mut argv = vec![self.ssh.clone(), "-tt".into(), "-e".into(), "none".into()];
        argv.extend(self.options().map_err(|problem| anyhow!("{problem}"))?);
        argv.extend([
            "--".to_owned(),
            self.target.clone(),
            command_line(&[corral.as_str(), "attach", name]),
        ]);
        Ok(argv)
    }

    /// Runs `command` there. ssh's own failures come back as the problem they are; the command's
    /// output, whatever its status, as it is.
    pub fn run(
        &self,
        command: &str,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<crate::command::Output, Problem> {
        let args = self.args(command)?;
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let output =
            crate::command::run(&self.ssh, &args, None, timeout, cancel).map_err(|error| {
                match error.chain().nth(1) {
                    // Could not start: ssh itself is missing.
                    Some(cause) if !cancel.load(Ordering::Relaxed) => {
                        Problem::Local(format!("ssh did not run: {cause}"))
                    }
                    _ => Problem::Unreachable(format!(
                        "no answer from {} in {}s",
                        self.name,
                        timeout.as_secs()
                    )),
                }
            })?;
        if output.status.code() == Some(SSH_FAILED as i32) {
            return Err(classify(&String::from_utf8_lossy(&output.stderr)));
        }
        Ok(output)
    }

    /// Runs corral there with `args`, once connected.
    pub fn corral(
        &self,
        args: &[&str],
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<crate::command::Output, Problem> {
        let corral = self
            .snapshot()
            .corral
            .ok_or_else(|| Problem::Unreachable(format!("{} is not connected yet", self.name)))?;
        let mut words = vec![corral.as_str()];
        words.extend_from_slice(args);
        self.run(&command_line(&words), timeout, cancel)
    }

    /// Connects: checks the login shell's quoting, finds corral and its contract. Online after.
    pub fn connect(&self, cancel: &AtomicBool) -> Result<(), Problem> {
        let said = self.run(&format!("printf '%s' {}", quote(PROBE)), CALL, cancel)?;
        if said.stdout != PROBE.as_bytes() {
            return Err(Problem::Quoting);
        }
        let found = self.run(
            "printf '%s\\n' \"$HOME\"; command -v corral 2>/dev/null; printf '\\n'; \
             if test -x \"$HOME/.local/bin/corral\"; then printf 'yes\\n'; fi",
            CALL,
            cancel,
        )?;
        let found = String::from_utf8_lossy(&found.stdout);
        let mut lines = found.lines();
        let home = lines.next().unwrap_or_default().to_owned();
        let on_path = lines.next().unwrap_or_default();
        let ranch = lines.any(|line| line == "yes");
        let corral = locate(self.configured.as_deref(), &home, on_path, ranch)?;
        let answer = self.run(&command_line(&[corral.as_str(), "--version"]), CALL, cancel)?;
        let version = version(&answer).map_err(|why| {
            if answer.status.code() == Some(127) {
                Problem::NoCorral(format!("{corral} does not run there"))
            } else {
                Problem::NoCorral(format!("{corral} --version: {why}"))
            }
        })?;
        if version.1 != CONTRACT {
            let mut state = self.state.lock().unwrap();
            state.snapshot.corral = Some(corral);
            state.snapshot.version = Some(version.clone());
            return Err(Problem::Contract(version.1));
        }
        let mut state = self.state.lock().unwrap();
        state.snapshot.corral = Some(corral);
        state.snapshot.version = Some(version);
        state.snapshot.home = (!home.is_empty()).then_some(home);
        Ok(())
    }

    /// It answered.
    pub fn up(&self) {
        let mut state = self.state.lock().unwrap();
        state.failures = 0;
        state.snapshot.status = Status::Online;
    }

    /// It failed: down until the retry, each wait twice the one before, up to a minute.
    pub fn down(&self, problem: Problem) {
        let mut state = self.state.lock().unwrap();
        let wait = retry_wait(state.failures);
        state.failures += 1;
        state.snapshot.error = Some((SystemTime::now(), problem.to_string()));
        state.snapshot.status = Status::Down {
            problem,
            retry: Instant::now() + wait,
        };
    }

    /// A read failed for corral's own reason; the connection stands.
    pub fn failed(&self, error: &str) {
        self.state.lock().unwrap().snapshot.error = Some((SystemTime::now(), error.to_owned()));
    }
}

/// How long to wait after `failures` failures in a row before trying again.
pub fn retry_wait(failures: u32) -> Duration {
    FIRST_RETRY
        .saturating_mul(2u32.saturating_pow(failures.min(16)))
        .min(LAST_RETRY)
}

/// corral's path there: as configured (`~` its `$HOME`), else where `ssh` runs commands finds it,
/// else ranch's place for it.
fn locate(
    configured: Option<&str>,
    home: &str,
    on_path: &str,
    ranch: bool,
) -> Result<String, Problem> {
    if let Some(configured) = configured {
        return Ok(match configured.strip_prefix('~') {
            Some(rest) if !home.is_empty() => format!("{home}{rest}"),
            Some(_) => {
                return Err(Problem::NoCorral(
                    "$HOME is not set there, for corral's ~".into(),
                ));
            }
            None => configured.to_owned(),
        });
    }
    if on_path.starts_with('/') {
        return Ok(on_path.to_owned());
    }
    if ranch && !home.is_empty() {
        return Ok(format!("{home}/.local/bin/corral"));
    }
    Err(Problem::NoCorral(
        "corral is neither on the PATH ssh runs commands with nor in ~/.local/bin".into(),
    ))
}

/// `corral --version`'s version and contract.
fn version(answer: &crate::command::Output) -> Result<(String, String), String> {
    let value: Value = serde_json::from_slice(&answer.stdout).map_err(|_| "not JSON".to_owned())?;
    let version = value["version"].as_str().unwrap_or("?").to_owned();
    let contract = value["contract"]
        .as_str()
        .ok_or_else(|| "no contract in the answer".to_owned())?
        .to_owned();
    Ok((version, contract))
}

/// paddock's private directory for control sockets under `base`: made `0700` when missing, else
/// used only when it is a real directory of this user's, which is then kept `0700`.
pub fn control_dir(base: &Path, uid: u32) -> Result<PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let dir = base.join(format!("paddock-{uid}"));
    if let Err(error) = std::fs::DirBuilder::new().mode(0o700).create(&dir)
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(anyhow!("can't make {}: {error}", dir.display()));
    }
    let meta = std::fs::symlink_metadata(&dir)?;
    anyhow::ensure!(
        meta.file_type().is_dir(),
        "{} is not a directory paddock made",
        dir.display()
    );
    anyhow::ensure!(
        meta.uid() == uid,
        "{} belongs to another user",
        dir.display()
    );
    if meta.mode() & 0o077 != 0 {
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

/// The control socket for a machine in `dir`: named by its name and target, so another target
/// never uses its connection, and short enough for OpenSSH to bind.
pub fn socket_path(dir: &Path, name: &str, target: &str) -> Result<PathBuf> {
    // FNV-1a: the same name in every run, without a dependency.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.bytes().chain([0]).chain(target.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let path = dir.join(format!("{hash:016x}"));
    let length = path.as_os_str().len();
    anyhow::ensure!(
        length + SOCKET_SUFFIX < SOCKET_ROOM,
        "the control socket's path {} is too long for macOS",
        path.display()
    );
    Ok(path)
}

/// What failed in a read: the connection, or corral.
pub enum Fault {
    Ssh(Problem),
    Corral(anyhow::Error),
}

impl From<Problem> for Fault {
    fn from(problem: Problem) -> Self {
        Fault::Ssh(problem)
    }
}

/// The agents there, read in two calls whatever their number: `corral ls`, then every agent's
/// `corral status` in one command, their answers apart by a line of their own.
pub fn collect(link: &Link, cancel: &AtomicBool) -> Result<Vec<Agent>, Fault> {
    #[derive(serde::Deserialize)]
    struct Listing {
        agents: Vec<Agent>,
    }
    let listed = link.corral(&["ls"], CALL, cancel)?;
    let value = crate::corral::answer(&["ls"], &listed).map_err(Fault::Corral)?;
    let mut agents = serde_json::from_value::<Listing>(value)
        .map_err(|error| Fault::Corral(error.into()))?
        .agents;
    let asked: Vec<usize> = (0..agents.len())
        .filter(|&i| !agents[i].starting && !agents[i].incompatible)
        .collect();
    if !asked.is_empty() {
        let corral = link.snapshot().corral.unwrap_or_default();
        let command = statuses(&corral, asked.iter().map(|&i| agents[i].name.as_str()));
        let output = link.run(&command, CALL, cancel)?;
        let answers = split(&output.stdout);
        for (at, &index) in asked.iter().enumerate() {
            let name = agents[index].name.clone();
            let status = match answers.get(at) {
                Some(text) => serde_json::from_str::<Value>(text)
                    .map_err(|_| anyhow!("status {name}: invalid JSON"))
                    .and_then(|value| crate::corral::checked(&["status", &name], true, value)),
                None => Err(anyhow!("status {name}: no answer")),
            };
            crate::corral::merge(&mut agents[index], status);
        }
    }
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(agents)
}

/// One command asking corral the status of each of `names`, each answer followed by a line
/// holding only [`SEPARATOR`].
pub fn statuses<'a>(corral: &str, names: impl Iterator<Item = &'a str>) -> String {
    names
        .map(|name| {
            format!(
                "{}; printf '\\n\\036\\n'",
                command_line(&[corral, "status", name])
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The answers of [`statuses`], in order.
pub fn split(output: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(output);
    let mut answers: Vec<String> = text
        .split(&format!("\n{SEPARATOR}\n"))
        .map(|answer| answer.trim().to_owned())
        .collect();
    // After the last separator.
    if answers.last().is_some_and(String::is_empty) {
        answers.pop();
    }
    answers
}

/// The poller's rounds for one machine: connects when it is not, reads the agents every
/// [`EVERY`] while it answers, and after a failure waits as [`retry_wait`] says, unless asked to
/// read now (`forced`) or the Mac has just woken. `None` when it is not time.
pub struct Rounds {
    link: std::sync::Arc<Link>,
    listed: Option<Instant>,
    clock: (Instant, SystemTime),
}

impl Rounds {
    pub fn new(link: std::sync::Arc<Link>) -> Self {
        Self {
            link,
            listed: None,
            clock: (Instant::now(), SystemTime::now()),
        }
    }

    pub fn round(&mut self, forced: bool, cancel: &AtomicBool) -> Option<Result<Vec<Agent>>> {
        let (now, wall) = (Instant::now(), SystemTime::now());
        // The wall clock went on while the Mac slept, and the other did not.
        let slept = wall
            .duration_since(self.clock.1)
            .unwrap_or_default()
            .saturating_sub(now.duration_since(self.clock.0))
            > Duration::from_secs(5);
        self.clock = (now, wall);
        let due = match self.link.snapshot().status {
            Status::Online => self.listed.is_none_or(|at| now.duration_since(at) >= EVERY),
            Status::Connecting => true,
            Status::Down { retry, .. } => now >= retry,
        };
        if !(forced || slept || due) {
            return None;
        }
        self.listed = Some(now);
        let link = &self.link;
        if !link.online()
            && let Err(problem) = link.connect(cancel)
        {
            link.down(problem.clone());
            return Some(Err(anyhow!("{}", problem.label())));
        }
        Some(match collect(link, cancel) {
            Ok(agents) => {
                link.up();
                Ok(agents)
            }
            Err(Fault::Ssh(problem)) => {
                link.down(problem.clone());
                Err(anyhow!("{}", problem.label()))
            }
            Err(Fault::Corral(error)) => {
                link.up();
                link.failed(&format!("{error:#}"));
                Err(error)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corral_is_looked_for_as_configured_then_on_the_path_then_where_ranch_puts_it() {
        let found = |configured, on_path, ranch| locate(configured, "/home/me", on_path, ranch);
        assert_eq!(
            found(Some("~/bin/corral"), "/usr/bin/corral", true),
            Ok("/home/me/bin/corral".into())
        );
        assert_eq!(
            found(Some("/opt/corral"), "", false),
            Ok("/opt/corral".into())
        );
        assert_eq!(
            found(None, "/usr/bin/corral", true),
            Ok("/usr/bin/corral".into())
        );
        // An alias or function is not a path.
        assert_eq!(
            found(None, "corral", true),
            Ok("/home/me/.local/bin/corral".into())
        );
        assert!(matches!(found(None, "", false), Err(Problem::NoCorral(_))));
        assert!(matches!(
            locate(Some("~/corral"), "", "", false),
            Err(Problem::NoCorral(_))
        ));
    }

    #[test]
    fn statuses_come_apart_in_order() {
        let command = statuses("/c", ["a", "it's"].into_iter());
        assert_eq!(
            command,
            "'/c' 'status' 'a'; printf '\\n\\036\\n'; '/c' 'status' 'it'\\''s'; printf '\\n\\036\\n'"
        );
        let answers = split(b"{\n \"x\": 1\n}\n\n\x1e\n{\"y\":2}\n\n\x1e\n");
        assert_eq!(answers, ["{\n \"x\": 1\n}", "{\"y\":2}"]);
        assert!(split(b"").is_empty());
    }

    #[test]
    fn retries_wait_twice_as_long_each_time_up_to_a_minute() {
        let waits: Vec<u64> = (0..9).map(|n| retry_wait(n).as_secs()).collect();
        assert_eq!(waits, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(retry_wait(u32::MAX), LAST_RETRY);
    }

    #[test]
    fn ssh_errors_are_told_apart_by_what_ssh_wrote() {
        assert!(matches!(
            classify("Warning: x\nme@h: Permission denied (publickey).\n"),
            Problem::SignIn(said) if said == "me@h: Permission denied (publickey)."
        ));
        assert!(matches!(
            classify(
                "No ED25519 host key is known for h and you have requested strict checking.\nHost key verification failed.\n"
            ),
            Problem::HostKey(_)
        ));
        assert!(matches!(
            classify("ssh: Could not resolve hostname h\n"),
            Problem::Unreachable(_)
        ));
        assert!(matches!(classify(""), Problem::Unreachable(said) if said == "ssh failed"));
    }
}
