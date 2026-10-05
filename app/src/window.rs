//! The window's root view: the Agents sidebar on the left; on the right a tab strip and the active
//! tab's panes, each a terminal in a frame with its title and controls. `layout.rs` holds the
//! rules; this file draws them and keeps one terminal view per pane.
use crate::{
    attention::Kind as AttentionKind,
    config::Config,
    corral::Role,
    layout::{Axis, Direction, Node, PaneId, Placement, Shown, Workspace},
    menu,
    new_agent::{self, Place, Started},
    new_agent_view::Seed,
    pet::PetView,
    search,
    sidebar::{Sidebar, SidebarEvent},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::{Launch, Options, TerminalView, hsla},
    viewer::AgentMetadata,
    windows,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, FocusHandle, Focusable, FontWeight,
    Hsla, MouseButton, PromptLevel, Render, ScrollHandle, Stateful, Task, Window, div, prelude::*,
    px,
};
use std::{collections::HashMap, rc::Rc};

/// What "New shell" and the `Shell` choice start.
pub struct NewShell {
    pub program: String,
    pub cwd: String,
}

/// The small chooser behind `+` and `Split ▾`.
#[derive(Clone, Copy, PartialEq)]
enum Popup {
    /// Choose what the new tab shows.
    NewTab,
    /// First the direction, then what the new pane shows.
    Split(Option<Direction>),
    /// The Attention list.
    Attention,
    /// Go to Agent.
    Search,
}

/// A close waiting on an answer, by a pane in it, so it still finds its target if the layout
/// changes meanwhile.
#[derive(Clone, Copy)]
enum Closing {
    Pane(PaneId),
    /// The tab holding this pane.
    Tab(PaneId),
}

/// The system prompt before live shells end.
#[derive(Debug, PartialEq)]
pub struct Question {
    pub message: String,
    pub detail: String,
    pub confirm: &'static str,
}

impl Question {
    /// Shows it; resolves to whether the confirming button was chosen.
    fn ask(
        &self,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> impl std::future::Future<Output = bool> + use<> {
        let answer = window.prompt(
            PromptLevel::Warning,
            &self.message,
            Some(&self.detail),
            &[self.confirm, "Cancel"],
            cx,
        );
        async move { matches!(answer.await, Ok(0)) }
    }
}

/// What to ask before closing ends `shells` (their titles) and detaches `agents` views, or quitting
/// does: `None` when no live shell would end, so nothing needs asking. As in Saddle.
pub fn close_question(shells: &[String], agents: usize, quit: bool) -> Option<Question> {
    if shells.is_empty() {
        return None;
    }
    let (them, it, button) = if shells.len() == 1 {
        ("shell", "it", "End Shell")
    } else {
        ("shells", "them", "End Shells")
    };
    let message = if quit {
        format!("Quit paddock and end the running {them}?")
    } else {
        format!("End the running {them}?")
    };
    let mut detail = shells.join("\n");
    detail.push_str(&format!(
        "\n\nWhat runs in {it} in the foreground ends too."
    ));
    if agents > 0 {
        detail.push_str(" Agent views only detach; the agents keep running.");
    }
    Some(Question {
        message,
        detail,
        confirm: if quit { "Quit" } else { button },
    })
}

/// Something to open in a new tab or pane.
#[derive(Clone)]
enum Choice {
    Shell,
    Agent(String),
}

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

const GAP: f32 = 6.0;
const TEXT: f32 = 12.0;
/// Tall enough for the pets at twice their pixel size.
const TAB_STRIP: f32 = 48.0;

pub struct PaddockWindow {
    theme: Rc<Theme>,
    sidebar: Entity<Sidebar>,
    workspace: Workspace,
    panes: HashMap<PaneId, Entity<TerminalView>>,
    /// Font, size and corral program for new panes; its launch is replaced each time.
    template: Options,
    new_shell: NewShell,
    popup: Option<Popup>,
    tabs: ScrollHandle,
    /// The pet in the tab strip's spare room, unless turned off.
    pet: Option<Entity<PetView>>,
    /// The View menu ticks last set: folded, sorted by name.
    menu_state: Option<(bool, bool)>,
    /// The pet as configured: shown, and which.
    pet_setting: (bool, crate::pet::Pet),
    /// A close or quit question is showing.
    asking: bool,
    /// The selected row of the Attention list.
    attention_index: usize,
    /// The Go to Agent box while it is open, and its selected row.
    search: Option<Entity<TextInput>>,
    search_index: usize,
}

impl PaddockWindow {
    pub fn new(
        config: &Config,
        theme: Theme,
        options: Options,
        new_shell: NewShell,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = Rc::new(theme);
        // The sidebar's technical lines use the terminal's font.
        let mut mono = gpui::font(options.font_family.clone());
        if !options.fallbacks.is_empty() {
            mono.fallbacks = Some(gpui::FontFallbacks::from_fonts(options.fallbacks.clone()));
        }
        let sidebar = {
            let theme = theme.clone();
            let (width, corral) = (config.sidebar_width, options.corral.clone());
            let refresh = std::time::Duration::from_millis(config.refresh_ms);
            cx.new(|cx| Sidebar::new(theme, width, mono, corral, refresh, cx))
        };
        cx.subscribe_in(&sidebar, window, Self::on_sidebar).detach();
        let shown = match &options.launch {
            Launch::Empty => Shown::Empty,
            Launch::Agent { name } => Shown::Agent(name.clone()),
            Launch::Shell { .. } | Launch::Command { .. } => Shown::Shell,
        };
        let (workspace, first) = Workspace::new(shown);
        let mut this = Self {
            theme,
            sidebar,
            workspace,
            panes: HashMap::new(),
            template: Options {
                launch: Launch::Empty,
                ..options.clone()
            },
            new_shell,
            popup: None,
            tabs: ScrollHandle::new(),
            pet: config
                .mascot_enabled
                .then(|| cx.new(|cx| PetView::new(config.mascot, cx))),
            menu_state: None,
            pet_setting: (config.mascot_enabled, config.mascot),
            asking: false,
            attention_index: 0,
            search: None,
            search_index: 0,
        };
        let view = this.view(options.launch, window, cx);
        this.panes.insert(first, view);
        this.sync(cx);
        this
    }

    fn view(
        &self,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        let options = Options {
            launch,
            ..self.template.clone()
        };
        let theme = self.theme.clone();
        cx.new(|cx| TerminalView::new(options, theme, window, cx))
    }

    fn shell(&self) -> Launch {
        Launch::Shell {
            program: self.new_shell.program.clone(),
            cwd: self.new_shell.cwd.clone(),
        }
    }

    /// Tells the sidebar what is shown: the active pane's agent and every agent open here.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let selected = self.workspace.active_agent().map(str::to_owned);
        let here = self.workspace.agents();
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_view(selected, here, cx));
        cx.notify();
    }

    /// Views for panes the layout made on its own (the empty pane left after closing the last tab).
    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let missing: Vec<PaneId> = self
            .workspace
            .tabs
            .iter()
            .flat_map(|tab| tab.panes())
            .filter(|pane| !self.panes.contains_key(pane))
            .collect();
        for pane in missing {
            let view = self.view(Launch::Empty, window, cx);
            self.panes.insert(pane, view);
        }
    }

    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.scroll_to_item(self.workspace.active_tab);
        if let Some(view) = self.panes.get(&self.workspace.active_pane()) {
            let focus: FocusHandle = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
        self.sync(cx);
    }

    fn focus_pane(&mut self, pane: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.active_pane() != pane {
            self.workspace.focus(pane);
            self.focus_active(window, cx);
        }
    }

    fn on_sidebar(
        &mut self,
        _: &Entity<Sidebar>,
        event: &SidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SidebarEvent::Attach { name, metadata } => {
                self.show_agent(name, metadata.clone(), window, cx)
            }
            SidebarEvent::NewShell => self.new_shell(window, cx),
            // After this update: opening the window reads this one.
            SidebarEvent::NewAgent => cx.defer(|cx| windows::open_new_agent(Place::Current, cx)),
            SidebarEvent::Stop => self.stop_agent(window, cx),
            SidebarEvent::Attention => self.toggle_attention(cx),
            SidebarEvent::Alive(names) => {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                for view in self.panes.values() {
                    view.update(cx, |v, _| v.disappeared(&names));
                }
            }
        }
    }

    /// What the New Agent window starts from: the directories to offer and the active pane's.
    pub fn seed(&self, cx: &gpui::App) -> Seed {
        let sidebar = self.sidebar.read(cx);
        let active = self.workspace.active_pane();
        let project = match self.workspace.shown(active) {
            Shown::Agent(name) => sidebar.metadata(name).cwd,
            Shown::Shell => self.panes[&active].read(cx).cwd().map(str::to_owned),
            Shown::Empty => None,
        }
        .unwrap_or_else(|| self.new_shell.cwd.clone());
        let mut projects = sidebar.projects();
        projects.push(self.new_shell.cwd.clone());
        projects.sort();
        projects.dedup();
        Seed {
            theme: self.theme.clone(),
            corral: self.template.corral.clone(),
            mono: self.template.font_family.clone().into(),
            projects,
            project,
        }
    }

    /// An agent the New Agent window started, opened where it said. A running shell in the
    /// active pane is kept: the agent gets a new tab instead.
    pub fn open_started(
        &mut self,
        started: &Started,
        cwd: &str,
        place: Place,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = started.name.clone();
        let shown = Shown::Agent(name.clone());
        let active = self.workspace.active_pane();
        let pane = match place {
            Place::Current if !self.panes[&active].read(cx).shell_live() => {
                self.workspace.set_shown(active, shown);
                active
            }
            Place::Current | Place::Tab => self.workspace.new_tab(shown),
            Place::Split(direction) => self.workspace.split(direction, shown),
        };
        if !self.panes.contains_key(&pane) {
            let view = self.view(Launch::Empty, window, cx);
            self.panes.insert(pane, view);
        }
        let metadata = AgentMetadata {
            cwd: Some(cwd.to_owned()),
            instance: started.instance.clone(),
        };
        self.attach(pane, &name, metadata, cx);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.note(format!("Started {name}"), false, cx);
            sidebar.refresh();
        });
        self.focus_active(window, cx);
    }

    /// Stops the active pane's agent with `corral stop`, after asking.
    fn stop_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.workspace.active_agent().map(str::to_owned) else {
            self.sidebar.update(cx, |sidebar, cx| {
                let text = "Stop acts on the agent in the active pane; open one first.";
                sidebar.note(text.into(), true, cx)
            });
            return;
        };
        if self.asking {
            return;
        }
        self.asking = true;
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!("Stop {name}?"),
            Some("corral stop ends the agent and its session. Panes showing it stay, saying it has gone."),
            &["Stop", "Cancel"],
            cx,
        );
        let corral = self.template.corral.clone();
        cx.spawn_in(window, async move |this, cx| {
            let stop = matches!(answer.await, Ok(0));
            let _ = this.update(cx, |this, cx| {
                this.asking = false;
                if stop {
                    this.sidebar.update(cx, |sidebar, cx| {
                        sidebar.note(format!("Stopping {name}…"), false, cx)
                    });
                }
            });
            if !stop {
                return;
            }
            let target = name.clone();
            let result = cx
                .background_spawn(async move { new_agent::stop(&corral, &target) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |sidebar, cx| {
                    match result {
                        Ok(()) => sidebar.note(format!("Stopped {name}"), false, cx),
                        Err(error) => sidebar.note(format!("{error:#}"), true, cx),
                    }
                    sidebar.refresh();
                })
            });
        })
        .detach();
    }

    /// The colours in use, for the Settings and About windows.
    pub fn theme(&self) -> Rc<Theme> {
        self.theme.clone()
    }

    /// What Settings saved that takes effect at once: colours, sidebar width and the pet. The rest
    /// (fonts, refresh interval, corral command) waits for a restart.
    pub fn apply(&mut self, config: &Config, cx: &mut Context<Self>) {
        if let Ok(theme) = Theme::from_config(config) {
            let theme = Rc::new(theme);
            self.theme = theme.clone();
            let width = config.sidebar_width;
            let sidebar_theme = theme.clone();
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.restyle(sidebar_theme, width, cx));
            for view in self.panes.values() {
                let theme = theme.clone();
                view.update(cx, |view, cx| view.set_theme(theme, cx));
            }
        }
        let pet = (config.mascot_enabled, config.mascot);
        if pet != self.pet_setting {
            self.pet_setting = pet;
            self.pet = pet.0.then(|| cx.new(|cx| PetView::new(pet.1, cx)));
        }
        cx.notify();
    }

    /// A shell: in the active pane when it is empty, else in a new tab.
    fn new_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.workspace.active_pane();
        if self.workspace.shown(active) == &Shown::Empty {
            let NewShell { program, cwd } = &self.new_shell;
            let (program, cwd) = (program.clone(), cwd.clone());
            self.panes[&active].update(cx, |v, cx| v.start_shell(program, cwd, cx));
            self.workspace.set_shown(active, Shown::Shell);
        } else {
            let pane = self.workspace.new_tab(Shown::Shell);
            let view = self.view(self.shell(), window, cx);
            self.panes.insert(pane, view);
        }
        self.focus_active(window, cx);
    }

    fn open_popup(&mut self, popup: Popup, cx: &mut Context<Self>) {
        self.popup = Some(popup);
        cx.notify();
    }

    /// The tab after or before the active one, going round.
    fn cycle_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.workspace.tabs.len();
        let index = if forward {
            (self.workspace.active_tab + 1) % count
        } else {
            (self.workspace.active_tab + count - 1) % count
        };
        self.select_tab(index, window, cx);
    }

    /// ⌘1…⌘8 go to that tab if there is one; ⌘9 to the last, as in other macOS apps.
    fn nth_tab(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.workspace.tabs.len();
        let index = if n == 9 { count - 1 } else { n - 1 };
        if index < count {
            self.select_tab(index, window, cx);
        }
    }

    /// An agent chosen in the sidebar: where it already is, else by the layout rules.
    fn show_agent(
        &mut self,
        name: &str,
        metadata: AgentMetadata,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panes = &self.panes;
        let placement = self
            .workspace
            .place(name, |pane| panes[&pane].read(cx).shell_live());
        match placement {
            Placement::Existing { pane, .. } => {
                self.workspace.focus(pane);
                // Its attach may have ended since; open it again in the same place.
                if self.panes[&pane].read(cx).target() != Some(name) {
                    self.attach(pane, name, metadata, cx);
                }
            }
            Placement::Pane(pane) => {
                self.attach(pane, name, metadata, cx);
                self.workspace
                    .set_shown(pane, Shown::Agent(name.to_owned()));
            }
            Placement::NewTab => {
                let pane = self.workspace.new_tab(Shown::Agent(name.to_owned()));
                let view = self.view(Launch::Empty, window, cx);
                self.panes.insert(pane, view);
                self.attach(pane, name, metadata, cx);
            }
        }
        self.focus_active(window, cx);
    }

    fn attach(&self, pane: PaneId, name: &str, metadata: AgentMetadata, cx: &mut Context<Self>) {
        let name = name.to_owned();
        self.panes[&pane].update(cx, |v, cx| v.attach(name, metadata, cx));
    }

    /// A choice from the `+` or `Split ▾` dialog. An agent already open elsewhere moves here.
    fn choose(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.take() else {
            return;
        };
        let direction = match popup {
            Popup::NewTab => None,
            Popup::Split(Some(direction)) => Some(direction),
            Popup::Split(None) | Popup::Attention | Popup::Search => return,
        };
        if let Choice::Agent(name) = &choice
            && let Some(old) = self.workspace.find(name)
        {
            self.panes[&old].update(cx, |v, cx| v.close(cx));
            self.workspace.set_shown(old, Shown::Empty);
        }
        let shown = match &choice {
            Choice::Shell => Shown::Shell,
            Choice::Agent(name) => Shown::Agent(name.clone()),
        };
        let pane = match direction {
            None => self.workspace.new_tab(shown),
            Some(direction) => self.workspace.split(direction, shown),
        };
        let launch = match &choice {
            Choice::Shell => self.shell(),
            Choice::Agent(_) => Launch::Empty,
        };
        let view = self.view(launch, window, cx);
        self.panes.insert(pane, view);
        if let Choice::Agent(name) = &choice {
            let metadata = self.sidebar.read(cx).metadata(name);
            self.attach(pane, name, metadata, cx);
        }
        self.focus_active(window, cx);
    }

    fn close(&mut self, gone: Vec<PaneId>, window: &mut Window, cx: &mut Context<Self>) {
        for pane in gone {
            if let Some(view) = self.panes.remove(&pane) {
                view.update(cx, |v, cx| v.close(cx));
            }
        }
        self.fill(window, cx);
        self.focus_active(window, cx);
    }

    fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pane = self.workspace.active_pane();
        self.request_close(Closing::Pane(pane), window, cx);
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.workspace.tabs.get(index) {
            let pane = tab.active;
            self.request_close(Closing::Tab(pane), window, cx);
        }
    }

    /// The panes `closing` would close, as the layout is now.
    fn closing_panes(&self, closing: Closing) -> Vec<PaneId> {
        let (Closing::Pane(pane) | Closing::Tab(pane)) = closing;
        let Some(tab) = self
            .workspace
            .tabs
            .iter()
            .find(|t| t.panes().contains(&pane))
        else {
            return Vec::new();
        };
        match closing {
            Closing::Pane(_) => vec![pane],
            Closing::Tab(_) => tab.panes(),
        }
    }

    /// What to ask before closing `panes`: `None` when no live shell is among them.
    fn question(&self, panes: &[PaneId], quit: bool, cx: &Context<Self>) -> Option<Question> {
        let shells: Vec<String> = panes
            .iter()
            .filter(|pane| self.panes[pane].read(cx).shell_live())
            .map(|pane| self.title(*pane, cx))
            .collect();
        let agents = panes
            .iter()
            .filter(|pane| matches!(self.workspace.shown(**pane), Shown::Agent(_)))
            .count();
        close_question(&shells, agents, quit)
    }

    /// Closes at once, or after End Shells when live shells would end. A pane gone meanwhile is
    /// simply not closed.
    fn request_close(&mut self, closing: Closing, window: &mut Window, cx: &mut Context<Self>) {
        let panes = self.closing_panes(closing);
        let Some(question) = self.question(&panes, false, cx) else {
            self.finish_close(closing, window, cx);
            return;
        };
        if self.asking {
            return;
        }
        self.asking = true;
        let answer = question.ask(window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let end = answer.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.asking = false;
                if end {
                    this.finish_close(closing, window, cx);
                }
            });
        })
        .detach();
    }

    fn finish_close(&mut self, closing: Closing, window: &mut Window, cx: &mut Context<Self>) {
        let gone = match closing {
            Closing::Pane(pane) => self.workspace.close_pane(pane),
            Closing::Tab(pane) => {
                match self
                    .workspace
                    .tabs
                    .iter()
                    .position(|t| t.panes().contains(&pane))
                {
                    Some(index) => self.workspace.close_tab(index),
                    None => Vec::new(),
                }
            }
        };
        self.close(gone, window, cx);
    }

    /// Whether paddock may quit: yes without live shells, else as answered.
    pub fn confirm_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        let panes: Vec<PaneId> = self
            .workspace
            .tabs
            .iter()
            .flat_map(|tab| tab.panes())
            .collect();
        let Some(question) = self.question(&panes, true, cx) else {
            return Task::ready(true);
        };
        if self.asking {
            return Task::ready(false);
        }
        self.asking = true;
        let answer = question.ask(window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let quit = answer.await;
            let _ = this.update(cx, |this, _| this.asking = false);
            quit
        })
    }

    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.select_tab(index);
        self.focus_active(window, cx);
    }

    /// `Role · name` for an agent (`Agent` without a role label), `shell · <cwd>` for a shell,
    /// `Viewer` for an empty pane, as Saddle titles its panes.
    fn title(&self, pane: PaneId, cx: &Context<Self>) -> String {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => {
                let role = match self.sidebar.read(cx).role(name) {
                    Some(Role::Controller) => "Controller",
                    Some(Role::Regular) => "Regular",
                    Some(Role::Implementer) => "Implementer",
                    Some(Role::Reviewer) => "Reviewer",
                    None => "Agent",
                };
                format!("{role} · {name}")
            }
            Shown::Shell => self.panes[&pane].read(cx).subject().to_owned(),
            Shown::Empty => "Viewer".into(),
        }
    }

    /// A tab's label: what its active pane shows.
    fn tab_label(&self, pane: PaneId, cx: &Context<Self>) -> String {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => name.clone(),
            Shown::Shell => {
                let subject = self.panes[&pane].read(cx).subject().to_owned();
                match subject.rsplit('/').next() {
                    Some(dir) if !dir.is_empty() => format!("shell · {dir}"),
                    _ => "shell".into(),
                }
            }
            Shown::Empty => "empty".into(),
        }
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn highlight(&self) -> Hsla {
        hsla(self.theme.bg(|t| t.agent_selected), 1.0)
    }

    fn tab_strip(&self, cx: &mut Context<Self>) -> Div {
        let highlight = self.highlight();
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(px(26.0))
                .rounded(px(6.0))
                .text_size(px(14.0))
                .text_color(self.fg(|t| t.muted))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(label)
        };
        let count = self.workspace.tabs.len();
        let mut tabs = div()
            .id("tabs")
            .flex()
            .items_center()
            .gap(px(4.0))
            .min_w(px(0.0))
            .flex_shrink(1.0)
            .overflow_x_scroll()
            .track_scroll(&self.tabs);
        for (index, tab) in self.workspace.tabs.iter().enumerate() {
            let active = index == self.workspace.active_tab;
            let panes = tab.panes().len();
            let mut label = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .whitespace_nowrap()
                .child(self.tab_label(tab.active, cx));
            if panes > 1 {
                label = label.child(
                    div()
                        .text_size(px(TEXT - 2.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(panes.to_string()),
                );
            }
            let close = div()
                .id(("close-tab", index))
                .flex_shrink_0()
                .px(px(3.0))
                .rounded(px(4.0))
                .text_color(self.fg(|t| t.agents_dim))
                .hover(move |style| style.bg(highlight))
                .child("×")
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    this.close_tab(index, window, cx);
                }));
            let mut item = div()
                .id(("tab", index))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(28.0))
                .pl(px(12.0))
                .pr(px(6.0))
                .rounded(px(7.0))
                .border_1()
                .text_size(px(TEXT))
                .cursor_pointer();
            item = if active {
                item.bg(highlight)
                    .border_color(self.fg(|t| t.focus))
                    .text_color(self.fg(|t| t.agents_text))
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                item.border_color(self.fg(|t| t.agents_rule))
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(highlight.opacity(0.6)))
            };
            tabs = tabs.child(item.child(label).child(close).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| this.select_tab(index, window, cx),
            )));
        }
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(TAB_STRIP))
            .px(px(GAP))
            .child(
                button("new-tab", "+").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.popup = Some(Popup::NewTab);
                    cx.notify();
                })),
            )
            .child(button("prev-tab", "‹").on_click(cx.listener(
                |this, _: &ClickEvent, window, cx| {
                    let index = this.workspace.active_tab.saturating_sub(1);
                    this.select_tab(index, window, cx);
                },
            )))
            .child(button("next-tab", "›").on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    let index = (this.workspace.active_tab + 1).min(count - 1);
                    this.select_tab(index, window, cx);
                },
            )))
            .child(tabs)
            // Spare room, where the pet walks.
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .ml(px(GAP))
                    .children(self.pet.clone()),
            )
    }

    fn node(&self, node: &Node, cx: &mut Context<Self>) -> AnyElement {
        match node {
            Node::Pane(pane) => self.pane(*pane, cx),
            Node::Split {
                axis,
                first,
                second,
            } => {
                let split = div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .gap(px(GAP));
                let split = match axis {
                    Axis::Row => split.flex_row(),
                    Axis::Column => split.flex_col(),
                };
                split
                    .child(self.node(first, cx))
                    .child(self.node(second, cx))
                    .into_any_element()
            }
        }
    }

    fn pane(&self, pane: PaneId, cx: &mut Context<Self>) -> AnyElement {
        let active = pane == self.workspace.active_pane();
        let highlight = self.highlight();
        let control = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px(px(7.0))
                .py(px(2.0))
                .rounded(px(5.0))
                .text_color(self.fg(|t| t.muted))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(label)
        };
        let mut header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(2.0))
            .h(px(28.0))
            .pl(px(10.0))
            .pr(px(4.0))
            .text_size(px(TEXT))
            .border_b_1()
            .border_color(self.fg(|t| t.agents_rule))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if active {
                        self.fg(|t| t.agents_text)
                    } else {
                        self.fg(|t| t.muted)
                    })
                    .child(self.title(pane, cx)),
            );
        if active {
            header = header
                .child(control("split", "Split ▾").on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.popup = Some(Popup::Split(None));
                        cx.notify();
                    },
                )))
                .child(control("close-pane", "Close pane").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.close_pane(window, cx)),
                ))
                .child(control("close-this-tab", "Close tab").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        let index = this.workspace.active_tab;
                        this.close_tab(index, window, cx);
                    },
                )));
        }
        div()
            .id(("pane", pane as usize))
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(if active {
                self.fg(|t| t.focus)
            } else {
                self.fg(|t| t.agents_rule)
            })
            .overflow_hidden()
            .bg(hsla(self.theme.terminal().background, 1.0))
            .capture_any_mouse_down(
                cx.listener(move |this, _, window, cx| this.focus_pane(pane, window, cx)),
            )
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(self.panes[&pane].clone()),
            )
            .into_any_element()
    }

    fn toggle_attention(&mut self, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Attention) {
            self.popup = None;
        } else {
            self.popup = Some(Popup::Attention);
            self.attention_index = 0;
        }
        cx.notify();
    }

    /// Opens an Attention item's agent, where it is or by the layout rules; a failed read has
    /// nothing to open.
    fn open_attention(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup != Some(Popup::Attention) {
            return;
        }
        let items = self.sidebar.read(cx).attention();
        let Some(name) = items.get(index).and_then(|item| item.agent.clone()) else {
            return;
        };
        self.popup = None;
        let metadata = self.sidebar.read(cx).metadata(&name);
        self.show_agent(&name, metadata, window, cx);
    }

    /// The Attention list, floating under the sidebar's header.
    fn attention_panel(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let items = self.sidebar.read(cx).attention();
        let selected = self.attention_index.min(items.len().saturating_sub(1));
        let highlight = self.highlight();
        let mut list = div().flex().flex_col().gap(px(2.0));
        if items.is_empty() {
            list = list.child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child("Nothing needs attention."),
            );
        }
        for (index, item) in items.iter().enumerate() {
            let mark: Pick = match item.kind {
                AttentionKind::Waiting => |t| t.agent_blocked,
                AttentionKind::Error | AttentionKind::ReadFailed => |t| t.agent_error,
                AttentionKind::Reply => |t| t.unread,
            };
            let mut detail = div()
                .flex()
                .gap(px(6.0))
                .text_size(px(TEXT - 1.0))
                .child(div().text_color(self.fg(|t| t.muted)).child(item.reason()));
            if !item.note.is_empty() {
                detail = detail.child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(item.note.clone()),
                );
            }
            let mut row = div()
                .id(("attention-item", index))
                .flex()
                .items_start()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .child(
                    div()
                        .w(px(12.0))
                        .flex_shrink_0()
                        .font_weight(FontWeight::BOLD)
                        .text_color(self.fg(mark))
                        .child(item.mark()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(self.fg(|t| t.agents_text))
                                .child(item.label.clone()),
                        )
                        .child(detail),
                );
            if index == selected {
                row = row.bg(highlight);
            }
            if item.agent.is_some() {
                row = row
                    .cursor_pointer()
                    .hover(move |style| style.bg(highlight))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open_attention(index, window, cx)
                    }));
            }
            list = list.child(row);
        }
        let card = div()
            .id("attention-list")
            .absolute()
            .top(px(44.0))
            .left(px(GAP))
            .w(px(400.0))
            .max_h(px(520.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .p(px(8.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(self.fg(|t| t.focus))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow_lg()
            .text_size(px(TEXT + 1.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(8.0))
                    .px(px(6.0))
                    .pb(px(2.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(self.fg(|t| t.agents_text))
                            .child("Attention"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(TEXT - 1.0))
                            .text_color(self.fg(|t| t.agents_dim))
                            .child("↑↓  ⏎ open  esc"),
                    ),
            )
            .child(list);
        // A click outside closes it; nothing is dimmed, as for a menu.
        div()
            .id("attention-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.popup = None;
                    cx.notify();
                }),
            )
            .child(card)
    }

    /// Go to Agent: a search box over the window, focused.
    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Search) {
            self.close_popup(window, cx);
            return;
        }
        let colors = text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        };
        let input = cx
            .new(|cx| TextInput::new("", "Agent name or project, or a Settings page", colors, cx));
        cx.subscribe(&input, |this, _, _: &Changed, cx| {
            this.search_index = 0;
            cx.notify();
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.search = Some(input);
        self.search_index = 0;
        self.popup = Some(Popup::Search);
        cx.notify();
    }

    fn search_entries(&self, cx: &Context<Self>) -> Vec<search::Entry> {
        let query = self
            .search
            .as_ref()
            .map(|input| input.read(cx).text().to_owned())
            .unwrap_or_default();
        search::entries(&self.sidebar.read(cx).agents(), &query)
    }

    /// Closes the open chooser or list and gives the keys back to the active pane.
    fn close_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.search = None;
        self.focus_active(window, cx);
    }

    /// ↑↓ in the Attention or Go to Agent list.
    fn move_selection(&mut self, step: isize, cx: &mut Context<Self>) {
        let (count, index) = match self.popup {
            Some(Popup::Attention) => (
                self.sidebar.read(cx).attention().len(),
                &mut self.attention_index,
            ),
            Some(Popup::Search) => {
                let count = self.search_entries(cx).len();
                (count, &mut self.search_index)
            }
            _ => return,
        };
        if count > 0 {
            let moved = ((*index).min(count - 1) as isize + step).clamp(0, count as isize - 1);
            *index = moved as usize;
            cx.notify();
        }
    }

    /// ⏎ in the Attention or Go to Agent list.
    fn open_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.popup {
            Some(Popup::Attention) => {
                let index = self.attention_index;
                self.open_attention(index, window, cx)
            }
            Some(Popup::Search) => {
                let index = self.search_index;
                self.open_search_entry(index, window, cx)
            }
            _ => {}
        }
    }

    /// Opens a Go to Agent entry: the agent where it is or by the layout rules, or Settings at
    /// that page.
    fn open_search_entry(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.search_entries(cx).into_iter().nth(index) else {
            return;
        };
        self.close_popup(window, cx);
        match entry.target {
            search::Target::Agent(name) => {
                let metadata = self.sidebar.read(cx).metadata(&name);
                self.show_agent(&name, metadata, window, cx);
            }
            // After this update: opening Settings reads this window.
            search::Target::Settings(page) => {
                cx.defer(move |cx| windows::open_settings_at(page, cx))
            }
        }
    }

    /// The Go to Agent box and its matches, at the top of the window.
    fn search_panel(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let entries = self.search_entries(cx);
        let selected = self.search_index.min(entries.len().saturating_sub(1));
        let highlight = self.highlight();
        let mut list = div().flex().flex_col().gap(px(2.0));
        if entries.is_empty() {
            list = list.child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child("No agent or page matches."),
            );
        }
        for (index, entry) in entries.iter().enumerate() {
            let settings = matches!(entry.target, search::Target::Settings(_));
            let mut row = div()
                .id(("search-entry", index))
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(
                    div()
                        .w(px(14.0))
                        .flex_shrink_0()
                        .text_color(self.fg(|t| t.agents_accent))
                        .child(if settings { "⚙" } else { "›" }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(self.fg(|t| t.agents_text))
                        .child(entry.label.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(TEXT - 1.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(entry.detail.clone()),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.open_search_entry(index, window, cx)
                }));
            if index == selected {
                row = row.bg(highlight);
            }
            list = list.child(row);
        }
        let card = div()
            .id("search")
            .w(px(520.0))
            .max_h(px(460.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .p(px(8.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(self.fg(|t| t.focus))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow_lg()
            .text_size(px(TEXT + 1.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(30.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(self.fg(|t| t.agents_rule))
                    .bg(hsla(self.theme.terminal().background, 1.0))
                    .children(self.search.clone()),
            )
            .child(div().id("search-list").overflow_y_scroll().child(list));
        div()
            .id("search-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(70.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(card)
    }

    fn dialog(&self, popup: Popup, cx: &mut Context<Self>) -> Stateful<Div> {
        let highlight = self.highlight();
        let row = |id: ElementId| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
        };
        let title = match popup {
            Popup::NewTab => "Open in a new tab".to_owned(),
            Popup::Split(None) => "Split: which side?".to_owned(),
            Popup::Split(Some(direction)) => format!("Split {}", side(direction)),
            Popup::Attention | Popup::Search => String::new(),
        };
        let mut body = div().flex().flex_col().gap(px(2.0));
        if popup == Popup::Split(None) {
            let mut grid = div().flex().flex_wrap().gap(px(6.0));
            for (direction, label) in [
                (Direction::Left, "← Left"),
                (Direction::Right, "→ Right"),
                (Direction::Up, "↑ Up"),
                (Direction::Down, "↓ Down"),
            ] {
                grid = grid.child(
                    row(label.into())
                        .w(px(128.0))
                        .justify_center()
                        .border_1()
                        .border_color(self.fg(|t| t.agents_rule))
                        .child(label)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.popup = Some(Popup::Split(Some(direction)));
                            cx.notify();
                        })),
                );
            }
            body = body.child(grid);
        } else {
            body = body.child(
                row("choice-shell".into())
                    .child(
                        div()
                            .text_color(self.fg(|t| t.agents_accent))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("＋"),
                    )
                    .child("Shell")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.choose(Choice::Shell, window, cx)
                    })),
            );
            body = body.child(
                row("choice-new-agent".into())
                    .child(
                        div()
                            .text_color(self.fg(|t| t.agents_accent))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("＋"),
                    )
                    .child("New agent…")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        let place = match this.popup.take() {
                            Some(Popup::Split(Some(direction))) => Place::Split(direction),
                            _ => Place::Tab,
                        };
                        cx.notify();
                        cx.defer(move |cx| windows::open_new_agent(place, cx));
                    })),
            );
            let names = self.sidebar.read(cx).agent_names();
            if names.is_empty() {
                body = body.child(
                    div()
                        .px(px(10.0))
                        .py(px(5.0))
                        .text_color(self.fg(|t| t.agents_dim))
                        .child("No agents to open here."),
                );
            }
            for name in names {
                let open = self.workspace.find(&name).is_some();
                let mut entry = row(ElementId::Name(name.clone().into()))
                    .child(div().flex_1().min_w(px(0.0)).child(name.clone()));
                if open {
                    entry = entry.child(
                        div()
                            .text_size(px(TEXT - 1.0))
                            .text_color(self.fg(|t| t.connected))
                            .child("Move here"),
                    );
                }
                body = body.child(entry.on_click(cx.listener(
                    move |this, _: &ClickEvent, window, cx| {
                        this.choose(Choice::Agent(name.clone()), window, cx)
                    },
                )));
            }
        }
        let card = div()
            .id("dialog")
            .w(px(300.0))
            .max_h(px(480.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(10.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(self.fg(|t| t.focus))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(px(TEXT + 1.0))
            .text_color(self.fg(|t| t.agents_text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(4.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(body)
            .child(
                div().flex().justify_end().child(
                    row("cancel".into())
                        .text_color(self.fg(|t| t.muted))
                        .child("Cancel")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.popup = None;
                            cx.notify();
                        })),
                ),
            );
        // Only the dialog takes clicks while it is open; a click outside cancels it.
        div()
            .id("dialog-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.35))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.popup = None;
                    cx.notify();
                }),
            )
            .child(card)
    }
}

fn side(direction: Direction) -> &'static str {
    match direction {
        Direction::Left => "left",
        Direction::Right => "right",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

impl Render for PaddockWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fill(window, cx);
        // The View menu ticks follow the sidebar.
        let view_state = self.sidebar.read(cx).view_state();
        if self.menu_state != Some(view_state) {
            self.menu_state = Some(view_state);
            cx.set_menus(menu::menus(view_state.0, view_state.1));
        }
        let root = self.workspace.tab().root.clone();
        let content = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .px(px(GAP))
            .pb(px(GAP))
            .child(self.node(&root, cx));
        let main = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .child(self.tab_strip(cx))
            .child(content);
        let dialog = self.popup.map(|popup| match popup {
            Popup::Attention => self.attention_panel(cx),
            Popup::Search => self.search_panel(cx),
            popup => self.dialog(popup, cx),
        });
        div()
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .when(self.popup.is_some(), |root| root.key_context(menu::DIALOG))
            .on_action(
                cx.listener(|this, _: &menu::NewTab, _, cx| this.open_popup(Popup::NewTab, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::NewShell, window, cx| this.new_shell(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::SplitRight, _, cx| {
                this.open_popup(Popup::Split(Some(Direction::Right)), cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitDown, _, cx| {
                this.open_popup(Popup::Split(Some(Direction::Down)), cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitLeft, _, cx| {
                this.open_popup(Popup::Split(Some(Direction::Left)), cx)
            }))
            .on_action(cx.listener(|this, _: &menu::SplitUp, _, cx| {
                this.open_popup(Popup::Split(Some(Direction::Up)), cx)
            }))
            .on_action(
                cx.listener(|this, _: &menu::ClosePane, window, cx| this.close_pane(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::StopAgent, window, cx| this.stop_agent(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::ShowAttention, _, cx| this.toggle_attention(cx)),
            )
            .on_action(
                cx.listener(|this, _: &menu::GoToAgent, window, cx| this.open_search(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::SelectNext, _, cx| this.move_selection(1, cx)))
            .on_action(
                cx.listener(|this, _: &menu::SelectPrevious, _, cx| this.move_selection(-1, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                this.open_selected(window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::CloseTab, window, cx| {
                let index = this.workspace.active_tab;
                this.close_tab(index, window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::ToggleFold, _, cx| {
                this.sidebar.update(cx, |s, cx| s.toggle_fold(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &menu::ToggleSortByName, _, cx| {
                this.sidebar.update(cx, |s, cx| s.toggle_sort(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &menu::Minimize, window, _| window.minimize_window()))
            .on_action(cx.listener(|_, _: &menu::Zoom, window, _| window.zoom_window()))
            .on_action(
                cx.listener(|this, _: &menu::NextTab, window, cx| this.cycle_tab(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::PreviousTab, window, cx| {
                this.cycle_tab(false, window, cx)
            }))
            .on_action(cx.listener(|this, _: &menu::Tab1, w, cx| this.nth_tab(1, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab2, w, cx| this.nth_tab(2, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab3, w, cx| this.nth_tab(3, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab4, w, cx| this.nth_tab(4, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab5, w, cx| this.nth_tab(5, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab6, w, cx| this.nth_tab(6, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab7, w, cx| this.nth_tab(7, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab8, w, cx| this.nth_tab(8, w, cx)))
            .on_action(cx.listener(|this, _: &menu::Tab9, w, cx| this.nth_tab(9, w, cx)))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_popup(window, cx)),
            )
            .child(self.sidebar.clone())
            .child(main)
            .children(dialog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_live_shells_make_closing_ask() {
        assert_eq!(close_question(&[], 2, false), None);
        assert_eq!(close_question(&[], 0, true), None);
        let one = close_question(&["shell · /tmp".into()], 0, false).unwrap();
        assert_eq!(one.message, "End the running shell?");
        assert_eq!(one.confirm, "End Shell");
        assert!(
            one.detail
                .contains("What runs in it in the foreground ends too.")
        );
        assert!(one.detail.starts_with("shell · /tmp\n\n"));
        assert!(!one.detail.contains("Agent"));
    }

    #[test]
    fn the_question_lists_every_shell_and_says_agents_only_detach() {
        let shells = vec![
            "shell · /tmp".to_owned(),
            "shell · /Users/me/code".to_owned(),
        ];
        let quit = close_question(&shells, 1, true).unwrap();
        assert_eq!(quit.message, "Quit paddock and end the running shells?");
        assert_eq!(quit.confirm, "Quit");
        assert!(quit.detail.contains("What runs in them"));
        assert_eq!(
            close_question(&shells, 0, false).unwrap().confirm,
            "End Shells"
        );
        assert!(
            quit.detail
                .starts_with("shell · /tmp\nshell · /Users/me/code\n\n")
        );
        assert!(
            quit.detail
                .contains("Agent views only detach; the agents keep running.")
        );
    }
}
