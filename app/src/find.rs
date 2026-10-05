//! Finding text in a terminal's retained output, as Saddle's history search does
//! (`src/history.rs` at commit `df1c727`): the query is taken literally, letter case counts only
//! when it has a capital letter, and a match is selected and scrolled into view. The command
//! palette lists the matches in every open pane by the same rules.
use alacritty_terminal::{
    Term,
    event::EventListener,
    grid::Dimensions,
    index::{Boundary, Column, Direction, Line, Point, Side},
    selection::{Selection, SelectionType},
    term::{
        cell::Flags,
        search::{Match, RegexSearch},
    },
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    /// From the bottom of the view upwards, ignoring any selection: a query just typed.
    First,
    /// Before the current match, towards older output, wrapping round.
    Older,
    /// After the current match, towards newer output, wrapping round.
    Newer,
}

/// The query as a literal pattern.
fn literal(query: &str) -> String {
    let mut pattern = String::new();
    for c in query.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern
}

/// Selects the next match of `query` and scrolls to it; false when there is none (the selection
/// is then left alone).
pub fn find<T: EventListener>(term: &mut Term<T>, query: &str, how: Find) -> bool {
    if query.is_empty() {
        return false;
    }
    let Ok(mut regex) = RegexSearch::new(&literal(query)) else {
        return false;
    };
    let offset = term.grid().display_offset() as i32;
    let bottom = Point::new(
        Line(term.screen_lines() as i32 - 1 - offset),
        term.last_column(),
    );
    let top = Point::new(Line(-offset), Column(0));
    let current = term.selection.as_ref().and_then(|s| s.to_range(term));
    let found = match how {
        Find::First => term.search_next(&mut regex, bottom, Direction::Left, Side::Left, None),
        Find::Older => {
            let origin = current.map_or(bottom, |r| r.start.sub(term, Boundary::None, 1));
            term.search_next(&mut regex, origin, Direction::Left, Side::Left, None)
        }
        Find::Newer => {
            let origin = current.map_or(top, |r| r.end.add(term, Boundary::None, 1));
            term.search_next(&mut regex, origin, Direction::Right, Side::Left, None)
        }
    };
    let Some(found) = found else {
        return false;
    };
    select(term, found);
    true
}

fn select<T: EventListener>(term: &mut Term<T>, found: Match) {
    let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
    selection.update(*found.end(), Side::Right);
    term.selection = Some(selection);
    term.scroll_to_point(*found.start());
    term.scroll_to_point(*found.end());
}

/// A match the command palette lists: where it is, and the line it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    start: Point,
    end: Point,
    /// How long the history was, to find the match again once more output has pushed it up.
    history: usize,
    /// The line the match starts on, without its trailing blanks.
    pub line: String,
    /// The match in `line`; to the line's end when it goes on to the next.
    pub range: Range<usize>,
}

/// Up to `limit` matches of `query` in everything `term` keeps, newest first as ⌘G goes, and
/// whether it has more.
pub fn hits<T>(term: &Term<T>, query: &str, limit: usize) -> (Vec<Hit>, bool) {
    let mut hits = Vec::new();
    if query.is_empty() {
        return (hits, false);
    }
    let Ok(mut regex) = RegexSearch::new(&literal(query)) else {
        return (hits, false);
    };
    let top = Point::new(term.topmost_line(), Column(0));
    let mut origin = Point::new(term.bottommost_line(), term.last_column());
    while let Some(found) = term.regex_search_left(&mut regex, origin, top) {
        if hits.len() == limit {
            return (hits, true);
        }
        hits.push(hit(term, &found));
        if *found.start() <= top || *found.start() > origin {
            break;
        }
        origin = found.start().sub(term, Boundary::Grid, 1);
    }
    (hits, false)
}

/// Rows of a wrapped line joined at most, either side of a match's: enough for the palette's row.
const WRAPPED: i32 = 8;

fn hit<T>(term: &Term<T>, found: &Match) -> Hit {
    let (start, end) = (*found.start(), *found.end());
    let grid = term.grid();
    let wraps = |line: Line| {
        grid[line][term.last_column()]
            .flags
            .contains(Flags::WRAPLINE)
    };
    // The whole line the terminal wrapped across rows, as far as it goes.
    let mut first = start.line;
    while first > term.topmost_line() && first > start.line - WRAPPED && wraps(first - 1) {
        first -= 1;
    }
    let mut last = start.line.max(end.line);
    while last < term.bottommost_line() && last < start.line + WRAPPED && wraps(last) {
        last += 1;
    }
    let mut line = String::new();
    let (mut from, mut to) = (None, None);
    for row in first.0..=last.0 {
        for column in 0..term.columns() {
            let point = Point::new(Line(row), Column(column));
            if point == start {
                from = Some(line.len());
            }
            let cell = &grid[point];
            if !cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                line.push(cell.c);
                line.extend(cell.zerowidth().into_iter().flatten());
            }
            if point == end {
                to = Some(line.len());
            }
        }
    }
    let from = from.unwrap_or(line.len());
    let to = to.unwrap_or(line.len()).max(from);
    line.truncate(line.trim_end().len().max(to));
    Hit {
        start,
        end,
        history: term.history_size(),
        line,
        range: from..to,
    }
}

/// Selects the palette's `hit` of `query` and scrolls to it: where it was, gone up by the output
/// since; or, when it is no longer there, the newest match. False when there is none.
pub fn reveal<T: EventListener>(term: &mut Term<T>, query: &str, hit: &Hit) -> bool {
    let Ok(mut regex) = RegexSearch::new(&literal(query)) else {
        return false;
    };
    let up = term.history_size().saturating_sub(hit.history) as i32;
    let (start, end) = (
        Point::new(hit.start.line - up, hit.start.column),
        Point::new(hit.end.line - up, hit.end.column),
    );
    let inside = |point: Point| {
        point.line >= term.topmost_line()
            && point.line <= term.bottommost_line()
            && point.column <= term.last_column()
    };
    if inside(start)
        && inside(end)
        && term.regex_search_right(&mut regex, start, end) == Some(start..=end)
    {
        select(term, start..=end);
        return true;
    }
    let top = Point::new(term.topmost_line(), Column(0));
    let bottom = Point::new(term.bottommost_line(), term.last_column());
    match term.regex_search_left(&mut regex, bottom, top) {
        Some(found) => {
            select(term, found);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};

    fn screen(text: &str) -> Screen {
        let mut screen = Screen::new(Size { rows: 4, cols: 20 });
        screen.process(text.as_bytes());
        screen
    }

    /// The selected text and the line it starts on.
    fn selected(screen: &Screen) -> (String, i32) {
        let term = &screen.term;
        let range = term.selection.as_ref().unwrap().to_range(term).unwrap();
        (term.selection_to_string().unwrap(), range.start.line.0)
    }

    #[test]
    fn older_then_newer_wrapping_round() {
        // Seven lines on a four-line screen: the first three are in the history.
        let mut s = screen("alpha 0\r\nbeta\r\nalpha 2\r\ngamma\r\nalpha 4\r\ndelta\r\nend");
        assert!(find(&mut s.term, "alpha", Find::First));
        assert_eq!(selected(&s), ("alpha".into(), 1));
        assert!(find(&mut s.term, "alpha", Find::Older));
        assert_eq!(selected(&s).1, -1);
        assert!(find(&mut s.term, "alpha", Find::Older));
        assert_eq!(selected(&s).1, -3);
        assert!(s.term.grid().display_offset() >= 3);
        // Past the oldest it wraps round to the newest.
        assert!(find(&mut s.term, "alpha", Find::Older));
        assert_eq!(selected(&s).1, 1);
        assert!(find(&mut s.term, "alpha", Find::Newer));
        assert_eq!(selected(&s).1, -3);
    }

    #[test]
    fn no_match_leaves_the_selection() {
        let mut s = screen("one\r\ntwo");
        assert!(find(&mut s.term, "two", Find::First));
        assert!(!find(&mut s.term, "three", Find::Older));
        assert_eq!(selected(&s).0, "two");
        assert!(!find(&mut s.term, "", Find::First));
    }

    #[test]
    fn literal_text_and_smart_case() {
        let mut s = screen("a.b axb\r\nError error");
        assert!(find(&mut s.term, "a.b", Find::First));
        assert_eq!(selected(&s).0, "a.b");
        assert!(!find(&mut s.term, "a+b", Find::First));
        // Lower case matches either case; a capital asks for that case.
        assert!(find(&mut s.term, "error", Find::First));
        assert_eq!(selected(&s).0, "error");
        assert!(find(&mut s.term, "error", Find::Older));
        assert_eq!(selected(&s).0, "Error");
        assert!(find(&mut s.term, "Error", Find::First));
        assert_eq!(selected(&s).0, "Error");
        assert!(find(&mut s.term, "Error", Find::Older));
        assert_eq!(selected(&s).0, "Error");
    }

    fn lines(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.line.as_str()).collect()
    }

    fn matched(hit: &Hit) -> &str {
        &hit.line[hit.range.clone()]
    }

    #[test]
    fn hits_go_newest_first_up_to_the_limit() {
        let s = screen("alpha 0\r\nbeta\r\nalpha 2\r\ngamma\r\nalpha 4\r\ndelta\r\nend");
        let (all, more) = hits(&s.term, "alpha", 10);
        // The history counts too, as for ⌘G.
        assert_eq!(lines(&all), ["alpha 4", "alpha 2", "alpha 0"]);
        assert!(!more);
        assert_eq!(all[0].range, 0..5);
        let (two, more) = hits(&s.term, "alpha", 2);
        assert_eq!(lines(&two), ["alpha 4", "alpha 2"]);
        assert!(more);
        assert!(!hits(&s.term, "alpha", 3).1);
        assert_eq!(hits(&s.term, "", 10), (Vec::new(), false));
        assert_eq!(hits(&s.term, "omega", 10), (Vec::new(), false));
    }

    #[test]
    fn hits_take_the_text_literally_and_case_only_with_a_capital() {
        let s = screen("a.b axb\r\nError error");
        let (dots, _) = hits(&s.term, "a.b", 10);
        assert_eq!(dots.len(), 1);
        assert_eq!(matched(&dots[0]), "a.b");
        let (any, _) = hits(&s.term, "error", 10);
        assert_eq!(
            any.iter().map(matched).collect::<Vec<_>>(),
            ["error", "Error"]
        );
        let (capital, _) = hits(&s.term, "Error", 10);
        assert_eq!(capital.len(), 1);
        assert_eq!(capital[0].range, 0..5);
    }

    #[test]
    fn hits_mark_the_match_in_its_line() {
        let s = screen("one two two\r\n  wide 中文 here");
        let (twice, _) = hits(&s.term, "two", 10);
        let ranges: Vec<_> = twice.iter().map(|h| h.range.clone()).collect();
        assert_eq!(ranges, [8..11, 4..7]);
        // Wide characters take two cells but are one character of the line.
        let (wide, _) = hits(&s.term, "here", 10);
        assert_eq!(wide[0].line, "  wide 中文 here");
        assert_eq!(matched(&wide[0]), "here");
        // A line the terminal wrapped is listed whole, the match across the wrap marked in full.
        let long = format!("{}abcd", "x".repeat(18));
        let s = screen(&long);
        let (wrapped, _) = hits(&s.term, "abcd", 10);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].line, long);
        assert_eq!(matched(&wrapped[0]), "abcd");
        // A match on a later row of a wrapped line has the rows before it too.
        let long = format!("{} tail needle", "y".repeat(25));
        let s = screen(&format!("{long}\r\nnext"));
        let (later, _) = hits(&s.term, "needle", 10);
        assert_eq!(later[0].line, long);
        assert_eq!(matched(&later[0]), "needle");
    }

    #[test]
    fn a_hit_is_found_again_after_more_output() {
        let mut s = screen("alpha 0\r\nbeta\r\nalpha 2\r\n");
        let (found, _) = hits(&s.term, "alpha", 10);
        assert_eq!(lines(&found), ["alpha 2", "alpha 0"]);
        // Four more lines push "alpha 0" from the top row into the history.
        s.process(b"gamma\r\ndelta\r\nepsilon\r\nzeta\r\n");
        assert!(reveal(&mut s.term, "alpha", &found[1]));
        assert_eq!(selected(&s), ("alpha".into(), -4));
        assert!(s.term.grid().display_offset() >= 4);
        // ⌘G goes on from it, towards older output: none, so round to the newest.
        assert!(find(&mut s.term, "alpha", Find::Older));
        assert_eq!(selected(&s), ("alpha".into(), -2));
    }

    #[test]
    fn a_hit_no_longer_there_gives_way_to_the_newest_match() {
        let old = screen("x alpha");
        let (found, _) = hits(&old.term, "alpha", 10);
        let mut s = screen("alpha one\r\nalpha two\r\nzzz");
        assert!(reveal(&mut s.term, "alpha", &found[0]));
        assert_eq!(selected(&s), ("alpha".into(), 1));
        let mut none = screen("nothing");
        assert!(!reveal(&mut none.term, "alpha", &found[0]));
        assert!(none.term.selection.is_none());
    }
}
