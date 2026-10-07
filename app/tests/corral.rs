//! From Saddle `tests/corral.rs` at commit `df1c727`; the slow fake command is a shell script
//! instead of Python.
mod common;
use paddock::corral::Client;

#[test]
fn public_json_merges_status_with_listing_and_keeps_starting_agents() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
case "$1:$2" in
  ls:) echo '{"agents":[{"name":"demo/z","starting":true,"cwd":"/tmp/z"},{"name":"demo/a","cwd":"/tmp/a","instance":"abc123456"},{"name":"demo/old","incompatible":true,"proto":99}]}' ;;
  status:demo/a) echo '{"ok":true,"name":"demo/a","kind":"claude","state":"working","last_tool":"Bash","turn_started":100.0,"state_started":105.0,"last_output":110.0,"attached":1,"last_input_source":"human","title":"中文标题","instance":"abc123456"}' ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    );
    let agents = Client { program }.collect().unwrap();
    assert_eq!(agents.len(), 3);
    let a = &agents[0];
    assert_eq!(a.name, "demo/a");
    assert_eq!(a.cwd.as_deref(), Some("/tmp/a"));
    assert_eq!(a.state.as_deref(), Some("working"));
    assert_eq!(a.turn_started, Some(100.0));
    assert_eq!(a.state_started, Some(105.0));
    assert_eq!(a.attached, 1);
    assert_eq!(a.title.as_deref(), Some("中文标题"));
    assert!(agents[1].incompatible);
    assert!(agents[2].starting);
    assert!(agents[2].error.is_none());
}

#[test]
fn command_errors_and_timeouts_are_not_empty_successful_lists() {
    use std::{
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "bad",
        "#!/bin/sh\necho '{\"ok\":false,\"error\":\"unavailable\"}'\nexit 1\n",
    );
    assert!(
        Client { program }
            .collect()
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
    let program = common::script(temp.path(), "slow", "#!/bin/sh\nexec sleep 30\n");
    let start = Instant::now();
    let result =
        Client { program }.json(&["ls"], Duration::from_millis(100), &AtomicBool::new(false));
    assert!(result.unwrap_err().to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn only_public_effort_labels_in_the_three_known_tiers_are_reported() {
    use paddock::corral::Effort;
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
case "$1:$2" in
  ls:) echo '{"agents":[{"name":"d/m","labels":{"effort":"medium"}},{"name":"d/h","labels":{}},{"name":"d/x","labels":{"effort":"xhigh"}},{"name":"d/none"},{"name":"d/odd","labels":{"effort":"max"}},{"name":"d/case","labels":{"effort":"High"}},{"name":"d/start","starting":true,"labels":{"effort":"high"}}]}' ;;
  status:d/m) echo '{"ok":true,"state":"idle","labels":{"effort":"medium","model":"x"}}' ;;
  status:d/h) echo '{"ok":true,"state":"idle","labels":{"effort":"high"}}' ;;
  status:d/x) echo '{"ok":true,"state":"idle","labels":{"effort":"xhigh"}}' ;;
  status:d/none) echo '{"ok":true,"state":"idle"}' ;;
  status:d/odd) echo '{"ok":true,"state":"idle","labels":{"effort":"max"}}' ;;
  status:d/case) echo '{"ok":true,"state":"idle","labels":{"effort":"High"}}' ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    );
    let agents = Client { program }.collect().unwrap();
    let effort = |name: &str| agents.iter().find(|a| a.name == name).unwrap().effort();
    assert_eq!(effort("d/m"), Some(Effort::Medium));
    assert_eq!(effort("d/h"), Some(Effort::High)); // status is the fresher public source
    assert_eq!(effort("d/x"), Some(Effort::Xhigh));
    assert_eq!(effort("d/none"), None);
    assert_eq!(effort("d/odd"), None);
    assert_eq!(effort("d/case"), None);
    assert_eq!(effort("d/start"), Some(Effort::High)); // no status call while starting
    assert!(agents.iter().all(|a| a.error.is_none()));
}

#[test]
fn paused_is_read_and_an_older_corral_without_it_means_running() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
case "$1:$2" in
  ls:) echo '{"agents":[{"name":"p/frozen","paused":true,"paused_at":50.0},{"name":"p/old"},{"name":"p/new","paused":false},{"name":"p/boot","starting":true,"paused":true}]}' ;;
  status:p/frozen) echo '{"ok":true,"state":"working","paused":true,"paused_at":50.0}' ;;
  status:p/old) echo '{"ok":true,"state":"idle"}' ;;
  status:p/new) echo '{"ok":true,"state":"idle","paused":false,"paused_at":null}' ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    );
    let agents = Client { program }.collect().unwrap();
    let paused = |name: &str| {
        let a = agents.iter().find(|a| a.name == name).unwrap();
        (a.paused, a.paused_at)
    };
    assert_eq!(paused("p/frozen"), (true, Some(50.0)));
    // The state stays what it was: paused is apart from it.
    assert_eq!(agents[1].state.as_deref(), Some("working"));
    assert_eq!(paused("p/old"), (false, None));
    assert_eq!(paused("p/new"), (false, None));
    // No status while starting: the listing's word stands.
    assert_eq!(paused("p/boot"), (true, None));
    assert!(agents.iter().all(|a| a.error.is_none()));
}

#[test]
fn start_reads_the_started_name_and_stop_runs_the_public_command() {
    use paddock::new_agent::{Form, Place, Started, start, stop};
    let temp = common::tempdir();
    let log = temp.path().join("calls");
    let program = common::script(
        temp.path(),
        "corral",
        &format!(
            r##"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
case "$1" in
  start) echo '{{"ok":true,"name":"demo/main","instance":"0123456789ab"}}' ;;
  stop) echo '{{"ok":true}}' ;;
  *) echo '{{"ok":false,"error":"unexpected_command"}}'; exit 1 ;;
esac
"##,
            log = log.display()
        ),
    );
    let form = Form::new("/tmp/demo".into(), Place::Current);
    let started = start(&program, &form.args().unwrap()).unwrap();
    assert_eq!(
        started,
        Started {
            name: "demo/main".into(),
            instance: Some("0123456789ab".into())
        }
    );
    stop(&program, "demo/main").unwrap();
    let calls = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        calls,
        "start demo/main --cwd /tmp/demo --label role=controller -- codex --yolo\nstop demo/main\n"
    );
}

#[test]
fn a_start_without_a_name_or_a_failed_stop_is_an_error() {
    use paddock::new_agent::{start, stop};
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
case "$1" in
  start) echo '{"ok":true}' ;;
  *) echo '{"ok":false,"error":"not_found"}'; exit 2 ;;
esac
"##,
    );
    let error = start(&program, &["start".into(), "x/main".into()]).unwrap_err();
    assert!(error.to_string().contains("no name"), "{error}");
    let error = stop(&program, "x/main").unwrap_err();
    assert!(error.to_string().contains("not_found"), "{error}");
}
