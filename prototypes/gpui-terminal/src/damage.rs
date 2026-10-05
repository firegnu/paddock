//! Whether the screen changed since the last frame. Saddle's PTY session processes output on its
//! own thread and offers no change notification, so the window polls alacritty's damage, which
//! always includes the cursor cell; a bare cursor that did not move is not a change.
use alacritty_terminal::{
    event::EventListener,
    term::{Term, TermDamage},
};

pub fn take_changed<T: EventListener>(term: &mut Term<T>) -> bool {
    let cursor = term.grid().cursor.point;
    let (line, column) = (cursor.line.0.max(0) as usize, cursor.column.0);
    let changed = match term.damage() {
        TermDamage::Full => true,
        TermDamage::Partial(mut lines) => {
            lines.any(|d| !(d.line == line && d.left == column && d.right == column))
        }
    };
    term.reset_damage();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use saddle::terminal::{Screen, Size};

    #[test]
    fn output_and_cursor_moves_are_changes_and_are_consumed() {
        let mut screen = Screen::new(Size { rows: 5, cols: 20 });
        assert!(
            take_changed(&mut screen.term),
            "a new screen starts damaged"
        );
        assert!(!take_changed(&mut screen.term));
        screen.process(b"hi");
        assert!(take_changed(&mut screen.term));
        assert!(!take_changed(&mut screen.term));
        screen.process(b"\x1b[4;6H");
        assert!(take_changed(&mut screen.term));
        assert!(!take_changed(&mut screen.term));
    }

    #[test]
    fn scrolling_the_view_is_a_change() {
        let mut screen = Screen::new(Size { rows: 3, cols: 10 });
        screen.process(b"1\r\n2\r\n3\r\n4\r\n5\r\n");
        let _ = take_changed(&mut screen.term);
        screen
            .term
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(1));
        assert!(take_changed(&mut screen.term));
    }
}
