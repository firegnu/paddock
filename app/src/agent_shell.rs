//! The Agent shell's command shims (DESIGN §13 P5-61).
use anyhow::{Context, Result, bail};
use std::{
    ffi::{OsStr, OsString},
    io::IsTerminal,
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt, symlink},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const KINDS: &[&str] = &["claude", "codex", "pi", "omp"];

pub struct Setup {
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn supported(program: &str) -> bool {
    matches!(
        Path::new(program).file_name().and_then(|s| s.to_str()),
        Some("zsh" | "bash")
    )
}

pub fn setup(
    program: &str,
    dir: &Path,
    executable: &Path,
    original_zdotdir: Option<&str>,
) -> Result<Setup> {
    anyhow::ensure!(supported(program), "Agent shell supports only zsh and bash");
    let dir = crate::control::prepare_runtime(dir)?;
    let bin = crate::control::prepare_runtime(&dir.join("bin"))?;
    for kind in KINDS {
        let temporary = bin.join(crate::control::random_id()?);
        symlink(executable, &temporary)?;
        std::fs::rename(temporary, bin.join(kind))?;
    }
    for (name, contents) in [(".zshenv", ZSHENV), ("bashrc", BASHRC)] {
        use std::io::Write;
        let temporary = dir.join(crate::control::random_id()?);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(contents.as_bytes())?;
        std::fs::rename(temporary, dir.join(name))?;
    }
    let mut env = vec![(
        "PADDOCK_AGENT_SHIMS".into(),
        bin.to_string_lossy().into_owned(),
    )];
    let mut command = vec![program.into()];
    if program.ends_with("/zsh") || program == "zsh" {
        env.push(("ZDOTDIR".into(), dir.to_string_lossy().into_owned()));
        // Empty means the original ZDOTDIR was unset: use the shell's HOME.
        env.push((
            "PADDOCK_AGENT_ZDOTDIR".into(),
            original_zdotdir.unwrap_or_default().into(),
        ));
    } else {
        command.extend([
            "--rcfile".into(),
            dir.join("bashrc").to_string_lossy().into_owned(),
        ]);
    }
    command.push("-i".into());
    Ok(Setup { command, env })
}

const ZSHENV: &str = r#"export ZDOTDIR=${PADDOCK_AGENT_ZDOTDIR:-$HOME}
unset PADDOCK_AGENT_ZDOTDIR
[[ -r "$ZDOTDIR/.zshenv" ]] && source "$ZDOTDIR/.zshenv"
if [[ -o interactive ]]; then
    _paddock_agent_path() {
        local entry
        local -a kept
        kept=("$PADDOCK_AGENT_SHIMS")
        for entry in "${path[@]}"; do
            [[ "$entry" == "$PADDOCK_AGENT_SHIMS" ]] || kept+=("$entry")
        done
        path=("${kept[@]}")
        export PATH
    }
    autoload -Uz add-zsh-hook
    add-zsh-hook precmd _paddock_agent_path
    add-zsh-hook preexec _paddock_agent_path
fi
"#;

const BASHRC: &str = r#"[[ -r "$HOME/.bashrc" ]] && source "$HOME/.bashrc"
_paddock_agent_path() {
    local rest=$PATH entry result=$PADDOCK_AGENT_SHIMS
    while :; do
        entry=${rest%%:*}
        [[ "$entry" == "$PADDOCK_AGENT_SHIMS" ]] || result="$result:$entry"
        [[ "$rest" == *:* ]] || break
        rest=${rest#*:}
    done
    export PATH=$result
}
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    'declare -a '*) PROMPT_COMMAND+=(_paddock_agent_path);;
    *) PROMPT_COMMAND="${PROMPT_COMMAND}${PROMPT_COMMAND:+$'\n'}_paddock_agent_path";;
esac
_paddock_agent_path
"#;

fn clean_path(path: &OsStr, shims: &Path) -> OsString {
    let canonical = shims.canonicalize().ok();
    std::env::join_paths(std::env::split_paths(path).filter(|dir| {
        dir != shims && !(canonical.is_some() && dir.canonicalize().ok() == canonical)
    }))
    .expect("PATH entries contain no colon")
}

fn find_program(kind: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|dir| {
        let program = std::path::absolute(dir.join(kind)).ok()?;
        let metadata = program.metadata().ok()?;
        (metadata.is_file() && metadata.permissions().mode() & 0o111 != 0).then_some(program)
    })
}

pub fn run(kind: &str) -> i32 {
    let shims = std::env::var_os("PADDOCK_AGENT_SHIMS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::args_os()
                .next()
                .and_then(|p| Path::new(&p).parent().map(Path::to_owned))
        })
        .unwrap_or_default();
    let path = clean_path(&std::env::var_os("PATH").unwrap_or_default(), &shims);
    let Some(program) = find_program(kind, &path) else {
        eprintln!("{kind}: command not found");
        return 127;
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if passthrough(
        kind,
        &args,
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    ) {
        let error = Command::new(program).args(args).env("PATH", path).exec();
        eprintln!("{kind}: could not execute program: {error}");
        return 126;
    }
    match managed(kind, &program, &args, &path) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{kind}: {error:#}");
            1
        }
    }
}

fn excluded(key: &str) -> bool {
    key.starts_with("CORRAL_")
        || key.starts_with("PADDOCK_")
        || key.starts_with("SADDLE_")
        || matches!(
            key,
            "TERM" | "TERM_PROGRAM" | "TERM_PROGRAM_VERSION" | "SHLVL" | "PWD" | "OLDPWD" | "_"
        )
}

fn managed(kind: &str, program: &Path, args: &[OsString], path: &OsStr) -> Result<i32> {
    let corral =
        std::env::var_os("PADDOCK_AGENT_CORRAL").context("Agent shell has no corral program")?;
    let cwd = std::env::current_dir().context("cannot read current directory")?;
    let words: Vec<String> = std::iter::once(program.as_os_str())
        .chain(args.iter().map(OsString::as_os_str))
        .map(|s| {
            s.to_str()
                .map(str::to_owned)
                .context("agent command must be UTF-8")
        })
        .collect::<Result<_>>()?;
    let env: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(key, value)| {
            if key.to_str().is_some_and(excluded) {
                None
            } else {
                Some((key, value))
            }
        })
        .map(|(key, value)| {
            let key = key
                .into_string()
                .map_err(|_| anyhow::anyhow!("environment variable name is not UTF-8"))?;
            let value = if key == "PATH" {
                path.to_owned()
            } else {
                value
            };
            let value = value
                .into_string()
                .map_err(|_| anyhow::anyhow!("environment variable {key} is not UTF-8"))?;
            Ok((key, value))
        })
        .collect::<Result<_>>()?;
    let prefix = project_prefix(&cwd);
    let mut start = vec![
        "start".to_owned(),
        format!("{prefix}/{kind}"),
        "--unique".into(),
        "--cwd".into(),
        cwd.to_str()
            .context("current directory must be UTF-8")?
            .into(),
        "--label".into(),
        "role=regular".into(),
    ];
    let reading = crate::new_agent::read(&words);
    for (key, value) in [("model", reading.model), ("effort", reading.effort)] {
        if let Some(value) = value {
            start.extend(["--label".into(), format!("{key}={value}")]);
        }
    }
    for (key, value) in &env {
        start.extend(["--env".into(), format!("{key}={value}")]);
    }
    start.push("--".into());
    start.extend(words);
    let name = start_agent(&corral, &start, &env)?;
    if handoff(&name) {
        return Ok(0);
    }
    eprintln!("paddock ctl unavailable; attaching to {name} in this terminal.");
    let status = Command::new(corral)
        .args(["attach", &name])
        .status()
        .map_err(spawn_error)?;
    use std::os::unix::process::ExitStatusExt;
    Ok(status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)))
}

fn handoff(name: &str) -> bool {
    use crate::control::{Caller, Message, Operation, exchange, random_id};
    let caller = Caller::environment();
    let Some(instance) = caller.paddock_instance.clone() else {
        return false;
    };
    let Ok(request) = random_id() else {
        return false;
    };
    let mut message = Message {
        instance,
        caller,
        request_id: Some(request.clone()),
        operation: Operation::AgentShell { name: name.into() },
    };
    loop {
        let Ok(value) = exchange(&message) else {
            return false;
        };
        if value["ok"] != true {
            return false;
        }
        match value["state"].as_str() {
            Some("attaching" | "complete") => return true,
            Some("checking") => {
                message.operation = Operation::Request {
                    request: request.clone(),
                };
                message.request_id = None;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            _ => return false,
        }
    }
}

fn project_prefix(cwd: &Path) -> String {
    let output = crate::git::git(
        "git",
        cwd,
        &["rev-parse", "--show-toplevel"],
        &std::sync::atomic::AtomicBool::new(false),
    );
    let root = output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok());
    crate::new_agent::suggest_prefix(
        root.as_deref()
            .map(str::trim)
            .unwrap_or(&cwd.to_string_lossy()),
    )
}

fn spawn_error(error: std::io::Error) -> anyhow::Error {
    if error.raw_os_error() == Some(libc::E2BIG) {
        anyhow::anyhow!(
            "corral arguments and environment exceed the operating system limit; no environment variables were dropped"
        )
    } else {
        anyhow::anyhow!("could not execute corral: {error}")
    }
}

// Do not use Client::json here: its diagnostic includes argv, which contains the environment.
fn start_agent(corral: &OsStr, args: &[String], env: &[(String, String)]) -> Result<String> {
    let output = Command::new(corral)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(spawn_error)?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    if !output.status.success() || value["ok"] == false {
        let detail = value["error"]
            .as_str()
            .unwrap_or_else(|| std::str::from_utf8(&output.stderr).unwrap_or("command failed"));
        bail!("corral start: {}", redacted(detail, env));
    }
    value["name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .context("corral start returned no name; whether it started is unknown")
}

fn redacted(detail: &str, env: &[(String, String)]) -> String {
    let mut values: Vec<_> = env.iter().filter(|(_, value)| !value.is_empty()).collect();
    values.sort_by_key(|(_, value)| std::cmp::Reverse(value.len()));
    values
        .into_iter()
        .fold(detail.to_owned(), |text, (key, value)| {
            text.replace(value, &format!("[environment {key}]"))
        })
}

pub fn passthrough(kind: &str, args: &[OsString], stdin: bool, stdout: bool) -> bool {
    const FLAGS: &[&str] = &["-p", "--print", "--version", "-v", "--help", "-h"];
    const CLAUDE: &[&str] = &[
        "mcp",
        "config",
        "update",
        "doctor",
        "install",
        "migrate-installer",
        "setup-token",
    ];
    const CODEX: &[&str] = &[
        "exec",
        "e",
        "login",
        "logout",
        "mcp",
        "mcp-server",
        "apply",
        "a",
        "completion",
        "debug",
        "proto",
    ];
    let commands = match kind {
        "claude" => CLAUDE,
        "codex" => CODEX,
        _ => &[],
    };
    !stdin
        || !stdout
        || args
            .iter()
            .any(|arg| arg.to_str().is_some_and(|arg| FLAGS.contains(&arg)))
        || args
            .first()
            .and_then(|arg| arg.to_str())
            .is_some_and(|arg| commands.contains(&arg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_environment_fails_clearly_without_printing_or_dropping_values() {
        let value = "x".repeat(1024 * 1024);
        let error = start_agent(
            OsStr::new("/usr/bin/true"),
            &["--env".into(), format!("SYNTHETIC={value}")],
            &[("SYNTHETIC".into(), value)],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("operating system limit"));
        assert!(error.contains("no environment variables were dropped"));
        assert!(!error.contains("xxx"));
    }

    #[test]
    fn one_shot_commands_and_non_terminals_pass_through() {
        for kind in ["claude", "codex", "pi", "omp"] {
            for flag in ["-p", "--print", "--version", "-v", "--help", "-h"] {
                assert!(
                    passthrough(kind, &[flag.into()], true, true),
                    "{kind} {flag}"
                );
            }
            assert!(passthrough(kind, &[], false, true));
            assert!(passthrough(kind, &[], true, false));
            assert!(!passthrough(kind, &[], true, true));
            assert!(!passthrough(kind, &["hello".into()], true, true));
        }
        for (kind, commands) in [
            (
                "claude",
                &[
                    "mcp",
                    "config",
                    "update",
                    "doctor",
                    "install",
                    "migrate-installer",
                    "setup-token",
                ][..],
            ),
            (
                "codex",
                &[
                    "exec",
                    "e",
                    "login",
                    "logout",
                    "mcp",
                    "mcp-server",
                    "apply",
                    "a",
                    "completion",
                    "debug",
                    "proto",
                ][..],
            ),
        ] {
            for command in commands {
                assert!(
                    passthrough(kind, &[(*command).into()], true, true),
                    "{kind} {command}"
                );
                assert!(!passthrough(
                    kind,
                    &["prompt".into(), (*command).into()],
                    true,
                    true
                ));
            }
        }
        assert!(!passthrough("pi", &["exec".into()], true, true));
        assert!(!passthrough("omp", &["mcp".into()], true, true));
    }
}
