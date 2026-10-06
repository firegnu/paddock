//! What the Agents panel shows for each agent, worked out apart from drawing so it can be tested.
//! Ages follow Saddle's Agents panel in `src/ui.rs` at commit `df1c727`; the rest is paddock's own
//! card, read like a conversation (DESIGN §13 P5-18): the name, effort and how long; a preview of
//! what the agent is doing, asking or last said; where it works; and, opened, the branch as chips,
//! the rest in labelled cells, the full path and the instance.
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

/// How long, for the details, in two units: `57s`, `4m 12s`, `1h 12m`, `3d 4h`; the second is left
/// out when it is nought.
fn how_long(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let ((major, big), (minor, small)) = match s {
        0..60 => return format!("{s}s"),
        60..3600 => ((s / 60, 'm'), (s % 60, 's')),
        3600..86400 => ((s / 3600, 'h'), (s % 3600 / 60, 'm')),
        _ => ((s / 86400, 'd'), (s % 86400 / 3600, 'h')),
    };
    match minor {
        0 => format!("{major}{big}"),
        _ => format!("{major}{big} {minor}{small}"),
    }
}

/// How a note in the preview is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// That it is starting: dim.
    Quiet,
    /// It waits for a person: amber.
    Waiting,
    /// An error or an exit: red.
    Problem,
}

/// What the card says under the name, in up to two lines (six when open).
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    /// Working or stalled: the tool, in the status's colour, then what its terminal title says it
    /// is doing.
    Busy {
        tool: Option<String>,
        title: Option<String>,
    },
    /// Starting, waiting, an error or an exit, in words.
    Note(String, Tone),
    /// Idle: the start of its last reply.
    Reply(String),
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

/// Working or stalled, the tool and the title; starting, waiting, errors and exits say so in words;
/// idle, the start of the last reply. `None` when there is nothing to say.
fn preview(
    a: &Agent,
    status: Status,
    title: Option<String>,
    reply: Option<&str>,
) -> Option<Preview> {
    let (text, tone) = match status {
        Status::Working | Status::Stalled => {
            let tool = a.last_tool.clone();
            return (tool.is_some() || title.is_some()).then_some(Preview::Busy { tool, title });
        }
        Status::Idle => {
            return reply
                .map(opening)
                .filter(|text| !text.is_empty())
                .map(Preview::Reply);
        }
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
        Status::Unknown => return None,
    };
    Some(Preview::Note(text, tone))
}

/// The terminal title without the spinner or mark a program puts before it; `None` when nothing is
/// left, or when it only names the group.
fn title(a: &Agent, prefix: &str) -> Option<String> {
    let title = a
        .title
        .as_deref()?
        .trim_start_matches(|c: char| c.is_whitespace() || spinner(c))
        .trim_end();
    (!title.is_empty() && title != prefix.trim_end_matches('/')).then(|| title.to_owned())
}

/// Spinner frames and marks before a title: Braille dots, geometric shapes, dingbats such as `✳`.
fn spinner(c: char) -> bool {
    matches!(c, '\u{2800}'..='\u{28ff}' | '\u{25a0}'..='\u{25ff}' | '\u{2700}'..='\u{27bf}' | '·' | '•' | '*')
}

/// The most of a reply a preview keeps; far more than six lines show.
const OPENING: usize = 800;

/// The start of a reply as one run of text: its lines and spaces each one space.
fn opening(text: &str) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match words.char_indices().nth(OPENING) {
        Some((at, _)) => words[..at].to_owned(),
        None => words,
    }
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

/// The branch as the place line names it; `None` while Git is unread or unavailable.
fn branch(git: Option<&Option<Summary>>) -> Option<String> {
    match &git?.as_ref()?.head {
        Head::Branch(branch) => Some(branch.clone()),
        Head::Detached => Some("detached HEAD".into()),
        Head::Unknown => None,
    }
}

/// A chip of the open card, in coloured pieces.
pub type Chip = Vec<(String, Ink)>;

/// The branch and its changes as chips: the branch; lines added and deleted, with binary files;
/// untracked files; commits to push; or that the worktree is clean.
fn git_chips(s: &Summary) -> Vec<Chip> {
    let mut chips = Vec::new();
    let branch = match &s.head {
        Head::Branch(branch) => Some(branch.clone()),
        Head::Detached => Some("detached HEAD".into()),
        Head::Unknown => None,
    };
    if let Some(branch) = branch {
        chips.push(vec![("⎇ ".into(), Ink::Dimmer), (branch, Ink::Branch)]);
    }
    let mut clean = true;
    if let Some(c) = &s.changes
        && c.added + c.deleted + c.binary > 0
    {
        clean = false;
        let mut chip = vec![
            (format!("+{}", c.added), Ink::Added),
            (" ".into(), Ink::Dimmer),
            (format!("-{}", c.deleted), Ink::Deleted),
        ];
        if c.binary > 0 {
            chip.push((format!(", {} binary", c.binary), Ink::Dimmer));
        }
        chips.push(chip);
    }
    if let Some(untracked) = s.untracked
        && untracked > 0
    {
        clean = false;
        chips.push(vec![(format!("?{untracked}"), Ink::Dimmer)]);
    }
    if let Some((ahead, _)) = &s.ahead
        && *ahead > 0
    {
        chips.push(vec![(format!("↑{ahead} to push"), Ink::Ahead)]);
    }
    if clean && s.changes.is_some() && s.untracked.is_some() {
        chips.push(vec![("clean".into(), Ink::Dimmer)]);
    }
    chips
}

/// One of the open card's cells: a small label over its value.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub label: &'static str,
    pub value: String,
}

/// What the open card adds.
#[derive(Clone, Debug, PartialEq)]
pub struct Details {
    /// Empty while Git is unread or unavailable.
    pub chips: Vec<Chip>,
    /// Two to a row.
    pub cells: Vec<Cell>,
    /// The directory in full, with home written `~`.
    pub path: Option<String>,
    /// The instance id as shown; the card keeps it whole.
    pub instance: Option<String>,
}

/// `origin` is when what the card's age counts began.
fn details(
    a: &Agent,
    status: Status,
    origin: Option<f64>,
    git: Option<&Option<Summary>>,
    here: bool,
    home: Option<&str>,
    now: f64,
) -> Details {
    let cell = |label, value: String| Cell { label, value };
    let mut cells = Vec::new();
    let word = look(status).label;
    let state = match (status, &a.last_tool) {
        (Status::Working | Status::Stalled, Some(tool)) => format!("{word} · {tool}"),
        _ => word.to_owned(),
    };
    cells.push(cell("Status", state));
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
    cells.push(cell(
        "Model",
        if model.is_empty() {
            "—".into()
        } else {
            model.join(" · ")
        },
    ));
    if let Some(origin) = origin {
        let label = match status {
            Status::Idle => "Idle for",
            Status::Waiting => "Waiting for",
            _ => "This turn",
        };
        cells.push(cell(label, how_long(now - origin)));
    }
    if let Some(started) = a.started {
        cells.push(cell("Up", how_long(now - started)));
    }
    let windows = match a.attached {
        0 => "none".to_owned(),
        n => n.to_string(),
    };
    cells.push(cell(
        "Windows",
        if here {
            format!("{windows} · open here")
        } else {
            windows
        },
    ));
    if let Some(source) = &a.last_input_source {
        let who = match source.as_str() {
            "human" => "You",
            "send" => "corral send",
            "agent" => "agent",
            other => other,
        };
        cells.push(cell(
            "Last input",
            match a.last_input_at {
                Some(at) => format!("{who} · {} ago", short_time(Some(now - at))),
                None => who.to_owned(),
            },
        ));
    }
    Details {
        chips: match git {
            Some(Some(summary)) => git_chips(summary),
            _ => Vec::new(),
        },
        cells,
        path: a.cwd.as_deref().map(|cwd| tilde(cwd, home)),
        instance: a.instance.clone(),
    }
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
    /// The details show below the place line.
    pub expanded: bool,
    /// Open in one of this window's panes: an accent dot after the name.
    pub here: bool,
    /// Finished a turn this window hasn't shown yet: an accent dot after the name.
    pub unread: bool,
    /// The kind, in the avatar.
    pub brand: Option<Brand>,
    /// The `effort` label, quiet after the name.
    pub effort: Option<String>,
    pub time: String,
    /// The age's colour: the status's, drawn lighter, while it works, waits or fails; `None` for
    /// the dim default.
    pub time_color: Option<Pick>,
    /// Under the name; `None` when there is nothing to say.
    pub preview: Option<Preview>,
    /// Where the agent works, under the preview.
    pub place: Place,
    pub details: Details,
    pub instance: Option<String>,
    pub cwd: Option<String>,
}

/// Keeps the name readable on a tight first line: while `room(card)` leaves the name less than
/// `floor`, the effort goes (the details still have it). The age always stays.
pub fn yield_to_name(card: &mut Card, room: &dyn Fn(&Card) -> f32, floor: f32) {
    if room(card) < floor {
        card.effort = None;
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
    /// A group's name, how many agents it has, and their statuses for its bar: those waiting,
    /// working, stalled, the rest, then the idle.
    Group(String, usize, Vec<Status>),
    Agent(Box<Card>),
}

/// Where a status goes in a group's bar.
fn bar_rank(status: Status) -> u8 {
    match status {
        Status::Waiting => 0,
        Status::Working => 1,
        Status::Stalled => 2,
        Status::Idle => 4,
        _ => 3,
    }
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
            let mut bar: Vec<Status> = ordered[index..]
                .iter()
                .take_while(|agent| group(&agent.name) == prefix)
                .map(|agent| panel.status(agent, now))
                .collect();
            bar.sort_by_key(|status| bar_rank(*status));
            let title = if prefix.is_empty() { "agents/" } else { prefix };
            lines.push(Line::Group(title.to_owned(), bar.len(), bar));
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
    let git = a.cwd.as_ref().and_then(|cwd| panel.git.get(cwd));
    let place = match &a.cwd {
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
        time: short_time(origin.map(|v| now - v)),
        time_color,
        preview: preview(a, status, title(a, prefix), panel.reply(a, now)),
        place,
        details: details(a, status, origin, git, is_here, home, now),
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
                Line::Group(name, count, _) => Some((name.as_str(), *count)),
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
    fn the_place_line_and_notes_say_where_the_agent_works_and_what_it_needs() {
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
        let cards = cards(&lines);
        let notes: Vec<(String, Option<Preview>)> = cards
            .iter()
            .map(|c| (c.short.clone(), c.preview.clone()))
            .collect();
        let note = |text: &str, tone| Some(Preview::Note(text.into(), tone));
        assert_eq!(
            notes,
            [
                (
                    "ask".into(),
                    note("Waiting for you: Upgrade the dependencies?", Tone::Waiting)
                ),
                ("ask2".into(), note("Waiting for you", Tone::Waiting)),
                ("broken".into(), note("Error: status failed", Tone::Problem)),
                ("work".into(), None),
                ("new".into(), note("Starting…", Tone::Quiet)),
                ("plain".into(), None),
                ("gone".into(), note("Exited", Tone::Problem)),
            ]
        );
        let place = |short: &str| &cards.iter().find(|c| c.short == short).unwrap().place;
        assert_eq!(
            place("work"),
            &Place {
                dir: "~/code/paddock".into(),
                branch: Some("p2d-render".into()),
                changes: Some((1, 0)),
                untracked: 3,
                ahead: 2,
            }
        );
        // No Git there: only the directory, never "git unavailable".
        assert_eq!(
            place("plain"),
            &Place {
                dir: "/tmp".into(),
                branch: None,
                changes: None,
                untracked: 0,
                ahead: 0,
            }
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
    fn the_card_names_the_tool_effort_and_colours_the_age() {
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
                    match &c.preview {
                        Some(Preview::Busy { tool, .. }) => tool.clone(),
                        _ => None,
                    },
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
    fn the_effort_makes_room_for_the_name() {
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
        // 100 of room, less 30 for the effort.
        let room = |c: &Card| 100.0 - if c.effort.is_some() { 30.0 } else { 0.0 };
        let kept = |floor: f32| {
            let mut card = card.clone();
            yield_to_name(&mut card, &room, floor);
            (card.effort.is_some(), card.time.clone())
        };
        assert_eq!(kept(60.0), (true, "—".into()));
        assert_eq!(kept(90.0), (false, "—".into()));
        // Even when nothing more can go, the age stays; the name is cut instead.
        assert_eq!(kept(200.0), (false, "—".into()));
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
        // Every card starts folded; the first click only opens the agent.
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
    fn the_preview_says_what_it_does_asks_or_last_said() {
        let titled = Agent {
            last_tool: Some("Edit".into()),
            // The spinner a program puts before its title goes.
            title: Some("⠐ P5-17 pane cards".into()),
            ..agent("p/titled", "working")
        };
        let untitled = Agent {
            last_tool: Some("Bash".into()),
            // A title that only names the group says nothing.
            title: Some("✳ p".into()),
            ..agent("p/untitled", "working")
        };
        let asking = Agent {
            title: Some("Upgrade the dependencies?".into()),
            ..agent("p/asking", "blocked")
        };
        let answered = Agent {
            state_started: Some(90.0),
            ..agent("p/answered", "idle")
        };
        let silent = Agent {
            state_started: Some(90.0),
            ..agent("p/silent", "idle")
        };
        let mut panel = panel(vec![
            titled,
            untitled,
            agent("p/quiet", "working"),
            asking,
            answered,
            silent,
        ]);
        for spell in panel.replies_to_read(100.0) {
            let text = (spell.name == "p/answered")
                .then(|| "Merged.\n\n  Tests pass,\tclippy is clean.\n".to_owned());
            panel.absorb_reply(spell, text);
        }
        let lines = lines(&panel, None, None, &[], None, 100.0);
        let previews: Vec<(String, Option<Preview>)> = cards(&lines)
            .iter()
            .map(|c| (c.short.clone(), c.preview.clone()))
            .collect();
        let busy = |tool: &str, title: Option<&str>| {
            Some(Preview::Busy {
                tool: Some(tool.into()),
                title: title.map(Into::into),
            })
        };
        assert_eq!(
            previews,
            [
                (
                    "asking".into(),
                    Some(Preview::Note(
                        "Waiting for you: Upgrade the dependencies?".into(),
                        Tone::Waiting
                    ))
                ),
                // Working with neither a tool nor a title: nothing to preview.
                ("quiet".into(), None),
                ("titled".into(), busy("Edit", Some("P5-17 pane cards"))),
                ("untitled".into(), busy("Bash", None)),
                (
                    "answered".into(),
                    Some(Preview::Reply(
                        "Merged. Tests pass, clippy is clean.".into()
                    ))
                ),
                ("silent".into(), None),
            ]
        );
    }

    #[test]
    fn everything_the_card_showed_is_still_on_it_or_in_its_details() {
        let mut labels = serde_json::Map::new();
        labels.insert("model".into(), "opus".into());
        labels.insert("effort".into(), "high".into());
        let a = Agent {
            kind: Some("claude".into()),
            cwd: Some("/Users/me/code/paddock".into()),
            instance: Some("b18cda32ce36".into()),
            attached: 1,
            last_input_source: Some("human".into()),
            last_input_at: Some(100.0 - 150.0),
            last_tool: Some("Bash".into()),
            turn_started: Some(100.0 - 252.0),
            started: Some(100.0 - 1320.0),
            labels,
            ..agent("p/a", "working")
        };
        let b = Agent {
            state_started: Some(100.0 - 57.0),
            ..agent("p/b", "idle")
        };
        let mut panel = panel(vec![a, b]);
        panel.absorb_git(vec![("/Users/me/code/paddock".into(), summary(25, 3))]);
        let here = ["p/a".to_owned()];
        let lines = lines(&panel, None, None, &here, Some("/Users/me"), 100.0);
        let cards = cards(&lines);
        let card = cards[0];
        // On the card: status, kind, name, effort, open here, how long, the tool and the place.
        assert_eq!(card.look.label, "Working");
        assert_eq!(card.brand.as_ref().unwrap().kind, "claude");
        assert_eq!(card.short, "a");
        assert_eq!(card.effort.as_deref(), Some("high"));
        assert!(card.here);
        assert_eq!(card.time, "4m");
        assert_eq!(
            card.preview,
            Some(Preview::Busy {
                tool: Some("Bash".into()),
                title: None
            })
        );
        assert_eq!(
            card.place.text(),
            "~/code/paddock · ⎇ p2d-render  +25 -3 ?3 ↑2"
        );
        // In its details: the branch and changes, the rest in cells, the full path, the instance.
        let chips = |card: &Card| -> Vec<String> {
            card.details
                .chips
                .iter()
                .map(|chip| chip.iter().map(|(text, _)| text.as_str()).collect())
                .collect()
        };
        let cells = |card: &Card| -> Vec<(&str, String)> {
            card.details
                .cells
                .iter()
                .map(|cell| (cell.label, cell.value.clone()))
                .collect()
        };
        assert_eq!(chips(card), ["⎇ p2d-render", "+25 -3", "?3", "↑2 to push"]);
        assert_eq!(
            cells(card),
            [
                ("Status", "Working · Bash".into()),
                ("Model", "opus · high".into()),
                ("This turn", "4m 12s".into()),
                ("Up", "22m".into()),
                ("Windows", "1 · open here".into()),
                ("Last input", "You · 2m ago".into()),
            ]
        );
        assert_eq!(card.details.path.as_deref(), Some("~/code/paddock"));
        assert_eq!(card.details.instance.as_deref(), Some("b18cda32ce36"));
        // With less to say: Git still loading, no model, input, start or windows.
        assert!(chips(cards[1]).is_empty());
        assert_eq!(
            cells(cards[1]),
            [
                ("Status", "Idle".into()),
                ("Model", "—".into()),
                ("Idle for", "57s".into()),
                ("Windows", "none".into()),
            ]
        );
        assert_eq!(cards[1].details.path.as_deref(), Some("/w/p/b"));
        let clean = Summary {
            head: Head::Branch("main".into()),
            ahead: Some((1, "origin/main".into())),
            changes: Some(Changes {
                added: 0,
                deleted: 0,
                binary: 0,
            }),
            untracked: Some(0),
        };
        let texts: Vec<String> = git_chips(&clean)
            .iter()
            .map(|chip| chip.iter().map(|(text, _)| text.as_str()).collect())
            .collect();
        assert_eq!(texts, ["⎇ main", "↑1 to push", "clean"]);
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
