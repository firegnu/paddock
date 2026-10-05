//! The window's root view: the title bar, where the tabs share the traffic lights' row; below it
//! the Agents sidebar on the left and the active tab's panes on the right, each split pane under a
//! slim header. `layout.rs` holds the rules; this file draws them and keeps one terminal view per
//! pane.
use crate::{
    agents::Panel,
    attention::Kind as AttentionKind,
    card,
    config::Config,
    corral::{Agent, Role},
    diagnostics::{Report, Startup},
    fonts::UiFont,
    footer_icon::{self, Icon},
    layout::{Axis, Direction, Node, PaneId, Placement, Shown, Workspace},
    layout_state::{Content, Layout, Store},
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
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, ExternalPaths, FocusHandle, Focusable, FontWeight,
    Hsla, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, PromptLevel, Render,
    ScrollHandle, SharedString, Stateful, Task, Window, div, point, prelude::*, px,
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
/// The title bar's height at the base interface size: the traffic lights' row, which holds the
/// tabs. It grows with larger interface sizes, never shrinks below this.
pub const TITLE_BAR: f32 = 40.0;
/// The title bar with the pet shown: room for it at twice its pixel size, standing on the
/// terminal's top edge.
pub const PET_TITLE_BAR: f32 = 48.0;
/// Room the traffic lights take from the window's left edge, for when the sidebar is narrower.
const LIGHTS: f32 = 84.0;
/// What the dimmed panes are covered with: the terminal's background, this opaque.
const DIM: f32 = 0.42;

/// Where the traffic lights go in a title bar `height` points tall: in from the left edge, and
/// centred on the row (AppKit's buttons are 14 points tall).
pub fn traffic_lights(height: f32) -> Point<Pixels> {
    point(px(14.0), px(((height - 14.0) / 2.0).max(0.0)))
}

fn title_bar_height(ui: &UiFont, pet: bool) -> f32 {
    let base = if pet { PET_TITLE_BAR } else { TITLE_BAR };
    base.max(ui.scale(base))
}

/// What a pane shows, as far as its tab and its header name it.
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    Agent(String),
    /// A shell: the program and the directory it started in.
    Shell {
        program: String,
        cwd: String,
    },
    /// A program started with `-- PROGRAM ARG…`, as typed.
    Command(String),
    Empty,
}

/// The last part of a path: `paddock` for `/Users/me/paddock`, `/` for the root.
fn last_part(path: &str) -> &str {
    match path.trim_end_matches('/').rsplit('/').next() {
        Some(part) if !part.is_empty() => part,
        _ => "/",
    }
}

/// A tab's title, for the pane it shows: an agent by its name without the group prefix, unless
/// another agent open in the window (`open`) would read the same; a shell as `zsh · paddock`.
pub fn tab_title(subject: &Subject, open: &[String]) -> String {
    match subject {
        Subject::Agent(name) => {
            let short = name.rsplit('/').next().unwrap_or(name);
            let clash = open
                .iter()
                .any(|other| other != name && other.rsplit('/').next() == Some(short));
            if clash || short.is_empty() {
                name.clone()
            } else {
                short.to_owned()
            }
        }
        Subject::Shell { program, cwd } => format!("{} · {}", last_part(program), last_part(cwd)),
        Subject::Command(command) => command.clone(),
        Subject::Empty => "empty".into(),
    }
}

/// The faint count after a tab's title for the panes it holds besides the one named.
pub fn more_panes(panes: usize) -> Option<String> {
    (panes > 1).then(|| format!("+{}", panes - 1))
}

/// A directory, short: `~` for home, and only the last part of anything deeper, as `~/…/paddock`.
pub fn short_dir(path: &str, home: Option<&str>) -> String {
    let path = path.trim_end_matches('/');
    let (root, rest) = match home.map(|home| home.trim_end_matches('/')) {
        Some(home) if !home.is_empty() && path == home => return "~".into(),
        Some(home) if !home.is_empty() && path.starts_with(&format!("{home}/")) => {
            ("~/", &path[home.len() + 1..])
        }
        _ => ("/", path.trim_start_matches('/')),
    };
    match rest.split('/').count() {
        _ if rest.is_empty() => root.into(),
        1 => format!("{root}{rest}"),
        _ => format!("{root}…/{}", last_part(rest)),
    }
}

/// A split pane's header: the name, in full, and the directory, short, when there is one.
pub fn pane_name(
    subject: &Subject,
    agent_cwd: Option<&str>,
    home: Option<&str>,
) -> (String, Option<String>) {
    match subject {
        Subject::Agent(name) => (name.clone(), agent_cwd.map(|cwd| short_dir(cwd, home))),
        Subject::Shell { program, cwd } => {
            (last_part(program).to_owned(), Some(short_dir(cwd, home)))
        }
        Subject::Command(command) => (command.clone(), None),
        Subject::Empty => ("empty".into(), None),
    }
}

/// Panes get a header only when their tab is split: alone, the terminal is all there is. A zoomed
/// pane keeps its header, for Restore.
pub fn header_shown(panes_in_tab: usize) -> bool {
    panes_in_tab > 1
}

/// The panes on screen besides the active one are dimmed; one alone is never.
pub fn dimmed(panes_shown: usize, active: bool) -> bool {
    panes_shown > 1 && !active
}

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
    /// Where the layout is saved.
    store: Store,
    /// The config came from its file at startup, rather than defaults.
    config_from_file: bool,
    /// The Go to Agent box while it is open, and its selected row.
    search: Option<Entity<TextInput>>,
    search_index: usize,
    /// The sidebar's width, which the title bar leaves empty before the tabs.
    sidebar_width: f32,
    /// A press on the title bar's empty part: moving now drags the window.
    dragging: bool,
    /// The title bar height the traffic lights were last centred on.
    lights: Option<f32>,
}

impl PaddockWindow {
    pub fn new(
        config: &Config,
        theme: Theme,
        options: Options,
        new_shell: NewShell,
        // Where the layout is saved, and the saved layout to open.
        (store, saved): (Store, Option<Layout>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let theme = Rc::new(theme);
        let mono = mono_font(&options);
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
        let (workspace, restored) = match &saved {
            Some(layout) => {
                let (workspace, contents) = layout.workspace();
                (workspace, Some(contents))
            }
            None => (Workspace::new(shown).0, None),
        };
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
            sidebar_width: config.sidebar_width,
            dragging: false,
            lights: None,
            store,
            config_from_file: crate::config::default_path().exists(),
        };
        match restored {
            Some(contents) => this.restore(contents, window, cx),
            None => {
                let first = this.workspace.active_pane();
                let view = this.view(options.launch, window, cx);
                this.panes.insert(first, view);
            }
        }
        if let Some(problem) = this.store.problem() {
            this.sidebar
                .update(cx, |sidebar, cx| sidebar.note(problem, true, cx));
        }
        this.sync(cx);
        this
    }

    /// Opens the saved panes: shells start afresh in their directories, agents attach again
    /// (saying so when they are gone or were restarted), empty panes stay empty.
    fn restore(
        &mut self,
        contents: Vec<(PaneId, Content)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for (pane, content) in contents {
            let launch = match &content {
                Content::Shell { cwd } => Launch::Shell {
                    program: self.new_shell.program.clone(),
                    cwd: cwd.clone(),
                },
                Content::Empty | Content::Agent { .. } => Launch::Empty,
            };
            let view = self.view(launch, window, cx);
            self.panes.insert(pane, view);
            if let Content::Agent {
                name,
                cwd,
                instance,
            } = content
            {
                let metadata = AgentMetadata { cwd, instance };
                self.attach(pane, &name, metadata, cx);
            }
        }
        self.focus_active(window, cx);
    }

    /// What a pane shows, in full, for the saved layout.
    fn content(&self, pane: PaneId, cx: &gpui::App) -> Content {
        match self.workspace.shown(pane) {
            Shown::Empty => Content::Empty,
            Shown::Shell => {
                let cwd = self.panes.get(&pane).and_then(|view| view.read(cx).cwd());
                let cwd = cwd
                    .filter(|cwd| std::path::Path::new(cwd).is_absolute())
                    .unwrap_or(&self.new_shell.cwd);
                Content::Shell {
                    cwd: cwd.to_owned(),
                }
            }
            Shown::Agent(name) => {
                let mut metadata = self
                    .panes
                    .get(&pane)
                    .map(|view| view.read(cx).agent_metadata())
                    .unwrap_or_default();
                let listed = self.sidebar.read(cx).metadata(name);
                metadata.cwd = metadata.cwd.or(listed.cwd);
                metadata.instance = metadata.instance.or(listed.instance);
                Content::Agent {
                    name: name.clone(),
                    cwd: metadata
                        .cwd
                        .filter(|cwd| std::path::Path::new(cwd).is_absolute()),
                    instance: metadata.instance,
                }
            }
        }
    }

    /// What Diagnostics shows from this window: the commands it runs and how its reads and saves
    /// went.
    pub fn report(&self, cx: &gpui::App) -> Report {
        Report {
            corral: self.template.corral.clone(),
            shell: self.new_shell.program.clone(),
            agents: self.sidebar.read(cx).last_read(),
            config_path: crate::config::default_path(),
            config_from_file: self.config_from_file,
            layout_path: self.store.path().map(std::path::Path::to_path_buf),
            restore: self.store.restored.clone(),
            save: self.store.saved.clone(),
            save_off: self.store.protected(),
            startup: cx.try_global::<Startup>().cloned().unwrap_or_default(),
            checked: std::time::SystemTime::now(),
        }
    }

    /// Saves the layout when it changed since the last save.
    pub fn save_layout(&mut self, cx: &gpui::App) {
        let layout = Layout::of(&self.workspace, |pane| self.content(pane, cx));
        self.store.save(&layout);
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
        self.save_layout(cx);
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

    /// What Settings saved that takes effect at once: colours, sidebar width, the pet, the
    /// interface font and the terminal font, which open terminals take up as if resized. The rest
    /// (refresh interval, corral command) waits for a restart.
    pub fn apply(&mut self, config: &Config, cx: &mut Context<Self>) {
        let ui = UiFont::from_config(config);
        if UiFont::get(cx) != ui {
            cx.set_global(ui);
            cx.refresh_windows();
        }
        let template = &self.template;
        if (
            &template.font_family,
            &template.fallbacks,
            template.font_size,
            template.line_height,
        ) != (
            &config.font,
            &config.font_fallbacks,
            config.font_size,
            config.line_height,
        ) {
            self.template.font_family = config.font.clone();
            self.template.fallbacks = config.font_fallbacks.clone();
            self.template.font_size = config.font_size;
            self.template.line_height = config.line_height;
            for view in self.panes.values() {
                view.update(cx, |view, cx| view.set_font(&self.template, cx));
            }
            let mono = mono_font(&self.template);
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.set_mono(mono, cx));
        }
        if let Ok(theme) = Theme::from_config(config) {
            let theme = Rc::new(theme);
            self.theme = theme.clone();
            let width = config.sidebar_width;
            self.sidebar_width = width;
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

    fn toggle_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.toggle_zoom();
        self.focus_active(window, cx);
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

    /// What a pane shows, for its tab and its header.
    fn subject(&self, pane: PaneId, cx: &Context<Self>) -> Subject {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => Subject::Agent(name.clone()),
            Shown::Shell => {
                let view = self.panes[&pane].read(cx);
                match view.cwd() {
                    Some(cwd) => Subject::Shell {
                        program: self.new_shell.program.clone(),
                        cwd: cwd.to_owned(),
                    },
                    None => Subject::Command(view.subject().to_owned()),
                }
            }
            Shown::Empty => Subject::Empty,
        }
    }

    /// A pane's status dot: an agent's status colour as the sidebar shows it, a neutral one for
    /// a shell, a faint one for an empty pane or an agent no longer listed.
    fn dot(&self, pane: PaneId, agents: &[Agent], now: f64) -> Hsla {
        match self.workspace.shown(pane) {
            Shown::Agent(name) => match agents.iter().find(|a| &a.name == name) {
                Some(agent) => {
                    let status = Panel::default().status(agent, now);
                    self.fg(card::look(status).color)
                }
                None => self.fg(|t| t.agents_faint),
            },
            Shown::Shell => self.fg(|t| t.muted),
            Shown::Empty => self.fg(|t| t.agents_faint),
        }
    }

    fn fg(&self, pick: Pick) -> Hsla {
        hsla(self.theme.fg(pick), 1.0)
    }

    fn highlight(&self) -> Hsla {
        hsla(self.theme.bg(|t| t.agent_selected), 1.0)
    }

    /// The title bar: the traffic lights over the sidebar, then the tabs from the terminal's left
    /// edge, the `+` after the last, and the pet in the room left. What is not a tab or a button
    /// drags the window, and a double click there zooms or minimises it as the system is set.
    fn title_bar(&self, agents: &[Agent], now: f64, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
        let scale = ui.scale(1.0);
        let highlight = self.highlight();
        // A hovered tab's faint ground, solid, so the × drawn over its title can hide the text.
        let hovered = hsla(self.theme.bg(|t| t.agents_bg), 1.0).blend(highlight.opacity(0.6));
        let muted = self.theme.fg(|t| t.muted);
        let open = self.workspace.agents();
        // A press on a tab or a button is theirs, not the start of a drag.
        let keep = |_: &MouseDownEvent, _: &mut Window, cx: &mut gpui::App| cx.stop_propagation();
        let mut tabs = div()
            .id("tabs")
            .flex()
            .items_center()
            .gap(px(4.0))
            .min_w(px(0.0))
            .flex_shrink(1.0)
            .h_full()
            .overflow_x_scroll()
            .track_scroll(&self.tabs);
        for (index, tab) in self.workspace.tabs.iter().enumerate() {
            let active = index == self.workspace.active_tab;
            let group = SharedString::from(format!("tab-{index}"));
            // The active tab keeps its × in line; the others show it on hover over the end of
            // their title, so a narrow tab gives the title all its room.
            let under = if active { highlight } else { hovered };
            let close = div()
                .id(("close-tab", index))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(16.0))
                .rounded(px(4.0))
                .bg(under)
                .hover(move |style| style.bg(under.blend(hsla(muted, 0.18))))
                .when(!active, |close| {
                    close
                        .absolute()
                        .top(ui.px(6.0))
                        .right(ui.px(6.0))
                        .invisible()
                        .group_hover(group.clone(), |style| style.visible())
                })
                .child(footer_icon::icon(
                    Icon::Close,
                    self.fg(|t| t.muted),
                    scale * 0.85,
                ))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    this.close_tab(index, window, cx);
                }));
            let mut item = div()
                .id(("tab", index))
                .group(group)
                .relative()
                .max_w(ui.px(220.0))
                .flex()
                .items_center()
                .gap(ui.px(7.0))
                .h(ui.px(28.0))
                .pl(ui.px(11.0))
                .rounded(px(7.0))
                .text_size(ui.px(12.5))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, keep)
                .child(
                    div()
                        .flex_shrink_0()
                        .size(ui.px(7.0))
                        .rounded_full()
                        .bg(self.dot(tab.active, agents, now)),
                )
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(tab_title(&self.subject(tab.active, cx), &open)),
                )
                .children(more_panes(tab.panes().len()).map(|more| {
                    div()
                        .flex_shrink_0()
                        .text_color(self.fg(|t| t.agents_dim))
                        .child(more)
                }));
            // Short of room, the other tabs narrow first; the active one keeps its title.
            item = if active {
                item.flex_shrink_0()
                    .pr(ui.px(6.0))
                    .bg(highlight)
                    .text_color(self.fg(|t| t.agents_text))
            } else {
                item.flex_shrink(1.0)
                    .min_w(ui.px(56.0))
                    .pr(ui.px(11.0))
                    .text_color(self.fg(|t| t.muted))
                    .hover(move |style| style.bg(hovered))
            };
            tabs = tabs.child(item.child(close).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| this.select_tab(index, window, cx),
            )));
        }
        let new_tab = div()
            .id("new-tab")
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .size(ui.px(28.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(highlight))
            .on_mouse_down(MouseButton::Left, keep)
            .child(footer_icon::icon(Icon::Plus, self.fg(|t| t.muted), scale))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.popup = Some(Popup::NewTab);
                cx.notify();
            }));
        div()
            .id("title-bar")
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(px(title_bar_height(&ui, self.pet.is_some())))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, _| {
                    this.dragging = event.click_count < 2;
                    if event.click_count == 2 {
                        window.titlebar_double_click();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, _| {
                if this.dragging && event.pressed_button == Some(MouseButton::Left) {
                    this.dragging = false;
                    window.start_window_move();
                }
            }))
            // Over the sidebar: the traffic lights, and room to drag by.
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(self.sidebar_width.max(LIGHTS)))
                    .h_full(),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .pl(px(10.0))
                    .pr(px(10.0))
                    .child(tabs)
                    .child(new_tab)
                    // Spare room, where the pet walks along the terminal's top edge.
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .ml(px(GAP))
                            .children(self.pet.clone()),
                    ),
            )
    }

    /// `shown` is how many panes are on screen, for dimming all but the active one.
    fn node(
        &self,
        node: &Node,
        shown: usize,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Node::Pane(pane) => self.pane(*pane, shown, agents, cx),
            Node::Split {
                axis,
                first,
                second,
            } => {
                // The seam between the panes: the split's own colour, one pixel through the gap.
                let split = div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .gap(px(1.0))
                    .bg(self.fg(|t| t.agents_rule));
                let split = match axis {
                    Axis::Row => split.flex_row(),
                    Axis::Column => split.flex_col(),
                };
                split
                    .child(self.node(first, shown, agents, cx))
                    .child(self.node(second, shown, agents, cx))
                    .into_any_element()
            }
        }
    }

    fn pane(
        &self,
        pane: PaneId,
        shown: usize,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active = pane == self.workspace.active_pane();
        let header = header_shown(self.workspace.tab().panes().len());
        let background = hsla(self.theme.terminal().background, 1.0);
        // Around the terminal: roomier alone, tighter under a header (the view adds 6 itself).
        let (top, side) = if header { (6.0, 10.0) } else { (8.0, 12.0) };
        div()
            .id(("pane", pane as usize))
            .group(SharedString::from(format!("pane-{pane}")))
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .overflow_hidden()
            .bg(background)
            .capture_any_mouse_down(
                cx.listener(move |this, _, window, cx| this.focus_pane(pane, window, cx)),
            )
            .on_drop::<ExternalPaths>(cx.listener(move |this, paths, window, cx| {
                this.focus_pane(pane, window, cx);
                this.panes[&pane].update(cx, |view, cx| view.drop_files(paths, window, cx));
            }))
            .when(header, |this| {
                this.child(self.pane_header(pane, active, agents, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .pt(px(top))
                    .pb(px(top))
                    .px(px(side))
                    .child(self.panes[&pane].clone()),
            )
            // A veil rather than a frame: it takes no clicks, so they reach the terminal.
            .when(dimmed(shown, active), |this| {
                this.child(div().absolute().inset_0().bg(background.opacity(DIM)))
            })
            .into_any_element()
    }

    /// A split pane's slim header: status dot, name, short directory, and icons to split, zoom
    /// and close, always there on the active pane and on hover on the others.
    fn pane_header(
        &self,
        pane: PaneId,
        active: bool,
        agents: &[Agent],
        cx: &mut Context<Self>,
    ) -> Div {
        let ui = UiFont::get(cx);
        let scale = ui.scale(1.0);
        let highlight = self.highlight();
        let agent_cwd = match self.workspace.shown(pane) {
            Shown::Agent(name) => agents
                .iter()
                .find(|a| &a.name == name)
                .and_then(|a| a.cwd.clone()),
            _ => None,
        };
        let home = std::env::var("HOME").ok();
        let (name, dir) = pane_name(
            &self.subject(pane, cx),
            agent_cwd.as_deref(),
            home.as_deref(),
        );
        let now = now();
        let button = |id: &str, icon: Icon| {
            let group = SharedString::from(format!("{id}-{pane}"));
            div()
                .id(SharedString::from(format!("{id}-{pane}")))
                .group(group.clone())
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(28.0))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(ui.px(22.0))
                        .rounded(px(6.0))
                        .group_hover(group, move |style| style.bg(highlight))
                        .child(footer_icon::icon(icon, self.fg(|t| t.muted), scale)),
                )
        };
        let zoomed = self.workspace.zoomed().is_some();
        let buttons = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .when(!active, |buttons| {
                buttons
                    .invisible()
                    .group_hover(format!("pane-{pane}"), |style| style.visible())
            })
            .child(button("split", Icon::Split).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.popup = Some(Popup::Split(None));
                    cx.notify();
                },
            )))
            .child(
                button("zoom", if zoomed { Icon::Restore } else { Icon::Zoom }).on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_zoom(window, cx)),
                ),
            )
            .child(button("close-pane", Icon::Close).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    this.request_close(Closing::Pane(pane), window, cx)
                },
            )));
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .h(ui.px(28.0))
            .pl(ui.px(14.0))
            .pr(ui.px(4.0))
            .text_size(ui.px(12.0))
            .border_b_1()
            .border_color(hsla(self.theme.fg(|t| t.agents_rule), 0.6))
            .child(
                div()
                    .flex_shrink_0()
                    .size(ui.px(7.0))
                    .rounded_full()
                    .bg(self.dot(pane, agents, now)),
            )
            .child(
                div()
                    .flex_shrink(1.0)
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
                    .child(name),
            )
            .children(dir.map(|dir| {
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(self.fg(|t| t.agents_dim))
                    .child(format!("· {dir}"))
            }))
            .child(div().flex_1())
            .child(buttons)
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
        let ui = UiFont::get(cx);
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
                .text_size(ui.px(TEXT - 1.0))
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
            // Under the sidebar's header, below the title bar.
            .top(px(
                title_bar_height(&ui, self.pet.is_some()) + ui.scale(44.0)
            ))
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
            .text_size(ui.px(TEXT + 1.0))
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
                            .text_size(ui.px(TEXT - 1.0))
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
        let ui = UiFont::get(cx);
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
                        .text_size(ui.px(TEXT - 1.0))
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
            .text_size(ui.px(TEXT + 1.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(ui.px(30.0))
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
            .pt(px(title_bar_height(&ui, self.pet.is_some()) + 30.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
            )
            .child(card)
    }

    fn dialog(&self, popup: Popup, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = UiFont::get(cx);
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
                            .text_size(ui.px(TEXT - 1.0))
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
            .text_size(ui.px(TEXT + 1.0))
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

/// Seconds since the epoch, as the agent statuses count them.
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// The sidebar's technical lines use the terminal's font.
fn mono_font(options: &Options) -> gpui::Font {
    let mut mono = gpui::font(options.font_family.clone());
    if !options.fallbacks.is_empty() {
        mono.fallbacks = Some(gpui::FontFallbacks::from_fonts(options.fallbacks.clone()));
    }
    mono
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
        let ui = UiFont::get(cx);
        // Larger interface sizes make the title bar taller; the traffic lights stay centred on it.
        let height = title_bar_height(&ui, self.pet.is_some());
        if self.lights != Some(height) {
            self.lights = Some(height);
            window.set_traffic_light_position(traffic_lights(height));
        }
        // A zoomed pane fills the tab; the split waits underneath.
        let (root, shown) = match self.workspace.zoomed() {
            Some(pane) => (Node::Pane(pane), 1),
            None => {
                let tab = self.workspace.tab();
                (tab.root.clone(), tab.panes().len())
            }
        };
        let agents = self.sidebar.read(cx).agents();
        let now = now();
        let content = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .bg(hsla(self.theme.terminal().background, 1.0))
            .child(self.node(&root, shown, &agents, cx));
        let body = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_row()
            .child(self.sidebar.clone())
            .child(content);
        let dialog = self.popup.map(|popup| match popup {
            Popup::Attention => self.attention_panel(cx),
            Popup::Search => self.search_panel(cx),
            popup => self.dialog(popup, cx),
        });
        ui.apply(div())
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // GPUI's default size, scaled, for text nothing else sizes.
            .text_size(ui.px(16.0))
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
                cx.listener(|this, _: &menu::ZoomPane, window, cx| this.toggle_zoom(window, cx)),
            )
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
            .child(self.title_bar(&agents, now, cx))
            .child(body)
            .children(dialog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(program: &str, cwd: &str) -> Subject {
        Subject::Shell {
            program: program.into(),
            cwd: cwd.into(),
        }
    }

    #[test]
    fn tabs_name_agents_short_unless_that_reads_the_same_as_another() {
        let agent = Subject::Agent("paddock/main".into());
        let open = vec!["paddock/main".to_owned(), "paddock/dev-fonts".to_owned()];
        assert_eq!(tab_title(&agent, &open), "main");
        let clash = vec!["paddock/main".to_owned(), "saddle/main".to_owned()];
        assert_eq!(tab_title(&agent, &clash), "paddock/main");
        // Not open anywhere else, or without a group.
        assert_eq!(tab_title(&agent, &[]), "main");
        assert_eq!(tab_title(&Subject::Agent("solo".into()), &[]), "solo");
    }

    #[test]
    fn tabs_name_shells_by_program_and_directory() {
        let open = [];
        assert_eq!(
            tab_title(&shell("/bin/zsh", "/Users/me/paddock"), &open),
            "zsh · paddock"
        );
        assert_eq!(tab_title(&shell("/bin/zsh", "/tmp/x/"), &open), "zsh · x");
        assert_eq!(tab_title(&shell("fish", "/"), &open), "fish · /");
        assert_eq!(
            tab_title(&Subject::Command("htop -d 5".into()), &open),
            "htop -d 5"
        );
        assert_eq!(tab_title(&Subject::Empty, &open), "empty");
    }

    #[test]
    fn a_split_tab_counts_its_other_panes() {
        assert_eq!(more_panes(1), None);
        assert_eq!(more_panes(2).as_deref(), Some("+1"));
        assert_eq!(more_panes(4).as_deref(), Some("+3"));
    }

    #[test]
    fn directories_shorten_to_their_last_part() {
        let home = Some("/Users/me");
        assert_eq!(short_dir("/Users/me", home), "~");
        assert_eq!(short_dir("/Users/me/", home), "~");
        assert_eq!(short_dir("/Users/me/code", home), "~/code");
        assert_eq!(short_dir("/Users/me/code/paddock", home), "~/…/paddock");
        assert_eq!(short_dir("/Users/meow/x", home), "/…/x");
        assert_eq!(short_dir("/tmp", home), "/tmp");
        assert_eq!(short_dir("/", home), "/");
        assert_eq!(short_dir("/Users/me/code", None), "/…/code");
    }

    #[test]
    fn pane_headers_name_in_full_with_the_short_directory() {
        let home = Some("/Users/me");
        let agent = Subject::Agent("paddock/main".into());
        assert_eq!(
            pane_name(&agent, Some("/Users/me/code/paddock"), home),
            ("paddock/main".into(), Some("~/…/paddock".into()))
        );
        assert_eq!(pane_name(&agent, None, home), ("paddock/main".into(), None));
        assert_eq!(
            pane_name(&shell("/bin/zsh", "/Users/me/code"), None, home),
            ("zsh".into(), Some("~/code".into()))
        );
        assert_eq!(
            pane_name(&Subject::Empty, None, home),
            ("empty".into(), None)
        );
    }

    #[test]
    fn only_split_tabs_have_pane_headers() {
        assert!(!header_shown(1));
        assert!(header_shown(2));
        assert!(header_shown(3));
    }

    #[test]
    fn panes_other_than_the_active_one_dim_when_several_show() {
        assert!(!dimmed(1, true));
        // A zoomed pane shows alone: nothing to tell it from.
        assert!(!dimmed(1, false));
        assert!(!dimmed(2, true));
        assert!(dimmed(2, false));
        assert!(dimmed(4, false));
    }

    #[test]
    fn traffic_lights_centre_on_the_title_bar() {
        assert_eq!(traffic_lights(40.0), point(px(14.0), px(13.0)));
        assert_eq!(traffic_lights(54.0), point(px(14.0), px(20.0)));
        assert_eq!(title_bar_height(&UiFont::default(), false), TITLE_BAR);
        // The pet walks at twice its pixel size on the terminal's top edge: the bar makes room.
        assert_eq!(title_bar_height(&UiFont::default(), true), PET_TITLE_BAR);
        let large = UiFont {
            family: None,
            size: 18.0,
        };
        assert!(title_bar_height(&large, false) > TITLE_BAR);
        assert!(title_bar_height(&large, true) > PET_TITLE_BAR);
        let small = UiFont {
            family: None,
            size: 11.0,
        };
        assert_eq!(title_bar_height(&small, false), TITLE_BAR);
        assert_eq!(title_bar_height(&small, true), PET_TITLE_BAR);
    }

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
