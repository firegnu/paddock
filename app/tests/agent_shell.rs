mod common;
use common::{TempDir, script, tempdir};
use paddock::{pty::Session, terminal::Size};
use std::os::unix::fs::PermissionsExt;
use std::{fs, os::unix::fs::symlink, path::PathBuf, process::Command};
use std::{
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    home: TempDir,
    shims: PathBuf,
    real: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let home = tempdir();
        let shims = home.path().join("shims");
        let real = home.path().join("real");
        fs::create_dir(&shims).unwrap();
        fs::create_dir(&real).unwrap();
        for kind in ["claude", "codex", "pi", "omp"] {
            symlink(env!("CARGO_BIN_EXE_paddock"), shims.join(kind)).unwrap();
        }
        Self { home, shims, real }
    }
    fn command(&self, kind: &str) -> Command {
        let mut command = Command::new(self.shims.join(kind));
        command
            .env_clear()
            .env("HOME", self.home.path())
            .env(
                "PATH",
                format!(
                    "{}:{}:{}",
                    self.shims.display(),
                    self.real.display(),
                    self.shims.display()
                ),
            )
            .env("PADDOCK_AGENT_SHIMS", &self.shims)
            .current_dir(self.home.path());
        command
    }
    fn pty(&self, kind: &str, args: &[&str], env: &[(&str, &str)]) -> Session {
        self.pty_at(kind, args, env, self.home.path())
    }
    fn pty_at(
        &self,
        kind: &str,
        args: &[&str],
        env: &[(&str, &str)],
        cwd: &std::path::Path,
    ) -> Session {
        let mut command = vec![
            "/usr/bin/env".into(),
            "-i".into(),
            format!("HOME={}", self.home.path().display()),
            format!(
                "PATH={}:{}:{}:/usr/bin:/bin",
                self.shims.display(),
                self.real.display(),
                self.shims.display()
            ),
            format!("PADDOCK_AGENT_SHIMS={}", self.shims.display()),
            format!(
                "PADDOCK_AGENT_CORRAL={}",
                self.home.path().join("corral").display()
            ),
        ];
        command.extend(env.iter().map(|(k, v)| format!("{k}={v}")));
        command.push(self.shims.join(kind).display().to_string());
        command.extend(args.iter().map(|s| (*s).to_owned()));
        Session::spawn_shell(
            &command,
            cwd,
            Size {
                rows: 40,
                cols: 160,
            },
            &[],
        )
        .unwrap()
    }
}

fn finish(session: &mut Session) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !session.poll_exit().unwrap() {
        assert!(Instant::now() < deadline, "test command did not exit");
        thread::sleep(Duration::from_millis(10));
    }
}

fn arguments(path: PathBuf) -> Vec<String> {
    fs::read(path)
        .unwrap()
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).unwrap())
        .collect()
}

#[test]
fn interactive_start_transfers_environment_and_arguments_then_attaches_without_ctl() {
    let f = Fixture::new();
    script(&f.real, "codex", "#!/bin/sh\nexit 99\n");
    script(
        f.home.path(),
        "corral",
        r#"#!/bin/sh
case "$1" in
  start) printf '%s\0' "$@" > "$HOME/start-args"
         printf '{"ok":true,"name":"demo/codex-7","instance":"new-instance"}\n';;
  attach) printf '%s\0' "$@" > "$HOME/attach-args"; exit 23;;
esac
"#,
    );
    let mut session = f.pty(
        "codex",
        &[
            "--yolo",
            "-m",
            "test-model",
            "-c",
            "model_reasoning_effort=high",
            "two words",
        ],
        &[
            ("KEPT", "line one\nline=two"),
            ("EMPTY", ""),
            ("CORRAL_SECRET", "excluded"),
            ("PADDOCK_PRIVATE", "excluded"),
            ("SADDLE_PRIVATE", "excluded"),
            ("TERM", "excluded"),
            ("TERM_PROGRAM", "excluded"),
            ("TERM_PROGRAM_VERSION", "excluded"),
            ("SHLVL", "3"),
            ("PWD", "excluded"),
            ("OLDPWD", "excluded"),
            ("_", "excluded"),
        ],
    );
    finish(&mut session);
    assert_eq!(session.exit_code(), Some(23));
    let args = arguments(f.home.path().join("start-args"));
    let name = format!(
        "{}/codex",
        f.home.path().file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(
        &args[..7],
        [
            "start",
            &name,
            "--unique",
            "--cwd",
            f.home.path().to_str().unwrap(),
            "--label",
            "role=regular"
        ]
    );
    assert!(
        args.windows(2)
            .any(|p| p == ["--label", "model=test-model"])
    );
    assert!(args.windows(2).any(|p| p == ["--label", "effort=high"]));
    let env: Vec<_> = args
        .windows(2)
        .filter(|p| p[0] == "--env")
        .map(|p| p[1].as_str())
        .collect();
    assert!(env.contains(&"KEPT=line one\nline=two"));
    assert!(env.contains(&"EMPTY="));
    assert!(env.contains(&format!("PATH={}:/usr/bin:/bin", f.real.display()).as_str()));
    assert!(env.iter().all(|v| !v.contains("excluded")));
    assert!(!env.iter().any(|v| v.starts_with("SHLVL=")));
    assert!(!env.iter().any(|v| v.starts_with("PADDOCK_")));
    let command = args.iter().position(|s| s == "--").unwrap();
    assert_eq!(
        &args[command + 1..],
        [
            f.real.join("codex").to_str().unwrap(),
            "--yolo",
            "-m",
            "test-model",
            "-c",
            "model_reasoning_effort=high",
            "two words"
        ]
    );
    assert_eq!(
        arguments(f.home.path().join("attach-args")),
        ["attach", "demo/codex-7"]
    );
}

#[test]
fn start_uses_git_root_name_and_failure_returns_to_the_shell_without_logging_values() {
    let f = Fixture::new();
    assert!(
        Command::new("/usr/bin/git")
            .args(["init", "--quiet"])
            .arg(f.home.path())
            .env_clear()
            .status()
            .unwrap()
            .success()
    );
    let nested = f.home.path().join("nested");
    fs::create_dir(&nested).unwrap();
    script(&f.real, "claude", "#!/bin/sh\nexit 99\n");
    script(
        f.home.path(),
        "corral",
        r#"#!/bin/sh
printf '%s\0' "$@" > "$HOME/start-args"
printf '{"ok":false,"error":"rejected synthetic-secret-value"}\n'
exit 2
"#,
    );
    let mut session = f.pty_at(
        "claude",
        &[],
        &[("SYNTHETIC_SECRET", "synthetic-secret-value")],
        &nested,
    );
    finish(&mut session);
    assert_eq!(session.exit_code(), Some(1));
    let args = arguments(f.home.path().join("start-args"));
    assert_eq!(
        args[1],
        format!(
            "{}/claude",
            f.home.path().file_name().unwrap().to_str().unwrap()
        )
    );
    assert_eq!(args[4], nested.to_str().unwrap());
    let output = screen(&session);
    assert!(output.contains("corral start: rejected [environment SYNTHETIC_SECRET]"));
    assert!(!output.contains("synthetic-secret-value"));
    assert!(!f.home.path().join("attach-args").exists());
    let command = vec![
        "/usr/bin/env".into(),
        "-i".into(),
        format!("HOME={}", f.home.path().display()),
        format!(
            "PATH={}:{}:/usr/bin:/bin",
            f.shims.display(),
            f.real.display()
        ),
        format!("PADDOCK_AGENT_SHIMS={}", f.shims.display()),
        format!(
            "PADDOCK_AGENT_CORRAL={}",
            f.home.path().join("corral").display()
        ),
        "SYNTHETIC_SECRET=synthetic-secret-value".into(),
        "PS1=FAILURE_PROMPT> ".into(),
        "/bin/bash".into(),
        "--norc".into(),
        "-i".into(),
    ];
    let session = Session::spawn_shell(
        &command,
        &nested,
        Size {
            rows: 24,
            cols: 160,
        },
        &[],
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !screen(&session).contains("FAILURE_PROMPT>") {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    session
        .send(b"claude; printf '%s' \"$?\" > status; printf READY > after\n".to_vec())
        .unwrap();
    while !nested.join("after").exists() {
        assert!(
            Instant::now() < deadline,
            "failed start did not return to the shell"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(nested.join("status")).unwrap(), "1");
    assert!(session.running());
}

#[test]
fn missing_real_command_exits_127_and_reports_command_not_found() {
    let f = Fixture::new();
    let output = f.command("claude").arg("--version").output().unwrap();
    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("claude: command not found"));
}

#[test]
fn passthrough_skips_all_shims_preserves_argv_and_exit_status() {
    let f = Fixture::new();
    script(
        &f.real,
        "claude",
        "#!/bin/sh\nprintf '%s\\0' \"$@\"\nexit 23\n",
    );
    let output = f
        .command("claude")
        .args(["--print", "two words", "a=b\nc"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.stdout, b"--print\0two words\0a=b\nc\0");
}

#[test]
fn agent_shell_hooks_survive_rc_path_changes_without_changing_plain_shells() {
    for (shell, custom_zdotdir) in [
        ("/bin/zsh", false),
        ("/bin/zsh", true),
        ("/bin/bash", false),
    ] {
        for agent_shell in [false, true] {
            let f = Fixture::new();
            fs::set_permissions(f.home.path(), fs::Permissions::from_mode(0o700)).unwrap();
            script(&f.real, "claude", "#!/bin/sh\nexit 0\n");
            let dotdir = if custom_zdotdir {
                f.home.path().join("custom dotdir")
            } else {
                f.home.path().to_owned()
            };
            fs::create_dir_all(&dotdir).unwrap();
            fs::write(dotdir.join(".zshenv"), "export FROM_ZSHENV=loaded\n").unwrap();
            let rc = format!(
                "export PATH='{}:/usr/bin:/bin'\nPS1='FIXTURE> '\n",
                f.real.display()
            );
            fs::write(dotdir.join(".zshrc"), &rc).unwrap();
            fs::write(
                f.home.path().join(".bashrc"),
                format!(
                    "{rc}PROMPT_COMMAND='export PATH={}:/usr/bin:/bin;'\n",
                    f.real.display()
                ),
            )
            .unwrap();
            let mut command = vec![
                "/usr/bin/env".into(),
                "-i".into(),
                format!("HOME={}", f.home.path().display()),
                "TERM=xterm".into(),
                "PATH=/usr/bin:/bin".into(),
            ];
            if custom_zdotdir {
                command.push(format!("ZDOTDIR={}", dotdir.display()));
            }
            let dir = f.home.path().join("integration");
            if agent_shell {
                let setup = paddock::agent_shell::setup(
                    shell,
                    &dir,
                    std::path::Path::new(env!("CARGO_BIN_EXE_paddock")),
                    custom_zdotdir.then(|| dotdir.to_str().unwrap()),
                )
                .unwrap();
                command.extend(
                    setup
                        .env
                        .iter()
                        .map(|(key, value)| format!("{key}={value}")),
                );
                command.extend(setup.command);
            } else {
                command.extend([shell.into(), "-i".into()]);
            }
            let session = Session::spawn_shell(
                &command,
                f.home.path(),
                Size {
                    rows: 24,
                    cols: 160,
                },
                &[],
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while !screen(&session).contains("FIXTURE>") {
                assert!(Instant::now() < deadline, "shell did not reach its prompt");
                thread::sleep(Duration::from_millis(10));
            }
            if agent_shell {
                session
                    .send(
                        format!(
                            "export PATH='{1}:{0}:{0}:/usr/bin:/bin'\n",
                            dir.join("bin").display(),
                            f.real.display()
                        )
                        .into_bytes(),
                    )
                    .unwrap();
            }
            session
                .send(
                    b"command -v claude > found; printf '%s' \"${FROM_ZSHENV:-bash}\" > sourced\n"
                        .to_vec(),
                )
                .unwrap();
            while !fs::read_to_string(f.home.path().join("sourced")).is_ok_and(|s| !s.is_empty()) {
                assert!(Instant::now() < deadline, "shell command did not finish");
                thread::sleep(Duration::from_millis(10));
            }
            let expected = if agent_shell {
                dir.join("bin/claude")
            } else {
                f.real.join("claude")
            };
            assert_eq!(
                fs::read_to_string(f.home.path().join("found"))
                    .unwrap()
                    .trim(),
                expected.to_str().unwrap(),
                "{shell} agent_shell={agent_shell} custom_zdotdir={custom_zdotdir}"
            );
            assert_eq!(
                fs::read_to_string(f.home.path().join("sourced")).unwrap(),
                if shell.ends_with("zsh") {
                    "loaded"
                } else {
                    "bash"
                }
            );
            if agent_shell {
                assert_eq!(dir.metadata().unwrap().permissions().mode() & 0o777, 0o700);
            }
        }
    }
}

fn screen(session: &Session) -> String {
    session
        .screen
        .lock()
        .unwrap()
        .term
        .grid()
        .display_iter()
        .map(|c| c.c)
        .collect()
}

#[test]
fn shim_requests_its_own_pane_handoff_and_waits_for_the_listing_check() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
    };
    let f = Fixture::new();
    struct Runtime(PathBuf);
    impl Drop for Runtime {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let base = Runtime(PathBuf::from(format!(
        "/tmp/pad-as-{}",
        paddock::control::random_id().unwrap()
    )));
    fs::create_dir(&base.0).unwrap();
    fs::set_permissions(&base.0, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = base.0.join("paddock-ctl");
    fs::create_dir(&runtime).unwrap();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
    let instance = "0123456789abcdef";
    let listener = UnixListener::bind(runtime.join(format!("{instance}.sock"))).unwrap();
    fs::set_permissions(
        runtime.join(format!("{instance}.sock")),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    listener.set_nonblocking(true).unwrap();
    script(&f.real, "claude", "#!/bin/sh\nexit 99\n");
    script(
        f.home.path(),
        "corral",
        "#!/bin/sh\ncase \"$1\" in\n start) printf '{\"ok\":true,\"name\":\"demo/claude-1\"}\\n';;\n attach) printf unwanted > \"$HOME/attached\"; exit 23;;\nesac\n",
    );
    let mut session = f.pty(
        "claude",
        &[],
        &[
            ("PADDOCK_INSTANCE", instance),
            ("PADDOCK_PANE", "8"),
            ("XDG_RUNTIME_DIR", base.0.to_str().unwrap()),
        ],
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut request = String::new();
    for checking in [true, false] {
        let (mut stream, _) = loop {
            if let Ok(stream) = listener.accept() {
                break stream;
            }
            assert!(
                !session.poll_exit().unwrap(),
                "shim exited before requesting handoff"
            );
            assert!(Instant::now() < deadline, "shim never contacted ctl");
            thread::sleep(Duration::from_millis(10));
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line).unwrap();
        let message: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(message["instance"], instance);
        assert_eq!(message["caller"]["pane"], 8);
        assert_eq!(message["caller"]["paddock_instance"], instance);
        let state = if checking {
            assert_eq!(
                message["operation"],
                serde_json::json!({"command":"agent_shell","name":"demo/claude-1"})
            );
            request = message["request_id"].as_str().unwrap().into();
            "checking"
        } else {
            assert_eq!(
                message["operation"],
                serde_json::json!({"command":"request","request":request})
            );
            "attaching"
        };
        writeln!(
            stream,
            "{}",
            serde_json::json!({"ok":true,"accepted":true,"state":state})
        )
        .unwrap();
    }
    finish(&mut session);
    assert_eq!(session.exit_code(), Some(0));
    assert!(!f.home.path().join("attached").exists());
}
