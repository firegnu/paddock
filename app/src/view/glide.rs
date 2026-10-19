//! Scrolling back through a terminal's history by the pixel (P5-143c2): the trackpad moves the
//! view as far as the fingers go, its momentum as macOS sends it; a wheel's notch slides there.
//! Alacritty keeps the view in whole lines (`display_offset`); this keeps the pixels past them,
//! under a line, and the slide. The rules live here, on plain numbers: the pane (`view.rs`) hands
//! the whole lines to the grid, draws the pixels as a shift and asks for the next frame while a
//! slide goes on.
use alacritty_terminal::term::TermMode;
use std::time::{Duration, Instant};

/// How long a wheel's notch takes to slide into place.
pub const SLIDE: Duration = Duration::from_millis(150);

/// Pixels this close to a whole line are that line.
const HAIR: f32 = 0.01;

/// Where the wheel goes, by what the program asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wheel {
    /// To the program, which asked for the mouse: by the line, as before.
    Program,
    /// Arrow keys, for a full-screen program: by the line, as before.
    Arrows,
    /// The view through the history.
    History,
}

/// Where the wheel goes: ⇧ keeps it from a program that asked for the mouse, and a paused agent
/// takes it neither way.
pub fn wheel_goes(mode: TermMode, shift: bool, paused: bool) -> Wheel {
    if paused {
        Wheel::History
    } else if mode.intersects(TermMode::MOUSE_MODE) && !shift {
        Wheel::Program
    } else if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
        Wheel::Arrows
    } else {
        Wheel::History
    }
}

/// How far a slide has gone at `t` of the way through its time: fast at first, settling in.
fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// The view's pixels past its whole lines, and a wheel's slide.
#[derive(Debug, Default)]
pub struct Glide {
    /// Toward older output, under a line.
    pixels: f32,
    slide: Option<Slide>,
}

#[derive(Clone, Copy, Debug)]
struct Slide {
    /// How far it goes, toward older output.
    distance: f32,
    /// How far it went.
    done: f32,
    start: Instant,
}

impl Glide {
    /// Pixels past the whole lines, toward older output.
    pub fn pixels(&self) -> f32 {
        self.pixels
    }

    /// How far down the rows are drawn, on a display of `scale` device pixels to a pixel: on whole
    /// device pixels, so that rows meet on an edge between them.
    pub fn shift(&self, scale: f32) -> f32 {
        (self.pixels * scale).round() / scale
    }

    /// The fingers moved the view `by` pixels toward older output (negative: newer), from `lines`
    /// whole lines back, `history` lines kept, each `line` tall: it goes there at once, and a
    /// slide going on stops. The whole lines it is at now.
    pub fn drag(&mut self, lines: usize, by: f32, history: usize, line: f32) -> usize {
        self.slide = None;
        self.go(lines, by, history, line).0
    }

    /// A wheel turned `by` pixels: the view slides there over [`SLIDE`], on from where a slide
    /// still going is headed; a turn the other way leaves the rest of the one before.
    pub fn wheel(&mut self, by: f32, now: Instant) {
        let left = self.slide.map_or(0.0, |slide| slide.distance - slide.done);
        let left = if left * by < 0.0 { 0.0 } else { left };
        self.slide = Some(Slide {
            distance: left + by,
            done: 0.0,
            start: now,
        });
    }

    /// The frame drawn at `now`, the view `lines` whole lines back: the whole lines it is at, a
    /// slide having gone on, and whether the next frame is wanted for it. A slide stops at the
    /// oldest line and at the bottom; no pixels are left past the oldest line.
    pub fn frame(
        &mut self,
        lines: usize,
        history: usize,
        line: f32,
        now: Instant,
    ) -> (usize, bool) {
        if lines >= history {
            self.pixels = 0.0;
        }
        let Some(mut slide) = self.slide.take() else {
            return (lines, false);
        };
        let t = now.saturating_duration_since(slide.start).as_secs_f32() / SLIDE.as_secs_f32();
        let gone = slide.distance * ease(t);
        let (lines, stopped) = self.go(lines, gone - slide.done, history, line);
        slide.done = gone;
        if t < 1.0 && !stopped {
            self.slide = Some(slide);
        }
        (lines, self.slide.is_some())
    }

    /// `by` pixels on from `lines` whole lines back: the whole lines the view is at, and whether
    /// it stopped at either end short of going all the way.
    fn go(&mut self, lines: usize, by: f32, history: usize, line: f32) -> (usize, bool) {
        let total = self.pixels + by;
        let whole = (total / line).floor();
        let mut pixels = total - whole * line;
        let mut to = lines as i64 + whole as i64;
        if line - pixels < HAIR {
            (to, pixels) = (to + 1, 0.0);
        } else if pixels < HAIR {
            pixels = 0.0;
        }
        let top = history as i64;
        let (to, pixels, stopped) = if to < 0 {
            (0, 0.0, true)
        } else if to > top || (to == top && pixels > 0.0) {
            (top, 0.0, true)
        } else {
            (to, pixels, false)
        };
        self.pixels = pixels;
        (to as usize, stopped)
    }

    /// The view was put on whole lines (the bottom, a match): the pixels and a slide go.
    pub fn stop(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: f32 = 20.0;
    const HISTORY: usize = 100;

    fn at(glide: &Glide, lines: usize) -> (usize, f32) {
        (lines, glide.pixels())
    }

    #[test]
    fn the_fingers_move_the_view_by_the_pixel_across_lines_and_back() {
        let mut glide = Glide::default();
        let lines = glide.drag(0, 5.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (0, 5.0));
        let lines = glide.drag(lines, 20.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (1, 5.0));
        let lines = glide.drag(lines, 47.5, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (3, 12.5));
        // The other way, across a line.
        let lines = glide.drag(lines, -15.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (2, 17.5));
        let lines = glide.drag(lines, -17.5, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (2, 0.0));
    }

    #[test]
    fn the_view_stops_whole_at_the_oldest_line_and_at_the_bottom() {
        let mut glide = Glide::default();
        let lines = glide.drag(HISTORY - 1, 30.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (HISTORY, 0.0));
        let lines = glide.drag(lines, 7.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (HISTORY, 0.0));
        // Down past the bottom: no half line left.
        let lines = glide.drag(0, 5.0, HISTORY, LINE);
        let lines = glide.drag(lines, -12.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (0, 0.0));
        let lines = glide.drag(lines, -3.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (0, 0.0));
        // Without a history, nowhere to go.
        let lines = glide.drag(0, 9.0, 0, LINE);
        assert_eq!(at(&glide, lines), (0, 0.0));
        // A hair short of a line is that line.
        let lines = glide.drag(0, LINE - HAIR / 2.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (1, 0.0));
    }

    #[test]
    fn a_wheel_notch_slides_into_place_easing_out() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut glide = Glide::default();
        glide.wheel(3.0 * LINE, start);
        assert_eq!(glide.frame(0, HISTORY, LINE, start), (0, true));
        assert_eq!(glide.pixels(), 0.0);
        // Halfway through the time, most of the way there.
        let (lines, more) = glide.frame(0, HISTORY, LINE, ms(75));
        assert!(more);
        let gone = lines as f32 * LINE + glide.pixels();
        assert!((gone - 0.875 * 3.0 * LINE).abs() < 0.01, "{gone}");
        // There at the end, on a whole line, and still.
        let (lines, more) = glide.frame(lines, HISTORY, LINE, ms(150));
        assert_eq!((at(&glide, lines), more), ((3, 0.0), false));
        assert_eq!(glide.frame(lines, HISTORY, LINE, ms(200)), (3, false));
    }

    #[test]
    fn notches_one_after_another_slide_on_and_a_turn_back_leaves_the_rest() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut glide = Glide::default();
        glide.wheel(3.0 * LINE, start);
        let (lines, _) = glide.frame(0, HISTORY, LINE, ms(75));
        glide.wheel(3.0 * LINE, ms(75));
        let (lines, more) = glide.frame(lines, HISTORY, LINE, ms(100));
        assert!(more);
        let (lines, more) = glide.frame(lines, HISTORY, LINE, ms(225));
        assert_eq!((at(&glide, lines), more), ((6, 0.0), false));
        // Up a notch, and down one before it is there: down from where it is.
        glide.wheel(3.0 * LINE, ms(300));
        let (lines, _) = glide.frame(lines, HISTORY, LINE, ms(375));
        let midway = lines as f32 * LINE + glide.pixels();
        glide.wheel(-3.0 * LINE, ms(375));
        let (lines, more) = glide.frame(lines, HISTORY, LINE, ms(525));
        assert!(!more);
        let gone = lines as f32 * LINE + glide.pixels();
        assert!(
            (gone - (midway - 3.0 * LINE)).abs() < 0.01,
            "{gone} {midway}"
        );
    }

    #[test]
    fn a_slide_stops_at_either_end_and_the_fingers_stop_a_slide() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut glide = Glide::default();
        glide.wheel(10.0 * LINE, start);
        let (lines, more) = glide.frame(1, 2, LINE, ms(100));
        assert_eq!((at(&glide, lines), more), ((2, 0.0), false));
        glide.wheel(-10.0 * LINE, ms(200));
        let (lines, more) = glide.frame(lines, 2, LINE, ms(300));
        assert_eq!((at(&glide, lines), more), ((0, 0.0), false));
        // The fingers take over from a slide going on.
        glide.wheel(3.0 * LINE, ms(400));
        let (lines, _) = glide.frame(lines, HISTORY, LINE, ms(450));
        let lines = glide.drag(lines, 0.0, HISTORY, LINE);
        let held = at(&glide, lines);
        assert_eq!(glide.frame(lines, HISTORY, LINE, ms(500)), (held.0, false));
        assert_eq!(glide.pixels(), held.1);
    }

    #[test]
    fn no_pixels_are_left_past_the_oldest_line_once_the_history_shrinks() {
        let mut glide = Glide::default();
        let lines = glide.drag(3, 5.0, HISTORY, LINE);
        assert_eq!(at(&glide, lines), (3, 5.0));
        // Cleared: alacritty put the view on what is left.
        assert_eq!(glide.frame(0, 0, LINE, Instant::now()), (0, false));
        assert_eq!(glide.pixels(), 0.0);
    }

    #[test]
    fn the_rows_are_drawn_down_on_whole_device_pixels() {
        let mut glide = Glide::default();
        glide.drag(0, 5.3, HISTORY, LINE);
        assert_eq!(glide.shift(2.0), 5.5);
        assert_eq!(glide.shift(1.0), 5.0);
        glide.drag(0, 0.1, HISTORY, LINE);
        assert_eq!(glide.shift(2.0), 5.5);
        glide.stop();
        assert_eq!(glide.shift(2.0), 0.0);
    }

    #[test]
    fn the_wheel_goes_to_a_program_by_the_line_as_before_else_through_the_history() {
        let full = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
        let mouse = TermMode::MOUSE_REPORT_CLICK;
        assert_eq!(wheel_goes(mouse, false, false), Wheel::Program);
        assert_eq!(wheel_goes(mouse | full, false, false), Wheel::Program);
        assert_eq!(wheel_goes(full, false, false), Wheel::Arrows);
        assert_eq!(wheel_goes(full, true, false), Wheel::Arrows);
        // ⇧ over a program that asked for the mouse scrolls the history.
        assert_eq!(wheel_goes(mouse, true, false), Wheel::History);
        assert_eq!(
            wheel_goes(TermMode::ALT_SCREEN, false, false),
            Wheel::History
        );
        assert_eq!(wheel_goes(TermMode::empty(), false, false), Wheel::History);
        // A paused agent's view scrolls its history whatever its program asked for.
        assert_eq!(wheel_goes(mouse | full, false, true), Wheel::History);
    }
}
