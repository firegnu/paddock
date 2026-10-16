//! The right sidebar's Cairn tab, what it knows: whether cairn is at work in the focused pane's
//! directory, the handover notes agents have saved there, what the next session would be given,
//! and the project's ledger (DESIGN §13 P5-55, P5-79, P5-110, P5-118). All of it comes from
//! cairn's own commands, `cairn status --json` and, in a repository that has adopted it,
//! `cairn show --json`, `cairn list --json` and `cairn ledger --json`, then
//! `cairn show <ID> --json` for a note that is opened, and from Git for
//! the repository's name and branch; cairn's database, settings and hook files are never read
//! (DESIGN §3). One thing writes: `adopt`, which runs `cairn adopt`, and the tab runs it only once
//! the user has confirmed (`Adopting`).
use crate::{
    activity::{self, MONTHS},
    card::short_time,
    command, git,
};
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
const DAY: i64 = 86_400;
/// How many records are asked for at first, and how many more with each Show older: one session
/// can have saved dozens.
pub const PAGE: usize = 200;

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
    /// The repository's main worktree, in one: what its notes are kept by.
    pub repo: Option<PathBuf>,
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
    /// The project's ledger, read where the notes are: in an adopted repository with some agent's
    /// hooks.
    pub ledger: Option<Ledger>,
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
    /// Adopted, with something on record.
    Records(Records),
}

/// What an adopted repository has on record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Records {
    /// What the next session would be given, from its first section on (`shown`).
    pub text: String,
    /// The handover notes; `None` from a cairn before 0.3.0, which does not list them.
    pub notes: Option<Box<Notes>>,
}

/// The handover notes of a repository: its checkpoints, as `cairn list --json` gives them, one line
/// for each session that saved any (DESIGN §13 P5-110).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notes {
    /// The sessions the next session is given, the latest first.
    pub next: Vec<Session>,
    /// The other sessions, under the day of the note each is shown by, the latest first.
    pub days: Vec<Day>,
    /// cairn has more records than were asked for.
    pub older: bool,
}

/// The sessions shown by a note saved on one day of this machine's calendar, the latest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Day {
    /// `Today` or `Yesterday`, for those two.
    pub name: Option<&'static str>,
    /// `Sat, Oct 10`, and the year after it when that is not this one.
    pub date: String,
    pub sessions: Vec<Session>,
}

/// The notes one session saved: the one it is shown by, and the rest of those read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// The one the next session is given, or else its latest.
    pub note: Note,
    /// Its other notes, the newest first.
    pub earlier: Vec<Note>,
    /// It may have saved more than were read: it has the oldest note read, and cairn has older.
    pub more: bool,
}

/// One handover note in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    /// When it was saved, on this machine's clock: `17:44`.
    pub time: String,
    /// For one not saved today, which day: `Oct 9`, and the year after it when that is not this
    /// one.
    pub day: Option<String>,
    /// For one saved today, how long before the read: `2m ago`.
    pub ago: Option<String>,
    /// What cairn calls the agent that saved it: `claude`, `codex`, `local`, or one to come.
    pub agent: String,
    /// Who saved it: `Claude`, `Codex`, `Manual` for none of the agents, or what cairn calls it.
    pub by: String,
    pub branch: Option<String>,
    /// Its one line, as the agent wrote it; empty without one.
    pub summary: String,
    /// The next session here is given this one.
    pub next: bool,
}

/// One handover note in full, as `cairn show <ID> --json` gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Full {
    /// When it was saved, on this machine's clock: `Oct 10, 2026 at 17:44:51`.
    pub saved: String,
    /// Who saved it, as [`Note::by`].
    pub by: String,
    /// The first eight characters of the agent's session, for one saved in a session.
    pub session: Option<String>,
    pub branch: Option<String>,
    /// Its text, Markdown; `None` once deleted, or for a record that has none.
    pub body: Option<String>,
    /// The latest correction written for it: when that was saved, and its text.
    pub correction: Option<(String, Option<String>)>,
}

/// The project's ledger, what holds across sessions, as `cairn ledger --json` gives its entries
/// in force (cairn DESIGN §7.1, P5-118). Its trouble is its own: the notes show all the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ledger {
    /// A cairn before 0.5.0, which keeps none.
    Unsupported,
    /// `cairn ledger` failed, or said something the tab cannot read: the first line of why.
    Failed(String),
    /// The entries by kind, in cairn's order of kinds and those it may add after; none yet when
    /// empty.
    Groups(Vec<Group>),
}

/// The entries of one kind, by number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// `State`, `Open`, `Conventions`, or `Other` for kinds cairn adds later.
    pub name: &'static str,
    pub entries: Vec<Entry>,
}

/// One entry of the ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    /// Its number in the project, written `#no`.
    pub no: u64,
    pub title: String,
    /// Its text, Markdown; `None` without one.
    pub body: Option<String>,
    /// When it was added, on this machine's clock: `Oct 16, 2026 at 10:12:01`.
    pub added: String,
    /// When it was last changed, written as `added`, if ever since.
    pub updated: Option<String>,
    /// Who added it: `You`, `Claude`, `Codex`, or what cairn calls the agent.
    pub by: String,
    /// The first eight characters of the agent's session, for one an agent added.
    pub session: Option<String>,
    /// The next session here is given it.
    pub given: bool,
}

/// Why a cairn command gave nothing.
enum Trouble {
    /// There is no such command.
    Missing,
    Failed(String),
    /// It left with 2, as cairn does over arguments it does not know.
    Unknown(String),
}

impl From<Trouble> for Read {
    fn from(trouble: Trouble) -> Self {
        match trouble {
            Trouble::Missing => Read::NotInstalled,
            Trouble::Failed(why) | Trouble::Unknown(why) => Read::Failed(why),
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
    record_ids: Option<Vec<String>>,
    /// From cairn 0.5.0: the ledger's entries in the text.
    #[serde(default)]
    ledger_item_ids: Vec<String>,
}

/// What `cairn ledger --json` prints.
#[derive(Deserialize)]
struct LedgerOut {
    items: Vec<Item>,
}

/// An entry as `cairn ledger --json` gives one.
#[derive(Deserialize)]
struct Item {
    id: String,
    no: u64,
    kind: String,
    title: String,
    body: Option<String>,
    created_at: String,
    updated_at: String,
    created_by: String,
}

/// What `cairn list --json` prints.
#[derive(Deserialize)]
struct List {
    total: u64,
    records: Vec<Record>,
}

/// A record as `cairn list --json` and `cairn show <ID> --json` give one: the first with its
/// `summary`, the second with its `body` and `correction`.
#[derive(Deserialize)]
struct Record {
    id: String,
    created_at: String,
    agent: String,
    session_id: Option<String>,
    /// From `list`: the session that saved it, `<agent>:<session_id>`, or `local:<id>`.
    source_id: Option<String>,
    branch: Option<String>,
    kind: String,
    #[serde(default)]
    summary: String,
    body: Option<String>,
    correction: Option<Box<Record>>,
}

/// Reads `cwd`: `cairn status --json`, and only where it says the project is adopted and some
/// agent has its hooks, `cairn show --json`, which takes saved records into cairn's database
/// before it answers, then `cairn list --json` for the newest `limit` of them. Times are written
/// as of `now`, on the clock `offset` gives: how far east of UTC it is at a time.
pub fn read(
    program: &str,
    git: &str,
    cwd: &str,
    limit: usize,
    now: SystemTime,
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Read {
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
    let (body, ledger) = if !claude && !codex {
        (Body::NoHooks, None)
    } else if adopted {
        match records(program, cwd, limit, now, offset, cancel) {
            // With no database yet there is no ledger either.
            Ok((body, given)) => (
                body,
                Some(given.map_or(Ledger::Groups(Vec::new()), |given| {
                    self::ledger(program, cwd, &given, offset, cancel)
                })),
            ),
            Err(trouble) => return trouble.into(),
        }
    } else {
        (repo.clone().map_or(Body::Outside, Body::NotAdopted), None)
    };
    Read::Found(Found {
        cwd: cwd.to_owned(),
        name,
        repo,
        branch,
        adopted,
        claude,
        codex,
        claude_fired: fired(&status.agents.claude, adopted, now),
        codex_fired: fired(&status.agents.codex, adopted, now),
        uncollected: status.spool.pending_json,
        body,
        ledger,
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
    if !(1970..=9999).contains(&year)
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

/// What an adopted project in `cwd` has on record: `cairn show --json`, then the notes; beside
/// them the ledger's entries in the text the next session is given, or with no database yet
/// `None`.
fn records(
    program: &str,
    cwd: &str,
    limit: usize,
    now: SystemTime,
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Result<(Body, Option<Vec<String>>), Trouble> {
    let Some(Shown { text, next, given }) = show(program, cwd, cancel)? else {
        return Ok((Body::NoRecords, None));
    };
    let notes = notes(program, cwd, limit, &next, now, offset, cancel)?;
    let none = next.is_empty()
        && notes
            .as_ref()
            .is_none_or(|notes| notes.next.is_empty() && notes.days.is_empty());
    let body = if none {
        Body::NoRecords
    } else {
        Body::Records(Records {
            text: shown(&text).to_owned(),
            notes,
        })
    };
    Ok((body, Some(given)))
}

/// The text the next session would be given, and what is in it.
struct Shown {
    text: String,
    /// The records.
    next: Vec<String>,
    /// The ledger's entries.
    given: Vec<String>,
}

/// What `cairn show --json` in `cwd` has for an adopted project, or with no database yet nothing.
fn show(program: &str, cwd: &str, cancel: &AtomicBool) -> Result<Option<Shown>, Trouble> {
    let out = run(program, &["show", "--json"], cwd, cancel)?;
    match serde_json::from_slice::<Show>(&out) {
        Ok(Show {
            status: Some(status),
            ..
        }) if status == "no_data" => Ok(None),
        Ok(Show {
            text: Some(text),
            record_ids: Some(next),
            ledger_item_ids: given,
            ..
        }) => Ok(Some(Shown { text, next, given })),
        _ => Err(Trouble::Failed("cairn show: unexpected output".into())),
    }
}

/// The ledger's kinds as cairn names them, in its order, and what the tab calls each; those it
/// adds later go under `Other`, after them.
const KINDS: [(&str, &str); 3] = [
    ("state", "State"),
    ("open", "Open"),
    ("convention", "Conventions"),
];

/// The project's ledger: `cairn ledger --json` run in `cwd`, the entries in `given` marked as
/// what the next session is given.
fn ledger(
    program: &str,
    cwd: &str,
    given: &[String],
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Ledger {
    let out = match run(program, &["ledger", "--json"], cwd, cancel) {
        Ok(out) => out,
        // cairn before 0.5.0, which has no `ledger`.
        Err(Trouble::Unknown(_)) => return Ledger::Unsupported,
        Err(Trouble::Missing) => {
            return Ledger::Failed("cairn ledger: cairn is not installed".into());
        }
        Err(Trouble::Failed(why)) => return Ledger::Failed(why),
    };
    let unexpected = || Ledger::Failed("cairn ledger: unexpected output".into());
    let Ok(out) = serde_json::from_slice::<LedgerOut>(&out) else {
        return unexpected();
    };
    let mut groups: Vec<Group> = KINDS
        .iter()
        .map(|(_, name)| *name)
        .chain(["Other"])
        .map(|name| Group {
            name,
            entries: Vec::new(),
        })
        .collect();
    for item in out.items {
        let (Some(added), Some(updated)) = (
            stamp(&item.created_at, offset),
            stamp(&item.updated_at, offset),
        ) else {
            return unexpected();
        };
        // An agent's session is `<agent>:<session>`.
        let (by, session) = match item.created_by.split_once(':') {
            Some((agent, session)) => (by(agent), Some(session.chars().take(8).collect())),
            None if item.created_by == "user" => ("You".to_owned(), None),
            None => (item.created_by, None),
        };
        let kind = KINDS.iter().position(|(kind, _)| *kind == item.kind);
        groups[kind.unwrap_or(KINDS.len())].entries.push(Entry {
            given: given.contains(&item.id),
            id: item.id,
            no: item.no,
            title: item.title,
            body: item.body,
            updated: (updated != added).then_some(updated),
            added,
            by,
            session,
        });
    }
    groups.retain(|group| !group.entries.is_empty());
    Ledger::Groups(groups)
}

/// The project's handover notes, the newest `limit` records' worth, by the session that saved them,
/// those in `next` marked as what the next session is given; `None` from a cairn that does not
/// list them.
fn notes(
    program: &str,
    cwd: &str,
    limit: usize,
    next: &[String],
    now: SystemTime,
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Result<Option<Box<Notes>>, Trouble> {
    let limit = limit.to_string();
    let out = match run(program, &["list", "--json", "--limit", &limit], cwd, cancel) {
        Ok(out) => out,
        // cairn before 0.3.0, whose `list` has no `--json`.
        Err(Trouble::Unknown(_)) => return Ok(None),
        Err(trouble) => return Err(trouble),
    };
    let unexpected = || Trouble::Failed("cairn list: unexpected output".into());
    let list = serde_json::from_slice::<List>(&out).map_err(|_| unexpected())?;
    let now = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    let today = activity::day_of(now, offset(now));
    let year = |day| activity::date(day).0;
    let older = (list.records.len() as u64) < list.total;
    // Each session's notes, the newest first, beside when and on which day each was saved; the
    // sessions in the order of their newest.
    let mut sessions: Vec<Vec<(i64, activity::Day, Note)>> = Vec::new();
    let mut of_source: HashMap<String, usize> = HashMap::new();
    let mut oldest = None;
    // Corrections, retractions, restores and kinds yet to come are records too, but not notes.
    for record in list.records.into_iter().filter(|r| r.kind == "checkpoint") {
        let at = unix(&record.created_at).ok_or_else(unexpected)? as i64;
        let local = at + offset(at);
        let on = local.div_euclid(DAY);
        let of_day = local.rem_euclid(DAY);
        // A cairn that does not say which source falls back on the agent's session; a record of
        // none is a session of its own.
        let source = match (
            record.source_id.filter(|s| !s.is_empty()),
            &record.session_id,
        ) {
            (Some(source), _) => source,
            (None, Some(session)) => format!("{}:{session}", record.agent),
            (None, None) => format!("record:{}", record.id),
        };
        let day = (on != today).then(|| {
            let (_, month, of) = activity::date(on);
            let day = format!("{} {of}", MONTHS[month as usize - 1]);
            if year(on) == year(today) {
                day
            } else {
                format!("{day}, {}", year(on))
            }
        });
        let note = Note {
            next: next.contains(&record.id),
            id: record.id,
            time: format!("{:02}:{:02}", of_day / 3600, of_day % 3600 / 60),
            day,
            ago: (on == today)
                .then(|| format!("{} ago", short_time(Some((now - at).max(0) as f64)))),
            by: by(&record.agent),
            agent: record.agent,
            branch: record.branch.filter(|branch| !branch.is_empty()),
            summary: record.summary,
        };
        let index = *of_source.entry(source).or_insert_with(|| {
            sessions.push(Vec::new());
            sessions.len() - 1
        });
        sessions[index].push((at, on, note));
        oldest = Some(index);
    }
    // Each session shown by the note the next session is given, or else its newest.
    let mut shown: Vec<(i64, activity::Day, Session)> = sessions
        .into_iter()
        .enumerate()
        .map(|(index, mut notes)| {
            let by = notes.iter().position(|(_, _, note)| note.next).unwrap_or(0);
            let (at, on, note) = notes.remove(by);
            let session = Session {
                note,
                earlier: notes.into_iter().map(|(_, _, note)| note).collect(),
                more: older && oldest == Some(index),
            };
            (at, on, session)
        })
        .collect();
    shown.sort_by_key(|(at, _, _)| std::cmp::Reverse(*at));
    let (mut next, mut days) = (Vec::new(), Vec::<(activity::Day, Day)>::new());
    for (_, on, session) in shown {
        if session.note.next {
            next.push(session);
            continue;
        }
        match days.last_mut() {
            Some((last, day)) if *last == on => day.sessions.push(session),
            _ => {
                let mut date = activity::label(on);
                if year(on) != year(today) {
                    date = format!("{date}, {}", year(on));
                }
                let name = match today - on {
                    0 => Some("Today"),
                    1 => Some("Yesterday"),
                    _ => None,
                };
                days.push((
                    on,
                    Day {
                        name,
                        date,
                        sessions: vec![session],
                    },
                ));
            }
        }
    }
    Ok(Some(Box::new(Notes {
        next,
        days: days.into_iter().map(|(_, day)| day).collect(),
        older,
    })))
}

/// One note in full: `cairn show <ID> --json` run in `cwd`, which takes saved records into
/// cairn's database first; or `cairn show <ID>: ` and the first line of why not.
pub fn note(
    program: &str,
    cwd: &str,
    id: &str,
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Result<Full, String> {
    let out = match run(program, &["show", id, "--json"], cwd, cancel) {
        Ok(out) => out,
        Err(Trouble::Missing) => return Err(format!("cairn show {id}: cairn is not installed")),
        Err(Trouble::Failed(why) | Trouble::Unknown(why)) => return Err(why),
    };
    let unexpected = || format!("cairn show {id}: unexpected output");
    let record = serde_json::from_slice::<Record>(&out).map_err(|_| unexpected())?;
    let saved = |record: &Record| stamp(&record.created_at, offset);
    let correction = match record.correction.as_deref() {
        Some(correction) => Some((
            saved(correction).ok_or_else(unexpected)?,
            correction.body.clone(),
        )),
        None => None,
    };
    Ok(Full {
        saved: saved(&record).ok_or_else(unexpected)?,
        by: by(&record.agent),
        session: record
            .session_id
            .map(|session| session.chars().take(8).collect()),
        branch: record.branch.filter(|branch| !branch.is_empty()),
        body: record.body,
        correction,
    })
}

/// A time as cairn writes them, on the clock `offset` gives: `Oct 10, 2026 at 17:44:51`.
fn stamp(text: &str, offset: &dyn Fn(i64) -> i64) -> Option<String> {
    let at = unix(text)? as i64;
    let local = at + offset(at);
    let (year, month, day) = activity::date(local.div_euclid(DAY));
    let of_day = local.rem_euclid(DAY);
    Some(format!(
        "{} {day}, {year} at {:02}:{:02}:{:02}",
        MONTHS[month as usize - 1],
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    ))
}

/// Who saved a record, from what cairn calls its agent: `local` is none of them, a save by hand
/// or a correction the user wrote; one cairn learns of later goes by its own name.
fn by(agent: &str) -> String {
    match agent {
        "claude" => "Claude",
        "codex" => "Codex",
        "local" => "Manual",
        other => other,
    }
    .to_owned()
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
        Err(Trouble::Failed(why) | Trouble::Unknown(why)) => Err(why),
    }
}

/// Runs `cairn` with `args` in `cwd`: what it printed, or why not, as `cairn <command>: ` and the
/// first line of what it said, the command being the words before its first option (`show`,
/// `show <ID>`).
fn run(program: &str, args: &[&str], cwd: &str, cancel: &AtomicBool) -> Result<Vec<u8>, Trouble> {
    let words = args.iter().take_while(|arg| !arg.starts_with("--"));
    let command = words.copied().collect::<Vec<_>>().join(" ");
    let why = |why: &str| format!("cairn {command}: {why}");
    let failed = |text: &str| Trouble::Failed(why(text));
    let started = Instant::now();
    match command::run(program, args, Some(Path::new(cwd)), TIMEOUT, cancel) {
        Ok(out) if out.status.success() => Ok(out.stdout),
        Ok(out) => {
            let said = first_line(&out.stderr)
                .or_else(|| first_line(&out.stdout))
                .unwrap_or_else(|| out.status.to_string());
            Err(match out.status.code() {
                Some(2) => Trouble::Unknown(why(&said)),
                _ => failed(&said),
            })
        }
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
