//! From Saddle `tests/git.rs` at commit `df1c727`.
mod common;
use paddock::git::{Changes, Head, Poller, Summary, collect};
use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

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
fn repo(dir: &Path, branch: &str) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", branch]);
}
fn commit(dir: &Path, file: &str, text: &str) {
    fs::write(dir.join(file), text).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", file]);
}
fn set_old_mtime(file: &Path) {
    fs::File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000))
        .unwrap();
}
fn path(dir: &Path) -> String {
    dir.to_str().unwrap().into()
}
fn changes(added: u64, deleted: u64, binary: u64) -> Option<Changes> {
    Some(Changes {
        added,
        deleted,
        binary,
    })
}

#[test]
fn worktrees_report_branch_commits_against_base_uncommitted_lines_and_untracked_files() {
    let temp = common::tempdir();
    let main = temp.path().join("main");
    let dev = temp.path().join("dev");
    repo(&main, "main");
    commit(&main, "a.txt", "1\n2\n3\n");
    commit(&main, "bin.dat", "a\0b");
    // A local upstream ref without any network remote.
    git(&main, &["remote", "add", "origin", "/nonexistent"]);
    git(&main, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&main, &["branch", "--set-upstream-to=origin/main"]);
    git(
        &main,
        &["worktree", "add", "-q", "-b", "dev", path(&dev).as_str()],
    );
    commit(&main, "m.txt", "m\n");
    commit(&dev, "w1.txt", "w\n");
    commit(&dev, "w2.txt", "w\n");
    fs::write(dev.join("a.txt"), "2\n3\nx\ny\nz\n").unwrap();
    fs::write(dev.join("b.txt"), "p\nq\n").unwrap();
    git(&dev, &["add", "b.txt"]);
    fs::write(dev.join("bin.dat"), "c\0d").unwrap();
    fs::create_dir(dev.join("sub")).unwrap();
    fs::write(dev.join("sub/d.txt"), "").unwrap();
    fs::write(dev.join("c.txt"), "").unwrap();

    // Log every call so shared worktrees are provably queried once per round.
    let log = temp.path().join("calls");
    let program = common::script(
        temp.path(),
        "git-log",
        &format!("#!/bin/sh\necho \"$*\" >> {log:?}\nexec git \"$@\"\n"),
    );
    let gone = temp.path().join("gone");
    fs::create_dir(&gone).unwrap();
    let gone = path(&gone);
    fs::remove_dir(&gone).unwrap();
    let plain = temp.path().join("plain");
    fs::create_dir(&plain).unwrap();
    let cwds = [
        path(&main),
        path(&dev),
        path(&dev.join("sub")),
        path(&plain),
        gone,
        "relative/dir".to_string(),
    ];
    let results = collect(&program, &cwds, &AtomicBool::new(false));
    let get = |cwd: &str| results.iter().find(|(c, _)| c == cwd).unwrap().1.clone();

    assert_eq!(
        get(&cwds[0]),
        Some(Summary {
            main_repository: Some(main.canonicalize().unwrap()),
            head: Head::Branch("main".into()),
            ahead: Some((1, "origin/main".into())),
            changes: changes(0, 0, 0),
            untracked: Some(0),
        })
    );
    let dev_summary = Some(Summary {
        main_repository: Some(main.canonicalize().unwrap()),
        head: Head::Branch("dev".into()),
        ahead: Some((2, "main".into())),
        changes: changes(5, 1, 1),
        untracked: Some(2),
    });
    assert_eq!(get(&cwds[1]), dev_summary);
    assert_eq!(get(&cwds[2]), dev_summary); // a subdirectory reports its whole worktree
    for cwd in &cwds[3..] {
        assert_eq!(get(cwd), None, "{cwd}");
    }
    let calls = fs::read_to_string(&log).unwrap();
    // Two worktrees of one repository: two summaries, never merged by the common Git dir.
    assert_eq!(calls.matches("diff-index").count(), 2, "{calls}");
    assert!(!calls.contains("relative/dir"), "{calls}");
}

#[test]
fn undeterminable_fields_stay_unknown_instead_of_zero() {
    let temp = common::tempdir();
    let unborn = temp.path().join("unborn");
    repo(&unborn, "main");
    fs::write(unborn.join("u.txt"), "u\n").unwrap();
    let detached = temp.path().join("detached");
    repo(&detached, "main");
    commit(&detached, "a.txt", "a\n");
    git(&detached, &["checkout", "-q", "--detach"]);
    let no_main = temp.path().join("no-main");
    repo(&no_main, "trunk");
    commit(&no_main, "a.txt", "a\n");
    let no_upstream = temp.path().join("no-upstream");
    repo(&no_upstream, "main");
    commit(&no_upstream, "a.txt", "a\n");
    let cwds = [&unborn, &detached, &no_main, &no_upstream].map(|p| path(p));
    let results = collect("git", &cwds, &AtomicBool::new(false));
    let get = |i: usize| results[i].1.clone().unwrap();
    assert_eq!(
        get(0),
        Summary {
            main_repository: Some(unborn.canonicalize().unwrap()),
            head: Head::Branch("main".into()),
            ahead: None,
            changes: None,
            untracked: Some(1),
        }
    );
    assert_eq!(get(1).head, Head::Detached);
    assert_eq!(get(1).ahead, None);
    assert_eq!(get(1).changes, changes(0, 0, 0));
    assert_eq!(get(2).head, Head::Branch("trunk".into()));
    assert_eq!(get(2).ahead, None);
    assert_eq!(get(3).ahead, None);
}

#[test]
fn summaries_never_run_repository_helpers_or_write_the_index() {
    let temp = common::tempdir();
    let marker = temp.path().join("ran");
    let hook = common::script(
        temp.path(),
        "hook",
        &format!("#!/bin/sh\ntouch {marker:?}\ncat\n"),
    );
    // The same filter rule from each attributes source Git reads, with a clean or process filter.
    for (source, filter) in [
        ("worktree", "clean"),
        ("info", "clean"),
        ("global", "clean"),
        ("info", "process"),
    ] {
        let dir = temp.path().join(format!("{source}-{filter}"));
        repo(&dir, "main");
        commit(&dir, "a.txt", "a\n");
        commit(&dir, "b.md", "b\n");
        let rule = "*.txt diff=evil filter=evil\n";
        match source {
            "worktree" => commit(&dir, ".gitattributes", rule),
            "info" => fs::write(dir.join(".git/info/attributes"), rule).unwrap(),
            _ => {
                let file = temp.path().join(format!("{source}-{filter}.attributes"));
                fs::write(&file, rule).unwrap();
                git(&dir, &["config", "core.attributesFile", &path(&file)]);
            }
        }
        // An old, refreshed timestamp: otherwise Git must re-read the racily clean a.txt, which
        // needs the filter, and the diff is rightly unknown before a.txt is even touched.
        set_old_mtime(&dir.join("a.txt"));
        git(&dir, &["update-index", "-q", "--refresh"]);
        for (key, value) in [
            (format!("filter.evil.{filter}"), hook.as_str()),
            ("diff.evil.textconv".into(), hook.as_str()),
            ("diff.evil.command".into(), hook.as_str()),
            ("diff.external".into(), hook.as_str()),
            ("core.fsmonitor".into(), hook.as_str()),
        ] {
            git(&dir, &["config", &key, value]);
        }
        fs::write(dir.join("b.md"), "b\nc\n").unwrap();
        let index = fs::read(dir.join(".git/index")).unwrap();
        let summary = |dir: &Path| {
            collect("git", &[path(dir)], &AtomicBool::new(false))[0]
                .1
                .clone()
                .unwrap()
        };
        // Files outside the filter still count normally.
        assert_eq!(summary(&dir).changes, changes(1, 0, 0), "{source} {filter}");
        // A changed file that needs the filter makes the diff unknown instead of running it.
        fs::write(dir.join("a.txt"), "a\nb\n").unwrap();
        let s = summary(&dir);
        assert_eq!(s.changes, None, "{source} {filter}");
        assert_eq!(s.head, Head::Branch("main".into()));
        assert!(
            !marker.exists(),
            "{source} {filter}: a repository helper ran"
        );
        assert_eq!(fs::read(dir.join(".git/index")).unwrap(), index);
    }
}

#[test]
fn builtin_attributes_keep_normalised_line_counts_and_binary_marks() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    repo(&dir, "main");
    commit(
        &dir,
        ".gitattributes",
        "*.txt text eol=crlf\n*.asset binary\n",
    );
    commit(&dir, "a.txt", "a\r\nb\r\n");
    commit(&dir, "b.asset", "x\n");
    // Committed CRLF content, unchanged: only its timestamp moves.
    set_old_mtime(&dir.join("a.txt"));
    fs::write(dir.join("b.asset"), "y\n").unwrap();
    let results = collect("git", &[path(&dir)], &AtomicBool::new(false));
    assert_eq!(results[0].1.as_ref().unwrap().changes, changes(0, 0, 1));
}

#[test]
fn git_that_cannot_refuse_lazy_fetch_is_unavailable_and_missing_objects_stay_missing() {
    let temp = common::tempdir();
    let dir = temp.path().join("repo");
    repo(&dir, "main");
    commit(&dir, "a.txt", "a\n");
    // Stands in for Git older than 2.45, which rejects the option as unknown.
    let old = common::script(
        temp.path(),
        "old-git",
        "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = --no-lazy-fetch ] && { echo \"unknown option: $a\" >&2; exit 129; }; done\nexec git \"$@\"\n",
    );
    let results = collect(&old, &[path(&dir)], &AtomicBool::new(false));
    assert_eq!(results[0].1, None);

    // A partial clone whose HEAD blob is missing; its promisor is a local repo, so a lazy fetch
    // would succeed without any network, and must not happen.
    git(&dir, &["config", "uploadpack.allowFilter", "true"]);
    let clone = temp.path().join("clone");
    let origin = format!("file://{}", path(&dir));
    git(
        temp.path(),
        &[
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &origin,
            &path(&clone),
        ],
    );
    git(&clone, &["read-tree", "HEAD"]);
    fs::write(clone.join("a.txt"), "changed\n").unwrap();
    let blob = Command::new("git")
        .args(["--no-lazy-fetch", "rev-parse", "HEAD:a.txt"])
        .current_dir(&clone)
        .output()
        .unwrap();
    let blob = String::from_utf8(blob.stdout).unwrap();
    let missing = || {
        !Command::new("git")
            .args(["--no-lazy-fetch", "cat-file", "-e", blob.trim()])
            .current_dir(&clone)
            .status()
            .unwrap()
            .success()
    };
    assert!(missing());
    let results = collect("git", &[path(&clone)], &AtomicBool::new(false));
    let summary = results[0].1.as_ref().unwrap();
    assert_eq!(summary.head, Head::Branch("main".into()));
    assert_eq!(summary.changes, None);
    assert!(missing(), "the summary fetched a missing object");
}

#[test]
fn poller_follows_the_watched_directories_and_quits_while_git_hangs() {
    let temp = common::tempdir();
    let (a, b) = (temp.path().join("a"), temp.path().join("b"));
    repo(&a, "main");
    repo(&b, "side");
    let mut poller = Poller::start("git".into(), Duration::from_millis(50));
    let wait_for = |poller: &Poller, cwd: &str| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let batch = poller
                .updates
                .recv_timeout(deadline - Instant::now())
                .unwrap();
            if batch.iter().any(|(c, _)| c == cwd) {
                return batch;
            }
        }
    };
    poller.watch(vec![path(&a)]);
    let batch = wait_for(&poller, &path(&a));
    assert_eq!(batch.len(), 1);
    poller.watch(vec![path(&b)]);
    let batch = wait_for(&poller, &path(&b));
    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch[0].1.as_ref().unwrap().head,
        Head::Branch("side".into())
    );

    let slow = common::script(temp.path(), "slow-git", "#!/bin/sh\nexec sleep 30\n");
    let mut poller = Poller::start(slow, Duration::from_millis(50));
    poller.watch(vec![path(&a)]);
    std::thread::sleep(Duration::from_millis(300));
    let start = Instant::now();
    drop(poller);
    assert!(start.elapsed() < Duration::from_secs(2));
    // A cancelled round reports nothing rather than claiming the directory is unavailable.
    let cancelled = collect("git", &[path(&a)], &AtomicBool::new(true));
    assert!(cancelled.is_empty());
}

#[test]
fn parent_summaries_never_enter_submodule_worktrees_but_keep_gitlink_changes() {
    let temp = common::tempdir();
    let origin = temp.path().join("origin");
    repo(&origin, "main");
    commit(&origin, ".gitattributes", "*.txt filter=subprobe\n");
    commit(&origin, "a.txt", "one\n");
    let parent = temp.path().join("parent");
    repo(&parent, "main");
    commit(&parent, "p.txt", "p\n");
    git(
        &parent,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            &path(&origin),
            "child",
        ],
    );
    git(&parent, &["commit", "-q", "-m", "child"]);
    let child = parent.join("child");
    set_old_mtime(&child.join("a.txt"));
    git(&child, &["update-index", "-q", "--refresh"]);
    // The filter exists only in the submodule's own config; the parent has none to override.
    let marker = temp.path().join("ran");
    let hook = common::script(
        temp.path(),
        "hook",
        &format!("#!/bin/sh\ntouch {marker:?}\ncat\n"),
    );
    git(&child, &["config", "filter.subprobe.clean", &hook]);
    fs::write(child.join("a.txt"), "two\n").unwrap();
    let summary = |dir: &Path| {
        collect("git", &[path(dir)], &AtomicBool::new(false))[0]
            .1
            .clone()
            .unwrap()
    };
    // Uncommitted files inside the submodule are not the parent's changes.
    assert_eq!(summary(&parent).changes, changes(0, 0, 0));
    assert!(!marker.exists(), "the submodule's filter ran");

    // A new submodule commit is still a gitlink change of the parent.
    fs::write(child.join("b.md"), "b\n").unwrap();
    git(&child, &["-c", "filter.subprobe.clean=cat", "add", "b.md"]);
    git(
        &child,
        &["-c", "filter.subprobe.clean=cat", "commit", "-q", "-m", "b"],
    );
    assert_eq!(summary(&parent).changes, changes(1, 1, 0));
    assert!(!marker.exists(), "the submodule's filter ran");

    // As its own cwd, the submodule is summarised with its own filters blocked.
    assert_eq!(summary(&child).changes, None);
    assert!(!marker.exists(), "the submodule's filter ran");
}
