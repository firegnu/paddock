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
