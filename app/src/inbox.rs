//! The Inbox (DESIGN §13 P5-87): the bell's panel, where each controller's last reply is read and
//! answered without going to its pane. This is the part that needs no window: whom the list has
//! and in which group, how the list holds still while it is open, what a row says, whether a
//! reply can be typed and what the box says when not, and what a refused send says. `window.rs`
//! draws it and does the sending.
use crate::{
    agents::{Panel, Status},
    attention::{self, Item},
    corral::{Agent, Refused, Role},
    markdown::{self, Block},
    search,
};

/// The list's groups, the most urgent first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Waiting for a person, in error, or corral cannot be read: any agent.
    Needs,
    /// Controllers with a reply nobody has read.
    New,
    /// The other controllers.
    Read,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Needs, Group::New, Group::Read];

    pub fn title(self) -> &'static str {
        match self {
            Group::Needs => "Needs you",
            Group::New => "New replies",
            Group::Read => "Read",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub group: Group,
    /// The agent; `None` for a failed corral read.
    pub agent: Option<String>,
}

/// Labelled a controller. Only the public `role` label says so, never the name.
fn controller(a: &Agent) -> bool {
    a.role() == Some(Role::Controller)
}

/// What the bell counts, the rows of the first two groups: Attention's items, but the new replies
/// of controllers only. Another agent's new reply still marks its card and its tab.
pub fn ringing(panel: &Panel, corral_error: Option<&str>, now: f64) -> Vec<Item> {
    let mut items = attention::items(panel, corral_error, now);
    items.retain(|item| {
        item.needs()
            || panel
                .agents
                .iter()
                .any(|a| Some(&a.name) == item.agent.as_ref() && controller(a))
    });
    items
}

/// The list as it opens: what needs a person in Attention's order, the controllers with a new
/// reply by name, then the other controllers by name.
pub fn rows(panel: &Panel, corral_error: Option<&str>, now: f64) -> Vec<Row> {
    let mut rows: Vec<Row> = ringing(panel, corral_error, now)
        .into_iter()
        .map(|item| Row {
            group: if item.needs() {
                Group::Needs
            } else {
                Group::New
            },
            agent: item.agent,
        })
        .collect();
    let mut read: Vec<String> = panel
        .agents
        .iter()
        .filter(|a| controller(a))
        .filter(|a| !rows.iter().any(|row| row.agent.as_ref() == Some(&a.name)))
        .map(|a| a.name.clone())
        .collect();
    read.sort();
    rows.extend(read.into_iter().map(|name| Row {
        group: Group::Read,
        agent: Some(name),
    }));
    rows
}

/// The list while the Inbox stays open, and the row selected in it.
pub struct Open {
    rows: Vec<Row>,
    selected: usize,
}

impl Open {
    /// Sorted as things stand, the first row selected.
    pub fn new(panel: &Panel, corral_error: Option<&str>, now: f64) -> Self {
        Open {
            rows: rows(panel, corral_error, now),
            selected: 0,
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn index(&self) -> usize {
        self.selected
    }

    pub fn selected(&self) -> Option<&Row> {
        self.rows.get(self.selected)
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.rows.len().saturating_sub(1));
    }

    /// The selection moved by `by` rows, stopping at either end.
    pub fn step(&mut self, by: isize) {
        self.selected = search::step(self.selected, self.rows.len(), by);
    }

    /// Takes a new listing without a row jumping from under the hand: a row stays where it is
    /// once its reply is read or its question answered, and only leaves with its agent (or, the
    /// failed read, once corral answers). What is new comes in at the end of its group, a row
    /// moving only to a more urgent one. The selection stays on its row.
    pub fn refresh(&mut self, panel: &Panel, corral_error: Option<&str>, now: f64) {
        let fresh = rows(panel, corral_error, now);
        let there = |row: &Row| match &row.agent {
            Some(name) => panel.agents.iter().any(|a| &a.name == name),
            None => corral_error.is_some(),
        };
        let mut rows = Vec::new();
        for group in Group::ALL {
            for row in &self.rows {
                let now_in = fresh.iter().find(|f| f.agent == row.agent).map(|f| f.group);
                if row.group == group && there(row) && now_in.is_none_or(|now_in| now_in >= group) {
                    rows.push(row.clone());
                }
            }
            for row in fresh.iter().filter(|row| row.group == group) {
                let held = self
                    .rows
                    .iter()
                    .any(|old| old.agent == row.agent && old.group <= group);
                if !held {
                    rows.push(row.clone());
                }
            }
        }
        let on = self.selected().map(|row| row.agent.clone());
        let at = on.and_then(|agent| rows.iter().position(|row| row.agent == agent));
        self.rows = rows;
        self.select(at.unwrap_or(self.selected));
    }
}

/// The row's reply is on show, which reads it: no longer unread for the bell, its card or its tab.
/// Whether that changed anything.
pub fn read(panel: &mut Panel, row: &Row) -> bool {
    row.agent
        .as_ref()
        .is_some_and(|name| panel.unread.remove(name))
}

/// A reply sent from the Inbox, kept on show under the agent's last reply until its next one.
#[derive(Clone, Debug, PartialEq)]
pub struct Sent {
    pub text: String,
    /// When it was handed to corral, in seconds since the epoch.
    pub at: f64,
    /// The instance it went to.
    pub instance: Option<String>,
}

impl Sent {
    /// Whether `a` has yet to answer it: the same instance, and no turn ended since (it is not
    /// idle or waiting in a state begun after the reply went).
    pub fn awaited(&self, a: &Agent) -> bool {
        let ended = matches!(a.state.as_deref(), Some("idle" | "blocked"))
            && a.state_started.is_some_and(|since| since > self.at);
        a.instance == self.instance && !ended
    }

    /// Whether it is still on show under `a`'s last reply: until the reply of a turn ended since
    /// is read, so the older reply is never seen without it.
    pub fn shown(&self, panel: &Panel, a: &Agent, now: f64) -> bool {
        let answered = panel.said(a, now).is_some_and(|said| said.current);
        a.instance == self.instance && (self.awaited(a) || !answered)
    }
}

/// The agent's status as its row, its pill and the reply box go by: as the cards show it (a paused
/// agent takes nothing), and working from the moment a reply went to it, before corral's next
/// listing says so.
pub fn status(panel: &Panel, a: &Agent, sent: Option<&Sent>, now: f64) -> Status {
    match panel.shown(a, now) {
        Status::Idle if sent.is_some_and(|sent| sent.awaited(a)) => Status::Working,
        status => status,
    }
}

/// Whether a reply can be typed for an agent in `status` (`None`: no longer listed): only while it
/// idles. Otherwise what the box says in its place.
pub fn composer(status: Option<Status>) -> Result<(), &'static str> {
    Err(match status {
        Some(Status::Idle) => return Ok(()),
        Some(Status::Working | Status::Stalled) => "Working… you can reply when this turn ends",
        Some(Status::Starting) => "Starting… you can reply once it is ready",
        Some(Status::Paused) => "Paused… resume it to reply",
        Some(Status::Waiting) => "Can't reply here while a prompt is showing",
        Some(Status::Error) => "Can't reply while it is in error",
        Some(Status::Unknown) => "Can't reply while its state is unknown",
        Some(Status::Exited) | None => "It's no longer running",
    })
}

/// What the reply box says under a reply corral did not take: its own words for the refusals it
/// names, corral's sentence for anything else.
pub fn refusal(refused: &Refused) -> String {
    match refused.code.as_deref() {
        Some("not_idle") => "It's busy — try again when this turn ends".into(),
        Some("human_active") => "You typed in its pane a moment ago — try again shortly".into(),
        Some("not_found") => "It's no longer running".into(),
        _ => refused.message.clone(),
    }
}

/// How a row's line or a pill is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Waits for a person: amber.
    Waiting,
    /// In error: red.
    Problem,
    /// Working: blue.
    Working,
    /// A reply not read yet: the text's colour.
    New,
    /// Anything else: dim.
    Quiet,
}

/// What a row says under its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
    /// The first line of the reply just sent to it, shown after as `You: …`.
    pub sent: Option<String>,
}

/// A row's line: the first sentence of the last reply, why it needs a person, or that it works.
pub fn line(
    panel: &Panel,
    row: &Row,
    corral_error: Option<&str>,
    sent: Option<&Sent>,
    now: f64,
) -> Line {
    let say = |text: &str, tone| Line {
        text: text.to_owned(),
        tone,
        sent: None,
    };
    let Some(name) = &row.agent else {
        return say(corral_error.unwrap_or("Read failed"), Tone::Problem);
    };
    let Some(a) = panel.agents.iter().find(|a| &a.name == name) else {
        return say("No longer running", Tone::Quiet);
    };
    match status(panel, a, sent, now) {
        Status::Waiting => say("Waiting for input", Tone::Waiting),
        Status::Error => match attention::problem(a) {
            Some(problem) => say(&problem, Tone::Problem),
            None => say("Error", Tone::Problem),
        },
        Status::Working | Status::Stalled => Line {
            sent: sent.filter(|sent| sent.awaited(a)).map(|sent| {
                sent.text
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            }),
            ..say("Working", Tone::Working)
        },
        Status::Starting => say("Starting…", Tone::Quiet),
        Status::Paused => say("Paused", Tone::Quiet),
        Status::Exited => say("Exited", Tone::Quiet),
        Status::Idle | Status::Unknown => match panel.said(a, now) {
            Some(said) if panel.unread.contains(name) => {
                say(&first_sentence(&said.reply.text), Tone::New)
            }
            Some(said) => say(&first_sentence(&said.reply.text), Tone::Quiet),
            None => say("No reply yet", Tone::Quiet),
        },
    }
}

/// The small label after the agent's name over its reply; none for an idle agent whose reply is
/// not `new`.
pub fn pill(status: Status, new: bool) -> Option<(&'static str, Tone)> {
    Some(match status {
        Status::Waiting => ("Waiting for input", Tone::Waiting),
        Status::Error => ("Error", Tone::Problem),
        Status::Working | Status::Stalled => ("Working", Tone::Working),
        Status::Starting => ("Starting", Tone::Quiet),
        Status::Paused => ("Paused", Tone::Quiet),
        Status::Exited => ("Exited", Tone::Quiet),
        Status::Idle if new => ("New reply", Tone::New),
        Status::Idle | Status::Unknown => return None,
    })
}

/// The first sentence of a reply's first block, without its Markdown marks, for a row's line.
pub fn first_sentence(reply: &str) -> String {
    let words = |spans: &[markdown::Span]| spans.iter().map(|span| span.text.as_str()).collect();
    let first: String = match markdown::blocks_by_line(reply).into_iter().next() {
        Some(Block::Heading(_, spans) | Block::Paragraph(spans) | Block::Item { spans, .. }) => {
            words(&spans)
        }
        Some(Block::Code(code)) => code,
        Some(Block::Table { head, .. }) => {
            let cells: Vec<String> = head.iter().map(|cell| words(cell)).collect();
            cells.join(" · ")
        }
        None => String::new(),
    };
    let line = first.lines().next().unwrap_or_default();
    let mut chars = line.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let ends = match c {
            '。' | '！' | '？' => true,
            '.' | '!' | '?' => chars.peek().is_none_or(|(_, next)| next.is_whitespace()),
            _ => false,
        };
        if ends {
            return line[..at + c.len_utf8()].trim().to_owned();
        }
    }
    line.trim().to_owned()
}
