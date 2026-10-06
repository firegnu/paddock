//! The Agents list model: ordering, grouping, status and the Git summaries by directory. From
//! Saddle `src/agents.rs` at commit `df1c727`. Changed for paddock, which has no list cursor: being
//! selected no longer counts as read; only the agent shown in the active pane does. The list-wide
//! fold became each agent's own details, opened from its card, and it keeps the last reply read
//! for each idle agent.
use crate::corral::Agent;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Panel {
    pub agents: Vec<Agent>,
    pub selected: Option<String>,
    /// `s` switches from the default status order to plain name order.
    pub by_name: bool,
    /// The agents whose details are open, by name.
    pub expanded: HashSet<String>,
    pub show_reply: bool,
    pub reply_top: usize,
    pub top: usize,
    pub follow: bool,
    pub unread: HashSet<String>,
    pub first_seen: HashMap<String, f64>,
    pub message: String,
    pub confirm: Option<String>,
    pub stopping: bool,
    /// Git summaries by public cwd, kept apart from corral data; a missing key is still loading.
    pub git: HashMap<String, Option<crate::git::Summary>>,
    /// Last replies by the spell they were asked for in, kept only while that spell lasts; `None`
    /// while being read, or when there was none or it could not be read.
    replies: HashMap<Spell, Option<String>>,
}

/// A spell of idling: the agent, its instance and when the state began. Its last reply is read
/// once for each.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Spell {
    pub name: String,
    instance: Option<String>,
    since: Option<u64>,
}

impl Panel {
    pub fn absorb(&mut self, agents: Vec<Agent>, showing: Option<&str>, now: f64) {
        for a in &agents {
            self.first_seen.entry(a.name.clone()).or_insert(now);
            if let Some(old) = self.agents.iter().find(|old| old.name == a.name) {
                if old.instance != a.instance {
                    self.unread.remove(&a.name);
                    self.first_seen.insert(a.name.clone(), now);
                } else if a.state.as_deref() == Some("idle")
                    && (old.state.as_deref() == Some("working")
                        || (a.turn_started.is_some() && a.turn_started != old.turn_started))
                {
                    self.unread.insert(a.name.clone());
                }
            }
        }
        for old in &self.agents {
            if !agents.iter().any(|a| a.name == old.name) {
                self.unread.remove(&old.name);
                self.first_seen.remove(&old.name);
                if self.confirm.is_none() {
                    self.message = format!("{} exited", old.name);
                }
            }
        }
        self.expanded
            .retain(|name| agents.iter().any(|a| &a.name == name));
        self.agents = agents;
        if !self
            .agents
            .iter()
            .any(|a| Some(&a.name) == self.selected.as_ref())
        {
            // A selection made for the list, not by looking: it leaves new replies unread.
            let unread = self.unread.clone();
            self.select(self.ordered(now).first().map(|a| a.name.clone()));
            self.unread = unread;
        }
        if let Some(name) = showing {
            self.unread.remove(name);
        }
    }

    /// Keeps results only for directories agents use now, so a late result for a directory an
    /// agent has left is never shown under it.
    pub fn absorb_git(&mut self, batch: crate::git::Batch) {
        let current: HashSet<&str> = self
            .agents
            .iter()
            .filter_map(|a| a.cwd.as_deref())
            .collect();
        self.git.retain(|cwd, _| current.contains(cwd.as_str()));
        for (cwd, summary) in batch {
            if current.contains(cwd.as_str()) {
                self.git.insert(cwd, summary);
            }
        }
    }

    /// Opens or closes one agent's details.
    pub fn toggle_details(&mut self, name: &str) {
        if !self.expanded.remove(name) {
            self.expanded.insert(name.to_owned());
        }
    }
    /// Closes every agent's details.
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// The spell an idle agent is in; `None` in any other state (waiting shows its question).
    fn spell(&self, a: &Agent, now: f64) -> Option<Spell> {
        (self.status(a, now) == Status::Idle).then(|| Spell {
            name: a.name.clone(),
            instance: a.instance.clone(),
            since: a.state_started.map(f64::to_bits),
        })
    }

    /// The spells whose reply is still to be asked for, now taken as asked: each is asked once,
    /// whatever the answer. Spells that have ended are forgotten with their replies.
    pub fn replies_to_read(&mut self, now: f64) -> Vec<Spell> {
        let current: HashSet<Spell> = self
            .agents
            .iter()
            .filter_map(|a| self.spell(a, now))
            .collect();
        self.replies.retain(|spell, _| current.contains(spell));
        let mut new: Vec<Spell> = current
            .into_iter()
            .filter(|spell| !self.replies.contains_key(spell))
            .collect();
        new.sort_by(|a, b| a.name.cmp(&b.name));
        for spell in &new {
            self.replies.insert(spell.clone(), None);
        }
        new
    }

    /// A reply read for a spell; dropped when the spell has ended meanwhile.
    pub fn absorb_reply(&mut self, spell: Spell, text: Option<String>) {
        if let Some(slot) = self.replies.get_mut(&spell) {
            *slot = text;
        }
    }

    /// The agent's last reply, as read in the spell it is in now.
    pub fn reply(&self, a: &Agent, now: f64) -> Option<&str> {
        self.replies.get(&self.spell(a, now)?)?.as_deref()
    }

    pub fn select(&mut self, name: Option<String>) {
        if self.selected != name {
            self.reply_top = 0;
        }
        if let Some(name) = &name {
            self.unread.remove(name);
        }
        self.selected = name;
        self.follow = true;
    }

    pub fn move_selection(&mut self, step: isize, now: f64) {
        let ordered = self.ordered(now);
        if ordered.is_empty() {
            return;
        }
        let current = ordered
            .iter()
            .position(|a| Some(&a.name) == self.selected.as_ref())
            .unwrap_or(0);
        let at = current.saturating_add_signed(step).min(ordered.len() - 1);
        self.select(Some(ordered[at].name.clone()));
    }

    pub fn ordered(&self, now: f64) -> Vec<&Agent> {
        let mut agents: Vec<_> = self.agents.iter().collect();
        agents.sort_by(|a, b| {
            group(&a.name)
                .cmp(group(&b.name))
                .then_with(|| {
                    if self.by_name {
                        std::cmp::Ordering::Equal
                    } else {
                        self.status(a, now).cmp(&self.status(b, now))
                    }
                })
                .then_with(|| a.name.cmp(&b.name))
        });
        agents
    }

    pub fn suspect(&self, a: &Agent, now: f64) -> bool {
        match a.state.as_deref() {
            Some("working") => a.last_output.is_some_and(|last| now - last >= 120.0),
            Some("starting") => {
                now - a
                    .started
                    .or_else(|| self.first_seen.get(&a.name).copied())
                    .unwrap_or(now)
                    >= 60.0
            }
            _ if a.starting => now - self.first_seen.get(&a.name).copied().unwrap_or(now) >= 60.0,
            _ => false,
        }
    }
    /// The Agents view of the public state; `blocked` shows as waiting. Declaration order
    /// is the default order within a group, agents needing a person first.
    pub fn status(&self, a: &Agent, now: f64) -> Status {
        if a.error.is_some() || a.incompatible {
            return Status::Error;
        }
        if self.suspect(a, now) {
            return Status::Stalled;
        }
        if a.starting {
            return Status::Starting;
        }
        match a.state.as_deref() {
            Some("blocked") => Status::Waiting,
            Some("working") => Status::Working,
            Some("starting") => Status::Starting,
            Some("idle") => Status::Idle,
            Some("exited") => Status::Exited,
            _ => Status::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Waiting,
    Error,
    Stalled,
    Working,
    Starting,
    Unknown,
    Idle,
    Exited,
}

pub fn group(name: &str) -> &str {
    name.find('/').map(|i| &name[..=i]).unwrap_or("")
}
