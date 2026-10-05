//! From Saddle `tests/input.rs` at commit `df1c727`, without the tests of the TUI focus routing.
//! Since M1 the events are paddock's own types; the expected bytes are unchanged.
use paddock::input::{KeyCode as K, KeyEvent, Modifiers as M};
fn key(code: K, modifiers: M) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}
#[test]
fn terminal_keys_preserve_utf8_control_alt_and_cursor_modes() {
    use paddock::input::encode_key;
    assert_eq!(
        encode_key(key(K::Char('中'), M::NONE), false),
        "中".as_bytes()
    );
    assert_eq!(encode_key(key(K::Char('c'), M::CONTROL), false), b"\x03");
    assert_eq!(encode_key(key(K::Char('x'), M::ALT), false), b"\x1bx");
    assert_eq!(encode_key(key(K::Up, M::NONE), false), b"\x1b[A");
    assert_eq!(encode_key(key(K::Up, M::NONE), true), b"\x1bOA");
    assert_eq!(encode_key(key(K::Left, M::CONTROL), true), b"\x1b[1;5D");
    assert_eq!(encode_key(key(K::F(5), M::SHIFT), false), b"\x1b[15;2~");
}

#[test]
fn mouse_coordinates_are_local_and_paste_obeys_inner_terminal_mode() {
    use alacritty_terminal::term::TermMode as T;
    use paddock::input::{
        Area, MouseButton as B, MouseEvent, MouseEventKind as E, encode_mouse, encode_paste,
    };
    let area = Area::new(53, 1, 66, 38);
    let event = MouseEvent {
        kind: E::Down(B::Left),
        column: 55,
        row: 3,
        modifiers: M::NONE,
    };
    assert_eq!(
        encode_mouse(event, area, T::MOUSE_REPORT_CLICK | T::SGR_MOUSE),
        b"\x1b[<0;3;3M"
    );
    assert_eq!(
        encode_mouse(
            MouseEvent {
                kind: E::Up(B::Left),
                ..event
            },
            area,
            T::MOUSE_REPORT_CLICK | T::SGR_MOUSE
        ),
        b"\x1b[<0;3;3m"
    );
    assert!(encode_mouse(event, area, T::empty()).is_empty());
    assert!(
        encode_mouse(
            MouseEvent { column: 0, ..event },
            area,
            T::MOUSE_REPORT_CLICK
        )
        .is_empty()
    );
    assert!(
        encode_mouse(
            MouseEvent {
                kind: E::Moved,
                ..event
            },
            area,
            T::MOUSE_DRAG | T::SGR_MOUSE
        )
        .is_empty()
    );
    assert_eq!(
        encode_paste("中文\ntext", true),
        "\x1b[200~中文\ntext\x1b[201~".as_bytes()
    );
    assert_eq!(encode_paste("plain", false), b"plain");
}

#[test]
fn legacy_control_digit_aliases_keep_their_original_bytes() {
    use paddock::input::encode_key;
    for (digit, byte) in [('4', 0x1c), ('6', 0x1e), ('7', 0x1f)] {
        assert_eq!(encode_key(key(K::Char(digit), M::CONTROL), false), [byte]);
    }
}

#[test]
fn modified_enter_is_distinct_from_submit_for_multiline_prompts() {
    use paddock::input::encode_key;
    assert_eq!(encode_key(key(K::Enter, M::SHIFT), false), b"\x1b[13;2u");
    assert_eq!(encode_key(key(K::Enter, M::CONTROL), false), b"\x1b[13;5u");
    assert_eq!(encode_key(key(K::Enter, M::ALT), false), b"\x1b\r");
}
