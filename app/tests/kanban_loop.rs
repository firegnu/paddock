//! The Kanban Loop against a throwaway repository and a fake `corral`: a task is handed over, the
//! next only once a wrap-up commit for it is on main, and with nothing left the loop turns itself
//! off; what it holds is read back as paddock would on opening again. Never the real corral.
mod common;
use paddock::{
    agents::Status,
    corral::Client,
    kanban::{Board, Read, Seen, board, cache, read},
    kanban_loop::{self, Delivery, Loop, Loops, Stand, Timing},
};
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

fn board_of(repo: &Path, agents: &[Seen], now: f64) -> Board {
    let no = AtomicBool::new(false);
    match read("git", repo.to_str().unwrap(), &cache(), &no) {
        Read::Board(facts) => board(&facts, agents, now),
        other => panic!("{other:?}"),
    }
}

const T: Timing = Timing {
    stuck: 900.0,
    retry: 30.0,
};

#[test]
fn tasks_go_one_after_another_and_the_loop_turns_itself_off() {
    let temp = common::tempdir();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for (file, title) in [
        ("P1-1-甲.md", "甲"),
        ("P1-2-乙.md", "乙"),
        ("P1-3-丙.md", "丙"),
    ] {
        let text = format!("# 任务：{title}\n\n依据：x\n");
        commit(
            &repo,
            &format!("docs/任务/{file}"),
            &text,
            &format!("任务: {file}"),
        );
    }
    // The fake corral answers a send, keeping each argument on a line.
    let calls = temp.path().join("calls");
    let corral = common::script(
        temp.path(),
        "corral",
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done >> '{}'\n\
             echo '{{\"ok\":true,\"merged_with_draft\":false}}'\n",
            calls.display()
        ),
    );
    let client = Client::local(corral);
    let no = AtomicBool::new(false);
    let main = Seen {
        name: "repo/main".into(),
        kind: Some("claude".into()),
        cwd: Some(repo.display().to_string()),
        task: None,
        status: Status::Idle,
        said_done: false,
        since: Some(0.0),
        controller: true,
    };
    let agents = [main];
    let kept = temp.path().join("state/kanban-loops.json");
    let mut loops = Loops::load(Some(kept.clone()));
    let first = board_of(&repo, &agents, 100.0);
    let key = first.repo.clone();
    // P1-2 first, then P1-1, as the user ordered them; P1-3 never put in.
    let mut lp = Loop::default();
    assert!(lp.add("P1-2") && lp.add("P1-1") && lp.turn(true));
    loops.put(&key, lp);

    // One round as the board does it: settle, look, and hand over what it says to.
    let round = |loops: &mut Loops, now: f64| -> Stand {
        let board = board_of(&repo, &agents, now);
        let mut lp = loops.get(&key);
        kanban_loop::settle(&mut lp, &board);
        let stand = kanban_loop::look(&lp, &board, &agents, now, &T, None, false);
        if let Stand::Send {
            id,
            title,
            file,
            to,
        } = &stand
        {
            let words = kanban_loop::start_words(id, title, file);
            assert_eq!(
                kanban_loop::deliver(&client, to, &words, &no),
                Delivery::Taken { merged: false }
            );
            lp.tasks.retain(|t| t != id);
            lp.sent = Some(kanban_loop::Sent {
                id: id.clone(),
                at: now,
                nudged: None,
            });
        }
        loops.put(&key, lp);
        stand
    };

    assert!(matches!(round(&mut loops, 100.0), Stand::Send { ref id, .. } if id == "P1-2"));
    // Not wrapped up yet: nothing more goes, whatever the controller replies.
    assert_eq!(
        round(&mut loops, 110.0),
        Stand::Working {
            id: "P1-2".into(),
            begun: false
        }
    );
    // Read back as on opening again: as it was.
    let mut loops = Loops::load(Some(kept.clone()));
    assert_eq!(loops.get(&key).sent.map(|s| s.id), Some("P1-2".into()));
    assert_eq!(loops.get(&key).tasks, ["P1-1"]);
    git(
        &repo,
        &["commit", "-q", "--allow-empty", "-m", "收尾: P1-2 做完"],
    );
    assert!(matches!(round(&mut loops, 120.0), Stand::Send { ref id, .. } if id == "P1-1"));
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "收尾: P1-1 不做：重复",
        ],
    );
    // Both wrapped up, one dropped: off, finished, two done.
    let stand = round(&mut loops, 130.0);
    assert_eq!(
        stand,
        Stand::Off {
            tasks: 0,
            finished: Some(2)
        }
    );
    assert_eq!(stand.words(0, &T), "Loop finished · 2 done");
    let after = Loops::load(Some(kept));
    assert!(after.on().is_empty());
    assert_eq!(after.get(&key).finished, Some(2));
    // Two sends, in order, to the controller by name, never forced.
    let said = fs::read_to_string(&calls).unwrap();
    let expected = format!(
        "send\nrepo/main\n{}\nsend\nrepo/main\n{}\n",
        kanban_loop::start_words("P1-2", "乙", "P1-2-乙.md"),
        kanban_loop::start_words("P1-1", "甲", "P1-1-甲.md"),
    );
    assert_eq!(said, expected);
}

#[test]
fn what_leaves_queued_in_git_leaves_the_loop_and_a_task_started_otherwise_holds_it() {
    let temp = common::tempdir();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for id in ["P1-1", "P1-2", "P1-3", "P1-4"] {
        let text = format!(
            "# 任务：{id}\n\n依据：x\n\n## 在哪里干活\n- worktree：`{}`，分支 `{}`（已从 main 建好）。\n",
            temp.path().join(id).display(),
            id.to_lowercase()
        );
        commit(&repo, &format!("docs/任务/{id}-x.md"), &text, id);
    }
    // P1-1 wrapped up, P1-2's file gone, P1-3 started by hand, P1-4 still queued.
    git(
        &repo,
        &["commit", "-q", "--allow-empty", "-m", "收尾: P1-1 做完"],
    );
    git(&repo, &["rm", "-q", "docs/任务/P1-2-x.md"]);
    git(&repo, &["commit", "-q", "-m", "P1-2 删掉"]);
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p1-3",
            temp.path().join("P1-3").to_str().unwrap(),
        ],
    );
    let board = board_of(&repo, &[], 100.0);
    let mut lp = Loop::default();
    for id in ["P1-1", "P1-2", "P1-3", "P1-4"] {
        lp.add(id);
    }
    lp.turn(true);
    assert!(kanban_loop::settle(&mut lp, &board).changed);
    assert_eq!(lp.tasks, ["P1-3", "P1-4"]);
    let main = Seen {
        name: "repo/main".into(),
        kind: None,
        cwd: Some(repo.display().to_string()),
        task: None,
        status: Status::Idle,
        said_done: false,
        since: None,
        controller: true,
    };
    assert_eq!(
        kanban_loop::look(&lp, &board, &[main], 100.0, &T, None, false),
        Stand::Busy { id: "P1-3".into() }
    );
}
