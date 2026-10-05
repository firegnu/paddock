//! The Agents sidebar footer's icons, drawn as lines rather than taken from a font, so all five
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
    }
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
