//! The grid kept as it is while the window moves the pane's edges (P5-148a): a pane growing to
//! fill its tab, or squeezed by a sliding sidebar, keeps its rows and columns, the program its
//! size, until the window says the move is over; then the grid fits the pane once. The rows that
//! no longer fit meanwhile are cut by the pane's edge, and room left over shows the ground.
use crate::terminal::Size;

/// The grid's size this frame: the one it has while `held` (once it has one, `laid_out`), else
/// the one that `fits` the pane.
pub fn grid(held: bool, laid_out: bool, current: Size, fits: Size) -> Size {
    if held && laid_out { current } else { fits }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(cols: u16, rows: u16) -> Size {
        Size { rows, cols }
    }

    fn cr(size: Size) -> (u16, u16) {
        (size.cols, size.rows)
    }

    #[test]
    fn held_through_a_move_the_grid_is_resized_once_when_let_go() {
        // A pane growing from 80×24 to 160×48 over a few frames, held, then let go.
        let frames = [
            (true, size(80, 24)),
            (true, size(100, 30)),
            (true, size(130, 40)),
            (true, size(160, 48)),
            (false, size(160, 48)),
            (false, size(160, 48)),
        ];
        let mut current = size(80, 24);
        let mut resized = Vec::new();
        for (held, fits) in frames {
            let next = grid(held, true, current, fits);
            if cr(next) != cr(current) {
                resized.push(cr(next));
                current = next;
            }
        }
        assert_eq!(resized, [(160, 48)]);
    }

    #[test]
    fn a_pane_not_yet_laid_out_takes_its_size_even_held() {
        assert_eq!(
            cr(grid(true, false, size(80, 24), size(120, 40))),
            (120, 40)
        );
        assert_eq!(
            cr(grid(false, true, size(80, 24), size(120, 40))),
            (120, 40)
        );
    }
}
