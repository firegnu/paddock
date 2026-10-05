//! From Saddle `tests/agents.rs` at commit `df1c727`.
use paddock::{agents::Panel, corral::Agent};
fn agent(name: &str, state: &str) -> Agent {
    Agent {
        name: name.into(),
        instance: Some("123".into()),
        state: Some(state.into()),
        ..Default::default()
    }
}
#[test]
fn refresh_keeps_selection_and_marks_unseen_completed_turns() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![agent("p/a", "working"), agent("p/b", "idle")],
        None,
        100.0,
    );
    panel.selected = Some("p/b".into());
    panel.absorb(
        vec![
            agent("new/a", "idle"),
            agent("p/a", "idle"),
            agent("p/b", "idle"),
        ],
        None,
        101.0,
    );
    assert_eq!(panel.selected.as_deref(), Some("p/b"));
    assert!(panel.unread.contains("p/a"));
    panel.absorb(vec![agent("p/a", "idle")], Some("p/a"), 102.0);
    assert_eq!(panel.selected.as_deref(), Some("p/a"));
    assert!(panel.unread.is_empty());
    assert!(panel.message.contains("p/b exited"));
}

#[test]
fn state_sort_stays_within_projects_and_selection_follows_display_order() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![
            agent("a/idle", "idle"),
            agent("b/blocked", "blocked"),
            agent("a/blocked", "blocked"),
            agent("a/working", "working"),
        ],
        None,
        100.0,
    );
    let names: Vec<_> = panel
        .ordered(100.0)
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, ["a/blocked", "a/working", "a/idle", "b/blocked"]);
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("a/working"));
    panel.absorb(
        vec![Agent {
            instance: Some("456".into()),
            ..agent("a/working", "idle")
        }],
        None,
        101.0,
    );
    assert!(panel.unread.is_empty());
}

#[test]
fn git_results_are_kept_only_for_directories_agents_currently_use() {
    use paddock::git::{Head, Summary};
    let summary = |branch: &str| {
        Some(Summary {
            head: Head::Branch(branch.into()),
            ahead: None,
            changes: None,
            untracked: None,
        })
    };
    let with_cwd = |cwd: &str| Agent {
        cwd: Some(cwd.into()),
        ..agent("p/a", "idle")
    };
    let mut panel = Panel::default();
    panel.absorb(vec![with_cwd("/w/a")], None, 100.0);
    panel.absorb_git(vec![
        ("/w/a".into(), summary("a")),
        ("/w/b".into(), summary("b")),
    ]);
    assert_eq!(panel.git.get("/w/a"), Some(&summary("a")));
    assert!(!panel.git.contains_key("/w/b"));
    // After the agent moves, a late result for its old directory is dropped, not shown as its.
    panel.absorb(vec![with_cwd("/w/c")], None, 101.0);
    panel.absorb_git(vec![("/w/a".into(), summary("a"))]);
    assert!(panel.git.is_empty());
}

#[test]
fn default_order_puts_agents_needing_people_first_and_s_switches_to_names() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![
            agent("p/a-idle", "idle"),
            agent("p/b-exited", "exited"),
            agent("p/c-working", "working"),
            Agent {
                error: Some("status failed".into()),
                ..agent("p/d-error", "idle")
            },
            agent("p/e-waiting", "blocked"),
            agent("p/f-idle", "idle"),
            agent("q/a-idle", "idle"),
        ],
        None,
        100.0,
    );
    let names = |panel: &Panel| -> Vec<String> {
        panel
            .ordered(100.0)
            .iter()
            .map(|a| a.name.clone())
            .collect()
    };
    assert_eq!(
        names(&panel),
        [
            "p/e-waiting",
            "p/d-error",
            "p/c-working",
            "p/a-idle",
            "p/f-idle",
            "p/b-exited",
            "q/a-idle"
        ]
    );
    assert_eq!(panel.selected.as_deref(), Some("p/e-waiting"));
    panel.by_name = true;
    assert_eq!(
        names(&panel),
        [
            "p/a-idle",
            "p/b-exited",
            "p/c-working",
            "p/d-error",
            "p/e-waiting",
            "p/f-idle",
            "q/a-idle"
        ]
    );
    // Selection follows the agent, not its position.
    assert_eq!(panel.selected.as_deref(), Some("p/e-waiting"));
}

#[test]
fn details_open_per_agent_and_outlast_refreshes_until_closed() {
    let many = |n: usize| (0..n).map(|i| agent(&format!("p/{i}"), "idle")).collect();
    let mut panel = Panel::default();
    // However many agents there are, every card starts closed.
    panel.absorb(many(8), None, 100.0);
    assert!(panel.expanded.is_empty());
    panel.toggle_details("p/1");
    panel.toggle_details("p/5");
    panel.absorb(many(6), None, 101.0);
    assert_eq!(panel.expanded.len(), 2);
    // An agent that goes away takes its open details with it.
    panel.absorb(many(2), None, 102.0);
    assert!(panel.expanded.contains("p/1") && !panel.expanded.contains("p/5"));
    panel.toggle_details("p/1");
    assert!(panel.expanded.is_empty());
    panel.toggle_details("p/0");
    panel.toggle_details("p/1");
    panel.collapse_all();
    assert!(panel.expanded.is_empty());
}

/// paddock has no list cursor: the agent first in the list keeps its new reply until it is shown
/// in the active pane.
#[test]
fn without_a_cursor_only_the_shown_agent_counts_as_read() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![agent("p/a", "working"), agent("p/b", "idle")],
        None,
        100.0,
    );
    panel.absorb(
        vec![agent("p/a", "idle"), agent("p/b", "idle")],
        None,
        101.0,
    );
    assert!(panel.unread.contains("p/a"));
    panel.absorb(
        vec![agent("p/a", "idle"), agent("p/b", "idle")],
        Some("p/a"),
        102.0,
    );
    assert!(!panel.unread.contains("p/a"));
}
