//! Which repository the Kanban tab shows (P5-85): what its menu lists, the kept list and its
//! file, the directory the board reads, and what the button back to the focused pane says.
mod common;
use paddock::kanban_repos::{KEPT, Looking, OPEN_NOW, Row, back_label, menu};
use std::{fs, path::PathBuf};

fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

fn repo(text: &str, kept: bool, on: bool) -> Row {
    Row::Repo {
        path: path(text),
        kept,
        on,
    }
}

fn follow(name: Option<&str>, on: bool) -> Row {
    Row::Follow {
        name: name.map(str::to_owned),
        on,
    }
}

#[test]
fn the_menu_lists_open_repositories_once_by_name_and_the_kept_ones_apart() {
    let mut looking = Looking::load(None);
    // Nothing open and nothing kept: neither section, only the two lines.
    assert_eq!(menu(None, &[], &looking), [follow(None, true), Row::Choose]);
    // Two agents and a shell in ranch: listed once; the lot by name, not as they came.
    let open = [
        path("/w/ranch"),
        path("/w/paddock"),
        path("/w/ranch"),
        path("/w/cairn"),
        path("/w/ranch"),
    ];
    let focus = path("/w/paddock");
    assert_eq!(
        menu(Some(&focus), &open, &looking),
        [
            follow(Some("paddock"), true),
            Row::Heading(OPEN_NOW),
            repo("/w/cairn", false, false),
            repo("/w/paddock", false, false),
            repo("/w/ranch", false, false),
            Row::Choose,
        ]
    );
    // A kept one is listed under Kept only, and the one the board stays on is the one ticked.
    looking.choose(path("/w/pasture"));
    looking.choose(path("/w/ranch"));
    assert_eq!(
        menu(Some(&focus), &open, &looking),
        [
            follow(Some("paddock"), false),
            Row::Heading(OPEN_NOW),
            repo("/w/cairn", false, false),
            repo("/w/paddock", false, false),
            Row::Heading(KEPT),
            repo("/w/pasture", true, false),
            repo("/w/ranch", true, true),
            Row::Choose,
        ]
    );
    // Back following: the tick goes back to the first line.
    looking.follow();
    assert_eq!(
        menu(Some(&focus), &open, &looking)[0],
        follow(Some("paddock"), true)
    );
    // Everything open is kept: no Open now.
    assert_eq!(
        menu(None, &[path("/w/ranch"), path("/w/pasture")], &looking),
        [
            follow(None, true),
            Row::Heading(KEPT),
            repo("/w/pasture", true, false),
            repo("/w/ranch", true, false),
            Row::Choose,
        ]
    );
    // Two of one name are both listed, told apart by their paths.
    let twins = [path("/b/site"), path("/a/site")];
    assert_eq!(
        menu(None, &twins, &Looking::load(None)),
        [
            follow(None, true),
            Row::Heading(OPEN_NOW),
            repo("/a/site", false, false),
            repo("/b/site", false, false),
            Row::Choose,
        ]
    );
}

#[test]
fn kept_repositories_are_saved_as_paths_and_read_back() {
    let temp = common::tempdir();
    let file = temp.path().join("state/paddock/kanban-repos.json");
    let (a, b) = (temp.path().join("a"), temp.path().join("b"));
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    // No list yet.
    let mut looking = Looking::load(Some(file.clone()));
    assert!(looking.kept().is_empty());
    // Picked ones are kept, each once.
    looking.choose(a.clone());
    looking.choose(b.clone());
    looking.choose(a.clone());
    assert_eq!(looking.kept(), [a.clone(), b.clone()]);
    let written: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!([a.to_str().unwrap(), b.to_str().unwrap()])
    );
    // A new window has the list, and follows the focused pane: where the board stayed is not kept.
    let mut other = Looking::load(Some(file.clone()));
    assert_eq!(other.kept(), [a.clone(), b.clone()]);
    assert_eq!(other.chosen(), None);
    // What one window removes, the other no longer lists once it reads again; what it keeps
    // itself is added to the list as it is now.
    looking.remove(&a);
    let c = temp.path().join("c");
    fs::create_dir_all(&c).unwrap();
    other.choose(c.clone());
    assert_eq!(other.kept(), [b.clone(), c.clone()]);
    looking.refresh();
    assert_eq!(looking.kept(), [b.clone(), c.clone()]);
    // A repository no longer there is left out.
    fs::remove_dir_all(&b).unwrap();
    assert_eq!(
        Looking::load(Some(file.clone())).kept(),
        std::slice::from_ref(&c)
    );
    // The last one removed leaves an empty list.
    looking.remove(&c);
    assert!(looking.kept().is_empty());
    assert!(Looking::load(Some(file)).kept().is_empty());
}

#[test]
fn the_board_reads_the_focused_panes_directory_until_a_repository_is_picked() {
    let focus = Some(Some("/w/paddock-worktrees/p5-85".to_owned()));
    let pasture = Some(Some("/w/pasture".to_owned()));
    let mut looking = Looking::load(None);
    // Following: the focused pane's directory, whatever it is.
    assert_eq!(looking.dir(&focus), focus);
    assert_eq!(looking.dir(&Some(None)), Some(None));
    assert_eq!(looking.dir(&None), None);
    // Picked: that repository, wherever the focus is, and with no pane in focus too.
    looking.choose(path("/w/pasture"));
    assert_eq!(looking.chosen(), Some(path("/w/pasture").as_path()));
    assert_eq!(looking.dir(&focus), pasture);
    assert_eq!(looking.dir(&None), pasture);
    // Another kept one removed: the board stays where it is.
    looking.choose(path("/w/ranch"));
    looking.choose(path("/w/pasture"));
    looking.remove(&path("/w/ranch"));
    assert_eq!(looking.dir(&focus), pasture);
    assert_eq!(looking.kept(), [path("/w/pasture")]);
    // The one it stays on removed: back to the focused pane.
    looking.remove(&path("/w/pasture"));
    assert_eq!(looking.chosen(), None);
    assert_eq!(looking.dir(&focus), focus);
    assert!(looking.kept().is_empty());
    // Following again keeps the list.
    looking.choose(path("/w/pasture"));
    looking.follow();
    assert_eq!(looking.dir(&focus), focus);
    assert_eq!(looking.kept(), [path("/w/pasture")]);
}

#[test]
fn the_button_back_names_the_focused_panes_repository_when_it_is_another() {
    let (paddock, pasture) = (path("/w/paddock"), path("/w/pasture"));
    assert_eq!(back_label(Some(&paddock), &pasture), "Back to paddock");
    // No pane in focus, or one that is in no repository: there is none to name.
    assert_eq!(back_label(None, &pasture), "Follow focus");
    // The focused pane is in the repository the board stays on.
    assert_eq!(back_label(Some(&pasture), &pasture), "Follow focus");
}
