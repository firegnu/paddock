//! The Changes tab's History reads, in throwaway repositories: order and lanes, a merge's diff
//! against its first parent, empty commits, task files, and the next batch.
mod common;
use paddock::history::{self, DiffRead, Read};
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

fn commit(dir: &Path, file: &str, text: &str, message: &str) {
    if let Some(parent) = Path::new(file).parent() {
        fs::create_dir_all(dir.join(parent)).unwrap();
    }
    fs::write(dir.join(file), text).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn empty_commit(dir: &Path, message: &str) {
    git(dir, &["commit", "-q", "--allow-empty", "-m", message]);
}

fn history(dir: &Path, limit: usize) -> paddock::history::History {
    match history::read("git", dir.to_str().unwrap(), limit, &AtomicBool::new(false)) {
        Some(Read::Commits(history)) => history,
        other => panic!("{other:?}"),
    }
}

fn subjects(history: &paddock::history::History) -> Vec<&str> {
    history
        .commits
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect()
}

#[test]
fn history_is_topological_and_a_merge_opens_and_closes_one_lane() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base.txt", "base\n", "base");
    git(&dir, &["checkout", "-q", "-b", "feature"]);
    commit(&dir, "feature.txt", "one\n", "feature one");
    commit(&dir, "feature.txt", "two\n", "feature two");
    git(&dir, &["checkout", "-q", "main"]);
    commit(&dir, "main.txt", "main\n", "main work");
    git(
        &dir,
        &[
            "merge",
            "--no-ff",
            "-q",
            "feature",
            "-m",
            "合并 P5-2：Feature work",
        ],
    );
    fs::create_dir_all(dir.join("docs/任务")).unwrap();
    fs::write(dir.join("docs/任务/P5-2-Feature.md"), "# 任务：Feature\n").unwrap();
    let index = fs::read(dir.join(".git/index")).unwrap();
    let head = {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        output.stdout
    };

    let history = history(&dir, 200);
    assert_eq!(history.head, "main");
    // On main with no upstream there is no base to compare with, as in Branch vs main.
    assert_eq!(history.base, None);
    assert!(!history.more);
    assert_eq!(history.lanes, 2);
    // Newest first, children before parents, both sides of the merge before the base.
    let names = subjects(&history);
    assert_eq!(names.first(), Some(&"合并 P5-2：Feature work"));
    assert_eq!(names.last(), Some(&"base"));
    let at = |name: &str| {
        names
            .iter()
            .position(|subject| *subject == name)
            .unwrap_or_else(|| panic!("{name} is not in {names:?}"))
    };
    assert!(at("feature two") < at("feature one"));
    assert!(at("feature one") < at("base"));
    assert!(at("main work") < at("base"));

    let merge = &history.commits[0];
    assert_eq!(merge.label.id.as_deref(), Some("P5-2"));
    assert_eq!(merge.label.rest, "Feature work");
    assert!(!merge.label.wrap_up);
    assert_eq!(merge.graph.lane, 0);
    assert!(merge.graph.forks.contains(&1));
    assert_eq!(merge.graph.bottom, vec![0, 1]);
    let feature = history
        .commits
        .iter()
        .find(|commit| commit.subject == "feature two")
        .unwrap();
    assert_eq!(feature.graph.lane, 1);
    let base = history
        .commits
        .iter()
        .find(|commit| commit.subject == "base")
        .unwrap();
    assert_eq!(base.graph.lane, 0);
    assert_eq!(base.graph.joins, vec![1]);
    // One task file with the merge's id, so Open task can find it.
    assert!(history.tasks.contains_key("P5-2"));

    // A merge against its first parent is the feature's whole change, not main's.
    let diff = match history::diff("git", &history.top, merge, &AtomicBool::new(false)) {
        Some(DiffRead::Diff(diff)) => diff,
        other => panic!("{other:?}"),
    };
    let paths: Vec<&str> = diff.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["feature.txt"]);
    assert_eq!((diff.added(), diff.deleted()), (1, 0));

    // A root commit is compared with the empty tree.
    let base = history
        .commits
        .iter()
        .find(|commit| commit.subject == "base")
        .unwrap();
    let root = match history::diff("git", &history.top, base, &AtomicBool::new(false)) {
        Some(DiffRead::Diff(diff)) => diff,
        other => panic!("{other:?}"),
    };
    let paths: Vec<&str> = root.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["base.txt"]);
    assert_eq!((root.added(), root.deleted()), (1, 0));

    // Reading wrote nothing.
    assert_eq!(fs::read(dir.join(".git/index")).unwrap(), index);
    let after = {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        output.stdout
    };
    assert_eq!(after, head);
}

#[test]
fn two_branches_open_together_and_an_empty_wrap_up_has_no_diff() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base.txt", "base\n", "base");
    git(&dir, &["checkout", "-q", "-b", "left"]);
    commit(&dir, "left.txt", "left\n", "left work");
    git(&dir, &["checkout", "-q", "main"]);
    git(&dir, &["checkout", "-q", "-b", "right"]);
    commit(&dir, "right.txt", "right\n", "right work");
    git(&dir, &["checkout", "-q", "main"]);
    git(
        &dir,
        &[
            "merge",
            "--no-ff",
            "-q",
            "left",
            "right",
            "-m",
            "合并 P5-9：Two branches",
        ],
    );
    empty_commit(&dir, "收尾: P5-9 做完了");

    let history = history(&dir, 200);
    assert_eq!(history.lanes, 3);
    let wrap = &history.commits[0];
    assert_eq!(wrap.subject, "收尾: P5-9 做完了");
    assert!(wrap.label.wrap_up);
    assert_eq!(wrap.label.id.as_deref(), Some("P5-9"));
    assert_eq!(wrap.label.rest, "做完了");
    let merge = history
        .commits
        .iter()
        .find(|commit| commit.subject == "合并 P5-9：Two branches")
        .unwrap();
    assert_eq!(merge.parents.len(), 3);
    assert_eq!(merge.graph.lane, 0);
    assert_eq!(merge.graph.forks, vec![1, 2]);
    assert_eq!(merge.graph.bottom, vec![0, 1, 2]);
    let left = history
        .commits
        .iter()
        .find(|commit| commit.subject == "left work")
        .unwrap();
    let right = history
        .commits
        .iter()
        .find(|commit| commit.subject == "right work")
        .unwrap();
    assert_eq!(left.graph.lane, 1);
    assert_eq!(right.graph.lane, 2);
    // A wrap-up commit has nothing to show.
    let diff = match history::diff("git", &history.top, wrap, &AtomicBool::new(false)) {
        Some(DiffRead::Diff(diff)) => diff,
        other => panic!("{other:?}"),
    };
    assert!(diff.files.is_empty());
    assert_eq!((diff.added(), diff.deleted()), (0, 0));
}

#[test]
fn task_ids_need_exactly_one_task_file() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "README.md", "x\n", "start");
    fs::create_dir_all(dir.join("docs/任务")).unwrap();
    fs::write(dir.join("docs/任务/P5-2-One.md"), "# 任务：One\n").unwrap();
    fs::write(dir.join("docs/任务/P5-3-A.md"), "# 任务：A\n").unwrap();
    fs::write(dir.join("docs/任务/P5-3-B.md"), "# 任务：B\n").unwrap();
    fs::write(dir.join("docs/任务/README.md"), "not a task\n").unwrap();
    let history = history(&dir, 200);
    assert!(history.tasks.contains_key("P5-2"));
    assert!(!history.tasks.contains_key("P5-3"));
    assert!(!history.tasks.contains_key("README"));
}

#[test]
fn the_next_batch_starts_after_the_last_commit_read() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    for n in 0..205 {
        empty_commit(&dir, &format!("c{n}"));
    }
    let first = history(&dir, 200);
    assert_eq!(first.commits.len(), 200);
    assert!(first.more);
    assert_eq!(first.commits[0].subject, "c204");
    assert_eq!(first.commits[199].subject, "c5");

    let next = history(&dir, 400);
    assert_eq!(next.commits.len(), 205);
    assert!(!next.more);
    assert_eq!(next.commits[199].hash, first.commits[199].hash);
    assert_eq!(next.commits[200].subject, "c4");
    assert_eq!(next.commits[204].subject, "c0");
}
