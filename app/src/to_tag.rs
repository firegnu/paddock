//! The name tag on the active pane's terminal (DESIGN §13 P5-77): who the typing goes to, by the
//! input. Faint at rest, with the agent's name alone; lit for a moment, with `To` and the
//! session's topic, by the first text typed after a pause. The rules live here: when it lights,
//! where it goes and what it says. The window draws it (`window.rs`), the pane's view tells when
//! it was typed in and where its cursor is (`view.rs`).
use std::time::{Duration, Instant};

/// How long a pane goes without typing before the next text lights its tag; how long the tag
/// stays lit, and how long it takes to fade back.
const PAUSE: Duration = Duration::from_secs(10);
const LIT: Duration = Duration::from_secs(1);
const FADE: Duration = Duration::from_millis(400);

/// When a pane was typed in; each pane keeps its own.
#[derive(Debug, Default)]
pub struct Typing {
    last: Option<Instant>,
    lit: Option<Instant>,
}

impl Typing {
    /// `text` went from the keyboard to the pane's program at `now`: a key's letter, digit,
    /// symbol or space, what the input method composes or commits, or a paste. Whether the tag
    /// lit: the pane had not been typed in for [`PAUSE`]. No text (a composition given up) is
    /// no typing. The keys that bring no text never come here: the terminal encodes them itself
    /// (`keys`), and ⌘ shortcuts are the menu's.
    pub fn typed(&mut self, text: &str, now: Instant) -> bool {
        if text.is_empty() {
            return false;
        }
        let paused = self
            .last
            .is_none_or(|last| now.saturating_duration_since(last) >= PAUSE);
        self.last = Some(now);
        if paused {
            self.lit = Some(now);
        }
        paused
    }

    /// How lit the tag is at `now`, 0 at rest to 1.
    pub fn glow(&self, now: Instant) -> f32 {
        let Some(lit) = self.lit else {
            return 0.0;
        };
        let fading = now.saturating_duration_since(lit).saturating_sub(LIT);
        (1.0 - fading.as_secs_f32() / FADE.as_secs_f32()).max(0.0)
    }
}

/// The row of the view the cursor is on, for a cursor on `line` of a screen `rows` tall scrolled
/// `offset` lines back into its history; none when that leaves it out of view.
pub fn row_in_view(line: i32, offset: usize, rows: usize) -> Option<u16> {
    let row = usize::try_from(line).ok()? + offset;
    u16::try_from(row).ok().filter(|_| row < rows)
}

/// How far below the top of the terminal's area the top of a tag `tag` tall goes, the cursor on
/// `row` of lines `line` tall: centred on the line above the cursor's, where Claude Code draws
/// the upper edge of a one-line input, and never past the area's top. `None` for the area's
/// lower right corner: the cursor on the first line, or out of view.
pub fn top(row: Option<u16>, line: f32, tag: f32) -> Option<f32> {
    let above = row?.checked_sub(1)?;
    Some(
        (f32::from(above) * line + (line - tag) / 2.0)
            .round()
            .max(0.0),
    )
}

/// What a tag says: `To` before the name or not, the agent's whole name, and the session's topic.
#[derive(Debug, PartialEq)]
pub struct Label {
    pub to: bool,
    pub name: String,
    pub topic: Option<String>,
}

/// The tag of the agent `name` whose session is about `topic`: at rest the name alone, `lit` with
/// `To` before it and the topic after, when there is one.
pub fn label(name: &str, topic: Option<&str>, lit: bool) -> Label {
    Label {
        to: lit,
        name: name.to_owned(),
        topic: topic.filter(|_| lit).map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(seconds: f32) -> Duration {
        Duration::from_secs_f32(seconds)
    }

    #[test]
    fn the_first_text_after_ten_seconds_without_typing_lights_the_tag() {
        let start = Instant::now();
        let mut typing = Typing::default();
        assert_eq!(typing.glow(start), 0.0);
        // A pane never typed in has gone without typing.
        assert!(typing.typed("a", start));
        assert_eq!(typing.glow(start), 1.0);
        // Typing on does not light it again, however long it goes on.
        for seconds in [0.2, 5.0, 14.0, 23.0] {
            assert!(!typing.typed("b", start + secs(seconds)), "{seconds}");
        }
        assert_eq!(typing.glow(start + secs(23.0)), 0.0);
        // Ten seconds after the last text, the next lights it; a little less does not.
        assert!(!typing.typed("c", start + secs(32.5)));
        assert!(typing.typed("d", start + secs(42.5)));
        assert_eq!(typing.glow(start + secs(42.5)), 1.0);
    }

    #[test]
    fn the_tag_stays_lit_a_second_and_fades_in_four_tenths() {
        let start = Instant::now();
        let mut typing = Typing::default();
        typing.typed("a", start);
        // Text typed meanwhile neither cuts the second short nor starts it again.
        typing.typed("b", start + secs(0.5));
        for (seconds, glow) in [(0.0, 1.0), (0.99, 1.0), (1.2, 0.5), (1.4, 0.0), (9.0, 0.0)] {
            let lit = typing.glow(start + secs(seconds));
            assert!((lit - glow).abs() < 0.01, "{seconds}: {lit}");
        }
    }

    #[test]
    fn each_pane_counts_its_own_typing() {
        let start = Instant::now();
        let (mut one, mut other) = (Typing::default(), Typing::default());
        one.typed("a", start);
        assert!(other.typed("b", start + secs(3.0)));
        assert!(!one.typed("c", start + secs(6.0)));
    }

    #[test]
    fn text_typed_composed_or_pasted_is_typing_and_no_text_is_not() {
        let start = Instant::now();
        // A letter, a capital, a digit, a symbol, a space; the input method composing and
        // committing; a paste.
        for text in ["a", "A", "7", "/", " ", "ni", "你好", "cargo test\n--all"] {
            assert!(Typing::default().typed(text, start), "{text:?}");
        }
        // A composition given up sends nothing: the pause goes on, and the next text lights.
        let mut typing = Typing::default();
        assert!(!typing.typed("", start));
        assert_eq!(typing.glow(start), 0.0);
        assert!(typing.typed("a", start + secs(1.0)));
        assert!(!typing.typed("", start + secs(12.0)));
        assert!(typing.typed("b", start + secs(12.5)));
    }

    #[test]
    fn the_tag_sits_on_the_line_above_the_cursor() {
        // Centred on the line above: the 22 point tag reaches a point past a 20 point line.
        assert_eq!(top(Some(30), 20.0, 22.0), Some(579.0));
        assert_eq!(top(Some(31), 20.0, 22.0), Some(599.0));
        assert_eq!(top(Some(2), 30.0, 22.0), Some(34.0));
        // On the first line there is none above; nor when the cursor is out of view.
        assert_eq!(top(Some(0), 20.0, 22.0), None);
        assert_eq!(top(None, 20.0, 22.0), None);
        // Under the first line it does not reach past the area's top.
        assert_eq!(top(Some(1), 20.0, 22.0), Some(0.0));
    }

    #[test]
    fn a_cursor_scrolled_out_of_view_has_no_row() {
        assert_eq!(row_in_view(30, 0, 40), Some(30));
        // Scrolled back, the cursor's line moves down the view, then out of it.
        assert_eq!(row_in_view(30, 9, 40), Some(39));
        assert_eq!(row_in_view(30, 10, 40), None);
        assert_eq!(row_in_view(0, 0, 40), Some(0));
        assert_eq!(row_in_view(-1, 0, 40), None);
    }

    #[test]
    fn the_tag_names_the_agent_and_lit_adds_to_and_the_topic() {
        let topic = Some("读取 HANDOFF 进入状态");
        assert_eq!(
            label("paddock/main", topic, false),
            Label {
                to: false,
                name: "paddock/main".into(),
                topic: None,
            }
        );
        assert_eq!(
            label("paddock/main", topic, true),
            Label {
                to: true,
                name: "paddock/main".into(),
                topic: topic.map(str::to_owned),
            }
        );
        // Without a topic, the name alone after `To`.
        assert_eq!(
            label("paddock/main", None, true),
            Label {
                to: true,
                name: "paddock/main".into(),
                topic: None,
            }
        );
    }
}
