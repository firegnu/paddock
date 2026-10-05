//! Started from Finder, Launchpad or the Dock, paddock gets launchd's bare environment: a `PATH`
//! without `~/.local/bin` or Homebrew, and `/` as the working directory. Like other desktop apps
//! with terminals, it asks the user's login shell for its `PATH` once at start.
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const START: &str = "__PADDOCK_PATH_START__";
const END: &str = "__PADDOCK_PATH_END__";

/// Not started from a terminal: launchd never sets `TERM`, and LaunchServices starts apps in `/`
/// (an app opened with `open` from a terminal keeps that terminal's `TERM`, but not its directory).
pub fn from_desktop() -> bool {
    std::env::var_os("TERM").is_none()
        || std::env::current_dir().is_ok_and(|dir| dir == Path::new("/"))
}

/// The `PATH` the login shell printed between the markers; anything a profile prints around it is
/// ignored.
pub fn parse_path(output: &str) -> Option<String> {
    let start = output.find(START)? + START.len();
    let end = output[start..].find(END)? + start;
    let path = output[start..end].trim();
    (!path.is_empty()).then(|| path.to_owned())
}

/// Asks `shell` as a login shell for its `PATH`, giving up after `timeout`. Runs on the calling
/// thread only, so the caller may still change the environment afterwards.
pub fn login_path(shell: &Path, timeout: Duration) -> Option<String> {
    let script = format!("printf '{START}%s{END}' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-l", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    parse_path(&output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn the_path_is_read_between_the_markers() {
        let output = format!("Welcome!\n{START}/opt/homebrew/bin:/usr/bin{END}\nbye");
        assert_eq!(
            parse_path(&output).as_deref(),
            Some("/opt/homebrew/bin:/usr/bin")
        );
        assert_eq!(parse_path("no markers"), None);
        assert_eq!(parse_path(&format!("{START}{END}")), None);
        assert_eq!(parse_path(&format!("{START}/bin")), None);
    }

    /// A fake login shell in a temporary directory.
    fn shell(name: &str, body: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("paddock-launch-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shell");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn a_login_shell_is_asked_once_and_a_slow_one_is_given_up_on() {
        // A profile that prints a banner, then runs the command it was given.
        let talkative = shell(
            "talkative",
            "echo 'Last login: today'\nPATH=/custom/bin:/usr/bin\nshift\nexec /bin/sh -c \"$2\"",
        );
        assert_eq!(
            login_path(&talkative, Duration::from_secs(5)).as_deref(),
            Some("/custom/bin:/usr/bin")
        );
        let slow = shell("slow", "exec sleep 30");
        let started = Instant::now();
        assert_eq!(login_path(&slow, Duration::from_millis(200)), None);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            login_path(Path::new("/no/such/shell"), Duration::from_secs(1)),
            None
        );
        for path in [talkative, slow] {
            let _ = std::fs::remove_dir_all(path.parent().unwrap());
        }
    }
}
