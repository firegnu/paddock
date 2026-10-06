//! The Agents sidebar's icons, drawn as lines rather than taken from a font, so they all
//! share one size, one line weight and one centre whatever fonts the system has.
use gpui::{Bounds, Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, canvas, point, px};

/// The icons' square, in points, at the base interface size.
pub const SIZE: f32 = 14.0;
const LINE: f32 = 1.25;

#[derive(Clone, Copy)]
pub enum Icon {
    /// An up and a down arrow side by side.
    Sort,
    /// A square with a bar across: fold everything.
    Fold,
    /// A circled plus.
    NewAgent,
    /// A terminal window with a `>_` prompt.
    NewShell,
    /// A filled square.
    Stop,
    /// The window's own: a cross, for closing a tab or a pane.
    Close,
    /// A plus: a new tab.
    Plus,
    /// A frame cut into two equal halves, a plus in the right one: split the pane.
    Split,
    /// Arrows to two corners: zoom the pane.
    Zoom,
    /// Arrows in from two corners: restore the zoomed pane.
    Restore,
    /// A bell: the sidebar header's Attention badge.
    Bell,
    /// A magnifier: the title bar's Search and the command palette's field.
    Search,
    /// Two sliders: a Settings page in the command palette.
    Settings,
    /// A chevron: a command in the command palette.
    Command,
    /// A window with its title bar: a card's mark for an agent open in this window.
    Here,
    /// Three sliders: the sidebar's button for its menu of actions.
    Actions,
    /// A wheel with spokes: Settings in that menu.
    Gear,
    /// A tick: a menu item that is on.
    Check,
    /// A panel with a narrow column of lines on its left: collapse the sidebar. The mirror of
    /// `RightSidebar`.
    LeftSidebar,
    /// A panel with a narrow column of lines on its right: open or close the right sidebar.
    RightSidebar,
    /// A plus over a minus: the right sidebar's Changes.
    Changes,
    /// A globe: the right sidebar's Browser.
    Browser,
}

/// `icon` in `color`, `SIZE` points square times `scale` (the interface size over the base).
pub fn icon(icon: Icon, color: Hsla, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            for path in shapes(icon, bounds, scale) {
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .flex_shrink_0()
    .size(px(SIZE * scale))
}

/// The paths for `icon`, laid out in `bounds`; coordinates are drawn for `SIZE` and multiplied by
/// `scale`.
fn shapes(icon: Icon, bounds: Bounds<Pixels>, scale: f32) -> Vec<PathBuilder> {
    let at =
        |x: f32, y: f32| -> Point<Pixels> { bounds.origin + point(px(x * scale), px(y * scale)) };
    let stroke = || PathBuilder::stroke(px(LINE * scale));
    match icon {
        Icon::Sort => {
            let mut up = stroke();
            up.move_to(at(4.5, 12.0));
            up.line_to(at(4.5, 2.5));
            up.move_to(at(2.0, 5.0));
            up.line_to(at(4.5, 2.5));
            up.line_to(at(7.0, 5.0));
            let mut down = stroke();
            down.move_to(at(9.5, 2.0));
            down.line_to(at(9.5, 11.5));
            down.move_to(at(7.0, 9.0));
            down.line_to(at(9.5, 11.5));
            down.line_to(at(12.0, 9.0));
            vec![up, down]
        }
        Icon::Fold => {
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.5, 1.5), at(12.5, 12.5), 2.5 * scale);
            let mut bar = stroke();
            bar.move_to(at(4.5, 7.0));
            bar.line_to(at(9.5, 7.0));
            vec![frame, bar]
        }
        Icon::NewAgent => {
            let mut ring = stroke();
            let r = px(5.75 * scale);
            ring.move_to(at(12.75, 7.0));
            ring.arc_to(point(r, r), px(0.0), false, true, at(1.25, 7.0));
            ring.arc_to(point(r, r), px(0.0), false, true, at(12.75, 7.0));
            ring.close();
            let mut plus = stroke();
            plus.move_to(at(4.25, 7.0));
            plus.line_to(at(9.75, 7.0));
            plus.move_to(at(7.0, 4.25));
            plus.line_to(at(7.0, 9.75));
            vec![ring, plus]
        }
        Icon::NewShell => {
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.0, 2.0), at(13.0, 12.0), 2.0 * scale);
            let mut prompt = stroke();
            prompt.move_to(at(3.75, 5.0));
            prompt.line_to(at(6.0, 7.0));
            prompt.line_to(at(3.75, 9.0));
            prompt.move_to(at(7.25, 9.25));
            prompt.line_to(at(10.5, 9.25));
            vec![frame, prompt]
        }
        Icon::Stop => {
            let mut square = PathBuilder::fill();
            rounded_rect(&mut square, at(3.0, 3.0), at(11.0, 11.0), 1.5 * scale);
            vec![square]
        }
        Icon::Close => {
            let mut cross = stroke();
            cross.move_to(at(3.5, 3.5));
            cross.line_to(at(10.5, 10.5));
            cross.move_to(at(10.5, 3.5));
            cross.line_to(at(3.5, 10.5));
            vec![cross]
        }
        Icon::Plus => {
            let mut plus = stroke();
            plus.move_to(at(7.0, 2.5));
            plus.line_to(at(7.0, 11.5));
            plus.move_to(at(2.5, 7.0));
            plus.line_to(at(11.5, 7.0));
            vec![plus]
        }
        Icon::Split => {
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.0, 2.0), at(13.0, 12.0), 2.0 * scale);
            let mut seam = stroke();
            seam.move_to(at(7.0, 2.0));
            seam.line_to(at(7.0, 12.0));
            let mut plus = stroke();
            plus.move_to(at(10.0, 5.5));
            plus.line_to(at(10.0, 8.5));
            plus.move_to(at(8.5, 7.0));
            plus.line_to(at(11.5, 7.0));
            vec![frame, seam, plus]
        }
        Icon::Zoom => {
            let mut arrows = stroke();
            arrows.move_to(at(8.5, 2.5));
            arrows.line_to(at(11.5, 2.5));
            arrows.line_to(at(11.5, 5.5));
            arrows.move_to(at(11.5, 2.5));
            arrows.line_to(at(8.0, 6.0));
            arrows.move_to(at(5.5, 11.5));
            arrows.line_to(at(2.5, 11.5));
            arrows.line_to(at(2.5, 8.5));
            arrows.move_to(at(2.5, 11.5));
            arrows.line_to(at(6.0, 8.0));
            vec![arrows]
        }
        Icon::Restore => {
            let mut arrows = stroke();
            arrows.move_to(at(8.0, 3.0));
            arrows.line_to(at(8.0, 6.0));
            arrows.line_to(at(11.0, 6.0));
            arrows.move_to(at(8.0, 6.0));
            arrows.line_to(at(11.5, 2.5));
            arrows.move_to(at(6.0, 11.0));
            arrows.line_to(at(6.0, 8.0));
            arrows.line_to(at(3.0, 8.0));
            arrows.move_to(at(6.0, 8.0));
            arrows.line_to(at(2.5, 11.5));
            vec![arrows]
        }
        Icon::Bell => {
            let mut bell = stroke();
            let r = px(3.5 * scale);
            bell.move_to(at(3.5, 9.0));
            bell.line_to(at(3.5, 6.0));
            bell.arc_to(point(r, r), px(0.0), false, true, at(10.5, 6.0));
            bell.line_to(at(10.5, 9.0));
            bell.line_to(at(11.75, 10.25));
            bell.line_to(at(2.25, 10.25));
            bell.close();
            let mut clapper = stroke();
            clapper.move_to(at(5.75, 12.0));
            clapper.line_to(at(8.25, 12.0));
            vec![bell, clapper]
        }
        Icon::Search => {
            let mut lens = stroke();
            circle(&mut lens, at(6.0, 6.0), 4.25 * scale);
            let mut handle = stroke();
            handle.move_to(at(9.25, 9.25));
            handle.line_to(at(12.5, 12.5));
            vec![lens, handle]
        }
        Icon::Settings => {
            let mut rails = stroke();
            rails.move_to(at(1.5, 4.5));
            rails.line_to(at(7.25, 4.5));
            rails.move_to(at(10.75, 4.5));
            rails.line_to(at(12.5, 4.5));
            rails.move_to(at(1.5, 9.5));
            rails.line_to(at(3.25, 9.5));
            rails.move_to(at(6.75, 9.5));
            rails.line_to(at(12.5, 9.5));
            let mut upper = stroke();
            circle(&mut upper, at(9.0, 4.5), 1.75 * scale);
            let mut lower = stroke();
            circle(&mut lower, at(5.0, 9.5), 1.75 * scale);
            vec![rails, upper, lower]
        }
        Icon::Command => {
            let mut chevron = stroke();
            chevron.move_to(at(5.5, 3.5));
            chevron.line_to(at(9.0, 7.0));
            chevron.line_to(at(5.5, 10.5));
            vec![chevron]
        }
        Icon::Here => {
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.5, 2.5), at(12.5, 11.5), 2.0 * scale);
            let mut bar = stroke();
            bar.move_to(at(1.5, 5.25));
            bar.line_to(at(12.5, 5.25));
            vec![frame, bar]
        }
        Icon::Actions => {
            let mut paths = Vec::new();
            for (y, knob) in [(3.5, 4.5), (7.0, 9.5), (10.5, 6.0)] {
                let mut rail = stroke();
                rail.move_to(at(1.5, y));
                rail.line_to(at(knob - 1.5, y));
                rail.move_to(at(knob + 1.5, y));
                rail.line_to(at(12.5, y));
                let mut ring = stroke();
                circle(&mut ring, at(knob, y), 1.5 * scale);
                paths.extend([rail, ring]);
            }
            paths
        }
        Icon::Gear => {
            let mut hub = stroke();
            circle(&mut hub, at(7.0, 7.0), 2.0 * scale);
            let mut spokes = stroke();
            for ((x0, y0), (x1, y1)) in [
                ((7.0, 1.2), (7.0, 2.8)),
                ((7.0, 11.2), (7.0, 12.8)),
                ((1.2, 7.0), (2.8, 7.0)),
                ((11.2, 7.0), (12.8, 7.0)),
                ((2.9, 2.9), (4.0, 4.0)),
                ((10.0, 10.0), (11.1, 11.1)),
                ((2.9, 11.1), (4.0, 10.0)),
                ((10.0, 4.0), (11.1, 2.9)),
            ] {
                spokes.move_to(at(x0, y0));
                spokes.line_to(at(x1, y1));
            }
            vec![hub, spokes]
        }
        Icon::Check => {
            let mut tick = stroke();
            tick.move_to(at(3.0, 7.5));
            tick.line_to(at(5.75, 10.25));
            tick.line_to(at(11.0, 4.0));
            vec![tick]
        }
        Icon::LeftSidebar | Icon::RightSidebar => {
            // The column's edge, and its lines across the middle of it.
            let (edge, from, to) = match icon {
                Icon::LeftSidebar => (5.0, 2.0, 4.0),
                _ => (9.0, 10.0, 12.0),
            };
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.0, 2.0), at(13.0, 12.0), 2.0 * scale);
            let mut side = stroke();
            side.move_to(at(edge, 2.0));
            side.line_to(at(edge, 12.0));
            let mut lines = stroke();
            for y in [5.0, 7.0, 9.0] {
                lines.move_to(at(from, y));
                lines.line_to(at(to, y));
            }
            vec![frame, side, lines]
        }
        Icon::Changes => {
            let mut plus = stroke();
            plus.move_to(at(7.0, 1.75));
            plus.line_to(at(7.0, 7.25));
            plus.move_to(at(4.25, 4.5));
            plus.line_to(at(9.75, 4.5));
            let mut minus = stroke();
            minus.move_to(at(4.25, 10.75));
            minus.line_to(at(9.75, 10.75));
            vec![plus, minus]
        }
        Icon::Browser => {
            let mut ring = stroke();
            circle(&mut ring, at(7.0, 7.0), 5.5 * scale);
            let mut meridian = stroke();
            let radii = point(px(2.5 * scale), px(5.5 * scale));
            meridian.move_to(at(7.0, 1.5));
            meridian.arc_to(radii, px(0.0), false, true, at(7.0, 12.5));
            meridian.arc_to(radii, px(0.0), false, true, at(7.0, 1.5));
            meridian.close();
            let mut equator = stroke();
            equator.move_to(at(1.5, 7.0));
            equator.line_to(at(12.5, 7.0));
            vec![ring, meridian, equator]
        }
    }
}

/// A closed circle round `centre` of radius `r`.
fn circle(path: &mut PathBuilder, centre: Point<Pixels>, r: f32) {
    let (r, radii) = (px(r), point(px(r), px(r)));
    path.move_to(point(centre.x + r, centre.y));
    path.arc_to(radii, px(0.0), false, true, point(centre.x - r, centre.y));
    path.arc_to(radii, px(0.0), false, true, point(centre.x + r, centre.y));
    path.close();
}

/// A closed rectangle from `min` to `max` with corners of radius `r`.
fn rounded_rect(path: &mut PathBuilder, min: Point<Pixels>, max: Point<Pixels>, r: f32) {
    let r = px(r);
    let radii = point(r, r);
    path.move_to(point(min.x + r, min.y));
    path.line_to(point(max.x - r, min.y));
    path.arc_to(radii, px(0.0), false, true, point(max.x, min.y + r));
    path.line_to(point(max.x, max.y - r));
    path.arc_to(radii, px(0.0), false, true, point(max.x - r, max.y));
    path.line_to(point(min.x + r, max.y));
    path.arc_to(radii, px(0.0), false, true, point(min.x, max.y - r));
    path.line_to(point(min.x, min.y + r));
    path.arc_to(radii, px(0.0), false, true, point(min.x + r, min.y));
    path.close();
}
