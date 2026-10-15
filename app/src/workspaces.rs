//! Workspaces (DESIGN §13 P5-89): a layer over the sidebar's groups. A workspace is a name the
//! user gave to a set of whole repositories, by their main worktree's path, whose agents all come,
//! those started there later too: the agents of a repository are all where it is (P5-105, which
//! took out single agents). A repository is in one workspace at most: adding it to another moves
//! it. The list is kept on this machine only, in paddock's state directory beside the layout, as
//! the Kanban tab's repositories are (`kanban_repos.rs`); nothing is written into a repository,
//! and no agent's name or labels change. `agents.rs` asks it where an agent is, `card.rs` lays the
//! sidebar's lines out by it, and `sidebar.rs` draws them and the panel that edits it.
use crate::{kanban_repos, layout_state};
use serde::{Deserialize, Serialize, Serializer};
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
    /// Agents an older paddock recorded by themselves, by full name (P5-89): read, to give way to
    /// their repositories ([`Workspaces::adopt`]), and written empty (P5-105). They put no agent
    /// in the workspace.
    #[serde(default, serialize_with = "written_empty")]
    pub agents: Vec<String>,
}

/// The agents' names as a workspace is kept now: none.
fn written_empty<S: Serializer>(_: &[String], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(std::iter::empty::<&str>())
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

    /// Writes the list to `path`: names and repositories' paths, nothing else; an older paddock's
    /// agents' names are left out, as they no longer do anything.
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

    pub fn remove_repo(&mut self, repo: &Path) {
        for workspace in &mut self.list {
            workspace.repos.retain(|member| member != repo);
        }
    }

    /// `workspace`'s repositories as the panel's `picks` leaves them: those ticked since it opened
    /// come, from wherever they are; those unticked since go, if still there. Any other stays as it
    /// is now, so one the workspace gained meanwhile, as an older paddock's names gave way (P5-105)
    /// or another window added it, is not taken out.
    pub fn set_picked(&mut self, workspace: &str, picks: &Picks) {
        for repo in &picks.repos {
            if !picks.base.contains(repo) && self.of_repo(repo) != Some(workspace) {
                self.add_repo(workspace, repo);
            }
        }
        for repo in &picks.base {
            if !picks.repo(repo) && self.of_repo(repo) == Some(workspace) {
                self.remove_repo(repo);
            }
        }
    }

    /// The workspace the whole repository is in.
    pub fn of_repo(&self, repo: &Path) -> Option<&str> {
        let has = |workspace: &&Workspace| workspace.repos.iter().any(|member| member == repo);
        self.list.iter().find(has).map(|w| w.name.as_str())
    }

    /// The workspace of an agent: the one `repo`, the main worktree of the repository it works
    /// in, is in, whatever its name (P5-105). With no repository read, it goes by `prefix`, its
    /// name's: into the workspace with a repository of that name, when only one workspace has such
    /// a one, as groups of the same name are one group (P5-80).
    pub fn of(&self, repo: Option<&Path>, prefix: &str) -> Option<&str> {
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
    /// worktree of the repository it works in: the whole repository joins the workspace when it
    /// is in none (P5-105). One in another workspace stays there, the agent with it; outside Git
    /// nothing is recorded. Whether it joined.
    pub fn join(&mut self, workspace: &str, repo: Option<&Path>) -> bool {
        let Some(repo) = repo else {
            return false;
        };
        if self.get(workspace).is_none() || self.of_repo(repo).is_some() {
            return false;
        }
        self.add_repo(workspace, repo);
        true
    }

    /// Whether an older paddock left agents recorded by name in the list (P5-105).
    pub fn named(&self) -> bool {
        self.list
            .iter()
            .any(|workspace| !workspace.agents.is_empty())
    }

    /// Each agent an older paddock recorded by name whose repository `repo` gives, it running and
    /// Git having told, gives way to that repository (P5-105): the repository joins the workspace
    /// the name was in, unless one has it already, and the name goes. The other names do nothing,
    /// and go when the list is next kept. Whether any gave way.
    pub fn adopt(&mut self, repo: impl Fn(&str) -> Option<PathBuf>) -> bool {
        let mut adopted = false;
        for at in 0..self.list.len() {
            let mut kept = Vec::new();
            for agent in std::mem::take(&mut self.list[at].agents) {
                let Some(repo) = repo(&agent) else {
                    kept.push(agent);
                    continue;
                };
                if self.of_repo(&repo).is_none() {
                    self.list[at].repos.push(repo);
                }
                adopted = true;
            }
            self.list[at].agents = kept;
        }
        adopted
    }
}

/// A repository agents work in now, by its main worktree, for the panel that picks a workspace's
/// repositories: one row each, two of the same name apart (P5-105).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub repo: PathBuf,
    /// How many agents work in it.
    pub agents: usize,
}

/// The repositories ticked in the panel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picks {
    pub repos: Vec<PathBuf>,
    /// The workspace's own as the panel has them from it: what is ticked counts against them
    /// ([`Workspaces::set_picked`]). None for a new workspace.
    pub base: Vec<PathBuf>,
}

impl Picks {
    /// A workspace's repositories, as the panel opens for it.
    pub fn of(workspace: &Workspace) -> Self {
        Picks {
            repos: workspace.repos.clone(),
            base: workspace.repos.clone(),
        }
    }

    /// Repositories `workspace` gained since the panel opened for it, as an older paddock's names
    /// gave way (P5-105) or another window added them, come ticked, as though there as it opened.
    pub fn follow(&mut self, workspace: &Workspace) {
        for repo in &workspace.repos {
            if !self.base.contains(repo) {
                self.base.push(repo.clone());
                if !self.repo(repo) {
                    self.repos.push(repo.clone());
                }
            }
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

    /// What the panel says it has ticked: `4 repositories`; nothing with nothing.
    pub fn summary(&self) -> String {
        match self.repos.len() {
            0 => String::new(),
            1 => "1 repository".to_owned(),
            count => format!("{count} repositories"),
        }
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
