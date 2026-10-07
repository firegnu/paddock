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

/// A new split's share for its first part: half.
pub const EVEN: f32 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Pane(PaneId),
    /// Named by its second part's first pane, the one just after its seam.
    Split {
        axis: Axis,
        /// The first part's share, between 0 and 1.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    pub fn first_pane(&self) -> PaneId {
        match self {
            Node::Pane(id) => *id,
            Node::Split { first, .. } => first.first_pane(),
        }
    }

    /// The split whose seam comes just before the pane `after`.
    pub fn split_before(&self, after: PaneId) -> Option<&Node> {
        match self {
            Node::Pane(_) => None,
            Node::Split { first, second, .. } => {
                if second.first_pane() == after {
                    Some(self)
                } else {
                    first
                        .split_before(after)
                        .or_else(|| second.split_before(after))
                }
            }
        }
    }

    fn ratio_before(&mut self, after: PaneId) -> Option<&mut f32> {
        match self {
            Node::Pane(_) => None,
            Node::Split {
                ratio,
                first,
                second,
                ..
            } => {
                if second.first_pane() == after {
                    Some(ratio)
                } else {
                    first
                        .ratio_before(after)
                        .or_else(|| second.ratio_before(after))
                }
            }
        }
    }

    /// How long the tree must be along `axis` for every pane to keep `pane` (its least width and
    /// height), with `gap` between neighbours.
    pub fn least(&self, axis: Axis, pane: (f32, f32), gap: f32) -> f32 {
        match self {
            Node::Pane(_) => match axis {
                Axis::Row => pane.0,
                Axis::Column => pane.1,
            },
            Node::Split {
                axis: along,
                first,
                second,
                ..
            } => {
                let (first, second) = (first.least(axis, pane, gap), second.least(axis, pane, gap));
                if *along == axis {
                    first + gap + second
                } else {
                    first.max(second)
                }
            }
        }
    }

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
                    ratio: EVEN,
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

    /// Gives the pane `old` the ID `new`, where it is.
    fn rename(&mut self, old: PaneId, new: PaneId) -> bool {
        match self {
            Node::Pane(id) if *id == old => {
                *id = new;
                true
            }
            Node::Pane(_) => false,
            Node::Split { first, second, .. } => first.rename(old, new) || second.rename(old, new),
        }
    }

    /// Removes the pane `id`; its sibling takes the split's place. `None` when nothing is left.
    fn remove(self, id: PaneId) -> Option<Node> {
        match self {
            Node::Pane(pane) if pane == id => None,
            Node::Pane(_) => Some(self),
            Node::Split {
                axis,
                ratio,
                first,
                second,
            } => match (first.remove(id), second.remove(id)) {
                (Some(first), Some(second)) => Some(Node::Split {
                    axis,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            },
        }
    }
}

/// The share a split `length` long, its seam `gap` wide, may give its first part when asked for
/// `ratio`: each part keeps at least its least length (`least`). When both cannot, they share in
/// proportion to those.
pub fn clamp_ratio(ratio: f32, length: f32, gap: f32, least: (f32, f32)) -> f32 {
    let room = length - gap;
    if room <= 0.0 || least.0 + least.1 <= 0.0 {
        return EVEN;
    }
    let (low, high) = (least.0 / room, 1.0 - least.1 / room);
    if low > high {
        least.0 / (least.0 + least.1)
    } else {
        ratio.clamp(low, high)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    /// Stable while the window runs, for `paddock ctl`; not saved.
    pub id: u64,
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

/// Where `paddock ctl open` puts a pane, beside the pane it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    /// A new tab just after that pane's tab.
    Tab,
    Side(Direction),
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
    /// How many times a pane's content was set since it was made, for `paddock ctl`.
    changes: HashMap<PaneId, u64>,
    next: PaneId,
    next_tab: u64,
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
        let mut next_tab = 0;
        Workspace {
            tabs: tabs
                .into_iter()
                .map(|(root, active)| {
                    next_tab += 1;
                    Tab {
                        id: next_tab,
                        root,
                        active,
                        zoomed: None,
                    }
                })
                .collect(),
            active_tab,
            shown,
            changes: HashMap::new(),
            next: last,
            next_tab,
        }
    }

    fn pane(&mut self, shown: Shown) -> PaneId {
        self.next += 1;
        self.shown.insert(self.next, shown);
        self.next
    }

    fn tab_id(&mut self) -> u64 {
        self.next_tab += 1;
        self.next_tab
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
        if let Some(old) = self.shown.get_mut(&pane) {
            *old = shown;
            *self.changes.entry(pane).or_default() += 1;
        }
    }

    /// What the pane shows starts again (an agent attached anew): a new revision, as when it is
    /// set.
    pub fn touch(&mut self, pane: PaneId) {
        if self.shown.contains_key(&pane) {
            *self.changes.entry(pane).or_default() += 1;
        }
    }

    /// The pane's revision: it goes up each time what the pane shows is set, so a request about
    /// what it showed before can tell. `None` for a pane that is not open.
    pub fn revision(&self, pane: PaneId) -> Option<u64> {
        self.shown
            .contains_key(&pane)
            .then(|| self.changes.get(&pane).copied().unwrap_or(0) + 1)
    }

    /// The index of the tab holding `pane`.
    pub fn tab_of(&self, pane: PaneId) -> Option<usize> {
        self.tabs.iter().position(|t| t.panes().contains(&pane))
    }

    /// The index of the tab with this ID.
    pub fn tab_index(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Gives `pane` a fresh ID where it is, showing `shown`: a shell started in it later must not
    /// be taken for one that ran there before (`paddock ctl` knows shells by their pane). Returns
    /// the new ID.
    pub fn renew(&mut self, pane: PaneId, shown: Shown) -> PaneId {
        let Some(index) = self.tab_of(pane) else {
            return pane;
        };
        let new = self.pane(shown);
        self.shown.remove(&pane);
        self.changes.remove(&pane);
        let tab = &mut self.tabs[index];
        tab.root.rename(pane, new);
        if tab.active == pane {
            tab.active = new;
        }
        if tab.zoomed == Some(pane) {
            tab.zoomed = Some(new);
        }
        new
    }

    /// Opens a pane showing `shown` beside `anchor` (`paddock ctl open`). Which tab and pane are
    /// active stays as it was. `None` when `anchor` is not open.
    pub fn open_at(&mut self, anchor: PaneId, at: At, shown: Shown) -> Option<PaneId> {
        let index = self.tab_of(anchor)?;
        let pane = self.pane(shown);
        self.insert_at(index, anchor, at, pane);
        Some(pane)
    }

    /// Moves `pane` beside `anchor`, keeping its ID and what it shows; a tab it leaves empty
    /// closes. Which tab and pane are active stays as it was where it can. False when nothing
    /// moved: either is not open, a pane beside itself, or already alone in a tab for `Tab`.
    pub fn move_pane(&mut self, pane: PaneId, anchor: PaneId, at: At) -> bool {
        let (Some(from), Some(to)) = (self.tab_of(pane), self.tab_of(anchor)) else {
            return false;
        };
        // The anchor's tab outlives the move: only the moved pane's own tab can close.
        let to = self.tabs[to].id;
        let alone = self.tabs[from].panes().len() == 1;
        if (pane == anchor && at != At::Tab) || (alone && at == At::Tab && pane == anchor) {
            return false;
        }
        if alone {
            self.tabs.remove(from);
            if self.active_tab > from || (self.active_tab == from && self.active_tab > 0) {
                self.active_tab -= 1;
            }
        } else {
            let tab = &mut self.tabs[from];
            tab.root = tab.root.clone().remove(pane).expect("other panes remain");
            if tab.zoomed == Some(pane) {
                tab.zoomed = None;
            }
            if tab.active == pane {
                tab.active = tab.panes()[0];
            }
        }
        let to = self.tab_index(to).expect("the anchor's tab stays");
        self.insert_at(to, anchor, at, pane);
        true
    }

    /// Puts `pane`, in no tab yet, beside `anchor`, which is open in the tab at `index`.
    fn insert_at(&mut self, index: usize, anchor: PaneId, at: At, pane: PaneId) {
        match at {
            At::Tab => {
                let id = self.tab_id();
                self.tabs.insert(
                    index + 1,
                    Tab {
                        id,
                        root: Node::Pane(pane),
                        active: pane,
                        zoomed: None,
                    },
                );
                if self.active_tab > index {
                    self.active_tab += 1;
                }
            }
            At::Side(direction) => {
                self.tabs[index].root.split(anchor, pane, direction);
            }
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
            id: self.tab_id(),
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

    /// Gives the active tab's split before the pane `after` this share for its first part, when it
    /// is strictly between 0 and 1. Whether it changed.
    pub fn set_ratio(&mut self, after: PaneId, share: f32) -> bool {
        if !(share > 0.0 && share < 1.0) {
            return false;
        }
        match self.tabs[self.active_tab].root.ratio_before(after) {
            Some(ratio) if *ratio != share => {
                *ratio = share;
                true
            }
            _ => false,
        }
    }

    /// Splits the active tab's split before the pane `after` in half again.
    pub fn even(&mut self, after: PaneId) -> bool {
        self.set_ratio(after, EVEN)
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
        self.changes.remove(&pane);
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
            self.changes.remove(pane);
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
                ratio: EVEN,
                first: Box::new(Node::Pane(first)),
                second: Box::new(Node::Pane(right)),
            }
        );
        let up = w.split(Direction::Up, Shown::Empty);
        assert_eq!(
            w.tab().root,
            Node::Split {
                axis: Axis::Row,
                ratio: EVEN,
                first: Box::new(Node::Pane(first)),
                second: Box::new(Node::Split {
                    axis: Axis::Column,
                    ratio: EVEN,
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

    fn ratios(node: &Node, out: &mut Vec<f32>) {
        if let Node::Split {
            ratio,
            first,
            second,
            ..
        } = node
        {
            out.push(*ratio);
            ratios(first, out);
            ratios(second, out);
        }
    }

    #[test]
    fn moving_a_nested_seam_changes_only_its_split() {
        // first | (up / right)
        let (mut w, first) = Workspace::new(Shown::Shell);
        let right = w.split(Direction::Right, Shown::Empty);
        let up = w.split(Direction::Up, Shown::Empty);
        assert_eq!(w.tab().root.split_before(up), Some(&w.tab().root));
        let mut all = Vec::new();
        ratios(&w.tab().root, &mut all);
        assert_eq!(all, [EVEN, EVEN]);

        // The inner seam, between up and right.
        assert!(w.set_ratio(right, 0.7));
        let mut all = Vec::new();
        ratios(&w.tab().root, &mut all);
        assert_eq!(all, [EVEN, 0.7]);
        // The outer one, between first and the inner split.
        assert!(w.set_ratio(up, 0.3));
        let mut all = Vec::new();
        ratios(&w.tab().root, &mut all);
        assert_eq!(all, [0.3, 0.7]);

        // The same share again, no seam before the first pane, or a share out of range: nothing.
        assert!(!w.set_ratio(up, 0.3));
        assert!(!w.set_ratio(first, 0.4));
        assert!(!w.set_ratio(right, 0.0));
        assert!(!w.set_ratio(right, 1.0));
        assert!(!w.set_ratio(right, f32::NAN));
        let mut all = Vec::new();
        ratios(&w.tab().root, &mut all);
        assert_eq!(all, [0.3, 0.7]);

        // Closing a pane keeps the other splits' shares.
        w.close_pane(right);
        assert_eq!(
            w.tab().root,
            Node::Split {
                axis: Axis::Row,
                ratio: 0.3,
                first: Box::new(Node::Pane(first)),
                second: Box::new(Node::Pane(up)),
            }
        );
    }

    #[test]
    fn a_double_click_splits_in_half_again() {
        let (mut w, _) = Workspace::new(Shown::Shell);
        let right = w.split(Direction::Right, Shown::Empty);
        let down = w.split(Direction::Down, Shown::Empty);
        w.set_ratio(right, 0.25);
        w.set_ratio(down, 0.8);
        assert!(w.even(down));
        let mut all = Vec::new();
        ratios(&w.tab().root, &mut all);
        assert_eq!(all, [0.25, EVEN]);
        assert!(!w.even(down));
    }

    #[test]
    fn seams_stop_at_each_parts_least_size() {
        // 1000 long with an 8 seam: 992 to share; each part at least 200.
        let least = (200.0, 200.0);
        assert_eq!(clamp_ratio(0.6, 1000.0, 8.0, least), 0.6);
        assert_eq!(clamp_ratio(0.05, 1000.0, 8.0, least), 200.0 / 992.0);
        assert_eq!(clamp_ratio(0.99, 1000.0, 8.0, least), 1.0 - 200.0 / 992.0);
        // Too small for both: they share by their least sizes, never a negative one.
        assert_eq!(clamp_ratio(0.9, 300.0, 8.0, (100.0, 300.0)), 0.25);
        assert_eq!(clamp_ratio(0.9, 4.0, 8.0, least), EVEN);

        // A part's least size counts the panes in it along the seam's axis.
        let (mut w, _) = Workspace::new(Shown::Shell);
        w.split(Direction::Right, Shown::Empty);
        w.split(Direction::Down, Shown::Empty);
        let pane = (150.0, 80.0);
        let root = &w.tab().root;
        assert_eq!(root.least(Axis::Row, pane, 8.0), 150.0 + 8.0 + 150.0);
        assert_eq!(root.least(Axis::Column, pane, 8.0), 80.0 + 8.0 + 80.0);
    }

    #[test]
    fn ctl_opens_beside_any_pane_without_moving_the_focus() {
        // first | second, second active; another tab after it.
        let (mut w, first) = Workspace::new(Shown::Shell);
        let second = w.split(Direction::Right, Shown::Shell);
        let other = w.new_tab(Shown::Empty);
        w.focus(second);
        let ids: Vec<u64> = w.tabs.iter().map(|t| t.id).collect();

        // Each side splits the pane named, not the active one, and nothing else changes.
        for (direction, axis, new_first) in [
            (Direction::Left, Axis::Row, true),
            (Direction::Right, Axis::Row, false),
            (Direction::Up, Axis::Column, true),
            (Direction::Down, Axis::Column, false),
        ] {
            let mut w2 = Workspace::from_parts(
                w.tabs.iter().map(|t| (t.root.clone(), t.active)).collect(),
                w.active_tab,
                [first, second, other]
                    .into_iter()
                    .map(|p| (p, w.shown(p).clone()))
                    .collect(),
                other,
            );
            let new = w2
                .open_at(first, At::Side(direction), Shown::Empty)
                .unwrap();
            let (a, b) = if new_first {
                (new, first)
            } else {
                (first, new)
            };
            assert_eq!(
                w2.tabs[0].root,
                Node::Split {
                    axis: Axis::Row,
                    ratio: EVEN,
                    first: Box::new(Node::Split {
                        axis,
                        ratio: EVEN,
                        first: Box::new(Node::Pane(a)),
                        second: Box::new(Node::Pane(b)),
                    }),
                    second: Box::new(Node::Pane(second)),
                },
                "{direction:?}"
            );
            assert_eq!((w2.active_tab, w2.active_pane()), (0, second));
        }

        // A tab goes just after the named pane's tab; the active tab stays the same tab.
        w.select_tab(1);
        let tabbed = w.open_at(first, At::Tab, Shown::Shell).unwrap();
        assert_eq!(w.tabs.len(), 3);
        assert_eq!(w.tabs[1].active, tabbed);
        assert_eq!((w.active_tab, w.active_pane()), (2, other));
        assert_eq!(&ids[..], &[w.tabs[0].id, w.tabs[2].id]);
        assert!(!ids.contains(&w.tabs[1].id));
        // Nothing for a pane that is not open.
        assert_eq!(w.open_at(999, At::Tab, Shown::Empty), None);
    }

    #[test]
    fn ctl_moves_a_pane_with_its_id_and_closes_the_tab_it_empties() {
        let (mut w, first) = Workspace::new(Shown::Shell);
        let agent_tab = w.new_tab(agent("p/a"));
        let revision = w.revision(agent_tab);
        w.select_tab(0);
        assert!(w.move_pane(agent_tab, first, At::Side(Direction::Right)));
        assert_eq!(w.tabs.len(), 1);
        assert_eq!(w.tab().panes(), [first, agent_tab]);
        assert_eq!(w.active_pane(), first);
        assert_eq!(w.shown(agent_tab), &agent("p/a"));
        assert_eq!(w.revision(agent_tab), revision);
        // Beside itself it stays; alone in its tab, a new tab changes nothing.
        assert!(!w.move_pane(agent_tab, agent_tab, At::Side(Direction::Down)));
        assert!(w.move_pane(agent_tab, agent_tab, At::Tab));
        assert_eq!(w.tabs.len(), 2);
        assert!(!w.move_pane(agent_tab, agent_tab, At::Tab));
        assert_eq!(w.tabs[1].panes(), [agent_tab]);
        assert_eq!(w.active_tab, 0);
    }

    #[test]
    fn revisions_rise_with_the_content_and_renewing_gives_a_fresh_id() {
        let (mut w, pane) = Workspace::new(Shown::Empty);
        assert_eq!(w.revision(pane), Some(1));
        w.set_shown(pane, Shown::Shell);
        w.set_shown(pane, agent("p/a"));
        assert_eq!(w.revision(pane), Some(3));
        w.toggle_zoom();
        let other = w.split(Direction::Down, Shown::Empty);
        w.focus(pane);
        w.toggle_zoom();
        let renewed = w.renew(pane, Shown::Shell);
        assert!(renewed != pane && renewed > other);
        assert_eq!(w.revision(pane), None);
        assert_eq!(w.revision(renewed), Some(1));
        assert_eq!(w.tab().panes(), [renewed, other]);
        assert_eq!((w.active_pane(), w.zoomed()), (renewed, Some(renewed)));
        w.close_pane(renewed);
        assert_eq!(w.revision(renewed), None);
    }
}
