//! Tabs and split panes, as a model apart from drawing. A tab holds a tree of panes and its active
//! pane; splits are relative to the active pane in four directions. The rules follow Saddle's
//! workspace (`docs/DESIGN.md` §27, §39 at commit `df1c727`), with one paddock rule: a running
//! shell is never replaced by an agent opened from the sidebar.
use std::collections::HashMap;

pub type PaneId = u64;

/// What a pane shows, as far as the layout rules care.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    Empty,
    Shell,
    Agent(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Side by side.
    Row,
    /// One above the other.
    Column,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Pane(PaneId),
    Split {
        axis: Axis,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    fn panes(&self, out: &mut Vec<PaneId>) {
        match self {
            Node::Pane(id) => out.push(*id),
            Node::Split { first, second, .. } => {
                first.panes(out);
                second.panes(out);
            }
        }
    }

    /// Replaces the pane `at` with a split holding it and `new` on the given side.
    fn split(&mut self, at: PaneId, new: PaneId, direction: Direction) -> bool {
        match self {
            Node::Pane(id) if *id == at => {
                let (axis, new_first) = match direction {
                    Direction::Left => (Axis::Row, true),
                    Direction::Right => (Axis::Row, false),
                    Direction::Up => (Axis::Column, true),
                    Direction::Down => (Axis::Column, false),
                };
                let (old, new) = (Box::new(Node::Pane(at)), Box::new(Node::Pane(new)));
                let (first, second) = if new_first { (new, old) } else { (old, new) };
                *self = Node::Split {
                    axis,
                    first,
                    second,
                };
                true
            }
            Node::Pane(_) => false,
            Node::Split { first, second, .. } => {
                first.split(at, new, direction) || second.split(at, new, direction)
            }
        }
    }

    /// Removes the pane `id`; its sibling takes the split's place. `None` when nothing is left.
    fn remove(self, id: PaneId) -> Option<Node> {
        match self {
            Node::Pane(pane) if pane == id => None,
            Node::Pane(_) => Some(self),
            Node::Split {
                axis,
                first,
                second,
            } => match (first.remove(id), second.remove(id)) {
                (Some(first), Some(second)) => Some(Node::Split {
                    axis,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub root: Node,
    pub active: PaneId,
    /// The pane filling the tab for now (Zoom); the split stays underneath.
    zoomed: Option<PaneId>,
}

impl Tab {
    pub fn panes(&self) -> Vec<PaneId> {
        let mut panes = Vec::new();
        self.root.panes(&mut panes);
        panes
    }
}

/// Where an agent chosen in the sidebar goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Already open there: just bring it forward.
    Existing { tab: usize, pane: PaneId },
    /// Into this (the active) pane.
    Pane(PaneId),
    /// Into a new tab, because the active pane runs a shell.
    NewTab,
}

#[derive(Debug, Default)]
pub struct Workspace {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    shown: HashMap<PaneId, Shown>,
    next: PaneId,
}

impl Workspace {
    /// A workspace with one tab of one pane.
    pub fn new(shown: Shown) -> (Self, PaneId) {
        let mut workspace = Workspace::default();
        let pane = workspace.new_tab(shown);
        (workspace, pane)
    }

    /// A workspace from saved tabs: each tab's tree and active pane, what every pane shows, and
    /// the highest pane ID used, so new panes get fresh ones.
    pub fn from_parts(
        tabs: Vec<(Node, PaneId)>,
        active_tab: usize,
        shown: HashMap<PaneId, Shown>,
        last: PaneId,
    ) -> Self {
        Workspace {
            tabs: tabs
                .into_iter()
                .map(|(root, active)| Tab {
                    root,
                    active,
                    zoomed: None,
                })
                .collect(),
            active_tab,
            shown,
            next: last,
        }
    }

    fn pane(&mut self, shown: Shown) -> PaneId {
        self.next += 1;
        self.shown.insert(self.next, shown);
        self.next
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn active_pane(&self) -> PaneId {
        self.tab().active
    }

    pub fn shown(&self, pane: PaneId) -> &Shown {
        self.shown.get(&pane).unwrap_or(&Shown::Empty)
    }

    pub fn set_shown(&mut self, pane: PaneId, shown: Shown) {
        if self.shown.contains_key(&pane) {
            self.shown.insert(pane, shown);
        }
    }

    /// Every agent open somewhere.
    pub fn agents(&self) -> Vec<String> {
        let mut agents: Vec<String> = self
            .shown
            .values()
            .filter_map(|shown| match shown {
                Shown::Agent(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        agents.sort();
        agents
    }

    /// The active pane's agent, if it shows one.
    pub fn active_agent(&self) -> Option<&str> {
        match self.shown(self.active_pane()) {
            Shown::Agent(name) => Some(name),
            _ => None,
        }
    }

    /// Opens a new tab after the current one and makes it active; returns its pane.
    pub fn new_tab(&mut self, shown: Shown) -> PaneId {
        let pane = self.pane(shown);
        let tab = Tab {
            root: Node::Pane(pane),
            active: pane,
            zoomed: None,
        };
        let at = if self.tabs.is_empty() {
            0
        } else {
            self.active_tab + 1
        };
        self.tabs.insert(at, tab);
        self.active_tab = at;
        pane
    }

    /// Splits the active pane; the new pane becomes active. Returns it.
    pub fn split(&mut self, direction: Direction, shown: Shown) -> PaneId {
        let pane = self.pane(shown);
        let tab = &mut self.tabs[self.active_tab];
        tab.root.split(tab.active, pane, direction);
        tab.active = pane;
        tab.zoomed = None;
        pane
    }

    /// Fills the tab with its active pane, or back to the split. Only with several panes.
    pub fn toggle_zoom(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.zoomed = match tab.zoomed {
            Some(_) => None,
            None if tab.panes().len() > 1 => Some(tab.active),
            None => None,
        };
    }

    /// The active tab's zoomed pane.
    pub fn zoomed(&self) -> Option<PaneId> {
        self.tab().zoomed
    }

    /// Makes `pane` active, and its tab.
    pub fn focus(&mut self, pane: PaneId) {
        if let Some(index) = self.tabs.iter().position(|t| t.panes().contains(&pane)) {
            self.active_tab = index;
            let tab = &mut self.tabs[index];
            tab.active = pane;
            // Another pane ends the zoom.
            if tab.zoomed.is_some_and(|zoomed| zoomed != pane) {
                tab.zoomed = None;
            }
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    /// Closes `pane`; a tab left empty closes too, and the last tab keeps one empty pane. Returns
    /// the panes that are gone, for their sessions to end.
    pub fn close_pane(&mut self, pane: PaneId) -> Vec<PaneId> {
        let Some(index) = self.tabs.iter().position(|t| t.panes().contains(&pane)) else {
            return Vec::new();
        };
        self.shown.remove(&pane);
        let tab = &mut self.tabs[index];
        match tab.root.clone().remove(pane) {
            Some(root) => {
                tab.root = root;
                if tab.zoomed == Some(pane) {
                    tab.zoomed = None;
                }
                if tab.active == pane {
                    tab.active = tab.panes()[0];
                }
            }
            None => {
                self.remove_tab(index);
            }
        }
        vec![pane]
    }

    /// Closes the tab at `index` and returns its panes.
    pub fn close_tab(&mut self, index: usize) -> Vec<PaneId> {
        if index >= self.tabs.len() {
            return Vec::new();
        }
        let panes = self.tabs[index].panes();
        for pane in &panes {
            self.shown.remove(pane);
        }
        self.remove_tab(index);
        panes
    }

    fn remove_tab(&mut self, index: usize) {
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.new_tab(Shown::Empty);
        } else if self.active_tab >= index && self.active_tab > 0 {
            self.active_tab -= 1;
        }
    }

    /// Where an agent chosen in the sidebar goes. `shell_live` says whether a pane showing a shell
    /// still runs it.
    pub fn place(&self, agent: &str, shell_live: impl Fn(PaneId) -> bool) -> Placement {
        for (tab, t) in self.tabs.iter().enumerate() {
            for pane in t.panes() {
                if self.shown(pane) == &Shown::Agent(agent.to_owned()) {
                    return Placement::Existing { tab, pane };
                }
            }
        }
        let active = self.active_pane();
        match self.shown(active) {
            Shown::Shell if shell_live(active) => Placement::NewTab,
            _ => Placement::Pane(active),
        }
    }

    /// The pane showing `agent`, if any.
    pub fn find(&self, agent: &str) -> Option<PaneId> {
        self.shown
            .iter()
            .find(|(_, shown)| **shown == Shown::Agent(agent.to_owned()))
            .map(|(pane, _)| *pane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str) -> Shown {
        Shown::Agent(name.into())
    }

    #[test]
    fn splits_go_where_asked_and_become_active() {
        let (mut w, first) = Workspace::new(Shown::Shell);
        let right = w.split(Direction::Right, Shown::Empty);
        assert_eq!(w.active_pane(), right);
        assert_eq!(
            w.tab().root,
            Node::Split {
                axis: Axis::Row,
                first: Box::new(Node::Pane(first)),
                second: Box::new(Node::Pane(right)),
            }
        );
        let up = w.split(Direction::Up, Shown::Empty);
        assert_eq!(
            w.tab().root,
            Node::Split {
                axis: Axis::Row,
                first: Box::new(Node::Pane(first)),
                second: Box::new(Node::Split {
                    axis: Axis::Column,
                    first: Box::new(Node::Pane(up)),
                    second: Box::new(Node::Pane(right)),
                }),
            }
        );
        w.focus(first);
        let left = w.split(Direction::Left, Shown::Empty);
        let down = w.split(Direction::Down, Shown::Empty);
        assert_eq!(w.tab().panes(), [left, down, first, up, right]);
    }

    #[test]
    fn closing_panes_and_tabs_keeps_something_active() {
        let (mut w, first) = Workspace::new(Shown::Shell);
        let second = w.split(Direction::Right, agent("p/a"));
        assert_eq!(w.close_pane(second), [second]);
        assert_eq!(w.tab().root, Node::Pane(first));
        assert_eq!(w.active_pane(), first);
        assert_eq!(w.agents(), Vec::<String>::new());

        let other = w.new_tab(agent("p/b"));
        assert_eq!((w.tabs.len(), w.active_tab), (2, 1));
        assert_eq!(w.close_pane(other), [other]);
        assert_eq!((w.tabs.len(), w.active_tab), (1, 0));

        // Closing the last tab leaves one empty pane.
        assert_eq!(w.close_tab(0), [first]);
        assert_eq!(w.tabs.len(), 1);
        assert_eq!(w.shown(w.active_pane()), &Shown::Empty);
    }

    #[test]
    fn an_agent_opens_once() {
        let (mut w, _) = Workspace::new(Shown::Empty);
        let a = w.split(Direction::Down, agent("p/a"));
        w.new_tab(Shown::Empty);
        assert_eq!(
            w.place("p/a", |_| true),
            Placement::Existing { tab: 0, pane: a }
        );
        assert_eq!(w.find("p/a"), Some(a));
        w.focus(a);
        assert_eq!(w.active_tab, 0);
        assert_eq!(w.active_agent(), Some("p/a"));
    }

    #[test]
    fn a_running_shell_is_not_replaced() {
        let (mut w, shell) = Workspace::new(Shown::Shell);
        assert_eq!(w.place("p/a", |_| true), Placement::NewTab);
        // A shell that has exited may be replaced, like an empty pane or another agent.
        assert_eq!(w.place("p/a", |_| false), Placement::Pane(shell));
        w.set_shown(shell, agent("p/b"));
        assert_eq!(w.place("p/a", |_| true), Placement::Pane(shell));
    }

    #[test]
    fn new_tabs_open_after_the_current_one() {
        let (mut w, _) = Workspace::new(Shown::Empty);
        let b = w.new_tab(Shown::Empty);
        w.select_tab(0);
        let c = w.new_tab(Shown::Empty);
        assert_eq!(w.active_tab, 1);
        assert_eq!(w.tabs[1].active, c);
        assert_eq!(w.tabs[2].active, b);
    }

    #[test]
    fn zoom_needs_several_panes_and_toggles() {
        let (mut w, first) = Workspace::new(Shown::Shell);
        w.toggle_zoom();
        assert_eq!(w.zoomed(), None);
        let second = w.split(Direction::Right, Shown::Empty);
        w.toggle_zoom();
        assert_eq!(w.zoomed(), Some(second));
        w.toggle_zoom();
        assert_eq!(w.zoomed(), None);
        assert_eq!(w.active_pane(), second);
        w.focus(first);
        w.toggle_zoom();
        assert_eq!(w.zoomed(), Some(first));
    }

    #[test]
    fn zoom_ends_on_another_pane_a_close_or_a_split() {
        let (mut w, first) = Workspace::new(Shown::Shell);
        let second = w.split(Direction::Down, Shown::Shell);
        w.toggle_zoom();
        w.focus(first);
        assert_eq!(w.zoomed(), None);
        w.toggle_zoom();
        w.split(Direction::Left, Shown::Empty);
        assert_eq!(w.zoomed(), None);
        w.focus(second);
        w.toggle_zoom();
        w.close_pane(second);
        assert_eq!(w.zoomed(), None);
    }

    #[test]
    fn zoom_is_kept_per_tab() {
        let (mut w, _) = Workspace::new(Shown::Shell);
        let zoomed = w.split(Direction::Right, Shown::Shell);
        w.toggle_zoom();
        w.new_tab(Shown::Empty);
        assert_eq!(w.zoomed(), None);
        w.select_tab(0);
        assert_eq!(w.zoomed(), Some(zoomed));
    }
}
