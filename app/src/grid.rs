//! Pixel geometry of the character grid.
use crate::terminal::Size;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub cell_width: f32,
    pub line_height: f32,
}

/// Whole cells that fit; never less than the 1×2 the PTY layer accepts.
pub fn grid_size(width: f32, height: f32, metrics: Metrics) -> Size {
    let count = |length: f32, cell: f32, min: u16| {
        ((length / cell).floor().clamp(0.0, f32::from(u16::MAX)) as u16).max(min)
    };
    Size {
        rows: count(height, metrics.line_height, 1),
        cols: count(width, metrics.cell_width, 2),
    }
}

/// The cell under a point relative to the grid origin, clamped into the grid, and whether the
/// point is in the cell's right half.
pub fn cell_at(x: f32, y: f32, metrics: Metrics, size: Size) -> (u16, u16, bool) {
    let column = (x / metrics.cell_width).max(0.0);
    let col = (column.floor() as u16).min(size.cols.saturating_sub(1));
    let row = ((y / metrics.line_height).max(0.0).floor() as u16).min(size.rows.saturating_sub(1));
    let right = column - f32::from(col) >= 0.5;
    (col, row, right)
}

/// Turns pixel or line wheel deltas into whole lines, keeping the remainder.
/// Positive means toward older output.
#[derive(Debug, Default)]
pub struct Scroll {
    pending: f32,
}
impl Scroll {
    pub fn lines(&mut self, delta: f32) -> i32 {
        // Reversing direction drops the unused remainder of the old direction.
        if self.pending * delta < 0.0 {
            self.pending = 0.0;
        }
        self.pending += delta;
        let whole = self.pending.trunc();
        self.pending -= whole;
        whole as i32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const M: Metrics = Metrics {
        cell_width: 8.0,
        line_height: 16.0,
    };

    #[test]
    fn size_counts_whole_cells() {
        let size = grid_size(800.0, 600.0, M);
        assert_eq!((size.cols, size.rows), (100, 37));
    }

    #[test]
    fn size_never_collapses() {
        let size = grid_size(3.0, 2.0, M);
        assert_eq!((size.cols, size.rows), (2, 1));
    }

    #[test]
    fn points_map_to_clamped_cells() {
        let size = Size { rows: 10, cols: 20 };
        assert_eq!(cell_at(0.0, 0.0, M, size), (0, 0, false));
        assert_eq!(cell_at(13.0, 17.0, M, size), (1, 1, true));
        assert_eq!(cell_at(-5.0, 1000.0, M, size), (0, 9, false));
        assert_eq!(cell_at(9999.0, 5.0, M, size), (19, 0, true));
    }

    #[test]
    fn small_deltas_accumulate_into_lines() {
        let mut scroll = Scroll::default();
        assert_eq!(scroll.lines(0.4), 0);
        assert_eq!(scroll.lines(0.4), 0);
        assert_eq!(scroll.lines(0.4), 1);
        assert_eq!(scroll.lines(-2.5), -2);
        assert_eq!(scroll.lines(3.0), 3);
    }
}
