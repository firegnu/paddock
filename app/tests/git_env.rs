//! From Saddle `tests/git_env.rs` at commit `df1c727`.
mod common;
// Its own test binary: it changes this process's environment, which no other test may observe.
use paddock::git::{Head, collect};
use std::{fs, path::Path, process::Command, sync::atomic::AtomicBool};

fn repo(dir: &Path, branch: &str) {
    fs::create_dir_all(dir).unwrap();
    let status = Command::new("git")
        .args(["init", "-q", "-b", branch])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn inherited_git_environment_does_not_redirect_or_trace_summaries() {
    let temp = common::tempdir();
    let (a, b) = (temp.path().join("a"), temp.path().join("b"));
    repo(&a, "branch-a");
    repo(&b, "branch-b");
    fs::write(b.join("new.txt"), "").unwrap();
    let trace = temp.path().join("trace");
    // SAFETY: this binary has a single test, so no other thread reads the environment meanwhile.
    unsafe {
        std::env::set_var("GIT_DIR", a.join(".git"));
        std::env::set_var("GIT_WORK_TREE", &a);
        std::env::set_var("GIT_INDEX_FILE", temp.path().join("other-index"));
        std::env::set_var("GIT_TRACE", &trace);
    }
    let cwds = [&a, &b].map(|p| p.to_str().unwrap().to_owned());
    let results = collect("git", &cwds, &AtomicBool::new(false));
    let a = results[0].1.as_ref().unwrap();
    let b = results[1].1.as_ref().unwrap();
    assert_eq!(a.head, Head::Branch("branch-a".into()));
    assert_eq!(b.head, Head::Branch("branch-b".into()));
    assert_eq!(b.untracked, Some(1));
    assert!(!trace.exists(), "GIT_TRACE was written");
}
