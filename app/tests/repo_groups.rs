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
            Line::Group(name, count, bar) => Some((name, count, bar)),
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
