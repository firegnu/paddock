//! The Inbox (DESIGN §13 P5-87): whom it lists and where, whether a reply can be typed, what a
//! fake corral is sent, and what counts as read.
mod common;
use paddock::{
    agents::{Panel, Status},
    attention::{self, Urgency},
    corral::{Agent, Client, Reply},
    inbox::{self, Group, Open, Row, Sent},
};
use std::sync::atomic::AtomicBool;

fn agent(name: &str, state: &str) -> Agent {
    Agent {
        name: name.into(),
        instance: Some("123".into()),
        state: Some(state.into()),
        ..Default::default()
    }
}

fn controller(name: &str, state: &str) -> Agent {
    let mut agent = agent(name, state);
    agent.labels.insert("role".into(), "controller".into());
    agent
}

/// The rows as `(group, agent)`, a failed read as `corral`.
fn listed(rows: &[Row]) -> Vec<(Group, &str)> {
    rows.iter()
        .map(|row| (row.group, row.agent.as_deref().unwrap_or("corral")))
        .collect()
}

/// Controllers idle, with a new reply, working and paused; other agents waiting, in error, with a
/// new reply and idle; and a controller waiting.
fn panel() -> Panel {
    let mut panel = Panel::default();
    panel.absorb(
        vec![
            controller("c/ask", "blocked"),
            controller("c/new", "idle"),
            Agent {
                paused: true,
                ..controller("c/paused", "idle")
            },
            controller("c/read", "idle"),
            controller("c/work", "working"),
            agent("w/ask", "blocked"),
            Agent {
                error: Some("status failed".into()),
                ..agent("w/broken", "idle")
            },
            agent("w/quiet", "idle"),
            agent("w/replied", "idle"),
        ],
        None,
        100.0,
    );
    panel
        .unread
        .extend(["c/new".to_owned(), "w/replied".to_owned()]);
    panel
}

#[test]
fn every_need_is_listed_but_only_controllers_replies() {
    let panel = panel();
    let rows = inbox::rows(&panel, Some("corral: timed out"), 100.0);
    assert_eq!(
        listed(&rows),
        [
            // Waiting, then in error, then the failed read, whatever the role.
            (Group::Needs, "c/ask"),
            (Group::Needs, "w/ask"),
            (Group::Needs, "w/broken"),
            (Group::Needs, "corral"),
            (Group::New, "c/new"),
            // The other controllers by name; the other agents not at all, new reply or not.
            (Group::Read, "c/paused"),
            (Group::Read, "c/read"),
            (Group::Read, "c/work"),
        ]
    );
    // The bell counts what the first two groups list, in the colour of the most urgent.
    let ringing = inbox::ringing(&panel, Some("corral: timed out"), 100.0);
    assert_eq!(attention::bell(&ringing), (5, Urgency::Needs));
    // Nothing to list: no group at all.
    let mut quiet = Panel::default();
    quiet.absorb(vec![agent("w/quiet", "idle")], None, 100.0);
    quiet.unread.insert("w/quiet".into());
    assert!(inbox::rows(&quiet, None, 100.0).is_empty());
    assert_eq!(
        attention::bell(&inbox::ringing(&quiet, None, 100.0)),
        (0, Urgency::Quiet)
    );
}

#[test]
fn while_open_a_row_keeps_its_group_and_new_replies_still_come_in() {
    let mut panel = panel();
    let mut open = Open::new(&panel, None, 100.0);
    // Opened on its first row.
    assert_eq!(open.selected().unwrap().agent.as_deref(), Some("c/ask"));
    open.select(3);
    assert_eq!(open.selected().unwrap().agent.as_deref(), Some("c/new"));
    // Its reply read, and the waiting ones answered in their panes: nothing moves.
    panel.unread.remove("c/new");
    let mut agents = panel.agents.clone();
    for a in &mut agents {
        if a.name.ends_with("/ask") {
            a.state = Some("working".into());
        }
    }
    panel.absorb(agents, None, 101.0);
    open.refresh(&panel, None, 101.0);
    let before = [
        (Group::Needs, "c/ask"),
        (Group::Needs, "w/ask"),
        (Group::Needs, "w/broken"),
        (Group::New, "c/new"),
        (Group::Read, "c/paused"),
        (Group::Read, "c/read"),
        (Group::Read, "c/work"),
    ];
    assert_eq!(listed(open.rows()), before);
    // A controller's turn ends: its new reply joins New replies, after the row already there, and
    // the selection stays on the row it was on.
    let mut agents = panel.agents.clone();
    for a in &mut agents {
        if a.name == "c/work" {
            a.state = Some("idle".into());
        }
    }
    panel.absorb(agents, None, 102.0);
    assert!(panel.unread.contains("c/work"));
    open.refresh(&panel, None, 102.0);
    assert_eq!(
        listed(open.rows()),
        [
            (Group::Needs, "c/ask"),
            (Group::Needs, "w/ask"),
            (Group::Needs, "w/broken"),
            (Group::New, "c/new"),
            (Group::New, "c/work"),
            (Group::Read, "c/paused"),
            (Group::Read, "c/read"),
        ]
    );
    assert_eq!(open.selected().unwrap().agent.as_deref(), Some("c/new"));
    // Moving stops at either end.
    open.step(-10);
    assert_eq!(open.index(), 0);
    open.step(10);
    assert_eq!(open.selected().unwrap().agent.as_deref(), Some("c/read"));
    // Closed and opened again, it is sorted as things stand: the worker that was asking is not a
    // controller and is gone, the read reply is under Read.
    assert_eq!(
        listed(&inbox::rows(&panel, None, 102.0)),
        [
            (Group::Needs, "w/broken"),
            (Group::New, "c/work"),
            (Group::Read, "c/ask"),
            (Group::Read, "c/new"),
            (Group::Read, "c/paused"),
            (Group::Read, "c/read"),
        ]
    );
}

#[test]
fn only_an_idle_agent_can_be_replied_to_and_the_box_says_why_not() {
    assert_eq!(inbox::composer(Some(Status::Idle)), Ok(()));
    for (status, why) in [
        (
            Status::Working,
            "Working… you can reply when this turn ends",
        ),
        (
            Status::Stalled,
            "Working… you can reply when this turn ends",
        ),
        (Status::Paused, "Paused… resume it to reply"),
        (
            Status::Waiting,
            "Can't reply here while a prompt is showing",
        ),
        (Status::Error, "Can't reply while it is in error"),
        (Status::Starting, "Starting… you can reply once it is ready"),
        (Status::Exited, "It's no longer running"),
    ] {
        assert_eq!(inbox::composer(Some(status)), Err(why), "{status:?}");
    }
    // No longer listed at all.
    assert_eq!(inbox::composer(None), Err("It's no longer running"));

    // As the cards show it: a paused agent takes nothing, whatever it was doing.
    let panel = panel();
    let status = |name: &str, sent: Option<&Sent>| {
        let agent = panel.agents.iter().find(|a| a.name == name).unwrap();
        inbox::status(&panel, agent, sent, 100.0)
    };
    assert_eq!(status("c/paused", None), Status::Paused);
    assert_eq!(status("c/read", None), Status::Idle);
    // Just replied to, before corral's next listing says so: it is working on it.
    let sent = Sent {
        text: "go on".into(),
        at: 100.0,
        instance: Some("123".into()),
    };
    assert_eq!(status("c/read", Some(&sent)), Status::Working);
    // Its next turn has ended, or it is another instance: the reply sent is behind it.
    let ended = Agent {
        state_started: Some(130.0),
        ..controller("c/read", "idle")
    };
    assert!(!sent.awaited(&ended));
    assert!(sent.awaited(&controller("c/read", "working")));
    let again = Agent {
        instance: Some("456".into()),
        ..controller("c/read", "working")
    };
    assert!(!sent.awaited(&again));
}

/// A fake corral that writes down what it was given, one argument in brackets a line, and answers
/// by the agent's name.
fn fake_corral(dir: &std::path::Path) -> String {
    common::script(
        dir,
        "corral",
        r##"#!/bin/sh
out="$(dirname "$0")/args"
printf '%s\n' "$#" > "$out"
for arg in "$@"; do printf '[%s]\n' "$arg" >> "$out"; done
case "$2" in
  p/main) echo '{"ok":true,"name":"p/main","instance":"123","confirmed":true}' ;;
  p/busy) echo '{"ok":false,"error":"not_idle","message":"p/busy is not idle","name":"p/busy","state":"working"}'; exit 7 ;;
  p/typing) echo '{"ok":false,"error":"human_active","message":"someone typed in an attached window recently"}'; exit 8 ;;
  p/gone) echo '{"ok":false,"error":"not_found","message":"p/gone is not running"}'; exit 2 ;;
  p/paused) echo '{"ok":false,"error":"paused","message":"p/paused is paused; corral resume p/paused first"}'; exit 10 ;;
  *) echo 'not json'; exit 1 ;;
esac
"##,
    )
}

#[test]
fn a_reply_goes_to_corral_send_as_typed_and_a_refusal_says_why() {
    let temp = common::tempdir();
    let client = Client::local(fake_corral(temp.path()));
    let cancel = AtomicBool::new(false);
    let text = "line one\n\"quoted\" 'single' $HOME `x` \\n\n  --force 第二行";
    assert_eq!(client.send("p/main", text, &cancel), Ok(()));
    // `send`, the full name and the text as one argument, untouched; nothing else, no `--force`.
    let args = std::fs::read_to_string(temp.path().join("args")).unwrap();
    assert_eq!(args, format!("3\n[send]\n[p/main]\n[{text}]\n"));

    let why = |name: &str| inbox::refusal(&client.send(name, "hello", &cancel).unwrap_err());
    assert_eq!(why("p/busy"), "It's busy — try again when this turn ends");
    assert_eq!(
        why("p/typing"),
        "You typed in its pane a moment ago — try again shortly"
    );
    assert_eq!(why("p/gone"), "It's no longer running");
    // Anything else: what corral said.
    assert_eq!(
        why("p/paused"),
        "p/paused is paused; corral resume p/paused first"
    );
    // No answer to read, or no corral to run: something is said, and not the reply back.
    let garbled = why("p/odd");
    assert!(
        !garbled.is_empty() && !garbled.contains("hello"),
        "{garbled}"
    );
    let missing = Client::local(temp.path().join("nowhere").to_str().unwrap());
    let none = inbox::refusal(&missing.send("p/main", "hello", &cancel).unwrap_err());
    assert!(!none.is_empty() && !none.contains("hello"), "{none}");
}

#[test]
fn a_reply_shown_is_read_and_the_bell_counts_what_is_left() {
    let mut panel = panel();
    let bell = |panel: &Panel| attention::bell(&inbox::ringing(panel, None, 100.0));
    // Two waiting, one in error, one controller's new reply.
    assert_eq!(bell(&panel), (4, Urgency::Needs));
    let mut open = Open::new(&panel, None, 100.0);
    // A row with nothing unread changes nothing.
    assert!(!inbox::read(&mut panel, open.selected().unwrap()));
    assert_eq!(bell(&panel), (4, Urgency::Needs));
    open.select(3);
    assert!(inbox::read(&mut panel, open.selected().unwrap()));
    assert!(!panel.unread.contains("c/new"));
    assert_eq!(bell(&panel), (3, Urgency::Needs));
    // Another agent's new reply never counted, and stays unread for its card and its tab.
    assert!(panel.unread.contains("w/replied"));

    // Only new replies: the accent; all read: quiet.
    let mut replies = Panel::default();
    replies.absorb(
        vec![controller("c/a", "idle"), controller("c/b", "idle")],
        None,
        100.0,
    );
    replies.unread.extend(["c/a".to_owned(), "c/b".to_owned()]);
    assert_eq!(bell(&replies), (2, Urgency::Replies));
    for row in inbox::rows(&replies, None, 100.0) {
        inbox::read(&mut replies, &row);
    }
    assert_eq!(bell(&replies), (0, Urgency::Quiet));
}

#[test]
fn the_last_reply_is_read_while_waiting_too_and_kept_while_the_agent_works_on() {
    let since = |name: &str, state: &str, since: f64| Agent {
        state_started: Some(since),
        ..controller(name, state)
    };
    let mut panel = Panel::default();
    let listing = |idle: Agent| vec![since("c/ask", "blocked", 90.0), idle];
    panel.absorb(listing(since("c/main", "idle", 95.0)), None, 100.0);
    let asked: Vec<String> = panel
        .replies_to_read(100.0)
        .into_iter()
        .map(|spell| {
            let text = format!("**Done** with `{}`. Next: the review.\n\n- one", spell.name);
            let name = spell.name.clone();
            panel.absorb_reply(
                spell,
                Some(Reply {
                    text,
                    at: Some(94.0),
                }),
            );
            name
        })
        .collect();
    assert_eq!(asked, ["c/ask", "c/main"]);
    let said = |panel: &Panel, name: &str, now: f64| {
        let agent = panel.agents.iter().find(|a| a.name == name).unwrap();
        panel
            .said(agent, now)
            .map(|said| (said.reply.text.clone(), said.reply.at, said.current))
    };
    let main = "**Done** with `c/main`. Next: the review.\n\n- one".to_owned();
    assert_eq!(
        said(&panel, "c/main", 100.0),
        Some((main.clone(), Some(94.0), true))
    );
    assert!(said(&panel, "c/ask", 100.0).is_some_and(|said| said.2));
    // The row's line: the reply's first sentence, its Markdown marks gone.
    assert_eq!(inbox::first_sentence(&main), "Done with c/main.");
    assert_eq!(
        inbox::first_sentence("## 结果\n\nP5-86 做完了，已合并；重开就能用。样子没人看过。"),
        "结果"
    );
    assert_eq!(
        inbox::first_sentence("P5-86 做完了，已合并；重开就能用。样子没人看过。"),
        "P5-86 做完了，已合并；重开就能用。"
    );
    assert_eq!(inbox::first_sentence("v1.2 is out\nmore"), "v1.2 is out");
    assert_eq!(inbox::first_sentence(""), "");
    // Replied to and working on it: the reply before stays, as an earlier one.
    panel.absorb(listing(since("c/main", "working", 101.0)), None, 102.0);
    assert!(panel.replies_to_read(102.0).is_empty());
    assert_eq!(
        said(&panel, "c/main", 102.0),
        Some((main, Some(94.0), false))
    );
    // What was sent to it stays on show under that reply while it works, and after its turn until
    // the reply of that turn is read; then it goes.
    let sent = Sent {
        text: "go on".into(),
        at: 100.5,
        instance: Some("123".into()),
    };
    let shown = |panel: &Panel, now: f64| {
        let agent = panel.agents.iter().find(|a| a.name == "c/main").unwrap();
        sent.shown(panel, agent, now)
    };
    assert!(shown(&panel, 102.0));
    panel.absorb(listing(since("c/main", "idle", 110.0)), None, 111.0);
    assert!(shown(&panel, 111.0));
    for spell in panel.replies_to_read(111.0) {
        panel.absorb_reply(spell, Some("Reviewed.".to_owned()));
    }
    assert!(!shown(&panel, 111.0));
    // Another instance under the name: nothing of the one before.
    let again = Agent {
        instance: Some("456".into()),
        ..since("c/main", "working", 113.0)
    };
    panel.absorb(listing(again), None, 114.0);
    assert_eq!(said(&panel, "c/main", 114.0), None);
    assert!(!shown(&panel, 114.0));
}

#[test]
fn what_the_kanban_loop_says_has_rows_of_its_own_while_it_says_it() {
    let panel = panel();
    let notice = |text: &str, needs: bool| inbox::Notice {
        repo: "/w/paddock".into(),
        name: "paddock".into(),
        text: text.into(),
        needs,
    };
    let stuck = notice("Stuck: P5-1 not wrapped up, controller idle 15m", true);
    let finished = notice("Loop finished · 2 done", false);
    let sources = [inbox::Source::here(&panel, None)];
    let rows = inbox::rows_of(&sources, &[stuck.clone(), finished.clone()], 100.0);
    let loop_rows: Vec<(usize, Group)> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.notice.is_some())
        .map(|(at, row)| (at, row.group))
        .collect();
    // What needs the user after the agents that do; what is only to be read after the new replies.
    assert_eq!(loop_rows, [(3, Group::Needs), (5, Group::New)]);
    assert!(
        rows.iter()
            .all(|row| row.notice.is_none() || row.agent.is_none())
    );
    // A loop row is not the failed read's, though neither has an agent.
    let failed = Row {
        group: Group::Needs,
        agent: None,
        machine: None,
        notice: None,
    };
    assert!(!rows[3].same(&failed));
    // Open, its rows stay while the loop says them, and go once it no longer does.
    let mut open = Open::of(&sources, &[stuck.clone(), finished.clone()], 100.0);
    open.select(5);
    open.refresh_of(&sources, &[stuck.clone(), finished.clone()], 101.0);
    assert_eq!(
        open.selected().and_then(|r| r.notice.clone()),
        Some(finished.clone())
    );
    open.refresh_of(&sources, std::slice::from_ref(&finished), 102.0);
    assert!(
        open.rows()
            .iter()
            .all(|row| row.notice.as_ref() != Some(&stuck))
    );
    assert_eq!(
        open.selected().and_then(|r| r.notice.clone()),
        Some(finished)
    );
}
