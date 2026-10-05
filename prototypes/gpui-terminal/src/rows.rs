//! Splitting one screen row into background spans and text runs. A run keeps one style and only
//! narrow cells, so it can be shaped with a forced cell width; wide characters and drawn glyphs
//! are placed on their own column.
use crate::palette::Rgb;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The character plus any combining marks.
    pub text: String,
    pub bg: Rgb,
    pub style: Style,
    pub wide: bool,
    /// The second column of a wide character.
    pub spacer: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub col: usize,
    pub len: usize,
    pub color: Rgb,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    Text {
        col: usize,
        text: String,
        style: Style,
    },
    Wide {
        col: usize,
        text: String,
        style: Style,
    },
    Glyph {
        col: usize,
        c: char,
        color: Rgb,
    },
}

pub fn split(cells: &[Cell], default_bg: Rgb) -> (Vec<Span>, Vec<Run>) {
    let mut spans: Vec<Span> = Vec::new();
    for (col, cell) in cells.iter().enumerate() {
        if cell.bg == default_bg {
            continue;
        }
        match spans.last_mut() {
            Some(span) if span.color == cell.bg && span.col + span.len == col => span.len += 1,
            _ => spans.push(Span {
                col,
                len: 1,
                color: cell.bg,
            }),
        }
    }

    let mut runs = Vec::new();
    // The open text run and spaces seen since its last visible character.
    let mut open: Option<(usize, String, Style)> = None;
    let mut spaces = 0;
    let close = |open: &mut Option<(usize, String, Style)>, runs: &mut Vec<Run>| {
        if let Some((col, text, style)) = open.take() {
            runs.push(Run::Text { col, text, style });
        }
    };
    for (col, cell) in cells.iter().enumerate() {
        if cell.spacer {
            continue;
        }
        let blank = cell.text == " " && !cell.style.underline && !cell.style.strike;
        if blank {
            spaces += usize::from(open.is_some());
            continue;
        }
        let mut chars = cell.text.chars();
        let first = chars.next().unwrap_or(' ');
        let single = chars.next().is_none();
        if cell.wide {
            close(&mut open, &mut runs);
            runs.push(Run::Wide {
                col,
                text: cell.text.clone(),
                style: cell.style,
            });
        } else if single && crate::glyphs::pieces(first, 1.0, 1.0).is_some() {
            close(&mut open, &mut runs);
            runs.push(Run::Glyph {
                col,
                c: first,
                color: cell.style.fg,
            });
        } else {
            match &mut open {
                Some((_, text, style)) if *style == cell.style => {
                    text.extend(std::iter::repeat_n(' ', spaces));
                    text.push_str(&cell.text);
                }
                _ => {
                    close(&mut open, &mut runs);
                    open = Some((col, cell.text.clone(), cell.style));
                }
            }
        }
        spaces = 0;
    }
    close(&mut open, &mut runs);
    (spans, runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    const BG: Rgb = (0, 0, 0);
    const RED: Rgb = (255, 0, 0);

    fn style() -> Style {
        Style {
            fg: (200, 200, 200),
            ..Style::default()
        }
    }
    fn cells(text: &str) -> Vec<Cell> {
        let mut out = Vec::new();
        for c in text.chars() {
            let wide = unicode_wide(c);
            out.push(Cell {
                text: c.into(),
                bg: BG,
                style: style(),
                wide,
                spacer: false,
            });
            if wide {
                out.push(Cell {
                    text: " ".into(),
                    bg: BG,
                    style: style(),
                    wide: false,
                    spacer: true,
                });
            }
        }
        out
    }
    fn unicode_wide(c: char) -> bool {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
    }
    fn text(col: usize, text: &str) -> Run {
        Run::Text {
            col,
            text: text.into(),
            style: style(),
        }
    }

    #[test]
    fn spaces_stay_inside_runs_but_not_at_the_ends() {
        assert_eq!(split(&cells("a b"), BG).1, [text(0, "a b")]);
        assert_eq!(split(&cells("  ab  "), BG).1, [text(2, "ab")]);
        assert!(split(&cells("    "), BG).1.is_empty());
    }

    #[test]
    fn style_changes_start_a_new_run() {
        let mut row = cells("abc");
        row[1].style.bold = true;
        let runs = split(&row, BG).1;
        assert_eq!(runs.len(), 3);
        assert_eq!(
            runs[1],
            Run::Text {
                col: 1,
                text: "b".into(),
                style: Style {
                    bold: true,
                    ..style()
                }
            }
        );
    }

    #[test]
    fn wide_characters_and_drawn_glyphs_get_their_own_column() {
        let runs = split(&cells("a中b─c"), BG).1;
        assert_eq!(
            runs,
            [
                text(0, "a"),
                Run::Wide {
                    col: 1,
                    text: "中".into(),
                    style: style()
                },
                text(3, "b"),
                Run::Glyph {
                    col: 4,
                    c: '─',
                    color: style().fg
                },
                text(5, "c"),
            ]
        );
    }

    #[test]
    fn combining_marks_stay_with_their_base() {
        let mut row = cells("ab");
        row[0].text = "e\u{301}".into();
        assert_eq!(split(&row, BG).1, [text(0, "e\u{301}b")]);
    }

    #[test]
    fn backgrounds_merge_and_skip_the_default() {
        let mut row = cells("abcd");
        for i in [0, 1, 3] {
            row[i].bg = RED;
        }
        assert_eq!(
            split(&row, BG).0,
            [
                Span {
                    col: 0,
                    len: 2,
                    color: RED
                },
                Span {
                    col: 3,
                    len: 1,
                    color: RED
                }
            ]
        );
    }
}

/// Reads one visible row out of the parsed grid, resolving colours and the selection highlight.
pub fn read<T>(
    term: &alacritty_terminal::Term<T>,
    row: u16,
    cols: u16,
    theme: &crate::palette::Theme,
    selection: Option<&alacritty_terminal::selection::SelectionRange>,
) -> Vec<Cell> {
    use crate::palette::{dim, resolve};
    use alacritty_terminal::{
        grid::Dimensions,
        index::{Column, Line, Point},
        term::cell::Flags,
    };
    let offset = term.grid().display_offset() as i32;
    let cols = cols.min(term.columns() as u16);
    let colors = term.colors();
    let mut cells = Vec::with_capacity(usize::from(cols));
    for x in 0..cols {
        let point = Point::new(Line(i32::from(row) - offset), Column(usize::from(x)));
        let cell = &term.grid()[point];
        let flags = cell.flags;
        let mut fg = resolve(cell.fg, colors, theme);
        let mut bg = resolve(cell.bg, colors, theme);
        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if selection.is_some_and(|s| s.contains(point)) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if flags.intersects(Flags::DIM) {
            fg = dim(fg);
        }
        if flags.contains(Flags::HIDDEN) {
            fg = bg;
        }
        let mut text = cell.c.to_string();
        if let Some(marks) = cell.zerowidth() {
            text.extend(marks);
        }
        // A wide character cut by the right edge cannot be drawn whole.
        let wide = flags.contains(Flags::WIDE_CHAR);
        if wide && x + 1 >= cols {
            text = " ".into();
        }
        cells.push(Cell {
            text,
            bg,
            style: Style {
                fg,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.intersects(Flags::ALL_UNDERLINES),
                strike: flags.contains(Flags::STRIKEOUT),
            },
            wide: wide && x + 1 < cols,
            spacer: flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER),
        });
    }
    cells
}

#[cfg(test)]
mod parsed {
    use super::*;
    use crate::palette::Theme;
    use saddle::terminal::{Screen, Size};

    const THEME: Theme = Theme {
        foreground: (220, 220, 220),
        background: (0, 0, 0),
        cursor: (1, 1, 1),
    };

    fn runs(bytes: &[u8]) -> Vec<Run> {
        let mut screen = Screen::new(Size { rows: 2, cols: 30 });
        screen.process(bytes);
        split(&read(&screen.term, 0, 30, &THEME, None), THEME.background).1
    }

    #[test]
    fn heavy_and_light_lines_from_the_parser_are_drawn_glyphs() {
        let glyphs = |runs: Vec<Run>| -> String {
            runs.iter()
                .filter_map(|r| match r {
                    Run::Glyph { c, .. } => Some(*c),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(glyphs(runs("┏━━┓".as_bytes())), "┏━━┓");
        assert_eq!(glyphs(runs("┌──┐".as_bytes())), "┌──┐");
    }

    #[test]
    fn wide_characters_from_the_parser_skip_their_spacer() {
        assert_eq!(
            runs("a中b".as_bytes()),
            [
                Run::Text {
                    col: 0,
                    text: "a".into(),
                    style: Style {
                        fg: THEME.foreground,
                        ..Style::default()
                    }
                },
                Run::Wide {
                    col: 1,
                    text: "中".into(),
                    style: Style {
                        fg: THEME.foreground,
                        ..Style::default()
                    }
                },
                Run::Text {
                    col: 3,
                    text: "b".into(),
                    style: Style {
                        fg: THEME.foreground,
                        ..Style::default()
                    }
                },
            ]
        );
    }

    #[test]
    fn strikethrough_and_inverse_are_read() {
        let row = {
            let mut screen = Screen::new(Size { rows: 1, cols: 4 });
            screen.process(b"\x1b[9ma\x1b[0;7mb");
            read(&screen.term, 0, 4, &THEME, None)
        };
        assert!(row[0].style.strike);
        assert_eq!(
            (row[1].style.fg, row[1].bg),
            (THEME.background, THEME.foreground)
        );
    }
}
