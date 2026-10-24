//! A shell's commands as blocks in its pane (DESIGN §13 P5-149): which one the mouse is over,
//! where ⌘↑ and ⌘↓ go, what a click on a command's bar selects, how a jumped-to prompt is lit.
//! Apart from GPUI, so that these can be tested; the pane draws them.
use super::fade::Fade;
use crate::segments::Segment;
use alacritty_terminal::{
    index::{Line, Side},
    selection::{Selection, SelectionType},
};
use std::time::Duration;

/// The command whose lines include `line`; the later one where two share it.
pub fn at(segments: &[Segment], line: Line) -> Option<&Segment> {
    segments
        .iter()
        .rev()
        .find(|segment| segment.prompt <= line && line <= segment.last())
}

/// Where a jump starts: the command ⌘↑ or ⌘↓ went to last (`jumped`: its id and the view row its
/// prompt was put on) while the view, `offset` lines back, still has it there; else the view's top
/// line; at the bottom, below every command (none).
pub fn from(segments: &[Segment], jumped: Option<(i64, i32)>, offset: usize) -> Option<Line> {
    let offset = offset as i32;
    let kept = jumped.and_then(|(id, row)| {
        segments
            .iter()
            .find(|segment| segment.id == id && segment.prompt.0 + offset == row)
    });
    match kept {
        Some(segment) => Some(segment.prompt),
        None => (offset > 0).then_some(Line(-offset)),
    }
}

/// Where ⌘↑ (`up`) or ⌘↓ goes `from` a line (none: below every command): the command whose
/// prompt is the nearest before or after it; none past either end.
pub fn target(segments: &[Segment], from: Option<Line>, up: bool) -> Option<&Segment> {
    match (from, up) {
        (None, true) => segments.last(),
        (None, false) => None,
        (Some(line), true) => segments.iter().rev().find(|segment| segment.prompt < line),
        (Some(line), false) => segments.iter().find(|segment| segment.prompt > line),
    }
}

/// How far back the view goes for `prompt` to be its top line, as far as `history` allows.
pub fn offset_for(prompt: Line, history: usize) -> usize {
    (-i64::from(prompt.0)).clamp(0, history as i64) as usize
}

/// The selection a click on a command's bar makes: its output, without its command line; none
/// while it runs or when it printed nothing. `columns`: the grid's width.
pub fn output_selection(segment: &Segment, columns: usize) -> Option<Selection> {
    let (first, last) = segment.output(columns)?;
    let mut selection = Selection::new(SelectionType::Simple, first, Side::Left);
    selection.update(last, Side::Right);
    Some(selection)
}

/// The lines a jump lights: the prompt's to the last before the output.
pub fn command_lines(segment: &Segment) -> (Line, Line) {
    let output = segment.output;
    let last = if output.column.0 == 0 {
        output.line - 1
    } else {
        output.line
    };
    (segment.prompt, last.max(segment.prompt))
}

/// How a jumped-to prompt is lit: held a moment, then faded within half a second; with the
/// system's Reduce Motion (`still`), held as long and then gone, without fading.
pub fn flash(still: bool) -> Fade {
    if still {
        Fade::new(Duration::from_millis(500), Duration::ZERO)
    } else {
        Fade::new(Duration::from_millis(100), Duration::from_millis(400))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Screen, Size};
    use alacritty_terminal::{
        grid::Dimensions,
        index::{Column, Point},
    };
    use std::time::Instant;

    /// A finished command: prompt on `prompt`, output from the next line to just before `end`.
    fn command(id: i64, prompt: i32, end: i32, exit: i32) -> Segment {
        Segment {
            id,
            prompt: Line(prompt),
            output: Point::new(Line(prompt + 1), Column(0)),
            end: Some(Point::new(Line(end), Column(0))),
            exit: Some(exit),
        }
    }

    fn prompts(segments: &[Segment]) -> Vec<i32> {
        segments.iter().map(|s| s.prompt.0).collect()
    }

    fn commands() -> Vec<Segment> {
        vec![
            command(1, -30, -20, 0),
            command(2, -20, -5, 1),
            command(3, -5, 2, 0),
            command(4, 2, 8, 0),
            command(5, 8, 9, 0),
        ]
    }

    #[test]
    fn the_command_under_a_line_is_found_from_its_prompt_to_its_last_line() {
        let segments = commands();
        let id = |line: i32| at(&segments, Line(line)).map(|s| s.id);
        assert_eq!(id(-31), None);
        assert_eq!(id(-30), Some(1));
        assert_eq!(id(-21), Some(1));
        assert_eq!(id(-20), Some(2));
        assert_eq!(id(8), Some(5));
        assert_eq!(id(9), None, "past the last command's output");
        // Output that ends mid-line shares that line with the next prompt: the later command.
        let shared = [
            Segment {
                end: Some(Point::new(Line(3), Column(4))),
                ..command(1, 0, 3, 0)
            },
            command(2, 3, 5, 0),
        ];
        assert_eq!(at(&shared, Line(3)).map(|s| s.id), Some(2));
        assert_eq!(at(&shared, Line(2)).map(|s| s.id), Some(1));
    }

    #[test]
    fn jumps_go_command_by_command_and_stop_at_either_end() {
        let segments = commands();
        let to =
            |from: Option<i32>, up: bool| target(&segments, from.map(Line), up).map(|s| s.prompt.0);
        // From the bottom, ⌘↑ goes to the latest command; ⌘↓ has nowhere to go.
        assert_eq!(to(None, true), Some(8));
        assert_eq!(to(None, false), None);
        // From a prompt, to the one before or after it.
        assert_eq!(to(Some(8), true), Some(2));
        assert_eq!(to(Some(-20), false), Some(-5));
        assert_eq!(to(Some(-30), true), None, "the first command");
        assert_eq!(to(Some(8), false), None, "the last command");
        // From the middle of a command's output: up to its own prompt, down to the next.
        assert_eq!(to(Some(-12), true), Some(-20));
        assert_eq!(to(Some(-12), false), Some(-5));
        assert_eq!(target(&[], None, true), None, "no commands, nothing moves");
        assert_eq!(prompts(&segments), [-30, -20, -5, 2, 8]);
    }

    #[test]
    fn a_jump_starts_from_the_last_while_the_view_keeps_it_else_from_the_top() {
        let segments = commands();
        // Jumped to the prompt on line 2, which could not go to the top (view at the bottom).
        assert_eq!(from(&segments, Some((4, 2)), 0), Some(Line(2)));
        // Jumped to the one on line -20 and put at the top, 20 lines back: still there.
        assert_eq!(from(&segments, Some((2, 0)), 20), Some(Line(-20)));
        // Scrolled since: from the view's top line, or from the bottom when back there.
        assert_eq!(from(&segments, Some((2, 0)), 25), Some(Line(-25)));
        assert_eq!(from(&segments, Some((4, 2)), 0).map(|l| l.0), Some(2));
        assert_eq!(from(&segments, Some((4, 5)), 0), None);
        assert_eq!(from(&segments, None, 12), Some(Line(-12)));
        assert_eq!(from(&segments, None, 0), None);
        // The command gone (trimmed): from the view.
        assert_eq!(from(&segments, Some((9, 0)), 7), Some(Line(-7)));
    }

    #[test]
    fn a_jump_puts_the_prompt_at_the_top_as_far_as_the_history_goes() {
        assert_eq!(offset_for(Line(-20), 100), 20);
        assert_eq!(offset_for(Line(0), 100), 0);
        assert_eq!(
            offset_for(Line(5), 100),
            0,
            "on screen below the top: the bottom"
        );
        assert_eq!(offset_for(Line(-200), 100), 100, "the oldest history kept");
    }

    #[test]
    fn a_jump_lights_the_prompt_and_command_lines_only() {
        let segment = command(1, -4, 3, 0);
        assert_eq!(command_lines(&segment), (Line(-4), Line(-4)));
        // A command line wrapped over two rows, output from the next.
        let wrapped = Segment {
            output: Point::new(Line(-2), Column(0)),
            ..segment
        };
        assert_eq!(command_lines(&wrapped), (Line(-4), Line(-3)));
        // Output starting on the command's own row.
        let same = Segment {
            output: Point::new(Line(-4), Column(9)),
            ..segment
        };
        assert_eq!(command_lines(&same), (Line(-4), Line(-4)));
    }

    #[test]
    fn the_light_fades_within_half_a_second_or_with_reduce_motion_just_goes() {
        let start = Instant::now();
        let ms = |n| start + Duration::from_millis(n);
        let mut moving = flash(false);
        moving.poke(start);
        assert_eq!(moving.alpha(ms(50)), 1.0);
        let fading = moving.alpha(ms(300));
        assert!(fading > 0.0 && fading < 1.0, "{fading}");
        assert_eq!(moving.alpha(ms(500)), 0.0);
        let mut still = flash(true);
        still.poke(start);
        assert_eq!(still.alpha(ms(50)), 1.0);
        assert_eq!(still.alpha(ms(450)), 1.0, "no fade");
        assert_eq!(still.alpha(ms(500)), 0.0);
    }

    /// A shell's screen with commands run: their output selected by a click on the bar.
    #[test]
    fn the_bar_selects_the_output_without_the_command_line() {
        const A: &str = "\x1b]133;A\x07";
        const C: &str = "\x1b]133;C\x07";
        let end = |code: i32| format!("\x1b]133;D;{code}\x07");
        let mut screen = Screen::new(Size { rows: 12, cols: 10 });
        screen.keep_segments();
        screen.process(format!("{A}$ true\r\n{C}{}", end(0)).as_bytes());
        screen.process(format!("{A}$ ls\r\n{C}a\r\nbbbbbbbbbbccc\r\n{}", end(1)).as_bytes());
        screen.process(format!("{A}$ printf x\r\n{C}x{}", end(0)).as_bytes());
        screen.process(format!("{A}$ sleep 9\r\n{C}zz").as_bytes());
        let segments = screen.segments();
        assert_eq!(segments.len(), 4);
        let selected = |screen: &mut Screen, segment: &Segment| {
            let selection = output_selection(segment, screen.term.columns())?;
            screen.term.selection = Some(selection);
            screen.term.selection_to_string()
        };
        assert_eq!(selected(&mut screen, &segments[0]), None, "printed nothing");
        // Its lines, the wrapped one whole, and not `$ ls`.
        assert_eq!(
            selected(&mut screen, &segments[1]).as_deref(),
            Some("a\nbbbbbbbbbbccc")
        );
        // Output that ends mid-line, the next prompt after it.
        assert_eq!(selected(&mut screen, &segments[2]).as_deref(), Some("x"));
        assert_eq!(selected(&mut screen, &segments[3]), None, "still running");
    }
}
