//! The app icon: a horse's head facing right, yellow through pink to violet with a rounded blue
//! mane, on a deep indigo rounded square in the macOS icon grid. Drawn from the P5-58 design
//! (`docs/设计稿/P5-58-应用图标/Final.dc.html`), kept as SVG in `icon/` and rendered with resvg.
//! `examples/bundle.rs` writes it into `paddock.app`; the About window shows it too.
use resvg::{tiny_skia, usvg};

/// The whole icon, in a 1024-unit square.
const FULL: &str = include_str!("icon/paddock.svg");
/// The same horse without the blush, glare, rim light and shadow on the face, and with a bigger
/// eye: those only smudge at this size.
const SIMPLE: &str = include_str!("icon/paddock-small.svg");
/// At this size and below the simpler drawing is used.
const SMALL: u32 = 32;

/// An RGBA image `size` pixels square: transparent outside the rounded square, which takes
/// 824/1024 of the width like Apple's app icon grid.
pub fn icon(size: u32) -> Vec<u8> {
    render(if size <= SMALL { SIMPLE } else { FULL }, size)
}

fn render(svg: &str, size: u32) -> Vec<u8> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .expect("the icon's SVG is part of the source");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("an icon is at least a pixel");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * size + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    /// A point in the design's 1024 square, in pixels of a `size` icon.
    fn at(size: u32, x: u32, y: u32) -> (u32, u32) {
        (x * size / 1024, y * size / 1024)
    }

    #[test]
    fn the_horse_sits_on_an_indigo_rounded_square() {
        let size = 1024;
        let rgba = icon(size);
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        // Outside the square and in its rounded corner: transparent.
        assert_eq!(pixel(&rgba, size, 10, 10)[3], 0);
        assert_eq!(pixel(&rgba, size, 110, 110)[3], 0);
        // Inside near the top edge: the ground, opaque, dark and blue.
        let [r, g, b, a] = pixel(&rgba, size, 512, 140);
        assert_eq!(a, 255);
        assert!(r < 0x40 && g < 0x40 && b > r && b < 0x70, "{r} {g} {b}");
        // The muzzle is warm pink, the bottom of the mane blue.
        let [r, g, b, _] = pixel(&rgba, size, 780, 600);
        assert!(r > 0xd0 && r > b && g < r, "muzzle {r} {g} {b}");
        let [r, _, b, _] = pixel(&rgba, size, 320, 800);
        assert!(b > 0x90 && b > r, "mane {r} {b}");
        // The eye is dark.
        let [r, g, b, _] = pixel(&rgba, size, 588, 440);
        assert!(r < 0x40 && g < 0x40 && b < 0x60, "eye {r} {g} {b}");
    }

    #[test]
    fn small_icons_use_the_simpler_drawing() {
        for size in [16, 32] {
            assert!(icon(size) == render(SIMPLE, size), "{size}");
            assert!(icon(size) != render(FULL, size), "{size}");
        }
        assert!(icon(64) == render(FULL, 64));
        // The simpler drawing's eye is bigger, so it still shows at 32.
        let (x, y) = at(32, 590, 420);
        let [r, g, _, a] = pixel(&icon(32), 32, x, y);
        assert!(a == 255 && r < 0x60 && g < 0x60, "eye {r} {g}");
    }

    #[test]
    fn every_icon_size_has_the_horse() {
        for size in [16, 32, 64, 128, 256, 512, 1024] {
            let rgba = icon(size);
            let opaque = rgba.chunks(4).filter(|p| p[3] == 255).count();
            assert!(opaque > (size * size / 3) as usize, "{size}: {opaque}");
            // The face and mane are bright against the dark ground.
            let bright = rgba
                .chunks(4)
                .filter(|p| p[3] == 255 && p[..3].iter().map(|&c| c as u32).sum::<u32>() > 400)
                .count();
            assert!(
                bright > (size * size / 8) as usize,
                "{size}: {bright} bright"
            );
        }
    }
}
