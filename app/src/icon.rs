//! The app icon: the cat's standing pose, pixel for pixel, on a rounded dark square in the macOS
//! icon grid. `examples/bundle.rs` writes it into `paddock.app`.
use crate::pet::{Pet, Rgb};

/// The icon's background, Dune's Agents panel colour.
const BACKGROUND: Rgb = (0x1d, 0x1a, 0x16);
/// A faint rim, so the square reads on dark docks.
const RIM: Rgb = (0x3a, 0x35, 0x2e);

/// An RGBA image `size` pixels square: transparent outside the rounded square, which takes
/// 824/1024 of the width like Apple's app icon grid; the cat sits in the middle at a whole
/// number of icon pixels per picture pixel, so it stays crisp.
pub fn icon(size: u32) -> Vec<u8> {
    let pack = Pet::Cat.pack();
    let image = pack.image("stand").expect("the cat stands");
    let (width, height) = (pack.width, pack.height);
    // The cat's own bounds inside its picture.
    let solid = |x: usize, y: usize| image[y * width + x].is_some();
    let xs: Vec<usize> = (0..width)
        .filter(|&x| (0..height).any(|y| solid(x, y)))
        .collect();
    let ys: Vec<usize> = (0..height)
        .filter(|&y| (0..width).any(|x| solid(x, y)))
        .collect();
    let (left, right) = (xs[0], *xs.last().unwrap());
    let (top, bottom) = (ys[0], *ys.last().unwrap());
    let (cat_w, cat_h) = ((right - left + 1) as u32, (bottom - top + 1) as u32);

    let s = size as f32;
    let square = s * 824.0 / 1024.0;
    let inset = (s - square) / 2.0;
    let radius = square * 0.225;
    // Whole icon pixels per picture pixel where the icon is large enough; below that, a fraction.
    let fit = ((square * 0.72) / cat_w as f32).min((square * 0.62) / cat_h as f32);
    let scale = if fit >= 1.0 { fit.floor() } else { fit };
    let origin_x = (s - cat_w as f32 * scale) / 2.0;
    let origin_y = (s - cat_h as f32 * scale) / 2.0 + s / 40.0;

    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for py in 0..size {
        for px in 0..size {
            // Distance outside the rounded square, in pixels (negative inside).
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let dx = (inset + radius - x).max(x - (s - inset - radius)).max(0.0);
            let dy = (inset + radius - y).max(y - (s - inset - radius)).max(0.0);
            let outside = (dx * dx + dy * dy).sqrt() - radius;
            let outside = outside.max(
                (inset - x)
                    .max(x - (s - inset))
                    .max(inset - y)
                    .max(y - (s - inset)),
            );
            if outside > 0.5 {
                continue;
            }
            let rim = outside > -(s / 256.0).max(1.0);
            let mut color = if rim { RIM } else { BACKGROUND };
            let (cx, cy) = (x - origin_x, y - origin_y);
            if cx >= 0.0 && cy >= 0.0 {
                let (ix, iy) = ((cx / scale) as usize, (cy / scale) as usize);
                if ix < cat_w as usize
                    && iy < cat_h as usize
                    && let Some(pixel) = image[(top + iy) * width + left + ix]
                {
                    color = pixel;
                }
            }
            // Soft edge: partial coverage on the boundary pixel.
            let alpha = (0.5 - outside).clamp(0.0, 1.0);
            let at = ((py * size + px) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[color.0, color.1, color.2, (alpha * 255.0) as u8]);
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * size + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn the_cat_sits_on_a_rounded_square() {
        let size = 1024;
        let rgba = icon(size);
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        // Outside the square and in its rounded corner: transparent.
        assert_eq!(pixel(&rgba, size, 10, 10)[3], 0);
        assert_eq!(pixel(&rgba, size, 110, 110)[3], 0);
        // Inside near an edge: the background, opaque.
        let (r, g, b) = BACKGROUND;
        assert_eq!(pixel(&rgba, size, 512, 140), [r, g, b, 255]);
        // The middle holds the cat's colours, not the background.
        let cat = (300..724)
            .flat_map(|x| (300..724).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(&rgba, size, x, y)[..3] != [r, g, b])
            .count();
        assert!(cat > 20_000, "only {cat} cat pixels");
    }

    #[test]
    fn every_icon_size_has_the_cat() {
        for size in [16, 32, 128, 512] {
            let rgba = icon(size);
            let opaque = rgba.chunks(4).filter(|p| p[3] == 255).count();
            assert!(opaque > (size * size / 3) as usize, "{size}: {opaque}");
        }
    }
}
