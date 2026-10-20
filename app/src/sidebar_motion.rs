//! The agent cards in motion (P5-154, the approved demo's 4A and 5A). When the cards' order
//! changes (an agent comes or goes, a new status moves one, the list is sorted the other way),
//! each card that stays slides from where it was to its new place; one that comes fades in where
//! it is laid out, and one that goes fades out where it was as the cards below close up over it.
//! A card that moves for any other reason (one above opened or closed, the list scrolled) is there
//! at once. An agent that stops or comes to need you sends one faint ring out of its status dot.
//! What the list shows when it is drawn open again, or first read, is there as it is. With the
//! system's Reduce Motion on, cards only fade in and out: nothing slides or rings. Nothing asks for
//! frames once all is still.
use super::*;
use crate::view::tween::{POP_CURVE, Tween};
use gpui::{Bounds, Element, GlobalElementId, InspectorElementId, LayoutId, Point, size};
use std::{cell::RefCell, collections::HashSet};

/// A card sliding from its old place to its new one.
const SLIDE: Duration = Duration::from_millis(160);
/// A card coming in or going, faded in or out.
const FADE: Duration = Duration::from_millis(140);
/// The ring a status dot sends out: it grows from the dot to this many times across as it fades
/// from this opaque to nothing.
const RIPPLE: Duration = Duration::from_millis(600);
const RIPPLE_REACH: f32 = 3.0;
const RIPPLE_OPACITY: f32 = 0.55;
/// Nearer than this a card has not moved: what is laid out is snapped to the screen's pixels.
const NEAR: f32 = 0.5;

/// A card's place in the list as last laid out, from the list's own corner, and how far it still
/// has to slide there: `from` its place at the start, `went` of the way.
#[derive(Clone, Copy, Debug)]
struct Place {
    at: Bounds<f32>,
    from: Point<f32>,
    went: Tween,
}

impl Place {
    /// How far from its place it is drawn at `now`.
    fn offset(&self, now: Instant) -> Point<f32> {
        let left = 1.0 - self.went.value(now);
        point(self.from.x * left, self.from.y * left)
    }
}

/// A card whose agent has gone, fading out where it was last drawn.
struct Ghost {
    card: Card,
    at: Bounds<f32>,
    fade: Tween,
}

/// What moves among the cards, from frame to frame.
#[derive(Default)]
pub(super) struct CardMotion {
    /// The agents as last listed, with the status each showed; none until the list is drawn open
    /// with corral read, and again once it is not.
    seen: Option<HashMap<String, Status>>,
    /// The cards drawn last frame, in their order, and whether the list was sorted by name.
    order: Vec<String>,
    by_name: bool,
    drawn: HashMap<String, Card>,
    /// The cards' order changed this frame: the moves laid out now slide.
    armed: bool,
    /// Reduce Motion was on this frame.
    still: bool,
    /// When the frame being drawn is.
    now: Option<Instant>,
    /// Where the list is laid out this frame, which the cards' places are taken from.
    origin: Point<f32>,
    places: HashMap<String, Place>,
    entering: HashMap<String, Tween>,
    leaving: Vec<Ghost>,
    /// The rings on their way, with the status each rang for.
    ripples: HashMap<String, (Status, Tween)>,
}

impl CardMotion {
    /// The list drawn open at `now`: every agent `listed` with the status it shows (none before
    /// corral is read) and the `cards` drawn, in order.
    pub(super) fn frame(
        &mut self,
        listed: Option<Vec<(String, Status)>>,
        cards: &[&Card],
        by_name: bool,
        still: bool,
        now: Instant,
    ) {
        let Some(listed) = listed else {
            self.forget();
            return;
        };
        self.now = Some(now);
        self.still = still;
        let listed: HashMap<String, Status> = listed.into_iter().collect();
        let names: Vec<String> = cards.iter().map(|card| card.name.clone()).collect();
        let mut changed = false;
        if let Some(seen) = self.seen.replace(listed.clone()) {
            for name in listed.keys().filter(|name| !seen.contains_key(*name)) {
                changed = true;
                let mut fade = Tween::at(0.0);
                fade.go(1.0, now, FADE, POP_CURVE);
                self.entering.insert(name.clone(), fade);
            }
            for name in seen.keys().filter(|name| !listed.contains_key(*name)) {
                changed = true;
                // Fading out where it was drawn last, on its way or not.
                if let (Some(card), Some(place)) = (self.drawn.remove(name), self.places.get(name))
                {
                    let offset = place.offset(now);
                    let origin = point(place.at.origin.x + offset.x, place.at.origin.y + offset.y);
                    let mut fade = Tween::at(1.0);
                    fade.go(0.0, now, FADE, POP_CURVE);
                    self.leaving.push(Ghost {
                        card,
                        at: Bounds::new(origin, place.at.size),
                        fade,
                    });
                }
            }
            for (name, status) in &listed {
                let rings = matches!(status, Status::Idle | Status::Waiting);
                if !still && rings && seen.get(name).is_some_and(|was| was != status) {
                    let mut ring = Tween::at(0.0);
                    ring.go(1.0, now, RIPPLE, POP_CURVE);
                    self.ripples.insert(name.clone(), (*status, ring));
                }
            }
            // The cards drawn both times, in the order they were and are.
            let kept = |order: &[String], other: &[String]| -> Vec<String> {
                let other: HashSet<&String> = other.iter().collect();
                order
                    .iter()
                    .filter(|name| other.contains(name))
                    .cloned()
                    .collect()
            };
            changed |=
                by_name != self.by_name || kept(&self.order, &names) != kept(&names, &self.order);
        }
        self.armed = changed && !still;
        self.entering.retain(|_, fade| fade.moving(now));
        self.leaving
            .retain(|ghost| ghost.fade.moving(now) && !listed.contains_key(&ghost.card.name));
        self.ripples
            .retain(|name, (status, ring)| ring.moving(now) && listed.get(name) == Some(status));
        let drawn: HashSet<&String> = names.iter().collect();
        self.places.retain(|name, _| drawn.contains(name));
        self.drawn = cards
            .iter()
            .map(|&card| (card.name.clone(), card.clone()))
            .collect();
        self.order = names;
        self.by_name = by_name;
    }

    /// The list is not drawn open: what it shows next time is there at once.
    pub(super) fn forget(&mut self) {
        *self = Self::default();
    }

    /// When the frame being drawn is.
    fn now(&self) -> Instant {
        self.now.unwrap_or_else(Instant::now)
    }

    /// How far the card of `name` shows as it comes in, 0 to 1.
    pub(super) fn shown(&self, name: &str) -> f32 {
        self.entering
            .get(name)
            .map_or(1.0, |fade| fade.value(self.now()))
    }

    /// How far the ring of `name`'s status dot has gone out, 0 to 1, while it does.
    pub(super) fn ripple(&self, name: &str) -> Option<f32> {
        let now = self.now();
        self.ripples
            .get(name)
            .filter(|(_, ring)| ring.moving(now))
            .map(|(_, ring)| ring.value(now))
    }

    /// The cards fading out: each, where it was and how far it still shows.
    pub(super) fn leaving(&self) -> Vec<(&Card, Bounds<f32>, f32)> {
        let now = self.now();
        self.leaving
            .iter()
            .filter(|ghost| ghost.fade.moving(now))
            .map(|ghost| (&ghost.card, ghost.at, ghost.fade.value(now)))
            .collect()
    }

    /// A card comes in or goes, or a ring goes out, at `now`: the list is drawn again next frame.
    pub(super) fn fading(&self, now: Instant) -> bool {
        self.entering.values().any(|fade| fade.moving(now))
            || self.leaving.iter().any(|ghost| ghost.fade.moving(now))
            || self.ripples.values().any(|(_, ring)| ring.moving(now))
    }

    /// The list is laid out at `origin` this frame.
    fn set_origin(&mut self, origin: Point<f32>) {
        self.origin = origin;
    }

    /// The card of `name` is laid out `at` this frame: how far from there it is drawn, and whether
    /// it still slides. Moved as the cards' order changed, it slides from where it is drawn.
    fn place(&mut self, name: &str, at: Bounds<f32>) -> (Point<f32>, bool) {
        let now = self.now();
        let origin = point(at.origin.x - self.origin.x, at.origin.y - self.origin.y);
        let at = Bounds::new(origin, at.size);
        let (armed, still) = (self.armed, self.still);
        let place = self.places.entry(name.to_owned()).or_insert(Place {
            at,
            from: point(0.0, 0.0),
            went: Tween::at(1.0),
        });
        let (x, y) = (
            place.at.origin.x - at.origin.x,
            place.at.origin.y - at.origin.y,
        );
        if still {
            place.went.jump(1.0);
        } else if armed && (x.abs() > NEAR || y.abs() > NEAR) {
            let offset = place.offset(now);
            place.from = point(offset.x + x, offset.y + y);
            place.went = Tween::at(0.0);
            place.went.go(1.0, now, SLIDE, POP_CURVE);
        }
        place.at = at;
        (place.offset(now), place.went.moving(now))
    }
}

impl Sidebar {
    /// The cards whose agents have gone, each fading out where it was drawn last, over the list
    /// (`stale`, dimmed) and doing nothing when clicked.
    pub(super) fn leaving_cards(
        &self,
        stale: bool,
        grounds: Grounds,
        in_well: Grounds,
    ) -> Vec<AnyElement> {
        let leaving: Vec<(Card, Bounds<f32>, f32)> = (self.cards.borrow().leaving().into_iter())
            .map(|(card, at, shown)| (card.clone(), at, shown))
            .collect();
        let dim = if stale { STALE } else { 1.0 };
        let none = || -> OnClick { Box::new(|_, _, _| {}) };
        (leaving.into_iter())
            .map(|(card, at, shown)| {
                let actions = CardActions {
                    reply: none(),
                    copy: None,
                    pause: none(),
                    confirm: none(),
                    cancel: none(),
                    leave: Box::new(|_, _, _| {}),
                    stop: none(),
                    details: none(),
                };
                let grounds = if card.workspace.is_some() {
                    in_well
                } else {
                    grounds
                };
                div()
                    .absolute()
                    .left(px(at.origin.x))
                    .top(px(at.origin.y))
                    .w(px(at.size.width))
                    .opacity(dim * shown)
                    .child(AgentCard {
                        card,
                        theme: self.theme.clone(),
                        mono: self.mono.clone(),
                        on_click: none(),
                        actions,
                        workspace: None,
                        sandbox: None,
                        delegates: None,
                        grounds,
                        asking: false,
                        ripple: None,
                    })
                    .into_any_element()
            })
            .collect()
    }
}

/// The ring a status dot `dot` points across sends out in `color`, `went` of the way.
pub(super) fn ripple(color: Hsla, dot: f32, went: f32) -> BoxShadow {
    BoxShadow {
        color: color.opacity(RIPPLE_OPACITY * (1.0 - went)),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(dot * (RIPPLE_REACH - 1.0) / 2.0 * went),
        inset: false,
    }
}

fn layout_place(bounds: Bounds<Pixels>, window: &Window) -> Bounds<f32> {
    // What is laid out is drawn as far over as the elements around it are: the list's scroll.
    let over = window.pixel_snap_point(window.element_offset());
    Bounds::new(
        point(
            f32::from(bounds.origin.x - over.x),
            f32::from(bounds.origin.y - over.y),
        ),
        size(f32::from(bounds.size.width), f32::from(bounds.size.height)),
    )
}

/// The list of cards, noting where it is laid out for the cards in it.
pub(super) struct Anchor {
    pub motion: Rc<RefCell<CardMotion>>,
    pub list: AnyElement,
}

/// A card in the list, drawn as far from where it is laid out as it still has to slide.
pub(super) struct Slot {
    pub motion: Rc<RefCell<CardMotion>>,
    pub name: String,
    pub card: AnyElement,
}

impl IntoElement for Anchor {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl IntoElement for Slot {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Anchor {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.list.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let origin = layout_place(bounds, window).origin;
        self.motion.borrow_mut().set_origin(origin);
        self.list.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.list.paint(window, cx);
    }
}

impl Element for Slot {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.card.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let at = layout_place(bounds, window);
        let (offset, sliding) = self.motion.borrow_mut().place(&self.name, at);
        if sliding {
            window.request_animation_frame();
        }
        window.with_element_offset(point(px(offset.x), px(offset.y)), |window| {
            self.card.prepaint(window, cx)
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.card.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            cwd: Some(format!("/work/{name}")),
            ..Agent::default()
        }
    }

    /// The agents as corral lists them, and the cards the list draws for them.
    struct List {
        listing: Listing,
        by_name: bool,
    }

    impl List {
        fn new() -> Self {
            Self {
                listing: Listing::default(),
                by_name: false,
            }
        }

        fn read(&mut self, agents: &[(&str, &str)]) {
            let agents = agents.iter().map(|(name, state)| agent(name, state));
            let _ = self.listing.absorb(Ok(agents.collect()), None, 1000.0);
            self.listing.panel.by_name = self.by_name;
        }

        /// Drawn at `now`: the cards' names in order.
        fn draw(&self, motion: &mut CardMotion, still: bool, now: Instant) -> Vec<String> {
            let lines = self.listing.lines(None, &[], None, 1000.0);
            let cards: Vec<&Card> = lines
                .iter()
                .filter_map(|line| match line {
                    Line::Agent(card) => Some(&**card),
                    _ => None,
                })
                .collect();
            let panel = &self.listing.panel;
            let listed = panel
                .agents
                .iter()
                .map(|a| (a.name.clone(), panel.shown(a, 1000.0)))
                .collect();
            motion.frame(Some(listed), &cards, self.by_name, still, now);
            cards.iter().map(|card| card.name.clone()).collect()
        }
    }

    /// The cards laid out one under another, `tall` points each, the list at `origin`.
    fn lay_out(
        motion: &mut CardMotion,
        names: &[String],
        origin: Point<f32>,
        tall: f32,
    ) -> Vec<(Point<f32>, bool)> {
        motion.set_origin(origin);
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let at = Bounds::new(
                    point(origin.x + 8.0, origin.y + i as f32 * tall),
                    size(200.0, tall - 4.0),
                );
                motion.place(name, at)
            })
            .collect()
    }

    const TALL: f32 = 60.0;

    /// Anything moves at `now`: a card slides or fades, or a ring goes out.
    fn moving(motion: &CardMotion, now: Instant) -> bool {
        motion.fading(now) || sliding(motion, now)
    }

    /// A card slides at `now`.
    fn sliding(motion: &CardMotion, now: Instant) -> bool {
        motion.places.values().any(|place| place.went.moving(now))
    }

    fn zero() -> Point<f32> {
        point(0.0, 0.0)
    }

    #[test]
    fn a_card_moved_by_a_new_status_slides_from_its_old_place_and_on_from_where_it_is_if_moved_again()
     {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        // Waiting agents first.
        list.read(&[("p/a", "working"), ("p/b", "working"), ("p/c", "working")]);
        let names = list.draw(&mut motion, false, start);
        assert_eq!(names, ["p/a", "p/b", "p/c"]);
        lay_out(&mut motion, &names, zero(), TALL);
        // `p/c` comes to need you: it goes to the top, the others down one.
        let moved = start + ms(1000);
        list.read(&[("p/a", "working"), ("p/b", "working"), ("p/c", "blocked")]);
        let names = list.draw(&mut motion, false, moved);
        assert_eq!(names, ["p/c", "p/a", "p/b"]);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        // Drawn where they were, and on their way.
        assert_eq!(laid[0], (point(0.0, 2.0 * TALL), true));
        assert_eq!(laid[1], (point(0.0, -TALL), true));
        assert!(moving(&motion, moved));
        // Partway there along the curve, there in 160 ms, and then still.
        let middle = moved + ms(80);
        let off = motion.places["p/c"].offset(middle).y;
        assert!(close(off, 2.0 * TALL * (1.0 - POP_CURVE.at(0.5))), "{off}");
        assert!(sliding(&motion, moved + ms(159)));
        assert_eq!(motion.places["p/c"].offset(moved + SLIDE), zero());
        assert!(!sliding(&motion, moved + SLIDE));
        // Moved again halfway: it goes on from where it is drawn, the whole slide anew.
        let again = moved + ms(80);
        let was = 2.0 * TALL * (1.0 - POP_CURVE.at(0.5));
        list.read(&[("p/a", "blocked"), ("p/b", "working"), ("p/c", "blocked")]);
        let names = list.draw(&mut motion, false, again);
        assert_eq!(names, ["p/a", "p/c", "p/b"]);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        // `p/c` laid out one lower than it was, drawn where it was on its way.
        assert!(close(laid[1].0.y, was - TALL), "{:?}", laid[1]);
        assert!(sliding(&motion, again + SLIDE - ms(1)));
        assert!(!sliding(&motion, again + SLIDE));
    }

    #[test]
    fn a_card_moved_without_the_order_changing_is_there_at_once() {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        list.read(&[("p/a", "working"), ("p/b", "idle")]);
        let names = list.draw(&mut motion, false, start);
        lay_out(&mut motion, &names, zero(), TALL);
        // The first card opened, the list scrolled or laid out elsewhere: no slide.
        let later = start + ms(500);
        list.draw(&mut motion, false, later);
        let laid = lay_out(&mut motion, &names, point(30.0, 90.0), TALL * 2.0);
        assert!(
            laid.iter()
                .all(|&(offset, sliding)| offset == zero() && !sliding)
        );
        assert!(!moving(&motion, later));
    }

    #[test]
    fn sorted_the_other_way_the_cards_slide() {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        list.read(&[("p/b", "blocked"), ("p/a", "idle")]);
        let names = list.draw(&mut motion, false, start);
        assert_eq!(names, ["p/b", "p/a"]);
        lay_out(&mut motion, &names, zero(), TALL);
        list.by_name = true;
        list.read(&[("p/b", "blocked"), ("p/a", "idle")]);
        let later = start + ms(500);
        let names = list.draw(&mut motion, false, later);
        assert_eq!(names, ["p/a", "p/b"]);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        assert_eq!(laid[0], (point(0.0, TALL), true));
        assert_eq!(laid[1], (point(0.0, -TALL), true));
    }

    #[test]
    fn a_card_comes_in_fading_where_it_is_and_one_going_fades_out_where_it_was() {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        list.read(&[("p/a", "working"), ("p/b", "working")]);
        let names = list.draw(&mut motion, false, start);
        // As first drawn: all there.
        assert_eq!(motion.shown("p/a"), 1.0);
        assert!(!motion.fading(start));
        lay_out(&mut motion, &names, zero(), TALL);
        // `p/0` comes, above them: it fades in where it is laid out; they slide down.
        let came = start + ms(500);
        list.read(&[("p/0", "working"), ("p/a", "working"), ("p/b", "working")]);
        let names = list.draw(&mut motion, false, came);
        assert_eq!(names, ["p/0", "p/a", "p/b"]);
        assert_eq!(motion.shown("p/0"), 0.0);
        assert_eq!(motion.shown("p/a"), 1.0);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        assert_eq!(laid[0], (zero(), false));
        assert_eq!(laid[1], (point(0.0, -TALL), true));
        list.draw(&mut motion, false, came + ms(70));
        let half = motion.shown("p/0");
        assert!(close(half, POP_CURVE.at(0.5)), "{half}");
        assert!(motion.fading(came + ms(70)));
        list.draw(&mut motion, false, came + FADE);
        assert_eq!(motion.shown("p/0"), 1.0);
        assert!(!motion.fading(came + FADE));
        lay_out(&mut motion, &names, zero(), TALL);
        // `p/a` goes: drawn where it was, fading out, as `p/b` slides up over its room.
        let went = came + ms(1000);
        list.read(&[("p/0", "working"), ("p/b", "working")]);
        let names = list.draw(&mut motion, false, went);
        let leaving = motion.leaving();
        assert_eq!(leaving.len(), 1);
        let (card, at, shown) = leaving[0];
        assert_eq!(card.name, "p/a");
        assert_eq!(at.origin, point(8.0, TALL));
        assert_eq!(at.size.width, 200.0);
        assert_eq!(shown, 1.0);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        assert_eq!(laid[1], (point(0.0, TALL), true));
        list.draw(&mut motion, false, went + ms(70));
        let shown = motion.leaving()[0].2;
        assert!(close(shown, 1.0 - POP_CURVE.at(0.5)), "{shown}");
        // Gone: nothing left of it, and nothing more to draw.
        list.draw(&mut motion, false, went + SLIDE);
        assert!(motion.leaving().is_empty());
        lay_out(&mut motion, &names, zero(), TALL);
        assert!(!moving(&motion, went + SLIDE));
    }

    #[test]
    fn a_dot_rings_once_as_its_agent_stops_or_comes_to_need_you_and_not_as_first_read() {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        // First read: no ring, whatever the status.
        list.read(&[("p/a", "working"), ("p/b", "idle"), ("p/c", "blocked")]);
        list.draw(&mut motion, false, start);
        for name in ["p/a", "p/b", "p/c"] {
            assert_eq!(motion.ripple(name), None);
        }
        assert!(!motion.fading(start));
        // `p/a` stops, `p/b` starts working: only `p/a` rings.
        let stopped = start + ms(1000);
        list.read(&[("p/a", "idle"), ("p/b", "working"), ("p/c", "blocked")]);
        list.draw(&mut motion, false, stopped);
        assert_eq!(motion.ripple("p/a"), Some(0.0));
        assert_eq!(motion.ripple("p/b"), None);
        assert_eq!(motion.ripple("p/c"), None);
        assert!(motion.fading(stopped));
        list.draw(&mut motion, false, stopped + ms(300));
        let half = motion.ripple("p/a").unwrap();
        assert!(close(half, POP_CURVE.at(0.5)), "{half}");
        // Once: done in 600 ms, and no more frames for it.
        list.draw(&mut motion, false, stopped + RIPPLE);
        assert_eq!(motion.ripple("p/a"), None);
        assert!(!motion.fading(stopped + RIPPLE));
        list.draw(&mut motion, false, stopped + ms(2000));
        assert_eq!(motion.ripple("p/a"), None);
        // Coming to need you rings too; a status changed meanwhile ends it.
        let asks = stopped + ms(3000);
        list.read(&[("p/a", "idle"), ("p/b", "blocked"), ("p/c", "blocked")]);
        list.draw(&mut motion, false, asks);
        assert_eq!(motion.ripple("p/b"), Some(0.0));
        list.read(&[("p/a", "idle"), ("p/b", "working"), ("p/c", "blocked")]);
        list.draw(&mut motion, false, asks + ms(100));
        assert_eq!(motion.ripple("p/b"), None);
        // Drawn open again: what it shows is there as it is.
        motion.forget();
        list.read(&[("p/a", "blocked"), ("p/b", "idle"), ("p/c", "blocked")]);
        list.draw(&mut motion, false, asks + ms(200));
        assert_eq!(motion.ripple("p/a"), None);
        assert_eq!(motion.ripple("p/b"), None);
    }

    #[test]
    fn with_reduce_motion_cards_only_fade_and_nothing_slides_or_rings() {
        let start = Instant::now();
        let mut motion = CardMotion::default();
        let mut list = List::new();
        list.read(&[("p/a", "working"), ("p/b", "working")]);
        let names = list.draw(&mut motion, true, start);
        lay_out(&mut motion, &names, zero(), TALL);
        let later = start + ms(500);
        list.read(&[("p/0", "working"), ("p/a", "working"), ("p/b", "idle")]);
        let names = list.draw(&mut motion, true, later);
        let laid = lay_out(&mut motion, &names, zero(), TALL);
        assert!(
            laid.iter()
                .all(|&(offset, sliding)| offset == zero() && !sliding)
        );
        assert_eq!(motion.ripple("p/b"), None);
        // Fading in, and out.
        assert_eq!(motion.shown("p/0"), 0.0);
        list.read(&[("p/0", "working"), ("p/b", "idle")]);
        list.draw(&mut motion, true, later + ms(70));
        assert_eq!(motion.leaving().len(), 1);
        assert!(motion.fading(later + ms(70)));
    }
}
