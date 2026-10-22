//! The window's things in motion (P5-148a): popups, menus and dialogs fade in as they rise to their
//! place and fade out quicker where they are; a zoomed pane grows from its place in the split to
//! fill the tab as the others fade, and goes back the same way; the sidebars slide; the active
//! pane's rim slides to the pane that takes the keys as the veils over the others cross-fade; a
//! tab switched to fades in as it moves over from its side, as a popup rises (P5-154, 1A).
//! Timings and curves are the approved demo's (1A, 2A, 3B; the sidebars as 2A). With the system's
//! Reduce Motion on, popups and a tab switched to only fade and the veils still cross-fade; the
//! rest is there at once.
//! While a pane's edges move its terminal keeps its size, fitted once they stop; nothing asks for
//! frames once all is still.
use super::*;
use crate::view::tween::{self, Curve, Tween, lerp};

/// A pane growing to fill its tab or going back, a sidebar sliding (2A).
const MOVE: Duration = Duration::from_millis(180);
const MOVE_CURVE: Curve = Curve(0.2, 0.8, 0.2, 1.0);
/// The rim going from pane to pane, the veils crossing as it goes (3B), eased out as 2A.
const FOCUS: Duration = Duration::from_millis(200);
/// How opaque the sliding rim is: over a pane's own rim it makes the active pane's, as a still
/// one is drawn; between panes, fainter.
pub(super) const SLIDING_RIM: f32 = 1.0 - (1.0 - PANEL_RIM_ACTIVE) / (1.0 - PANEL_RIM);

/// Whether the popup on show stays the one that came in when it turns from `a` to `b`: the split
/// panel turned to another side does.
fn same_popup(a: Popup, b: Popup) -> bool {
    matches!((a, b), (Popup::Split(_), Popup::Split(_))) || a == b
}

/// Whether `popup` dims the window: it rises within the dimming, which stays where it is.
fn dims(popup: Popup) -> bool {
    matches!(
        popup,
        Popup::Palette | Popup::NewTask | Popup::NewAgent | Popup::RemoteAgent
    )
}

/// `from` to `to`, `t` of the way.
fn between(from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    let at = |a: Pixels, b: Pixels| px(lerp(f32::from(a), f32::from(b), t));
    Bounds::new(
        point(
            at(from.origin.x, to.origin.x),
            at(from.origin.y, to.origin.y),
        ),
        size(
            at(from.size.width, to.size.width),
            at(from.size.height, to.size.height),
        ),
    )
}

/// A pane's panel from where it noted itself: its spot lies inside the rim.
pub(super) fn panel_bounds(spot: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(spot.origin.x - px(1.0), spot.origin.y - px(1.0)),
        size(spot.size.width + px(2.0), spot.size.height + px(2.0)),
    )
}

/// What a popup fading out is drawn from, kept until it has gone.
pub(super) struct Leaving {
    popup: Popup,
    /// How far it shows, on its way to nothing, and how far below its place it stays.
    fade: Tween,
    rise: f32,
    palette: Option<Palette>,
    chooser: Chooser,
    inbox: Option<inbox_panel::Open>,
    new_task: Option<Entity<TaskDialog>>,
    new_agent: Option<Entity<NewAgentView>>,
    remote_agent: Option<Entity<RemoteAgentView>>,
    send: Option<send_panel::SendPanel>,
}

/// The tab on show, as far as a pane growing or the rim sliding goes: its machine, where it is
/// among the tabs and its panes.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TabKey {
    seat: usize,
    tab: usize,
    panes: Vec<PaneId>,
}

/// The rim sliding from where it was to the pane with the keys.
#[derive(Clone, Copy, Debug)]
struct Ring {
    from: Bounds<Pixels>,
    to: PaneId,
    went: Tween,
}

/// What moves in the window, from frame to frame.
pub(super) struct Motion {
    /// The popup on show as last drawn, how far it has come in, and whether it only fades.
    popup: Option<(Popup, Tween, bool)>,
    leaving: Option<Leaving>,
    /// The tab as last drawn and the pane zoomed in it.
    zoomed: Option<(TabKey, Option<PaneId>)>,
    /// The pane growing to fill its tab (to 1) or going back (to 0), and its place in the split as
    /// it started to grow, until that place is drawn.
    growing: Option<(PaneId, Tween)>,
    grow_from: Option<Bounds<Pixels>>,
    /// The left sidebar folded to the strip (1) or not, the right one open (1) or not.
    fold: Tween,
    right: Tween,
    /// The tab as last drawn with several panes on show, and the one with the keys.
    active: Option<(TabKey, PaneId)>,
    ring: Option<Ring>,
    /// The panes drawn last frame and how veiled, 0 to 1.
    veils: HashMap<PaneId, Tween>,
    /// The tab as last drawn, by its machine and id; switched to, which side it comes in from (1
    /// the right, -1 the left, 0 neither) and how far it has come in.
    tab: Option<(usize, u64)>,
    switched: Option<(f32, Tween)>,
    /// When the frame being drawn is: all of it is drawn as of then.
    frame: Instant,
}

impl Motion {
    /// Still, the sidebars as the window opens with them.
    pub(super) fn new(folded: bool, right_open: bool) -> Self {
        let one = |on: bool| Tween::at(if on { 1.0 } else { 0.0 });
        Self {
            popup: None,
            leaving: None,
            zoomed: None,
            growing: None,
            grow_from: None,
            fold: one(folded),
            right: one(right_open),
            active: None,
            ring: None,
            veils: HashMap::new(),
            tab: None,
            switched: None,
            frame: Instant::now(),
        }
    }

    /// The popup on show now: one that was not comes in from nothing, or from as far as the same
    /// one fading out had got, which then goes.
    fn popup(&mut self, popup: Option<Popup>, still: bool, now: Instant) {
        match (&mut self.popup, popup) {
            (Some((was, ..)), Some(popup)) if same_popup(*was, popup) => {
                *was = popup;
                return;
            }
            (None, None) => return,
            _ => {}
        }
        self.popup = popup.map(|popup| {
            let back = self
                .leaving
                .take_if(|leaving| same_popup(leaving.popup, popup));
            let mut tween = Tween::at(back.map_or(0.0, |leaving| leaving.fade.value(now)));
            tween.go(1.0, now, tween::POP_IN, tween::POP_CURVE);
            (popup, tween, still)
        });
    }

    /// The popup on show at `now`: how far it shows, 0 to 1, and how far below its place.
    fn popup_look(&self, now: Instant) -> (f32, f32) {
        match &self.popup {
            Some((_, tween, still)) => {
                let shown = tween.value(now);
                let rise = if *still {
                    0.0
                } else {
                    tween::POP_RISE * (1.0 - shown)
                };
                (shown, rise)
            }
            None => (1.0, 0.0),
        }
    }

    /// The popup on show goes, `kept` with what draws it: it fades out from where it is, or, never
    /// drawn, at once.
    fn leave(&mut self, mut kept: Leaving, now: Instant) {
        let (shown, rise) = match &self.popup {
            Some((popup, ..)) if same_popup(*popup, kept.popup) => self.popup_look(now),
            _ => (0.0, 0.0),
        };
        self.popup = None;
        kept.fade = Tween::at(shown);
        kept.fade.go(0.0, now, tween::POP_OUT, tween::POP_CURVE);
        kept.rise = rise;
        self.leaving = (shown > 0.0).then_some(kept);
    }

    /// The popup fading out at `now`, how far it shows and how far below its place; none once it
    /// has gone, which lets it go.
    fn leaving_look(&mut self, now: Instant) -> Option<(f32, f32)> {
        let look = self
            .leaving
            .as_ref()
            .filter(|leaving| leaving.fade.moving(now))
            .map(|leaving| (leaving.fade.value(now), leaving.rise));
        if look.is_none() {
            self.leaving = None;
        }
        look
    }

    /// The sidebars as the window has them: the left one folded to the strip, the right one open.
    /// They slide there, or with `still` are there at once.
    fn sides(&mut self, folded: bool, right_open: bool, still: bool, now: Instant) {
        for (tween, on) in [(&mut self.fold, folded), (&mut self.right, right_open)] {
            let to = if on { 1.0 } else { 0.0 };
            if tween.target() == to {
                continue;
            }
            if still {
                tween.jump(to);
            } else {
                tween.go(to, now, MOVE, MOVE_CURVE);
            }
        }
    }

    /// How far the left sidebar is folded to the strip at `now`, and whether it is on its way.
    fn folded(&self, now: Instant) -> (f32, bool) {
        (self.fold.value(now), self.fold.moving(now))
    }

    /// How far the right sidebar is open at `now`, and whether it is on its way.
    fn opened(&self, now: Instant) -> (f32, bool) {
        (self.right.value(now), self.right.moving(now))
    }

    /// The tab on show and the pane zoomed in it, and where that pane was last drawn (`slot`): a
    /// pane zoomed or let back in the same tab grows or shrinks from where it is, or with `still`
    /// is there at once; anything else is there at once. A pane that starts anew is named, its
    /// place as last drawn there no longer standing.
    fn zoom(
        &mut self,
        tab: TabKey,
        zoomed: Option<PaneId>,
        slot: Option<Bounds<Pixels>>,
        still: bool,
        now: Instant,
    ) -> Option<PaneId> {
        let was = self.zoomed.replace((tab.clone(), zoomed));
        let (was_tab, was_zoomed) = was?;
        if was_zoomed == zoomed {
            return None;
        }
        let going = |pane: PaneId| {
            self.growing
                .filter(|(growing, tween)| *growing == pane && tween.moving(now))
        };
        let (pane, to, turned) = match (was_zoomed, zoomed) {
            _ if was_tab != tab || still => {
                self.growing = None;
                return None;
            }
            (None, Some(pane)) => (pane, 1.0, going(pane)),
            (Some(pane), None) => (pane, 0.0, going(pane)),
            _ => {
                self.growing = None;
                return None;
            }
        };
        let mut tween = match turned {
            Some((_, tween)) => tween,
            None => Tween::at(1.0 - to),
        };
        tween.go(to, now, MOVE, MOVE_CURVE);
        self.growing = Some((pane, tween));
        if turned.is_some() {
            return None;
        }
        self.grow_from = slot.filter(|_| to == 1.0);
        Some(pane)
    }

    /// The pane growing or going back and how far, 1 filling the tab, while it moves.
    fn growing(&self, now: Instant) -> Option<(PaneId, f32)> {
        self.growing
            .filter(|(_, tween)| tween.moving(now))
            .map(|(pane, tween)| (pane, tween.value(now)))
    }

    /// The pane with the keys in the tab on show, among several (none when the tab shows one):
    /// another pane of the same tab, and the rim slides to it from `from`, unless `still`.
    fn focus(
        &mut self,
        active: Option<(TabKey, PaneId)>,
        from: Option<Bounds<Pixels>>,
        still: bool,
        now: Instant,
    ) {
        let was = std::mem::replace(&mut self.active, active.clone());
        match (was, active) {
            (Some((was_tab, was_pane)), Some((tab, pane))) if was_tab == tab => {
                if was_pane == pane {
                    return;
                }
                self.ring = from.filter(|_| !still).map(|from| {
                    let mut went = Tween::at(0.0);
                    went.go(1.0, now, FOCUS, MOVE_CURVE);
                    Ring {
                        from,
                        to: pane,
                        went,
                    }
                });
            }
            _ => self.ring = None,
        }
    }

    /// The pane that had the keys among several, as last drawn.
    fn last_active(&self) -> Option<PaneId> {
        self.active.as_ref().map(|(_, pane)| *pane)
    }

    /// The rim on its way at `now`: where from, to which pane, how far.
    fn ring(&self, now: Instant) -> Option<(Bounds<Pixels>, PaneId, f32)> {
        self.ring
            .filter(|ring| ring.went.moving(now))
            .map(|ring| (ring.from, ring.to, ring.went.value(now)))
    }

    /// The panes drawn this frame and whether each is veiled: one drawn last frame cross-fades to
    /// it, any other is as it should be at once.
    fn veils(&mut self, panes: &[(PaneId, bool)], now: Instant) {
        let mut veils = HashMap::new();
        for &(pane, veiled) in panes {
            let to = if veiled { 1.0 } else { 0.0 };
            let tween = match self.veils.remove(&pane) {
                Some(mut tween) => {
                    tween.go(to, now, FOCUS, MOVE_CURVE);
                    tween
                }
                None => Tween::at(to),
            };
            veils.insert(pane, tween);
        }
        self.veils = veils;
    }

    /// How veiled `pane` is at `now`, 0 to 1.
    fn veil(&self, pane: PaneId, now: Instant) -> f32 {
        self.veils.get(&pane).map_or(0.0, |tween| tween.value(now))
    }

    /// The tab on show: its machine, the tabs' ids in order and where it is among them. Switched
    /// to from another tab of the same machine still open, it comes in from that tab's side, or
    /// with `still` only fades in; anything else is there at once.
    fn switch(&mut self, seat: usize, tabs: &[u64], at: usize, still: bool, now: Instant) {
        let Some(&id) = tabs.get(at) else {
            return;
        };
        let was = self.tab.replace((seat, id));
        if was == Some((seat, id)) {
            return;
        }
        let from = was
            .filter(|(was_seat, _)| *was_seat == seat)
            .and_then(|(_, was_id)| tabs.iter().position(|&tab| tab == was_id));
        self.switched = from.map(|from| {
            let side = if at > from { 1.0 } else { -1.0 };
            let mut went = Tween::at(0.0);
            went.go(1.0, now, tween::POP_IN, tween::POP_CURVE);
            (if still { 0.0 } else { side }, went)
        });
    }

    /// The tab on show at `now`: how far it shows, 0 to 1, and how far right of its place.
    fn tab_look(&self, now: Instant) -> (f32, f32) {
        match self.switched {
            Some((side, went)) => {
                let shown = went.value(now);
                (shown, side * tween::POP_RISE * (1.0 - shown))
            }
            None => (1.0, 0.0),
        }
    }

    /// The panes' edges move at `now`: the terminals keep their size until they stop.
    fn holding(&self, now: Instant) -> bool {
        self.fold.moving(now) || self.right.moving(now) || self.growing(now).is_some()
    }

    /// Anything moves at `now`: the window is drawn again next frame.
    fn moving(&self, now: Instant) -> bool {
        self.holding(now)
            || self
                .popup
                .as_ref()
                .is_some_and(|(_, tween, _)| tween.moving(now))
            || self
                .leaving
                .as_ref()
                .is_some_and(|leaving| leaving.fade.moving(now))
            || self.ring(now).is_some()
            || self.veils.values().any(|tween| tween.moving(now))
            || self.switched.is_some_and(|(_, went)| went.moving(now))
    }
}

/// `element`, a popup drawn over the window, `shown` from 0 to 1 and `rise` points below its place.
fn lift(element: AnyElement, id: &'static str, shown: f32, rise: f32) -> AnyElement {
    div()
        .id(id)
        .absolute()
        .top(px(rise))
        .left_0()
        .size_full()
        .opacity(shown)
        .child(element)
        .into_any_element()
}

impl PaddockWindow {
    /// The popup on show closes or gives way to `next`: it is kept, with what draws it, to fade
    /// out. Called before what draws it is let go.
    pub(super) fn let_go(&mut self, next: Option<Popup>) {
        let Some(popup) = self.popup else {
            return;
        };
        if next.is_some_and(|next| same_popup(popup, next)) {
            return;
        }
        let kept = Leaving {
            popup,
            fade: Tween::at(0.0),
            rise: 0.0,
            palette: self.palette.take(),
            chooser: std::mem::take(&mut self.chooser),
            inbox: None,
            new_task: self.new_task.clone(),
            new_agent: self.new_agent.clone(),
            remote_agent: self.remote_agent.clone(),
            send: self.send.take(),
        };
        self.motion.leave(kept, Instant::now());
    }

    /// The Inbox's panel let go: kept to fade out if the Inbox is the popup fading.
    pub(super) fn keep_leaving_inbox(&mut self, open: inbox_panel::Open) {
        if let Some(leaving) = &mut self.motion.leaving
            && leaving.popup == Popup::Attention
            && leaving.inbox.is_none()
        {
            leaving.inbox = Some(open);
        }
    }

    /// What is drawn this frame and what moves: the popup on show, the sidebars, a pane zoomed,
    /// the pane with the keys and the veils, each from where it was; the terminals told whether
    /// to keep their size; the next frame asked for while anything moves.
    pub(super) fn move_on(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.motion.frame = now;
        let still = crate::reduce_motion::on();
        self.motion.popup(self.popup, still, now);
        self.motion
            .sides(self.collapsed, self.right.open, still, now);
        let ids: Vec<u64> = self.workspace.tabs.iter().map(|tab| tab.id).collect();
        self.motion.switch(
            self.seats.here(),
            &ids,
            self.workspace.active_tab,
            still,
            now,
        );
        let tab = self.workspace.tab();
        let key = TabKey {
            seat: self.seats.here(),
            tab: self.workspace.active_tab,
            panes: tab.panes(),
        };
        let zoomed = self.workspace.zoomed();
        let spot = |spot: Spot| self.spots.borrow().get(&spot).copied();
        let slot = zoomed.and_then(|pane| spot(Spot::Pane(pane)).map(panel_bounds));
        if let Some(pane) = self.motion.zoom(key.clone(), zoomed, slot, still, now) {
            self.spots.borrow_mut().remove(&Spot::Slot(pane));
        }
        let growing = self.motion.growing(now);
        let active = self.workspace.active_pane();
        let several = zoomed.is_none() && growing.is_none() && key.panes.len() > 1;
        let from = self.sliding_rim(now).or_else(|| {
            self.motion
                .last_active()
                .and_then(|pane| spot(Spot::Pane(pane)))
                .map(panel_bounds)
        });
        self.motion
            .focus(several.then(|| (key.clone(), active)), from, still, now);
        let drawn = match (zoomed, growing) {
            (Some(pane), None) => vec![pane],
            _ => key.panes.clone(),
        };
        let shown = drawn.len();
        let veiled: Vec<_> = drawn
            .iter()
            .map(|&pane| (pane, dimmed(shown, pane == active)))
            .collect();
        self.motion.veils(&veiled, now);
        let hold = self.motion.holding(now);
        for view in self.panes.values() {
            view.update(cx, |view, _| view.hold_size(hold));
        }
        if self.motion.moving(now) {
            window.request_animation_frame();
        }
    }

    /// How far the left sidebar is folded this frame, and whether it is on its way.
    pub(super) fn sidebar_fold(&self) -> (f32, bool) {
        self.motion.folded(self.motion.frame)
    }

    /// How far the right sidebar is open this frame, and whether it is on its way.
    pub(super) fn right_slide(&self) -> (f32, bool) {
        self.motion.opened(self.motion.frame)
    }

    /// The pane growing to fill its tab or going back this frame, and how far, while it moves.
    pub(super) fn growing(&self) -> Option<(PaneId, f32)> {
        self.motion.growing(self.motion.frame)
    }

    /// The sidebar's width as drawn this frame, the strip's when folded.
    pub(super) fn sidebar_now(&self, ui: &UiFont) -> f32 {
        lerp(
            self.sidebar_width,
            ui.scale(sidebar::RAIL),
            self.sidebar_fold().0,
        )
    }

    /// The title bar's left part as wide as it is drawn this frame (see [`bar_left`]).
    pub(super) fn bar_left_now(&self, full_screen: bool, ui: &UiFont) -> f32 {
        lerp(
            bar_left(false, full_screen, self.sidebar_width, ui),
            bar_left(true, full_screen, self.sidebar_width, ui),
            self.sidebar_fold().0,
        )
    }

    /// The sidebar's column as drawn this frame (see [`sidebar_column`]).
    pub(super) fn sidebar_column_now(
        &self,
        bar_height: f32,
        window: Size<Pixels>,
        ui: &UiFont,
    ) -> Bounds<Pixels> {
        let column =
            |collapsed| sidebar_column(collapsed, self.sidebar_width, bar_height, window, ui);
        between(column(false), column(true), self.sidebar_fold().0)
    }

    /// A pane's rim: as bright as the active pane's among several, unless the rim slides; the
    /// pane growing goes from what it had in the split to what it has alone.
    pub(super) fn pane_rim(&self, pane: PaneId, shown: usize, active: bool) -> f32 {
        let now = self.motion.frame;
        let split = if shown > 1 && active && self.motion.ring(now).is_none() {
            PANEL_RIM_ACTIVE
        } else {
            PANEL_RIM
        };
        match self.motion.growing(now) {
            Some((growing, grown)) if growing == pane => lerp(split, PANEL_RIM, grown),
            _ => split,
        }
    }

    /// How opaque a pane is drawn: the others fade as one grows to fill the tab.
    pub(super) fn pane_shown(&self, pane: PaneId) -> f32 {
        match self.growing() {
            Some((growing, grown)) if growing != pane => 1.0 - grown,
            _ => 1.0,
        }
    }

    /// The tab switched to this frame (P5-154): how far it shows, 0 to 1, and how far right of
    /// its place.
    pub(super) fn tab_look(&self) -> (f32, f32) {
        self.motion.tab_look(self.motion.frame)
    }

    /// How veiled a pane is, 0 to 1.
    pub(super) fn pane_veil(&self, pane: PaneId) -> f32 {
        self.motion.veil(pane, self.motion.frame)
    }

    /// Where the sliding rim is at `now`, while it slides.
    fn sliding_rim(&self, now: Instant) -> Option<Bounds<Pixels>> {
        let (from, to, went) = self.motion.ring(now)?;
        let to = panel_bounds(self.spots.borrow().get(&Spot::Pane(to)).copied()?);
        Some(between(from, to, went))
    }

    /// Where the sliding rim is this frame, while it slides.
    pub(super) fn sliding_now(&self) -> Option<Bounds<Pixels>> {
        self.sliding_rim(self.motion.frame)
    }

    /// The active pane's rim on its way from the pane that had the keys, over the panes (3B); left
    /// open, as the panel's own, where the active tab joins a panel (P5-165).
    pub(super) fn rim_layer(&self) -> Option<impl IntoElement> {
        let at = self.sliding_now()?;
        let color = self.fg(|t| t.agents_text).opacity(SLIDING_RIM);
        let joint = self.joint.clone();
        Some(
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    join::paint_rim(at, color, joint.get().map(|j| j.shape), window)
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }

    /// The pane growing or going back, over the tab where `full` is laid out, from its place in
    /// the split towards the whole of it.
    pub(super) fn growing_pane(
        &self,
        pane: PaneId,
        grown: f32,
        shown: usize,
        agents: &[Agent],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let spots = self.spots.borrow();
        let full = spots.get(&Spot::Tab).copied();
        let slot = spots
            .get(&Spot::Slot(pane))
            .copied()
            .or(self.motion.grow_from);
        drop(spots);
        let place = div().absolute().flex();
        let place = match (full, slot) {
            (Some(full), Some(slot)) => {
                let at = between(slot, full, grown);
                place
                    .left(at.origin.x - full.origin.x)
                    .top(at.origin.y - full.origin.y)
                    .w(at.size.width)
                    .h(at.size.height)
            }
            _ => place.inset_0(),
        };
        place.child(self.pane(pane, shown, agents, window, cx))
    }

    /// The popup on show, fading and rising in as it comes.
    pub(super) fn popup_layer(
        &mut self,
        hidden: &[usize],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let popup = self.popup?;
        let (shown, rise) = self.motion.popup_look(self.motion.frame);
        let (inside, outside) = if dims(popup) {
            (rise, 0.0)
        } else {
            (0.0, rise)
        };
        let dialog = self.dialog(hidden, inside, window, cx)?;
        Some(lift(dialog, "popup", shown, outside))
    }

    /// The popup fading out, drawn as it was from what was kept of it, under the one on show.
    pub(super) fn leaving_layer(
        &mut self,
        full_screen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (shown, rise) = self.motion.leaving_look(self.motion.frame)?;
        let mut kept = self.motion.leaving.take()?;
        let popup = self.popup.replace(kept.popup);
        self.swap_kept(&mut kept);
        let hidden = match kept.popup {
            Popup::Overflow => self.fit_tabs(full_screen, window, cx).hidden,
            _ => Vec::new(),
        };
        let (inside, outside) = if dims(kept.popup) {
            (rise, 0.0)
        } else {
            (0.0, rise)
        };
        // The `+N` menu has nothing left to list once every tab has room.
        let dialog = if kept.popup == Popup::Overflow && hidden.is_empty() {
            None
        } else {
            self.dialog(&hidden, inside, window, cx)
        };
        self.swap_kept(&mut kept);
        self.popup = popup;
        self.motion.leaving = Some(kept);
        Some(lift(dialog?, "leaving-popup", shown, outside))
    }

    /// What draws the popup put in place of what `kept` has, and back.
    fn swap_kept(&mut self, kept: &mut Leaving) {
        std::mem::swap(&mut self.palette, &mut kept.palette);
        std::mem::swap(&mut self.chooser, &mut kept.chooser);
        self.swap_inbox(&mut kept.inbox);
        std::mem::swap(&mut self.new_task, &mut kept.new_task);
        std::mem::swap(&mut self.new_agent, &mut kept.new_agent);
        std::mem::swap(&mut self.remote_agent, &mut kept.remote_agent);
        std::mem::swap(&mut self.send, &mut kept.send);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn kept(popup: Popup) -> Leaving {
        Leaving {
            popup,
            fade: Tween::at(0.0),
            rise: 0.0,
            palette: None,
            chooser: Chooser::default(),
            inbox: None,
            new_task: None,
            new_agent: None,
            remote_agent: None,
            send: None,
        }
    }

    fn tab(panes: &[PaneId]) -> TabKey {
        TabKey {
            seat: 0,
            tab: 0,
            panes: panes.to_vec(),
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn a_popup_rises_in_then_fades_out_quicker_where_it_is() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.popup(Some(Popup::Palette), false, start);
        // Coming in: from nothing, 4 points low.
        assert_eq!(motion.popup_look(start), (0.0, tween::POP_RISE));
        let (shown, rise) = motion.popup_look(start + ms(40));
        assert!(shown > 0.0 && shown < 1.0 && rise > 0.0 && rise < tween::POP_RISE);
        assert!(motion.moving(start + ms(40)));
        // In place: still, and no frames.
        assert_eq!(motion.popup_look(start + ms(120)), (1.0, 0.0));
        assert!(!motion.moving(start + ms(120)));
        // Closed: it fades out where it is, quicker than it came.
        let shut = start + ms(500);
        motion.leave(kept(Popup::Palette), shut);
        motion.popup(None, false, shut);
        assert_eq!(motion.leaving_look(shut), Some((1.0, 0.0)));
        let (fading, rise) = motion.leaving_look(shut + ms(42)).unwrap();
        assert!(fading > 0.0 && fading < 1.0 && rise == 0.0);
        assert!(motion.moving(shut + ms(42)));
        assert_eq!(motion.leaving_look(shut + ms(84)), None);
        assert!(motion.leaving.is_none());
        assert!(!motion.moving(shut + ms(84)));
    }

    #[test]
    fn a_popup_closed_on_its_way_in_fades_from_there_and_one_opened_again_comes_back_from_there() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.popup(Some(Popup::Attention), false, start);
        let turn = start + ms(30);
        let (shown, rise) = motion.popup_look(turn);
        motion.leave(kept(Popup::Attention), turn);
        motion.popup(None, false, turn);
        assert_eq!(motion.leaving_look(turn), Some((shown, rise)));
        // It takes the part of the fade that part of the way takes.
        let gone = tween::POP_OUT.mul_f64(f64::from(shown));
        assert!(motion.leaving_look(turn + gone - ms(1)).is_some());
        // Opened again meanwhile: the one fading goes, the new one comes in from as far as it was.
        let again = turn + ms(10);
        let fading = motion.leaving_look(again).unwrap().0;
        motion.popup(Some(Popup::Attention), false, again);
        assert!(motion.leaving.is_none());
        assert!(close(motion.popup_look(again).0, fading));
    }

    #[test]
    fn a_popup_closed_before_it_was_drawn_goes_at_once_and_another_takes_its_place_from_nothing() {
        let now = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.leave(kept(Popup::NewTask), now);
        assert!(motion.leaving.is_none());
        motion.popup(Some(Popup::Split(Direction::Right)), false, now);
        motion.popup(Some(Popup::Split(Direction::Down)), false, now + ms(60));
        // Turned to another side, it is the same panel: it keeps on coming in.
        assert!(motion.popup_look(now + ms(60)).0 > 0.0);
    }

    #[test]
    fn with_reduce_motion_a_popup_only_fades_and_the_rest_is_there_at_once() {
        let now = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.popup(Some(Popup::NewAgent), true, now);
        let (shown, rise) = motion.popup_look(now + ms(40));
        assert!(shown > 0.0 && shown < 1.0);
        assert_eq!(rise, 0.0);
        motion.leave(kept(Popup::NewAgent), now + ms(200));
        assert_eq!(
            motion.leaving_look(now + ms(200)).map(|look| look.1),
            Some(0.0)
        );
        // The sidebars, a pane zoomed and the rim go straight there.
        motion.sides(true, true, true, now);
        assert_eq!(motion.folded(now), (1.0, false));
        assert_eq!(motion.opened(now), (1.0, false));
        let panes = tab(&[1, 2]);
        motion.zoom(panes.clone(), None, None, true, now);
        motion.zoom(
            panes.clone(),
            Some(1),
            Some(rect(0., 0., 100., 100.)),
            true,
            now,
        );
        assert_eq!(motion.growing(now), None);
        assert!(!motion.holding(now));
        motion.focus(Some((panes.clone(), 1)), None, true, now);
        motion.focus(Some((panes, 2)), Some(rect(0., 0., 100., 100.)), true, now);
        assert_eq!(motion.ring(now), None);
        // The veils still cross-fade.
        motion.veils(&[(1, false), (2, true)], now);
        motion.veils(&[(1, true), (2, false)], now);
        assert!(motion.veil(1, now + ms(100)) > 0.0 && motion.veil(1, now + ms(100)) < 1.0);
    }

    #[test]
    fn the_sidebars_slide_and_the_terminals_keep_their_size_only_meanwhile() {
        let start = Instant::now();
        let mut motion = Motion::new(false, true);
        assert_eq!(motion.folded(start), (0.0, false));
        assert_eq!(motion.opened(start), (1.0, false));
        motion.sides(true, false, false, start);
        let (folded, folding) = motion.folded(start + ms(90));
        assert!(folding && folded > 0.5 && folded < 1.0, "{folded}");
        let (open, opening) = motion.opened(start + ms(90));
        assert!(opening && open > 0.0 && open < 0.5, "{open}");
        assert!(motion.holding(start + ms(90)));
        assert!(motion.moving(start + ms(90)));
        // Turned round halfway: back from there.
        let turn = start + ms(90);
        motion.sides(false, false, false, turn);
        assert!(close(motion.folded(turn).0, folded));
        assert!(motion.folded(turn + ms(60)).0 < folded);
        // Still: the terminals fit the panes again, and no frames are asked for.
        let later = start + ms(400);
        assert_eq!(motion.folded(later), (0.0, false));
        assert_eq!(motion.opened(later), (0.0, false));
        assert!(!motion.holding(later));
        assert!(!motion.moving(later));
    }

    #[test]
    fn a_zoomed_pane_grows_from_its_place_and_goes_back_from_where_it_is() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        let panes = tab(&[1, 2]);
        let place = rect(10., 10., 100., 80.);
        assert_eq!(motion.zoom(panes.clone(), None, None, false, start), None);
        // Zoomed: it grows from where it was, which is noted as starting anew.
        assert_eq!(
            motion.zoom(panes.clone(), Some(1), Some(place), false, start),
            Some(1)
        );
        assert_eq!(motion.grow_from, Some(place));
        assert_eq!(motion.growing(start), Some((1, 0.0)));
        let turn = start + ms(60);
        let (_, grown) = motion.growing(turn).unwrap();
        assert!(grown > 0.0 && grown < 1.0);
        assert!(motion.holding(turn));
        // Let back halfway: from there, not anew.
        assert_eq!(motion.zoom(panes.clone(), None, None, false, turn), None);
        assert!(close(motion.growing(turn).unwrap().1, grown));
        assert_eq!(motion.grow_from, Some(place));
        let back = turn + MOVE.mul_f64(f64::from(grown));
        assert_eq!(motion.growing(back), None);
        assert!(!motion.holding(back));
        assert!(!motion.moving(back));
        // Zoomed and let back at rest: it shrinks from the whole tab.
        motion.zoom(panes.clone(), Some(1), Some(place), false, back);
        let full = back + ms(200);
        assert_eq!(motion.growing(full), None);
        assert_eq!(motion.zoom(panes.clone(), None, None, false, full), Some(1));
        assert_eq!(motion.growing(full), Some((1, 1.0)));
        assert_eq!(motion.grow_from, None);
        // Another tab, or the panes changed: at once.
        assert_eq!(
            motion.zoom(tab(&[1, 2, 3]), Some(1), None, false, full),
            None
        );
        assert_eq!(motion.growing(full), None);
    }

    #[test]
    fn the_rim_slides_to_the_pane_with_the_keys_within_a_tab_only() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        let panes = tab(&[1, 2]);
        let from = rect(0., 0., 100., 100.);
        motion.focus(Some((panes.clone(), 1)), None, false, start);
        assert_eq!(motion.last_active(), Some(1));
        motion.focus(Some((panes.clone(), 2)), Some(from), false, start);
        let (at, to, went) = motion.ring(start + ms(100)).unwrap();
        assert_eq!((at, to), (from, 2));
        assert!(went > 0.5 && went < 1.0, "{went}");
        assert!(!motion.holding(start + ms(100)));
        assert!(motion.moving(start + ms(100)));
        assert_eq!(motion.ring(start + ms(200)), None);
        assert!(!motion.moving(start + ms(200)));
        // To another tab: no slide.
        motion.focus(Some((panes.clone(), 1)), Some(from), false, start + ms(300));
        assert!(motion.ring(start + ms(300)).is_some());
        motion.focus(Some((tab(&[3, 4]), 4)), Some(from), false, start + ms(310));
        assert_eq!(motion.ring(start + ms(310)), None);
        // One pane alone has no rim to slide.
        motion.focus(None, Some(from), false, start + ms(320));
        motion.focus(Some((tab(&[3, 4]), 3)), Some(from), false, start + ms(330));
        assert_eq!(motion.ring(start + ms(330)), None);
    }

    #[test]
    fn the_veils_cross_fade_and_a_pane_new_to_the_frame_is_as_it_should_be_at_once() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.veils(&[(1, false), (2, true)], start);
        assert_eq!((motion.veil(1, start), motion.veil(2, start)), (0.0, 1.0));
        motion.veils(&[(1, true), (2, false), (3, true)], start);
        let middle = start + ms(100);
        let (one, two) = (motion.veil(1, middle), motion.veil(2, middle));
        assert!(
            one > 0.5 && one < 1.0 && two > 0.0 && two < 0.5,
            "{one} {two}"
        );
        assert_eq!(motion.veil(3, middle), 1.0);
        assert!(motion.moving(middle));
        let end = start + ms(200);
        assert_eq!((motion.veil(1, end), motion.veil(2, end)), (1.0, 0.0));
        assert!(!motion.moving(end));
        // A pane not drawn for a while comes back as it should be.
        motion.veils(&[(1, true)], end);
        motion.veils(&[(1, true), (2, true)], end);
        assert_eq!(motion.veil(2, end), 1.0);
    }

    #[test]
    fn a_tab_switched_to_fades_in_as_it_moves_over_from_the_side_it_lies_on() {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        let tabs = [11, 12, 13];
        // The first tab drawn is there at once.
        motion.switch(0, &tabs, 0, false, start);
        assert_eq!(motion.tab_look(start), (1.0, 0.0));
        assert!(!motion.moving(start));
        // To a tab on the right: from 4 points to the right, faded out.
        motion.switch(0, &tabs, 2, false, start);
        assert_eq!(motion.tab_look(start), (0.0, tween::POP_RISE));
        let (shown, shift) = motion.tab_look(start + ms(40));
        assert!(shown > 0.0 && shown < 1.0, "{shown}");
        assert!(shift > 0.0 && shift < tween::POP_RISE, "{shift}");
        assert!(close(shift, tween::POP_RISE * (1.0 - shown)));
        assert!(motion.moving(start + ms(40)));
        // In place: still, and no frames.
        assert_eq!(motion.tab_look(start + ms(120)), (1.0, 0.0));
        assert!(!motion.moving(start + ms(120)));
        // To a tab on the left: from the left; another switch on the way starts afresh.
        let left = start + ms(200);
        motion.switch(0, &tabs, 1, false, left);
        assert_eq!(motion.tab_look(left), (0.0, -tween::POP_RISE));
        let again = left + ms(30);
        motion.switch(0, &tabs, 2, false, again);
        assert_eq!(motion.tab_look(again), (0.0, tween::POP_RISE));
        // The same tab drawn again keeps on.
        motion.switch(0, &tabs, 2, false, again + ms(10));
        assert!(motion.tab_look(again + ms(10)).0 > 0.0);
        assert_eq!(motion.tab_look(again + ms(120)), (1.0, 0.0));
    }

    #[test]
    fn a_tab_comes_at_once_on_another_machine_after_its_own_closed_or_with_reduce_motion_fading_only()
     {
        let start = Instant::now();
        let mut motion = Motion::new(false, false);
        motion.switch(0, &[1, 2], 0, false, start);
        // Another machine's tabs: at once.
        motion.switch(1, &[1, 2], 1, false, start);
        assert_eq!(motion.tab_look(start), (1.0, 0.0));
        assert!(!motion.moving(start));
        // The tab shown closed: the one taking its place is there at once.
        motion.switch(1, &[1], 0, false, start);
        assert_eq!(motion.tab_look(start), (1.0, 0.0));
        assert!(!motion.moving(start));
        // Reduce Motion: it only fades.
        motion.switch(1, &[1, 3], 1, true, start);
        let (shown, shift) = motion.tab_look(start + ms(40));
        assert!(shown > 0.0 && shown < 1.0, "{shown}");
        assert_eq!(shift, 0.0);
        assert!(motion.moving(start + ms(40)));
        assert!(!motion.moving(start + ms(120)));
    }

    #[test]
    fn the_sliding_rim_over_a_panes_own_makes_the_active_ones() {
        let together = 1.0 - (1.0 - SLIDING_RIM) * (1.0 - PANEL_RIM);
        assert!(close(together, PANEL_RIM_ACTIVE));
        const { assert!(SLIDING_RIM > 0.0 && SLIDING_RIM < PANEL_RIM_ACTIVE) };
    }

    #[test]
    fn between_goes_from_one_place_to_another() {
        let (from, to) = (rect(0., 0., 100., 100.), rect(100., 50., 200., 300.));
        assert_eq!(between(from, to, 0.0), from);
        assert_eq!(between(from, to, 1.0), to);
        assert_eq!(between(from, to, 0.5), rect(50., 25., 150., 200.));
        assert_eq!(
            panel_bounds(rect(10., 10., 20., 20.)),
            rect(9., 9., 22., 22.)
        );
    }
}
