//! `paddock ctl` in the window (P5-39b): what `control_ui` decides, done on the panes. The
//! transport (`control.rs`) hands each request over on the UI thread; steps that take time report
//! back through [`PaddockWindow::control_tick`]. Nothing here sends keys, reads a screen, stops
//! an agent or reads a page.
use super::*;
use crate::{
    control::{CloseTarget, Message, Operation, Records},
    control_ui::{self, Facts, Listed, Model, Plan, Track},
    layout::At,
    new_agent::Started,
};
use serde_json::{Value, json};
use std::collections::HashSet;

/// `paddock ctl`'s part of the window.
#[derive(Default)]
pub(super) struct Ctl {
    /// Close confirmations used, or void because their target changed.
    used: HashSet<String>,
    /// Requests whose pane is still on its way.
    tracks: Vec<Track>,
    /// New agents waiting on `corral start`, or on the user to be done before their pane opens.
    starting: Vec<Starting>,
    /// Results changed since the transport last asked.
    updates: Vec<(String, Value)>,
}

struct Starting {
    request: String,
    anchor: PaneId,
    at: At,
    focus: bool,
    cwd: String,
    value: Value,
    /// What `corral start` said, once it has.
    result: Option<anyhow::Result<Started>>,
}

impl PaddockWindow {
    /// The identity a shell in `pane` starts with, for `paddock ctl` to know it by.
    pub(super) fn identity(&self, pane: PaneId) -> Vec<(String, String)> {
        vec![
            ("PADDOCK_INSTANCE".into(), self.new_shell.instance.clone()),
            ("PADDOCK_PANE".into(), pane.to_string()),
        ]
    }

    /// What the user is in the middle of, when a change from `paddock ctl` would get in the way.
    fn ctl_busy(&self, cx: &gpui::App) -> Option<&'static str> {
        if self.resizing.is_some()
            || self.right.resizing()
            || self.splitting.is_some()
            || self.dragging
        {
            Some("dragging a divider or the window")
        } else if self.asking {
            Some("answering a question")
        } else if self.popup.is_some() {
            Some("using a panel or menu")
        } else if windows::new_agent_busy(cx) {
            Some("creating an agent in New Agent")
        } else if windows::quitting(cx) {
            Some("quitting paddock")
        } else {
            None
        }
    }

    fn pane_facts(&self, cx: &gpui::App) -> HashMap<PaneId, Facts> {
        self.panes
            .iter()
            .map(|(pane, view)| (*pane, view.read(cx).facts()))
            .collect()
    }

    fn listed(&self, cx: &gpui::App) -> Vec<Listed> {
        self.sidebar
            .read(cx)
            .agents()
            .into_iter()
            .map(|agent| Listed {
                name: agent.name,
                instance: agent.instance,
                cwd: agent.cwd,
                error: agent.error.is_some(),
            })
            .collect()
    }

    /// One `paddock ctl` request, on the UI thread: `inspect`, `open`, `close` or `browse`.
    pub fn control(
        &mut self,
        message: &Message,
        records: &Records,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Value {
        let facts = self.pane_facts(cx);
        let listed = self.listed(cx);
        let model = Model {
            instance: &self.new_shell.instance,
            workspace: &self.workspace,
            facts: &facts,
            listed: &listed,
            startup_cwd: &self.new_shell.cwd,
            busy: self.ctl_busy(cx),
        };
        if matches!(message.operation, Operation::Inspect) {
            return control_ui::inspect(&model, &message.caller);
        }
        let plan = control_ui::plan(&model, message, records.values(), &mut self.ctl.used);
        let request = message.request_id.clone().unwrap_or_default();
        let relative = |this: &Self, anchor: PaneId| json!({"pane": anchor, "revision": this.workspace.revision(anchor)});
        match plan {
            Plan::Reply(value) => value,
            Plan::Shell {
                anchor,
                at,
                cwd,
                cwd_source,
                focus,
            } => {
                let relative_to = relative(self, anchor);
                let before = window.focused(cx);
                let pane = self
                    .workspace
                    .open_at(anchor, at, Shown::Shell)
                    .expect("the plan checked the anchor");
                let launch = Launch::Shell {
                    program: self.new_shell.program.clone(),
                    cwd: cwd.clone(),
                    env: Vec::new(),
                };
                let view = self.view(pane, launch, window, cx);
                self.panes.insert(pane, view);
                self.opened(pane, focus, before, window, cx);
                let value = json!({
                    "ok": true, "accepted": true, "state": "starting", "pty": "pending",
                    "pane": pane, "revision": self.workspace.revision(pane),
                    "cwd": cwd, "cwd_source": cwd_source, "relative_to": relative_to,
                });
                self.track(request, pane, None, &value);
                value
            }
            Plan::Attach {
                name,
                cwd,
                instance,
                anchor,
                at,
                focus,
            } => {
                let relative_to = relative(self, anchor);
                let before = window.focused(cx);
                let pane = self
                    .workspace
                    .open_at(anchor, at, Shown::Agent(name.clone()))
                    .expect("the plan checked the anchor");
                let view = self.view(pane, Launch::Empty, window, cx);
                self.panes.insert(pane, view);
                let metadata = AgentMetadata {
                    cwd: cwd.clone(),
                    instance,
                };
                self.attach(pane, &name, metadata, cx);
                self.opened(pane, focus, before, window, cx);
                let value = json!({
                    "ok": true, "accepted": true, "state": "attaching", "pty": "pending",
                    "pane": pane, "revision": self.workspace.revision(pane),
                    "agent": name, "cwd": cwd, "relative_to": relative_to,
                });
                self.track(request, pane, Some(name), &value);
                value
            }
            Plan::Move {
                name,
                pane,
                anchor,
                at,
                reattach,
                focus,
            } => {
                let relative_to = relative(self, anchor);
                let moved = self.workspace.move_pane(pane, anchor, at);
                if reattach {
                    let metadata = self.sidebar.read(cx).metadata(&name);
                    self.attach(pane, &name, metadata, cx);
                }
                if focus {
                    self.workspace.focus(pane);
                    self.focus_active(window, cx);
                } else {
                    self.sync(cx);
                }
                let attached =
                    self.panes[&pane].read(cx).facts().attached.as_deref() == Some(name.as_str());
                let value = json!({
                    "ok": true, "accepted": true,
                    "state": if attached { "complete" } else { "attaching" },
                    "pty": if attached { "running" } else { "pending" },
                    "pane": pane, "revision": self.workspace.revision(pane),
                    "agent": name, "moved": moved, "relative_to": relative_to,
                });
                if !attached {
                    self.track(request, pane, Some(name), &value);
                }
                value
            }
            Plan::Start {
                args,
                cwd,
                cwd_source,
                anchor,
                at,
                focus,
            } => {
                let value = json!({
                    "ok": true, "accepted": true, "state": "starting", "pty": "not_started",
                    "pane": null, "revision": null, "agent_created": null,
                    "cwd": cwd, "cwd_source": cwd_source, "relative_to": relative(self, anchor),
                });
                // As New Agent does: the agent first, then its pane.
                let corral = self.template.corral.clone();
                let task = cx.background_spawn(async move { new_agent::start(&corral, &args) });
                let id = request.clone();
                cx.spawn(async move |this, cx| {
                    let result = task.await;
                    let _ = this.update(cx, |this, _| {
                        if let Some(starting) =
                            this.ctl.starting.iter_mut().find(|s| s.request == id)
                        {
                            starting.result = Some(result);
                        }
                    });
                })
                .detach();
                self.ctl.starting.push(Starting {
                    request,
                    anchor,
                    at,
                    focus,
                    cwd,
                    value: value.clone(),
                    result: None,
                });
                value
            }
            Plan::Close { target, panes } => {
                let gone = match target {
                    CloseTarget::Pane(pane) => self.workspace.close_pane(pane),
                    CloseTarget::Tab(id) => self
                        .workspace
                        .tab_index(id)
                        .map(|index| self.workspace.close_tab(index))
                        .unwrap_or_default(),
                };
                self.close_quietly(gone, window, cx);
                json!({
                    "ok": true, "accepted": true, "state": "complete",
                    "target": target, "closed": panes,
                })
            }
            Plan::Browse { url, focus } => {
                self.right.open = true;
                self.right.tab = RightTab::Browser;
                self.browser.update(cx, |browser, cx| {
                    browser.visit(url.clone(), cx);
                    if focus {
                        browser.give_back(window, cx);
                    }
                });
                self.save_layout(cx);
                cx.notify();
                json!({"ok": true, "accepted": true, "state": "complete", "url": url, "focus": focus})
            }
        }
    }

    fn track(&mut self, request: String, pane: PaneId, agent: Option<String>, value: &Value) {
        self.ctl.tracks.push(Track {
            request,
            pane,
            revision: self.workspace.revision(pane).unwrap_or_default(),
            agent,
            value: value.clone(),
        });
    }

    /// A pane `paddock ctl` opened: the keyboard goes to it with `--focus`; otherwise it stays
    /// where it was (a new terminal takes it as it is made).
    fn opened(
        &mut self,
        pane: PaneId,
        focus: bool,
        before: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if focus {
            self.workspace.focus(pane);
            self.focus_active(window, cx);
        } else {
            refocus(before, window, cx);
            self.sync(cx);
        }
    }

    /// Closes `gone`'s views as closing them in the window does, without moving the keyboard
    /// unless it was in one of them.
    fn close_quietly(&mut self, gone: Vec<PaneId>, window: &mut Window, cx: &mut Context<Self>) {
        let had = gone.iter().any(|pane| {
            self.panes
                .get(pane)
                .is_some_and(|view| view.read(cx).focus_handle(cx).contains_focused(window, cx))
        });
        let before = window.focused(cx);
        for pane in gone {
            if let Some(view) = self.panes.remove(&pane) {
                view.update(cx, |v, cx| v.close(cx));
            }
        }
        self.fill(window, cx);
        if had {
            self.focus_active(window, cx);
        } else {
            refocus(before, window, cx);
            self.sync(cx);
        }
    }

    /// Moves `paddock ctl`'s requests along, on the UI thread: opens the panes of new agents
    /// corral has started (once the user is not in the middle of something), and notes requests
    /// that have finished. Returns the results that changed, for the transport's records.
    pub fn control_tick(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<(String, Value)> {
        let busy = self.ctl_busy(cx).is_some();
        let (ready, waiting): (Vec<Starting>, Vec<Starting>) =
            std::mem::take(&mut self.ctl.starting)
                .into_iter()
                .partition(|s| match &s.result {
                    Some(Ok(_)) => !busy,
                    Some(Err(_)) => true,
                    None => false,
                });
        self.ctl.starting = waiting;
        for starting in ready {
            self.started(starting, window, cx);
        }
        for track in std::mem::take(&mut self.ctl.tracks) {
            let facts = self
                .panes
                .get(&track.pane)
                .map(|view| view.read(cx).facts());
            match control_ui::progress(&track, &self.workspace, facts.as_ref()) {
                Some(value) => self.ctl.updates.push((track.request, value)),
                None => self.ctl.tracks.push(track),
            }
        }
        std::mem::take(&mut self.ctl.updates)
    }

    /// Opens the pane of an agent `corral start` answered for, beside the pane asked for.
    fn started(&mut self, starting: Starting, window: &mut Window, cx: &mut Context<Self>) {
        let Starting {
            request,
            anchor,
            at,
            focus,
            cwd,
            mut value,
            result,
        } = starting;
        let started = match result.expect("answered") {
            Ok(started) => started,
            Err(error) => {
                value["ok"] = json!(false);
                value["state"] = json!("failed");
                value["error"] = json!({"code": "start_failed", "message": format!("{error:#}")});
                self.ctl.updates.push((request, value));
                return;
            }
        };
        let name = started.name.clone();
        value["agent_created"] = json!({"name": name, "instance": started.instance});
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.note(format!("Started {name}"), false, cx);
            sidebar.refresh();
        });
        // The pane it was to open beside closed meanwhile; the agent keeps running.
        if self.workspace.revision(anchor).is_none() {
            value["ok"] = json!(false);
            value["state"] = json!("target_invalid");
            value["pty"] = json!("target_invalid");
            value["error"] = json!({"code": "target_invalid", "message": "the pane to open beside closed; the created agent keeps running"});
            self.ctl.updates.push((request, value));
            return;
        }
        let before = window.focused(cx);
        let pane = self
            .workspace
            .open_at(anchor, at, Shown::Agent(name.clone()))
            .expect("anchor is open");
        let view = self.view(pane, Launch::Empty, window, cx);
        self.panes.insert(pane, view);
        let metadata = AgentMetadata {
            cwd: Some(cwd),
            instance: started.instance,
        };
        self.attach(pane, &name, metadata, cx);
        self.opened(pane, focus, before, window, cx);
        value["state"] = json!("attaching");
        value["pty"] = json!("pending");
        value["pane"] = json!(pane);
        value["revision"] = json!(self.workspace.revision(pane));
        self.ctl.updates.push((request.clone(), value.clone()));
        self.track(request, pane, Some(name), &value);
    }
}

/// Puts the keyboard back where it was.
fn refocus(before: Option<FocusHandle>, window: &mut Window, cx: &mut gpui::App) {
    match before {
        Some(focus) => window.focus(&focus, cx),
        None => window.blur(cx),
    }
}
