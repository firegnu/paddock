//! Icon buttons that move when the pointer comes over them (DESIGN §13, P5-26): the sidebar's bell
//! swings, a card's Copy nudges and shows a tick once clicked, its Stop presses in and reddens, the
//! menu button's knobs slide. The title bar's way in to things too (P5-27): a sidebar switch's
//! edge slides the way its click goes, the new tab `+` turns, Split parts, Search's magnifier
//! tilts. Pause's bars dip together, and Resume's triangle slides the way it plays (P5-33). A
//! motion plays once as the pointer comes in, never loops, and asks for no frames once it is still.
//! With the system's Reduce Motion on, nothing moves: colours and grounds still change and the tick
//! still shows. The timings and curves are the approved demo's (`docs/设计稿/P5-26-图标动效/`),
//! and in its manner for the title bar's and Pause's (DESIGN §13).
use crate::{
    footer_icon::{Icon, Pose},
    reduce_motion,
};
use gpui::{
    App, Div, ElementId, Hsla, IntoElement, RenderOnce, Rgba, Stateful, StatefulInteractiveElement,
    Window,
};
use std::time::{Duration, Instant};

/// How long a button's motions last; zero for one it does not have.
#[derive(Clone, Copy, Debug, Default)]
pub struct Timing {
    /// The motion played once as the pointer comes in.
    pub play: Duration,
    /// The change that follows the pointer in and out (see [`Hover::lit`]).
    pub fade: Duration,
    /// How long a click's mark stays (see [`Hover::flash`]).
    pub flash: Duration,
}

/// The bell's swing, the clapper's a beat later.
pub const RING: Timing = Timing {
    play: ms(RING_MS + CLAPPER_DELAY_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// Copy's nudge, and its tick after a click.
pub const COPY: Timing = Timing {
    play: ms(NUDGE_MS),
    fade: Duration::ZERO,
    flash: ms(COPIED_MS),
};
/// Stop's press, in as the pointer comes and out as it goes.
pub const PRESS: Timing = Timing {
    play: Duration::ZERO,
    fade: ms(PRESS_MS),
    flash: Duration::ZERO,
};
/// The knobs sliding one after another.
pub const SLIDE: Timing = Timing {
    play: ms(SLIDE_MS + 2.0 * KNOB_DELAY_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// A sidebar switch's edge sliding out and back.
pub const SHIFT: Timing = Timing {
    play: ms(SHIFT_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// The new tab `+` turning a quarter.
pub const SPIN: Timing = Timing {
    play: ms(SPIN_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// Split's halves parting and closing.
pub const PART: Timing = Timing {
    play: ms(PART_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// The magnifier tilting.
pub const TILT: Timing = Timing {
    play: ms(TILT_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};
/// Pause's bars pressed down together and back (P5-33).
pub const DIP: Timing = Timing {
    play: ms(DIP_MS),
    fade: Duration::ZERO,
    flash: Duration::ZERO,
};

const RING_MS: f32 = 520.0;
const CLAPPER_DELAY_MS: f32 = 40.0;
/// The swing in degrees clockwise, at fractions of its time.
const RING_FRAMES: [(f32, f32); 6] = [
    (0.0, 0.0),
    (0.15, 16.0),
    (0.35, -12.0),
    (0.55, 7.0),
    (0.75, -3.0),
    (1.0, 0.0),
];
const RING_EASE: Bezier = Bezier(0.36, 0.07, 0.19, 0.97);
const NUDGE_MS: f32 = 340.0;
/// The front square's push up and to the left, in points, at fractions of its time.
const NUDGE_FRAMES: [(f32, f32); 3] = [(0.0, 0.0), (0.45, 1.6), (1.0, 0.0)];
/// Overshoots a little before it settles.
const SPRING: Bezier = Bezier(0.34, 1.56, 0.64, 1.0);
const COPIED_MS: f32 = 1200.0;
const POP_MS: f32 = 260.0;
/// The tick's size as it pops in, from this to whole.
const POP_FROM: f32 = 0.4;
const PRESS_MS: f32 = 160.0;
/// The CSS `ease-out` curve.
const EASE_OUT: Bezier = Bezier(0.0, 0.0, 0.58, 1.0);
/// Stop's square, pressed: its size and its corners' radius, from resting 1 and 2.5.
const PRESSED_SCALE: f32 = 0.84;
const PRESSED_RADIUS: f32 = 3.4;
const RESTING_RADIUS: f32 = 2.5;
/// Stop's ground when pressed, over the red.
pub const PRESSED_GROUND: f32 = 0.12;
const SLIDE_MS: f32 = 620.0;
const KNOB_DELAY_MS: f32 = 60.0;
/// How far each knob goes, top to bottom, halfway through its slide.
const KNOB_REACH: [f32; 3] = [4.5, -5.0, 4.0];
const SLIDE_EASE: Bezier = Bezier(0.45, 0.0, 0.2, 1.0);
const SHIFT_MS: f32 = 360.0;
/// How far a sidebar switch's edge goes, halfway through.
const SHIFT_REACH: f32 = 1.5;
const SPIN_MS: f32 = 320.0;
const SPIN_DEGREES: f32 = 90.0;
const PART_MS: f32 = 380.0;
/// How far each half of Split goes from the seam, halfway through.
const PART_REACH: f32 = 0.9;
const TILT_MS: f32 = 420.0;
/// The magnifier's tilt in degrees clockwise, at fractions of its time.
const TILT_FRAMES: [(f32, f32); 4] = [(0.0, 0.0), (0.3, -14.0), (0.65, 8.0), (1.0, 0.0)];
/// The bell's curve.
const TILT_EASE: Bezier = RING_EASE;
const DIP_MS: f32 = 320.0;
/// How far Pause's bars go down, at fractions of its time.
const DIP_FRAMES: [(f32, f32); 3] = [(0.0, 0.0), (0.4, 1.5), (1.0, 0.0)];
/// The activity grid's cells popping in one after another along the diagonal from the top left
/// (P5-32, the design's `cellIn`): each takes this long, growing from [`CELL_FROM`] of its size
/// as it fades in, its neighbour this much later.
pub const CELL_IN: Duration = ms(450.0);
pub const CELL_STAGGER: Duration = ms(18.0);
const CELL_FROM: f32 = 0.3;
const CELL_EASE: Bezier = Bezier(0.2, 0.8, 0.2, 1.0);
/// Today's cell breathing: one breath in and out.
pub const GLOW: Duration = ms(2400.0);
/// The CSS `ease-in-out` curve.
const EASE_IN_OUT: Bezier = Bezier(0.42, 0.0, 0.58, 1.0);
/// A cell whose commits rose at a refresh swells this long and settles.
pub const BUMP: Duration = ms(560.0);
/// Its size at fractions of that time.
const BUMP_FRAMES: [(f32, f32); 3] = [(0.0, 1.0), (0.35, 1.3), (1.0, 1.0)];

const fn ms(ms: f32) -> Duration {
    Duration::from_millis(ms as u64)
}

/// Where a button's motions are as it is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hover {
    /// Whether the pointer is over it.
    pub hovered: bool,
    /// How long since the pointer came in, while the motion that plays then lasts; it finishes
    /// even if the pointer leaves. Never with Reduce Motion on.
    pub play: Option<Duration>,
    /// 0 resting, 1 hovered, eased between along the fade as the pointer comes and goes.
    pub lit: f32,
    /// How long since the last click, while its mark lasts.
    pub flash: Option<Duration>,
    /// The system's Reduce Motion, as it was when the pointer last came or went.
    pub still: bool,
}

/// What a button remembers between frames.
#[derive(Clone, Copy, Debug, Default)]
struct State {
    hovered: bool,
    /// When the pointer last came in.
    entered: Option<Instant>,
    /// When the pointer last came or went, and how lit it was then.
    changed: Option<(Instant, f32)>,
    clicked: Option<Instant>,
    still: bool,
    /// The wake-up already asked for, so each is asked for once.
    wake: Option<Instant>,
}

/// What a button needs next.
#[derive(Debug, PartialEq)]
enum Next {
    /// Nothing until the pointer or a click changes something.
    Idle,
    /// The next frame: something is moving.
    Frame,
    /// A redraw at this moment, nothing moving until then.
    At(Instant),
}

impl State {
    fn lit(&self, now: Instant, fade: Duration) -> f32 {
        let target = if self.hovered { 1.0 } else { 0.0 };
        match self.changed {
            Some((at, from)) if !fade.is_zero() => {
                let t = (now - at).as_secs_f32() / fade.as_secs_f32();
                from + (target - from) * EASE_OUT.at(t.min(1.0))
            }
            _ => target,
        }
    }

    /// The pointer came in (`hovered`) or went, with Reduce Motion `still`.
    fn hover(&mut self, hovered: bool, now: Instant, still: bool, timing: Timing) {
        let lit = self.lit(now, timing.fade);
        self.hovered = hovered;
        self.changed = Some((now, lit));
        self.still = still;
        if hovered {
            self.entered = Some(now);
        }
    }

    fn at(&self, now: Instant, timing: Timing) -> (Hover, Next) {
        let since = |at: Option<Instant>, lasts: Duration| {
            at.map(|at| now - at).filter(|since| *since < lasts)
        };
        let play = since(self.entered, timing.play).filter(|_| !self.still);
        let fading = self.changed.is_some_and(|(at, _)| now - at < timing.fade);
        let flash = since(self.clicked, timing.flash);
        let hover = Hover {
            hovered: self.hovered,
            play,
            lit: if fading {
                self.lit(now, timing.fade)
            } else if self.hovered {
                1.0
            } else {
                0.0
            },
            flash,
            still: self.still,
        };
        let popping = flash.is_some_and(|since| since < ms(POP_MS) && !self.still);
        let next = if play.is_some() || fading || popping {
            Next::Frame
        } else if let (Some(at), Some(_)) = (self.clicked, flash) {
            Next::At(at + timing.flash)
        } else {
            Next::Idle
        };
        (hover, next)
    }
}

/// A button whose look follows its [`Hover`]: `build` makes it for each frame, this tracks the
/// pointer and its clicks and draws it again while anything moves.
#[derive(IntoElement)]
pub struct HoverMotion {
    key: ElementId,
    timing: Timing,
    build: Box<dyn FnOnce(Hover) -> Stateful<Div>>,
}

/// A button `build` makes for each frame from its [`Hover`], `key` naming its state among its
/// siblings.
pub fn hover_motion(
    key: impl Into<ElementId>,
    timing: Timing,
    build: impl FnOnce(Hover) -> Stateful<Div> + 'static,
) -> HoverMotion {
    HoverMotion {
        key: key.into(),
        timing,
        build: Box::new(build),
    }
}

impl HoverMotion {
    /// The button changed further by `f`, for its callers.
    pub fn map(self, f: impl FnOnce(Stateful<Div>) -> Stateful<Div> + 'static) -> Self {
        let build = self.build;
        Self {
            build: Box::new(move |hover| f(build(hover))),
            ..self
        }
    }
}

impl RenderOnce for HoverMotion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let timing = self.timing;
        let state = window.use_keyed_state(self.key, cx, |_, _| State::default());
        let now = Instant::now();
        let (hover, next) = state.read(cx).at(now, timing);
        match next {
            Next::Idle => {}
            Next::Frame => window.request_animation_frame(),
            Next::At(at) if state.read(cx).wake != Some(at) => {
                state.update(cx, |state, _| state.wake = Some(at));
                let state = state.clone();
                window
                    .spawn(cx, async move |cx| {
                        cx.background_executor().timer(at - now).await;
                        state.update(cx, |_, cx| cx.notify());
                    })
                    .detach();
            }
            Next::At(_) => {}
        }
        let mut button = (self.build)(hover).on_hover({
            let state = state.clone();
            move |&hovered, _, cx| {
                let still = reduce_motion::on();
                state.update(cx, |state, cx| {
                    state.hover(hovered, Instant::now(), still, timing);
                    cx.notify();
                });
            }
        });
        if !timing.flash.is_zero() {
            button = button.on_click(move |_, _, cx| {
                state.update(cx, |state, cx| {
                    state.clicked = Some(Instant::now());
                    cx.notify();
                });
            });
        }
        button
    }
}

/// The bell's pose `play` into its swing.
pub fn ring(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let swing = |delay: f32| {
        let t = (millis(play) - delay) / RING_MS;
        if (0.0..1.0).contains(&t) {
            keyframes(t, &RING_FRAMES, RING_EASE)
        } else {
            0.0
        }
    };
    Pose::Swing {
        body: swing(0.0),
        clapper: swing(CLAPPER_DELAY_MS),
    }
}

/// Copy's icon, pose and opacity: `Copy` nudged `play` into its push, or the tick `flash` after a
/// click.
pub fn copy(hover: Hover) -> (Icon, Pose, f32) {
    if let Some(flash) = hover.flash {
        let (pose, opacity) = pop(flash, hover.still);
        return (Icon::Copied, pose, opacity);
    }
    let pose = match hover.play {
        Some(play) => {
            let t = (millis(play) / NUDGE_MS).min(1.0);
            Pose::Nudge(keyframes(t, &NUDGE_FRAMES, SPRING))
        }
        None => Pose::Rest,
    };
    (Icon::Copy, pose, 1.0)
}

/// The tick's pose and opacity `since` the click: popping up to its size, or there at once when
/// `still`.
fn pop(since: Duration, still: bool) -> (Pose, f32) {
    if still {
        return (Pose::Grow(1.0), 1.0);
    }
    let grown = SPRING.at((millis(since) / POP_MS).min(1.0));
    (
        Pose::Grow(POP_FROM + (1.0 - POP_FROM) * grown),
        grown.clamp(0.0, 1.0),
    )
}

/// Stop's square `lit` of the way to pressed; when `still` only its colour changes, which is the
/// caller's.
pub fn press(lit: f32, still: bool) -> Pose {
    if still {
        return Pose::Rest;
    }
    Pose::Press {
        scale: 1.0 + (PRESSED_SCALE - 1.0) * lit,
        radius: RESTING_RADIUS + (PRESSED_RADIUS - RESTING_RADIUS) * lit,
    }
}

/// The knobs' pose `play` into their slide, each a little after the one above.
pub fn slide(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let mut by = [0.0; 3];
    for (i, (by, reach)) in by.iter_mut().zip(KNOB_REACH).enumerate() {
        let t = (millis(play) - KNOB_DELAY_MS * i as f32) / SLIDE_MS;
        if (0.0..1.0).contains(&t) {
            *by = keyframes(t, &[(0.0, 0.0), (0.5, reach), (1.0, 0.0)], SLIDE_EASE);
        }
    }
    Pose::Slide(by)
}

/// A sidebar switch's edge `play` into its slide, to the right when `rightward`, else to the
/// left: the way its click will take the sidebar's edge.
pub fn shift(play: Option<Duration>, rightward: bool) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let t = (millis(play) / SHIFT_MS).min(1.0);
    let by = keyframes(t, &[(0.0, 0.0), (0.5, SHIFT_REACH), (1.0, 0.0)], SLIDE_EASE);
    Pose::Shift(if rightward { by } else { -by })
}

/// The `+` `play` into its quarter turn, overshooting a little before it settles.
pub fn spin(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    Pose::Turn(SPIN_DEGREES * SPRING.at(millis(play) / SPIN_MS))
}

/// Split's halves `play` into parting and closing again.
pub fn part(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let t = (millis(play) / PART_MS).min(1.0);
    Pose::Part(keyframes(
        t,
        &[(0.0, 0.0), (0.5, PART_REACH), (1.0, 0.0)],
        SLIDE_EASE,
    ))
}

/// The magnifier `play` into its tilt about the lens.
pub fn tilt(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let t = (millis(play) / TILT_MS).min(1.0);
    Pose::Turn(keyframes(t, &TILT_FRAMES, TILT_EASE))
}

/// Pause's bars `play` into their dip.
pub fn dip(play: Option<Duration>) -> Pose {
    let Some(play) = play else {
        return Pose::Rest;
    };
    let t = (millis(play) / DIP_MS).min(1.0);
    Pose::Dip(keyframes(t, &DIP_FRAMES, SLIDE_EASE))
}

/// An activity cell's size (a share of its whole) and opacity `since` its turn to pop in came;
/// `None` before it, when it is not there yet.
pub fn cell_in(since: Option<Duration>) -> (f32, f32) {
    let Some(since) = since else {
        return (CELL_FROM, 0.0);
    };
    let t = CELL_EASE.at(millis(since) / millis(CELL_IN));
    (CELL_FROM + (1.0 - CELL_FROM) * t, t.clamp(0.0, 1.0))
}

/// How far today's glow has swelled, 0 to 1 and back, `t` (0 to 1) through a breath.
pub fn glow(t: f32) -> f32 {
    EASE_IN_OUT.at(1.0 - (2.0 * t.clamp(0.0, 1.0) - 1.0).abs())
}

/// The size of a cell whose commits rose, `since` they did: it swells and settles.
pub fn bump(since: Duration) -> f32 {
    keyframes(
        (millis(since) / millis(BUMP)).min(1.0),
        &BUMP_FRAMES,
        EASE_OUT,
    )
}

/// `from` blended `t` of the way to `to`, through red, green and blue as CSS does.
pub fn mix(from: Hsla, to: Hsla, t: f32) -> Hsla {
    let (from, to) = (Rgba::from(from), Rgba::from(to));
    let at = |a: f32, b: f32| a + (b - a) * t;
    Rgba {
        r: at(from.r, to.r),
        g: at(from.g, to.g),
        b: at(from.b, to.b),
        a: at(from.a, to.a),
    }
    .into()
}

fn millis(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

/// The value `t` (0 to 1) of the way through `frames`, `(time, value)` pairs from 0 to 1, eased
/// along each step between two of them as CSS keyframes are.
fn keyframes(t: f32, frames: &[(f32, f32)], ease: Bezier) -> f32 {
    for step in frames.windows(2) {
        let ((t0, from), (t1, to)) = (step[0], step[1]);
        if t <= t1 {
            return from + (to - from) * ease.at(((t - t0) / (t1 - t0)).max(0.0));
        }
    }
    frames.last().map_or(0.0, |&(_, value)| value)
}

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve.
#[derive(Clone, Copy, Debug)]
struct Bezier(f32, f32, f32, f32);

impl Bezier {
    /// The curve's value `x` (0 to 1) of the way through the time.
    fn at(self, x: f32) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn after(ms: f32) -> Duration {
        Duration::from_micros((ms * 1000.0).round() as u64)
    }

    fn swing(play: f32) -> (f32, f32) {
        match ring(Some(after(play))) {
            Pose::Swing { body, clapper } => (body, clapper),
            pose => panic!("{pose:?}"),
        }
    }

    fn knobs(play: f32) -> [f32; 3] {
        match slide(Some(after(play))) {
            Pose::Slide(by) => by,
            pose => panic!("{pose:?}"),
        }
    }

    #[test]
    fn bezier_matches_css() {
        assert_eq!(RING_EASE.at(0.0), 0.0);
        assert_eq!(RING_EASE.at(1.0), 1.0);
        assert!(close(Bezier(0.0, 0.0, 1.0, 1.0).at(0.3), 0.3));
        // The spring overshoots past 1 before it settles.
        assert!(SPRING.at(0.6) > 1.0);
    }

    #[test]
    fn bell_swings_as_the_demo() {
        // 15%, 35%, 55%, 75% of 520ms: 16°, −12°, 7°, −3°.
        for (ms, degrees) in [(78.0, 16.0), (182.0, -12.0), (286.0, 7.0), (390.0, -3.0)] {
            let (body, _) = swing(ms);
            assert!(close(body, degrees), "{body} at {ms}ms");
        }
        // The clapper a beat behind: still on its way at 78ms, at 16° 40ms later.
        let (_, clapper) = swing(78.0);
        assert!(clapper > 0.0 && clapper < 16.0, "{clapper}");
        assert!(close(swing(118.0).1, 16.0));
        // The body done at 520ms, the clapper at 560ms.
        assert_eq!(swing(520.0).0, 0.0);
        assert_eq!(swing(560.0), (0.0, 0.0));
        assert_eq!(ring(None), Pose::Rest);
    }

    #[test]
    fn copy_nudges_then_ticks() {
        let hover = |play: Option<f32>, flash: Option<f32>| Hover {
            play: play.map(after),
            flash: flash.map(after),
            ..Hover::default()
        };
        // 45% of 340ms: pushed the whole 1.6 points.
        match copy(hover(Some(153.0), None)) {
            (Icon::Copy, Pose::Nudge(by), 1.0) => assert!(close(by, 1.6), "{by}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(copy(hover(None, None)), (Icon::Copy, Pose::Rest, 1.0));
        // A click shows the tick, small and clear at first, whole by 260ms.
        assert_eq!(
            copy(hover(Some(100.0), Some(0.0))),
            (Icon::Copied, Pose::Grow(0.4), 0.0)
        );
        assert_eq!(
            copy(hover(None, Some(260.0))),
            (Icon::Copied, Pose::Grow(1.0), 1.0)
        );
    }

    #[test]
    fn stop_presses_in() {
        assert_eq!(
            press(1.0, false),
            Pose::Press {
                scale: 0.84,
                radius: 3.4
            }
        );
        assert_eq!(
            press(0.0, false),
            Pose::Press {
                scale: 1.0,
                radius: 2.5
            }
        );
    }

    #[test]
    fn knobs_slide_one_after_another() {
        // Halfway through each one's own slide: +4.5 at 310ms, −5 at 370ms, +4 at 430ms.
        for (knob, ms, reach) in [(0, 310.0, 4.5), (1, 370.0, -5.0), (2, 430.0, 4.0)] {
            let by = knobs(ms);
            assert!(close(by[knob], reach), "{by:?} at {ms}ms");
        }
        // The later knobs wait their turn.
        let by = knobs(30.0);
        assert!(by[0] > 0.0);
        assert_eq!(&by[1..], &[0.0, 0.0]);
        assert_eq!(knobs(740.0), [0.0; 3]);
        assert_eq!(slide(None), Pose::Rest);
    }

    #[test]
    fn sidebar_switch_edge_slides_the_way_of_the_click() {
        let by = |play: f32, rightward: bool| match shift(Some(after(play)), rightward) {
            Pose::Shift(by) => by,
            pose => panic!("{pose:?}"),
        };
        // Halfway through 360ms: the whole 1.5 points, right or left.
        assert!(close(by(180.0, true), 1.5));
        assert!(close(by(180.0, false), -1.5));
        let early = by(60.0, true);
        assert!(early > 0.0 && early < 1.5, "{early}");
        assert!(close(by(360.0, false), 0.0));
        assert_eq!(shift(None, true), Pose::Rest);
    }

    #[test]
    fn plus_turns_a_quarter_with_a_bounce() {
        let turn = |play: f32| match spin(Some(after(play))) {
            Pose::Turn(degrees) => degrees,
            pose => panic!("{pose:?}"),
        };
        assert_eq!(turn(0.0), 0.0);
        // Past the quarter on the way, settled on it at 320ms, where it looks as it did.
        assert!(turn(200.0) > 90.0, "{}", turn(200.0));
        assert_eq!(turn(320.0), 90.0);
        assert_eq!(spin(None), Pose::Rest);
    }

    #[test]
    fn split_parts_and_closes() {
        let by = |play: f32| match part(Some(after(play))) {
            Pose::Part(by) => by,
            pose => panic!("{pose:?}"),
        };
        // Each half 0.9 points out halfway through 380ms, closed again at the end.
        assert!(close(by(190.0), 0.9));
        let early = by(50.0);
        assert!(early > 0.0 && early < 0.9, "{early}");
        assert!(close(by(380.0), 0.0));
        assert_eq!(part(None), Pose::Rest);
    }

    #[test]
    fn magnifier_tilts_back_then_forward() {
        let degrees = |play: f32| match tilt(Some(after(play))) {
            Pose::Turn(degrees) => degrees,
            pose => panic!("{pose:?}"),
        };
        // 30% and 65% of 420ms: −14°, then 8°; upright again at the end.
        assert!(close(degrees(126.0), -14.0));
        assert!(close(degrees(273.0), 8.0));
        assert!(close(degrees(420.0), 0.0));
        assert_eq!(tilt(None), Pose::Rest);
    }

    #[test]
    fn pause_bars_dip_together_and_come_back() {
        let by = |play: f32| match dip(Some(after(play))) {
            Pose::Dip(by) => by,
            pose => panic!("{pose:?}"),
        };
        // 40% of 320ms: the whole 1.5 points down, back up by the end.
        assert!(close(by(128.0), 1.5));
        let early = by(40.0);
        assert!(early > 0.0 && early < 1.5, "{early}");
        assert!(close(by(320.0), 0.0));
        assert_eq!(dip(None), Pose::Rest);
        // With Reduce Motion it never plays.
        let start = Instant::now();
        let mut state = State::default();
        state.hover(true, start, true, DIP);
        let (hover, next) = state.at(start + after(100.0), DIP);
        assert_eq!((dip(hover.play), next), (Pose::Rest, Next::Idle));
    }

    #[test]
    fn title_bar_motions_still_with_reduce_motion() {
        let start = Instant::now();
        for timing in [SHIFT, SPIN, PART, TILT] {
            let mut state = State::default();
            state.hover(true, start, true, timing);
            let (hover, next) = state.at(start + after(100.0), timing);
            assert_eq!((hover.play, hover.hovered, next), (None, true, Next::Idle));
            assert_eq!(shift(hover.play, true), Pose::Rest);
            assert_eq!(spin(hover.play), Pose::Rest);
            assert_eq!(part(hover.play), Pose::Rest);
            assert_eq!(tilt(hover.play), Pose::Rest);
        }
    }

    #[test]
    fn plays_once_on_entering_and_again_on_coming_back() {
        let start = Instant::now();
        let mut state = State::default();
        assert_eq!(state.at(start, RING), (Hover::default(), Next::Idle));
        state.hover(true, start, false, RING);
        let (hover, next) = state.at(start + after(100.0), RING);
        assert_eq!((hover.play, next), (Some(after(100.0)), Next::Frame));
        // Leaving lets it finish, then nothing more is drawn.
        state.hover(false, start + after(200.0), false, RING);
        assert_eq!(
            state.at(start + after(300.0), RING).0.play,
            Some(after(300.0))
        );
        assert_eq!(state.at(start + after(600.0), RING).1, Next::Idle);
        // Coming back plays it again from the start.
        state.hover(true, start + after(700.0), false, RING);
        assert_eq!(
            state.at(start + after(710.0), RING).0.play,
            Some(after(10.0))
        );
    }

    #[test]
    fn reduce_motion_never_moves() {
        let start = Instant::now();
        let mut state = State::default();
        state.hover(true, start, true, SLIDE);
        let (hover, next) = state.at(start + after(100.0), SLIDE);
        assert_eq!((hover.play, next), (None, Next::Idle));
        assert_eq!(slide(hover.play), Pose::Rest);
        assert_eq!(press(1.0, true), Pose::Rest);
        // The tick is there whole at once, and only its end is waited for.
        state.clicked = Some(start);
        let (hover, next) = state.at(start + after(10.0), COPY);
        assert_eq!(copy(hover), (Icon::Copied, Pose::Grow(1.0), 1.0));
        assert_eq!(next, Next::At(start + COPY.flash));
    }

    #[test]
    fn activity_cells_pop_in_then_breathe_and_swell() {
        // Not there before its turn; then growing from 0.3 as it fades in, whole at the end.
        assert_eq!(cell_in(None), (0.3, 0.0));
        assert_eq!(cell_in(Some(Duration::ZERO)), (0.3, 0.0));
        let (size, opacity) = cell_in(Some(after(150.0)));
        assert!(
            size > 0.6 && size < 1.0 && opacity > 0.5,
            "{size} {opacity}"
        );
        for done in [CELL_IN, after(2000.0)] {
            let (size, opacity) = cell_in(Some(done));
            assert!(close(size, 1.0) && close(opacity, 1.0), "{size} {opacity}");
        }
        // Today's glow rests at the ends of a breath and is fullest halfway.
        assert_eq!(glow(0.0), 0.0);
        assert_eq!(glow(1.0), 0.0);
        assert!(close(glow(0.5), 1.0));
        assert!(close(glow(0.25), glow(0.75)));
        // A cell that rose swells to 1.3 and settles back.
        assert_eq!(bump(Duration::ZERO), 1.0);
        assert!(close(bump(after(0.35 * 560.0)), 1.3));
        assert_eq!(bump(BUMP), 1.0);
        assert_eq!(bump(after(5000.0)), 1.0);
    }

    #[test]
    fn stop_follows_the_pointer_in_and_out() {
        let start = Instant::now();
        let mut state = State::default();
        state.hover(true, start, false, PRESS);
        assert_eq!(state.at(start, PRESS).0.lit, 0.0);
        let (hover, next) = state.at(start + after(80.0), PRESS);
        assert!(hover.lit > 0.5 && hover.lit < 1.0, "{}", hover.lit);
        assert_eq!(next, Next::Frame);
        let (hover, next) = state.at(start + after(160.0), PRESS);
        assert_eq!((hover.lit, next), (1.0, Next::Idle));
        // Leaving, then coming back halfway out, turns round from where it is.
        state.hover(false, start + after(200.0), false, PRESS);
        state.hover(true, start + after(240.0), false, PRESS);
        let turned = state.at(start + after(240.0), PRESS).0.lit;
        assert!(turned > 0.0 && turned < 1.0, "{turned}");
    }
}
