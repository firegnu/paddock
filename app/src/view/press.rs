//! What the left mouse button does in a terminal pane (P5-143b): select, go to the program that
//! asked for the mouse, or with ⌘ on a link, open it. Apart from GPUI, so that whole click
//! sequences can be tested. The pane hands it what happens over the pane (`down`, `pane_move`,
//! `pane_up`) and what happens anywhere in the window (`window_move`, `window_up`, `cancel`); a
//! press on a link is followed through the window, the others through the pane, as before.
use super::links::{Link, Target};
use crate::input::{MouseButton as B, MouseEventKind as K};
use alacritty_terminal::{
    index::{Point, Side},
    selection::SelectionType,
};

/// A grid cell and which half of it the mouse is on.
pub type Cell = (Point, Side);

/// What the pane does about a mouse event.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Nothing,
    /// Tell the program, which asked for the mouse.
    Report(K),
    /// A new selection of a kind, from a cell.
    Select(SelectionType, Cell),
    /// The selection now ends at a cell.
    Extend(Cell),
    /// A selection from where a link was pressed to where the mouse is now.
    SelectFrom(Cell, Cell),
    /// The press is over: an empty selection goes.
    EndSelect,
    Open(Target),
}

/// A left press over the pane.
pub struct Down {
    /// With ⌘ held, the link under the mouse.
    pub link: Option<Link>,
    pub command: bool,
    pub clicks: usize,
    pub shift: bool,
    /// The program asked for the mouse and may have it (its agent is not paused).
    pub reports: bool,
    /// The cell pressed; none before the pane has a grid.
    pub cell: Option<Cell>,
}

#[derive(Clone, Debug, PartialEq)]
enum Press {
    Select,
    Reported,
    /// With ⌘ on `link`, at `at`: opened when let go there without having left it.
    Link {
        link: Link,
        at: Cell,
    },
    /// A press on a link that left its cell: a selection, followed through the window.
    LinkDrag,
    /// A later click of a ⌘ click on a link: nothing at all.
    Swallowed,
}

#[derive(Default)]
pub struct Mouse {
    pressed: Option<Press>,
    /// The clicks going on started with ⌘ on a link.
    on_link: bool,
}

impl Mouse {
    /// A press is going on.
    pub fn pressed(&self) -> bool {
        self.pressed.is_some()
    }

    pub fn down(&mut self, down: Down) -> Act {
        if down.clicks <= 1 {
            self.on_link = false;
        }
        let Some(cell) = down.cell else {
            return Act::Nothing;
        };
        // The second and third click of a ⌘ click on a link, which opened it once already.
        if down.clicks > 1 && self.on_link && down.command {
            self.pressed = Some(Press::Swallowed);
            return Act::Nothing;
        }
        if let Some(link) = down.link.filter(|_| down.clicks == 1) {
            self.on_link = true;
            self.pressed = Some(Press::Link { link, at: cell });
            return Act::Nothing;
        }
        if !down.shift && down.reports {
            self.pressed = Some(Press::Reported);
            return Act::Report(K::Down(B::Left));
        }
        self.pressed = Some(Press::Select);
        Act::Select(selection_kind(down.clicks), cell)
    }

    /// The mouse moved over the pane, `left` held or not.
    pub fn pane_move(&mut self, cell: Option<Cell>, left: bool) -> Act {
        match (&self.pressed, cell, left) {
            (Some(Press::Reported), _, true) => Act::Report(K::Drag(B::Left)),
            (Some(Press::Select), Some(cell), true) => Act::Extend(cell),
            _ => Act::Nothing,
        }
    }

    /// Let go over the pane.
    pub fn pane_up(&mut self) -> Act {
        match self.pressed {
            Some(Press::Reported) => {
                self.pressed = None;
                Act::Report(K::Up(B::Left))
            }
            Some(Press::Select) => {
                self.pressed = None;
                Act::EndSelect
            }
            _ => Act::Nothing,
        }
    }

    /// The mouse moved anywhere in the window; `cell`: the nearest the pane has, `inside`: over
    /// its grid.
    pub fn window_move(&mut self, cell: Option<Cell>, inside: bool, left: bool) -> Act {
        if !left {
            // Let go where the window was not told.
            return self.window_up(cell, false);
        }
        match (&self.pressed, cell) {
            (Some(Press::Link { at, .. }), Some(cell)) if !inside || cell.0 != at.0 => {
                let at = *at;
                self.pressed = Some(Press::LinkDrag);
                self.on_link = false;
                Act::SelectFrom(at, cell)
            }
            (Some(Press::LinkDrag), Some(cell)) => Act::Extend(cell),
            _ => Act::Nothing,
        }
    }

    /// Let go anywhere in the window: a link opens only where it was pressed, never having left.
    pub fn window_up(&mut self, cell: Option<Cell>, inside: bool) -> Act {
        match &self.pressed {
            Some(Press::Link { at, .. }) => {
                let here = inside && cell.is_some_and(|cell| cell.0 == at.0);
                let Some(Press::Link { link, .. }) = self.pressed.take() else {
                    unreachable!()
                };
                if here {
                    Act::Open(link.target)
                } else {
                    Act::Nothing
                }
            }
            Some(Press::LinkDrag) => {
                self.pressed = None;
                Act::EndSelect
            }
            Some(Press::Swallowed) => {
                self.pressed = None;
                Act::Nothing
            }
            _ => Act::Nothing,
        }
    }

    /// The window lost the keyboard: a press on a link ends, as if let go out of the pane.
    pub fn cancel(&mut self) -> Act {
        self.on_link = false;
        self.window_up(None, false)
    }
}

/// A double click selects a word, a triple click a line.
fn selection_kind(clicks: usize) -> SelectionType {
    match clicks {
        2 => SelectionType::Semantic,
        3.. => SelectionType::Lines,
        _ => SelectionType::Simple,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line};

    fn cell(col: usize) -> Cell {
        (Point::new(Line(0), Column(col)), Side::Left)
    }

    fn link() -> Link {
        Link {
            start: cell(0).0,
            end: cell(9).0,
            target: Target::Url("https://example.com".into()),
            uri: None,
        }
    }

    /// Everything the pane did.
    #[derive(Default)]
    struct Done(Vec<Act>);

    impl Done {
        fn opened(&self) -> usize {
            self.0.iter().filter(|a| matches!(a, Act::Open(_))).count()
        }
        fn selected(&self) -> Vec<&Act> {
            self.0
                .iter()
                .filter(|a| matches!(a, Act::Select(..) | Act::Extend(_) | Act::SelectFrom(..)))
                .collect()
        }
        fn reported(&self) -> Vec<&Act> {
            self.0
                .iter()
                .filter(|a| matches!(a, Act::Report(_)))
                .collect()
        }
    }

    /// One click at column 3 as GPUI hands it over: the press over the pane, then letting go
    /// there, which both the pane and the window hear (`window_first`: the window first).
    fn click(
        mouse: &mut Mouse,
        done: &mut Done,
        clicks: usize,
        command: bool,
        reports: bool,
        window_first: bool,
    ) {
        done.0.push(mouse.down(Down {
            link: command.then(link),
            command,
            clicks,
            shift: false,
            reports,
            cell: Some(cell(3)),
        }));
        if window_first {
            done.0.push(mouse.window_up(Some(cell(3)), true));
            done.0.push(mouse.pane_up());
        } else {
            done.0.push(mouse.pane_up());
            done.0.push(mouse.window_up(Some(cell(3)), true));
        }
    }

    /// The mouse over the pane, which both hear, or off it, which only the window hears.
    fn moved(mouse: &mut Mouse, done: &mut Done, col: usize, inside: bool) {
        if inside {
            done.0.push(mouse.pane_move(Some(cell(col)), true));
        }
        done.0
            .push(mouse.window_move(Some(cell(col)), inside, true));
    }

    #[test]
    fn command_clicks_on_a_link_open_it_once_and_nothing_else() {
        for reports in [false, true] {
            for window_first in [false, true] {
                for clicks in 1..=3 {
                    let (mut mouse, mut done) = (Mouse::default(), Done::default());
                    for count in 1..=clicks {
                        click(&mut mouse, &mut done, count, true, reports, window_first);
                    }
                    let case = format!("{clicks} clicks, reports {reports}");
                    assert_eq!(done.opened(), 1, "{case}");
                    assert!(done.selected().is_empty(), "{case}: {:?}", done.0);
                    assert!(done.reported().is_empty(), "{case}: {:?}", done.0);
                    assert!(!mouse.pressed(), "{case}");
                }
            }
        }
    }

    #[test]
    fn without_command_clicks_select_or_go_to_the_program_as_before() {
        let kinds = [
            SelectionType::Simple,
            SelectionType::Semantic,
            SelectionType::Lines,
        ];
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        for count in 1..=3 {
            click(&mut mouse, &mut done, count, false, false, false);
        }
        let expected: Vec<Act> = kinds
            .iter()
            .flat_map(|kind| [Act::Select(*kind, cell(3)), Act::EndSelect, Act::Nothing])
            .collect();
        assert_eq!(done.0, expected);
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        for count in 1..=3 {
            click(&mut mouse, &mut done, count, false, true, false);
        }
        let expected: Vec<Act> = (0..3)
            .flat_map(|_| {
                [
                    Act::Report(K::Down(B::Left)),
                    Act::Report(K::Up(B::Left)),
                    Act::Nothing,
                ]
            })
            .collect();
        assert_eq!(done.0, expected);
        // Shift selects even then; dragging extends the selection.
        let mut mouse = Mouse::default();
        let down = Down {
            link: None,
            command: false,
            clicks: 1,
            shift: true,
            reports: true,
            cell: Some(cell(3)),
        };
        assert_eq!(
            mouse.down(down),
            Act::Select(SelectionType::Simple, cell(3))
        );
        assert_eq!(mouse.pane_move(Some(cell(5)), true), Act::Extend(cell(5)));
        assert_eq!(mouse.window_move(Some(cell(5)), true, true), Act::Nothing);
        assert_eq!(mouse.pane_up(), Act::EndSelect);
    }

    fn press_link(mouse: &mut Mouse, done: &mut Done) {
        done.0.push(mouse.down(Down {
            link: Some(link()),
            command: true,
            clicks: 1,
            shift: false,
            reports: true,
            cell: Some(cell(3)),
        }));
    }

    fn let_go(mouse: &mut Mouse, done: &mut Done, col: usize, inside: bool) {
        if inside {
            done.0.push(mouse.pane_up());
        }
        done.0.push(mouse.window_up(Some(cell(col)), inside));
    }

    #[test]
    fn a_link_press_that_leaves_its_cell_or_the_pane_is_a_selection_and_does_not_open() {
        // Out over the pane's edge and back to the cell it was pressed on.
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        press_link(&mut mouse, &mut done);
        moved(&mut mouse, &mut done, 0, false);
        moved(&mut mouse, &mut done, 3, true);
        let_go(&mut mouse, &mut done, 3, true);
        assert_eq!(done.opened(), 0, "{:?}", done.0);
        assert_eq!(done.selected()[0], &Act::SelectFrom(cell(3), cell(0)));
        assert!(!mouse.pressed());
        // Let go out of the pane, then back in.
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        press_link(&mut mouse, &mut done);
        moved(&mut mouse, &mut done, 9, false);
        let_go(&mut mouse, &mut done, 9, false);
        assert!(!mouse.pressed(), "{:?}", done.0);
        assert_eq!(mouse.pane_move(Some(cell(3)), false), Act::Nothing);
        assert_eq!(done.opened(), 0);
        assert_eq!(done.0.last(), Some(&Act::EndSelect));
        // To another cell and back.
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        press_link(&mut mouse, &mut done);
        moved(&mut mouse, &mut done, 5, true);
        moved(&mut mouse, &mut done, 3, true);
        let_go(&mut mouse, &mut done, 3, true);
        assert_eq!(done.opened(), 0, "{:?}", done.0);
        assert_eq!(done.selected()[0], &Act::SelectFrom(cell(3), cell(5)));
        assert!(done.reported().is_empty());
        // Moving within its cell and letting go there opens it.
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        press_link(&mut mouse, &mut done);
        moved(&mut mouse, &mut done, 3, true);
        let_go(&mut mouse, &mut done, 3, true);
        assert_eq!(done.opened(), 1, "{:?}", done.0);
    }

    #[test]
    fn a_window_that_loses_the_keyboard_ends_a_link_press() {
        let (mut mouse, mut done) = (Mouse::default(), Done::default());
        press_link(&mut mouse, &mut done);
        done.0.push(mouse.cancel());
        let_go(&mut mouse, &mut done, 3, true);
        assert_eq!(done.opened(), 0, "{:?}", done.0);
        assert!(!mouse.pressed());
    }
}
