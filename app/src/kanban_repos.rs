//! Which repository the Kanban tab shows (DESIGN §13 P5-85): the focused pane's, or one picked
//! from the menu under the repository's name, which the board then stays on wherever the focus
//! goes. The repositories picked are kept in a list on this machine, in paddock's state directory
//! beside the layout, until the user removes them; only their paths are written, as the activity
//! grid's list is (`activity::save_repos`). Where the board stays is not kept: a new window
//! follows the focused pane. A repository here is its main worktree, as
//! `activity::main_repository` gives it. `kanban_view.rs` draws it.
use crate::{activity, layout_state};
use std::path::{Path, PathBuf};

/// The list of kept repositories, beside the layout: `$XDG_STATE_HOME/paddock/kanban-repos.json`.
pub fn kept_path() -> Option<PathBuf> {
    layout_state::default_path().map(|path| path.with_file_name("kanban-repos.json"))
}

/// A repository's name: its directory's.
pub fn name(repo: &Path) -> String {
    repo.file_name().map_or_else(
        || repo.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Which repository the board shows, and the ones kept.
pub struct Looking {
    /// Where the list is kept.
    path: Option<PathBuf>,
    kept: Vec<PathBuf>,
    chosen: Option<PathBuf>,
}

impl Looking {
    /// Following the focused pane, with the list kept at `path`; `None` keeps it only in memory.
    pub fn load(path: Option<PathBuf>) -> Self {
        let mut looking = Looking {
            path,
            kept: Vec::new(),
            chosen: None,
        };
        looking.refresh();
        looking
    }

    /// The kept repositories, in the order they were picked.
    pub fn kept(&self) -> &[PathBuf] {
        &self.kept
    }

    /// The repository the board stays on; `None` while it follows the focused pane.
    pub fn chosen(&self) -> Option<&Path> {
        self.chosen.as_deref()
    }

    /// Reads the list again: another window may have changed it.
    pub fn refresh(&mut self) {
        if let Some(path) = &self.path {
            self.kept = activity::load_repos(path);
        }
    }

    fn save(&self) {
        if let Some(path) = &self.path {
            let _ = activity::save_repos(path, &self.kept);
        }
    }

    /// The board stays on `repo`, which is kept from now on.
    pub fn choose(&mut self, repo: PathBuf) {
        self.refresh();
        if !self.kept.contains(&repo) {
            self.kept.push(repo.clone());
            self.save();
        }
        self.chosen = Some(repo);
    }

    /// Back to the focused pane's repository.
    pub fn follow(&mut self) {
        self.chosen = None;
    }

    /// `repo` is kept no longer; the board goes back to the focused pane when it stayed on it.
    pub fn remove(&mut self, repo: &Path) {
        self.refresh();
        let before = self.kept.len();
        self.kept.retain(|kept| kept != repo);
        if self.kept.len() != before {
            self.save();
        }
        if self.chosen.as_deref() == Some(repo) {
            self.chosen = None;
        }
    }

    /// The directory the board reads, `focus` being the focused pane's: `None` with no pane in
    /// focus, `Some(None)` for a pane with no directory to read.
    pub fn dir(&self, focus: &Option<Option<String>>) -> Option<Option<String>> {
        match &self.chosen {
            Some(repo) => Some(Some(repo.to_string_lossy().into_owned())),
            None => focus.clone(),
        }
    }
}

/// A line of the menu under the repository's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// `Follow focused pane`, with the name of that pane's repository when it is in one; `on`
    /// while the board follows it.
    Follow { name: Option<String>, on: bool },
    /// A section's name, over its repositories.
    Heading(&'static str),
    /// A repository, open now or (`kept`) picked before; `on` when the board stays on it.
    Repo { path: PathBuf, kept: bool, on: bool },
    /// `Choose folder…`
    Choose,
}

pub const OPEN_NOW: &str = "Open now";
pub const KEPT: &str = "Kept";

/// The menu's lines: following the focused pane, whose repository is `focus`; the repositories
/// open in paddock now (`open`, one for each agent and shell, so with repeats), each once and
/// without the kept ones; the kept ones; and choosing a folder. Both lists go by name, and a list
/// with nothing in it is left out with its heading.
pub fn menu(focus: Option<&Path>, open: &[PathBuf], looking: &Looking) -> Vec<Row> {
    let by_name = |repos: &mut Vec<&PathBuf>| {
        repos.sort_by(|a, b| name(a).cmp(&name(b)).then_with(|| a.cmp(b)));
        repos.dedup();
    };
    let mut open: Vec<&PathBuf> = open
        .iter()
        .filter(|repo| !looking.kept.contains(repo))
        .collect();
    by_name(&mut open);
    let mut kept: Vec<&PathBuf> = looking.kept.iter().collect();
    by_name(&mut kept);
    let mut rows = vec![Row::Follow {
        name: focus.map(name),
        on: looking.chosen.is_none(),
    }];
    for (heading, repos, kept) in [(OPEN_NOW, open, false), (KEPT, kept, true)] {
        if repos.is_empty() {
            continue;
        }
        rows.push(Row::Heading(heading));
        rows.extend(repos.into_iter().map(|repo| Row::Repo {
            path: repo.clone(),
            kept,
            on: looking.chosen.as_ref() == Some(repo),
        }));
    }
    rows.push(Row::Choose);
    rows
}

/// What the button that goes back to following the focused pane says while the board stays on
/// `chosen`: the name of that pane's repository, `focus`, unless there is none (no pane in focus,
/// or one outside any repository) or it is the one the board stays on.
pub fn back_label(focus: Option<&Path>, chosen: &Path) -> String {
    match focus {
        Some(repo) if repo != chosen => format!("Back to {}", name(repo)),
        _ => "Follow focus".into(),
    }
}
