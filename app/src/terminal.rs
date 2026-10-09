//! The terminal parser and its replies to terminal queries. From Saddle `src/terminal.rs` at
//! commit `df1c727`, without `Screen::render`/`color` (they draw into a ratatui buffer) and the
//! `history` field that only Saddle's history mode uses; paddock keeps the title the program sets,
//! for its pane's header (DESIGN §13 P5-59).
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    vte::ansi::{Processor, Rgb},
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
}
impl Screen {
    pub fn new(size: Size) -> Self {
        let (tx, events) = mpsc::channel();
        Self {
            term: Term::new(Default::default(), &size, Events(tx)),
            parser: Processor::new(),
            events,
            title: None,
        }
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
                Event::ColorRequest(index, format) => format(default_rgb(index)),
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
    pub fn resize(&mut self, size: Size) {
        self.term.resize(size);
    }
    /// The title the program last set for the terminal (OSC 0 or 2), when it set one.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref().filter(|title| !title.is_empty())
    }
}

fn default_rgb(index: usize) -> Rgb {
    let (r, g, b) = match index {
        0..=15 => [
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
        ][index],
        16..=231 => {
            let n = index - 16;
            let c = [0, 95, 135, 175, 215, 255];
            (c[n / 36], c[n / 6 % 6], c[n % 6])
        }
        232..=255 => {
            let v = 8 + (index as u8 - 232) * 10;
            (v, v, v)
        }
        257 => (0, 0, 0),
        _ => (229, 229, 229),
    };
    Rgb { r, g, b }
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
}
