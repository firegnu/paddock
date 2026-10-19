//! A controller's Delegates setting (DESIGN §13 P5-141): which fence profile the agents it starts
//! from now on run in, or none. corral keeps it (`corral child-sandbox`, R18) for one run of the
//! controller and says it in `corral ls` as `child_sandbox`; paddock only shows it and asks corral
//! to change it. Here is what the card shows, and each choice being saved or refused until the
//! listing has caught up with it. Drawing is `sidebar_delegates.rs`'s.
use crate::corral::{Agent, ChildSandbox, Refused, Role, Sandbox};
use std::{collections::HashMap, time::Instant};

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
    let save = saves.of(a);
    Some(Delegates {
        profile: child.as_ref().map(|sandbox| sandbox.profile.clone()),
        saving: matches!(save, Some(State::Busy)),
        refused: match save {
            Some(State::Refused(refused)) => Some(refused.clone()),
            _ => None,
        },
    })
}

/// What became of a choice.
#[derive(Clone, Debug, PartialEq)]
enum State {
    /// corral has been asked.
    Busy,
    /// corral turned it down.
    Refused(Refused),
    /// corral took it, at `at`: a listing read before then still has the old setting.
    Saved { child: Option<Sandbox>, at: Instant },
}

/// The choices being saved, refused or just saved, by controller name, each for the run it was
/// made for: one for a run no longer listed (the controller stopped or started again) is
/// forgotten, as corral forgets the setting with the run.
#[derive(Clone, Debug, Default)]
pub struct Saves(HashMap<String, (Option<String>, State)>);

impl Saves {
    fn of(&self, a: &Agent) -> Option<&State> {
        self.0
            .get(&a.name)
            .filter(|(instance, _)| *instance == a.instance)
            .map(|(_, state)| state)
    }

    /// A choice for `a` is being saved.
    pub fn busy(&self, a: &Agent) -> bool {
        matches!(self.of(a), Some(State::Busy))
    }

    /// corral has been asked to change `a`'s setting; what it said of the one before is gone.
    pub fn begin(&mut self, a: &Agent) {
        self.0
            .insert(a.name.clone(), (a.instance.clone(), State::Busy));
    }

    /// corral's answer to the choice begun for `name`'s run `instance`, come at `at`. Taken, it
    /// shows on the card at once, in `agents`, without waiting for the next listing; turned down,
    /// the card says why and keeps the setting. An answer for a choice no longer waited for (its
    /// run gone meanwhile) is let go.
    pub fn answer(
        &mut self,
        name: &str,
        instance: Option<&str>,
        answer: Result<ChildSandbox, Refused>,
        agents: &mut [Agent],
        at: Instant,
    ) {
        let Some((run, state)) = self.0.get_mut(name) else {
            return;
        };
        if run.as_deref() != instance || *state != State::Busy {
            return;
        }
        match answer {
            Ok(told) => {
                // The run corral holds it for; the one asked about when it does not say.
                if told.instance.is_some() && told.instance.as_deref() != instance {
                    self.0.remove(name);
                    return;
                }
                let shown = agents
                    .iter_mut()
                    .find(|a| a.name == name && a.instance.as_deref() == instance);
                if let Some(agent) = shown {
                    agent.child_sandbox = Some(told.child_sandbox.clone());
                }
                *state = State::Saved {
                    child: told.child_sandbox,
                    at,
                };
            }
            Err(refused) => *state = State::Refused(refused),
        }
    }

    /// Takes in a listing read from `began` on: forgets the choices for runs no longer in it, and
    /// those saved before the read began, which it has caught up with; a choice saved after it
    /// began stands over what it says, so the card does not flick back.
    pub fn settle(&mut self, agents: &mut [Agent], began: Instant) {
        self.0.retain(|name, (instance, state)| {
            let Some(agent) = agents
                .iter_mut()
                .find(|a| a.name == *name && a.instance == *instance)
            else {
                return false;
            };
            match state {
                State::Saved { at, .. } if *at <= began => false,
                State::Saved { child, .. } => {
                    agent.child_sandbox = Some(child.clone());
                    true
                }
                State::Busy | State::Refused(_) => true,
            }
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
            instance: Some("i-1".into()),
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

    #[test]
    fn a_restarted_controller_forgets_the_choice() {
        let mut agents = vec![controller(Some(None))];
        let mut saves = Saves::default();
        saves.begin(&agents[0]);
        // It started again meanwhile: the listing has another run, with no setting.
        agents[0].instance = Some("i-2".into());
        saves.settle(&mut agents, Instant::now());
        let told = ChildSandbox {
            instance: Some("i-1".into()),
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
            instance: Some("i-3".into()),
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
