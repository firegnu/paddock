//! The Agents sidebar's icons, drawn as lines rather than taken from a font, so they all
//! share one size, one line weight and one centre whatever fonts the system has.
use gpui::{Bounds, Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, canvas, point, px};

/// The icons' square, in points, at the base interface size.
pub const SIZE: f32 = 14.0;
const LINE: f32 = 1.25;
/// `Copied`'s heavier line.
const COPIED_LINE: f32 = 1.4;
/// Where the bell hangs from: it swings about this point.
const BELL_PIVOT: (f32, f32) = (7.0, 1.8);

#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// A filled square filling most of the box, as heavy as `Copy` beside it: a card's Stop….
    StopAgent,
    /// Two rounded squares, one behind and up to the left of the other: copy.
    Copy,
    /// The window's own: a cross, for closing a tab or a pane.
    Close,
    /// A plus: a new tab.
    Plus,
    /// A frame cut into two equal halves, a plus in the right one: split the pane.
    Split,
    /// A frame cut into two equal halves: a split tab, after its title.
    Panes,
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
    /// Three sliders: the sidebar's button for its menu of actions.
    Actions,
    /// A wheel with spokes: Settings in that menu.
    Gear,
    /// A tick: a menu item that is on.
    Check,
    /// A bolder tick: a card's Copy once it has copied.
    Copied,
    /// A panel with a narrow column of lines on its left: collapse the sidebar. The mirror of
    /// `RightSidebar`.
    LeftSidebar,
    /// A panel with a narrow column of lines on its right: open or close the right sidebar.
    RightSidebar,
    /// A plus over a minus: the right sidebar's Changes.
    Changes,
    /// A globe: the right sidebar's Browser.
    Browser,
    /// A chevron pointing left: the Browser's Back.
    Back,
    /// A chevron pointing right: the Browser's Forward.
    Forward,
    /// A circle open at its top right, an arrowhead at the gap: the Browser's Reload.
    Reload,
    /// A box with an arrow out of its top right corner: open the page in the default browser.
    External,
    /// Three rounded bars hanging from one line, of different lengths: the right sidebar's Kanban.
    Kanban,
    /// A page with its corner folded and two lines: a Kanban card's task file.
    Document,
    /// A folder: the repository the Kanban shows.
    Folder,
    /// A chevron pointing down: an open group.
    Down,
    /// Two upright bars: pause an agent, or every agent.
    Pause,
    /// A triangle pointing right: resume a paused agent, or every agent.
    Resume,
}

/// How far an icon's parts are from where they rest, for its hover motion (`motion.rs`), in the
/// icon's own points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Rest,
    /// `Bell`: the body and the clapper turned clockwise about the top, in degrees.
    Swing {
        body: f32,
        clapper: f32,
    },
    /// `Copy`: the front square moved up and to the left by this much.
    Nudge(f32),
    /// `StopAgent`: the square scaled about the centre, its corners this round before scaling.
    Press {
        scale: f32,
        radius: f32,
    },
    /// `Actions`: each knob moved right (left when negative) along its rail, top to bottom.
    Slide([f32; 3]),
    /// `Copied`: the tick scaled about the centre.
    Grow(f32),
    /// `LeftSidebar`, `RightSidebar`: the column's edge and its lines moved right (left when
    /// negative); `Resume`: the triangle moved right.
    Shift(f32),
    /// `Plus` about its centre, `Search` about the lens: turned clockwise, in degrees.
    Turn(f32),
    /// `Split`: cut at the seam, each half moved this far out, the plus with the right one.
    Part(f32),
    /// `Pause`: both bars moved down by this much.
    Dip(f32),
}

/// `icon` in `color`, `SIZE` points square times `scale` (the interface size over the base).
pub fn icon(icon: Icon, color: Hsla, scale: f32) -> impl IntoElement {
    posed(icon, Pose::Rest, color, scale)
}

/// `icon` as [`icon`] draws it, its parts moved as `pose` says.
pub fn posed(icon: Icon, pose: Pose, color: Hsla, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            for path in shapes(icon, pose, bounds, scale) {
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .flex_shrink_0()
    .size(px(SIZE * scale))
}

/// The paths for `icon` in `pose`, laid out in `bounds`; coordinates are drawn for `SIZE` and
/// multiplied by `scale`.
fn shapes(icon: Icon, pose: Pose, bounds: Bounds<Pixels>, scale: f32) -> Vec<PathBuilder> {
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
        Icon::StopAgent => {
            let (size, radius) = match pose {
                Pose::Press { scale, radius } => (scale, radius),
                _ => (1.0, 2.5),
            };
            let (min, max) = (7.0 - 5.25 * size, 7.0 + 5.25 * size);
            let mut square = PathBuilder::fill();
            rounded_rect(
                &mut square,
                at(min, min),
                at(max, max),
                radius * size * scale,
            );
            vec![square]
        }
        Icon::Copy => {
            let nudge = match pose {
                Pose::Nudge(by) => by,
                _ => 0.0,
            };
            let (min, max) = (4.75 - nudge, 12.5 - nudge);
            let mut front = stroke();
            rounded_rect(&mut front, at(min, min), at(max, max), 1.75 * scale);
            // Only the back one's top and left show, round its corner.
            let mut back = stroke();
            let r = px(1.75 * scale);
            let radii = point(r, r);
            back.move_to(at(3.25, 9.25));
            back.arc_to(radii, px(0.0), false, true, at(1.5, 7.5));
            back.line_to(at(1.5, 3.25));
            back.arc_to(radii, px(0.0), false, true, at(3.25, 1.5));
            back.line_to(at(7.5, 1.5));
            back.arc_to(radii, px(0.0), false, true, at(9.25, 3.25));
            vec![front, back]
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
            let turn = match pose {
                Pose::Turn(degrees) => degrees,
                _ => 0.0,
            };
            let at = |x, y| {
                let (x, y) = turned(turn, (7.0, 7.0), x, y);
                at(x, y)
            };
            let mut plus = stroke();
            plus.move_to(at(7.0, 2.5));
            plus.line_to(at(7.0, 11.5));
            plus.move_to(at(2.5, 7.0));
            plus.line_to(at(11.5, 7.0));
            vec![plus]
        }
        Icon::Split => {
            let by = match pose {
                Pose::Part(by) => by,
                _ => 0.0,
            };
            let mut plus = stroke();
            plus.move_to(at(10.0 + by, 5.5));
            plus.line_to(at(10.0 + by, 8.5));
            plus.move_to(at(8.5 + by, 7.0));
            plus.line_to(at(11.5 + by, 7.0));
            if by == 0.0 {
                let mut frame = stroke();
                rounded_rect(&mut frame, at(1.0, 2.0), at(13.0, 12.0), 2.0 * scale);
                let mut seam = stroke();
                seam.move_to(at(7.0, 2.0));
                seam.line_to(at(7.0, 12.0));
                return vec![frame, seam, plus];
            }
            // Parted, each half of the frame is closed along its own side of the seam.
            let r = px(2.0 * scale);
            let radii = point(r, r);
            let mut left = stroke();
            left.move_to(at(7.0 - by, 2.0));
            left.line_to(at(3.0 - by, 2.0));
            left.arc_to(radii, px(0.0), false, false, at(1.0 - by, 4.0));
            left.line_to(at(1.0 - by, 10.0));
            left.arc_to(radii, px(0.0), false, false, at(3.0 - by, 12.0));
            left.line_to(at(7.0 - by, 12.0));
            left.close();
            let mut right = stroke();
            right.move_to(at(7.0 + by, 2.0));
            right.line_to(at(11.0 + by, 2.0));
            right.arc_to(radii, px(0.0), false, true, at(13.0 + by, 4.0));
            right.line_to(at(13.0 + by, 10.0));
            right.arc_to(radii, px(0.0), false, true, at(11.0 + by, 12.0));
            right.line_to(at(7.0 + by, 12.0));
            right.close();
            vec![left, right, plus]
        }
        Icon::Panes => {
            let mut frame = stroke();
            rounded_rect(&mut frame, at(1.0, 2.0), at(13.0, 12.0), 2.0 * scale);
            let mut seam = stroke();
            seam.move_to(at(7.0, 2.0));
            seam.line_to(at(7.0, 12.0));
            vec![frame, seam]
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
            let (body, clapper_turn) = match pose {
                Pose::Swing { body, clapper } => (body, clapper),
                _ => (0.0, 0.0),
            };
            // A point turned clockwise by `degrees` about the top, where the bell hangs.
            let turned = |degrees: f32, x: f32, y: f32| {
                let (sin, cos) = degrees.to_radians().sin_cos();
                let (dx, dy) = (x - BELL_PIVOT.0, y - BELL_PIVOT.1);
                at(
                    BELL_PIVOT.0 + dx * cos - dy * sin,
                    BELL_PIVOT.1 + dx * sin + dy * cos,
                )
            };
            let at = |x, y| turned(body, x, y);
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
            clapper.move_to(turned(clapper_turn, 5.75, 12.0));
            clapper.line_to(turned(clapper_turn, 8.25, 12.0));
            vec![bell, clapper]
        }
        Icon::Search => {
            let tilt = match pose {
                Pose::Turn(degrees) => degrees,
                _ => 0.0,
            };
            // Turned about the lens's centre, which leaves the lens where it is.
            let handle_at = |x, y| {
                let (x, y) = turned(tilt, (6.0, 6.0), x, y);
                at(x, y)
            };
            let mut lens = stroke();
            circle(&mut lens, at(6.0, 6.0), 4.25 * scale);
            let mut handle = stroke();
            handle.move_to(handle_at(9.25, 9.25));
            handle.line_to(handle_at(12.5, 12.5));
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
        Icon::Actions => {
            let slide = match pose {
                Pose::Slide(by) => by,
                _ => [0.0; 3],
            };
            let mut paths = Vec::new();
            // The rail breaks wherever its knob is, so the knob always covers it.
            for ((y, knob), by) in [(3.5, 4.5), (7.0, 9.5), (10.5, 6.0)].into_iter().zip(slide) {
                let knob = knob + by;
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
        Icon::Copied => {
            let grow = match pose {
                Pose::Grow(by) => by,
                _ => 1.0,
            };
            let at = |x: f32, y: f32| at(7.0 + (x - 7.0) * grow, 7.0 + (y - 7.0) * grow);
            let mut tick = PathBuilder::stroke(px(COPIED_LINE * grow * scale));
            tick.move_to(at(3.0, 7.4));
            tick.line_to(at(5.6, 10.0));
            tick.line_to(at(11.0, 4.4));
            vec![tick]
        }
        Icon::LeftSidebar | Icon::RightSidebar => {
            // The column's edge, and its lines across the middle of it.
            let (edge, from, to) = match icon {
                Icon::LeftSidebar => (5.0, 2.0, 4.0),
                _ => (9.0, 10.0, 12.0),
            };
            let (edge, from, to) = match pose {
                Pose::Shift(by) => (edge + by, from + by, to + by),
                _ => (edge, from, to),
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
        Icon::Back | Icon::Forward => {
            let (tip, end) = match icon {
                Icon::Back => (4.25, 8.75),
                _ => (9.75, 5.25),
            };
            let mut chevron = stroke();
            chevron.move_to(at(end, 2.5));
            chevron.line_to(at(tip, 7.0));
            chevron.line_to(at(end, 11.5));
            vec![chevron]
        }
        Icon::Reload => {
            // Round from the right, clockwise, almost all the way.
            let mut arc = stroke();
            let r = px(4.67 * scale);
            arc.move_to(at(11.67, 7.0));
            arc.arc_to(point(r, r), px(0.0), true, true, at(10.27, 3.68));
            let mut head = stroke();
            head.move_to(at(11.67, 2.1));
            head.line_to(at(11.67, 4.9));
            head.line_to(at(8.87, 4.9));
            vec![arc, head]
        }
        Icon::External => {
            let mut arrow = stroke();
            arrow.move_to(at(8.15, 1.75));
            arrow.line_to(at(12.25, 1.75));
            arrow.line_to(at(12.25, 5.85));
            arrow.move_to(at(12.25, 1.75));
            arrow.line_to(at(7.0, 7.0));
            // The box, open where the arrow leaves it.
            let mut open = stroke();
            let r = px(1.15 * scale);
            let radii = point(r, r);
            open.move_to(at(10.5, 8.75));
            open.line_to(at(10.5, 11.1));
            open.arc_to(radii, px(0.0), false, true, at(9.35, 12.25));
            open.line_to(at(2.9, 12.25));
            open.arc_to(radii, px(0.0), false, true, at(1.75, 11.1));
            open.line_to(at(1.75, 4.65));
            open.arc_to(radii, px(0.0), false, true, at(2.9, 3.5));
            open.line_to(at(5.25, 3.5));
            vec![arrow, open]
        }
        Icon::Kanban => [12.25, 8.75, 10.5]
            .into_iter()
            .enumerate()
            .map(|(i, bottom)| {
                let left = 1.4 + i as f32 * 4.08;
                let mut bar = stroke();
                rounded_rect(
                    &mut bar,
                    at(left, 1.75),
                    at(left + 3.03, bottom),
                    0.93 * scale,
                );
                bar
            })
            .collect(),
        Icon::Document => {
            let mut page = stroke();
            page.move_to(at(3.25, 1.6));
            page.line_to(at(8.1, 1.6));
            page.line_to(at(10.75, 4.3));
            page.line_to(at(10.75, 12.4));
            page.line_to(at(3.25, 12.4));
            page.close();
            let mut fold = stroke();
            fold.move_to(at(8.1, 1.6));
            fold.line_to(at(8.1, 4.3));
            fold.line_to(at(10.75, 4.3));
            let mut lines = stroke();
            for y in [7.55, 9.7] {
                lines.move_to(at(5.15, y));
                lines.line_to(at(8.85, y));
            }
            vec![page, fold, lines]
        }
        Icon::Folder => {
            let mut folder = stroke();
            let r = px(1.08 * scale);
            let radii = point(r, r);
            folder.move_to(at(1.6, 3.75));
            folder.arc_to(radii, px(0.0), false, true, at(2.7, 2.7));
            folder.line_to(at(5.9, 2.7));
            folder.line_to(at(7.2, 4.2));
            folder.line_to(at(11.85, 4.2));
            folder.arc_to(radii, px(0.0), false, true, at(12.9, 5.3));
            folder.line_to(at(12.9, 11.3));
            folder.arc_to(radii, px(0.0), false, true, at(11.85, 12.4));
            folder.line_to(at(2.7, 12.4));
            folder.arc_to(radii, px(0.0), false, true, at(1.6, 11.3));
            folder.close();
            vec![folder]
        }
        Icon::Down => {
            let mut chevron = stroke();
            chevron.move_to(at(3.0, 5.0));
            chevron.line_to(at(7.0, 9.0));
            chevron.line_to(at(11.0, 5.0));
            vec![chevron]
        }
        Icon::Pause => {
            let by = match pose {
                Pose::Dip(by) => by,
                _ => 0.0,
            };
            [3.75, 8.25]
                .into_iter()
                .map(|left| {
                    let mut bar = PathBuilder::fill();
                    rounded_rect(
                        &mut bar,
                        at(left, 2.5 + by),
                        at(left + 2.0, 11.5 + by),
                        scale,
                    );
                    bar
                })
                .collect()
        }
        Icon::Resume => {
            let by = match pose {
                Pose::Shift(by) => by,
                _ => 0.0,
            };
            // Its middle a little right of the box's, so it looks centred.
            let mut triangle = PathBuilder::fill();
            triangle.move_to(at(4.25 + by, 2.5));
            triangle.line_to(at(11.75 + by, 7.0));
            triangle.line_to(at(4.25 + by, 11.5));
            triangle.close();
            vec![triangle]
        }
    }
}

/// The point `(x, y)` turned clockwise by `degrees` about `pivot`.
fn turned(degrees: f32, pivot: (f32, f32), x: f32, y: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (dx, dy) = (x - pivot.0, y - pivot.1);
    (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
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
