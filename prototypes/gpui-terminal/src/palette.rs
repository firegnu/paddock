//! Resolving cell colours. Program-set palette entries win; otherwise the xterm defaults that
//! Saddle's `terminal::Screen` also answers to colour queries, and the window's fg/bg/cursor.
use alacritty_terminal::{
    term::color::Colors,
    vte::ansi::{Color, NamedColor},
};

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
}

const ANSI: [Rgb; 16] = [
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

pub fn indexed(index: u8) -> Rgb {
    match index {
        0..=15 => ANSI[usize::from(index)],
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
        0..=255 => indexed(index as u8),
        _ => match color {
            Color::Named(NamedColor::Background) => theme.background,
            Color::Named(NamedColor::Cursor) => theme.cursor,
            Color::Named(NamedColor::DimForeground) => dim(theme.foreground),
            // Dim black…white sit right after the cursor entry, in normal-colour order.
            Color::Named(name)
                if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize)
                    .contains(&(name as usize)) =>
            {
                dim(ANSI[name as usize - NamedColor::DimBlack as usize])
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

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::Rgb as A;

    const THEME: Theme = Theme {
        foreground: (220, 220, 220),
        background: (30, 30, 30),
        cursor: (200, 200, 0),
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
}
