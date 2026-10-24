//! Where each command a shell ran is in the terminal's text (DESIGN §13 P5-149). The shell marks
//! where its prompt starts, where the command's output starts and that the command ended, with
//! its status (OSC 133; paddock's own marks come from `shell_marks`, other shells may send their
//! own). This keeps each mark on the line it was made on as output scrolls it into history,
//! history is trimmed or cleared and the pane is resized. A command whose prompt and command line
//! no longer read as they did when it started running is dropped, not drawn in the wrong place.
//! Nothing is kept from a program that has the alternate screen (vim, less).
//!
//! Lines are counted from the first the screen ever had, so a line keeps its number while it
//! scrolls: the parser is allowed [`SLACK`] lines of history over [`HISTORY`] and trimmed back
//! after every step, so it never drops a line without the count seeing it.
use alacritty_terminal::{
    Term,
    event::EventListener,
    grid::{Dimensions, Grid},
    index::{Column, Line, Point},
    term::{
        TermMode,
        cell::{Cell, Flags},
    },
};
use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
};

/// The history a terminal that keeps segments shows, as much as one that does not.
pub const HISTORY: usize = 10_000;
/// History the parser may hold over [`HISTORY`] until the next trim: more than any one step adds.
pub const SLACK: usize = 1024;

/// One OSC 133 mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// `A`: the prompt starts here.
    Prompt,
    /// `B`: the command line starts here (not kept: nothing is drawn from it).
    Input,
    /// `C`: the command's output starts here.
    Output,
    /// `D;<status>`: the command has ended, with its status when the shell gave a number.
    End(Option<i32>),
}

impl Mark {
    /// The mark an OSC's parameters (`133;D;1`, `133;A;aid=7`) make, when it is one.
    pub fn parse(params: &[&[u8]]) -> Option<Mark> {
        if params.first() != Some(&&b"133"[..]) {
            return None;
        }
        match *params.get(1)? {
            b"A" => Some(Mark::Prompt),
            b"B" => Some(Mark::Input),
            b"C" => Some(Mark::Output),
            b"D" => {
                Some(Mark::End(params.get(2).and_then(|code| {
                    std::str::from_utf8(code).ok()?.parse().ok()
                })))
            }
            _ => None,
        }
    }
}

/// One command the shell ran, where it is in the grid now: lines below 0 are in history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    /// Which command it is: the same while its lines scroll and history is trimmed or cleared
    /// above it; a resize may change it.
    pub id: i64,
    /// The line its prompt starts on.
    pub prompt: Line,
    /// Where its output starts.
    pub output: Point,
    /// Just past where its output ends (its column may be the grid's width); `None` while it runs.
    pub end: Option<Point>,
    /// Its exit status, when the shell gave one.
    pub exit: Option<i32>,
}

impl Segment {
    /// It has ended with a status other than 0.
    pub fn failed(&self) -> bool {
        self.end.is_some() && self.exit.is_some_and(|code| code != 0)
    }
    /// The last line it covers: its output's last, or its command line's when it printed nothing.
    pub fn last(&self) -> Line {
        let end = self.end.unwrap_or(self.output);
        let last = if end.column.0 == 0 {
            end.line - 1
        } else {
            end.line
        };
        last.max(self.prompt)
    }
    /// Its output's first and last cell, when it has ended and printed something; the command
    /// line is left out. `columns`: the grid's width.
    pub fn output(&self, columns: usize) -> Option<(Point, Point)> {
        let end = self.end?;
        let first = if self.output.column.0 >= columns {
            Point::new(self.output.line + 1, Column(0))
        } else {
            self.output
        };
        let last = if end.column.0 == 0 {
            Point::new(end.line - 1, Column(columns - 1))
        } else {
            Point::new(end.line, Column(end.column.0.min(columns) - 1))
        };
        (first <= last).then_some((first, last))
    }
}

/// A place in the text: `row` counts the primary screen's lines from its first; `col` may be the
/// width, just past the line's last cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Spot {
    row: i64,
    col: usize,
}

#[derive(Clone, Debug)]
struct Record {
    prompt: Spot,
    output: Option<Spot>,
    end: Option<Spot>,
    exit: Option<i32>,
    /// How the prompt and command line read when the output started.
    text: Option<u64>,
}

/// Where a place is in the text regardless of width: lines (as the program wrote them, wrapped
/// rows together) above the cursor's, and cells into its line.
#[derive(Clone, Copy, Debug)]
struct Place {
    up: i64,
    offset: usize,
}

/// What the primary screen looked like when a program took the alternate screen: if the pane is
/// resized meanwhile, the marks are found again from here when it comes back.
struct Away {
    places: Vec<Option<Place>>,
    resized: bool,
}

/// The history and screen in effect before a step of the parser.
#[derive(Clone, Copy)]
pub struct Before {
    alt: bool,
    history: usize,
}

impl Before {
    pub fn of<T>(term: &Term<T>) -> Self {
        Self {
            alt: alternate(term),
            history: term.grid().history_size(),
        }
    }
}

/// The commands a shell's terminal has run (off for other terminals).
#[derive(Default)]
pub struct Segments {
    on: bool,
    /// The row number of the primary screen's `Line(0)`.
    base: i64,
    done: VecDeque<Record>,
    /// The command whose prompt is the latest.
    open: Option<Record>,
    away: Option<Away>,
}

impl Segments {
    /// Keeps segments from now on; the terminal's history limit must then be
    /// [`HISTORY`] + [`SLACK`].
    pub fn start(&mut self) {
        self.on = true;
    }
    pub fn on(&self) -> bool {
        self.on
    }

    /// Takes a mark the shell has just sent, at the cursor.
    pub fn mark<T>(&mut self, term: &Term<T>, mark: Mark) {
        if !self.on || alternate(term) {
            return;
        }
        let grid = term.grid();
        let point = grid.cursor.point;
        let col = if grid.cursor.input_needs_wrap {
            grid.columns()
        } else {
            point.column.0
        };
        let at = Spot {
            row: self.base + i64::from(point.line.0),
            col,
        };
        // Where something starts: past a full line is the next line's start.
        let start = if col >= grid.columns() {
            Spot {
                row: at.row + 1,
                col: 0,
            }
        } else {
            at
        };
        match mark {
            Mark::Prompt => {
                // A command that ran without saying it ended ends where the next prompt starts; a
                // prompt left without running anything (or drawn again) is replaced.
                if let Some(open) = self.open.take()
                    && open.output.is_some()
                {
                    self.done.push_back(Record {
                        end: Some(at),
                        ..open
                    });
                }
                self.open = Some(Record {
                    prompt: start,
                    output: None,
                    end: None,
                    exit: None,
                    text: None,
                });
            }
            Mark::Input => {}
            Mark::Output => {
                if let Some(open) = &mut self.open
                    && open.output.is_none()
                {
                    open.output = Some(start);
                    open.text = read(grid, self.base, open.prompt, start);
                    if open.text.is_none() {
                        self.open = None;
                    }
                }
            }
            Mark::End(exit) => {
                if let Some(open) = self.open.take()
                    && open.output.is_some()
                {
                    self.done.push_back(Record {
                        end: Some(at),
                        exit,
                        ..open
                    });
                }
            }
        }
    }

    /// After a step of the parser that may scroll lines into history or clear it.
    pub fn after<T: EventListener>(&mut self, term: &mut Term<T>, before: Before) {
        if !self.on || before.alt || alternate(term) {
            return;
        }
        let history = term.grid().history_size();
        if history < before.history {
            // Cleared: what was in history has gone.
            self.cut(-(history as i64));
        } else {
            self.base += (history - before.history) as i64;
        }
        self.trim(term);
    }

    /// The terminal was reset: nothing on it is where it was.
    pub fn forget(&mut self) {
        self.done.clear();
        self.open = None;
        self.away = None;
    }

    /// A program is about to take the alternate screen.
    pub fn leave<T>(&mut self, term: &Term<T>) {
        if !self.on {
            return;
        }
        self.away = Some(Away {
            places: self.places(term.grid()),
            resized: false,
        });
    }

    /// The program has given the primary screen back.
    pub fn back<T: EventListener>(&mut self, term: &mut Term<T>) {
        if let Some(away) = self.away.take()
            && away.resized
        {
            self.put(term.grid(), away.places);
            self.trim(term);
            self.check(term.grid(), true);
        }
    }

    /// Resizes `term` to `size`, keeping the marks on their text as lines rewrap.
    pub fn resize<T: EventListener, S: Dimensions>(&mut self, term: &mut Term<T>, size: S) {
        let same = term.columns() == size.columns() && term.screen_lines() == size.screen_lines();
        if !self.on || same {
            term.resize(size);
            return;
        }
        if alternate(term) {
            if let Some(away) = &mut self.away {
                away.resized = true;
            }
            term.resize(size);
            return;
        }
        let places = self.places(term.grid());
        term.resize(size);
        self.put(term.grid(), places);
        self.trim(term);
        self.check(term.grid(), true);
    }

    /// Drops the commands whose prompt and command line no longer read as they did: those on the
    /// screen, where output can write over them (history does not change).
    pub fn verify<T>(&mut self, term: &Term<T>) {
        if self.on && !alternate(term) {
            self.check(term.grid(), false);
        }
    }

    /// The commands that have run, oldest first, on the primary screen.
    pub fn segments<T>(&self, term: &Term<T>) -> Vec<Segment> {
        if !self.on || alternate(term) {
            return Vec::new();
        }
        let top = i64::from(term.grid().topmost_line().0);
        let point = |spot: Spot| Point::new(Line((spot.row - self.base) as i32), Column(spot.col));
        self.done
            .iter()
            .chain(self.open.iter())
            .filter(|record| record.prompt.row - self.base >= top)
            .filter_map(|record| {
                Some(Segment {
                    id: record.prompt.row,
                    prompt: point(record.prompt).line,
                    output: point(record.output?),
                    end: record.end.map(point),
                    exit: record.exit,
                })
            })
            .collect()
    }

    fn records(&mut self) -> impl Iterator<Item = &mut Record> {
        self.done.iter_mut().chain(self.open.iter_mut())
    }

    /// Drops what starts above `line` (history cleared or trimmed down to it).
    fn cut(&mut self, line: i64) {
        let gone = self.base + line;
        self.done.retain(|record| record.prompt.row >= gone);
        if self
            .open
            .as_ref()
            .is_some_and(|record| record.prompt.row < gone)
        {
            self.open = None;
        }
    }

    /// Back to [`HISTORY`]: oldest lines go, and the commands that started on them.
    fn trim<T>(&mut self, term: &mut Term<T>) {
        let history = term.grid().history_size();
        if history >= HISTORY + SLACK {
            // The parser may have dropped lines the count did not see.
            self.forget();
        }
        if history > HISTORY {
            let grid = term.grid_mut();
            grid.update_history(HISTORY);
            grid.update_history(HISTORY + SLACK);
            self.cut(-(HISTORY as i64));
        }
    }

    fn places(&self, grid: &Grid<Cell>) -> Vec<Option<Place>> {
        let text = Wrapped::new(grid);
        let place = |spot: Option<Spot>| {
            let spot = spot?;
            text.place(grid, (spot.row - self.base) as i32, spot.col)
        };
        self.done
            .iter()
            .chain(self.open.iter())
            .flat_map(|record| {
                [
                    place(Some(record.prompt)),
                    place(record.output),
                    place(record.end),
                ]
            })
            .collect()
    }

    /// Puts every mark back where `places` (from [`Self::places`] before the grid rewrapped) says.
    fn put(&mut self, grid: &Grid<Cell>, places: Vec<Option<Place>>) {
        let text = Wrapped::new(grid);
        let base = self.base;
        let spot = |place: Option<Place>| {
            let (line, col) = text.spot(grid, place?)?;
            Some(Spot {
                row: base + i64::from(line),
                col,
            })
        };
        let mut places = places.chunks(3);
        let mut lost = Vec::new();
        for (index, record) in self.records().enumerate() {
            let Some(&[prompt, output, end]) = places.next() else {
                lost.push(index);
                continue;
            };
            let (prompt, output, end) = (spot(prompt), spot(output), spot(end));
            match prompt {
                Some(prompt)
                    if output.is_some() == record.output.is_some()
                        && end.is_some() == record.end.is_some() =>
                {
                    record.prompt = prompt;
                    record.output = output;
                    record.end = end;
                }
                _ => lost.push(index),
            }
        }
        self.drop_records(&lost);
    }

    /// Drops the commands whose prompt and command line read differently now: all of them, or
    /// only those that reach the screen.
    fn check(&mut self, grid: &Grid<Cell>, all: bool) {
        let base = self.base;
        let lost: Vec<usize> = self
            .records()
            .enumerate()
            .filter(|(_, record)| {
                let (Some(output), Some(text)) = (record.output, record.text) else {
                    return false;
                };
                (all || output.row - base >= 0)
                    && read(grid, base, record.prompt, output) != Some(text)
            })
            .map(|(index, _)| index)
            .collect();
        self.drop_records(&lost);
    }

    /// Drops records by their place in [`Self::records`], ascending.
    fn drop_records(&mut self, lost: &[usize]) {
        let done = self.done.len();
        if lost.last().is_some_and(|&index| index >= done) {
            self.open = None;
        }
        for &index in lost.iter().rev().filter(|&&index| index < done) {
            self.done.remove(index);
        }
    }
}

fn alternate<T>(term: &Term<T>) -> bool {
    term.mode().contains(TermMode::ALT_SCREEN)
}

/// The text from `from` up to `to`, as a number that stays the same however the lines are
/// wrapped: wrapped rows run on, spaces at the end of a line are left out. `None` when it is not
/// all in the grid.
fn read(grid: &Grid<Cell>, base: i64, from: Spot, to: Spot) -> Option<u64> {
    let (top, bottom) = (grid.topmost_line().0, grid.bottommost_line().0);
    let columns = grid.columns();
    let (from_line, to_line) = (from.row - base, to.row - base);
    if from_line < i64::from(top)
        || to_line > i64::from(bottom)
        || (from_line, from.col) > (to_line, to.col)
    {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    let mut spaces = 0;
    for line in from_line as i32..=to_line as i32 {
        let row = &grid[Line(line)];
        let first = if i64::from(line) == from_line {
            from.col
        } else {
            0
        };
        let last = if i64::from(line) == to_line {
            to.col.min(columns)
        } else {
            columns
        };
        for cell in (first..last).map(|col| &row[Column(col)]) {
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            if cell.c == ' ' {
                spaces += 1;
                continue;
            }
            for _ in 0..spaces {
                ' '.hash(&mut hasher);
            }
            spaces = 0;
            cell.c.hash(&mut hasher);
        }
        if i64::from(line) < to_line && !row[Column(columns - 1)].flags.contains(Flags::WRAPLINE) {
            spaces = 0;
            '\n'.hash(&mut hasher);
        }
    }
    Some(hasher.finish())
}

/// The grid's rows grouped into the lines the program wrote: a row the text wrapped onto belongs
/// with the one before it.
struct Wrapped {
    top: i32,
    bottom: i32,
    /// The first row of each line, top to bottom.
    starts: Vec<i32>,
    /// The line the cursor is on, as an index into `starts`.
    cursor: usize,
}

impl Wrapped {
    fn new(grid: &Grid<Cell>) -> Self {
        let (top, bottom) = (grid.topmost_line().0, grid.bottommost_line().0);
        let last = Column(grid.columns() - 1);
        let mut starts = vec![top];
        starts.extend(
            (top + 1..=bottom)
                .filter(|&line| !grid[Line(line - 1)][last].flags.contains(Flags::WRAPLINE)),
        );
        let cursor = starts.partition_point(|&start| start <= grid.cursor.point.line.0) - 1;
        Self {
            top,
            bottom,
            starts,
            cursor,
        }
    }

    /// Cells a row holds: one fewer when a wide character did not fit at its end.
    fn width(grid: &Grid<Cell>, line: i32) -> usize {
        let columns = grid.columns();
        let spacer = grid[Line(line)][Column(columns - 1)]
            .flags
            .contains(Flags::LEADING_WIDE_CHAR_SPACER);
        columns - usize::from(spacer)
    }

    fn place(&self, grid: &Grid<Cell>, line: i32, col: usize) -> Option<Place> {
        if line < self.top || line > self.bottom {
            return None;
        }
        let index = self.starts.partition_point(|&start| start <= line) - 1;
        let offset = (self.starts[index]..line)
            .map(|row| Self::width(grid, row))
            .sum::<usize>()
            + col;
        Some(Place {
            up: self.cursor as i64 - index as i64,
            offset,
        })
    }

    fn spot(&self, grid: &Grid<Cell>, place: Place) -> Option<(i32, usize)> {
        let index = usize::try_from(self.cursor as i64 - place.up).ok()?;
        let start = *self.starts.get(index)?;
        let end = self
            .starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.bottom + 1);
        let (mut line, mut offset) = (start, place.offset);
        while line + 1 < end && offset >= Self::width(grid, line) {
            offset -= Self::width(grid, line);
            line += 1;
        }
        Some((line, offset.min(grid.columns())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};

    const A: &str = "\x1b]133;A\x07";
    const B: &str = "\x1b]133;B\x07";
    const C: &str = "\x1b]133;C\x07";

    fn shell(rows: u16, cols: u16) -> Screen {
        let mut screen = Screen::new(Size { rows, cols });
        screen.keep_segments();
        screen
    }

    fn end(code: i32) -> String {
        format!("\x1b]133;D;{code}\x07")
    }

    /// A prompt, as a shell draws it.
    fn prompt(screen: &mut Screen) {
        screen.process(format!("{A}$ {B}").as_bytes());
    }

    /// A command typed at the prompt, its output and its end.
    fn enter(screen: &mut Screen, command: &str, output: &str, code: i32) {
        screen.process(format!("{command}\r\n{C}{output}{}", end(code)).as_bytes());
    }

    fn run(screen: &mut Screen, command: &str, output: &str, code: i32) {
        prompt(screen);
        enter(screen, command, output, code);
    }

    /// The line as the program wrote it from `line` on, wrapped rows together.
    fn line_text(screen: &Screen, line: Line) -> String {
        let grid = screen.term.grid();
        let last = Column(grid.columns() - 1);
        let mut end = line;
        while grid[end][last].flags.contains(Flags::WRAPLINE) {
            end += 1;
        }
        screen
            .term
            .bounds_to_string(Point::new(line, Column(0)), Point::new(end, last))
    }

    /// Each command's prompt line and output, as the screen reads now.
    fn read_back(screen: &Screen) -> Vec<(String, Option<String>)> {
        let columns = screen.term.columns();
        screen
            .segments()
            .iter()
            .map(|segment| {
                let output = segment
                    .output(columns)
                    .map(|(first, last)| screen.term.bounds_to_string(first, last));
                (line_text(screen, segment.prompt), output)
            })
            .collect()
    }

    fn kept(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
        pairs
            .iter()
            .map(|(prompt, output)| ((*prompt).to_owned(), output.map(str::to_owned)))
            .collect()
    }

    #[test]
    fn marks_and_exit_statuses_are_parsed() {
        let parse = |text: &str| {
            let params: Vec<&[u8]> = text.split(';').map(str::as_bytes).collect();
            Mark::parse(&params)
        };
        assert_eq!(parse("133;A"), Some(Mark::Prompt));
        assert_eq!(parse("133;A;aid=7;cl=m"), Some(Mark::Prompt));
        assert_eq!(parse("133;B"), Some(Mark::Input));
        assert_eq!(parse("133;C"), Some(Mark::Output));
        assert_eq!(parse("133;C;"), Some(Mark::Output));
        assert_eq!(parse("133;D"), Some(Mark::End(None)));
        assert_eq!(parse("133;D;0"), Some(Mark::End(Some(0))));
        assert_eq!(parse("133;D;130;aid=7"), Some(Mark::End(Some(130))));
        assert_eq!(parse("133;D;-1"), Some(Mark::End(Some(-1))));
        assert_eq!(parse("133;D;"), Some(Mark::End(None)));
        assert_eq!(parse("133;D;x"), Some(Mark::End(None)));
        assert_eq!(parse("133;D;99999999999"), Some(Mark::End(None)));
        for other in ["133", "133;", "133;P;k=i", "133;Z", "9;4;1;40", "1337;A"] {
            assert_eq!(parse(other), None, "{other}");
        }
    }

    #[test]
    fn each_command_keeps_its_prompt_output_and_status() {
        let mut screen = shell(10, 20);
        run(&mut screen, "true", "", 0);
        run(&mut screen, "false", "no\r\n", 1);
        run(&mut screen, "printf hi", "hi", 0);
        screen.process(format!("{A}$ {B}sleep").as_bytes());
        let segments = screen.segments();
        assert_eq!(
            segments.len(),
            3,
            "the prompt being typed at is not a command yet"
        );
        assert_eq!(
            segments
                .iter()
                .map(|s| (s.prompt.0, s.exit, s.failed()))
                .collect::<Vec<_>>(),
            [(0, Some(0), false), (1, Some(1), true), (3, Some(0), false)]
        );
        assert_eq!(segments[0].output(20), None, "printed nothing");
        assert_eq!(segments[0].last(), Line(0), "its command line");
        assert_eq!(
            segments[1].output(20),
            Some((
                Point::new(Line(2), Column(0)),
                Point::new(Line(2), Column(19))
            ))
        );
        assert_eq!(segments[1].last(), Line(2));
        assert_eq!(
            segments[2].output(20),
            Some((
                Point::new(Line(4), Column(0)),
                Point::new(Line(4), Column(1))
            ))
        );
        assert_eq!(segments[2].last(), Line(4));
        assert_eq!(
            read_back(&screen),
            kept(&[
                ("$ true", None),
                ("$ false", Some("no")),
                ("$ printf hi", Some("hi")),
            ])
        );
        // Once it runs it is a command, without an end yet.
        screen.process(format!("\r\n{C}").as_bytes());
        let running = screen.segments();
        assert_eq!(running.len(), 4);
        assert_eq!((running[3].end, running[3].exit), (None, None));
        assert!(!running[3].failed());
    }

    #[test]
    fn a_status_the_shell_did_not_give_is_not_a_failure() {
        let mut screen = shell(10, 20);
        prompt(&mut screen);
        screen.process(format!("one\r\n{C}out\r\n\x1b]133;D\x07").as_bytes());
        // Ended by the next prompt, without a D.
        prompt(&mut screen);
        screen.process(format!("two\r\n{C}out\r\n").as_bytes());
        prompt(&mut screen);
        let segments = screen.segments();
        assert_eq!(
            segments
                .iter()
                .map(|s| (s.exit, s.failed()))
                .collect::<Vec<_>>(),
            [(None, false), (None, false)]
        );
        assert_eq!(segments[1].end, Some(Point::new(Line(4), Column(0))));
    }

    #[test]
    fn repeated_marks_from_two_integrations_make_one_command() {
        let mut screen = shell(10, 30);
        let code = end(1);
        screen.process(
            format!("{code}{A}{A}$ {B}{B}false\r\n{C}{C}{code}{code}{A}{A}$ {B}{B}").as_bytes(),
        );
        let segments = screen.segments();
        assert_eq!(segments.len(), 1);
        assert_eq!((segments[0].prompt, segments[0].exit), (Line(0), Some(1)));
    }

    #[test]
    fn a_prompt_left_without_running_anything_is_not_a_command() {
        let mut screen = shell(10, 30);
        // Enter on an empty line (zsh: a new prompt; bash: a D without a C), then ^C at a prompt.
        prompt(&mut screen);
        screen.process(b"\r\n");
        prompt(&mut screen);
        screen.process(format!("\r\n{}", end(0)).as_bytes());
        prompt(&mut screen);
        screen.process(format!("typed^C\r\n{}", end(130)).as_bytes());
        run(&mut screen, "true", "", 0);
        let segments = screen.segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(line_text(&screen, segments[0].prompt), "$ true");
    }

    #[test]
    fn marks_stay_on_their_text_as_output_scrolls_into_history() {
        let mut screen = shell(5, 20);
        run(&mut screen, "one", "first\r\n", 0);
        let many: String = (0..30).map(|n| format!("line {n}\r\n")).collect();
        run(&mut screen, "two", &many, 3);
        prompt(&mut screen);
        assert!(screen.term.grid().history_size() > 0);
        let lines = many.trim_end().replace("\r\n", "\n");
        let expected = kept(&[("$ one", Some("first")), ("$ two", Some(&lines))]);
        assert_eq!(read_back(&screen), expected);
        assert!(screen.segments()[0].prompt.0 < 0, "in history");
        // Scrolling the view does not move the text.
        screen
            .term
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(7));
        assert_eq!(read_back(&screen), expected);
        // Each command keeps its number as its lines scroll.
        let ids =
            |screen: &Screen| -> Vec<i64> { screen.segments().iter().map(|s| s.id).collect() };
        let before = ids(&screen);
        assert_ne!(before[0], before[1]);
        enter(&mut screen, "three", &many, 0);
        prompt(&mut screen);
        assert_eq!(ids(&screen)[..2], before[..]);
    }

    #[test]
    fn trimmed_history_takes_the_commands_that_started_in_it() {
        let mut screen = shell(5, 20);
        run(&mut screen, "one", "first\r\n", 0);
        // Rows 0 and 1, then the second command's prompt on row 2 and its output: enough to
        // push row 0 out of the history kept, and no more.
        run(&mut screen, "two", &"x\r\n".repeat(HISTORY + 2), 0);
        prompt(&mut screen);
        assert_eq!(
            screen.term.grid().history_size(),
            HISTORY,
            "as much history as before"
        );
        let xs = vec!["x"; HISTORY + 2].join("\n");
        assert_eq!(read_back(&screen), kept(&[("$ two", Some(&xs))]));
        assert_eq!(line_text(&screen, screen.segments()[0].prompt - 1), "first");
        // Long after, the second goes too; the count has not slipped.
        enter(&mut screen, "three", &"y\r\n".repeat(HISTORY), 0);
        prompt(&mut screen);
        enter(&mut screen, "four", "z\r\n", 1);
        prompt(&mut screen);
        let ys = vec!["y"; HISTORY].join("\n");
        assert_eq!(
            read_back(&screen),
            kept(&[("$ three", Some(&ys)), ("$ four", Some("z"))])
        );
    }

    #[test]
    fn clearing_the_screen_keeps_marks_and_clearing_history_drops_them() {
        let mut screen = shell(6, 20);
        run(&mut screen, "one", "first\r\n", 0);
        run(&mut screen, "two", "second\r\n", 1);
        prompt(&mut screen);
        let before = read_back(&screen);
        // ^L in zsh: the screen is cleared and the prompt drawn again at the top.
        screen.process(format!("\x1b[H\x1b[2J{A}$ {B}").as_bytes());
        assert_eq!(
            read_back(&screen),
            before,
            "scrolled into history, still on their text"
        );
        assert!(screen.segments().iter().all(|s| s.prompt.0 < 0));
        // `clear`: its own prompt goes with the history it clears.
        enter(&mut screen, "clear", "\x1b[H\x1b[2J\x1b[3J", 0);
        prompt(&mut screen);
        assert_eq!(read_back(&screen), []);
        enter(&mut screen, "three", "third\r\n", 0);
        assert_eq!(read_back(&screen), kept(&[("$ three", Some("third"))]));
    }

    #[test]
    fn a_reset_terminal_has_no_commands() {
        let mut screen = shell(6, 20);
        run(&mut screen, "one", "first\r\n", 0);
        screen.process(b"\x1bc");
        assert_eq!(screen.segments(), []);
    }

    #[test]
    fn resizing_rewraps_marks_with_their_text() {
        let mut screen = shell(6, 20);
        run(
            &mut screen,
            "echo a long line that wraps",
            "output that is long enough to wrap\r\n",
            0,
        );
        run(
            &mut screen,
            "wide 你好你好你好你好",
            "你好你好你好你好你好你好\r\n",
            2,
        );
        prompt(&mut screen);
        let expected = kept(&[
            (
                "$ echo a long line that wraps",
                Some("output that is long enough to wrap"),
            ),
            ("$ wide 你好你好你好你好", Some("你好你好你好你好你好你好")),
        ]);
        assert_eq!(read_back(&screen), expected);
        for (rows, cols) in [(6, 13), (4, 7), (9, 50), (3, 9), (6, 20), (20, 11)] {
            screen.resize(Size { rows, cols });
            assert_eq!(read_back(&screen), expected, "{rows}x{cols}");
            assert_eq!(screen.segments()[1].exit, Some(2));
        }
    }

    #[test]
    fn nothing_is_kept_from_the_alternate_screen() {
        let mut screen = shell(6, 20);
        run(&mut screen, "one", "first\r\n", 0);
        prompt(&mut screen);
        screen.process(format!("vim\r\n{C}\x1b[?1049h").as_bytes());
        // What a program draws there, marks included, is not a command.
        run(&mut screen, "inside", "drawn\r\n", 1);
        assert_eq!(screen.segments(), [], "nothing to draw over the program");
        screen.process(format!("\x1b[?1049l{}", end(0)).as_bytes());
        prompt(&mut screen);
        assert_eq!(
            read_back(&screen),
            kept(&[("$ one", Some("first")), ("$ vim", None)])
        );
    }

    #[test]
    fn a_resize_while_away_on_the_alternate_screen_is_followed() {
        let mut screen = shell(6, 20);
        run(
            &mut screen,
            "echo a long line that wraps",
            "output long enough to wrap\r\n",
            0,
        );
        prompt(&mut screen);
        screen.process(format!("less\r\n{C}\x1b[?1049h").as_bytes());
        screen.resize(Size { rows: 4, cols: 9 });
        screen.resize(Size { rows: 5, cols: 12 });
        screen.process(format!("\x1b[?1049l{}", end(0)).as_bytes());
        prompt(&mut screen);
        assert_eq!(
            read_back(&screen),
            kept(&[
                (
                    "$ echo a long line that wraps",
                    Some("output long enough to wrap")
                ),
                ("$ less", None),
            ])
        );
    }

    #[test]
    fn a_command_written_over_is_dropped() {
        let mut screen = shell(8, 20);
        run(&mut screen, "one", "first\r\n", 0);
        run(&mut screen, "two", "second\r\n", 0);
        // A program moves up and writes over the first command's line.
        screen.process(b"\x1b[1;1Hoverwritten\x1b[8;1H");
        assert_eq!(read_back(&screen), kept(&[("$ two", Some("second"))]));
    }

    #[test]
    fn marks_in_a_synchronized_update_or_split_reads_land_on_their_text() {
        let stream = format!(
            "{A}$ {B}one\r\n{C}first\r\n{}\x1b[?2026h{A}$ {B}two\r\n{C}second\r\n{}\x1b[?2026l{A}$ {B}",
            end(0),
            end(1)
        );
        let expected = kept(&[("$ one", Some("first")), ("$ two", Some("second"))]);
        let mut whole = shell(8, 20);
        whole.process(stream.as_bytes());
        assert_eq!(read_back(&whole), expected);
        let mut bytes = shell(8, 20);
        for byte in stream.as_bytes() {
            bytes.process(&[*byte]);
        }
        assert_eq!(read_back(&bytes), expected);
        assert_eq!(bytes.segments(), whole.segments());
    }

    #[test]
    fn a_terminal_that_does_not_keep_segments_ignores_marks() {
        let mut screen = Screen::new(Size { rows: 6, cols: 20 });
        run(&mut screen, "one", "first\r\n", 1);
        assert_eq!(screen.segments(), []);
        // Its history is as before.
        screen.process("x\r\n".repeat(HISTORY + 50).as_bytes());
        assert_eq!(screen.term.grid().history_size(), HISTORY);
    }
}
