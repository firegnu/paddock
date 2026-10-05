//! The Agents sidebar: `corral ls` through Saddle's poller, ordered and judged by Saddle's Agents
//! panel, one row per agent. Clicking a row attaches the terminal pane to that agent; the footer
//! opens a shell instead.
use crate::{
    theme::Theme,
    view::{TerminalView, hsla},
};
use anyhow::Result;
use gpui::{
    App, ClickEvent, Context, ElementId, Entity, Focusable, Render, RenderOnce, SharedString,
    Window, div, prelude::*, px,
};
use ratatui::style::Color;
use saddle::{
    agents::{Panel, Status, group},
    corral::{Agent, Client, Poller},
    viewer::AgentMetadata,
};
use std::{rc::Rc, time::Duration};

/// How often corral is asked, as Saddle's default `refresh_ms`.
const REFRESH: Duration = Duration::from_secs(1);

/// What "＋ 新 shell" starts.
pub struct NewShell {
    pub program: String,
    pub cwd: String,
}

/// A Saddle interface colour, as `Theme::fg/bg` take it.
type Pick = fn(&saddle::theme::Theme) -> Color;

/// The state label and its colour, as Saddle's Agents panel shows them.
pub fn look(status: Status) -> (&'static str, Pick) {
    match status {
        Status::Waiting => ("waiting", |t| t.agents_yellow),
        Status::Error => ("error", |t| t.agents_red),
        Status::Stalled => ("stalled", |t| t.agent_stalled),
        Status::Working => ("working", |t| t.agents_blue),
        Status::Starting => ("starting", |t| t.agent_starting),
        Status::Unknown => ("unknown", |t| t.agents_dim),
        Status::Idle => ("idle", |t| t.agents_green),
        Status::Exited => ("exited", |t| t.agents_faint),
    }
}

/// One agent's row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub name: String,
    /// The name without its group prefix.
    pub short: String,
    pub status: Status,
    pub cwd: Option<String>,
    pub instance: Option<String>,
}

impl Row {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            cwd: self.cwd.clone(),
            instance: self.instance.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// Why the list could not be read.
    Error(String),
    /// A group name on its own line.
    Group(String),
    Agent(Row),
}

/// The list model: the last good `corral ls` and the last error, if the latest read failed.
#[derive(Default)]
pub struct Listing {
    panel: Panel,
    error: Option<String>,
}

impl Listing {
    /// Takes one poller result. On success returns the agents still alive, for the pane to let go
    /// of one that disappeared; an error keeps the previous list.
    pub fn absorb(&mut self, update: Result<Vec<Agent>>, now: f64) -> Option<Vec<String>> {
        match update {
            Ok(agents) => {
                self.error = None;
                let alive = agents
                    .iter()
                    .filter(|a| a.state.as_deref() != Some("exited"))
                    .map(|a| a.name.clone())
                    .collect();
                self.panel.absorb(agents, None, now);
                Some(alive)
            }
            Err(error) => {
                self.error = Some(format!("corral: {error:#}"));
                None
            }
        }
    }

    pub fn lines(&self, now: f64) -> Vec<Line> {
        let mut lines: Vec<Line> = self.error.iter().cloned().map(Line::Error).collect();
        let mut previous = None;
        for a in self.panel.ordered(now) {
            let prefix = group(&a.name);
            if previous != Some(prefix) {
                let title = if prefix.is_empty() { "agents/" } else { prefix };
                lines.push(Line::Group(title.to_owned()));
                previous = Some(prefix);
            }
            lines.push(Line::Agent(Row {
                name: a.name.clone(),
                short: a.name.strip_prefix(prefix).unwrap_or(&a.name).to_owned(),
                status: self.panel.status(a, now),
                cwd: a.cwd.clone(),
                instance: a.instance.clone(),
            }));
        }
        lines
    }
}

pub struct Sidebar {
    theme: Rc<Theme>,
    width: f32,
    listing: Listing,
    poller: Poller,
    terminal: Entity<TerminalView>,
    new_shell: NewShell,
}

impl Sidebar {
    pub fn new(
        theme: Rc<Theme>,
        width: f32,
        corral: String,
        new_shell: NewShell,
        terminal: Entity<TerminalView>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |sidebar, cx| sidebar.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            theme,
            width,
            listing: Listing::default(),
            poller: Poller::start(Client { program: corral }, REFRESH),
            terminal,
            new_shell,
        }
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let updates: Vec<_> = self.poller.updates.try_iter().collect();
        if updates.is_empty() {
            return;
        }
        for update in updates {
            if let Some(alive) = self.listing.absorb(update, now()) {
                let alive: Vec<&str> = alive.iter().map(String::as_str).collect();
                self.terminal.update(cx, |t, _| t.disappeared(&alive));
            }
        }
        cx.notify();
    }

    fn attach(&mut self, row: &Row, window: &mut Window, cx: &mut Context<Self>) {
        let (name, metadata) = (row.name.clone(), row.metadata());
        self.terminal
            .update(cx, |t, cx| t.attach(name, metadata, cx));
        self.focus_terminal(window, cx);
    }

    fn start_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (program, cwd) = (self.new_shell.program.clone(), self.new_shell.cwd.clone());
        self.terminal
            .update(cx, |t, cx| t.start_shell(program, cwd, cx));
        self.focus_terminal(window, cx);
    }

    fn focus_terminal(&self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.terminal.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }
}

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let current = self.terminal.read(cx).target().map(str::to_owned);
        let mut list = div()
            .id("agents")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .py(px(4.0));
        for line in self.listing.lines(now()) {
            list = list.child(match line {
                Line::Error(text) => div()
                    .px(px(10.0))
                    .py(px(3.0))
                    .text_color(hsla(theme.fg(|t| t.agents_red), 1.0))
                    .child(text)
                    .into_any_element(),
                Line::Group(title) => div()
                    .px(px(10.0))
                    .pt(px(8.0))
                    .pb(px(2.0))
                    .text_color(hsla(theme.fg(|t| t.agents_accent), 1.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(title)
                    .into_any_element(),
                Line::Agent(row) => {
                    let selected = current.as_deref() == Some(row.name.as_str());
                    let on_click = {
                        let row = row.clone();
                        cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.attach(&row, window, cx)
                        })
                    };
                    AgentRow {
                        row,
                        selected,
                        theme: theme.clone(),
                        on_click: Box::new(on_click),
                    }
                    .into_any_element()
                }
            });
        }
        div()
            .flex_shrink_0()
            .w(px(self.width))
            .h_full()
            .flex()
            .flex_col()
            .text_size(px(13.0))
            .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
            .child(list)
            .child(
                div()
                    .id("new-shell")
                    .flex_shrink_0()
                    .px(px(10.0))
                    .py(px(6.0))
                    .border_t_1()
                    .border_color(hsla(theme.fg(|t| t.agents_rule), 1.0))
                    .text_color(hsla(theme.fg(|t| t.agents_text), 1.0))
                    .cursor_pointer()
                    .child("＋ 新 shell")
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.start_shell(window, cx)
                        }),
                    ),
            )
    }
}

/// One agent in the list. Its own element, so rows can grow more lines later without the list
/// assuming a height.
type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct AgentRow {
    row: Row,
    selected: bool,
    theme: Rc<Theme>,
    on_click: OnClick,
}

impl RenderOnce for AgentRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let theme = &self.theme;
        let (label, color) = look(self.row.status);
        let color = hsla(theme.fg(color), 1.0);
        let name_color = if self.row.status == Status::Exited {
            theme.fg(|t| t.agents_faint)
        } else {
            theme.fg(|t| t.agents_text)
        };
        let mut row = div()
            .id(ElementId::Name(SharedString::from(self.row.name)))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(7.0))
            .px(px(10.0))
            .py(px(3.0))
            .cursor_pointer()
            .on_click(self.on_click);
        if self.selected {
            row = row.bg(hsla(theme.bg(|t| t.agent_selected), 1.0));
        }
        row.child(div().flex_shrink_0().size(px(8.0)).rounded_full().bg(color))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_color(hsla(name_color, 1.0))
                    .child(self.row.short),
            )
            .child(div().flex_shrink_0().text_color(color).child(label))
    }
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, state: &str) -> Agent {
        Agent {
            name: name.into(),
            state: Some(state.into()),
            cwd: Some(format!("/work/{name}")),
            instance: Some(format!("i-{name}")),
            ..Agent::default()
        }
    }

    fn shown(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| match line {
                Line::Error(text) => format!("! {text}"),
                Line::Group(title) => format!("# {title}"),
                Line::Agent(row) => format!("{} {}", row.short, look(row.status).0),
            })
            .collect()
    }

    #[test]
    fn groups_order_and_short_names() {
        let mut listing = Listing::default();
        let alive = listing.absorb(
            Ok(vec![
                agent("paddock/main", "idle"),
                agent("solo", "working"),
                agent("paddock/dev-agents", "working"),
                agent("paddock/dev-theme", "blocked"),
                agent("saddle/main", "exited"),
            ]),
            1000.0,
        );
        assert_eq!(
            alive.unwrap(),
            [
                "paddock/main",
                "solo",
                "paddock/dev-agents",
                "paddock/dev-theme"
            ]
        );
        let lines = listing.lines(1000.0);
        // Groups in name order, the ungrouped first; within a group, those needing a person first.
        assert_eq!(
            shown(&lines),
            [
                "# agents/",
                "solo working",
                "# paddock/",
                "dev-theme waiting",
                "dev-agents working",
                "main idle",
                "# saddle/",
                "main exited",
            ]
        );
        let Line::Agent(row) = &lines[3] else {
            panic!()
        };
        assert_eq!(row.name, "paddock/dev-theme");
        let metadata = row.metadata();
        assert_eq!(metadata.cwd.as_deref(), Some("/work/paddock/dev-theme"));
        assert_eq!(metadata.instance.as_deref(), Some("i-paddock/dev-theme"));
    }

    #[test]
    fn status_refreshes_with_each_listing() {
        let mut listing = Listing::default();
        listing.absorb(Ok(vec![agent("p/a", "working")]), 1000.0);
        assert_eq!(shown(&listing.lines(1000.0))[1], "a working");
        listing.absorb(Ok(vec![agent("p/a", "idle")]), 1001.0);
        assert_eq!(shown(&listing.lines(1001.0))[1], "a idle");
        listing.absorb(Ok(vec![]), 1002.0);
        assert!(listing.lines(1002.0).is_empty());
    }

    #[test]
    fn statuses_use_the_agents_panel_colours() {
        let dune = saddle::theme::Preset::Dune.theme();
        let expected: [(Status, &str, Color); 8] = [
            (Status::Waiting, "waiting", dune.agents_yellow),
            (Status::Error, "error", dune.agents_red),
            (Status::Stalled, "stalled", dune.agent_stalled),
            (Status::Working, "working", dune.agents_blue),
            (Status::Starting, "starting", dune.agent_starting),
            (Status::Unknown, "unknown", dune.agents_dim),
            (Status::Idle, "idle", dune.agents_green),
            (Status::Exited, "exited", dune.agents_faint),
        ];
        for (status, label, color) in expected {
            let (shown, pick) = look(status);
            assert_eq!(shown, label);
            assert_eq!(pick(&dune), color, "{status:?}");
        }
        // Errors and stalls come from the panel's judgement, not just corral's state.
        let mut listing = Listing::default();
        let mut broken = agent("p/broken", "idle");
        broken.error = Some("status failed".into());
        let mut quiet = agent("p/quiet", "working");
        quiet.last_output = Some(0.0);
        listing.absorb(Ok(vec![broken, quiet]), 1000.0);
        assert_eq!(
            shown(&listing.lines(1000.0)),
            ["# p/", "broken error", "quiet stalled"]
        );
    }

    /// A fake `corral` in a temporary directory; never the real one.
    fn fake_corral(name: &str, script: &str) -> (std::path::PathBuf, String) {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "paddock-sidebar-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corral");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, path.display().to_string())
    }

    fn first_update(program: String) -> Result<Vec<Agent>> {
        let poller = Poller::start(Client { program }, Duration::from_secs(60));
        poller
            .updates
            .recv_timeout(Duration::from_secs(20))
            .expect("poller result")
    }

    #[test]
    fn corral_errors_become_a_line_and_keep_the_list() {
        let (dir, failing) = fake_corral(
            "failing",
            r#"echo '{"ok": false, "error": "daemon unreachable"}'; exit 1"#,
        );
        let (_, garbage) = fake_corral("garbage", "echo not json");
        let mut listing = Listing::default();
        listing.absorb(Ok(vec![agent("p/a", "idle")]), 1000.0);

        assert!(listing.absorb(first_update(failing), 1001.0).is_none());
        let lines = shown(&listing.lines(1001.0));
        assert!(lines[0].starts_with("! corral: "), "{lines:?}");
        assert!(lines[0].contains("daemon unreachable"), "{lines:?}");
        assert_eq!(lines[1..], ["# p/", "a idle"]);

        assert!(listing.absorb(first_update(garbage), 1002.0).is_none());
        let lines = shown(&listing.lines(1002.0));
        assert!(lines[0].contains("invalid JSON"), "{lines:?}");

        let missing = dir.join("no-such-corral").display().to_string();
        assert!(listing.absorb(first_update(missing), 1003.0).is_none());
        assert!(shown(&listing.lines(1003.0))[0].starts_with("! corral: "));

        // The next good listing clears the error.
        listing.absorb(Ok(vec![agent("p/a", "working")]), 1004.0);
        assert_eq!(shown(&listing.lines(1004.0)), ["# p/", "a working"]);
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(dir.with_file_name(format!(
            "paddock-sidebar-test-{}-garbage",
            std::process::id()
        )))
        .unwrap();
    }

    #[test]
    fn fake_corral_listing_reaches_the_rows() {
        let (dir, program) = fake_corral(
            "listing",
            r#"case "$1" in
ls) echo '{"agents": [{"name": "p/a", "cwd": "/tmp/a", "instance": "i1"}]}' ;;
status) echo '{"name": "p/a", "state": "working", "instance": "i1"}' ;;
esac"#,
        );
        let mut listing = Listing::default();
        let alive = listing.absorb(first_update(program), 1000.0).unwrap();
        assert_eq!(alive, ["p/a"]);
        let lines = listing.lines(1000.0);
        assert_eq!(shown(&lines), ["# p/", "a working"]);
        let Line::Agent(row) = &lines[1] else {
            panic!()
        };
        assert_eq!(row.cwd.as_deref(), Some("/tmp/a"));
        assert_eq!(row.instance.as_deref(), Some("i1"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
