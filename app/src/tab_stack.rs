//! A split tab's mark in the title bar (DESIGN §13 P5-76): a card for each of its panes, stacked,
//! the active pane's in front and leaning a little to the left, the others upright behind it,
//! each a step to the right of the one before.
use crate::{footer_icon, kind_icon::KindIcon, layout::PaneId};
use gpui::{
    AnyElement, Div, Hsla, PathBuilder, Pixels, Point, black, canvas, div, point, prelude::*, px,
};

/// The most cards a stack shows: the front one and two behind it.
pub const MOST: usize = 3;
/// A card: its side, its corners and the dark edge round it; how far each card behind the front
/// one stands to the right of the one before it; how far the front one leans, clockwise in
/// degrees; how much darker the last of three is; and how big what a card shows on its ground is
/// (a silhouette, an original that is not a tile, the terminal). In points.
const CARD: f32 = 18.0;
const RADIUS: f32 = 5.0;
const EDGE: f32 = 1.5;
const STEP: f32 = 4.0;
const LEAN: f32 = -6.0;
const LAST_DARKER: f32 = 0.2;
const GLYPH: f32 = 12.0;
/// The stack's height: a card and a point above and below it.
const HEIGHT: f32 = 20.0;
/// A shell's card, in front and behind: how far its ground is from the edge's colour towards the
/// light one.
const SHELL: [f32; 2] = [0.28, 0.35];
/// The terminal on a shell's card, drawn as `footer_icon`'s `NewShell` is: its line's width, in
/// that icon's points.
const TERMINAL_LINE: f32 = 1.25;

/// The panes whose cards a tab holding `panes` (as they lie, left to right and top to bottom)
/// stacks, the front one first: the `active` pane's, then the others' as they lie, [`MOST`] in
/// all; none for a tab of one pane, which keeps its plain icon.
pub fn cards(panes: &[PaneId], active: PaneId) -> Vec<PaneId> {
    if panes.len() < 2 {
        return Vec::new();
    }
    let behind = panes.iter().copied().filter(|&pane| pane != active);
    std::iter::once(active).chain(behind).take(MOST).collect()
}

/// How wide the stack of a tab holding `panes` panes is, in points.
pub fn width(panes: usize) -> f32 {
    CARD + STEP * (panes.clamp(1, MOST) - 1) as f32
}

/// What a pane's card shows.
#[derive(Clone, Copy)]
pub enum Face {
    /// An agent of a kind with an icon, and the kind's colour.
    Kind(KindIcon, Hsla),
    Shell,
    /// An empty pane, or an agent of a kind without an icon: this colour and nothing on it.
    Blank(Hsla),
}

/// The colours a stack takes from the theme: the cards' dark edge, and the light one of a ground
/// under an original's glyph, or of a silhouette or the terminal on a darker ground.
#[derive(Clone, Copy)]
pub struct Inks {
    pub edge: Hsla,
    pub light: Hsla,
}

impl Face {
    /// The card's ground in front: under a silhouette its kind's colour, under an original that
    /// is not a tile the light one; a tile covers its own, the edge's colour.
    fn front(self, inks: Inks) -> Hsla {
        match self {
            Face::Kind(icon, _) if icon.tile() => inks.edge,
            Face::Kind(icon, _) if icon.original() => inks.light,
            Face::Kind(_, color) | Face::Blank(color) => color,
            Face::Shell => inks.edge.blend(inks.light.opacity(SHELL[0])),
        }
    }

    /// The card's colour behind, where it shows only its edge: a tile's own colour, else its
    /// kind's.
    fn behind(self, inks: Inks) -> Hsla {
        match self {
            Face::Kind(icon, color) => icon.tint().unwrap_or(color),
            Face::Blank(color) => color,
            Face::Shell => inks.edge.blend(inks.light.opacity(SHELL[1])),
        }
    }

    /// What the front card shows of an agent's kind, leaning as the card does: a tile whole, a
    /// silhouette in the light colour, another original in its own.
    fn shown(self, inks: Inks, scale: f32) -> Option<AnyElement> {
        match self {
            Face::Kind(icon, _) if icon.tile() => {
                Some(icon.render_tile_turned(px(CARD * scale), px(RADIUS * scale), LEAN))
            }
            Face::Kind(icon, _) => {
                let height = px(GLYPH * scale / icon.width.max(1.0));
                Some(icon.render_turned(height, inks.light, LEAN))
            }
            Face::Shell | Face::Blank(_) => None,
        }
    }
}

/// The stack for `faces`, the front card's first (see [`cards`]), with `mark` laid over the front
/// card's box as its child, to hang the status dot from; `scale` is the interface's size over its
/// base.
pub fn stack(faces: &[Face], inks: Inks, mark: AnyElement, scale: f32) -> Div {
    let front = faces[0];
    let grounds: Vec<Hsla> = faces
        .iter()
        .enumerate()
        .map(|(card, face)| match card {
            0 => face.front(inks),
            _ if card + 1 == MOST => face.behind(inks).blend(black().opacity(LAST_DARKER)),
            _ => face.behind(inks),
        })
        .collect();
    let cards = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let centre = |card: usize| {
                let x = CARD / 2.0 + STEP * card as f32;
                bounds.origin + point(px(x * scale), px(HEIGHT / 2.0 * scale))
            };
            let mut paint = |shape: PathBuilder, color: Hsla| {
                if let Ok(path) = shape.build() {
                    window.paint_path(path, color);
                }
            };
            // From the back, each its edge and then its ground over that.
            for (card, &ground) in grounds.iter().enumerate().rev() {
                let lean = if card == 0 { LEAN } else { 0.0 };
                for (side, radius, color) in [
                    (CARD + 2.0 * EDGE, RADIUS + EDGE, inks.edge),
                    (CARD, RADIUS, ground),
                ] {
                    let mut shape = PathBuilder::fill();
                    let side = side * scale;
                    rounded(&mut shape, centre(card), (side, side), radius * scale, lean);
                    paint(shape, color);
                }
            }
            if matches!(front, Face::Shell) {
                for line in terminal(centre(0), GLYPH / footer_icon::SIZE * scale) {
                    paint(line, inks.light);
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    div()
        .relative()
        .flex_shrink_0()
        .w(px(width(faces.len()) * scale))
        .h(px(HEIGHT * scale))
        .child(cards)
        .child(
            div()
                .absolute()
                .left_0()
                .top(px((HEIGHT - CARD) / 2.0 * scale))
                .size(px(CARD * scale))
                .flex()
                .items_center()
                .justify_center()
                .children(front.shown(inks, scale))
                .child(mark),
        )
}

/// `(x, y)` from `centre`, turned clockwise by `degrees` about it.
fn turned(centre: Point<Pixels>, degrees: f32, x: f32, y: f32) -> Point<Pixels> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    centre + point(px(x * cos - y * sin), px(x * sin + y * cos))
}

/// A closed rectangle `width` by `height` about `centre` with corners of radius `r`, turned
/// clockwise by `degrees`.
fn rounded(
    path: &mut PathBuilder,
    centre: Point<Pixels>,
    (width, height): (f32, f32),
    r: f32,
    degrees: f32,
) {
    let (w, h) = (width / 2.0, height / 2.0);
    let at = |(x, y): (f32, f32)| turned(centre, degrees, x, y);
    let radii = point(px(r), px(r));
    path.move_to(at((r - w, -h)));
    for (side, corner) in [
        ((w - r, -h), (w, r - h)),
        ((w, h - r), (w - r, h)),
        ((r - w, h), (-w, h - r)),
        ((-w, r - h), (r - w, -h)),
    ] {
        path.line_to(at(side));
        path.arc_to(radii, px(0.0), false, true, at(corner));
    }
    path.close();
}

/// The terminal a shell's card shows, about `centre` and leaning as the front card does: its
/// frame, and its prompt and line; `unit` is one of the icon's points.
fn terminal(centre: Point<Pixels>, unit: f32) -> [PathBuilder; 2] {
    let stroke = || PathBuilder::stroke(px(TERMINAL_LINE * unit));
    let at = |x: f32, y: f32| turned(centre, LEAN, x * unit, y * unit);
    let mut frame = stroke();
    rounded(
        &mut frame,
        centre,
        (12.0 * unit, 10.0 * unit),
        2.0 * unit,
        LEAN,
    );
    let mut prompt = stroke();
    prompt.move_to(at(-3.25, -2.0));
    prompt.line_to(at(-1.0, 0.0));
    prompt.line_to(at(-3.25, 2.0));
    prompt.move_to(at(0.25, 2.25));
    prompt.line_to(at(3.5, 2.25));
    [frame, prompt]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_active_pane_s_card_comes_first_then_the_others_as_they_lie() {
        assert_eq!(cards(&[1, 2, 3], 1), [1, 2, 3]);
        assert_eq!(cards(&[1, 2, 3], 2), [2, 1, 3]);
        assert_eq!(cards(&[1, 2, 3], 3), [3, 1, 2]);
        assert_eq!(cards(&[4, 9], 9), [9, 4]);
    }

    #[test]
    fn a_stack_shows_three_cards_at_most_the_active_pane_s_among_them() {
        assert_eq!(cards(&[1, 2, 3, 4, 5], 1), [1, 2, 3]);
        assert_eq!(cards(&[1, 2, 3, 4, 5], 5), [5, 1, 2]);
        assert_eq!(cards(&[1, 2, 3, 4], 3), [3, 1, 2]);
    }

    #[test]
    fn a_tab_of_one_pane_has_no_stack() {
        assert!(cards(&[7], 7).is_empty());
    }

    #[test]
    fn the_stack_widens_by_a_step_for_each_card_behind_up_to_two() {
        assert_eq!(width(2), 22.0);
        assert_eq!(width(3), 26.0);
        assert_eq!(width(5), 26.0);
    }
}
