//! The saved layout: tabs, their splits and what each pane shows, in paddock's own state file, so
//! the next start can open the same. Versioned and checked on reading, as Saddle's
//! `src/layout_state.rs` (commit `df1c727`) is; a file that cannot be read is kept, not
//! overwritten. Terminal output is not saved and commands are not replayed.
use crate::layout::{Axis, Node, PaneId, Shown, Workspace};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::SystemTime,
};

const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    Empty,
    Shell {
        cwd: String,
    },
    Agent {
        name: String,
        cwd: Option<String>,
        instance: Option<String>,
    },
}

/// A split tree over the tab's panes, by their place in `SavedTab::panes`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SavedNode {
    Pane {
        pane: usize,
    },
    Split {
        /// `row`: side by side; `column`: one above the other.
        axis: String,
        first: Box<SavedNode>,
        second: Box<SavedNode>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedTab {
    pub tree: SavedNode,
    /// The active pane, by its place in `panes`.
    pub active: usize,
    pub panes: Vec<Content>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub active_tab: usize,
    pub tabs: Vec<SavedTab>,
}

impl SavedNode {
    fn leaves(&self, out: &mut Vec<usize>) -> Result<()> {
        match self {
            SavedNode::Pane { pane } => out.push(*pane),
            SavedNode::Split {
                axis,
                first,
                second,
            } => {
                ensure!(axis == "row" || axis == "column", "invalid split");
                first.leaves(out)?;
                second.leaves(out)?;
            }
        }
        Ok(())
    }
}

impl Layout {
    /// Every rule the file must keep before anything is opened from it.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == VERSION,
            "unsupported layout version {}",
            self.version
        );
        ensure!(!self.tabs.is_empty(), "layout has no tabs");
        ensure!(self.active_tab < self.tabs.len(), "active tab missing");
        for tab in &self.tabs {
            ensure!(!tab.panes.is_empty(), "tab has no panes");
            ensure!(tab.active < tab.panes.len(), "active pane missing");
            let mut leaves = Vec::new();
            tab.tree.leaves(&mut leaves)?;
            let unique: HashSet<usize> = leaves.iter().copied().collect();
            ensure!(
                leaves.len() == tab.panes.len()
                    && unique.len() == leaves.len()
                    && leaves.iter().all(|&pane| pane < tab.panes.len()),
                "invalid split tree"
            );
            for content in &tab.panes {
                let cwd = match content {
                    Content::Empty => None,
                    Content::Shell { cwd } => Some(cwd.as_str()),
                    Content::Agent { name, cwd, .. } => {
                        ensure!(
                            !name.is_empty()
                                && !name.starts_with('-')
                                && !name.chars().any(char::is_whitespace),
                            "invalid agent name"
                        );
                        cwd.as_deref()
                    }
                };
                if let Some(cwd) = cwd {
                    ensure!(
                        std::path::Path::new(cwd).is_absolute() && !cwd.contains('\0'),
                        "invalid working directory"
                    );
                }
            }
        }
        Ok(())
    }

    /// The workspace's layout; `content` says what a pane shows in full.
    pub fn of(workspace: &Workspace, content: impl Fn(PaneId) -> Content) -> Layout {
        fn node(tree: &Node, places: &HashMap<PaneId, usize>) -> SavedNode {
            match tree {
                Node::Pane(pane) => SavedNode::Pane { pane: places[pane] },
                Node::Split {
                    axis,
                    first,
                    second,
                } => SavedNode::Split {
                    axis: match axis {
                        Axis::Row => "row",
                        Axis::Column => "column",
                    }
                    .into(),
                    first: Box::new(node(first, places)),
                    second: Box::new(node(second, places)),
                },
            }
        }
        let tabs = workspace
            .tabs
            .iter()
            .map(|tab| {
                let panes = tab.panes();
                let places: HashMap<PaneId, usize> =
                    panes.iter().enumerate().map(|(i, p)| (*p, i)).collect();
                SavedTab {
                    tree: node(&tab.root, &places),
                    active: places[&tab.active],
                    panes: panes.iter().map(|pane| content(*pane)).collect(),
                }
            })
            .collect();
        Layout {
            version: VERSION,
            active_tab: workspace.active_tab,
            tabs,
        }
    }

    /// A workspace laid out as saved, and what each of its panes is to show. Call `validate`
    /// first.
    pub fn workspace(&self) -> (Workspace, Vec<(PaneId, Content)>) {
        fn node(saved: &SavedNode, ids: &[PaneId]) -> Node {
            match saved {
                SavedNode::Pane { pane } => Node::Pane(ids[*pane]),
                SavedNode::Split {
                    axis,
                    first,
                    second,
                } => Node::Split {
                    axis: if axis == "row" {
                        Axis::Row
                    } else {
                        Axis::Column
                    },
                    first: Box::new(node(first, ids)),
                    second: Box::new(node(second, ids)),
                },
            }
        }
        let mut next: PaneId = 0;
        let mut tabs = Vec::new();
        let mut contents = Vec::new();
        for tab in &self.tabs {
            let ids: Vec<PaneId> = tab
                .panes
                .iter()
                .map(|_| {
                    next += 1;
                    next
                })
                .collect();
            for (id, content) in ids.iter().zip(&tab.panes) {
                contents.push((*id, content.clone()));
            }
            tabs.push((node(&tab.tree, &ids), ids[tab.active]));
        }
        let shown = contents
            .iter()
            .map(|(id, content)| {
                let shown = match content {
                    Content::Empty => Shown::Empty,
                    Content::Shell { .. } => Shown::Shell,
                    Content::Agent { name, .. } => Shown::Agent(name.clone()),
                };
                (*id, shown)
            })
            .collect();
        (
            Workspace::from_parts(tabs, self.active_tab, shown, next),
            contents,
        )
    }
}

/// `$XDG_STATE_HOME/paddock/layout.json`, else `~/.local/state/paddock/layout.json`.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|state| state.join("paddock/layout.json"))
}

/// Where the layout is saved, and how the last restore and save went.
pub struct Store {
    path: Option<PathBuf>,
    /// The file could not be restored: it is kept as it is, so nothing is saved over it.
    protected: bool,
    last: Option<Vec<u8>>,
    /// The startup restore: whether a saved layout was used, or why not.
    pub restored: Option<(SystemTime, Result<bool, String>)>,
    /// The latest save.
    pub saved: Option<(SystemTime, Result<(), String>)>,
}

impl Store {
    /// Reads the saved layout, if there is a good one.
    pub fn open(path: Option<PathBuf>) -> (Store, Option<Layout>) {
        let loaded = (|| -> Result<Option<Layout>> {
            let path = path.as_ref().context("HOME is unavailable")?;
            let bytes = match fs::read(path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error.into()),
            };
            let layout: Layout = serde_json::from_slice(&bytes)?;
            layout.validate()?;
            Ok(Some(layout))
        })();
        let now = SystemTime::now();
        let mut store = Store {
            path,
            protected: false,
            last: None,
            restored: None,
            saved: None,
        };
        match loaded {
            Ok(layout) => {
                store.restored = Some((now, Ok(layout.is_some())));
                (store, layout)
            }
            Err(error) => {
                store.protected = true;
                store.restored = Some((now, Err(format!("{error:#}"))));
                (store, None)
            }
        }
    }

    /// paddock was started to open something in particular: the saved layout is neither used
    /// nor saved over this time.
    pub fn skip(path: Option<PathBuf>) -> Store {
        Store {
            path,
            protected: true,
            last: None,
            restored: Some((SystemTime::now(), Ok(false))),
            saved: None,
        }
    }

    /// Why the saved layout was not used, when it could not be read.
    pub fn problem(&self) -> Option<String> {
        match &self.restored {
            Some((_, Err(error))) => Some(format!(
                "The saved layout could not be restored and is kept as it was: {error}"
            )),
            _ => None,
        }
    }

    /// Writes `layout` unless it is what was written last, or the file is being kept.
    pub fn save(&mut self, layout: &Layout) {
        if self.protected {
            return;
        }
        let bytes = serde_json::to_vec_pretty(layout).expect("a layout serializes");
        if self.last.as_ref() == Some(&bytes) {
            return;
        }
        let result = (|| -> Result<()> {
            let path = self.path.as_ref().context("HOME is unavailable")?;
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let temporary = path.with_extension("json.saving");
            fs::write(&temporary, &bytes)?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_ok() {
            self.last = Some(bytes);
        }
        self.saved = Some((SystemTime::now(), result.map_err(|e| format!("{e:#}"))));
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Direction;

    fn sample() -> (Workspace, Layout) {
        let (mut w, first) = Workspace::new(Shown::Shell);
        w.split(Direction::Right, Shown::Agent("p/main".into()));
        w.split(Direction::Down, Shown::Empty);
        w.focus(first);
        w.new_tab(Shown::Agent("p/review".into()));
        w.select_tab(0);
        let content = |pane: PaneId| match w.shown(pane) {
            Shown::Empty => Content::Empty,
            Shown::Shell => Content::Shell {
                cwd: format!("/work/{pane}"),
            },
            Shown::Agent(name) => Content::Agent {
                name: name.clone(),
                cwd: Some("/work".into()),
                instance: Some("0123456789ab".into()),
            },
        };
        let layout = Layout::of(&w, content);
        (w, layout)
    }

    #[test]
    fn a_layout_round_trips_through_the_file_format() {
        let (w, layout) = sample();
        layout.validate().unwrap();
        let text = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&text).unwrap();
        assert_eq!(back, layout);
        let (restored, contents) = back.workspace();
        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.active_tab, 0);
        assert_eq!(restored.tab().panes().len(), 3);
        assert_eq!(restored.shown(restored.active_pane()), &Shown::Shell);
        assert_eq!(restored.tabs[1].panes().len(), 1);
        assert_eq!(restored.agents(), w.agents());
        assert_eq!(contents.len(), 4);
        // The restored workspace saves to the same layout.
        let again = Layout::of(&restored, |pane| {
            contents
                .iter()
                .find(|(id, _)| *id == pane)
                .unwrap()
                .1
                .clone()
        });
        assert_eq!(again, layout);
        // New panes do not reuse restored IDs.
        let mut restored = restored;
        let new = restored.new_tab(Shown::Empty);
        assert!(contents.iter().all(|(id, _)| *id != new));
    }

    #[test]
    fn broken_layouts_are_refused() {
        let (_, good) = sample();
        let mut cases = Vec::new();
        let mut bad = good.clone();
        bad.version = 9;
        cases.push(bad);
        let mut bad = good.clone();
        bad.tabs.clear();
        cases.push(bad);
        let mut bad = good.clone();
        bad.active_tab = 5;
        cases.push(bad);
        let mut bad = good.clone();
        bad.tabs[0].panes.pop();
        cases.push(bad);
        let mut bad = good.clone();
        bad.tabs[1].tree = SavedNode::Split {
            axis: "row".into(),
            first: Box::new(SavedNode::Pane { pane: 0 }),
            second: Box::new(SavedNode::Pane { pane: 0 }),
        };
        cases.push(bad);
        let mut bad = good.clone();
        bad.tabs[0].panes[0] = Content::Shell {
            cwd: "relative".into(),
        };
        cases.push(bad);
        let mut bad = good.clone();
        bad.tabs[1].panes[0] = Content::Agent {
            name: "two words".into(),
            cwd: None,
            instance: None,
        };
        cases.push(bad);
        for case in cases {
            assert!(case.validate().is_err(), "{case:?}");
        }
    }
}
