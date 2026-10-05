//! The window's root view: the Agents sidebar on the left; on the right a tab strip and the active
//! tab's panes, each a terminal in a frame with its title and controls. `layout.rs` holds the
//! rules; this file draws them and keeps one terminal view per pane.
use crate::{
    config::Config,
    corral::Role,
    layout::{Axis, Direction, Node, PaneId, Placement, Shown, Workspace},
    pet::PetView,
    sidebar::{Sidebar, SidebarEvent},
    theme::Theme,
    view::{Launch, Options, TerminalView, hsla},
    viewer::AgentMetadata,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, Entity, FocusHandle, Focusable, FontWeight,
    Hsla, MouseButton, Render, ScrollHandle, Stateful, Window, div, prelude::*, px,
};
use std::{collections::HashMap, rc::Rc};

/// What "New shell" and the `Shell` choice start.
pub struct NewShell {
    pub program: String,
    pub cwd: String,
}

/// The small dialog behind `+` and `Split ▾`.
#[derive(Clone, Copy, PartialEq)]
enum Popup {
    /// Choose what the new tab shows.
    NewTab,
    /// First the direction, then what the new pane shows.
    Split(Option<Direction>),
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
            cx.new(|cx| Sidebar::new(theme, width, mono, corral, cx))
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
            SidebarEvent::NewShell => {
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
            SidebarEvent::Alive(names) => {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                for view in self.panes.values() {
                    view.update(cx, |v, _| v.disappeared(&names));
                }
            }
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
            Popup::Split(None) => return,
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
        let gone = self.workspace.close_pane(self.workspace.active_pane());
        self.close(gone, window, cx);
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let gone = self.workspace.close_tab(index);
        self.close(gone, window, cx);
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
        let dialog = self.popup.map(|popup| self.dialog(popup, cx));
        div()
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .child(self.sidebar.clone())
            .child(main)
            .children(dialog)
    }
}
