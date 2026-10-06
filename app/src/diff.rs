//! What the Changes tab shows: a worktree's changes read from Git with read-only commands and
//! parsed into files, hunks and lines. Pure apart from [`read`], which runs Git and reads the
//! worktree's files, so the parsing can be tested on text.
use crate::git::{self, Head};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read as _,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

/// The most a diff's output may take before the tab says it is too large to read.
const OUTPUT_LIMIT: usize = 48 << 20;
/// An untracked file larger than this is listed with its size, not read.
const UNTRACKED_LIMIT: u64 = 1 << 20;
/// A file whose lines are not counted for the trailing "N more lines", past this size.
const COUNT_LIMIT: u64 = 4 << 20;
/// How many binary files have their sizes looked up, each one more Git call.
const SIZED_BINARIES: usize = 16;
/// What Git looks at to call a file binary: a NUL in its first 8000 bytes.
const BINARY_PROBE: usize = 8000;

/// Which changes the tab shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Staged, unstaged and untracked, against HEAD.
    #[default]
    Uncommitted,
    /// Everything since the branch left its base, the uncommitted included.
    Branch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Modified,
    Added,
    Deleted,
    Renamed,
}

impl Status {
    pub fn letter(self) -> &'static str {
        match self {
            Status::Modified => "M",
            Status::Added => "A",
            Status::Deleted => "D",
            Status::Renamed => "R",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Context,
    Added,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    /// Its number on the old side; 0 for an added line.
    pub old: u32,
    /// Its number on the new side; 0 for a deleted line.
    pub new: u32,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_len: u32,
    pub new_start: u32,
    pub new_len: u32,
    /// What Git puts after the second `@@`: the enclosing function, as it guesses it.
    pub context: String,
    pub lines: Vec<Line>,
}

impl Hunk {
    /// `@@ -88,9 +88,14 @@`, as Git wrote it.
    pub fn header(&self) -> String {
        format!(
            "@@ -{} +{} @@",
            range(self.old_start, self.old_len),
            range(self.new_start, self.new_len)
        )
    }

    /// Its first line on the new side, and the one after its last: a hunk that adds nothing
    /// starts after the line Git names.
    pub fn new_lines(&self) -> (u32, u32) {
        let first = if self.new_len == 0 {
            self.new_start + 1
        } else {
            self.new_start
        };
        (first, first + self.new_len)
    }

    /// The same on the old side.
    pub fn old_lines(&self) -> (u32, u32) {
        let first = if self.old_len == 0 {
            self.old_start + 1
        } else {
            self.old_start
        };
        (first, first + self.old_len)
    }
}

fn range(start: u32, len: u32) -> String {
    if len == 1 {
        start.to_string()
    } else {
        format!("{start},{len}")
    }
}

/// Why a file has no lines to show, or what else to say about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// Its sizes before and after, when known.
    Binary(Option<u64>, Option<u64>),
    /// Untracked and too large to read: its size.
    TooLarge(u64),
    /// Only its mode changed.
    Mode(String, String),
    /// An empty file added or deleted, or a change Git shows no lines for.
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub path: String,
    /// Where it was before a rename.
    pub old_path: Option<String>,
    pub status: Status,
    pub added: u32,
    pub deleted: u32,
    pub hunks: Vec<Hunk>,
    pub note: Option<Note>,
    /// Its blob before, as Git names it, for a binary file's old size.
    pub blob: Option<String>,
    /// How many lines the file has now, for the unchanged lines after the last hunk; `None`
    /// when it is gone or was not counted.
    pub lines_now: Option<u32>,
}

impl File {
    fn new(path: String) -> Self {
        File {
            path,
            old_path: None,
            status: Status::Modified,
            added: 0,
            deleted: 0,
            hunks: Vec::new(),
            note: None,
            blob: None,
            lines_now: None,
        }
    }

    /// The directory part with its trailing slash, and the name.
    pub fn dir_and_name(&self) -> (&str, &str) {
        match self.path.rfind('/') {
            Some(i) => self.path.split_at(i + 1),
            None => ("", &self.path),
        }
    }

    /// Lines added and deleted together.
    pub fn changed(&self) -> u32 {
        self.added + self.deleted
    }

    /// A lock file or another file tools write: its diff waits until asked for.
    pub fn generated(&self) -> bool {
        const NAMES: [&str; 13] = [
            "Cargo.lock",
            "package-lock.json",
            "npm-shrinkwrap.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "bun.lock",
            "Gemfile.lock",
            "poetry.lock",
            "uv.lock",
            "composer.lock",
            "Podfile.lock",
            "flake.lock",
            "go.sum",
        ];
        NAMES.contains(&self.dir_and_name().1)
    }
}

/// What the tab found in a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    NotRepository,
    /// The Branch scope with nothing to compare with: detached, on main without an upstream, or
    /// no main.
    NoBase {
        head: String,
    },
    Failed(String),
    Changes(Changes),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    /// The worktree's top, which the paths are relative to.
    pub top: String,
    /// The branch, or the commit when detached.
    pub head: String,
    /// The base branch's name, where there is one.
    pub base: Option<String>,
    pub files: Vec<File>,
}

impl Changes {
    pub fn added(&self) -> u32 {
        self.files.iter().map(|f| f.added).sum()
    }

    pub fn deleted(&self) -> u32 {
        self.files.iter().map(|f| f.deleted).sum()
    }
}

/// The changes under `cwd` in `scope`, read with read-only Git commands. A cancelled read
/// returns `None`.
pub fn read(program: &str, cwd: &str, scope: Scope, cancel: &AtomicBool) -> Option<Read> {
    let read = read_inner(program, cwd, scope, cancel);
    (!cancel.load(Ordering::Relaxed)).then_some(read)
}

fn read_inner(program: &str, cwd: &str, scope: Scope, cancel: &AtomicBool) -> Read {
    let dir = Path::new(cwd);
    if !dir.is_absolute() || !dir.is_dir() {
        return Read::NotRepository;
    }
    let run =
        |dir: &Path, args: &[&str]| git::git_bounded(program, dir, args, cancel, OUTPUT_LIMIT);
    let top = match run(dir, &["rev-parse", "--show-toplevel"]) {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            std::path::PathBuf::from(text.trim_end_matches('\n'))
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
    let head_name = match &head {
        Head::Branch(name) => name.clone(),
        _ => text(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "HEAD".into()),
    };
    let born = text(&["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]).is_some();
    let from = match scope {
        Scope::Uncommitted if born => "HEAD".to_owned(),
        // No commit yet: everything is new, against the empty tree (which hashing nothing names).
        Scope::Uncommitted => match text(&["hash-object", "-t", "tree", "--stdin"]) {
            Some(empty) => empty,
            None => return Read::Failed("Couldn’t name the empty tree".into()),
        },
        Scope::Branch => {
            let Some((rev, _)) = &base else {
                return Read::NoBase { head: head_name };
            };
            match text(&["merge-base", rev, "HEAD"]) {
                Some(fork) => fork,
                None => return Read::NoBase { head: head_name },
            }
        }
    };
    // A changed file that needs a blocked filter fails the diff, which then says so.
    let Some(overrides) = git::filter_overrides(program, &top, cancel) else {
        return Read::Failed("Couldn’t list the repository’s filters".into());
    };
    let mut args: Vec<&str> = overrides.iter().flat_map(|o| ["-c", o.as_str()]).collect();
    args.extend([
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
        "--",
    ]);
    let patch = match run(&top, &args) {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => return Read::Failed(failure(&o)),
        Err(error) => return Read::Failed(format!("{error:#}")),
    };
    let mut files = parse(&String::from_utf8_lossy(&patch));
    let untracked = match run(&top, &["ls-files", "--others", "--exclude-standard", "-z"]) {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => return Read::Failed(failure(&o)),
        Err(error) => return Read::Failed(format!("{error:#}")),
    };
    for path in untracked.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(path).into_owned();
        files.push(untracked_file(&top, path));
    }
    let mut sized = 0;
    for file in &mut files {
        let blob = file.blob.clone();
        if let Some(Note::Binary(before, after)) = &mut file.note
            && sized < SIZED_BINARIES
            && after.is_none()
        {
            sized += 1;
            if file.status != Status::Deleted {
                *after = fs::metadata(top.join(&file.path)).ok().map(|m| m.len());
            }
            if let Some(blob) = blob.filter(|b| b.bytes().any(|c| c != b'0')) {
                *before = text(&["cat-file", "-s", &blob]).and_then(|s| s.parse().ok());
            }
        }
        if file.status != Status::Deleted && !file.hunks.is_empty() && file.lines_now.is_none() {
            file.lines_now = count_lines(&top.join(&file.path));
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Read::Changes(Changes {
        top: top.to_string_lossy().into_owned(),
        head: head_name,
        base: base.map(|(_, name)| name),
        files,
    })
}

/// Git's complaint, first line, or its exit status.
fn failure(output: &crate::command::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    match stderr.lines().map(str::trim).find(|l| !l.is_empty()) {
        Some(line) => line.to_owned(),
        None => match output.status.code() {
            Some(code) => format!("git exited with status {code}"),
            None => "git was stopped".into(),
        },
    }
}

/// An untracked file as an added one: its lines, or a note when it is binary or too large.
fn untracked_file(top: &Path, path: String) -> File {
    let mut file = File::new(path);
    file.status = Status::Added;
    let full = top.join(&file.path);
    let size = fs::symlink_metadata(&full).map(|m| m.len()).unwrap_or(0);
    let bytes = if size > UNTRACKED_LIMIT {
        file.note = Some(Note::TooLarge(size));
        return file;
    } else {
        match fs::symlink_metadata(&full) {
            // A link's content is where it points, as Git stores it.
            Ok(meta) if meta.file_type().is_symlink() => fs::read_link(&full)
                .map(|t| t.to_string_lossy().into_owned().into_bytes())
                .unwrap_or_default(),
            _ => fs::read(&full).unwrap_or_default(),
        }
    };
    if bytes[..bytes.len().min(BINARY_PROBE)].contains(&0) {
        file.note = Some(Note::Binary(None, Some(size)));
        return file;
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<Line> = text
        .lines()
        .enumerate()
        .map(|(i, text)| Line {
            kind: Kind::Added,
            old: 0,
            new: i as u32 + 1,
            text: untab(text.trim_end_matches('\r')),
        })
        .collect();
    if lines.is_empty() {
        file.note = Some(Note::Empty);
        return file;
    }
    file.added = lines.len() as u32;
    file.lines_now = Some(file.added);
    file.hunks.push(Hunk {
        old_start: 0,
        old_len: 0,
        new_start: 1,
        new_len: file.added,
        context: String::new(),
        lines,
    });
    file
}

/// A line as the tab draws it: tabs as four spaces, so the code lines up in its columns.
pub fn untab(text: &str) -> String {
    if text.contains('\t') {
        text.replace('\t', "    ")
    } else {
        text.to_owned()
    }
}

/// How many lines a file has now, if it is small enough to count.
fn count_lines(path: &Path) -> Option<u32> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > COUNT_LIMIT {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path).ok()?.read_to_end(&mut bytes).ok()?;
    let newlines = bytes.iter().filter(|b| **b == b'\n').count();
    let last = !bytes.is_empty() && bytes.last() != Some(&b'\n');
    Some((newlines + usize::from(last)) as u32)
}

/// The files of `git diff` output, in its order.
pub fn parse(patch: &str) -> Vec<File> {
    let mut files: Vec<File> = Vec::new();
    // Lines still to come in the current hunk, old side and new.
    let mut left = (0u32, 0u32);
    // Where the next line of the hunk falls, old side and new.
    let mut at = (0u32, 0u32);
    for line in patch.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // A line that cannot be in a hunk ends it early, and is read as what it is.
        let hunk_line = matches!(
            line.as_bytes().first(),
            None | Some(b'+' | b'-' | b' ' | b'\\')
        );
        if left != (0, 0) && hunk_line {
            let Some(file) = files.last_mut() else { break };
            let Some(hunk) = file.hunks.last_mut() else {
                break;
            };
            let (kind, text) = match line.as_bytes().first() {
                Some(b'+') => (Kind::Added, &line[1..]),
                Some(b'-') => (Kind::Deleted, &line[1..]),
                Some(b' ') => (Kind::Context, &line[1..]),
                // "\ No newline at end of file" belongs to the line before it.
                Some(b'\\') => continue,
                // An empty context line, where `diff.suppressBlankEmpty` drops the space.
                None => (Kind::Context, ""),
                Some(_) => continue,
            };
            let (old, new) = match kind {
                Kind::Added => {
                    left.1 = left.1.saturating_sub(1);
                    file.added += 1;
                    at.1 += 1;
                    (0, at.1 - 1)
                }
                Kind::Deleted => {
                    left.0 = left.0.saturating_sub(1);
                    file.deleted += 1;
                    at.0 += 1;
                    (at.0 - 1, 0)
                }
                Kind::Context => {
                    left.0 = left.0.saturating_sub(1);
                    left.1 = left.1.saturating_sub(1);
                    at.0 += 1;
                    at.1 += 1;
                    (at.0 - 1, at.1 - 1)
                }
            };
            hunk.lines.push(Line {
                kind,
                old,
                new,
                text: untab(text),
            });
            continue;
        }
        left = (0, 0);
        if let Some(rest) = line.strip_prefix("diff --git ") {
            files.push(File::new(git_line_path(rest).unwrap_or_default()));
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("@@ ") {
            if let Some(hunk) = hunk_header(rest) {
                left = (hunk.old_len, hunk.new_len);
                at = (hunk.old_lines().0, hunk.new_lines().0);
                file.hunks.push(hunk);
            }
        } else if line.starts_with("new file mode ") {
            file.status = Status::Added;
        } else if line.starts_with("deleted file mode ") {
            file.status = Status::Deleted;
        } else if let Some(mode) = line.strip_prefix("old mode ") {
            file.note = Some(Note::Mode(mode.to_owned(), String::new()));
        } else if let Some(mode) = line.strip_prefix("new mode ") {
            if let Some(Note::Mode(_, new)) = &mut file.note {
                *new = mode.to_owned();
            }
        } else if let Some(ids) = line.strip_prefix("index ") {
            file.blob = ids.split("..").next().map(str::to_owned);
        } else if let Some(from) = line.strip_prefix("rename from ") {
            file.old_path = Some(unquote(from));
            file.status = Status::Renamed;
        } else if let Some(to) = line.strip_prefix("rename to ") {
            file.path = unquote(to);
        } else if let Some(name) = line.strip_prefix("--- ") {
            if let Some(path) = side_path(name, "a/")
                && file.status != Status::Renamed
                && file.path.is_empty()
            {
                file.path = path;
            }
        } else if let Some(name) = line.strip_prefix("+++ ") {
            if let Some(path) = side_path(name, "b/") {
                file.path = path;
            }
        } else if line.starts_with("Binary files ") || line == "GIT binary patch" {
            file.note = Some(Note::Binary(None, None));
        }
    }
    for file in &mut files {
        // A deleted file's path comes from its `---` line, which `diff --git` already gave.
        if file.note.is_none() && file.hunks.is_empty() && file.status != Status::Renamed {
            file.note = Some(Note::Empty);
        }
    }
    files
}

/// `-88,9 +88,14 @@ fn name`.
fn hunk_header(rest: &str) -> Option<Hunk> {
    let (ranges, context) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(' ')?;
    let parse = |r: &str| -> Option<(u32, u32)> {
        match r.split_once(',') {
            Some((start, len)) => Some((start.parse().ok()?, len.parse().ok()?)),
            None => Some((r.parse().ok()?, 1)),
        }
    };
    let (old_start, old_len) = parse(old.strip_prefix('-')?)?;
    let (new_start, new_len) = parse(new.strip_prefix('+')?)?;
    Some(Hunk {
        old_start,
        old_len,
        new_start,
        new_len,
        context: context.trim().to_owned(),
        lines: Vec::new(),
    })
}

/// The path on a `---` or `+++` line, without its prefix; `None` for `/dev/null`. Git ends a
/// name that has a space with a tab.
fn side_path(name: &str, prefix: &str) -> Option<String> {
    let name = name.strip_suffix('\t').unwrap_or(name);
    if name == "/dev/null" {
        return None;
    }
    let name = unquote(name);
    Some(name.strip_prefix(prefix).unwrap_or(&name).to_owned())
}

/// The path on a `diff --git a/X b/X` line, for a file whose `---`/`+++` lines are missing (binary,
/// mode only, renamed without changes; renames name it again later): both sides are the same.
fn git_line_path(rest: &str) -> Option<String> {
    if rest.starts_with('"') {
        let (_, b) = split_quoted(rest)?;
        let b = unquote(b.trim_start());
        return Some(b.strip_prefix("b/").unwrap_or(&b).to_owned());
    }
    let half = rest.len().checked_sub(5)? / 2;
    let a = rest.get(2..2 + half)?;
    let b = rest.get(half + 5..)?;
    (rest.starts_with("a/") && a == b).then(|| a.to_owned())
}

/// A C-quoted name and what follows it.
fn split_quoted(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some((&text[..=i], &text[i + 1..])),
            _ => i += 1,
        }
    }
    None
}

/// A name as Git writes it: as is, or C-quoted with escapes and octal bytes.
fn unquote(name: &str) -> String {
    let Some(inner) = name.strip_prefix('"').and_then(|n| n.strip_suffix('"')) else {
        return name.to_owned();
    };
    let mut bytes = Vec::new();
    let mut chars = inner.bytes().peekable();
    while let Some(b) = chars.next() {
        if b != b'\\' {
            bytes.push(b);
            continue;
        }
        match chars.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(b'a') => bytes.push(7),
            Some(b'b') => bytes.push(8),
            Some(b'f') => bytes.push(12),
            Some(b'v') => bytes.push(11),
            Some(d @ b'0'..=b'7') => {
                let mut value = u32::from(d - b'0');
                for _ in 0..2 {
                    match chars.peek() {
                        Some(&d @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(d - b'0');
                            chars.next();
                        }
                        _ => break,
                    }
                }
                bytes.push(value as u8);
            }
            Some(other) => bytes.push(other),
            None => {}
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(hunk: &Hunk) -> String {
        hunk.lines
            .iter()
            .map(|l| match l.kind {
                Kind::Context => ' ',
                Kind::Added => '+',
                Kind::Deleted => '-',
            })
            .collect()
    }

    #[test]
    fn a_modified_file_has_its_hunks_lines_and_numbers() {
        let patch = "\
diff --git a/app/src/lib.rs b/app/src/lib.rs
index 1111111111111111111111111111111111111111..2222222222222222222222222222222222222222 100644
--- a/app/src/lib.rs
+++ b/app/src/lib.rs
@@ -88,3 +88,4 @@ impl RightPanel
     match self.tab {
-        old(),
+        new(),
+        more(),
 
@@ -136 +137 @@ fn clamp_width
-\tlet a = 1;
+\tlet a = 2;
\\ No newline at end of file
";
        let files = parse(patch);
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(file.path, "app/src/lib.rs");
        assert_eq!(file.dir_and_name(), ("app/src/", "lib.rs"));
        assert_eq!(file.status, Status::Modified);
        assert_eq!((file.added, file.deleted), (3, 2));
        assert_eq!(file.note, None);
        assert_eq!(file.hunks.len(), 2);
        let first = &file.hunks[0];
        assert_eq!(first.header(), "@@ -88,3 +88,4 @@");
        assert_eq!(first.context, "impl RightPanel");
        assert_eq!(kinds(first), " -++ ");
        let numbers: Vec<(u32, u32)> = first.lines.iter().map(|l| (l.old, l.new)).collect();
        assert_eq!(numbers, [(88, 88), (89, 0), (0, 89), (0, 90), (90, 91)]);
        // An empty context line keeps its place.
        assert_eq!(first.lines[4].text, "");
        let second = &file.hunks[1];
        assert_eq!(second.header(), "@@ -136 +137 @@");
        assert_eq!((second.old_len, second.new_len), (1, 1));
        // Tabs are drawn as four spaces; the no-newline marker is not a line.
        assert_eq!(second.lines[0].text, "    let a = 1;");
        assert_eq!(kinds(second), "-+");
    }

    #[test]
    fn new_deleted_and_renamed_files() {
        let patch = "\
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000000000000000000000000000000000000..3333333333333333333333333333333333333333
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+one
+two
diff --git a/gone.md b/gone.md
deleted file mode 100644
index 4444444444444444444444444444444444444444..0000000000000000000000000000000000000000
--- a/gone.md
+++ /dev/null
@@ -1 +0,0 @@
-bye
diff --git a/src/icon.rs b/src/app_icon.rs
similarity index 100%
rename from src/icon.rs
rename to src/app_icon.rs
diff --git a/a/old name.rs b/b/new name.rs
similarity index 80%
rename from a/old name.rs
rename to b/new name.rs
index 5555555555555555555555555555555555555555..6666666666666666666666666666666666666666 100644
--- a/a/old name.rs\t
+++ b/b/new name.rs\t
@@ -1 +1 @@
-x
+y
";
        let files = parse(patch);
        let summary: Vec<(&str, Status, u32, u32)> = files
            .iter()
            .map(|f| (f.path.as_str(), f.status, f.added, f.deleted))
            .collect();
        assert_eq!(
            summary,
            [
                ("new.txt", Status::Added, 2, 0),
                ("gone.md", Status::Deleted, 0, 1),
                ("src/app_icon.rs", Status::Renamed, 0, 0),
                ("b/new name.rs", Status::Renamed, 1, 1),
            ]
        );
        let new = &files[0].hunks[0];
        assert_eq!(new.new_lines(), (1, 3));
        assert_eq!(new.lines[1].new, 2);
        let gone = &files[1].hunks[0];
        assert_eq!((gone.old_lines(), gone.new_lines()), ((1, 2), (1, 1)));
        // Renamed without changes: no lines and nothing else to say but that.
        assert_eq!(files[2].old_path.as_deref(), Some("src/icon.rs"));
        assert!(files[2].hunks.is_empty());
        assert_eq!(files[2].note, None);
        assert_eq!(files[3].old_path.as_deref(), Some("a/old name.rs"));
    }

    #[test]
    fn binary_mode_only_and_quoted_names() {
        let patch = "\
diff --git a/icon.png b/icon.png
index 7777777777777777777777777777777777777777..8888888888888888888888888888888888888888 100644
Binary files a/icon.png and b/icon.png differ
diff --git a/run.sh b/run.sh
old mode 100644
new mode 100755
diff --git \"a/caf\\303\\251 \\\"x\\\".txt\" \"b/caf\\303\\251 \\\"x\\\".txt\"
index 9999999999999999999999999999999999999999..aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 100644
--- \"a/caf\\303\\251 \\\"x\\\".txt\"
+++ \"b/caf\\303\\251 \\\"x\\\".txt\"
@@ -1 +1 @@
-a
+b
diff --git a/empty.txt b/empty.txt
new file mode 100644
index 0000000000000000000000000000000000000000..e69de29bb2d1d6434b8b29ae775ad8c2e48c5391
";
        let files = parse(patch);
        assert_eq!(files[0].path, "icon.png");
        assert_eq!(files[0].note, Some(Note::Binary(None, None)));
        assert_eq!(
            files[0].blob.as_deref(),
            Some("7777777777777777777777777777777777777777")
        );
        assert_eq!(files[1].path, "run.sh");
        assert_eq!(
            files[1].note,
            Some(Note::Mode("100644".into(), "100755".into()))
        );
        assert_eq!(files[2].path, "café \"x\".txt");
        assert_eq!((files[2].added, files[2].deleted), (1, 1));
        assert_eq!(files[3].path, "empty.txt");
        assert_eq!(files[3].status, Status::Added);
        assert_eq!(files[3].note, Some(Note::Empty));
    }

    #[test]
    fn lock_files_are_generated() {
        let mut file = File::new("app/Cargo.lock".into());
        assert!(file.generated());
        file.path = "app/src/lock.rs".into();
        assert!(!file.generated());
    }
}
