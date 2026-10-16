//! Which machine's sidebar, layout and panes are in the window's own fields, and where each other
//! machine's wait meanwhile, still connected (DESIGN §13 P5-90). Seat 0 is this Mac.

pub struct Seating<T> {
    /// The seat in the window's fields.
    here: usize,
    /// Every seat's things while another is in the fields; `None` at `here`.
    parked: Vec<Option<T>>,
}

impl<T> Default for Seating<T> {
    /// This Mac alone, in the fields.
    fn default() -> Self {
        Seating {
            here: 0,
            parked: vec![None],
        }
    }
}

impl<T> Seating<T> {
    /// Another machine's seat, waiting; its place.
    pub fn push(&mut self, seat: T) -> usize {
        self.parked.push(Some(seat));
        self.parked.len() - 1
    }

    pub fn here(&self) -> usize {
        self.here
    }

    /// Seats in all, this Mac's among them.
    pub fn len(&self) -> usize {
        self.parked.len()
    }

    /// Never: this Mac's seat is always there.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Seat `seat`'s things while they wait; `None` for the one in the fields.
    pub fn parked(&self, seat: usize) -> Option<&T> {
        self.parked[seat].as_ref()
    }

    pub fn parked_mut(&mut self, seat: usize) -> Option<&mut T> {
        self.parked[seat].as_mut()
    }

    /// Puts seat `seat` in the fields: `swap` exchanges the fields with what waited there, and what
    /// was in them waits in its own place.
    pub fn bring(&mut self, seat: usize, swap: impl FnOnce(&mut T)) {
        if seat == self.here || seat >= self.parked.len() {
            return;
        }
        let mut waiting = self.parked[seat]
            .take()
            .expect("a seat not in the fields waits");
        swap(&mut waiting);
        self.parked[self.here] = Some(waiting);
        self.here = seat;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_ui::{Activity, busy_reason};

    /// What the window holds in its fields, as a name for the test.
    fn bring(seating: &mut Seating<&'static str>, fields: &mut &'static str, seat: usize) {
        seating.bring(seat, |waiting| std::mem::swap(fields, waiting));
    }

    /// `paddock ctl` while another machine is on show: it acts on this Mac's seat, brought into
    /// the fields and put back after, and nothing about it is busy; only what the user is doing
    /// in the window makes it wait, as before (P5-90 rework).
    #[test]
    fn this_macs_ctl_works_on_its_own_seat_while_another_machine_is_on_show() {
        let mut seating = Seating::default();
        let mut fields = "this Mac";
        let studio = seating.push("studio");
        bring(&mut seating, &mut fields, studio);
        assert_eq!((seating.here(), fields), (studio, "studio"));
        assert_eq!(seating.parked(0), Some(&"this Mac"));

        // As `on_local` does around a request.
        let away = seating.here();
        bring(&mut seating, &mut fields, 0);
        assert_eq!((seating.here(), fields), (0, "this Mac"));
        assert_eq!(seating.parked(studio), Some(&"studio"));
        let idle = Activity::default();
        assert_eq!(busy_reason(&idle), None);
        // The user's own moment still waits it, whichever machine is on show.
        let menu = Activity {
            popup: true,
            ..idle
        };
        assert_eq!(busy_reason(&menu), Some("using a panel or menu"));
        bring(&mut seating, &mut fields, away);
        assert_eq!((seating.here(), fields), (studio, "studio"));
        assert_eq!(seating.parked(0), Some(&"this Mac"));
        assert_eq!(seating.parked(studio), None);
    }

    #[test]
    fn bringing_the_seat_in_the_fields_or_none_changes_nothing() {
        let mut seating = Seating::default();
        let mut fields = "this Mac";
        seating.push("studio");
        bring(&mut seating, &mut fields, 0);
        bring(&mut seating, &mut fields, 5);
        assert_eq!((seating.here(), fields), (0, "this Mac"));
        assert_eq!(seating.len(), 2);
    }
}
