//! Started from Finder, Launchpad or the Dock, paddock gets launchd's bare environment: a `PATH`
//! without `~/.local/bin` or Homebrew, and `/` as the working directory. Like other desktop apps
//! with terminals, it asks the user's login shell for its `PATH` once at start, allowing a slow
//! login to finish after the window opens. Children use that PATH without changing our environment.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{RwLock, atomic::AtomicBool, mpsc},
    time::Duration,
};

const START: &str = "__PADDOCK_PATH_START__";
const END: &str = "__PADDOCK_PATH_END__";

static LOGIN_PATH: RwLock<Option<OsString>> = RwLock::new(None);

/// A complete snapshot for one child or executable lookup. Terminal/headless launches keep the
/// caller's environment; only a successful desktop login installs an override.
pub fn path() -> Option<OsString> {
    LOGIN_PATH
        .read()
        .unwrap()
        .clone()
        .or_else(|| std::env::var_os("PATH"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginPath {
    Loading,
    Ready,
    Failed,
}

impl LoginPath {
    pub fn notice(self) -> Option<&'static str> {
        match self {
            Self::Loading => Some(
                "Still loading the login shell's PATH. New terminals and commands are using the limited startup PATH; brew and corral may be unavailable. Wait for this message to clear, then open a new terminal.",
            ),
            Self::Failed => Some(
                "Couldn't load the login shell's PATH. New terminals and commands are using the limited startup PATH; brew and corral may be unavailable. Check your login shell startup files, then quit and reopen paddock, or launch it from a working terminal.",
            ),
            Self::Ready => None,
        }
    }
}

/// Waits only `foreground` before allowing the window to open. The same login shell continues
/// off-thread, bounded by `timeout`; the receiver reports its final result after PATH is installed.
pub fn load_login_path(
    shell: PathBuf,
    foreground: Duration,
    timeout: Duration,
) -> (LoginPath, mpsc::Receiver<LoginPath>) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let state = match login_path(&shell, timeout) {
            Some(path) => {
                *LOGIN_PATH.write().unwrap() = Some(path.into());
                LoginPath::Ready
            }
            None => LoginPath::Failed,
        };
        let _ = tx.send(state);
    });
    let state = match rx.recv_timeout(foreground) {
        Ok(state) => state,
        Err(mpsc::RecvTimeoutError::Timeout) => LoginPath::Loading,
        Err(mpsc::RecvTimeoutError::Disconnected) => LoginPath::Failed,
    };
    (state, rx)
}

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

/// Asks `shell` for PATH with bounded output and a timeout covering both exit and pipe reads.
pub fn login_path(shell: &Path, timeout: Duration) -> Option<String> {
    let script = format!("printf '{START}%s{END}' \"$PATH\"");
    let output = crate::command::run_bounded(
        shell.to_str()?,
        &["-l", "-c", &script],
        None,
        &[],
        timeout,
        &AtomicBool::new(false),
        1024 * 1024,
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_path(std::str::from_utf8(&output.stdout).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, time::Instant};

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
    fn login_shell_output_is_parsed_and_the_final_deadline_is_bounded() {
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
