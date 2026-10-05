//! From Saddle `tests/agents.rs` at commit `df1c727`, without the Git summary test.
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
fn fold_keeps_initial_choice_across_agent_count_changes_until_toggled() {
    let many = |n: usize| (0..n).map(|i| agent(&format!("p/{i}"), "idle")).collect();
    let mut panel = Panel::default();
    panel.absorb(many(5), None, 100.0);
    assert!(!panel.folded());
    panel.absorb(many(6), None, 101.0);
    assert!(!panel.folded());
    panel.absorb(many(8), None, 102.0);
    assert!(!panel.folded());
    // A manual choice also outlasts later changes in the agent count.
    panel.toggle_fold();
    panel.absorb(many(2), None, 103.0);
    assert!(panel.folded());
    panel.toggle_fold();
    panel.absorb(many(6), None, 104.0);
    assert!(!panel.folded());
}

#[test]
fn initially_large_list_stays_folded_after_agents_exit() {
    let mut panel = Panel::default();
    panel.absorb(
        (0..6).map(|i| agent(&format!("p/{i}"), "idle")).collect(),
        None,
        100.0,
    );
    assert!(panel.folded());
    panel.absorb(vec![agent("p/0", "idle")], None, 101.0);
    assert!(panel.folded());
}

#[test]
fn initially_empty_list_stays_expanded_when_agents_arrive() {
    let mut panel = Panel::default();
    panel.absorb(vec![], None, 100.0);
    assert!(!panel.folded());
    panel.absorb(
        (0..6).map(|i| agent(&format!("p/{i}"), "idle")).collect(),
        None,
        101.0,
    );
    assert!(!panel.folded());
}
