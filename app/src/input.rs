//! Key, mouse and paste encoding for the terminal. From Saddle `src/input.rs` at commit `df1c727`,
//! without the TUI's focus routing (`Focus`, `Route`).
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Vec<u8> {
    if key.kind == KeyEventKind::Release {
        return Vec::new();
    }
    let modifiers = key.modifiers;
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
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
    event: crossterm::event::MouseEvent,
    area: ratatui::layout::Rect,
    mode: alacritty_terminal::term::TermMode,
) -> Vec<u8> {
    use alacritty_terminal::term::TermMode as T;
    use crossterm::event::{MouseButton as B, MouseEventKind as E};
    if !area.contains((event.column, event.row).into()) || !mode.intersects(T::MOUSE_MODE) {
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
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
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
