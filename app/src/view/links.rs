//! Links in a terminal's output (P5-143b): what is under the mouse while ⌘ is held, and how a
//! file found there is opened. Only the cells around the mouse are read, never the whole screen,
//! and never the file system on the UI thread: [`find`] reads the grid alone and says which paths
//! to look for; [`Hover`] hands that look to another thread ([`Job`]) and keeps what it found.
use crate::terminal::Screen;
use alacritty_terminal::{
    Term,
    grid::Dimensions,
    index::{Column, Point},
    term::cell::{Cell, Flags},
};
use anyhow::{Context as _, Result, bail};
use std::{
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

/// What a link opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// `http://` or `https://`: in the right sidebar's Browser.
    Url(String),
    /// A file or folder on this Mac, with the line and column written after it.
    File {
        path: PathBuf,
        line: Option<u32>,
        column: Option<u32>,
    },
}

/// A link on the grid: its first and last cell, which may be rows apart when it wraps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub start: Point,
    pub end: Point,
    pub target: Target,
    /// What the program marked it with (OSC 8), shown by it: the text can say something else.
    pub uri: Option<String>,
}

impl Link {
    pub fn contains(&self, point: Point) -> bool {
        self.start <= point && point <= self.end
    }
}

/// Where the paths written in a pane are on this Mac. Another machine's pane has none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// What a relative path is relative to: the pane's directory.
    pub base: Option<PathBuf>,
    /// What `~/` stands for.
    pub home: Option<PathBuf>,
    /// This Mac's name, which a `file://` link may name.
    pub host: Option<String>,
}

/// What is under the mouse, as far as the grid says.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    /// A web address: a link at once.
    Ready(Link),
    /// Paths: a link once something is found at one of them ([`check`]).
    Check(Candidate),
}

/// The links that may be under the mouse, best first: the first whose path something is at is.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate(Vec<Link>);

/// How far either way from the mouse a link is looked for, in cells.
const REACH: usize = 2048;

/// Ends a web address or a path: full-width punctuation, as around addresses in Chinese text.
const FULL_WIDTH: &str = "，。、；：！？（）【】《》「」『』“”‘’⟨⟩";

/// This Mac's name, as `file://` links from its programs (`ls --hyperlink`) write it.
pub fn this_host() -> Option<String> {
    static HOST: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    HOST.get_or_init(|| {
        let mut name = [0u8; 256];
        // SAFETY: the buffer is as long as said, and the name in it ends with a NUL.
        let ok = unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } == 0;
        let end = name.iter().position(|&b| b == 0)?;
        ok.then(|| String::from_utf8(name[..end].to_vec()).ok())
            .flatten()
            .filter(|name| !name.is_empty())
    })
    .clone()
}

/// What is under `point`: a link the program marked (OSC 8), else a web address, else paths. Reads
/// the grid alone. `place`: none for another machine's pane, whose paths are not this Mac's.
pub fn find<T>(term: &Term<T>, point: Point, place: Option<&Place>) -> Option<Found> {
    if point.column.0 >= term.columns()
        || point.line.0 >= term.screen_lines() as i32
        || point.line < term.topmost_line()
    {
        return None;
    }
    let point = on_char(term, point)?;
    if let Some(found) = marked(term, point, place) {
        return found;
    }
    let (cells, here) = around(term, point, |cell| {
        !cell.c.is_whitespace() && cell.c != '\0'
    });
    // Each cell's whole text, combining marks too, and which cell each character is in.
    let (mut chars, mut owner, mut at) = (Vec::new(), Vec::new(), 0);
    for (index, (_, text)) in cells.iter().enumerate() {
        if index == here {
            at = chars.len();
        }
        for c in text.chars() {
            chars.push(c);
            owner.push(index);
        }
    }
    if chars.is_empty() {
        return None;
    }
    let link = |range: std::ops::Range<usize>, target| Link {
        start: cells[owner[range.start]].0,
        end: cells[owner[range.end - 1]].0,
        target,
        uri: None,
    };
    if let Some(range) = url_in(&chars, at) {
        let url = chars[range.clone()].iter().collect();
        return Some(Found::Ready(link(range, Target::Url(url))));
    }
    let tries: Vec<Link> = path_in(&chars, at, place?)
        .into_iter()
        .map(|(range, target)| link(range, target))
        .collect();
    (!tries.is_empty()).then_some(Found::Check(Candidate(tries)))
}

/// The first of `candidate`'s links that something is at, as `exists` says: off the UI thread.
pub fn check(candidate: &Candidate, exists: &dyn Fn(&Path) -> bool) -> Option<Link> {
    candidate
        .0
        .iter()
        .find(|link| match &link.target {
            Target::File { path, .. } => exists(path),
            Target::Url(_) => true,
        })
        .cloned()
}

/// Whether something is at a path, asked off the UI thread.
pub type Exists = Arc<dyn Fn(&Path) -> bool + Send + Sync>;

/// What the mouse is over in one pane while ⌘ is held: a web address at once, a path only once
/// another thread found something there. One look at the files goes on at a time; what it found
/// is kept until ⌘ is let go, and an answer for what the mouse has left shows nothing.
pub struct Hover {
    exists: Exists,
    /// What the mouse is over, when it needs a look at the files.
    current: Option<Candidate>,
    /// A look is under way.
    looking: bool,
    known: Vec<(Candidate, Option<Link>)>,
}

/// One look at the files, to run off the UI thread.
pub struct Job {
    candidate: Candidate,
    exists: Exists,
}

impl Job {
    pub fn run(self) -> (Candidate, Option<Link>) {
        let link = check(&self.candidate, &*self.exists);
        (self.candidate, link)
    }
}

/// How many looks a pane keeps.
const KNOWN: usize = 32;

impl Hover {
    pub fn new(exists: Exists) -> Self {
        Self {
            exists,
            current: None,
            looking: false,
            known: Vec::new(),
        }
    }

    /// What shows for what [`find`] found: a web address at once; a path once found there.
    pub fn show(&mut self, found: Option<Found>) -> Option<Link> {
        match found {
            Some(Found::Ready(link)) => {
                self.current = None;
                Some(link)
            }
            Some(Found::Check(candidate)) => {
                self.current = Some(candidate);
                self.link()
            }
            None => {
                self.current = None;
                None
            }
        }
    }

    /// The link the mouse is over, as far as the files have been looked at.
    pub fn link(&self) -> Option<Link> {
        let current = self.current.as_ref()?;
        let (_, link) = self.known.iter().find(|(known, _)| known == current)?;
        link.clone()
    }

    /// The look to start now: at what the mouse is over, unless known or another look is under
    /// way (which may never come back, from a disk that does not answer).
    pub fn job(&mut self) -> Option<Job> {
        let current = self.current.as_ref()?;
        if self.looking || self.known.iter().any(|(known, _)| known == current) {
            return None;
        }
        self.looking = true;
        Some(Job {
            candidate: current.clone(),
            exists: self.exists.clone(),
        })
    }

    /// A look came back; true when it is for what the mouse is over.
    pub fn checked(&mut self, candidate: Candidate, link: Option<Link>) -> bool {
        self.looking = false;
        if self.known.len() >= KNOWN {
            self.known.remove(0);
        }
        let current = self.current.as_ref() == Some(&candidate);
        self.known.push((candidate, link));
        current
    }

    /// ⌘ let go or the mouse gone: nothing is over, and what was found is forgotten, as files
    /// come and go.
    pub fn forget(&mut self) {
        self.current = None;
        self.known.clear();
    }
}

/// The link under `point` of a pane's screen as far as known now. The screen is locked only to
/// read the grid; the files are looked at elsewhere ([`Hover::job`]).
pub fn look(
    screen: &Mutex<Screen>,
    point: Point,
    place: Option<&Place>,
    hover: &mut Hover,
) -> Option<Link> {
    let found = find(&screen.lock().unwrap().term, point, place);
    hover.show(found)
}

/// `point` as the cell with its character: a wide character's second column is its first; the
/// blank left at a row's end before a wide character that did not fit is none.
fn on_char<T>(term: &Term<T>, point: Point) -> Option<Point> {
    let flags = term.grid()[point].flags;
    if flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) {
        return None;
    }
    if flags.contains(Flags::WIDE_CHAR_SPACER) && point.column.0 > 0 {
        return Some(Point::new(point.line, point.column - 1));
    }
    Some(point)
}

/// The cell before `point` on its line, across the end of a row it wrapped from.
fn before<T>(term: &Term<T>, point: Point) -> Option<Point> {
    if point.column.0 > 0 {
        return Some(Point::new(point.line, point.column - 1));
    }
    let up = point.line - 1i32;
    let last = term.last_column();
    (up >= term.topmost_line() && term.grid()[up][last].flags.contains(Flags::WRAPLINE))
        .then(|| Point::new(up, last))
}

/// The cell after `point` on its line, across the end of a row it wraps at.
fn after<T>(term: &Term<T>, point: Point) -> Option<Point> {
    let last = term.last_column();
    if point.column < last {
        return Some(Point::new(point.line, point.column + 1));
    }
    let wraps = term.grid()[point.line][last]
        .flags
        .contains(Flags::WRAPLINE);
    (wraps && point.line < term.bottommost_line()).then(|| Point::new(point.line + 1i32, Column(0)))
}

/// A cell's text: its character and the combining marks on it.
fn text(cell: &Cell) -> String {
    let mut text = cell.c.to_string();
    if let Some(marks) = cell.zerowidth() {
        text.extend(marks);
    }
    text
}

/// The cells next to each other around `point` while `keep` holds, across wrapped rows, at most
/// [`REACH`] either way, with where each is and its text; and which of them is at `point`.
fn around<T>(
    term: &Term<T>,
    point: Point,
    keep: impl Fn(&Cell) -> bool,
) -> (Vec<(Point, String)>, usize) {
    let grid = term.grid();
    if !keep(&grid[point]) {
        return (Vec::new(), 0);
    }
    let spacer = Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
    let mut cells = Vec::new();
    let mut at = point;
    while cells.len() < REACH {
        let Some(previous) = before(term, at) else {
            break;
        };
        at = previous;
        let cell = &grid[at];
        if cell.flags.intersects(spacer) {
            continue;
        }
        if !keep(cell) {
            break;
        }
        cells.push((at, text(cell)));
    }
    let here = cells.len();
    cells.reverse();
    cells.push((point, text(&grid[point])));
    let mut at = point;
    while cells.len() < here + 1 + REACH {
        let Some(next) = after(term, at) else {
            break;
        };
        at = next;
        let cell = &grid[at];
        if cell.flags.intersects(spacer) {
            continue;
        }
        if !keep(cell) {
            break;
        }
        cells.push((at, text(cell)));
    }
    (cells, here)
}

/// What the link the program marked at `point` with OSC 8 is, over every cell it marked the same
/// way: a web address, or on this Mac a `file://` one to look for. `None` when nothing is marked
/// there or with what paddock does not open (`mailto:` and so on): the text is looked at then.
/// `Some(None)` for a `file://` link that is not one: another machine's, or not written right.
fn marked<T>(term: &Term<T>, point: Point, place: Option<&Place>) -> Option<Option<Found>> {
    let mark = term.grid()[point].hyperlink()?;
    let uri = mark.uri();
    let web = scheme(uri, "http://") || scheme(uri, "https://");
    if !web && !scheme(uri, "file://") {
        return None;
    }
    let (cells, _) = around(term, point, |cell| cell.hyperlink().as_ref() == Some(&mark));
    let link = |target| Link {
        start: cells[0].0,
        end: cells[cells.len() - 1].0,
        target,
        uri: Some(uri.to_owned()),
    };
    if web {
        return Some(Some(Found::Ready(link(Target::Url(uri.to_owned())))));
    }
    let found = place.and_then(|place| {
        let path = file_uri(uri, place.host.as_deref())?;
        let target = Target::File {
            path,
            line: None,
            column: None,
        };
        Some(Found::Check(Candidate(vec![link(target)])))
    });
    Some(found)
}

/// The path of a `file://` URI naming this Mac (no host, `localhost` or `host`), its path alone
/// decoded; none for anything else, or an escape that is not one.
fn file_uri(uri: &str, host: Option<&str>) -> Option<PathBuf> {
    let rest = &uri["file://".len()..];
    let (authority, rest) = rest.split_at(rest.find('/')?);
    let bare = |name: &str| {
        let name = name.trim_end_matches('.').to_ascii_lowercase();
        name.strip_suffix(".local").unwrap_or(&name).to_owned()
    };
    let here = authority.is_empty()
        || authority.eq_ignore_ascii_case("localhost")
        || host.is_some_and(|host| !host.is_empty() && bare(host) == bare(authority));
    if !here {
        return None;
    }
    // The query and the fragment are not part of the file.
    let path = rest.split(['?', '#']).next()?;
    let path = unescape(path)?;
    (!path.contains('\0')).then(|| PathBuf::from(path))
}

/// `uri` starts with `prefix`, the scheme in any case.
fn scheme(uri: &str, prefix: &str) -> bool {
    uri.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

/// `%XX` in a URI's path as the bytes they stand for; none for a `%` not followed by two hex
/// digits, or bytes that are not UTF-8 text.
fn unescape(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn starts(chars: &[char], at: usize, prefix: &str) -> bool {
    (at..)
        .zip(prefix.chars())
        .all(|(i, c)| chars.get(i) == Some(&c))
}

/// Can be in a web address as it is written in a terminal.
fn url_char(c: char) -> bool {
    !c.is_whitespace() && !c.is_control() && !"<>\"{}|\\^`".contains(c) && !FULL_WIDTH.contains(c)
}

/// The web address in `chars` over `here`: from `http://` or `https://` up to the first
/// character an address does not have, less a closing bracket it did not open and the
/// punctuation that ends a sentence.
fn url_in(chars: &[char], here: usize) -> Option<std::ops::Range<usize>> {
    for start in 0..=here {
        let scheme = if starts(chars, start, "https://") {
            8
        } else if starts(chars, start, "http://") {
            7
        } else {
            continue;
        };
        let mut end = start;
        while end < chars.len() && url_char(chars[end]) {
            end += 1;
        }
        let end = start + trimmed(&chars[start..end]);
        if end > start + scheme && here < end {
            return Some(start..end);
        }
    }
    None
}

/// How much of `url` is the address.
fn trimmed(url: &[char]) -> usize {
    let mut len = url.len();
    let (mut round, mut square) = (0, 0);
    for (i, c) in url.iter().enumerate() {
        let open = match c {
            '(' => &mut round,
            '[' => &mut square,
            ')' if round == 0 => {
                len = i;
                break;
            }
            ']' if square == 0 => {
                len = i;
                break;
            }
            ')' => {
                round -= 1;
                continue;
            }
            ']' => {
                square -= 1;
                continue;
            }
            _ => continue,
        };
        *open += 1;
    }
    while len > 0 && ".,:;!?'*".contains(url[len - 1]) {
        len -= 1;
    }
    len
}

/// Can be in a path as it is written in a terminal: not quotes, brackets, `,`, `;` or `=`
/// around it.
fn path_char(c: char) -> bool {
    !"\"'`()[]{}<>|,;=".contains(c) && !FULL_WIDTH.contains(c)
}

/// The paths in `chars` over `here`, with a line and column after them, best first: `/…`,
/// `~/…`, or relative to the pane's directory with a `/` in it (`./…`, `../…`, `src/main.rs`).
/// A word and a colon before one (`warning:src/main.rs:3`) are tried without too. Which is a
/// link depends on what is there ([`check`]).
fn path_in(chars: &[char], here: usize, place: &Place) -> Vec<(std::ops::Range<usize>, Target)> {
    let mut tries = Vec::new();
    if !path_char(chars[here]) {
        return tries;
    }
    let mut start = here;
    while start > 0 && path_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = here + 1;
    while end < chars.len() && path_char(chars[end]) {
        end += 1;
    }
    // A sentence's or a message's end after it; `..` and `/.` stay.
    while end > start + 1 && ".:!?".contains(chars[end - 1]) {
        if chars[end - 1] == '.' && matches!(chars[end - 2], '.' | '/') {
            break;
        }
        end -= 1;
    }
    if here >= end {
        return tries;
    }
    let colons = (start..here).filter(|&i| chars[i] == ':').map(|i| i + 1);
    for from in std::iter::once(start).chain(colons) {
        let text: String = chars[from..end].iter().collect();
        let (path, line, column) = position(&text);
        if !path.contains('/') {
            continue;
        }
        let full = if let Some(rest) = path.strip_prefix("~/") {
            place.home.as_ref().map(|home| home.join(rest))
        } else if path.starts_with('/') {
            Some(PathBuf::from(path))
        } else {
            place.base.as_ref().map(|base| base.join(path))
        };
        if let Some(full) = full {
            let path = full.components().collect();
            tries.push((from..end, Target::File { path, line, column }));
        }
    }
    tries
}

/// `path:line:column`, `path:line` or `path`.
fn position(text: &str) -> (&str, Option<u32>, Option<u32>) {
    let number = |text: &str| {
        (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
            .then(|| text.parse().ok())
            .flatten()
    };
    let Some((rest, last)) = text.rsplit_once(':') else {
        return (text, None, None);
    };
    let Some(last) = number(last) else {
        return (text, None, None);
    };
    match rest.rsplit_once(':') {
        Some((path, middle)) if number(middle).is_some() => (path, number(middle), Some(last)),
        _ => (rest, Some(last), None),
    }
}

/// The program and arguments that open `path` with the Settings' `open_files_with`: empty, the
/// app macOS picks (`open`); else that command, split at spaces, given `path:line:column`.
pub fn opener(
    setting: &str,
    path: &Path,
    line: Option<u32>,
    column: Option<u32>,
) -> (String, Vec<String>) {
    let mut words = setting.split_whitespace().map(str::to_owned);
    let Some(program) = words.next() else {
        return ("open".into(), vec![path.display().to_string()]);
    };
    let mut at = path.display().to_string();
    for number in [line, column].into_iter().map_while(|n| n) {
        at.push_str(&format!(":{number}"));
    }
    let mut args: Vec<String> = words.collect();
    args.push(at);
    (program, args)
}

/// Why a file is not opened with the app macOS picks, which for these would run something: an
/// app or another bundle, a script, an installer or a shortcut, or an executable file. Looked at
/// where it really is, through symbolic links; a folder's `x` is not taken for executable.
pub fn refusal(path: &Path) -> Option<&'static str> {
    let Ok(real) = std::fs::canonicalize(path) else {
        return Some("it is no longer there");
    };
    let Ok(meta) = std::fs::metadata(&real) else {
        return Some("it is no longer there");
    };
    let extension = real
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if meta.is_dir() {
        let bundle =
            BUNDLES.contains(&extension.as_str()) || real.join("Contents/Info.plist").exists();
        return bundle.then_some("it is an app or another bundle, which would run");
    }
    if RUNS.contains(&extension.as_str()) {
        return Some("it is a script, an installer or a shortcut, which would run something");
    }
    (meta.permissions().mode() & 0o111 != 0).then_some("it is an executable file, which would run")
}

/// Folders macOS opens as one thing, an app or what an app loads; any with `Contents/Info.plist`
/// is taken for one too.
const BUNDLES: &[&str] = &[
    "app",
    "appex",
    "bundle",
    "framework",
    "plugin",
    "prefpane",
    "saver",
    "kext",
    "xpc",
    "qlgenerator",
    "mdimporter",
    "osax",
    "service",
    "action",
    "workflow",
    "scptd",
    "pkg",
    "mpkg",
];

/// Files the app macOS picks for them runs, or opens something else with: scripts and Terminal
/// settings, Automator, installers, Java, and Finder's shortcuts.
const RUNS: &[&str] = &[
    "command",
    "tool",
    "terminal",
    "workflow",
    "scpt",
    "applescript",
    "pkg",
    "mpkg",
    "jar",
    "fileloc",
    "inetloc",
    "webloc",
];

/// Opens `path` with the Settings' `open_files_with` and waits for it, so off the UI thread:
/// with the app macOS picks (empty), only what [`refusal`] lets through, looked at again now.
pub fn open_file(setting: &str, path: &Path, line: Option<u32>, column: Option<u32>) -> Result<()> {
    if !path.exists() {
        bail!("it is no longer there");
    }
    if setting.trim().is_empty()
        && let Some(why) = refusal(path)
    {
        bail!("{why}");
    }
    let (program, args) = opener(setting, path, line, column);
    let child = launch(&program, &args)?;
    finish(child, &program)
}

/// Starts `program` with `args`, found on paddock's `PATH`: nothing to read, what it prints
/// dropped, what it says on standard error kept for [`finish`].
pub fn launch(program: &str, args: &[String]) -> Result<Child> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // Under paddock's spawn lock (P5-126), let go of before anything waits.
    crate::command::start(&mut command).with_context(|| format!("couldn't run {program}"))
}

/// Waits for `child` on this thread: an error when it ended other than with success, with the
/// start of what it said.
pub fn finish(mut child: Child, program: &str) -> Result<()> {
    let said = Arc::new(Mutex::new(Vec::new()));
    let (done, ended) = mpsc::channel::<()>();
    if let Some(mut stderr) = child.stderr.take() {
        let said = said.clone();
        // Read to its end, so that it never waits on a full pipe; only the start kept.
        let _ = std::thread::Builder::new()
            .name("paddock-open-stderr".into())
            .spawn(move || {
                let mut buffer = [0; 4096];
                while let Ok(read @ 1..) = stderr.read(&mut buffer) {
                    let mut said = said.lock().unwrap();
                    let room = SAID.saturating_sub(said.len());
                    said.extend_from_slice(&buffer[..read.min(room)]);
                }
                let _ = done.send(());
            });
    }
    let status = child
        .wait()
        .with_context(|| format!("couldn't wait for {program}"))?;
    // What it left behind may keep its standard error open: what came so far is enough.
    let _ = ended.recv_timeout(Duration::from_millis(200));
    if status.success() {
        return Ok(());
    }
    let status = match status.code() {
        Some(code) => format!("{program} exited with status {code}"),
        None => format!("{program} exited on a signal"),
    };
    let said = said.lock().unwrap();
    let said = String::from_utf8_lossy(&said);
    let said = said.trim();
    if said.is_empty() {
        bail!("{status}");
    }
    bail!("{status}: {said}");
}

/// How much of what an opener says on standard error is kept, in bytes.
const SAID: usize = 2000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};
    use alacritty_terminal::index::Line;

    fn screen(cols: u16, bytes: &str) -> Screen {
        let mut screen = Screen::new(Size { rows: 8, cols });
        screen.process(bytes.as_bytes());
        screen
    }

    fn p(line: i32, col: usize) -> Point {
        Point::new(Line(line), Column(col))
    }

    fn url(text: &str) -> Option<Target> {
        Some(Target::Url(text.into()))
    }

    /// The link at `point` in a pane on another machine: no files, so known at once.
    fn web(screen: &Screen, point: Point) -> Option<Link> {
        match find(&screen.term, point, None)? {
            Found::Ready(link) => Some(link),
            Found::Check(candidate) => panic!("files looked for: {candidate:?}"),
        }
    }

    #[test]
    fn a_web_address_ends_before_punctuation_after_it() {
        let s = screen(60, "see https://example.com/a?b=1. Then");
        let link = web(&s, p(0, 10)).unwrap();
        assert_eq!(link.target, url("https://example.com/a?b=1").unwrap());
        assert_eq!((link.start, link.end), (p(0, 4), p(0, 28)));
        // The full stop and what is not the address are no link.
        assert_eq!(web(&s, p(0, 29)), None);
        assert_eq!(web(&s, p(0, 1)), None);
        let s = screen(60, "'https://example.com/x', \"https://example.com/y\"!");
        assert_eq!(
            web(&s, p(0, 3)).unwrap().target,
            url("https://example.com/x").unwrap()
        );
        assert_eq!(
            web(&s, p(0, 30)).unwrap().target,
            url("https://example.com/y").unwrap()
        );
        // http too; a bare scheme is not an address.
        let s = screen(60, "http://localhost:8080/ and https:// alone");
        assert_eq!(
            web(&s, p(0, 3)).unwrap().target,
            url("http://localhost:8080/").unwrap()
        );
        assert_eq!(web(&s, p(0, 30)), None);
    }

    #[test]
    fn a_closing_bracket_with_no_opening_one_in_the_address_is_not_part_of_it() {
        let s = screen(
            70,
            "(see https://example.com/x) [doc](https://example.com/y).",
        );
        assert_eq!(
            web(&s, p(0, 8)).unwrap().target,
            url("https://example.com/x").unwrap()
        );
        let link = web(&s, p(0, 40)).unwrap();
        assert_eq!(link.target, url("https://example.com/y").unwrap());
        assert_eq!(link.end, p(0, 54));
        let s = screen(70, "https://en.wikipedia.org/wiki/Rust_(language), more");
        assert_eq!(
            web(&s, p(0, 3)).unwrap().target,
            url("https://en.wikipedia.org/wiki/Rust_(language)").unwrap()
        );
        // Full-width punctuation around it, as in Chinese text.
        let s = screen(70, "见https://example.com/a，然后（https://example.com/b）");
        assert_eq!(
            web(&s, p(0, 5)).unwrap().target,
            url("https://example.com/a").unwrap()
        );
        assert_eq!(
            web(&s, p(0, 34)).unwrap().target,
            url("https://example.com/b").unwrap()
        );
    }

    #[test]
    fn a_web_address_wrapped_onto_the_next_rows_is_one_link() {
        // 16 columns: the address goes on over two more rows.
        let s = screen(16, "go https://example.com/a/very/long/path.\r\nnext");
        let whole = url("https://example.com/a/very/long/path");
        for point in [p(0, 5), p(1, 0), p(1, 15), p(2, 6)] {
            let link = web(&s, point).unwrap();
            assert_eq!(link.target, whole.clone().unwrap(), "{point:?}");
            assert_eq!((link.start, link.end), (p(0, 3), p(2, 6)));
            assert!(link.contains(p(1, 7)));
        }
        assert_eq!(web(&s, p(2, 7)), None);
        // A line ended by the program is not continued on the next.
        let s = screen(40, "https://example.com/a\r\nb/c");
        assert_eq!(
            web(&s, p(0, 3)).unwrap().target,
            url("https://example.com/a").unwrap()
        );
        assert_eq!(web(&s, p(1, 0)), None);
    }

    /// A folder with `src/main.rs` and `notes.txt` in it, and a home with `docs/a.md`.
    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str) -> Tree {
            let root =
                std::env::temp_dir().join(format!("paddock-links-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for dir in ["work/src", "home/docs"] {
                std::fs::create_dir_all(root.join(dir)).unwrap();
            }
            for file in ["work/src/main.rs", "work/notes.txt", "home/docs/a.md"] {
                std::fs::write(root.join(file), "").unwrap();
            }
            Tree(root)
        }
        fn work(&self) -> PathBuf {
            self.0.join("work")
        }
        /// This Mac's pane in the folder, on a Mac named `mac.local`.
        fn place(&self) -> Place {
            Place {
                base: Some(self.work()),
                home: Some(self.0.join("home")),
                host: Some("Mac.local".into()),
            }
        }
        /// The link at `point`, once the files have been looked at.
        fn link(&self, screen: &Screen, point: Point) -> Option<Link> {
            match find(&screen.term, point, Some(&self.place()))? {
                Found::Ready(link) => Some(link),
                Found::Check(candidate) => check(&candidate, &|path| path.exists()),
            }
        }
        fn file(&self, path: PathBuf, line: Option<u32>, column: Option<u32>) -> Option<Target> {
            Some(Target::File { path, line, column })
        }
    }

    impl Tree {
        /// An empty file at `path` under the folder.
        fn touch(&self, path: &str) -> PathBuf {
            let path = self.0.join(path);
            std::fs::write(&path, "").unwrap();
            path
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn paths_something_is_at_are_links_with_their_line_and_column() {
        let tree = Tree::new("paths");
        let main = tree.work().join("src/main.rs");
        let target = |screen: &Screen, point| tree.link(screen, point).map(|link| link.target);
        // Absolute, with a line and a column, and the colon after them left out.
        let text = format!("error {}:12:5: oops", main.display());
        let s = screen(200, &text);
        let link = tree.link(&s, p(0, 8)).unwrap();
        assert_eq!(
            Some(link.target),
            tree.file(main.clone(), Some(12), Some(5))
        );
        let end = 6 + main.display().to_string().chars().count() + 4;
        assert_eq!((link.start, link.end), (p(0, 6), p(0, end)));
        // On the line number too.
        assert_eq!(
            target(&s, p(0, end - 2)),
            tree.file(main.clone(), Some(12), Some(5))
        );
        // Relative to the pane's directory, with `/`, `./` or `../`; `~/`.
        let s = screen(
            80,
            "  src/main.rs:7 ./notes.txt, ../work/notes.txt ~/docs/a.md",
        );
        assert_eq!(target(&s, p(0, 3)), tree.file(main.clone(), Some(7), None));
        let notes = tree.work().join("notes.txt");
        assert_eq!(target(&s, p(0, 18)), tree.file(notes.clone(), None, None));
        assert_eq!(
            target(&s, p(0, 30)),
            tree.file(tree.0.join("work/../work/notes.txt"), None, None)
        );
        assert_eq!(
            target(&s, p(0, 50)),
            tree.file(tree.0.join("home/docs/a.md"), None, None)
        );
        // Quoted, or after a word and a colon.
        let s = screen(80, "File \"src/main.rs\", line 3 warning:src/main.rs:9");
        assert_eq!(target(&s, p(0, 8)), tree.file(main.clone(), None, None));
        assert_eq!(target(&s, p(0, 40)), tree.file(main.clone(), Some(9), None));
        assert_eq!(target(&s, p(0, 30)), None);
    }

    #[test]
    fn a_path_nothing_is_at_or_without_a_slash_is_no_link() {
        let tree = Tree::new("missing");
        let s = screen(80, "src/missing.rs notes.txt and/or ~/nothing/here");
        // Blanks between and after the words too.
        for col in [3, 17, 27, 36, 14, 60] {
            assert_eq!(tree.link(&s, p(0, col)), None, "column {col}");
        }
        // Another machine's pane: its paths are not this Mac's, its addresses still links.
        let main = tree.work().join("src/main.rs");
        let s = screen(200, &format!("{} https://example.com", main.display()));
        assert_eq!(web(&s, p(0, 3)), None);
        let after = main.display().to_string().chars().count() + 4;
        assert_eq!(
            web(&s, p(0, after)).unwrap().target,
            url("https://example.com").unwrap()
        );
    }

    #[test]
    fn a_link_the_program_marked_opens_what_it_marked() {
        let s = screen(
            60,
            "\x1b]8;;https://example.com/docs\x1b\\click here\x1b]8;;\x1b\\ after",
        );
        let link = web(&s, p(0, 8)).unwrap();
        assert_eq!(link.target, url("https://example.com/docs").unwrap());
        assert_eq!((link.start, link.end), (p(0, 0), p(0, 9)));
        assert_eq!(web(&s, p(0, 12)), None);
        // Marked over text that reads as another address: the mark wins.
        let s = screen(
            60,
            "\x1b]8;id=1;https://real.example/\x07https://shown.example/\x1b]8;;\x07",
        );
        assert_eq!(
            web(&s, p(0, 4)).unwrap().target,
            url("https://real.example/").unwrap()
        );
        // A file, on this Mac only, named by its name in any case.
        let tree = Tree::new("osc8");
        let notes = tree.work().join("notes.txt");
        let uri = format!("file://mac.LOCAL{}", notes.display());
        let s = screen(200, &format!("\x1b]8;;{uri}\x1b\\a b\x1b]8;;\x1b\\"));
        let link = tree.link(&s, p(0, 2)).unwrap();
        assert_eq!(Some(link.target), tree.file(notes, None, None));
        assert_eq!(link.uri, Some(uri));
        assert_eq!(web(&s, p(0, 0)), None);
    }

    #[test]
    fn a_file_opens_with_the_app_macos_picks_or_the_command_set() {
        let path = Path::new("/w/src/main.rs");
        let strings = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            opener("", path, Some(3), Some(4)),
            ("open".into(), strings(&["/w/src/main.rs"]))
        );
        assert_eq!(
            opener("zed", path, Some(3), Some(4)),
            ("zed".into(), strings(&["/w/src/main.rs:3:4"]))
        );
        assert_eq!(
            opener("  code   -g ", path, Some(3), None),
            ("code".into(), strings(&["-g", "/w/src/main.rs:3"]))
        );
        assert_eq!(
            opener("cursor -g", path, None, None),
            ("cursor".into(), strings(&["-g", "/w/src/main.rs"]))
        );
    }

    #[test]
    fn a_marked_file_is_this_macs_with_its_path_alone_and_must_be_there() {
        let tree = Tree::new("uri");
        let work = tree.work();
        let spaced = tree.touch("work/a b.txt");
        let hashed = tree.touch("work/x#y.txt");
        let notes = work.join("notes.txt");
        let marked = |uri: String| {
            let s = screen(60, &format!("\x1b]8;;{uri}\x1b\\name\x1b]8;;\x1b\\"));
            tree.link(&s, p(0, 1)).map(|link| link.target)
        };
        let w = work.display();
        assert_eq!(
            marked(format!("file://{w}/notes.txt")),
            tree.file(notes.clone(), None, None)
        );
        assert_eq!(
            marked(format!("file://localhost{w}/notes.txt")),
            tree.file(notes.clone(), None, None)
        );
        assert_eq!(
            marked(format!("file://{w}/a%20b.txt")),
            tree.file(spaced, None, None)
        );
        // An escaped `#` is in the name; a bare one starts the fragment, as `?` the query.
        assert_eq!(
            marked(format!("file://{w}/x%23y.txt")),
            tree.file(hashed, None, None)
        );
        assert_eq!(
            marked(format!("file://{w}/notes.txt#L10")),
            tree.file(notes.clone(), None, None)
        );
        assert_eq!(
            marked(format!("file://{w}/notes.txt?a=1")),
            tree.file(notes.clone(), None, None)
        );
        // Another machine's file, an escape that is not one, bytes that are not text, and a
        // file that is not there: no link.
        for uri in [
            format!("file://other.example{w}/notes.txt"),
            format!("file://{w}/notes%zz.txt"),
            format!("file://{w}/notes%ff.txt"),
            format!("file://{w}/gone.txt"),
        ] {
            assert_eq!(marked(uri.clone()), None, "{uri}");
        }
    }

    #[test]
    fn combining_marks_are_part_of_a_path_or_an_address() {
        let tree = Tree::new("marks");
        let accented = tree.touch("work/cafe\u{301}.txt");
        tree.touch("work/cafe.txt");
        let s = screen(60, "./cafe\u{301}.txt https://example.com/cafe\u{301}");
        assert_eq!(
            tree.link(&s, p(0, 3)).map(|link| link.target),
            tree.file(accented, None, None)
        );
        assert_eq!(
            web(&s, p(0, 15)).unwrap().target,
            url("https://example.com/cafe\u{301}").unwrap()
        );
    }

    #[test]
    fn files_are_looked_at_off_the_ui_thread_and_a_late_answer_does_not_cover_a_newer_one() {
        let tree = Tree::new("hover");
        let screen = Arc::new(Mutex::new(screen(
            80,
            "src/main.rs https://example.com ./notes.txt",
        )));
        let main = tree.work().join("src/main.rs");
        let notes = tree.work().join("notes.txt");
        // Files that are not answered for until let go, as on a disk that hangs.
        let (go, gate) = mpsc::channel::<()>();
        let gate = Mutex::new(gate);
        let exists: Exists = Arc::new(move |path: &Path| {
            let _ = gate.lock().unwrap().recv();
            path.exists()
        });
        let place = tree.place();
        // Looking does not wait for the files, nor keep the screen locked.
        let (sent, looked) = mpsc::channel();
        {
            let (screen, place) = (screen.clone(), place.clone());
            std::thread::spawn(move || {
                let mut hover = Hover::new(exists);
                let shown = look(&screen, p(0, 3), Some(&place), &mut hover);
                let job = hover.job();
                let _ = sent.send((hover, shown, job));
            });
        }
        let (mut hover, shown, job) = looked
            .recv_timeout(Duration::from_secs(2))
            .expect("looking waited for the files");
        assert!(screen.try_lock().is_ok());
        // Not a link until something is found there.
        assert_eq!(shown, None);
        let job = job.expect("a look to start");
        let first = std::thread::spawn(move || job.run());
        // A web address shows at once, files or not.
        let web = look(&screen, p(0, 15), Some(&place), &mut hover).unwrap();
        assert_eq!(web.target, url("https://example.com").unwrap());
        assert!(hover.job().is_none());
        // Over another path meanwhile: one look at a time.
        assert_eq!(look(&screen, p(0, 36), Some(&place), &mut hover), None);
        assert!(hover.job().is_none());
        drop(go);
        let (candidate, found) = first.join().unwrap();
        assert_eq!(
            found.as_ref().map(|link| &link.target),
            tree.file(main.clone(), None, None).as_ref()
        );
        // The answer for the first path does not show over the second.
        assert!(!hover.checked(candidate, found));
        assert_eq!(hover.link(), None);
        let (candidate, found) = hover.job().expect("the second path's turn").run();
        assert!(hover.checked(candidate, found));
        assert_eq!(
            hover.link().map(|link| link.target),
            tree.file(notes, None, None)
        );
        // Back over the first: known, not looked at again.
        let shown = look(&screen, p(0, 3), Some(&place), &mut hover);
        assert_eq!(shown.map(|link| link.target), tree.file(main, None, None));
        assert!(hover.job().is_none());
        // ⌘ let go: forgotten, looked at again next time.
        hover.forget();
        assert_eq!(look(&screen, p(0, 3), Some(&place), &mut hover), None);
        assert!(hover.job().is_some());
    }

    #[test]
    fn what_would_run_is_not_opened_with_the_app_macos_picks() {
        use std::os::unix::fs::symlink;
        let tree = Tree::new("refuse");
        let dir = |path: &str| {
            let path = tree.0.join(path);
            std::fs::create_dir_all(&path).unwrap();
            path
        };
        let executable = |path: &str| {
            let path = tree.touch(path);
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        let app = dir("work/Tool.app");
        let bundle = dir("work/Thing/Contents");
        tree.touch("work/Thing/Contents/Info.plist");
        let bundle = bundle.parent().unwrap().to_owned();
        let refused = [
            app.clone(),
            bundle,
            tree.touch("work/run.command"),
            tree.touch("work/setup.PKG"),
            tree.touch("work/script.scpt"),
            tree.touch("work/script.applescript"),
            tree.touch("work/shell.terminal"),
            tree.touch("work/thing.tool"),
            dir("work/Flow.workflow"),
            executable("work/tool"),
        ];
        let link = tree.0.join("work/link.txt");
        symlink(&app, &link).unwrap();
        for path in refused.iter().chain([&link]) {
            assert!(refusal(path).is_some(), "{}", path.display());
        }
        // Documents, folders (their x is not executable) and what is gone are left alone here.
        for path in [
            tree.work().join("notes.txt"),
            tree.work().join("src"),
            tree.0.join("home/docs/a.md"),
        ] {
            assert_eq!(refusal(&path), None, "{}", path.display());
        }
        // Refused before anything runs, saying why; a command set in Settings gets it anyway.
        let error = open_file("", &app, None, None).unwrap_err();
        assert!(format!("{error:#}").contains("app"), "{error:#}");
        let error = open_file("", &tree.work().join("gone.txt"), None, None).unwrap_err();
        assert!(
            format!("{error:#}").contains("no longer there"),
            "{error:#}"
        );
        assert!(open_file("true", &app, Some(3), None).is_ok());
        let error = open_file("false -g", &tree.work().join("notes.txt"), None, None).unwrap_err();
        assert!(
            format!("{error:#}").contains("false exited with status 1"),
            "{error:#}"
        );
    }

    /// `script` run by `sh`, as a file's opener.
    fn sh(script: &str) -> Result<()> {
        let args = ["-c".to_owned(), script.to_owned()];
        finish(launch("/bin/sh", &args)?, "sh")
    }

    #[test]
    fn an_opener_is_waited_for_and_says_how_it_failed_in_a_few_words() {
        let started = std::time::Instant::now();
        assert!(sh("sleep 0.3").is_ok());
        assert!(started.elapsed() >= Duration::from_millis(300));
        let error = sh("sleep 0.2; echo cannot open that >&2; exit 3").unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            "sh exited with status 3: cannot open that"
        );
        // A lot said: only its start is kept, and it does not stall the opener.
        let error = sh("yes 'too much' | head -c 200000 >&2; exit 1").unwrap_err();
        let said = format!("{error:#}");
        assert!(
            said.starts_with("sh exited with status 1: too much"),
            "{}",
            &said[..80]
        );
        assert!(said.len() < 3000, "{} bytes", said.len());
        // Done when it ends, even if something it left behind keeps its standard error open.
        let started = std::time::Instant::now();
        assert!(sh("sleep 5 >/dev/null & exit 0").is_ok());
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
        let missing = launch("paddock-no-such-command", &[]).unwrap_err();
        assert!(
            format!("{missing:#}").contains("couldn't run paddock-no-such-command"),
            "{missing:#}"
        );
    }
}
