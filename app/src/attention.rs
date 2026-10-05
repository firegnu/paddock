//! Attention: the agents waiting for input or in error, a failed corral read, then the agents with
//! a reply nobody has looked at. The agent part of Saddle's Attention (`src/attention.rs` at
//! commit `df1c727`); plugin items come later. Opening an item never changes an agent's state.
use crate::agents::{Panel, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Waiting,
    Error,
    ReadFailed,
    Reply,
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
        match self.kind {
            Kind::Waiting => "?",
            Kind::Error | Kind::ReadFailed => "!",
            Kind::Reply => "•",
        }
    }
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
            .filter(|a| panel.status(a, now) == wanted)
        {
            let incompatible = a
                .incompatible
                .then(|| format!("incompatible protocol {}", a.proto.unwrap_or(0)));
            items.push(Item {
                kind,
                agent: Some(a.name.clone()),
                label: a.name.clone(),
                note: a.error.clone().or(incompatible).unwrap_or_default(),
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
        assert_eq!((items[0].mark(), items[3].mark()), ("?", "•"));
    }

    #[test]
    fn nothing_to_look_at_is_an_empty_list() {
        let mut panel = Panel::default();
        panel.absorb(vec![agent("p/a", "idle")], None, 100.0);
        assert!(items(&panel, None, 100.0).is_empty());
    }
}
