//! The terminal cursor in motion (P5-143c1): it blinks, fading out and in, and slides to where it
//! goes, its leading corners ahead of the trailing ones. The rules live here, on plain numbers:
//! the pane (`view.rs`) says where the cursor is each frame and under what conditions, draws what
//! comes back and asks for the next frame when told.
use std::time::{Duration, Instant};

/// Blinking, from the last time the cursor moved or was typed at: shown, fading out, hidden,
/// fading in, over and over.
const SHOWN: Duration = Duration::from_millis(500);
const FADE: Duration = Duration::from_millis(180);
const HIDDEN: Duration = Duration::from_millis(320);
/// How often a fade is drawn anew: less often than the display, to keep the pane light.
const FADE_STEP: Duration = Duration::from_millis(33);

/// A slide: the leading corners take [`LEAD`], the trailing ones [`TAIL`]; a step of a cell or
/// two along a row, as typing makes, takes [`SHORT`] for all four.
const LEAD: Duration = Duration::from_millis(70);
const TAIL: Duration = Duration::from_millis(150);
const SHORT: Duration = Duration::from_millis(45);
/// Further than this many rows the cursor jumps instead.
const FAR_ROWS: u16 = 4;
/// How many columns along a row are a step.
const STEP_COLS: u16 = 2;

/// A point in the pane, in pixels from its grid's top left.
pub type Corner = (f32, f32);

/// Where the cursor is drawn, in pixels from the grid's top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Top left, top right, bottom right, bottom left.
    fn corners(&self) -> [Corner; 4] {
        let (r, b) = (self.x + self.w, self.y + self.h);
        [(self.x, self.y), (r, self.y), (r, b), (self.x, b)]
    }
}

/// The cursor as the grid has it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Column and row.
    pub cell: (u16, u16),
    pub rect: Rect,
}

/// What the frame is drawn under.
#[derive(Clone, Copy, Debug, Default)]
pub struct Conditions {
    /// The pane has the keyboard and its window is in front: only then does the cursor move or
    /// blink.
    pub focused: bool,
    /// Settings let it blink and the program has not asked for a steady cursor.
    pub blink: bool,
    /// Settings let it slide and the system does not ask for less motion.
    pub slide: bool,
    /// The cursor goes straight to its place this frame: the screen was redrawn as a whole, the
    /// pane resized, or the input method composes.
    pub snap: bool,
}

/// When the pane is to be drawn again for the cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Next {
    /// Not for the cursor.
    Idle,
    /// The very next frame.
    Frame,
    /// Then.
    At(Instant),
}

/// How the cursor is drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Mid-slide, its four corners (as [`Rect::corners`]); else none, drawn in its place.
    pub corners: Option<[Corner; 4]>,
    /// How opaque: 1 shown, 0 hidden.
    pub alpha: f32,
    pub next: Next,
}

impl Look {
    const STILL: Look = Look {
        corners: None,
        alpha: 1.0,
        next: Next::Idle,
    };
}

/// One pane's cursor, from frame to frame.
#[derive(Debug, Default)]
pub struct Caret {
    /// Where it was drawn last.
    last: Option<Target>,
    slide: Option<Slide>,
    /// When the blinking started over: the cursor moved, was typed at or got the keyboard.
    since: Option<Instant>,
}

impl Caret {
    /// Typed at: shown solid again, blinking only once the typing stops.
    pub fn touch(&mut self, now: Instant) {
        self.since = Some(now);
    }

    /// How to draw `target` (none: the cursor is not drawn) at `now`.
    pub fn frame(&mut self, target: Option<Target>, conditions: Conditions, now: Instant) -> Look {
        let Some(target) = target else {
            *self = Self::default();
            return Look::STILL;
        };
        if !conditions.focused {
            *self = Self {
                last: Some(target),
                ..Self::default()
            };
            return Look::STILL;
        }
        if self.last != Some(target) {
            let from = match &self.slide {
                Some(slide) => Some(slide.at(now)),
                None => self.last.map(|last| last.rect.corners()),
            };
            self.slide = match (self.last, from) {
                (Some(last), Some(from))
                    if conditions.slide && !conditions.snap && near(last.cell, target.cell) =>
                {
                    let step = step(last.cell, target.cell);
                    Some(Slide::new(from, target.rect.corners(), step, now))
                }
                _ => None,
            };
            self.since = Some(now);
        }
        self.last = Some(target);
        if conditions.snap {
            self.slide = None;
        }
        let since = *self.since.get_or_insert(now);
        if let Some(slide) = &self.slide {
            if now.saturating_duration_since(slide.start) < slide.length() {
                return Look {
                    corners: Some(slide.at(now)),
                    alpha: 1.0,
                    next: Next::Frame,
                };
            }
            self.slide = None;
        }
        if !conditions.blink {
            return Look::STILL;
        }
        let (alpha, wait) = blink(now.saturating_duration_since(since));
        Look {
            corners: None,
            alpha,
            next: Next::At(now + wait),
        }
    }
}

/// The cursor on its way from one place to another.
#[derive(Clone, Copy, Debug)]
struct Slide {
    from: [Corner; 4],
    to: [Corner; 4],
    start: Instant,
    /// How long each corner takes.
    takes: [Duration; 4],
}

/// How opaque a blinking cursor is `elapsed` after it started over, and how long until it is to
/// be drawn again.
fn blink(elapsed: Duration) -> (f32, Duration) {
    let out = SHOWN + FADE;
    let back = out + HIDDEN;
    let period = back + FADE;
    let t = Duration::from_nanos((elapsed.as_nanos() % period.as_nanos()) as u64);
    let fading = |from: Duration| smooth((t - from).as_secs_f32() / FADE.as_secs_f32());
    if t < SHOWN {
        (1.0, SHOWN - t)
    } else if t < out {
        (1.0 - fading(SHOWN), FADE_STEP.min(out - t))
    } else if t < back {
        (0.0, back - t)
    } else {
        (fading(back), FADE_STEP.min(period - t))
    }
}

fn smooth(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}

fn ease_out(p: f32) -> f32 {
    1.0 - (1.0 - p.clamp(0.0, 1.0)).powi(3)
}

/// Close enough to slide to.
fn near((_, from): (u16, u16), (_, to): (u16, u16)) -> bool {
    from.abs_diff(to) <= FAR_ROWS
}

/// A step along a row, as typing makes, or a change of shape in place.
fn step((from_col, from_row): (u16, u16), (to_col, to_row): (u16, u16)) -> bool {
    from_row == to_row && from_col.abs_diff(to_col) <= STEP_COLS
}

impl Slide {
    fn new(from: [Corner; 4], to: [Corner; 4], step: bool, start: Instant) -> Self {
        let center = |c: &[Corner; 4]| {
            let (x, y) = c.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
            (x / 4.0, y / 4.0)
        };
        let ((fx, fy), (tx, ty)) = (center(&from), center(&to));
        let (dx, dy) = (tx - fx, ty - fy);
        // How far each corner sits ahead in the direction of the move.
        let ahead = to.map(|(x, y)| (x - tx) * dx + (y - ty) * dy);
        let (low, high) = ahead
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), a| (l.min(*a), h.max(*a)));
        let takes = ahead.map(|a| {
            if step {
                SHORT
            } else if (high - low).abs() < f32::EPSILON {
                LEAD
            } else {
                TAIL - (TAIL - LEAD).mul_f32((a - low) / (high - low))
            }
        });
        Self {
            from,
            to,
            start,
            takes,
        }
    }

    /// How long until every corner is there.
    fn length(&self) -> Duration {
        self.takes.into_iter().max().unwrap_or_default()
    }

    fn at(&self, now: Instant) -> [Corner; 4] {
        let elapsed = now.saturating_duration_since(self.start).as_secs_f32();
        std::array::from_fn(|i| {
            let e = ease_out(elapsed / self.takes[i].as_secs_f32());
            let ((fx, fy), (tx, ty)) = (self.from[i], self.to[i]);
            (fx + (tx - fx) * e, fy + (ty - fy) * e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: (f32, f32) = (8.0, 16.0);

    fn at(col: u16, row: u16) -> Target {
        Target {
            cell: (col, row),
            rect: Rect {
                x: f32::from(col) * CELL.0,
                y: f32::from(row) * CELL.1,
                w: CELL.0,
                h: CELL.1,
            },
        }
    }

    fn lively() -> Conditions {
        Conditions {
            focused: true,
            blink: true,
            slide: true,
            snap: false,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn blinking_holds_then_fades_out_stays_hidden_and_fades_back_in() {
        assert_eq!(blink(Duration::ZERO), (1.0, SHOWN));
        assert_eq!(blink(SHOWN - ms(100)), (1.0, ms(100)));
        let (half, next) = blink(SHOWN + FADE / 2);
        assert!((0.3..0.7).contains(&half), "{half}");
        assert_eq!(next, FADE_STEP);
        // The last step of a fade ends with it.
        assert_eq!(blink(SHOWN + FADE - ms(10)).1, ms(10));
        assert_eq!(blink(SHOWN + FADE), (0.0, HIDDEN));
        let back = blink(SHOWN + FADE + HIDDEN + FADE / 2).0;
        assert!((0.3..0.7).contains(&back), "{back}");
        let period = SHOWN + FADE + HIDDEN + FADE;
        assert_eq!(blink(period), (1.0, SHOWN));
        assert_eq!(blink(period * 3 + ms(20)), (1.0, SHOWN - ms(20)));
    }

    #[test]
    fn a_focused_cursor_blinks_and_typing_shows_it_solid_again() {
        let mut caret = Caret::default();
        let start = Instant::now();
        let look = caret.frame(Some(at(3, 2)), lively(), start);
        assert_eq!(look.alpha, 1.0);
        assert_eq!(look.next, Next::At(start + SHOWN));
        let hidden = start + SHOWN + FADE + ms(10);
        assert_eq!(caret.frame(Some(at(3, 2)), lively(), hidden).alpha, 0.0);
        caret.touch(hidden);
        let look = caret.frame(Some(at(3, 2)), lively(), hidden);
        assert_eq!((look.alpha, look.next), (1.0, Next::At(hidden + SHOWN)));
    }

    #[test]
    fn no_blinking_without_the_keyboard_or_when_the_program_or_settings_say_so() {
        let start = Instant::now();
        let later = start + SHOWN + FADE + ms(10);
        for conditions in [
            Conditions {
                focused: false,
                ..lively()
            },
            Conditions {
                blink: false,
                ..lively()
            },
        ] {
            let mut caret = Caret::default();
            caret.frame(Some(at(3, 2)), conditions, start);
            assert_eq!(caret.frame(Some(at(3, 2)), conditions, later), Look::STILL);
        }
    }

    #[test]
    fn moving_slides_with_the_leading_corners_ahead_then_blinks_from_there() {
        let mut caret = Caret::default();
        let start = Instant::now();
        caret.frame(Some(at(2, 1)), lively(), start);
        let moved = start + ms(1000);
        let look = caret.frame(Some(at(12, 1)), lively(), moved);
        assert_eq!(look.next, Next::Frame);
        assert_eq!(look.alpha, 1.0);
        let corners = look.corners.expect("sliding");
        assert_eq!(corners, at(2, 1).rect.corners(), "starts where it was");
        let mid = caret
            .frame(Some(at(12, 1)), lively(), moved + LEAD / 2)
            .corners
            .unwrap();
        let (from, to) = (at(2, 1).rect.corners(), at(12, 1).rect.corners());
        // Moving right: the right corners lead, the left ones trail.
        let done = |i: usize| (mid[i].0 - from[i].0) / (to[i].0 - from[i].0);
        assert!(done(1) > done(0) && done(2) > done(3), "{mid:?}");
        assert_eq!(done(1), done(2), "a straight move keeps the box square");
        assert!(mid.iter().all(|c| c.1 == from[0].1 || c.1 == from[2].1));
        let arrived = moved + TAIL;
        let look = caret.frame(Some(at(12, 1)), lively(), arrived);
        assert_eq!(look.corners, None);
        assert_eq!(look.next, Next::At(moved + SHOWN), "blinks from the move");
    }

    #[test]
    fn a_step_while_typing_is_short_and_square() {
        let mut caret = Caret::default();
        let start = Instant::now();
        caret.frame(Some(at(5, 0)), lively(), start);
        let look = caret.frame(Some(at(6, 0)), lively(), start + ms(500));
        assert!(look.corners.is_some());
        let mid = caret
            .frame(Some(at(6, 0)), lively(), start + ms(500) + SHORT / 2)
            .corners
            .unwrap();
        assert!(
            (mid[1].0 - mid[0].0 - CELL.0).abs() < 1e-4,
            "no trail: {mid:?}"
        );
        let look = caret.frame(Some(at(6, 0)), lively(), start + ms(500) + SHORT);
        assert_eq!(look.corners, None);
    }

    #[test]
    fn far_jumps_snaps_unfocused_panes_and_a_cursor_just_shown_go_straight_there() {
        let start = Instant::now();
        let cases: [(Conditions, Option<Target>); 4] = [
            (lively(), Some(at(2, 2 + FAR_ROWS + 1))),
            (
                Conditions {
                    snap: true,
                    ..lively()
                },
                Some(at(4, 3)),
            ),
            (
                Conditions {
                    slide: false,
                    ..lively()
                },
                Some(at(4, 3)),
            ),
            (
                Conditions {
                    focused: false,
                    ..lively()
                },
                Some(at(4, 3)),
            ),
        ];
        for (conditions, to) in cases {
            let mut caret = Caret::default();
            caret.frame(Some(at(2, 2)), lively(), start);
            let look = caret.frame(to, conditions, start + ms(1000));
            assert_eq!(look.corners, None, "{conditions:?}");
        }
        // Shown again after being hidden: no slide from where it was before.
        let mut caret = Caret::default();
        caret.frame(Some(at(2, 2)), lively(), start);
        assert_eq!(caret.frame(None, lively(), start + ms(10)), Look::STILL);
        let look = caret.frame(Some(at(3, 3)), lively(), start + ms(20));
        assert_eq!(look.corners, None);
    }

    #[test]
    fn moving_again_mid_slide_goes_on_from_where_it_is() {
        let mut caret = Caret::default();
        let start = Instant::now();
        caret.frame(Some(at(0, 0)), lively(), start);
        caret.frame(Some(at(20, 0)), lively(), start + ms(100));
        let mid = caret
            .frame(Some(at(20, 0)), lively(), start + ms(140))
            .corners
            .unwrap();
        let look = caret.frame(Some(at(20, 3)), lively(), start + ms(140));
        assert_eq!(look.corners, Some(mid));
    }

    #[test]
    fn a_shape_change_in_place_eases_and_restarts_the_blinking() {
        let mut caret = Caret::default();
        let start = Instant::now();
        caret.frame(Some(at(4, 4)), lively(), start);
        let beam = Target {
            rect: Rect {
                w: 2.0,
                ..at(4, 4).rect
            },
            ..at(4, 4)
        };
        let later = start + ms(2000);
        let look = caret.frame(Some(beam), lively(), later);
        assert!(look.corners.is_some());
        let look = caret.frame(Some(beam), lively(), later + SHORT);
        assert_eq!(look.next, Next::At(later + SHOWN));
    }

    /// The input method starting to compose mid-slide: the slide ends there, not drawn over the
    /// composition.
    #[test]
    fn a_snap_mid_slide_puts_the_cursor_in_its_place_at_once() {
        let mut caret = Caret::default();
        let start = Instant::now();
        caret.frame(Some(at(0, 0)), lively(), start);
        caret.frame(Some(at(20, 0)), lively(), start + ms(100));
        let snap = Conditions {
            snap: true,
            ..lively()
        };
        let look = caret.frame(Some(at(20, 0)), snap, start + ms(110));
        assert_eq!(look.corners, None);
        assert_ne!(look.next, Next::Frame);
    }
}
