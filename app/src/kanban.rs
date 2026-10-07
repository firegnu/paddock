//! The right sidebar's Kanban tab, what it knows: each task file in `docs/任务/` on the focused
//! repository's main is a card, and where it stands is read from Git and corral, never stored
//! (DESIGN §13 P5-29). Git is read in the background with read-only commands (`read`); the agents
//! come from the left sidebar's listing (`Seen`); `board` puts the two together. Nothing here
//! acts on an agent, and the only file written is a new draft (`create_draft`), never one that is
//! there already.
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
/// How many DONE cards are listed, newest first, until all are asked for.
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
    /// From an optional `待用户：` line at the head of the file: what the user is waited on for.
    pub asks: Option<String>,
}

/// What a task file's text gives, apart from its name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    title: Option<String>,
    worktree: Option<String>,
    branch: Option<String>,
    depends: Option<String>,
    asks: Option<String>,
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
    task_id(file).map(|id| entry(file, id, text))
}

/// A draft: shown even when its name has no id, then with an empty id and its name for a title.
pub fn draft(file: &str, text: &str) -> Task {
    entry(file, task_id(file).unwrap_or_default(), &read_text(text))
}

fn entry(file: &str, id: String, text: &Text) -> Task {
    let title = text.title.clone().unwrap_or_else(|| {
        let stem = file.strip_suffix(".md").unwrap_or(file);
        let rest = stem[id.len()..].trim_start_matches('-');
        if rest.is_empty() {
            id.clone()
        } else {
            rest.to_owned()
        }
    });
    Task {
        id,
        title,
        file: file.to_owned(),
        worktree: text.worktree.clone(),
        branch: text.branch.clone(),
        depends: text.depends.clone(),
        asks: text.asks.clone(),
    }
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
        // Only at the head, before any section: a record that quotes the line does not count.
        if section.is_empty()
            && out.asks.is_none()
            && let Some(rest) = line
                .strip_prefix("待用户：")
                .or_else(|| line.strip_prefix("待用户:"))
        {
            out.asks = Some(rest.trim().to_owned());
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

/// Why a wrap-up commit drops its task (`收尾: <id> 不做：<why>`), when it does.
pub fn dropped(subject: &str) -> Option<&str> {
    let id = wrapped_id(subject)?;
    let rest = subject[subject.find(id)? + id.len()..].trim_start();
    let why = rest
        .strip_prefix("不做：")
        .or_else(|| rest.strip_prefix("不做:"))?;
    Some(why.trim())
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

    /// What the group holds, shown when the mouse is on its header.
    pub fn note(self) -> &'static str {
        match self {
            Column::Queued => "Not started",
            Column::InProgress => "",
            Column::ToReview => "Not merged",
            Column::Merged => "Not wrapped up",
            Column::Done => "Last 5",
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
    /// Task files in the main worktree that main does not have, untracked or only staged, in
    /// natural order of their ids.
    pub drafts: Vec<Task>,
    /// Ids with a wrap-up commit on main, and its time.
    pub wrapped: HashMap<String, i64>,
    /// Ids whose latest wrap-up drops them (`收尾: <id> 不做：<why>`), and why.
    pub dropped: HashMap<String, String>,
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

/// Reads the repository `cwd` is in: its main worktree's task files on main and its drafts, main's
/// merges and wrap-ups, its branches and worktrees, and for each task not yet merged its
/// completion record and changed lines. Only read-only Git commands, through [`git::git`]'s
/// options; the drafts are read from disk.
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
    // Drafts: the main worktree's task files main does not have, read as they are on disk. A
    // main worktree Git cannot list (a bare repository) has none.
    let on_main: HashSet<&str> = files.iter().map(|(file, _)| file.as_str()).collect();
    let listed = run(
        &repo,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            &format!("{TASKS}/"),
        ],
    )
    .unwrap_or_default();
    let mut drafts: Vec<Task> = listed
        .split(|b| *b == 0)
        .filter_map(|path| {
            let path = std::str::from_utf8(path).ok()?;
            let file = path.strip_prefix(TASKS)?.strip_prefix('/')?;
            if !file.ends_with(".md") || file.contains('/') || on_main.contains(file) {
                return None;
            }
            let text = std::fs::read_to_string(repo.join(path)).ok()?;
            Some(draft(file, &text))
        })
        .collect();
    drafts.sort_by(|a, b| natural(&a.id, &b.id).then_with(|| a.file.cmp(&b.file)));
    if tasks.is_empty() && drafts.is_empty() {
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
    let (mut wrapped, mut merges, mut dropped_ids) =
        (HashMap::new(), HashMap::new(), HashMap::new());
    for record in log.split('\x1e') {
        let mut fields = record.trim_start_matches('\n').splitn(3, '\x1f');
        let (Some(hash), Some(time), Some(subject)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let time: i64 = time.parse().unwrap_or(0);
        line.insert(hash.to_owned());
        // Newest first: the latest wrap-up says whether it was dropped.
        if let Some(id) = wrapped_id(subject)
            && !wrapped.contains_key(id)
        {
            wrapped.insert(id.to_owned(), time);
            if let Some(why) = dropped(subject) {
                dropped_ids.insert(id.to_owned(), why.to_owned());
            }
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
        drafts,
        wrapped,
        dropped: dropped_ids,
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

/// Where an id is already taken by a task file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// On a local branch, main or another.
    Branch(String),
    /// In the main worktree's `docs/任务/` on disk, committed or not.
    Worktree,
}

/// Every id a task file has in the repository whose main worktree is `repo`, and the first place
/// it was found: main, then the main worktree on disk, then the other local branches. `None` when
/// Git could not be read. Only read-only Git commands, through [`git::git`]'s options.
pub fn taken(program: &str, repo: &Path, cancel: &AtomicBool) -> Option<HashMap<String, Place>> {
    let run = |args: &[&str]| -> Option<Vec<u8>> {
        let out = git::git(program, repo, args, cancel)?;
        out.status.success().then_some(out.stdout)
    };
    let refs = run(&["for-each-ref", "--format=%(refname:short)", "refs/heads/"])?;
    let refs = String::from_utf8_lossy(&refs);
    let branches: Vec<&str> = refs.lines().collect();
    let files = |branch: &str| -> Option<Vec<String>> {
        let listing = run(&[
            "ls-tree",
            "-z",
            "--name-only",
            &format!("refs/heads/{branch}"),
            "--",
            &format!("{TASKS}/"),
        ])?;
        Some(
            listing
                .split(|b| *b == 0)
                .filter_map(|path| {
                    let path = std::str::from_utf8(path).ok()?;
                    Some(path.strip_prefix(TASKS)?.strip_prefix('/')?.to_owned())
                })
                .collect(),
        )
    };
    let on_disk: Vec<String> = std::fs::read_dir(repo.join(TASKS))
        .map(|dir| {
            dir.filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    // Main first, so an id there is told as main's; then the worktree; then the other branches.
    let mut places = Vec::new();
    if branches.contains(&MAIN) {
        places.push((Place::Branch(MAIN.to_owned()), files(MAIN)?));
    }
    places.push((Place::Worktree, on_disk));
    for branch in branches.iter().filter(|b| **b != MAIN) {
        places.push((Place::Branch((*branch).to_owned()), files(branch)?));
    }
    let mut ids = HashMap::new();
    for (place, files) in places {
        for file in files.iter().filter(|f| f.ends_with(".md")) {
            if let Some(id) = task_id(file) {
                ids.entry(id).or_insert_with(|| place.clone());
            }
        }
    }
    Some(ids)
}

/// The id offered for a new task: the next number in the highest series that numbers its tasks
/// (`P5-29c`, `P5-10-11` → `P5-30`); with no such series, the one after the highest id of one part
/// (`M3` → `M4`); `None` with no ids at all.
pub fn next_id<'a>(ids: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut series: Option<(&str, u64)> = None;
    let mut single: Option<&str> = None;
    for id in ids {
        let mut parts = id.split('-');
        let Some(first) = parts.next() else {
            continue;
        };
        let number = parts.next().and_then(|part| {
            let end = part
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(part.len());
            part[..end].parse::<u64>().ok()
        });
        match (number, series) {
            (Some(n), Some((top, most))) => match natural(first, top) {
                std::cmp::Ordering::Greater => series = Some((first, n)),
                std::cmp::Ordering::Equal if n > most => series = Some((first, n)),
                _ => {}
            },
            (Some(n), None) => series = Some((first, n)),
            (None, _) if first == id && single.is_none_or(|s| natural(id, s).is_gt()) => {
                single = Some(id);
            }
            (None, _) => {}
        }
    }
    if let Some((first, most)) = series {
        return Some(format!("{first}-{}", most + 1));
    }
    let single = single?;
    let letters = single.bytes().take_while(u8::is_ascii_uppercase).count();
    let digits = single[letters..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    let number: u64 = single[letters..letters + digits].parse().ok()?;
    Some(format!("{}{}", &single[..letters], number + 1))
}

/// How much of the title goes into the file's name, in characters.
const NAME_TITLE: usize = 80;

/// The new task file's name: `<id>-<title>.md`, the title's spaces, control characters and what a
/// file name cannot hold (`/ \ : * ? " < > |`) turned into `-`, runs of `-` made one and none left
/// at either end. Just `<id>.md` when nothing is left, or when the title's start would be read as
/// more of the id (`3D view` after `P5-30`).
pub fn draft_file(id: &str, title: &str) -> String {
    let mut words = String::new();
    for c in title.chars().take(NAME_TITLE) {
        let c = if c.is_whitespace() || c.is_control() || r#"/\:*?"<>|"#.contains(c) {
            '-'
        } else {
            c
        };
        if !(c == '-' && (words.is_empty() || words.ends_with('-'))) {
            words.push(c);
        }
    }
    let words = words.trim_matches(['-', '.']);
    let named = format!("{id}-{words}.md");
    if words.is_empty() || task_id(&named).as_deref() != Some(id) {
        format!("{id}.md")
    } else {
        named
    }
}

/// What a new draft holds: its title line and an empty 「用户原话」, the rest left to the task's
/// author.
pub fn draft_text(title: &str) -> String {
    format!("# 任务：{title}\n\n## 用户原话\n")
}

/// Why a draft was not created.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    BadId,
    NoTitle,
    Taken(String, Place),
    Exists(String),
    NoGit,
    Write(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::BadId => write!(f, "Not a task ID: use the form P5-30"),
            Refused::NoTitle => write!(f, "Enter a title"),
            Refused::Taken(id, Place::Branch(branch)) => write!(f, "{id} is already on {branch}"),
            Refused::Taken(id, Place::Worktree) => {
                write!(f, "{id} is already in the main worktree")
            }
            Refused::Exists(file) => write!(f, "{file} already exists"),
            Refused::NoGit => write!(f, "Git couldn't be read, so nothing was created"),
            Refused::Write(why) => write!(f, "Couldn't write the file: {why}"),
        }
    }
}

/// Writes a new task file for `id` and `title` in the main worktree `repo`'s `docs/任务/`, holding
/// only [`draft_text`]: not added to Git, and never over a file that is there. Refused for an id
/// [`task_id`] would not read whole, an empty title, or an id that a task file has on any local
/// branch or in the main worktree. The new file's path.
pub fn create_draft(
    program: &str,
    repo: &Path,
    id: &str,
    title: &str,
    cancel: &AtomicBool,
) -> Result<PathBuf, Refused> {
    use std::io::Write;
    let id = id.trim();
    let title: String = title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let title = title.trim();
    if task_id(id).as_deref() != Some(id) {
        return Err(Refused::BadId);
    }
    if title.is_empty() {
        return Err(Refused::NoTitle);
    }
    let ids = taken(program, repo, cancel).ok_or(Refused::NoGit)?;
    if let Some(place) = ids.get(id) {
        return Err(Refused::Taken(id.to_owned(), place.clone()));
    }
    let file = draft_file(id, title);
    let path = repo.join(TASKS).join(&file);
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => Refused::Exists(format!("{TASKS}/{file}")),
            _ => Refused::Write(e.to_string()),
        })?;
    out.write_all(draft_text(title).as_bytes())
        .map_err(|e| Refused::Write(e.to_string()))?;
    Ok(path)
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
    /// Labelled `role=controller`.
    pub controller: bool,
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
            controller: agent.role() == Some(crate::corral::Role::Controller),
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

/// The repository's controller: an agent labelled `role=controller` working in the main worktree
/// `repo` or a directory under it, the first of them not exited; none without one.
pub fn controller_for<'a>(repo: &Path, agents: &'a [Seen]) -> Option<&'a Seen> {
    agents.iter().find(|a| {
        a.controller
            && a.status != Status::Exited
            && a.cwd
                .as_deref()
                .is_some_and(|cwd| Path::new(cwd).starts_with(repo))
    })
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
    /// The repository's controller, on a card to review.
    pub controller: Option<Seen>,
    pub state: Option<(String, Tone)>,
    pub branch: Option<String>,
    pub lines: Option<(u64, u64)>,
    /// A task file not yet on main.
    pub draft: bool,
    /// Its agent is waiting on the user, or its task file says what the user is waited on for.
    pub needs_you: bool,
    /// What its task file's `待用户：` line says the user is waited on for.
    pub asks: Option<String>,
    /// Wrapped up without being done, and why.
    pub dropped: Option<String>,
    /// When it last moved, for ordering.
    time: i64,
}

/// The words for lines added and deleted: a side with none is left out, never `+0` or `−0`.
pub fn line_words((added, deleted): (u64, u64)) -> (Option<String>, Option<String>) {
    (
        (added > 0).then(|| format!("+{added}")),
        (deleted > 0).then(|| format!("−{deleted}")),
    )
}

/// The board: the repository, and the cards in each column.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub name: String,
    pub repo: PathBuf,
    pub columns: [Vec<Card>; 5],
}

impl Board {
    pub fn cards(&self, column: Column) -> &[Card] {
        &self.columns[column as usize]
    }

    /// The cards a group lists: all of them, but DONE only its last few and any that need the
    /// user, unless `all` asks for every one.
    pub fn listed(&self, column: Column, all: bool) -> Vec<&Card> {
        let cards = self.cards(column).iter();
        if column != Column::Done || all {
            return cards.collect();
        }
        cards
            .enumerate()
            .filter(|(at, card)| *at < DONE_KEPT || card.needs_you)
            .map(|(_, card)| card)
            .collect()
    }

    /// The count a group's header shows: how many it lists, and of how many when that is not
    /// all of them (`5 / 73`).
    pub fn count(&self, column: Column, all: bool) -> String {
        let (listed, total) = (self.listed(column, all).len(), self.cards(column).len());
        if listed == total {
            total.to_string()
        } else {
            format!("{listed} / {total}")
        }
    }

    /// How many cards need the user.
    pub fn need_you(&self) -> usize {
        self.columns
            .iter()
            .flatten()
            .filter(|c| c.needs_you)
            .count()
    }
}

/// The board for `facts` and the agents now listed, at `now` (seconds since the epoch).
pub fn board(facts: &Facts, agents: &[Seen], now: f64) -> Board {
    let mut columns: [Vec<Card>; 5] = Default::default();
    let controller = controller_for(&facts.repo, agents);
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
            controller: controller.filter(|_| column == Column::ToReview).cloned(),
            state,
            branch: task.branch.clone().filter(|_| shows_branch),
            lines: matches!(column, Column::InProgress | Column::ToReview)
                .then(|| facts.lines.get(&task.id).copied())
                .flatten(),
            draft: false,
            needs_you: task.asks.is_some() || agent.is_some_and(|a| a.status == Status::Waiting),
            asks: task.asks.clone(),
            dropped: facts
                .dropped
                .get(&task.id)
                .filter(|_| column == Column::Done)
                .cloned(),
            time: time.unwrap_or(0),
        });
    }
    // Drafts are not dispatched: no agent, nothing on main to wait on the user.
    for task in &facts.drafts {
        columns[Column::Queued as usize].push(Card {
            id: task.id.clone(),
            title: task.title.clone(),
            file: task.file.clone(),
            column: Column::Queued,
            age: String::new(),
            agent: None,
            controller: None,
            state: None,
            branch: None,
            lines: None,
            draft: true,
            needs_you: false,
            asks: None,
            dropped: None,
            time: 0,
        });
    }
    for (column, cards) in Column::ALL.iter().zip(columns.iter_mut()) {
        // Queued in the order of their ids, drafts first; the rest most recent first.
        if *column == Column::Queued {
            cards.sort_by_key(|c| !c.draft);
        } else {
            cards.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| natural(&a.id, &b.id)));
        }
    }
    Board {
        name: facts.name.clone(),
        repo: facts.repo.clone(),
        columns,
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
                asks: None,
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
    fn a_line_at_the_head_says_what_the_user_is_waited_on_for() {
        let head = "# 任务：x\n\n依据：y\n依赖：P5-1\n待用户：实测 hover 的样子\n\n## 要做的\n";
        let task = parse("P5-2-x.md", head).unwrap();
        assert_eq!(task.asks.as_deref(), Some("实测 hover 的样子"));
        assert_eq!(
            parse("P5-2-x.md", "# 任务：x\n待用户: 回答 Q1\n")
                .unwrap()
                .asks
                .as_deref(),
            Some("回答 Q1")
        );
        // None at the head, though a section quotes it at a line's start.
        let quoted = "# 任务：x\n依据：y\n\n## 完成记录\n待用户：只是举例\n";
        assert_eq!(parse("P5-2-x.md", quoted).unwrap().asks, None);
        assert_eq!(parse("P5-2-x.md", "# 任务：x\n").unwrap().asks, None);
    }

    #[test]
    fn a_wrap_up_can_drop_its_task() {
        assert_eq!(
            dropped("收尾: P5-7 不做：已被 P5-9 取代"),
            Some("已被 P5-9 取代")
        );
        assert_eq!(dropped("收尾：P5-7 不做: 重复"), Some("重复"));
        assert_eq!(dropped("收尾: P5-7 做完了"), None);
        assert_eq!(dropped("合并 P5-7：不做：x"), None);
        assert_eq!(wrapped_id("收尾: P5-7 不做：x"), Some("P5-7"));
    }

    #[test]
    fn a_side_with_no_lines_is_left_out() {
        assert_eq!(line_words((3, 0)), (Some("+3".into()), None));
        assert_eq!(line_words((0, 2)), (None, Some("−2".into())));
        assert_eq!(line_words((4, 1)), (Some("+4".into()), Some("−1".into())));
    }

    fn bare(id: &str, asks: Option<&str>) -> Task {
        Task {
            id: id.into(),
            title: id.into(),
            file: format!("{id}-x.md"),
            worktree: None,
            branch: None,
            depends: None,
            asks: asks.map(str::to_owned),
        }
    }

    #[test]
    fn cards_that_need_the_user_say_so_and_stay_in_sight() {
        // Seven done, the oldest waiting on the user; one queued whose agent is waiting; a draft.
        let mut facts = Facts {
            tasks: (1..=7)
                .map(|n| bare(&format!("P1-{n}"), (n == 1).then_some("看一眼")))
                .chain([bare("P2-1", None), bare("P2-2", None)])
                .collect(),
            drafts: vec![draft("P3-1-新的.md", "# 任务：新的活\n")],
            ..Default::default()
        };
        for n in 1..=7 {
            facts.wrapped.insert(format!("P1-{n}"), 100 + n);
        }
        facts.dropped.insert("P1-7".into(), "重复了".into());
        let agents = [seen("p/dev", None, Some("P2-2"), Status::Waiting)];
        let board = board(&facts, &agents, 1_000.0);
        let done: Vec<&str> = board
            .listed(Column::Done, false)
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(done, ["P1-7", "P1-6", "P1-5", "P1-4", "P1-3", "P1-1"]);
        // The header counts what is listed, of all.
        assert_eq!(board.count(Column::Done, false), "6 / 7");
        assert_eq!(board.count(Column::Done, true), "7");
        assert_eq!(board.listed(Column::Done, true).len(), 7);
        let card = |id: &str| {
            Column::ALL
                .iter()
                .flat_map(|c| board.cards(*c))
                .find(|c| c.id == id)
                .unwrap()
        };
        assert!(card("P1-1").needs_you);
        assert_eq!(card("P1-1").asks.as_deref(), Some("看一眼"));
        assert_eq!(card("P1-7").dropped.as_deref(), Some("重复了"));
        assert_eq!(card("P1-6").dropped, None);
        // The waiting agent starts its task, and it needs the user.
        assert_eq!(card("P2-2").column, Column::InProgress);
        assert!(card("P2-2").needs_you);
        assert_eq!(card("P2-2").asks, None);
        assert!(!card("P2-1").needs_you);
        assert_eq!(board.need_you(), 2);
        // The draft first in QUEUED.
        let queued: Vec<(&str, bool)> = board
            .cards(Column::Queued)
            .iter()
            .map(|c| (c.id.as_str(), c.draft))
            .collect();
        assert_eq!(queued, [("P3-1", true), ("P2-1", false)]);
    }

    #[test]
    fn a_draft_without_an_id_goes_by_its_name() {
        let task = draft("想法.md", "随手记的\n");
        assert_eq!((task.id.as_str(), task.title.as_str()), ("", "想法"));
        let titled = draft("想法.md", "# 任务：整理右侧栏\n");
        assert_eq!(titled.title, "整理右侧栏");
        assert_eq!(draft("P9-1-x.md", "# 任务：y\n").id, "P9-1");
    }

    #[test]
    fn a_new_task_is_offered_the_next_number_of_the_highest_series() {
        let ids = [
            "M0", "M3", "T76", "P1-T2", "P4-3", "P5-9", "P5-10-11", "P5-29c", "P5-29r2", "P5-3",
        ];
        assert_eq!(next_id(ids).as_deref(), Some("P5-30"));
        assert_eq!(next_id(["P5-9", "P10-1"]).as_deref(), Some("P10-2"));
        // Only ids of one part: the one after the highest.
        assert_eq!(next_id(["M0", "M3", "M12"]).as_deref(), Some("M13"));
        assert_eq!(next_id(["P1-T2"]), None);
        assert_eq!(next_id([]), None);
    }

    #[test]
    fn a_draft_file_is_named_by_its_id_and_title() {
        assert_eq!(draft_file("P5-30", "Kanban 新建"), "P5-30-Kanban-新建.md");
        // What a file name cannot hold, spaces and control characters become one `-`.
        assert_eq!(
            draft_file("P5-30", r#" a/b\c:d*e?f"g<h>i|j  k	l "#),
            "P5-30-a-b-c-d-e-f-g-h-i-j-k-l.md"
        );
        assert_eq!(draft_file("P5-30", "../x"), "P5-30-x.md");
        assert_eq!(draft_file("P5-30", "///"), "P5-30.md");
        // A title that would be read as more of the id stays out of the name.
        assert_eq!(draft_file("P5-30", "3D view"), "P5-30.md");
        assert_eq!(draft_file("P5-30", "T1 主题"), "P5-30.md");
        assert_eq!(draft_file("P5-30", "Tab 顺序"), "P5-30-Tab-顺序.md");
        // Long titles are cut.
        let long = draft_file("P5-30", &"长".repeat(200));
        assert_eq!(
            long.chars().count(),
            "P5-30-".len() + NAME_TITLE + ".md".len()
        );
        for name in ["P5-30-Kanban-新建.md", "P5-30.md"] {
            assert_eq!(task_id(name).as_deref(), Some("P5-30"));
        }
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
            controller: false,
        }
    }

    fn controller(name: &str, cwd: &str, status: Status) -> Seen {
        Seen {
            controller: true,
            ..seen(name, Some(cwd), None, status)
        }
    }

    #[test]
    fn the_controller_is_labelled_so_and_works_in_the_repository() {
        let repo = Path::new("/w/paddock");
        let main = controller("paddock/main", "/w/paddock/", Status::Idle);
        let below = controller("paddock/main-1", "/w/paddock/app", Status::Working);
        // Not a controller by its name alone; not one in another repository or a worktree.
        let unlabelled = seen("paddock/main", Some("/w/paddock"), None, Status::Idle);
        let elsewhere = controller("ranch/main", "/w/ranch", Status::Idle);
        let worktree = controller("x/main", "/w/paddock-worktrees/p5-1", Status::Idle);
        let exited = controller("paddock/main-2", "/w/paddock", Status::Exited);
        assert_eq!(
            controller_for(repo, &[unlabelled.clone(), main.clone()]),
            Some(&main)
        );
        assert_eq!(
            controller_for(repo, std::slice::from_ref(&below)),
            Some(&below)
        );
        // Of several, the first not exited.
        assert_eq!(
            controller_for(repo, &[exited.clone(), below.clone(), main.clone()]),
            Some(&below)
        );
        assert_eq!(
            controller_for(repo, &[unlabelled, elsewhere, worktree, exited]),
            None
        );
        assert_eq!(controller_for(repo, &[]), None);
    }

    #[test]
    fn only_a_card_to_review_shows_the_controller() {
        let place = |id: &str| Task {
            worktree: Some(format!("/w/{id}")),
            branch: Some(id.to_lowercase()),
            ..bare(id, None)
        };
        let mut facts = Facts {
            repo: PathBuf::from("/w/paddock"),
            tasks: ["P1-1", "P1-2", "P1-3", "P1-4", "P1-5"].map(place).to_vec(),
            ..Default::default()
        };
        // P1-1 queued; P1-2 in progress; P1-3 to review; P1-4 merged; P1-5 done.
        facts
            .worktrees
            .push(("/w/P1-2".into(), Some("p1-2".into())));
        facts.records.insert("P1-3".into());
        facts.merges.insert("P1-4".into(), 10);
        facts.wrapped.insert("P1-5".into(), 20);
        let main = controller("paddock/main", "/w/paddock", Status::Idle);
        let dev = seen("paddock/dev-a", None, Some("P1-3"), Status::Idle);
        let board = board(&facts, &[main.clone(), dev.clone()], 1_000.0);
        for column in Column::ALL {
            let cards = board.cards(column);
            assert_eq!(cards.len(), 1, "{column:?}");
            let want = (column == Column::ToReview).then_some(&main);
            assert_eq!(cards[0].controller.as_ref(), want, "{column:?}");
        }
        // The card's own agent stays the dev agent.
        assert_eq!(board.cards(Column::ToReview)[0].agent.as_ref(), Some(&dev));
        // No controller, no line.
        let without = super::board(&facts, std::slice::from_ref(&dev), 1_000.0);
        assert_eq!(without.cards(Column::ToReview)[0].controller, None);
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
