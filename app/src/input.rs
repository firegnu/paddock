//! Key, mouse and paste encoding for the terminal. From Saddle `src/input.rs` at commit `df1c727`,
//! without the TUI's focus routing (`Focus`, `Route`). Since M1 the events are paddock's own types
//! below instead of crossterm's (and the pane area instead of ratatui's `Rect`); the bytes are
//! unchanged.

/// A key the terminal encodes itself; plain text comes through the platform text input instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    BackTab,
    Backspace,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
}

/// The modifiers the terminal encodings know about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}
impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        alt: false,
        control: false,
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const CONTROL: Self = Self {
        control: true,
        ..Self::NONE
    };
}

/// A key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}
impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

/// A mouse event at a cell, in the same coordinates as the `Area` it is encoded against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}

/// The cells the terminal occupies; mouse positions are reported relative to its corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
impl Area {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    fn contains(&self, column: u16, row: u16) -> bool {
        column >= self.x
            && row >= self.y
            && u32::from(column) < u32::from(self.x) + u32::from(self.width)
            && u32::from(row) < u32::from(self.y) + u32::from(self.height)
    }
}

pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Vec<u8> {
    let Modifiers {
        shift,
        alt,
        control: ctrl,
    } = key.modifiers;
    let parameter = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    if key.code == KeyCode::Enter && (shift || ctrl) {
        return format!("\x1b[13;{parameter}u").into_bytes();
    }
    let special = match key.code {
        KeyCode::Up => Some(('A', 1)),
        KeyCode::Down => Some(('B', 1)),
        KeyCode::Right => Some(('C', 1)),
        KeyCode::Left => Some(('D', 1)),
        KeyCode::Home => Some(('H', 1)),
        KeyCode::End => Some(('F', 1)),
        KeyCode::Insert => Some(('~', 2)),
        KeyCode::Delete => Some(('~', 3)),
        KeyCode::PageUp => Some(('~', 5)),
        KeyCode::PageDown => Some(('~', 6)),
        KeyCode::F(n @ 1..=4) => Some(((b'P' + n - 1) as char, 1)),
        KeyCode::F(n @ 5..=12) => Some(('~', [15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)])),
        _ => None,
    };
    if let Some((suffix, number)) = special {
        let text = if parameter > 1 {
            format!("\x1b[{number};{parameter}{suffix}")
        } else if suffix == '~' {
            format!("\x1b[{number}~")
        } else if application_cursor || matches!(key.code, KeyCode::F(_)) {
            format!("\x1bO{suffix}")
        } else {
            format!("\x1b[{suffix}")
        };
        return text.into_bytes();
    }
    let mut bytes = match key.code {
        KeyCode::Char(c) if ctrl => {
            let c = c.to_ascii_uppercase();
            match c {
                '?' => vec![0x7f],
                '4'..='7' => vec![c as u8 - b'4' + 0x1c],
                ' ' | '@'..='_' => vec![(c as u8) & 0x1f],
                _ => c.to_string().into_bytes(),
            }
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        _ => Vec::new(),
    };
    if alt {
        bytes.insert(0, 0x1b);
    }
    bytes
}

pub fn encode_mouse(
    event: MouseEvent,
    area: Area,
    mode: alacritty_terminal::term::TermMode,
) -> Vec<u8> {
    use alacritty_terminal::term::TermMode as T;
    use {MouseButton as B, MouseEventKind as E};
    if !area.contains(event.column, event.row) || !mode.intersects(T::MOUSE_MODE) {
        return Vec::new();
    }
    let (mut code, release) = match event.kind {
        E::Down(button) | E::Up(button) | E::Drag(button) => {
            if matches!(event.kind, E::Drag(_)) && !mode.intersects(T::MOUSE_DRAG | T::MOUSE_MOTION)
            {
                return Vec::new();
            }
            let button = match button {
                B::Left => 0,
                B::Middle => 1,
                B::Right => 2,
            };
            (
                button
                    + if matches!(event.kind, E::Drag(_)) {
                        32
                    } else {
                        0
                    },
                matches!(event.kind, E::Up(_)),
            )
        }
        E::Moved if mode.contains(T::MOUSE_MOTION) => (35, false),
        E::ScrollUp => (64, false),
        E::ScrollDown => (65, false),
        E::ScrollLeft => (66, false),
        E::ScrollRight => (67, false),
        _ => return Vec::new(),
    };
    if release && !mode.contains(T::SGR_MOUSE) {
        code = 3;
    }
    if event.modifiers.shift {
        code += 4;
    }
    if event.modifiers.alt {
        code += 8;
    }
    if event.modifiers.control {
        code += 16;
    }
    let x = u32::from(event.column - area.x + 1);
    let y = u32::from(event.row - area.y + 1);
    if mode.contains(T::SGR_MOUSE) {
        return format!("\x1b[<{code};{x};{y}{}", if release { 'm' } else { 'M' }).into_bytes();
    }
    let mut result = b"\x1b[M".to_vec();
    for value in [code + 32, x + 32, y + 32] {
        if mode.contains(T::UTF8_MOUSE) {
            if let Some(c) = char::from_u32(value) {
                result.extend(c.to_string().as_bytes());
            }
        } else if value <= 255 {
            result.push(value as u8);
        } else {
            return Vec::new();
        }
    }
    result
}
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.as_bytes().to_vec()
    }
}
