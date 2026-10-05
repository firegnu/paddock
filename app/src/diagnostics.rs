//! Diagnostics: a read-only look at the commands paddock runs and the latest outcomes of its agent
//! reads, config and layout, after Saddle's (`src/diagnostics.rs` at commit `df1c727`). Checked when
//! the page opens or Refresh is pressed; only the latest results are kept, in memory.
use crate::config::Config;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime},
};

/// The latest outcome of something paddock does again and again: when, and why it failed.
pub type Last = Option<(SystemTime, Result<String, String>)>;

/// How paddock was started, kept from `main`.
#[derive(Clone, Debug, Default)]
pub struct Startup {
    /// From Finder or the Dock rather than a terminal.
    pub desktop: bool,
    /// From the desktop: whether the login shell gave its PATH.
    pub login_path: Option<bool>,
}

impl gpui::Global for Startup {}

/// What paddock knows when the page is opened or refreshed.
pub struct Report {
    pub corral: String,
    pub shell: String,
    /// The latest `corral ls`: how many agents, or the error.
    pub agents: Last,
    pub config_path: PathBuf,
    /// The config came from the file at startup, rather than defaults for a missing one.
    pub config_from_file: bool,
    pub layout_path: Option<PathBuf>,
    /// The startup restore: whether a saved layout was used, or why it could not be.
    pub restore: Option<(SystemTime, Result<bool, String>)>,
    pub save: Option<(SystemTime, Result<(), String>)>,
    /// Saving is off: a layout file that could not be restored is kept, or paddock was started
    /// for one agent or program.
    pub save_off: bool,
    pub startup: Startup,
    pub checked: SystemTime,
}

/// A command as paddock would start it.
#[derive(Debug)]
pub struct Probe {
    pub path: Result<PathBuf, String>,
    pub version: Result<String, String>,
}

/// The checks that may wait on the file system or a command, run off the UI thread.
#[derive(Debug)]
pub struct Checks {
    pub corral: Probe,
    pub git: Probe,
    pub shell: Result<PathBuf, String>,
    /// The config file as it reads now: whether it exists, or why it cannot be used.
    pub config: Result<bool, String>,
}

/// The executable a command name or path starts: a name is looked up on PATH.
pub fn find(program: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let runnable = |path: &Path| {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if program.contains('/') {
        let path = std::path::absolute(program).map_err(|e| e.to_string())?;
        return if runnable(&path) {
            Ok(path)
        } else {
            Err(format!("{} is not an executable file", path.display()))
        };
    }
    std::env::var_os("PATH")
        .iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(program))
        .find(|path| runnable(path))
        .ok_or_else(|| format!("{program} not found on PATH"))
}

/// `corral --version`, the public version query: `version (contract N)`.
fn corral_version(corral: &str, timeout: Duration, cancel: &AtomicBool) -> Result<String, String> {
    let output = crate::command::run(corral, &["--version"], None, timeout, cancel)
        .map_err(|e| format!("{e:#}"))?;
    if !output.status.success() {
        return Err(format!(
            "exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("not JSON: {e}"))?;
    let version = value["version"]
        .as_str()
        .ok_or_else(|| "no version in the answer".to_owned())?;
    Ok(match value["contract"].as_str() {
        Some(contract) => format!("{version} (contract {contract})"),
        None => version.to_owned(),
    })
}

/// `git --version`, its first line.
fn git_version(git: &str, timeout: Duration, cancel: &AtomicBool) -> Result<String, String> {
    let output = crate::command::run(git, &["--version"], None, timeout, cancel)
        .map_err(|e| format!("{e:#}"))?;
    if !output.status.success() {
        return Err(format!("exited with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned())
}

/// Runs every check; blocks, so call it off the UI thread.
pub fn check(corral: &str, shell: &str, config: &Path, timeout: Duration) -> Checks {
    let cancel = AtomicBool::new(false);
    let probe =
        |program: &str, version: fn(&str, Duration, &AtomicBool) -> Result<String, String>| {
            let path = find(program);
            let version = match &path {
                Ok(_) => version(program, timeout, &cancel),
                Err(_) => Err("not run".into()),
            };
            Probe { path, version }
        };
    let config = match std::fs::read_to_string(config) {
        Ok(text) => Config::parse(&text)
            .map(|_| true)
            .map_err(|e| format!("{e:#}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    };
    Checks {
        corral: probe(corral, corral_version),
        git: probe("git", git_version),
        shell: find(shell),
        config,
    }
}

/// `HH:MM:SS` in local time.
pub fn clock(time: SystemTime) -> String {
    let seconds = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as libc::time_t;
    // SAFETY: `localtime_r` fills the `tm` it is given and keeps no pointer to it.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&seconds, &mut tm);
        tm
    };
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("paddock-diag-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script(dir: &Path, name: &str, text: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.display().to_string()
    }

    #[test]
    fn commands_are_found_as_they_would_start() {
        let dir = temp("find");
        let program = script(&dir, "tool", "#!/bin/sh\n");
        assert_eq!(find(&program).unwrap(), PathBuf::from(&program));
        let plain = dir.join("plain");
        std::fs::write(&plain, "").unwrap();
        assert!(
            find(plain.to_str().unwrap())
                .unwrap_err()
                .contains("not an executable")
        );
        assert!(find("sh").unwrap().is_absolute());
        assert!(
            find("no-such-command-here")
                .unwrap_err()
                .contains("not found on PATH")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corral_reports_its_version_or_why_not() {
        let dir = temp("version");
        let timeout = Duration::from_secs(5);
        let cancel = AtomicBool::new(false);
        let good = script(
            &dir,
            "corral",
            "#!/bin/sh\necho '{\"ok\":true,\"version\":\"0.1.0\",\"contract\":\"1\"}'\n",
        );
        assert_eq!(
            corral_version(&good, timeout, &cancel).unwrap(),
            "0.1.0 (contract 1)"
        );
        let odd = script(&dir, "odd", "#!/bin/sh\necho hello\n");
        assert!(
            corral_version(&odd, timeout, &cancel)
                .unwrap_err()
                .contains("not JSON")
        );
        let failing = script(&dir, "failing", "#!/bin/sh\necho broken >&2\nexit 3\n");
        assert!(
            corral_version(&failing, timeout, &cancel)
                .unwrap_err()
                .contains("broken")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_config_is_read_again_now() {
        let dir = temp("config");
        let corral = script(&dir, "corral", "#!/bin/sh\necho '{\"version\":\"1\"}'\n");
        let path = dir.join("config.toml");
        let checks = check(&corral, "sh", &path, Duration::from_secs(5));
        assert_eq!(checks.config, Ok(false));
        assert_eq!(checks.corral.version.as_deref(), Ok("1"));
        assert!(checks.shell.is_ok());
        std::fs::write(&path, "theme = \"tide\"\n").unwrap();
        assert_eq!(
            check(&corral, "sh", &path, Duration::from_secs(5)).config,
            Ok(true)
        );
        std::fs::write(&path, "refresh_ms = 0\n").unwrap();
        assert!(
            check(&corral, "sh", &path, Duration::from_secs(5))
                .config
                .is_err()
        );
        let missing = check("no-such-corral", "sh", &path, Duration::from_secs(5));
        assert!(missing.corral.path.is_err());
        assert_eq!(missing.corral.version.unwrap_err(), "not run");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
