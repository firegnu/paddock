//! Finding text in a terminal's retained output, as Saddle's history search does
//! (`src/history.rs` at commit `df1c727`): the query is taken literally, letter case counts only
//! when it has a capital letter, and a match is selected and scrolled into view.
use alacritty_terminal::{
    Term,
    event::EventListener,
    grid::Dimensions,
    index::{Boundary, Column, Direction, Line, Point, Side},
    selection::{Selection, SelectionType},
    term::search::RegexSearch,
};

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
    let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
    selection.update(*found.end(), Side::Right);
    term.selection = Some(selection);
    term.scroll_to_point(*found.start());
    term.scroll_to_point(*found.end());
    true
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
}
