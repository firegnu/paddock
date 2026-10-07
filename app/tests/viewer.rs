//! From Saddle `tests/viewer.rs` at commit `df1c727`. The fake `corral` programs are shell scripts
//! instead of Python (Saddle's second test used its `fixtures/corral.py`; this one does only what
//! that test needs: log each call, hold `status` while `hold-status` exists, attach until SIGINT).
mod common;
use paddock::{terminal::Size, viewer::Viewer};
use std::{
    thread,
    time::{Duration, Instant},
};

#[test]
fn choosing_current_agent_again_during_detach_cancels_the_obsolete_switch() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r#"#!/bin/sh
root=$(dirname "$0")
trap 'sleep 0.1; exit 0' INT
echo "$2" >> "$root/events"
printf READY
while :; do sleep 1 & wait $!; done
"#,
    );
    let size = Size { rows: 10, cols: 40 };
    let mut viewer = Viewer::new(program);
    viewer.select("p/a".into()).unwrap();
    viewer.tick(size).unwrap();
    let events = temp.path().join("events");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !events.exists() || viewer.showing.as_deref() != Some("p/a") {
        assert!(Instant::now() < deadline);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    viewer.select("p/b".into()).unwrap();
    viewer.select("p/a".into()).unwrap();
    // Also wait for the viewer to publish the new attach: the shell fake starts faster than Saddle's
    // Python one did, so its second event line can land before that tick.
    while std::fs::read_to_string(&events).unwrap().lines().count() < 2
        || viewer.showing.as_deref() != Some("p/a")
    {
        assert!(Instant::now() < deadline);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(viewer.showing.as_deref(), Some("p/a"));
    assert_eq!(std::fs::read_to_string(events).unwrap(), "p/a\np/a\n");
}

#[test]
fn t25_closed_during_identity_check_never_publishes_a_session() {
    use paddock::viewer::AgentMetadata;
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r#"#!/bin/sh
root=$(dirname "$0")
echo "$*" >> "$root/events"
case "$1" in
  status)
    while [ -e "$root/hold-status" ]; do sleep 0.02; done
    echo "{\"ok\":true,\"name\":\"$2\",\"state\":\"idle\",\"instance\":\"abcdef123\",\"attached\":0}"
    ;;
  attach)
    trap 'exit 0' INT
    printf READY
    while :; do sleep 1 & wait $!; done
    ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"#,
    );
    std::fs::write(temp.path().join("hold-status"), "").unwrap();
    let mut viewer = Viewer::new(program);
    viewer
        .select_agent(
            "p/a".into(),
            AgentMetadata {
                cwd: Some(temp.path().display().to_string()),
                instance: Some("abcdef123".into()),
            },
        )
        .unwrap();
    let size = Size { rows: 10, cols: 40 };
    viewer.tick(size).unwrap();
    let events = || std::fs::read_to_string(temp.path().join("events")).unwrap_or_default();
    let deadline = Instant::now() + Duration::from_secs(4);
    while !events().contains("status p/a") {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    viewer.close().unwrap();
    std::fs::remove_file(temp.path().join("hold-status")).unwrap();
    while !viewer.closed() {
        assert!(Instant::now() < deadline, "{}", events());
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    assert!(viewer.session.is_none());
    assert!(viewer.showing.is_none());
    assert!(!events().contains("stop "));
}

#[test]
fn a_paused_agent_gets_no_input_and_none_is_kept_for_later() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r#"#!/bin/sh
root=$(dirname "$0")
case "$1" in
  attach)
    stty raw -echo
    printf READY
    exec cat > "$root/typed"
    ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"#,
    );
    let size = Size { rows: 10, cols: 40 };
    let mut viewer = Viewer::new(program);
    viewer.select("p/a".into()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let ready = |viewer: &Viewer| {
        viewer.session.as_ref().is_some_and(|session| {
            let screen = session.screen.lock().unwrap();
            let text: String = screen.term.grid().display_iter().map(|c| c.c).collect();
            text.contains("READY")
        })
    };
    while viewer.showing.as_deref() != Some("p/a") || !ready(&viewer) {
        assert!(Instant::now() < deadline);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    // Paused: what is typed goes nowhere.
    viewer.paused = true;
    assert!(!viewer.send(b"lost".to_vec()).unwrap());
    // Resumed: typing reaches the agent again, and nothing typed while paused comes with it.
    viewer.paused = false;
    assert!(viewer.send(b"kept".to_vec()).unwrap());
    let typed = temp.path().join("typed");
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::fs::read_to_string(&typed).unwrap_or_default() != "kept" {
        assert!(
            Instant::now() < deadline,
            "{:?}",
            std::fs::read_to_string(&typed)
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn input_flows_again_once_the_pane_shows_something_else() {
    use paddock::viewer::Shell;
    let mut viewer = Viewer::new("unused".into());
    viewer.select("p/a".into()).unwrap();
    viewer.paused = true;
    // Choosing it again keeps it paused; another agent, a shell or nothing takes input.
    viewer.select("p/a".into()).unwrap();
    assert!(viewer.paused);
    viewer.select("p/b".into()).unwrap();
    assert!(!viewer.paused);
    viewer.paused = true;
    viewer.start_shell(Shell {
        program: "/bin/sh".into(),
        cwd: "/".into(),
        state: "starting",
        exit_code: None,
        env: Vec::new(),
    });
    assert!(!viewer.paused);
    viewer.paused = true;
    viewer.close().unwrap();
    assert!(!viewer.paused);
}

/// `paddock --attach NAME` (`Launch::Agent`): the pane's agent can be the caller of `paddock ctl`,
/// known by the instance public corral gave before the attach.
#[test]
fn an_agent_attached_at_startup_is_known_by_its_instance() {
    use paddock::{
        control::Caller,
        control_ui::{Facts, Listed, Model, caller_pane},
        layout::{Shown, Workspace},
        viewer::public_metadata,
    };
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r#"#!/bin/sh
case "$1" in
  status) echo '{"ok":true,"name":"p/start","state":"idle","attached":0,"instance":"i-start"}' ;;
  attach) trap 'exit 0' INT; printf READY; while :; do sleep 1 & wait $!; done ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"#,
    );
    let size = Size { rows: 10, cols: 40 };
    let mut viewer = Viewer::new(program.clone());
    viewer
        .select_agent("p/start".into(), public_metadata(&program, "p/start"))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while viewer.state() != "running" {
        assert!(Instant::now() < deadline, "{}", viewer.note);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    // As `TerminalView::facts` reads it.
    let facts = Facts {
        state: viewer.state().into(),
        attached: viewer.showing.clone(),
        instance: viewer.target_metadata().instance.clone(),
        ..Facts::default()
    };
    let (workspace, pane) = Workspace::new(Shown::Agent("p/start".into()));
    let listed = [Listed {
        name: "p/start".into(),
        instance: Some("i-start".into()),
        ..Listed::default()
    }];
    let model = Model {
        instance: "0123456789abcdef",
        workspace: &workspace,
        facts: &[(pane, facts)].into_iter().collect(),
        listed: &listed,
        startup_cwd: "/",
        busy: None,
    };
    let caller = Caller {
        name: Some("p/start".into()),
        corral_instance: Some("i-start".into()),
        ..Caller::default()
    };
    assert_eq!(caller_pane(&model, &caller), Ok(pane));
    viewer.close().unwrap();
    while !viewer.closed() {
        assert!(Instant::now() < deadline + Duration::from_secs(5));
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
}
