//! Links in a terminal's output (P5-143b): what is under the mouse while ⌘ is held, and how a
//! file found there is opened. Only the cells around the mouse are read, never the whole screen.
use alacritty_terminal::{
    Term,
    grid::Dimensions,
    index::{Column, Point},
    term::cell::{Cell, Flags},
};
use anyhow::{Context as _, Result, bail};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
}

impl Link {
    pub fn contains(&self, point: Point) -> bool {
        self.start <= point && point <= self.end
    }
}

/// Where the paths written in a pane are on this Mac.
pub struct Files<'a> {
    /// What a relative path is relative to: the pane's directory.
    pub base: Option<&'a Path>,
    /// What `~/` stands for.
    pub home: Option<&'a Path>,
    /// Whether something is at a path; only a path something is at is a link.
    pub exists: &'a dyn Fn(&Path) -> bool,
}

/// How far either way from the mouse a link is looked for, in cells.
const REACH: usize = 2048;

/// Ends a web address or a path: full-width punctuation, as around addresses in Chinese text.
const FULL_WIDTH: &str = "，。、；：！？（）【】《》「」『』“”‘’⟨⟩";

/// The link under `point`: one the program marked (OSC 8), else a web address, else a path
/// something is at. `files`: none for another machine's pane, whose paths are not this Mac's.
pub fn at<T>(term: &Term<T>, point: Point, files: Option<&Files>) -> Option<Link> {
    let point = on_char(term, point)?;
    if let Some(link) = marked(term, point, files.is_some()) {
        return Some(link);
    }
    let (cells, here) = around(term, point, |cell| {
        !cell.c.is_whitespace() && cell.c != '\0'
    });
    if cells.is_empty() {
        return None;
    }
    let chars: Vec<char> = cells.iter().map(|(_, c)| *c).collect();
    let (range, target) = match url_in(&chars, here) {
        Some(range) => {
            let url = chars[range.clone()].iter().collect();
            (range, Target::Url(url))
        }
        None => path_in(&chars, here, files?)?,
    };
    Some(Link {
        start: cells[range.start].0,
        end: cells[range.end - 1].0,
        target,
    })
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

/// The characters next to each other around `point` while `keep` holds, across wrapped rows, at
/// most [`REACH`] either way, with where each is; and which of them is at `point`.
fn around<T>(
    term: &Term<T>,
    point: Point,
    keep: impl Fn(&Cell) -> bool,
) -> (Vec<(Point, char)>, usize) {
    let grid = term.grid();
    if !keep(&grid[point]) {
        return (Vec::new(), 0);
    }
    let spacer = Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
    let mut back = Vec::new();
    let mut at = point;
    while back.len() < REACH {
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
        back.push((at, cell.c));
    }
    let here = back.len();
    back.reverse();
    back.push((point, grid[point].c));
    let mut at = point;
    while back.len() < here + 1 + REACH {
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
        back.push((at, cell.c));
    }
    (back, here)
}

/// The link the program marked at `point` with OSC 8, over every cell it marked the same way: a
/// web address, or with `files`, a `file://` one. Others (`mailto:` and so on) are left alone.
fn marked<T>(term: &Term<T>, point: Point, files: bool) -> Option<Link> {
    let mark = term.grid()[point].hyperlink()?;
    let uri = mark.uri();
    let target = if scheme(uri, "http://") || scheme(uri, "https://") {
        Target::Url(uri.to_owned())
    } else if files && scheme(uri, "file://") {
        // `file://host/path`: the host is this Mac's, as only this Mac's panes get here.
        let rest = &uri["file://".len()..];
        let path = &rest[rest.find('/')?..];
        Target::File {
            path: PathBuf::from(unescape(path)),
            line: None,
            column: None,
        }
    } else {
        return None;
    };
    let (cells, _) = around(term, point, |cell| cell.hyperlink().as_ref() == Some(&mark));
    Some(Link {
        start: cells.first()?.0,
        end: cells.last()?.0,
        target,
    })
}

/// `uri` starts with `prefix`, the scheme in any case.
fn scheme(uri: &str, prefix: &str) -> bool {
    uri.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

/// `%XX` in a URI's path as the bytes they stand for.
fn unescape(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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

/// The path in `chars` over `here` that something is at, with a line and column after it:
/// `/…`, `~/…`, or relative to the pane's directory with a `/` in it (`./…`, `../…`,
/// `src/main.rs`). A word and a colon before it (`warning:src/main.rs:3`) are not part of it.
fn path_in(chars: &[char], here: usize, files: &Files) -> Option<(std::ops::Range<usize>, Target)> {
    if !path_char(chars[here]) {
        return None;
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
        return None;
    }
    let colons = (start..here).filter(|&i| chars[i] == ':').map(|i| i + 1);
    for from in std::iter::once(start).chain(colons) {
        let text: String = chars[from..end].iter().collect();
        let (path, line, column) = position(&text);
        if !path.contains('/') {
            continue;
        }
        let full = if let Some(rest) = path.strip_prefix("~/") {
            files.home.map(|home| home.join(rest))
        } else if path.starts_with('/') {
            Some(PathBuf::from(path))
        } else {
            files.base.map(|base| base.join(path))
        };
        let Some(full) = full.map(|full| full.components().collect::<PathBuf>()) else {
            continue;
        };
        if (files.exists)(&full) {
            let target = Target::File {
                path: full,
                line,
                column,
            };
            return Some((from..end, target));
        }
    }
    None
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

/// Runs `program` with `args`, found on paddock's `PATH`; an error when it cannot start or ends
/// other than with success.
pub fn open(program: &str, args: &[String]) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("couldn't run {program}"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        let said = said.trim();
        let status = match output.status.code() {
            Some(code) => format!("{program} exited with status {code}"),
            None => format!("{program} exited on a signal"),
        };
        if said.is_empty() {
            bail!("{status}");
        }
        bail!("{status}: {said}");
    }
    Ok(())
}

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

    /// The link at `point` in a pane on another machine: no files.
    fn web(screen: &Screen, point: Point) -> Option<Link> {
        at(&screen.term, point, None)
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
        fn link(&self, screen: &Screen, point: Point) -> Option<Link> {
            let (base, home) = (self.work(), self.0.join("home"));
            let exists = |path: &Path| path.exists();
            let files = Files {
                base: Some(&base),
                home: Some(&home),
                exists: &exists,
            };
            at(&screen.term, point, Some(&files))
        }
        fn file(&self, path: PathBuf, line: Option<u32>, column: Option<u32>) -> Option<Target> {
            Some(Target::File { path, line, column })
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
        // A file, on this Mac only.
        let tree = Tree::new("osc8");
        let s = screen(
            60,
            "\x1b]8;;file://mac.local/tmp/a%20b.txt\x1b\\a b\x1b]8;;\x1b\\",
        );
        assert_eq!(
            tree.link(&s, p(0, 2)).map(|link| link.target),
            tree.file(PathBuf::from("/tmp/a b.txt"), None, None)
        );
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
    fn a_command_that_fails_or_is_missing_says_so() {
        assert!(open("true", &[]).is_ok());
        let failed = open("false", &[]).unwrap_err();
        assert!(format!("{failed:#}").contains("exit"), "{failed:#}");
        let missing = open("paddock-no-such-command", &[]).unwrap_err();
        assert!(
            format!("{missing:#}").contains("paddock-no-such-command"),
            "{missing:#}"
        );
    }
}
