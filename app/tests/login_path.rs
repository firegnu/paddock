//! Each test has its own process: no PATH mutations in the parallel test runner, no user profiles.
mod common;

use paddock::{
    command, diagnostics,
    launch::{self, LoginPath},
    palette,
    pty::Session,
    terminal::Size,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    thread,
    time::{Duration, Instant},
};

const BARE: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

fn isolated(name: &str) -> Option<PathBuf> {
    const CHILD: &str = "PADDOCK_TEST_LOGIN_HOME";
    if let Some(home) = std::env::var_os(CHILD) {
        return Some(home.into());
    }
    let home = common::tempdir();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env_clear()
        .env(CHILD, home.path())
        .env("HOME", home.path())
        .env("ZDOTDIR", home.path())
        .env("PATH", BARE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

fn command_path() -> String {
    let out = command::run(
        "/bin/sh",
        &["-c", "printf '%s' \"$PATH\""],
        None,
        Duration::from_secs(5),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

fn shell(home: &Path) -> Session {
    Session::spawn_shell(
        &["/bin/sh".into(), "-i".into()],
        home,
        Size { rows: 24, cols: 80 },
        &[],
        palette::Shared::default(),
    )
    .unwrap()
}

fn shell_output(session: &Session, home: &Path, name: &str, command: &str) -> String {
    let file = home.join(name);
    let file = shell_words::quote(file.to_str().unwrap());
    session
        .send(format!("{command} > {file}.pending; /bin/mv {file}.pending {file}\n").into_bytes());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = fs::read_to_string(home.join(name))
            && !text.is_empty()
        {
            return text;
        }
        assert!(Instant::now() < deadline, "no output for {name}");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_login_path_ready_after_three_seconds_is_not_lost() {
    let Some(home) = isolated("a_login_path_ready_after_three_seconds_is_not_lost") else {
        return;
    };
    let path = format!("{}:{BARE}", home.display());
    common::script(&home, "corral", "#!/bin/sh\nprintf '%s' \"$PATH\"\n");
    common::script(&home, "brew", "#!/bin/sh\nprintf 'fake brew'\n");
    let login = common::script(
        &home,
        "login-shell",
        &format!(
            "#!/bin/sh\n/bin/sleep 4\nwhile [ ! -f '{}/release' ]; do /bin/sleep 0.01; done\nexport PATH='{path}'\nshift\nexec /bin/sh -c \"$2\"\n",
            home.display()
        ),
    );
    let old = shell(&home);
    assert_eq!(
        shell_output(&old, &home, "before", "printf '%s' \"$PATH\""),
        BARE
    );
    assert!(diagnostics::find("corral").is_err());
    let (state, ready) = launch::load_login_path(
        login.into(),
        Duration::from_secs(3),
        Duration::from_secs(10),
    );
    assert_eq!(state, LoginPath::Loading);
    assert!(state.notice().unwrap().contains("limited startup PATH"));
    assert_eq!(command_path(), BARE);
    let during = shell(&home);

    // Spawn concurrently across the replacement. Every child sees the entire old or new PATH.
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let path = path.clone();
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    let seen = command_path();
                    assert!(seen == BARE || seen == path, "partial PATH: {seen}");
                    if seen == path {
                        return;
                    }
                    assert!(Instant::now() < deadline, "PATH never recovered");
                }
            })
        })
        .collect();
    fs::write(home.join("release"), "").unwrap();
    assert_eq!(
        ready.recv_timeout(Duration::from_secs(5)).unwrap(),
        LoginPath::Ready
    );
    for reader in readers {
        reader.join().unwrap();
    }
    assert!(LoginPath::Ready.notice().is_none());
    assert_eq!(
        std::env::var("PATH").unwrap(),
        BARE,
        "parent environment changed"
    );
    for (session, name) in [(&old, "old-after"), (&during, "during-after")] {
        assert_eq!(
            shell_output(session, &home, name, "printf '%s' \"$PATH\""),
            BARE
        );
    }
    let new = shell(&home);
    assert_eq!(
        shell_output(&new, &home, "new", "printf '%s' \"$PATH\""),
        path
    );
    assert_eq!(
        shell_output(&new, &home, "brew-result", "brew"),
        "fake brew"
    );
    assert_eq!(diagnostics::find("corral").unwrap(), home.join("corral"));
    let out = command::run(
        "corral",
        &[],
        None,
        Duration::from_secs(5),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, path.as_bytes());

    // Agent shell setup adds its shims but inherits the newly recovered PATH underneath.
    let setup = paddock::agent_shell::setup(
        "/bin/bash",
        &home.join("agent-shell"),
        Path::new(env!("CARGO_BIN_EXE_paddock")),
        None,
    )
    .unwrap();
    let agent = Session::spawn_shell(
        &setup.command,
        &home,
        Size { rows: 24, cols: 80 },
        &setup.env,
        palette::Shared::default(),
    )
    .unwrap();
    assert_eq!(
        shell_output(&agent, &home, "agent-brew", "brew"),
        "fake brew"
    );
    assert_eq!(
        shell_output(&agent, &home, "agent-path", "printf '%s' \"$PATH\""),
        format!("{}/agent-shell/bin:{path}", home.display())
    );
}

#[test]
fn failed_login_paths_keep_the_startup_path_and_explain_what_to_do() {
    let Some(home) = isolated("failed_login_paths_keep_the_startup_path_and_explain_what_to_do")
    else {
        return;
    };
    for (name, body) in [
        ("never", "exec /bin/sleep 30"),
        (
            "nonzero",
            "printf '__PADDOCK_PATH_START__/wrong__PADDOCK_PATH_END__'; exit 1",
        ),
        ("no-markers", "printf '/wrong'"),
        (
            "empty",
            "printf '__PADDOCK_PATH_START____PADDOCK_PATH_END__'",
        ),
    ] {
        let login = common::script(&home, name, &format!("#!/bin/sh\n{body}\n"));
        let started = Instant::now();
        let (mut state, ready) = launch::load_login_path(
            login.into(),
            Duration::from_millis(20),
            Duration::from_millis(200),
        );
        if state == LoginPath::Loading {
            state = ready.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(state, LoginPath::Failed, "{name}");
        assert!(started.elapsed() < Duration::from_secs(5));
        let note = state.notice().unwrap();
        assert!(note.contains("limited startup PATH"));
        assert!(note.contains("quit and reopen paddock"));
        assert_eq!(command_path(), BARE);
        assert_eq!(launch::path().unwrap(), BARE);
    }
}

#[test]
fn terminal_launches_keep_their_path_and_fast_logins_are_ready_before_opening() {
    let Some(home) =
        isolated("terminal_launches_keep_their_path_and_fast_logins_are_ready_before_opening")
    else {
        return;
    };
    // Without desktop initialization, subprocesses and lookups keep the caller's PATH.
    assert_eq!(launch::path().unwrap(), BARE);
    assert_eq!(command_path(), BARE);
    assert!(diagnostics::find("sh").is_ok());
    let login = common::script(
        &home,
        "login",
        "#!/bin/sh\nprintf '__PADDOCK_PATH_START__/custom/bin:/usr/bin:/bin__PADDOCK_PATH_END__'\n",
    );
    let (state, _) =
        launch::load_login_path(login.into(), Duration::from_secs(3), Duration::from_secs(5));
    assert_eq!(state, LoginPath::Ready);
    assert_eq!(command_path(), "/custom/bin:/usr/bin:/bin");
    assert_eq!(std::env::var("PATH").unwrap(), BARE);
}
