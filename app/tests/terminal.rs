//! From Saddle `tests/terminal.rs` at commit `df1c727`: the parser test only; the rendering and
//! history tests cover code paddock did not take.
use alacritty_terminal::{
    index::{Column, Line},
    term::{TermMode, cell::Flags},
    vte::ansi::{Color, Rgb},
};
use paddock::terminal::{Screen, Size};
#[test]
fn screen_preserves_split_utf8_color_modes_and_replies_to_terminal_queries() {
    let mut screen = Screen::new(Size { rows: 6, cols: 20 });
    let mut replies = Vec::new();
    for byte in
        "\x1b[2J\x1b[H\x1b[38;2;12;34;56m中\x1b[38;5;196mA\x1b[?1002h\x1b[?1006h\x1b[?2004h\x1b[6n"
            .as_bytes()
    {
        replies.extend(screen.process(&[*byte]));
    }
    let grid = screen.term.grid();
    assert_eq!(grid[Line(0)][Column(0)].c, '中');
    assert!(
        grid[Line(0)][Column(1)]
            .flags
            .contains(Flags::WIDE_CHAR_SPACER)
    );
    assert_eq!(
        grid[Line(0)][Column(0)].fg,
        Color::Spec(Rgb {
            r: 12,
            g: 34,
            b: 56
        })
    );
    assert_eq!(grid[Line(0)][Column(2)].fg, Color::Indexed(196));
    assert_eq!(replies, b"\x1b[1;4R");
    assert!(
        screen
            .term
            .mode()
            .contains(TermMode::MOUSE_DRAG | TermMode::SGR_MOUSE | TermMode::BRACKETED_PASTE)
    );
}
