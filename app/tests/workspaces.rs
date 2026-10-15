//! Workspaces (P5-89, by repository only since P5-105): who is in which, one place each, the
//! sidebar's lines with them, folding, their file and what an older paddock left in it, their
//! names, what the panel ticks, and finding agents by them.
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

/// A workspace named `name` with these whole repositories.
fn workspace(workspaces: &mut Workspaces, name: &str, repos: &[&str]) {
    let name = workspaces.create(name).unwrap();
    for repo in repos {
        workspaces.add_repo(&name, Path::new(repo));
    }
}

/// The list as an older paddock kept it, `json`, agents recorded by name in it, read from `dir`.
fn older(dir: &Path, json: &str) -> Workspaces {
    let path = dir.join("workspaces.json");
    std::fs::write(&path, json).unwrap();
    Workspaces::load(&path)
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
            Line::Agent(card) => assert_eq!(card.workspace, None),
            _ => panic!("no other lines"),
        }
    }
}

#[test]
fn an_agent_is_where_its_repository_is_whatever_its_name_and_a_prefix_follows_the_name() {
    let temp = common::tempdir();
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/trees/p5", "idle"),
        ("paddock/review", "/trees/review", "idle"),
        ("paddock/test", "/plain", "idle"),
        ("ranch/main", "/w/ranch", "idle"),
        ("solo", "/plain", "idle"),
    ]);
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/trees/p5", "/w/paddock");
    repository(&mut panel, "/trees/review", "/w/paddock");
    repository(&mut panel, "/w/ranch", "/w/ranch");
    panel.absorb_git(vec![("/plain".into(), None)]);
    // An older paddock recorded paddock/dev by itself in Zeta: that does nothing now.
    panel.workspaces = older(
        temp.path(),
        r#"[{"name": "Tools", "repos": ["/w/paddock"]},
            {"name": "Zeta", "repos": [], "agents": ["paddock/dev", "ranch/main"]}]"#,
    );
    // The whole repository: its main worktree's agent and those in two worktrees; and the one
    // whose repository cannot be read, by its prefix, the repository's name.
    for name in [
        "paddock/main",
        "paddock/dev",
        "paddock/review",
        "paddock/test",
    ] {
        assert_eq!(of(&panel, name), Some("Tools"), "{name}");
    }
    // Neither in a workspace's repository nor by a prefix: in none, recorded by name or not.
    assert_eq!(of(&panel, "ranch/main"), None);
    assert_eq!(of(&panel, "solo"), None);
    // Two workspaces with a repository of that name: the prefix follows neither.
    panel
        .workspaces
        .add_repo("Zeta", Path::new("/elsewhere/paddock"));
    assert_eq!(of(&panel, "paddock/test"), None);
    assert_eq!(of(&panel, "paddock/main"), Some("Tools"));
}

#[test]
fn a_repository_is_in_one_workspace_only() {
    let mut all = Workspaces::default();
    workspace(&mut all, "One", &["/w/paddock", "/w/ranch"]);
    workspace(&mut all, "Two", &[]);
    all.add_repo("Two", Path::new("/w/paddock"));
    assert_eq!(all.get("One").unwrap().repos, [PathBuf::from("/w/ranch")]);
    assert_eq!(all.get("Two").unwrap().repos, [PathBuf::from("/w/paddock")]);
    assert_eq!(all.of_repo(Path::new("/w/paddock")), Some("Two"));
    // A workspace that is not there (another window removed it) takes nothing from the others.
    all.add_repo("Gone", Path::new("/w/ranch"));
    all.set_members("Gone", &[PathBuf::from("/w/paddock")]);
    assert_eq!(all.of_repo(Path::new("/w/ranch")), Some("One"));
    assert_eq!(all.of_repo(Path::new("/w/paddock")), Some("Two"));
    // Taken out, it is in none; the workspace gone, its repositories are in none.
    all.remove_repo(Path::new("/w/paddock"));
    assert!(all.get("Two").unwrap().repos.is_empty());
    all.remove("One");
    assert_eq!(all.of_repo(Path::new("/w/ranch")), None);
}

#[test]
fn an_agent_started_from_a_workspace_brings_its_repository_when_that_is_in_none() {
    let mut all = Workspaces::default();
    workspace(&mut all, "Big", &["/w/paddock"]);
    workspace(&mut all, "Other", &["/w/cairn"]);
    workspace(&mut all, "test", &[]);
    let repos = |all: &Workspaces, name: &str| all.get(name).unwrap().repos.clone();
    // Its repository is in the workspace already: nothing to record.
    assert!(!all.join("Big", Some(Path::new("/w/paddock"))));
    assert_eq!(repos(&all, "Big"), [PathBuf::from("/w/paddock")]);
    // In none: the whole repository comes, its other agents and those started later with it.
    assert!(all.join("test", Some(Path::new("/w/scratch"))));
    assert_eq!(repos(&all, "test"), [PathBuf::from("/w/scratch")]);
    // In another workspace: it stays there, and the agent with it.
    assert!(!all.join("Big", Some(Path::new("/w/cairn"))));
    assert_eq!(all.of_repo(Path::new("/w/cairn")), Some("Other"));
    assert_eq!(repos(&all, "Big"), [PathBuf::from("/w/paddock")]);
    // Outside Git: nothing is recorded.
    assert!(!all.join("test", None));
    assert_eq!(repos(&all, "test"), [PathBuf::from("/w/scratch")]);
    // A workspace that is gone (another window removed it) takes nothing.
    assert!(!all.join("Gone", Some(Path::new("/w/lost"))));
    assert_eq!(all.of_repo(Path::new("/w/lost")), None);
    // Its New Agent… starts in its first repository by name; with none, where it would anyway.
    all.add_repo("Big", Path::new("/w/b/ranch"));
    all.add_repo("Big", Path::new("/w/a/zed"));
    assert_eq!(
        all.get("Big").unwrap().first_repo(),
        Some(Path::new("/w/paddock"))
    );
    workspace(&mut all, "Vacant", &[]);
    assert_eq!(all.get("Vacant").unwrap().first_repo(), None);
}

#[test]
fn names_an_older_paddock_recorded_give_way_to_their_running_agents_repositories() {
    let temp = common::tempdir();
    let path = temp.path().join("workspaces.json");
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/trees/p5", "working"),
        ("paddock/review", "/trees/review", "idle"),
        ("paddock/scratch", "/plain", "idle"),
        ("cairn/main", "/w/cairn", "idle"),
        ("plain/main", "/plain", "idle"),
    ]);
    // The controller of paddock and two more by name in My-Harness, as P5-89 let it keep them;
    // paddock's agent in a worktree by name in Third; cairn whole in Other.
    panel.workspaces = older(
        temp.path(),
        r#"[{"name": "My-Harness", "repos": [],
             "agents": ["paddock/main", "cairn/main", "gone/main", "plain/main"]},
            {"name": "Other", "repos": ["/w/cairn"], "agents": []},
            {"name": "Third", "repos": [], "agents": ["paddock/dev"]}]"#,
    );
    assert!(panel.workspaces.named());
    // Till Git tells where the named ones that run work, they wait.
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/trees/review", "/w/paddock");
    assert!(!panel.named_read());
    repository(&mut panel, "/trees/p5", "/w/paddock");
    repository(&mut panel, "/w/cairn", "/w/cairn");
    assert!(!panel.named_read());
    panel.absorb_git(vec![("/plain".into(), None)]);
    assert!(panel.named_read());
    // The names do nothing meanwhile: paddock is in no workspace.
    assert_eq!(of(&panel, "paddock/main"), None);
    assert_eq!(of(&panel, "paddock/dev"), None);
    assert_eq!(of(&panel, "cairn/main"), Some("Other"));
    // Given way: paddock whole in My-Harness, the first name of it, by the workspaces' order;
    // cairn left in Other; paddock/dev's name in Third only goes, its repository taken already.
    assert!(panel.adopt_named());
    let repos = |panel: &Panel, name: &str| panel.workspaces.get(name).unwrap().repos.clone();
    assert_eq!(repos(&panel, "My-Harness"), [PathBuf::from("/w/paddock")]);
    assert_eq!(repos(&panel, "Other"), [PathBuf::from("/w/cairn")]);
    assert!(repos(&panel, "Third").is_empty());
    // Every agent of paddock with it: in the main worktree, in either worktree, and by prefix.
    for name in [
        "paddock/main",
        "paddock/dev",
        "paddock/review",
        "paddock/scratch",
    ] {
        assert_eq!(of(&panel, name), Some("My-Harness"), "{name}");
    }
    assert_eq!(of(&panel, "cairn/main"), Some("Other"));
    // Not running, or its repository not read: the name stays, doing nothing, and nothing more
    // gives way.
    assert_eq!(
        panel.workspaces.get("My-Harness").unwrap().agents,
        ["gone/main", "plain/main"]
    );
    assert_eq!(of(&panel, "plain/main"), None);
    assert!(!panel.adopt_named());
    // Kept, the list leaves the names out.
    panel.workspaces.save(&path).unwrap();
    let kept = Workspaces::load(&path);
    assert!(!kept.named());
    assert_eq!(
        kept.list(),
        [
            Workspace {
                name: "My-Harness".into(),
                repos: vec!["/w/paddock".into()],
                agents: vec![],
            },
            Workspace {
                name: "Other".into(),
                repos: vec!["/w/cairn".into()],
                agents: vec![],
            },
            Workspace {
                name: "Third".into(),
                repos: vec![],
                agents: vec![],
            },
        ]
    );
    // A list that cannot be read is no workspaces, and no names to give way.
    let mut broken = older(
        temp.path(),
        r#"[{"name": "My-Harness", "agents": ["paddock/main"#,
    );
    assert_eq!(broken, Workspaces::default());
    assert!(!broken.adopt(|_| Some(PathBuf::from("/w/paddock"))));
}

#[test]
fn the_agents_of_a_repository_are_under_one_heading_whatever_their_names_or_worktrees() {
    let temp = common::tempdir();
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev-agy-1", "/trees/p5-104", "working"),
        ("paddock/dev-repo-1", "/trees/p5-105", "working"),
        ("paddock/test", "/plain", "idle"),
        ("ranch/main", "/w/ranch", "idle"),
    ]);
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/trees/p5-104", "/w/paddock");
    repository(&mut panel, "/trees/p5-105", "/w/paddock");
    repository(&mut panel, "/w/ranch", "/w/ranch");
    panel.absorb_git(vec![("/plain".into(), None)]);
    // The controller recorded by name in My-Harness, as P5-89 kept it, its repository in none:
    // all of paddock below, once; My-Harness has none of it.
    panel.workspaces = older(
        temp.path(),
        r#"[{"name": "My-Harness", "repos": ["/w/ranch"], "agents": ["paddock/main"]}]"#,
    );
    assert_eq!(
        shown(&panel, None),
        [
            "= My-Harness 1 [Idle]",
            "  # ranch/ 1 [Idle]",
            "  ranch/main",
            "# paddock/ 4 [Working, Working, Idle, Idle]",
            "paddock/dev-agy-1",
            "paddock/dev-repo-1",
            "paddock/main",
            "paddock/test",
        ]
    );
    // The repository in it: all of paddock there, once, the one going by its prefix too.
    panel
        .workspaces
        .add_repo("My-Harness", Path::new("/w/paddock"));
    assert_eq!(
        shown(&panel, None),
        [
            "= My-Harness 5 [Working, Working, Idle, Idle, Idle]",
            "  # paddock/ 4 [Working, Working, Idle, Idle]",
            "  paddock/dev-agy-1",
            "  paddock/dev-repo-1",
            "  paddock/main",
            "  paddock/test",
            "  # ranch/ 1 [Idle]",
            "  ranch/main",
        ]
    );
    // A card's cell moves the repository it knows; the one going by its prefix knows none.
    let lines = card::lines(&panel, None, None, &[], None, 100.0);
    let repo = |name: &str| {
        lines
            .iter()
            .find_map(|line| match line {
                Line::Agent(card) if card.name == name => Some(card.repo.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(repo("paddock/dev-agy-1"), Some(PathBuf::from("/w/paddock")));
    assert_eq!(repo("paddock/test"), None);
}

#[test]
fn workspaces_come_first_by_name_with_their_groups_and_empty_repositories() {
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
    workspace(&mut panel.workspaces, "Zoo", &[]);
    workspace(&mut panel.workspaces, "fine-tuning", &["/w/jb"]);
    workspace(
        &mut panel.workspaces,
        "Agent tools",
        &["/w/ranch", "/w/saddle", "/w/paddock", "/w/cairn"],
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
            "= fine-tuning 2 [Working, Idle]",
            "  # jb/ 2 [Working, Idle]",
            "  jb/dev-1",
            "  jb/codex-1",
            "= Zoo 0 []",
            "  - No agents",
            "# mesh/ 1 [Idle]",
            "mesh/main",
        ]
    );
    // What a heading's button moves: the whole repository of its name in its workspace; a
    // group outside any, the repositories of its agents.
    let lines = card::lines(&panel, None, None, &[], None, 100.0);
    let heading = |workspace: Option<&str>, group: &str| {
        lines
            .iter()
            .find_map(|line| match line {
                Line::Group(heading)
                    if heading.workspace.as_deref() == workspace && heading.group == group =>
                {
                    Some(heading.repos.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        heading(Some("Agent tools"), "paddock"),
        [PathBuf::from("/w/paddock")]
    );
    assert_eq!(heading(Some("fine-tuning"), "jb"), [PathBuf::from("/w/jb")]);
    assert_eq!(heading(None, "mesh"), [PathBuf::from("/w/mesh")]);
    // The card says where its agent is.
    let card = |name: &str| {
        lines
            .iter()
            .find_map(|line| match line {
                Line::Agent(card) if card.name == name => Some(card.workspace.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(card("jb/codex-1"), Some("fine-tuning".to_owned()));
    assert_eq!(card("ranch/main"), Some("Agent tools".to_owned()));
    assert_eq!(card("mesh/main"), None);
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
    workspace(&mut panel.workspaces, "Tools", &["/w/paddock", "/w/ranch"]);
    workspace(&mut panel.workspaces, "Tuning", &["/w/jb"]);
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
    workspace(&mut panel.workspaces, "Tools", &["/w/paddock", "/w/ranch"]);
    workspace(&mut panel.workspaces, "Tuning", &["/w/jb"]);
    workspace(&mut panel.workspaces, "Vacant", &[]);
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
    workspace(&mut all, "Tools", &["/w/paddock", "/w/ranch"]);
    workspace(&mut all, "Empty", &[]);
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
                agents: vec![],
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
fn the_panel_ticks_whole_repositories_and_lists_no_group_that_goes_by_a_prefix() {
    let mut panel = panel(&[
        ("jb/codex-1", "/w/jb", "idle"),
        ("jb/dev-1", "/trees/jb-dev", "working"),
        ("jb/test", "/plain", "idle"),
        ("scratch/main", "/plain", "idle"),
    ]);
    repository(&mut panel, "/w/jb", "/w/jb");
    repository(&mut panel, "/trees/jb-dev", "/w/jb");
    panel.absorb_git(vec![("/plain".into(), None)]);
    // jb once, the agent of it that goes by its prefix counted too; scratch, by its prefix
    // alone, has no repository to tick.
    let groups = panel.groups();
    assert_eq!(
        groups,
        [Listed {
            name: "jb".into(),
            repos: vec!["/w/jb".into()],
            agents: 3,
        }]
    );
    let jb = &groups[0];
    let mut picks = Picks::default();
    // The whole repository: every agent in it comes, and any started there later.
    picks.toggle_whole(jb);
    assert!(picks.whole(jb));
    assert_eq!(picks.summary(), "1 repository");
    // Off, nothing of it is left.
    picks.toggle_whole(jb);
    assert_eq!(picks, Picks::default());
    assert_eq!(picks.summary(), "");
    // Set as ticked: repositories come from wherever they were, and what is not ticked goes.
    picks.toggle_whole(jb);
    picks.toggle_repo(Path::new("/w/new"));
    assert_eq!(picks.summary(), "2 repositories");
    let mut all = Workspaces::default();
    workspace(&mut all, "Other", &["/w/jb", "/w/solo"]);
    workspace(&mut all, "Tuning", &["/w/old"]);
    all.set_members("Tuning", &picks.repos);
    assert_eq!(
        all.list(),
        [
            Workspace {
                name: "Other".into(),
                repos: vec!["/w/solo".into()],
                agents: vec![],
            },
            Workspace {
                name: "Tuning".into(),
                repos: vec!["/w/jb".into(), "/w/new".into()],
                agents: vec![],
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
