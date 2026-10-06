//! The New Agent form's rules, as in Saddle's form (`src/launch.rs` at commit `df1c727`): a
//! project directory, Codex or Claude, a controller (always `main`) or a regular agent (named),
//! a prefix, the full command, a first message and where to open it; what it turns into is one
//! public `corral start` call. Starting and stopping run through `corral` here too.
use crate::{corral::Client, layout::Direction};
use anyhow::{Context, Result, bail};
use std::{sync::atomic::AtomicBool, time::Duration};

pub const CODEX_COMMAND: &str = "codex --yolo";
pub const CLAUDE_COMMAND: &str = "claude";
/// The prefix when the directory gives none, as in Saddle.
const FALLBACK_PREFIX: &str = "agents";
/// How long `corral start` and `corral stop` may take, as in Saddle.
const CORRAL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Codex,
    Claude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The active pane; a running shell there is kept and a new tab used instead.
    Current,
    Tab,
    Split(Direction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub project: String,
    pub tool: Tool,
    /// A regular agent, named; otherwise the project's controller, `main`.
    pub regular: bool,
    pub prefix: String,
    /// The regular agent's name; a controller is always `main`.
    pub name: String,
    pub command: String,
    pub prompt: String,
    pub place: Place,
    /// The prefix was typed, so it no longer follows the project.
    prefix_typed: bool,
}

/// A prefix from the project directory's name: its last part, with anything but letters, digits,
/// `-`, `_` and `.` turned into `-`.
pub fn suggest_prefix(project: &str) -> String {
    let last = project
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let prefix: String = last
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let prefix = prefix.trim_start_matches(['-', '.']).to_owned();
    if prefix.is_empty() {
        FALLBACK_PREFIX.into()
    } else {
        prefix
    }
}

/// `~` and `~/…` as the home directory.
fn expand_home(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{home}{rest}")
        }
        _ => path.to_owned(),
    }
}

impl Form {
    pub fn new(project: String, place: Place) -> Self {
        Self {
            prefix: suggest_prefix(&project),
            project,
            tool: Tool::Codex,
            regular: false,
            name: "main".into(),
            command: CODEX_COMMAND.into(),
            prompt: String::new(),
            place,
            prefix_typed: false,
        }
    }

    /// Another project; the prefix follows it until it has been typed.
    pub fn set_project(&mut self, project: String) {
        if !self.prefix_typed {
            self.prefix = suggest_prefix(&project);
        }
        self.project = project;
    }

    pub fn type_prefix(&mut self, prefix: String) {
        self.prefix_typed = true;
        self.prefix = prefix;
    }

    /// Codex or Claude; the command becomes that tool's.
    pub fn choose_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.command = match tool {
            Tool::Codex => CODEX_COMMAND,
            Tool::Claude => CLAUDE_COMMAND,
        }
        .into();
    }

    /// The name corral is asked for.
    pub fn full_name(&self) -> String {
        let name = if self.regular {
            self.name.as_str()
        } else {
            "main"
        };
        format!("{}/{name}", self.prefix)
    }

    /// The first problem, by field, as Saddle words it.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        let fields = [
            ("project", &self.project),
            ("prefix", &self.prefix),
            ("name", &self.name),
            ("command", &self.command),
            ("prompt", &self.prompt),
        ];
        if let Some((field, _)) = fields.iter().find(|(_, value)| value.contains('\0')) {
            return Some((field, "NUL bytes are not allowed".into()));
        }
        if self.project.trim().is_empty() {
            return Some(("project", "Directory is required".into()));
        }
        let bad = |value: &str| {
            value.trim().is_empty()
                || value.starts_with('-')
                || value.chars().any(char::is_whitespace)
        };
        if bad(&self.prefix) || self.prefix.contains('/') {
            return Some((
                "prefix",
                "Prefix needs text, no spaces, '/' or leading '-'".into(),
            ));
        }
        if self.regular && bad(&self.name) {
            return Some(("name", "Name needs text, no spaces or leading '-'".into()));
        }
        match shell_words::split(&self.command) {
            Err(_) => Some(("command", "Command has an unclosed quote".into())),
            Ok(words) if words.first().is_none_or(|w| w.is_empty()) => {
                Some(("command", "Command is required".into()))
            }
            Ok(_) => None,
        }
    }

    /// The arguments to `corral`.
    pub fn args(&self) -> Result<Vec<String>> {
        if let Some((_, problem)) = self.problem() {
            bail!("{problem}");
        }
        let words = shell_words::split(&self.command).context("Command has an unclosed quote")?;
        let role = if self.regular {
            "role=regular"
        } else {
            "role=controller"
        };
        let mut args = vec![
            "start".into(),
            self.full_name(),
            "--cwd".into(),
            expand_home(self.project.trim()),
            "--label".into(),
            role.into(),
        ];
        let program = words[0].rsplit('/').next().unwrap_or_default();
        if matches!(program, "claude" | "codex") {
            let mut model = None;
            let mut effort = None;
            let mut command_args = words.iter().skip(1).map(String::as_str);
            while let Some(arg) = command_args.next() {
                match arg {
                    "--" => break,
                    "--model" => model = command_args.next(),
                    "-m" if program == "codex" => model = command_args.next(),
                    "--effort" if program == "claude" => effort = command_args.next(),
                    "-c" | "--config" if program == "codex" => {
                        if let Some(value) = command_args
                            .next()
                            .and_then(|arg| arg.strip_prefix("model_reasoning_effort="))
                        {
                            effort = Some(value.trim_matches(['\'', '"']));
                        }
                    }
                    _ => {
                        if let Some(value) = arg.strip_prefix("--model=") {
                            model = Some(value);
                        } else if program == "claude"
                            && let Some(value) = arg.strip_prefix("--effort=")
                        {
                            effort = Some(value);
                        }
                    }
                }
            }
            for (key, value) in [("model", model), ("effort", effort)] {
                if let Some(value) = value.filter(|value| !value.is_empty()) {
                    args.extend(["--label".into(), format!("{key}={value}")]);
                }
            }
        }
        if !self.prompt.is_empty() {
            args.extend(["--prompt".into(), self.prompt.clone()]);
        }
        args.push("--".into());
        args.extend(words);
        Ok(args)
    }

    /// The call as it will run, quoted for reading.
    pub fn preview(&self, corral: &str) -> String {
        match self.args() {
            Ok(args) => std::iter::once(corral.to_owned())
                .chain(args)
                .map(|word| shell_words::quote(&word).into_owned())
                .collect::<Vec<_>>()
                .join(" "),
            Err(error) => format!("{error}"),
        }
    }
}

/// What `corral start` reported: the agent's name and instance.
#[derive(Debug, PartialEq)]
pub struct Started {
    pub name: String,
    pub instance: Option<String>,
}

/// Runs `corral start …`; blocks, so call it off the UI thread.
pub fn start(corral: &str, args: &[String]) -> Result<Started> {
    let client = Client {
        program: corral.to_owned(),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let value = client.json(&args, CORRAL_TIMEOUT, &AtomicBool::new(false))?;
    let name = value["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .context("corral start returned no name; whether it started is unknown")?;
    Ok(Started {
        name: name.to_owned(),
        instance: value["instance"].as_str().map(str::to_owned),
    })
}

/// Runs `corral stop NAME`; blocks, so call it off the UI thread.
pub fn stop(corral: &str, name: &str) -> Result<()> {
    let client = Client {
        program: corral.to_owned(),
    };
    client.json(&["stop", name], CORRAL_TIMEOUT, &AtomicBool::new(false))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_codex_controller_named_after_the_project() {
        let form = Form::new("/tmp/demo".into(), Place::Current);
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/main",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=controller",
                "--",
                "codex",
                "--yolo"
            ]
        );
    }

    #[test]
    fn claude_a_regular_name_and_a_first_message() {
        let mut form = Form::new("/tmp/demo".into(), Place::Tab);
        form.choose_tool(Tool::Claude);
        form.regular = true;
        form.name = "review".into();
        form.prompt = "先读 AGENTS.md".into();
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/review",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=regular",
                "--prompt",
                "先读 AGENTS.md",
                "--",
                "claude"
            ]
        );
        // A controller is always main, whatever the regular name says.
        form.regular = false;
        assert_eq!(form.full_name(), "demo/main");
    }

    #[test]
    fn commands_supply_model_and_effort_labels() {
        for (tool, command, model, effort, words) in [
            (
                Tool::Codex,
                "/opt/tools/claude --model 'opus[1m]' --effort high",
                "model=opus[1m]",
                "effort=high",
                vec![
                    "/opt/tools/claude",
                    "--model",
                    "opus[1m]",
                    "--effort",
                    "high",
                ],
            ),
            (
                Tool::Claude,
                r#"/opt/tools/codex --yolo -m gpt-6-astra -c 'model_reasoning_effort="xhigh"'"#,
                "model=gpt-6-astra",
                "effort=xhigh",
                vec![
                    "/opt/tools/codex",
                    "--yolo",
                    "-m",
                    "gpt-6-astra",
                    "-c",
                    r#"model_reasoning_effort="xhigh""#,
                ],
            ),
        ] {
            let mut form = Form::new("/tmp/demo".into(), Place::Current);
            form.choose_tool(tool);
            form.command = command.into();
            form.prompt = "hello".into();
            let mut expected = vec![
                "start",
                "demo/main",
                "--cwd",
                "/tmp/demo",
                "--label",
                "role=controller",
                "--label",
                model,
                "--label",
                effort,
                "--prompt",
                "hello",
                "--",
            ];
            expected.extend(words);
            assert_eq!(form.args().unwrap(), expected, "{command}");
        }
    }

    #[test]
    fn the_prefix_follows_the_project_until_typed() {
        let mut form = Form::new("/Users/me/My Project/".into(), Place::Current);
        assert_eq!(form.prefix, "My-Project");
        form.set_project("/tmp/paddock".into());
        assert_eq!(form.prefix, "paddock");
        form.type_prefix("work".into());
        form.set_project("/tmp/other".into());
        assert_eq!(form.prefix, "work");
        assert_eq!(suggest_prefix("/"), "agents");
        assert_eq!(suggest_prefix(".hidden"), "hidden");
    }

    #[test]
    fn problems_are_named_as_in_saddle() {
        let mut form = Form::new("/tmp/demo".into(), Place::Current);
        form.command = "codex 'unclosed".into();
        assert_eq!(form.problem().unwrap().0, "command");
        assert!(
            form.args()
                .unwrap_err()
                .to_string()
                .contains("unclosed quote")
        );
        form.choose_tool(Tool::Codex);
        form.prefix = "a/b".into();
        assert_eq!(form.problem().unwrap().0, "prefix");
        form.prefix = "demo".into();
        form.regular = true;
        form.name = "-x".into();
        assert_eq!(form.problem().unwrap().0, "name");
        form.name = "two words".into();
        assert_eq!(form.problem().unwrap().0, "name");
        form.name = "ok".into();
        form.project = "  ".into();
        assert_eq!(form.problem().unwrap().0, "project");
        form.project = "/tmp".into();
        form.command = String::new();
        assert_eq!(form.problem().unwrap().1, "Command is required");
    }

    #[test]
    fn the_preview_quotes_the_call() {
        let mut form = Form::new("/tmp/my demo".into(), Place::Current);
        form.prompt = "hello there".into();
        assert_eq!(
            form.preview("corral"),
            "corral start my-demo/main --cwd '/tmp/my demo' --label 'role=controller' \
             --prompt 'hello there' -- codex --yolo"
        );
    }

    #[test]
    fn home_is_expanded_in_the_directory() {
        let home = std::env::var("HOME").unwrap();
        let form = Form::new("~/code".into(), Place::Current);
        assert_eq!(form.args().unwrap()[3], format!("{home}/code"));
    }
}
