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
