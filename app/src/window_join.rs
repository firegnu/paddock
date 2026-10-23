//! The active tab joined to the pane's panel under it (P5-165): it grows down from its capsule's
//! top to the panel's top edge in the terminal's ground, its top corners rounded and a small curve
//! out at either side of its foot meeting that edge. The panel's rim goes up round it, left open
//! where the two meet; its status colour or the working light runs along that outline instead.
//! Where no one panel lies under the whole of it (over a seam, over the right sidebar, over
//! nothing), it stays the capsule it was.
//!
//! Where the tab and the panels are is known only once they are laid out, so what it joins is
//! worked out from where they noted themselves this frame, after the panes are laid out and
//! before anything is painted.
use super::*;
use gpui::{BorderStyle, ContentMask, PathBuilder, quad};
use std::f32::consts::{FRAC_PI_2, PI};

/// The joined tab's top corners and the curves at its foot, as the approved design has them,
/// scaled with the interface.
const CORNER: f32 = 10.0;
const FLARE: f32 = 8.0;
/// The room kept between the first tab's left curve and the panel's rounded corner; unscaled.
const CLEAR: f32 = 4.0;
/// How far in from the outline's outer edge its line runs: on the panel's rim, a point wide.
const INSET: f32 = 0.5;
/// The points each curve of the outline is drawn through.
const ARC_STEPS: usize = 8;
/// How far apart two edges can be and still meet.
const MEET: f32 = 0.5;

/// A pane's panel drawn this frame, the topmost last: how opaque it is drawn (fading as another
/// grows to fill the tab, or as its tab is switched to) and its rim's strength.
#[derive(Clone, Copy, Debug)]
pub(super) struct Drawn {
    pub(super) pane: PaneId,
    pub(super) shown: f32,
    pub(super) rim: f32,
}

/// The joined tab's outline, in window points: its sides and top, the panel's top edge it stands
/// on, its corners' and its foot's curves' radii.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shape {
    pub(super) left: f32,
    pub(super) right: f32,
    pub(super) top: f32,
    pub(super) foot: f32,
    pub(super) corner: f32,
    pub(super) flare: f32,
}

/// The active tab joined to a panel this frame: the panel's pane, the tab's shape, and the
/// panel's opacity and rim strength, which the tab takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Joint {
    pub(super) pane: PaneId,
    pub(super) shape: Shape,
    pub(super) shown: f32,
    pub(super) rim: f32,
}

/// What the active tab is drawn in: the working light's colour while it runs, else its status
/// colour; as a capsule, its ground and its ring; and whether the theme is a light one, for the
/// working light's head (P5-175).
#[derive(Clone, Copy)]
pub(super) struct TabLook {
    pub(super) light: Option<Hsla>,
    pub(super) light_theme: bool,
    pub(super) tint: Option<Hsla>,
    pub(super) ground: Hsla,
    pub(super) ring: Hsla,
}

/// Which of `panels` (window points, the topmost last) the tab from `left` to `right` joins, its
/// curves `flare` wide either side, standing on `foot`: the topmost whose top edge is there and
/// runs under it; none when that one does not run under the whole of it clear of its rounded
/// corners, or when none is there.
pub(super) fn under(
    left: f32,
    right: f32,
    foot: f32,
    flare: f32,
    panels: &[Bounds<Pixels>],
) -> Option<usize> {
    let (from, to) = (left - flare, right + flare);
    let (index, panel) = panels.iter().enumerate().rev().find(|(_, panel)| {
        (f32::from(panel.top()) - foot).abs() <= MEET
            && f32::from(panel.left()) < to
            && f32::from(panel.right()) > from
    })?;
    let clear = from >= f32::from(panel.left()) + PANEL_RADIUS
        && to <= f32::from(panel.right()) - PANEL_RADIUS;
    clear.then_some(index)
}

/// How far in from the start of the title bar's part after the sidebar, `start` (window points),
/// the tabs begin: the first one's left curve, `flare` wide, clear of the rounded corner of a
/// panel whose left edge is at `panel`; never less than the curve, which is drawn inside.
pub(super) fn inset(start: f32, panel: f32, flare: f32) -> f32 {
    (panel + PANEL_RADIUS + flare + CLEAR - start).max(flare)
}

impl Shape {
    /// The outline's parts, `inset` in from its outer edge, from its left end: the left curve, the
    /// left side, the top left corner, the top, the top right corner, the right side, the right
    /// curve; each part's length, and its radius when it is a curve.
    fn parts(&self, inset: f32) -> [(f32, Option<f32>); 7] {
        let curve = self.flare + inset;
        let corner = (self.corner - inset).max(0.0);
        let side = (self.foot - self.flare - self.top - self.corner).max(0.0);
        let top = (self.right - self.left - 2.0 * self.corner).max(0.0);
        [
            (FRAC_PI_2 * curve, Some(curve)),
            (side, None),
            (FRAC_PI_2 * corner, Some(corner)),
            (top, None),
            (FRAC_PI_2 * corner, Some(corner)),
            (side, None),
            (FRAC_PI_2 * curve, Some(curve)),
        ]
    }

    /// The point `along` points along the outline's part `part` (see [`Self::parts`]).
    fn on(&self, part: usize, along: f32, inset: f32) -> (f32, f32) {
        let (curve, corner) = (self.flare + inset, (self.corner - inset).max(0.0));
        let round = |(x, y): (f32, f32), radius: f32, angle: f32| {
            (x + radius * angle.cos(), y + radius * angle.sin())
        };
        let low = self.foot - self.flare;
        let high = self.top + self.corner;
        match part {
            // Down is the way y grows: from straight under the curve's centre round to its right.
            0 => round(
                (self.left - self.flare, low),
                curve,
                FRAC_PI_2 - along / curve.max(f32::EPSILON),
            ),
            1 => (self.left + inset, low - along),
            2 => round(
                (self.left + self.corner, high),
                corner,
                PI + along / corner.max(f32::EPSILON),
            ),
            3 => (self.left + self.corner + along, self.top + inset),
            4 => round(
                (self.right - self.corner, high),
                corner,
                1.5 * PI + along / corner.max(f32::EPSILON),
            ),
            5 => (self.right - inset, high + along),
            _ => round(
                (self.right + self.flare, low),
                curve,
                PI - along / curve.max(f32::EPSILON),
            ),
        }
    }

    /// The point `t` of the way along the outline, `inset` in from its outer edge: from the foot
    /// of the left curve, up it and the left side, over the top, down the right side and the
    /// right curve to its foot.
    pub(super) fn point(&self, t: f32, inset: f32) -> (f32, f32) {
        let parts = self.parts(inset);
        let total: f32 = parts.iter().map(|(length, _)| length).sum();
        let mut along = t.clamp(0.0, 1.0) * total;
        for (part, (length, _)) in parts.iter().enumerate() {
            if along <= *length || part == parts.len() - 1 {
                return self.on(part, along.min(*length), inset);
            }
            along -= length;
        }
        unreachable!("the outline has parts")
    }

    /// The outline `inset` in from its outer edge, as points to draw it through: each curve's
    /// in steps, each straight part's ends.
    fn outline(&self, inset: f32) -> Vec<Point<Pixels>> {
        let mut points = vec![self.on(0, 0.0, inset)];
        for (part, (length, curve)) in self.parts(inset).iter().enumerate() {
            let steps = if curve.is_some() { ARC_STEPS } else { 1 };
            for step in 1..=steps {
                points.push(self.on(part, length * step as f32 / steps as f32, inset));
            }
        }
        points
            .into_iter()
            .map(|(x, y)| point(px(x), px(y)))
            .collect()
    }

    /// Where the panel's rim is left open under the tab: from the left curve's foot to the right
    /// one's.
    fn gap(&self) -> (f32, f32) {
        (self.left - self.flare, self.right + self.flare)
    }
}

/// The tab's ground in `color`, down to the panel's top edge.
fn paint_ground(shape: &Shape, color: Hsla, window: &mut Window) {
    let mut path = PathBuilder::fill();
    for (index, at) in shape.outline(0.0).into_iter().enumerate() {
        if index == 0 {
            path.move_to(at);
        } else {
            path.line_to(at);
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

/// The tab's outline in `color`, `weight` points wide, on the panel's rim at either end.
fn paint_outline(shape: &Shape, color: Hsla, weight: f32, window: &mut Window) {
    let mut path = PathBuilder::stroke(px(weight));
    for (index, at) in shape.outline(INSET).into_iter().enumerate() {
        if index == 0 {
            path.move_to(at);
        } else {
            path.line_to(at);
        }
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

/// A panel's rim, `bounds` its outer edge, in `color`; left open where the tab joined to it in
/// `shape` stands on its top edge, when one does.
pub(super) fn paint_rim(
    bounds: Bounds<Pixels>,
    color: Hsla,
    shape: Option<Shape>,
    window: &mut Window,
) {
    let rim = || {
        quad(
            bounds,
            px(PANEL_RADIUS),
            gpui::transparent_black(),
            px(1.0),
            color,
            BorderStyle::Solid,
        )
    };
    let Some(shape) = shape else {
        window.paint_quad(rim());
        return;
    };
    // Painted in three parts that do not overlap, for a faint rim not to show darker where two
    // would: left of the opening, right of it, and below the top edge's line between.
    let (from, to) = shape.gap();
    let far = px(1.0e6);
    let open = px(shape.foot + 1.0);
    let parts = [
        Bounds::from_corners(point(-far, -far), point(px(from), far)),
        Bounds::from_corners(point(px(to), -far), point(far, far)),
        Bounds::from_corners(point(px(from), open), point(px(to), far)),
    ];
    for part in parts {
        window.with_content_mask(Some(ContentMask { bounds: part }), |window| {
            window.paint_quad(rim())
        });
    }
}

impl PaddockWindow {
    /// How far in the tabs begin from the title bar's part after the sidebar, as wide as it is
    /// drawn this frame (see [`inset`]).
    pub(super) fn tabs_inset(&self, full_screen: bool, ui: &UiFont) -> f32 {
        inset(
            self.bar_left_now(full_screen, ui) + BAR_END,
            self.sidebar_now(ui) + CARD_GAP,
            ui.scale(FLARE),
        )
    }

    /// The curves at the joined tab's foot, as wide as they are drawn.
    pub(super) fn tab_flare(&self, ui: &UiFont) -> f32 {
        ui.scale(FLARE)
    }

    /// The panels drawn this frame, the topmost last: `panes` laid out in the tab, `count` of
    /// them on show, and the one `growing` over them.
    pub(super) fn drawn(
        &self,
        panes: &[PaneId],
        growing: Option<PaneId>,
        count: usize,
    ) -> Rc<[Drawn]> {
        let active = self.workspace.active_pane();
        // A tab switched to fades in, its panes and all; the pane growing is drawn over it.
        let look = self.tab_look().0;
        let drawn = |pane: PaneId, shown: f32| Drawn {
            pane,
            shown,
            rim: self.pane_rim(pane, count, pane == active),
        };
        panes
            .iter()
            .filter(|&&pane| Some(pane) != growing)
            .map(|&pane| drawn(pane, self.pane_shown(pane) * look))
            .chain(growing.map(|pane| drawn(pane, self.pane_shown(pane))))
            .collect()
    }

    /// The active tab's ground, under the tabs: joined, the tab down to the panel; else the
    /// capsule it was, its ring or its working light round it.
    pub(super) fn tab_ground(&self, look: TabLook, ui: &UiFont) -> AnyElement {
        let spots = self.spots.clone();
        let joint = self.joint.clone();
        let terminal = hsla(self.theme.terminal().background, 1.0);
        let scale = ui.scale(1.0);
        let ground = move |phase: f32| {
            let (spots, joint) = (spots.clone(), joint.clone());
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    if let Some(joint) = joint.get() {
                        paint_ground(&joint.shape, terminal.opacity(joint.shown), window);
                        return;
                    }
                    let Some(inside) = spots.borrow().get(&Spot::ActiveTab).copied() else {
                        return;
                    };
                    let bounds = inside.dilate(px(TAB_RING));
                    window.paint_quad(quad(
                        bounds,
                        bounds.size.height / 2.0,
                        look.ground,
                        px(TAB_RING),
                        look.ring,
                        BorderStyle::Solid,
                    ));
                    if let Some(ink) = look.light {
                        let head = orbit::head(ink, look.light_theme);
                        orbit::paint(inside, phase, ink, head, 1.0, scale, window);
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        match look.light {
            Some(_) => div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .with_animation("tab-ground", orbit::beat(), move |layer, phase| {
                    layer.child(ground(phase))
                })
                .into_any_element(),
            None => ground(0.0).into_any_element(),
        }
    }

    /// Over the panes: works out what the active tab joins this frame, from where it and the
    /// `drawn` panels are, the title bar `foot` points tall; and draws its outline in `look`'s
    /// colours over the panel's opened rim, the panel's own rim when it has none, a step brighter
    /// where the active pane's rim slides under it (P5-148a).
    pub(super) fn join_layer(
        &self,
        drawn: Rc<[Drawn]>,
        look: TabLook,
        foot: f32,
        ui: &UiFont,
    ) -> AnyElement {
        let spots = self.spots.clone();
        let joint = self.joint.clone();
        let joined = self.joined.clone();
        let text = self.fg(|t| t.agents_text);
        let sliding = self.sliding_now();
        let scale = ui.scale(1.0);
        let corner = ui.scale(CORNER);
        let flare = ui.scale(FLARE);
        let layer = move |phase: f32| {
            let (spots, joint, joined, drawn) =
                (spots.clone(), joint.clone(), joined.clone(), drawn.clone());
            canvas(
                move |_, window, _| {
                    let found = find(&spots.borrow(), &drawn, foot, corner, flare);
                    joint.set(found);
                    // What the tab's dot was ringed in is right from the next frame.
                    if joined.replace(found.is_some()) != found.is_some() {
                        window.request_animation_frame();
                    }
                    found
                },
                move |_, found, window, _| {
                    let Some(Joint {
                        shape, shown, rim, ..
                    }) = found
                    else {
                        return;
                    };
                    if let Some(ink) = look.light {
                        let at = |t: f32| {
                            let (x, y) = shape.point(t, INSET);
                            point(px(x), px(y))
                        };
                        let head = orbit::head(ink, look.light_theme);
                        orbit::paint_open(at, phase, ink, head, shown, scale, window);
                    } else if let Some(tint) = look.tint {
                        let color = tint.opacity(RING_TINTED * shown);
                        paint_outline(&shape, color, TAB_RING, window);
                    } else {
                        paint_outline(&shape, text.opacity(rim * shown), 1.0, window);
                        // The sliding rim brightens what of the outline stands on its top edge.
                        if let Some(at) =
                            sliding.filter(|at| (f32::from(at.top()) - shape.foot).abs() <= MEET)
                        {
                            let far = px(1.0e6);
                            let under = Bounds::from_corners(
                                point(at.left(), -far),
                                point(at.right(), far),
                            );
                            window.with_content_mask(Some(ContentMask { bounds: under }), |w| {
                                paint_outline(&shape, text.opacity(moving::SLIDING_RIM), 1.0, w)
                            });
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        match look.light {
            Some(_) => div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .with_animation("tab-join", orbit::beat(), move |over, phase| {
                    over.child(layer(phase))
                })
                .into_any_element(),
            None => layer(0.0).into_any_element(),
        }
    }

    /// A pane's rim, `rim` opaque, over its panel: left open where the active tab joins it.
    pub(super) fn panel_rim(&self, pane: PaneId, rim: f32) -> impl IntoElement {
        let joint = self.joint.clone();
        let color = self.fg(|t| t.agents_text).opacity(rim);
        canvas(
            |_, _, _| {},
            move |inside, _, window, _| {
                let shape = joint.get().filter(|j| j.pane == pane).map(|j| j.shape);
                paint_rim(moving::panel_bounds(inside), color, shape, window);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}

/// What the active tab joins, from where it and the `drawn` panels noted themselves in `spots`,
/// the title bar `foot` points tall, its corners `corner` and its curves `flare`.
fn find(
    spots: &HashMap<Spot, Bounds<Pixels>>,
    drawn: &[Drawn],
    foot: f32,
    corner: f32,
    flare: f32,
) -> Option<Joint> {
    let tab = spots.get(&Spot::ActiveTab)?.dilate(px(TAB_RING));
    let panels: Vec<(Drawn, Bounds<Pixels>)> = drawn
        .iter()
        .filter_map(|d| {
            let spot = spots.get(&Spot::Pane(d.pane))?;
            Some((*d, moving::panel_bounds(*spot)))
        })
        .collect();
    let bounds: Vec<Bounds<Pixels>> = panels.iter().map(|(_, b)| *b).collect();
    let (left, right) = (f32::from(tab.left()), f32::from(tab.right()));
    let (panel, _) = panels[under(left, right, foot, flare, &bounds)?];
    Some(Joint {
        pane: panel.pane,
        shape: Shape {
            left,
            right,
            top: f32::from(tab.top()),
            foot,
            corner,
            flare,
        },
        shown: panel.shown,
        rim: panel.rim,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }

    fn panel(left: f32, top: f32, right: f32, bottom: f32) -> Bounds<Pixels> {
        Bounds::from_corners(point(px(left), px(top)), point(px(right), px(bottom)))
    }

    #[test]
    fn a_tab_over_one_panel_joins_it() {
        // One panel from 228 to 1000 under a 40 point title bar; the tab from 260 to 400.
        let panels = [panel(228.0, 40.0, 1000.0, 700.0)];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), Some(0));
    }

    #[test]
    fn side_by_side_a_tab_joins_the_panel_right_under_it() {
        let panels = [
            panel(228.0, 40.0, 600.0, 700.0),
            panel(608.0, 40.0, 1000.0, 700.0),
        ];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), Some(0));
        assert_eq!(under(650.0, 800.0, 40.0, 8.0, &panels), Some(1));
    }

    #[test]
    fn one_above_another_a_tab_joins_the_top_one() {
        let panels = [
            panel(228.0, 40.0, 1000.0, 360.0),
            panel(228.0, 368.0, 1000.0, 700.0),
        ];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), Some(0));
    }

    #[test]
    fn a_tab_over_the_seam_between_two_panels_joins_neither() {
        let panels = [
            panel(228.0, 40.0, 600.0, 700.0),
            panel(608.0, 40.0, 1000.0, 700.0),
        ];
        // Right across the seam.
        assert_eq!(under(540.0, 680.0, 40.0, 8.0, &panels), None);
        // Inside the left panel, but its right curve on that panel's rounded corner.
        assert_eq!(under(450.0, 585.0, 40.0, 8.0, &panels), None);
    }

    #[test]
    fn a_tab_with_no_panel_under_it_joins_none() {
        // Nothing drawn at all, the right sidebar's card not being a pane's panel.
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &[]), None);
        // The panes' panels end left of the tab, where the right sidebar is.
        let panels = [panel(228.0, 40.0, 600.0, 700.0)];
        assert_eq!(under(650.0, 800.0, 40.0, 8.0, &panels), None);
        // The only panel under it starts lower down.
        let panels = [panel(228.0, 368.0, 1000.0, 700.0)];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), None);
    }

    #[test]
    fn a_panel_drawn_over_another_is_the_one_joined() {
        // A zoomed pane grown over the split, drawn last.
        let panels = [
            panel(228.0, 40.0, 600.0, 700.0),
            panel(228.0, 40.0, 1000.0, 700.0),
        ];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), Some(1));
        // Still growing from below, its top not yet up at the title bar: the one under it.
        let panels = [
            panel(228.0, 40.0, 1000.0, 360.0),
            panel(228.0, 200.0, 1000.0, 700.0),
        ];
        assert_eq!(under(260.0, 400.0, 40.0, 8.0, &panels), Some(0));
    }

    #[test]
    fn the_first_tab_starts_clear_of_the_panels_rounded_corner() {
        // Beside the sidebar 220 wide: the panel's edge at 228, the tabs' part from 230.
        let start = 230.0;
        let first = start + inset(start, 228.0, 8.0);
        assert!(first - 8.0 >= 228.0 + PANEL_RADIUS, "{first}");
        let panels = [panel(228.0, 40.0, 1000.0, 700.0)];
        assert_eq!(under(first, first + 100.0, 40.0, 8.0, &panels), Some(0));
        // Far enough in already beside the strip: only the curve's own room.
        assert_eq!(inset(130.0, 60.0, 8.0), 8.0);
    }

    fn shape() -> Shape {
        // A tab from 100 to 300, its top at 6, standing on a panel's top edge at 40.
        Shape {
            left: 100.0,
            right: 300.0,
            top: 6.0,
            foot: 40.0,
            corner: 10.0,
            flare: 8.0,
        }
    }

    #[test]
    fn the_outline_starts_at_the_left_curves_foot_and_ends_at_the_right_ones() {
        let shape = shape();
        // On the panel's rim, half a point under its top edge.
        assert!(near(shape.point(0.0, 0.5), (92.0, 40.5)));
        assert!(near(shape.point(1.0, 0.5), (308.0, 40.5)));
        // Without the inset, on the edge itself.
        assert!(near(shape.point(0.0, 0.0), (92.0, 40.0)));
    }

    #[test]
    fn half_way_along_the_outline_is_the_middle_of_the_top() {
        assert!(near(shape().point(0.5, 0.5), (200.0, 6.5)));
    }

    #[test]
    fn the_outline_goes_up_the_curve_and_the_side_before_the_top() {
        let shape = shape();
        let parts = shape.parts(0.0);
        let total: f32 = parts.iter().map(|(length, _)| length).sum();
        // The left curve done: at the tab's left side, as high as the curve's centre.
        let curve = parts[0].0;
        assert!(near(shape.point(curve / total, 0.0), (100.0, 32.0)));
        // Up the side to where the top left corner begins.
        let side = curve + parts[1].0;
        assert!(near(shape.point(side / total, 0.0), (100.0, 16.0)));
        // Round the corner onto the top.
        let corner = side + parts[2].0;
        assert!(near(shape.point(corner / total, 0.0), (110.0, 6.0)));
        // Along the top to the top right corner, then down the right side to its curve.
        let top = corner + parts[3].0;
        assert!(near(shape.point(top / total, 0.0), (290.0, 6.0)));
        let down = top + parts[4].0 + parts[5].0;
        assert!(near(shape.point(down / total, 0.0), (300.0, 32.0)));
    }

    #[test]
    fn the_outline_is_drawn_through_its_ends_and_corners() {
        let points = shape().outline(0.0);
        let first = (f32::from(points[0].x), f32::from(points[0].y));
        let last = points.last().unwrap();
        assert!(near(first, (92.0, 40.0)));
        assert!(near((f32::from(last.x), f32::from(last.y)), (308.0, 40.0)));
        // Each of the four curves in steps, each straight part by its end.
        assert_eq!(points.len(), 1 + 4 * ARC_STEPS + 3);
    }
}
