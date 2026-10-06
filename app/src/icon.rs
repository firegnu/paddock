//! The app icon: a white cat's head, split into cyan and magenta at the edges, on a black rounded
//! square in the macOS icon grid. Drawn from the P5-20 design
//! (`docs/设计稿/P5-20-应用图标/Icon.dc.html`, ground `black`, split 1, glow on); every layer is
//! the design's, in its 1024-unit square. `examples/bundle.rs` writes it into `paddock.app`.
use std::f32::consts::PI;

type Point = (f32, f32);
type Rgb = [u8; 3];

/// The square's vertical gradient, top and bottom.
const GROUND_TOP: Rgb = [0x1c, 0x1c, 0x21];
const GROUND_BOTTOM: Rgb = [0x05, 0x05, 0x06];
const WHITE: Rgb = [0xff, 0xff, 0xff];
const HEAD: Rgb = [0xf7, 0xf7, 0xf9];
const MAGENTA: Rgb = [0xff, 0x3b, 0xd4];
const CYAN: Rgb = [0x29, 0xe6, 0xff];

/// Opacity of the white sheen at the top, fading out a little over a third of the way down.
const SHEEN: f32 = 0.07;
const SHEEN_DEPTH: f32 = 0.35;
/// Opacity of the inner rim.
const RIM: f32 = 0.09;
/// Opacity of the glow under the head, and its blur in design units.
const GLOW: f32 = 0.26;
const GLOW_BLUR: f32 = 26.0;
/// Opacity of the cyan and magenta copies, and how far they move, in head units.
const SPLIT_OPACITY: f32 = 0.92;
const SPLIT: f32 = 1.0;
/// At this size and below the split and the glow are left out and the head sits on whole pixels.
const SMALL: u32 = 32;

/// The head's outline in its 100-unit box, as in the design's `head` path.
enum Step {
    Line(Point),
    /// A quadratic curve: control point, end point.
    Quad(Point, Point),
}
const START: Point = (20.0, 66.0);
const OUTLINE: [Step; 14] = [
    Step::Line((20.0, 40.0)),
    Step::Line((22.8, 21.0)),
    Step::Quad((24.0, 13.0), (30.0, 18.0)),
    Step::Line((41.0, 28.5)),
    Step::Quad((43.0, 30.5), (46.0, 30.5)),
    Step::Line((54.0, 30.5)),
    Step::Quad((57.0, 30.5), (59.0, 28.5)),
    Step::Line((70.0, 18.0)),
    Step::Quad((76.0, 13.0), (77.2, 21.0)),
    Step::Line((80.0, 40.0)),
    Step::Line((80.0, 66.0)),
    Step::Quad((80.0, 81.0), (65.0, 81.0)),
    Step::Line((35.0, 81.0)),
    Step::Quad((20.0, 81.0), (20.0, 66.0)),
];
/// The eyes, cut out of the head: centres and radii in head units.
const EYES: [Point; 2] = [(38.5, 56.0), (61.5, 56.0)];
const EYE_RADII: Point = (4.6, 7.0);
/// The head's sides, and the tops of its ears, the notch between them and its chin, in head units.
const SIDES: [f32; 2] = [20.0, 80.0];
const EAR_TIP: f32 = 16.08;
const NOTCH: f32 = 30.5;
const CHIN: f32 = 81.0;

/// Where the head's box goes in the design's 1024 square: `translate(197 226) scale(6.3)`.
fn place(size: u32, (x, y): Point) -> Point {
    let k = size as f32 / 1024.0;
    ((197.0 + 6.3 * x) * k, (226.0 + 6.3 * y) * k)
}

/// An RGBA image `size` pixels square: transparent outside the rounded square, which takes
/// 824/1024 of the width like Apple's app icon grid.
pub fn icon(size: u32) -> Vec<u8> {
    let n = size as usize;
    let k = size as f32 / 1024.0;
    let small = size <= SMALL;

    let white = if small {
        fill(&snapped_head(size), n)
    } else {
        fill(&head(size, (0.0, 0.0)), n)
    };
    let (glow, magenta, cyan) = if small {
        (None, None, None)
    } else {
        (
            Some(blur(&white, n, GLOW_BLUR * k)),
            Some(fill(&head(size, (SPLIT, SPLIT * 0.35)), n)),
            Some(fill(&head(size, (-SPLIT, 0.0)), n)),
        )
    };

    let mut rgba = vec![0u8; n * n * 4];
    for py in 0..n {
        for px in 0..n {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let at = py * n + px;
            // Premultiplied colour and alpha, each layer laid over the last.
            let mut pixel = [0.0f32; 4];
            let square = rounded_square(size, (x, y), 512.0, 412.0, 185.0);
            let inside = (0.5 - square).clamp(0.0, 1.0);
            let t = ((y / k - 100.0) / 824.0).clamp(0.0, 1.0);
            over(&mut pixel, mix(GROUND_TOP, GROUND_BOTTOM, t), inside);
            let sheen = SHEEN * (1.0 - t / SHEEN_DEPTH).max(0.0);
            over(&mut pixel, color(WHITE), sheen * inside);
            // A 4-unit stroke along a square 2 units in.
            let rim = rounded_square(size, (x, y), 512.0, 410.0, 183.0);
            let half = 2.0 * k;
            let band = ((rim + 0.5).min(half) - (rim - 0.5).max(-half)).max(0.0);
            over(&mut pixel, color(WHITE), RIM * band);
            if let Some(glow) = &glow {
                over(&mut pixel, color(WHITE), GLOW * glow[at]);
            }
            if let Some(magenta) = &magenta {
                over(&mut pixel, color(MAGENTA), SPLIT_OPACITY * magenta[at]);
            }
            if let Some(cyan) = &cyan {
                over(&mut pixel, color(CYAN), SPLIT_OPACITY * cyan[at]);
            }
            over(&mut pixel, color(HEAD), white[at]);

            let alpha = pixel[3];
            if alpha > 0.0 {
                let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                rgba[at * 4..at * 4 + 4].copy_from_slice(&[
                    byte(pixel[0] / alpha),
                    byte(pixel[1] / alpha),
                    byte(pixel[2] / alpha),
                    byte(alpha),
                ]);
            }
        }
    }
    rgba
}

/// The head moved by `offset` head units, outline and eyes as polygons in pixels.
fn head(size: u32, (ox, oy): Point) -> Vec<Vec<Point>> {
    let map = |(x, y): Point| place(size, (x + ox, y + oy));
    let mut contours = vec![outline(map)];
    for (cx, cy) in EYES {
        let (rx, ry) = EYE_RADII;
        let steps = 96;
        contours.push(
            (0..steps)
                .map(|i| {
                    let a = 2.0 * PI * i as f32 / steps as f32;
                    map((cx + rx * a.cos(), cy + ry * a.sin()))
                })
                .collect(),
        );
    }
    contours
}

/// The head for small sizes: its sides, ear tips, notch and chin on whole pixels, the outline
/// stretched to match, and the eyes as whole-pixel cut-outs at least one pixel wide and taller
/// than wide.
fn snapped_head(size: u32) -> Vec<Vec<Point>> {
    let centre = size as f32 / 2.0;
    // Rounding about the centre keeps the two sides mirror images.
    let snap_x = |x: f32| centre + (place(size, (x, 0.0)).0 - centre).round();
    let snap_y = |y: f32| place(size, (0.0, y)).1.round();
    let xs: Vec<Point> = SIDES.iter().map(|&x| (x, snap_x(x))).collect();
    let mut ys: Vec<Point> = Vec::new();
    for y in [EAR_TIP, NOTCH, CHIN] {
        let floor = ys.last().map_or(f32::MIN, |&(_, v)| v + 1.0);
        ys.push((y, snap_y(y).max(floor)));
    }
    let mut contours = vec![outline(|(x, y)| (along(&xs, x), along(&ys, y)))];

    let ((cx, cy), (rx, ry)) = (EYES[0], EYE_RADII);
    let left = snap_x(cx - rx);
    let right = snap_x(cx + rx).max(left + 1.0);
    let top = snap_y(cy - ry);
    let bottom = snap_y(cy + ry).max(top + (right - left) + 1.0);
    for (a, b) in [(left, right), (2.0 * centre - right, 2.0 * centre - left)] {
        contours.push(vec![(a, top), (b, top), (b, bottom), (a, bottom)]);
    }
    contours
}

/// The outline flattened into a polygon, each point passed through `map`.
fn outline(map: impl Fn(Point) -> Point) -> Vec<Point> {
    let mut points = vec![map(START)];
    let mut from = START;
    for step in &OUTLINE {
        match *step {
            Step::Line(to) => {
                points.push(map(to));
                from = to;
            }
            Step::Quad(control, to) => {
                let steps = 32;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t);
                    points.push(map((
                        a * from.0 + b * control.0 + c * to.0,
                        a * from.1 + b * control.1 + c * to.1,
                    )));
                }
                from = to;
            }
        }
    }
    points
}

/// Maps `v` through the line joining `keys` (from, to), extending the end pieces.
fn along(keys: &[Point], v: f32) -> f32 {
    let piece = keys
        .windows(2)
        .position(|w| v <= w[1].0)
        .unwrap_or(keys.len() - 2);
    let ((a, fa), (b, fb)) = (keys[piece], keys[piece + 1]);
    fa + (v - a) * (fb - fa) / (b - a)
}

/// How much of each pixel `contours` cover, 0 to 1, under the even-odd rule: exact across each
/// row, sixteen samples down it.
fn fill(contours: &[Vec<Point>], n: usize) -> Vec<f32> {
    const ROWS: usize = 16;
    let edges: Vec<(Point, Point)> = contours
        .iter()
        .flat_map(|c| c.iter().copied().zip(c.iter().copied().cycle().skip(1)))
        .collect();
    let mut cover = vec![0.0f32; n * n];
    let mut xs = Vec::new();
    for py in 0..n {
        let row = &mut cover[py * n..(py + 1) * n];
        for s in 0..ROWS {
            let y = py as f32 + (s as f32 + 0.5) / ROWS as f32;
            xs.clear();
            for &((x0, y0), (x1, y1)) in &edges {
                if (y0 <= y) != (y1 <= y) {
                    xs.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for span in xs.chunks_exact(2) {
                let (a, b) = (span[0].clamp(0.0, n as f32), span[1].clamp(0.0, n as f32));
                let mut px = a.floor() as usize;
                while (px as f32) < b {
                    let overlap = b.min(px as f32 + 1.0) - a.max(px as f32);
                    row[px] += overlap / ROWS as f32;
                    px += 1;
                }
            }
        }
    }
    for c in &mut cover {
        *c = c.min(1.0);
    }
    cover
}

/// SVG's `feGaussianBlur` of an `n`-square image: three box blurs each way, sized as the SVG
/// specification describes.
fn blur(image: &[f32], n: usize, sigma: f32) -> Vec<f32> {
    let d = (sigma * 3.0 * (2.0 * PI).sqrt() / 4.0 + 0.5).floor() as usize;
    let mut image = image.to_vec();
    if d == 0 {
        return image;
    }
    // (width, how many pixels of the window lie before the one written)
    let passes = if d % 2 == 1 {
        [(d, d / 2); 3]
    } else {
        [(d, d / 2), (d, d / 2 - 1), (d + 1, d / 2)]
    };
    let mut line = vec![0.0f32; n];
    let mut sums = vec![0.0f32; n + 1];
    for across in [true, false] {
        for i in 0..n {
            let at = |j: usize| if across { i * n + j } else { j * n + i };
            for j in 0..n {
                line[j] = image[at(j)];
            }
            for (width, before) in passes {
                for j in 0..n {
                    sums[j + 1] = sums[j] + line[j];
                }
                for (j, value) in line.iter_mut().enumerate() {
                    let from = j.saturating_sub(before);
                    let to = (j + width - before).min(n);
                    *value = (sums[to] - sums[from]) / width as f32;
                }
            }
            for j in 0..n {
                image[at(j)] = line[j];
            }
        }
    }
    image
}

/// Signed distance in pixels from `point` to a rounded square given in design units (centre,
/// half its side, corner radius): negative inside.
fn rounded_square(size: u32, (x, y): Point, centre: f32, half: f32, radius: f32) -> f32 {
    let k = size as f32 / 1024.0;
    let (centre, half, radius) = (centre * k, half * k, radius * k);
    let qx = (x - centre).abs() - (half - radius);
    let qy = (y - centre).abs() - (half - radius);
    let out = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    out + qx.max(qy).min(0.0) - radius
}

fn color(rgb: Rgb) -> [f32; 3] {
    rgb.map(|c| c as f32 / 255.0)
}

fn mix(a: Rgb, b: Rgb, t: f32) -> [f32; 3] {
    let (a, b) = (color(a), color(b));
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Lays `rgb` at `alpha` over a premultiplied pixel.
fn over(pixel: &mut [f32; 4], rgb: [f32; 3], alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    for i in 0..3 {
        pixel[i] = rgb[i] * alpha + pixel[i] * (1.0 - alpha);
    }
    pixel[3] = alpha + pixel[3] * (1.0 - alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * size + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    fn at(size: u32, point: Point) -> (u32, u32) {
        let (x, y) = place(size, point);
        (x as u32, y as u32)
    }

    #[test]
    fn the_head_sits_on_a_black_rounded_square() {
        let size = 1024;
        let rgba = icon(size);
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        // Outside the square and in its rounded corner: transparent.
        assert_eq!(pixel(&rgba, size, 10, 10)[3], 0);
        assert_eq!(pixel(&rgba, size, 110, 110)[3], 0);
        // Inside near an edge: the dark ground, opaque.
        let [r, g, b, a] = pixel(&rgba, size, 512, 140);
        assert_eq!(a, 255);
        assert!(r < 0x30 && g < 0x30 && b < 0x30, "{r} {g} {b}");
        // The middle of the head is white.
        let (x, y) = at(size, (50.0, 70.0));
        assert_eq!(pixel(&rgba, size, x, y), [0xf7, 0xf7, 0xf9, 255]);
        // The eyes show the ground through.
        for eye in EYES {
            let (x, y) = at(size, eye);
            let [r, g, b, a] = pixel(&rgba, size, x, y);
            assert_eq!(a, 255);
            assert!(r < 0x50 && g < 0x50 && b < 0x50, "eye {r} {g} {b}");
        }
        // Cyan shows past the left side, magenta past the right.
        let (x, y) = at(size, (19.6, 60.0));
        let [r, _, b, _] = pixel(&rgba, size, x, y);
        assert!(b > 0xc0 && r < 0x80, "left {r} {b}");
        let (x, y) = at(size, (80.4, 60.0));
        let [r, g, _, _] = pixel(&rgba, size, x, y);
        assert!(r > 0xc0 && g < 0x80, "right {r} {g}");
    }

    #[test]
    fn small_icons_stay_grey_and_crisp() {
        for size in [16, 32] {
            let rgba = icon(size);
            // No split at this size: every pixel is a grey.
            for p in rgba.chunks(4) {
                assert!(p[0].abs_diff(p[2]) < 8, "{size}: {p:?}");
            }
            // The head's middle is solid white, its eyes the ground.
            let (x, y) = at(size, (50.0, 70.0));
            assert_eq!(pixel(&rgba, size, x, y), [0xf7, 0xf7, 0xf9, 255], "{size}");
            let (x, y) = at(size, EYES[0]);
            let [r, _, _, a] = pixel(&rgba, size, x, y);
            assert!(a == 255 && r < 0x30, "{size}: eye {r}");
        }
    }

    #[test]
    fn every_icon_size_has_the_head() {
        for size in [16, 32, 64, 128, 256, 512, 1024] {
            let rgba = icon(size);
            let opaque = rgba.chunks(4).filter(|p| p[3] == 255).count();
            assert!(opaque > (size * size / 3) as usize, "{size}: {opaque}");
            let white = rgba
                .chunks(4)
                .filter(|p| p[..3] == [0xf7, 0xf7, 0xf9])
                .count();
            assert!(white > (size * size / 20) as usize, "{size}: {white} white");
        }
    }
}
