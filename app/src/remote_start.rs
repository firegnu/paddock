//! Starting an agent on another machine (DESIGN §13 P5-124), the simple way: the kind, the
//! directory and the name, nothing else. What runs there goes through the user's own login shell
//! there (`$SHELL -l -i -c`), as an agent installed under nvm or the like is found only there; every
//! word the user typed is quoted as [`crate::remote::quote`] has it. `~` is that machine's home, as
//! it said on connecting, never this Mac's. Nothing here runs on the UI thread.
use crate::{
    new_agent::{Started, Tool, suggest_prefix},
    remote::{Link, command_line, quote},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

/// The kinds offered there, in this order: amp needs its plugin and agy a first message, which the
/// simple dialog has neither of.
pub const KINDS: [Tool; 4] = [Tool::Claude, Tool::Codex, Tool::Pi, Tool::Omp];
/// How long looking for the kinds may take.
const LOOK: Duration = Duration::from_secs(20);
/// How long `corral start` may take there, as on this Mac.
const START: Duration = Duration::from_secs(120);
/// How many directories are remembered for each machine.
const RECENT: usize = 5;

/// `inner` run by the shell of the user there, the one `$SHELL` names (`sh` when unset), as an
/// interactive login shell, its standard input on nothing: what its profile or its rc puts on the
/// PATH is found, as in a terminal there. zsh reads `~/.zshrc`, where nvm is set up, only when
/// interactive (P5-124b).
pub fn login(inner: &str) -> String {
    format!(
        "exec \"${{SHELL:-/bin/sh}}\" -l -i -c {} </dev/null",
        quote(inner)
    )
}

/// The command that says which of [`KINDS`] the login shell there finds: `command -v` for each,
/// the kind's name on a line of its own for each found.
pub fn look_command() -> String {
    let kinds: Vec<&str> = KINDS.iter().map(|tool| tool.kind()).collect();
    login(&format!(
        "for kind in {}; do if command -v \"$kind\" >/dev/null 2>&1; then printf '%s\\n' \"$kind\"; fi; done",
        kinds.join(" ")
    ))
}

/// The kinds [`look_command`] found, in the order of [`KINDS`]; what else the login shell printed
/// (a profile's greeting) is passed over.
pub fn found(stdout: &[u8]) -> Vec<Tool> {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    KINDS
        .into_iter()
        .filter(|tool| lines.contains(&tool.kind()))
        .collect()
}

/// Looks for the kinds there; blocks, so call it off the UI thread.
pub fn look(link: &Link) -> Result<Vec<Tool>, String> {
    let output = link
        .run(&look_command(), LOOK, &AtomicBool::new(false))
        .map_err(|problem| problem.to_string())?;
    Ok(found(&output.stdout))
}

/// Which field a problem is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Dir,
    Name,
}

/// The directory as typed: a full path, `~` or `~/…`, else why not.
pub fn check_dir(dir: &str) -> Result<(), String> {
    let dir = dir.trim();
    if dir.is_empty() {
        return Err("Directory is required".into());
    }
    if dir.chars().any(char::is_control) {
        return Err("Directory can't hold control characters".into());
    }
    if dir.starts_with('/') || dir == "~" || dir.starts_with("~/") {
        Ok(())
    } else if dir.starts_with('~') {
        Err("Only your own ~ works here; write ~/… or a full path".into())
    } else {
        Err("Write a full path, ~ or ~/…".into())
    }
}

/// The directory there: `~` and `~/…` under `home`, that machine's home.
pub fn expand(dir: &str, home: &str) -> String {
    let dir = dir.trim();
    match dir.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
        _ => dir.to_owned(),
    }
}

/// The name used when none is typed: the directory's last part (`home` for `~`), then the kind.
/// corral is asked for a free one like it (`--unique`), and says which.
pub fn default_name(dir: &str, tool: Tool) -> String {
    format!("{}/{}", name_prefix(dir), tool.kind())
}

/// The part of [`default_name`] before the `/`.
pub fn name_prefix(dir: &str) -> String {
    let dir = dir.trim().trim_end_matches('/');
    if dir == "~" {
        "home".into()
    } else {
        suggest_prefix(dir)
    }
}

/// A typed name, as on this Mac: `prefix/name`, no spaces, neither part starting with `-`.
pub fn check_name(name: &str) -> Result<(), String> {
    let problem = || Err("Name needs prefix/name, no spaces or leading '-'".into());
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return problem();
    }
    match name.split_once('/') {
        Some((prefix, rest))
            if !prefix.is_empty()
                && !rest.is_empty()
                && !prefix.starts_with('-')
                && !rest.starts_with('-') =>
        {
            Ok(())
        }
        _ => problem(),
    }
}

/// What the dialog asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub tool: Tool,
    /// As typed: `~` not yet that machine's home.
    pub dir: String,
    /// As typed; empty for [`default_name`].
    pub name: String,
}

impl Request {
    /// The first problem, by field.
    pub fn problem(&self) -> Option<(Field, String)> {
        if let Err(problem) = check_dir(&self.dir) {
            return Some((Field::Dir, problem));
        }
        if !self.name.trim().is_empty()
            && let Err(problem) = check_name(&self.name)
        {
            return Some((Field::Name, problem));
        }
        None
    }

    /// The name corral is asked for.
    pub fn name(&self) -> String {
        match self.name.trim() {
            "" => default_name(&self.dir, self.tool),
            _ => self.name.clone(),
        }
    }

    /// The command for ssh: `corral start` through the login shell there, its directory under
    /// `home`, that machine's home.
    pub fn command(&self, corral: &str, home: &str) -> String {
        let mut words = vec![
            corral.to_owned(),
            "start".into(),
            self.name(),
            "--unique".into(),
            "--cwd".into(),
            expand(&self.dir, home),
            "--label".into(),
            "role=regular".into(),
            "--".into(),
        ];
        words.extend(self.tool.command().split_whitespace().map(str::to_owned));
        login(&command_line(&words))
    }
}

/// Why it did not start: what came back, as it came, and the field it is about when corral says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub text: String,
    pub field: Option<Field>,
}

impl Failure {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            field: None,
        }
    }
}

/// Runs [`Request::command`] there with the corral and home it found on connecting; blocks, so
/// call it off the UI thread.
pub fn start(link: &Link, request: &Request) -> Result<Started, Failure> {
    let snapshot = link.snapshot();
    let label = link.label();
    let corral = snapshot
        .corral
        .ok_or_else(|| Failure::plain(format!("{label} is not connected")))?;
    let home = snapshot
        .home
        .ok_or_else(|| Failure::plain(format!("{label} did not say where its home is")))?;
    let command = request.command(&corral, &home);
    let output = link
        .run(&command, START, &AtomicBool::new(false))
        .map_err(|problem| Failure::plain(problem.to_string()))?;
    answer(&output)
}

/// What `corral start` answered there: the agent it started, or what it said, as it said it.
pub fn answer(output: &crate::command::Output) -> Result<Started, Failure> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (said, value) = match json_in(&stdout) {
        Some((at, value)) => (Some(stdout[at..].trim()), Some(value)),
        None => (None, None),
    };
    let refused = value
        .as_ref()
        .is_none_or(|value| value.get("ok") == Some(&Value::Bool(false)));
    if output.status.success() && !refused {
        let value = value.unwrap_or_default();
        let name = value["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                Failure::plain("corral start returned no name; whether it started is unknown")
            })?;
        return Ok(Started {
            name: name.to_owned(),
            instance: value["instance"].as_str().map(str::to_owned),
        });
    }
    // corral's own answer when there is one; else all that came back.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let said: Vec<&str> = match said {
        Some(json) => vec![json],
        None => [stdout.trim(), stderr.trim()]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect(),
    };
    let text = match said.is_empty() {
        true => match output.status.code() {
            Some(code) => format!("corral start exited {code} and said nothing"),
            None => "corral start was stopped and said nothing".into(),
        },
        false => said.join("\n"),
    };
    let field = value
        .filter(|value| value["error"] == "bad_cwd")
        .map(|_| Field::Dir);
    Err(Failure { text, field })
}

/// corral's JSON in what came back, and where it starts: all of it, or from a line on that starts
/// with `{`, after whatever a login profile printed first.
fn json_in(text: &str) -> Option<(usize, Value)> {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with('{')
            && let Ok(value) = serde_json::from_str::<Value>(&text[at..])
        {
            return Some((at, value));
        }
        at += line.len();
    }
    None
}

/// The directories recently started in, latest first, each machine's apart (by its `name`), and
/// apart from this Mac's New Agent's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecentDirs(BTreeMap<String, Vec<String>>);

impl RecentDirs {
    /// As kept at `path`; none when there is no list or it cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .map(Self)
            .unwrap_or_default()
    }

    pub fn of(&self, machine: &str) -> &[String] {
        self.0.get(machine).map_or(&[], Vec::as_slice)
    }

    /// Puts `dir` first among `machine`'s, once, keeping the latest few.
    pub fn remember(&mut self, machine: &str, dir: &str) {
        let dir = dir.trim();
        let dirs = self.0.entry(machine.to_owned()).or_default();
        dirs.retain(|known| known != dir);
        dirs.insert(0, dir.to_owned());
        dirs.truncate(RECENT);
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.0).map_err(std::io::Error::other)?;
        let temporary = path.with_extension("json.saving");
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(&temporary, path)
    }
}

/// Where they are kept, beside the layout and this Mac's recent projects.
pub fn recent_path() -> Option<PathBuf> {
    crate::layout_state::default_path().map(|path| path.with_file_name("recent-remote-dirs.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn output(code: i32, stdout: &str, stderr: &str) -> crate::command::Output {
        crate::command::Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// The words the login shell there is given, as `sh` reads the command ssh hands it.
    fn as_run(command: &str) -> Vec<String> {
        let words = shell_words::split(command).unwrap();
        assert_eq!(words[..5], ["exec", "${SHELL:-/bin/sh}", "-l", "-i", "-c"]);
        assert_eq!(words.len(), 7, "{command}");
        assert_eq!(words[6], "</dev/null");
        shell_words::split(&words[5]).unwrap()
    }

    #[test]
    fn it_starts_through_the_login_shell_there_every_typed_word_as_written() {
        let request = Request {
            tool: Tool::Codex,
            dir: "~/my dir/it's $(rm -rf ~); `id` \"q\"".into(),
            name: "p/it's;$x".into(),
        };
        let command = request.command("/home/me/.local/bin/corral", "/home/me");
        assert!(command.starts_with("exec \"${SHELL:-/bin/sh}\" -l -i -c '"));
        assert_eq!(
            as_run(&command),
            [
                "/home/me/.local/bin/corral",
                "start",
                "p/it's;$x",
                "--unique",
                "--cwd",
                "/home/me/my dir/it's $(rm -rf ~); `id` \"q\"",
                "--label",
                "role=regular",
                "--",
                "codex",
                "--yolo",
            ]
        );
    }

    #[test]
    fn tilde_is_that_machines_home_and_a_full_path_stays() {
        assert_eq!(expand("~", "/home/me"), "/home/me");
        assert_eq!(expand(" ~/x/y ", "/home/me"), "/home/me/x/y");
        assert_eq!(expand("/srv/a", "/home/me"), "/srv/a");
        let request = |dir: &str| Request {
            tool: Tool::Pi,
            dir: dir.into(),
            name: String::new(),
        };
        // That machine's home, whatever this Mac's is.
        let there = "/remote/home/of/someone";
        assert_ne!(std::env::var("HOME").ok().as_deref(), Some(there));
        let words = as_run(&request("~").command("/c", there));
        assert_eq!(words[5], there);
        let words = as_run(&request("~/w").command("/c", there));
        assert_eq!(words[5], format!("{there}/w"));
    }

    #[test]
    fn an_empty_name_is_the_directorys_last_part_and_the_kind() {
        let named = |dir: &str, tool, name: &str| {
            Request {
                tool,
                dir: dir.into(),
                name: name.into(),
            }
            .name()
        };
        assert_eq!(named("~", Tool::Pi, ""), "home/pi");
        assert_eq!(named("~/", Tool::Omp, ""), "home/omp");
        assert_eq!(named("~/Develop/ranch", Tool::Pi, ""), "ranch/pi");
        assert_eq!(named("/srv/my proj/", Tool::Claude, "  "), "my-proj/claude");
        assert_eq!(named("~/x", Tool::Codex, "a/b"), "a/b");
        // Asked for with --unique either way: corral says which name it took.
        for name in ["", "a/b"] {
            let words = as_run(
                &Request {
                    tool: Tool::Pi,
                    dir: "~".into(),
                    name: name.into(),
                }
                .command("/c", "/h"),
            );
            assert_eq!(words[3], "--unique");
        }
    }

    #[test]
    fn only_a_full_path_or_your_own_tilde_is_taken() {
        for good in ["/", "/srv/a b", "~", "~/", "~/x", " ~/x "] {
            assert_eq!(check_dir(good), Ok(()), "{good}");
        }
        assert_eq!(check_dir(""), Err("Directory is required".into()));
        assert_eq!(check_dir("   "), Err("Directory is required".into()));
        assert_eq!(
            check_dir("src/app"),
            Err("Write a full path, ~ or ~/…".into())
        );
        assert_eq!(check_dir("./x"), Err("Write a full path, ~ or ~/…".into()));
        assert!(check_dir("~root").unwrap_err().contains("own ~"));
        assert!(check_dir("~root/x").is_err());
        assert!(check_dir("/a\nb").is_err());
        let request = Request {
            tool: Tool::Pi,
            dir: "rel".into(),
            name: String::new(),
        };
        assert_eq!(request.problem().map(|p| p.0), Some(Field::Dir));
    }

    #[test]
    fn a_typed_name_keeps_this_macs_rules() {
        for good in ["a/b", "ranch/pi-2", "a/b/c"] {
            assert_eq!(check_name(good), Ok(()), "{good}");
        }
        for bad in ["ab", "a b/c", "-a/b", "a/-b", "/b", "a/", "a/\tb"] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
        let request = Request {
            tool: Tool::Pi,
            dir: "~".into(),
            name: "no slash".into(),
        };
        assert_eq!(request.problem().map(|p| p.0), Some(Field::Name));
    }

    #[test]
    fn the_kinds_found_are_read_in_order_past_whatever_the_profile_printed() {
        let command = look_command();
        let inner = &shell_words::split(&command).unwrap()[5];
        assert!(command.starts_with("exec \"${SHELL:-/bin/sh}\" -l -i -c '"));
        assert!(inner.contains("for kind in claude codex pi omp; do"));
        assert!(inner.contains("command -v \"$kind\""));
        assert_eq!(found(b"Welcome to 5090\nomp\npi\n"), [Tool::Pi, Tool::Omp]);
        assert_eq!(found(b"codex\nclaude\n"), [Tool::Claude, Tool::Codex]);
        // A path or an alias is not a kind's own line; amp and agy are never offered.
        assert!(found(b"/usr/bin/pi\npip\namp\nagy\n").is_empty());
        assert!(found(b"").is_empty());
    }

    #[test]
    fn corrals_answer_is_the_agent_or_what_it_said_as_it_said_it() {
        let started = answer(&output(
            0,
            "hello from .profile\n{\"ok\":true,\"name\":\"ranch/pi-2\",\"instance\":\"i-9\"}\n",
            "",
        ));
        assert_eq!(
            started,
            Ok(Started {
                name: "ranch/pi-2".into(),
                instance: Some("i-9".into()),
            })
        );
        let bad_cwd =
            r#"{"cwd":"/home/me/nope","error":"bad_cwd","message":"no such directory","ok":false}"#;
        assert_eq!(
            answer(&output(1, &format!("Welcome\n{bad_cwd}\n"), "a warning\n")),
            Err(Failure {
                text: bad_cwd.into(),
                field: Some(Field::Dir),
            })
        );
        let usage = r#"{"error":"usage","message":"bad name","ok":false}"#;
        assert_eq!(answer(&output(0, usage, "")).unwrap_err().field, None);
        assert_eq!(
            answer(&output(127, "", "zsh:1: no such file or directory: /c\n")),
            Err(Failure::plain("zsh:1: no such file or directory: /c"))
        );
        assert!(
            answer(&output(0, "{\"ok\":true}", ""))
                .unwrap_err()
                .text
                .contains("returned no name")
        );
        assert_eq!(
            answer(&output(3, "", "")).unwrap_err().text,
            "corral start exited 3 and said nothing"
        );
    }

    #[test]
    fn recent_directories_are_kept_for_each_machine_apart() {
        let mut recent = RecentDirs::default();
        recent.remember("studio", "~/a");
        recent.remember("5090", "~");
        recent.remember("studio", " ~/b ");
        recent.remember("studio", "~/a");
        assert_eq!(recent.of("studio"), ["~/a", "~/b"]);
        assert_eq!(recent.of("5090"), ["~"]);
        assert!(recent.of("other").is_empty());
        for n in 0..9 {
            recent.remember("5090", &format!("/p{n}"));
        }
        assert_eq!(recent.of("5090").len(), RECENT);
        assert_eq!(recent.of("5090")[0], "/p8");
        assert_eq!(recent.of("studio"), ["~/a", "~/b"]);

        let dir = std::env::temp_dir().join(format!("paddock-recent-dirs-{}", std::process::id()));
        let path = dir.join("recent-remote-dirs.json");
        assert_eq!(RecentDirs::load(&path), RecentDirs::default());
        recent.save(&path).unwrap();
        assert_eq!(RecentDirs::load(&path), recent);
        // This Mac's list is another file, and stays as it is.
        let local = dir.join("recent-projects.json");
        crate::new_agent::save_recent(&local, &["/mine".to_owned()]).unwrap();
        recent.save(&path).unwrap();
        assert_eq!(crate::new_agent::load_recent(&local), ["/mine"]);
        assert_ne!(
            recent_path().map(|p| p.file_name().map(ToOwned::to_owned)),
            crate::new_agent::recent_path().map(|p| p.file_name().map(ToOwned::to_owned))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
