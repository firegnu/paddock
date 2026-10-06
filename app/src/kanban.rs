//! The right sidebar's Kanban tab, what it knows: each task file in `docs/任务/` on the focused
//! repository's main is a card, and where it stands is read from Git and corral, never stored
//! (DESIGN §13 P5-29). Git is read in the background with read-only commands (`read`); the agents
//! come from the left sidebar's listing (`Seen`); `board` puts the two together. Nothing here
//! writes a file or acts on an agent.
use crate::{agents::Status, card, git};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// Where the task files are, in every repository.
pub const TASKS: &str = "docs/任务";
/// The branch the task files are read from, and branches are measured against.
pub const MAIN: &str = "main";
/// How many DONE cards are kept, newest first.
pub const DONE_KEPT: usize = 5;
/// How far back main's history is read for merges and wrap-ups.
const HISTORY: &str = "5000";

/// A task file, as its text says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// From the start of the file's name, such as `P5-29a`.
    pub id: String,
    /// The first line after `# 任务：`, or the rest of the file's name.
    pub title: String,
    /// The file's name within `docs/任务/`.
    pub file: String,
    /// From the `- worktree：` line of 「在哪里干活」.
    pub worktree: Option<String>,
    pub branch: Option<String>,
    /// From an optional `依赖：` line: the task this one waits for.
    pub depends: Option<String>,
}

/// What a task file's text gives, apart from its name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    title: Option<String>,
    worktree: Option<String>,
    branch: Option<String>,
    depends: Option<String>,
    /// It has a `## 完成记录` section.
    record: bool,
}

/// The id a task file's name starts with: a first part of capitals and digits (`P5`, `M0`,
/// `T76`), then parts that start with a digit (`29a`) or a capital followed by a digit (`T1`).
pub fn task_id(file: &str) -> Option<String> {
    let stem = file.strip_suffix(".md").unwrap_or(file);
    let mut parts = stem.split('-');
    let first = parts.next()?;
    let letters = first.bytes().take_while(u8::is_ascii_uppercase).count();
    let digits = first[letters..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if letters == 0
        || digits == 0
        || !first[letters + digits..]
            .bytes()
            .all(|b| b.is_ascii_lowercase())
    {
        return None;
    }
    let mut id = first.to_owned();
    for part in parts {
        let bytes = part.as_bytes();
        let more = match bytes {
            [b'0'..=b'9', ..] => part.bytes().all(|b| b.is_ascii_alphanumeric()),
            [b'A'..=b'Z', b'0'..=b'9', ..] => part.bytes().all(|b| b.is_ascii_alphanumeric()),
            _ => false,
        };
        if !more {
            break;
        }
        id.push('-');
        id.push_str(part);
    }
    Some(id)
}

/// The task in `file`, or `None` for a file whose name has no id.
pub fn parse(file: &str, text: &str) -> Option<Task> {
    task(file, &read_text(text))
}

fn task(file: &str, text: &Text) -> Option<Task> {
    let id = task_id(file)?;
    let title = text.title.clone().unwrap_or_else(|| {
        let stem = file.strip_suffix(".md").unwrap_or(file);
        let rest = stem[id.len()..].trim_start_matches('-');
        if rest.is_empty() {
            id.clone()
        } else {
            rest.to_owned()
        }
    });
    Some(Task {
        id,
        title,
        file: file.to_owned(),
        worktree: text.worktree.clone(),
        branch: text.branch.clone(),
        depends: text.depends.clone(),
    })
}

/// Whether the file has a completion record: a `## 完成记录` heading.
pub fn has_record(text: &str) -> bool {
    read_text(text).record
}

fn read_text(text: &str) -> Text {
    let mut out = Text::default();
    let mut section = "";
    for line in text.lines() {
        let line = line.trim_end();
        if out.title.is_none()
            && let Some(rest) = line.strip_prefix("# ")
        {
            out.title = Some(heading(rest));
            continue;
        }
        if let Some(name) = line.strip_prefix("## ") {
            section = name.trim();
            if section == "完成记录" {
                out.record = true;
            }
            continue;
        }
        if out.depends.is_none()
            && let Some(rest) = line
                .strip_prefix("依赖：")
                .or_else(|| line.strip_prefix("依赖:"))
        {
            let token: String = rest
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            out.depends = task_id(token.trim_end_matches('-'));
            continue;
        }
        // The first worktree line of 「在哪里干活」: an earlier mention, such as a prototype's
        // branch in the background, is not this task's.
        if section == "在哪里干活"
            && out.worktree.is_none()
            && out.branch.is_none()
            && let Some(rest) = line
                .strip_prefix("- worktree：")
                .or_else(|| line.strip_prefix("- worktree:"))
        {
            let quoted: Vec<&str> = rest.split('`').skip(1).step_by(2).collect();
            out.worktree = quoted
                .first()
                .map(|path| path.trim_end_matches('/').to_owned())
                .filter(|path| !path.is_empty());
            out.branch = rest
                .find("分支")
                .and_then(|at| rest[at..].split('`').nth(1))
                .map(str::to_owned)
                .filter(|branch| !branch.is_empty());
        }
    }
    out
}

/// A title line's words after `任务：` (and any id before it, as in `任务 P3-1：`).
fn heading(rest: &str) -> String {
    let rest = rest.trim();
    let Some(after) = rest.strip_prefix("任务") else {
        return rest.to_owned();
    };
    match after.find(['：', ':']) {
        Some(at) => {
            let colon = after[at..].chars().next().map_or(1, char::len_utf8);
            after[at + colon..].trim().to_owned()
        }
        None => after.trim().to_owned(),
    }
}

/// The id at the start of a commit subject after `prefix` (`收尾:`, `合并`): the whole of it, so
/// `P5-24a` is never taken for `P5-24`.
fn subject_id<'a>(subject: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let rest = prefixes
        .iter()
        .find_map(|prefix| subject.strip_prefix(prefix))?
        .trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(rest.len());
    let id = rest[..end].trim_end_matches('-');
    (!id.is_empty()).then_some(id)
}

/// The id a wrap-up commit (`收尾: <id> …`) is for.
pub fn wrapped_id(subject: &str) -> Option<&str> {
    subject_id(subject, &["收尾:", "收尾："])
}

/// The id a merge commit (`合并 <id>：…`) is for.
pub fn merged_id(subject: &str) -> Option<&str> {
    subject_id(subject, &["合并 ", "合并"])
}

/// The board's five columns, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    Queued,
    InProgress,
    ToReview,
    Merged,
    Done,
}

impl Column {
    pub const ALL: [Column; 5] = [
        Column::Queued,
        Column::InProgress,
        Column::ToReview,
        Column::Merged,
        Column::Done,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Column::Queued => "QUEUED",
            Column::InProgress => "IN PROGRESS",
            Column::ToReview => "TO REVIEW",
            Column::Merged => "MERGED",
            Column::Done => "DONE",
        }
    }

    /// The group's line at the right of its header.
    pub fn note(self) -> &'static str {
        match self {
            Column::Queued => "not started",
            Column::InProgress => "",
            Column::ToReview => "not merged",
            Column::Merged => "not wrapped up",
            Column::Done => "last 5",
        }
    }
}

/// The groups folded when nothing says otherwise: DONE.
pub fn folded_default() -> Vec<Column> {
    vec![Column::Done]
}

/// A branch as Git has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    /// When its tip was committed, in seconds since the epoch.
    pub time: i64,
    /// Merged into main: its tip is reachable from main but not on main's own line, so a branch
    /// just made from main, with nothing of its own, is not taken for merged.
    pub merged: bool,
}

/// What Git says about one repository.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// The main worktree, where the task files are opened.
    pub repo: PathBuf,
    /// Its directory's name.
    pub name: String,
    /// The task files on main, in natural order of their ids.
    pub tasks: Vec<Task>,
    /// Ids with a wrap-up commit on main, and its time.
    pub wrapped: HashMap<String, i64>,
    /// Ids with a merge commit on main, and its time.
    pub merges: HashMap<String, i64>,
    pub branches: HashMap<String, Branch>,
    /// Each worktree's path and the branch it has out.
    pub worktrees: Vec<(String, Option<String>)>,
    /// Ids whose task file has a completion record: on its branch, or on main when the branch
    /// is not there.
    pub records: HashSet<String>,
    /// Lines added and deleted since main, by id, for tasks not yet merged: in its worktree,
    /// committed or not, else on its branch.
    pub lines: HashMap<String, (u64, u64)>,
}

impl Facts {
    /// The task's worktree is there: its path, or its branch checked out somewhere.
    pub fn worktree(&self, task: &Task) -> Option<&str> {
        self.worktrees
            .iter()
            .find(|(path, branch)| {
                task.worktree.as_deref() == Some(path.as_str())
                    || (task.branch.is_some() && branch.as_deref() == task.branch.as_deref())
            })
            .map(|(path, _)| path.as_str())
    }

    /// The task's branch, when it is there and is not main.
    fn branch(&self, task: &Task) -> Option<&Branch> {
        let name = task.branch.as_deref().filter(|b| *b != MAIN)?;
        self.branches.get(name)
    }
}

/// What reading a repository found.
#[derive(Clone, Debug, PartialEq)]
pub enum Read {
    NotRepository,
    /// A repository without a main branch.
    NoMain {
        name: String,
    },
    /// Main has no task files.
    NoTasks {
        name: String,
    },
    Board(Box<Facts>),
    /// Git could not be run or failed.
    Failed,
}

/// Task files' texts as read, by blob id, so a file is read only when it changes.
pub type Cache = Mutex<HashMap<String, Text>>;

pub fn cache() -> Cache {
    Mutex::new(HashMap::new())
}

/// Reads the repository `cwd` is in: its main worktree's task files on main, main's merges and
/// wrap-ups, its branches and worktrees, and for each task not yet merged its completion record
/// and changed lines. Only read-only Git commands, through [`git::git`]'s options.
pub fn read(program: &str, cwd: &str, cache: &Cache, cancel: &AtomicBool) -> Read {
    let Some((top, _)) = git::repository(program, cwd, cancel) else {
        return Read::NotRepository;
    };
    let run = |dir: &Path, args: &[&str]| -> Option<Vec<u8>> {
        let out = git::git(program, dir, args, cancel)?;
        out.status.success().then_some(out.stdout)
    };
    let text = |dir: &Path, args: &[&str]| -> Option<String> {
        run(dir, args).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    };
    let Some(list) = text(&top, &["worktree", "list", "--porcelain"]) else {
        return Read::Failed;
    };
    let worktrees = worktrees(&list);
    // The first worktree listed is the main one.
    let repo = worktrees
        .first()
        .map(|(path, _)| PathBuf::from(path))
        .unwrap_or_else(|| top.clone());
    let name = repo
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let main = format!("refs/heads/{MAIN}");
    if run(
        &repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{main}^{{commit}}"),
        ],
    )
    .is_none()
    {
        return if cancel.load(Ordering::Relaxed) {
            Read::Failed
        } else {
            Read::NoMain { name }
        };
    }
    let Some(listing) = run(&repo, &["ls-tree", "-z", &main, "--", &format!("{TASKS}/")]) else {
        return Read::Failed;
    };
    let files: Vec<(String, String)> = listing
        .split(|b| *b == 0)
        .filter_map(|entry| {
            let entry = std::str::from_utf8(entry).ok()?;
            let (meta, path) = entry.split_once('\t')?;
            let mut meta = meta.split(' ');
            let (_, kind, oid) = (meta.next()?, meta.next()?, meta.next()?);
            let file = path.strip_prefix(TASKS)?.strip_prefix('/')?;
            (kind == "blob" && file.ends_with(".md") && !file.contains('/'))
                .then(|| (file.to_owned(), oid.to_owned()))
        })
        .collect();
    let blob = |oid: &str| -> Option<Text> {
        if let Some(text) = cache.lock().ok()?.get(oid) {
            return Some(text.clone());
        }
        let bytes = run(&repo, &["cat-file", "blob", oid])?;
        let text = read_text(&String::from_utf8_lossy(&bytes));
        cache.lock().ok()?.insert(oid.to_owned(), text.clone());
        Some(text)
    };
    let mut tasks = Vec::new();
    let mut main_records = HashSet::new();
    for (file, oid) in &files {
        let Some(text) = blob(oid) else {
            return Read::Failed;
        };
        if let Some(task) = task(file, &text) {
            if text.record {
                main_records.insert(task.id.clone());
            }
            tasks.push(task);
        }
    }
    if tasks.is_empty() {
        return if cancel.load(Ordering::Relaxed) {
            Read::Failed
        } else {
            Read::NoTasks { name }
        };
    }
    tasks.sort_by(|a, b| natural(&a.id, &b.id).then_with(|| a.file.cmp(&b.file)));
    // Main's own line: its merges and wrap-ups, newest first, and the commits on it.
    let Some(log) = text(
        &repo,
        &[
            "log",
            "--first-parent",
            "-n",
            HISTORY,
            "--format=%H%x1f%ct%x1f%s%x1e",
            &main,
            "--",
        ],
    ) else {
        return Read::Failed;
    };
    let mut line = HashSet::new();
    let (mut wrapped, mut merges) = (HashMap::new(), HashMap::new());
    for record in log.split('\x1e') {
        let mut fields = record.trim_start_matches('\n').splitn(3, '\x1f');
        let (Some(hash), Some(time), Some(subject)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let time: i64 = time.parse().unwrap_or(0);
        line.insert(hash.to_owned());
        if let Some(id) = wrapped_id(subject) {
            wrapped.entry(id.to_owned()).or_insert(time);
        }
        if let Some(id) = merged_id(subject) {
            merges.entry(id.to_owned()).or_insert(time);
        }
    }
    let Some(refs) = text(
        &repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)%1f%(objectname)%1f%(committerdate:unix)",
            "refs/heads/",
        ],
    ) else {
        return Read::Failed;
    };
    let Some(merged) = text(
        &repo,
        &["branch", "--merged", &main, "--format=%(refname:short)"],
    ) else {
        return Read::Failed;
    };
    let merged: HashSet<&str> = merged.lines().collect();
    let branches: HashMap<String, Branch> = refs
        .lines()
        .filter_map(|row| {
            let mut fields = row.split('\x1f');
            let (name, oid, time) = (fields.next()?, fields.next()?, fields.next()?);
            Some((
                name.to_owned(),
                Branch {
                    time: time.parse().unwrap_or(0),
                    merged: merged.contains(name) && !line.contains(oid),
                },
            ))
        })
        .collect();
    let mut facts = Facts {
        repo: repo.clone(),
        name,
        tasks,
        wrapped,
        merges,
        branches,
        worktrees,
        records: HashSet::new(),
        lines: HashMap::new(),
    };
    // What only the tasks still open need: their completion records and changed lines.
    let open: Vec<Task> = facts
        .tasks
        .iter()
        .filter(|t| {
            !facts.wrapped.contains_key(&t.id)
                && !facts.merges.contains_key(&t.id)
                && !facts.branch(t).is_some_and(|b| b.merged)
        })
        .cloned()
        .collect();
    for task in &open {
        if cancel.load(Ordering::Relaxed) {
            return Read::Failed;
        }
        let record = match (facts.branch(task), &task.branch) {
            (Some(_), Some(branch)) => {
                let path = format!("{TASKS}/{}", task.file);
                run(&repo, &["ls-tree", "-z", branch, "--", &path])
                    .and_then(|entry| {
                        let entry = String::from_utf8_lossy(&entry).into_owned();
                        let oid = entry.split('\t').next()?.split(' ').nth(2)?.to_owned();
                        blob(&oid)
                    })
                    .is_some_and(|text| text.record)
            }
            _ => main_records.contains(&task.id),
        };
        if record {
            facts.records.insert(task.id.clone());
        }
        let lines = match (facts.worktree(task), &task.branch) {
            (Some(path), _) => git::lines_since(program, Path::new(path), &main, cancel),
            (None, Some(branch)) if facts.branch(task).is_some() => {
                git::branch_lines(program, &repo, &main, branch, cancel)
            }
            _ => None,
        };
        if let Some(lines) = lines.filter(|(added, deleted)| added + deleted > 0) {
            facts.lines.insert(task.id.clone(), lines);
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Read::Failed;
    }
    Read::Board(Box::new(facts))
}

/// `git worktree list --porcelain`: each worktree's path and the branch it has out.
fn worktrees(list: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for block in list.split("\n\n") {
        let mut path = None;
        let mut branch = None;
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.trim_end_matches('/').to_owned());
            } else if let Some(rest) = line.strip_prefix("branch ") {
                branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_owned());
            }
        }
        if let Some(path) = path {
            out.push((path, branch));
        }
    }
    out
}

/// Ids compared with their numbers as numbers: `P5-9` before `P5-10`.
pub fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    fn chunks(s: &str) -> Vec<(bool, &str)> {
        let mut out = Vec::new();
        let mut start = 0;
        let bytes = s.as_bytes();
        for i in 1..=bytes.len() {
            if i == bytes.len() || bytes[i].is_ascii_digit() != bytes[start].is_ascii_digit() {
                out.push((bytes[start].is_ascii_digit(), &s[start..i]));
                start = i;
            }
        }
        out
    }
    let (a, b) = (chunks(a), chunks(b));
    for (x, y) in a.iter().zip(&b) {
        let order = match (x, y) {
            ((true, x), (true, y)) => x
                .parse::<u64>()
                .unwrap_or(0)
                .cmp(&y.parse::<u64>().unwrap_or(0))
                .then_with(|| x.cmp(y)),
            ((_, x), (_, y)) => x.cmp(y),
        };
        if order.is_ne() {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

/// An agent as the left sidebar lists it, as far as the board needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub name: String,
    pub kind: Option<String>,
    pub cwd: Option<String>,
    /// Its `task` label.
    pub task: Option<String>,
    pub status: Status,
    /// Idle, its last reply ending in DONE.
    pub said_done: bool,
    /// When its state began, in seconds since the epoch.
    pub since: Option<f64>,
}

impl Seen {
    /// From the sidebar's agent, its status and, while idle, its last reply.
    pub fn of(agent: &crate::corral::Agent, status: Status, reply: Option<&str>) -> Self {
        Seen {
            name: agent.name.clone(),
            kind: agent.kind.clone(),
            cwd: agent.cwd.clone(),
            task: agent
                .labels
                .get("task")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            status,
            said_done: status == Status::Idle
                && reply.is_some_and(|reply| reply.trim_end().ends_with("DONE")),
            since: agent.state_started,
        }
    }
}

/// The task's agent: the one labelled `task=<id>`, else the one working in its worktree; none
/// when neither is there. Names are never guessed (corral adds `-1` and the like to them).
pub fn agent_for<'a>(task: &Task, agents: &'a [Seen]) -> Option<&'a Seen> {
    // Of several, one still running before one that has exited.
    let best = |found: Vec<&'a Seen>| {
        found
            .iter()
            .find(|a| a.status != Status::Exited)
            .or(found.first())
            .copied()
    };
    let labelled: Vec<&Seen> = agents
        .iter()
        .filter(|a| a.task.as_deref() == Some(task.id.as_str()))
        .collect();
    if !labelled.is_empty() {
        return best(labelled);
    }
    // By directory, only among agents not labelled for another task.
    let worktree = task.worktree.as_deref()?;
    best(
        agents
            .iter()
            .filter(|a| {
                a.task.is_none()
                    && a.cwd.as_deref().map(|c| c.trim_end_matches('/')) == Some(worktree)
            })
            .collect(),
    )
}

/// Where the task stands, judged from the last column back, the first that holds. When it cannot
/// be told, it stays in the earlier column.
pub fn column(task: &Task, facts: &Facts, agent: Option<&Seen>) -> Column {
    if facts.wrapped.contains_key(&task.id) {
        Column::Done
    } else if facts.merges.contains_key(&task.id) || facts.branch(task).is_some_and(|b| b.merged) {
        Column::Merged
    } else if facts.records.contains(&task.id) || agent.is_some_and(|a| a.said_done) {
        Column::ToReview
    } else if facts.worktree(task).is_some() || agent.is_some() {
        Column::InProgress
    } else {
        Column::Queued
    }
}

/// How a card's state words are coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Dim,
    /// The agent's status colour.
    Agent,
    Review,
    Merged,
}

/// One card, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub file: String,
    pub column: Column,
    /// How long ago it last moved, as the sidebar writes ages; empty when unknown.
    pub age: String,
    pub agent: Option<Seen>,
    pub state: Option<(String, Tone)>,
    pub branch: Option<String>,
    pub lines: Option<(u64, u64)>,
    /// When it last moved, for ordering.
    time: i64,
}

/// The board: the repository, and the cards in each column, DONE cut to its last few.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub name: String,
    pub repo: PathBuf,
    pub columns: [Vec<Card>; 5],
    /// How many are done in all, before the cut.
    pub done: usize,
}

impl Board {
    pub fn cards(&self, column: Column) -> &[Card] {
        &self.columns[column as usize]
    }

    /// The count a group's header shows.
    pub fn count(&self, column: Column) -> usize {
        match column {
            Column::Done => self.done,
            _ => self.cards(column).len(),
        }
    }
}

/// The board for `facts` and the agents now listed, at `now` (seconds since the epoch).
pub fn board(facts: &Facts, agents: &[Seen], now: f64) -> Board {
    let mut columns: [Vec<Card>; 5] = Default::default();
    for task in &facts.tasks {
        let agent = agent_for(task, agents);
        let column = column(task, facts, agent);
        let branch_time = facts.branch(task).map(|b| b.time);
        let since = agent.and_then(|a| a.since).map(|s| s as i64);
        let time = match column {
            Column::Done => facts.wrapped.get(&task.id).copied(),
            Column::Merged => facts.merges.get(&task.id).copied().or(branch_time),
            Column::ToReview | Column::InProgress => branch_time.max(since),
            Column::Queued => None,
        };
        let worktree = facts.worktree(task).is_some();
        let state = match column {
            Column::Queued => task
                .depends
                .as_ref()
                .filter(|dep| !facts.wrapped.contains_key(*dep))
                .map(|dep| (format!("Waits for {dep}"), Tone::Dim)),
            Column::InProgress => Some(match agent {
                Some(a) => (card::look(a.status).label.to_owned(), Tone::Agent),
                None => ("No agent".to_owned(), Tone::Dim),
            }),
            Column::ToReview => Some(if agent.is_some_and(|a| a.said_done) {
                ("Done".to_owned(), Tone::Review)
            } else {
                ("Completion record".to_owned(), Tone::Review)
            }),
            Column::Merged => Some((
                if worktree {
                    "Merged · worktree still there".to_owned()
                } else {
                    "Merged".to_owned()
                },
                Tone::Merged,
            )),
            Column::Done => None,
        };
        let shows_branch = matches!(
            column,
            Column::InProgress | Column::ToReview | Column::Merged
        );
        columns[column as usize].push(Card {
            id: task.id.clone(),
            title: task.title.clone(),
            file: task.file.clone(),
            column,
            age: time.map_or_else(String::new, |t| card::short_time(Some(now - t as f64))),
            agent: agent.filter(|_| column != Column::Done).cloned(),
            state,
            branch: task.branch.clone().filter(|_| shows_branch),
            lines: matches!(column, Column::InProgress | Column::ToReview)
                .then(|| facts.lines.get(&task.id).copied())
                .flatten(),
            time: time.unwrap_or(0),
        });
    }
    for (column, cards) in Column::ALL.iter().zip(columns.iter_mut()) {
        // Queued in the order of their ids; the rest most recent first.
        if *column != Column::Queued {
            cards.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| natural(&a.id, &b.id)));
        }
    }
    let done = columns[Column::Done as usize].len();
    columns[Column::Done as usize].truncate(DONE_KEPT);
    Board {
        name: facts.name.clone(),
        repo: facts.repo.clone(),
        columns,
        done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_come_from_the_start_of_the_name() {
        assert_eq!(
            task_id("P5-29a-Kanban只读流水线看板.md").as_deref(),
            Some("P5-29a")
        );
        assert_eq!(
            task_id("P5-21-New-Agent记下模型和强度.md").as_deref(),
            Some("P5-21")
        );
        assert_eq!(task_id("P1-T2-主题系统.md").as_deref(), Some("P1-T2"));
        assert_eq!(
            task_id("P5-10-11-侧栏底部按钮与收起.md").as_deref(),
            Some("P5-10-11")
        );
        assert_eq!(task_id("M0-迁移Saddle代码.md").as_deref(), Some("M0"));
        assert_eq!(task_id("T76-GPUI终端原型.md").as_deref(), Some("T76"));
        assert_eq!(task_id("P5-3-Command-palette.md").as_deref(), Some("P5-3"));
        assert_eq!(task_id("README.md"), None);
        assert_eq!(task_id("说明.md"), None);
    }

    #[test]
    fn a_task_file_gives_its_title_place_and_dependency() {
        let text = "# 任务：右侧栏 Kanban 标签\n\n2026-10-07，paddock/main 交给 paddock/dev-kanban。\n\
            依据：本轮做只读看板。\n依赖：P5-28a\n\n## 背景\n原型分支 `p5-13r-browser`。\n\
            - worktree：`/old/place`，分支 `old`\n\n## 在哪里干活\n\
            - worktree：`/w/p5-29a-kanban/`，分支 `p5-29a-kanban`（已从 main 建好）。\n\
            - worktree：`/w/other`，分支 `other`\n";
        let task = parse("P5-29a-Kanban只读流水线看板.md", text).unwrap();
        assert_eq!(
            task,
            Task {
                id: "P5-29a".into(),
                title: "右侧栏 Kanban 标签".into(),
                file: "P5-29a-Kanban只读流水线看板.md".into(),
                worktree: Some("/w/p5-29a-kanban".into()),
                branch: Some("p5-29a-kanban".into()),
                depends: Some("P5-28a".into()),
            }
        );
        assert!(!has_record(text));
        assert!(has_record(&format!("{text}\n## 完成记录\n做了。\n")));
        // A title with the id before the colon, as the early files have.
        let early = parse("P3-1-Settings.md", "# 任务 P3-1：Settings 设置页\n").unwrap();
        assert_eq!(early.title, "Settings 设置页");
        // Lines missing: the title from the name, no place, no dependency.
        let bare = parse("P9-1-Something-new.md", "Some notes.\n").unwrap();
        assert_eq!(bare.title, "Something-new");
        assert_eq!(
            (bare.worktree, bare.branch, bare.depends),
            (None, None, None)
        );
        let only_id = parse("P9-2.md", "").unwrap();
        assert_eq!(only_id.title, "P9-2");
        // A worktree line without a branch keeps the path.
        let path_only = parse(
            "P9-3-x.md",
            "# 任务：x\n## 在哪里干活\n- worktree：`/w/x`。\n",
        )
        .unwrap();
        assert_eq!(path_only.worktree.as_deref(), Some("/w/x"));
        assert_eq!(path_only.branch, None);
        assert_eq!(parse("notes.md", "# 任务：x\n"), None);
    }

    #[test]
    fn commit_subjects_name_whole_ids() {
        assert_eq!(wrapped_id("收尾: P5-24a 窄条换种类 logo"), Some("P5-24a"));
        assert_eq!(wrapped_id("收尾：P5-24、P5-25 一起"), Some("P5-24"));
        assert_eq!(wrapped_id("收尾: P5-10-11-侧栏"), Some("P5-10-11"));
        assert_eq!(wrapped_id("合并 P5-24：x"), None);
        assert_eq!(merged_id("合并 P5-28a：右侧栏 Browser"), Some("P5-28a"));
        assert_eq!(merged_id("合并P5-28a：x"), Some("P5-28a"));
        assert_eq!(merged_id("收尾: P5-28a"), None);
        assert_ne!(wrapped_id("收尾: P5-24a 窄条"), Some("P5-24"));
    }

    #[test]
    fn ids_sort_by_their_numbers() {
        let mut ids = vec!["P5-10", "P5-9", "P5-29a", "P5-29", "P4-2", "M1"];
        ids.sort_by(|a, b| natural(a, b));
        assert_eq!(ids, ["M1", "P4-2", "P5-9", "P5-10", "P5-29", "P5-29a"]);
    }

    fn seen(name: &str, cwd: Option<&str>, task: Option<&str>, status: Status) -> Seen {
        Seen {
            name: name.into(),
            kind: Some("claude".into()),
            cwd: cwd.map(str::to_owned),
            task: task.map(str::to_owned),
            status,
            said_done: false,
            since: None,
        }
    }

    #[test]
    fn a_task_finds_its_agent_by_label_then_by_directory() {
        let task = parse(
            "P5-29a-Kanban.md",
            "# 任务：Kanban\n## 在哪里干活\n- worktree：`/w/p5-29a`，分支 `p5-29a`\n",
        )
        .unwrap();
        let by_dir = seen(
            "paddock/dev-kanban-1",
            Some("/w/p5-29a/"),
            None,
            Status::Working,
        );
        let by_label = seen(
            "paddock/dev-other",
            Some("/elsewhere"),
            Some("P5-29a"),
            Status::Idle,
        );
        let wrong_label = seen(
            "paddock/dev-x",
            Some("/w/p5-29a"),
            Some("P5-29"),
            Status::Idle,
        );
        let agents = [by_dir.clone(), by_label.clone()];
        assert_eq!(agent_for(&task, &agents), Some(&by_label));
        assert_eq!(
            agent_for(&task, std::slice::from_ref(&by_dir)),
            Some(&by_dir)
        );
        // One labelled for another task is not taken by its directory.
        assert_eq!(
            agent_for(&task, &[wrong_label.clone(), by_dir.clone()]),
            Some(&by_dir)
        );
        assert_eq!(agent_for(&task, &[wrong_label]), None);
        // Neither: no agent, even with a name that looks right.
        let named = seen(
            "paddock/dev-kanban",
            Some("/w/other"),
            None,
            Status::Working,
        );
        assert_eq!(agent_for(&task, &[named]), None);
        // Of two labelled, the running one.
        let exited = seen("paddock/dev-a", None, Some("P5-29a"), Status::Exited);
        let running = seen("paddock/dev-a-1", None, Some("P5-29a"), Status::Working);
        assert_eq!(agent_for(&task, &[exited, running.clone()]), Some(&running));
    }

    #[test]
    fn an_idle_agent_said_done_when_its_reply_ends_so() {
        let agent = crate::corral::Agent {
            name: "paddock/dev-a".into(),
            ..Default::default()
        };
        assert!(Seen::of(&agent, Status::Idle, Some("All done.\n\nDONE\n")).said_done);
        assert!(!Seen::of(&agent, Status::Idle, Some("DONE? not yet")).said_done);
        assert!(!Seen::of(&agent, Status::Working, Some("DONE")).said_done);
        assert!(!Seen::of(&agent, Status::Idle, None).said_done);
    }
}
