//! What `paddock ctl` asks of the window (P5-39b), decided without a window so it can be tested:
//! who the caller is, what `inspect` shows, where `open` puts what, what `close` must have
//! confirmed, and how a request still in progress is going. The rules follow Saddle's
//! `src/app_control.rs` (commit `f7d1bbaf`), rewritten for paddock's layout; `window_ctl.rs`
//! carries the plans out on the panes.
use crate::{
    control::{self, Caller, CloseTarget, Content, Message, Operation, Place},
    layout::{At, Axis, Direction, Node, PaneId, Shown, Workspace},
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// A pane's terminal as the window reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// `starting`, `running`, `exited`, `attaching`, `failed`, `disconnected` or `empty`.
    pub state: String,
    /// The agent attached here, its attach running.
    pub attached: Option<String>,
    /// The corral instance the pane attached to, or is attaching to.
    pub instance: Option<String>,
    pub shell_live: bool,
    /// `PADDOCK_INSTANCE` and `PADDOCK_PANE` its shell was started with.
    pub identity: Option<(String, PaneId)>,
    /// Its shell's directory, or its agent's.
    pub cwd: Option<String>,
    /// Its shell's program, or the command it runs.
    pub program: Option<String>,
    pub exit_code: Option<u32>,
    /// A `paddock -- PROGRAM` pane.
    pub command: bool,
    /// What the pane says, for a failed start or attach.
    pub note: String,
}

/// An agent in corral's public listing, as the sidebar last read it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listed {
    pub name: String,
    pub instance: Option<String>,
    pub cwd: Option<String>,
    pub error: bool,
}

/// The window as a request sees it.
pub struct Model<'a> {
    pub instance: &'a str,
    pub workspace: &'a Workspace,
    pub facts: &'a HashMap<PaneId, Facts>,
    pub listed: &'a [Listed],
    /// Where a shell starts when no pane says: paddock's startup directory.
    pub startup_cwd: &'a str,
    /// What the user is in the middle of, when changes would get in the way.
    pub busy: Option<&'static str>,
}

/// What to do for a request.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// Nothing to change: answer this.
    Reply(Value),
    Shell {
        anchor: PaneId,
        at: At,
        cwd: String,
        cwd_source: &'static str,
        focus: bool,
    },
    /// An agent already shown here moves, keeping its attach; `reattach` when that attach has
    /// ended and is not being made again.
    Move {
        name: String,
        pane: PaneId,
        anchor: PaneId,
        at: At,
        reattach: bool,
        focus: bool,
    },
    Attach {
        name: String,
        cwd: Option<String>,
        instance: Option<String>,
        anchor: PaneId,
        at: At,
        focus: bool,
    },
    /// `corral start ARGS`; the pane opens once the agent runs.
    Start {
        args: Vec<String>,
        cwd: String,
        cwd_source: &'static str,
        anchor: PaneId,
        at: At,
        focus: bool,
    },
    Close {
        target: CloseTarget,
        panes: Vec<PaneId>,
    },
    Browse {
        url: String,
        focus: bool,
    },
}

/// A failed request, recorded so `request` can tell.
pub fn failed(code: &str, message: impl ToString) -> Value {
    let mut value = control::error(code, message);
    value["state"] = json!("failed");
    value["accepted"] = json!(false);
    value
}

/// What the user may be in the middle of in the main window, as it is now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Activity {
    /// A divider, a seam or the window is being dragged.
    pub dragging: bool,
    /// paddock's own question (closing shells, stopping, pausing) is showing.
    pub asking: bool,
    /// A system sheet is on the window: a page's alert, confirm or prompt, its file chooser.
    pub sheet: bool,
    /// A panel or menu is open.
    pub popup: bool,
    /// Kanban is asking whether to clear a task.
    pub kanban_confirming: bool,
    /// New Agent is creating an agent.
    pub creating: bool,
    pub quitting: bool,
}

/// Why a change from `paddock ctl` would get in the user's way now, if it would.
pub fn busy_reason(activity: &Activity) -> Option<&'static str> {
    if activity.dragging {
        Some("dragging a divider or the window")
    } else if activity.asking || activity.sheet || activity.kanban_confirming {
        Some("answering a question")
    } else if activity.popup {
        Some("using a panel or menu")
    } else if activity.creating {
        Some("creating an agent in New Agent")
    } else if activity.quitting {
        Some("quitting paddock")
    } else {
        None
    }
}

/// Where the pane of an agent `corral start` has started goes, as the window is when it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Not shown yet: a new pane beside the anchor.
    Open,
    /// This pane already shows that very instance: it moves there and is followed.
    Reuse(PaneId),
    /// This pane shows another instance under the name.
    Conflict(PaneId),
}

/// [`Landing`] for `name`, started as `instance`.
pub fn landing(
    workspace: &Workspace,
    facts: &HashMap<PaneId, Facts>,
    name: &str,
    instance: Option<&str>,
) -> Landing {
    match workspace.find(name) {
        None => Landing::Open,
        Some(pane)
            if instance.is_some()
                && facts.get(&pane).and_then(|f| f.instance.as_deref()) == instance =>
        {
            Landing::Reuse(pane)
        }
        Some(pane) => Landing::Conflict(pane),
    }
}

/// Not done because the user is in the middle of something; the same request may come again.
pub fn busy(what: &str) -> Value {
    let mut value = control::error(
        "busy",
        format!("the user is {what}; try again when they are done"),
    );
    value["state"] = json!("busy");
    value
}

/// The pane the caller runs in: an agent by its public corral name and instance, a paddock shell
/// by the instance and pane it was started with. Never a guess.
pub fn caller_pane(model: &Model, caller: &Caller) -> Result<PaneId, String> {
    if caller.name.is_some() || caller.corral_instance.is_some() {
        let name = caller.name.as_deref().ok_or("missing CORRAL_NAME")?;
        let instance = caller
            .corral_instance
            .as_deref()
            .ok_or("missing CORRAL_INSTANCE")?;
        if !model
            .listed
            .iter()
            .any(|a| a.name == name && a.instance.as_deref() == Some(instance) && !a.error)
        {
            return Err(format!(
                "{name} with this CORRAL_INSTANCE is not in corral's listing"
            ));
        }
        let pane = model
            .workspace
            .find(name)
            .ok_or_else(|| format!("{name} is not shown in this paddock"))?;
        let facts = model.facts.get(&pane).cloned().unwrap_or_default();
        if facts.attached.as_deref() != Some(name) || facts.instance.as_deref() != Some(instance) {
            return Err(format!(
                "{name}'s pane is not attached to this instance of it (attaching, ended or replaced)"
            ));
        }
        return Ok(pane);
    }
    let paddock = caller.paddock_instance.as_deref().ok_or(
        "caller has no identity: neither CORRAL_NAME/CORRAL_INSTANCE nor PADDOCK_INSTANCE/PADDOCK_PANE",
    )?;
    if paddock != model.instance {
        return Err("caller's PADDOCK_INSTANCE is another paddock".into());
    }
    let pane = caller.pane.ok_or("missing PADDOCK_PANE")?;
    let facts = model.facts.get(&pane);
    let live = model.workspace.revision(pane).is_some()
        && model.workspace.shown(pane) == &Shown::Shell
        && facts.is_some_and(|f| {
            f.shell_live && f.identity.as_ref() == Some(&(model.instance.to_owned(), pane))
        });
    if !live {
        return Err(format!(
            "pane {pane} no longer runs the shell started with this identity"
        ));
    }
    Ok(pane)
}

/// The layout tree as `inspect` shows it.
pub fn layout(node: &Node) -> Value {
    match node {
        Node::Pane(id) => json!({"pane": id}),
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => json!({
            "split": match axis { Axis::Row => "row", Axis::Column => "column" },
            "ratio": ratio,
            "first": layout(first),
            "second": layout(second),
        }),
    }
}

fn kind(model: &Model, pane: PaneId) -> &'static str {
    match model.workspace.shown(pane) {
        Shown::Empty => "empty",
        Shown::Agent(_) => "agent",
        Shown::Shell if model.facts.get(&pane).is_some_and(|f| f.command) => "command",
        Shown::Shell => "shell",
    }
}

fn pane_json(model: &Model, pane: PaneId) -> Value {
    let facts = model.facts.get(&pane).cloned().unwrap_or_default();
    let agent = match model.workspace.shown(pane) {
        Shown::Agent(name) => Some(name.as_str()),
        _ => None,
    };
    let shell = matches!(model.workspace.shown(pane), Shown::Shell);
    json!({
        "id": pane,
        "revision": model.workspace.revision(pane),
        "kind": kind(model, pane),
        "agent": agent,
        "corral_instance": agent.and(facts.instance.as_deref()),
        "cwd": source_cwd(model, pane),
        "state": facts.state,
        "shell": shell.then(|| json!({"program": facts.program, "exit_code": facts.exit_code})),
    })
}

/// `paddock ctl inspect`.
pub fn inspect(model: &Model, caller: &Caller) -> Value {
    let caller = match caller_pane(model, caller) {
        Ok(pane) => json!({"pane": pane, "revision": model.workspace.revision(pane)}),
        Err(e) => json!({"pane": null, "error": {"code": "caller_unresolved", "message": e}}),
    };
    let workspace = model.workspace;
    let tabs: Vec<Value> = workspace
        .tabs
        .iter()
        .map(|tab| {
            json!({
                "id": tab.id,
                "active_pane": tab.active,
                "layout": layout(&tab.root),
                "panes": tab.panes().into_iter().map(|p| pane_json(model, p)).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "ok": true,
        "instance": model.instance,
        "active_tab": workspace.tab().id,
        "active_pane": workspace.active_pane(),
        "zoomed": workspace.zoomed(),
        "caller": caller,
        "tabs": tabs,
    })
}

/// The directory a pane is known to have: its shell's, or its agent's. Not where a shell has
/// `cd`ed since.
pub fn source_cwd(model: &Model, pane: PaneId) -> Option<String> {
    let facts = model.facts.get(&pane);
    match model.workspace.shown(pane) {
        Shown::Empty => None,
        Shown::Shell => facts.and_then(|f| f.cwd.clone()),
        Shown::Agent(name) => facts.and_then(|f| f.cwd.clone()).or_else(|| {
            model
                .listed
                .iter()
                .find(|a| &a.name == name)
                .and_then(|a| a.cwd.clone())
        }),
    }
}

/// Where `place` is.
pub fn at(place: Place) -> At {
    match place {
        Place::Tab => At::Tab,
        Place::Left => At::Side(Direction::Left),
        Place::Right => At::Side(Direction::Right),
        Place::Up => At::Side(Direction::Up),
        Place::Down => At::Side(Direction::Down),
    }
}

/// What `close` would close, as the confirmation lists it; it changes whenever a target does.
pub fn close_targets(model: &Model, panes: &[PaneId]) -> Value {
    json!(
        panes
            .iter()
            .map(|&pane| {
                let facts = model.facts.get(&pane).cloned().unwrap_or_default();
                json!({
                    "pane": pane,
                    "revision": model.workspace.revision(pane),
                    "kind": kind(model, pane),
                    "running_shell": facts.shell_live,
                    "cwd": source_cwd(model, pane),
                    "program": facts.program,
                    "agent": match model.workspace.shown(pane) {
                        Shown::Agent(name) => Some(name),
                        _ => None,
                    },
                    // Another instance under the same name is another target.
                    "corral_instance": match model.workspace.shown(pane) {
                        Shown::Agent(_) => facts.instance.as_deref(),
                        _ => None,
                    },
                })
            })
            .collect::<Vec<_>>()
    )
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.chars().any(char::is_whitespace)
}

/// Decides what a changing request (`open`, `close`, `browse`) does. Changes nothing itself,
/// besides marking a close confirmation used: one used, or presented after its target changed,
/// never works again. `records` are the results recorded so far.
pub fn plan<'r>(
    model: &Model,
    message: &Message,
    records: impl Iterator<Item = &'r Value>,
    used: &mut HashSet<String>,
) -> Plan {
    // Before anything else: busy changes nothing and records nothing.
    if let Some(what) = model.busy {
        return Plan::Reply(busy(what));
    }
    match decide(model, message, records, used) {
        Ok(plan) => plan,
        Err(value) => Plan::Reply(value),
    }
}

fn decide<'r>(
    model: &Model,
    message: &Message,
    records: impl Iterator<Item = &'r Value>,
    used: &mut HashSet<String>,
) -> Result<Plan, Value> {
    let workspace = model.workspace;
    match &message.operation {
        Operation::Open {
            relative_to,
            place,
            content,
            focus,
        } => {
            let anchor = match relative_to.as_str() {
                "self" => caller_pane(model, &message.caller)
                    .map_err(|e| failed("caller_unresolved", e))?,
                "active" => workspace.active_pane(),
                id => {
                    let pane: PaneId = id.parse().map_err(|_| {
                        failed(
                            "invalid_request",
                            "relative-to must be self, active or a pane ID",
                        )
                    })?;
                    workspace.revision(pane).ok_or_else(|| {
                        failed("invalid_target", format!("pane {pane} is not open"))
                    })?;
                    pane
                }
            };
            let (at, focus) = (at(*place), *focus);
            let directory = |explicit: &Option<String>| -> Result<(String, &'static str), Value> {
                let (cwd, source) = match (explicit, source_cwd(model, anchor)) {
                    (Some(cwd), _) => (cwd.clone(), "explicit"),
                    (None, Some(cwd)) => (cwd, "source_pane"),
                    (None, None) => (model.startup_cwd.to_owned(), "startup_directory"),
                };
                let path = std::path::Path::new(&cwd);
                if !(path.is_absolute() && path.is_dir()) {
                    return Err(failed(
                        "invalid_request",
                        format!("cwd must be an existing absolute directory: {cwd}"),
                    ));
                }
                Ok((cwd, source))
            };
            match content {
                Content::Shell { cwd } => {
                    let (cwd, cwd_source) = directory(cwd)?;
                    Ok(Plan::Shell {
                        anchor,
                        at,
                        cwd,
                        cwd_source,
                        focus,
                    })
                }
                Content::Agent { name } => {
                    let listed = model
                        .listed
                        .iter()
                        .find(|a| &a.name == name && !a.error)
                        .ok_or_else(|| {
                            failed(
                                "invalid_target",
                                format!("{name} is not in corral's listing"),
                            )
                        })?;
                    match workspace.find(name) {
                        Some(pane) => {
                            let facts = model.facts.get(&pane).cloned().unwrap_or_default();
                            Ok(Plan::Move {
                                name: name.clone(),
                                pane,
                                anchor,
                                at,
                                reattach: facts.attached.as_deref() != Some(name.as_str())
                                    && facts.state != "attaching",
                                focus,
                            })
                        }
                        None => Ok(Plan::Attach {
                            name: name.clone(),
                            cwd: listed.cwd.clone(),
                            instance: listed.instance.clone(),
                            anchor,
                            at,
                            focus,
                        }),
                    }
                }
                Content::NewAgent {
                    name,
                    cwd,
                    role,
                    prompt,
                    argv,
                } => {
                    if !valid_name(name) {
                        return Err(failed(
                            "invalid_request",
                            "agent name needs text, no spaces or leading '-'",
                        ));
                    }
                    if !["regular", "controller", "implementer", "reviewer"]
                        .contains(&role.as_str())
                    {
                        return Err(failed(
                            "invalid_request",
                            "role must be regular, controller, implementer or reviewer",
                        ));
                    }
                    if argv.first().is_none_or(|a| a.is_empty()) {
                        return Err(failed("invalid_request", "new agent needs -- PROGRAM ARG…"));
                    }
                    let (cwd, cwd_source) = directory(cwd)?;
                    let mut args = vec![
                        "start".to_owned(),
                        name.clone(),
                        "--cwd".into(),
                        cwd.clone(),
                        "--label".into(),
                        format!("role={role}"),
                    ];
                    // Only a prompt given explicitly goes along.
                    if let Some(prompt) = prompt {
                        args.extend(["--prompt".into(), prompt.clone()]);
                    }
                    args.push("--".into());
                    args.extend(argv.iter().cloned());
                    Ok(Plan::Start {
                        args,
                        cwd,
                        cwd_source,
                        anchor,
                        at,
                        focus,
                    })
                }
            }
        }
        Operation::Close {
            target,
            confirmation,
            confirm_shells,
        } => {
            let panes = match *target {
                CloseTarget::Pane(id) => workspace.revision(id).map(|_| vec![id]),
                CloseTarget::Tab(id) => workspace.tab_index(id).map(|i| workspace.tabs[i].panes()),
            }
            .ok_or_else(|| failed("invalid_target", "close target is not open"))?;
            let targets = close_targets(model, &panes);
            let shells = targets
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["running_shell"] == true);
            if !*confirm_shells && confirmation.is_none() {
                if !shells {
                    return Ok(Plan::Close {
                        target: *target,
                        panes,
                    });
                }
                let token = control::random_id().map_err(|e| failed("failed", e))?;
                return Err(json!({
                    "ok": true,
                    "accepted": true,
                    "state": "confirmation_required",
                    "confirmation": token,
                    "target": target,
                    "targets": targets,
                    "message": "running shells and what runs in them end; ask the user, then close again with --confirm-shells --confirmation TOKEN and a new request ID",
                }));
            }
            let (true, Some(token)) = (*confirm_shells, confirmation) else {
                return Err(failed(
                    "invalid_request",
                    "a confirmed close needs both --confirm-shells and --confirmation",
                ));
            };
            if used.contains(token) {
                return Err(failed(
                    "confirmation_invalid",
                    "confirmation already used or out of date; close again without it for a new one",
                ));
            }
            let mut records = records;
            let Some(asked) = records.find(|v| {
                v["state"] == "confirmation_required" && v["confirmation"].as_str() == Some(token)
            }) else {
                return Err(failed(
                    "confirmation_invalid",
                    "no such confirmation in this instance",
                ));
            };
            used.insert(token.clone());
            if asked["target"] != json!(target) || asked["targets"] != targets {
                return Err(failed(
                    "confirmation_invalid",
                    "the close target changed since it was confirmed; close again without the confirmation and ask again",
                ));
            }
            Ok(Plan::Close {
                target: *target,
                panes,
            })
        }
        Operation::Browse { url, focus } => {
            let web = url.split_once("://").is_some_and(|(scheme, _)| {
                matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
            });
            if !web {
                return Err(failed(
                    "invalid_request",
                    "browse accepts only http/https URLs",
                ));
            }
            Ok(Plan::Browse {
                url: url.clone(),
                focus: *focus,
            })
        }
        Operation::Inspect | Operation::Instances | Operation::Request { .. } => {
            Err(failed("invalid_request", "not a change"))
        }
    }
}

/// A request whose pane is still on its way.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub request: String,
    pub pane: PaneId,
    /// The pane's revision when the request put its content there.
    pub revision: u64,
    /// The agent it attaches, for an agent.
    pub agent: Option<String>,
    /// The corral instance that attach is for, when known.
    pub instance: Option<String>,
    /// The result as recorded so far.
    pub value: Value,
}

/// How a tracked request ended, once it has: `complete` once the pane shows its shell or agent
/// (not that the model is ready), `failed` when that did not work, `target_invalid` when the pane
/// was closed or given something else first. `None` while it is still on its way.
pub fn progress(track: &Track, workspace: &Workspace, facts: Option<&Facts>) -> Option<Value> {
    let mut value = track.value.clone();
    let facts = facts.cloned().unwrap_or_default();
    // Attached under its name, but to another instance than the one this request attached.
    let other_instance = track.agent.is_some()
        && track.instance.is_some()
        && facts.attached == track.agent
        && facts.instance != track.instance;
    let invalid = workspace.revision(track.pane) != Some(track.revision) || other_instance;
    if invalid {
        let created = value["agent_created"].is_object();
        value["ok"] = json!(false);
        value["state"] = json!("target_invalid");
        value["pty"] = json!("target_invalid");
        value["error"] = json!({
            "code": "target_invalid",
            "message": if created {
                "display target closed or replaced; the created agent keeps running"
            } else {
                "display target closed or replaced"
            },
        });
        return Some(value);
    }
    let done = match &track.agent {
        Some(name) => facts.attached.as_deref() == Some(name.as_str()),
        None => matches!(facts.state.as_str(), "running" | "exited"),
    };
    if done {
        value["state"] = json!("complete");
        value["pty"] = json!(facts.state);
        value["exit_code"] = json!(facts.exit_code);
        return Some(value);
    }
    if matches!(facts.state.as_str(), "failed" | "disconnected") {
        value["ok"] = json!(false);
        value["state"] = json!("failed");
        value["pty"] = json!("failed");
        value["error"] = json!({"code": "pty_failed", "message": facts.note});
        return Some(value);
    }
    None
}

#[cfg(test)]
#[path = "control_ui_tests.rs"]
mod tests;
