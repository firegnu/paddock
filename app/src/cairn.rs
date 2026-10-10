//! The right sidebar's Cairn tab, what it knows: whether cairn is at work in the focused pane's
//! directory, and what the next session there would be given (DESIGN §13 P5-55). All of it comes
//! from cairn's own commands, `cairn status --json` and, in a repository that has adopted it,
//! `cairn show --json`, and from Git for the repository's name and branch; cairn's database,
//! settings and hook files are never read (DESIGN §3). One thing writes: `adopt`, which runs
//! `cairn adopt`, and the tab runs it only once the user has confirmed (`Adopting`).
use crate::{card::short_time, command, git};
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The command, as PATH finds it.
pub const PROGRAM: &str = "cairn";
const TIMEOUT: Duration = Duration::from_secs(5);

/// What a read of a directory found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// There is no `cairn` command.
    NotInstalled,
    /// A command failed, timed out, or said something the tab cannot read: the first line of why,
    /// after the command's name.
    Failed(String),
    Found(Found),
}

/// cairn as it stands in a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The directory read, where Adopt runs.
    pub cwd: String,
    /// The repository's name, or outside one the directory's.
    pub name: String,
    /// The branch out in the directory's worktree, when there is one.
    pub branch: Option<String>,
    pub adopted: bool,
    /// Whether each agent's hooks are installed.
    pub claude: bool,
    pub codex: bool,
    /// When each agent's hooks last ran here.
    pub claude_fired: Fired,
    pub codex_fired: Fired,
    /// The records agents have saved that cairn has not yet taken into its database.
    pub uncollected: u64,
    pub body: Body,
}

/// When an agent's hooks last ran in the repository read, as far as cairn tells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fired {
    /// Not told: cairn before 0.2.0, an agent without its hooks, or a project that is not
    /// adopted, where cairn records nothing.
    Unknown,
    /// cairn has recorded none.
    Never,
    /// How long before the read the latest one ran, as the cards write ages (`short_time`).
    Ago(String),
}

/// What the tab shows under its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// Hooks are installed for neither agent: nothing is recorded, adopted or not.
    NoHooks,
    /// Not adopted, in the repository whose main worktree this is: Adopt is offered.
    NotAdopted(PathBuf),
    /// Not adopted, and in no Git repository.
    Outside,
    /// Adopted, with nothing recorded yet.
    NoRecords,
    /// Adopted: what the next session would be given, from its first section on (`shown`).
    Records(String),
}

/// Why a cairn command gave nothing.
enum Trouble {
    /// There is no such command.
    Missing,
    Failed(String),
}

impl From<Trouble> for Read {
    fn from(trouble: Trouble) -> Self {
        match trouble {
            Trouble::Missing => Read::NotInstalled,
            Trouble::Failed(why) => Read::Failed(why),
        }
    }
}

/// The fields of `cairn status --json` the tab reads.
#[derive(Deserialize)]
struct Status {
    agents: Agents,
    project: Project,
    spool: Spool,
}

#[derive(Deserialize)]
struct Agents {
    claude: Agent,
    codex: Agent,
}

#[derive(Deserialize)]
struct Agent {
    installed: bool,
    /// From cairn 0.2.0: when each of the agent's hook events last ran in this project, null for
    /// one that has not.
    last_seen: Option<HashMap<String, Option<String>>>,
}

#[derive(Deserialize)]
struct Project {
    status: Adoption,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Adoption {
    Adopted,
    NotAdopted,
    /// cairn has no database yet, so nothing is adopted.
    NoData,
}

#[derive(Deserialize)]
struct Spool {
    pending_json: u64,
}

/// What `cairn show --json` prints: the text and its records, or with no database yet a status.
#[derive(Deserialize)]
struct Show {
    status: Option<String>,
    text: Option<String>,
    record_ids: Option<Vec<serde_json::Value>>,
}

/// Reads `cwd`: `cairn status --json`, and only where it says the project is adopted and some
/// agent has its hooks, `cairn show --json`, which takes saved records into cairn's database
/// before it answers.
pub fn read(program: &str, git: &str, cwd: &str, now: SystemTime, cancel: &AtomicBool) -> Read {
    let status = match run(program, &["status", "--json"], cwd, cancel) {
        Ok(out) => out,
        Err(trouble) => return trouble.into(),
    };
    let Ok(status) = serde_json::from_slice::<Status>(&status) else {
        return Read::Failed("cairn status: unexpected output".into());
    };
    let adopted = matches!(status.project.status, Adoption::Adopted);
    let (claude, codex) = (
        status.agents.claude.installed,
        status.agents.codex.installed,
    );
    let (name, branch, repo) = match git::repository(git, cwd, cancel) {
        Some((top, common)) => {
            let repo = main_worktree(&top, &common);
            let branch = git::git(
                git,
                &top,
                &["symbolic-ref", "--quiet", "--short", "HEAD"],
                cancel,
            )
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
            .filter(|branch| !branch.is_empty());
            (last_part(&repo), branch, Some(repo))
        }
        None => (last_part(Path::new(cwd)), None, None),
    };
    let body = if !claude && !codex {
        Body::NoHooks
    } else if adopted {
        match show(program, cwd, cancel) {
            Ok(body) => body,
            Err(trouble) => return trouble.into(),
        }
    } else {
        repo.map_or(Body::Outside, Body::NotAdopted)
    };
    Read::Found(Found {
        cwd: cwd.to_owned(),
        name,
        branch,
        adopted,
        claude,
        codex,
        claude_fired: fired(&status.agents.claude, adopted, now),
        codex_fired: fired(&status.agents.codex, adopted, now),
        uncollected: status.spool.pending_json,
        body,
    })
}

/// When `agent`'s hooks last ran before `now`: the latest of its events.
fn fired(agent: &Agent, adopted: bool, now: SystemTime) -> Fired {
    let Some(seen) = agent
        .last_seen
        .as_ref()
        .filter(|_| adopted && agent.installed)
    else {
        return Fired::Unknown;
    };
    let mut latest = None;
    for at in seen.values().flatten() {
        let Some(at) = unix(at) else {
            return Fired::Unknown;
        };
        latest = latest.max(Some(at));
    }
    let Some(latest) = latest else {
        return Fired::Never;
    };
    let now = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    Fired::Ago(short_time(Some(now.saturating_sub(latest) as f64)))
}

/// The seconds since 1970 of a time as cairn writes them, RFC 3339 in UTC:
/// `2026-10-10T09:43:55.004Z`. The fraction of a second is dropped.
fn unix(text: &str) -> Option<u64> {
    let (date, time) = text.strip_suffix('Z')?.split_once('T')?;
    let three = |text: &str, by: char| {
        let mut parts = text.splitn(3, by).map(|part| part.parse::<u64>().ok());
        Some((parts.next()??, parts.next()??, parts.next()??))
    };
    let (year, month, day) = three(date, '-')?;
    let (hour, minute, second) = three(time.split('.').next()?, ':')?;
    if year < 1970
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Days since 1970-01-01, counting years from March so a leap day ends one.
    let year = if month <= 2 { year - 1 } else { year };
    let (era, of_era) = (year / 400, year % 400);
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// What `cairn show --json` in `cwd` has for an adopted project.
fn show(program: &str, cwd: &str, cancel: &AtomicBool) -> Result<Body, Trouble> {
    let out = run(program, &["show", "--json"], cwd, cancel)?;
    match serde_json::from_slice::<Show>(&out) {
        Ok(Show {
            status: Some(status),
            ..
        }) if status == "no_data" => Ok(Body::NoRecords),
        Ok(Show {
            text: Some(text),
            record_ids: Some(ids),
            ..
        }) => Ok(if ids.is_empty() {
            Body::NoRecords
        } else {
            Body::Records(shown(&text).to_owned())
        }),
        _ => Err(Trouble::Failed("cairn show: unexpected output".into())),
    }
}

/// What the tab shows of `text`, what `cairn show` gives a session: from its first section, a line
/// starting `### `, leaving out the rules written for the agent before it; all of it when it has
/// no such line.
pub fn shown(text: &str) -> &str {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("### ") {
            return &text[at..];
        }
        at += line.len();
    }
    text
}

/// Adopts the repository `cwd` is in, with `cairn adopt` run there; or the first line of why not.
pub fn adopt(program: &str, cwd: &str, cancel: &AtomicBool) -> Result<(), String> {
    match run(program, &["adopt"], cwd, cancel) {
        Ok(_) => Ok(()),
        Err(Trouble::Missing) => Err("cairn adopt: cairn is not installed".into()),
        Err(Trouble::Failed(why)) => Err(why),
    }
}

/// Runs `cairn` with `args` in `cwd`: what it printed, or why not, as `cairn <command>: ` and the
/// first line of what it said.
fn run(program: &str, args: &[&str], cwd: &str, cancel: &AtomicBool) -> Result<Vec<u8>, Trouble> {
    let failed = |why: &str| Trouble::Failed(format!("cairn {}: {why}", args[0]));
    let started = Instant::now();
    match command::run(program, args, Some(Path::new(cwd)), TIMEOUT, cancel) {
        Ok(out) if out.status.success() => Ok(out.stdout),
        Ok(out) => Err(failed(
            &first_line(&out.stderr)
                .or_else(|| first_line(&out.stdout))
                .unwrap_or_else(|| out.status.to_string()),
        )),
        Err(_) if started.elapsed() >= TIMEOUT => Err(failed("timed out")),
        Err(error) => {
            // A directory that is gone fails to start a command the same way a missing one does.
            let missing = error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == ErrorKind::NotFound);
            if missing && Path::new(cwd).is_dir() {
                Err(Trouble::Missing)
            } else {
                Err(failed(&format!("{error:#}")))
            }
        }
    }
}

/// The first line of `bytes` with anything on it.
fn first_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

/// The main worktree of the repository whose worktree `top` is, which every worktree of it is
/// named by: beside `common`, the directory they share, or where that is not a `.git` (a bare
/// repository, a submodule) `top` itself.
fn main_worktree(top: &Path, common: &Path) -> PathBuf {
    match common.parent() {
        Some(parent) if common.file_name().is_some_and(|name| name == ".git") => parent.to_owned(),
        _ => top.to_owned(),
    }
}

/// A directory's own name.
fn last_part(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// `path` with the home directory written `~`.
pub fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Where Adopt stands, and for which repository: nothing is run until the question is answered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Adopting {
    #[default]
    Idle,
    /// The confirmation card is up.
    Asking(PathBuf),
    Running(PathBuf),
    /// Adopted: until cairn is read again, which then shows it.
    Done(PathBuf),
    /// `cairn adopt` failed, and the first line of why: the card stays, to try again or cancel.
    Refused(PathBuf, String),
}

impl Adopting {
    /// Adopt… on `repo`: the card comes up, unless one is up or it is running.
    pub fn ask(&mut self, repo: &Path) {
        if *self == Adopting::Idle {
            *self = Adopting::Asking(repo.to_owned());
        }
    }

    /// Cancel: the card closes, unless it is running. Whether it did.
    pub fn cancel(&mut self) -> bool {
        let up = matches!(self, Adopting::Asking(_) | Adopting::Refused(..));
        if up {
            *self = Adopting::Idle;
        }
        up
    }

    /// Adopt on the card: whether to run `cairn adopt` now, which it says once for each asking.
    pub fn confirm(&mut self) -> bool {
        match std::mem::take(self) {
            Adopting::Asking(repo) | Adopting::Refused(repo, _) => {
                *self = Adopting::Running(repo);
                true
            }
            other => {
                *self = other;
                false
            }
        }
    }

    /// `cairn adopt` for `repo` came back: done, or why not, when it is still what runs.
    pub fn finish(&mut self, repo: &Path, result: Result<(), String>) {
        if !matches!(self, Adopting::Running(running) if running == repo) {
            return;
        }
        *self = match result {
            Ok(()) => Adopting::Done(repo.to_owned()),
            Err(why) => Adopting::Refused(repo.to_owned(), why),
        };
    }

    /// cairn was read again, and the tab now shows `repo` as not adopted, or something else
    /// (`None`): a card for another repository closes, one running for it is no longer waited
    /// for, and one that is done has been read past. Whether it changed.
    pub fn follow(&mut self, repo: Option<&Path>) -> bool {
        let of = match self {
            Adopting::Idle => return false,
            Adopting::Done(_) => None,
            Adopting::Asking(of) | Adopting::Running(of) | Adopting::Refused(of, _) => Some(of),
        };
        if of.is_some_and(|of| repo == Some(of.as_path())) {
            return false;
        }
        *self = Adopting::Idle;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_under_home_is_written_with_a_tilde() {
        let home = Some(Path::new("/Users/me"));
        let tilde = |path: &str| tilde(Path::new(path), home);
        assert_eq!(tilde("/Users/me/Developer/ranch"), "~/Developer/ranch");
        assert_eq!(tilde("/Users/me"), "~");
        assert_eq!(tilde("/Users/mel/ranch"), "/Users/mel/ranch");
        assert_eq!(tilde("/opt/ranch"), "/opt/ranch");
        assert_eq!(super::tilde(Path::new("/opt/ranch"), None), "/opt/ranch");
    }

    #[test]
    fn a_worktree_is_named_by_its_repositorys_main_one() {
        let main = |top: &str, common: &str| main_worktree(Path::new(top), Path::new(common));
        assert_eq!(main("/r/ranch", "/r/ranch/.git"), Path::new("/r/ranch"));
        assert_eq!(main("/w/ranch-fix", "/r/ranch/.git"), Path::new("/r/ranch"));
        // A bare repository's worktree, and a submodule, have no main worktree beside them.
        assert_eq!(main("/w/fix", "/r/ranch.git"), Path::new("/w/fix"));
        assert_eq!(
            main("/r/ranch/sub", "/r/ranch/.git/modules/sub"),
            Path::new("/r/ranch/sub")
        );
    }
}
