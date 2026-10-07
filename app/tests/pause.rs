//! Pausing and resuming agents through a fake `corral` (DESIGN §13 P5-33); never a real agent.
mod common;
use paddock::pause::{run, summary};

/// A fake corral that logs each call and refuses some agents as corral would: one under an older
/// corral, one gone.
fn corral(dir: &std::path::Path) -> String {
    common::script(
        dir,
        "corral",
        r##"#!/bin/sh
printf '%s\n' "$*" >> "$(dirname "$0")/calls"
case "$2" in
  p/old) echo '{"ok":false,"error":"unsupported","message":"p/old runs under a corral without pause; corral upgrade p/old first","name":"p/old"}'; exit 9 ;;
  p/gone) echo '{"ok":false,"error":"no_such_agent","message":"no such agent"}'; exit 2 ;;
esac
case "$1" in
  pause) printf '{"ok":true,"name":"%s","instance":"i","paused":true,"paused_at":1.0}\n' "$2" ;;
  resume) printf '{"ok":true,"name":"%s","instance":"i","paused":false,"paused_at":null}\n' "$2" ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    )
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn every_name_reaches_corral_and_the_note_counts_them() {
    let temp = common::tempdir();
    let program = corral(temp.path());
    let all = names(&["p/a", "p/b", "q/main"]);
    let results = run(&program, &all, true);
    assert!(results.iter().all(|(_, result)| result.is_ok()));
    assert_eq!(
        summary(&results, true),
        ("Paused 3 agents".to_owned(), false)
    );
    let results = run(&program, &all, false);
    assert_eq!(
        summary(&results, false),
        ("Resumed 3 agents".to_owned(), false)
    );
    let calls = std::fs::read_to_string(temp.path().join("calls")).unwrap();
    assert_eq!(
        calls,
        "pause p/a\npause p/b\npause q/main\nresume p/a\nresume p/b\nresume q/main\n"
    );
    // One agent is named.
    let one = run(&program, &names(&["p/a"]), true);
    assert_eq!(summary(&one, true), ("Paused p/a".to_owned(), false));
}

#[test]
fn a_partial_failure_names_each_agent_and_why() {
    let temp = common::tempdir();
    let program = corral(temp.path());
    let results = run(&program, &names(&["p/a", "p/old", "p/b", "p/gone"]), true);
    // The rest are still paused after one fails.
    let calls = std::fs::read_to_string(temp.path().join("calls")).unwrap();
    assert_eq!(calls, "pause p/a\npause p/old\npause p/b\npause p/gone\n");
    assert_eq!(
        summary(&results, true),
        (
            "Paused 2 of 4 agents; not paused: p/old (needs corral upgrade), p/gone (no_such_agent)"
                .to_owned(),
            true
        )
    );
    let results = run(&program, &names(&["p/old", "p/gone"]), false);
    assert_eq!(
        summary(&results, false),
        (
            "Couldn't resume 2 agents: p/old (needs corral upgrade), p/gone (no_such_agent)"
                .to_owned(),
            true
        )
    );
    let one = run(&program, &names(&["p/old"]), true);
    assert_eq!(
        summary(&one, true),
        (
            "Couldn't pause p/old: needs corral upgrade".to_owned(),
            true
        )
    );
    // A corral that cannot be run says why.
    let missing = temp.path().join("no-such-corral").display().to_string();
    let (text, problem) = summary(&run(&missing, &names(&["p/a"]), true), true);
    assert!(text.starts_with("Couldn't pause p/a: "), "{text}");
    assert!(problem);
}

#[test]
fn pause_all_asks_only_when_one_is_working_and_reaches_every_live_agent() {
    use paddock::{corral::Client, pause::every};
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
root=$(dirname "$0")
case "$1:$2" in
  ls:) echo '{"agents":[{"name":"p/a"},{"name":"p/b"},{"name":"p/done"},{"name":"q/main"}]}' ;;
  status:p/a) echo '{"ok":true,"state":"idle"}' ;;
  status:p/b) printf '{"ok":true,"state":"%s"}\n' "$(cat "$root/b-state")" ;;
  status:p/done) echo '{"ok":true,"state":"exited"}' ;;
  status:q/main) echo '{"ok":true,"state":"blocked"}' ;;
  pause:*) printf '%s\n' "$*" >> "$root/calls"; echo '{"ok":true,"paused":true}' ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    );
    let client = Client {
        program: program.clone(),
    };
    // Nobody working: straight on, every live agent and never the one that exited.
    std::fs::write(temp.path().join("b-state"), "idle").unwrap();
    let request = every(&client.collect().unwrap()).unwrap();
    assert!(request.pause);
    assert_eq!(request.ask, None);
    assert_eq!(request.names, names(&["p/a", "p/b", "q/main"]));
    // One working: it is named in the question.
    std::fs::write(temp.path().join("b-state"), "working").unwrap();
    let request = every(&client.collect().unwrap()).unwrap();
    assert_eq!(
        request.ask.as_deref(),
        Some("1 agent is working: p/b. Pause anyway?")
    );
    assert_eq!(request.names, names(&["p/a", "p/b", "q/main"]));
    // Each name reaches corral.
    run(&program, &request.names, request.pause);
    let calls = std::fs::read_to_string(temp.path().join("calls")).unwrap();
    assert_eq!(calls, "pause p/a\npause p/b\npause q/main\n");
}
