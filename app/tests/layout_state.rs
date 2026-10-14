//! The saved layout file: written and read back, not rewritten unchanged, and kept when broken.
mod common;
use paddock::{
    layout::{Shown, Workspace},
    layout_state::{Content, Layout, Store},
};

fn layout() -> Layout {
    let (workspace, _) = Workspace::new(Shown::Shell);
    Layout::of(&workspace, |_| Content::Shell { cwd: "/tmp".into() })
}

#[test]
fn a_saved_layout_is_restored_next_time() {
    let temp = common::tempdir();
    let path = temp.path().join("state/paddock/layout.json");
    let (mut store, saved) = Store::open(Some(path.clone()));
    assert_eq!(saved, None);
    assert!(store.problem().is_none());
    store.save(&layout());
    assert!(matches!(store.saved, Some((_, Ok(())))));
    let (_, saved) = Store::open(Some(path));
    assert_eq!(saved, Some(layout()));
}

#[test]
fn folded_groups_are_saved_and_a_file_from_before_them_opens_every_group() {
    let temp = common::tempdir();
    let path = temp.path().join("layout.json");
    // Written as the sidebar folds groups, a workspace and a group inside one…
    let folded = serde_json::json!({
        "groups": ["paddock", ""],
        "workspaces": ["Agent tools"],
        "within": [["Fine-tuning", "jb-finetune"]],
    });
    let mut file = serde_json::to_value(layout()).unwrap();
    file["sidebar_folded"] = folded.clone();
    std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
    let (mut store, saved) = Store::open(Some(path.clone()));
    assert!(store.problem().is_none());
    let saved = saved.unwrap();
    let folds = &saved.sidebar_folded;
    assert!(folds.group(None, "paddock") && folds.workspace("Agent tools"));
    assert!(folds.group(Some("Fine-tuning"), "jb-finetune"));
    // …read back, and written again as it was.
    store.save(&saved);
    let again: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(again["sidebar_folded"], folded);
    // A file from when only groups folded keeps them.
    file["sidebar_folded"] = serde_json::json!({ "groups": ["paddock"] });
    std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
    let (_, saved) = Store::open(Some(path.clone()));
    let folds = saved.unwrap().sidebar_folded;
    assert!(folds.group(None, "paddock") && folds.workspaces.is_empty());
    // A file from before groups could fold has no such field: every group is open.
    let mut old = serde_json::to_value(layout()).unwrap();
    old.as_object_mut().unwrap().remove("sidebar_folded");
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let (store, saved) = Store::open(Some(path));
    assert!(store.problem().is_none());
    assert_eq!(saved, Some(layout()));
}

#[test]
fn an_unchanged_layout_is_not_written_again() {
    let temp = common::tempdir();
    let path = temp.path().join("layout.json");
    let (mut store, _) = Store::open(Some(path.clone()));
    store.save(&layout());
    std::fs::remove_file(&path).unwrap();
    store.save(&layout());
    assert!(!path.exists());
}

#[test]
fn a_file_that_cannot_be_restored_is_kept() {
    let temp = common::tempdir();
    let path = temp.path().join("layout.json");
    std::fs::write(&path, "{ not a layout").unwrap();
    let (mut store, saved) = Store::open(Some(path.clone()));
    assert_eq!(saved, None);
    assert!(store.problem().unwrap().contains("kept as it was"));
    store.save(&layout());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not a layout");
}
