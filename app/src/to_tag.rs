//! The name tag on the active pane's terminal (DESIGN §13 P5-77, P5-77c, P5-77d): who the typing
//! goes to, by the input. At rest a faint capsule with the agent's name, at the right of the
//! nearest free row above the cursor. While the pane is typed in, a larger, solid one in its
//! place, with `To` and the session's topic: it slides up as the typing begins, and fades three
//! seconds after the last text or when Enter sends it. The rules live here: when it is
//! lit, where the tags go and what they say. The window draws them (`window.rs`), the pane's
//! view tells when it was typed in, where its cursor is and which rows are free (`view.rs`).
use crate::layout::PaneId;
use std::time::{Duration, Instant};

/// How long after the last text the tag stays lit, and how long it takes to fade back; how long
/// it takes to slide up as it begins, and from how many points below.
const HOLD: Duration = Duration::from_secs(3);
const FADE: Duration = Duration::from_millis(400);
const RISE: Duration = Duration::from_millis(200);
pub const RISE_FROM: f32 = 10.0;
/// How many rows above the cursor a tag looks for a free one.
const REACH: u16 = 6;

/// When a pane was typed in; each pane keeps its own.
#[derive(Debug, Default)]
pub struct Typing {
    /// When the typing that lit the tag began, its last text, and when Enter sent it.
    began: Option<Instant>,
    last: Option<Instant>,
    entered: Option<Instant>,
    /// The start of the fade has been told ([`Self::due`]).
    told: bool,
}

impl Typing {
    /// When the tag starts to fade: [`HOLD`] after the last text, or as Enter sent it.
    fn fades(&self) -> Option<Instant> {
        let held = self.last? + HOLD;
        Some(self.entered.map_or(held, |entered| entered.min(held)))
    }

    /// `text` went from the keyboard to the pane's program at `now`: a key's letter, digit,
    /// symbol or space, what the input method composes or commits, or a paste. The tag is lit
    /// from then; whether that began it, rather than kept it lit. No text (a composition given
    /// up) is no typing. The keys that bring no text never come here: the terminal encodes them
    /// itself (`keys`), and ⌘ shortcuts are the menu's.
    pub fn typed(&mut self, text: &str, now: Instant) -> bool {
        if text.is_empty() {
            return false;
        }
        let begins = self.glow(now) <= 0.0;
        if begins {
            self.began = Some(now);
        }
        self.last = Some(now);
        self.entered = None;
        self.told = false;
        begins
    }

    /// Enter sent what was typed, at `now`: a lit tag fades from then.
    pub fn sent(&mut self, now: Instant) {
        if self.entered.is_none() && self.fades().is_some_and(|fades| now < fades) {
            self.entered = Some(now);
        }
    }

    /// The typing goes to someone else than before through this pane (see [`Target`]): a lit tag
    /// goes at once, not to name the new one in the old one's place.
    pub fn elsewhere(&mut self) {
        *self = Self::default();
    }

    /// How lit the tag is at `now`, 0 at rest to 1.
    pub fn glow(&self, now: Instant) -> f32 {
        let Some(fades) = self.fades() else {
            return 0.0;
        };
        let gone = now.saturating_duration_since(fades);
        (1.0 - gone.as_secs_f32() / FADE.as_secs_f32()).max(0.0)
    }

    /// How many points below its place the lit tag is at `now`: [`RISE_FROM`] as the typing
    /// begins, sliding up to none, fast at first.
    pub fn rise(&self, now: Instant) -> f32 {
        let Some(began) = self.began else {
            return 0.0;
        };
        let risen = now.saturating_duration_since(began).as_secs_f32() / RISE.as_secs_f32();
        let left = 1.0 - risen.min(1.0);
        RISE_FROM * left * left
    }

    /// Whether the lit tag is on its way at `now`, sliding up or fading, and so drawn anew each
    /// frame.
    pub fn moving(&self, now: Instant) -> bool {
        let glow = self.glow(now);
        glow > 0.0 && (glow < 1.0 || self.rise(now) > 0.0)
    }

    /// Whether the fade starts by `now` and has not been told yet: once each time the tag is
    /// lit, for the view to have it drawn again when nothing else would.
    pub fn due(&mut self, now: Instant) -> bool {
        let due = !self.told && self.fades().is_some_and(|fades| now >= fades);
        self.told |= due;
        due
    }
}

/// Who the typing goes to, as the window last had it: the active pane and the agent it shows.
#[derive(Debug, Default)]
pub struct Target(Option<(PaneId, Option<String>)>);

impl Target {
    /// Takes the active `pane` and the `agent` it shows. Whether the typing goes elsewhere than
    /// before: to another pane, or through the same pane to another agent, as when the one pane
    /// of a window shows whichever agent the sidebar chose.
    pub fn moved(&mut self, pane: PaneId, agent: Option<&str>) -> bool {
        let now = Some((pane, agent.map(str::to_owned)));
        let moved = self.0 != now;
        self.0 = now;
        moved
    }
}

/// The row of the view the cursor is on, for a cursor on `line` of a screen `rows` tall scrolled
/// `offset` lines back into its history; none when that leaves it out of view.
pub fn row_in_view(line: i32, offset: usize, rows: usize) -> Option<u16> {
    let row = usize::try_from(line).ok()? + offset;
    u16::try_from(row).ok().filter(|_| row < rows)
}

/// How far from the terminal's left a tag `width` wide starts, kept to the right of the `room`
/// the terminal has, `end` in from it: wherever the cursor is, not to move as the typing goes
/// (P5-77d). From the terminal's left when there is no room for it.
pub fn flush_right(width: f32, room: f32, end: f32) -> f32 {
    (room - end - width).max(0.0)
}

/// Whether a cell showing `c` leaves a tag room: a space, or a line of a drawn box, as the
/// upper edge of Claude Code's input.
pub fn blank(c: char) -> bool {
    matches!(c, ' ' | '\0' | '\u{2500}'..='\u{257F}')
}

/// The row a tag goes on, for a cursor on `row`: the nearest above it, [`REACH`] rows away at
/// most, that `free` says holds nothing where the tag would be. Over a one-line input that is
/// its upper edge, right above the cursor; over more lines, the edge above them, not the text
/// just typed. None for the terminal's lower right corner: no such row, the cursor on the first
/// row, or out of view.
pub fn free_row(row: Option<u16>, free: impl Fn(u16) -> bool) -> Option<u16> {
    let row = row?;
    (1..=REACH)
        .map_while(|up| row.checked_sub(up))
        .find(|&row| free(row))
}

/// How far below the top of the terminal's area the top of a tag `tag` tall goes on `row` of
/// lines `line` tall: in the row's middle, or when taller than the row, its foot on the row's,
/// not to reach into the cursor's row under it; never past the area's top.
pub fn top_on(row: u16, line: f32, tag: f32) -> f32 {
    let spare = line - tag;
    (f32::from(row) * line + spare.min(spare / 2.0))
        .round()
        .max(0.0)
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

    fn near(got: f32, want: f32) -> bool {
        (got - want).abs() < 0.01
    }

    #[test]
    fn the_tag_is_lit_while_the_pane_is_typed_in_and_fades_three_seconds_after_the_last_text() {
        let start = Instant::now();
        let mut typing = Typing::default();
        assert_eq!(typing.glow(start), 0.0);
        // The first text begins it.
        assert!(typing.typed("a", start));
        assert_eq!(typing.glow(start + secs(0.5)), 1.0);
        // Typing on keeps it lit and begins nothing new, however long it goes on.
        for seconds in [1.0, 3.5, 6.0, 8.5] {
            assert!(!typing.typed("b", start + secs(seconds)), "{seconds}");
            assert_eq!(typing.glow(start + secs(seconds + 2.9)), 1.0, "{seconds}");
        }
        // Three seconds after the last text it fades, in four tenths.
        assert_eq!(typing.glow(start + secs(11.4)), 1.0);
        assert!(near(typing.glow(start + secs(11.7)), 0.5));
        assert_eq!(typing.glow(start + secs(11.95)), 0.0);
        // The next text begins it again, however soon.
        assert!(typing.typed("c", start + secs(12.0)));
        assert_eq!(typing.glow(start + secs(12.0)), 1.0);
    }

    #[test]
    fn text_typed_while_the_tag_fades_lights_it_again_without_beginning_anew() {
        let start = Instant::now();
        let mut typing = Typing::default();
        typing.typed("a", start);
        assert!(near(typing.glow(start + secs(3.2)), 0.5));
        assert!(!typing.typed("b", start + secs(3.2)));
        assert_eq!(typing.glow(start + secs(3.2)), 1.0);
        assert_eq!(typing.rise(start + secs(3.2)), 0.0);
    }

    #[test]
    fn enter_sends_what_was_typed_and_the_tag_fades() {
        let start = Instant::now();
        let mut typing = Typing::default();
        // Nothing lit, nothing to fade.
        typing.sent(start);
        assert_eq!(typing.glow(start), 0.0);
        typing.typed("a", start);
        typing.sent(start + secs(1.0));
        assert_eq!(typing.glow(start + secs(1.0)), 1.0);
        assert!(near(typing.glow(start + secs(1.2)), 0.5));
        assert_eq!(typing.glow(start + secs(1.45)), 0.0);
        // Enter again while it fades does not start the fade over.
        typing.sent(start + secs(1.3));
        assert_eq!(typing.glow(start + secs(1.45)), 0.0);
        // The next message begins it again.
        assert!(typing.typed("b", start + secs(1.5)));
        assert_eq!(typing.glow(start + secs(4.4)), 1.0);
    }

    #[test]
    fn the_tag_goes_at_once_when_the_typing_goes_elsewhere() {
        let start = Instant::now();
        let mut typing = Typing::default();
        typing.typed("a", start);
        typing.elsewhere();
        assert_eq!(typing.glow(start + secs(0.1)), 0.0);
        assert!(typing.typed("b", start + secs(0.2)));
        assert_eq!(typing.glow(start + secs(0.2)), 1.0);
    }

    #[test]
    fn the_lit_tag_slides_up_ten_points_in_two_tenths_as_it_begins() {
        let start = Instant::now();
        let mut typing = Typing::default();
        typing.typed("a", start);
        assert_eq!(typing.rise(start), 10.0);
        let (early, late) = (
            typing.rise(start + secs(0.05)),
            typing.rise(start + secs(0.15)),
        );
        assert!(10.0 > early && early > late && late > 0.0, "{early} {late}");
        assert_eq!(typing.rise(start + secs(0.25)), 0.0);
        // Text typed meanwhile does not send it back down.
        typing.typed("b", start + secs(0.1));
        assert_eq!(typing.rise(start + secs(0.25)), 0.0);
        // The next time it begins, it slides again.
        typing.sent(start + secs(1.0));
        typing.typed("c", start + secs(2.0));
        assert_eq!(typing.rise(start + secs(2.0)), 10.0);
    }

    #[test]
    fn the_lit_tag_is_on_its_way_while_it_slides_up_and_while_it_fades() {
        let start = Instant::now();
        let mut typing = Typing::default();
        assert!(!typing.moving(start));
        typing.typed("a", start);
        assert!(typing.moving(start + secs(0.1)));
        // Lit and in place: nothing to draw anew.
        assert!(!typing.moving(start + secs(1.0)));
        assert!(typing.moving(start + secs(3.2)));
        assert!(!typing.moving(start + secs(3.5)));
    }

    #[test]
    fn the_fade_is_due_once_three_seconds_after_the_last_text() {
        let start = Instant::now();
        let mut typing = Typing::default();
        assert!(!typing.due(start));
        typing.typed("a", start);
        assert!(!typing.due(start + secs(2.9)));
        assert!(typing.due(start + secs(3.0)));
        assert!(!typing.due(start + secs(3.1)));
        typing.typed("b", start + secs(5.0));
        typing.typed("c", start + secs(6.0));
        assert!(!typing.due(start + secs(8.5)));
        assert!(typing.due(start + secs(9.0)));
    }

    #[test]
    fn each_pane_counts_its_own_typing() {
        let start = Instant::now();
        let (mut one, mut other) = (Typing::default(), Typing::default());
        one.typed("a", start);
        assert!(other.typed("b", start + secs(1.0)));
        assert!(!one.typed("c", start + secs(2.0)));
        other.sent(start + secs(2.0));
        assert_eq!(one.glow(start + secs(4.5)), 1.0);
        assert_eq!(other.glow(start + secs(2.5)), 0.0);
    }

    #[test]
    fn text_typed_composed_or_pasted_is_typing_and_no_text_is_not() {
        let start = Instant::now();
        // A letter, a capital, a digit, a symbol, a space; the input method composing and
        // committing; a paste.
        for text in ["a", "A", "7", "/", " ", "ni", "你好", "cargo test\n--all"] {
            assert!(Typing::default().typed(text, start), "{text:?}");
        }
        // A composition given up sends nothing: the tag is not lit by it, nor kept lit.
        let mut typing = Typing::default();
        assert!(!typing.typed("", start));
        assert_eq!(typing.glow(start), 0.0);
        assert!(typing.typed("a", start + secs(1.0)));
        assert!(!typing.typed("", start + secs(3.5)));
        assert_eq!(typing.glow(start + secs(4.5)), 0.0);
    }

    #[test]
    fn the_typing_goes_elsewhere_with_another_pane_or_another_agent_in_the_same_pane() {
        let mut target = Target::default();
        assert!(target.moved(1, Some("paddock/main")));
        assert!(!target.moved(1, Some("paddock/main")));
        // The one pane of a window, showing the agent the sidebar chose.
        assert!(target.moved(1, Some("mock_server/codex-1")));
        assert!(!target.moved(1, Some("mock_server/codex-1")));
        assert!(target.moved(1, Some("paddock/main")));
        // Another tab or another pane of a split.
        assert!(target.moved(2, Some("global-mesh/main")));
        // A shell or an empty pane, and back to an agent in it.
        assert!(target.moved(3, None));
        assert!(!target.moved(3, None));
        assert!(target.moved(3, Some("paddock/main")));
    }

    #[test]
    fn a_tag_keeps_to_the_terminal_s_right_wherever_the_cursor_is() {
        // Its right end ten points in from the terminal's.
        assert_eq!(flush_right(200.0, 1000.0, 10.0), 790.0);
        assert_eq!(flush_right(90.0, 1000.0, 10.0), 900.0);
        // No room for it and the ten points: from the terminal's left.
        assert_eq!(flush_right(995.0, 1000.0, 10.0), 0.0);
        assert_eq!(flush_right(1200.0, 1000.0, 10.0), 0.0);
    }

    #[test]
    fn spaces_and_box_lines_leave_a_tag_room_and_text_does_not() {
        for c in [' ', '\0', '─', '│', '╭', '╮', '╰', '━'] {
            assert!(blank(c), "{c:?}");
        }
        for c in ['a', '>', '❯', '你', '-', '_', '█', '·'] {
            assert!(!blank(c), "{c:?}");
        }
    }

    #[test]
    fn a_tag_goes_on_the_nearest_free_row_above_the_cursor() {
        // The row above the cursor: the upper edge of a one-line input.
        assert_eq!(free_row(Some(30), |_| true), Some(29));
        // Rows of the input's own text above the cursor: on to the edge above them.
        assert_eq!(free_row(Some(30), |row| row <= 27), Some(27));
        // Six rows up at most.
        assert_eq!(free_row(Some(30), |row| row <= 24), Some(24));
        assert_eq!(free_row(Some(30), |row| row <= 23), None);
        assert_eq!(free_row(Some(30), |_| false), None);
        // Not past the first row; none above it, nor for a cursor out of view.
        assert_eq!(free_row(Some(2), |row| row == 0), Some(0));
        assert_eq!(free_row(Some(0), |_| true), None);
        assert_eq!(free_row(None, |_| true), None);
    }

    #[test]
    fn a_tag_is_centred_on_its_row_and_a_taller_one_stands_on_its_lower_edge() {
        // Shorter than the line: in its middle.
        assert_eq!(top_on(2, 30.0, 22.0), 64.0);
        // Taller: its foot on the row's, so it never reaches into the cursor's row below.
        assert_eq!(top_on(29, 20.0, 22.0), 578.0);
        assert_eq!(top_on(29, 20.0, 28.0), 572.0);
        // Never past the area's top.
        assert_eq!(top_on(0, 20.0, 28.0), 0.0);
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
