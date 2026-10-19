//! The Agents list model: ordering, grouping, status and the Git summaries by directory. From
//! Saddle `src/agents.rs` at commit `df1c727`. Changed for paddock, which has no list cursor: being
//! selected no longer counts as read; only the agent shown in the active pane does. The list-wide
//! fold became each agent's own details, opened from its card, and it keeps the last reply read
//! for each idle or waiting agent, the latest of them past its spell for the Inbox.
use crate::{
    corral::{Agent, Reply},
    kanban_repos,
    workspaces::{Listed, Workspaces},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Panel {
    pub agents: Vec<Agent>,
    pub selected: Option<String>,
    /// `s` switches from the default status order to plain name order.
    pub by_name: bool,
    /// The agents whose details are open, by name.
    pub expanded: HashSet<String>,
    /// The groups and workspaces folded to one line (DESIGN §13 P5-89).
    pub folded: Folds,
    /// The workspaces, over the groups (DESIGN §13 P5-89).
    pub workspaces: Workspaces,
    /// Workspaces with no agent that stay above, open, rather than at the list's foot: opened
    /// from there or just made (P5-134). Each goes back once its title is clicked, or once it
    /// has had agents and they have stopped. Kept in memory alone.
    opened: Vec<String>,
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
    replies: HashMap<Spell, Option<Reply>>,
    /// The last reply read of each agent with the instance it was read from, kept past its spell
    /// for the Inbox (DESIGN §13 P5-87), which shows it while the agent works on.
    last: HashMap<String, (Option<String>, Reply)>,
    /// Whether fence is there and what its wall stopped for each sandboxed agent (DESIGN §13
    /// P5-120); this Mac's alone.
    pub fence: crate::fence::Seen,
    /// The controllers' Delegates choices being saved or refused (DESIGN §13 P5-141).
    pub delegates: crate::delegates::Saves,
}

/// The groups folded to their heading and the workspaces folded to their title, their cards out
/// of sight: saved with the layout, so one stays folded while it has no agents (DESIGN §13
/// P5-89).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Folds {
    /// The groups in no workspace, by name, as [`Panel::group`] gives it.
    pub groups: Vec<String>,
    /// By name.
    pub workspaces: Vec<String>,
    /// The groups inside a workspace: the workspace, then the group.
    pub within: Vec<(String, String)>,
    /// The `Idle workspaces` group at the list's foot is folded to its heading (P5-134). Left out
    /// of the file while open, as files from before it have it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub idle: bool,
    /// The workspaces whose one line of repositories with no agent is opened out, a line each,
    /// by name (P5-134); left out while there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vacant_open: Vec<String>,
}

impl Folds {
    /// Whether the group is folded: the one of that name in `workspace`, or in none.
    pub fn group(&self, workspace: Option<&str>, group: &str) -> bool {
        match workspace {
            Some(workspace) => self
                .within
                .iter()
                .any(|(w, g)| w == workspace && g == group),
            None => self.groups.iter().any(|folded| folded == group),
        }
    }

    /// Folds the group, or opens it again.
    pub fn toggle_group(&mut self, workspace: Option<&str>, group: &str) {
        match workspace {
            Some(workspace) => toggle(&mut self.within, (workspace.to_owned(), group.to_owned())),
            None => toggle(&mut self.groups, group.to_owned()),
        }
    }

    pub fn workspace(&self, name: &str) -> bool {
        self.workspaces.iter().any(|folded| folded == name)
    }

    /// Folds the workspace, or opens it again.
    pub fn toggle_workspace(&mut self, name: &str) {
        toggle(&mut self.workspaces, name.to_owned());
    }

    /// Opens the workspace, if it was folded.
    pub fn open_workspace(&mut self, name: &str) {
        self.workspaces.retain(|folded| folded != name);
    }

    /// Whether the workspace's line of repositories with no agent is opened out (P5-134).
    pub fn vacant_open(&self, workspace: &str) -> bool {
        self.vacant_open.iter().any(|open| open == workspace)
    }

    /// Opens out the workspace's line of repositories with no agent, or puts them back in one.
    pub fn toggle_vacant(&mut self, workspace: &str) {
        toggle(&mut self.vacant_open, workspace.to_owned());
    }

    /// A workspace has a new name: what was folded or opened out of it stays so.
    pub fn rename_workspace(&mut self, from: &str, to: &str) {
        let names = self.workspaces.iter_mut().chain(&mut self.vacant_open);
        for name in names.chain(self.within.iter_mut().map(|(workspace, _)| workspace)) {
            if name == from {
                to.clone_into(name);
            }
        }
    }

    /// A workspace is gone, and its folds with it.
    pub fn forget_workspace(&mut self, name: &str) {
        self.workspaces.retain(|folded| folded != name);
        self.within.retain(|(workspace, _)| workspace != name);
        self.vacant_open.retain(|open| open != name);
    }
}

/// Takes `item` out of `list`, or adds it when it was not there.
fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T) {
    match list.iter().position(|listed| *listed == item) {
        Some(at) => {
            list.remove(at);
        }
        None => list.push(item),
    }
}

/// An agent's last reply as the Inbox shows it.
pub struct Said<'a> {
    pub reply: &'a Reply,
    /// Read in the spell the agent is in now, rather than kept from one before.
    pub current: bool,
}

/// A spell of idling or of waiting for a person: the agent, its instance and when the state began.
/// Its last reply is read once for each.
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
                    self.last.remove(&a.name);
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
                self.last.remove(&old.name);
                self.first_seen.remove(&old.name);
                if self.confirm.is_none() {
                    self.message = format!("{} exited", old.name);
                }
            }
        }
        self.expanded
            .retain(|name| agents.iter().any(|a| &a.name == name));
        self.agents = agents;
        self.settle_opened();
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
        // An agent's repository, read, may put it in an opened workspace.
        self.settle_opened();
    }

    /// Whether the workspace stays above while it has no agent, opened (P5-134).
    pub fn opened(&self, workspace: &str) -> bool {
        self.opened.iter().any(|opened| opened == workspace)
    }

    /// A workspace with no agent comes up from the list's foot, open whatever was kept of it
    /// (P5-134): opened there, or just made. One made with agents is up as any is.
    pub fn raise(&mut self, workspace: &str) {
        if !self.opened(workspace) {
            self.opened.push(workspace.to_owned());
        }
        self.folded.open_workspace(workspace);
        self.settle_opened();
    }

    /// The workspace goes back down while it has no agent; what is kept folded of it stays as it
    /// was. Whether it was up.
    pub fn lower(&mut self, workspace: &str) -> bool {
        let up = self.opened(workspace);
        self.opened.retain(|opened| opened != workspace);
        up
    }

    /// A workspace has a new name: what is folded, opened out or up of it stays so.
    pub fn rename_workspace(&mut self, from: &str, to: &str) {
        self.folded.rename_workspace(from, to);
        for name in &mut self.opened {
            if name == from {
                to.clone_into(name);
            }
        }
    }

    /// A workspace is gone, and what was kept of it.
    pub fn forget_workspace(&mut self, name: &str) {
        self.folded.forget_workspace(name);
        self.lower(name);
    }

    /// An opened workspace that has agents is up as any is: it goes down once they stop.
    fn settle_opened(&mut self) {
        let busy: HashSet<String> = (self.agents.iter())
            .filter_map(|agent| self.workspace(agent).map(str::to_owned))
            .collect();
        self.opened.retain(|name| !busy.contains(name));
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

    /// The spell an idle or waiting agent is in; `None` in any other state. A waiting agent's card
    /// shows its question; the Inbox shows what it said before asking.
    fn spell(&self, a: &Agent, now: f64) -> Option<Spell> {
        matches!(self.status(a, now), Status::Idle | Status::Waiting).then(|| Spell {
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
    pub fn absorb_reply(&mut self, spell: Spell, reply: Option<impl Into<Reply>>) {
        if let Some(slot) = self.replies.get_mut(&spell) {
            *slot = reply.map(Into::into);
            if let Some(reply) = slot {
                let kept = (spell.instance, reply.clone());
                self.last.insert(spell.name, kept);
            }
        }
    }

    /// The agent's last reply, as read in the spell it is in now.
    pub fn reply(&self, a: &Agent, now: f64) -> Option<&str> {
        let reply = self.replies.get(&self.spell(a, now)?)?.as_ref()?;
        Some(&reply.text)
    }

    /// The agent's last reply for the Inbox: the one read in the spell it is in now, else the
    /// last one read of this instance before.
    pub fn said(&self, a: &Agent, now: f64) -> Option<Said<'_>> {
        let current = self
            .spell(a, now)
            .and_then(|spell| self.replies.get(&spell)?.as_ref());
        if let Some(reply) = current {
            return Some(Said {
                reply,
                current: true,
            });
        }
        let (instance, reply) = self.last.get(&a.name)?;
        (*instance == a.instance).then_some(Said {
            reply,
            current: false,
        })
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

    /// Moves the selection `step` cards on, among the cards in view: those of folded groups are
    /// passed over (P5-89).
    pub fn move_selection(&mut self, step: isize, now: f64) {
        let ordered = self.ordered(now);
        let in_view: Vec<&Agent> = ordered
            .iter()
            .copied()
            .filter(|a| self.in_view(a))
            .collect();
        let Some(last) = in_view.len().checked_sub(1) else {
            return;
        };
        let selected = ordered
            .iter()
            .position(|a| Some(&a.name) == self.selected.as_ref());
        let from = selected.map_or(0, |at| {
            let before = ordered[..at].iter().filter(|a| self.in_view(a)).count() as isize;
            // Folded away itself, it stands between its neighbours in view.
            if self.in_view(ordered[at]) || step <= 0 {
                before
            } else {
                before - 1
            }
        });
        let at = (from + step).clamp(0, last as isize) as usize;
        self.select(Some(in_view[at].name.clone()));
    }

    /// Whether the agent's card shows: neither its group nor its workspace is folded.
    fn in_view(&self, agent: &Agent) -> bool {
        let workspace = self.workspace(agent);
        !workspace.is_some_and(|name| self.folded.workspace(name))
            && !self.folded.group(workspace, self.group(agent))
    }

    /// The main worktree of the repository the agent works in, once Git has told. This only
    /// consults the background Git results, never the filesystem.
    pub fn repository(&self, agent: &Agent) -> Option<&Path> {
        let summary = self.git.get(agent.cwd.as_ref()?)?.as_ref()?;
        summary.main_repository.as_deref()
    }

    /// The workspace the agent is in (DESIGN §13 P5-89): its repository's, whatever its name
    /// (P5-105); one whose repository cannot be read goes by its name's prefix.
    pub fn workspace(&self, agent: &Agent) -> Option<&str> {
        let prefix = group(&agent.name).trim_end_matches('/');
        self.workspaces.of(self.repository(agent), prefix)
    }

    /// Whether Git has told, of every agent running under a name an older paddock recorded in a
    /// workspace, which repository it works in, if any (P5-105): till then those names wait, so
    /// that none goes when the list is kept before its repository could be read.
    pub fn named_read(&self) -> bool {
        let mut names = self.workspaces.list().iter().flat_map(|w| &w.agents);
        names.all(|name| {
            let running = self.agents.iter().find(|a| a.name == *name);
            let cwd = running.and_then(|a| a.cwd.as_ref());
            cwd.is_none_or(|cwd| self.git.contains_key(cwd))
        })
    }

    /// The names an older paddock recorded in a workspace give way to the repositories their
    /// running agents work in, as Git has told them (see [`Workspaces::adopt`]). Whether any did.
    pub fn adopt_named(&mut self) -> bool {
        let repos: HashMap<&str, PathBuf> = (self.agents.iter())
            .filter_map(|a| Some((a.name.as_str(), self.repository(a)?.to_path_buf())))
            .collect();
        self.workspaces.adopt(|name| repos.get(name).cloned())
    }

    /// The repositories agents work in now, each once by its main worktree, by name and then
    /// path, for the panel that picks a workspace's repositories: two of the same name are two
    /// rows there, though the list has them in one group (P5-105). An agent that goes by its
    /// prefix has none, and is in none of them.
    pub fn repositories(&self) -> Vec<Listed> {
        let mut listed: Vec<Listed> = Vec::new();
        let repos = (self.agents.iter()).filter_map(|agent| self.repository(agent));
        for repo in repos {
            match listed.iter_mut().find(|listed| listed.repo == repo) {
                Some(listed) => listed.agents += 1,
                None => listed.push(Listed {
                    repo: repo.to_owned(),
                    agents: 1,
                }),
            }
        }
        listed.sort_by_cached_key(|listed| (kanban_repos::name(&listed.repo), listed.repo.clone()));
        listed
    }

    /// Changes the workspaces kept at `path` (in memory alone with none) by `change`, read again
    /// first, as another window may have changed them (P5-89). The names an older paddock recorded
    /// give way to their repositories first, as far as Git has told (P5-105). The list is kept
    /// when `change` changed it, or when names gave way and none is still waiting for Git: kept,
    /// the list leaves the names out, and one still waiting would be lost. Else it stays as read.
    pub fn edit_workspaces(
        &mut self,
        path: Option<&Path>,
        change: impl FnOnce(&mut Workspaces),
    ) -> io::Result<()> {
        if let Some(path) = path {
            self.workspaces = Workspaces::load(path);
        }
        let read = self.workspaces.clone();
        let adopted = self.adopt_named();
        let before = self.workspaces.clone();
        change(&mut self.workspaces);
        // Repositories with agents added to an opened workspace put it up as any is.
        self.settle_opened();
        if self.workspaces == before && !(adopted && self.named_read()) {
            self.workspaces = read;
            return Ok(());
        }
        match path {
            Some(path) => self.workspaces.save(path),
            None => Ok(()),
        }
    }

    /// The main worktree's directory name, or the original prefix while unread/unavailable.
    /// This only consults the background Git results, never the filesystem.
    pub fn group<'a>(&'a self, agent: &'a Agent) -> &'a str {
        agent
            .cwd
            .as_ref()
            .and_then(|cwd| self.git.get(cwd))
            .and_then(Option::as_ref)
            .and_then(|summary| summary.main_repository.as_ref())
            .and_then(|main| main.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| group(&agent.name).trim_end_matches('/'))
    }

    /// The agents as listed: those of the workspaces first, a workspace after another by name
    /// (P5-89), then those in none; in each by group, and in a group by status or by name.
    pub fn ordered(&self, now: f64) -> Vec<&Agent> {
        // Where an agent's workspace comes in their list; after them all for one in none.
        let rank = |agent: &Agent| {
            let workspaces = self.workspaces.list();
            self.workspace(agent)
                .and_then(|name| workspaces.iter().position(|w| w.name == name))
                .unwrap_or(workspaces.len())
        };
        let mut agents: Vec<_> = self.agents.iter().map(|a| (rank(a), a)).collect();
        agents.sort_by(|(rank_a, a), (rank_b, b)| {
            rank_a
                .cmp(rank_b)
                .then_with(|| self.group(a).cmp(self.group(b)))
                .then_with(|| {
                    if self.by_name {
                        std::cmp::Ordering::Equal
                    } else {
                        self.shown(a, now).cmp(&self.shown(b, now))
                    }
                })
                .then_with(|| a.name.cmp(&b.name))
        });
        agents.into_iter().map(|(_, agent)| agent).collect()
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

    /// The status the cards show and sort by: `Paused` in place of what a paused agent was
    /// doing, unless it failed or exited. Everything else (the Kanban, Attention) reads
    /// [`Self::status`], the state as corral gives it.
    pub fn shown(&self, a: &Agent, now: f64) -> Status {
        match self.status(a, now) {
            status @ (Status::Error | Status::Exited) => status,
            _ if a.paused => Status::Paused,
            status => status,
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
    /// Frozen by `corral pause`: only the cards show it (see [`Panel::shown`]).
    Paused,
    Exited,
}

pub fn group(name: &str) -> &str {
    name.find('/').map(|i| &name[..=i]).unwrap_or("")
}
