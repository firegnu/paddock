//! A terminal pane's left margin: a shell's commands put their bars in it (P5-149b), so it is
//! wider than the room the grid keeps from the other edges. Apart from GPUI, so that this can be
//! tested; the pane lays it out and draws the bars.
use crate::fonts::UiFont;

/// The left margin at the interface's base size: a shell's, as the design has it.
pub const SHELL: f32 = 18.0;
/// The margin every other pane keeps, as from the top, right and bottom.
pub const OTHER: f32 = 6.0;

/// The grid's room from the pane's left edge: a shell's is [`SHELL`] at the base interface size,
/// scaling with it (P5-149b); other panes keep [`OTHER`].
pub fn left(shell: bool, ui: &UiFont) -> f32 {
    if shell { ui.scale(SHELL) } else { OTHER }
}

/// Whether a point `x` across the pane is on a command's bar: anywhere in the left margin, up to
/// the grid's left edge (`grid`); the grid's first column is text.
pub fn on_bar(x: f32, grid: f32) -> bool {
    x < grid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::BASE_SIZE;

    #[test]
    fn a_shells_left_margin_is_18_and_an_agents_stays_6() {
        let base = UiFont::default();
        assert_eq!(left(true, &base), 18.0);
        assert_eq!(left(false, &base), 6.0);
        // A shell's scales with the interface; an agent's does not.
        let large = UiFont {
            family: None,
            size: BASE_SIZE * 1.5,
        };
        assert_eq!(left(true, &large), 27.0);
        assert_eq!(left(false, &large), 6.0);
    }

    #[test]
    fn the_whole_left_margin_is_the_bar_and_the_first_column_is_text() {
        let margin = left(true, &UiFont::default());
        for x in [0.0, 0.5, 6.0, 9.0, 17.9] {
            assert!(on_bar(x, margin), "{x} is in the margin");
        }
        assert!(!on_bar(margin, margin), "the grid's first column");
        assert!(!on_bar(margin + 1.0, margin), "text past it");
    }
}
