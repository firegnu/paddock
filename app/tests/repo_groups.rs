use paddock::{
    agents::{Panel, Status},
    card::{self, Line},
    corral::Agent,
    git::{Head, Summary},
    search,
};

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

fn groups(panel: &Panel) -> Vec<(String, usize, Vec<Status>)> {
    card::lines(panel, None, None, &[], None, 100.0)
        .into_iter()
        .filter_map(|line| match line {
            Line::Group(heading) => Some((heading.title, heading.count, heading.bar)),
            _ => None,
        })
        .collect()
}

#[test]
fn repository_groups_replace_prefixes_after_loading_and_merge_equal_names() {
    let mut panel = panel(&[
        ("jb-finetune/codex-1", "/repos/jb-finetune/sub", "idle"),
        ("jbfinetune/dev", "/trees/generation", "blocked"),
        ("jb-finetune/test", "/plain", "working"),
        ("unread/main", "/unread", "idle"),
        ("gone/main", "/gone", "idle"),
        ("solo", "/plain", "idle"),
    ]);
    // No Git result yet: the original prefix groups are used.
    assert_eq!(
        groups(&panel).into_iter().map(|g| g.0).collect::<Vec<_>>(),
        ["agents/", "gone/", "jb-finetune/", "jbfinetune/", "unread/"]
    );
    repository(&mut panel, "/repos/jb-finetune/sub", "/repos/jb-finetune");
    repository(&mut panel, "/trees/generation", "/repos/jb-finetune");
    panel.absorb_git(vec![("/plain".into(), None), ("/gone".into(), None)]);
    assert_eq!(
        groups(&panel),
        vec![
            ("agents/".into(), 1, vec![Status::Idle]),
            ("gone/".into(), 1, vec![Status::Idle]),
            (
                "jb-finetune/".into(),
                3,
                vec![Status::Waiting, Status::Working, Status::Idle]
            ),
            ("unread/".into(), 1, vec![Status::Idle]),
        ]
    );
    panel.select(Some("jbfinetune/dev".into()));
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("jb-finetune/test"));
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("jb-finetune/codex-1"));
    panel.by_name = true;
    assert_eq!(
        panel
            .ordered(100.0)
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        [
            "solo",
            "gone/main",
            "jb-finetune/codex-1",
            "jb-finetune/test",
            "jbfinetune/dev",
            "unread/main"
        ]
    );
}

#[test]
fn only_duplicate_short_names_in_the_same_group_keep_their_prefixes() {
    let mut panel = panel(&[
        ("jb-finetune/main", "/repo", "idle"),
        ("jbfinetune/main", "/tree", "idle"),
        ("jbfinetune/dev/task", "/tree", "idle"),
        ("other/main", "/other", "idle"),
    ]);
    repository(&mut panel, "/repo", "/repos/jb-finetune");
    repository(&mut panel, "/tree", "/repos/jb-finetune");
    let names: Vec<_> = card::lines(&panel, None, None, &[], None, 100.0)
        .into_iter()
        .filter_map(|line| match line {
            Line::Agent(card) => Some((card.name, card.short)),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        [
            ("jb-finetune/main".into(), "jb-finetune/main".into()),
            ("jbfinetune/dev/task".into(), "dev/task".into()),
            ("jbfinetune/main".into(), "jbfinetune/main".into()),
            ("other/main".into(), "main".into()),
        ]
    );
}

#[test]
fn search_matches_the_repository_group_and_the_original_full_name() {
    let mut panel = panel(&[("jbfinetune/dev", "/trees/generation", "idle")]);
    repository(&mut panel, "/trees/generation", "/repos/jb-finetune");
    let rows = search::agents(&panel, 100.0, None);
    for query in ["jb-finetune", "jbfinetune", "jbfinetune/dev"] {
        let found = search::everything(&rows, &[], &[], &[], query);
        assert_eq!(found.len(), 1, "{query}");
        assert_eq!(
            found[0].rows[0].target,
            search::Target::Agent("jbfinetune/dev".into())
        );
    }
}

#[test]
fn search_detail_omits_the_group_when_the_name_prefix_already_says_it() {
    let cwd = "/Users/me/code/paddock";
    let mut panel = panel(&[("paddock/main", cwd, "working")]);
    repository(&mut panel, cwd, cwd);
    let rows = search::agents(&panel, 100.0, Some("/Users/me"));
    assert_eq!(rows[0].title, "paddock/main");
    assert_eq!(rows[0].detail, "Working · ~/…/paddock");
    for query in ["paddock", "paddock/main"] {
        let found = search::everything(&rows, &[], &[], &[], query);
        assert_eq!(found.len(), 1, "{query}");
        assert_eq!(
            found[0].rows[0].target,
            search::Target::Agent("paddock/main".into())
        );
    }
}

/// Every line as one string: a group's heading with its count and bar (`folded` and `showing`
/// said when so), a card by its agent's full name.
fn shown(panel: &Panel, selected: Option<&str>) -> Vec<String> {
    card::lines(panel, None, selected, &[], None, 100.0)
        .into_iter()
        .map(|line| match line {
            Line::Error(text) => format!("! {text}"),
            Line::Group(heading) => {
                let mut text = format!("# {} {} {:?}", heading.title, heading.count, heading.bar);
                if heading.folded {
                    text += " folded";
                }
                if heading.showing {
                    text += " showing";
                }
                text
            }
            Line::Agent(card) => card.name,
            other => panic!("no workspaces here: {other:?}"),
        })
        .collect()
}

#[test]
fn a_folded_group_keeps_its_heading_count_and_bar_and_leaves_out_its_cards() {
    let mut panel = panel(&[
        ("paddock/main", "/w/paddock", "idle"),
        ("paddock/dev", "/w/paddock", "working"),
        ("ranch/main", "/w/ranch", "blocked"),
    ]);
    let open = [
        "# paddock/ 2 [Working, Idle]",
        "paddock/dev",
        "paddock/main",
        "# ranch/ 1 [Waiting]",
        "ranch/main",
    ];
    assert_eq!(shown(&panel, None), open);
    panel.folded.toggle_group(None, "paddock");
    assert_eq!(
        shown(&panel, None),
        [
            "# paddock/ 2 [Working, Idle] folded",
            "# ranch/ 1 [Waiting]",
            "ranch/main",
        ]
    );
    // Its bar follows its agents while it stays folded; the active pane's agent in it is said.
    panel.absorb(
        vec![
            Agent {
                name: "paddock/main".into(),
                cwd: Some("/w/paddock".into()),
                state: Some("blocked".into()),
                ..Default::default()
            },
            Agent {
                name: "ranch/main".into(),
                cwd: Some("/w/ranch".into()),
                state: Some("idle".into()),
                ..Default::default()
            },
        ],
        None,
        100.0,
    );
    assert_eq!(
        shown(&panel, Some("paddock/main")),
        [
            "# paddock/ 1 [Waiting] folded showing",
            "# ranch/ 1 [Idle]",
            "ranch/main",
        ]
    );
    // The strip shows every agent whatever is folded.
    let tiles: Vec<String> = card::unfolded(&panel, None, None, &[], None, 100.0)
        .into_iter()
        .filter_map(|line| match line {
            Line::Agent(card) => Some(card.name),
            _ => None,
        })
        .collect();
    assert_eq!(tiles, ["paddock/main", "ranch/main"]);
    // Opened again, the lines are what they were.
    panel.folded.toggle_group(None, "paddock");
    assert_eq!(
        shown(&panel, None),
        [
            "# paddock/ 1 [Waiting]",
            "paddock/main",
            "# ranch/ 1 [Idle]",
            "ranch/main",
        ]
    );
}

#[test]
fn moving_between_cards_skips_folded_groups() {
    let mut panel = panel(&[
        ("a/one", "/w/a", "idle"),
        ("b/one", "/w/b", "idle"),
        ("b/two", "/w/b", "idle"),
        ("c/one", "/w/c", "idle"),
    ]);
    panel.folded.toggle_group(None, "b");
    panel.select(Some("a/one".into()));
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("c/one"));
    panel.move_selection(-1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("a/one"));
    // From a card folded away, to the nearest one in view either way.
    panel.select(Some("b/two".into()));
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("c/one"));
    panel.select(Some("b/one".into()));
    panel.move_selection(-1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("a/one"));
    // Everything folded: the selection stays.
    panel.folded.toggle_group(None, "a");
    panel.folded.toggle_group(None, "c");
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("a/one"));
}
