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
    vte::ansi::{
        Attr, CharsetIndex, ClearMode, Color, CursorShape, CursorStyle, Handler, Hyperlink,
        KeyboardModes, KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys,
        NamedColor, PrivateMode, Processor, Rgb, ScpCharPath, ScpUpdateMode, StandardCharset,
        TabulationClearMode,
    },
};
use cursor_icon::CursorIcon;
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
    /// Where colour replies go, in turn with the terminal's other replies.
    replies: Events,
    /// What the program last set as the terminal's title, for its pane's header (P5-59).
    title: Option<String>,
    /// The colours the pane draws with: what a program asking for a colour it has not set is told.
    theme: palette::Shared,
}
impl Screen {
    pub fn new(size: Size) -> Self {
        Self::with_theme(size, palette::Shared::default())
    }
    /// `theme`: the pane's colours, read when a program asks for one.
    pub fn with_theme(size: Size, theme: palette::Shared) -> Self {
        let (tx, events) = mpsc::channel();
        let replies = Events(tx);
        Self {
            term: Term::new(Default::default(), &size, replies.clone()),
            parser: Processor::new(),
            events,
            replies,
            title: None,
            theme,
        }
    }
    pub fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut answering = Answering {
            term: &mut self.term,
            theme: &self.theme,
            replies: &self.replies,
        };
        self.parser.advance(&mut answering, bytes);
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
                // `Answering` answers colour queries itself; this only stays in case the terminal
                // asks some other way, answered with the colour as it is now.
                Event::ColorRequest(index, format) => {
                    format(color_at(&self.term, &self.theme.get(), index))
                }
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

/// What palette entry `index` (OSC 4's 0–255, then OSC 10–12's foreground, background and cursor)
/// is as the pane draws it: what the program set there, or else the theme's.
fn color_at(term: &Term<Events>, theme: &palette::Theme, index: usize) -> Rgb {
    let color = match index {
        0..=255 => Color::Indexed(index as u8),
        257 => Color::Named(NamedColor::Background),
        258 => Color::Named(NamedColor::Cursor),
        _ => Color::Named(NamedColor::Foreground),
    };
    let (r, g, b) = palette::resolve(color, term.colors(), theme);
    Rgb { r, g, b }
}

/// Hands everything the parser finds to `Term`, except colour queries, which it answers the
/// moment the parser reaches them: with the colours as they are then, even in the middle of one
/// OSC 4 or of a synchronized update the parser holds back and parses at its end (P5-143a).
///
/// Every `Handler` method has an empty default, so one left out here would quietly stop working.
/// When upgrading vte or alacritty_terminal, check this again against vte's `pub trait Handler`
/// and alacritty_terminal's `impl Handler for Term`: every method forwarded, only
/// `dynamic_color_sequence` answered here.
struct Answering<'a> {
    term: &'a mut Term<Events>,
    theme: &'a palette::Shared,
    replies: &'a Events,
}

impl Handler for Answering<'_> {
    /// As `Term` words its reply, with the colour as it is now.
    fn dynamic_color_sequence(&mut self, prefix: String, index: usize, terminator: &str) {
        let color = color_at(self.term, &self.theme.get(), index);
        self.replies.send_event(Event::PtyWrite(format!(
            "\x1b]{};rgb:{1:02x}{1:02x}/{2:02x}{2:02x}/{3:02x}{3:02x}{4}",
            prefix, color.r, color.g, color.b, terminator
        )));
    }
    fn set_title(&mut self, a0: Option<String>) {
        Handler::set_title(&mut *self.term, a0);
    }
    fn set_cursor_style(&mut self, a0: Option<CursorStyle>) {
        Handler::set_cursor_style(&mut *self.term, a0);
    }
    fn set_cursor_shape(&mut self, shape: CursorShape) {
        Handler::set_cursor_shape(&mut *self.term, shape);
    }
    fn input(&mut self, c: char) {
        Handler::input(&mut *self.term, c);
    }
    fn goto(&mut self, line: i32, col: usize) {
        Handler::goto(&mut *self.term, line, col);
    }
    fn goto_line(&mut self, line: i32) {
        Handler::goto_line(&mut *self.term, line);
    }
    fn goto_col(&mut self, col: usize) {
        Handler::goto_col(&mut *self.term, col);
    }
    fn insert_blank(&mut self, a0: usize) {
        Handler::insert_blank(&mut *self.term, a0);
    }
    fn move_up(&mut self, a0: usize) {
        Handler::move_up(&mut *self.term, a0);
    }
    fn move_down(&mut self, a0: usize) {
        Handler::move_down(&mut *self.term, a0);
    }
    fn identify_terminal(&mut self, intermediate: Option<char>) {
        Handler::identify_terminal(&mut *self.term, intermediate);
    }
    fn device_status(&mut self, a0: usize) {
        Handler::device_status(&mut *self.term, a0);
    }
    fn move_forward(&mut self, col: usize) {
        Handler::move_forward(&mut *self.term, col);
    }
    fn move_backward(&mut self, col: usize) {
        Handler::move_backward(&mut *self.term, col);
    }
    fn move_down_and_cr(&mut self, row: usize) {
        Handler::move_down_and_cr(&mut *self.term, row);
    }
    fn move_up_and_cr(&mut self, row: usize) {
        Handler::move_up_and_cr(&mut *self.term, row);
    }
    fn put_tab(&mut self, count: u16) {
        Handler::put_tab(&mut *self.term, count);
    }
    fn backspace(&mut self) {
        Handler::backspace(&mut *self.term);
    }
    fn carriage_return(&mut self) {
        Handler::carriage_return(&mut *self.term);
    }
    fn linefeed(&mut self) {
        Handler::linefeed(&mut *self.term);
    }
    fn bell(&mut self) {
        Handler::bell(&mut *self.term);
    }
    fn substitute(&mut self) {
        Handler::substitute(&mut *self.term);
    }
    fn newline(&mut self) {
        Handler::newline(&mut *self.term);
    }
    fn set_horizontal_tabstop(&mut self) {
        Handler::set_horizontal_tabstop(&mut *self.term);
    }
    fn scroll_up(&mut self, a0: usize) {
        Handler::scroll_up(&mut *self.term, a0);
    }
    fn scroll_down(&mut self, a0: usize) {
        Handler::scroll_down(&mut *self.term, a0);
    }
    fn insert_blank_lines(&mut self, a0: usize) {
        Handler::insert_blank_lines(&mut *self.term, a0);
    }
    fn delete_lines(&mut self, a0: usize) {
        Handler::delete_lines(&mut *self.term, a0);
    }
    fn erase_chars(&mut self, a0: usize) {
        Handler::erase_chars(&mut *self.term, a0);
    }
    fn delete_chars(&mut self, a0: usize) {
        Handler::delete_chars(&mut *self.term, a0);
    }
    fn move_backward_tabs(&mut self, count: u16) {
        Handler::move_backward_tabs(&mut *self.term, count);
    }
    fn move_forward_tabs(&mut self, count: u16) {
        Handler::move_forward_tabs(&mut *self.term, count);
    }
    fn save_cursor_position(&mut self) {
        Handler::save_cursor_position(&mut *self.term);
    }
    fn restore_cursor_position(&mut self) {
        Handler::restore_cursor_position(&mut *self.term);
    }
    fn clear_line(&mut self, mode: LineClearMode) {
        Handler::clear_line(&mut *self.term, mode);
    }
    fn clear_screen(&mut self, mode: ClearMode) {
        Handler::clear_screen(&mut *self.term, mode);
    }
    fn clear_tabs(&mut self, mode: TabulationClearMode) {
        Handler::clear_tabs(&mut *self.term, mode);
    }
    fn set_tabs(&mut self, interval: u16) {
        Handler::set_tabs(&mut *self.term, interval);
    }
    fn reset_state(&mut self) {
        Handler::reset_state(&mut *self.term);
    }
    fn reverse_index(&mut self) {
        Handler::reverse_index(&mut *self.term);
    }
    fn terminal_attribute(&mut self, attr: Attr) {
        Handler::terminal_attribute(&mut *self.term, attr);
    }
    fn set_mode(&mut self, mode: Mode) {
        Handler::set_mode(&mut *self.term, mode);
    }
    fn unset_mode(&mut self, mode: Mode) {
        Handler::unset_mode(&mut *self.term, mode);
    }
    fn report_mode(&mut self, mode: Mode) {
        Handler::report_mode(&mut *self.term, mode);
    }
    fn set_private_mode(&mut self, mode: PrivateMode) {
        Handler::set_private_mode(&mut *self.term, mode);
    }
    fn unset_private_mode(&mut self, mode: PrivateMode) {
        Handler::unset_private_mode(&mut *self.term, mode);
    }
    fn report_private_mode(&mut self, mode: PrivateMode) {
        Handler::report_private_mode(&mut *self.term, mode);
    }
    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        Handler::set_scrolling_region(&mut *self.term, top, bottom);
    }
    fn set_keypad_application_mode(&mut self) {
        Handler::set_keypad_application_mode(&mut *self.term);
    }
    fn unset_keypad_application_mode(&mut self) {
        Handler::unset_keypad_application_mode(&mut *self.term);
    }
    fn set_active_charset(&mut self, a0: CharsetIndex) {
        Handler::set_active_charset(&mut *self.term, a0);
    }
    fn configure_charset(&mut self, a0: CharsetIndex, a1: StandardCharset) {
        Handler::configure_charset(&mut *self.term, a0, a1);
    }
    fn set_color(&mut self, a0: usize, a1: Rgb) {
        Handler::set_color(&mut *self.term, a0, a1);
    }
    fn reset_color(&mut self, a0: usize) {
        Handler::reset_color(&mut *self.term, a0);
    }
    fn clipboard_store(&mut self, a0: u8, a1: &[u8]) {
        Handler::clipboard_store(&mut *self.term, a0, a1);
    }
    fn clipboard_load(&mut self, a0: u8, a1: &str) {
        Handler::clipboard_load(&mut *self.term, a0, a1);
    }
    fn decaln(&mut self) {
        Handler::decaln(&mut *self.term);
    }
    fn push_title(&mut self) {
        Handler::push_title(&mut *self.term);
    }
    fn pop_title(&mut self) {
        Handler::pop_title(&mut *self.term);
    }
    fn text_area_size_pixels(&mut self) {
        Handler::text_area_size_pixels(&mut *self.term);
    }
    fn text_area_size_chars(&mut self) {
        Handler::text_area_size_chars(&mut *self.term);
    }
    fn set_hyperlink(&mut self, a0: Option<Hyperlink>) {
        Handler::set_hyperlink(&mut *self.term, a0);
    }
    fn set_mouse_cursor_icon(&mut self, a0: CursorIcon) {
        Handler::set_mouse_cursor_icon(&mut *self.term, a0);
    }
    fn report_keyboard_mode(&mut self) {
        Handler::report_keyboard_mode(&mut *self.term);
    }
    fn push_keyboard_mode(&mut self, mode: KeyboardModes) {
        Handler::push_keyboard_mode(&mut *self.term, mode);
    }
    fn pop_keyboard_modes(&mut self, to_pop: u16) {
        Handler::pop_keyboard_modes(&mut *self.term, to_pop);
    }
    fn set_keyboard_mode(&mut self, mode: KeyboardModes, behavior: KeyboardModesApplyBehavior) {
        Handler::set_keyboard_mode(&mut *self.term, mode, behavior);
    }
    fn set_modify_other_keys(&mut self, mode: ModifyOtherKeys) {
        Handler::set_modify_other_keys(&mut *self.term, mode);
    }
    fn report_modify_other_keys(&mut self) {
        Handler::report_modify_other_keys(&mut *self.term);
    }
    fn set_scp(&mut self, char_path: ScpCharPath, update_mode: ScpUpdateMode) {
        Handler::set_scp(&mut *self.term, char_path, update_mode);
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
        Screen::with_theme(Size { rows: 4, cols: 20 }, palette::Shared::new(THEME))
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
        let theme = palette::Shared::new(THEME);
        let mut screen = Screen::with_theme(Size { rows: 4, cols: 20 }, theme.clone());
        let mut light = THEME;
        light.background = (0xf0, 0xf1, 0xf2);
        light.ansi[1] = (0x44, 0x55, 0x66);
        // The pane changes its colours; the screen reads them.
        theme.set(light);
        assert_eq!(
            reply(&mut screen, b"\x1b]11;?\x07"),
            "\x1b]11;rgb:f0f0/f1f1/f2f2\x07"
        );
        assert_eq!(
            reply(&mut screen, b"\x1b]4;1;?\x07"),
            "\x1b]4;1;rgb:4444/5555/6666\x07"
        );
    }

    /// The replies to `stream` read in `parts` (cut at these offsets).
    fn replies_in_parts(stream: &[u8], cuts: &[usize]) -> String {
        let mut screen = themed();
        let mut replies = Vec::new();
        let mut start = 0;
        for &cut in cuts.iter().chain(std::iter::once(&stream.len())) {
            replies.extend(screen.process(&stream[start..cut]));
            start = cut;
        }
        String::from_utf8(replies).unwrap()
    }

    /// However the PTY splits the output, each query is answered with the colour as it was right
    /// then: whole, cut after each sequence, cut between every byte (an ESC and its `\` apart).
    #[test]
    fn a_query_is_answered_as_the_colour_was_then_however_the_output_is_cut() {
        let bel: &[&[u8]] = &[
            b"\x1b]4;1;rgb:12/34/56\x07",
            b"\x1b]4;1;?\x07",
            b"\x1b]104;1\x07",
            b"\x1b]4;1;?\x07",
            b"\x1b]11;rgb:01/02/03\x07",
            b"\x1b]11;?\x07",
            b"\x1b]11;rgb:04/05/06\x07",
            b"\x1b]11;?\x07",
            b"\x1b]111\x07",
            b"\x1b]11;?\x07",
        ];
        let st: Vec<Vec<u8>> = bel
            .iter()
            .map(|part| {
                let mut part = part[..part.len() - 1].to_vec();
                part.extend_from_slice(b"\x1b\\");
                part
            })
            .collect();
        let want_bel = "\x1b]4;1;rgb:1212/3434/5656\x07\x1b]4;1;rgb:1111/2222/3333\x07\
                        \x1b]11;rgb:0101/0202/0303\x07\x1b]11;rgb:0404/0505/0606\x07\
                        \x1b]11;rgb:1e1e/1f1f/2020\x07";
        let want_st = want_bel.replace('\x07', "\x1b\\");
        for (parts, want) in [
            (
                bel.iter().map(|p| p.to_vec()).collect::<Vec<_>>(),
                want_bel.to_owned(),
            ),
            (st, want_st),
        ] {
            let stream: Vec<u8> = parts.concat();
            let mut each = Vec::new();
            let mut at = 0;
            for part in &parts[..parts.len() - 1] {
                at += part.len();
                each.push(at);
            }
            let every: Vec<usize> = (1..stream.len()).collect();
            assert_eq!(replies_in_parts(&stream, &each), want, "cut after each");
            assert_eq!(replies_in_parts(&stream, &every), want, "cut at every byte");
            assert_eq!(replies_in_parts(&stream, &[]), want, "whole");
            // An ESC at the end of one read, its `\` and the rest at the start of the next.
            if let Some(st) = stream.windows(2).position(|w| w == b"\x1b\\") {
                assert_eq!(replies_in_parts(&stream, &[st + 1]), want, "ESC, \\ apart");
            }
        }
    }

    /// Inside a synchronized update the parser holds the bytes back and parses them all at its end:
    /// still each query is answered as the colour was right then.
    #[test]
    fn a_query_inside_a_synchronized_update_is_answered_as_the_colour_was_then() {
        let mut screen = Screen::new(Size { rows: 4, cols: 20 });
        let replies = screen.process(
            b"\x1b[?2026h\x1b]4;1;rgb:12/34/56\x07\x1b]4;1;?\x07\x1b]104;1\x07\x1b]4;1;?\x07\x1b[?2026l",
        );
        assert_eq!(
            String::from_utf8(replies).unwrap(),
            "\x1b]4;1;rgb:1212/3434/5656\x07\x1b]4;1;rgb:cdcd/0000/0000\x07"
        );
    }

    /// One OSC 4 that sets and asks in turn: each ask is told the colour as set just before it.
    #[test]
    fn sets_and_queries_in_one_osc_4_are_answered_in_turn() {
        let mut screen = Screen::new(Size { rows: 4, cols: 20 });
        let replies = screen.process(b"\x1b]4;1;rgb:12/34/56;1;?;1;rgb:65/43/21;1;?\x07");
        assert_eq!(
            String::from_utf8(replies).unwrap(),
            "\x1b]4;1;rgb:1212/3434/5656\x07\x1b]4;1;rgb:6565/4343/2121\x07"
        );
    }
}
