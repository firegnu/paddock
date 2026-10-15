//! Settings › Agents › Integrations (DESIGN §13 P5-94): which agents corral supports, whether
//! each is on this Mac, how corral connects it, and whether the corral, corral-dispatch and paddock
//! skills are in the places it reads skills from. Read from `corral integration status`, `ranch
//! dispatch install-skills --dry-run` and paddock's own skill; installing runs the same commands
//! with `--target <place> --yes` once the person has confirmed. An agent corral cannot connect at
//! start (amp, P5-98; agy, P5-88) needs a plugin in its own configuration, installed or removed with `corral
//! integration install|uninstall`.
use serde_json::Value;
use std::{sync::atomic::AtomicBool, time::Duration};

/// The skills a row looks after, in the order a note names them.
pub const SKILLS: [&str; 3] = ["corral", "corral-dispatch", "paddock"];

/// One agent kind corral supports.
#[derive(Clone, Debug, PartialEq)]
pub struct Agent {
    pub kind: String,
    /// The program found on PATH, if any.
    pub path: Option<String>,
    pub version: Option<String>,
    /// How corral connects its hooks: `launch` for when it starts the agent, `global` for a file
    /// installed once in the agent's own configuration.
    pub hooks: String,
    /// That file, for a `global` agent.
    pub install: Option<Install>,
    /// The places it reads skills from, by `install-skills --target` name.
    pub targets: Vec<String>,
}

/// The file or folder corral installs in an agent's own configuration (amp's plugin, agy's).
#[derive(Clone, Debug, PartialEq)]
pub struct Install {
    pub path: String,
    /// `create`, `same`, `overwrite`, `foreign`, or anything newer, shown as it is.
    pub status: String,
    /// Whether the agent itself has the plugin turned on: only agy says (P5-88); none when it
    /// doesn't say or corral couldn't tell.
    pub enabled: Option<bool>,
}

/// The Status column of a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// corral connects it each time it starts the agent.
    BuiltIn,
    /// The plugin: not there yet, there and current, there but different, or not corral's.
    Install,
    Installed,
    /// There and current, but turned off in the agent itself (agy).
    Disabled,
    Update,
    NotManaged,
    /// A word paddock doesn't know, as corral gives it.
    Other(String),
}

/// What the plugin's button would do, for the confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginPlan {
    pub kind: String,
    pub name: String,
    pub path: String,
    /// How the file stood when the row was read: `create`, `overwrite` or, to remove it, `same`.
    pub status: String,
    pub remove: bool,
}

/// Why nothing was done when the plugin no longer stands as the confirmation showed it.
pub const CHANGED: &str = "the file changed while you were deciding; nothing was written";

/// One skill file at one place, as `install-skills --dry-run` reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub skill: &'static str,
    pub target: String,
    pub path: String,
    /// `create`, `same`, `overwrite`, `foreign`, or anything newer, taken as nothing to do.
    pub status: String,
}

/// What became of asking ranch about corral-dispatch.
#[derive(Clone, Debug, PartialEq)]
pub enum Ranch {
    Answered,
    NotFound,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub agents: Vec<Agent>,
    pub files: Vec<File>,
    pub ranch: Ranch,
}

/// The Skills column of a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Skills {
    /// The agent isn't on this Mac, or reads no place corral installs to.
    None,
    UpToDate,
    /// Only files that aren't the skills' own are there; nothing is touched.
    NotManaged,
    /// Some skill is missing or different: `Install` when none is there yet, else `Update`.
    Act {
        install: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub kind: String,
    pub name: String,
    /// The version under the name, `Installed` without one, `Not found` without the program.
    pub detail: String,
    pub found: bool,
    pub status: Option<Status>,
    /// Beside the Status: why the plugin needs updating or is left alone.
    pub status_note: Option<String>,
    pub skills: Skills,
    /// Beside the button: which skills are missing or changed, and a ranch that didn't answer.
    pub note: Option<String>,
}

/// What a button would write, for the confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub kind: String,
    pub name: String,
    pub install: bool,
    /// Each skill at each place that needs writing.
    pub steps: Vec<(&'static str, String)>,
    /// The files written, each with whether it replaces one.
    pub files: Vec<(String, bool)>,
    /// The skills folders written into.
    pub folders: Vec<String>,
    /// Other agents on this Mac reading those folders, who get the skills too.
    pub shared: Vec<String>,
}

/// The name a row shows for an agent kind.
pub fn name(kind: &str) -> String {
    match kind {
        "claude" => "Claude Code".into(),
        "codex" => "Codex".into(),
        _ => kind.into(),
    }
}

const UNREADABLE: &str = "its answer wasn't understood";

/// The answer's own complaint when it isn't `ok`.
fn ok(value: &Value) -> Result<(), String> {
    if value["ok"] == true {
        return Ok(());
    }
    Err(value["error"]["message"]
        .as_str()
        .or(value["error"].as_str())
        .unwrap_or(UNREADABLE)
        .into())
}

/// `corral integration status`: the agents and where the corral skill stands.
pub fn parse_status(value: &Value) -> Result<(Vec<Agent>, Vec<File>), String> {
    ok(value)?;
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let mut agents = Vec::new();
    let mut files: Vec<File> = Vec::new();
    for agent in value["agents"].as_array().ok_or(UNREADABLE)? {
        let kind = text(&agent["kind"]).ok_or(UNREADABLE)?;
        let mut targets = Vec::new();
        for skill in agent["skills"].as_array().into_iter().flatten() {
            let file = File {
                skill: "corral",
                target: text(&skill["target"]).ok_or(UNREADABLE)?,
                path: text(&skill["path"]).ok_or(UNREADABLE)?,
                status: text(&skill["status"]).ok_or(UNREADABLE)?,
            };
            targets.push(file.target.clone());
            if !files.iter().any(|f| f.target == file.target) {
                files.push(file);
            }
        }
        agents.push(Agent {
            kind,
            path: text(&agent["path"]),
            version: text(&agent["version"]),
            hooks: text(&agent["hooks"]).unwrap_or_default(),
            install: match (
                text(&agent["install"]["path"]),
                text(&agent["install"]["status"]),
            ) {
                (Some(path), Some(status)) => Some(Install {
                    path,
                    status,
                    enabled: agent["install"]["enabled"].as_bool(),
                }),
                _ => None,
            },
            targets,
        });
    }
    Ok((agents, files))
}

/// An `install-skills --dry-run` answer for `skill`: its files, by place.
pub fn parse_plan(skill: &'static str, value: &Value) -> Result<Vec<File>, String> {
    ok(value)?;
    let text = |value: &Value| value.as_str().map(str::to_owned).ok_or(UNREADABLE);
    value["items"]
        .as_array()
        .ok_or(UNREADABLE)?
        .iter()
        .map(|item| {
            Ok(File {
                skill,
                target: text(&item["agent"])?,
                path: text(&item["path"])?,
                status: text(&item["status"])?,
            })
        })
        .collect()
}

/// paddock's own skill, as `skills::install` reports it.
type Paddock<'a> = &'a dyn Fn(&[String], bool) -> anyhow::Result<Value>;

/// Asks corral, ranch and paddock's own skill. Slow: corral runs every agent's `--version`.
pub fn read(corral: &str, cancel: &AtomicBool) -> Result<Report, String> {
    read_with(corral, "ranch", &crate::skills::install, cancel)
}

/// Long enough for corral's three seconds per `--version`, and for writing a few files.
const TIMEOUT: Duration = Duration::from_secs(20);

/// One JSON answer from `program`, or why there is none.
fn ask(program: &str, args: &[&str], cancel: &AtomicBool) -> Result<Value, String> {
    crate::diagnostics::find(program).map_err(|_| format!("{program} not found"))?;
    let output =
        crate::command::run(program, args, None, TIMEOUT, cancel).map_err(|e| format!("{e:#}"))?;
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        let errors = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if errors.is_empty() {
            UNREADABLE.into()
        } else {
            errors
        });
    };
    ok(&value)?;
    Ok(value)
}

fn read_with(
    corral: &str,
    ranch: &str,
    paddock: Paddock,
    cancel: &AtomicBool,
) -> Result<Report, String> {
    let (agents, mut files) = parse_status(&ask(corral, &["integration", "status"], cancel)?)?;
    let ranch = if crate::diagnostics::find(ranch).is_err() {
        Ranch::NotFound
    } else {
        match ask(ranch, &["dispatch", "install-skills", "--dry-run"], cancel)
            .and_then(|value| parse_plan("corral-dispatch", &value))
        {
            Ok(dispatch) => {
                files.extend(dispatch);
                Ranch::Answered
            }
            Err(error) => Ranch::Failed(error),
        }
    };
    let own = paddock(&[], false).map_err(|e| format!("paddock's skill: {e:#}"))?;
    files.extend(parse_plan("paddock", &own)?);
    Ok(Report {
        agents,
        files,
        ranch,
    })
}

/// Writes what `plan` says.
pub fn install(corral: &str, plan: &Plan, cancel: &AtomicBool) -> Result<(), String> {
    install_with(corral, "ranch", &crate::skills::install, plan, cancel)
}

fn install_with(
    corral: &str,
    ranch: &str,
    paddock: Paddock,
    plan: &Plan,
    cancel: &AtomicBool,
) -> Result<(), String> {
    for (skill, target) in &plan.steps {
        let target = target.as_str();
        let answer = match *skill {
            "corral" => ask(
                corral,
                &["install-skills", "--target", target, "--yes"],
                cancel,
            ),
            "corral-dispatch" => ask(
                ranch,
                &["dispatch", "install-skills", "--target", target, "--yes"],
                cancel,
            ),
            _ => paddock(&[target.to_owned()], true)
                .map_err(|e| format!("{e:#}"))
                .and_then(|value| ok(&value).map(|()| value)),
        };
        answer.map_err(|error| format!("{skill}: {error}"))?;
    }
    Ok(())
}

/// Whether writing would change the file: it is missing or different.
fn needed(status: &str) -> bool {
    matches!(status, "create" | "overwrite")
}

/// The skill files at `targets`.
fn standing<'a>(report: &'a Report, targets: &[String]) -> Vec<&'a File> {
    report
        .files
        .iter()
        .filter(|f| targets.contains(&f.target))
        .collect()
}

/// The rows, in corral's order.
pub fn rows(report: &Report) -> Vec<Row> {
    report
        .agents
        .iter()
        .map(|agent| {
            let found = agent.path.is_some();
            let detail = match (&agent.path, &agent.version) {
                (None, _) => "Not found".into(),
                // amp adds when it was released and how long ago, which goes stale on screen.
                (Some(_), Some(version)) => match version.find(" (released ") {
                    Some(at) => version[..at].to_owned(),
                    None => version.clone(),
                },
                (Some(_), None) => "Installed".into(),
            };
            let (status, status_note) = match (found, agent.hooks.as_str(), &agent.install) {
                (false, ..) => (None, None),
                (true, "launch", _) => (Some(Status::BuiltIn), None),
                (true, "global", Some(install)) => {
                    let file = install.path.rsplit('/').next().unwrap_or_default();
                    match install.status.as_str() {
                        "create" => (Some(Status::Install), None),
                        "same" if install.enabled == Some(false) => (
                            Some(Status::Disabled),
                            Some(format!("Run {} plugin enable corral", agent.kind)),
                        ),
                        "same" => (Some(Status::Installed), None),
                        "overwrite" => (Some(Status::Update), Some("plugin changed".into())),
                        "foreign" => (
                            Some(Status::NotManaged),
                            Some(format!("{file} isn't corral's")),
                        ),
                        other => (Some(Status::Other(other.into())), None),
                    }
                }
                (true, other, _) => (Some(Status::Other(other.into())), None),
            };
            let files = standing(report, &agent.targets);
            let skills = if !found || files.is_empty() {
                Skills::None
            } else if files.iter().any(|f| needed(&f.status)) {
                Skills::Act {
                    install: files.iter().all(|f| f.status == "create"),
                }
            } else if files.iter().any(|f| f.status == "same") {
                Skills::UpToDate
            } else {
                Skills::NotManaged
            };
            let mut notes = Vec::new();
            if skills == (Skills::Act { install: false }) {
                for skill in SKILLS {
                    let of = || files.iter().filter(|f| f.skill == skill);
                    if of().any(|f| f.status == "overwrite") {
                        notes.push(format!("{skill} changed"));
                    } else if of().any(|f| f.status == "create") {
                        notes.push(format!("{skill} missing"));
                    }
                }
            }
            if found {
                match report.ranch {
                    Ranch::Answered => {}
                    Ranch::NotFound => notes.push("ranch not found".into()),
                    Ranch::Failed(_) => notes.push("ranch didn't answer".into()),
                }
            }
            Row {
                kind: agent.kind.clone(),
                name: name(&agent.kind),
                detail,
                found,
                status,
                status_note,
                skills,
                note: (!notes.is_empty()).then(|| notes.join(", ")),
            }
        })
        .collect()
}

/// What the button on `kind`'s row would write.
pub fn plan(report: &Report, kind: &str) -> Option<Plan> {
    let agent = report.agents.iter().find(|a| a.kind == kind)?;
    agent.path.as_ref()?;
    let files: Vec<&File> = standing(report, &agent.targets)
        .into_iter()
        .filter(|f| needed(&f.status))
        .collect();
    if files.is_empty() {
        return None;
    }
    let mut steps = Vec::new();
    let mut folders = Vec::new();
    for target in &agent.targets {
        for skill in SKILLS {
            let here = || {
                files
                    .iter()
                    .filter(|f| f.skill == skill && &f.target == target)
            };
            if here().next().is_some() {
                steps.push((skill, target.clone()));
            }
            for file in here() {
                let folder = skills_folder(&file.path, skill);
                if !folders.contains(&folder) {
                    folders.push(folder);
                }
            }
        }
    }
    let targets: Vec<&String> = steps.iter().map(|(_, target)| target).collect();
    let shared = report
        .agents
        .iter()
        .filter(|other| other.kind != kind && other.path.is_some())
        .filter(|other| other.targets.iter().any(|t| targets.contains(&t)))
        .map(|other| name(&other.kind))
        .collect();
    Some(Plan {
        kind: kind.into(),
        name: name(kind),
        install: report
            .files
            .iter()
            .filter(|f| agent.targets.contains(&f.target))
            .all(|f| f.status == "create"),
        steps,
        files: files
            .iter()
            .map(|f| (f.path.clone(), f.status == "overwrite"))
            .collect(),
        folders,
        shared,
    })
}

/// What `kind`'s plugin button would do: install or update it, or with `remove`, remove it; none
/// when the row offers no such button.
pub fn plugin_plan(report: &Report, kind: &str, remove: bool) -> Option<PluginPlan> {
    let agent = report.agents.iter().find(|a| a.kind == kind)?;
    agent.path.as_ref()?;
    let install = agent.install.as_ref().filter(|_| agent.hooks == "global")?;
    let offered = if remove {
        install.status == "same"
    } else {
        needed(&install.status)
    };
    offered.then(|| PluginPlan {
        kind: kind.into(),
        name: name(kind),
        path: install.path.clone(),
        status: install.status.clone(),
        remove,
    })
}

/// How `kind`'s plugin stands (`create`, `same`…, and turned off in agy), for New Agent to warn
/// before starting it; none when corral doesn't answer or the kind has no plugin. Slow, like
/// [`read`].
pub fn plugin_status(corral: &str, kind: &str, cancel: &AtomicBool) -> Option<Install> {
    let (agents, _) = parse_status(&ask(corral, &["integration", "status"], cancel).ok()?).ok()?;
    let agent = agents.into_iter().find(|a| a.kind == kind)?;
    agent.install
}

/// Installs, updates or removes the plugin as `plan` says, first checking with a dry run that it
/// still stands as the confirmation showed it. Done, it gives corral's warnings, such as agy's
/// plugin folder left in place.
pub fn plugin(corral: &str, plan: &PluginPlan, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    let (action, expected) = if plan.remove {
        ("uninstall", "remove")
    } else {
        ("install", plan.status.as_str())
    };
    let call = |last: &str| {
        ask(
            corral,
            &["integration", action, "--agent", &plan.kind, last],
            cancel,
        )
    };
    if call("--dry-run")?["status"] != expected {
        return Err(CHANGED.into());
    }
    let done = call("--yes")?;
    let warnings: Vec<String> = done["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if done["written"] != true {
        return Err(if warnings.is_empty() {
            "nothing was written".into()
        } else {
            format!("nothing was written: {}", warnings.join("; "))
        });
    }
    Ok(warnings)
}

/// The skills folder a skill's file sits in: `<folder>/<skill>/…`.
fn skills_folder(path: &str, skill: &str) -> String {
    let marker = format!("/{skill}/");
    path.rfind(&marker)
        .map_or_else(|| path.to_owned(), |at| path[..at].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status() -> Value {
        json!({"ok":true,"agents":[
            {"kind":"claude","path":"/bin/claude","version":"2.1.299 (Claude Code)","hooks":"launch",
             "skills":[{"target":"claude","path":"/h/.claude/skills/corral/SKILL.md","status":"same"}]},
            {"kind":"codex","path":"/bin/codex","version":"codex-cli 0.162.1","hooks":"launch",
             "skills":[{"target":"codex","path":"/h/.agents/skills/corral/SKILL.md","status":"same"}]},
            {"kind":"pi","path":"/bin/pi","version":null,"hooks":"launch",
             "skills":[{"target":"codex","path":"/h/.agents/skills/corral/SKILL.md","status":"same"}]},
            {"kind":"omp","path":null,"version":null,"hooks":"launch",
             "skills":[{"target":"claude","path":"/h/.claude/skills/corral/SKILL.md","status":"same"},
                       {"target":"codex","path":"/h/.agents/skills/corral/SKILL.md","status":"same"}]}
        ]})
    }
    fn plan_json(claude: &str, codex: &str) -> Value {
        json!({"ok":true,"action":"install","dry_run":true,"written":false,"warnings":[],"items":[
            {"agent":"claude","path":"/h/.claude/skills/paddock/SKILL.md","status":claude},
            {"agent":"codex","path":"/h/.agents/skills/paddock/SKILL.md","status":codex}]})
    }
    fn dispatch(status: &str) -> Value {
        let items: Vec<_> = ["claude", "codex"]
            .iter()
            .flat_map(|target| {
                let base = if *target == "claude" { ".claude" } else { ".agents" };
                ["SKILL.md", "README.md"].map(|file| {
                    json!({"agent":target,"path":format!("/h/{base}/skills/corral-dispatch/{file}"),"status":status})
                })
            })
            .collect();
        json!({"ok":true,"action":"install","dry_run":true,"written":false,"warnings":[],"items":items})
    }
    fn report(paddock: (&str, &str), dispatch_status: &str) -> Report {
        let (agents, mut files) = parse_status(&status()).unwrap();
        files.extend(parse_plan("corral-dispatch", &dispatch(dispatch_status)).unwrap());
        files.extend(parse_plan("paddock", &plan_json(paddock.0, paddock.1)).unwrap());
        Report {
            agents,
            files,
            ranch: Ranch::Answered,
        }
    }
    fn row<'a>(rows: &'a [Row], kind: &str) -> &'a Row {
        rows.iter().find(|row| row.kind == kind).unwrap()
    }

    #[test]
    fn the_status_gives_each_agent_its_places_and_the_corral_skill_at_each() {
        let (agents, files) = parse_status(&status()).unwrap();
        assert_eq!(
            agents.iter().map(|a| a.kind.as_str()).collect::<Vec<_>>(),
            ["claude", "codex", "pi", "omp"]
        );
        assert_eq!(agents[3].targets, ["claude", "codex"]);
        assert_eq!(agents[3].path, None);
        assert_eq!(agents[0].version.as_deref(), Some("2.1.299 (Claude Code)"));
        // A place several agents read is one file.
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .all(|f| f.skill == "corral" && f.status == "same")
        );
    }

    #[test]
    fn an_answer_that_is_not_ok_or_not_understood_is_an_error() {
        assert!(parse_status(&json!({"ok":false,"error":"usage"})).is_err());
        assert!(parse_status(&json!({"ok":true})).is_err());
        assert!(parse_status(&json!({"ok":true,"agents":[{"path":"/bin/x"}]})).is_err());
        assert!(parse_plan("paddock", &json!({"ok":true})).is_err());
    }

    #[test]
    fn rows_follow_corral_and_say_what_each_agent_is() {
        let rows = rows(&report(("same", "same"), "same"));
        assert_eq!(
            rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Claude Code", "Codex", "pi", "omp"]
        );
        let claude = row(&rows, "claude");
        assert_eq!(claude.detail, "2.1.299 (Claude Code)");
        assert_eq!(claude.status, Some(Status::BuiltIn));
        assert_eq!(claude.status_note, None);
        assert_eq!(claude.skills, Skills::UpToDate);
        assert_eq!(claude.note, None);
        assert_eq!(row(&rows, "pi").detail, "Installed");
        let omp = row(&rows, "omp");
        assert_eq!((omp.found, omp.detail.as_str()), (false, "Not found"));
        assert_eq!(
            (omp.status.clone(), omp.skills.clone()),
            (None, Skills::None)
        );
    }

    #[test]
    fn hooks_paddock_does_not_know_are_shown_as_they_are() {
        let mut report = report(("same", "same"), "same");
        report.agents[0].hooks = "spliced".into();
        assert_eq!(
            rows(&report)[0].status,
            Some(Status::Other("spliced".into()))
        );
        // Global, but without the file it would install: nothing to offer.
        report.agents[0].hooks = "global".into();
        assert_eq!(
            rows(&report)[0].status,
            Some(Status::Other("global".into()))
        );
    }

    #[test]
    fn a_changed_skill_in_a_shared_place_asks_every_agent_reading_it_to_update() {
        let rows = rows(&report(("same", "overwrite"), "same"));
        assert_eq!(row(&rows, "claude").skills, Skills::UpToDate);
        for kind in ["codex", "pi"] {
            assert_eq!(row(&rows, kind).skills, Skills::Act { install: false });
            assert_eq!(row(&rows, kind).note.as_deref(), Some("paddock changed"));
        }
    }

    #[test]
    fn nothing_there_yet_is_install_without_a_note() {
        let mut report = report(("create", "create"), "create");
        for file in &mut report.files {
            file.status = "create".into();
        }
        let rows = rows(&report);
        assert_eq!(row(&rows, "claude").skills, Skills::Act { install: true });
        assert_eq!(row(&rows, "claude").note, None);
    }

    #[test]
    fn missing_and_changed_skills_are_named_together() {
        let rows = rows(&report(("create", "same"), "overwrite"));
        assert_eq!(
            row(&rows, "claude").note.as_deref(),
            Some("corral-dispatch changed, paddock missing")
        );
        assert_eq!(row(&rows, "claude").skills, Skills::Act { install: false });
    }

    #[test]
    fn files_that_are_not_the_skills_own_are_left_out() {
        let report = report(("foreign", "same"), "same");
        assert_eq!(row(&rows(&report), "claude").skills, Skills::UpToDate);
        let mut report = report;
        for file in report.files.iter_mut().filter(|f| f.target == "claude") {
            file.status = "foreign".into();
        }
        assert_eq!(row(&rows(&report), "claude").skills, Skills::NotManaged);
    }

    #[test]
    fn without_ranch_corral_dispatch_is_not_counted_and_the_row_says_so() {
        let mut report = report(("same", "same"), "create");
        report.files.retain(|f| f.skill != "corral-dispatch");
        report.ranch = Ranch::NotFound;
        let claude = rows(&report).remove(0);
        assert_eq!(claude.skills, Skills::UpToDate);
        assert_eq!(claude.note.as_deref(), Some("ranch not found"));
        report.ranch = Ranch::Failed("boom".into());
        assert_eq!(
            rows(&report)[0].note.as_deref(),
            Some("ranch didn't answer")
        );
    }

    #[test]
    fn the_plan_writes_only_what_is_needed_where_this_agent_reads() {
        let report = report(("same", "overwrite"), "same");
        let plan = plan(&report, "pi").unwrap();
        assert!(!plan.install);
        assert_eq!(plan.steps, [("paddock", "codex".to_string())]);
        assert_eq!(
            plan.files,
            [("/h/.agents/skills/paddock/SKILL.md".to_string(), true)]
        );
        assert_eq!(plan.folders, ["/h/.agents/skills"]);
        // omp reads the folder too but isn't on this Mac.
        assert_eq!(plan.shared, ["Codex"]);
        assert_eq!(super::plan(&report, "claude"), None);
    }

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "paddock-integrations-{}",
            crate::control::random_id().unwrap()
        ));
        std::fs::create_dir(&dir).unwrap();
        dir
    }
    fn script(dir: &std::path::Path, name: &str, text: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.display().to_string()
    }
    fn echo(value: &Value) -> String {
        format!("#!/bin/sh\necho '{value}'\n")
    }
    fn paddock(targets: &[String], write: bool) -> anyhow::Result<Value> {
        assert!(targets.is_empty() && !write, "reading only looks");
        Ok(plan_json("same", "create"))
    }

    #[test]
    fn reading_asks_corral_ranch_and_paddock() {
        let dir = temp();
        let corral = script(&dir, "corral", &echo(&status()));
        let ranch = script(&dir, "ranch", &echo(&dispatch("same")));
        let report = read_with(&corral, &ranch, &paddock, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.agents.len(), 4);
        assert_eq!(report.ranch, Ranch::Answered);
        assert_eq!(report.files.len(), 2 + 4 + 2);
        assert!(
            report
                .files
                .iter()
                .any(|f| f.skill == "paddock" && f.status == "create")
        );
    }

    #[test]
    fn without_ranch_the_rest_still_reads() {
        let dir = temp();
        let corral = script(&dir, "corral", &echo(&status()));
        let missing = dir.join("ranch").display().to_string();
        let report = read_with(&corral, &missing, &paddock, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.ranch, Ranch::NotFound);
        assert!(report.files.iter().all(|f| f.skill != "corral-dispatch"));
        let failing = script(&dir, "failing", "#!/bin/sh\necho oops >&2\nexit 2\n");
        let report = read_with(&corral, &failing, &paddock, &AtomicBool::new(false)).unwrap();
        assert!(matches!(report.ranch, Ranch::Failed(_)));
    }

    #[test]
    fn corral_missing_or_refusing_is_the_error() {
        let dir = temp();
        let missing = dir.join("corral").display().to_string();
        let error = read_with(&missing, "ranch", &paddock, &AtomicBool::new(false)).unwrap_err();
        assert!(error.contains("not found"), "{error}");
        let old = script(
            &dir,
            "old",
            "#!/bin/sh\necho '{\"ok\":false,\"error\":{\"code\":\"usage\",\"message\":\"unknown command integration\"}}'\nexit 1\n",
        );
        let error = read_with(&old, "ranch", &paddock, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(error, "unknown command integration");
    }

    #[test]
    fn installing_runs_each_step_at_its_place() {
        let dir = temp();
        let log = dir.join("log");
        let record = |name| {
            script(
                &dir,
                name,
                &format!(
                    "#!/bin/sh\necho \"{name} $*\" >> '{}'\necho '{{\"ok\":true,\"written\":true}}'\n",
                    log.display()
                ),
            )
        };
        let (corral, ranch) = (record("corral"), record("ranch"));
        let written = std::cell::RefCell::new(Vec::new());
        let paddock = |targets: &[String], write: bool| {
            written.borrow_mut().push((targets.to_vec(), write));
            Ok(json!({"ok":true,"written":true}))
        };
        let plan = Plan {
            kind: "omp".into(),
            name: "omp".into(),
            install: false,
            steps: vec![
                ("corral", "claude".into()),
                ("corral-dispatch", "codex".into()),
                ("paddock", "codex".into()),
            ],
            files: Vec::new(),
            folders: Vec::new(),
            shared: Vec::new(),
        };
        install_with(&corral, &ranch, &paddock, &plan, &AtomicBool::new(false)).unwrap();
        let log = std::fs::read_to_string(log).unwrap();
        assert_eq!(
            log.lines().collect::<Vec<_>>(),
            [
                "corral install-skills --target claude --yes",
                "ranch dispatch install-skills --target codex --yes",
            ]
        );
        assert_eq!(*written.borrow(), [(vec!["codex".to_string()], true)]);
    }

    #[test]
    fn a_step_that_fails_stops_with_its_reason() {
        let dir = temp();
        let corral = script(
            &dir,
            "corral",
            "#!/bin/sh\necho '{\"ok\":false,\"error\":{\"code\":\"io\",\"message\":\"permission denied\"}}'\nexit 1\n",
        );
        let plan = Plan {
            kind: "pi".into(),
            name: "pi".into(),
            install: true,
            steps: vec![("corral", "codex".into()), ("paddock", "codex".into())],
            files: Vec::new(),
            folders: Vec::new(),
            shared: Vec::new(),
        };
        let paddock = |_: &[String], _| -> anyhow::Result<Value> { panic!("stopped before") };
        let error =
            install_with(&corral, "ranch", &paddock, &plan, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(error, "corral: permission denied");
    }

    #[test]
    fn the_plan_for_an_agent_reading_two_places_covers_both() {
        let mut report = report(("create", "create"), "same");
        report.agents[3].path = Some("/bin/omp".into());
        let plan = plan(&report, "omp").unwrap();
        assert_eq!(
            plan.steps,
            [
                ("paddock", "claude".to_string()),
                ("paddock", "codex".to_string())
            ]
        );
        assert_eq!(plan.folders, ["/h/.claude/skills", "/h/.agents/skills"]);
        assert_eq!(plan.shared, ["Claude Code", "Codex", "pi"]);
    }

    const PLUGIN: &str = "/h/.config/amp/plugins/corral.ts";

    /// The status with amp, its plugin standing at `install`.
    fn with_amp(install: &str) -> Report {
        let mut value = status();
        value["agents"].as_array_mut().unwrap().push(json!(
            {"kind":"amp","path":"/bin/amp",
             "version":"0.0.1791964847-g13f24a (released 2026-10-14T08:00:47.000Z, 3h ago)",
             "hooks":"global","install":{"path":PLUGIN,"status":install,"version":1},
             "skills":[{"target":"claude","path":"/h/.claude/skills/corral/SKILL.md","status":"same"},
                       {"target":"codex","path":"/h/.agents/skills/corral/SKILL.md","status":"same"}]}
        ));
        let (agents, mut files) = parse_status(&value).unwrap();
        files.extend(parse_plan("paddock", &plan_json("same", "same")).unwrap());
        Report {
            agents,
            files,
            ranch: Ranch::Answered,
        }
    }

    #[test]
    fn an_agent_with_a_plugin_brings_where_it_goes_and_how_it_stands() {
        let report = with_amp("create");
        let amp = &report.agents[4];
        assert_eq!(amp.hooks, "global");
        assert_eq!(
            amp.install,
            Some(Install {
                path: PLUGIN.into(),
                status: "create".into(),
                enabled: None,
            })
        );
        assert_eq!(report.agents[0].install, None);
    }

    #[test]
    fn the_plugin_shows_what_can_be_done_with_it() {
        let cases = [
            ("create", Status::Install, None),
            ("same", Status::Installed, None),
            ("overwrite", Status::Update, Some("plugin changed")),
            (
                "foreign",
                Status::NotManaged,
                Some("corral.ts isn't corral's"),
            ),
            ("newer", Status::Other("newer".into()), None),
        ];
        for (install, status, note) in cases {
            let rows = rows(&with_amp(install));
            let amp = row(&rows, "amp");
            assert_eq!(amp.status.as_ref(), Some(&status), "{install}");
            assert_eq!(amp.status_note.as_deref(), note, "{install}");
            assert_eq!(amp.skills, Skills::UpToDate);
        }
    }

    #[test]
    fn a_release_date_that_goes_stale_is_left_off_the_version() {
        let rows = rows(&with_amp("same"));
        assert_eq!(row(&rows, "amp").detail, "0.0.1791964847-g13f24a");
        assert_eq!(row(&rows, "claude").detail, "2.1.299 (Claude Code)");
    }

    #[test]
    fn without_the_program_the_plugin_offers_nothing() {
        let mut report = with_amp("create");
        report.agents[4].path = None;
        let amp = rows(&report).remove(4);
        assert_eq!((amp.status, amp.status_note), (None, None));
        assert_eq!(plugin_plan(&report, "amp", false), None);
    }

    #[test]
    fn the_plugin_plan_installs_updates_or_removes_only_what_the_row_offers() {
        let plan = plugin_plan(&with_amp("create"), "amp", false).unwrap();
        assert_eq!(
            plan,
            PluginPlan {
                kind: "amp".into(),
                name: "amp".into(),
                path: PLUGIN.into(),
                status: "create".into(),
                remove: false,
            }
        );
        assert_eq!(
            plugin_plan(&with_amp("overwrite"), "amp", false).map(|p| p.status),
            Some("overwrite".into())
        );
        let remove = plugin_plan(&with_amp("same"), "amp", true).unwrap();
        assert!(remove.remove);
        assert_eq!(plugin_plan(&with_amp("same"), "amp", false), None);
        assert_eq!(plugin_plan(&with_amp("create"), "amp", true), None);
        assert_eq!(plugin_plan(&with_amp("foreign"), "amp", false), None);
        assert_eq!(plugin_plan(&with_amp("foreign"), "amp", true), None);
        assert_eq!(plugin_plan(&with_amp("create"), "claude", false), None);
    }

    /// A corral that answers `integration … --dry-run` with `seen`, logs every call, and reports
    /// `written` for `--yes`.
    fn plugin_corral(
        dir: &std::path::Path,
        seen: &str,
        written: bool,
    ) -> (String, std::path::PathBuf) {
        let log = dir.join("log");
        let corral = script(
            dir,
            "corral",
            &format!(
                "#!/bin/sh\necho \"$*\" >> '{log}'\ncase \"$*\" in\n*--dry-run*) echo '{{\"ok\":true,\"status\":\"{seen}\",\"written\":false,\"warnings\":[]}}';;\n*) echo '{{\"ok\":true,\"status\":\"{seen}\",\"written\":{written},\"warnings\":[\"left in place\"]}}';;\nesac\n",
                log = log.display()
            ),
        );
        (corral, log)
    }

    #[test]
    fn installing_the_plugin_checks_it_still_stands_as_shown_then_writes() {
        let dir = temp();
        let (corral, log) = plugin_corral(&dir, "create", true);
        let plan = plugin_plan(&with_amp("create"), "amp", false).unwrap();
        plugin(&corral, &plan, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                "integration install --agent amp --dry-run",
                "integration install --agent amp --yes"
            ]
        );
        let dir = temp();
        let (corral, log) = plugin_corral(&dir, "remove", true);
        let plan = plugin_plan(&with_amp("same"), "amp", true).unwrap();
        plugin(&corral, &plan, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&log).unwrap().lines().last(),
            Some("integration uninstall --agent amp --yes")
        );
    }

    #[test]
    fn new_agent_can_ask_how_one_plugin_stands() {
        let dir = temp();
        let mut value = status();
        value["agents"].as_array_mut().unwrap().push(
            json!({"kind":"amp","path":"/bin/amp","hooks":"global","install":{"path":PLUGIN,"status":"create"}}),
        );
        let corral = script(&dir, "corral", &echo(&value));
        let cancel = AtomicBool::new(false);
        assert_eq!(
            plugin_status(&corral, "amp", &cancel)
                .map(|install| install.status)
                .as_deref(),
            Some("create")
        );
        assert_eq!(plugin_status(&corral, "claude", &cancel), None);
        let missing = dir.join("nothing").display().to_string();
        assert_eq!(plugin_status(&missing, "amp", &cancel), None);
    }

    #[test]
    fn a_plugin_that_changed_since_it_was_shown_is_left_alone() {
        let dir = temp();
        let (corral, log) = plugin_corral(&dir, "foreign", true);
        let plan = plugin_plan(&with_amp("overwrite"), "amp", false).unwrap();
        assert_eq!(
            plugin(&corral, &plan, &AtomicBool::new(false)).unwrap_err(),
            CHANGED
        );
        assert_eq!(std::fs::read_to_string(&log).unwrap().lines().count(), 1);
    }

    const AGY_PLUGIN: &str = "/h/.gemini/config/plugins/corral";

    /// The status with amp and agy, agy's plugin standing at `install` with `enabled`.
    fn with_agy(install: &str, enabled: Value) -> Report {
        let mut report = with_amp("same");
        let (agents, files) = parse_status(&json!({"ok":true,"agents":[
            {"kind":"agy","path":"/bin/agy","version":"1.3.5","hooks":"global",
             "install":{"path":AGY_PLUGIN,"status":install,"version":1,"enabled":enabled},
             "skills":[{"target":"gemini","path":"/h/.gemini/config/skills/corral/SKILL.md","status":"create"}]}
        ]}))
        .unwrap();
        report.agents.extend(agents);
        report.files.extend(files);
        report
    }

    #[test]
    fn agys_plugin_brings_whether_agy_has_it_turned_on() {
        let report = with_agy("same", json!(false));
        let agy = &report.agents[5];
        assert_eq!(
            agy.install,
            Some(Install {
                path: AGY_PLUGIN.into(),
                status: "same".into(),
                enabled: Some(false),
            })
        );
        assert_eq!(agy.targets, ["gemini"]);
        // Only agy says; amp's has no word, which is not "off".
        assert_eq!(report.agents[4].install.as_ref().unwrap().enabled, None);
        let unknown = with_agy("same", Value::Null);
        assert_eq!(unknown.agents[5].install.as_ref().unwrap().enabled, None);
    }

    #[test]
    fn agys_plugin_turned_off_in_agy_is_disabled_and_can_still_be_removed() {
        let shown = rows(&with_agy("same", json!(false)));
        let agy = row(&shown, "agy");
        assert_eq!(agy.status, Some(Status::Disabled));
        assert_eq!(
            agy.status_note.as_deref(),
            Some("Run agy plugin enable corral")
        );
        assert!(plugin_plan(&with_agy("same", json!(false)), "agy", true).is_some());
        for enabled in [json!(true), Value::Null] {
            let rows = rows(&with_agy("same", enabled));
            assert_eq!(row(&rows, "agy").status, Some(Status::Installed));
        }
        let cases = [
            ("create", Status::Install, None),
            ("overwrite", Status::Update, Some("plugin changed")),
            ("foreign", Status::NotManaged, Some("corral isn't corral's")),
        ];
        for (install, status, note) in cases {
            let rows = rows(&with_agy(install, json!(true)));
            let agy = row(&rows, "agy");
            assert_eq!(agy.status.as_ref(), Some(&status), "{install}");
            assert_eq!(agy.status_note.as_deref(), note, "{install}");
        }
        // The gemini place is drawn from the status like the others.
        assert_eq!(row(&shown, "agy").skills, Skills::Act { install: true });
        assert_eq!(row(&shown, "amp").status, Some(Status::Installed));
    }

    #[test]
    fn removing_agys_plugin_passes_on_what_corral_left_behind() {
        let dir = temp();
        let (corral, log) = plugin_corral(&dir, "remove", true);
        let plan = plugin_plan(&with_agy("same", json!(true)), "agy", true).unwrap();
        assert_eq!(
            plugin(&corral, &plan, &AtomicBool::new(false)).unwrap(),
            ["left in place"]
        );
        assert_eq!(
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                "integration uninstall --agent agy --dry-run",
                "integration uninstall --agent agy --yes"
            ]
        );
    }

    #[test]
    fn new_agent_hears_whether_agys_plugin_is_turned_on() {
        let dir = temp();
        let mut value = status();
        value["agents"].as_array_mut().unwrap().push(json!(
            {"kind":"agy","path":"/bin/agy","hooks":"global",
             "install":{"path":AGY_PLUGIN,"status":"same","enabled":false}}
        ));
        let corral = script(&dir, "corral", &echo(&value));
        assert_eq!(
            plugin_status(&corral, "agy", &AtomicBool::new(false)),
            Some(Install {
                path: AGY_PLUGIN.into(),
                status: "same".into(),
                enabled: Some(false),
            })
        );
    }

    #[test]
    fn nothing_written_is_an_error_with_corrals_reason() {
        let dir = temp();
        let (corral, _) = plugin_corral(&dir, "create", false);
        let plan = plugin_plan(&with_amp("create"), "amp", false).unwrap();
        assert_eq!(
            plugin(&corral, &plan, &AtomicBool::new(false)).unwrap_err(),
            "nothing was written: left in place"
        );
    }
}
