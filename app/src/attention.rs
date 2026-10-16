//! Attention: the agents waiting for input or in error, a failed corral read, then the agents with
//! a reply nobody has looked at. The agent part of Saddle's Attention (`src/attention.rs` at
//! commit `df1c727`); plugin items come later. The tabs' marks go by it, and the Inbox (`inbox.rs`,
//! DESIGN §13 P5-87) builds its list and the bell's count on it.
use crate::{
    agents::{Panel, Status},
    corral::Agent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Waiting,
    Error,
    ReadFailed,
    Reply,
}

impl Kind {
    pub fn mark(self) -> &'static str {
        match self {
            Kind::Waiting => "?",
            Kind::Error | Kind::ReadFailed => "!",
            Kind::Reply => "•",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: Kind,
    /// The agent to open; `None` for a failed read, which has nothing to open.
    pub agent: Option<String>,
    pub label: String,
    /// An error text, or `new reply` when an agent needing attention also has one.
    pub note: String,
}

impl Item {
    /// Needs a person, rather than only having a reply to read.
    pub fn needs(&self) -> bool {
        self.kind != Kind::Reply
    }

    pub fn reason(&self) -> &'static str {
        match self.kind {
            Kind::Waiting => "Waiting for input",
            Kind::Error => "Error",
            Kind::ReadFailed => "Read failed",
            Kind::Reply => "New reply",
        }
    }

    pub fn mark(&self) -> &'static str {
        self.kind.mark()
    }
}

/// How the header's bell looks: quiet with nothing to look at, the accent for new replies only,
/// amber when an agent waits for a person, is in error or corral cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    Quiet,
    Replies,
    Needs,
}

/// The bell: how many `items` there are, and how urgent the most urgent one is.
pub fn bell(items: &[Item]) -> (usize, Urgency) {
    let urgency = if items.iter().any(Item::needs) {
        Urgency::Needs
    } else if items.is_empty() {
        Urgency::Quiet
    } else {
        Urgency::Replies
    };
    (items.len(), urgency)
}

/// What a machine's capsule in the switcher shows of the `items` that ring its bell (DESIGN §13
/// P5-130): the most urgent [`Kind`] (`Error` and `ReadFailed` before `Waiting`, before `Reply`),
/// how many there are in all, and the hover line naming the counts by kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capsule {
    pub kind: Kind,
    pub count: usize,
    pub line: String,
}

/// [`Capsule`] for a machine's `items`; `None` with nothing to look at.
pub fn capsule(items: &[Item]) -> Option<Capsule> {
    let order = [Kind::Error, Kind::ReadFailed, Kind::Waiting, Kind::Reply];
    let kind = order
        .into_iter()
        .find(|&wanted| items.iter().any(|item| item.kind == wanted))?;
    let mut parts = Vec::new();
    for wanted in order {
        let n = items.iter().filter(|item| item.kind == wanted).count();
        if n == 0 {
            continue;
        }
        let part = match (wanted, n) {
            (Kind::Error, 1) => "1 error".into(),
            (Kind::Error, n) => format!("{n} errors"),
            (Kind::ReadFailed, 1) => "1 read failed".into(),
            (Kind::ReadFailed, n) => format!("{n} reads failed"),
            (Kind::Waiting, n) => format!("{n} waiting"),
            (Kind::Reply, 1) => "1 new reply".into(),
            (Kind::Reply, n) => format!("{n} new replies"),
        };
        parts.push(part);
    }
    Some(Capsule {
        kind,
        count: items.len(),
        line: parts.join(" · "),
    })
}

/// Why an agent is in error: corral's error for it, or that its protocol is not this one's.
pub fn problem(a: &Agent) -> Option<String> {
    let incompatible = a
        .incompatible
        .then(|| format!("incompatible protocol {}", a.proto.unwrap_or(0)));
    a.error.clone().or(incompatible)
}

/// Needs attention first (waiting, then error, then a failed read), then new replies by name;
/// one row per agent, its need before its reply.
pub fn items(panel: &Panel, corral_error: Option<&str>, now: f64) -> Vec<Item> {
    let mut items = Vec::new();
    for (wanted, kind) in [
        (Status::Waiting, Kind::Waiting),
        (Status::Error, Kind::Error),
    ] {
        for a in panel
            .agents
            .iter()
            // As the cards show it: a paused agent asks nothing until it is resumed.
            .filter(|a| panel.shown(a, now) == wanted)
        {
            items.push(Item {
                kind,
                agent: Some(a.name.clone()),
                label: a.name.clone(),
                note: problem(a).unwrap_or_default(),
            });
        }
    }
    if let Some(error) = corral_error {
        items.push(Item {
            kind: Kind::ReadFailed,
            agent: None,
            label: "corral".into(),
            note: error.to_owned(),
        });
    }
    let mut unread: Vec<&str> = panel
        .agents
        .iter()
        .filter(|a| panel.unread.contains(&a.name))
        .map(|a| a.name.as_str())
        .collect();
    unread.sort();
    for name in unread {
        if let Some(item) = items.iter_mut().find(|i| i.agent.as_deref() == Some(name)) {
            item.note = if item.note.is_empty() {
                "new reply".into()
            } else {
                format!("new reply · {}", item.note)
            };
            continue;
        }
        items.push(Item {
            kind: Kind::Reply,
            agent: Some(name.to_owned()),
            label: name.to_owned(),
            note: String::new(),
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corral::Agent;

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            instance: Some("123".into()),
            state: Some(state.into()),
            ..Default::default()
        }
    }

    #[test]
    fn needs_come_first_then_replies_one_row_per_agent() {
        let mut panel = Panel::default();
        panel.absorb(
            vec![
                agent("p/idle", "idle"),
                agent("p/ask", "blocked"),
                Agent {
                    error: Some("status failed".into()),
                    ..agent("p/broken", "error")
                },
                agent("p/work", "working"),
            ],
            None,
            100.0,
        );
        panel
            .unread
            .extend(["p/work".to_owned(), "p/ask".to_owned()]);
        let items = items(&panel, Some("corral: timed out"), 100.0);
        let rows: Vec<(Kind, &str, &str)> = items
            .iter()
            .map(|i| (i.kind, i.label.as_str(), i.note.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                (Kind::Waiting, "p/ask", "new reply"),
                (Kind::Error, "p/broken", "status failed"),
                (Kind::ReadFailed, "corral", "corral: timed out"),
                (Kind::Reply, "p/work", ""),
            ]
        );
        assert!(items[2].agent.is_none());
        assert!(items[0].needs() && !items[3].needs());
        // The bell counts every row and takes the colour of the most urgent.
        assert_eq!(bell(&items), (4, Urgency::Needs));
        assert_eq!((items[0].mark(), items[3].mark()), ("?", "•"));
    }

    #[test]
    fn a_paused_agent_waits_without_ringing_but_an_error_still_does() {
        let mut panel = Panel::default();
        panel.absorb(
            vec![
                Agent {
                    paused: true,
                    ..agent("p/ask", "blocked")
                },
                Agent {
                    paused: true,
                    error: Some("status failed".into()),
                    ..agent("p/broken", "error")
                },
            ],
            None,
            100.0,
        );
        let rows: Vec<(Kind, String)> = items(&panel, None, 100.0)
            .iter()
            .map(|i| (i.kind, i.label.clone()))
            .collect();
        assert_eq!(rows, [(Kind::Error, "p/broken".to_owned())]);
    }

    #[test]
    fn nothing_to_look_at_is_an_empty_list() {
        let mut panel = Panel::default();
        panel.absorb(vec![agent("p/a", "idle")], None, 100.0);
        assert!(items(&panel, None, 100.0).is_empty());
    }

    #[test]
    fn the_bell_stays_and_shows_how_urgent() {
        // Nothing to look at: the bell is still there, quiet and without a number.
        assert_eq!(bell(&[]), (0, Urgency::Quiet));
        let reply = Item {
            kind: Kind::Reply,
            agent: Some("p/a".into()),
            label: "p/a".into(),
            note: String::new(),
        };
        assert_eq!(bell(&[reply.clone(), reply.clone()]), (2, Urgency::Replies));
        let waiting = Item {
            kind: Kind::Waiting,
            ..reply.clone()
        };
        assert_eq!(bell(&[reply, waiting]), (2, Urgency::Needs));
    }

    #[test]
    fn machine_capsule_takes_the_most_urgent_kind_counts_all_and_names_each() {
        let item = |kind: Kind| Item {
            kind,
            agent: (kind != Kind::ReadFailed).then(|| "p/a".into()),
            label: "p/a".into(),
            note: String::new(),
        };
        assert_eq!(capsule(&[]), None);
        // Only new replies.
        assert_eq!(
            capsule(&[item(Kind::Reply)]),
            Some(Capsule {
                kind: Kind::Reply,
                count: 1,
                line: "1 new reply".into(),
            })
        );
        assert_eq!(
            capsule(&[item(Kind::Reply), item(Kind::Reply)]),
            Some(Capsule {
                kind: Kind::Reply,
                count: 2,
                line: "2 new replies".into(),
            })
        );
        // Waiting and new replies: `?` for waiting, the total of both.
        assert_eq!(
            capsule(&[item(Kind::Waiting), item(Kind::Reply), item(Kind::Reply)]),
            Some(Capsule {
                kind: Kind::Waiting,
                count: 3,
                line: "1 waiting · 2 new replies".into(),
            })
        );
        // Error, waiting and new replies: `!` for the error, even when waiting comes first.
        assert_eq!(
            capsule(&[
                item(Kind::Waiting),
                item(Kind::Error),
                item(Kind::Reply),
                item(Kind::Reply),
            ]),
            Some(Capsule {
                kind: Kind::Error,
                count: 4,
                line: "1 error · 1 waiting · 2 new replies".into(),
            })
        );
        // Only a failed read: `!`, written apart from agent errors.
        assert_eq!(
            capsule(&[item(Kind::ReadFailed)]),
            Some(Capsule {
                kind: Kind::ReadFailed,
                count: 1,
                line: "1 read failed".into(),
            })
        );
        assert_eq!(
            capsule(&[item(Kind::Error), item(Kind::Error), item(Kind::ReadFailed)]),
            Some(Capsule {
                kind: Kind::Error,
                count: 3,
                line: "2 errors · 1 read failed".into(),
            })
        );
    }
}
