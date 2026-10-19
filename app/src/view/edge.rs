//! A selection dragged past the top or bottom of a pane goes on through the history (P5-143d2):
//! above the grid toward older output, below it toward newer, faster the farther out, up to a
//! cap; the mouse held still, it keeps going, frame by frame. The rules live here, on plain
//! numbers: the pane (`view.rs`) hands the whole lines to the grid, takes the selection's end to
//! the row under the mouse and asks for the next frame while it goes on.
use super::glide::Glide;
use std::time::{Duration, Instant};

/// Lines a second just past the edge.
const SLOWEST: f32 = 8.0;
/// Lines a second more for each line farther out.
const PER_LINE: f32 = 16.0;
/// Lines a second at most.
const FASTEST: f32 = 120.0;
/// A frame's time at most: after a pause (the window hidden), no leap.
const LONGEST_FRAME: Duration = Duration::from_millis(50);

/// Whether a drag goes on past the edge: a selection, `selecting`, not in a full-screen program,
/// which has no history, the mouse above or below a grid `height` tall, at `y` from its top.
pub fn wanted(selecting: bool, full_screen: bool, y: f32, height: f32, line: f32) -> bool {
    selecting && !full_screen && speed(y, height, line) != 0.0
}

/// How fast the view goes, in lines a second toward older output (negative: newer), with the
/// mouse at `y` from the top of a grid `height` tall: none over the grid.
pub fn speed(y: f32, height: f32, line: f32) -> f32 {
    let out = if y < 0.0 {
        -y
    } else if y >= height {
        height - y
    } else {
        return 0.0;
    };
    let lines = (SLOWEST + PER_LINE * out.abs() / line).min(FASTEST);
    if out > 0.0 { lines } else { -lines }
}

/// A selection being dragged past an edge.
#[derive(Debug)]
pub struct Edge {
    /// The mouse from the top of the grid, and the grid's height.
    y: f32,
    height: f32,
    last: Instant,
    /// Pixels not yet a whole line, by the line (Settings' `smooth_scroll` off).
    carry: f32,
}

impl Edge {
    pub fn new(y: f32, height: f32, now: Instant) -> Self {
        Self {
            y,
            height,
            last: now,
            carry: 0.0,
        }
    }

    /// The mouse moved, to `y` from the top of a grid `height` tall.
    pub fn aim(&mut self, y: f32, height: f32) {
        (self.y, self.height) = (y, height);
    }

    /// The frame at `now`, the view `lines` whole lines back, `history` kept, each `line` tall:
    /// the whole lines it is at, and whether it goes on. By the pixel through `glide`, or
    /// `smooth` off, by the line; it stops at the oldest line, at the bottom, and over the grid.
    pub fn frame(
        &mut self,
        glide: &mut Glide,
        smooth: bool,
        lines: usize,
        history: usize,
        line: f32,
        now: Instant,
    ) -> (usize, bool) {
        let time = now.saturating_duration_since(self.last).min(LONGEST_FRAME);
        self.last = now;
        let speed = speed(self.y, self.height, line);
        if speed == 0.0 {
            return (lines, false);
        }
        let by = speed * line * time.as_secs_f32();
        let (to, at_end) = if smooth {
            let to = glide.drag(lines, by, history, line);
            (
                to,
                if by > 0.0 {
                    to >= history
                } else {
                    to == 0 && glide.pixels() == 0.0
                },
            )
        } else {
            self.carry += by;
            let whole = (self.carry / line).trunc();
            self.carry -= whole * line;
            let to = (lines as i64 + whole as i64).clamp(0, history as i64) as usize;
            (to, if by > 0.0 { to >= history } else { to == 0 })
        };
        (to, !at_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: f32 = 20.0;
    const HEIGHT: f32 = 400.0;
    const HISTORY: usize = 1000;

    #[test]
    fn above_goes_back_below_comes_down_and_over_the_grid_stays() {
        assert!(speed(-1.0, HEIGHT, LINE) > 0.0);
        assert!(speed(HEIGHT + 1.0, HEIGHT, LINE) < 0.0);
        for y in [0.0, 1.0, HEIGHT / 2.0, HEIGHT - 0.5] {
            assert_eq!(speed(y, HEIGHT, LINE), 0.0, "at {y}");
        }
        // The same either way.
        assert_eq!(
            speed(-30.0, HEIGHT, LINE),
            -speed(HEIGHT + 30.0, HEIGHT, LINE)
        );
    }

    #[test]
    fn only_a_selection_past_the_edge_outside_a_full_screen_program_goes_on() {
        assert!(wanted(true, false, -1.0, HEIGHT, LINE));
        assert!(wanted(true, false, HEIGHT + 1.0, HEIGHT, LINE));
        assert!(!wanted(true, false, 10.0, HEIGHT, LINE));
        assert!(!wanted(false, false, -1.0, HEIGHT, LINE));
        assert!(!wanted(true, true, -1.0, HEIGHT, LINE));
    }

    #[test]
    fn farther_out_is_faster_up_to_a_cap() {
        let near = speed(-1.0, HEIGHT, LINE);
        let far = speed(-3.0 * LINE, HEIGHT, LINE);
        assert!(far > near, "{far} not faster than {near}");
        assert_eq!(speed(-1000.0, HEIGHT, LINE), FASTEST);
        assert_eq!(speed(HEIGHT + 1000.0, HEIGHT, LINE), -FASTEST);
        assert!(near >= SLOWEST);
    }

    /// Frames 10 ms apart from `start`, `n` of them: the lines the view is at after each, and
    /// whether it went on.
    fn run(
        edge: &mut Edge,
        glide: &mut Glide,
        smooth: bool,
        mut lines: usize,
        start: Instant,
        n: u32,
    ) -> (usize, bool) {
        let mut going = true;
        for i in 1..=n {
            let now = start + Duration::from_millis(10 * u64::from(i));
            (lines, going) = edge.frame(glide, smooth, lines, HISTORY, LINE, now);
            if !going {
                break;
            }
        }
        (lines, going)
    }

    #[test]
    fn held_still_above_it_keeps_going_back_by_the_pixel() {
        let start = Instant::now();
        let mut edge = Edge::new(-2.0 * LINE, HEIGHT, start);
        let mut glide = Glide::default();
        // A second of frames: as far as the speed says, less a frame's rounding.
        let (lines, going) = run(&mut edge, &mut glide, true, 0, start, 100);
        assert!(going);
        let want = speed(-2.0 * LINE, HEIGHT, LINE);
        let gone = lines as f32 + glide.pixels() / LINE;
        assert!((gone - want).abs() < 0.5, "went {gone} lines, not {want}");
        // Below, back down.
        edge.aim(HEIGHT + LINE, HEIGHT);
        let (back, _) = run(
            &mut edge,
            &mut glide,
            true,
            lines,
            start + Duration::from_secs(1),
            10,
        );
        assert!(back < lines);
    }

    #[test]
    fn by_the_line_without_smooth_scrolling() {
        let start = Instant::now();
        let mut edge = Edge::new(-2.0 * LINE, HEIGHT, start);
        let mut glide = Glide::default();
        let (lines, going) = run(&mut edge, &mut glide, false, 0, start, 100);
        assert!(going);
        assert!(lines > 0);
        assert_eq!(glide.pixels(), 0.0);
    }

    #[test]
    fn it_stops_at_the_oldest_line_and_at_the_bottom() {
        let start = Instant::now();
        for smooth in [true, false] {
            let mut glide = Glide::default();
            let mut edge = Edge::new(-1000.0, HEIGHT, start);
            let (lines, going) = run(&mut edge, &mut glide, smooth, HISTORY - 2, start, 100);
            assert_eq!((lines, going), (HISTORY, false), "smooth {smooth}");
            let mut edge = Edge::new(HEIGHT + 1000.0, HEIGHT, start);
            let (lines, going) = run(&mut edge, &mut glide, smooth, 2, start, 100);
            assert_eq!((lines, going), (0, false), "smooth {smooth}");
            assert_eq!(glide.pixels(), 0.0);
        }
    }

    #[test]
    fn back_over_the_grid_it_stops() {
        let start = Instant::now();
        let mut edge = Edge::new(-LINE, HEIGHT, start);
        let mut glide = Glide::default();
        let (lines, going) = run(&mut edge, &mut glide, true, 0, start, 5);
        assert!(going && lines + (glide.pixels() > 0.0) as usize > 0);
        edge.aim(HEIGHT / 2.0, HEIGHT);
        let now = start + Duration::from_millis(60);
        let at = edge.frame(&mut glide, true, lines, HISTORY, LINE, now);
        assert_eq!(at, (lines, false));
    }

    #[test]
    fn a_frame_after_a_long_pause_takes_no_leap() {
        let start = Instant::now();
        let mut edge = Edge::new(-1000.0, HEIGHT, start);
        let mut glide = Glide::default();
        let (lines, _) = edge.frame(
            &mut glide,
            false,
            0,
            HISTORY,
            LINE,
            start + Duration::from_secs(5),
        );
        let most = FASTEST * LONGEST_FRAME.as_secs_f32();
        assert!(lines as f32 <= most.ceil(), "leapt {lines} lines");
    }
}
