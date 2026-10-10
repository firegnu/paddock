//! The small icon drawn for an agent's kind. The originals kept on this machine in
//! `~/.config/paddock/icons/` (`claude`, `codex`, `pi` or `omp`, `.svg` or `.png`; the SVG when
//! both are there) are drawn in their own colours; without one, or when it cannot be read, the
//! one-colour silhouette compiled in from `assets/kinds/` (where the README says where each comes
//! from) is drawn in the theme's colour for its kind. The originals are read once, the first time
//! an icon is asked for, and drawn at each size from memory.
use gpui::{
    AnyElement, App, Bounds, Corners, DevicePixels, Hsla, IntoElement, Pixels, RenderImage, Rgba,
    Styled, SvgRenderer, SvgSize, Transformation, canvas, px, radians, size, svg,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

/// The kinds with an icon, as the kind colours match them (any case).
const KINDS: [&str; 4] = ["claude", "codex", "pi", "omp"];

/// The width an original is measured at to find where its picture lies, in pixels.
const PROBE: u32 = 256;
/// How opaque a pixel must be to count as the picture rather than its margin or shadow.
const SOLID: u8 = 128;
/// How much of its cut a picture must cover to count as a tile: a rounded square, even a macOS
/// squircle, covers well over this; a glyph such as pi's well under.
const TILE_COVER: f32 = 0.85;
/// How far from square a tile may be, as width over height.
const TILE_ASPECT: std::ops::RangeInclusive<f32> = 0.9..=1.1;

/// A kind's icon and how wide it is drawn for its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KindIcon {
    look: Look,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Look {
    /// A silhouette, drawn in one colour.
    Mask(&'static [u8]),
    /// An original from this machine, in its own colours.
    Picture(&'static Picture),
}

/// An original, as an SVG document showing just its picture: the file wrapped in an `<image>`,
/// cut to where it is opaque, so an app icon's transparent margin and shadow are left out.
#[derive(Debug, PartialEq)]
pub struct Picture {
    kind: usize,
    document: Vec<u8>,
    /// Width over height.
    aspect: f32,
    /// Whether it brings its own ground: squarish and opaque nearly to the edges of its cut, like
    /// an app icon, rather than a glyph meant to sit on something.
    pub tile: bool,
    /// The file as a `data:` address, how big it is laid out, and the part of that which is its
    /// picture (`x`, `y`, width, height), to wrap it again turned ([`Picture::turned`]).
    href: String,
    whole: (f32, f32),
    cut: (f32, f32, f32, f32),
    /// The mean colour of its opaque pixels: red, green, blue.
    tint: [u8; 3],
}

/// How an original is drawn on a card: turned clockwise by `degrees` about its centre, its
/// corners cut round by `round` of its height.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Turn {
    degrees: f32,
    round: f32,
}

/// The room a box `width` by `height` takes once turned by `degrees`.
fn reach((width, height): (f32, f32), degrees: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (sin, cos) = (sin.abs(), cos.abs());
    (width * cos + height * sin, width * sin + height * cos)
}

impl Picture {
    /// An SVG document showing the picture as `turn` says, in the room that takes ([`reach`]). A
    /// tile is made square first, as it is drawn (see [`KindIcon::render_tile`]).
    fn turned(&self, turn: Turn) -> String {
        let (x, y, width, height) = self.cut;
        let squash = if self.tile { height / width } else { 1.0 };
        let (x, width) = (x * squash, width * squash);
        let (across, down) = reach((width, height), turn.degrees);
        let (centre_x, centre_y) = (x + width / 2.0, y + height / 2.0);
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{across}" height="{down}" viewBox="{} {} {across} {down}"><clipPath id="cut"><rect x="{x}" y="{y}" width="{width}" height="{height}" rx="{}"/></clipPath><g transform="rotate({} {centre_x} {centre_y})"><g clip-path="url(#cut)"><image width="{}" height="{}" preserveAspectRatio="none" xlink:href="{}"/></g></g></svg>"#,
            centre_x - across / 2.0,
            centre_y - down / 2.0,
            turn.round * height,
            turn.degrees,
            self.whole.0 * squash,
            self.whole.1,
            self.href
        )
    }
}

/// The icon for `kind`: this machine's original when it has a good one, else the silhouette;
/// `None` for a kind without one.
pub fn of(kind: &str) -> Option<KindIcon> {
    let index = KINDS.iter().position(|k| k.eq_ignore_ascii_case(kind))?;
    match &originals()[index] {
        Some(picture) => Some(KindIcon {
            look: Look::Picture(picture),
            width: picture.aspect,
        }),
        None => Some(silhouette(index)),
    }
}

/// The compiled-in silhouette. The official files are used as they are, so their widths even
/// out their weight: pi's solid badge is drawn smaller, omp's wide, thin-legged π larger.
fn silhouette(index: usize) -> KindIcon {
    let (data, width): (&'static [u8], _) = match index {
        0 => (include_bytes!("../assets/kinds/claude.svg"), 1.0),
        1 => (include_bytes!("../assets/kinds/codex.svg"), 1.0),
        2 => (include_bytes!("../assets/kinds/pi.svg"), 0.8),
        _ => (include_bytes!("../assets/kinds/omp.svg"), 1.15),
    };
    KindIcon {
        look: Look::Mask(data),
        width,
    }
}

/// Where this machine keeps its originals.
fn dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/paddock/icons")
}

/// The originals, read the first time they are asked for.
fn originals() -> &'static [Option<Picture>; 4] {
    static ORIGINALS: OnceLock<[Option<Picture>; 4]> = OnceLock::new();
    ORIGINALS.get_or_init(|| {
        let dir = dir();
        std::array::from_fn(|index| load(&dir, index))
    })
}

/// `kind`'s original in `dir`: the SVG, else the PNG; `None` when neither is there and good.
fn load(dir: &Path, kind: usize) -> Option<Picture> {
    let renderer = SvgRenderer::new(Arc::new(()));
    ["svg", "png"].into_iter().find_map(|extension| {
        let bytes = std::fs::read(dir.join(format!("{}.{extension}", KINDS[kind]))).ok()?;
        picture(&renderer, kind, extension, &bytes)
    })
}

/// The file as a [`Picture`]; `None` when it is not a picture of that format or shows nothing.
fn picture(renderer: &SvgRenderer, kind: usize, extension: &str, bytes: &[u8]) -> Option<Picture> {
    // Its proportions, so it can be wrapped in a document of its own shape.
    let (mime, aspect) = match extension {
        "svg" => {
            let probe = renderer
                .render_parsed(
                    &renderer.parse_svg(bytes).ok()?,
                    SvgSize::Size(size(DevicePixels(PROBE as i32), DevicePixels(1))),
                )
                .ok()?;
            let shape = probe.size(0);
            (
                "image/svg+xml",
                shape.width.0 as f32 / shape.height.0.max(1) as f32,
            )
        }
        "png" => {
            // The signature, then the header's width and height.
            if bytes.len() < 24 || bytes[..8] != *b"\x89PNG\r\n\x1a\n" || bytes[12..16] != *b"IHDR"
            {
                return None;
            }
            let number = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
            let (width, height) = (number(16), number(20));
            ("image/png", width as f32 / height.max(1) as f32)
        }
        _ => return None,
    };
    if !aspect.is_finite() || aspect <= 0.0 {
        return None;
    }
    let (width, height) = (PROBE as f32, PROBE as f32 / aspect);
    let href = format!("data:{mime};base64,{}", base64(bytes));
    let whole = wrap(&href, (0.0, 0.0, width, height), (width, height));
    // Where it is opaque, measured on the whole.
    let probe = renderer
        .render_parsed(
            &renderer.parse_svg(whole.as_bytes()).ok()?,
            SvgSize::Size(size(DevicePixels(PROBE as i32), DevicePixels(1))),
        )
        .ok()?;
    let shape = probe.size(0);
    let (columns, rows) = (shape.width.0 as usize, shape.height.0 as usize);
    let pixels = probe.as_bytes(0)?;
    let (mut left, mut top, mut right, mut bottom) = (columns, rows, 0, 0);
    let mut solid = 0;
    let mut sum = [0u64; 3];
    for (index, pixel) in pixels.chunks_exact(4).enumerate() {
        if pixel[3] >= SOLID {
            solid += 1;
            // Kept as blue, green, red.
            for (sum, channel) in sum.iter_mut().zip([pixel[2], pixel[1], pixel[0]]) {
                *sum += u64::from(channel);
            }
            let (x, y) = (index % columns, index / columns);
            left = left.min(x);
            right = right.max(x + 1);
            top = top.min(y);
            bottom = bottom.max(y + 1);
        }
    }
    if left >= right || top >= bottom {
        return None;
    }
    let unit = width / columns as f32;
    let cut = (
        left as f32 * unit,
        top as f32 * unit,
        (right - left) as f32 * unit,
        (bottom - top) as f32 * unit,
    );
    let aspect = cut.2 / cut.3;
    let cover = solid as f32 / ((right - left) * (bottom - top)) as f32;
    Some(Picture {
        kind,
        document: wrap(&href, cut, (width, height)).into_bytes(),
        aspect,
        tile: TILE_ASPECT.contains(&aspect) && cover >= TILE_COVER,
        href,
        whole: (width, height),
        cut,
        tint: sum.map(|sum| (sum / solid as u64) as u8),
    })
}

/// An SVG document showing the part `(x, y, width, height)` of the image at `href`, laid out
/// `size` big.
fn wrap(href: &str, (x, y, width, height): (f32, f32, f32, f32), size: (f32, f32)) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="{x} {y} {width} {height}"><image width="{}" height="{}" preserveAspectRatio="none" xlink:href="{href}"/></svg>"#,
        size.0, size.1
    )
}

/// Standard Base64 with padding, for a `data:` address.
fn base64(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = chunk
            .iter()
            .enumerate()
            .fold(0u32, |group, (i, &b)| group | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                text.push(DIGITS[(group >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// `picture` drawn `height` device pixels tall, turned as `turn` says (then in the room that
/// takes, see [`reach`]) or upright; drawn once per size and turn and kept.
fn raster(
    picture: &Picture,
    height: u32,
    turn: Option<Turn>,
    cx: &App,
) -> Option<Arc<RenderImage>> {
    type Key = (usize, u32, Option<(u32, u32)>);
    thread_local! {
        static RASTERS: RefCell<HashMap<Key, Arc<RenderImage>>> = RefCell::default();
    }
    let key = (
        picture.kind,
        height,
        turn.map(|turn| (turn.degrees.to_bits(), turn.round.to_bits())),
    );
    RASTERS.with_borrow_mut(|rasters| {
        if let Some(image) = rasters.get(&key) {
            return Some(image.clone());
        }
        let renderer = cx.svg_renderer();
        let height = height as f32;
        let turned;
        let (document, room) = match turn {
            Some(turn) => {
                let aspect = if picture.tile { 1.0 } else { picture.aspect };
                turned = picture.turned(turn).into_bytes();
                (&turned, reach((height * aspect, height), turn.degrees))
            }
            None => (&picture.document, (height * picture.aspect, height)),
        };
        let pixels = |length: f32| DevicePixels(length.round().max(1.0) as i32);
        let image = renderer
            .render_parsed(
                &renderer.parse_svg(document).ok()?,
                SvgSize::Size(size(pixels(room.0), pixels(room.1))),
            )
            .ok()?;
        rasters.insert(key, image.clone());
        Some(image)
    })
}

impl KindIcon {
    /// The icon `height` tall, centred in its width: a silhouette in `color`, an original in its
    /// own colours.
    pub fn render(self, height: Pixels, color: Hsla) -> AnyElement {
        self.draw(height, color, Pixels::ZERO, 0.0)
    }

    /// The icon as [`Self::render`] draws it, turned clockwise by `degrees` about its centre; its
    /// corners reach past its box.
    pub fn render_turned(self, height: Pixels, color: Hsla, degrees: f32) -> AnyElement {
        self.draw(height, color, Pixels::ZERO, degrees)
    }

    /// Whether this is one of this machine's originals rather than a silhouette.
    pub fn original(self) -> bool {
        matches!(self.look, Look::Picture(_))
    }

    /// Whether this is an original that brings its own ground (see [`Picture::tile`]).
    pub fn tile(self) -> bool {
        matches!(self.look, Look::Picture(picture) if picture.tile)
    }

    /// A [tile](Self::tile) drawn `side` square with its corners rounded `radius`, to stand in for
    /// a square of that size.
    pub fn render_tile(self, side: Pixels, radius: Pixels) -> AnyElement {
        self.render_tile_turned(side, radius, 0.0)
    }

    /// A tile as [`Self::render_tile`] draws it, turned clockwise by `degrees` about its centre;
    /// its corners reach past its box.
    pub fn render_tile_turned(self, side: Pixels, radius: Pixels, degrees: f32) -> AnyElement {
        let icon = KindIcon { width: 1.0, ..self };
        icon.draw(side, gpui::transparent_black(), radius, degrees)
    }

    /// A [tile](Self::tile)'s own colour, the mean of its picture's, for what stands in for it
    /// too small to show it; `None` for any other icon.
    pub fn tint(self) -> Option<Hsla> {
        match self.look {
            Look::Picture(picture) if picture.tile => {
                let [r, g, b] = picture.tint.map(|channel| f32::from(channel) / 255.0);
                Some(Rgba { r, g, b, a: 1.0 }.into())
            }
            _ => None,
        }
    }

    fn draw(self, height: Pixels, color: Hsla, radius: Pixels, degrees: f32) -> AnyElement {
        let turn = (degrees != 0.0).then(|| Turn {
            degrees,
            round: radius / height,
        });
        match self.look {
            Look::Mask(data) => {
                let icon = svg()
                    .data(data)
                    .flex_shrink_0()
                    .w(height * self.width)
                    .h(height)
                    .text_color(color);
                match turn {
                    Some(turn) => icon.with_transformation(Transformation::rotate(radians(
                        turn.degrees.to_radians(),
                    ))),
                    None => icon,
                }
                .into_any_element()
            }
            Look::Picture(picture) => canvas(
                |_, _, _| {},
                move |bounds, _, window, cx| {
                    let pixels = f32::from(bounds.size.height) * window.scale_factor();
                    let pixels = pixels.round().max(1.0) as u32;
                    // A turned picture has its corners cut in its document, and is drawn over the
                    // room it turns into.
                    let (corners, room) = match turn {
                        Some(turn) => {
                            let upright = (bounds.size.width.into(), bounds.size.height.into());
                            let (across, down) = reach(upright, turn.degrees);
                            let room = size(px(across), px(down));
                            (
                                Corners::default(),
                                Bounds::centered_at(bounds.center(), room),
                            )
                        }
                        None => (Corners::all(radius), bounds),
                    };
                    if let Some(image) = raster(picture, pixels, turn, cx) {
                        let _ = window.paint_image(room, room, corners, image, 0, false);
                    }
                },
            )
            .flex_shrink_0()
            .w(height * self.width)
            .h(height)
            .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silhouette_file(kind: &str) -> Option<&'static str> {
        let index = KINDS.iter().position(|k| k.eq_ignore_ascii_case(kind))?;
        let Look::Mask(data) = silhouette(index).look else {
            return None;
        };
        KINDS.into_iter().find(|name| {
            let path = format!("{}/assets/kinds/{name}.svg", env!("CARGO_MANIFEST_DIR"));
            std::fs::read(path).unwrap() == data
        })
    }

    #[test]
    fn each_known_kind_has_its_own_silhouette_and_others_none() {
        assert_eq!(silhouette_file("claude"), Some("claude"));
        assert_eq!(silhouette_file("Codex"), Some("codex"));
        assert_eq!(silhouette_file("pi"), Some("pi"));
        assert_eq!(silhouette_file("OMP"), Some("omp"));
        assert_eq!(of("gemini"), None);
        assert_eq!(of(""), None);
    }

    /// A 4×4 PNG, opaque only in its middle 2×2 (made with the `png` crate for this test).
    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = ::png::Encoder::new(&mut bytes, 4, 4);
        encoder.set_color(::png::ColorType::Rgba);
        encoder.set_depth(::png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let mut data = vec![0u8; 4 * 4 * 4];
        for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
            data[(y * 4 + x) * 4..][..4].copy_from_slice(&[200, 80, 40, 255]);
        }
        writer.write_image_data(&data).unwrap();
        drop(writer);
        bytes
    }

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50"><rect x="0" y="0" width="100" height="50" fill="#4D9ABF"/></svg>"##;

    fn folder(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("paddock-kind-icon-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (file, bytes) in files {
            std::fs::write(dir.join(file), bytes).unwrap();
        }
        dir
    }

    #[test]
    fn the_svg_wins_over_the_png_and_either_works_alone() {
        let png = png();
        let both = folder(
            "both",
            &[("claude.svg", SVG.as_bytes()), ("claude.png", &png)],
        );
        let picture = load(&both, 0).unwrap();
        assert!(String::from_utf8_lossy(&picture.document).contains("data:image/svg+xml"));
        assert_eq!(picture.aspect, 2.0);

        let only_png = folder("png", &[("codex.png", &png)]);
        let picture = load(&only_png, 1).unwrap();
        let document = String::from_utf8_lossy(&picture.document).into_owned();
        assert!(document.contains("data:image/png"), "{document}");
        // The transparent margin is cut away: the middle half of the picture is left, give or
        // take the edge the enlarged pixels blur into.
        let view: Vec<f32> = document
            .split("viewBox=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap()
            .split(' ')
            .map(|n| n.parse().unwrap())
            .collect();
        for (got, want) in view.iter().zip([64.0, 64.0, 128.0, 128.0]) {
            assert!((got - want).abs() <= 4.0, "{document}");
        }
        assert_eq!(picture.aspect, 1.0);
        for dir in [both, only_png] {
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn missing_or_broken_files_fall_back_to_the_silhouette() {
        let mut broken_png = png();
        broken_png.truncate(40);
        let dir = folder(
            "broken",
            &[
                ("claude.svg", b"<svg not really"),
                ("codex.png", &broken_png),
                ("pi.png", SVG.as_bytes()),
                // Good, but shows nothing.
                (
                    "omp.svg",
                    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>"#,
                ),
            ],
        );
        for (kind, name) in KINDS.iter().enumerate() {
            assert_eq!(load(&dir, kind), None, "{name}");
        }
        assert_eq!(load(&dir.join("missing"), 0), None);
        // A broken SVG beside a good PNG: the PNG.
        std::fs::write(dir.join("claude.png"), png()).unwrap();
        assert!(load(&dir, 0).is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn only_a_squarish_picture_with_its_own_ground_is_a_tile() {
        let rounded = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="12" fill="#0f0a14"/><path fill="#9b4dff" d="M14 16h36v8H14z"/></svg>"##;
        // pi's logo: three coloured pieces with no ground of their own.
        let glyph = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 800"><path fill="#F09082" d="M165.29 165.29H517.36V400H400V282.65H165.29Z"/><path fill="#4D9ABF" d="M165.29 282.65H282.65V400H400V517.36H282.65V634.72H165.29Z"/><path fill="#F1BE58" d="M517.36 400H634.72V634.72H517.36Z"/></svg>"##;
        let dir = folder(
            "tile",
            &[
                ("claude.svg", rounded),
                ("codex.svg", SVG.as_bytes()),
                ("pi.svg", glyph),
                ("omp.png", &png()),
            ],
        );
        let tile = |kind| load(&dir, kind).unwrap().tile;
        assert!(tile(0), "a rounded square");
        assert!(!tile(1), "solid but twice as wide as tall");
        assert!(!tile(2), "a glyph");
        assert!(tile(3), "solid once its margin is cut away");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A square of one colour: a tile.
    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#4D9ABF"/></svg>"##;

    #[test]
    fn a_picture_s_tint_is_the_mean_colour_of_what_it_shows() {
        let renderer = SvgRenderer::new(Arc::new(()));
        let square = picture(&renderer, 0, "svg", SQUARE.as_bytes()).unwrap();
        assert!(square.tile);
        assert_eq!(square.tint, [0x4D, 0x9A, 0xBF]);
        // The transparent margin is not counted: the colour of its middle, give or take what
        // enlarging it blurs.
        let tint = picture(&renderer, 0, "png", &png()).unwrap().tint;
        for (got, want) in tint.into_iter().zip([200u8, 80, 40]) {
            assert!(got.abs_diff(want) <= 3, "{tint:?}");
        }
    }

    #[test]
    fn a_turned_picture_leans_the_way_asked_in_the_room_that_takes_its_corners_cut_round() {
        let renderer = SvgRenderer::new(Arc::new(()));
        let square = picture(&renderer, 0, "svg", SQUARE.as_bytes()).unwrap();
        // Six degrees anticlockwise, a pixel to each unit of the square's side.
        let shown = |round: f32| {
            let turn = Turn {
                degrees: -6.0,
                round,
            };
            let (across, down) = reach((100.0, 100.0), turn.degrees);
            assert!((across - 109.9).abs() < 0.1 && across == down);
            let image = renderer
                .render_parsed(
                    &renderer.parse_svg(square.turned(turn).as_bytes()).unwrap(),
                    SvgSize::Size(size(DevicePixels(110), DevicePixels(110))),
                )
                .unwrap();
            assert_eq!(image.size(0), size(DevicePixels(110), DevicePixels(110)));
            let pixels = image.as_bytes(0).unwrap().to_vec();
            move |x: usize, y: usize| pixels[(y * 110 + x) * 4 + 3] > SOLID
        };
        // Its top edge runs from ten pixels down on the left up to the top right corner.
        let square_cornered = shown(0.0);
        assert!(square_cornered(55, 55));
        assert!(
            !square_cornered(10, 3),
            "the left of its top edge went down"
        );
        assert!(square_cornered(95, 3), "the right of its top edge went up");
        assert!(square_cornered(3, 14) && !square_cornered(106, 14));
        // With its corners cut, the corner is gone and the middle of each edge stays.
        let rounded = shown(0.28);
        assert!(rounded(55, 55));
        assert!(!rounded(95, 3) && !rounded(3, 14));
        assert!(rounded(50, 8) && rounded(8, 60));
    }

    #[test]
    fn base64_matches_the_standard() {
        for (bytes, text) in [
            (&b""[..], ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foobar", "Zm9vYmFy"),
            (&[0xff, 0xfe, 0x00][..], "//4A"),
        ] {
            assert_eq!(base64(bytes), text);
        }
    }
}
