//! Resolving cell colours. Program-set palette entries win; otherwise the theme's 16 colours, the
//! xterm 256-colour cube and gray ramp, and the theme's fg/bg/cursor.
use alacritty_terminal::{
    term::color::Colors,
    vte::ansi::{Color, NamedColor},
};
use std::sync::{Arc, Mutex};

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// The 16 basic colours, black…white then their bright variants.
    pub ansi: [Rgb; 16],
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    /// Drawn behind selected text, which keeps its own colour.
    pub selection: Rgb,
}

impl Theme {
    /// xterm's own colours, for a terminal not handed a theme yet.
    pub const XTERM: Theme = Theme {
        ansi: XTERM,
        foreground: (229, 229, 229),
        background: (0, 0, 0),
        cursor: (229, 229, 229),
        selection: (0, 0, 0),
    };

    /// A palette entry as this theme shows it before any program changes it.
    pub fn indexed(&self, index: u8) -> Rgb {
        match index {
            0..=15 => self.ansi[usize::from(index)],
            _ => indexed(index),
        }
    }
}

/// A pane's colours, shared with every session it runs or is starting: a program asking for a
/// colour is told them as they are then. The pane changes them in place.
#[derive(Clone)]
pub struct Shared(Arc<Mutex<Theme>>);
impl Shared {
    pub fn new(theme: Theme) -> Self {
        Self(Arc::new(Mutex::new(theme)))
    }
    pub fn get(&self) -> Theme {
        *self.0.lock().unwrap()
    }
    pub fn set(&self, theme: Theme) {
        *self.0.lock().unwrap() = theme;
    }
}
/// xterm's own colours, until the pane hands its own.
impl Default for Shared {
    fn default() -> Self {
        Self::new(Theme::XTERM)
    }
}

/// The xterm defaults for the 16 basic colours.
pub const XTERM: [Rgb; 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// The xterm 256-colour palette.
pub fn indexed(index: u8) -> Rgb {
    match index {
        0..=15 => XTERM[usize::from(index)],
        16..=231 => {
            let n = index - 16;
            let level = [0, 95, 135, 175, 215, 255];
            (
                level[usize::from(n / 36)],
                level[usize::from(n / 6 % 6)],
                level[usize::from(n % 6)],
            )
        }
        _ => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}

pub fn resolve(color: Color, colors: &Colors, theme: &Theme) -> Rgb {
    let index = match color {
        Color::Spec(rgb) => return (rgb.r, rgb.g, rgb.b),
        Color::Indexed(index) => usize::from(index),
        Color::Named(name) => name as usize,
    };
    if let Some(rgb) = colors[index] {
        return (rgb.r, rgb.g, rgb.b);
    }
    match index {
        0..=255 => theme.indexed(index as u8),
        _ => match color {
            Color::Named(NamedColor::Background) => theme.background,
            Color::Named(NamedColor::Cursor) => theme.cursor,
            Color::Named(NamedColor::DimForeground) => dim(theme.foreground),
            // Dim black…white sit right after the cursor entry, in normal-colour order.
            Color::Named(name)
                if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize)
                    .contains(&(name as usize)) =>
            {
                dim(theme.ansi[name as usize - NamedColor::DimBlack as usize])
            }
            _ => theme.foreground,
        },
    }
}

/// SGR 2 (faint) as two thirds of the colour.
pub fn dim((r, g, b): Rgb) -> Rgb {
    let f = |v: u8| (u16::from(v) * 2 / 3) as u8;
    (f(r), f(g), f(b))
}

/// The least contrast text keeps with its background while Settings' `minimum_contrast` is on
/// (P5-143e): about where text stops being readable at a glance.
pub const MIN_CONTRAST: f32 = 3.0;

/// How far apart two colours are to the eye, as WCAG measures it: 1 (the same) to 21.
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Text in `fg` on `bg` moved toward white or black, whichever is farther from `bg`, just far
/// enough to stand `min` apart (P5-143e: text a program drew too close to its background); text
/// far enough already, or drawn in the background's own colour on purpose, is left alone.
pub fn legible(fg: Rgb, bg: Rgb, min: f32) -> Rgb {
    if fg == bg || contrast(fg, bg) >= min {
        return fg;
    }
    let far = if contrast((255, 255, 255), bg) >= contrast((0, 0, 0), bg) {
        255.0
    } else {
        0.0
    };
    let toward = |t: f32| {
        let mix = |c: u8| (f32::from(c) + (far - f32::from(c)) * t).round() as u8;
        (mix(fg.0), mix(fg.1), mix(fg.2))
    };
    // The least step that is far enough; all the way when even that is not.
    let (mut near, mut enough) = (0.0, 1.0);
    for _ in 0..12 {
        let t = (near + enough) / 2.0;
        if contrast(toward(t), bg) >= min {
            enough = t;
        } else {
            near = t;
        }
    }
    toward(enough)
}

/// How bright a colour is to the eye, 0 to 1 (WCAG's relative luminance).
fn luminance((r, g, b): Rgb) -> f32 {
    static LINEAR: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    let linear = LINEAR.get_or_init(|| {
        std::array::from_fn(|i| {
            let c = i as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
    });
    0.2126 * linear[usize::from(r)]
        + 0.7152 * linear[usize::from(g)]
        + 0.0722 * linear[usize::from(b)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::Rgb as A;

    const THEME: Theme = Theme {
        ansi: XTERM,
        foreground: (220, 220, 220),
        background: (30, 30, 30),
        cursor: (200, 200, 0),
        selection: (60, 60, 60),
    };

    #[test]
    fn xterm_palette() {
        assert_eq!(indexed(1), (205, 0, 0));
        assert_eq!(indexed(12), (92, 92, 255));
        assert_eq!(indexed(16), (0, 0, 0));
        assert_eq!(indexed(21), (0, 0, 255));
        assert_eq!(indexed(231), (255, 255, 255));
        assert_eq!(indexed(232), (8, 8, 8));
        assert_eq!(indexed(255), (238, 238, 238));
    }

    #[test]
    fn named_and_direct_colours() {
        let colors = Colors::default();
        assert_eq!(
            resolve(Color::Spec(A { r: 1, g: 2, b: 3 }), &colors, &THEME),
            (1, 2, 3)
        );
        assert_eq!(
            resolve(Color::Named(NamedColor::Foreground), &colors, &THEME),
            THEME.foreground
        );
        assert_eq!(
            resolve(Color::Named(NamedColor::Background), &colors, &THEME),
            THEME.background
        );
        assert_eq!(
            resolve(Color::Named(NamedColor::Red), &colors, &THEME),
            (205, 0, 0)
        );
        assert_eq!(resolve(Color::Indexed(232), &colors, &THEME), (8, 8, 8));
    }

    #[test]
    fn program_set_palette_wins() {
        let mut colors = Colors::default();
        colors[1] = Some(A { r: 9, g: 8, b: 7 });
        assert_eq!(
            resolve(Color::Named(NamedColor::Red), &colors, &THEME),
            (9, 8, 7)
        );
    }

    #[test]
    fn contrast_runs_from_one_to_twenty_one() {
        assert!((contrast((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 0.01);
        assert!((contrast((80, 80, 80), (80, 80, 80)) - 1.0).abs() < 0.001);
        assert_eq!(
            contrast((10, 20, 30), (200, 100, 0)),
            contrast((200, 100, 0), (10, 20, 30))
        );
    }

    #[test]
    fn text_far_enough_from_its_background_is_left_alone() {
        let bg = (0x1d, 0x1a, 0x16);
        assert_eq!(legible((0xe8, 0xdf, 0xcc), bg, 3.0), (0xe8, 0xdf, 0xcc));
        // The background's own colour, on purpose.
        assert_eq!(legible(bg, bg, 3.0), bg);
    }

    #[test]
    fn text_too_close_is_moved_just_far_enough_toward_the_far_end() {
        // Dark grey on a dark background: lighter.
        let (fg, bg) = ((0x30, 0x30, 0x30), (0x1d, 0x1a, 0x16));
        let moved = legible(fg, bg, 3.0);
        let now = contrast(moved, bg);
        assert!((3.0..3.3).contains(&now), "{moved:?} stands {now} apart");
        assert!(moved.0 > fg.0);
        // Light grey on white: darker.
        let (fg, bg) = ((0xdd, 0xdd, 0xdd), (0xff, 0xff, 0xff));
        let moved = legible(fg, bg, 3.0);
        assert!(contrast(moved, bg) >= 3.0);
        assert!(moved.0 < fg.0);
        // A hue keeps its lean: a dim blue stays bluer than it is red.
        let moved = legible((0x10, 0x18, 0x50), (0x10, 0x10, 0x18), 3.0);
        assert!(moved.2 > moved.0);
    }
}
