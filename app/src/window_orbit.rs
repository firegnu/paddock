//! The title bar's working tabs run a light round their capsule's edge (P5-163): the whole edge a
//! pale wash of the working colour, a tail brightening towards a near-white head that goes once
//! round in about 2.5 seconds, and every working tab on the same beat.
use super::*;
use gpui::PathBuilder;

/// How long the light takes to go round once.
const PERIOD: Duration = Duration::from_millis(2500);
/// The share of the edge the brightening tail covers behind its head.
const TAIL: f32 = 0.28;
/// The short fade from the head back to the pale edge, as a share of the edge.
const AFTER: f32 = 0.03;
/// The strokes the tail and the fade are split into, enough that their steps do not show.
const BANDS: usize = 20;
const AFTER_BANDS: usize = 4;
/// How far each stroke runs back into the one before it, so no seam shows where they meet.
const OVERLAP: f32 = 0.0015;
/// The points the pale edge is sampled at.
const STEPS: usize = 72;
/// An unselected working tab's light, dimmed to this.
pub(super) const DIM: f32 = 0.6;
/// The edge's weight, a little over the capsule's ring so the light reads.
const WEIGHT: f32 = TAB_RING + 0.5;
/// The pale edge's strength, and how much of the head's near-white is the working colour.
const PALE: f32 = 0.18;
const HEAD_TINT: f32 = 0.15;
/// The frames a second its animation redraws at, as the status dots' breathing has it.
const FPS: f32 = 30.0;

/// The edge of a capsule `width` by `height` at `t` of the way round it, clockwise from its top
/// edge's left end: two half-circles joined by straight runs along the top and the bottom.
pub(super) fn edge_point(t: f32, width: f32, height: f32) -> (f32, f32) {
    let radius = (height / 2.0).min(width / 2.0);
    if radius <= 0.0 {
        return (0.0, 0.0);
    }
    let straight = (width - 2.0 * radius).max(0.0);
    let turn = std::f32::consts::PI * radius;
    let total = 2.0 * straight + 2.0 * turn;
    let mut along = t.rem_euclid(1.0) * total;
    if along <= straight {
        return (radius + along, 0.0);
    }
    along -= straight;
    if along <= turn {
        let angle = -std::f32::consts::FRAC_PI_2 + along / radius;
        return (
            width - radius + radius * angle.cos(),
            radius + radius * angle.sin(),
        );
    }
    along -= turn;
    if along <= straight {
        return (width - radius - along, height);
    }
    along -= straight;
    let angle = std::f32::consts::FRAC_PI_2 + along / radius;
    (radius + radius * angle.cos(), radius + radius * angle.sin())
}

/// The light's edge over the capsule, `id` telling this tab's light from another's: in `ink`, the
/// working colour, at `dim` of its strength (`DIM` for an unselected tab), its weight scaled by
/// `scale`.
pub(super) fn edge(id: usize, ink: Hsla, dim: f32, scale: f32) -> impl IntoElement {
    let head = head(ink);
    div()
        .absolute()
        .inset_0()
        .with_animation(("orbit", id), beat(), move |edge, phase| {
            edge.child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| paint(bounds, phase, ink, head, dim, scale, window),
                )
                .size_full(),
            )
        })
}

/// The beat every light keeps: once round in [`PERIOD`], all of them together.
pub(super) fn beat() -> Animation {
    Animation::new(PERIOD).repeat_synced().with_max_fps(FPS)
}

/// The light's near-white head, faintly in the working colour `ink`.
pub(super) fn head(ink: Hsla) -> Hsla {
    gpui::white().blend(ink.opacity(HEAD_TINT))
}

/// The edge painted `phase` of the way round, on `bounds`: the pale ring, the fade after the
/// head, and the tail running up to the head.
pub(super) fn paint(
    bounds: Bounds<Pixels>,
    phase: f32,
    ink: Hsla,
    head: Hsla,
    dim: f32,
    scale: f32,
    window: &mut Window,
) {
    let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    // The light sits on the capsule's ring, half of it either side of the padding box's edge.
    let half = TAB_RING / 2.0 * scale;
    let at = |t: f32| {
        let (x, y) = edge_point(t, width + 2.0 * half, height + 2.0 * half);
        bounds.origin + point(px(x - half), px(y - half))
    };
    let weight = px(WEIGHT * scale);
    let mut ring = PathBuilder::stroke(weight);
    for step in 0..=STEPS {
        let point = at(step as f32 / STEPS as f32);
        if step == 0 {
            ring.move_to(point);
        } else {
            ring.line_to(point);
        }
    }
    if let Ok(ring) = ring.build() {
        window.paint_path(ring, ink.opacity(PALE * dim));
    }
    let stroke = |from: f32, to: f32, color: Hsla, window: &mut Window| {
        let mut path = PathBuilder::stroke(weight);
        path.move_to(at(from));
        path.line_to(at(to));
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    };
    // A band of the light from `from` to `to` of the way round, split at the top where it wraps.
    let band = |from: f32, to: f32, color: Hsla, window: &mut Window| {
        let (a, b) = (from.rem_euclid(1.0), to.rem_euclid(1.0));
        if b <= a && to > from {
            stroke(a, 1.0, color, window);
            stroke(0.0, b, color, window);
        } else {
            stroke(a, b, color, window);
        }
    };
    lights(phase, ink, head, dim, |from, to, color| {
        band(from, to, color, window)
    });
}

/// The light run along an open line instead of round (P5-165): `at` the point `t` of the way
/// from its start to its end. The light comes in at the start, `phase` of the way through the
/// beat, and runs out past the end before it comes in again; the rest as [`paint`].
pub(super) fn paint_open(
    at: impl Fn(f32) -> Point<Pixels>,
    phase: f32,
    ink: Hsla,
    dim: f32,
    scale: f32,
    window: &mut Window,
) {
    let weight = px(WEIGHT * scale);
    let mut line = PathBuilder::stroke(weight);
    for step in 0..=STEPS {
        let point = at(step as f32 / STEPS as f32);
        if step == 0 {
            line.move_to(point);
        } else {
            line.line_to(point);
        }
    }
    if let Ok(line) = line.build() {
        window.paint_path(line, ink.opacity(PALE * dim));
    }
    // From just before the start, the fade ahead of the head unseen, to past the end with the
    // whole tail.
    let front = -AFTER + phase * (1.0 + TAIL + AFTER);
    lights(front, ink, head(ink), dim, |from, to, color| {
        let (from, to) = (from.max(0.0), to.min(1.0));
        if from < to {
            let mut path = PathBuilder::stroke(weight);
            path.move_to(at(from));
            path.line_to(at(to));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        }
    });
}

/// The light's bands with the head at `front`, each from where to where along the edge and in
/// what colour: the tail brightening up to the head, then the short fade after it.
fn lights(front: f32, ink: Hsla, head: Hsla, dim: f32, mut band: impl FnMut(f32, f32, Hsla)) {
    for step in 0..BANDS {
        let behind = step as f32 / BANDS as f32;
        let share = (step + 1) as f32 / BANDS as f32;
        let from = front - TAIL * (1.0 - behind) - OVERLAP;
        let to = front - TAIL * (1.0 - share);
        let color = ink.blend(head.opacity(share)).opacity(share * dim);
        band(from, to, color);
    }
    for step in 0..AFTER_BANDS {
        let behind = step as f32 / AFTER_BANDS as f32;
        let share = (step + 1) as f32 / AFTER_BANDS as f32;
        let from = front + AFTER * behind - OVERLAP;
        let to = front + AFTER * share;
        let color = ink
            .blend(head.opacity(1.0 - share))
            .opacity((1.0 - share) * dim);
        band(from, to, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn the_edge_starts_at_the_top_s_left_end_turns_and_comes_back() {
        // A 200 by 26 capsule: half the height is the ends' radius.
        let (x, y) = edge_point(0.0, 200.0, 26.0);
        assert!(near(x, 13.0) && near(y, 0.0), "{x}, {y}");
        // Half way round the right end, level with its centre.
        let straight = 200.0 - 26.0;
        let half_turn = straight + std::f32::consts::PI * 13.0 / 2.0;
        let total = 2.0 * straight + 2.0 * std::f32::consts::PI * 13.0;
        let (x, y) = edge_point(half_turn / total, 200.0, 26.0);
        assert!(near(x, 200.0) && near(y, 13.0), "{x}, {y}");
        // Half a lap from the start: the bottom edge's right end.
        let (x, y) = edge_point(0.5, 200.0, 26.0);
        assert!(near(x, 187.0) && near(y, 26.0), "{x}, {y}");
        // A whole lap is the start again.
        let (x, y) = edge_point(1.0, 200.0, 26.0);
        assert!(near(x, 13.0) && near(y, 0.0), "{x}, {y}");
    }
}
