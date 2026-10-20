//! Program progress and visible bells: time-based samples, with no timers of their own.
use super::tween::{Curve, Tween};
use crate::terminal::{Progress, Signals};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    pub progress: Progress,
    pub fraction: f32,
    pub offset: f32,
    pub opacity: f32,
    pub completing: bool,
    pub glow: f32,
    pub bell: f32,
    pub swing: Option<Duration>,
    pub moving: bool,
}

pub struct Animation {
    session: Option<u64>,
    last: Signals,
    progress: Progress,
    fraction: Tween,
    started: Instant,
    completed: Option<Instant>,
    rang: Option<Instant>,
}

impl Default for Animation {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation {
    pub fn new() -> Self {
        Self {
            session: None,
            last: Signals::default(),
            progress: Progress::None,
            fraction: Tween::at(0.0),
            started: Instant::now(),
            completed: None,
            rang: None,
        }
    }

    pub fn update(
        &mut self,
        session: Option<u64>,
        signals: Signals,
        now: Instant,
        still: bool,
    ) -> bool {
        if self.session == session && self.last == signals {
            return false;
        }
        if self.session != session || signals.ended {
            *self = Self::new();
            self.session = session;
        }
        if signals.ended {
            self.last = signals;
            return true;
        }
        if signals.bells != self.last.bells {
            self.rang = Some(now);
        }
        if signals.progress != self.last.progress {
            let to = match signals.progress {
                Progress::None => {
                    self.completed = Some(now);
                    1.0
                }
                progress => {
                    self.completed = None;
                    self.progress = progress;
                    self.started = now;
                    match progress {
                        Progress::Percent(n) | Progress::Error(n) | Progress::Paused(n) => {
                            f32::from(n) / 100.0
                        }
                        _ => 0.28,
                    }
                }
            };
            if still {
                self.fraction.jump(to);
            } else {
                self.fraction.go(
                    to,
                    now,
                    Duration::from_millis(200),
                    Curve(0.2, 0.7, 0.2, 1.0),
                );
            }
        }
        self.last = signals;
        true
    }

    pub fn frame(&self, now: Instant, still: bool) -> Frame {
        let mut frame = Frame {
            progress: self.progress,
            ..Frame::default()
        };
        if self.progress != Progress::None {
            frame.fraction = if still {
                self.fraction.target()
            } else {
                self.fraction.value(now)
            };
            frame.opacity = 1.0;
            frame.moving = !still && self.fraction.moving(now);
            if let Some(done) = self.completed {
                frame.completing = true;
                // First fill to the end (at most 200 ms), then fade for 300 ms.
                let elapsed = now.saturating_duration_since(done).as_secs_f32();
                frame.opacity = (1.0 - (elapsed - 0.2).max(0.0) / 0.3).max(0.0);
                frame.moving = elapsed < 0.5;
            } else if self.progress == Progress::Indeterminate {
                if still {
                    frame.fraction = 1.0;
                    frame.opacity = 0.3;
                    frame.moving = false;
                } else {
                    frame.fraction = 0.28;
                    frame.offset =
                        (now.saturating_duration_since(self.started).as_secs_f32() / 1.5).fract();
                    frame.moving = true;
                }
            }
        }
        if let Some(rang) = self.rang {
            let elapsed = now.saturating_duration_since(rang);
            frame.glow = (1.0 - elapsed.as_secs_f32() / 0.45).max(0.0);
            frame.bell = (1.0 - elapsed.as_secs_f32() / 2.5).max(0.0);
            frame.swing = (!still && elapsed < Duration::from_millis(450)).then_some(elapsed);
            frame.moving |= frame.bell > 0.0;
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(progress: Progress) -> Signals {
        Signals {
            progress,
            ..Signals::default()
        }
    }
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn signals_percentage_and_completion_stop_requesting_frames() {
        let now = Instant::now();
        let mut animation = Animation::new();
        assert!(!animation.frame(now, false).moving);
        animation.update(Some(1), at(Progress::Percent(40)), now, false);
        assert!(animation.frame(now + ms(20), false).moving);
        let settled = animation.frame(now + ms(300), false);
        assert!(!settled.moving);
        assert_eq!(settled.fraction, 0.4);
        assert_eq!(settled.opacity, 1.0);
        assert!(!animation.update(Some(1), at(Progress::Percent(40)), now + ms(300), false));
        animation.update(Some(1), Signals::default(), now + ms(300), false);
        let full = animation.frame(now + ms(500), false);
        assert_eq!(full.fraction, 1.0);
        assert!(full.opacity > 0.0);
        let gone = animation.frame(now + ms(1000), false);
        assert_eq!(gone.opacity, 0.0);
        assert!(!gone.moving);
    }

    #[test]
    fn signals_indeterminate_and_reduce_motion_frame_policy() {
        let now = Instant::now();
        let mut animation = Animation::new();
        animation.update(Some(1), at(Progress::Indeterminate), now, false);
        let first = animation.frame(now, false);
        let later = animation.frame(now + ms(750), false);
        assert!(later.moving);
        assert_eq!(later.fraction, 0.28);
        assert_ne!(first.offset, later.offset);
        let still = animation.frame(now + ms(750), true);
        assert!(!still.moving);
        assert_eq!(still.offset, 0.0);
        assert_eq!(still.fraction, 1.0);
        assert!(still.opacity > 0.0 && still.opacity < 1.0);
        for progress in [Progress::Error(40), Progress::Paused(40)] {
            animation.update(Some(1), at(progress), now, false);
            assert!(!animation.frame(now + ms(300), false).moving);
        }
    }

    #[test]
    fn signals_bell_fades_and_repeated_bells_restart_without_timers() {
        let now = Instant::now();
        let mut animation = Animation::new();
        let bell = Signals {
            bells: 1,
            ..Signals::default()
        };
        animation.update(Some(1), bell, now, false);
        let first = animation.frame(now, false);
        assert_eq!(first.glow, 1.0);
        assert!(first.swing.is_some());
        assert!(first.moving);
        assert!(animation.frame(now, true).swing.is_none());
        let later = animation.frame(now + ms(450), false);
        assert_eq!(later.glow, 0.0);
        assert!(later.bell > 0.0);
        let gone = animation.frame(now + ms(2500), false);
        assert_eq!(gone.bell, 0.0);
        assert!(!gone.moving);
        animation.update(Some(1), Signals { bells: 2, ..bell }, now + ms(3000), false);
        assert_eq!(animation.frame(now + ms(3000), false).glow, 1.0);
    }

    #[test]
    fn signals_session_replacement_and_exit_clear_without_completion() {
        let now = Instant::now();
        let mut animation = Animation::new();
        animation.update(
            Some(1),
            Signals {
                bells: 1,
                ..at(Progress::Indeterminate)
            },
            now,
            false,
        );
        assert!(animation.frame(now, false).opacity > 0.0);
        assert!(animation.frame(now, false).bell > 0.0);
        animation.update(Some(2), Signals::default(), now + ms(20), false);
        let clear = animation.frame(now + ms(20), false);
        assert_eq!(clear.opacity, 0.0);
        assert_eq!(clear.bell, 0.0);
        assert!(!clear.moving);
        animation.update(Some(2), at(Progress::Indeterminate), now + ms(30), false);
        animation.update(
            Some(2),
            Signals {
                ended: true,
                ..Signals::default()
            },
            now + ms(40),
            false,
        );
        let ended = animation.frame(now + ms(40), false);
        assert_eq!(ended.opacity, 0.0);
        assert!(!ended.moving);
    }
}
