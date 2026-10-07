//! Runs only the headless CLI, with isolated home and runtime directories.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::{DirBuilderExt, PermissionsExt},
        net::UnixListener,
    },
    path::PathBuf,
    process::Command,
    thread,
};

fn sandbox() -> PathBuf {
    let dir =
        PathBuf::from("/tmp").join(format!("pctl-{}", paddock::control::random_id().unwrap()));
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    dir
}
fn command(home: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_paddock"));
    cmd.env("HOME", home)
        .env("XDG_RUNTIME_DIR", home)
        .env("TMPDIR", home)
        .env_remove("CORRAL_NAME")
        .env_remove("CORRAL_INSTANCE")
        .env_remove("PADDOCK_INSTANCE")
        .env_remove("PADDOCK_PANE");
    cmd
}
#[test]
fn help_and_errors_are_headless_json() {
    let home = sandbox();
    for args in [vec!["ctl", "--help"], vec!["install-skills", "--help"]] {
        let output = command(&home).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], true);
    }
    let output = command(&home)
        .args(["ctl", "send", "secret"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(!home.join("paddock").exists());
}
#[test]
fn caller_identity_is_forwarded_before_gui_environment_cleanup() {
    let home = sandbox();
    let runtime = home.join("paddock");
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let socket = runtime.join("0123456789abcdef.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let worker = thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut line = String::new();
                    BufReader::new(&stream).read_line(&mut line).unwrap();
                    let message: Value = serde_json::from_str(&line).unwrap();
                    writeln!(stream, "{}", json!({"ok":true,"caller":message["caller"]})).unwrap();
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < deadline, "CLI never connected");
                    thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    let output = command(&home)
        .env("CORRAL_NAME", "synthetic/test")
        .env("CORRAL_INSTANCE", "corral-test")
        .env("PADDOCK_INSTANCE", "fedcba9876543210")
        .env("PADDOCK_PANE", "42")
        .args(["ctl", "inspect", "--instance", "0123456789abcdef"])
        .output()
        .unwrap();
    let finished = worker.join();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    finished.unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["caller"],
        json!({"name":"synthetic/test","corral_instance":"corral-test","paddock_instance":"fedcba9876543210","pane":42})
    );
}

#[test]
fn skill_cli_dry_run_confirmation_and_remove_use_only_the_supplied_home() {
    let home = sandbox();
    for args in [vec!["--dry-run"], vec![]] {
        let output = command(&home)
            .arg("install-skills")
            .args(&args)
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["written"], false);
        assert_eq!(output.status.success(), !args.is_empty());
        assert!(!home.join(".agents").exists());
    }
    let output = command(&home)
        .args(["install-skills", "--yes"])
        .output()
        .unwrap();
    assert!(output.status.success());
    for base in [".agents", ".claude"] {
        assert!(
            fs::read_to_string(home.join(base).join("skills/paddock/SKILL.md"))
                .unwrap()
                .contains("由 paddock install-skills 写入")
        );
    }
    let output = command(&home)
        .args(["install-skills", "--remove", "--yes"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!home.join(".agents/skills/paddock/SKILL.md").exists());
    assert!(!home.join(".claude/skills/paddock/SKILL.md").exists());
}
