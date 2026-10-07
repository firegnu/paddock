//! From Saddle `tests/pty.rs` at commit `df1c727`. The fake programs are shell scripts instead of
//! Python; they do the same things: raw mode, report the size on SIGWINCH, echo input, note SIGINT;
//! and for the owned shell, a foreground job in its own process group that notes SIGHUP.
mod common;
use alacritty_terminal::grid::Dimensions;
use paddock::{pty::Session, terminal::Size};
use std::{
    thread,
    time::{Duration, Instant},
};
fn wait_for(session: &Session, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let text: String = session
            .screen
            .lock()
            .unwrap()
            .term
            .grid()
            .display_iter()
            .map(|c| c.c)
            .collect();
        if text.contains(expected) {
            return;
        }
        assert!(Instant::now() < deadline, "missing {expected:?}: {text:?}");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn pty_input_resize_and_interrupt_only_end_the_owned_attach_process() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "fake-attach",
        r#"#!/bin/sh
# Synthetic agent lifetime is independent of its attach process.
printf alive > agent-alive
stty raw -echo
size() { set -- $(stty size); printf '\r\nSIZE=%sx%s\r\n' "$2" "$1"; }
trap size WINCH
trap 'printf SIGINT > detached; exit 0' INT
size
printf 'READY\r\n'
# Read in the background so the traps run while waiting for input.
exec 3<&0
while :; do
  dd bs=4096 count=1 <&3 2>/dev/null > input &
  while ! wait $!; do :; done
  [ -s input ] || break
  printf 'INPUT:%s\r\n' "$(cat input)"
done
"#,
    );
    let mut session =
        Session::spawn(&[program], Some(temp.path()), Size { rows: 10, cols: 40 }).unwrap();
    wait_for(&session, "READY");
    session.send(b"hello".to_vec()).unwrap();
    wait_for(&session, "INPUT:hello");
    session.resize(Size { rows: 12, cols: 50 }).unwrap();
    wait_for(&session, "SIZE=50x12");
    assert_eq!(session.screen.lock().unwrap().term.columns(), 50);
    session.interrupt().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !session.poll_exit().unwrap() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(temp.path().join("detached")).unwrap(),
        "SIGINT"
    );
    assert!(temp.path().join("agent-alive").exists());
}

#[test]
fn closing_owned_shell_signals_its_foreground_job_without_using_attach_interrupt() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "shell",
        r#"#!/bin/sh
cd "$(dirname "$0")"
trap '' TTOU
trap 'printf HUP > shell-hup; wait; exit 0' HUP
# Job control puts the job in its own process group and hands it the terminal.
set -m
sh -c 'trap "printf HUP > job-hup; exit 0" HUP; printf "JOB READY\r\n"; while :; do sleep 1 & wait $!; done'
"#,
    );
    let mut session = Session::spawn_shell(
        &[program, "-i".into()],
        temp.path(),
        Size { rows: 10, cols: 40 },
        &[],
    )
    .unwrap();
    wait_for(&session, "JOB READY");
    session.interrupt().unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    while !session.poll_exit().unwrap() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(temp.path().join("job-hup")).unwrap(),
        "HUP"
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("shell-hup")).unwrap(),
        "HUP"
    );
}
#[test]
fn a_shell_gets_the_identity_paddock_ctl_knows_it_by() {
    let temp = common::tempdir();
    let program = common::script(
        temp.path(),
        "shell",
        "#!/bin/sh\nprintf 'ID=%s/%s\\r\\n' \"$PADDOCK_INSTANCE\" \"$PADDOCK_PANE\"\nexec sleep 30\n",
    );
    let mut session = Session::spawn_shell(
        &[program, "-i".into()],
        temp.path(),
        Size { rows: 10, cols: 40 },
        &[
            ("PADDOCK_INSTANCE".into(), "0123456789abcdef".into()),
            ("PADDOCK_PANE".into(), "7".into()),
        ],
    )
    .unwrap();
    wait_for(&session, "ID=0123456789abcdef/7");
    session.interrupt().unwrap();
}
