//! Read-only Git summaries of the agents' working trees for the Agents panel. From Saddle
//! `src/git.rs` at commit `df1c727`; since P5-13b with helpers for the Changes tab added.
use crate::command::{Output, run_bounded, run_without_env};
use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    Detached,
    Unknown,
}
/// Uncommitted changes of the working tree against HEAD, staged and unstaged together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    pub added: u64,
    pub deleted: u64,
    /// Files without line counts, such as binaries; not folded into the line totals.
    pub binary: u64,
}
/// Read-only Git state of one worktree; `None` fields could not be determined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub head: Head,
    /// Commits on HEAD beyond the base: local main, or the upstream when on main.
    pub ahead: Option<(u64, String)>,
    pub changes: Option<Changes>,
    pub untracked: Option<u64>,
}
/// Results by the cwd they were computed for; `None` means Git was unavailable there.
pub type Batch = Vec<(String, Option<Summary>)>;

struct Worktree<'a> {
    program: &'a str,
    top: PathBuf,
    cancel: &'a AtomicBool,
}
impl Worktree<'_> {
    fn run(&self, args: &[&str]) -> Option<Output> {
        git(self.program, &self.top, args, self.cancel)
    }
    // `-c` overrides that blank every configured filter driver, from any config scope, and mark
    // it required: whichever attributes file assigns it, Git then fails instead of running a
    // program. Built-in text/eol/binary attributes still apply. `None` if drivers can't be listed.
    fn blocked_filters(&self) -> Option<Vec<String>> {
        let output = self.run(&["config", "-z", "--name-only", "--get-regexp", r"^filter\."])?;
        // Exit 1 means no filter is configured at all.
        if !output.status.success() && output.status.code() != Some(1) {
            return None;
        }
        let mut drivers = BTreeSet::new();
        for name in output.stdout.split(|b| *b == 0).filter(|n| !n.is_empty()) {
            let name = std::str::from_utf8(name).ok()?;
            let Some((driver, _)) = name
                .strip_prefix("filter.")
                .and_then(|n| n.rsplit_once('.'))
            else {
                continue;
            };
            // `-c name=value` splits at the first '=', so such a driver cannot be overridden.
            if driver.contains('=') {
                return None;
            }
            drivers.insert(driver.to_owned());
        }
        Some(
            drivers
                .into_iter()
                .flat_map(|driver| {
                    ["clean=", "smudge=", "process=", "required=true"]
                        .map(|key| format!("filter.{driver}.{key}"))
                })
                .collect(),
        )
    }
    fn text(&self, args: &[&str]) -> Option<String> {
        self.run(args)
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
    }
    fn summary(&self) -> Summary {
        let (head, base) = self.head_and_base();
        let ahead = base.and_then(|(rev, name)| {
            let count = self.text(&["rev-list", "--count", &format!("{rev}..HEAD")])?;
            Some((count.parse().ok()?, name))
        });
        // A changed file that needs a blocked filter fails the diff, so the counts stay unknown.
        // Submodule worktrees are never entered (their own filters are not blocked here); a
        // submodule whose HEAD moved still counts as a gitlink change.
        let changes = self
            .blocked_filters()
            .and_then(|overrides| {
                let mut args: Vec<&str> =
                    overrides.iter().flat_map(|o| ["-c", o.as_str()]).collect();
                args.extend([
                    "diff-index",
                    "--numstat",
                    "-z",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--ignore-submodules=dirty",
                    "HEAD",
                    "--",
                ]);
                self.run(&args)
            })
            .filter(|o| o.status.success())
            .map(|o| {
                let mut changes = Changes {
                    added: 0,
                    deleted: 0,
                    binary: 0,
                };
                for record in o.stdout.split(|b| *b == 0) {
                    let record = String::from_utf8_lossy(record);
                    let mut fields = record.splitn(3, '\t');
                    match (fields.next(), fields.next()) {
                        (Some("-"), Some("-")) => changes.binary += 1,
                        (Some(added), Some(deleted)) => {
                            changes.added += added.parse::<u64>().unwrap_or(0);
                            changes.deleted += deleted.parse::<u64>().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
                changes
            });
        let untracked = self
            .run(&["ls-files", "--others", "--exclude-standard", "-z"])
            .filter(|o| o.status.success())
            .map(|o| o.stdout.iter().filter(|b| **b == 0).count() as u64);
        Summary {
            head,
            ahead,
            changes,
            untracked,
        }
    }
    /// HEAD, and the base its branch is measured against with the revision to name it by: local
    /// main, or the upstream when on main; none when detached or there is no such branch.
    fn head_and_base(&self) -> (Head, Option<(&'static str, String)>) {
        let head = match self.run(&["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            Some(o) if o.status.success() => {
                Head::Branch(String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
            }
            Some(o) if o.status.code() == Some(1) => Head::Detached,
            _ => Head::Unknown,
        };
        let base = match &head {
            Head::Branch(branch) if branch == "main" => self
                .text(&[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ])
                .map(|name| ("@{upstream}", name)),
            Head::Branch(_) => self
                .text(&[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    "refs/heads/main^{commit}",
                ])
                .map(|_| ("refs/heads/main", "main".to_owned())),
            _ => None,
        };
        (head, base)
    }
}

// Every call refuses lazy fetches of missing objects (Git 2.45+; older Git rejects the option,
// so the directory shows as unavailable), skips optional index writes and fsmonitor hooks, and
// inherits no GIT_* variable, which could redirect the repository, index or objects, inject
// config, or write traces.
pub(crate) fn git(program: &str, dir: &Path, args: &[&str], cancel: &AtomicBool) -> Option<Output> {
    let mut full = vec![
        "--no-lazy-fetch",
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
    ];
    full.extend(args);
    let inherited: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| name.as_encoded_bytes().starts_with(b"GIT_"))
        .collect();
    run_without_env(program, &full, Some(dir), &inherited, TIMEOUT, cancel).ok()
}

/// Like [`git`], with the same options, but keeping at most `limit` bytes of each stream and
/// saying why it failed: the Changes tab reads whole diffs, and shows the reason when it cannot.
pub(crate) fn git_bounded(
    program: &str,
    dir: &Path,
    args: &[&str],
    cancel: &AtomicBool,
    limit: usize,
) -> anyhow::Result<Output> {
    let mut full = vec![
        "--no-lazy-fetch",
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
    ];
    full.extend(args);
    let inherited: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| name.as_encoded_bytes().starts_with(b"GIT_"))
        .collect();
    run_bounded(
        program,
        &full,
        Some(dir),
        &inherited,
        TIMEOUT,
        cancel,
        limit,
    )
}

/// The `-c` overrides that block every configured filter driver for the worktree at `top`, as the
/// summaries do; `None` when the drivers cannot be listed.
pub(crate) fn filter_overrides(
    program: &str,
    top: &Path,
    cancel: &AtomicBool,
) -> Option<Vec<String>> {
    Worktree {
        program,
        top: top.to_owned(),
        cancel,
    }
    .blocked_filters()
}

/// HEAD of the worktree at `top`, and the base its branch is compared with, as the summaries
/// count commits ahead: the revision to give Git and the name to show.
pub fn head_and_base(
    program: &str,
    top: &Path,
    cancel: &AtomicBool,
) -> (Head, Option<(String, String)>) {
    let (head, base) = Worktree {
        program,
        top: top.to_owned(),
        cancel,
    }
    .head_and_base();
    (head, base.map(|(rev, name)| (rev.to_owned(), name)))
}

/// Summaries for each cwd; directories in the same worktree share one query per call.
/// A cancelled round returns nothing, so a partial round never reports directories as unavailable.
pub fn collect(program: &str, cwds: &[String], cancel: &AtomicBool) -> Batch {
    let mut shared: HashMap<PathBuf, Summary> = HashMap::new();
    let batch = cwds
        .iter()
        .map(|cwd| {
            let summary = locate(program, cwd, cancel).map(|top| {
                shared
                    .entry(top.clone())
                    .or_insert_with(|| {
                        Worktree {
                            program,
                            top,
                            cancel,
                        }
                        .summary()
                    })
                    .clone()
            });
            (cwd.clone(), summary)
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        Vec::new()
    } else {
        batch
    }
}

fn locate(program: &str, cwd: &str, cancel: &AtomicBool) -> Option<PathBuf> {
    if !Path::new(cwd).is_absolute() {
        return None;
    }
    let output = git(
        program,
        Path::new(cwd),
        &["rev-parse", "--show-toplevel"],
        cancel,
    )
    .filter(|o| o.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim_end_matches('\n').into())
}

/// The worktree top and the repository directory its worktrees share, both canonical; the
/// second is the same for every worktree and subdirectory of one repository.
pub fn repository(program: &str, cwd: &str, cancel: &AtomicBool) -> Option<(PathBuf, PathBuf)> {
    if !Path::new(cwd).is_absolute() {
        return None;
    }
    let output = git(
        program,
        Path::new(cwd),
        &[
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-common-dir",
        ],
        cancel,
    )
    .filter(|o| o.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines().map(PathBuf::from);
    let (top, common) = (lines.next()?, lines.next()?);
    Some((
        top.canonicalize().unwrap_or(top),
        common.canonicalize().unwrap_or(common),
    ))
}

/// Re-reads the watched directories in the background every `every`, and at once when they change.
pub struct Poller {
    pub updates: mpsc::Receiver<Batch>,
    watched: Vec<String>,
    wake: mpsc::Sender<Vec<String>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Poller {
    pub fn start(program: String, every: Duration) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            let mut cwds = Vec::new();
            loop {
                match wait.recv_timeout(every) {
                    Ok(next) => cwds = next,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                if let Some(latest) = wait.try_iter().last() {
                    cwds = latest;
                }
                if quitting.load(Ordering::Relaxed) {
                    break;
                }
                if cwds.is_empty() {
                    continue;
                }
                let batch = collect(&program, &cwds, &quitting);
                if quitting.load(Ordering::Relaxed) || tx.send(batch).is_err() {
                    break;
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
    pub fn watch(&mut self, cwds: Vec<String>) {
        if cwds != self.watched {
            self.watched.clone_from(&cwds);
            let _ = self.wake.send(cwds);
        }
    }
}
impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.wake.send(Vec::new());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
