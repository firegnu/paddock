//! The Cairn tab's reads and its Adopt, through a fake `cairn` and made-up JSON (DESIGN §13
//! P5-55, P5-79); never the real one.
mod common;
use paddock::cairn::{
    Adopting, Body, Day, Fired, Found, Full, Note, PAGE, Read, Records, adopt, note, read, shown,
};
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

/// The clock the tab reads by: eight hours east of UTC, where that is 17:45 on Saturday.
fn east(_: i64) -> i64 {
    8 * 3600
}

/// A fake cairn that logs each call after the directory it ran in, and answers a command from the
/// files beside it: `<command>.out`, `<command>.err` and `<command>.code`, the command being its
/// first word, and for `show <ID>` `show-<ID>`.
fn cairn(dir: &Path) -> String {
    common::script(
        dir,
        "cairn",
        r##"#!/bin/sh
root=$(dirname "$0")
printf '%s|%s\n' "$(pwd -P)" "$*" >> "$root/calls"
name=$1
case "$2" in ''|--*) ;; *) name="$1-$2" ;; esac
if [ -f "$root/$name.err" ]; then cat "$root/$name.err" >&2; fi
if [ -f "$root/$name.out" ]; then cat "$root/$name.out"; fi
if [ -f "$root/$name.code" ]; then exit "$(cat "$root/$name.code")"; fi
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
    let at = |place: &Path| read(&program, "git", dir(place), PAGE, now(), &east, &none);
    let found = |place: &Path| match at(place) {
        Read::Found(found) => found,
        other => panic!("{other:?}"),
    };
    let put = |file: &str, text: &str| fs::write(bin.join(file), text).unwrap();
    let take = |file: &str| fs::remove_file(bin.join(file)).unwrap();

    // 1. No cairn command.
    let missing = bin.join("no-such-cairn");
    assert_eq!(
        read(dir(&missing), "git", dir(&repo), PAGE, now(), &east, &none),
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
            repo: Some(repo.clone()),
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
    // From here a cairn before 0.3.0, whose `list` knows no `--json` (the notes have their own
    // tests below).
    put("list.err", "error: unexpected argument '--json' found\n");
    put("list.code", "2");
    put("show.out", &show(&[]));
    assert_eq!(found(&repo).body, Body::NoRecords);

    // 7. Adopted, with records: the text from its first section on.
    put("show.out", &show(&["01M4K2R7T9XQ4C8NVB3HZD6WEA"]));
    assert_eq!(
        found(&repo).body,
        Body::Records(Records {
            text: shown(TEXT).to_owned(),
            notes: None,
        })
    );
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

    // Reading runs nothing but the three.
    let all = calls(&bin);
    assert!(
        all.iter().all(|call| call.ends_with("|status --json")
            || call.ends_with("|show --json")
            || call.ends_with("|list --json --limit 50")),
        "{all:?}"
    );
}

/// A record as `cairn list --json` gives one, with what the tab does not use left in.
fn record(
    id: &str,
    at: &str,
    agent: &str,
    branch: Option<&str>,
    kind: &str,
    summary: &str,
) -> serde_json::Value {
    let session = (agent != "local").then_some("bfb02923-1c0d-4e5f-9a6b-7c8d9e0f1a2b");
    serde_json::json!({
        "id": id,
        "created_at": at,
        "agent": agent,
        "session_id": session,
        "source_id": format!("{agent}:5f0c"),
        "line_path": "/r/ranch",
        "branch": branch,
        "kind": kind,
        "target_id": null,
        "deleted_at": null,
        "replaced_by": null,
        "retracted": false,
        "summary": summary,
    })
}

/// What `cairn list --json` prints.
fn list(total: u64, records: &[serde_json::Value]) -> String {
    serde_json::json!({ "total": total, "records": records }).to_string()
}

/// A fake cairn's directory and an adopted repository with both agents' hooks, whose next session
/// is given the records `next`.
fn adopted(root: &Path, next: &[&str]) -> (PathBuf, PathBuf, String) {
    let (bin, repo, _) = places(root);
    let program = cairn(&bin);
    fs::write(bin.join("status.out"), status(true, true, "adopted", 0)).unwrap();
    let show = serde_json::json!({ "text": TEXT, "record_ids": next, "omitted_sources": 0 });
    fs::write(bin.join("show.out"), show.to_string()).unwrap();
    (bin, repo, program)
}

#[test]
fn the_notes_are_listed_by_the_day_they_were_saved_on_the_newest_first() {
    let temp = common::tempdir();
    let (bin, repo, program) = adopted(temp.path(), &["01A", "01D"]);
    let none = AtomicBool::new(false);
    let notes = |limit: usize| match read(&program, "git", dir(&repo), limit, now(), &east, &none) {
        Read::Found(Found {
            body: Body::Records(Records { text, notes }),
            ..
        }) => {
            // The text the next session is given is kept beside them.
            assert_eq!(text, shown(TEXT));
            notes.unwrap()
        }
        other => panic!("{other:?}"),
    };
    let checkpoint = |id: &str, at: &str, agent: &str, branch: Option<&str>, summary: &str| {
        record(id, at, agent, branch, "checkpoint", summary)
    };
    let other = |id: &str, kind: &str| {
        record(
            id,
            "2026-10-10T09:00:00.000Z",
            "local",
            Some("main"),
            kind,
            "不画",
        )
    };
    let records = [
        checkpoint(
            "01A",
            "2026-10-10T09:44:15.999Z",
            "claude",
            Some("main"),
            "P5-78 看过，没问题。",
        ),
        // A correction, a retraction, a restore, and a kind not known yet: none is a note.
        other("01X", "correction"),
        checkpoint("01B", "2026-10-10T07:39:00.000Z", "codex", None, ""),
        other("01Y", "retraction"),
        // Half past midnight here is still yesterday in UTC.
        checkpoint(
            "01C",
            "2026-10-09T16:30:00.000Z",
            "claude",
            Some("p5-74-arrow"),
            "样稿已出，等用户选。",
        ),
        checkpoint(
            "01D",
            "2026-10-09T15:59:59.000Z",
            "local",
            Some("main"),
            "手动存的一条。",
        ),
        other("01Z", "restore"),
        checkpoint(
            "01E",
            "2026-10-07T01:02:03.000Z",
            "gemini",
            Some(""),
            "别家写的。",
        ),
        other("01W", "summary"),
        checkpoint(
            "01F",
            "2025-12-31T16:00:00.000Z",
            "claude",
            Some("main"),
            "今年第一条。",
        ),
        checkpoint(
            "01G",
            "2025-12-31T15:59:59.000Z",
            "claude",
            Some("main"),
            "去年最后一条。",
        ),
    ];
    fs::write(bin.join("list.out"), list(40, &records)).unwrap();

    let a_note = |id: &str, time: &str, ago: Option<&str>, by: &str, branch: Option<&str>| Note {
        id: id.into(),
        time: time.into(),
        ago: ago.map(str::to_owned),
        by: by.into(),
        branch: branch.map(str::to_owned),
        summary: String::new(),
        next: false,
    };
    let day = |name: Option<&'static str>, date: &str, notes: Vec<Note>| Day {
        name,
        date: date.into(),
        notes,
    };
    let listed = notes(PAGE);
    assert_eq!(
        listed.days,
        [
            // Today's say how long ago, as the Hooks line writes it; the two the next session is
            // given are marked.
            day(
                Some("Today"),
                "Sat, Oct 10",
                vec![
                    Note {
                        summary: "P5-78 看过，没问题。".into(),
                        next: true,
                        ..a_note("01A", "17:44", Some("45s ago"), "Claude", Some("main"))
                    },
                    a_note("01B", "15:39", Some("2.1h ago"), "Codex", None),
                    Note {
                        summary: "样稿已出，等用户选。".into(),
                        ..a_note(
                            "01C",
                            "00:30",
                            Some("17h ago"),
                            "Claude",
                            Some("p5-74-arrow")
                        )
                    },
                ]
            ),
            day(
                Some("Yesterday"),
                "Fri, Oct 9",
                vec![Note {
                    summary: "手动存的一条。".into(),
                    next: true,
                    ..a_note("01D", "23:59", None, "Manual", Some("main"))
                }]
            ),
            // Earlier days go by their date alone, with the year when it is not this one.
            day(
                None,
                "Wed, Oct 7",
                vec![Note {
                    summary: "别家写的。".into(),
                    ..a_note("01E", "09:02", None, "gemini", None)
                }]
            ),
            day(
                None,
                "Thu, Jan 1",
                vec![Note {
                    summary: "今年第一条。".into(),
                    ..a_note("01F", "00:00", None, "Claude", Some("main"))
                }]
            ),
            day(
                None,
                "Wed, Dec 31, 2025",
                vec![Note {
                    summary: "去年最后一条。".into(),
                    ..a_note("01G", "23:59", None, "Claude", Some("main"))
                }]
            ),
        ]
    );
    // cairn counts 40 and gave 11: there are older ones.
    assert!(listed.older);
    // Status, show, which takes in what was just saved, then the list, in the pane's directory.
    assert_eq!(
        calls(&bin),
        ["status --json", "show --json", "list --json --limit 50"]
            .map(|args| format!("{}|{args}", repo.display()))
    );

    // Show older asks for fifty more; with all of them given, there are none older.
    fs::write(bin.join("list.out"), list(11, &records)).unwrap();
    let all = notes(PAGE + 50);
    assert_eq!(all.days, listed.days);
    assert!(!all.older);
    assert_eq!(
        calls(&bin).last().unwrap(),
        &format!("{}|list --json --limit 100", repo.display())
    );

    // With nothing for the next session, the notes are listed all the same, none marked.
    let show = serde_json::json!({ "text": TEXT, "record_ids": [] }).to_string();
    fs::write(bin.join("show.out"), show).unwrap();
    let unmarked = notes(PAGE);
    assert_eq!(unmarked.days.len(), 5);
    assert!(
        unmarked
            .days
            .iter()
            .flat_map(|day| &day.notes)
            .all(|note| !note.next)
    );
}

#[test]
fn a_note_is_read_in_full_when_it_is_opened() {
    let temp = common::tempdir();
    let (bin, repo, program) = adopted(temp.path(), &["01A"]);
    let none = AtomicBool::new(false);
    let pane = repo.join("docs");
    let open = |id: &str| note(&program, dir(&pane), id, &east, &none);
    let put = |file: &str, text: &str| fs::write(bin.join(file), text).unwrap();
    let full = |id: &str, at: &str, agent: &str, branch: Option<&str>, body: Option<&str>| {
        let mut record = record(id, at, agent, branch, "checkpoint", "");
        let fields = record.as_object_mut().unwrap();
        fields.remove("summary");
        fields.insert("body".into(), body.into());
        fields.insert("correction".into(), serde_json::Value::Null);
        record
    };

    // Saved by an agent in a session: when, on this machine's clock, who, and the text as written.
    let body = "## 停点\nP5-78 看过，没问题。\n\n## 下一步（建议，非授权）\n- P5-54\n";
    let saved = full(
        "01A",
        "2026-10-10T09:44:51.250Z",
        "claude",
        Some("main"),
        Some(body),
    );
    put("show-01A.out", &saved.to_string());
    assert_eq!(
        open("01A"),
        Ok(Full {
            saved: "Oct 10, 2026 at 17:44:51".into(),
            by: "Claude".into(),
            session: Some("bfb02923".into()),
            branch: Some("main".into()),
            body: Some(body.into()),
            correction: None,
        })
    );
    // One command, in the pane's directory.
    assert_eq!(calls(&bin), [format!("{}|show 01A --json", pane.display())]);

    // Saved by hand, on no branch, its text since deleted, and corrected: the correction's own
    // time and text come with it.
    let mut corrected = full("01D", "2026-01-04T16:05:09.000Z", "local", None, None);
    corrected["correction"] = full(
        "01K",
        "2026-10-09T23:30:00.000Z",
        "local",
        None,
        Some("其实是 P5-76。"),
    );
    put("show-01D.out", &corrected.to_string());
    assert_eq!(
        open("01D"),
        Ok(Full {
            saved: "Jan 5, 2026 at 00:05:09".into(),
            by: "Manual".into(),
            session: None,
            branch: None,
            body: None,
            correction: Some((
                "Oct 10, 2026 at 07:30:00".into(),
                Some("其实是 P5-76。".into())
            )),
        })
    );

    // It cannot be read: the command with the record after it, and the first line of why.
    put("show-01B.err", "记录不存在：01B\nsecond line\n");
    put("show-01B.code", "1");
    assert_eq!(
        open("01B"),
        Err("cairn show 01B: 记录不存在：01B".to_owned())
    );
    for unreadable in ["{\"status\":\"no_data\"}", "not json", ""] {
        put("show-01C.out", unreadable);
        assert_eq!(
            open("01C"),
            Err("cairn show 01C: unexpected output".to_owned())
        );
    }
    let mut odd = full("01E", "last week", "claude", None, Some("x"));
    put("show-01E.out", &odd.to_string());
    assert_eq!(
        open("01E"),
        Err("cairn show 01E: unexpected output".to_owned())
    );
    odd["created_at"] = "2026-10-10T09:44:51.250Z".into();
    odd["correction"] = full("01K", "soon", "local", None, None);
    put("show-01E.out", &odd.to_string());
    assert_eq!(
        open("01E"),
        Err("cairn show 01E: unexpected output".to_owned())
    );
    let missing = bin.join("no-such-cairn");
    let why = note(dir(&missing), dir(&repo), "01A", &east, &none).unwrap_err();
    assert!(why.starts_with("cairn show 01A: "), "{why}");
}

#[test]
fn a_cairn_that_cannot_list_shows_the_text_alone_and_one_that_fails_is_trouble() {
    let temp = common::tempdir();
    let (bin, repo, program) = adopted(temp.path(), &["01A"]);
    let none = AtomicBool::new(false);
    let at = || read(&program, "git", dir(&repo), PAGE, now(), &east, &none);
    let body = || match at() {
        Read::Found(found) => found.body,
        other => panic!("{other:?}"),
    };
    let put = |file: &str, text: &str| fs::write(bin.join(file), text).unwrap();
    let take = |file: &str| fs::remove_file(bin.join(file)).unwrap();
    let one = record(
        "01A",
        "2026-10-10T09:44:15.999Z",
        "claude",
        Some("main"),
        "checkpoint",
        "P5-78 看过，没问题。",
    );

    // cairn before 0.3.0 leaves with 2 over `--json`: no trouble, the text as before and no list.
    put("list.err", "error: unexpected argument '--json' found\n");
    put("list.code", "2");
    assert_eq!(
        body(),
        Body::Records(Records {
            text: shown(TEXT).to_owned(),
            notes: None,
        })
    );

    // Leaving with 1 is trouble, like any other command's.
    put("list.err", "database is locked\nsecond line\n");
    put("list.code", "1");
    assert_eq!(at(), Read::Failed("cairn list: database is locked".into()));
    take("list.err");
    take("list.code");
    // So is output without its records, or a note whose time cannot be read.
    let unexpected = Read::Failed("cairn list: unexpected output".into());
    for unreadable in ["", "not json", "{\"total\":3}", "{\"records\":[]}"] {
        put("list.out", unreadable);
        assert_eq!(at(), unexpected);
    }
    let mut odd = one.clone();
    odd["created_at"] = "this morning".into();
    put("list.out", &list(1, &[odd.clone()]));
    assert_eq!(at(), unexpected);
    // What is not a note is not read that closely.
    odd["kind"] = "correction".into();
    put("list.out", &list(2, &[one.clone(), odd]));
    match body() {
        Body::Records(Records {
            notes: Some(notes), ..
        }) => assert_eq!(notes.days.len(), 1),
        other => panic!("{other:?}"),
    }

    // Nothing recorded at all: no text for the next session and no notes, whatever else cairn
    // counts. With no database yet the list is not asked for.
    let show = |ids: &[&str]| {
        put(
            "show.out",
            &serde_json::json!({ "text": TEXT, "record_ids": ids }).to_string(),
        )
    };
    show(&[]);
    put("list.out", &list(0, &[]));
    assert_eq!(body(), Body::NoRecords);
    let correction = record(
        "01X",
        "2026-10-10T09:00:00.000Z",
        "local",
        None,
        "correction",
        "",
    );
    put("list.out", &list(1, &[correction]));
    assert_eq!(body(), Body::NoRecords);
    calls(&bin);
    put("show.out", "{\"status\":\"no_data\"}");
    assert_eq!(body(), Body::NoRecords);
    assert_eq!(
        calls(&bin),
        ["status --json", "show --json"].map(|args| format!("{}|{args}", repo.display()))
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
        match read(&program, "git", dir(&repo), PAGE, now(), &east, &none) {
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
