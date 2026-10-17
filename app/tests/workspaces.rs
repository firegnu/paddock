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
    read(&mut panel, entries);
    panel
}

/// corral lists these agents now: name, directory, state.
fn read(panel: &mut Panel, entries: &[(&str, &str, &str)]) {
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
            Line::Vacancies { repos, open, .. } => {
                let arrow = if open { "v" } else { ">" };
                format!("  {arrow} {} No agents", repos.join(", "))
            }
            Line::Empty(_) => "  - No agents".to_owned(),
            Line::Agent(card) => format!("{}{}", inset(&card.workspace), card.name),
            Line::Idle { count, folded } => {
                marks(format!("Idle workspaces {count}"), folded, false)
            }
            Line::IdleWorkspace { name, repos } => format!("~ {name} {repos}"),
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
    let picks = Picks {
        repos: vec![PathBuf::from("/w/paddock")],
        ..Picks::default()
    };
    all.set_picked("Gone", &picks);
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
fn a_change_that_changes_nothing_keeps_no_name_from_waiting_for_git() {
    let temp = common::tempdir();
    let path = temp.path().join("workspaces.json");
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("ranch/dev", "/trees/ranch-dev", "working"),
        ("cairn/main", "/w/cairn", "idle"),
    ]);
    // Two names in My-Harness, as P5-89 let it keep them; cairn whole in Other.
    let json = r#"[{"name": "My-Harness", "repos": [], "agents": ["paddock/main", "ranch/dev"]},
                   {"name": "Other", "repos": ["/w/cairn"], "agents": []}]"#;
    panel.workspaces = older(temp.path(), json);
    // Git has told where paddock/main works, not yet ranch/dev.
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/w/cairn", "/w/cairn");
    assert!(!panel.named_read());
    // Agents started from My-Harness's New Agent… in a repository of another workspace, outside
    // Git, and from a workspace another window removed, and the list's own turn: none changes
    // the list, so none keeps paddock/main's giving way, which would lose ranch/dev's name.
    let changes: [fn(&mut Workspaces); 4] = [
        |all| assert!(!all.join("My-Harness", Some(Path::new("/w/cairn")))),
        |all| assert!(!all.join("My-Harness", None)),
        |all| assert!(!all.join("Gone", Some(Path::new("/w/lost")))),
        |_| {},
    ];
    for change in changes {
        panel.edit_workspaces(Some(&path), change).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
        assert_eq!(
            Workspaces::load(&path).get("My-Harness").unwrap().agents,
            ["paddock/main", "ranch/dev"]
        );
        // Not kept, the names do nothing yet.
        assert_eq!(of(&panel, "paddock/main"), None);
    }
    // Git tells of ranch/dev: both give way, kept, and no name is left.
    repository(&mut panel, "/trees/ranch-dev", "/w/ranch");
    panel.edit_workspaces(Some(&path), |_| {}).unwrap();
    let kept = Workspaces::load(&path);
    assert!(!kept.named());
    assert_eq!(
        kept.get("My-Harness").unwrap().repos,
        [PathBuf::from("/w/paddock"), PathBuf::from("/w/ranch")]
    );
    assert_eq!(of(&panel, "ranch/dev"), Some("My-Harness"));
}

#[test]
fn done_keeps_what_names_gave_way_to_while_the_panel_was_open() {
    let temp = common::tempdir();
    let path = temp.path().join("workspaces.json");
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("cairn/main", "/w/cairn", "idle"),
    ]);
    panel.workspaces = older(
        temp.path(),
        r#"[{"name": "My-Harness", "repos": ["/w/cairn"], "agents": ["paddock/main"]}]"#,
    );
    let repos = || {
        Workspaces::load(&path)
            .get("My-Harness")
            .unwrap()
            .repos
            .clone()
    };
    // The panel opens on the list as read, before Git has told where paddock/main works.
    let opened = Picks::of(panel.workspaces.get("My-Harness").unwrap());
    assert!(!opened.repo(Path::new("/w/paddock")));
    // Git tells; the name gives way to paddock, kept.
    repository(&mut panel, "/w/paddock", "/w/paddock");
    repository(&mut panel, "/w/cairn", "/w/cairn");
    panel.edit_workspaces(Some(&path), |_| {}).unwrap();
    let both = [PathBuf::from("/w/cairn"), PathBuf::from("/w/paddock")];
    assert_eq!(repos(), both);
    // The open panel follows: paddock comes ticked.
    let mut followed = opened.clone();
    followed.follow(panel.workspaces.get("My-Harness").unwrap());
    assert!(followed.repo(Path::new("/w/paddock")));
    assert_eq!(followed.summary(), "2 repositories");
    // Done with nothing ticked or unticked, as the panel opened or as it follows: paddock stays.
    for picks in [&opened, &followed] {
        panel
            .edit_workspaces(Some(&path), |all| all.set_picked("My-Harness", picks))
            .unwrap();
        assert_eq!(repos(), both);
    }
    // Unticked, it goes.
    followed.toggle_repo(Path::new("/w/paddock"));
    panel
        .edit_workspaces(Some(&path), |all| all.set_picked("My-Harness", &followed))
        .unwrap();
    assert_eq!(repos(), [PathBuf::from("/w/cairn")]);
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
            "# mesh/ 1 [Idle]",
            "mesh/main",
            // One with no agent lies at the foot (P5-134).
            "Idle workspaces 1",
            "~ Zoo 0",
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
            // One with no agent has no title: it lies at the list's foot (P5-134).
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
fn the_panel_ticks_whole_repositories_and_lists_none_for_agents_that_go_by_a_prefix() {
    let mut panel = panel(&[
        ("jb/codex-1", "/w/jb", "idle"),
        ("jb/dev-1", "/trees/jb-dev", "working"),
        ("jb/test", "/plain", "idle"),
        ("scratch/main", "/plain", "idle"),
    ]);
    repository(&mut panel, "/w/jb", "/w/jb");
    repository(&mut panel, "/trees/jb-dev", "/w/jb");
    panel.absorb_git(vec![("/plain".into(), None)]);
    // jb once, with the agents that work in it; jb/test and scratch, going by their prefix,
    // work in no repository to tick.
    assert_eq!(
        panel.repositories(),
        [Listed {
            repo: "/w/jb".into(),
            agents: 2,
        }]
    );
    let jb = Path::new("/w/jb");
    let mut picks = Picks::default();
    // The whole repository: every agent in it comes, and any started there later.
    picks.toggle_repo(jb);
    assert!(picks.repo(jb));
    assert_eq!(picks.summary(), "1 repository");
    // Off, nothing of it is left.
    picks.toggle_repo(jb);
    assert_eq!(picks, Picks::default());
    assert_eq!(picks.summary(), "");
    // As ticked since the panel opened: repositories come from wherever they were, and those
    // unticked go.
    let mut all = Workspaces::default();
    workspace(&mut all, "Other", &["/w/jb", "/w/solo"]);
    workspace(&mut all, "Tuning", &["/w/old"]);
    let mut picks = Picks::of(all.get("Tuning").unwrap());
    picks.toggle_repo(jb);
    picks.toggle_repo(Path::new("/w/new"));
    picks.toggle_repo(Path::new("/w/old"));
    assert_eq!(picks.summary(), "2 repositories");
    all.set_picked("Tuning", &picks);
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
fn two_repositories_of_one_name_are_ticked_apart_and_the_list_keeps_one_group_of_them() {
    let mut panel = panel(&[
        ("demo/main", "/one/demo", "idle"),
        ("demo/dev", "/trees/demo-dev", "working"),
        ("demo/two", "/two/demo", "idle"),
    ]);
    repository(&mut panel, "/one/demo", "/one/demo");
    repository(&mut panel, "/trees/demo-dev", "/one/demo");
    repository(&mut panel, "/two/demo", "/two/demo");
    // Outside workspaces the list has them in one demo group, as P5-80 does; the panel has a
    // row for each.
    assert_eq!(
        shown(&panel, None),
        [
            "# demo/ 3 [Working, Idle, Idle]",
            "demo/dev",
            "demo/main",
            "demo/two",
        ]
    );
    assert_eq!(
        panel.repositories(),
        [
            Listed {
                repo: "/one/demo".into(),
                agents: 2,
            },
            Listed {
                repo: "/two/demo".into(),
                agents: 1,
            },
        ]
    );
    // One in One, the other in Two.
    workspace(&mut panel.workspaces, "One", &["/one/demo"]);
    workspace(&mut panel.workspaces, "Two", &["/two/demo"]);
    let one = Path::new("/one/demo");
    let two = Path::new("/two/demo");
    // One's panel has its own ticked, not Two's; Done as it opened moves nothing.
    let mut picks = Picks::of(panel.workspaces.get("One").unwrap());
    assert!(picks.repo(one) && !picks.repo(two));
    panel
        .edit_workspaces(None, |all| all.set_picked("One", &picks))
        .unwrap();
    assert_eq!(panel.workspaces.of_repo(one), Some("One"));
    assert_eq!(panel.workspaces.of_repo(two), Some("Two"));
    // Unticked, One's goes alone; Two keeps the other.
    picks.toggle_repo(one);
    panel
        .edit_workspaces(None, |all| all.set_picked("One", &picks))
        .unwrap();
    assert_eq!(panel.workspaces.of_repo(one), None);
    assert_eq!(panel.workspaces.of_repo(two), Some("Two"));
    // Ticked again, it comes back alone; each agent is where its repository is.
    let mut picks = Picks::of(panel.workspaces.get("One").unwrap());
    picks.toggle_repo(one);
    panel
        .edit_workspaces(None, |all| all.set_picked("One", &picks))
        .unwrap();
    assert_eq!(panel.workspaces.get("One").unwrap().repos, [one]);
    assert_eq!(panel.workspaces.get("Two").unwrap().repos, [two]);
    assert_eq!(of(&panel, "demo/dev"), Some("One"));
    assert_eq!(of(&panel, "demo/main"), Some("One"));
    assert_eq!(of(&panel, "demo/two"), Some("Two"));
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

/// The workspaces with no agent (P5-134): at the list's foot under `Idle workspaces`, by name,
/// how many repositories each holds; up again as soon as one of their repositories has an agent,
/// one that goes by its name's prefix too, as they were kept; down again once those stop.
#[test]
fn workspaces_with_no_agent_lie_at_the_foot_by_name_and_come_up_with_an_agent() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("mesh/main", "/w/mesh", "idle"),
    ]);
    for repo in ["paddock", "mesh"] {
        let path = format!("/w/{repo}");
        repository(&mut panel, &path, &path);
    }
    workspace(&mut panel.workspaces, "Tools", &["/w/paddock"]);
    workspace(&mut panel.workspaces, "zeta", &["/w/a", "/w/b", "/w/c"]);
    workspace(&mut panel.workspaces, "Beta", &["/w/ranch"]);
    workspace(&mut panel.workspaces, "alpha", &[]);
    let above = [
        "= Tools 1 [Idle]",
        "  # paddock/ 1 [Idle]",
        "  paddock/main",
        "# mesh/ 1 [Idle]",
        "mesh/main",
    ];
    assert_eq!(
        shown(&panel, None),
        [
            &above[..],
            &["Idle workspaces 3", "~ alpha 0", "~ Beta 1", "~ zeta 3"],
        ]
        .concat()
    );
    // Folded, the group is its heading alone.
    panel.folded.idle = true;
    assert_eq!(
        shown(&panel, None),
        [&above[..], &["Idle workspaces 3 folded"]].concat()
    );
    panel.folded.idle = false;
    // An agent in one of its repositories, its name's prefix saying so before Git has: up, by
    // name, folded as it was kept.
    panel.folded.toggle_workspace("Beta");
    let with_ranch = [
        ("paddock/main", "/w/paddock", "idle"),
        ("mesh/main", "/w/mesh", "idle"),
        ("ranch/dev", "/w/ranch", "working"),
    ];
    read(&mut panel, &with_ranch);
    let up = [
        "= Beta 1 [Working] folded",
        "= Tools 1 [Idle]",
        "  # paddock/ 1 [Idle]",
        "  paddock/main",
        "# mesh/ 1 [Idle]",
        "mesh/main",
        "Idle workspaces 2",
        "~ alpha 0",
        "~ zeta 3",
    ];
    assert_eq!(shown(&panel, None), up);
    repository(&mut panel, "/w/ranch", "/w/ranch");
    assert_eq!(shown(&panel, None), up);
    // Its last agent stopped: down again.
    read(
        &mut panel,
        &[
            ("paddock/main", "/w/paddock", "idle"),
            ("mesh/main", "/w/mesh", "idle"),
        ],
    );
    assert_eq!(
        shown(&panel, None),
        [
            &above[..],
            &["Idle workspaces 3", "~ alpha 0", "~ Beta 1", "~ zeta 3"],
        ]
        .concat()
    );
    assert!(panel.folded.workspace("Beta"));
    // With no agent at all, the group is all there is.
    read(&mut panel, &[]);
    assert_eq!(
        shown(&panel, None),
        [
            "Idle workspaces 4",
            "~ alpha 0",
            "~ Beta 1",
            "~ Tools 1",
            "~ zeta 3"
        ]
    );
}

/// Opened from the foot, or just made, a workspace with no agent stays up, open, its repositories
/// a line each, till its title is clicked: then it goes back down, what is kept folded of it as
/// it was. Once it has had agents and they stop, it goes down too. None of this is kept: a new
/// panel, as paddock opened again, has them all at the foot (P5-134).
#[test]
fn opened_and_new_workspaces_stay_up_open_till_their_title_sends_them_back() {
    let mut panel = panel(&[("mesh/main", "/w/mesh", "idle")]);
    repository(&mut panel, "/w/mesh", "/w/mesh");
    workspace(&mut panel.workspaces, "Tools", &["/w/ranch", "/w/paddock"]);
    workspace(&mut panel.workspaces, "Zoo", &[]);
    panel.folded.toggle_workspace("Tools");
    panel.raise("Tools");
    let lines = card::lines(&panel, None, None, &[], None, 100.0);
    let Line::Workspace(tray) = &lines[0] else {
        panic!("its title first");
    };
    assert!(tray.opened && !tray.folded);
    assert_eq!(
        shown(&panel, None),
        [
            "= Tools 0 []",
            "  - paddock No agents",
            "  - ranch No agents",
            "# mesh/ 1 [Idle]",
            "mesh/main",
            "Idle workspaces 1",
            "~ Zoo 0",
        ]
    );
    // Opened, it is open; and so it stays once back down.
    assert!(!panel.folded.workspace("Tools"));
    // Just made: up too, saying it has nothing.
    workspace(&mut panel.workspaces, "New", &[]);
    panel.raise("New");
    assert_eq!(shown(&panel, None)[..2], ["= New 0 []", "  - No agents"]);
    // Its title clicked, it goes back down; the other stays up.
    assert!(panel.lower("New"));
    assert!(!panel.folded.workspace("New"));
    assert_eq!(
        shown(&panel, None),
        [
            "= Tools 0 []",
            "  - paddock No agents",
            "  - ranch No agents",
            "# mesh/ 1 [Idle]",
            "mesh/main",
            "Idle workspaces 2",
            "~ New 0",
            "~ Zoo 0",
        ]
    );
    // An agent started in it: up as any is, then down once it stops.
    read(
        &mut panel,
        &[
            ("mesh/main", "/w/mesh", "idle"),
            ("paddock/dev", "/w/paddock", "working"),
        ],
    );
    repository(&mut panel, "/w/paddock", "/w/paddock");
    assert!(!panel.opened("Tools"));
    assert_eq!(
        shown(&panel, None)[..4],
        [
            "= Tools 1 [Working]",
            "  # paddock/ 1 [Working]",
            "  paddock/dev",
            "  - ranch No agents",
        ]
    );
    read(&mut panel, &[("mesh/main", "/w/mesh", "idle")]);
    assert_eq!(
        shown(&panel, None),
        [
            "# mesh/ 1 [Idle]",
            "mesh/main",
            "Idle workspaces 3",
            "~ New 0",
            "~ Tools 2",
            "~ Zoo 0",
        ]
    );
    // Made with a repository that has agents: up as any is, not kept up after.
    workspace(&mut panel.workspaces, "Mesh", &["/w/mesh"]);
    panel.raise("Mesh");
    assert!(!panel.opened("Mesh"));
    // Renamed, an opened one stays up, its line of repositories opened out stays so; removed,
    // both are forgotten.
    panel.raise("Zoo");
    panel.folded.toggle_vacant("Zoo");
    panel.workspaces.rename("Zoo", "Zed").unwrap();
    panel.rename_workspace("Zoo", "Zed");
    assert!(panel.opened("Zed") && !panel.opened("Zoo"));
    assert!(panel.folded.vacant_open("Zed") && !panel.folded.vacant_open("Zoo"));
    panel.workspaces.remove("Zed");
    panel.forget_workspace("Zed");
    assert!(!panel.opened("Zed") && !panel.folded.vacant_open("Zed"));
    // paddock opened again: kept in memory alone, so every one with no agent is at the foot.
    panel.raise("New");
    let mut again = Panel::default();
    again.workspaces = panel.workspaces.clone();
    again.folded = panel.folded.clone();
    read(&mut again, &[]);
    assert!(!again.opened("New"));
    assert_eq!(shown(&again, None)[0], "Idle workspaces 3");
}

/// In a workspace with agents, two or more repositories with no agent are one line after its
/// groups, opened out on a click into a line each; one alone is the line it was, among the
/// groups by name (P5-134).
#[test]
fn repositories_with_no_agent_beside_agents_are_one_line_when_two_or_more() {
    let mut panel = panel(&[("paddock/main", "/w/paddock", "idle")]);
    repository(&mut panel, "/w/paddock", "/w/paddock");
    workspace(
        &mut panel.workspaces,
        "Tools",
        &["/w/saddle", "/w/paddock", "/w/ranch", "/w/cairn"],
    );
    let groups = [
        "= Tools 1 [Idle]",
        "  # paddock/ 1 [Idle]",
        "  paddock/main",
    ];
    assert_eq!(
        shown(&panel, None),
        [&groups[..], &["  > cairn, ranch, saddle No agents"]].concat()
    );
    panel.folded.toggle_vacant("Tools");
    assert_eq!(
        shown(&panel, None),
        [
            &groups[..],
            &[
                "  v cairn, ranch, saddle No agents",
                "  - cairn No agents",
                "  - ranch No agents",
                "  - saddle No agents",
            ],
        ]
        .concat()
    );
    // The workspace folded, none of it shows.
    panel.folded.toggle_workspace("Tools");
    assert_eq!(shown(&panel, None), ["= Tools 1 [Idle] folded"]);
    panel.folded.toggle_workspace("Tools");
    // One left: as before.
    panel.workspaces.remove_repo(Path::new("/w/ranch"));
    panel.workspaces.remove_repo(Path::new("/w/saddle"));
    assert_eq!(
        shown(&panel, None),
        [
            "= Tools 1 [Idle]",
            "  - cairn No agents",
            "  # paddock/ 1 [Idle]",
            "  paddock/main",
        ]
    );
}
