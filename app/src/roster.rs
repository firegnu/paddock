//! The other machines in the window (DESIGN §13 P5-90, P5-123): the ones seated, in the config's
//! order, each with its connection; and the saved panes of the ones not seated, hidden or no longer
//! in the config, kept for when they come back. A hidden machine is as if not configured: no
//! connection, no switcher tab, no Inbox rows. Machines are known by their `name`; the label is
//! only what people see.
use crate::{layout_state::Board, remote::Link};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Default)]
pub struct Roster {
    /// Seat `n` (after this Mac's, 0) is `links[n - 1]`.
    links: Vec<Arc<Link>>,
    /// Saved boards of the machines not seated, by name.
    boards: BTreeMap<String, Board>,
}

impl Roster {
    /// None seated yet, and every machine's board as saved.
    pub fn new(boards: BTreeMap<String, Board>) -> Self {
        Self {
            links: Vec::new(),
            boards,
        }
    }

    pub fn links(&self) -> &[Arc<Link>] {
        &self.links
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// The machine at `seat`; none at this Mac's, 0.
    pub fn link(&self, seat: usize) -> Option<&Arc<Link>> {
        seat.checked_sub(1).and_then(|index| self.links.get(index))
    }

    /// Machine `name`'s seat, while it is seated.
    pub fn seat_of(&self, name: &str) -> Option<usize> {
        self.links
            .iter()
            .position(|link| link.name == name)
            .map(|index| index + 1)
    }

    /// The name people see for machine `name`: its label while seated, else its name.
    pub fn label(&self, name: &str) -> String {
        self.links
            .iter()
            .find(|link| link.name == name)
            .map_or_else(|| name.to_owned(), |link| link.label())
    }

    /// Seats `link`'s machine, at its place among those seated as `order` (every configured
    /// machine's name, in the config's order) has them: the seat it takes, and its saved board,
    /// taken back, when there is one that keeps a layout's rules.
    pub fn seat(&mut self, link: Arc<Link>, order: &[&str]) -> (usize, Option<Board>) {
        let rank = |name: &str| order.iter().position(|n| *n == name).unwrap_or(usize::MAX);
        let mine = rank(&link.name);
        let index = self
            .links
            .iter()
            .take_while(|seated| rank(&seated.name) <= mine)
            .count();
        let board = self
            .boards
            .remove(&link.name)
            .filter(|board| board.validate().is_ok());
        self.links.insert(index, link);
        (index + 1, board)
    }

    /// Hides machine `name`: it is not connected or read any more, and its `board` is kept for
    /// when it is shown again.
    pub fn hide(&mut self, name: &str, board: Board) {
        self.unseat(name);
        self.boards.insert(name.to_owned(), board);
    }

    /// Removes machine `name`, seated or hidden: not connected or read any more, and its saved
    /// board forgotten.
    pub fn remove(&mut self, name: &str) {
        self.unseat(name);
        self.boards.remove(name);
    }

    fn unseat(&mut self, name: &str) {
        if let Some(index) = self.links.iter().position(|link| link.name == name) {
            self.links.remove(index).retire();
        }
    }

    /// The boards of the machines not seated, for the saved layout.
    pub fn boards(&self) -> &BTreeMap<String, Board> {
        &self.boards
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        layout_state::{Content, SavedNode, SavedTab},
    };

    /// A board of one tab whose pane shows `agent`, its folds empty.
    fn board(agent: &str) -> Board {
        Board {
            active_tab: 0,
            tabs: vec![SavedTab {
                tree: SavedNode::Pane { pane: 0 },
                active: 0,
                panes: vec![Content::Agent {
                    name: agent.into(),
                    cwd: None,
                    instance: None,
                }],
            }],
            folded: Default::default(),
        }
    }

    fn agent_of(board: &Board) -> String {
        format!("{:?}", board.tabs[0].panes[0])
    }

    const CONFIG: &str = "[[machines]]\nname = \"a\"\nssh = \"a\"\n\
        [[machines]]\nname = \"b\"\nssh = \"b\"\nshow = false\n\
        [[machines]]\nname = \"c\"\nssh = \"c\"\nlabel = \"Cloud box\"\n";

    /// The roster as the window starts it: every saved board, then the shown machines seated.
    fn started(config: &Config) -> (Roster, Vec<Option<Board>>) {
        let saved = BTreeMap::from(
            ["a", "b", "c", "gone"].map(|name| (name.to_owned(), board(&format!("{name}/pi")))),
        );
        let mut roster = Roster::new(saved);
        let order: Vec<&str> = config.machines.iter().map(|m| m.name.as_str()).collect();
        let boards = config
            .machines
            .iter()
            .filter(|m| m.show)
            .map(|m| roster.seat(link(m), &order).1)
            .collect();
        (roster, boards)
    }

    fn link(machine: &crate::config::Machine) -> Arc<Link> {
        Arc::new(Link::with(machine, "false".into(), Err("no socket".into())))
    }

    /// One switcher tab per seat after This Mac's, and one Inbox source per seat: by name.
    fn seated(roster: &Roster) -> Vec<&str> {
        roster
            .links()
            .iter()
            .map(|link| link.name.as_str())
            .collect()
    }

    #[test]
    fn a_hidden_machine_is_not_seated_and_its_saved_panes_wait() {
        let config = Config::parse(CONFIG).unwrap();
        let (roster, boards) = started(&config);
        assert_eq!(seated(&roster), ["a", "c"]);
        assert_eq!(roster.seat_of("b"), None);
        assert_eq!(
            (roster.seat_of("a"), roster.seat_of("c")),
            (Some(1), Some(2))
        );
        // The seated ones took their boards back; the hidden one's, and one no longer in the
        // config, wait.
        let agents: Vec<String> = boards
            .iter()
            .map(|b| agent_of(b.as_ref().unwrap()))
            .collect();
        assert!(
            agents[0].contains("a/pi") && agents[1].contains("c/pi"),
            "{agents:?}"
        );
        let kept: Vec<&str> = roster.boards().keys().map(String::as_str).collect();
        assert_eq!(kept, ["b", "gone"]);
    }

    #[test]
    fn hidden_then_shown_again_a_machine_comes_back_in_its_place_with_its_panes() {
        let config = Config::parse(CONFIG).unwrap();
        let order: Vec<&str> = config.machines.iter().map(|m| m.name.as_str()).collect();
        let (mut roster, _) = started(&config);
        let c = roster.link(2).unwrap().clone();
        // Hidden with its panes as they are now: no seat, not run there any more.
        roster.hide("c", board("c/build"));
        assert_eq!(seated(&roster), ["a"]);
        assert!(c.retired());
        assert!(roster.boards().contains_key("c"));
        // b, hidden from the start, shown: between a and c as the config has them.
        let (seat, panes) = roster.seat(link(&config.machines[1]), &order);
        assert_eq!((seat, seated(&roster)), (2, vec!["a", "b"]));
        assert!(agent_of(&panes.unwrap()).contains("b/pi"));
        // c shown again: after b, its panes as they were when it was hidden.
        let (seat, panes) = roster.seat(link(&config.machines[2]), &order);
        assert_eq!((seat, seated(&roster)), (3, vec!["a", "b", "c"]));
        assert!(agent_of(&panes.unwrap()).contains("c/build"));
        assert_eq!(roster.boards().keys().collect::<Vec<_>>(), ["gone"]);
        // And a first one before them all.
        roster.hide("a", board("a/pi"));
        let (seat, _) = roster.seat(link(&config.machines[0]), &order);
        assert_eq!((seat, seated(&roster)), (1, vec!["a", "b", "c"]));
    }

    #[test]
    fn a_removed_machine_is_gone_with_its_saved_panes() {
        let config = Config::parse(CONFIG).unwrap();
        let (mut roster, _) = started(&config);
        let a = roster.link(1).unwrap().clone();
        roster.remove("a");
        assert_eq!(seated(&roster), ["c"]);
        assert!(a.retired());
        assert!(!roster.boards().contains_key("a"));
        // A hidden one removed: its waiting board goes too.
        roster.remove("b");
        assert_eq!(roster.boards().keys().collect::<Vec<_>>(), ["gone"]);
        assert_eq!(seated(&roster), ["c"]);
        assert_eq!(roster.link(0).map(|l| l.name.clone()), None);
    }

    #[test]
    fn people_see_the_label_and_the_machine_is_known_by_its_name() {
        let config = Config::parse(CONFIG).unwrap();
        let (mut roster, _) = started(&config);
        assert_eq!(roster.label("c"), "Cloud box");
        assert_eq!(roster.label("a"), "a");
        // Not seated: its name.
        assert_eq!(roster.label("b"), "b");
        // Renamed while in use: seen at once, still found by its name.
        roster.link(2).unwrap().relabel("GPU");
        assert_eq!(roster.label("c"), "GPU");
        assert_eq!(roster.seat_of("c"), Some(2));
        assert_eq!(roster.seat_of("GPU"), None);
        // Its panes are kept by its name.
        roster.hide("c", board("c/pi"));
        assert!(roster.boards().contains_key("c") && !roster.boards().contains_key("GPU"));
    }
}
