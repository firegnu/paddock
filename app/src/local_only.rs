//! What starts on This Mac alone while another machine is on show (DESIGN §13 P5-90, P5-122): a
//! new agent, a shell and a workspace. The sidebar's menu has them off, `This Mac` at their right;
//! asked for another way (a shortcut, the menu bar, the new tab panel), the sidebar says so in
//! neutral words with `Switch to This Mac`, which shows This Mac and then does what was asked. The
//! window does the switching and the doing (`window.rs`); the rules live here.

/// Seat 0, as the window's seats have it.
pub const THIS_MAC: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Only {
    NewAgent,
    NewShell,
    NewWorkspace,
}

impl Only {
    /// What the sidebar says while another machine is on show.
    pub fn hint(self) -> &'static str {
        match self {
            Only::NewAgent => "New agents start on This Mac.",
            Only::NewShell => "Shells open on This Mac.",
            Only::NewWorkspace => "Workspaces are on This Mac.",
        }
    }
}

/// What the window does when `only` is asked for with machine `seat` on show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// This Mac is on show: as ever.
    Go,
    /// Another machine is: the hint, and the offer to switch.
    Offer(Only),
}

pub fn ask(seat: usize, only: Only) -> Ask {
    if seat == THIS_MAC {
        Ask::Go
    } else {
        Ask::Offer(only)
    }
}

/// The offer taken, in order: This Mac on show, then `only` asked for again there, where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Show(usize),
    Then(Only),
}

pub fn take(only: Only) -> [Step; 2] {
    [Step::Show(THIS_MAC), Step::Then(only)]
}

/// One of them in the sidebar's menu with machine `seat` on show: whether it can be chosen, and
/// what its right says when it cannot.
pub fn menu_row(seat: usize) -> (bool, Option<&'static str>) {
    match seat {
        THIS_MAC => (true, None),
        _ => (false, Some("This Mac")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seating::Seating;

    const ALL: [Only; 3] = [Only::NewAgent, Only::NewShell, Only::NewWorkspace];

    #[test]
    fn on_this_mac_they_go_as_ever_and_the_menu_has_them_on() {
        for only in ALL {
            assert_eq!(ask(THIS_MAC, only), Ask::Go);
        }
        assert_eq!(menu_row(THIS_MAC), (true, None));
    }

    #[test]
    fn on_another_machine_the_menu_has_them_off_and_asking_offers_to_switch() {
        for only in ALL {
            assert_eq!(ask(1, only), Ask::Offer(only));
        }
        assert_eq!(menu_row(1), (false, Some("This Mac")));
        assert_eq!(menu_row(2), (false, Some("This Mac")));
        assert_eq!(Only::NewAgent.hint(), "New agents start on This Mac.");
    }

    /// `Switch to This Mac`: the window takes the steps as the switcher's choice and the asking do,
    /// and the thing asked for goes on This Mac, not offered again.
    #[test]
    fn taking_the_offer_shows_this_mac_then_does_what_was_asked_there() {
        for only in ALL {
            let mut seating: Seating<&str> = Seating::default();
            let remote = seating.push("studio");
            let mut fields = "this mac";
            seating.bring(remote, |waiting| std::mem::swap(&mut fields, waiting));
            seating.look();
            assert_eq!(ask(seating.here(), only), Ask::Offer(only));

            let mut done = Vec::new();
            for step in take(only) {
                match step {
                    Step::Show(seat) => {
                        seating.bring(seat, |waiting| std::mem::swap(&mut fields, waiting));
                        seating.look();
                    }
                    Step::Then(only) => done.push((ask(seating.here(), only), fields)),
                }
            }
            assert_eq!(done, [(Ask::Go, "this mac")], "{only:?}");
            assert!(seating.in_sight(THIS_MAC));
        }
    }
}
