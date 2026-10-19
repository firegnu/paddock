//! A few words over a pane's bottom, for a while: why a link did not open.

use std::time::{Duration, Instant};

/// How long a notice shows.
pub const NOTICE: Duration = Duration::from_secs(5);

pub struct Notice {
    text: String,
    /// When it was first drawn: its time runs from then, not while its pane is not shown.
    seen: Option<Instant>,
}

impl Notice {
    pub fn new(text: String) -> Self {
        Self { text, seen: None }
    }

    /// Whether it has been drawn yet.
    pub fn seen(&self) -> bool {
        self.seen.is_some()
    }

    /// The pane is drawn at `now`: the text while it still shows, `None` once it has had its time.
    pub fn drawn(&mut self, now: Instant) -> Option<&str> {
        let seen = *self.seen.get_or_insert(now);
        (now.saturating_duration_since(seen) < NOTICE).then_some(self.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notice_shows_for_its_time_from_when_it_is_first_seen() {
        let mut seen = Notice::new("Couldn't open a".into());
        let mut hidden = Notice::new("Couldn't open b".into());
        let arrived = Instant::now();
        let second = Duration::from_secs(1);
        // Seen at once: gone after its time.
        assert_eq!(seen.drawn(arrived), Some("Couldn't open a"));
        assert_eq!(seen.drawn(arrived + 4 * second), Some("Couldn't open a"));
        assert_eq!(seen.drawn(arrived + NOTICE), None);
        // Its pane hidden, another tab's or machine's, for longer than that: there when it shows
        // again, then for its time.
        let back = arrived + 60 * second;
        assert_eq!(hidden.drawn(back), Some("Couldn't open b"));
        assert_eq!(hidden.drawn(back + 4 * second), Some("Couldn't open b"));
        assert_eq!(hidden.drawn(back + NOTICE), None);
    }
}
