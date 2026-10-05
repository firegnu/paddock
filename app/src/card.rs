//! What the Agents panel shows for each agent, worked out apart from drawing so it can be tested.
//! The rules (status dots and their animation, the activity line, ages, the Git line and where its
//! changes go, how directories shorten) follow Saddle's Agents panel in `src/ui.rs` at commit
//! `df1c727`; paddock draws them with native widgets instead of terminal cells.
use crate::{
    agents::{Panel, Status, group},
    corral::{Agent, Effort},
    git::{Head, Summary},
    preset::{Color, Theme},
};
use unicode_width::UnicodeWidthStr;

/// A Saddle interface colour, as `Theme::fg/bg` take it.
pub type Pick = fn(&Theme) -> Color;

/// Status dot, state label and colour.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub dot: &'static str,
    pub label: &'static str,
    pub color: Pick,
}

/// Working agents turn their dot every 360 ms, as in Saddle.
pub fn look(status: Status, now: f64) -> Look {
    let (dot, label, color): (_, _, Pick) = match status {
        Status::Waiting => ("?", "waiting", |t| t.agents_yellow),
        Status::Error => ("!", "error", |t| t.agents_red),
        Status::Stalled => ("▲", "stalled", |t| t.agent_stalled),
        Status::Working => (
            ["◐", "◓", "◑", "◒"][(now * 1000.0 / 360.0) as usize % 4],
            "working",
            |t| t.agents_blue,
        ),
        Status::Starting => ("◌", "starting", |t| t.agent_starting),
        Status::Unknown => ("·", "unknown", |t| t.agents_dim),
        Status::Idle => ("○", "idle", |t| t.agents_green),
        Status::Exited => ("✕", "exited", |t| t.agents_faint),
    };
    Look { dot, label, color }
}

/// The braille spinner in front of `working`, one frame every 120 ms.
pub fn spinner(now: f64) -> &'static str {
    ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"][(now * 1000.0 / 120.0) as usize % 8]
}

/// Which agent program runs it: a short mark and the name, in the program's colour.
#[derive(Clone, Debug)]
pub struct Brand {
    pub mark: Option<&'static str>,
    pub kind: String,
    pub color: Pick,
}

pub fn brand(kind: &str) -> Brand {
    let (mark, color): (_, Pick) = match kind.to_ascii_lowercase().as_str() {
        "claude" => (Some("✳"), |t| t.claude),
        "codex" => (Some(">_"), |t| t.codex),
        "pi" => (Some("π"), |t| t.pi),
        "omp" => (Some("π"), |t| t.omp),
        _ => (None, |t| t.agents_dim),
    };
    Brand {
        mark,
        kind: kind.to_owned(),
        color,
    }
}

/// How many of the three effort bars are lit, and their colour; Saddle's tiers.
pub fn effort(effort: Effort) -> (usize, Pick) {
    match effort {
        Effort::Medium => (1, |t| t.agent_idle),
        Effort::High => (2, |t| t.agent_working),
        Effort::Xhigh => (3, |t| t.agent_starting),
    }
}

/// One line of what the agent is doing or needs: `DOING Bash · 4m`, `ASK …`, `ERR …`.
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub label: &'static str,
    pub text: String,
    pub duration: Option<String>,
}

/// From public fields only. Waiting has no public summary of the question, so it says so plainly.
fn activity(a: &Agent, status: Status, time: &str) -> Vec<Activity> {
    let mut lines = Vec::new();
    let line = |label, text: &str, timed: bool| Activity {
        label,
        text: text.to_owned(),
        duration: timed.then(|| time.to_owned()),
    };
    match status {
        Status::Waiting => lines.push(line("ASK", "waiting for input", true)),
        Status::Working | Status::Stalled => lines.push(line(
            "DOING",
            a.last_tool.as_deref().unwrap_or("thinking"),
            true,
        )),
        _ => {}
    }
    if let Some(error) = &a.error {
        lines.push(line("ERR", error, false));
    }
    if a.incompatible {
        let text = format!("incompatible protocol {}", a.proto.unwrap_or(0));
        lines.push(line("ERR", &text, false));
    }
    lines
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

/// The Git line: the branch (left out when it repeats the agent's name), commits beyond its base,
/// and uncommitted changes, which move to a line of their own rather than squeezing the branch.
#[derive(Clone, Debug, PartialEq)]
pub enum GitLine {
    Loading,
    Unavailable,
    Known {
        head: String,
        /// Commits on HEAD beyond the base, and the base's name; `None` when Git couldn't tell.
        ahead: Option<(u64, String)>,
        /// `+added`, `-deleted`, binary files; `None` when Git couldn't count.
        changes: Option<(u64, u64, u64)>,
        untracked: Option<u64>,
        /// The changes don't fit beside the branch within the panel's width.
        changes_below: bool,
    },
}

impl GitLine {
    /// `columns` is the room for the line, in monospace cells.
    pub fn new(git: Option<&Option<Summary>>, name: &str, columns: usize) -> Self {
        let Some(git) = git else {
            return GitLine::Loading;
        };
        let Some(s) = git else {
            return GitLine::Unavailable;
        };
        let head = match &s.head {
            Head::Branch(branch) if branch == name => String::new(),
            Head::Branch(branch) => branch.clone(),
            Head::Detached => "HEAD detached".into(),
            Head::Unknown => "—".into(),
        };
        let changes = s.changes.as_ref().map(|c| (c.added, c.deleted, c.binary));
        let mut line = GitLine::Known {
            head,
            ahead: s.ahead.clone(),
            changes,
            untracked: s.untracked,
            changes_below: false,
        };
        let (left, right) = (line.left_text(), line.changes_text());
        if let GitLine::Known { changes_below, .. } = &mut line {
            *changes_below = left.width() + 1 + right.width() > columns;
        }
        line
    }

    /// The text left of the changes, as it is drawn: `⎇ branch ↑n base`.
    pub fn left_text(&self) -> String {
        match self {
            GitLine::Loading => "git …".into(),
            GitLine::Unavailable => "git unavailable".into(),
            GitLine::Known { head, ahead, .. } => {
                let mut text = "⎇".to_owned();
                if !head.is_empty() {
                    text += &format!(" {head}");
                }
                match ahead {
                    Some((count, base)) => text += &format!(" ↑{count} {base}"),
                    None => text += " ↑—",
                }
                text
            }
        }
    }

    /// The changes as drawn: `+a -d [n binary] ?u`.
    pub fn changes_text(&self) -> String {
        let GitLine::Known {
            changes, untracked, ..
        } = self
        else {
            return String::new();
        };
        let mut text = match changes {
            Some((added, deleted, 0)) => format!("+{added} -{deleted}"),
            Some((added, deleted, binary)) => format!("+{added} -{deleted} {binary} binary"),
            None => "+— -—".into(),
        };
        text += &format!(" ?{}", untracked.map_or("—".into(), |n| n.to_string()));
        text
    }
}

/// The full directory; when its last level repeats the group, only the parent's last level.
/// Anything wider than `width` cells loses leading levels, then leading characters, keeping the
/// end. Saddle's `agent_path`.
pub fn agent_path(path: &str, prefix: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    let trailing = if !prefix.is_empty() && parts.last() == Some(&prefix.trim_end_matches('/')) {
        parts.pop();
        "/"
    } else {
        ""
    };
    let mut start = if trailing.is_empty() {
        0
    } else {
        parts.len().saturating_sub(1)
    };
    let text = |start: usize| {
        let joined = parts[start..].join("/");
        if start > 0 {
            format!("…/{joined}{trailing}")
        } else if path.starts_with('/') {
            format!("/{joined}{trailing}")
        } else {
            format!("{joined}{trailing}")
        }
    };
    if parts.is_empty() {
        return path.to_owned();
    }
    while text(start).width() > width && start + 1 < parts.len() {
        start += 1;
    }
    let result = text(start);
    if result.width() <= width {
        return result;
    }
    let mut kept = String::new();
    let mut used = 1;
    for c in result.chars().rev() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        kept.insert(0, c);
        used += w;
    }
    format!("…{kept}")
}

/// Everything the panel shows for one agent.
#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    /// The name without its group prefix.
    pub short: String,
    pub status: Status,
    pub look: Look,
    /// Shown in this window's pane.
    pub here: bool,
    /// Finished a turn this window hasn't shown yet.
    pub unread: bool,
    /// Every line, not just the first: unfolded, or the one shown here.
    pub expanded: bool,
    pub brand: Option<Brand>,
    pub effort: Option<Effort>,
    pub time: String,
    pub time_color: Pick,
    pub title: Option<String>,
    pub activity: Vec<Activity>,
    /// `None` when corral reports no directory.
    pub git: Option<GitLine>,
    pub path: String,
    pub instance: Option<String>,
    pub cwd: Option<String>,
    pub attached: usize,
    pub via: String,
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

/// The panel's lines. `here` is the agent this window shows; `columns` is the room for the
/// detail lines, in monospace cells.
pub fn lines(
    panel: &Panel,
    error: Option<&str>,
    here: Option<&str>,
    columns: usize,
    now: f64,
) -> Vec<Line> {
    let mut lines: Vec<Line> = error
        .map(|e| Line::Error(e.to_owned()))
        .into_iter()
        .collect();
    let ordered = panel.ordered(now);
    let folded = panel.folded();
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
            panel, a, prefix, here, folded, columns, now,
        ))));
    }
    lines
}

fn card(
    panel: &Panel,
    a: &Agent,
    prefix: &str,
    here: Option<&str>,
    folded: bool,
    columns: usize,
    now: f64,
) -> Card {
    let status = panel.status(a, now);
    let short = a.name.strip_prefix(prefix).unwrap_or(&a.name).to_owned();
    let is_here = here == Some(a.name.as_str());
    let origin = match status {
        Status::Working | Status::Stalled if a.state.as_deref() == Some("working") => {
            a.turn_started
        }
        Status::Idle | Status::Waiting => a.state_started,
        _ => None,
    };
    let time = short_time(origin.map(|v| now - v));
    let time_color: Pick = if is_here {
        |t| t.agents_green
    } else if status == Status::Working {
        |t| t.agents_blue
    } else if status == Status::Exited {
        |t| t.agents_faint
    } else {
        |t| t.agents_text
    };
    let title = a
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty() && *title != prefix.trim_end_matches('/'))
        .map(str::to_owned);
    Card {
        name: a.name.clone(),
        look: look(status, now),
        status,
        here: is_here,
        unread: !is_here && panel.unread.contains(&a.name),
        expanded: !folded || is_here,
        brand: a.kind.as_deref().map(brand),
        effort: a.effort(),
        activity: activity(a, status, &time),
        time,
        time_color,
        title,
        git: a
            .cwd
            .as_ref()
            .map(|cwd| GitLine::new(panel.git.get(cwd), &short, columns)),
        path: agent_path(a.cwd.as_deref().unwrap_or("—"), prefix, columns),
        short,
        instance: a.instance.clone(),
        cwd: a.cwd.clone(),
        attached: a.attached,
        via: a.last_input_source.clone().unwrap_or_else(|| "—".into()),
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

    #[test]
    fn groups_carry_their_counts_and_names_lose_the_prefix() {
        let mut panel = Panel::default();
        panel.absorb(
            vec![
                agent("p/a", "idle"),
                agent("p/b", "idle"),
                agent("q/c", "idle"),
                agent("solo", "idle"),
            ],
            None,
            100.0,
        );
        let lines = lines(&panel, None, None, 40, 100.0);
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

    #[test]
    fn lines_show_only_when_they_have_something_to_say() {
        let working = Agent {
            title: Some("p".into()),
            last_tool: Some("Bash".into()),
            turn_started: Some(40.0),
            kind: Some("codex".into()),
            ..agent("p/a", "working")
        };
        let idle = Agent {
            title: Some("Handoff memory".into()),
            state_started: Some(100.0 - 7200.0),
            ..agent("p/b", "idle")
        };
        let mut panel = Panel::default();
        panel.absorb(vec![working, idle], None, 100.0);
        let lines = lines(&panel, None, None, 40, 100.0);
        let cards = cards(&lines);
        let (a, b) = (cards[0], cards[1]);
        // A title that only repeats the group is not shown.
        assert_eq!(a.title, None);
        assert_eq!(
            a.activity,
            [Activity {
                label: "DOING",
                text: "Bash".into(),
                duration: Some("1m".into()),
            }]
        );
        assert_eq!(a.time, "1m");
        assert_eq!(a.brand.as_ref().unwrap().mark, Some(">_"));
        assert_eq!(b.title.as_deref(), Some("Handoff memory"));
        assert!(b.activity.is_empty());
        assert_eq!(b.time, "2.0h");
        assert_eq!(b.look.label, "idle");
    }

    #[test]
    fn waiting_and_errors_say_so() {
        let waiting = agent("p/w", "blocked");
        let broken = Agent {
            error: Some("status failed".into()),
            ..agent("p/x", "idle")
        };
        let mut panel = Panel::default();
        panel.absorb(vec![waiting, broken], None, 100.0);
        let lines = lines(&panel, None, None, 40, 100.0);
        let labels: Vec<Vec<_>> = cards(&lines)
            .iter()
            .map(|c| c.activity.iter().map(|a| a.label).collect())
            .collect();
        assert_eq!(labels, [vec!["ASK"], vec!["ERR"]]);
    }

    #[test]
    fn folding_keeps_the_agent_shown_here_expanded() {
        let mut panel = Panel::default();
        panel.absorb(
            (0..6).map(|i| agent(&format!("p/{i}"), "idle")).collect(),
            None,
            100.0,
        );
        assert!(panel.folded(), "more than five agents fold by default");
        let lines = lines(&panel, None, Some("p/3"), 40, 100.0);
        let expanded: Vec<_> = cards(&lines)
            .iter()
            .filter(|c| c.expanded)
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(expanded, ["p/3"]);
        panel.toggle_fold();
        let lines = super::lines(&panel, None, Some("p/3"), 40, 100.0);
        assert!(cards(&lines).iter().all(|c| c.expanded));
    }

    #[test]
    fn sorting_switches_between_status_and_name() {
        let mut panel = Panel::default();
        panel.absorb(
            vec![
                agent("p/a", "idle"),
                agent("p/b", "working"),
                agent("p/c", "blocked"),
            ],
            None,
            100.0,
        );
        let names = |panel: &Panel| -> Vec<String> {
            cards(&lines(panel, None, None, 40, 100.0))
                .iter()
                .map(|c| c.short.clone())
                .collect()
        };
        assert_eq!(names(&panel), ["c", "b", "a"]);
        panel.by_name = true;
        assert_eq!(names(&panel), ["a", "b", "c"]);
    }

    #[test]
    fn here_marks_time_and_unread() {
        let mut panel = Panel::default();
        let a = Agent {
            state_started: Some(90.0),
            ..agent("p/a", "idle")
        };
        panel.absorb(vec![a.clone(), agent("p/b", "idle")], None, 100.0);
        let lines = lines(&panel, None, Some("p/a"), 40, 100.0);
        let card = cards(&lines)[0];
        assert!(card.here);
        assert_eq!(card.time, "10s");
        assert_eq!(
            (card.time_color)(&Theme::default()),
            Theme::default().agents_green
        );
    }

    fn summary(added: u64, deleted: u64) -> Option<Summary> {
        Some(Summary {
            head: Head::Branch("p2d-render".into()),
            ahead: Some((0, "main".into())),
            changes: Some(Changes {
                added,
                deleted,
                binary: 0,
            }),
            untracked: Some(3),
        })
    }

    #[test]
    fn git_changes_move_below_rather_than_squeeze_the_branch() {
        let line = GitLine::new(Some(&summary(25, 0)), "dev-render", 44);
        assert_eq!(line.left_text(), "⎇ p2d-render ↑0 main");
        assert_eq!(line.changes_text(), "+25 -0 ?3");
        assert!(matches!(
            line,
            GitLine::Known {
                changes_below: false,
                ..
            }
        ));
        let long = GitLine::new(Some(&summary(12847, 3291)), "dev-render", 30);
        assert!(matches!(
            long,
            GitLine::Known {
                changes_below: true,
                ..
            }
        ));
        // The branch is left out when it repeats the agent's name.
        let same = GitLine::new(Some(&summary(1, 0)), "p2d-render", 44);
        assert_eq!(same.left_text(), "⎇ ↑0 main");
        assert_eq!(GitLine::new(None, "x", 44), GitLine::Loading);
        assert_eq!(GitLine::new(Some(&None), "x", 44), GitLine::Unavailable);
    }

    #[test]
    fn paths_keep_their_end() {
        let path = "/Users/me/Developer/personal_projs/cairn-worktrees/p2d-render";
        assert_eq!(agent_path(path, "cairn/", 80), path);
        assert_eq!(
            agent_path(path, "cairn/", 30),
            "…/cairn-worktrees/p2d-render"
        );
        // A directory named after the group shows its parent's last level.
        assert_eq!(
            agent_path("/Users/me/Developer/personal_projs/cairn", "cairn/", 80),
            "…/personal_projs/"
        );
        assert_eq!(agent_path(path, "cairn/", 8).width(), 8);
    }

    #[test]
    fn effort_tiers_light_more_bars() {
        assert_eq!(effort(Effort::Medium).0, 1);
        assert_eq!(effort(Effort::High).0, 2);
        assert_eq!(effort(Effort::Xhigh).0, 3);
    }
}
