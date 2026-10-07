use super::*;

const ME: &str = "0123456789abcdef";

/// A window: shell (pane 1) | agent p/a (pane 2), then a tab with an empty pane (3).
struct Fixture {
    workspace: Workspace,
    facts: HashMap<PaneId, Facts>,
    listed: Vec<Listed>,
    busy: Option<&'static str>,
}

impl Fixture {
    fn new() -> Self {
        let (mut workspace, shell) = Workspace::new(Shown::Shell);
        let agent = workspace.split(Direction::Right, Shown::Agent("p/a".into()));
        let empty = workspace.new_tab(Shown::Empty);
        workspace.select_tab(0);
        workspace.focus(shell);
        let mut facts = HashMap::new();
        facts.insert(shell, shell_facts(shell));
        facts.insert(
            agent,
            Facts {
                state: "running".into(),
                attached: Some("p/a".into()),
                instance: Some("i-a".into()),
                cwd: Some("/".into()),
                ..Facts::default()
            },
        );
        facts.insert(
            empty,
            Facts {
                state: "empty".into(),
                ..Facts::default()
            },
        );
        Fixture {
            workspace,
            facts,
            listed: vec![
                Listed {
                    name: "p/a".into(),
                    instance: Some("i-a".into()),
                    cwd: Some("/".into()),
                    error: false,
                },
                Listed {
                    name: "p/b".into(),
                    instance: Some("i-b".into()),
                    cwd: Some("/tmp".into()),
                    error: false,
                },
            ],
            busy: None,
        }
    }

    fn model(&self) -> Model<'_> {
        Model {
            instance: ME,
            workspace: &self.workspace,
            facts: &self.facts,
            listed: &self.listed,
            startup_cwd: "/",
            busy: self.busy,
        }
    }

    fn plan(&self, operation: Operation, caller: Caller, records: &[Value]) -> Plan {
        self.plan_used(operation, caller, records, &mut HashSet::new())
    }

    fn plan_used(
        &self,
        operation: Operation,
        caller: Caller,
        records: &[Value],
        used: &mut HashSet<String>,
    ) -> Plan {
        let message = Message {
            instance: ME.into(),
            request_id: Some("r".into()),
            caller,
            operation,
        };
        plan(&self.model(), &message, records.iter(), used)
    }
}

fn shell_facts(pane: PaneId) -> Facts {
    Facts {
        state: "running".into(),
        shell_live: true,
        identity: Some((ME.into(), pane)),
        cwd: Some("/".into()),
        program: Some("/bin/zsh".into()),
        ..Facts::default()
    }
}

fn agent_caller(name: &str, instance: &str) -> Caller {
    Caller {
        name: Some(name.into()),
        corral_instance: Some(instance.into()),
        ..Caller::default()
    }
}

fn shell_caller(instance: &str, pane: PaneId) -> Caller {
    Caller {
        paddock_instance: Some(instance.into()),
        pane: Some(pane),
        ..Caller::default()
    }
}

fn open(relative_to: &str, place: Place, content: Content) -> Operation {
    Operation::Open {
        relative_to: relative_to.into(),
        place,
        content,
        focus: false,
    }
}

fn shell() -> Content {
    Content::Shell { cwd: None }
}

fn reply(plan: Plan) -> Value {
    match plan {
        Plan::Reply(value) => value,
        other => panic!("expected a reply, got {other:?}"),
    }
}

#[test]
fn the_caller_is_found_by_its_corral_identity_or_its_shell_identity() {
    let f = Fixture::new();
    let m = f.model();
    assert_eq!(caller_pane(&m, &agent_caller("p/a", "i-a")), Ok(2));
    assert_eq!(caller_pane(&m, &shell_caller(ME, 1)), Ok(1));
    // Corral identity first, even with an inherited shell hint.
    let mut both = agent_caller("p/a", "i-a");
    both.paddock_instance = Some(ME.into());
    both.pane = Some(1);
    assert_eq!(caller_pane(&m, &both), Ok(2));
}

#[test]
fn an_unknown_or_mismatched_caller_is_not_guessed() {
    let mut f = Fixture::new();
    for caller in [
        Caller::default(),
        // Another instance of the same agent (restarted under its name).
        agent_caller("p/a", "i-old"),
        // Half an identity.
        Caller {
            name: Some("p/a".into()),
            ..Caller::default()
        },
        // Listed, but not shown here.
        agent_caller("p/b", "i-b"),
        // Another paddock, a pane that is not open, a pane that is not a shell.
        shell_caller("fedcba9876543210", 1),
        shell_caller(ME, 99),
        shell_caller(ME, 2),
        Caller {
            paddock_instance: Some(ME.into()),
            ..Caller::default()
        },
    ] {
        assert!(caller_pane(&f.model(), &caller).is_err(), "{caller:?}");
    }
    // The agent's attach ended: its pane no longer stands for it.
    f.facts.get_mut(&2).unwrap().attached = None;
    assert!(caller_pane(&f.model(), &agent_caller("p/a", "i-a")).is_err());
    // A shell that has exited.
    f.facts.get_mut(&1).unwrap().shell_live = false;
    assert!(caller_pane(&f.model(), &shell_caller(ME, 1)).is_err());
}

#[test]
fn a_shell_identity_stops_working_once_its_pane_shows_something_else() {
    let mut f = Fixture::new();
    // The pane now shows an agent (its shell had exited).
    f.workspace.set_shown(1, Shown::Agent("p/b".into()));
    assert!(caller_pane(&f.model(), &shell_caller(ME, 1)).is_err());

    // Empty again, and a new shell started there: the pane gets a new ID, the old one is gone.
    f.workspace.set_shown(1, Shown::Empty);
    let renewed = f.workspace.renew(1, Shown::Shell);
    f.facts.remove(&1);
    f.facts.insert(renewed, shell_facts(renewed));
    assert!(caller_pane(&f.model(), &shell_caller(ME, 1)).is_err());
    assert_eq!(
        caller_pane(&f.model(), &shell_caller(ME, renewed)),
        Ok(renewed)
    );

    // A shell whose identity names another pane is not this pane's.
    f.facts.get_mut(&renewed).unwrap().identity = Some((ME.into(), 1));
    assert!(caller_pane(&f.model(), &shell_caller(ME, renewed)).is_err());
}

#[test]
fn inspect_lists_tabs_panes_layout_and_the_caller_when_known() {
    let f = Fixture::new();
    let value = inspect(&f.model(), &shell_caller(ME, 1));
    assert_eq!(value["instance"], ME);
    assert_eq!(value["active_pane"], 1);
    assert_eq!(value["active_tab"], f.workspace.tabs[0].id);
    assert_eq!(value["caller"], json!({"pane": 1, "revision": 1}));
    let tabs = value["tabs"].as_array().unwrap();
    assert_eq!(tabs.len(), 2);
    assert_eq!(
        tabs[0]["layout"],
        json!({"split": "row", "ratio": 0.5, "first": {"pane": 1}, "second": {"pane": 2}})
    );
    let panes = tabs[0]["panes"].as_array().unwrap();
    assert_eq!(panes[0]["kind"], "shell");
    assert_eq!(panes[0]["cwd"], "/");
    assert_eq!(panes[0]["shell"]["program"], "/bin/zsh");
    assert_eq!(panes[1]["kind"], "agent");
    assert_eq!(panes[1]["agent"], "p/a");
    assert_eq!(panes[1]["corral_instance"], "i-a");
    assert_eq!(tabs[1]["panes"][0]["kind"], "empty");

    let unknown = inspect(&f.model(), &Caller::default());
    assert!(unknown["caller"]["pane"].is_null());
    assert_eq!(unknown["caller"]["error"]["code"], "caller_unresolved");
}

#[test]
fn open_goes_beside_self_active_or_a_pane_id_in_all_five_places() {
    let f = Fixture::new();
    let places = [
        (Place::Tab, At::Tab),
        (Place::Left, At::Side(Direction::Left)),
        (Place::Right, At::Side(Direction::Right)),
        (Place::Up, At::Side(Direction::Up)),
        (Place::Down, At::Side(Direction::Down)),
    ];
    for (place, expected) in places {
        for (relative_to, caller, anchor) in [
            ("self", agent_caller("p/a", "i-a"), 2),
            ("self", shell_caller(ME, 1), 1),
            ("active", Caller::default(), 1),
            ("3", Caller::default(), 3),
        ] {
            let plan = f.plan(open(relative_to, place, shell()), caller, &[]);
            let Plan::Shell {
                anchor: got, at, ..
            } = plan
            else {
                panic!("{plan:?}")
            };
            assert_eq!((got, at), (anchor, expected), "{relative_to} {place:?}");
        }
    }
    // Self without an identity, or a pane that is not open: nothing opens.
    let unknown = reply(f.plan(open("self", Place::Right, shell()), Caller::default(), &[]));
    assert_eq!(unknown["error"]["code"], "caller_unresolved");
    let gone = reply(f.plan(open("42", Place::Right, shell()), Caller::default(), &[]));
    assert_eq!(gone["error"]["code"], "invalid_target");
}

#[test]
fn a_shell_starts_where_the_source_pane_is_known_to_be() {
    let mut f = Fixture::new();
    f.facts.get_mut(&1).unwrap().cwd = Some("/tmp".into());
    let from = |f: &Fixture, relative_to: &str, cwd: Option<&str>| match f.plan(
        open(
            relative_to,
            Place::Down,
            Content::Shell {
                cwd: cwd.map(str::to_owned),
            },
        ),
        Caller::default(),
        &[],
    ) {
        Plan::Shell {
            cwd, cwd_source, ..
        } => (cwd, cwd_source),
        other => panic!("{other:?}"),
    };
    assert_eq!(from(&f, "1", None), ("/tmp".into(), "source_pane"));
    assert_eq!(from(&f, "2", None), ("/".into(), "source_pane"));
    assert_eq!(from(&f, "3", None), ("/".into(), "startup_directory"));
    assert_eq!(from(&f, "1", Some("/usr")), ("/usr".into(), "explicit"));
    let bad = reply(f.plan(
        open(
            "1",
            Place::Down,
            Content::Shell {
                cwd: Some("relative".into()),
            },
        ),
        Caller::default(),
        &[],
    ));
    assert_eq!(bad["state"], "failed");
}

#[test]
fn an_agent_already_shown_moves_instead_of_attaching_again() {
    let mut f = Fixture::new();
    let agent = |name: &str| Content::Agent { name: name.into() };
    assert_eq!(
        f.plan(
            open("3", Place::Right, agent("p/a")),
            Caller::default(),
            &[]
        ),
        Plan::Move {
            name: "p/a".into(),
            pane: 2,
            anchor: 3,
            at: At::Side(Direction::Right),
            reattach: false,
            focus: false,
        }
    );
    assert_eq!(
        f.plan(open("3", Place::Tab, agent("p/b")), Caller::default(), &[]),
        Plan::Attach {
            name: "p/b".into(),
            cwd: Some("/tmp".into()),
            instance: Some("i-b".into()),
            anchor: 3,
            at: At::Tab,
            focus: false,
        }
    );
    let missing = reply(f.plan(open("3", Place::Tab, agent("p/zz")), Caller::default(), &[]));
    assert_eq!(missing["error"]["code"], "invalid_target");
    // Shown, but its attach ended: it moves and attaches once more.
    f.facts.get_mut(&2).unwrap().attached = None;
    f.facts.get_mut(&2).unwrap().state = "disconnected".into();
    assert!(matches!(
        f.plan(
            open("3", Place::Right, agent("p/a")),
            Caller::default(),
            &[]
        ),
        Plan::Move { reattach: true, .. }
    ));
}

#[test]
fn a_new_agent_starts_through_corral_with_a_prompt_only_when_given() {
    let f = Fixture::new();
    let new = |prompt: Option<&str>, role: &str| Content::NewAgent {
        name: "p/new".into(),
        cwd: None,
        role: role.into(),
        prompt: prompt.map(str::to_owned),
        argv: vec!["codex".into(), "--yolo".into()],
    };
    let Plan::Start { args, cwd, .. } = f.plan(
        open("1", Place::Tab, new(None, "reviewer")),
        Caller::default(),
        &[],
    ) else {
        panic!()
    };
    assert_eq!(cwd, "/");
    assert_eq!(
        args,
        [
            "start",
            "p/new",
            "--cwd",
            "/",
            "--label",
            "role=reviewer",
            "--",
            "codex",
            "--yolo"
        ]
    );
    let Plan::Start { args, .. } = f.plan(
        open("1", Place::Tab, new(Some("hello"), "regular")),
        Caller::default(),
        &[],
    ) else {
        panic!()
    };
    assert!(args.windows(2).any(|w| w == ["--prompt", "hello"]));
    let bad = reply(f.plan(
        open("1", Place::Tab, new(None, "boss")),
        Caller::default(),
        &[],
    ));
    assert_eq!(bad["state"], "failed");
}

fn close(target: CloseTarget, confirmation: Option<&str>) -> Operation {
    Operation::Close {
        target,
        confirmation: confirmation.map(str::to_owned),
        confirm_shells: confirmation.is_some(),
    }
}

#[test]
fn closing_running_shells_waits_for_a_one_time_confirmation() {
    let f = Fixture::new();
    let tab = CloseTarget::Tab(f.workspace.tabs[0].id);
    let mut used = HashSet::new();
    // First: what would end, and a token; nothing closes.
    let asked = reply(f.plan_used(close(tab, None), Caller::default(), &[], &mut used));
    assert_eq!(asked["state"], "confirmation_required");
    assert_eq!(asked["ok"], true);
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    let targets = asked["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0]["running_shell"], true);
    assert_eq!(targets[1]["agent"], "p/a");
    let records = [asked.clone()];

    // Both flags are needed.
    let half = Operation::Close {
        target: tab,
        confirmation: Some(token.clone()),
        confirm_shells: false,
    };
    assert_eq!(
        reply(f.plan_used(half, Caller::default(), &records, &mut used))["error"]["code"],
        "invalid_request"
    );
    // Confirmed: it closes, once.
    assert_eq!(
        f.plan_used(
            close(tab, Some(&token)),
            Caller::default(),
            &records,
            &mut used
        ),
        Plan::Close {
            target: tab,
            panes: vec![1, 2]
        }
    );
    let again = reply(f.plan_used(
        close(tab, Some(&token)),
        Caller::default(),
        &records,
        &mut used,
    ));
    assert_eq!(again["error"]["code"], "confirmation_invalid");
    // A token this instance never gave.
    let made_up = reply(f.plan_used(
        close(tab, Some("guess")),
        Caller::default(),
        &records,
        &mut used,
    ));
    assert_eq!(made_up["error"]["code"], "confirmation_invalid");
    // Without shells it closes at once.
    let empty = CloseTarget::Pane(3);
    assert_eq!(
        f.plan(close(empty, None), Caller::default(), &[]),
        Plan::Close {
            target: empty,
            panes: vec![3]
        }
    );
}

#[test]
fn a_confirmation_is_void_once_its_target_changed() {
    let mut f = Fixture::new();
    let target = CloseTarget::Pane(1);
    let mut used = HashSet::new();
    let asked = reply(f.plan_used(close(target, None), Caller::default(), &[], &mut used));
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    // Another pane's token does not close this one.
    let other = reply(f.plan_used(
        close(CloseTarget::Pane(2), Some(&token)),
        Caller::default(),
        std::slice::from_ref(&asked),
        &mut HashSet::new(),
    ));
    assert_eq!(other["error"]["code"], "confirmation_invalid");
    // The pane's content changed (its revision rose): the token is void, now and later.
    f.workspace.set_shown(1, Shown::Shell);
    let changed = reply(f.plan_used(
        close(target, Some(&token)),
        Caller::default(),
        std::slice::from_ref(&asked),
        &mut used,
    ));
    assert_eq!(changed["error"]["code"], "confirmation_invalid");
    assert!(used.contains(&token));
    // Its shell changing state counts too.
    let f = Fixture::new();
    let mut used = HashSet::new();
    let asked = reply(f.plan_used(close(target, None), Caller::default(), &[], &mut used));
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    let mut g = Fixture::new();
    g.facts.get_mut(&1).unwrap().shell_live = false;
    let ended = reply(g.plan_used(
        close(target, Some(&token)),
        Caller::default(),
        std::slice::from_ref(&asked),
        &mut used,
    ));
    assert_eq!(ended["error"]["code"], "confirmation_invalid");
}

#[test]
fn busy_answers_before_any_change_and_records_nothing() {
    let mut f = Fixture::new();
    let tab = CloseTarget::Tab(f.workspace.tabs[0].id);
    let asked = reply(f.plan(close(tab, None), Caller::default(), &[]));
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    f.busy = Some("dragging a divider");
    let mut used = HashSet::new();
    for operation in [
        open("active", Place::Right, shell()),
        close(CloseTarget::Pane(3), None),
        close(tab, Some(&token)),
        Operation::Browse {
            url: "http://127.0.0.1:8000".into(),
            focus: false,
        },
    ] {
        let value = reply(f.plan_used(
            operation,
            Caller::default(),
            std::slice::from_ref(&asked),
            &mut used,
        ));
        assert_eq!(value["error"]["code"], "busy");
        assert_eq!(value["state"], "busy");
    }
    // The token was not spent: once the user is done it still works.
    assert!(used.is_empty());
    f.busy = None;
    assert!(matches!(
        f.plan_used(
            close(tab, Some(&token)),
            Caller::default(),
            &[asked],
            &mut used
        ),
        Plan::Close { .. }
    ));
}

#[test]
fn browse_takes_only_http_and_https() {
    let f = Fixture::new();
    for url in ["http://127.0.0.1:3000/x", "HTTPS://example.com"] {
        assert!(matches!(
            f.plan(
                Operation::Browse {
                    url: url.into(),
                    focus: true
                },
                Caller::default(),
                &[]
            ),
            Plan::Browse { focus: true, .. }
        ));
    }
    for url in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "ftp://example.com",
    ] {
        let value = reply(f.plan(
            Operation::Browse {
                url: url.into(),
                focus: false,
            },
            Caller::default(),
            &[],
        ));
        assert_eq!(value["error"]["code"], "invalid_request", "{url}");
    }
}

#[test]
fn two_changes_in_a_row_each_see_the_other() {
    let mut f = Fixture::new();
    let mut used = HashSet::new();
    // Two opens beside the same pane: two panes, the second beside the first's neighbour.
    let mut opened = Vec::new();
    for _ in 0..2 {
        let Plan::Shell { anchor, at, .. } = f.plan_used(
            open("1", Place::Down, shell()),
            Caller::default(),
            &[],
            &mut used,
        ) else {
            panic!()
        };
        let pane = f.workspace.open_at(anchor, at, Shown::Shell).unwrap();
        f.facts.insert(pane, shell_facts(pane));
        opened.push(pane);
    }
    assert_ne!(opened[0], opened[1]);
    assert_eq!(f.workspace.tabs[0].panes(), [1, opened[1], opened[0], 2]);
    // Two confirmed closes with one token: only the first.
    let target = CloseTarget::Pane(opened[0]);
    let asked = reply(f.plan_used(close(target, None), Caller::default(), &[], &mut used));
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    let records = [asked];
    let first = f.plan_used(
        close(target, Some(&token)),
        Caller::default(),
        &records,
        &mut used,
    );
    let second = f.plan_used(
        close(target, Some(&token)),
        Caller::default(),
        &records,
        &mut used,
    );
    assert!(matches!(first, Plan::Close { .. }));
    assert_eq!(reply(second)["error"]["code"], "confirmation_invalid");
}

#[test]
fn a_request_in_progress_ends_complete_failed_or_target_invalid() {
    let mut f = Fixture::new();
    let pane = f
        .workspace
        .open_at(1, At::Side(Direction::Down), Shown::Agent("p/b".into()))
        .unwrap();
    let track = Track {
        request: "r".into(),
        pane,
        revision: f.workspace.revision(pane).unwrap(),
        agent: Some("p/b".into()),
        instance: None,
        value: json!({"ok": true, "state": "attaching", "pane": pane}),
    };
    let mut facts = Facts {
        state: "attaching".into(),
        ..Facts::default()
    };
    assert_eq!(progress(&track, &f.workspace, Some(&facts)), None);
    facts.state = "running".into();
    facts.attached = Some("p/b".into());
    let done = progress(&track, &f.workspace, Some(&facts)).unwrap();
    assert_eq!(
        (done["state"].as_str(), done["ok"].as_bool()),
        (Some("complete"), Some(true))
    );
    facts = Facts {
        state: "failed".into(),
        note: "agent identity changed before attach".into(),
        ..Facts::default()
    };
    let bad = progress(&track, &f.workspace, Some(&facts)).unwrap();
    assert_eq!(bad["state"], "failed");
    assert_eq!(
        bad["error"]["message"],
        "agent identity changed before attach"
    );

    // Given something else meanwhile, or closed while the request was on its way.
    f.workspace.set_shown(pane, Shown::Empty);
    let replaced = progress(&track, &f.workspace, None).unwrap();
    assert_eq!(replaced["state"], "target_invalid");
    let mut g = Fixture::new();
    let pane = g.workspace.open_at(1, At::Tab, Shown::Shell).unwrap();
    let mut track = Track {
        request: "r".into(),
        pane,
        revision: 1,
        agent: None,
        instance: None,
        value: json!({"ok": true, "state": "starting", "agent_created": {"name": "p/new"}}),
    };
    let starting = Facts {
        state: "starting".into(),
        ..Facts::default()
    };
    assert_eq!(progress(&track, &g.workspace, Some(&starting)), None);
    g.workspace.close_pane(pane);
    track.value["pane"] = json!(pane);
    let closed = progress(&track, &g.workspace, Some(&starting)).unwrap();
    assert_eq!(closed["state"], "target_invalid");
    assert_eq!(closed["ok"], false);
    assert!(
        closed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("keeps running")
    );
}

#[test]
fn a_confirmation_is_void_once_its_agent_is_another_instance_of_the_same_name() {
    let mut f = Fixture::new();
    let tab = CloseTarget::Tab(f.workspace.tabs[0].id);
    let mut used = HashSet::new();
    let asked = reply(f.plan_used(close(tab, None), Caller::default(), &[], &mut used));
    let token = asked["confirmation"].as_str().unwrap().to_owned();
    // p/a ended and its new instance, same name and directory, shows in the same pane.
    f.facts.get_mut(&2).unwrap().instance = Some("i-new".into());
    let changed = reply(f.plan_used(
        close(tab, Some(&token)),
        Caller::default(),
        &[asked],
        &mut used,
    ));
    assert_eq!(changed["error"]["code"], "confirmation_invalid");
}

#[test]
fn a_request_follows_only_the_instance_it_attached() {
    let mut f = Fixture::new();
    let pane = f
        .workspace
        .open_at(1, At::Side(Direction::Down), Shown::Agent("p/b".into()))
        .unwrap();
    let track = Track {
        request: "r".into(),
        pane,
        revision: f.workspace.revision(pane).unwrap(),
        agent: Some("p/b".into()),
        instance: Some("i-old".into()),
        value: json!({"ok": true, "state": "attaching", "agent_created": {"name": "p/b", "instance": "i-old"}}),
    };
    // The same pane now shows another instance under the name: not this request's result.
    let other = Facts {
        state: "running".into(),
        attached: Some("p/b".into()),
        instance: Some("i-new".into()),
        ..Facts::default()
    };
    let value = progress(&track, &f.workspace, Some(&other)).unwrap();
    assert_eq!(value["state"], "target_invalid");
    // A new attach is a new revision, so a request about the old one ends there too.
    f.workspace.touch(pane);
    let mut same = other.clone();
    same.instance = Some("i-old".into());
    assert_eq!(
        progress(&track, &f.workspace, Some(&same)).unwrap()["state"],
        "target_invalid"
    );
}

#[test]
fn the_main_windows_sheets_and_kanban_confirmations_make_it_busy() {
    for activity in [
        Activity {
            sheet: true,
            ..Activity::default()
        },
        Activity {
            kanban_confirming: true,
            ..Activity::default()
        },
        Activity {
            dragging: true,
            ..Activity::default()
        },
        Activity {
            asking: true,
            ..Activity::default()
        },
        Activity {
            popup: true,
            ..Activity::default()
        },
        Activity {
            creating: true,
            ..Activity::default()
        },
        Activity {
            quitting: true,
            ..Activity::default()
        },
    ] {
        let mut f = Fixture::new();
        let tab = CloseTarget::Tab(f.workspace.tabs[0].id);
        let asked = reply(f.plan(close(tab, None), Caller::default(), &[]));
        let token = asked["confirmation"].as_str().unwrap().to_owned();
        f.busy = busy_reason(&activity);
        let mut used = HashSet::new();
        let value = reply(f.plan_used(
            close(tab, Some(&token)),
            Caller::default(),
            &[asked],
            &mut used,
        ));
        assert_eq!(value["state"], "busy", "{activity:?}");
        assert!(used.is_empty(), "{activity:?}");
        assert_eq!(f.workspace.tabs[0].panes(), [1, 2], "{activity:?}");
    }
    assert_eq!(busy_reason(&Activity::default()), None);
}

#[test]
fn a_late_start_reuses_the_pane_of_that_instance_and_refuses_another() {
    let mut f = Fixture::new();
    assert_eq!(
        landing(&f.workspace, &f.facts, "p/new", Some("i-new")),
        Landing::Open
    );
    // While corral was starting it, the agent was opened here (from the sidebar or `--agent`).
    let pane = f
        .workspace
        .open_at(3, At::Tab, Shown::Agent("p/new".into()))
        .unwrap();
    f.facts.insert(
        pane,
        Facts {
            state: "attaching".into(),
            instance: Some("i-new".into()),
            ..Facts::default()
        },
    );
    assert_eq!(
        landing(&f.workspace, &f.facts, "p/new", Some("i-new")),
        Landing::Reuse(pane)
    );
    assert_eq!(
        landing(&f.workspace, &f.facts, "p/new", Some("i-other")),
        Landing::Conflict(pane)
    );
    assert_eq!(
        landing(&f.workspace, &f.facts, "p/new", None),
        Landing::Conflict(pane)
    );
}
