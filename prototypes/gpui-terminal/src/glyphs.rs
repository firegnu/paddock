//! Box-drawing lines and block elements drawn as rectangles on the cell, so neighbouring cells
//! join without the gaps font glyphs leave. Other characters (double, dashed, diagonal lines,
//! shades are approximated by alpha) still go through the font.

/// A rectangle in pixels relative to the cell's top-left corner, with an opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub alpha: f32,
}

pub fn pieces(c: char, w: f32, h: f32) -> Option<Vec<Piece>> {
    let code = c as u32;
    match code {
        0x2580..=0x259F => Some(block(code, w, h)),
        _ => lines(c, w, h),
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Piece {
    Piece {
        x,
        y,
        w,
        h,
        alpha: 1.0,
    }
}

/// Block elements, with edges rounded to whole pixels.
fn block(code: u32, w: f32, h: f32) -> Vec<Piece> {
    let half_w = (w / 2.0).round();
    let half_h = (h / 2.0).round();
    // Lower `n` eighths and left `n` eighths.
    let lower = |n: f32| {
        let top = (h * (1.0 - n / 8.0)).round();
        rect(0.0, top, w, h - top)
    };
    let left = |n: f32| rect(0.0, 0.0, (w * n / 8.0).round().max(1.0), h);
    let quadrant = |right: bool, bottom: bool| {
        let (x, cw) = if right {
            (half_w, w - half_w)
        } else {
            (0.0, half_w)
        };
        let (y, ch) = if bottom {
            (half_h, h - half_h)
        } else {
            (0.0, half_h)
        };
        rect(x, y, cw, ch)
    };
    let quads = |list: &[(bool, bool)]| list.iter().map(|&(r, b)| quadrant(r, b)).collect();
    let (ul, ur, ll, lr) = ((false, false), (true, false), (false, true), (true, true));
    match code {
        0x2580 => vec![rect(0.0, 0.0, w, half_h)],
        0x2581..=0x2588 => vec![lower((code - 0x2580) as f32)],
        0x2589..=0x258F => vec![left((0x2590 - code) as f32)],
        0x2590 => vec![rect(half_w, 0.0, w - half_w, h)],
        0x2591..=0x2593 => {
            vec![Piece {
                alpha: (code - 0x2590) as f32 * 0.25,
                ..rect(0.0, 0.0, w, h)
            }]
        }
        0x2594 => vec![rect(0.0, 0.0, w, (h / 8.0).round().max(1.0))],
        0x2595 => {
            let x = w - (w / 8.0).round().max(1.0);
            vec![rect(x, 0.0, w - x, h)]
        }
        0x2596 => quads(&[ll]),
        0x2597 => quads(&[lr]),
        0x2598 => quads(&[ul]),
        0x2599 => quads(&[ul, ll, lr]),
        0x259A => quads(&[ul, lr]),
        0x259B => quads(&[ul, ur, ll]),
        0x259C => quads(&[ul, ur, lr]),
        0x259D => quads(&[ur]),
        0x259E => quads(&[ur, ll]),
        _ => quads(&[ur, ll, lr]),
    }
}

/// Light and heavy straight lines, corners, tees and crosses. Each arm runs from the cell edge to
/// the far side of the centre line, so arms of one cell overlap and neighbours join.
fn lines(c: char, w: f32, h: f32) -> Option<Vec<Piece>> {
    // Arms: up, right, down, left. 1 = light, 2 = heavy.
    let (up, right, down, left) = match c {
        '─' => (0, 1, 0, 1),
        '━' => (0, 2, 0, 2),
        '│' => (1, 0, 1, 0),
        '┃' => (2, 0, 2, 0),
        '┌' | '╭' => (0, 1, 1, 0),
        '┐' | '╮' => (0, 0, 1, 1),
        '└' | '╰' => (1, 1, 0, 0),
        '┘' | '╯' => (1, 0, 0, 1),
        '┏' => (0, 2, 2, 0),
        '┓' => (0, 0, 2, 2),
        '┗' => (2, 2, 0, 0),
        '┛' => (2, 0, 0, 2),
        '├' => (1, 1, 1, 0),
        '┤' => (1, 0, 1, 1),
        '┬' => (0, 1, 1, 1),
        '┴' => (1, 1, 0, 1),
        '┼' => (1, 1, 1, 1),
        '┣' => (2, 2, 2, 0),
        '┫' => (2, 0, 2, 2),
        '┳' => (0, 2, 2, 2),
        '┻' => (2, 2, 0, 2),
        '╋' => (2, 2, 2, 2),
        '╴' => (0, 0, 0, 1),
        '╵' => (1, 0, 0, 0),
        '╶' => (0, 1, 0, 0),
        '╷' => (0, 0, 1, 0),
        _ => return None,
    };
    let light = (w / 8.0).round().max(1.0);
    let thickness = |weight: u8| light * f32::from(weight);
    // Centre lines on whole pixels; the same in every cell of a row or column.
    let line_y = |t: f32| ((h - t) / 2.0).floor();
    let line_x = |t: f32| ((w - t) / 2.0).floor();
    let mut out = Vec::new();
    let horizontal = |weight: u8, from: f32, to: f32| {
        let t = thickness(weight);
        rect(from, line_y(t), to - from, t)
    };
    let vertical = |weight: u8, from: f32, to: f32| {
        let t = thickness(weight);
        rect(line_x(t), from, t, to - from)
    };
    // A straight line through the cell is one piece.
    if left > 0 && left == right && up == 0 && down == 0 {
        return Some(vec![horizontal(left, 0.0, w)]);
    }
    if up > 0 && up == down && left == 0 && right == 0 {
        return Some(vec![vertical(up, 0.0, h)]);
    }
    let reach_x = |t: f32| line_x(t) + t;
    let reach_y = |t: f32| line_y(t) + t;
    let cross_h = thickness(up.max(down)).max(1.0);
    let cross_v = thickness(left.max(right)).max(1.0);
    if left > 0 {
        out.push(horizontal(left, 0.0, reach_x(cross_h)));
    }
    if right > 0 {
        out.push(horizontal(right, line_x(cross_h), w));
    }
    if up > 0 {
        out.push(vertical(up, 0.0, reach_y(cross_v)));
    }
    if down > 0 {
        out.push(vertical(down, line_y(cross_v), h));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    const W: f32 = 8.0;
    const H: f32 = 17.0;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Piece {
        Piece {
            x,
            y,
            w,
            h,
            alpha: 1.0,
        }
    }
    fn area(pieces: &[Piece]) -> f32 {
        pieces.iter().map(|p| p.w * p.h).sum()
    }

    #[test]
    fn text_is_left_to_the_font() {
        assert_eq!(pieces('a', W, H), None);
        assert_eq!(pieces('中', W, H), None);
        assert_eq!(pieces('═', W, H), None);
    }

    #[test]
    fn blocks_cover_the_cell_on_whole_pixels() {
        assert_eq!(pieces('█', W, H).unwrap(), [rect(0.0, 0.0, W, H)]);
        assert_eq!(pieces('▀', W, H).unwrap(), [rect(0.0, 0.0, W, 9.0)]);
        assert_eq!(pieces('▄', W, H).unwrap(), [rect(0.0, 9.0, W, 8.0)]);
        assert_eq!(pieces('▌', W, H).unwrap(), [rect(0.0, 0.0, 4.0, H)]);
        assert_eq!(pieces('▐', W, H).unwrap(), [rect(4.0, 0.0, 4.0, H)]);
        assert_eq!(pieces('▁', W, H).unwrap(), [rect(0.0, 15.0, W, 2.0)]);
        assert_eq!(pieces('▏', W, H).unwrap(), [rect(0.0, 0.0, 1.0, H)]);
    }

    #[test]
    fn quadrants_combine() {
        let top = pieces('▛', W, H).unwrap();
        assert_eq!(top.len(), 3);
        assert!(top.contains(&rect(0.0, 0.0, 4.0, 9.0)));
        assert!(top.contains(&rect(4.0, 0.0, 4.0, 9.0)));
        assert!(top.contains(&rect(0.0, 9.0, 4.0, 8.0)));
        assert_eq!(area(&pieces('▚', W, H).unwrap()), 4.0 * 9.0 + 4.0 * 8.0);
    }

    #[test]
    fn shades_are_translucent_full_cells() {
        let shade = pieces('▒', W, H).unwrap();
        assert_eq!(shade.len(), 1);
        assert_eq!((shade[0].w, shade[0].h), (W, H));
        assert!(shade[0].alpha > 0.0 && shade[0].alpha < 1.0);
    }

    #[test]
    fn lines_span_the_cell_and_meet_at_the_centre() {
        let horizontal = pieces('─', W, H).unwrap();
        assert_eq!(horizontal.len(), 1);
        assert_eq!((horizontal[0].x, horizontal[0].w), (0.0, W));
        let vertical = pieces('│', W, H).unwrap();
        assert_eq!((vertical[0].y, vertical[0].h), (0.0, H));
        // Lines of neighbouring cells sit at the same offset, so they join.
        assert_eq!(horizontal[0].y, pieces('┼', W, H).unwrap()[0].y);
        let heavy = pieces('━', W, H).unwrap();
        assert!(heavy[0].h > horizontal[0].h);
    }

    #[test]
    fn corners_have_two_arms_touching_the_edges() {
        let corner = pieces('┌', W, H).unwrap();
        assert_eq!(corner.len(), 2);
        assert!(corner.iter().any(|p| p.x + p.w == W));
        assert!(corner.iter().any(|p| p.y + p.h == H));
        assert!(corner.iter().all(|p| p.x > 0.0 || p.y > 0.0));
        // Rounded corners are drawn square so they line up with the drawn lines.
        assert_eq!(pieces('╭', W, H), pieces('┌', W, H));
        assert_eq!(pieces('┼', W, H).unwrap().len(), 4);
    }
}
