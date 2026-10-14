//! Workspaces (DESIGN §13 P5-89): a layer over the sidebar's groups. A workspace is a name the
//! user gave to a set of members: whole repositories, by their main worktree's path, whose agents
//! all come, those started there later too; and single agents, by full name. A repository is in
//! one workspace at most, and so is an agent's name: adding either to another moves it. The list
//! is kept on this machine only, in paddock's state directory beside the layout, as the Kanban
//! tab's repositories are (`kanban_repos.rs`); nothing is written into a repository, and no
//! agent's name or labels change. `agents.rs` asks it where an agent is, `card.rs` lays the
//! sidebar's lines out by it, and `sidebar.rs` draws them and the panel that edits it.
use crate::{kanban_repos, layout_state};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The list of workspaces, beside the layout: `$XDG_STATE_HOME/paddock/workspaces.json`.
pub fn path() -> Option<PathBuf> {
    layout_state::default_path().map(|path| path.with_file_name("workspaces.json"))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    /// Whole repositories, by their main worktree, in the order they were added.
    #[serde(default)]
    pub repos: Vec<PathBuf>,
    /// Agents recorded by themselves, by full name: kept while they are not running, so one
    /// started again under its name is here again.
    #[serde(default)]
    pub agents: Vec<String>,
}

impl Workspace {
    /// Where its `New Agent…` starts (P5-93): its first whole repository by name.
    pub fn first_repo(&self) -> Option<&Path> {
        (self.repos.iter())
            .min_by_key(|repo| kanban_repos::name(repo))
            .map(PathBuf::as_path)
    }
}

/// Why a name will not do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    /// Another workspace has it, whatever the case.
    Taken,
}

impl NameError {
    /// What the panel says under the field.
    pub fn text(self) -> &'static str {
        match self {
            NameError::Empty => "Name the workspace to create it.",
            NameError::Taken => "Another workspace has this name.",
        }
    }
}

/// The workspaces, by name whatever the case.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Workspaces {
    list: Vec<Workspace>,
}

impl Workspaces {
    /// The workspaces listed at `path`; none when there is no list or it cannot be read.
    pub fn load(path: &Path) -> Self {
        let listed = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<Workspace>>(&bytes).ok())
            .unwrap_or_default();
        // A list edited by hand may break the rules: a name is kept once, the first time.
        let mut all = Workspaces::default();
        for workspace in listed {
            if let Ok(name) = all.name(&workspace.name, None) {
                all.list.push(Workspace { name, ..workspace });
            }
        }
        all.sort();
        all
    }

    /// Writes the list to `path`: names, repositories' paths and agents' names, nothing else.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.list).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.saving");
        fs::write(&temporary, bytes)?;
        fs::rename(&temporary, path)
    }

    fn sort(&mut self) {
        self.list
            .sort_by_key(|workspace| workspace.name.to_lowercase());
    }

    /// By name.
    pub fn list(&self) -> &[Workspace] {
        &self.list
    }

    pub fn get(&self, name: &str) -> Option<&Workspace> {
        self.list.iter().find(|workspace| workspace.name == name)
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Workspace> {
        self.list
            .iter_mut()
            .find(|workspace| workspace.name == name)
    }

    /// `typed` as a workspace would keep it, without the blanks around it; an error when it is
    /// empty or another workspace has it, whatever the case. `renaming` is the workspace being
    /// named, whose own name does not count.
    pub fn name(&self, typed: &str, renaming: Option<&str>) -> Result<String, NameError> {
        let name = typed.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        let taken = self.list.iter().any(|workspace| {
            Some(workspace.name.as_str()) != renaming
                && workspace.name.to_lowercase() == name.to_lowercase()
        });
        if taken {
            return Err(NameError::Taken);
        }
        Ok(name.to_owned())
    }

    /// A new workspace with no members; its name as kept.
    pub fn create(&mut self, name: &str) -> Result<String, NameError> {
        let name = self.name(name, None)?;
        self.list.push(Workspace {
            name: name.clone(),
            ..Workspace::default()
        });
        self.sort();
        Ok(name)
    }

    /// A new name for `from`, its members staying; the name as kept.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<String, NameError> {
        let name = self.name(to, Some(from))?;
        if let Some(workspace) = self.get_mut(from) {
            workspace.name.clone_from(&name);
        }
        self.sort();
        Ok(name)
    }

    /// The workspace is gone; its members are in none.
    pub fn remove(&mut self, name: &str) {
        self.list.retain(|workspace| workspace.name != name);
    }

    /// The whole repository is in `workspace`, and in no other; left where it is when there is
    /// no such workspace.
    pub fn add_repo(&mut self, workspace: &str, repo: &Path) {
        if self.get(workspace).is_none() {
            return;
        }
        self.remove_repo(repo);
        if let Some(workspace) = self.get_mut(workspace) {
            workspace.repos.push(repo.to_owned());
        }
    }

    /// The agent is recorded by itself in `workspace`, and in no other; left where it is when
    /// there is no such workspace.
    pub fn add_agent(&mut self, workspace: &str, agent: &str) {
        if self.get(workspace).is_none() {
            return;
        }
        self.remove_agent(agent);
        if let Some(workspace) = self.get_mut(workspace) {
            workspace.agents.push(agent.to_owned());
        }
    }

    pub fn remove_repo(&mut self, repo: &Path) {
        for workspace in &mut self.list {
            workspace.repos.retain(|member| member != repo);
        }
    }

    /// The agent is recorded by itself in none: it is where its repository is.
    pub fn remove_agent(&mut self, agent: &str) {
        for workspace in &mut self.list {
            workspace.agents.retain(|member| member != agent);
        }
    }

    /// `workspace` has exactly these members, taken from wherever they were.
    pub fn set_members(&mut self, workspace: &str, repos: &[PathBuf], agents: &[String]) {
        if let Some(workspace) = self.get_mut(workspace) {
            workspace.repos.clear();
            workspace.agents.clear();
        }
        for repo in repos {
            self.add_repo(workspace, repo);
        }
        for agent in agents {
            self.add_agent(workspace, agent);
        }
    }

    /// The workspace the whole repository is in.
    pub fn of_repo(&self, repo: &Path) -> Option<&str> {
        let has = |workspace: &&Workspace| workspace.repos.iter().any(|member| member == repo);
        self.list.iter().find(has).map(|w| w.name.as_str())
    }

    /// The workspace the agent is recorded in by itself.
    pub fn of_agent(&self, agent: &str) -> Option<&str> {
        let has = |workspace: &&Workspace| workspace.agents.iter().any(|member| member == agent);
        self.list.iter().find(has).map(|w| w.name.as_str())
    }

    /// The agents recorded by themselves in a workspace other than `workspace`.
    pub fn taken(&self, workspace: Option<&str>) -> Vec<String> {
        self.list
            .iter()
            .filter(|other| Some(other.name.as_str()) != workspace)
            .flat_map(|other| other.agents.iter().cloned())
            .collect()
    }

    /// The workspace of an agent: the one it is recorded in by itself; else the one `repo`, the
    /// main worktree of the repository it works in, is in. With no repository read, it goes by
    /// `prefix`, its name's: into the workspace with a repository of that name, when only one
    /// workspace has such a one, as groups of the same name are one group (P5-80).
    pub fn of(&self, agent: &str, repo: Option<&Path>, prefix: &str) -> Option<&str> {
        if let Some(own) = self.of_agent(agent) {
            return Some(own);
        }
        if let Some(repo) = repo {
            return self.of_repo(repo);
        }
        let mut named = self.list.iter().filter(|workspace| {
            let mut names = workspace.repos.iter().map(|repo| kanban_repos::name(repo));
            !prefix.is_empty() && names.any(|name| name == prefix)
        });
        let only = named.next()?;
        named.next().is_none().then_some(only.name.as_str())
    }

    /// An agent started from `workspace`'s own `New Agent…` (P5-93), in `repo`, the main
    /// worktree of the repository it works in (none outside Git): recorded there by itself
    /// unless it is there already, its repository left where it is. Whether it was recorded.
    pub fn join(&mut self, workspace: &str, agent: &str, repo: Option<&Path>) -> bool {
        if self.get(workspace).is_none() || self.of(agent, repo, "") == Some(workspace) {
            return false;
        }
        self.add_agent(workspace, agent);
        true
    }
}

/// A group as the sidebar lists it now, for the panel that picks members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// As `Panel::group` names it.
    pub name: String,
    /// The repositories its agents work in: none for a group that goes by its agents' prefix,
    /// which cannot be ticked whole.
    pub repos: Vec<PathBuf>,
    /// By full name.
    pub agents: Vec<String>,
}

/// What is ticked in the panel: whole repositories, and single agents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picks {
    pub repos: Vec<PathBuf>,
    pub agents: Vec<String>,
}

impl Picks {
    /// A workspace's members, as the panel opens for it.
    pub fn of(workspace: &Workspace) -> Self {
        Picks {
            repos: workspace.repos.clone(),
            agents: workspace.agents.clone(),
        }
    }

    pub fn repo(&self, repo: &Path) -> bool {
        self.repos.iter().any(|picked| picked == repo)
    }

    pub fn toggle_repo(&mut self, repo: &Path) {
        match self.repos.iter().position(|picked| picked == repo) {
            Some(at) => {
                self.repos.remove(at);
            }
            None => self.repos.push(repo.to_owned()),
        }
    }

    /// The group's whole repository is ticked: every agent in it comes, and any started later.
    pub fn whole(&self, group: &Listed) -> bool {
        !group.repos.is_empty() && group.repos.iter().all(|repo| self.repo(repo))
    }

    /// Whether `agent` of `group` is ticked: by itself, or with its whole repository. One of
    /// `taken`, those recorded by themselves in another workspace, does not come with its
    /// repository: it comes when ticked itself.
    pub fn agent(&self, group: &Listed, agent: &str, taken: &[String]) -> bool {
        let alone = self.agents.iter().any(|picked| picked == agent);
        alone || (self.whole(group) && !taken.iter().any(|other| other == agent))
    }

    /// How many of the group's agents are ticked.
    pub fn ticked(&self, group: &Listed, taken: &[String]) -> usize {
        let ticked = |agent: &&String| self.agent(group, agent, taken);
        group.agents.iter().filter(ticked).count()
    }

    /// The tick before the group's name: its whole repository on, or everything of it off.
    pub fn toggle_whole(&mut self, group: &Listed, taken: &[String]) {
        let whole = self.whole(group);
        self.repos.retain(|repo| !group.repos.contains(repo));
        if whole {
            self.agents.retain(|agent| !group.agents.contains(agent));
        } else {
            self.repos.extend(group.repos.iter().cloned());
            // The repository brings them: only those it does not bring stay ticked singly.
            self.agents
                .retain(|agent| !group.agents.contains(agent) || taken.contains(agent));
        }
    }

    /// The tick before an agent's name. Taking one off a whole repository leaves the others
    /// ticked one by one.
    pub fn toggle_agent(&mut self, group: &Listed, agent: &str, taken: &[String]) {
        if let Some(at) = self.agents.iter().position(|picked| picked == agent) {
            self.agents.remove(at);
        } else if self.whole(group) && !taken.iter().any(|other| other == agent) {
            let others: Vec<String> = group
                .agents
                .iter()
                .filter(|other| *other != agent && self.agent(group, other, taken))
                .filter(|other| !self.agents.contains(other))
                .cloned()
                .collect();
            self.repos.retain(|repo| !group.repos.contains(repo));
            self.agents.extend(others);
        } else {
            self.agents.push(agent.to_owned());
        }
    }

    /// What the panel says it has ticked: `4 repositories · 1 agent`; nothing with nothing.
    pub fn summary(&self) -> String {
        let count = |count: usize, one: &str, many: &str| match count {
            0 => None,
            1 => Some(format!("1 {one}")),
            _ => Some(format!("{count} {many}")),
        };
        let parts = [
            count(self.repos.len(), "repository", "repositories"),
            count(self.agents.len(), "agent", "agents"),
        ];
        parts.into_iter().flatten().collect::<Vec<_>>().join(" · ")
    }
}

/// How many agents, in words: `No agents`, `1 agent`, `3 agents`.
pub fn agents(count: usize) -> String {
    match count {
        0 => "No agents".to_owned(),
        1 => "1 agent".to_owned(),
        _ => format!("{count} agents"),
    }
}
