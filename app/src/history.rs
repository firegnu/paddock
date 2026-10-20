//! The Changes tab's History scope: the current branch's commits, as Git lists them for
//! `git log --graph` (topological order, merged branches included), and the first parent's diff
//! for one of them. Only Git's read-only commands are run.
use crate::{diff, git, kanban};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// Commits read at once, and again each time the list reaches its end.
pub const BATCH: usize = 200;
/// The most a read's output may take before the tab says it is too large to read.
const OUTPUT_LIMIT: usize = 16 << 20;

/// A commit's place in the graph, worked out from the commits above it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphRow {
    /// The column the commit's dot sits in.
    pub lane: usize,
    /// Columns with a line entering this row from above.
    pub top: Vec<usize>,
    /// Columns above that end at this commit and curve into its dot.
    pub joins: Vec<usize>,
    /// Columns this commit opens for its other parents, curving out of its dot.
    pub forks: Vec<usize>,
    /// Columns with a line leaving this row below.
    pub bottom: Vec<usize>,
    /// How many columns this row needs.
    pub lanes: usize,
}

/// What a subject says about a task: its id when one is at the start (after `合并`, `收尾:`,
/// `任务:` and the like), the rest to show beside the id, and whether it is a wrap-up commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Label {
    pub id: Option<String>,
    pub rest: String,
    pub wrap_up: bool,
}

impl Label {
    /// `合并 P5-1：标题` -> id `P5-1`, rest `标题`; `收尾: P5-2 做完了` -> id `P5-2`, rest
    /// `做完了`, a wrap-up; a subject with no id at its start keeps all of its words.
    pub fn of(subject: &str) -> Self {
        let wrap_up = subject.starts_with("收尾:") || subject.starts_with("收尾：");
        let mut rest = subject;
        if let Some(after) = [
            "收尾:",
            "收尾：",
            "合并 ",
            "合并",
            "任务:",
            "任务：",
            "任务 ",
        ]
        .iter()
        .find_map(|prefix| rest.strip_prefix(prefix))
        {
            rest = after.trim_start();
        }
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(rest.len());
        let candidate = rest[..end].trim_end_matches('-');
        let Some(id) = kanban::task_id(candidate).filter(|id| id == candidate) else {
            return Label {
                id: None,
                rest: subject.to_owned(),
                wrap_up,
            };
        };
        let after = rest[candidate.len()..].trim_start();
        let after = after.trim_start_matches(['：', ':', '-', '—', '·']).trim();
        Label {
            id: Some(id),
            rest: after.to_owned(),
            wrap_up,
        }
    }
}

/// One commit as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub parents: Vec<String>,
    pub author: String,
    /// The author's time, in seconds since the Unix epoch.
    pub time: i64,
    pub subject: String,
    pub label: Label,
    pub graph: GraphRow,
}

/// The commits of a branch, and what a task action needs besides them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    pub top: String,
    pub head: String,
    /// The base `Branch vs main` compares with, when there is one.
    pub base: Option<String>,
    pub commits: Vec<Commit>,
    /// Git had more commits than were read.
    pub more: bool,
    /// The widest row's column count.
    pub lanes: usize,
    /// Task files by id, only those whose id has exactly one file.
    pub tasks: HashMap<String, PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    NotRepository,
    NoCommits { head: String },
    Failed(String),
    Commits(History),
}

/// One commit's changes as the diff view draws them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub top: String,
    pub files: Vec<diff::File>,
}

impl Diff {
    pub fn added(&self) -> u32 {
        self.files.iter().map(|f| f.added).sum()
    }

    pub fn deleted(&self) -> u32 {
        self.files.iter().map(|f| f.deleted).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffRead {
    Failed(String),
    Diff(Diff),
}

/// The commits of the branch `cwd` is on, at most `limit` of them, with Git's read-only commands.
/// A cancelled read returns `None`.
pub fn read(program: &str, cwd: &str, limit: usize, cancel: &AtomicBool) -> Option<Read> {
    let read = read_inner(program, cwd, limit, cancel);
    (!cancel.load(Ordering::Relaxed)).then_some(read)
}

fn read_inner(program: &str, cwd: &str, limit: usize, cancel: &AtomicBool) -> Read {
    let dir = Path::new(cwd);
    if !dir.is_absolute() || !dir.is_dir() {
        return Read::NotRepository;
    }
    let run =
        |dir: &Path, args: &[&str]| git::git_bounded(program, dir, args, cancel, OUTPUT_LIMIT);
    let top = match run(dir, &["rev-parse", "--show-toplevel"]) {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            PathBuf::from(text.trim_end_matches('\n'))
        }
        // Inside `.git`, or not a repository at all.
        Ok(_) => return Read::NotRepository,
        Err(error) => return Read::Failed(format!("{error:#}")),
    };
    let text = |args: &[&str]| -> Option<String> {
        run(&top, args)
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
    };
    let (head, base) = git::head_and_base(program, &top, cancel);
    let head_name = match head {
        git::Head::Branch(name) => name,
        _ => text(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "HEAD".into()),
    };
    match run(&top, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]) {
        Ok(o) if o.status.success() => {}
        Ok(o) if o.status.code() == Some(1) => return Read::NoCommits { head: head_name },
        Ok(o) => return Read::Failed(diff::failure(&o)),
        Err(error) => return Read::Failed(format!("{error:#}")),
    }
    let limit = limit.max(1);
    let max = limit.saturating_add(1).to_string();
    let args = [
        "-c",
        "core.quotePath=false",
        "log",
        "--topo-order",
        "--no-color",
        "--no-decorate",
        "--no-show-signature",
        "-z",
        "--format=%H%x1f%h%x1f%P%x1f%an%x1f%at%x1f%s%x1e",
        "--max-count",
        max.as_str(),
        "HEAD",
        "--",
    ];
    let output = match run(&top, &args) {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => return Read::Failed(diff::failure(&o)),
        Err(error) => return Read::Failed(format!("{error:#}")),
    };
    let mut commits = parse_log(&String::from_utf8_lossy(&output));
    let more = commits.len() > limit;
    if more {
        commits.truncate(limit);
    }
    let (rows, lanes) = layout(&commits, !more);
    for (commit, row) in commits.iter_mut().zip(rows) {
        commit.graph = row;
    }
    Read::Commits(History {
        top: top.to_string_lossy().into_owned(),
        head: head_name,
        base: base.map(|(_, name)| name),
        commits,
        more,
        lanes,
        tasks: task_files(&top),
    })
}

/// The commits in `git log -z --format=…` output, in its order.
fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\0')
        .filter_map(|record| {
            let record = record
                .trim_end_matches('\n')
                .trim_end_matches('\x1e')
                .trim_start_matches('\n');
            if record.is_empty() {
                return None;
            }
            let mut fields = record.split('\x1f');
            let hash = fields.next()?.to_owned();
            let short = match fields.next()? {
                "" => hash.get(..7).unwrap_or(&hash).to_owned(),
                short => short.to_owned(),
            };
            let parents = fields
                .next()?
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            let author = fields.next()?.to_owned();
            let time = fields.next()?.parse().unwrap_or(0);
            let subject = fields.next()?.to_owned();
            Some(Commit {
                hash,
                short,
                parents,
                author,
                time,
                label: Label::of(&subject),
                subject,
                graph: GraphRow::default(),
            })
        })
        .collect()
}

/// One commit's changes against its first parent (the whole of what a merge brought in), or
/// against the empty tree for a root commit. A cancelled read returns `None`.
pub fn diff(program: &str, top: &str, commit: &Commit, cancel: &AtomicBool) -> Option<DiffRead> {
    let read = diff_inner(program, top, commit, cancel);
    (!cancel.load(Ordering::Relaxed)).then_some(read)
}

fn diff_inner(program: &str, top: &str, commit: &Commit, cancel: &AtomicBool) -> DiffRead {
    let dir = Path::new(top);
    if !dir.is_absolute() || !dir.is_dir() {
        return DiffRead::Failed("The repository is gone".into());
    }
    let run = |args: &[&str]| git::git_bounded(program, dir, args, cancel, OUTPUT_LIMIT);
    let text = |args: &[&str]| -> Option<String> {
        run(args)
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
    };
    let from = match commit.parents.first() {
        Some(parent) => parent.clone(),
        None => match text(&["hash-object", "-t", "tree", "--stdin"]) {
            Some(empty) => empty,
            None => return DiffRead::Failed("Couldn’t name the empty tree".into()),
        },
    };
    let args = [
        "-c",
        "core.quotePath=false",
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--no-relative",
        "--full-index",
        "--submodule=short",
        "--ignore-submodules=dirty",
        "--find-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        from.as_str(),
        commit.hash.as_str(),
        "--",
    ];
    let patch = match run(&args) {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => return DiffRead::Failed(diff::failure(&o)),
        Err(error) => return DiffRead::Failed(format!("{error:#}")),
    };
    let mut files = diff::parse(&String::from_utf8_lossy(&patch));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    DiffRead::Diff(Diff {
        top: top.to_owned(),
        files,
    })
}

/// Where each commit sits, and how many columns there are. `complete` says no older commits
/// remain: lines to parents Git cannot show (a shallow clone's boundary) then stop.
fn layout(commits: &[Commit], complete: bool) -> (Vec<GraphRow>, usize) {
    let known: HashSet<&str> = commits.iter().map(|c| c.hash.as_str()).collect();
    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());
    for (i, commit) in commits.iter().enumerate() {
        let expected = lanes
            .iter()
            .position(|lane| lane.as_deref() == Some(commit.hash.as_str()));
        let top: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter_map(|(lane, expected)| expected.is_some().then_some(lane))
            .collect();
        let lane = expected.unwrap_or_else(|| match lanes.iter().position(Option::is_none) {
            Some(lane) => lane,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        });
        let own = lane;
        let joins: Vec<usize> = top
            .iter()
            .copied()
            .filter(|&lane| lane != own && lanes[lane].as_deref() == Some(commit.hash.as_str()))
            .collect();
        for &lane in &joins {
            lanes[lane] = None;
        }
        lanes[lane] = None;
        let mut forks = Vec::new();
        if let Some(first) = commit.parents.first() {
            lanes[lane] = Some(first.clone());
        }
        for parent in commit.parents.iter().skip(1) {
            let fork = if let Some(fork) = lanes
                .iter()
                .position(|lane| lane.as_deref() == Some(parent.as_str()))
            {
                fork
            } else {
                let fork = match lanes.iter().position(Option::is_none) {
                    Some(fork) => fork,
                    None => {
                        lanes.push(None);
                        lanes.len() - 1
                    }
                };
                lanes[fork] = Some(parent.clone());
                fork
            };
            forks.push(fork);
        }
        // A shallow clone's boundary names parents whose objects are not here: no line runs on
        // into nothing at the end of a complete history.
        if complete && i + 1 == commits.len() {
            for lane in &mut lanes {
                if lane.as_deref().is_some_and(|hash| !known.contains(hash)) {
                    *lane = None;
                }
            }
        }
        let bottom: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter_map(|(lane, expected)| expected.is_some().then_some(lane))
            .collect();
        let count = top
            .iter()
            .chain(&bottom)
            .copied()
            .max()
            .unwrap_or(0)
            .max(lane)
            + 1;
        rows.push(GraphRow {
            lane,
            top,
            joins,
            forks,
            bottom,
            lanes: count,
        });
    }
    let lanes = rows.iter().map(|row| row.lanes).max().unwrap_or(0).max(1);
    (rows, lanes)
}

/// Task files by id, only those whose id has exactly one file under `top/docs/任务`.
fn task_files(top: &Path) -> HashMap<String, PathBuf> {
    let mut found: HashMap<String, Option<PathBuf>> = HashMap::new();
    let Ok(entries) = fs::read_dir(top.join(kanban::TASKS)) else {
        return HashMap::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = kanban::task_id(name) else {
            continue;
        };
        match found.entry(id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                // Two files with one id: neither is the one the button should open.
                entry.insert(None);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Some(path));
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, path)| path.map(|path| (id, path)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, parents: &[&str]) -> Commit {
        Commit {
            hash: hash.into(),
            short: hash.into(),
            parents: parents.iter().map(|p| (*p).to_owned()).collect(),
            author: String::new(),
            time: 0,
            subject: String::new(),
            label: Label::default(),
            graph: GraphRow::default(),
        }
    }

    #[test]
    fn subjects_name_the_task_after_the_usual_prefixes() {
        assert_eq!(
            Label::of("合并 P5-1：标题"),
            Label {
                id: Some("P5-1".into()),
                rest: "标题".into(),
                wrap_up: false
            }
        );
        assert_eq!(
            Label::of("收尾: P5-2 做完了"),
            Label {
                id: Some("P5-2".into()),
                rest: "做完了".into(),
                wrap_up: true
            }
        );
        assert_eq!(
            Label::of("P5-3：直接写的"),
            Label {
                id: Some("P5-3".into()),
                rest: "直接写的".into(),
                wrap_up: false
            }
        );
        assert_eq!(
            Label::of("任务: P5-4 标题"),
            Label {
                id: Some("P5-4".into()),
                rest: "标题".into(),
                wrap_up: false
            }
        );
        // The whole id, not the number before its suffix.
        assert_eq!(Label::of("合并 P5-24a：返工").id.as_deref(), Some("P5-24a"));
        // No id: the subject stays as it was, prefix and all.
        assert_eq!(
            Label::of("收尾了没有编号"),
            Label {
                id: None,
                rest: "收尾了没有编号".into(),
                wrap_up: false
            }
        );
        assert_eq!(Label::of("Fix the thing").id, None);
    }

    #[test]
    fn a_merge_opens_a_lane_that_joins_back_at_the_base() {
        let commits = [
            commit("M", &["A", "F1"]),
            commit("F1", &["F0"]),
            commit("F0", &["B"]),
            commit("A", &["B"]),
            commit("B", &[]),
        ];
        let (rows, lanes) = layout(&commits, true);
        assert_eq!(lanes, 2);
        assert_eq!(
            rows[0],
            GraphRow {
                lane: 0,
                top: vec![],
                joins: vec![],
                forks: vec![1],
                bottom: vec![0, 1],
                lanes: 2,
            }
        );
        assert_eq!(rows[1].lane, 1);
        assert_eq!(rows[1].top, vec![0, 1]);
        assert_eq!(rows[3].lane, 0);
        assert_eq!(
            rows[4],
            GraphRow {
                lane: 0,
                top: vec![0, 1],
                joins: vec![1],
                forks: vec![],
                bottom: vec![],
                lanes: 2,
            }
        );
    }

    #[test]
    fn two_branches_open_at_once_take_the_next_columns() {
        let commits = [
            commit("M", &["A", "B", "C"]),
            commit("A", &["base"]),
            commit("B", &["base"]),
            commit("C", &["base"]),
            commit("base", &[]),
        ];
        let (rows, lanes) = layout(&commits, true);
        assert_eq!(lanes, 3);
        assert_eq!(rows[0].bottom, vec![0, 1, 2]);
        assert_eq!(rows[0].forks, vec![1, 2]);
        assert_eq!(rows[1].top, vec![0, 1, 2]);
        assert_eq!(rows[2].top, vec![0, 1, 2]);
        assert_eq!(rows[3].top, vec![0, 1, 2]);
        assert_eq!(rows[4].joins, vec![1, 2]);
    }

    #[test]
    fn a_second_merge_opens_the_next_free_column() {
        let commits = [
            commit("M2", &["A1", "G1"]),
            commit("G1", &["G0"]),
            commit("G0", &["base"]),
            commit("A1", &["A0", "F1"]),
            commit("F1", &["F0"]),
            commit("F0", &["base"]),
            commit("A0", &["base"]),
            commit("base", &[]),
        ];
        let (rows, lanes) = layout(&commits, true);
        assert_eq!(lanes, 3);
        assert_eq!(rows[0].forks, vec![1]);
        assert_eq!(rows[3].lane, 0);
        assert_eq!(rows[3].forks, vec![2]);
        assert_eq!(rows[7].lane, 0);
        assert_eq!(rows[7].joins, vec![1, 2]);
        assert_eq!(rows[7].bottom, Vec::<usize>::new());
    }
}
