//! The crown that marks a controller, shared by New Agent's Controller switch (P5-102) and the
//! sidebar's cards and strip tiles (P5-182).
use gpui::{Hsla, PathBuilder, canvas, point, prelude::*, px};

/// A crown `size` points square, stroked in `color` and filled with `fill` when there is one.
pub fn crown(color: Hsla, fill: Option<Hsla>, size: f32) -> impl IntoElement {
    const CORNERS: [(f32, f32); 7] = [
        (3.0, 8.0),
        (7.5, 12.0),
        (12.0, 5.0),
        (16.5, 12.0),
        (21.0, 8.0),
        (19.0, 19.0),
        (5.0, 19.0),
    ];
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let unit = size / 24.0;
            let at = |(x, y): (f32, f32)| bounds.origin + point(px(x * unit), px(y * unit));
            let outline = |mut path: PathBuilder| {
                path.move_to(at(CORNERS[0]));
                for &corner in &CORNERS[1..] {
                    path.line_to(at(corner));
                }
                path.close();
                path.build()
            };
            if let Some(fill) = fill
                && let Ok(path) = outline(PathBuilder::fill())
            {
                window.paint_path(path, fill);
            }
            if let Ok(path) = outline(PathBuilder::stroke(px(2.0 * unit))) {
                window.paint_path(path, color);
            }
        },
    )
    .flex_shrink_0()
    .size(px(size))
}
