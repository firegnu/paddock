//! Pausing agents (DESIGN §13 P5-33): corral freezes an agent with its processes and lets them go
//! on again; paddock only picks which agents, asks before pausing one that may be in the middle of
//! something, and says in the footer how it went. It never signals a process itself.
use crate::corral::{Agent, Client};
use std::{sync::atomic::AtomicBool, time::Duration};

/// How long one `corral pause` or `resume` may take.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Pausing (`pause`) or resuming some agents, as a click asks it: whom, and the question to ask
/// first, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub names: Vec<String>,
    pub pause: bool,
    pub ask: Option<String>,
}

/// An agent corral still runs: any it lists but those that have exited.
fn live(a: &Agent) -> bool {
    a.state.as_deref() != Some("exited")
}

/// The one button for every agent, clicked: pause every live agent not paused yet, asking first
/// when one may be working, or resume them all once all are paused; `None` with no live agent.
pub fn every(agents: &[Agent]) -> Option<Request> {
    let pause = all(agents)?;
    let names = targets(agents, pause);
    let ask = if pause {
        question(agents, &names)
    } else {
        None
    };
    Some(Request { names, pause, ask })
}

/// What the one button for every agent does: pause them (`true`) while any live agent is not
/// paused, resume them once all are; `None` with no live agent.
pub fn all(agents: &[Agent]) -> Option<bool> {
    let live: Vec<&Agent> = agents.iter().filter(|a| live(a)).collect();
    (!live.is_empty()).then(|| live.iter().any(|a| !a.paused))
}

/// Whom pausing (`pause`) or resuming every agent reaches: the live agents not paused yet, or the
/// paused ones.
fn targets(agents: &[Agent], pause: bool) -> Vec<String> {
    agents
        .iter()
        .filter(|a| live(a) && a.paused != pause)
        .map(|a| a.name.clone())
        .collect()
}

/// Before pausing `names` together: the question naming those working, or in a state paddock does
/// not know, which may be; `None` to go ahead.
pub fn question(agents: &[Agent], names: &[String]) -> Option<String> {
    let busy: Vec<&Agent> = agents
        .iter()
        .filter(|a| names.contains(&a.name))
        .filter(|a| !matches!(a.state.as_deref(), Some("idle" | "blocked" | "starting")))
        .collect();
    let list = busy
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sure = busy.iter().all(|a| a.state.as_deref() == Some("working"));
    let are = match (busy.len(), sure) {
        (0, _) => return None,
        (1, true) => "1 agent is",
        (1, false) => "1 agent may be",
        (n, true) => return Some(format!("{n} agents are working: {list}. Pause anyway?")),
        (n, false) => return Some(format!("{n} agents may be working: {list}. Pause anyway?")),
    };
    Some(format!("{are} working: {list}. Pause anyway?"))
}

/// Pauses (`pause`) or resumes each of `names` in turn with corral: each name, with why it failed.
pub fn run(corral: &str, names: &[String], pause: bool) -> Vec<(String, Result<(), String>)> {
    let client = Client {
        program: corral.to_owned(),
    };
    let op = if pause { "pause" } else { "resume" };
    names
        .iter()
        .map(|name| {
            let result = client
                .json(&[op, name], TIMEOUT, &AtomicBool::new(false))
                .map(drop)
                .map_err(|error| reason(format!("{error:#}"), &format!("{op} {name}: ")));
            (name.clone(), result)
        })
        .collect()
}

/// Why corral refused, after `prefix`, the command it answers: its error, in which an agent under
/// an older corral, which cannot pause, needs `corral upgrade`. Otherwise the whole error.
fn reason(error: String, prefix: &str) -> String {
    match error.strip_prefix(prefix) {
        Some("unsupported") => "needs corral upgrade".into(),
        Some(why) => why.to_owned(),
        None => error,
    }
}

/// The footer's note while it runs.
pub fn doing(names: &[String], pause: bool) -> String {
    let doing = if pause { "Pausing" } else { "Resuming" };
    match names {
        [name] => format!("{doing} {name}…"),
        _ => format!("{doing} {} agents…", names.len()),
    }
}

/// The footer's note on how it went, and whether anything failed: by name for one agent; else how
/// many, naming each that failed and why.
pub fn summary(results: &[(String, Result<(), String>)], pause: bool) -> (String, bool) {
    let (done, verb) = if pause {
        ("Paused", "pause")
    } else {
        ("Resumed", "resume")
    };
    let failed: Vec<String> = results
        .iter()
        .filter_map(|(name, result)| result.as_ref().err().map(|why| format!("{name} ({why})")))
        .collect();
    let (all, left) = (results.len(), results.len() - failed.len());
    let text = match results {
        [(name, Ok(()))] => format!("{done} {name}"),
        [(name, Err(why))] => format!("Couldn't {verb} {name}: {why}"),
        _ if failed.is_empty() => format!("{done} {all} agents"),
        _ if left == 0 => format!("Couldn't {verb} {all} agents: {}", failed.join(", ")),
        _ => format!(
            "{done} {left} of {all} agents; not {}: {}",
            done.to_lowercase(),
            failed.join(", ")
        ),
    };
    (text, !failed.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, state: Option<&str>, paused: bool) -> Agent {
        Agent {
            name: name.into(),
            state: state.map(Into::into),
            paused,
            ..Agent::default()
        }
    }

    #[test]
    fn the_one_button_pauses_until_every_live_agent_is_paused() {
        let mixed = [
            agent("p/a", Some("idle"), true),
            agent("p/b", Some("working"), false),
            agent("p/gone", Some("exited"), false),
        ];
        assert_eq!(all(&mixed), Some(true));
        // Only those not paused yet are paused, never an agent that has exited.
        assert_eq!(targets(&mixed, true), ["p/b"]);
        let paused = [
            agent("p/a", Some("idle"), true),
            agent("p/b", Some("working"), true),
            agent("p/gone", Some("exited"), false),
        ];
        assert_eq!(all(&paused), Some(false));
        assert_eq!(targets(&paused, false), ["p/a", "p/b"]);
        // Nothing to do without a live agent.
        assert_eq!(all(&[agent("p/gone", Some("exited"), false)]), None);
        assert_eq!(all(&[]), None);
    }

    #[test]
    fn pausing_asks_only_when_an_agent_may_be_working() {
        let agents = [
            agent("p/idle", Some("idle"), false),
            agent("p/ask", Some("blocked"), false),
            agent("p/boot", Some("starting"), false),
            agent("p/busy", Some("working"), false),
            agent("p/also", Some("working"), false),
            agent("p/odd", Some("unknown"), false),
            agent("p/unread", None, false),
        ];
        let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(
            question(&agents, &names(&["p/idle", "p/ask", "p/boot"])),
            None
        );
        assert_eq!(
            question(&agents, &names(&["p/idle", "p/busy", "p/also"])).as_deref(),
            Some("2 agents are working: p/busy, p/also. Pause anyway?")
        );
        assert_eq!(
            question(&agents, &names(&["p/busy"])).as_deref(),
            Some("1 agent is working: p/busy. Pause anyway?")
        );
        // A state it does not know, or could not read, may be working.
        assert_eq!(
            question(&agents, &names(&["p/busy", "p/odd", "p/unread"])).as_deref(),
            Some("3 agents may be working: p/busy, p/odd, p/unread. Pause anyway?")
        );
        assert_eq!(
            question(&agents, &names(&["p/unread"])).as_deref(),
            Some("1 agent may be working: p/unread. Pause anyway?")
        );
    }

    #[test]
    fn the_note_while_it_runs_names_one_and_counts_more() {
        assert_eq!(doing(&["p/a".into()], true), "Pausing p/a…");
        assert_eq!(
            doing(&["p/a".into(), "p/b".into()], false),
            "Resuming 2 agents…"
        );
    }
}
