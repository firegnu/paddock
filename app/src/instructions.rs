//! Each agent's global instructions file, which Settings › Instructions shows and edits (DESIGN
//! §13 P5-119): one main file per agent paddock hosts, where the agent reads it when a session
//! starts. Nothing else of the agent's is read: not what the file pulls in with `@`, not its
//! memories or settings. A save backs the old file up first, refuses a file that changed since it
//! was read, writes where a link leads rather than over the link, and replaces the file whole.
use crate::activity;
use similar::{ChangeTag, TextDiff};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    time::SystemTime,
};

/// The agents in Settings' order: those paddock hosts.
pub const KINDS: &[&str] = crate::agent_shell::KINDS;

/// Where `kind` reads its global instructions, under `home` or the folder its environment
/// variable names (`var` looks one up); none for a kind paddock doesn't know.
pub fn path(kind: &str, home: &Path, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let dir = |name: &str, default: &str| {
        var(name)
            .filter(|dir| !dir.is_empty())
            .map_or_else(|| home.join(default), PathBuf::from)
    };
    Some(match kind {
        "claude" => dir("CLAUDE_CONFIG_DIR", ".claude").join("CLAUDE.md"),
        "codex" => dir("CODEX_HOME", ".codex").join("AGENTS.md"),
        "pi" => dir("PI_CODING_AGENT_DIR", ".pi/agent").join("AGENTS.md"),
        "omp" => home.join(".omp/agent/AGENTS.md"),
        "amp" => home.join(".config/amp/AGENTS.md"),
        "agy" => home.join(".gemini/GEMINI.md"),
        _ => return None,
    })
}

/// Where a link leads, link after link; `path` itself when it is no link. The place need not
/// exist: a link to a file not made yet leads to where it would be.
pub fn target(path: &Path) -> PathBuf {
    let mut at = path.to_path_buf();
    // As many links as the system follows before it gives up.
    for _ in 0..40 {
        let Ok(next) = fs::read_link(&at) else {
            break;
        };
        at = match at.parent() {
            Some(dir) if next.is_relative() => dir.join(next),
            _ => next,
        };
    }
    at
}

/// One agent's file as read.
#[derive(Clone, Debug)]
pub struct File {
    pub kind: &'static str,
    /// Where the agent reads it.
    pub path: PathBuf,
    /// What is read and written: where `path` leads when it is a link.
    pub target: PathBuf,
    pub state: State,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Not there yet.
    Missing,
    Text {
        text: String,
        /// In bytes.
        size: u64,
        modified: Option<SystemTime>,
    },
    /// There, but not readable as text, and why.
    Unreadable(String),
}

impl File {
    /// Whether there is a file, readable or not.
    pub fn exists(&self) -> bool {
        self.state != State::Missing
    }
}

/// `kind`'s file at `path`, as it is now.
pub fn read(kind: &'static str, path: PathBuf) -> File {
    let target = target(&path);
    let state = match fs::read(&target) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => {
                let metadata = fs::metadata(&target).ok();
                State::Text {
                    size: text.len() as u64,
                    modified: metadata.and_then(|m| m.modified().ok()),
                    text,
                }
            }
            Err(_) => State::Unreadable("it isn't UTF-8 text".into()),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => State::Missing,
        Err(error) => State::Unreadable(error.to_string()),
    };
    File {
        kind,
        path,
        target,
        state,
    }
}

/// Every agent's file under this machine's home folder and environment, in [`KINDS`] order;
/// none without a home folder.
pub fn read_all() -> Vec<File> {
    let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    KINDS
        .iter()
        .filter_map(|kind| {
            Some(read(
                kind,
                path(kind, &home, |name| std::env::var_os(name))?,
            ))
        })
        .collect()
}

/// Where a save puts the old file: `~/.config/paddock/backups`.
pub fn backups(home: &Path) -> PathBuf {
    home.join(".config/paddock/backups")
}

/// The backup of `kind`'s `target` made at `now`: the agent, the file's name and the local time,
/// `claude-CLAUDE.md.20261016-105230`.
pub fn backup_name(kind: &str, target: &Path, now: SystemTime) -> String {
    let file = target
        .file_name()
        .map_or_else(|| "file".into(), |name| name.to_string_lossy());
    let at = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let offset = activity::local_offset(at);
    let (year, month, day) = activity::date(activity::day_of(at, offset));
    let of_day = (at + offset).rem_euclid(86_400);
    format!(
        "{kind}-{file}.{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// Why a save didn't write.
#[derive(Debug)]
pub enum SaveError {
    /// The file no longer holds what was read when editing began.
    Changed,
    /// Couldn't back up or write, and why; the file is as it was.
    Failed(String),
}

/// Whether `target` still holds `opened`, what was read when editing began.
pub fn unchanged(target: &Path, opened: &str) -> bool {
    fs::read(target).is_ok_and(|bytes| bytes == opened.as_bytes())
}

/// Writes `text` over `target`, which must still hold `opened`: first the file as it is copied to
/// `backup` (or, when that is taken, `backup-2` and on; never over another backup), then `text`
/// written to a file beside it and put in its place, with the old file's permissions. Nothing is
/// written when the backup fails. The backup made.
pub fn save(target: &Path, text: &str, opened: &str, backup: &Path) -> Result<PathBuf, SaveError> {
    let failed = |what: &str, error: io::Error| SaveError::Failed(format!("{what}: {error}"));
    let current = fs::read(target).map_err(|_| SaveError::Changed)?;
    if current != opened.as_bytes() {
        return Err(SaveError::Changed);
    }
    let permissions = fs::metadata(target)
        .map_err(|e| failed("Couldn't read the file", e))?
        .permissions();
    let backup = keep(&current, backup).map_err(|e| failed("Couldn't back it up", e))?;
    replace(target, text.as_bytes(), permissions).map_err(|e| failed("Couldn't write it", e))?;
    Ok(backup)
}

/// `bytes` in a new file at `path`, or the first of `path-2`, `path-3`… not taken.
fn keep(bytes: &[u8], path: &Path) -> io::Result<PathBuf> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    for n in 1..=100 {
        let at = if n == 1 {
            path.to_path_buf()
        } else {
            let mut name = path.as_os_str().to_owned();
            name.push(format!("-{n}"));
            PathBuf::from(name)
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&at)
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                return Ok(at);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many backups with that name",
    ))
}

/// `bytes` in place of `target`, whole or not at all: written to a new file in the same folder,
/// given `permissions`, then renamed over it.
fn replace(target: &Path, bytes: &[u8], permissions: fs::Permissions) -> io::Result<()> {
    let dir = target.parent().unwrap_or(Path::new("."));
    let name = target
        .file_name()
        .map_or_else(|| "file".into(), |name| name.to_string_lossy());
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let temporary = dir.join(format!(
        ".{name}.paddock-saving-{}-{nanos}",
        std::process::id()
    ));
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::set_permissions(&temporary, permissions)?;
        fs::rename(&temporary, target)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}

/// An empty file at `target`, its folders made first; never over one that is there.
pub fn create(target: &Path) -> io::Result<()> {
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map(drop)
}

/// `3.7 KB`, `512 bytes`, `Empty`; thousands, as Finder counts.
pub fn size_label(size: u64) -> String {
    match size {
        0 => "Empty".into(),
        1 => "1 byte".into(),
        n if n < 1000 => format!("{n} bytes"),
        n if n < 1_000_000 => format!("{:.1} KB", n as f64 / 1000.0),
        n => format!("{:.1} MB", n as f64 / 1_000_000.0),
    }
}

/// `changed Oct 15`, with the year when it isn't this one.
pub fn changed_label(modified: SystemTime) -> String {
    let at = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (year, month, day) = activity::date(activity::day_of(at, activity::local_offset(at)));
    let (this_year, _, _) = activity::date(activity::today());
    let month = activity::MONTHS[month as usize - 1];
    if year == this_year {
        format!("changed {month} {day}")
    } else {
        format!("changed {month} {day}, {year}")
    }
}

/// How a line of the changes reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Same,
    Added,
    Removed,
}

/// A row of the changes before a save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffRow {
    /// A line: its number (in the old file for a removed one, else in the new) and its text.
    Line {
        number: usize,
        sign: Sign,
        text: String,
    },
    /// Unchanged lines left out.
    Gap,
}

/// Unchanged lines kept around each change.
const CONTEXT: usize = 3;

/// The changes from `old` to `new` line by line, unchanged lines further than [`CONTEXT`] from a
/// change folded into a gap.
pub fn diff(old: &str, new: &str) -> Vec<DiffRow> {
    let diff = TextDiff::from_lines(old, new);
    let groups = diff.grouped_ops(CONTEXT);
    let lines = diff.old_slices().len();
    let mut rows = Vec::new();
    for (n, group) in groups.iter().enumerate() {
        let start = group.first().map_or(0, |op| op.old_range().start);
        if n > 0 || start > 0 {
            rows.push(DiffRow::Gap);
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let (index, sign) = match change.tag() {
                    ChangeTag::Equal => (change.new_index(), Sign::Same),
                    ChangeTag::Insert => (change.new_index(), Sign::Added),
                    ChangeTag::Delete => (change.old_index(), Sign::Removed),
                };
                rows.push(DiffRow::Line {
                    number: index.unwrap_or(0) + 1,
                    sign,
                    text: change.value().trim_end_matches(['\n', '\r']).to_owned(),
                });
            }
        }
    }
    let end = groups
        .last()
        .and_then(|group| group.last())
        .map_or(lines, |op| op.old_range().end);
    if !groups.is_empty() && end < lines {
        rows.push(DiffRow::Gap);
    }
    rows
}

/// `1 line added`, `2 lines added, 1 removed`, `No changes`.
pub fn summary(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let (mut added, mut removed) = (0, 0);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    let lines = |n: usize| if n == 1 { "line" } else { "lines" };
    match (added, removed) {
        (0, 0) => "No changes".into(),
        (a, 0) => format!("{a} {} added", lines(a)),
        (0, r) => format!("{r} {} removed", lines(r)),
        (a, r) => format!("{a} {} added, {r} removed", lines(a)),
    }
}
