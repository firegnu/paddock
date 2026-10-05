//! What the Agents panel shows for each agent, worked out apart from drawing so it can be tested.
//! Ages follow Saddle's Agents panel in `src/ui.rs` at commit `df1c727`; the rest is paddock's own
//! two-line card (DESIGN §13): a status dot, name, program and age, then one line saying where the
//! agent works or what it needs, with the internal fields kept for the expanded details.
use crate::{
    agents::{Panel, Status, group},
    corral::Agent,
    git::{Head, Summary},
    preset::{Color, Theme},
};

/// A Saddle interface colour, as `Theme::fg/bg` take it.
pub type Pick = fn(&Theme) -> Color;

/// The status dot's colour, whether it breathes, and the status in words for the details.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub color: Pick,
    pub breathing: bool,
    pub label: &'static str,
}

/// Only a working agent's dot breathes.
pub fn look(status: Status) -> Look {
    let (label, color): (_, Pick) = match status {
        Status::Waiting => ("Waiting", |t| t.agents_yellow),
        Status::Error => ("Error", |t| t.agents_red),
        Status::Stalled => ("Stalled", |t| t.agent_stalled),
        Status::Working => ("Working", |t| t.agents_blue),
        Status::Starting => ("Starting", |t| t.agent_starting),
        Status::Unknown => ("Unknown", |t| t.agents_dim),
        Status::Idle => ("Idle", |t| t.agents_green),
        Status::Exited => ("Exited", |t| t.agents_red),
    };
    Look {
        color,
        breathing: status == Status::Working,
        label,
    }
}

/// Which agent program runs it, in the program's colour.
#[derive(Clone, Debug)]
pub struct Brand {
    pub kind: String,
    pub color: Pick,
}

pub fn brand(kind: &str) -> Brand {
    let color: Pick = match kind.to_ascii_lowercase().as_str() {
        "claude" => |t| t.claude,
        "codex" => |t| t.codex,
        "pi" => |t| t.pi,
        "omp" => |t| t.omp,
        _ => |t| t.agents_dim,
    };
    Brand {
        kind: kind.to_owned(),
        color,
    }
}

/// Ages: at most four characters, as in Saddle's time column.
pub fn short_time(value: Option<f64>) -> String {
    match value {
        Some(s) if s >= 1000.0 * 86400.0 => format!("{}y", (s / (365.0 * 86400.0)) as u64),
        Some(s) if s >= 100.0 * 3600.0 => format!("{}d", (s / 86400.0) as u64),
        Some(s) if s >= 36000.0 => format!("{}h", (s / 3600.0) as u64),
        Some(s) if s >= 3600.0 => format!("{:.1}h", s / 3600.0),
        Some(s) if s >= 60.0 => format!("{}m", (s / 60.0) as u64),
        Some(s) => format!("{}s", s.max(0.0) as u64),
        None => "—".into(),
    }
}

/// How long ago, in words, for the details.
fn ago(seconds: f64) -> String {
    let s = seconds.max(0.0);
    if s < 60.0 {
        "just now".into()
    } else if s < 3600.0 {
        format!("{} min ago", (s / 60.0) as u64)
    } else if s < 86400.0 {
        format!("{} hr ago", (s / 3600.0) as u64)
    } else {
        match (s / 86400.0) as u64 {
            1 => "1 day ago".into(),
            n => format!("{n} days ago"),
        }
    }
}

/// How the second line is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Where the agent works, or that it is starting: dim.
    Quiet,
    /// It waits for a person: amber.
    Waiting,
    /// An error or an exit: red.
    Problem,
}

/// The card's second line.
#[derive(Clone, Debug, PartialEq)]
pub struct Second {
    pub text: String,
    pub tone: Tone,
}

/// Starting, waiting, errors and exits say so; otherwise the directory and branch.
fn second(
    a: &Agent,
    status: Status,
    title: Option<&str>,
    place: impl FnOnce() -> String,
) -> Second {
    let (text, tone) = match status {
        Status::Starting => ("Starting…".to_owned(), Tone::Quiet),
        Status::Waiting => match title {
            Some(title) => (format!("Waiting for you: {title}"), Tone::Waiting),
            None => ("Waiting for you".to_owned(), Tone::Waiting),
        },
        Status::Error => {
            let text = match (&a.error, a.incompatible) {
                (Some(error), _) => format!("Error: {error}"),
                (None, true) => format!("Incompatible protocol ({})", a.proto.unwrap_or(0)),
                (None, false) => "Error".to_owned(),
            };
            (text, Tone::Problem)
        }
        Status::Exited => ("Exited".to_owned(), Tone::Problem),
        _ => (place(), Tone::Quiet),
    };
    Second { text, tone }
}

/// `path` with the home directory written `~`.
pub fn tilde(path: &str, home: Option<&str>) -> String {
    let Some(home) = home
        .map(|h| h.trim_end_matches('/'))
        .filter(|h| !h.is_empty())
    else {
        return path.to_owned();
    };
    match path.strip_prefix(home) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_owned(),
    }
}

/// The directory, then the branch after a `·`. A directory too long for the line loses its middle
/// levels, down to its start and last level (`~/…/paddock`); what still does not fit is cut at the
/// end when drawn.
pub fn place(
    cwd: &str,
    branch: Option<&str>,
    home: Option<&str>,
    fits: &dyn Fn(&str) -> bool,
) -> String {
    let path = tilde(cwd, home);
    let line = |dir: &str| match branch {
        Some(branch) => format!("{dir} · {branch}"),
        None => dir.to_owned(),
    };
    let (head, rest) = match path.strip_prefix('/') {
        Some(rest) => ("", rest),
        None => path.split_once('/').unwrap_or((path.as_str(), "")),
    };
    let levels: Vec<&str> = rest.split('/').filter(|l| !l.is_empty()).collect();
    let full = line(&path);
    if levels.len() < 2 || fits(&full) {
        return full;
    }
    let mut candidate = full;
    for start in 1..levels.len() {
        candidate = line(&format!("{head}/…/{}", levels[start..].join("/")));
        if fits(&candidate) {
            break;
        }
    }
    candidate
}

/// `text` as it fits: whole, or cut at the end with `…`, down to the `…` alone.
pub fn elide(text: &str, fits: &dyn Fn(&str) -> bool) -> String {
    if fits(text) {
        return text.to_owned();
    }
    let mut kept: Vec<char> = text.chars().collect();
    while kept.pop().is_some() {
        let cut = format!("{}…", kept.iter().collect::<String>());
        if fits(&cut) {
            return cut;
        }
    }
    "…".to_owned()
}

/// The branch as the second line names it; `None` while Git is unread or unavailable.
fn branch(git: Option<&Option<Summary>>) -> Option<String> {
    match &git?.as_ref()?.head {
        Head::Branch(branch) => Some(branch.clone()),
        Head::Detached => Some("detached HEAD".into()),
        Head::Unknown => None,
    }
}

/// The branch with its uncommitted changes and the commits it is ahead of its base.
fn branch_details(s: &Summary) -> String {
    let mut parts = vec![match &s.head {
        Head::Branch(branch) => branch.clone(),
        Head::Detached => "detached HEAD".into(),
        Head::Unknown => "—".into(),
    }];
    let mut clean = true;
    if let Some(c) = &s.changes
        && c.added + c.deleted + c.binary > 0
    {
        clean = false;
        let mut text = format!("+{} -{}", c.added, c.deleted);
        if c.binary > 0 {
            text += &format!(", {} binary", c.binary);
        }
        parts.push(text);
    }
    if let Some(untracked) = s.untracked
        && untracked > 0
    {
        clean = false;
        parts.push(format!("{untracked} untracked"));
    }
    if clean && s.changes.is_some() && s.untracked.is_some() {
        parts.push("clean".into());
    }
    if let Some((ahead, _)) = &s.ahead
        && *ahead > 0
    {
        parts.push(format!("{ahead} ahead"));
    }
    parts.join(" · ")
}

/// One row of the expanded details.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub label: &'static str,
    pub value: String,
    /// Drawn in the terminal font: the instance id.
    pub mono: bool,
}

fn details(
    a: &Agent,
    look: &Look,
    status: Status,
    git: Option<&Option<Summary>>,
    home: Option<&str>,
    now: f64,
) -> Vec<Detail> {
    let row = |label, value: String| Detail {
        label,
        value,
        mono: false,
    };
    let mut rows = Vec::new();
    let state = match (status, &a.last_tool) {
        (Status::Working | Status::Stalled, Some(tool)) => format!("{} · {tool}", look.label),
        _ => look.label.to_owned(),
    };
    rows.push(row("Status", state));
    if let Some(cwd) = &a.cwd {
        rows.push(row("Directory", tilde(cwd, home)));
    }
    if let Some(Some(summary)) = git {
        rows.push(row("Branch", branch_details(summary)));
    }
    let label = |key: &str| {
        a.labels
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
    };
    let model: Vec<&str> = [label("model"), label("effort")]
        .into_iter()
        .flatten()
        .collect();
    if !model.is_empty() {
        rows.push(row("Model", model.join(" · ")));
    }
    rows.push(row(
        "Attached",
        match a.attached {
            0 => "no windows".into(),
            1 => "1 window".into(),
            n => format!("{n} windows"),
        },
    ));
    if let Some(source) = &a.last_input_source {
        let who = match source.as_str() {
            "human" => "You",
            "send" => "corral send",
            "agent" => "agent",
            other => other,
        };
        let when = a.last_input_at.map(|at| ago(now - at));
        rows.push(row(
            "Last input",
            match when {
                Some(when) => format!("{who}, {when}"),
                None => who.to_owned(),
            },
        ));
    }
    if let Some(instance) = &a.instance {
        rows.push(Detail {
            label: "Instance",
            value: instance.clone(),
            mono: true,
        });
    }
    rows
}

/// Everything the panel shows for one agent.
#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    /// The name without its group prefix.
    pub short: String,
    pub status: Status,
    pub look: Look,
    /// Shown in the active pane: highlighted, and a click opens its details instead.
    pub selected: bool,
    /// The details show below the second line.
    pub expanded: bool,
    pub brand: Option<Brand>,
    pub time: String,
    pub second: Second,
    pub details: Vec<Detail>,
    pub instance: Option<String>,
    pub cwd: Option<String>,
}

/// What a click on a card does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// Show the agent.
    Open,
    /// Expand or collapse its details: it is already the one shown.
    Details,
}

pub fn click(card: &Card) -> Click {
    if card.selected {
        Click::Details
    } else {
        Click::Open
    }
}

/// A line of the panel.
#[derive(Clone, Debug)]
pub enum Line {
    /// Why the list could not be read.
    Error(String),
    /// A group's name and how many agents it has.
    Group(String, usize),
    Agent(Box<Card>),
}

/// The panel's lines. `selected` is the active pane's agent; `home` the home directory, written
/// `~`; `fits` whether a second line fits the card.
pub fn lines(
    panel: &Panel,
    error: Option<&str>,
    selected: Option<&str>,
    home: Option<&str>,
    fits: &dyn Fn(&str) -> bool,
    now: f64,
) -> Vec<Line> {
    let mut lines: Vec<Line> = error
        .map(|e| Line::Error(e.to_owned()))
        .into_iter()
        .collect();
    let ordered = panel.ordered(now);
    let mut previous = None;
    for (index, a) in ordered.iter().enumerate() {
        let prefix = group(&a.name);
        if previous != Some(prefix) {
            let count = ordered[index..]
                .iter()
                .take_while(|agent| group(&agent.name) == prefix)
                .count();
            let title = if prefix.is_empty() { "agents/" } else { prefix };
            lines.push(Line::Group(title.to_owned(), count));
            previous = Some(prefix);
        }
        lines.push(Line::Agent(Box::new(card(
            panel, a, prefix, selected, home, fits, now,
        ))));
    }
    lines
}

fn card(
    panel: &Panel,
    a: &Agent,
    prefix: &str,
    selected: Option<&str>,
    home: Option<&str>,
    fits: &dyn Fn(&str) -> bool,
    now: f64,
) -> Card {
    let status = panel.status(a, now);
    let look = look(status);
    let short = a.name.strip_prefix(prefix).unwrap_or(&a.name).to_owned();
    let origin = match status {
        Status::Working | Status::Stalled if a.state.as_deref() == Some("working") => {
            a.turn_started
        }
        Status::Idle | Status::Waiting => a.state_started,
        _ => None,
    };
    let title = a
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty() && *title != prefix.trim_end_matches('/'));
    let git = a.cwd.as_ref().and_then(|cwd| panel.git.get(cwd));
    let place = || match &a.cwd {
        Some(cwd) => place(cwd, branch(git).as_deref(), home, fits),
        None => "—".into(),
    };
    Card {
        name: a.name.clone(),
        status,
        selected: selected == Some(a.name.as_str()),
        expanded: panel.expanded.contains(&a.name),
        brand: a.kind.as_deref().map(brand),
        time: short_time(origin.map(|v| now - v)),
        second: second(a, status, title, place),
        details: details(a, &look, status, git, home, now),
        look,
        short,
        instance: a.instance.clone(),
        cwd: a.cwd.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Changes;

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            instance: Some("abcdef123".into()),
            cwd: Some(format!("/w/{name}")),
            ..Agent::default()
        }
    }

    fn roomy(_: &str) -> bool {
        true
    }

    fn cards(lines: &[Line]) -> Vec<&Card> {
        lines
            .iter()
            .filter_map(|line| match line {
                Line::Agent(card) => Some(card.as_ref()),
                _ => None,
            })
            .collect()
    }

    fn panel(agents: Vec<Agent>) -> Panel {
        let mut panel = Panel::default();
        panel.absorb(agents, None, 100.0);
        panel
    }

    #[test]
    fn groups_carry_their_counts_and_names_lose_the_prefix() {
        let panel = panel(vec![
            agent("p/a", "idle"),
            agent("p/b", "idle"),
            agent("q/c", "idle"),
            agent("solo", "idle"),
        ]);
        let lines = lines(&panel, None, None, None, &roomy, 100.0);
        let groups: Vec<_> = lines
            .iter()
            .filter_map(|line| match line {
                Line::Group(name, count) => Some((name.as_str(), *count)),
                _ => None,
            })
            .collect();
        assert_eq!(groups, [("agents/", 1), ("p/", 2), ("q/", 1)]);
        let shorts: Vec<_> = cards(&lines).iter().map(|c| c.short.as_str()).collect();
        assert_eq!(shorts, ["solo", "a", "b", "c"]);
    }

    fn summary(added: u64, deleted: u64) -> Option<Summary> {
        Some(Summary {
            head: Head::Branch("p2d-render".into()),
            ahead: Some((2, "main".into())),
            changes: Some(Changes {
                added,
                deleted,
                binary: 0,
            }),
            untracked: Some(3),
        })
    }

    #[test]
    fn the_second_line_is_the_directory_and_branch_or_what_the_agent_needs() {
        let working = Agent {
            cwd: Some("/home/me/code/paddock".into()),
            ..agent("p/work", "working")
        };
        let plain = Agent {
            cwd: Some("/tmp".into()),
            ..agent("p/plain", "idle")
        };
        let waiting = Agent {
            title: Some("Upgrade the dependencies?".into()),
            ..agent("p/ask", "blocked")
        };
        let asking = Agent {
            // A title that only repeats the group says nothing.
            title: Some("p".into()),
            ..agent("p/ask2", "blocked")
        };
        let broken = Agent {
            error: Some("status failed".into()),
            ..agent("p/broken", "idle")
        };
        let starting = Agent {
            starting: true,
            ..agent("p/new", "idle")
        };
        let mut panel = panel(vec![
            working,
            plain,
            waiting,
            asking,
            broken,
            starting,
            agent("p/gone", "exited"),
        ]);
        panel.absorb_git(vec![
            ("/home/me/code/paddock".into(), summary(1, 0)),
            ("/tmp".into(), None),
        ]);
        let lines = lines(&panel, None, None, Some("/home/me"), &roomy, 100.0);
        let seconds: Vec<(String, Second)> = cards(&lines)
            .iter()
            .map(|c| (c.short.clone(), c.second.clone()))
            .collect();
        let line = |text: &str, tone| Second {
            text: text.into(),
            tone,
        };
        assert_eq!(
            seconds,
            [
                (
                    "ask".into(),
                    line("Waiting for you: Upgrade the dependencies?", Tone::Waiting)
                ),
                ("ask2".into(), line("Waiting for you", Tone::Waiting)),
                ("broken".into(), line("Error: status failed", Tone::Problem)),
                (
                    "work".into(),
                    line("~/code/paddock · p2d-render", Tone::Quiet)
                ),
                ("new".into(), line("Starting…", Tone::Quiet)),
                // No Git there: only the directory, never "git unavailable".
                ("plain".into(), line("/tmp", Tone::Quiet)),
                ("gone".into(), line("Exited", Tone::Problem)),
            ]
        );
    }

    #[test]
    fn long_directories_lose_their_middle_first() {
        let home = Some("/Users/me");
        let path = "/Users/me/Developer/personal_projs/paddock";
        let within = |n: usize| move |text: &str| text.chars().count() <= n;
        assert_eq!(
            place(path, Some("main"), home, &within(80)),
            "~/Developer/personal_projs/paddock · main"
        );
        assert_eq!(
            place(path, Some("main"), home, &within(34)),
            "~/…/personal_projs/paddock · main"
        );
        assert_eq!(
            place(path, Some("main"), home, &within(20)),
            "~/…/paddock · main"
        );
        // Never shorter than the start and the last level; the drawing cuts the rest.
        assert_eq!(
            place(path, Some("main"), home, &within(4)),
            "~/…/paddock · main"
        );
        assert_eq!(place("/opt/a/b/c", None, home, &within(6)), "/…/b/c");
        assert_eq!(place("/opt/a/b/c", None, home, &within(5)), "/…/c");
        assert_eq!(place("/Users/me", None, home, &within(1)), "~");
        assert_eq!(tilde("/Users/melody/x", home), "/Users/melody/x");
    }

    #[test]
    fn long_names_are_cut_at_the_end() {
        let within = |n: usize| move |text: &str| text.chars().count() <= n;
        assert_eq!(elide("dev-buttons", &within(11)), "dev-buttons");
        assert_eq!(elide("dev-buttons", &within(8)), "dev-but…");
        assert_eq!(elide("dev-buttons", &within(0)), "…");
    }

    #[test]
    fn a_second_click_on_the_shown_agent_opens_its_details() {
        let mut panel = panel(vec![agent("p/a", "idle"), agent("p/b", "idle")]);
        let shown = |panel: &Panel, selected| -> Vec<(String, Click, bool)> {
            cards(&lines(panel, None, selected, None, &roomy, 100.0))
                .iter()
                .map(|c| (c.short.clone(), click(c), c.expanded))
                .collect()
        };
        // Every card starts as two lines; the first click only opens the agent.
        assert_eq!(
            shown(&panel, None),
            [
                ("a".into(), Click::Open, false),
                ("b".into(), Click::Open, false)
            ]
        );
        // Once it is the one shown, another click opens its details, and the next closes them.
        assert_eq!(
            shown(&panel, Some("p/a"))[0],
            ("a".into(), Click::Details, false)
        );
        panel.toggle_details("p/a");
        assert_eq!(
            shown(&panel, Some("p/a"))[0],
            ("a".into(), Click::Details, true)
        );
        // Details stay open while another agent is shown, until collapsed.
        panel.toggle_details("p/b");
        assert_eq!(
            shown(&panel, Some("p/b")),
            [
                ("a".into(), Click::Open, true),
                ("b".into(), Click::Details, true)
            ]
        );
        panel.collapse_all();
        assert!(shown(&panel, Some("p/b")).iter().all(|(_, _, open)| !open));
        panel.toggle_details("p/a");
        panel.absorb(vec![agent("p/b", "idle")], None, 101.0);
        panel.absorb(
            vec![agent("p/a", "idle"), agent("p/b", "idle")],
            None,
            102.0,
        );
        assert!(
            !panel.expanded.contains("p/a"),
            "an agent that went away comes back folded"
        );
    }

    #[test]
    fn details_name_the_internals_in_words() {
        let mut labels = serde_json::Map::new();
        labels.insert("model".into(), "opus".into());
        labels.insert("effort".into(), "high".into());
        let a = Agent {
            cwd: Some("/Users/me/code/paddock".into()),
            instance: Some("b18cda32ce36".into()),
            attached: 1,
            last_input_source: Some("human".into()),
            last_input_at: Some(100.0 - 150.0),
            last_tool: Some("Bash".into()),
            labels,
            ..agent("p/a", "working")
        };
        let mut panel = panel(vec![a, agent("p/b", "idle")]);
        panel.absorb_git(vec![("/Users/me/code/paddock".into(), summary(25, 3))]);
        let lines = lines(&panel, None, None, Some("/Users/me"), &roomy, 100.0);
        let rows = |card: &Card| -> Vec<(&str, String)> {
            card.details
                .iter()
                .map(|d| (d.label, d.value.clone()))
                .collect()
        };
        let cards = cards(&lines);
        assert_eq!(
            rows(cards[0]),
            [
                ("Status", "Working · Bash".into()),
                ("Directory", "~/code/paddock".into()),
                (
                    "Branch",
                    "p2d-render · +25 -3 · 3 untracked · 2 ahead".into()
                ),
                ("Model", "opus · high".into()),
                ("Attached", "1 window".into()),
                ("Last input", "You, 2 min ago".into()),
                ("Instance", "b18cda32ce36".into()),
            ]
        );
        assert!(cards[0].details.last().unwrap().mono);
        // Rows with nothing to say are left out; Git is still loading for this one.
        assert_eq!(
            rows(cards[1]),
            [
                ("Status", "Idle".into()),
                ("Directory", "/w/p/b".into()),
                ("Attached", "no windows".into()),
                ("Instance", "abcdef123".into()),
            ]
        );
        let clean = Summary {
            head: Head::Branch("main".into()),
            ahead: Some((0, "origin/main".into())),
            changes: Some(Changes {
                added: 0,
                deleted: 0,
                binary: 0,
            }),
            untracked: Some(0),
        };
        assert_eq!(branch_details(&clean), "main · clean");
    }

    #[test]
    fn only_a_working_dot_breathes() {
        let dune = crate::preset::Preset::Dune.theme();
        let expected = [
            (Status::Waiting, dune.agents_yellow),
            (Status::Error, dune.agents_red),
            (Status::Stalled, dune.agent_stalled),
            (Status::Working, dune.agents_blue),
            (Status::Starting, dune.agent_starting),
            (Status::Unknown, dune.agents_dim),
            (Status::Idle, dune.agents_green),
            (Status::Exited, dune.agents_red),
        ];
        for (status, color) in expected {
            let look = look(status);
            assert_eq!((look.color)(&dune), color, "{status:?}");
            assert_eq!(look.breathing, status == Status::Working, "{status:?}");
        }
    }

    #[test]
    fn input_ages_read_as_words() {
        assert_eq!(ago(20.0), "just now");
        assert_eq!(ago(150.0), "2 min ago");
        assert_eq!(ago(2.5 * 3600.0), "2 hr ago");
        assert_eq!(ago(1.5 * 86400.0), "1 day ago");
        assert_eq!(ago(3.0 * 86400.0), "3 days ago");
    }

    #[test]
    fn ages_count_from_the_current_turn_or_state() {
        let working = Agent {
            turn_started: Some(40.0),
            kind: Some("codex".into()),
            ..agent("p/a", "working")
        };
        let idle = Agent {
            state_started: Some(100.0 - 7200.0),
            ..agent("p/b", "idle")
        };
        let panel = panel(vec![working, idle]);
        let lines = lines(&panel, None, None, None, &roomy, 100.0);
        let cards = cards(&lines);
        assert_eq!(cards[0].time, "1m");
        assert_eq!(cards[0].brand.as_ref().unwrap().kind, "codex");
        assert_eq!(cards[1].time, "2.0h");
    }

    #[test]
    fn sorting_switches_between_status_and_name() {
        let mut panel = panel(vec![
            agent("p/a", "idle"),
            agent("p/b", "working"),
            agent("p/c", "blocked"),
        ]);
        let names = |panel: &Panel| -> Vec<String> {
            cards(&lines(panel, None, None, None, &roomy, 100.0))
                .iter()
                .map(|c| c.short.clone())
                .collect()
        };
        assert_eq!(names(&panel), ["c", "b", "a"]);
        panel.by_name = true;
        assert_eq!(names(&panel), ["a", "b", "c"]);
    }
}
