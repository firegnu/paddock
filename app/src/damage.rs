//! Whether the screen changed since the last frame. The PTY session processes output on its own
//! thread and rings the pane (`pty::Bell`), which then reads alacritty's damage: it always includes
//! the cursor cell, and a bare cursor that did not move is not a change.
use alacritty_terminal::{
    event::EventListener,
    term::{Term, TermDamage},
};

/// How much changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    None,
    Rows,
    /// Cleared, scrolled, resized, switched to or from the other screen.
    Whole,
}

pub fn take<T: EventListener>(term: &mut Term<T>) -> Change {
    let cursor = term.grid().cursor.point;
    let (line, column) = (cursor.line.0.max(0) as usize, cursor.column.0);
    let change = match term.damage() {
        TermDamage::Full => Change::Whole,
        TermDamage::Partial(mut lines) => {
            if lines.any(|d| !(d.line == line && d.left == column && d.right == column)) {
                Change::Rows
            } else {
                Change::None
            }
        }
    };
    term.reset_damage();
    change
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};

    #[test]
    fn output_and_cursor_moves_are_changes_and_are_consumed() {
        let mut screen = Screen::new(Size { rows: 5, cols: 20 });
        assert_ne!(
            take(&mut screen.term),
            Change::None,
            "a new screen starts damaged"
        );
        assert_eq!(take(&mut screen.term), Change::None);
        screen.process(b"hi");
        assert_ne!(take(&mut screen.term), Change::None);
        assert_eq!(take(&mut screen.term), Change::None);
        screen.process(b"\x1b[4;6H");
        assert_ne!(take(&mut screen.term), Change::None);
        assert_eq!(take(&mut screen.term), Change::None);
    }

    #[test]
    fn scrolling_the_view_is_a_change() {
        let mut screen = Screen::new(Size { rows: 3, cols: 10 });
        screen.process(b"1\r\n2\r\n3\r\n4\r\n5\r\n");
        let _ = take(&mut screen.term);
        screen
            .term
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(1));
        assert_ne!(take(&mut screen.term), Change::None);
    }

    /// A cleared or scrolled screen is changed as a whole, typing on a row is not (P5-143c1).
    #[test]
    fn a_screen_redrawn_as_a_whole_is_told_apart_from_a_few_rows() {
        let mut screen = Screen::new(Size { rows: 5, cols: 20 });
        let _ = take(&mut screen.term);
        screen.process(b"ab");
        assert_eq!(take(&mut screen.term), Change::Rows);
        screen.process(b"\x1b[H\x1b[2J");
        assert_eq!(take(&mut screen.term), Change::Whole);
        screen.process(b"\r\n\r\n\r\n\r\n\r\n");
        assert_eq!(take(&mut screen.term), Change::Whole, "scrolled");
        assert_eq!(take(&mut screen.term), Change::None);
    }
}
