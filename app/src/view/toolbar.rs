//! The small bar that fades in beside a selection made with the mouse, where the mouse let go
//! (P5-148c): `Copy` and `Send to agent…`. It goes once the selection does (a click elsewhere,
//! typing, output that clears it, Esc), or the pane loses the keyboard; a press told to the program
//! that asked for the mouse never brings it. Apart from GPUI, so that this can be tested; what its
//! buttons do is the pane's.
use super::tween::{self, Tween};
use std::time::Instant;

/// Room between the bar and the selected rows.
pub const GAP: f32 = 4.0;

/// Where the mouse let go: how far from the grid's left, and whether that was the selection's
/// bottom end (dragged down), so that the bar goes under it, or its top end, over it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct At {
    pub x: f32,
    pub below: bool,
}

/// The bar of one pane.
#[derive(Default)]
pub struct Toolbar {
    at: Option<At>,
    /// How far it has faded in, 0 to 1.
    shown: Option<Tween>,
}

impl Toolbar {
    /// A press was let go. `ended`: it ended a selection made with the mouse (not a press told to
    /// the program, nor a link opened); `selected`: a selection is left, not empty.
    pub fn let_go(&mut self, ended: bool, selected: bool, at: At, now: Instant) {
        if !ended {
            return;
        }
        if !selected {
            return self.hide();
        }
        let mut shown = Tween::at(0.0);
        shown.go(1.0, now, tween::POP_IN, tween::POP_CURVE);
        (self.at, self.shown) = (Some(at), Some(shown));
    }

    /// Gone at once: a new press, typing, Esc, one of its buttons used.
    pub fn hide(&mut self) {
        (self.at, self.shown) = (None, None);
    }

    /// Whether it stays, drawn this frame: not once the selection has gone or the pane has lost
    /// the keyboard.
    pub fn keep(&mut self, selected: bool, focused: bool) -> Option<At> {
        if !(selected && focused) {
            self.hide();
        }
        self.at
    }

    /// How far it has faded in at `now`, 0 to 1, and whether it is still on its way.
    pub fn look(&self, now: Instant) -> (f32, bool) {
        match &self.shown {
            Some(shown) => (shown.value(now), shown.moving(now)),
            None => (0.0, false),
        }
    }
}

/// Where the bar goes, from the grid's top left: `bar` big, beside where the mouse let go (`at`),
/// under the selection's last row or over its first (`rows`, as drawn in view; either may be out
/// of it) so that it covers neither, and inside the `grid`. Where neither side has room, inside the
/// grid's bottom. Nothing while no selected row is in view.
pub fn place(
    at: At,
    rows: (i32, i32),
    line_height: f32,
    bar: (f32, f32),
    grid: (f32, f32),
) -> Option<(f32, f32)> {
    let ((first, last), (width, height)) = (rows, bar);
    let in_view = (grid.1 / line_height).floor() as i32;
    if last < 0 || first >= in_view {
        return None;
    }
    let x = at.x.min(grid.0 - width).max(0.0);
    let under = (last + 1) as f32 * line_height + GAP;
    let over = first as f32 * line_height - GAP - height;
    let (under, over) = (
        (under + height <= grid.1).then_some(under),
        (over >= 0.0).then_some(over),
    );
    let y = if at.below {
        under.or(over)
    } else {
        over.or(under)
    };
    Some((x, y.unwrap_or((grid.1 - height).max(0.0))))
}

#[cfg(test)]
mod tests {
    use super::super::press::{Act, Down, Mouse};
    use super::*;
    use alacritty_terminal::index::{Column, Line, Point, Side};
    use std::time::Duration;

    const AT: At = At {
        x: 120.0,
        below: true,
    };

    fn down(mouse: &mut Mouse, clicks: usize, reports: bool, shift: bool) -> Act {
        mouse.down(Down {
            link: None,
            command: false,
            clicks,
            shift,
            reports,
            cell: Some((Point::new(Line(0), Column(3)), Side::Left)),
        })
    }

    /// A press let go over the pane, as the pane hands it to the bar: the release ends a
    /// selection only when [`Mouse`] says so; `selected`, whether one is left.
    fn release(mouse: &mut Mouse, bar: &mut Toolbar, selected: bool, now: Instant) {
        let ended = mouse.pane_up() == Act::EndSelect;
        bar.let_go(ended, selected, AT, now);
        // The window hears the same letting go after the pane: nothing more.
        let ended = mouse.window_up(None, true) == Act::EndSelect;
        bar.let_go(ended, selected, AT, now);
    }

    #[test]
    fn a_selection_let_go_brings_the_bar_and_fades_it_in() {
        let now = Instant::now();
        // Dragged, double and triple clicked.
        for clicks in 1..=3 {
            let (mut mouse, mut bar) = (Mouse::default(), Toolbar::default());
            down(&mut mouse, clicks, false, false);
            release(&mut mouse, &mut bar, true, now);
            assert_eq!(bar.keep(true, true), Some(AT), "{clicks} clicks");
            assert_eq!(bar.look(now), (0.0, true));
            assert_eq!(bar.look(now + Duration::from_secs(1)), (1.0, false));
        }
        // Shift selects even where the program has the mouse: it comes then too.
        let (mut mouse, mut bar) = (Mouse::default(), Toolbar::default());
        down(&mut mouse, 1, true, true);
        release(&mut mouse, &mut bar, true, now);
        assert_eq!(bar.keep(true, true), Some(AT));
    }

    #[test]
    fn no_bar_for_a_plain_click_or_a_press_told_to_the_program() {
        let now = Instant::now();
        // A click leaves no selection.
        let (mut mouse, mut bar) = (Mouse::default(), Toolbar::default());
        down(&mut mouse, 1, false, false);
        release(&mut mouse, &mut bar, false, now);
        assert_eq!(bar.keep(false, true), None);
        // The program asked for the mouse: the press is its, whatever is selected from before.
        let (mut mouse, mut bar) = (Mouse::default(), Toolbar::default());
        assert!(matches!(down(&mut mouse, 1, true, false), Act::Report(_)));
        release(&mut mouse, &mut bar, true, now);
        assert_eq!(bar.keep(true, true), None);
    }

    #[test]
    fn the_bar_goes_with_the_selection_the_keyboard_or_when_told() {
        let now = Instant::now();
        let shown = || {
            let (mut mouse, mut bar) = (Mouse::default(), Toolbar::default());
            down(&mut mouse, 1, false, false);
            release(&mut mouse, &mut bar, true, now);
            (mouse, bar)
        };
        // Output, typing or a click cleared the selection.
        let (_, mut bar) = shown();
        assert_eq!(bar.keep(false, true), None);
        assert_eq!(bar.keep(true, true), None, "gone for good");
        // Another pane or a panel took the keyboard.
        let (_, mut bar) = shown();
        assert_eq!(bar.keep(true, false), None);
        assert_eq!(bar.keep(true, true), None);
        // Told: a new press, a key, Esc, a button used.
        let (mut mouse, mut bar) = shown();
        assert_eq!(bar.keep(true, true), Some(AT));
        bar.hide();
        assert_eq!(bar.keep(true, true), None);
        assert_eq!(bar.look(now), (0.0, false));
        // A new selection brings it again.
        down(&mut mouse, 1, false, false);
        release(&mut mouse, &mut bar, true, now);
        assert_eq!(bar.keep(true, true), Some(AT));
    }

    const LINE: f32 = 20.0;
    const BAR: (f32, f32) = (170.0, 28.0);
    const GRID: (f32, f32) = (800.0, 400.0);

    #[test]
    fn the_bar_goes_under_or_over_the_selection_never_on_it() {
        // Dragged down, rows 3 to 5: under row 5, at the mouse.
        assert_eq!(
            place(AT, (3, 5), LINE, BAR, GRID),
            Some((120.0, 6.0 * LINE + GAP))
        );
        // Dragged up: over row 3.
        let up = At {
            x: 120.0,
            below: false,
        };
        assert_eq!(
            place(up, (3, 5), LINE, BAR, GRID),
            Some((120.0, 3.0 * LINE - GAP - BAR.1))
        );
        // No room under the last rows: over them.
        assert_eq!(
            place(AT, (17, 19), LINE, BAR, GRID),
            Some((120.0, 17.0 * LINE - GAP - BAR.1))
        );
        // No room over the first: under.
        assert_eq!(
            place(up, (0, 2), LINE, BAR, GRID),
            Some((120.0, 3.0 * LINE + GAP))
        );
        // No room on either side: inside the grid's bottom, the least it can cover.
        assert_eq!(
            place(AT, (0, 19), LINE, BAR, GRID),
            Some((120.0, GRID.1 - BAR.1))
        );
        // Let go near the right edge or past the left: kept inside.
        let right = At {
            x: 790.0,
            below: true,
        };
        assert_eq!(
            place(right, (3, 5), LINE, BAR, GRID).map(|(x, _)| x),
            Some(GRID.0 - BAR.0)
        );
        let left = At {
            x: -30.0,
            below: true,
        };
        assert_eq!(
            place(left, (3, 5), LINE, BAR, GRID).map(|(x, _)| x),
            Some(0.0)
        );
        // A selection scrolled partly out of view: beside the part in it.
        assert_eq!(
            place(up, (-4, 2), LINE, BAR, GRID),
            Some((120.0, 3.0 * LINE + GAP))
        );
        // Out of view altogether: nowhere.
        assert_eq!(place(AT, (-9, -2), LINE, BAR, GRID), None);
        assert_eq!(place(AT, (20, 24), LINE, BAR, GRID), None);
    }
}
