//! ranch's dispatch table (DESIGN §13 P5-161): which agents each kind of work goes to, with what
//! model and effort, read and changed through ranch's public commands only, as ranch's
//! `docs/CONTRACT.md` sets them out: `ranch dispatch table` (R31), `edit` and `reset` (R35). Each
//! prints one line of JSON: `ok:true` and exit 0, or `ok:false` with `error` and `message` and
//! exit 1 (3 for a conflict, 2 for usage). Unknown fields are left alone. Nothing here reads or
//! writes ranch's files: ranch checks every change, writes it and answers with the whole table.
//!
//! The cells, the agents and their order come from the data; none is named here. ranch is looked
//! for on PATH, as fence is. Every call blocks, so call them off the UI thread.
use serde::{Deserialize, Deserializer, de};
use serde_json::Value;
use std::{fmt, marker::PhantomData, sync::atomic::AtomicBool, time::Duration};

/// The command, found on PATH.
pub const PROGRAM: &str = "ranch";
/// How long one call may take: reading and writing a small file.
const TIMEOUT: Duration = Duration::from_secs(15);

/// This Mac's own table file.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Local {
    pub path: String,
    pub exists: bool,
    /// What `edit` and `reset` check against (`missing`, or `sha256:…`), kept as given; none from
    /// a ranch too old to change the table (before R35).
    pub version: Option<String>,
}

/// An agent's template, before a cell fills it in.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Agent {
    /// `builtin` or `user`; another value is shown as it is.
    pub source: String,
    pub argv: Vec<String>,
    pub labels: Vec<String>,
    /// The efforts it takes; empty when it takes none (or ranch is older than R35).
    pub efforts: Vec<String>,
    /// The models it suggests; any other may be typed.
    pub models: Vec<String>,
}

/// How a cell starts one of its agents.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Choice {
    pub agent: String,
    pub source: String,
    pub argv: Vec<String>,
    /// The same as one shell line.
    pub command: String,
    pub labels: Vec<String>,
}

/// One kind of work and where it goes.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Cell {
    pub name: String,
    /// `builtin`, or `user` when this Mac's file mentions it.
    pub source: String,
    /// ranch's words for it, shown as they are; empty before R35.
    pub description: String,
    /// The agents it may go to, in order; may be empty.
    pub agents: Vec<String>,
    /// None for each agent's own.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// The cell that reviews its work, and the one rework moves to.
    pub cross_review: Option<String>,
    pub rework: Option<String>,
    pub choices: Vec<Choice>,
}

impl Cell {
    /// Changed on this Mac.
    pub fn changed(&self) -> bool {
        self.source == "user"
    }

    /// Its model and effort as a small tag: `opus[1m] · high`, one of them, or `default`.
    pub fn tag(&self) -> String {
        let parts: Vec<&str> = [&self.model, &self.effort]
            .into_iter()
            .filter_map(|part| part.as_deref().filter(|part| !part.is_empty()))
            .collect();
        if parts.is_empty() {
            "default".into()
        } else {
            parts.join(" · ")
        }
    }
}

/// `ranch dispatch table`: the agents' templates and the cells as they take effect, each in
/// ranch's order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub local: Local,
    pub agents: Vec<(String, Agent)>,
    pub cells: Vec<Cell>,
}

impl Table {
    /// ranch can change it from here: its table carries a version (R35).
    pub fn editable(&self) -> bool {
        self.local.version.is_some()
    }

    /// How many cells this Mac's file changes.
    pub fn changed(&self) -> usize {
        self.cells.iter().filter(|cell| cell.changed()).count()
    }

    pub fn cell(&self, name: &str) -> Option<&Cell> {
        self.cells.iter().find(|cell| cell.name == name)
    }

    pub fn agent(&self, name: &str) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, agent)| agent)
    }

    /// The agents for the graph's middle column, ordered so few lines cross: by the mean place of
    /// the cells that go to each, ties and unused ones in ranch's order; an agent a cell names
    /// that the table doesn't list comes too.
    pub fn agent_order(&self) -> Vec<String> {
        let mut names: Vec<String> = self.agents.iter().map(|(name, _)| name.clone()).collect();
        for cell in &self.cells {
            for agent in &cell.agents {
                if !names.contains(agent) {
                    names.push(agent.clone());
                }
            }
        }
        let place = |name: &String| {
            let at: Vec<usize> = (self.cells.iter().enumerate())
                .filter(|(_, cell)| cell.agents.contains(name))
                .map(|(index, _)| index)
                .collect();
            if at.is_empty() {
                f32::INFINITY
            } else {
                at.iter().sum::<usize>() as f32 / at.len() as f32
            }
        };
        let mut placed: Vec<(f32, usize, String)> = names
            .into_iter()
            .enumerate()
            .map(|(index, name)| (place(&name), index, name))
            .collect();
        placed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        placed.into_iter().map(|(_, _, name)| name).collect()
    }

    /// The efforts every one of `agents` takes, in the first one's order; none for no agent.
    pub fn efforts(&self, agents: &[String]) -> Vec<String> {
        let takes = |name: &String| {
            self.agent(name)
                .map(|a| a.efforts.as_slice())
                .unwrap_or(&[])
        };
        let Some(first) = agents.first() else {
            return Vec::new();
        };
        takes(first)
            .iter()
            .filter(|effort| agents.iter().all(|name| takes(name).contains(effort)))
            .cloned()
            .collect()
    }

    /// The models `agents` suggest, each once, in their order.
    pub fn models(&self, agents: &[String]) -> Vec<String> {
        let mut models: Vec<String> = Vec::new();
        for agent in agents.iter().filter_map(|name| self.agent(name)) {
            for model in &agent.models {
                if !models.contains(model) {
                    models.push(model.clone());
                }
            }
        }
        models
    }
}

impl<'de> Deserialize<'de> for Table {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            local: Local,
            #[serde(default, deserialize_with = "ordered")]
            agents: Vec<(String, Agent)>,
            #[serde(default, deserialize_with = "ordered")]
            cells: Vec<(String, Cell)>,
        }
        let raw = Raw::deserialize(deserializer)?;
        Ok(Table {
            local: raw.local,
            agents: raw.agents,
            cells: raw
                .cells
                .into_iter()
                .map(|(name, mut cell)| {
                    cell.name = name;
                    cell
                })
                .collect(),
        })
    }
}

/// A JSON object as its entries in the order written (a map would sort them).
fn ordered<'de, D, T>(deserializer: D) -> Result<Vec<(String, T)>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Entries<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Entries<T> {
        type Value = Vec<(String, T)>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            while let Some(entry) = map.next_entry()? {
                entries.push(entry);
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(Entries(PhantomData))
}

/// What is being changed in one cell, before it is saved: only the fields touched, so a table
/// read again shows what changed elsewhere in the rest. An empty string clears a field (each
/// agent's own model or effort; no cell).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub agents: Option<Vec<String>>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub cross_review: Option<String>,
    pub rework: Option<String>,
}

impl Draft {
    /// The agents shown: as drafted, else the cell's.
    pub fn agents<'a>(&'a self, cell: &'a Cell) -> &'a [String] {
        self.agents.as_deref().unwrap_or(&cell.agents)
    }

    pub fn model<'a>(&'a self, cell: &'a Cell) -> &'a str {
        shown(&self.model, &cell.model)
    }

    pub fn effort<'a>(&'a self, cell: &'a Cell) -> &'a str {
        shown(&self.effort, &cell.effort)
    }

    pub fn cross_review<'a>(&'a self, cell: &'a Cell) -> &'a str {
        shown(&self.cross_review, &cell.cross_review)
    }

    pub fn rework<'a>(&'a self, cell: &'a Cell) -> &'a str {
        shown(&self.rework, &cell.rework)
    }

    /// The options `ranch dispatch edit` takes for what differs from `cell`, in its order;
    /// none when nothing does.
    pub fn options(&self, cell: &Cell) -> Vec<(&'static str, String)> {
        let mut options = Vec::new();
        if let Some(agents) = &self.agents
            && *agents != cell.agents
        {
            let json = serde_json::to_string(agents).expect("strings write as JSON");
            options.push(("--agents", json));
        }
        let fields = [
            ("--model", &self.model, &cell.model),
            ("--effort", &self.effort, &cell.effort),
            ("--cross-review", &self.cross_review, &cell.cross_review),
            ("--rework", &self.rework, &cell.rework),
        ];
        for (flag, drafted, current) in fields {
            if let Some(value) = drafted
                && value.as_str() != current.as_deref().unwrap_or_default()
            {
                options.push((flag, value.clone()));
            }
        }
        options
    }
}

/// The drafted value, else the cell's; empty for none.
fn shown<'a>(drafted: &'a Option<String>, current: &'a Option<String>) -> &'a str {
    drafted
        .as_deref()
        .or(current.as_deref())
        .unwrap_or_default()
}

/// `ranch dispatch edit` for `cell` as drafted, checked against `version`; none when the draft
/// changes nothing.
pub fn edit_args(cell: &Cell, draft: &Draft, version: &str) -> Option<Vec<String>> {
    let options = draft.options(cell);
    if options.is_empty() {
        return None;
    }
    let mut args = vec!["dispatch".to_owned(), "edit".to_owned(), cell.name.clone()];
    for (flag, value) in options {
        args.extend([flag.to_owned(), value]);
    }
    args.extend(["--if-version".to_owned(), version.to_owned()]);
    Some(args)
}

/// `ranch dispatch reset` for the cell `name`, checked against `version`.
pub fn reset_args(name: &str, version: &str) -> Vec<String> {
    ["dispatch", "reset", name, "--if-version", version]
        .map(str::to_owned)
        .to_vec()
}

/// What a refusal asks of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The file changed since it was read: nothing was saved; read it again.
    Conflict,
    /// ranch can't change the file without rewriting its layout: it is to be edited by hand.
    FormatNotPreserved,
    /// Anything else ranch refused, or couldn't do: nothing was saved.
    Refused,
}

/// Why a call did not answer: ranch's `error` and `message`, and its exit code; an empty code
/// when ranch could not be run or its answer could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refusal {
    pub code: String,
    pub message: String,
    pub exit: Option<i32>,
}

impl Refusal {
    pub fn problem(&self) -> Problem {
        match self.code.as_str() {
            "conflict" => Problem::Conflict,
            "format_not_preserved" => Problem::FormatNotPreserved,
            _ => Problem::Refused,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.code.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

/// Runs ranch with `args` and reads the table it answers with.
fn call(program: &str, args: &[String], cancel: &AtomicBool) -> Result<Table, Refusal> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output =
        crate::command::run(program, &args, None, TIMEOUT, cancel).map_err(|error| Refusal {
            message: format!("ranch did not run: {error:#}"),
            ..Refusal::default()
        })?;
    let exit = output.status.code();
    let unreadable = || Refusal {
        message: format!(
            "ranch's answer could not be read{}",
            match String::from_utf8_lossy(&output.stderr).trim() {
                "" => String::new(),
                error => format!(": {error}"),
            }
        ),
        exit,
        ..Refusal::default()
    };
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|_| unreadable())?;
    if output.status.success() && value.get("ok") == Some(&Value::Bool(true)) {
        // From the text, not the value: a value may sort the cells.
        return serde_json::from_slice(&output.stdout).map_err(|error| Refusal {
            message: format!("ranch's answer could not be read: {error}"),
            exit,
            ..Refusal::default()
        });
    }
    let text = |key| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match (text("error"), text("message")) {
        (None, None) => Err(unreadable()),
        (code, message) => Err(Refusal {
            message: message.or_else(|| code.clone()).unwrap_or_default(),
            code: code.unwrap_or_default(),
            exit,
        }),
    }
}

/// The table as it takes effect now.
pub fn table(program: &str, cancel: &AtomicBool) -> Result<Table, Refusal> {
    call(program, &["dispatch".into(), "table".into()], cancel)
}

/// Runs `edit_args` or `reset_args`; the table as written.
pub fn change(program: &str, args: &[String], cancel: &AtomicBool) -> Result<Table, Refusal> {
    call(program, args, cancel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};

    /// Never cancelled.
    static NO: AtomicBool = AtomicBool::new(false);

    /// A fake `ranch` in a fresh directory: it writes each argument on a line of `args` beside
    /// itself, then runs `script`.
    fn fake(name: &str, script: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!(
            "paddock-ranch-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ranch");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done > \"$(dirname \"$0\")/args\"\n{script}\n"
            ),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let program = path.display().to_string();
        (dir, program)
    }

    /// The arguments the fake was last run with.
    fn args(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_to_string(dir.join("args"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// A table as R35 writes it, cells deliberately out of name order; one cell changed on this
    /// Mac, one with no agent, an unknown field.
    const TABLE: &str = r#"{"agents":{"codex":{"argv":["codex","-m","${model}"],"efforts":["medium","high","xhigh"],"labels":["model=${model}"],"models":["gpt-6-astra"],"source":"builtin"},"claude":{"argv":["claude","--model","${model}"],"efforts":["medium","high","xhigh"],"labels":[],"models":["opus[1m]","sonnet"],"source":"builtin"},"pi":{"argv":["pi"],"efforts":[],"labels":[],"models":[],"source":"user"},"omp":{"argv":["omp"],"efforts":[],"labels":[],"models":[],"source":"builtin"}},"cells":{"review":{"agents":["codex"],"choices":[{"agent":"codex","argv":["codex","-m","gpt-6-astra"],"command":"codex -m gpt-6-astra","labels":["model=gpt-6-astra"],"source":"builtin"}],"cross_review":null,"description":"交叉审查","effort":"xhigh","model":"gpt-6-astra","name":"review","review":null,"rework":null,"source":"builtin"},"design":{"agents":["claude"],"choices":[],"cross_review":"review","description":"出方案","effort":"high","model":"opus[1m]","name":"design","review":{"name":"review"},"rework":null,"source":"user"},"chores":{"agents":["pi","omp"],"choices":[],"cross_review":null,"description":"杂活","effort":null,"model":null,"name":"chores","review":null,"rework":"design","source":"builtin"},"empty":{"agents":[],"choices":[],"cross_review":null,"description":"","effort":"high","model":null,"name":"empty","review":null,"rework":null,"source":"user","new_field":1}},"local":{"exists":true,"path":"/Users/u/.ranch/dispatch.toml","version":"sha256:abc"},"ok":true}"#;

    fn parsed() -> Table {
        serde_json::from_str(TABLE).unwrap()
    }

    #[test]
    fn the_table_is_read_in_ranchs_order_with_descriptions_options_and_version() {
        let (_dir, program) = fake("table", &format!("echo '{TABLE}'"));
        let table = table(&program, &NO).unwrap();
        let names: Vec<&str> = table.cells.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["review", "design", "chores", "empty"]);
        let agents: Vec<&str> = table.agents.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(agents, ["codex", "claude", "pi", "omp"]);
        assert_eq!(table.local.version.as_deref(), Some("sha256:abc"));
        assert_eq!(table.local.path, "/Users/u/.ranch/dispatch.toml");
        assert!(table.local.exists && table.editable());
        assert_eq!(table.changed(), 2);
        let design = table.cell("design").unwrap();
        assert_eq!(design.description, "出方案");
        assert_eq!(design.cross_review.as_deref(), Some("review"));
        assert!(design.changed());
        assert_eq!(design.tag(), "opus[1m] · high");
        assert_eq!(table.cell("chores").unwrap().tag(), "default");
        assert_eq!(table.cell("empty").unwrap().tag(), "high");
        assert_eq!(
            table.cell("review").unwrap().choices[0].command,
            "codex -m gpt-6-astra"
        );
        let claude = table.agent("claude").unwrap();
        assert_eq!(claude.efforts, ["medium", "high", "xhigh"]);
        assert_eq!(claude.models, ["opus[1m]", "sonnet"]);
        assert_eq!(table.agent("pi").unwrap().source, "user");
        assert_eq!(args(&_dir), ["dispatch", "table"]);
    }

    #[test]
    fn a_ranch_before_r35_reads_without_descriptions_options_or_version_and_cant_edit() {
        let old = r#"{"agents":{"codex":{"argv":["codex"],"labels":[],"source":"builtin"}},"cells":{"implementation":{"agents":["codex"],"choices":[],"cross_review":null,"effort":"medium","model":"gpt-6-astra","name":"implementation","review":null,"rework":null,"source":"builtin"}},"local":{"exists":false,"path":"/Users/u/.ranch/dispatch.toml"},"ok":true}"#;
        let table: Table = serde_json::from_str(old).unwrap();
        assert!(!table.editable());
        assert_eq!(table.local.version, None);
        let cell = table.cell("implementation").unwrap();
        assert_eq!(cell.description, "");
        let codex = table.agent("codex").unwrap();
        assert!(codex.efforts.is_empty() && codex.models.is_empty());
        // R35 with no file yet: `missing` is a version like any other.
        let missing: Table = serde_json::from_str(
            r#"{"agents":{},"cells":{},"local":{"exists":false,"path":"/x","version":"missing"},"ok":true}"#,
        )
        .unwrap();
        assert!(missing.editable());
        assert_eq!(missing.changed(), 0);
    }

    #[test]
    fn efforts_are_those_every_agent_takes_and_models_those_any_suggests() {
        let table = parsed();
        let both = ["codex".to_owned(), "claude".to_owned()];
        assert_eq!(table.efforts(&both), ["medium", "high", "xhigh"]);
        assert_eq!(table.models(&both), ["gpt-6-astra", "opus[1m]", "sonnet"]);
        let mixed = ["claude".to_owned(), "pi".to_owned()];
        assert!(table.efforts(&mixed).is_empty());
        assert_eq!(table.models(&mixed), ["opus[1m]", "sonnet"]);
        assert!(table.efforts(&[]).is_empty() && table.models(&[]).is_empty());
        // An agent the table doesn't list takes nothing.
        assert!(table.efforts(&["ghost".to_owned()]).is_empty());
    }

    #[test]
    fn agents_are_ordered_by_the_cells_that_go_to_them() {
        let table = parsed();
        // review → codex (0); design → claude (1); chores → pi, omp (2); empty → none.
        assert_eq!(table.agent_order(), ["codex", "claude", "pi", "omp"]);
        let mut table = table;
        // codex now used last: it moves below the others; an unlisted agent a cell names comes too.
        table.cells[0].agents = vec!["ghost".into()];
        table.cells[3].agents = vec!["codex".into()];
        assert_eq!(
            table.agent_order(),
            ["ghost", "claude", "pi", "omp", "codex"]
        );
        // An agent no cell uses goes last.
        table.cells[2].agents = vec!["pi".into()];
        assert_eq!(
            table.agent_order(),
            ["ghost", "claude", "pi", "codex", "omp"]
        );
    }

    #[test]
    fn an_edit_carries_only_what_changed_with_cleared_as_empty_and_the_version() {
        let table = parsed();
        let design = table.cell("design").unwrap();
        // Nothing touched, or touched back to what it is: nothing to save.
        assert_eq!(edit_args(design, &Draft::default(), "sha256:abc"), None);
        let same = Draft {
            model: Some("opus[1m]".into()),
            agents: Some(vec!["claude".into()]),
            ..Draft::default()
        };
        assert_eq!(edit_args(design, &same, "sha256:abc"), None);
        let draft = Draft {
            agents: Some(vec!["pi".into(), "claude".into()]),
            model: Some(String::new()),
            effort: Some("high".into()),
            cross_review: Some(String::new()),
            rework: Some("chores".into()),
        };
        assert_eq!(draft.model(design), "");
        assert_eq!(draft.effort(design), "high");
        assert_eq!(
            edit_args(design, &draft, "sha256:abc").unwrap(),
            [
                "dispatch",
                "edit",
                "design",
                "--agents",
                r#"["pi","claude"]"#,
                "--model",
                "",
                "--cross-review",
                "",
                "--rework",
                "chores",
                "--if-version",
                "sha256:abc",
            ]
        );
        // Every agent taken off.
        let none = Draft {
            agents: Some(Vec::new()),
            ..Draft::default()
        };
        assert_eq!(
            edit_args(design, &none, "missing").unwrap(),
            [
                "dispatch",
                "edit",
                "design",
                "--agents",
                "[]",
                "--if-version",
                "missing"
            ]
        );
        // A name needing JSON escapes is written as JSON.
        let odd = Draft {
            agents: Some(vec![r#"a"b"#.into()]),
            ..Draft::default()
        };
        assert_eq!(edit_args(design, &odd, "v").unwrap()[4], r#"["a\"b"]"#);
        assert_eq!(
            reset_args("design", "sha256:abc"),
            ["dispatch", "reset", "design", "--if-version", "sha256:abc"]
        );
    }

    #[test]
    fn a_change_is_run_as_built_and_answers_with_the_table_as_written() {
        let (dir, program) = fake("edit", &format!("echo '{TABLE}'"));
        let table = parsed();
        let draft = Draft {
            model: Some("sonnet".into()),
            ..Draft::default()
        };
        let args_built = edit_args(table.cell("design").unwrap(), &draft, "sha256:abc").unwrap();
        let written = change(&program, &args_built, &NO).unwrap();
        assert_eq!(written, table);
        assert_eq!(args(&dir), args_built);
    }

    #[test]
    fn a_conflict_a_layout_ranch_cant_keep_and_other_refusals_are_told_apart() {
        let script = r#"case "$3" in
  design) echo '{"actual_version":"sha256:new","error":"conflict","expected_version":"sha256:abc","message":"changed elsewhere","ok":false}'; exit 3;;
  chores) echo '{"error":"format_not_preserved","message":"dotted keys","ok":false}'; exit 1;;
  review) echo '{"error":"invalid_data","message":"/x: cells.review.effort: not taken by pi","ok":false}'; exit 1;;
  ghost) echo '{"error":"unknown_cell","message":"no cell ghost","ok":false}'; exit 1;;
  io) echo '{"error":"io_error","message":"disk full","ok":false}'; exit 1;;
  *) echo 'usage: ranch dispatch edit' >&2; exit 2;;
esac"#;
        let (_dir, program) = fake("refusals", script);
        let run = |cell: &str| change(&program, &reset_args(cell, "sha256:abc"), &NO).unwrap_err();
        let conflict = run("design");
        assert_eq!(conflict.problem(), Problem::Conflict);
        assert_eq!(conflict.exit, Some(3));
        assert_eq!(conflict.message, "changed elsewhere");
        let format = run("chores");
        assert_eq!(format.problem(), Problem::FormatNotPreserved);
        assert_eq!(format.message, "dotted keys");
        let invalid = run("review");
        assert_eq!(invalid.problem(), Problem::Refused);
        assert_eq!(invalid.code, "invalid_data");
        assert_eq!(
            invalid.to_string(),
            "invalid_data: /x: cells.review.effort: not taken by pi"
        );
        assert_eq!(run("ghost").code, "unknown_cell");
        assert_eq!(run("io").code, "io_error");
        // Not ranch's JSON: what it said on stderr.
        let usage = run("other");
        assert_eq!(
            (usage.problem(), usage.code.as_str()),
            (Problem::Refused, "")
        );
        assert!(
            usage.message.contains("usage: ranch dispatch edit"),
            "{usage}"
        );
        assert_eq!(usage.exit, Some(2));
        // Not there at all.
        let missing = table("/nonexistent/ranch", &NO).unwrap_err();
        assert_eq!(missing.problem(), Problem::Refused);
        assert!(
            missing.message.starts_with("ranch did not run"),
            "{missing}"
        );
    }
}
