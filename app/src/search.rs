//! The command palette's rules, apart from drawing (DESIGN §13, P5-3). What is typed first picks
//! the mode: plainly it finds agents (by name or project, as Saddle's Search in `src/search.rs` at
//! commit `df1c727`), tabs, Settings pages and commands; after `>` only commands; after `/` text in
//! the open panes, found by `find.rs`. The rows come in groups, the first selected.
use crate::{
    agents::Panel,
    card::{self, Pick},
    corral::Agent,
    layout::PaneId,
    menu,
    settings::Page,
    window::short_dir,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Agents, tabs, Settings pages and commands.
    Everything,
    /// `>`: every command in the menu bar.
    Commands,
    /// `/`: text in the output the open panes keep.
    Text,
}

impl Mode {
    /// The label before the field, saying what it searches; none for everything.
    pub fn chip(self) -> Option<&'static str> {
        match self {
            Mode::Everything => None,
            Mode::Commands => Some("Commands"),
            Mode::Text => Some("Text in all panes"),
        }
    }

    /// What Enter does, for the hint row.
    pub fn enter(self) -> &'static str {
        match self {
            Mode::Everything => "Open",
            Mode::Commands => "Run",
            Mode::Text => "Jump to match",
        }
    }
}

/// The mode `query` asks for and what to look for: the words, or after `/` the text as typed but
/// for the blanks right after the `/`.
pub fn mode(query: &str) -> (Mode, &str) {
    if let Some(rest) = query.strip_prefix('>') {
        (Mode::Commands, rest.trim())
    } else if let Some(rest) = query.strip_prefix('/') {
        (Mode::Text, rest.trim_start())
    } else {
        (Mode::Everything, query.trim())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Agent(String),
    /// The tab at this index.
    Tab(usize),
    Settings(Page),
    /// A menu bar command, by its action's name.
    Command(&'static str),
    /// A pane's match, by its place in that pane's list.
    Match {
        pane: PaneId,
        index: usize,
    },
}

/// What a row shows before its title.
#[derive(Clone, Copy, Debug)]
pub enum Lead {
    /// A status dot in this colour; a working agent's breathes.
    Dot {
        color: Pick,
        breathing: bool,
    },
    Settings,
    Command,
    /// Nothing: a line of terminal text.
    Nothing,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub target: Target,
    pub lead: Lead,
    pub title: String,
    pub detail: String,
    /// The shortcut, as keycaps.
    pub keys: Vec<String>,
    /// For a line of terminal text: the match in the title.
    pub hit: Option<Range<usize>>,
}

#[derive(Clone, Debug)]
pub struct Group {
    /// In capitals, as shown.
    pub label: String,
    /// Faint after the label: a count, or a hint.
    pub note: String,
    pub rows: Vec<Row>,
}

/// The commands listed before anything is typed, by action.
const COMMON: [&str; 3] = ["paddock::NewAgent", "paddock::NewTab", "paddock::NewShell"];

/// Matches a pane lists at most, newest first; its note says when it has more.
pub const PER_PANE: usize = 20;

/// Room kept before a match in a long line; the rest of its start gives way to `…`.
const BEFORE: usize = 24;

/// The agents by name: a status dot, the name, the status and the short directory.
pub fn agents(agents: &[Agent], now: f64, home: Option<&str>) -> Vec<Row> {
    let mut agents: Vec<&Agent> = agents.iter().collect();
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    agents
        .into_iter()
        .map(|agent| {
            let look = card::look(Panel::default().shown(agent, now));
            let detail = match agent.cwd.as_deref().filter(|cwd| !cwd.is_empty()) {
                Some(cwd) => format!("{} · {}", look.label, short_dir(cwd, home)),
                None => look.label.to_owned(),
            };
            Row {
                target: Target::Agent(agent.name.clone()),
                lead: Lead::Dot {
                    color: look.color,
                    breathing: look.breathing,
                },
                title: agent.name.clone(),
                detail,
                keys: Vec::new(),
                hit: None,
            }
        })
        .collect()
}

/// The Settings pages, saying what each holds.
pub fn settings() -> Vec<Row> {
    Page::ALL
        .into_iter()
        .map(|page| {
            let holds = match page {
                Page::General => "fonts, sidebar, pet",
                Page::Appearance => "theme and colors",
                Page::Agents => "refresh interval, corral command",
                Page::Diagnostics => "commands and paths",
            };
            Row {
                target: Target::Settings(page),
                lead: Lead::Settings,
                title: page.label().to_owned(),
                detail: format!("Settings · {holds}"),
                keys: Vec::new(),
                hit: None,
            }
        })
        .collect()
}

/// The menu bar's commands with their shortcuts.
pub fn commands(commands: &[menu::Command]) -> Vec<Row> {
    commands
        .iter()
        .map(|command| Row {
            target: Target::Command(command.action.name()),
            lead: Lead::Command,
            title: command.title.clone(),
            detail: String::new(),
            keys: command.keys.clone(),
            hit: None,
        })
        .collect()
}

/// A tab's detail: its number, and the agents it shows in full.
pub fn tab_detail(index: usize, agents: &[String]) -> String {
    let number = format!("Tab {}", index + 1);
    if agents.is_empty() {
        number
    } else {
        format!("{number} · {}", agents.join(", "))
    }
}

/// The ⌘ number that goes to the tab at `index` of `count`: ⌘1…⌘8 their tab, ⌘9 the last.
pub fn tab_shortcut(index: usize, count: usize) -> Option<usize> {
    if index < 8 {
        Some(index + 1)
    } else if index + 1 == count {
        Some(9)
    } else {
        None
    }
}

/// A matched line as its row shows it: leading blanks dropped, and the start of a long line cut
/// to `…` so the match stays in view.
pub fn line(pane: PaneId, index: usize, line: &str, hit: Range<usize>) -> Row {
    let start = (line.len() - line.trim_start().len()).min(hit.start);
    let before = &line[start..hit.start];
    let chars = before.chars().count();
    let (title, hit) = if chars <= BEFORE {
        (line[start..].to_owned(), hit.start - start..hit.end - start)
    } else {
        let cut = start
            + before
                .char_indices()
                .nth(chars - BEFORE)
                .map_or(0, |(i, _)| i);
        let shift = '…'.len_utf8();
        (
            format!("…{}", &line[cut..]),
            hit.start - cut + shift..hit.end - cut + shift,
        )
    };
    Row {
        target: Target::Match { pane, index },
        lead: Lead::Nothing,
        title,
        detail: String::new(),
        keys: Vec::new(),
        hit: Some(hit),
    }
}

/// Every word of `needle` in the row's title or detail, ignoring case.
fn matches(row: &Row, needle: &str) -> bool {
    let text = format!("{} {}", row.title, row.detail).to_lowercase();
    needle
        .split_whitespace()
        .all(|word| text.contains(&word.to_lowercase()))
}

fn filter(rows: &[Row], needle: &str) -> Vec<Row> {
    rows.iter()
        .filter(|row| matches(row, needle))
        .cloned()
        .collect()
}

/// The groups with rows, in order.
fn groups(groups: impl IntoIterator<Item = (&'static str, String, Vec<Row>)>) -> Vec<Group> {
    groups
        .into_iter()
        .filter(|(_, _, rows)| !rows.is_empty())
        .map(|(label, note, rows)| Group {
            label: label.to_owned(),
            note,
            rows,
        })
        .collect()
}

/// Typed plainly: agents, tabs, Settings pages and commands with every word of `needle` in their
/// title or detail; before anything is typed, only a few common commands.
pub fn everything(
    agents: &[Row],
    tabs: &[Row],
    settings: &[Row],
    commands: &[Row],
    needle: &str,
) -> Vec<Group> {
    let agents = filter(agents, needle);
    let (commands, hint) = if needle.is_empty() {
        let common = COMMON.iter().filter_map(|name| {
            commands
                .iter()
                .find(|row| row.target == Target::Command(name))
        });
        (common.cloned().collect(), "type > for all")
    } else {
        (filter(commands, needle), "")
    };
    groups([
        ("AGENTS", agents.len().to_string(), agents),
        ("TABS", String::new(), filter(tabs, needle)),
        ("SETTINGS", String::new(), filter(settings, needle)),
        ("COMMANDS", hint.to_owned(), commands),
    ])
}

/// After `>`: the commands with every word of `needle`.
pub fn only_commands(commands: &[Row], needle: &str) -> Vec<Group> {
    let commands = filter(commands, needle);
    groups([("COMMANDS", commands.len().to_string(), commands)])
}

/// A pane's matches under its name, saying how many; `20+` when it has more than it lists.
pub fn pane(name: &str, rows: Vec<Row>, more: bool) -> Option<Group> {
    let note = match (rows.len(), more) {
        (0, _) => return None,
        (count, true) => format!("{count}+ matches"),
        (1, false) => "1 match".to_owned(),
        (count, false) => format!("{count} matches"),
    };
    Some(Group {
        label: name.to_uppercase(),
        note,
        rows,
    })
}

/// What the list says when nothing matches what was typed.
pub fn empty(groups: &[Group], needle: &str) -> Option<String> {
    (groups.is_empty() && !needle.is_empty()).then(|| format!("No results for “{needle}”"))
}

/// How many rows the groups hold.
pub fn count(groups: &[Group]) -> usize {
    groups.iter().map(|group| group.rows.len()).sum()
}

/// The selected row: the one at `index` counting every group's rows in order, or the last when
/// there are fewer now.
pub fn selected(groups: &[Group], index: usize) -> Option<(usize, &Row)> {
    let index = index.min(count(groups).checked_sub(1)?);
    let row = groups.iter().flat_map(|group| &group.rows).nth(index)?;
    Some((index, row))
}

/// The selection moved by `by` rows, stopping at either end.
pub fn step(index: usize, count: usize, by: isize) -> usize {
    let Some(last) = count.checked_sub(1) else {
        return 0;
    };
    index.min(last).saturating_add_signed(by).min(last)
}

/// Where the row at `index` is among the list's children, headers included; a group's first row
/// gives its header, so scrolling to it shows both.
pub fn child(groups: &[Group], index: usize) -> usize {
    let (mut child, mut rows) = (0, 0);
    for group in groups {
        if index < rows + group.rows.len() {
            return match index - rows {
                0 => child,
                within => child + 1 + within,
            };
        }
        child += 1 + group.rows.len();
        rows += group.rows.len();
    }
    child
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, cwd: &str) -> Agent {
        Agent {
            name: name.into(),
            cwd: Some(cwd.into()),
            ..Default::default()
        }
    }

    fn titles(groups: &[Group], label: &str) -> Vec<String> {
        groups
            .iter()
            .find(|g| g.label == label)
            .map(|g| g.rows.iter().map(|r| r.title.clone()).collect())
            .unwrap_or_default()
    }

    fn labels(groups: &[Group]) -> Vec<&str> {
        groups.iter().map(|g| g.label.as_str()).collect()
    }

    fn rows(titles: &[&str]) -> Vec<Row> {
        titles
            .iter()
            .enumerate()
            .map(|(index, title)| Row {
                target: Target::Tab(index),
                lead: Lead::Nothing,
                title: (*title).into(),
                detail: String::new(),
                keys: Vec::new(),
                hit: None,
            })
            .collect()
    }

    #[test]
    fn the_first_character_picks_the_mode() {
        assert_eq!(mode(""), (Mode::Everything, ""));
        assert_eq!(mode("  web main "), (Mode::Everything, "web main"));
        assert_eq!(mode(">"), (Mode::Commands, ""));
        assert_eq!(mode("> split "), (Mode::Commands, "split"));
        // Text keeps what it is given, but the blanks right after the `/`.
        assert_eq!(mode("/ merge"), (Mode::Text, "merge"));
        assert_eq!(mode("/a b "), (Mode::Text, "a b "));
        assert_eq!(mode("/"), (Mode::Text, ""));
        // Only the first character counts.
        assert_eq!(mode("a/b"), (Mode::Everything, "a/b"));
        assert_eq!(Mode::Everything.chip(), None);
        assert_eq!(Mode::Commands.chip(), Some("Commands"));
        assert_eq!(Mode::Text.chip(), Some("Text in all panes"));
        assert_eq!(Mode::Everything.enter(), "Open");
        assert_eq!(Mode::Commands.enter(), "Run");
        assert_eq!(Mode::Text.enter(), "Jump to match");
    }

    #[test]
    fn agents_by_name_or_project_ignoring_case() {
        let agents = agents(
            &[
                agent("web/main", "/code/Shop"),
                agent("api/review", "/code/api/"),
                agent("api/main", "/code/api"),
            ],
            0.0,
            None,
        );
        let find = |needle| titles(&everything(&agents, &[], &[], &[], needle), "AGENTS");
        assert_eq!(find("SHOP"), ["web/main"]);
        assert_eq!(find(mode(" api ").1), ["api/main", "api/review"]);
        assert_eq!(find("review"), ["api/review"]);
        // Every word, in any order.
        assert_eq!(find("main api"), ["api/main"]);
        assert_eq!(agents[2].target, Target::Agent("web/main".into()));
        assert!(
            agents[2].detail.ends_with("· /…/Shop"),
            "{}",
            agents[2].detail
        );
        assert!(matches!(agents[0].lead, Lead::Dot { .. }));
        let groups = everything(&agents, &[], &[], &[], "");
        assert_eq!(groups[0].label, "AGENTS");
        assert_eq!(groups[0].note, "3");
    }

    #[test]
    fn a_paused_agent_says_paused_and_does_not_breathe() {
        let mut paused = agent("paddock/main", "/Users/me/code/paddock");
        paused.state = Some("working".into());
        paused.paused = true;
        let rows = agents(&[paused], 0.0, Some("/Users/me"));
        assert_eq!(rows[0].detail, "Paused · ~/…/paddock");
        assert!(matches!(
            rows[0].lead,
            Lead::Dot {
                breathing: false,
                ..
            }
        ));
    }

    #[test]
    fn agents_say_their_status_and_short_directory() {
        let mut working = agent("paddock/main", "/Users/me/code/paddock");
        working.state = Some("working".into());
        let mut gone = agent("ranch/test", "");
        gone.state = Some("exited".into());
        let rows = agents(&[working, gone], 0.0, Some("/Users/me"));
        assert_eq!(rows[0].title, "paddock/main");
        assert_eq!(rows[0].detail, "Working · ~/…/paddock");
        assert!(matches!(
            rows[0].lead,
            Lead::Dot {
                breathing: true,
                ..
            }
        ));
        assert_eq!(rows[1].detail, "Exited");
        assert!(matches!(
            rows[1].lead,
            Lead::Dot {
                breathing: false,
                ..
            }
        ));
    }

    #[test]
    fn settings_pages_and_commands_follow_the_agents_and_tabs() {
        let agents = agents(&[agent("colors/main", "/code/x")], 0.0, None);
        let tabs = rows(&["zsh · colors", "main"]);
        let settings = settings();
        let commands = commands(&menu::commands());
        let found = everything(&agents, &tabs, &settings, &commands, "colors");
        assert_eq!(labels(&found), ["AGENTS", "TABS", "SETTINGS"]);
        assert_eq!(titles(&found, "SETTINGS"), ["Appearance"]);
        assert_eq!(found[2].rows[0].target, Target::Settings(Page::Appearance));
        assert_eq!(found[2].rows[0].detail, "Settings · theme and colors");
        // Settings pages match on what they hold, and all of them on "settings".
        assert_eq!(
            titles(&everything(&[], &[], &settings, &[], "font"), "SETTINGS"),
            ["General"]
        );
        assert_eq!(
            titles(&everything(&[], &[], &settings, &[], "refresh"), "SETTINGS"),
            ["Agents"]
        );
        let all = everything(&[], &[], &settings, &commands, "settings");
        assert_eq!(titles(&all, "SETTINGS").len(), 4);
        assert_eq!(titles(&all, "COMMANDS"), ["Settings…"]);
        // Nothing typed: everything, but only a few common commands, saying how to see all.
        let start = everything(&agents, &tabs, &settings, &commands, "");
        assert_eq!(labels(&start), ["AGENTS", "TABS", "SETTINGS", "COMMANDS"]);
        assert_eq!(
            titles(&start, "COMMANDS"),
            ["New Agent…", "New Tab…", "New Shell"]
        );
        assert_eq!(start[3].note, "type > for all");
        assert_eq!(start[1].note, "");
        // Typed: every command that matches, without the hint.
        let split = everything(&agents, &tabs, &settings, &commands, "split");
        assert_eq!(labels(&split), ["COMMANDS"]);
        assert_eq!(split[0].rows.len(), 4);
        assert_eq!(split[0].note, "");
    }

    #[test]
    fn commands_mode_lists_every_command_with_its_keys() {
        let commands = commands(&menu::commands());
        let all = only_commands(&commands, "");
        assert_eq!(labels(&all), ["COMMANDS"]);
        assert_eq!(all[0].rows.len(), menu::commands().len());
        assert_eq!(all[0].note, all[0].rows.len().to_string());
        let zoom = only_commands(&commands, "zoom pane");
        assert_eq!(titles(&zoom, "COMMANDS"), ["Zoom Pane"]);
        assert_eq!(zoom[0].rows[0].keys, ["⌘", "⇧", "↵"]);
        assert_eq!(zoom[0].rows[0].target, Target::Command("paddock::ZoomPane"));
        assert!(matches!(zoom[0].rows[0].lead, Lead::Command));
        assert!(only_commands(&commands, "nothing like this").is_empty());
    }

    #[test]
    fn tabs_say_their_number_and_agents_and_their_shortcut() {
        assert_eq!(tab_detail(0, &[]), "Tab 1");
        assert_eq!(
            tab_detail(1, &["paddock/main".into(), "paddock/dev".into()]),
            "Tab 2 · paddock/main, paddock/dev"
        );
        assert_eq!(tab_shortcut(0, 3), Some(1));
        assert_eq!(tab_shortcut(2, 3), Some(3));
        assert_eq!(tab_shortcut(7, 12), Some(8));
        // Past the eighth, only the last has one: ⌘9.
        assert_eq!(tab_shortcut(8, 12), None);
        assert_eq!(tab_shortcut(11, 12), Some(9));
        assert_eq!(tab_shortcut(8, 9), Some(9));
    }

    #[test]
    fn matched_lines_keep_the_match_in_view() {
        let prompt = "  ❯ git merge --no-ff";
        let at = prompt.find("merge").unwrap();
        let row = line(7, 2, prompt, at..at + 5);
        assert_eq!(row.title, "❯ git merge --no-ff");
        assert_eq!(&row.title[row.hit.clone().unwrap()], "merge");
        assert_eq!(row.target, Target::Match { pane: 7, index: 2 });
        assert!(matches!(row.lead, Lead::Nothing));
        let long = format!("{}needle and more", "word ".repeat(20));
        let start = long.find("needle").unwrap();
        let row = line(1, 0, &long, start..start + 6);
        assert!(row.title.starts_with('…'));
        assert_eq!(&row.title[row.hit.clone().unwrap()], "needle");
        assert_eq!(
            row.title[..row.hit.clone().unwrap().start].chars().count(),
            BEFORE + 1
        );
        // Blanks inside the match stay.
        let row = line(1, 0, "   x", 0..4);
        assert_eq!(row.title, "   x");
        assert_eq!(row.hit, Some(0..4));
    }

    #[test]
    fn panes_say_how_many_matches_they_list() {
        let one = pane("zsh · paddock", rows(&["a"]), false).unwrap();
        assert_eq!(one.label, "ZSH · PADDOCK");
        assert_eq!(one.note, "1 match");
        assert_eq!(
            pane("x", rows(&["a", "b"]), false).unwrap().note,
            "2 matches"
        );
        let many = pane("paddock/main", rows(&["a"; PER_PANE]), true).unwrap();
        assert_eq!(many.note, "20+ matches");
        assert!(pane("x", Vec::new(), false).is_none());
    }

    #[test]
    fn nothing_found_says_so() {
        assert_eq!(
            empty(&[], "merge").as_deref(),
            Some("No results for “merge”")
        );
        // Nothing typed after `/` yet: nothing to say.
        assert_eq!(empty(&[], ""), None);
        let found = vec![pane("x", rows(&["a"]), false).unwrap()];
        assert_eq!(empty(&found, "a"), None);
    }

    #[test]
    fn the_selection_follows_the_rows() {
        let groups = vec![
            pane("one", rows(&["a", "b"]), false).unwrap(),
            pane("two", rows(&["c"]), false).unwrap(),
        ];
        assert_eq!(count(&groups), 3);
        // The first row is selected to begin with.
        assert_eq!(selected(&groups, 0).unwrap().1.title, "a");
        assert_eq!(selected(&groups, 2).unwrap().1.title, "c");
        // Fewer rows now: the last one stays selected.
        let (index, row) = selected(&groups, 5).unwrap();
        assert_eq!((index, row.title.as_str()), (2, "c"));
        let fewer = vec![pane("one", rows(&["a"]), false).unwrap()];
        assert_eq!(selected(&fewer, 2).unwrap().1.title, "a");
        assert!(selected(&[], 0).is_none());
        // ↑↓ stop at either end.
        assert_eq!(step(0, 3, 1), 1);
        assert_eq!(step(2, 3, 1), 2);
        assert_eq!(step(0, 3, -1), 0);
        assert_eq!(step(7, 3, -1), 1);
        assert_eq!(step(0, 0, 1), 0);
        // Headers count among the list's children; a group's first row scrolls to its header.
        assert_eq!(child(&groups, 0), 0);
        assert_eq!(child(&groups, 1), 2);
        assert_eq!(child(&groups, 2), 3);
    }
}
