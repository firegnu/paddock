//! A controller's Delegates setting (DESIGN §13 P5-141): which fence profile the agents it starts
//! from now on run in, or none. corral keeps it (`corral child-sandbox`, R18) for one run of the
//! controller and says it in `corral ls` as `child_sandbox`; paddock only shows it and asks corral
//! to change it. Here is what the card shows, and each choice being saved or refused until the
//! listing has caught up with it. Drawing is `sidebar_delegates.rs`'s.
use crate::corral::{Agent, ChildSandbox, Client, Refused, Role, Sandbox};
use std::{collections::HashMap, sync::atomic::AtomicBool, time::Instant};

/// A controller's Delegates setting, as its card shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Delegates {
    /// The profile the agents it starts run in, as corral says; none when they start outside.
    pub profile: Option<String>,
    /// A choice is being saved: the cell says so and its menu does not open.
    pub saving: bool,
    /// corral turned the last choice down, in its own words; the setting stayed as it was.
    pub refused: Option<Refused>,
}

impl Delegates {
    /// The cell's words: `Saving…`, `In sandbox · dev` or `Not sandboxed`.
    pub fn value(&self) -> String {
        match (&self.profile, self.saving) {
            (_, true) => "Saving…".into(),
            (Some(profile), false) => format!("In sandbox · {profile}"),
            (None, false) => "Not sandboxed".into(),
        }
    }

    /// The collapsed card's mark says this on hover; none, and no mark, while it is not set.
    pub fn tip(&self) -> Option<String> {
        let profile = self.profile.as_ref()?;
        Some(format!("Agents it starts run in sandbox: {profile}"))
    }

    /// corral's refusal as the cell shows it, in red: its code, then its sentence.
    pub fn refusal(&self) -> Option<String> {
        let refused = self.refused.as_ref()?;
        Some(match &refused.code {
            Some(code) if *code != refused.message => format!("{code}: {}", refused.message),
            _ => refused.message.clone(),
        })
    }
}

/// `a`'s Delegates setting, for its card: only a controller's, only with fence on this Mac
/// (`fence` found: another machine's list never has it), and only when corral says it at all
/// (an older corral leaves `child_sandbox` out).
pub fn of(a: &Agent, fence: bool, saves: &Saves) -> Option<Delegates> {
    if a.role() != Some(Role::Controller) || !fence {
        return None;
    }
    let child = a.child_sandbox.as_ref()?;
    let save = saves.request(a);
    Some(Delegates {
        profile: child.as_ref().map(|sandbox| sandbox.profile.clone()),
        saving: matches!(save, Some(Request::Busy)),
        refused: match save {
            Some(Request::Refused(refused)) => Some(refused.clone()),
            _ => None,
        },
    })
}

/// A pick in a controller's Delegates menu, to ask corral: for which run it was made, and the
/// profile (none: `Not sandboxed`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub name: String,
    pub instance: Option<String>,
    pub profile: Option<String>,
}

impl Pick {
    /// Asks corral; blocks, so off the UI thread.
    pub fn send(&self, corral: &Client, cancel: &AtomicBool) -> Result<ChildSandbox, Refused> {
        corral.set_child_sandbox(&self.name, self.profile.as_deref(), cancel)
    }
}

/// What picking `profile` in the menu opened for `name`'s run `instance` asks corral, the choice
/// then begun in `saves`. None when the name is listed with another run now (the menu was for one
/// gone), while a choice is being saved, or when it is the setting already.
pub fn pick(
    agents: &[Agent],
    saves: &mut Saves,
    name: &str,
    instance: Option<&str>,
    profile: Option<String>,
) -> Option<Pick> {
    let agent = agents
        .iter()
        .find(|a| a.name == name && a.instance.as_deref() == instance)?;
    let wanted = profile.clone().map(|profile| Sandbox { profile });
    if saves.busy(agent) || agent.child_sandbox.as_ref() == Some(&wanted) {
        return None;
    }
    saves.begin(agent);
    Some(Pick {
        name: name.to_owned(),
        instance: agent.instance.clone(),
        profile,
    })
}

/// The choice asked of corral now, while it is.
#[derive(Clone, Debug, PartialEq)]
enum Request {
    /// corral has been asked.
    Busy,
    /// corral turned it down.
    Refused(Refused),
}

/// What paddock knows of one run of a controller: the setting corral last took and when, which
/// a listing read before then has not caught up with; and the choice asked now.
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    instance: Option<String>,
    confirmed: Option<(Option<Sandbox>, Instant)>,
    request: Option<Request>,
}

/// The choices being saved, refused or just saved, by controller name, each for the run it was
/// made for: one for a run no longer listed (the controller stopped or started again) is
/// forgotten, as corral forgets the setting with the run.
#[derive(Clone, Debug, Default)]
pub struct Saves(HashMap<String, Entry>);

impl Saves {
    fn of(&self, a: &Agent) -> Option<&Entry> {
        self.0
            .get(&a.name)
            .filter(|entry| entry.instance == a.instance)
    }

    fn request(&self, a: &Agent) -> Option<&Request> {
        self.of(a)?.request.as_ref()
    }

    /// A choice for `a` is being saved.
    pub fn busy(&self, a: &Agent) -> bool {
        matches!(self.request(a), Some(Request::Busy))
    }

    /// corral has been asked to change `a`'s setting; what it said of the choice before is gone,
    /// but a setting it took before still stands over listings older than it.
    pub fn begin(&mut self, a: &Agent) {
        let entry = self.0.entry(a.name.clone()).or_insert_with(|| Entry {
            instance: a.instance.clone(),
            confirmed: None,
            request: None,
        });
        if entry.instance != a.instance {
            entry.instance.clone_from(&a.instance);
            entry.confirmed = None;
        }
        entry.request = Some(Request::Busy);
    }

    /// corral's answer to the choice begun for `name`'s run `instance`, come at `at`. Taken, it
    /// shows on the card at once, in `agents`, without waiting for the next listing; turned down,
    /// the card says why and keeps the setting. An answer for a choice no longer waited for (its
    /// run gone meanwhile) is let go; one for another run than asked about drops what was kept,
    /// for the next listing to say.
    pub fn answer(
        &mut self,
        name: &str,
        instance: Option<&str>,
        answer: Result<ChildSandbox, Refused>,
        agents: &mut [Agent],
        at: Instant,
    ) {
        let Some(entry) = self.0.get_mut(name) else {
            return;
        };
        if entry.instance.as_deref() != instance || entry.request != Some(Request::Busy) {
            return;
        }
        match answer {
            Ok(told) => {
                if Some(told.instance.as_str()) != instance {
                    self.0.remove(name);
                    return;
                }
                let shown = agents
                    .iter_mut()
                    .find(|a| a.name == name && a.instance.as_deref() == instance);
                if let Some(agent) = shown {
                    agent.child_sandbox = Some(told.child_sandbox.clone());
                }
                entry.confirmed = Some((told.child_sandbox, at));
                entry.request = None;
            }
            Err(refused) => entry.request = Some(Request::Refused(refused)),
        }
    }

    /// Takes in a listing read from `began` on: forgets the runs no longer in it, and a setting
    /// taken before the read began, which it has caught up with; one taken after it began stands
    /// over what it says, so the card does not flick back.
    pub fn settle(&mut self, agents: &mut [Agent], began: Instant) {
        self.0.retain(|name, entry| {
            let Some(agent) = agents
                .iter_mut()
                .find(|a| a.name == *name && a.instance == entry.instance)
            else {
                return false;
            };
            match &entry.confirmed {
                Some((_, at)) if *at <= began => entry.confirmed = None,
                Some((child, _)) => agent.child_sandbox = Some(child.clone()),
                None => {}
            }
            entry.confirmed.is_some() || entry.request.is_some()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn controller(child: Option<Option<&str>>) -> Agent {
        let mut labels = serde_json::Map::new();
        labels.insert("role".into(), "controller".into());
        Agent {
            name: "p/main".into(),
            instance: Some("i-1".into()),
            labels,
            child_sandbox: child.map(|child| {
                child.map(|profile| Sandbox {
                    profile: profile.into(),
                })
            }),
            ..Agent::default()
        }
    }

    fn dev() -> Option<Sandbox> {
        Some(Sandbox {
            profile: "dev".into(),
        })
    }

    #[test]
    fn only_a_controller_on_this_mac_with_a_corral_that_says_it() {
        let saves = Saves::default();
        let shown = |a: &Agent, fence| of(a, fence, &saves);
        assert_eq!(
            shown(&controller(Some(None)), true),
            Some(Delegates {
                profile: None,
                saving: false,
                refused: None,
            })
        );
        let set = shown(&controller(Some(Some("dev"))), true).unwrap();
        assert_eq!(set.profile.as_deref(), Some("dev"));
        assert_eq!(set.value(), "In sandbox · dev");
        assert_eq!(
            set.tip().as_deref(),
            Some("Agents it starts run in sandbox: dev")
        );
        let unset = shown(&controller(Some(None)), true).unwrap();
        assert_eq!(
            (unset.value().as_str(), unset.tip()),
            ("Not sandboxed", None)
        );
        // An older corral leaves the field out: nothing to show.
        assert_eq!(shown(&controller(None), true), None);
        // No fence here, as for another machine's list: none either.
        assert_eq!(shown(&controller(Some(Some("dev"))), false), None);
        // Nor on an agent that is not a controller.
        for role in [None, Some("regular"), Some("implementer"), Some("test")] {
            let mut agent = controller(Some(Some("dev")));
            agent.labels.clear();
            if let Some(role) = role {
                agent.labels.insert("role".into(), role.into());
            }
            assert_eq!(shown(&agent, true), None, "{role:?}");
        }
    }

    #[test]
    fn a_choice_saves_then_shows_at_once() {
        let mut agents = vec![controller(Some(None))];
        let mut saves = Saves::default();
        saves.begin(&agents[0]);
        let card = of(&agents[0], true, &saves).unwrap();
        assert!(card.saving && saves.busy(&agents[0]));
        assert_eq!(card.value(), "Saving…");

        let at = Instant::now();
        let told = ChildSandbox {
            instance: "i-1".into(),
            child_sandbox: dev(),
        };
        saves.answer("p/main", Some("i-1"), Ok(told), &mut agents, at);
        assert_eq!(agents[0].child_sandbox, Some(dev()));
        let card = of(&agents[0], true, &saves).unwrap();
        assert_eq!(
            (card.saving, card.value()),
            (false, "In sandbox · dev".into())
        );

        // A listing read before it was saved still says the old setting: the card keeps the new.
        let mut stale = vec![controller(Some(None))];
        saves.settle(&mut stale, at - Duration::from_millis(500));
        assert_eq!(stale[0].child_sandbox, Some(dev()));
        // One read after it was saved is corral's word, whatever it says.
        let mut fresh = vec![controller(Some(None))];
        saves.settle(&mut fresh, at + Duration::from_millis(1));
        assert_eq!(fresh[0].child_sandbox, Some(None));
        let mut later = vec![controller(Some(None))];
        saves.settle(&mut later, at - Duration::from_millis(500));
        assert_eq!(later[0].child_sandbox, Some(None));
    }

    #[test]
    fn a_refusal_is_shown_word_for_word_and_the_setting_stays() {
        let mut agents = vec![controller(Some(Some("dev")))];
        let mut saves = Saves::default();
        saves.begin(&agents[0]);
        let refused = Refused {
            code: Some("sandbox_failed".into()),
            message: "fence check failed: unknown_profile".into(),
        };
        saves.answer(
            "p/main",
            Some("i-1"),
            Err(refused.clone()),
            &mut agents,
            Instant::now(),
        );
        assert_eq!(agents[0].child_sandbox, Some(dev()));
        let card = of(&agents[0], true, &saves).unwrap();
        assert_eq!(card.refused, Some(refused));
        assert_eq!(card.value(), "In sandbox · dev");
        assert_eq!(
            card.refusal().as_deref(),
            Some("sandbox_failed: fence check failed: unknown_profile")
        );
        // Listings keep it up while the run lasts…
        saves.settle(&mut agents, Instant::now());
        assert!(of(&agents[0], true, &saves).unwrap().refused.is_some());
        // …and the next choice takes it away.
        saves.begin(&agents[0]);
        assert_eq!(of(&agents[0], true, &saves).unwrap().refused, None);
        // When corral had no code, its sentence alone.
        let bare = Delegates {
            profile: None,
            saving: false,
            refused: Some(Refused {
                code: None,
                message: "corral did not answer in time".into(),
            }),
        };
        assert_eq!(
            bare.refusal().as_deref(),
            Some("corral did not answer in time")
        );
    }

    /// Not set, then `dev` saved at the instant returned; then a second choice begun.
    fn saved_dev_then_begun_again() -> (Vec<Agent>, Saves, Instant) {
        let mut agents = vec![controller(Some(None))];
        let mut saves = Saves::default();
        saves.begin(&agents[0]);
        let first = Instant::now();
        let told = ChildSandbox {
            instance: "i-1".into(),
            child_sandbox: dev(),
        };
        saves.answer("p/main", Some("i-1"), Ok(told), &mut agents, first);
        saves.begin(&agents[0]);
        (agents, saves, first)
    }

    #[test]
    fn a_second_choice_keeps_the_first_one_over_older_reads_when_refused() {
        let (_, mut saves, first) = saved_dev_then_begun_again();
        // A read begun before `dev` was saved still says it was not set.
        let mut agents = vec![controller(Some(None))];
        saves.settle(&mut agents, first - Duration::from_millis(500));
        let card = of(&agents[0], true, &saves).unwrap();
        assert_eq!(card.value(), "Saving…");
        assert_eq!(agents[0].child_sandbox, Some(dev()));
        let refused = Refused {
            code: Some("sandbox_failed".into()),
            message: "second choice refused".into(),
        };
        saves.answer(
            "p/main",
            Some("i-1"),
            Err(refused.clone()),
            &mut agents,
            first + Duration::from_millis(10),
        );
        let card = of(&agents[0], true, &saves).unwrap();
        assert_eq!(card.value(), "In sandbox · dev");
        assert_eq!(card.refused, Some(refused));
        // Another old read after the refusal: still `dev`, still refused.
        let mut older = vec![controller(Some(None))];
        saves.settle(&mut older, first - Duration::from_millis(400));
        let card = of(&older[0], true, &saves).unwrap();
        assert_eq!(card.value(), "In sandbox · dev");
        assert!(card.refused.is_some());
        // A read begun after `dev` was saved is corral's word.
        let mut fresh = vec![controller(Some(Some("dev")))];
        saves.settle(&mut fresh, first + Duration::from_millis(1));
        let card = of(&fresh[0], true, &saves).unwrap();
        assert_eq!(card.value(), "In sandbox · dev");
        assert!(card.refused.is_some());
    }

    #[test]
    fn a_second_choice_saved_stands_over_reads_older_than_it() {
        let (_, mut saves, first) = saved_dev_then_begun_again();
        let mut agents = vec![controller(Some(None))];
        saves.settle(&mut agents, first - Duration::from_millis(500));
        let second = first + Duration::from_millis(100);
        let told = ChildSandbox {
            instance: "i-1".into(),
            child_sandbox: Some(Sandbox {
                profile: "strict".into(),
            }),
        };
        saves.answer("p/main", Some("i-1"), Ok(told), &mut agents, second);
        assert_eq!(
            of(&agents[0], true, &saves).unwrap().value(),
            "In sandbox · strict"
        );
        // Reads begun before either save, or between them, still have an old setting.
        for began in [
            first - Duration::from_millis(400),
            first + Duration::from_millis(50),
        ] {
            let mut old = vec![controller(Some(Some("dev")))];
            saves.settle(&mut old, began);
            assert_eq!(
                of(&old[0], true, &saves).unwrap().value(),
                "In sandbox · strict"
            );
        }
        let mut fresh = vec![controller(Some(Some("strict")))];
        saves.settle(&mut fresh, second + Duration::from_millis(1));
        assert_eq!(
            of(&fresh[0], true, &saves).unwrap().value(),
            "In sandbox · strict"
        );
        // Caught up: corral's word from now on, whatever it says.
        let mut later = vec![controller(Some(None))];
        saves.settle(&mut later, first - Duration::from_millis(400));
        assert_eq!(later[0].child_sandbox, Some(None));
    }

    #[test]
    fn a_second_choice_clearing_stands_over_reads_older_than_it() {
        let (_, mut saves, first) = saved_dev_then_begun_again();
        let mut agents = vec![controller(Some(None))];
        saves.settle(&mut agents, first - Duration::from_millis(500));
        let second = first + Duration::from_millis(100);
        let told = ChildSandbox {
            instance: "i-1".into(),
            child_sandbox: None,
        };
        saves.answer("p/main", Some("i-1"), Ok(told), &mut agents, second);
        assert_eq!(
            of(&agents[0], true, &saves).unwrap().value(),
            "Not sandboxed"
        );
        let mut between = vec![controller(Some(Some("dev")))];
        saves.settle(&mut between, first + Duration::from_millis(50));
        assert_eq!(
            of(&between[0], true, &saves).unwrap().value(),
            "Not sandboxed"
        );
    }

    #[test]
    fn a_menu_for_a_run_gone_asks_corral_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "paddock-delegates-test-{}-pick",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("corral");
        std::fs::write(
            &program,
            "#!/bin/sh\necho \"$@\" >> \"$(dirname \"$0\")/calls\"\n\
             echo '{\"ok\":true,\"name\":\"p/main\",\"instance\":\"i-2\",\"child_sandbox\":{\"profile\":\"dev\"}}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let corral = Client::local(program.display().to_string());
        let no = AtomicBool::new(false);
        let calls = dir.join("calls");
        let send = |pick: Option<Pick>| {
            if let Some(pick) = pick {
                let _ = pick.send(&corral, &no);
            }
        };

        // The menu was opened for run i-1; the name has started again as i-2 since.
        let mut agents = vec![controller(Some(None))];
        agents[0].instance = Some("i-2".into());
        let mut saves = Saves::default();
        send(pick(
            &agents,
            &mut saves,
            "p/main",
            Some("i-1"),
            Some("dev".into()),
        ));
        assert!(!calls.exists(), "corral was asked");
        assert!(!saves.busy(&agents[0]));
        // Gone altogether: nothing either.
        send(pick(
            &[],
            &mut saves,
            "p/main",
            Some("i-2"),
            Some("dev".into()),
        ));
        assert!(!calls.exists(), "corral was asked");
        // The setting it has already: nothing to ask.
        send(pick(&agents, &mut saves, "p/main", Some("i-2"), None));
        assert!(!calls.exists(), "corral was asked");

        // For the run listed: asked once, and a second pick while it saves is not sent.
        let first = pick(
            &agents,
            &mut saves,
            "p/main",
            Some("i-2"),
            Some("dev".into()),
        );
        assert_eq!(
            first,
            Some(Pick {
                name: "p/main".into(),
                instance: Some("i-2".into()),
                profile: Some("dev".into()),
            })
        );
        assert!(saves.busy(&agents[0]));
        assert_eq!(pick(&agents, &mut saves, "p/main", Some("i-2"), None), None);
        send(first);
        assert_eq!(
            std::fs::read_to_string(&calls).unwrap(),
            "child-sandbox p/main dev\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_restarted_controller_forgets_the_choice() {
        let mut agents = vec![controller(Some(None))];
        let mut saves = Saves::default();
        saves.begin(&agents[0]);
        // It started again meanwhile: the listing has another run, with no setting.
        agents[0].instance = Some("i-2".into());
        saves.settle(&mut agents, Instant::now());
        let told = ChildSandbox {
            instance: "i-1".into(),
            child_sandbox: dev(),
        };
        saves.answer("p/main", Some("i-1"), Ok(told), &mut agents, Instant::now());
        assert_eq!(agents[0].child_sandbox, Some(None));
        assert_eq!(
            of(&agents[0], true, &saves),
            Some(Delegates {
                profile: None,
                saving: false,
                refused: None,
            })
        );
        // corral holding it for another run than the one asked about is not shown either.
        saves.begin(&agents[0]);
        let other = ChildSandbox {
            instance: "i-3".into(),
            child_sandbox: dev(),
        };
        saves.answer(
            "p/main",
            Some("i-2"),
            Ok(other),
            &mut agents,
            Instant::now(),
        );
        assert_eq!(agents[0].child_sandbox, Some(None));
        assert!(!saves.busy(&agents[0]));
    }
}
