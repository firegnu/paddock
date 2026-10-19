//! A number easing to where it goes (P5-148a): how far a popup has come in, a pane grown, a
//! sidebar folded. It goes from wherever it is when sent elsewhere, so a change of mind halfway
//! turns it round there, and it asks for frames only while it moves. Plain numbers and times, for
//! the pane's find bar and for the window around it.
use super::caret::Next;
use std::time::{Duration, Instant};

/// A popup, menu, dialog or find bar coming in (the approved demo's 1A): faded in over this long
/// as it rises this many points to its place; going, faded out quicker, where it is. With the
/// system's Reduce Motion on, it only fades.
pub const POP_IN: Duration = Duration::from_millis(120);
pub const POP_OUT: Duration = Duration::from_millis(84);
pub const POP_RISE: f32 = 4.0;
pub const POP_CURVE: Curve = Curve(0.2, 0.7, 0.2, 1.0);

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve(pub f32, pub f32, pub f32, pub f32);

impl Curve {
    /// The curve's value `x` (0 to 1) of the way through the time.
    pub fn at(self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        } else if x >= 1.0 {
            return 1.0;
        }
        let Self(x1, y1, x2, y2) = self;
        let along = |t: f32, p1: f32, p2: f32| {
            let u = 1.0 - t;
            3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t
        };
        // The time is monotonic in t for x1 and x2 within 0 to 1: halve towards it.
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..32 {
            let mid = (low + high) / 2.0;
            if along(mid, x1, x2) < x {
                low = mid;
            } else {
                high = mid;
            }
        }
        along((low + high) / 2.0, y1, y2)
    }
}

/// A number from 0 to 1 and where it is going.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    start: Instant,
    /// How long it takes from `from` to `to`.
    took: Duration,
    curve: Curve,
}

impl Tween {
    /// Still at `value`.
    pub fn at(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Instant::now(),
            took: Duration::ZERO,
            curve: Curve(0.0, 0.0, 1.0, 1.0),
        }
    }

    /// Where it is going.
    pub fn target(&self) -> f32 {
        self.to
    }

    /// Where it is at `now`.
    pub fn value(&self, now: Instant) -> f32 {
        if self.took.is_zero() {
            return self.to;
        }
        let t = now.saturating_duration_since(self.start).as_secs_f32() / self.took.as_secs_f32();
        self.from + (self.to - self.from) * self.curve.at(t)
    }

    /// Sent to `to` from where it is at `now`, along `curve`: the whole way, 0 to 1, takes
    /// `whole`, a part of it that part of the time. Already going there, it keeps on.
    pub fn go(&mut self, to: f32, now: Instant, whole: Duration, curve: Curve) {
        if to == self.to {
            return;
        }
        let from = self.value(now);
        let part = (to - from).abs();
        *self = Self {
            from,
            to,
            start: now,
            took: if part >= 1.0 {
                whole
            } else {
                whole.mul_f64(f64::from(part))
            },
            curve,
        };
    }

    /// At `to` at once.
    pub fn jump(&mut self, to: f32) {
        *self = Self::at(to);
    }

    /// Still on its way at `now`.
    pub fn moving(&self, now: Instant) -> bool {
        !self.took.is_zero() && now.saturating_duration_since(self.start) < self.took
    }

    /// When to draw again for it: every frame while it moves, else not for it.
    pub(super) fn next(&self, now: Instant) -> Next {
        if self.moving(now) {
            Next::Frame
        } else {
            Next::Idle
        }
    }
}

/// `from` to `to`, `t` of the way.
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOLE: Duration = Duration::from_millis(180);
    const EASE: Curve = Curve(0.2, 0.8, 0.2, 1.0);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn the_curve_matches_css() {
        let linear = Curve(0.0, 0.0, 1.0, 1.0);
        assert!(close(linear.at(0.3), 0.3));
        assert_eq!(EASE.at(0.0), 0.0);
        assert_eq!(EASE.at(1.0), 1.0);
        // Out fast and in slow: more than half way at a third of the time.
        assert!(EASE.at(0.33) > 0.6, "{}", EASE.at(0.33));
        assert!(EASE.at(0.5) < EASE.at(0.6));
    }

    #[test]
    fn it_starts_where_it_is_eases_on_and_stops_there() {
        let start = Instant::now();
        let mut tween = Tween::at(0.0);
        assert!(!tween.moving(start));
        assert_eq!(tween.next(start), Next::Idle);
        tween.go(1.0, start, WHOLE, EASE);
        assert_eq!(tween.value(start), 0.0);
        assert_eq!(tween.next(start), Next::Frame);
        let middle = tween.value(start + ms(90));
        assert!(close(middle, EASE.at(0.5)), "{middle}");
        assert!(tween.moving(start + ms(179)));
        // There: still, and no more frames asked for.
        assert_eq!(tween.value(start + ms(180)), 1.0);
        assert!(!tween.moving(start + ms(180)));
        assert_eq!(tween.next(start + ms(400)), Next::Idle);
        assert_eq!(tween.target(), 1.0);
    }

    #[test]
    fn turned_round_halfway_it_goes_back_from_there_in_that_part_of_the_time() {
        let start = Instant::now();
        let mut tween = Tween::at(0.0);
        tween.go(1.0, start, WHOLE, EASE);
        let turn = start + ms(45);
        let there = tween.value(turn);
        assert!(there > 0.0 && there < 1.0);
        tween.go(0.0, turn, WHOLE, EASE);
        // No jump where it turns.
        assert!(close(tween.value(turn), there));
        // Back in the part of the time that part of the way takes.
        let back = WHOLE.mul_f32(there);
        assert!(tween.moving(turn + back - ms(1)));
        assert_eq!(tween.value(turn + back), 0.0);
        assert!(!tween.moving(turn + back));
    }

    #[test]
    fn sent_where_it_goes_already_it_keeps_on_and_a_jump_is_there_at_once() {
        let start = Instant::now();
        let mut tween = Tween::at(0.0);
        tween.go(1.0, start, WHOLE, EASE);
        tween.go(1.0, start + ms(90), WHOLE, EASE);
        assert!(close(tween.value(start + ms(90)), EASE.at(0.5)));
        tween.jump(0.0);
        assert_eq!(tween.value(start + ms(91)), 0.0);
        assert_eq!(tween.next(start + ms(91)), Next::Idle);
    }

    #[test]
    fn lerp_goes_between() {
        assert_eq!(lerp(10.0, 20.0, 0.0), 10.0);
        assert_eq!(lerp(10.0, 20.0, 0.25), 12.5);
        assert_eq!(lerp(10.0, 20.0, 1.0), 20.0);
    }
}
