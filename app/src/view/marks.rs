//! The selection and the find bar's match, drawn as one rounded, see-through shape over the cells'
//! backgrounds and under the text (P5-143d3, the user's choice from the mockup: rows that touch
//! join into one shape, its outer and inner corners rounded alike; 4 px; 60% opaque). Plain
//! geometry here; the pane (`view.rs`) finds the rows and paints the outlines.
use alacritty_terminal::{
    Term,
    grid::Dimensions,
    index::{Column, Line, Point},
    term::cell::Flags,
};

/// The corners' radius, in pixels.
pub const RADIUS: f32 = 4.0;
/// How opaque the selection is.
pub const OPACITY: f32 = 0.6;
/// How opaque the find bar's match is, in the theme's yellow.
pub const FOUND_OPACITY: f32 = 0.33;
/// How opaque the find bar's other matches are, in the same yellow (P5-143e).
pub const OTHERS_OPACITY: f32 = 0.14;

/// The cells `covers` takes in on the rows shown (a selection, a match), `first` (-1 while the
/// view is part of a line down) to `rows`: each row's first column and the one past its last.
pub fn spans<T>(
    term: &Term<T>,
    covers: impl Fn(Point) -> bool,
    first: i32,
    rows: u16,
    cols: u16,
) -> Vec<(i32, usize, usize)> {
    let offset = term.grid().display_offset() as i32;
    let cols = usize::from(cols).min(term.columns());
    let mut spans = Vec::new();
    for row in first..i32::from(rows) {
        let line = Line(row - offset);
        let covered = |&col: &usize| covers(Point::new(line, Column(col)));
        if let Some(mut start) = (0..cols).find(covered) {
            let mut end = (start..cols)
                .rev()
                .find(covered)
                .map_or(start, |last| last + 1);
            // Half a wide character: the shape takes it whole.
            let flags = |col: usize| term.grid()[Point::new(line, Column(col))].flags;
            if start > 0 && flags(start).contains(Flags::WIDE_CHAR_SPACER) {
                start -= 1;
            }
            if end < cols && flags(end - 1).contains(Flags::WIDE_CHAR) {
                end += 1;
            }
            spans.push((row, start, end));
        }
    }
    spans
}

/// A colour that, `opacity` opaque over `background`, looks like `color`: the selection keeps its
/// look on the plain background, and a cell's own background shows through it.
pub fn lifted(color: (u8, u8, u8), background: (u8, u8, u8), opacity: f32) -> (u8, u8, u8) {
    let channel = |c: u8, b: u8| {
        let (c, b) = (f32::from(c), f32::from(b));
        (b + (c - b) / opacity).round().clamp(0.0, 255.0) as u8
    };
    (
        channel(color.0, background.0),
        channel(color.1, background.1),
        channel(color.2, background.2),
    )
}

/// The colour and opacity the shape is filled with: the find bar's match in the theme's yellow,
/// a selection in its own colour, both see-through.
pub fn fill(theme: &crate::palette::Theme, found: bool) -> ((u8, u8, u8), f32) {
    if found {
        (theme.ansi[11], FOUND_OPACITY)
    } else {
        (lifted(theme.selection, theme.background, OPACITY), OPACITY)
    }
}

/// The find bar's other matches: the current one's yellow, fainter (P5-143e).
pub fn others(theme: &crate::palette::Theme) -> ((u8, u8, u8), f32) {
    (theme.ansi[11], OTHERS_OPACITY)
}

/// A step of an outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Line(f32, f32),
    /// To `to`, bending toward `ctrl` (the corner).
    Curve {
        to: (f32, f32),
        ctrl: (f32, f32),
    },
}

/// A closed outline, from `start`.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub start: (f32, f32),
    pub steps: Vec<Step>,
}

/// The outlines around rows of `(row, left, right)` in pixels, in row order, each row `line` tall
/// from `row * line`: rows that follow on and overlap join into one; every corner, out or in, is
/// rounded by `radius`, or half the shorter side next to it.
pub fn outlines(rows: &[(i32, f32, f32)], line: f32, radius: f32) -> Vec<Outline> {
    polygons(rows, line)
        .iter()
        .map(|points| rounded(points, radius))
        .collect()
}

/// The corners of each joined shape, clockwise from the first row's top left.
fn polygons(rows: &[(i32, f32, f32)], line: f32) -> Vec<Vec<(f32, f32)>> {
    let mut groups: Vec<Vec<(i32, f32, f32)>> = Vec::new();
    for &(row, left, right) in rows {
        match groups.last_mut() {
            Some(group)
                if group
                    .last()
                    .is_some_and(|&(above, l, r)| row == above + 1 && left < r && right > l) =>
            {
                group.push((row, left, right))
            }
            _ => groups.push(vec![(row, left, right)]),
        }
    }
    groups
        .iter()
        .map(|group| {
            let top = |row: i32| row as f32 * line;
            let (first, last) = (group[0], group[group.len() - 1]);
            let mut points = vec![(first.1, top(first.0)), (first.2, top(first.0))];
            // Down the right side, a step at each row whose end differs.
            for pair in group.windows(2) {
                let (upper, lower) = (pair[0], pair[1]);
                if upper.2 != lower.2 {
                    points.push((upper.2, top(lower.0)));
                    points.push((lower.2, top(lower.0)));
                }
            }
            points.push((last.2, top(last.0 + 1)));
            points.push((last.1, top(last.0 + 1)));
            // Up the left side.
            for pair in group.windows(2).rev() {
                let (upper, lower) = (pair[0], pair[1]);
                if upper.1 != lower.1 {
                    points.push((lower.1, top(lower.0)));
                    points.push((upper.1, top(lower.0)));
                }
            }
            points
        })
        .collect()
}

fn rounded(points: &[(f32, f32)], radius: f32) -> Outline {
    let n = points.len();
    let length = |a: (f32, f32), b: (f32, f32)| (b.0 - a.0).hypot(b.1 - a.1);
    let toward = |from: (f32, f32), to: (f32, f32), by: f32| {
        let whole = length(from, to);
        if whole == 0.0 {
            return from;
        }
        let t = by / whole;
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
    };
    // Each corner: where the rounding starts on the side before it, and ends on the side after.
    let corners: Vec<_> = (0..n)
        .map(|k| {
            let (before, at, after) = (points[(k + n - 1) % n], points[k], points[(k + 1) % n]);
            let r = radius
                .min(length(before, at) / 2.0)
                .min(length(at, after) / 2.0);
            (toward(at, before, r), at, toward(at, after, r))
        })
        .collect();
    let mut steps = Vec::with_capacity(2 * n);
    for (k, &(into, at, out)) in corners.iter().enumerate() {
        if k > 0 {
            steps.push(Step::Line(into.0, into.1));
        }
        steps.push(Step::Curve { to: out, ctrl: at });
    }
    let start = corners.first().map_or((0.0, 0.0), |c| c.0);
    steps.push(Step::Line(start.0, start.1));
    Outline { start, steps }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};
    use alacritty_terminal::{
        index::Side,
        selection::{Selection, SelectionType},
    };

    const H: f32 = 20.0;

    /// `text` on a 4×12 screen, `from` to `to` (row, column, inclusive) selected as `kind`.
    fn selected(
        text: &str,
        kind: SelectionType,
        from: (i32, usize),
        to: (i32, usize),
    ) -> Vec<(i32, usize, usize)> {
        let mut screen = Screen::new(Size { rows: 4, cols: 12 });
        screen.process(text.as_bytes());
        let term = &mut screen.term;
        let mut selection =
            Selection::new(kind, Point::new(Line(from.0), Column(from.1)), Side::Left);
        selection.update(Point::new(Line(to.0), Column(to.1)), Side::Right);
        term.selection = Some(selection);
        let range = term.selection.as_ref().unwrap().to_range(term).unwrap();
        spans(term, |p| range.contains(p), 0, 4, 12)
    }

    #[test]
    fn the_cells_selected_on_each_row() {
        let text = "hello world\r\nsecond line\r\nthird";
        // On one row.
        assert_eq!(
            selected(text, SelectionType::Simple, (0, 2), (0, 6)),
            [(0, 2, 7)]
        );
        // From the middle of a row to the middle of another: the rows between whole.
        assert_eq!(
            selected(text, SelectionType::Simple, (0, 6), (2, 2)),
            [(0, 6, 12), (1, 0, 12), (2, 0, 3)]
        );
        // Whole rows.
        assert_eq!(
            selected(text, SelectionType::Lines, (0, 3), (1, 3)),
            [(0, 0, 12), (1, 0, 12)]
        );
        // A block.
        assert_eq!(
            selected(text, SelectionType::Block, (0, 2), (2, 4)),
            [(0, 2, 5), (1, 2, 5), (2, 2, 5)]
        );
    }

    #[test]
    fn a_wide_character_is_selected_whole() {
        // 中 takes columns 1 and 2; from its first half, or to it, both are covered.
        assert_eq!(
            selected("a中b", SelectionType::Simple, (0, 1), (0, 1)),
            [(0, 1, 3)]
        );
        assert_eq!(
            selected("a中b", SelectionType::Simple, (0, 0), (0, 1)),
            [(0, 0, 3)]
        );
    }

    #[test]
    fn rows_scrolled_back_and_the_row_above_the_first() {
        let mut screen = Screen::new(Size { rows: 2, cols: 8 });
        screen.process(b"one\r\ntwo\r\nthree\r\nfour");
        let term = &mut screen.term;
        term.scroll_display(alacritty_terminal::grid::Scroll::Delta(1));
        // "two" is the first row shown now, "one" the row above it.
        let mut selection = Selection::new(
            SelectionType::Simple,
            Point::new(Line(-2), Column(0)),
            Side::Left,
        );
        selection.update(Point::new(Line(-1), Column(1)), Side::Right);
        term.selection = Some(selection);
        let range = term.selection.as_ref().unwrap().to_range(term).unwrap();
        assert_eq!(
            spans(term, |p| range.contains(p), -1, 2, 8),
            [(-1, 0, 8), (0, 0, 2)]
        );
        assert_eq!(spans(term, |p| range.contains(p), 0, 2, 8), [(0, 0, 2)]);
    }

    #[test]
    fn the_colour_looks_the_same_on_the_plain_background() {
        let (selection, background) = ((0x50, 0x49, 0x45), (0x1d, 0x1a, 0x16));
        let soft = lifted(selection, background, OPACITY);
        let over = |c: u8, b: u8| f32::from(b) + (f32::from(c) - f32::from(b)) * OPACITY;
        assert!((over(soft.0, background.0) - f32::from(selection.0)).abs() <= 1.0);
        assert!((over(soft.1, background.1) - f32::from(selection.1)).abs() <= 1.0);
        assert!((over(soft.2, background.2) - f32::from(selection.2)).abs() <= 1.0);
        // As far as it goes: never past white.
        assert_eq!(lifted((250, 250, 250), (0, 0, 0), 0.5), (255, 255, 255));
    }

    #[test]
    fn the_find_bar_s_match_is_yellow_a_selection_its_own_colour() {
        let theme = crate::palette::Theme {
            background: (0x1d, 0x1a, 0x16),
            selection: (0x50, 0x49, 0x45),
            ..crate::palette::Theme::XTERM
        };
        assert_eq!(fill(&theme, true), (theme.ansi[11], FOUND_OPACITY));
        assert_eq!(
            fill(&theme, false),
            (lifted(theme.selection, theme.background, OPACITY), OPACITY)
        );
    }

    #[test]
    fn one_row_is_a_box() {
        assert_eq!(
            polygons(&[(3, 10.0, 50.0)], H),
            [vec![(10.0, 60.0), (50.0, 60.0), (50.0, 80.0), (10.0, 80.0)]]
        );
    }

    #[test]
    fn rows_that_follow_on_and_overlap_are_one_shape() {
        // From the middle of a row, whole rows, to the middle of another.
        let rows = [(0, 30.0, 100.0), (1, 0.0, 100.0), (2, 0.0, 50.0)];
        assert_eq!(
            polygons(&rows, H),
            [vec![
                (30.0, 0.0),
                (100.0, 0.0),
                (100.0, 40.0),
                (50.0, 40.0),
                (50.0, 60.0),
                (0.0, 60.0),
                (0.0, 20.0),
                (30.0, 20.0),
            ]]
        );
    }

    #[test]
    fn rows_apart_or_side_by_side_are_shapes_of_their_own() {
        // The second row ends before the first begins.
        let shapes = polygons(&[(0, 60.0, 100.0), (1, 0.0, 40.0)], H);
        assert_eq!(shapes.len(), 2);
        // A row skipped.
        let shapes = polygons(&[(0, 0.0, 100.0), (2, 0.0, 100.0)], H);
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn every_corner_is_rounded_at_most_half_its_shorter_side() {
        let outline = rounded(&[(10.0, 0.0), (50.0, 0.0), (50.0, 20.0), (10.0, 20.0)], 4.0);
        // From just under the top left corner, round it, along the top, round the next.
        assert_eq!(outline.start, (10.0, 4.0));
        assert_eq!(
            outline.steps[..3],
            [
                Step::Curve {
                    to: (14.0, 0.0),
                    ctrl: (10.0, 0.0)
                },
                Step::Line(46.0, 0.0),
                Step::Curve {
                    to: (50.0, 4.0),
                    ctrl: (50.0, 0.0)
                },
            ]
        );
        assert_eq!(outline.steps.len(), 8);
        // A step in only 2 px: its corners have 1.
        let outline = rounded(
            &[
                (0.0, 0.0),
                (40.0, 0.0),
                (40.0, 20.0),
                (42.0, 20.0),
                (42.0, 40.0),
                (0.0, 40.0),
            ],
            4.0,
        );
        assert_eq!(
            outline.steps[4],
            Step::Curve {
                to: (41.0, 20.0),
                ctrl: (40.0, 20.0)
            }
        );
        assert_eq!(outline.steps[5], Step::Line(41.0, 20.0));
    }

    #[test]
    fn outlines_join_and_round() {
        let shapes = outlines(&[(0, 30.0, 100.0), (1, 0.0, 100.0)], H, RADIUS);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].steps.len(), 2 * 6);
    }

    #[test]
    fn a_match_across_a_wrapped_line_and_the_others_fainter() {
        let mut screen = Screen::new(Size { rows: 4, cols: 12 });
        screen.process(b"0123456789abcdef");
        let found = Point::new(Line(0), Column(9))..=Point::new(Line(1), Column(1));
        assert_eq!(
            spans(&screen.term, |p| found.contains(&p), 0, 4, 12),
            [(0, 9, 12), (1, 0, 2)]
        );
        let theme = crate::palette::Theme::XTERM;
        assert_eq!(others(&theme), (theme.ansi[11], OTHERS_OPACITY));
        const { assert!(OTHERS_OPACITY < FOUND_OPACITY) };
    }
}
