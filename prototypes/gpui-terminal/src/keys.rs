//! Which key presses the terminal encodes itself. Plain printable text is left to the platform
//! text input (and so to the input method); everything else is converted to the crossterm event
//! Saddle's existing `input::encode_key` already understands.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gpui::Keystroke;

pub fn key_event(keystroke: &Keystroke) -> Option<KeyEvent> {
    let m = &keystroke.modifiers;
    if m.platform {
        return None;
    }
    let mut modifiers = KeyModifiers::NONE;
    modifiers.set(KeyModifiers::SHIFT, m.shift);
    modifiers.set(KeyModifiers::ALT, m.alt);
    modifiers.set(KeyModifiers::CONTROL, m.control);
    let code = match keystroke.key.as_str() {
        "enter" => KeyCode::Enter,
        "tab" if m.shift => KeyCode::BackTab,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "escape" => KeyCode::Esc,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        key if key.len() > 1 && key.starts_with('f') => KeyCode::F(key[1..].parse().ok()?),
        // Typed text, including Option characters, comes through the platform text input.
        _ if !m.control => return None,
        "space" => KeyCode::Char(' '),
        key => {
            let mut chars = key.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(c)
        }
    };
    if code == KeyCode::BackTab {
        modifiers.remove(KeyModifiers::SHIFT);
    }
    Some(KeyEvent::new(code, modifiers))
}

/// The bytes for a key press, or `None` when the platform text input should handle it.
pub fn key_bytes(keystroke: &Keystroke, application_cursor: bool) -> Option<Vec<u8>> {
    key_event(keystroke).map(|event| saddle::input::encode_key(event, application_cursor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(source: &str) -> Option<Vec<u8>> {
        key_bytes(&Keystroke::parse(source).unwrap(), false)
    }

    #[test]
    fn plain_text_goes_to_the_input_method() {
        for source in ["a", "shift-a", "space", "1", "alt-a"] {
            assert_eq!(bytes(source), None, "{source}");
        }
    }

    #[test]
    fn command_shortcuts_stay_with_the_app() {
        assert_eq!(bytes("cmd-c"), None);
        assert_eq!(bytes("cmd-v"), None);
    }

    #[test]
    fn control_and_named_keys_use_saddle_encoding() {
        assert_eq!(bytes("ctrl-c").unwrap(), [0x03]);
        assert_eq!(bytes("ctrl-space").unwrap(), [0x00]);
        assert_eq!(bytes("enter").unwrap(), b"\r");
        assert_eq!(bytes("shift-enter").unwrap(), b"\x1b[13;2u");
        assert_eq!(bytes("tab").unwrap(), b"\t");
        assert_eq!(bytes("shift-tab").unwrap(), b"\x1b[Z");
        assert_eq!(bytes("escape").unwrap(), [0x1b]);
        assert_eq!(bytes("backspace").unwrap(), [0x7f]);
        assert_eq!(bytes("up").unwrap(), b"\x1b[A");
        assert_eq!(bytes("alt-left").unwrap(), b"\x1b[1;3D");
        assert_eq!(bytes("pagedown").unwrap(), b"\x1b[6~");
        assert_eq!(bytes("f5").unwrap(), b"\x1b[15~");
    }

    #[test]
    fn application_cursor_mode_changes_arrows() {
        let up = Keystroke::parse("up").unwrap();
        assert_eq!(key_bytes(&up, true).unwrap(), b"\x1bOA");
    }
}
