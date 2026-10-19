//! The terminal parser and its replies to terminal queries. From Saddle `src/terminal.rs` at
//! commit `df1c727`, without `Screen::render`/`color` (they draw into a ratatui buffer) and the
//! `history` field that only Saddle's history mode uses; paddock keeps the title the program sets,
//! for its pane's header (DESIGN §13 P5-59), and answers colour queries with the pane's theme
//! (P5-143a).
use crate::palette;
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    vte::ansi::{Color, NamedColor, Processor, Rgb},
};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Copy)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }
    fn screen_lines(&self) -> usize {
        usize::from(self.rows.max(1))
    }
    fn columns(&self) -> usize {
        usize::from(self.cols.max(2))
    }
}
#[derive(Clone)]
pub struct Events(Sender<Event>);
impl EventListener for Events {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event);
    }
}
pub struct Screen {
    pub term: Term<Events>,
    parser: Processor,
    events: Receiver<Event>,
    /// What the program last set as the terminal's title, for its pane's header (P5-59).
    title: Option<String>,
    /// The colours the pane draws with: what a program asking for a colour it has not set is told.
    theme: palette::Theme,
}
impl Screen {
    pub fn new(size: Size) -> Self {
        let (tx, events) = mpsc::channel();
        Self {
            term: Term::new(Default::default(), &size, Events(tx)),
            parser: Processor::new(),
            events,
            title: None,
            theme: palette::Theme::XTERM,
        }
    }
    /// The colours the pane draws with, for the answers to colour queries from now on.
    pub fn set_theme(&mut self, theme: palette::Theme) {
        self.theme = theme;
    }
    pub fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.parser.advance(&mut self.term, bytes);
        let mut replies = Vec::new();
        for event in self.events.try_iter() {
            let reply = match event {
                Event::PtyWrite(text) => text,
                Event::TextAreaSizeRequest(format) => {
                    format(alacritty_terminal::event::WindowSize {
                        num_lines: self.term.screen_lines() as u16,
                        num_cols: self.term.columns() as u16,
                        cell_width: 0,
                        cell_height: 0,
                    })
                }
                Event::ColorRequest(index, format) => format(self.color(index)),
                Event::Title(title) => {
                    self.title = Some(title);
                    continue;
                }
                Event::ResetTitle => {
                    self.title = None;
                    continue;
                }
                _ => continue,
            };
            replies.extend_from_slice(reply.as_bytes());
        }
        replies
    }
    /// What palette entry `index` (OSC 4's 0–255, then OSC 10–12's foreground, background and
    /// cursor) is as the pane draws it: what the program set there, or else the theme's.
    fn color(&self, index: usize) -> Rgb {
        let color = match index {
            0..=255 => Color::Indexed(index as u8),
            257 => Color::Named(NamedColor::Background),
            258 => Color::Named(NamedColor::Cursor),
            _ => Color::Named(NamedColor::Foreground),
        };
        let (r, g, b) = palette::resolve(color, self.term.colors(), &self.theme);
        Rgb { r, g, b }
    }
    pub fn resize(&mut self, size: Size) {
        self.term.resize(size);
    }
    /// The title the program last set for the terminal (OSC 0 or 2), when it set one.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref().filter(|title| !title.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_a_program_sets_reaches_the_screen() {
        let mut screen = Screen::new(Size { rows: 4, cols: 20 });
        assert_eq!(screen.title(), None);
        screen.process(b"\x1b]0;\xe2\x9c\xb3 Fix the seam\x07");
        assert_eq!(screen.title(), Some("✳ Fix the seam"));
        screen.process(b"\x1b]2;Another\x1b\\");
        assert_eq!(screen.title(), Some("Another"));
        // An empty title takes it back.
        screen.process(b"\x1b]2;\x07");
        assert_eq!(screen.title(), None);
    }

    const THEME: palette::Theme = palette::Theme {
        ansi: [
            (1, 2, 3),
            (0x11, 0x22, 0x33),
            (5, 6, 7),
            (8, 9, 10),
            (11, 12, 13),
            (14, 15, 16),
            (17, 18, 19),
            (20, 21, 22),
            (23, 24, 25),
            (26, 27, 28),
            (29, 30, 31),
            (32, 33, 34),
            (35, 36, 37),
            (38, 39, 40),
            (41, 42, 43),
            (0xfa, 0xfb, 0xfc),
        ],
        foreground: (0xd0, 0xd1, 0xd2),
        background: (0x1e, 0x1f, 0x20),
        cursor: (0xc0, 0xc1, 0xc2),
        selection: (60, 60, 60),
    };

    fn themed() -> Screen {
        let mut screen = Screen::new(Size { rows: 4, cols: 20 });
        screen.set_theme(THEME);
        screen
    }

    fn reply(screen: &mut Screen, query: &[u8]) -> String {
        String::from_utf8(screen.process(query)).unwrap()
    }

    #[test]
    fn colour_queries_are_answered_with_the_panes_theme() {
        let mut screen = themed();
        assert_eq!(
            reply(&mut screen, b"\x1b]11;?\x07"),
            "\x1b]11;rgb:1e1e/1f1f/2020\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]10;?\x1b\\"),
            "\x1b]10;rgb:d0d0/d1d1/d2d2\x1b\\"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]12;?\x07"),
            "\x1b]12;rgb:c0c0/c1c1/c2c2\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]4;1;?\x07"),
            "\x1b]4;1;rgb:1111/2222/3333\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]4;15;?\x07"),
            "\x1b]4;15;rgb:fafa/fbfb/fcfc\x07"
        );
        // Past the 16 basic colours, the standard xterm 256-colour palette.
        assert_eq!(
            reply(&mut screen, b"\x1b]4;21;?\x07"),
            "\x1b]4;21;rgb:0000/0000/ffff\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]4;232;?\x07"),
            "\x1b]4;232;rgb:0808/0808/0808\x07"
        );
    }

    #[test]
    fn a_colour_the_program_set_is_answered_until_it_resets_it() {
        let mut screen = themed();
        screen.process(b"\x1b]4;1;rgb:12/34/56\x07\x1b]11;rgb:01/02/03\x07");
        assert_eq!(
            reply(&mut screen, b"\x1b]4;1;?\x07"),
            "\x1b]4;1;rgb:1212/3434/5656\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]11;?\x07"),
            "\x1b]11;rgb:0101/0202/0303\x07"
        );
        screen.process(b"\x1b]104;1\x07\x1b]111\x07");
        assert_eq!(
            reply(&mut screen, b"\x1b]4;1;?\x07"),
            "\x1b]4;1;rgb:1111/2222/3333\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]11;?\x07"),
            "\x1b]11;rgb:1e1e/1f1f/2020\x07"
        );
    }

    #[test]
    fn after_a_theme_change_queries_are_answered_with_the_new_theme() {
        let mut screen = themed();
        let mut light = THEME;
        light.background = (0xf0, 0xf1, 0xf2);
        light.ansi[1] = (0x44, 0x55, 0x66);
        screen.set_theme(light);
        assert_eq!(
            reply(&mut screen, b"\x1b]11;?\x07"),
            "\x1b]11;rgb:f0f0/f1f1/f2f2\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]4;1;?\x07"),
            "\x1b]4;1;rgb:4444/5555/6666\x07"
        );
    }
}
