//! Things the pane shows for a moment and lets fade (P5-143e): the history's position bar while
//! the view is scrolled, the grid's size while the pane is resized. Shown at full strength for a
//! while after the last poke, then faded out; the pane is drawn again when the fade begins (a
//! timer) and every frame while it fades, and not at all after.
use super::caret::Next;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Fade {
    /// The last poke.
    since: Option<Instant>,
    /// How long it shows at full strength, then how long it takes to fade.
    hold: Duration,
    out: Duration,
}

impl Fade {
    pub const fn new(hold: Duration, out: Duration) -> Self {
        Self {
            since: None,
            hold,
            out,
        }
    }

    /// Shown again at full strength, from `now`.
    pub fn poke(&mut self, now: Instant) {
        self.since = Some(now);
    }

    /// Gone at once.
    pub fn hide(&mut self) {
        self.since = None;
    }

    /// How strongly it shows at `now`, 0 to 1.
    pub fn alpha(&self, now: Instant) -> f32 {
        let Some(since) = self.since else {
            return 0.0;
        };
        let gone = now.saturating_duration_since(since);
        if gone < self.hold {
            1.0
        } else if gone < self.hold + self.out {
            1.0 - (gone - self.hold).as_secs_f32() / self.out.as_secs_f32()
        } else {
            0.0
        }
    }

    /// When the pane is to be drawn again for it.
    pub fn next(&self, now: Instant) -> Next {
        let Some(since) = self.since else {
            return Next::Idle;
        };
        let gone = now.saturating_duration_since(since);
        if gone < self.hold {
            Next::At(since + self.hold)
        } else if gone < self.hold + self.out {
            Next::Frame
        } else {
            Next::Idle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOLD: Duration = Duration::from_millis(800);
    const OUT: Duration = Duration::from_millis(300);
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn unpoked_it_does_not_show_or_wake() {
        let fade = Fade::new(HOLD, OUT);
        let now = Instant::now();
        assert_eq!(fade.alpha(now), 0.0);
        assert_eq!(fade.next(now), Next::Idle);
    }

    #[test]
    fn it_holds_then_fades_then_is_gone_waking_only_as_needed() {
        let start = Instant::now();
        let mut fade = Fade::new(HOLD, OUT);
        fade.poke(start);
        // Held: full strength, a timer for when the fade begins.
        assert_eq!(fade.alpha(start + ms(500)), 1.0);
        assert_eq!(fade.next(start + ms(500)), Next::At(start + HOLD));
        // Fading: every frame.
        let half = fade.alpha(start + ms(950));
        assert!((half - 0.5).abs() < 0.01, "{half}");
        assert_eq!(fade.next(start + ms(950)), Next::Frame);
        // Gone: nothing more.
        assert_eq!(fade.alpha(start + ms(1100)), 0.0);
        assert_eq!(fade.next(start + ms(1100)), Next::Idle);
    }

    #[test]
    fn a_poke_while_fading_shows_it_again_and_hide_takes_it_away() {
        let start = Instant::now();
        let mut fade = Fade::new(HOLD, OUT);
        fade.poke(start);
        fade.poke(start + ms(1000));
        assert_eq!(fade.alpha(start + ms(1100)), 1.0);
        fade.hide();
        assert_eq!(fade.alpha(start + ms(1100)), 0.0);
        assert_eq!(fade.next(start + ms(1100)), Next::Idle);
    }
}
