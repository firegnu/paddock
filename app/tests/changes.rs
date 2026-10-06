//! The Changes tab's reads, in throwaway repositories: what each scope finds, and that reading
//! writes nothing.
mod common;
use paddock::diff::{Kind, Note, Read, Scope, Status, read};
use std::{fs, path::Path, process::Command, sync::atomic::AtomicBool};

// Fixture setup only; isolated from the user's Git config so commits never sign or prompt.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c"])
        .arg("commit.gpgsign=false")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

fn commit(dir: &Path, file: &str, text: &[u8]) {
    if let Some(parent) = Path::new(file).parent() {
        fs::create_dir_all(dir.join(parent)).unwrap();
    }
    fs::write(dir.join(file), text).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", file]);
}

fn changes(dir: &Path, scope: Scope) -> paddock::diff::Changes {
    match read("git", dir.to_str().unwrap(), scope, &AtomicBool::new(false)) {
        Some(Read::Changes(changes)) => changes,
        other => panic!("{other:?}"),
    }
}

fn summary(changes: &paddock::diff::Changes) -> Vec<(String, Status, u32, u32)> {
    changes
        .files
        .iter()
        .map(|f| (f.path.clone(), f.status, f.added, f.deleted))
        .collect()
}

#[test]
fn uncommitted_is_staged_unstaged_and_untracked_against_head() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let numbers: String = (1..=30).map(|n| format!("line {n}\n")).collect();
    commit(&dir, "src/a.txt", numbers.as_bytes());
    commit(&dir, "old.txt", b"keep\nthis\n");
    commit(&dir, "gone.txt", b"bye\n");
    commit(&dir, "logo.png", b"\x89PNG\0\0\x01");
    // Unstaged: two lines changed far apart, so two hunks with unchanged lines between.
    fs::write(
        dir.join("src/a.txt"),
        numbers
            .replace("line 2\n", "line two\n")
            .replace("line 25\n", "line 25\nline 25b\n"),
    )
    .unwrap();
    // Staged: a rename without changes and a deletion.
    git(&dir, &["mv", "old.txt", "new.txt"]);
    git(&dir, &["rm", "-q", "gone.txt"]);
    // A binary change, and two files Git has never seen.
    fs::write(dir.join("logo.png"), b"\x89PNG\0\0\x02\x03").unwrap();
    fs::write(dir.join("notes.md"), "# Notes\nhello\n").unwrap();
    fs::write(dir.join("blob.bin"), b"a\0b").unwrap();
    let index = fs::read(dir.join(".git/index")).unwrap();

    let changes = changes(&dir.join("src"), Scope::Uncommitted);
    assert_eq!(changes.head, "main");
    assert_eq!(changes.top, dir.to_str().unwrap());
    // The paths are the repository's, whichever directory the pane is in; sorted.
    assert_eq!(
        summary(&changes),
        [
            ("blob.bin".into(), Status::Added, 0, 0),
            ("gone.txt".into(), Status::Deleted, 0, 1),
            ("logo.png".into(), Status::Modified, 0, 0),
            ("new.txt".into(), Status::Renamed, 0, 0),
            ("notes.md".into(), Status::Added, 2, 0),
            ("src/a.txt".into(), Status::Modified, 2, 1),
        ]
    );
    let file = |path: &str| changes.files.iter().find(|f| f.path == path).unwrap();
    // The binary ones say so, with their sizes before and after.
    assert_eq!(file("logo.png").note, Some(Note::Binary(Some(7), Some(8))));
    assert_eq!(file("blob.bin").note, Some(Note::Binary(None, Some(3))));
    assert_eq!(file("new.txt").old_path.as_deref(), Some("old.txt"));
    // Untracked files come as added lines.
    let notes = &file("notes.md").hunks[0];
    assert!(notes.lines.iter().all(|l| l.kind == Kind::Added));
    assert_eq!(notes.lines[0].text, "# Notes");
    // The modified file's hunks, and its length for the lines after the last.
    let a = file("src/a.txt");
    assert_eq!(a.hunks.len(), 2);
    assert_eq!(a.lines_now, Some(31));
    let changed: Vec<&str> = a
        .hunks
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| l.kind != Kind::Context)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(changed, ["line 2", "line two", "line 25b"]);
    assert_eq!(changes.added(), 4);
    assert_eq!(changes.deleted(), 2);
    // Reading changed nothing: not the index, not the worktree.
    assert_eq!(fs::read(dir.join(".git/index")).unwrap(), index);
    assert!(!dir.join(".git/index.lock").exists());
}

#[test]
fn branch_is_everything_since_it_left_its_base() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "a.txt", b"1\n2\n3\n");
    git(&dir, &["checkout", "-q", "-b", "dev"]);
    commit(&dir, "b.txt", b"from dev\n");
    // Main moves on after dev left it: not part of dev's changes.
    git(&dir, &["checkout", "-q", "main"]);
    commit(&dir, "m.txt", b"on main\n");
    git(&dir, &["checkout", "-q", "dev"]);
    // Uncommitted on top.
    fs::write(dir.join("a.txt"), "1\n2\n3\n4\n").unwrap();
    fs::write(dir.join("c.txt"), "new\n").unwrap();

    let branch = changes(&dir, Scope::Branch);
    assert_eq!(branch.head, "dev");
    assert_eq!(branch.base.as_deref(), Some("main"));
    assert_eq!(
        summary(&branch),
        [
            ("a.txt".into(), Status::Modified, 1, 0),
            ("b.txt".into(), Status::Added, 1, 0),
            ("c.txt".into(), Status::Added, 1, 0),
        ]
    );
    // Uncommitted leaves out what dev committed.
    let uncommitted = changes(&dir, Scope::Uncommitted);
    assert_eq!(
        summary(&uncommitted),
        [
            ("a.txt".into(), Status::Modified, 1, 0),
            ("c.txt".into(), Status::Added, 1, 0),
        ]
    );

    // On main with no upstream there is nothing to compare with.
    git(&dir, &["stash", "-q", "-u"]);
    git(&dir, &["checkout", "-q", "main"]);
    let none = read(
        "git",
        dir.to_str().unwrap(),
        Scope::Branch,
        &AtomicBool::new(false),
    );
    assert_eq!(
        none,
        Some(Read::NoBase {
            head: "main".into()
        })
    );
    // With an upstream, main is compared with it: here the same commit, so nothing.
    git(&dir, &["remote", "add", "origin", "/nonexistent"]);
    git(&dir, &["update-ref", "refs/remotes/origin/main", "HEAD~1"]);
    git(&dir, &["branch", "--set-upstream-to=origin/main"]);
    let ahead = changes(&dir, Scope::Branch);
    assert_eq!(ahead.base.as_deref(), Some("origin/main"));
    assert_eq!(summary(&ahead), [("m.txt".into(), Status::Added, 1, 0)]);
}

#[test]
fn outside_a_repository_and_before_the_first_commit() {
    let temp = common::tempdir();
    let plain = temp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    // A directory of its own, outside any repository the temp directory might sit in.
    let outside = read(
        "git",
        plain.to_str().unwrap(),
        Scope::Uncommitted,
        &AtomicBool::new(false),
    );
    if !temp.path().ancestors().any(|p| p.join(".git").exists()) {
        assert_eq!(outside, Some(Read::NotRepository));
    }
    assert_eq!(
        read(
            "git",
            "relative/dir",
            Scope::Uncommitted,
            &AtomicBool::new(false)
        ),
        Some(Read::NotRepository)
    );

    let fresh = temp.path().join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    git(&fresh, &["init", "-q", "-b", "main"]);
    fs::write(fresh.join("staged.txt"), "s\n").unwrap();
    git(&fresh, &["add", "staged.txt"]);
    fs::write(fresh.join("loose.txt"), "l\n").unwrap();
    let changes = changes(&fresh, Scope::Uncommitted);
    assert_eq!(
        summary(&changes),
        [
            ("loose.txt".into(), Status::Added, 1, 0),
            ("staged.txt".into(), Status::Added, 1, 0),
        ]
    );
    // A cancelled read gives nothing.
    assert_eq!(
        read(
            "git",
            fresh.to_str().unwrap(),
            Scope::Uncommitted,
            &AtomicBool::new(true)
        ),
        None
    );
}
