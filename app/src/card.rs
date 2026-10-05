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

/// How a note on the second line is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// That it is starting: dim.
    Quiet,
    /// It waits for a person: amber.
    Waiting,
    /// An error or an exit: red.
    Problem,
}

/// The card's second line.
#[derive(Clone, Debug, PartialEq)]
pub enum Second {
    /// Starting, waiting, an error or an exit, in words.
    Note(String, Tone),
    /// Where the agent works, for one that is just working or idle.
    Place(Place),
}

/// How a piece of the place line is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    /// The directory.
    Dim,
    /// The `·` and `⎇` before the branch, and the untracked count.
    Dimmer,
    Branch,
    /// Lines added: green.
    Added,
    /// Lines deleted: red.
    Deleted,
    /// Commits ahead of the base: amber.
    Ahead,
}

/// `dir · ⎇ branch  +added -deleted ?untracked ↑ahead`, each Git part only when there is
/// something to count.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    /// The directory, with home written `~`.
    pub dir: String,
    /// `None` while Git is unread or unavailable.
    pub branch: Option<String>,
    /// Lines added and deleted, when the worktree has changes.
    pub changes: Option<(u64, u64)>,
    pub untracked: u64,
    pub ahead: u64,
}

impl Place {
    fn new(dir: String, git: Option<&Option<Summary>>) -> Self {
        let summary = git.and_then(Option::as_ref);
        Self {
            dir,
            branch: branch(git),
            changes: summary
                .and_then(|s| s.changes.as_ref())
                .filter(|c| c.added + c.deleted > 0)
                .map(|c| (c.added, c.deleted)),
            untracked: summary.and_then(|s| s.untracked).unwrap_or(0),
            ahead: summary.and_then(|s| s.ahead.as_ref()).map_or(0, |a| a.0),
        }
    }

    /// The line in pieces, each with its colour.
    pub fn spans(&self) -> Vec<(String, Ink)> {
        let mut spans = vec![(self.dir.clone(), Ink::Dim)];
        if let Some(branch) = &self.branch {
            spans.push((" · ⎇ ".into(), Ink::Dimmer));
            spans.push((branch.clone(), Ink::Branch));
        }
        let mut marks = Vec::new();
        if let Some((added, deleted)) = self.changes {
            marks.push((format!("+{added}"), Ink::Added));
            marks.push((format!("-{deleted}"), Ink::Deleted));
        }
        if self.untracked > 0 {
            marks.push((format!("?{}", self.untracked), Ink::Dimmer));
        }
        if self.ahead > 0 {
            marks.push((format!("↑{}", self.ahead), Ink::Ahead));
        }
        for (index, mark) in marks.into_iter().enumerate() {
            spans.push((if index == 0 { "  " } else { " " }.into(), Ink::Dimmer));
            spans.push(mark);
        }
        spans
    }

    pub fn text(&self) -> String {
        self.spans().into_iter().map(|(text, _)| text).collect()
    }

    /// The line as it fits. The directory gives way first, from the middle, but keeps its last
    /// level (`~/…/paddock`, then `…/paddock`); then the branch, from its end; only then the last
    /// level itself, from its end. The counts always stay.
    pub fn fit(&self, fits: &dyn Fn(&str) -> bool) -> Place {
        let with = |dir: &str, branch: Option<&str>| Place {
            dir: dir.to_owned(),
            branch: branch.map(str::to_owned),
            ..self.clone()
        };
        let line = |dir: &str, branch: Option<&str>| fits(&with(dir, branch).text());
        let branch = self.branch.as_deref();
        if line(&self.dir, branch) {
            return self.clone();
        }
        let (head, rest) = match self.dir.strip_prefix('/') {
            Some(rest) => ("", rest),
            None => self.dir.split_once('/').unwrap_or((&self.dir, "")),
        };
        let levels: Vec<&str> = rest.split('/').filter(|l| !l.is_empty()).collect();
        // The shortest the directory goes before the branch gives way, and what is left to cut.
        let (lead, last) = match levels.last() {
            Some(last) if levels.len() >= 2 => ("…/", *last),
            _ => ("", self.dir.as_str()),
        };
        if levels.len() >= 2 {
            for start in 1..levels.len() {
                let candidate = format!("{head}/…/{}", levels[start..].join("/"));
                if line(&candidate, branch) {
                    return with(&candidate, branch);
                }
            }
        }
        let dir = format!("{lead}{last}");
        if line(&dir, branch) {
            return with(&dir, branch);
        }
        let branch = branch.map(|branch| elide(branch, &|cut| line(&dir, Some(cut))));
        if line(&dir, branch.as_deref()) {
            return with(&dir, branch.as_deref());
        }
        let last = elide(last, &|cut| {
            line(&format!("{lead}{cut}"), branch.as_deref())
        });
        with(&format!("{lead}{last}"), branch.as_deref())
    }
}

/// Starting, waiting, errors and exits say so; otherwise where the agent works.
fn second(a: &Agent, status: Status, title: Option<&str>, place: impl FnOnce() -> Place) -> Second {
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
        _ => return Second::Place(place()),
    };
    Second::Note(text, tone)
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

/// A directory as it fits, shortened in the middle: first whole levels go, down to its start and
/// last level (`~/…/paddock`); then characters, keeping more of the end (`~/D…dock`), down to `…`.
pub fn shorten(path: &str, fits: &dyn Fn(&str) -> bool) -> String {
    if fits(path) {
        return path.to_owned();
    }
    let (head, rest) = match path.strip_prefix('/') {
        Some(rest) => ("", rest),
        None => path.split_once('/').unwrap_or((path, "")),
    };
    let levels: Vec<&str> = rest.split('/').filter(|l| !l.is_empty()).collect();
    let chars: Vec<char> = path.chars().collect();
    let mut keep = chars.len();
    for start in 1..levels.len() {
        let candidate = format!("{head}/…/{}", levels[start..].join("/"));
        if fits(&candidate) {
            return candidate;
        }
        keep = keep.min(candidate.chars().count());
    }
    while keep > 0 {
        keep -= 1;
        let front = keep / 3;
        let candidate: String = chars[..front]
            .iter()
            .chain(['…'].iter())
            .chain(chars[chars.len() - (keep - front)..].iter())
            .collect();
        if fits(&candidate) {
            return candidate;
        }
    }
    "…".to_owned()
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

/// How a detail's value is drawn, and where it gives way when too long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Cut at the end.
    Text,
    /// A directory: shortened in the middle.
    Path,
    /// The instance id, in the terminal font; cut at the end.
    Mono,
}

/// One row of the expanded details.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub label: &'static str,
    pub value: String,
    pub face: Face,
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
        face: Face::Text,
    };
    let mut rows = Vec::new();
    let state = match (status, &a.last_tool) {
        (Status::Working | Status::Stalled, Some(tool)) => format!("{} · {tool}", look.label),
        _ => look.label.to_owned(),
    };
    rows.push(row("Status", state));
    if let Some(cwd) = &a.cwd {
        rows.push(Detail {
            face: Face::Path,
            ..row("Directory", tilde(cwd, home))
        });
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
            face: Face::Mono,
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
    /// Open in one of this window's panes: a quiet mark after the name.
    pub here: bool,
    /// Finished a turn this window hasn't shown yet: an accent dot after the name.
    pub unread: bool,
    pub brand: Option<Brand>,
    /// The `effort` label, written after the program.
    pub effort: Option<String>,
    /// The tool a working agent is using, before its age.
    pub tool: Option<String>,
    pub time: String,
    /// The age's colour: the status's, drawn lighter, while it works, waits or fails; `None` for
    /// the dim default.
    pub time_color: Option<Pick>,
    pub second: Second,
    pub details: Vec<Detail>,
    pub instance: Option<String>,
    pub cwd: Option<String>,
}

/// Keeps the name readable on a tight first line: while `room(card)` leaves the name less than
/// `floor`, first the effort goes, then the tool. The program and the age always stay.
pub fn yield_to_name(card: &mut Card, room: &dyn Fn(&Card) -> f32, floor: f32) {
    if room(card) < floor {
        card.effort = None;
    }
    if room(card) < floor {
        card.tool = None;
    }
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

/// The panel's lines. `selected` is the active pane's agent and `here` every agent open in this
/// window; `home` the home directory, written `~`. Text is whole: the drawing fits it to the card.
pub fn lines(
    panel: &Panel,
    error: Option<&str>,
    selected: Option<&str>,
    here: &[String],
    home: Option<&str>,
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
            panel, a, prefix, selected, here, home, now,
        ))));
    }
    lines
}

fn card(
    panel: &Panel,
    a: &Agent,
    prefix: &str,
    selected: Option<&str>,
    here: &[String],
    home: Option<&str>,
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
        Some(cwd) => Place::new(tilde(cwd, home), git),
        None => Place::new("—".into(), None),
    };
    let is_here = here.contains(&a.name);
    let time_color: Option<Pick> = match status {
        Status::Working | Status::Waiting | Status::Error => Some(look.color),
        _ => None,
    };
    Card {
        name: a.name.clone(),
        status,
        selected: selected == Some(a.name.as_str()),
        expanded: panel.expanded.contains(&a.name),
        here: is_here,
        // Open in a pane here, it is in view: not unread.
        unread: !is_here && panel.unread.contains(&a.name),
        brand: a.kind.as_deref().map(brand),
        effort: a
            .labels
            .get("effort")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .map(str::to_owned),
        tool: match status {
            Status::Working | Status::Stalled => a.last_tool.clone(),
            _ => None,
        },
        time: short_time(origin.map(|v| now - v)),
        time_color,
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
        let lines = lines(&panel, None, None, &[], None, 100.0);
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
        let lines = lines(&panel, None, None, &[], Some("/home/me"), 100.0);
        let seconds: Vec<(String, Second)> = cards(&lines)
            .iter()
            .map(|c| (c.short.clone(), c.second.clone()))
            .collect();
        let note = |text: &str, tone| Second::Note(text.into(), tone);
        assert_eq!(
            seconds,
            [
                (
                    "ask".into(),
                    note("Waiting for you: Upgrade the dependencies?", Tone::Waiting)
                ),
                ("ask2".into(), note("Waiting for you", Tone::Waiting)),
                ("broken".into(), note("Error: status failed", Tone::Problem)),
                (
                    "work".into(),
                    Second::Place(Place {
                        dir: "~/code/paddock".into(),
                        branch: Some("p2d-render".into()),
                        changes: Some((1, 0)),
                        untracked: 3,
                        ahead: 2,
                    })
                ),
                ("new".into(), note("Starting…", Tone::Quiet)),
                // No Git there: only the directory, never "git unavailable".
                (
                    "plain".into(),
                    Second::Place(Place {
                        dir: "/tmp".into(),
                        branch: None,
                        changes: None,
                        untracked: 0,
                        ahead: 0,
                    })
                ),
                ("gone".into(), note("Exited", Tone::Problem)),
            ]
        );
    }

    fn place(dir: &str, branch: Option<&str>) -> Place {
        Place {
            dir: dir.into(),
            branch: branch.map(Into::into),
            changes: None,
            untracked: 0,
            ahead: 0,
        }
    }

    #[test]
    fn the_place_line_counts_only_what_there_is() {
        let full = Place {
            changes: Some((25, 3)),
            untracked: 2,
            ahead: 1,
            ..place("~/code/paddock", Some("main"))
        };
        assert_eq!(full.text(), "~/code/paddock · ⎇ main  +25 -3 ?2 ↑1");
        let inks: Vec<Ink> = full.spans().into_iter().map(|(_, ink)| ink).collect();
        assert_eq!(
            inks,
            [
                Ink::Dim,
                Ink::Dimmer,
                Ink::Branch,
                Ink::Dimmer,
                Ink::Added,
                Ink::Dimmer,
                Ink::Deleted,
                Ink::Dimmer,
                Ink::Dimmer,
                Ink::Dimmer,
                Ink::Ahead,
            ]
        );
        // A clean worktree shows no counts; untracked files and commits ahead show alone.
        assert_eq!(place("~/x", Some("main")).text(), "~/x · ⎇ main");
        let untracked = Place {
            untracked: 4,
            ..place("~/x", Some("main"))
        };
        assert_eq!(untracked.text(), "~/x · ⎇ main  ?4");
        let ahead = Place {
            ahead: 3,
            ..place("~/x", Some("main"))
        };
        assert_eq!(ahead.text(), "~/x · ⎇ main  ↑3");
        // Changes from Git: none counted means none shown; binary files stay in the details.
        let git = |added, deleted, binary| {
            Some(Summary {
                head: Head::Branch("main".into()),
                ahead: Some((0, "origin/main".into())),
                changes: Some(Changes {
                    added,
                    deleted,
                    binary,
                }),
                untracked: Some(0),
            })
        };
        assert_eq!(
            Place::new("~/x".into(), Some(&git(0, 0, 2))).text(),
            "~/x · ⎇ main"
        );
        assert_eq!(
            Place::new("~/x".into(), Some(&git(0, 7, 0))).text(),
            "~/x · ⎇ main  +0 -7"
        );
        assert_eq!(Place::new("~/x".into(), Some(&None)).text(), "~/x");
    }

    #[test]
    fn a_tight_place_line_keeps_the_project_and_gives_up_the_branch_next() {
        let within = |n: usize| move |text: &str| text.chars().count() <= n;
        let line = Place {
            changes: Some((12, 3)),
            ahead: 1,
            ..place("~/Developer/personal_projs/paddock", Some("p5-8-cards"))
        };
        let fit = |n| line.fit(&within(n)).text();
        assert_eq!(
            fit(80),
            "~/Developer/personal_projs/paddock · ⎇ p5-8-cards  +12 -3 ↑1"
        );
        // Whole levels go from the middle of the directory first…
        assert_eq!(
            fit(52),
            "~/…/personal_projs/paddock · ⎇ p5-8-cards  +12 -3 ↑1"
        );
        assert_eq!(fit(40), "~/…/paddock · ⎇ p5-8-cards  +12 -3 ↑1");
        // …down to the last level, the project…
        assert_eq!(fit(35), "…/paddock · ⎇ p5-8-cards  +12 -3 ↑1");
        // …then the branch, from its end; the counts stay…
        assert_eq!(fit(30), "…/paddock · ⎇ p5-8…  +12 -3 ↑1");
        assert_eq!(fit(26), "…/paddock · ⎇ …  +12 -3 ↑1");
        // …and only then the last level itself, from its end.
        assert_eq!(fit(23), "…/pad… · ⎇ …  +12 -3 ↑1");
        // A long branch gives way while the project still shows.
        let long = Place {
            changes: Some((400, 0)),
            ..place(
                "~/Developer/personal_projs/paddock",
                Some("feature/an-unusually-long-branch-name"),
            )
        };
        assert_eq!(
            long.fit(&within(37)).text(),
            "…/paddock · ⎇ feature/an-un…  +400 -0"
        );
        // Without Git, only the directory; one with a single level is cut from its end.
        assert_eq!(place("/opt/a/b/c", None).fit(&within(6)).text(), "/…/b/c");
        assert_eq!(
            place("/tmp-very-long", None).fit(&within(6)).text(),
            "/tmp-…"
        );
    }

    #[test]
    fn long_directories_lose_their_middle_first() {
        let home = Some("/Users/me");
        let path = tilde("/Users/me/Developer/personal_projs/paddock", home);
        let within = |n: usize| move |text: &str| text.chars().count() <= n;
        assert_eq!(
            shorten(&path, &within(80)),
            "~/Developer/personal_projs/paddock"
        );
        assert_eq!(shorten(&path, &within(27)), "~/…/personal_projs/paddock");
        assert_eq!(shorten(&path, &within(12)), "~/…/paddock");
        assert_eq!(shorten(&path, &within(10)), "~/D…addock");
        assert_eq!(shorten(&path, &within(4)), "~…ck");
        assert_eq!(shorten(&path, &within(0)), "…");
        assert_eq!(shorten("/opt/a/b/c", &within(6)), "/…/b/c");
        assert_eq!(shorten("/opt/a/b/c", &within(5)), "/…/c");
        assert_eq!(shorten("~", &within(1)), "~");
        assert_eq!(tilde("/Users/me", home), "~");
        assert_eq!(tilde("/Users/melody/x", home), "/Users/melody/x");
    }

    #[test]
    fn marks_after_the_name_only_when_there_is_something_to_say() {
        // `b` finished a turn nobody here has seen; `c` did too but is open in a pane here.
        let mut panel = panel(vec![
            agent("p/a", "working"),
            agent("p/b", "working"),
            agent("p/c", "working"),
        ]);
        panel.absorb(
            vec![
                agent("p/a", "working"),
                agent("p/b", "idle"),
                agent("p/c", "idle"),
            ],
            None,
            101.0,
        );
        let here = ["p/c".to_owned()];
        let marks: Vec<(String, bool, bool)> =
            cards(&lines(&panel, None, None, &here, None, 101.0))
                .iter()
                .map(|c| (c.short.clone(), c.unread, c.here))
                .collect();
        assert_eq!(
            marks,
            [
                ("a".into(), false, false),
                ("b".into(), true, false),
                ("c".into(), false, true),
            ]
        );
        // Shown in the active pane, it is read.
        panel.absorb(
            vec![
                agent("p/a", "working"),
                agent("p/b", "idle"),
                agent("p/c", "idle"),
            ],
            Some("p/b"),
            102.0,
        );
        // Once no pane here shows it, an unseen turn shows again.
        let unread: Vec<String> = cards(&lines(&panel, None, None, &[], None, 102.0))
            .iter()
            .filter(|c| c.unread)
            .map(|c| c.short.clone())
            .collect();
        assert_eq!(unread, ["c"]);
    }

    #[test]
    fn the_first_line_names_the_tool_effort_and_colours_the_age() {
        let mut labels = serde_json::Map::new();
        labels.insert("effort".into(), "high".into());
        let working = Agent {
            last_tool: Some("Bash".into()),
            kind: Some("claude".into()),
            labels,
            ..agent("p/a", "working")
        };
        let idle = Agent {
            // The last tool of a finished turn is not shown.
            last_tool: Some("Edit".into()),
            ..agent("p/b", "idle")
        };
        let mut empty = serde_json::Map::new();
        empty.insert("effort".into(), "".into());
        let waiting = Agent {
            labels: empty,
            ..agent("p/c", "blocked")
        };
        let panel = panel(vec![working, idle, waiting]);
        let dune = crate::preset::Preset::Dune.theme();
        let lines = lines(&panel, None, None, &[], None, 100.0);
        let first: Vec<_> = cards(&lines)
            .iter()
            .map(|c| {
                (
                    c.short.clone(),
                    c.tool.clone(),
                    c.effort.clone(),
                    c.time_color.map(|pick| pick(&dune)),
                )
            })
            .collect();
        assert_eq!(
            first,
            [
                ("c".into(), None, None, Some(dune.agents_yellow)),
                (
                    "a".into(),
                    Some("Bash".into()),
                    Some("high".into()),
                    Some(dune.agents_blue)
                ),
                ("b".into(), None, None, None),
            ]
        );
    }

    #[test]
    fn the_effort_then_the_tool_make_room_for_the_name() {
        let mut labels = serde_json::Map::new();
        labels.insert("effort".into(), "xhigh".into());
        let a = Agent {
            last_tool: Some("Bash".into()),
            kind: Some("claude".into()),
            labels,
            ..agent("p/a", "working")
        };
        let panel = panel(vec![a]);
        let lines = lines(&panel, None, None, &[], None, 100.0);
        let card = cards(&lines)[0].clone();
        // 100 of room, less 30 for the effort and 40 for the tool.
        let room = |c: &Card| {
            100.0
                - if c.effort.is_some() { 30.0 } else { 0.0 }
                - if c.tool.is_some() { 40.0 } else { 0.0 }
        };
        let kept = |floor: f32| {
            let mut card = card.clone();
            yield_to_name(&mut card, &room, floor);
            (
                card.effort.is_some(),
                card.tool.is_some(),
                card.brand.is_some(),
            )
        };
        assert_eq!(kept(30.0), (true, true, true));
        assert_eq!(kept(60.0), (false, true, true));
        assert_eq!(kept(90.0), (false, false, true));
        // Even when nothing more can go, the program stays; the name is cut instead.
        assert_eq!(kept(200.0), (false, false, true));
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
            cards(&lines(panel, None, selected, &[], None, 100.0))
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
        let lines = lines(&panel, None, None, &[], Some("/Users/me"), 100.0);
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
        assert_eq!(cards[0].details.last().unwrap().face, Face::Mono);
        assert_eq!(cards[0].details[1].face, Face::Path);
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
        let lines = lines(&panel, None, None, &[], None, 100.0);
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
            cards(&lines(panel, None, None, &[], None, 100.0))
                .iter()
                .map(|c| c.short.clone())
                .collect()
        };
        assert_eq!(names(&panel), ["c", "b", "a"]);
        panel.by_name = true;
        assert_eq!(names(&panel), ["a", "b", "c"]);
    }
}
