//! The saved layout: tabs, their splits and what each pane shows, in paddock's own state file, so
//! the next start can open the same. Versioned and checked on reading, as Saddle's
//! `src/layout_state.rs` (commit `df1c727`) is; a file that cannot be read is kept, not
//! overwritten. Terminal output is not saved and commands are not replayed.
use crate::{
    layout::{self, Axis, Node, PaneId, Shown, Workspace},
    right_panel,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::SystemTime,
};

const VERSION: u32 = 1;

/// The main window's normal size in logical pixels; no position or fullscreen state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: f32,
    pub height: f32,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 800.0,
        }
    }
}

impl WindowSize {
    /// Fits a restored size to the current display's usable area and the window's minimum.
    pub fn for_startup(self, available: Self, minimum: Self) -> Self {
        Self {
            width: self.width.max(minimum.width).min(available.width),
            height: self.height.max(minimum.height).min(available.height),
        }
    }
}

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
        /// The first part's share; files from before it was saved split in half.
        #[serde(default = "even")]
        ratio: f32,
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
    /// The sidebar is collapsed to its narrow strip; files from before it was saved open it
    /// expanded.
    #[serde(default)]
    pub sidebar_collapsed: bool,
    /// The right sidebar: open or not, its width and its tab; files from before it was saved open
    /// it closed, at the default width, on Changes.
    #[serde(default)]
    pub right_sidebar: right_panel::Saved,
    /// The activity panel over the sidebar's footer is folded to one line; files from before it
    /// was saved open it unfolded.
    #[serde(default)]
    pub activity_folded: bool,
    #[serde(default)]
    pub window_size: WindowSize,
}

fn even() -> f32 {
    layout::EVEN
}

impl SavedNode {
    fn leaves(&self, out: &mut Vec<usize>) -> Result<()> {
        match self {
            SavedNode::Pane { pane } => out.push(*pane),
            SavedNode::Split {
                axis,
                ratio,
                first,
                second,
            } => {
                ensure!(axis == "row" || axis == "column", "invalid split");
                ensure!(*ratio > 0.0 && *ratio < 1.0, "invalid split ratio");
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

    /// The workspace's layout, with the sidebar expanded and the right one closed; `content` says
    /// what a pane shows in full.
    pub fn of(workspace: &Workspace, content: impl Fn(PaneId) -> Content) -> Layout {
        fn node(tree: &Node, places: &HashMap<PaneId, usize>) -> SavedNode {
            match tree {
                Node::Pane(pane) => SavedNode::Pane { pane: places[pane] },
                Node::Split {
                    axis,
                    ratio,
                    first,
                    second,
                } => SavedNode::Split {
                    axis: match axis {
                        Axis::Row => "row",
                        Axis::Column => "column",
                    }
                    .into(),
                    ratio: *ratio,
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
            sidebar_collapsed: false,
            right_sidebar: right_panel::Saved::default(),
            activity_folded: false,
            window_size: WindowSize::default(),
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
                    ratio,
                    first,
                    second,
                } => Node::Split {
                    axis: if axis == "row" {
                        Axis::Row
                    } else {
                        Axis::Column
                    },
                    ratio: *ratio,
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

    /// Saving is off: the file could not be restored, or paddock was started for one thing.
    pub fn protected(&self) -> bool {
        self.protected
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
    fn split_ratios_are_saved_and_older_files_split_in_half() {
        let (mut w, _) = sample();
        let tab = &w.tabs[0];
        let (outer, inner) = (tab.panes()[1], tab.panes()[2]);
        w.set_ratio(outer, 0.3);
        w.set_ratio(inner, 0.65);
        let layout = Layout::of(&w, |_| Content::Empty);
        let dir = std::env::temp_dir().join(format!("paddock-ratio-test-{}", std::process::id()));
        let path = dir.join("layout.json");
        let (mut store, _) = Store::open(Some(path.clone()));
        store.save(&layout);
        let (store, back) = Store::open(Some(path.clone()));
        assert!(store.problem().is_none());
        let (restored, _) = back.unwrap().workspace();
        assert_eq!(restored.tabs[0].root, w.tabs[0].root);

        // A file written before splits kept their share has none: every split is even.
        let mut old = serde_json::to_value(&layout).unwrap();
        old["tabs"][0]["tree"]
            .as_object_mut()
            .unwrap()
            .remove("ratio");
        old["tabs"][0]["tree"]["second"]
            .as_object_mut()
            .unwrap()
            .remove("ratio");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path));
        assert!(store.problem().is_none());
        let (restored, _) = back.unwrap().workspace();
        let Node::Split { ratio, second, .. } = &restored.tabs[0].root else {
            panic!("a split")
        };
        assert_eq!(*ratio, layout::EVEN);
        assert!(matches!(**second, Node::Split { ratio, .. } if ratio == layout::EVEN));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn window_size_round_trips_through_the_layout_store() {
        let (_, layout) = sample();
        let mut value = serde_json::to_value(layout).unwrap();
        value["window_size"] = serde_json::json!({ "width": 1100.0, "height": 720.0 });
        let layout: Layout = serde_json::from_value(value).unwrap();
        let dir = std::env::temp_dir().join(format!("paddock-window-size-{}", std::process::id()));
        let path = dir.join("layout.json");
        let (mut store, _) = Store::open(Some(path.clone()));
        store.save(&layout);
        let (store, back) = Store::open(Some(path));
        assert!(store.problem().is_none());
        let back = serde_json::to_value(back.unwrap()).unwrap();
        assert_eq!(
            back["window_size"],
            serde_json::json!({ "width": 1100.0, "height": 720.0 })
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn old_layouts_default_to_the_original_window_size() {
        let (_, layout) = sample();
        let mut old = serde_json::to_value(layout).unwrap();
        old.as_object_mut().unwrap().remove("window_size");
        let dir =
            std::env::temp_dir().join(format!("paddock-old-window-size-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layout.json");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path));
        assert!(store.problem().is_none());
        assert_eq!(
            back.unwrap().window_size,
            WindowSize {
                width: 1280.0,
                height: 800.0,
            }
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn startup_window_size_fits_the_usable_screen() {
        let available = WindowSize {
            width: 1440.0,
            height: 850.0,
        };
        let minimum = WindowSize {
            width: 300.0,
            height: 200.0,
        };
        assert_eq!(
            WindowSize {
                width: 2200.0,
                height: 1400.0,
            }
            .for_startup(available, minimum),
            available
        );
        let normal = WindowSize {
            width: 1100.0,
            height: 720.0,
        };
        assert_eq!(normal.for_startup(available, minimum), normal);
    }

    #[test]
    fn startup_window_size_respects_the_native_minimum() {
        assert_eq!(
            WindowSize {
                width: 40.0,
                height: 50.0,
            }
            .for_startup(
                WindowSize {
                    width: 1440.0,
                    height: 850.0,
                },
                WindowSize {
                    width: 300.0,
                    height: 200.0,
                },
            ),
            WindowSize {
                width: 300.0,
                height: 200.0,
            }
        );
    }

    #[test]
    fn the_collapsed_sidebar_is_saved_and_older_files_open_it_expanded() {
        let (_, layout) = sample();
        assert!(!layout.sidebar_collapsed);
        let dir = std::env::temp_dir().join(format!("paddock-layout-test-{}", std::process::id()));
        let path = dir.join("layout.json");
        let (mut store, _) = Store::open(Some(path.clone()));
        store.save(&Layout {
            sidebar_collapsed: true,
            ..layout.clone()
        });
        let (_, back) = Store::open(Some(path.clone()));
        assert!(back.unwrap().sidebar_collapsed);
        // A file written before the sidebar could collapse has no such field.
        let mut old = serde_json::to_value(&layout).unwrap();
        old.as_object_mut().unwrap().remove("sidebar_collapsed");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path));
        assert!(store.problem().is_none());
        assert_eq!(back, Some(layout));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_folded_activity_panel_is_saved_and_older_files_open_it() {
        let (_, layout) = sample();
        assert!(!layout.activity_folded);
        let folded = Layout {
            activity_folded: true,
            ..layout.clone()
        };
        let text = serde_json::to_string(&folded).unwrap();
        assert!(
            serde_json::from_str::<Layout>(&text)
                .unwrap()
                .activity_folded
        );
        // A file written before the panel has no such field.
        let mut old = serde_json::to_value(&layout).unwrap();
        old.as_object_mut().unwrap().remove("activity_folded");
        let back: Layout = serde_json::from_value(old).unwrap();
        assert_eq!(back, layout);
    }

    #[test]
    fn the_right_sidebar_is_saved_and_older_files_open_it_closed() {
        let (_, layout) = sample();
        assert_eq!(layout.right_sidebar, right_panel::Saved::default());
        let dir = std::env::temp_dir().join(format!("paddock-right-test-{}", std::process::id()));
        let path = dir.join("layout.json");
        let (mut store, _) = Store::open(Some(path.clone()));
        let right = right_panel::Saved {
            open: true,
            width: 512.0,
            tab: right_panel::Tab::Browser,
            scope: crate::diff::Scope::Branch,
            split: true,
            url: Some("http://localhost:5173/settings".into()),
            kanban_folded: vec![crate::kanban::Column::Queued],
        };
        store.save(&Layout {
            right_sidebar: right.clone(),
            ..layout.clone()
        });
        let (_, back) = Store::open(Some(path.clone()));
        assert_eq!(back.unwrap().right_sidebar, right);
        // A file written before the right sidebar was saved has no such field: closed, 420
        // wide, on Changes.
        let mut old = serde_json::to_value(&layout).unwrap();
        old.as_object_mut().unwrap().remove("right_sidebar");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path.clone()));
        assert!(store.problem().is_none());
        let back = back.unwrap().right_sidebar;
        assert!(!back.open);
        assert_eq!(back.width, right_panel::DEFAULT_WIDTH);
        assert_eq!(back.width, 420.0);
        assert_eq!(back.tab, right_panel::Tab::Changes);
        // One with only some of its fields keeps those and defaults the rest.
        let mut part = serde_json::to_value(&layout).unwrap();
        part["right_sidebar"] = serde_json::json!({ "open": true });
        fs::write(&path, serde_json::to_vec(&part).unwrap()).unwrap();
        let (_, back) = Store::open(Some(path.clone()));
        assert_eq!(
            back.unwrap().right_sidebar,
            right_panel::Saved {
                open: true,
                ..right_panel::Saved::default()
            }
        );
        // One from before the Changes tab saved its scope and layout (P5-13a): uncommitted, unified.
        let mut before = serde_json::to_value(&layout).unwrap();
        before["right_sidebar"] =
            serde_json::json!({ "open": true, "width": 500.0, "tab": "changes" });
        fs::write(&path, serde_json::to_vec(&before).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path.clone()));
        assert!(store.problem().is_none());
        let back = back.unwrap().right_sidebar;
        assert_eq!((back.open, back.width), (true, 500.0));
        assert_eq!(back.scope, crate::diff::Scope::Uncommitted);
        assert!(!back.split);
        // One from before the Browser kept its address (P5-28a): nothing to open.
        assert_eq!(back.url, None);
        let mut earlier = serde_json::to_value(&layout).unwrap();
        earlier["right_sidebar"] = serde_json::json!({
            "open": true, "width": 500.0, "tab": "browser", "scope": "branch", "split": true
        });
        fs::write(&path, serde_json::to_vec(&earlier).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path.clone()));
        assert!(store.problem().is_none());
        let back = back.unwrap().right_sidebar;
        assert_eq!(back.tab, right_panel::Tab::Browser);
        assert_eq!(back.url, None);
        // One from before the Kanban (P5-29a): only its DONE group folded.
        assert_eq!(back.kanban_folded, [crate::kanban::Column::Done]);
        let mut kanban = serde_json::to_value(&layout).unwrap();
        kanban["right_sidebar"] = serde_json::json!({ "open": true, "tab": "kanban" });
        fs::write(&path, serde_json::to_vec(&kanban).unwrap()).unwrap();
        let (store, back) = Store::open(Some(path.clone()));
        assert!(store.problem().is_none());
        let back = back.unwrap().right_sidebar;
        assert_eq!(back.tab, right_panel::Tab::Kanban);
        assert_eq!(back.kanban_folded, [crate::kanban::Column::Done]);
        fs::remove_dir_all(&dir).unwrap();
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
            ratio: layout::EVEN,
            first: Box::new(SavedNode::Pane { pane: 0 }),
            second: Box::new(SavedNode::Pane { pane: 0 }),
        };
        cases.push(bad);
        for ratio in [0.0, 1.0, -0.2, f32::NAN] {
            let mut bad = good.clone();
            let SavedNode::Split { ratio: r, .. } = &mut bad.tabs[0].tree else {
                unreachable!()
            };
            *r = ratio;
            cases.push(bad);
        }
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
