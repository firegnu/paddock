//! Workspaces (P5-89): who is in which, one place each, the sidebar's lines with them, folding,
//! their file, their names, what the panel ticks, and finding agents by them.
mod common;
use paddock::{
    agents::Panel,
    card::{self, Line},
    corral::Agent,
    git::{Head, Summary},
    search,
    workspaces::{Listed, NameError, Picks, Workspace, Workspaces},
};
use std::path::{Path, PathBuf};

fn panel(entries: &[(&str, &str, &str)]) -> Panel {
    let mut panel = Panel::default();
    panel.absorb(
        entries
            .iter()
            .map(|(name, cwd, state)| Agent {
                name: (*name).into(),
                cwd: Some((*cwd).into()),
                state: Some((*state).into()),
                ..Default::default()
            })
            .collect(),
        None,
        100.0,
    );
    panel
}

/// Git has told that `cwd` is in the repository whose main worktree is `main`.
fn repository(panel: &mut Panel, cwd: &str, main: &str) {
    panel.absorb_git(vec![(
        cwd.into(),
        Some(Summary {
            main_repository: Some(main.into()),
            head: Head::Branch("main".into()),
            ahead: None,
            changes: None,
            untracked: None,
        }),
    )]);
}

/// A workspace named `name` with these whole repositories and single agents.
fn workspace(workspaces: &mut Workspaces, name: &str, repos: &[&str], agents: &[&str]) {
    let name = workspaces.create(name).unwrap();
    for repo in repos {
        workspaces.add_repo(&name, Path::new(repo));
    }
    for agent in agents {
        workspaces.add_agent(&name, agent);
    }
}

fn of<'a>(panel: &'a Panel, name: &str) -> Option<&'a str> {
    panel.workspace(panel.agents.iter().find(|a| a.name == name).unwrap())
}

/// Every line as one string, those of a workspace set in under its title: `= name count bar`
/// for a workspace, `# title count bar` for a group (`folded` and `showing` said when so), `-`
/// for a line that says there are no agents, a card by its agent's full name.
fn shown(panel: &Panel, selected: Option<&str>) -> Vec<String> {
    let marks = |mut text: String, folded: bool, showing: bool| {
        if folded {
            text += " folded";
        }
        if showing {
            text += " showing";
        }
        text
    };
    let inset = |workspace: &Option<String>| if workspace.is_some() { "  " } else { "" };
    card::lines(panel, None, selected, &[], None, 100.0)
        .into_iter()
        .map(|line| match line {
            Line::Error(text) => format!("! {text}"),
            Line::Workspace(tray) => marks(
                format!("= {} {} {:?}", tray.name, tray.count, tray.bar),
                tray.folded,
                tray.showing,
            ),
            Line::Group(heading) => marks(
                format!(
                    "{}# {} {} {:?}",
                    inset(&heading.workspace),
                    heading.title,
                    heading.count,
                    heading.bar
                ),
                heading.folded,
                heading.showing,
            ),
            Line::Vacant { repo, .. } => format!("  - {repo} No agents"),
            Line::Empty(_) => "  - No agents".to_owned(),
            Line::Agent(card) => format!("{}{}", inset(&card.workspace), card.name),
        })
        .collect()
}

#[test]
fn without_workspaces_or_folds_the_lines_are_what_they_were() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/trees/p5", "working"),
        ("ranch/main", "/w/ranch", "blocked"),
        ("solo", "/plain", "idle"),
    ]);
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/trees/p5", "/w/paddock");
    let lines = card::lines(&panel, None, Some("ranch/main"), &[], None, 100.0);
    assert_eq!(
        shown(&panel, Some("ranch/main")),
        [
            "# agents/ 1 [Idle]",
            "solo",
            "# paddock/ 2 [Working, Idle]",
            "paddock/dev",
            "paddock/main",
            "# ranch/ 1 [Waiting] showing",
            "ranch/main",
        ]
    );
    // Nothing of a workspace on any of them.
    for line in lines {
        match line {
            Line::Group(heading) => assert_eq!(heading.workspace, None),
            Line::Agent(card) => assert_eq!((card.workspace.clone(), card.alone), (None, false)),
            _ => panic!("no other lines"),
        }
    }
}

#[test]
fn an_agent_is_in_its_own_workspace_before_its_repositorys_and_a_prefix_follows_its_name() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/trees/p5", "idle"),
        ("paddock/test", "/plain", "idle"),
        ("ranch/main", "/w/ranch", "idle"),
        ("solo", "/plain", "idle"),
    ]);
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/trees/p5", "/w/paddock");
    repository(&mut panel, "/w/ranch", "/w/ranch");
    panel.absorb_git(vec![("/plain".into(), None)]);
    workspace(&mut panel.workspaces, "Tools", &["/w/paddock"], &[]);
    // The whole repository: its main worktree's agent and the one in a worktree; and the one
    // whose repository cannot be read, by its prefix, the repository's name.
    assert_eq!(of(&panel, "paddock/main"), Some("Tools"));
    assert_eq!(of(&panel, "paddock/dev"), Some("Tools"));
    assert_eq!(of(&panel, "paddock/test"), Some("Tools"));
    // In neither way: as before.
    assert_eq!(of(&panel, "ranch/main"), None);
    assert_eq!(of(&panel, "solo"), None);
    // Recorded by itself in another, it is there; the rest of its repository stays.
    workspace(&mut panel.workspaces, "Zeta", &[], &["paddock/dev"]);
    assert_eq!(of(&panel, "paddock/dev"), Some("Zeta"));
    assert_eq!(of(&panel, "paddock/main"), Some("Tools"));
    // Two workspaces with a repository of that name: the prefix follows neither.
    panel
        .workspaces
        .add_repo("Zeta", Path::new("/elsewhere/paddock"));
    assert_eq!(of(&panel, "paddock/test"), None);
    assert_eq!(of(&panel, "paddock/main"), Some("Tools"));
}

/// A workspace's whole repositories and single agents.
fn members(all: &Workspaces, name: &str) -> (Vec<PathBuf>, Vec<String>) {
    let workspace = all.get(name).unwrap();
    (workspace.repos.clone(), workspace.agents.clone())
}

#[test]
fn a_repository_and_an_agent_are_each_in_one_workspace_only() {
    let mut all = Workspaces::default();
    workspace(
        &mut all,
        "One",
        &["/w/paddock", "/w/ranch"],
        &["solo", "x/y"],
    );
    workspace(&mut all, "Two", &[], &[]);
    all.add_repo("Two", Path::new("/w/paddock"));
    all.add_agent("Two", "solo");
    assert_eq!(
        members(&all, "One"),
        (vec![PathBuf::from("/w/ranch")], vec!["x/y".to_owned()])
    );
    assert_eq!(
        members(&all, "Two"),
        (vec![PathBuf::from("/w/paddock")], vec!["solo".to_owned()])
    );
    assert_eq!(all.of_repo(Path::new("/w/paddock")), Some("Two"));
    assert_eq!(all.of_agent("solo"), Some("Two"));
    // A workspace that is not there (another window removed it) takes nothing from the others.
    all.add_repo("Gone", Path::new("/w/ranch"));
    all.add_agent("Gone", "x/y");
    all.set_members("Gone", &[PathBuf::from("/w/paddock")], &["solo".to_owned()]);
    assert_eq!(all.of_repo(Path::new("/w/ranch")), Some("One"));
    assert_eq!(all.of_agent("x/y"), Some("One"));
    assert_eq!(all.of_repo(Path::new("/w/paddock")), Some("Two"));
    assert_eq!(all.of_agent("solo"), Some("Two"));
    // Taken out, they are in none; the workspace gone, its members are in none.
    all.remove_repo(Path::new("/w/paddock"));
    all.remove_agent("solo");
    assert_eq!(members(&all, "Two"), (vec![], vec![]));
    all.remove("One");
    assert_eq!(all.of_repo(Path::new("/w/ranch")), None);
    assert_eq!(all.of_agent("x/y"), None);
}

#[test]
fn workspaces_come_first_by_name_with_their_groups_empty_repositories_and_split_ones() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/w/paddock", "working"),
        ("ranch/main", "/w/ranch", "blocked"),
        ("cairn/main", "/w/cairn", "idle"),
        ("jb/codex-1", "/w/jb", "idle"),
        ("jb/dev-1", "/w/jb", "working"),
        ("mesh/main", "/w/mesh", "idle"),
    ]);
    for repo in ["paddock", "ranch", "cairn", "jb", "mesh"] {
        let path = format!("/w/{repo}");
        repository(&mut panel, &path, &path);
    }
    workspace(&mut panel.workspaces, "Zoo", &[], &[]);
    workspace(&mut panel.workspaces, "fine-tuning", &[], &["jb/codex-1"]);
    workspace(
        &mut panel.workspaces,
        "Agent tools",
        &["/w/ranch", "/w/saddle", "/w/paddock", "/w/cairn"],
        &[],
    );
    assert_eq!(
        shown(&panel, Some("paddock/dev")),
        [
            // By name, whatever the case; in each, its groups by name, a repository with no
            // agent now among them.
            "= Agent tools 4 [Waiting, Working, Idle, Idle] showing",
            "  # cairn/ 1 [Idle]",
            "  cairn/main",
            "  # paddock/ 2 [Working, Idle] showing",
            "  paddock/dev",
            "  paddock/main",
            "  # ranch/ 1 [Waiting]",
            "  ranch/main",
            "  - saddle No agents",
            // One agent of jb taken by itself: jb is here and below, each with its own count.
            "= fine-tuning 1 [Idle]",
            "  # jb/ 1 [Idle]",
            "  jb/codex-1",
            "= Zoo 0 []",
            "  - No agents",
            "# jb/ 1 [Working]",
            "jb/dev-1",
            "# mesh/ 1 [Idle]",
            "mesh/main",
        ]
    );
    // What a heading's button moves: the whole repository, or the agents taken singly; a group
    // outside any, the repositories of its agents.
    let lines = card::lines(&panel, None, None, &[], None, 100.0);
    let heading = |workspace: Option<&str>, group: &str| {
        lines
            .iter()
            .find_map(|line| match line {
                Line::Group(heading)
                    if heading.workspace.as_deref() == workspace && heading.group == group =>
                {
                    Some((heading.repos.clone(), heading.singles.clone()))
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        heading(Some("Agent tools"), "paddock"),
        (vec![PathBuf::from("/w/paddock")], vec![])
    );
    assert_eq!(
        heading(Some("fine-tuning"), "jb"),
        (vec![], vec!["jb/codex-1".to_owned()])
    );
    assert_eq!(heading(None, "jb"), (vec![PathBuf::from("/w/jb")], vec![]));
    // The card says where its agent is, and whether by itself.
    let card = |name: &str| {
        lines
            .iter()
            .find_map(|line| match line {
                Line::Agent(card) if card.name == name => {
                    Some((card.workspace.clone(), card.alone))
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(card("jb/codex-1"), (Some("fine-tuning".to_owned()), true));
    assert_eq!(card("ranch/main"), (Some("Agent tools".to_owned()), false));
    assert_eq!(card("mesh/main"), (None, false));
}

#[test]
fn folded_groups_and_workspaces_keep_their_count_and_bar_and_are_passed_over() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/w/paddock", "working"),
        ("ranch/main", "/w/ranch", "blocked"),
        ("jb/main", "/w/jb", "idle"),
        ("mesh/main", "/w/mesh", "idle"),
    ]);
    for repo in ["paddock", "ranch", "jb", "mesh"] {
        let path = format!("/w/{repo}");
        repository(&mut panel, &path, &path);
    }
    workspace(
        &mut panel.workspaces,
        "Tools",
        &["/w/paddock", "/w/ranch"],
        &[],
    );
    workspace(&mut panel.workspaces, "Tuning", &["/w/jb"], &[]);
    // A group folded inside its workspace: the same name outside any would stay open.
    panel.folded.toggle_group(Some("Tools"), "paddock");
    assert_eq!(
        shown(&panel, Some("paddock/dev")),
        [
            "= Tools 3 [Waiting, Working, Idle] showing",
            "  # paddock/ 2 [Working, Idle] folded showing",
            "  # ranch/ 1 [Waiting]",
            "  ranch/main",
            "= Tuning 1 [Idle]",
            "  # jb/ 1 [Idle]",
            "  jb/main",
            "# mesh/ 1 [Idle]",
            "mesh/main",
        ]
    );
    // Up and down pass over what is folded.
    panel.select(Some("ranch/main".into()));
    panel.move_selection(-1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("ranch/main"));
    panel.folded.toggle_workspace("Tuning");
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("mesh/main"));
    // A folded workspace is its title alone, with the bar of all its agents.
    panel.folded.toggle_workspace("Tools");
    assert_eq!(
        shown(&panel, Some("paddock/dev")),
        [
            "= Tools 3 [Waiting, Working, Idle] folded showing",
            "= Tuning 1 [Idle] folded",
            "# mesh/ 1 [Idle]",
            "mesh/main",
        ]
    );
    // The strip still has every agent.
    let tiles = card::unfolded(&panel, None, None, &[], None, 100.0)
        .into_iter()
        .filter(|line| matches!(line, Line::Agent(_)))
        .count();
    assert_eq!(tiles, 5);
    // Renamed, a workspace stays folded, and its group in it; removed, its folds are forgotten.
    panel.folded.rename_workspace("Tools", "Kit");
    assert!(panel.folded.workspace("Kit") && panel.folded.group(Some("Kit"), "paddock"));
    assert!(!panel.folded.workspace("Tools"));
    panel.folded.forget_workspace("Kit");
    assert!(!panel.folded.workspace("Kit") && !panel.folded.group(Some("Kit"), "paddock"));
    assert!(panel.folded.workspace("Tuning"));
}

#[test]
fn a_folded_workspace_stacks_three_of_its_agents_cards_the_active_pane_s_in_front() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/w/paddock", "working"),
        ("ranch/main", "/w/ranch", "blocked"),
        ("ranch/dev", "/w/ranch", "idle"),
        ("jb/main", "/w/jb", "idle"),
        ("mesh/main", "/w/mesh", "idle"),
    ]);
    for repo in ["paddock", "ranch", "jb", "mesh"] {
        let path = format!("/w/{repo}");
        repository(&mut panel, &path, &path);
    }
    workspace(
        &mut panel.workspaces,
        "Tools",
        &["/w/paddock", "/w/ranch"],
        &[],
    );
    workspace(&mut panel.workspaces, "Tuning", &["/w/jb"], &[]);
    workspace(&mut panel.workspaces, "Vacant", &[], &[]);
    // As the open trays list them.
    assert_eq!(
        shown(&panel, None)[..9],
        [
            "= Tools 4 [Waiting, Working, Idle, Idle]",
            "  # paddock/ 2 [Working, Idle]",
            "  paddock/dev",
            "  paddock/main",
            "  # ranch/ 2 [Waiting, Idle]",
            "  ranch/main",
            "  ranch/dev",
            "= Tuning 1 [Idle]",
            "  # jb/ 1 [Idle]",
        ]
    );
    for name in ["Tools", "Tuning", "Vacant"] {
        panel.folded.toggle_workspace(name);
    }
    let stacks = |selected: Option<&str>| -> Vec<(String, Vec<String>)> {
        card::lines(&panel, None, selected, &[], None, 100.0)
            .into_iter()
            .filter_map(|line| match line {
                Line::Workspace(tray) => Some((tray.name, tray.stack)),
                _ => None,
            })
            .collect()
    };
    let stack = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
    // Nothing shown of it: the first three as listed.
    assert_eq!(
        stacks(None),
        [
            (
                "Tools".to_owned(),
                stack(&["paddock/dev", "paddock/main", "ranch/main"])
            ),
            ("Tuning".to_owned(), stack(&["jb/main"])),
            ("Vacant".to_owned(), stack(&[])),
        ]
    );
    assert_eq!(
        stacks(Some("mesh/main"))[0].1,
        ["paddock/dev", "paddock/main", "ranch/main"]
    );
    // The active pane's agent in front, then the others as listed, three in all.
    assert_eq!(
        stacks(Some("paddock/main"))[0].1,
        ["paddock/main", "paddock/dev", "ranch/main"]
    );
    assert_eq!(
        stacks(Some("ranch/dev"))[0].1,
        ["ranch/dev", "paddock/dev", "paddock/main"]
    );
    assert_eq!(stacks(Some("jb/main"))[1].1, ["jb/main"]);
}

#[test]
fn the_list_is_read_back_as_written_and_a_bad_file_is_no_workspaces() {
    let temp = common::tempdir();
    let path = temp.path().join("state/paddock/workspaces.json");
    // No file yet: none.
    assert_eq!(Workspaces::load(&path), Workspaces::default());
    let mut all = Workspaces::default();
    workspace(
        &mut all,
        "Tools",
        &["/w/paddock", "/w/ranch"],
        &["jb/codex-1"],
    );
    workspace(&mut all, "Empty", &[], &[]);
    all.save(&path).unwrap();
    assert_eq!(Workspaces::load(&path), all);
    assert_eq!(
        Workspaces::load(&path).list(),
        [
            Workspace {
                name: "Empty".into(),
                repos: vec![],
                agents: vec![],
            },
            Workspace {
                name: "Tools".into(),
                repos: vec!["/w/paddock".into(), "/w/ranch".into()],
                agents: vec!["jb/codex-1".into()],
            },
        ]
    );
    std::fs::write(&path, "{ not a list").unwrap();
    assert_eq!(Workspaces::load(&path), Workspaces::default());
}

#[test]
fn a_name_is_needed_and_taken_once_whatever_its_case() {
    let mut all = Workspaces::default();
    assert_eq!(all.create("  Agent tools "), Ok("Agent tools".to_owned()));
    assert_eq!(all.create(""), Err(NameError::Empty));
    assert_eq!(all.create("   "), Err(NameError::Empty));
    assert_eq!(all.create("agent TOOLS"), Err(NameError::Taken));
    assert_eq!(all.list().len(), 1);
    // Renaming: the same rules, its own name aside; its members go with it.
    all.add_repo("Agent tools", Path::new("/w/paddock"));
    assert_eq!(all.create("Tuning"), Ok("Tuning".to_owned()));
    assert_eq!(all.rename("Tuning", "agent tools"), Err(NameError::Taken));
    assert_eq!(all.rename("Tuning", " "), Err(NameError::Empty));
    assert_eq!(
        all.rename("Agent tools", " agent tools"),
        Ok("agent tools".to_owned())
    );
    assert_eq!(all.rename("agent tools", "Zed"), Ok("Zed".to_owned()));
    assert_eq!(all.of_repo(Path::new("/w/paddock")), Some("Zed"));
    assert_eq!(
        all.list().iter().map(|w| &*w.name).collect::<Vec<_>>(),
        ["Tuning", "Zed"]
    );
}

#[test]
fn the_panel_ticks_a_whole_repository_or_some_of_its_agents() {
    let jb = Listed {
        name: "jb".into(),
        repos: vec!["/w/jb".into()],
        agents: vec!["jb/codex-1".into(), "jb/dev-1".into(), "jb/dev-2".into()],
    };
    let ticked = |picks: &Picks, taken: &[String]| {
        jb.agents
            .iter()
            .filter(|agent| picks.agent(&jb, agent, taken))
            .count()
    };
    let mut picks = Picks::default();
    // The whole repository brings every agent in it.
    picks.toggle_whole(&jb, &[]);
    assert!(picks.whole(&jb));
    assert_eq!(ticked(&picks, &[]), 3);
    // One taken off: the repository is no longer whole, the others stay ticked one by one.
    picks.toggle_agent(&jb, "jb/dev-1", &[]);
    assert!(!picks.whole(&jb));
    assert_eq!(
        picks,
        Picks {
            repos: vec![],
            agents: vec!["jb/codex-1".into(), "jb/dev-2".into()],
        }
    );
    // Whole again, the single ticks are not needed; off, nothing of it is left.
    picks.toggle_whole(&jb, &[]);
    assert_eq!(
        picks,
        Picks {
            repos: vec!["/w/jb".into()],
            agents: vec![],
        }
    );
    picks.toggle_whole(&jb, &[]);
    assert_eq!(picks, Picks::default());
    // An agent recorded by itself in another workspace does not come with its repository: it
    // comes when ticked itself.
    let taken = ["jb/dev-2".to_owned()];
    picks.toggle_whole(&jb, &taken);
    assert_eq!(ticked(&picks, &taken), 2);
    picks.toggle_agent(&jb, "jb/dev-2", &taken);
    assert_eq!(ticked(&picks, &taken), 3);
    assert!(picks.whole(&jb));
    // Set as ticked: members come from wherever they were, and what is not ticked goes.
    let mut all = Workspaces::default();
    workspace(&mut all, "Other", &["/w/jb"], &["jb/dev-2", "solo"]);
    workspace(&mut all, "Tuning", &["/w/old"], &["gone"]);
    all.set_members("Tuning", &picks.repos, &picks.agents);
    assert_eq!(
        all.list(),
        [
            Workspace {
                name: "Other".into(),
                repos: vec![],
                agents: vec!["solo".into()],
            },
            Workspace {
                name: "Tuning".into(),
                repos: vec!["/w/jb".into()],
                agents: vec!["jb/dev-2".into()],
            },
        ]
    );
}

#[test]
fn search_finds_agents_by_their_workspaces_name_and_shows_the_group_as_before() {
    let mut panel = panel(&[
        ("paddock/main", "/Users/me/code/paddock", "working"),
        ("ranch/main", "/Users/me/code/ranch", "idle"),
    ]);
    repository(
        &mut panel,
        "/Users/me/code/paddock",
        "/Users/me/code/paddock",
    );
    workspace(
        &mut panel.workspaces,
        "Agent tools",
        &["/Users/me/code/paddock"],
        &[],
    );
    let rows = search::agents(&panel, 100.0, Some("/Users/me"));
    let found = search::everything(&rows, &[], &[], &[], "agent tools");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rows.len(), 1);
    assert_eq!(
        found[0].rows[0].target,
        search::Target::Agent("paddock/main".into())
    );
    // The row says what it said.
    assert_eq!(found[0].rows[0].detail, "Working · ~/…/paddock");
}
