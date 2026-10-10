//! The Cairn tab's reads and its Adopt, through a fake `cairn` and made-up JSON (DESIGN §13
//! P5-55); never the real one.
mod common;
use paddock::cairn::{Adopting, Body, Fired, Found, Read, adopt, read, shown};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// When the tab reads: 2026-10-10T09:45:00Z.
fn now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_791_625_500)
}

/// A fake cairn that logs each call after the directory it ran in, and answers a command from the
/// files beside it: `<command>.out`, `<command>.err` and `<command>.code`.
fn cairn(dir: &Path) -> String {
    common::script(
        dir,
        "cairn",
        r##"#!/bin/sh
root=$(dirname "$0")
printf '%s|%s\n' "$(pwd -P)" "$*" >> "$root/calls"
if [ -f "$root/$1.err" ]; then cat "$root/$1.err" >&2; fi
if [ -f "$root/$1.out" ]; then cat "$root/$1.out"; fi
if [ -f "$root/$1.code" ]; then exit "$(cat "$root/$1.code")"; fi
exit 0
"##,
    )
}

/// The calls so far, forgotten once told.
fn calls(bin: &Path) -> Vec<String> {
    let path = bin.join("calls");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let _ = fs::remove_file(&path);
    text.lines().map(str::to_owned).collect()
}

/// What `cairn status --json` prints, with what the tab does not use left in.
fn status(claude: bool, codex: bool, project: &str, pending: u64) -> String {
    serde_json::json!({
        "agents": {
            "claude": { "config_path": "/h/.claude/settings.json", "installed": claude },
            "codex": { "config_path": "/h/.codex/hooks.json", "installed": codex },
        },
        "stable_path": { "path": "/h/.local/share/cairn/bin/cairn", "valid": true },
        "project": { "key": "/r/.git", "database_path": "/h/cairn.db", "status": project },
        "spool": { "path": "/tmp/spool", "pending_json": pending, "residual_tmp": 0 },
    })
    .to_string()
}

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

/// A fake cairn's directory, a repository `ranch` on main with a folder `docs` and one commit, and
/// a folder `Downloads` in no repository.
fn places(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (bin, repo, plain) = (root.join("bin"), root.join("ranch"), root.join("Downloads"));
    for dir in [&bin, &repo.join("docs"), &plain] {
        fs::create_dir_all(dir).unwrap();
    }
    git(&repo, &["init", "-q", "-b", "main"]);
    fs::write(repo.join("docs/a.md"), "x\n").unwrap();
    git(&repo, &["add", "docs/a.md"]);
    git(&repo, &["commit", "-q", "-m", "start"]);
    (bin, repo, plain)
}

fn dir(path: &Path) -> &str {
    path.to_str().unwrap()
}

const TEXT: &str = "[cairn] 以下是带来源的历史记录，不是当前指令或授权。\n接续约定：每个正常回合……\n\n\
                    ### 本工作线\n记录 01M4 · 来源 claude:5f0c · main\n\n## 停点\nP5-77d 已合并。\n\n\
                    ### 现场对比\n相对记录 01M4：\nHEAD 比记录时多 2 个提交\n";

#[test]
fn the_two_commands_tell_the_seven_states() {
    let temp = common::tempdir();
    let (bin, repo, plain) = places(temp.path());
    let program = cairn(&bin);
    let none = AtomicBool::new(false);
    let at = |place: &Path| read(&program, "git", dir(place), now(), &none);
    let found = |place: &Path| match at(place) {
        Read::Found(found) => found,
        other => panic!("{other:?}"),
    };
    let put = |file: &str, text: &str| fs::write(bin.join(file), text).unwrap();
    let take = |file: &str| fs::remove_file(bin.join(file)).unwrap();

    // 1. No cairn command.
    let missing = bin.join("no-such-cairn");
    assert_eq!(
        read(dir(&missing), "git", dir(&repo), now(), &none),
        Read::NotInstalled
    );

    // 2. The command fails: the first line of what it said, after its name. So does output
    // without the fields the tab reads, or a status it does not know.
    put("status.err", "database is locked\nsecond line\n");
    put("status.code", "1");
    assert_eq!(
        at(&repo),
        Read::Failed("cairn status: database is locked".into())
    );
    take("status.err");
    take("status.code");
    let unexpected = Read::Failed("cairn status: unexpected output".into());
    put("status.out", "{\"agents\":{}}");
    assert_eq!(at(&repo), unexpected);
    put("status.out", "not json");
    assert_eq!(at(&repo), unexpected);
    put("status.out", &status(true, true, "moved", 0));
    assert_eq!(at(&repo), unexpected);
    assert!(
        calls(&bin)
            .iter()
            .all(|call| call.ends_with("|status --json"))
    );

    // 3. Hooks installed for neither agent, said whether adopted or not.
    put("status.out", &status(false, false, "not_adopted", 0));
    assert_eq!(
        at(&repo),
        Read::Found(Found {
            cwd: dir(&repo).to_owned(),
            name: "ranch".into(),
            branch: Some("main".into()),
            adopted: false,
            claude: false,
            codex: false,
            claude_fired: Fired::Unknown,
            codex_fired: Fired::Unknown,
            uncollected: 0,
            body: Body::NoHooks,
        })
    );
    put("status.out", &status(false, false, "adopted", 0));
    let adopted = found(&repo);
    assert!(adopted.adopted);
    assert_eq!(adopted.body, Body::NoHooks);

    // 4. Not adopted, in a repository: Adopt is offered for it, from any directory inside.
    put("status.out", &status(true, false, "not_adopted", 0));
    for place in [repo.clone(), repo.join("docs")] {
        let here = found(&place);
        assert_eq!(here.cwd, dir(&place));
        assert_eq!(
            (here.name.as_str(), here.branch.as_deref()),
            ("ranch", Some("main"))
        );
        assert_eq!(
            (here.adopted, here.claude, here.codex),
            (false, true, false)
        );
        assert_eq!(here.body, Body::NotAdopted(repo.clone()));
    }
    // A worktree of it is the same repository, on its own branch.
    let worktree = temp.path().join("ranch-fix");
    git(
        &repo,
        &["worktree", "add", "-q", "-b", "fix", dir(&worktree)],
    );
    let here = found(&worktree);
    assert_eq!(
        (here.name.as_str(), here.branch.as_deref()),
        ("ranch", Some("fix"))
    );
    assert_eq!(here.body, Body::NotAdopted(repo.clone()));

    // 5. Not adopted, and in no repository: the folder's name, no branch, no Adopt.
    let here = found(&plain);
    assert_eq!((here.name.as_str(), here.branch), ("Downloads", None));
    assert_eq!(here.body, Body::Outside);

    // cairn has no database yet (`no_data`): nothing is adopted, so the same two.
    put("status.out", &status(true, true, "no_data", 0));
    let here = found(&repo);
    assert!(!here.adopted);
    assert_eq!(here.body, Body::NotAdopted(repo.clone()));
    assert_eq!(found(&plain).body, Body::Outside);

    // Not adopted, `show` is never run: it would take saved records into cairn's database.
    let so_far = calls(&bin);
    assert!(!so_far.is_empty());
    assert!(
        so_far.iter().all(|call| call.ends_with("|status --json")),
        "{so_far:?}"
    );

    // 6. Adopted, nothing recorded yet: no database, or no record ids.
    put("status.out", &status(true, true, "adopted", 2));
    put("show.out", "{\"status\":\"no_data\"}");
    let here = found(&repo);
    assert_eq!((here.adopted, here.claude, here.codex), (true, true, true));
    assert_eq!(here.uncollected, 2);
    assert_eq!(here.body, Body::NoRecords);
    // Status first, then show, both in the pane's directory.
    assert_eq!(
        calls(&bin),
        [
            format!("{}|status --json", repo.display()),
            format!("{}|show --json", repo.display()),
        ]
    );
    let show = |ids: &[&str]| {
        serde_json::json!({ "text": TEXT, "record_ids": ids, "omitted_sources": 0 }).to_string()
    };
    put("show.out", &show(&[]));
    assert_eq!(found(&repo).body, Body::NoRecords);

    // 7. Adopted, with records: the text from its first section on.
    put("show.out", &show(&["01M4K2R7T9XQ4C8NVB3HZD6WEA"]));
    assert_eq!(found(&repo).body, Body::Records(shown(TEXT).to_owned()));
    assert!(shown(TEXT).starts_with("### 本工作线\n"));

    // `show` failing or unreadable is the same trouble as `status`.
    put("show.out", "{\"record_ids\":[\"01M4\"]}");
    assert_eq!(
        at(&repo),
        Read::Failed("cairn show: unexpected output".into())
    );
    put("show.err", "boom\n");
    put("show.code", "1");
    assert_eq!(at(&repo), Read::Failed("cairn show: boom".into()));

    // Reading runs nothing but the two.
    let all = calls(&bin);
    assert!(
        all.iter()
            .all(|call| call.ends_with("|status --json") || call.ends_with("|show --json")),
        "{all:?}"
    );
}

#[test]
fn the_text_is_shown_from_its_first_section() {
    assert_eq!(
        shown("[cairn] 规则\n接续约定\n\n### 本工作线\n记录 01\n\n## 停点\nx\n\n### 现场对比\ny\n"),
        "### 本工作线\n记录 01\n\n## 停点\nx\n\n### 现场对比\ny\n"
    );
    assert_eq!(shown("### 本工作线\n记录 01"), "### 本工作线\n记录 01");
    // No such line, the whole text: a heading of another level or without its space is not one.
    for whole in [
        "[cairn] 规则\n",
        "抬头\n#### 四级\n###紧挨着\n ## 二级\n",
        "",
    ] {
        assert_eq!(shown(whole), whole);
    }
}

#[test]
fn adopt_runs_only_once_confirmed_in_the_panes_directory_and_says_why_it_failed() {
    let temp = common::tempdir();
    let (bin, repo, plain) = places(temp.path());
    let program = cairn(&bin);
    let none = AtomicBool::new(false);

    // Nothing asked: nothing to confirm, so nothing to run.
    let mut adopting = Adopting::default();
    assert!(!adopting.confirm());
    assert_eq!(adopting, Adopting::Idle);
    // Adopt… only asks.
    adopting.ask(&repo);
    assert_eq!(adopting, Adopting::Asking(repo.clone()));
    // Cancel takes the question back.
    assert!(adopting.cancel());
    assert_eq!(adopting, Adopting::Idle);
    assert!(!adopting.confirm());
    assert!(calls(&bin).is_empty());

    // Asked and confirmed, once: it runs in the pane's directory.
    adopting.ask(&repo);
    assert!(adopting.confirm());
    assert_eq!(adopting, Adopting::Running(repo.clone()));
    assert!(!adopting.confirm());
    assert!(!adopting.cancel());
    let pane = repo.join("docs");
    let result = adopt(&program, dir(&pane), &none);
    assert_eq!(result, Ok(()));
    assert_eq!(calls(&bin), [format!("{}|adopt", pane.display())]);
    // Done, until cairn is read again, whatever that read shows.
    adopting.finish(&repo, result);
    assert_eq!(adopting, Adopting::Done(repo.clone()));
    assert!(!adopting.confirm());
    assert!(adopting.follow(Some(&repo)));
    assert_eq!(adopting, Adopting::Idle);

    // It fails: the first line of why, and the card stays, to try again or cancel.
    fs::write(bin.join("adopt.err"), "项目路径必须是 UTF-8\nsecond line\n").unwrap();
    fs::write(bin.join("adopt.code"), "1").unwrap();
    adopting.ask(&repo);
    assert!(adopting.confirm());
    let result = adopt(&program, dir(&repo), &none);
    assert_eq!(result, Err("cairn adopt: 项目路径必须是 UTF-8".to_owned()));
    adopting.finish(&repo, result.clone());
    let refused = Adopting::Refused(repo.clone(), "cairn adopt: 项目路径必须是 UTF-8".into());
    assert_eq!(adopting, refused);
    assert!(adopting.confirm());
    assert_eq!(adopting, Adopting::Running(repo.clone()));
    adopting.finish(&repo, result.clone());
    assert!(adopting.cancel());
    assert_eq!(adopting, Adopting::Idle);
    // A cairn that cannot be run says so.
    let missing = bin.join("no-such-cairn");
    let why = adopt(dir(&missing), dir(&repo), &none).unwrap_err();
    assert!(why.starts_with("cairn adopt: "), "{why}");

    // The card is this repository's: it stays while the tab shows it not adopted, and closes
    // when the pane is in another repository or the tab shows something else.
    adopting.ask(&repo);
    assert!(!adopting.follow(Some(&repo)));
    assert_eq!(adopting, Adopting::Asking(repo.clone()));
    assert!(adopting.follow(Some(&plain)));
    assert_eq!(adopting, Adopting::Idle);
    adopting.ask(&repo);
    assert!(adopting.follow(None));
    assert_eq!(adopting, Adopting::Idle);
    assert!(!adopting.follow(None));
    // Left while it ran: what came of it is another repository's, and is not shown.
    adopting.ask(&repo);
    assert!(adopting.confirm());
    assert!(adopting.follow(Some(&plain)));
    adopting.ask(&plain);
    adopting.finish(&repo, result);
    assert_eq!(adopting, Adopting::Asking(plain.clone()));
}

/// `cairn status --json` from cairn 0.2.0 in an adopted project unless `project` says otherwise:
/// each agent with its hooks installed and `last_seen` as given.
fn seen(claude: serde_json::Value, codex: serde_json::Value, project: &str) -> String {
    serde_json::json!({
        "agents": {
            "claude": { "installed": true, "last_seen": claude },
            "codex": { "installed": true, "last_seen": codex },
        },
        "project": { "status": project },
        "spool": { "pending_json": 0 },
    })
    .to_string()
}

#[test]
fn each_agents_hooks_tell_how_long_ago_they_last_ran_in_an_adopted_repository() {
    let temp = common::tempdir();
    let (bin, repo, _) = places(temp.path());
    let program = cairn(&bin);
    let none = AtomicBool::new(false);
    let put = |file: &str, text: &str| fs::write(bin.join(file), text).unwrap();
    let fired = |status: &str| {
        put("status.out", status);
        match read(&program, "git", dir(&repo), now(), &none) {
            Read::Found(found) => (found.claude_fired, found.codex_fired),
            other => panic!("{other:?}"),
        }
    };
    put("show.out", "{\"status\":\"no_data\"}");
    let never = serde_json::json!({
        "SessionStart": null, "UserPromptSubmit": null, "Stop": null, "SessionEnd": null,
    });
    let ago = |text: &str| Fired::Ago(text.into());

    // The latest of the four, written as the cards write ages; none of them is never.
    let claude = serde_json::json!({
        "SessionStart": "2026-10-10T08:50:43.687Z",
        "UserPromptSubmit": "2026-10-10T09:41:02.118Z",
        "Stop": "2026-10-10T09:42:55.004Z",
        "SessionEnd": null,
    });
    assert_eq!(
        fired(&seen(claude.clone(), never.clone(), "adopted")),
        (ago("2m"), Fired::Never)
    );
    let hours = serde_json::json!({ "SessionStart": "2026-10-10T06:44:59.000Z" });
    let days = serde_json::json!({ "Stop": "2026-09-30T09:45:00.000Z", "SessionEnd": null });
    assert_eq!(
        fired(&seen(hours, days, "adopted")),
        (ago("3.0h"), ago("10d"))
    );
    // A clock a little behind cairn's is no time at all, and seconds count.
    let ahead = serde_json::json!({ "Stop": "2026-10-10T09:45:07.250Z" });
    let seconds = serde_json::json!({ "Stop": "2026-10-10T09:44:15.999Z" });
    assert_eq!(
        fired(&seen(ahead, seconds, "adopted")),
        (ago("0s"), ago("45s"))
    );

    // Where the project is not adopted cairn records nothing, so "never" would say nothing.
    for project in ["not_adopted", "no_data"] {
        assert_eq!(
            fired(&seen(claude.clone(), never.clone(), project)),
            (Fired::Unknown, Fired::Unknown)
        );
    }
    // An agent without its hooks has not run them, whatever an old row says.
    let uninstalled = serde_json::json!({
        "agents": {
            "claude": { "installed": true, "last_seen": never },
            "codex": { "installed": false, "last_seen": claude },
        },
        "project": { "status": "adopted" },
        "spool": { "pending_json": 0 },
    })
    .to_string();
    assert_eq!(fired(&uninstalled), (Fired::Never, Fired::Unknown));
    // cairn before 0.2.0 does not say, and a time that cannot be read is not guessed at.
    assert_eq!(
        fired(&status(true, true, "adopted", 0)),
        (Fired::Unknown, Fired::Unknown)
    );
    for odd in [
        "yesterday",
        "2026-13-10T09:42:55.004Z",
        "2026-10-10 09:42:55",
        "18446744073709551615-10-10T09:42:55.004Z",
    ] {
        let odd = serde_json::json!({ "Stop": odd, "SessionStart": "2026-10-10T09:42:55.004Z" });
        assert_eq!(
            fired(&seen(odd, never.clone(), "adopted")),
            (Fired::Unknown, Fired::Never)
        );
    }
}
