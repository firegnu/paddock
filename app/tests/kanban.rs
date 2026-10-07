//! The Kanban tab's reads, in a throwaway repository with a task in each column: where each lands,
//! that ids are matched whole, that what cannot be told stays in an earlier column, and that an
//! agent moves its task as it should.
mod common;
use paddock::{
    agents::Status,
    kanban::{Board, Column, Read, Seen, board, cache, read},
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

/// A task file with its place, as the convention writes it.
fn task(title: &str, root: &Path, branch: &str, depends: Option<&str>) -> String {
    let worktree = root.join(branch);
    let depends = depends.map_or(String::new(), |d| format!("依赖：{d}\n"));
    format!(
        "# 任务：{title}\n\n2026-10-07，paddock/main 交给 paddock/dev-x。\n依据：x\n{depends}\n\
         ## 在哪里干活\n- worktree：`{}`，分支 `{branch}`（已从 main 建好）。\n",
        worktree.display()
    )
}

/// A column's ids in natural order: cards moved within one second keep no order between them.
fn ids(board: &Board, column: Column) -> Vec<&str> {
    let mut ids: Vec<&str> = board.cards(column).iter().map(|c| c.id.as_str()).collect();
    ids.sort_by(|a, b| paddock::kanban::natural(a, b));
    ids
}

#[test]
fn each_task_lands_in_its_column() {
    let temp = common::tempdir();
    let root = temp.path();
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, "README.md", "x\n", "start");
    let files = [
        ("P5-1-排队.md", task("排队的活", root, "p5-1", Some("P5-2"))),
        ("P5-2-在做.md", task("在做的活", root, "p5-2", None)),
        ("P5-3-待审.md", task("待审的活", root, "p5-3", None)),
        ("P5-24-已合并.md", task("已合并的活", root, "p5-24", None)),
        ("P5-24a-已收尾.md", task("已收尾的活", root, "p5-24a", None)),
        ("P5-5-分支已删.md", task("分支删了", root, "p5-5", None)),
        ("P5-6r-调研.md", task("调研", root, "p5-6r", None)),
        (
            "P5-7-合并没写.md",
            task("合并时没写编号", root, "p5-7", None),
        ),
        ("P5-8-刚开.md", task("刚开的分支", root, "p5-8", None)),
    ];
    for (file, text) in &files {
        commit(
            &repo,
            &format!("docs/任务/{file}"),
            text,
            &format!("{file}：任务文件"),
        );
    }
    let record = |text: &str| format!("{text}\n## 完成记录\n做完了。\n");
    // In progress: its worktree, a commit and an edit not committed.
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p5-2",
            root.join("p5-2").to_str().unwrap(),
        ],
    );
    commit(&root.join("p5-2"), "src/a.txt", "1\n2\n3\n", "p5-2 work");
    fs::write(root.join("p5-2/src/a.txt"), "1\n2\n3\n4\n5\n").unwrap();
    // To review: its completion record on its branch, no worktree.
    git(&repo, &["branch", "p5-3"]);
    git(&repo, &["checkout", "-q", "p5-3"]);
    commit(
        &repo,
        "docs/任务/P5-3-待审.md",
        &record(&files[2].1),
        "P5-3 完成记录",
    );
    git(&repo, &["checkout", "-q", "main"]);
    // Merged, its worktree still there.
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p5-24",
            root.join("p5-24").to_str().unwrap(),
        ],
    );
    commit(&root.join("p5-24"), "src/b.txt", "b\n", "p5-24 work");
    git(
        &repo,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "合并 P5-24：已合并的活",
            "p5-24",
        ],
    );
    // Done, wrapped up; `P5-24`'s merge does not count for it, nor its wrap-up for `P5-24`.
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "收尾: P5-24a 已收尾的活",
        ],
    );
    // A branch with work, deleted without being merged: nothing says where it went.
    git(&repo, &["branch", "p5-5"]);
    git(&repo, &["checkout", "-q", "p5-5"]);
    commit(&repo, "src/c.txt", "c\n", "p5-5 work");
    git(&repo, &["checkout", "-q", "main"]);
    git(&repo, &["branch", "-q", "-D", "p5-5"]);
    // Research: its document copied onto main by hand, not merged; the branch has the record.
    git(&repo, &["branch", "p5-6r"]);
    git(&repo, &["checkout", "-q", "p5-6r"]);
    commit(&repo, "docs/调研/x.md", "调研结论\n", "P5-6r 调研");
    commit(
        &repo,
        "docs/任务/P5-6r-调研.md",
        &record(&files[6].1),
        "P5-6r 完成记录",
    );
    git(&repo, &["checkout", "-q", "main"]);
    commit(&repo, "docs/调研/x.md", "调研结论\n", "摘 P5-6r 的调研文档");
    // Merged without the message saying so: its tip is in main, not on main's own line.
    git(&repo, &["branch", "p5-7"]);
    git(&repo, &["checkout", "-q", "p5-7"]);
    commit(&repo, "src/d.txt", "d\n", "p5-7 work");
    git(&repo, &["checkout", "-q", "main"]);
    git(
        &repo,
        &["merge", "-q", "--no-ff", "-m", "Merge branch p5-7", "p5-7"],
    );
    // A branch just made from main, its worktree open: in progress, not merged.
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p5-8",
            root.join("p5-8").to_str().unwrap(),
        ],
    );

    let cache = cache();
    // Read from a worktree: the board is the main repository's.
    let Read::Board(facts) = read(
        "git",
        root.join("p5-2").to_str().unwrap(),
        &cache,
        &AtomicBool::new(false),
    ) else {
        panic!("not a board");
    };
    assert_eq!(facts.name, "repo");
    assert_eq!(facts.repo, repo);
    let board = board(&facts, &[], 2_000_000_000.0);
    assert_eq!(ids(&board, Column::Queued), ["P5-1", "P5-5"]);
    assert_eq!(ids(&board, Column::InProgress), ["P5-2", "P5-8"]);
    assert_eq!(ids(&board, Column::ToReview), ["P5-3", "P5-6r"]);
    assert_eq!(ids(&board, Column::Merged), ["P5-7", "P5-24"]);
    assert_eq!(ids(&board, Column::Done), ["P5-24a"]);
    let card = |id: &str| {
        Column::ALL
            .iter()
            .flat_map(|c| board.cards(*c))
            .find(|c| c.id == id)
            .unwrap()
            .clone()
    };
    // Waiting for a task not yet done.
    assert_eq!(card("P5-1").state.unwrap().0, "Waits for P5-2");
    assert_eq!(card("P5-5").state, None);
    // Lines since main, committed and not.
    assert_eq!(card("P5-2").lines, Some((5, 0)));
    assert_eq!(card("P5-2").title, "在做的活");
    assert_eq!(card("P5-2").branch.as_deref(), Some("p5-2"));
    assert_eq!(
        card("P5-24").state.unwrap().0,
        "Merged · worktree still there"
    );
    assert_eq!(card("P5-7").state.unwrap().0, "Merged");
    // DONE is one line: no state, no branch, no agent.
    let done = card("P5-24a");
    assert_eq!((done.state, done.branch, done.agent), (None, None, None));
    assert_eq!(board.count(Column::Done, false), "1");

    // An agent labelled for a task: an idle one that said DONE sends it to review, a working
    // one in a worktree keeps it in progress; one for a queued task starts it.
    let agent = |name: &str, task: Option<&str>, cwd: Option<&Path>, status, said_done| Seen {
        name: name.into(),
        kind: Some("claude".into()),
        cwd: cwd.map(|p| p.to_str().unwrap().to_owned()),
        task: task.map(str::to_owned),
        status,
        said_done,
        since: None,
        controller: false,
    };
    let agents = [
        agent("paddock/dev-a-1", Some("P5-2"), None, Status::Idle, true),
        agent(
            "paddock/dev-b",
            None,
            Some(&root.join("p5-1")),
            Status::Working,
            false,
        ),
    ];
    let with = paddock::kanban::board(&facts, &agents, 2_000_000_000.0);
    assert_eq!(ids(&with, Column::ToReview), ["P5-2", "P5-3", "P5-6r"]);
    assert_eq!(ids(&with, Column::InProgress), ["P5-1", "P5-8"]);
    let p5_1 = with
        .cards(Column::InProgress)
        .iter()
        .find(|c| c.id == "P5-1")
        .unwrap();
    assert_eq!(p5_1.agent.as_ref().unwrap().name, "paddock/dev-b");
    assert_eq!(p5_1.state.as_ref().unwrap().0, "Working");
    // The controller, labelled so and in the main worktree, shows on the cards to review only;
    // one labelled so in a task's worktree is not this repository's.
    let controller = |name: &str, cwd: &Path| Seen {
        controller: true,
        ..agent(name, None, Some(cwd), Status::Idle, false)
    };
    let main = controller("paddock/main", &repo);
    let mut agents = agents.to_vec();
    agents.insert(0, controller("other/main", &root.join("p5-8")));
    agents.push(main.clone());
    let reviewed = paddock::kanban::board(&facts, &agents, 2_000_000_000.0);
    for column in Column::ALL {
        for card in reviewed.cards(column) {
            let want = (column == Column::ToReview).then_some(&main);
            assert_eq!(card.controller.as_ref(), want, "{}", card.id);
        }
    }
    assert_eq!(reviewed.cards(Column::ToReview).len(), 3);
    // The task files are read once: a second read finds them in the cache and agrees.
    let again = read(
        "git",
        repo.to_str().unwrap(),
        &cache,
        &AtomicBool::new(false),
    );
    assert_eq!(again, Read::Board(facts));
}

#[test]
fn done_keeps_the_last_five_and_counts_what_it_lists() {
    let temp = common::tempdir();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for n in 1..=7 {
        commit(
            &repo,
            &format!("docs/任务/P1-{n}-x.md"),
            &format!("# 任务：第 {n} 件\n"),
            "任务文件",
        );
    }
    for n in 1..=7 {
        // Commit times are whole seconds: set them apart.
        let date = format!("2026-10-0{n}T12:00:00");
        let output = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c"])
            .arg("commit.gpgsign=false")
            .args([
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                &format!("收尾: P1-{n} x"),
            ])
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    let Read::Board(facts) = read(
        "git",
        repo.to_str().unwrap(),
        &cache(),
        &AtomicBool::new(false),
    ) else {
        panic!("not a board");
    };
    let board = board(&facts, &[], 2_000_000_000.0);
    // The header counts what is listed, of all.
    assert_eq!(board.count(Column::Done, false), "5 / 7");
    assert_eq!(board.cards(Column::Done).len(), 7);
    // Newest first.
    let done: Vec<&str> = board
        .listed(Column::Done, false)
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(done, ["P1-7", "P1-6", "P1-5", "P1-4", "P1-3"]);
}

#[test]
fn quiet_states_without_a_board() {
    let temp = common::tempdir();
    let plain = temp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let read_at = |dir: &Path| {
        read(
            "git",
            dir.to_str().unwrap(),
            &cache(),
            &AtomicBool::new(false),
        )
    };
    assert_eq!(read_at(&plain), Read::NotRepository);
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "trunk"]);
    commit(&repo, "README.md", "x\n", "start");
    assert_eq!(
        read_at(&repo),
        Read::NoMain {
            name: "repo".into()
        }
    );
    git(&repo, &["branch", "-m", "main"]);
    assert_eq!(
        read_at(&repo),
        Read::NoTasks {
            name: "repo".into(),
            repo: repo.clone(),
        }
    );
    // Files that are not tasks do not make a board.
    commit(&repo, "docs/任务/说明.md", "x\n", "notes");
    assert_eq!(
        read_at(&repo),
        Read::NoTasks {
            name: "repo".into(),
            repo: repo.clone(),
        }
    );
}

#[test]
fn drafts_dropped_tasks_and_tasks_waiting_on_the_user() {
    let temp = common::tempdir();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(
        &repo,
        "docs/任务/P1-1-做完.md",
        "# 任务：做完的活\n依据：x\n待用户：实测一下\n",
        "任务文件",
    );
    commit(
        &repo,
        "docs/任务/P1-2-不做.md",
        "# 任务：不做的活\n",
        "任务文件",
    );
    commit(
        &repo,
        "docs/任务/P1-3-排队.md",
        "# 任务：排队的活\n",
        "任务文件",
    );
    git(
        &repo,
        &["commit", "-q", "--allow-empty", "-m", "收尾: P1-1 做完"],
    );
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "收尾: P1-2 不做：被 P1-3 取代",
        ],
    );
    // Drafts: one untracked, one staged, one without an id; an edit to a committed file is not.
    let tasks = repo.join("docs/任务");
    fs::write(tasks.join("P1-5-没跟踪.md"), "# 任务：没跟踪的草稿\n").unwrap();
    fs::write(tasks.join("P1-4-已暂存.md"), "# 任务：暂存的草稿\n").unwrap();
    git(&repo, &["add", "docs/任务/P1-4-已暂存.md"]);
    fs::write(tasks.join("想法.md"), "随手记\n").unwrap();
    fs::write(tasks.join("P1-3-排队.md"), "# 任务：改过的标题\n").unwrap();
    fs::write(tasks.join("notes.txt"), "x\n").unwrap();

    let Read::Board(facts) = read(
        "git",
        repo.to_str().unwrap(),
        &cache(),
        &AtomicBool::new(false),
    ) else {
        panic!("not a board");
    };
    let board = board(&facts, &[], 2_000_000_000.0);
    let queued: Vec<(&str, &str, bool)> = board
        .cards(Column::Queued)
        .iter()
        .map(|c| (c.id.as_str(), c.title.as_str(), c.draft))
        .collect();
    assert_eq!(
        queued,
        [
            ("", "想法", true),
            ("P1-4", "暂存的草稿", true),
            ("P1-5", "没跟踪的草稿", true),
            ("P1-3", "排队的活", false),
        ]
    );
    let done = |id: &str| {
        board
            .cards(Column::Done)
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .clone()
    };
    // Done, and still waiting on the user.
    assert!(done("P1-1").needs_you);
    assert_eq!(done("P1-1").asks.as_deref(), Some("实测一下"));
    assert_eq!(done("P1-1").dropped, None);
    // Dropped: in DONE, with the reason.
    assert_eq!(done("P1-2").dropped.as_deref(), Some("被 P1-3 取代"));
    assert!(!done("P1-2").needs_you);
    assert_eq!(board.need_you(), 1);

    // Drafts alone make a board.
    let fresh = temp.path().join("fresh");
    fs::create_dir_all(fresh.join("docs/任务")).unwrap();
    git(&fresh, &["init", "-q", "-b", "main"]);
    commit(&fresh, "README.md", "x\n", "start");
    fs::write(fresh.join("docs/任务/P1-1-x.md"), "# 任务：第一件\n").unwrap();
    let Read::Board(facts) = read(
        "git",
        fresh.to_str().unwrap(),
        &cache(),
        &AtomicBool::new(false),
    ) else {
        panic!("not a board");
    };
    assert_eq!(facts.tasks, []);
    assert_eq!(facts.drafts.len(), 1);
}

#[test]
fn a_new_draft_is_written_once_and_never_over_a_taken_id() {
    use paddock::kanban::{Place, Refused, create_draft, next_id, taken};
    let temp = common::tempdir();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, "docs/任务/P5-1-在main.md", "# 任务：a\n", "任务文件");
    // On a branch only, in a worktree of its own.
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "p5-2",
            temp.path().join("p5-2").to_str().unwrap(),
        ],
    );
    commit(
        &temp.path().join("p5-2"),
        "docs/任务/P5-2-只在分支.md",
        "# 任务：b\n",
        "分支上的任务文件",
    );
    // In the main worktree only, not committed.
    let tasks = repo.join("docs/任务");
    fs::write(tasks.join("P5-3-草稿.md"), "# 任务：c 原样\n").unwrap();
    let cancel = AtomicBool::new(false);
    let ids = taken("git", &repo, &cancel).unwrap();
    assert_eq!(ids["P5-1"], Place::Branch("main".into()));
    assert_eq!(ids["P5-2"], Place::Branch("p5-2".into()));
    assert_eq!(ids["P5-3"], Place::Worktree);
    assert_eq!(
        next_id(ids.keys().map(String::as_str)).as_deref(),
        Some("P5-4")
    );

    let create = |id: &str, title: &str| create_draft("git", &repo, id, title, &cancel);
    let refused = |id: &str, title: &str| create(id, title).unwrap_err().to_string();
    assert_eq!(refused("P5-1", "x"), "P5-1 is already on main");
    assert_eq!(refused("P5-2", "x"), "P5-2 is already on p5-2");
    assert_eq!(refused("P5-3", "x"), "P5-3 is already in the main worktree");
    assert_eq!(create("p5-4", "x"), Err(Refused::BadId));
    assert_eq!(create("P5-4-x", "x"), Err(Refused::BadId));
    assert_eq!(create("P5-4", "  "), Err(Refused::NoTitle));
    // Refusals leave the file there as it was, and write nothing.
    assert_eq!(
        fs::read_to_string(tasks.join("P5-3-草稿.md")).unwrap(),
        "# 任务：c 原样\n"
    );
    let listed = || {
        let mut names: Vec<String> = fs::read_dir(&tasks)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    };
    assert_eq!(listed(), ["P5-1-在main.md", "P5-3-草稿.md"]);

    // Written: the title line and an empty 「用户原话」, the name made safe, nothing staged.
    let path = create(" P5-4 ", " Kanban: 新建/草稿 ").unwrap();
    assert_eq!(path, tasks.join("P5-4-Kanban-新建-草稿.md"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "# 任务：Kanban: 新建/草稿\n\n## 用户原话\n"
    );
    let status = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&repo)
        .output()
        .unwrap();
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(
        status.lines().all(|line| line.starts_with("?? ")),
        "{status}"
    );
    // Once only: the same id again is taken now.
    assert_eq!(
        refused("P5-4", "again"),
        "P5-4 is already in the main worktree"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "# 任务：Kanban: 新建/草稿\n\n## 用户原话\n"
    );
}

#[test]
fn a_new_draft_makes_the_missing_task_folder_but_never_over_a_file() {
    use paddock::kanban::{create_draft, next_id, taken};
    let temp = common::tempdir();
    let cancel = AtomicBool::new(false);
    let status = |dir: &Path| {
        let out = Command::new("git")
            .args(["-c", "core.quotePath=false", "status", "--porcelain"])
            .arg("--untracked-files=all")
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };

    // No docs/ at all: no id to offer, and the folders are made for the draft.
    let repo = temp.path().join("bare");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, "README.md", "x\n", "start");
    let ids = taken("git", &repo, &cancel).unwrap();
    assert!(ids.is_empty());
    assert_eq!(next_id(ids.keys().map(String::as_str)), None);
    let path = create_draft("git", &repo, "P1-1", "第一件", &cancel).unwrap();
    assert_eq!(path, repo.join("docs/任务/P1-1-第一件.md"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "# 任务：第一件\n\n## 用户原话\n"
    );
    assert_eq!(status(&repo), "?? docs/任务/P1-1-第一件.md\n");
    // The draft alone makes a board now.
    assert!(matches!(
        read("git", repo.to_str().unwrap(), &cache(), &cancel),
        Read::Board(_)
    ));

    // docs/ there, docs/任务 a plain file: refused and said so, the file left as it was.
    let repo = temp.path().join("file");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, "docs/任务", "不是目录\n", "start");
    let refused = create_draft("git", &repo, "P1-1", "x", &cancel).unwrap_err();
    assert_eq!(refused.to_string(), "docs/任务 is a file, not a folder");
    assert_eq!(
        fs::read_to_string(repo.join("docs/任务")).unwrap(),
        "不是目录\n"
    );
    assert_eq!(status(&repo), "");

    // docs a plain file: the same, naming docs.
    let repo = temp.path().join("docs-file");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, "docs", "x\n", "start");
    let refused = create_draft("git", &repo, "P1-1", "x", &cancel).unwrap_err();
    assert_eq!(refused.to_string(), "docs is a file, not a folder");
    assert_eq!(fs::read_to_string(repo.join("docs")).unwrap(), "x\n");
}
