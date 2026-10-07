//! The activity grid's reads, in throwaway repositories with commits at chosen times: each commit
//! counted once on the day of its author time in the given time zone, wrap-ups on main counted
//! apart, and the list of repositories kept as paths only.
mod common;
use paddock::activity::{Day, count, day_of, load_repos, main_repository, remember, save_repos};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

/// 2026-10-07, a Wednesday.
const OCT_7: Day = 20_733;
const HOUR: i64 = 3600;

// Fixture setup only; isolated from the user's Git config so commits never sign or prompt.
fn git(dir: &Path, args: &[&str], at: Option<i64>) {
    let mut command = Command::new("git");
    command
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c"])
        .arg("commit.gpgsign=false")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(at) = at {
        let date = format!("{at} +0000");
        command
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date);
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

fn commit(dir: &Path, message: &str, at: i64) {
    git(
        dir,
        &["commit", "-q", "--allow-empty", "-m", message],
        Some(at),
    );
}

fn repo(root: &Path, name: &str) -> PathBuf {
    let repo = root.join(name);
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"], None);
    repo
}

/// Seconds since 1970 at `hour` UTC on `day`.
fn at(day: Day, hour: i64) -> i64 {
    day * 86_400 + hour * HOUR
}

#[test]
fn commits_land_on_their_local_days_once_each() {
    let temp = common::tempdir();
    let repo = repo(temp.path(), "paddock");
    // Long before the days shown: never counted. Git stops at the first commit older than the
    // days read, so history is made in order, as it is when written.
    commit(&repo, "old", at(OCT_7 - 200, 12));
    // Late on the 6th in UTC: the 7th eight hours east.
    commit(&repo, "late", at(OCT_7 - 1, 23) + 30 * 60);
    commit(&repo, "morning", at(OCT_7, 2));
    // A branch from here shares these commits and adds one.
    git(&repo, &["checkout", "-q", "-b", "p5-32"], None);
    commit(&repo, "on the branch", at(OCT_7, 3));
    git(&repo, &["checkout", "-q", "main"], None);
    let repos = [repo];
    let cancel = AtomicBool::new(false);
    let east = count("git", &repos, OCT_7 - 30, OCT_7, &|_| 8 * HOUR, &cancel).unwrap();
    assert_eq!(east.repos, ["paddock"]);
    assert_eq!(east.commits(OCT_7), 3);
    assert_eq!(east.commits(OCT_7 - 1), 0);
    assert_eq!(east.total(OCT_7 - 30), 3);
    let utc = count("git", &repos, OCT_7 - 30, OCT_7, &|_| 0, &cancel).unwrap();
    assert_eq!(utc.commits(OCT_7 - 1), 1);
    assert_eq!(utc.commits(OCT_7), 2);
    // Six hours west, the morning commits are still the 6th.
    let west = count("git", &repos, OCT_7 - 30, OCT_7, &|_| -6 * HOUR, &cancel).unwrap();
    assert_eq!((west.commits(OCT_7 - 1), west.commits(OCT_7)), (3, 0));
    assert_eq!(day_of(at(OCT_7, 3), -6 * HOUR), OCT_7 - 1);
}

#[test]
fn wrap_ups_are_counted_on_main_and_repositories_apart() {
    let temp = common::tempdir();
    let paddock = repo(temp.path(), "paddock");
    let ranch = repo(temp.path(), "ranch");
    commit(&paddock, "合并 P5-31：x", at(OCT_7, 1));
    commit(&paddock, "收尾: P5-31 侧栏提示", at(OCT_7, 2));
    commit(&paddock, "收尾： P5-30 全角冒号", at(OCT_7, 3));
    commit(&paddock, "收尾: P5-29r 不做：不需要", at(OCT_7 - 2, 3));
    // Not a wrap-up: no id, or not at the start.
    commit(&paddock, "收尾: ", at(OCT_7, 4));
    commit(&paddock, "HANDOFF：收尾: P5-31 之后", at(OCT_7, 5));
    // A wrap-up only on a branch is not one on main.
    git(&paddock, &["checkout", "-q", "-b", "side"], None);
    commit(&paddock, "收尾: P5-99 x", at(OCT_7, 6));
    git(&paddock, &["checkout", "-q", "main"], None);
    commit(&ranch, "fix", at(OCT_7, 7));
    commit(&ranch, "收尾: R-1 x", at(OCT_7, 8));
    let repos = [paddock, ranch];
    let counts = count(
        "git",
        &repos,
        OCT_7 - 30,
        OCT_7,
        &|_| 0,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(counts.wrapped(OCT_7), 3);
    assert_eq!(counts.wrapped(OCT_7 - 2), 1);
    assert_eq!(counts.commits(OCT_7), 8);
    assert_eq!(counts.repos_on(OCT_7), [("paddock", 6), ("ranch", 2)]);
}

#[test]
fn a_repository_git_cannot_read_counts_nothing() {
    let temp = common::tempdir();
    let empty = repo(temp.path(), "empty");
    let plain = temp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let busy = repo(temp.path(), "busy");
    commit(&busy, "x", at(OCT_7, 1));
    let counts = count(
        "git",
        &[empty, plain, busy],
        OCT_7 - 7,
        OCT_7,
        &|_| 0,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(counts.repos, ["empty", "plain", "busy"]);
    assert_eq!(counts.repos_on(OCT_7), [("busy", 1)]);
    assert_eq!(counts.days.len(), 1);
}

#[test]
fn worktrees_and_subdirectories_belong_to_their_main_repository() {
    let temp = common::tempdir();
    let main = repo(temp.path(), "paddock");
    commit(&main, "start", at(OCT_7, 1));
    let worktree = temp.path().join("paddock-worktrees/p5-32");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p5-32",
            worktree.to_str().unwrap(),
        ],
        None,
    );
    fs::create_dir_all(main.join("app/src")).unwrap();
    fs::create_dir_all(worktree.join("app")).unwrap();
    let cancel = AtomicBool::new(false);
    let of = |dir: &Path| main_repository("git", dir.to_str().unwrap(), &cancel);
    assert_eq!(of(&main), Some(main.clone()));
    assert_eq!(of(&main.join("app/src")), Some(main.clone()));
    assert_eq!(of(&worktree), Some(main.clone()));
    assert_eq!(of(&worktree.join("app")), Some(main.clone()));
    let plain = temp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    assert_eq!(of(&plain), None);
    assert_eq!(main_repository("git", "relative/dir", &cancel), None);
}

#[test]
fn the_list_keeps_paths_only_and_skips_those_gone() {
    let temp = common::tempdir();
    let path = temp.path().join("state/paddock/activity-repos.json");
    // No list yet.
    assert!(load_repos(&path).is_empty());
    let (a, b) = (temp.path().join("a"), temp.path().join("b"));
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let mut repos = Vec::new();
    assert!(remember(&mut repos, [b.clone(), a.clone()]));
    assert!(!remember(&mut repos, [a.clone()]));
    assert_eq!(repos, [a.clone(), b.clone()]);
    save_repos(&path, &repos).unwrap();
    let written: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!([a.to_str().unwrap(), b.to_str().unwrap()])
    );
    assert_eq!(load_repos(&path), [a.clone(), b.clone()]);
    // A repository no longer there is left out.
    fs::remove_dir_all(&b).unwrap();
    assert_eq!(load_repos(&path), [a]);
    // A list that cannot be read gives none.
    fs::write(&path, "{ not json").unwrap();
    assert!(load_repos(&path).is_empty());
}
