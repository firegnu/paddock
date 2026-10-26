//! Workspaces in the sidebar (DESIGN §13 P5-89): the tray a workspace's groups lie in, with its
//! title; the three menus (a group's button, a card's `Workspace` cell, a workspace's `⋯`); the
//! panel that names a new workspace and picks its repositories, or picks those of one there is;
//! and the title as a field while it is renamed. Each moves whole repositories (P5-105). Who is in
//! which workspace is `workspaces.rs`'s, and the list's lines are `card.rs`'s.
use super::*;
use crate::{
    activity, kanban_repos, menu, popover, tab_stack,
    text_input::{self, Changed, TextInput},
    workspaces::{self, Picks, Workspaces},
};
use gpui::{Anchor, Focusable, PathPromptOptions, Subscription, anchored, deferred};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

// Sizes, in points at the base interface size.
/// The tray: its corners, the room it keeps at its sides and foot, and above its title.
const TRAY_RADIUS: f32 = 14.0;
pub(super) const TRAY_X: f32 = 6.0;
const TRAY_TOP: f32 = 2.0;
/// The title's height in an open tray, and a folded tray's.
const TITLE: f32 = 30.0;
const FOLDED: f32 = 34.0;
/// Open, the tray is a well sunk in the column (P5-91): how much black darkens its face on a dark
/// sidebar and on a light one, and its edge as a share of the text colour; lit while the pointer
/// is over it or its menu is open.
const WELL: [f32; 2] = [0.26, 0.07];
const WELL_LIT: [f32; 2] = [0.21, 0.055];
const WELL_EDGE: f32 = 0.17;
const WELL_EDGE_LIT: f32 = 0.22;
/// Folded, it is a pile (P5-91): the top card's corners; how much of its foot each card under it
/// shows, how far in it stands at each side, and how high it is.
const PILE_RADIUS: f32 = 12.0;
const PILE_STEP: f32 = 5.0;
const PILE_INSET: f32 = 8.0;
const PILE_UNDER: f32 = 18.0;
/// The pile's cards from the top, their faces and edges as shares of the text colour over the
/// column's; the top one's lit.
const PILE_FACE: [f32; 3] = [0.065, 0.035, 0.015];
const PILE_EDGE: [f32; 3] = [0.18, 0.13, 0.10];
const PILE_FACE_LIT: f32 = 0.085;
const PILE_EDGE_LIT: f32 = 0.24;
const NAME: f32 = 13.0;
const COUNT: f32 = 11.0;
/// The idle shelf at the list's foot (P5-164): its face over the column, darker than an open
/// workspace's, and its edge, both a step fainter; the height of a line in it.
const SHELF: [f32; 2] = [0.14, 0.04];
const SHELF_EDGE: f32 = 0.09;
/// A line of the shelf is this high, its name at the size of a repository's heading.
const IDLE_ROW: f32 = 26.0;
/// The line of a workspace's repositories with no agent, before the pointer is on it (P5-134).
const VACANCIES: f32 = 0.75;
/// The menus, and the panel.
const GROUP_MENU: f32 = 232.0;
const AGENT_MENU: f32 = 216.0;
const WORKSPACE_MENU: f32 = 236.0;
const PANEL: f32 = 320.0;
const PANEL_PAD: f32 = 14.0;
/// A row of the panel's list.
const ROW: f32 = 32.0;
const BOX: f32 = 13.0;
/// The window's height the panel leaves to everything but its list.
const PANEL_REST: f32 = 340.0;
/// How long the folder a repository is in may run, beside its name when another has that name.
const TWIN_PATH: usize = 24;

/// A workspace: sheets lying on one another.
const LAYERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M8 2L14 5 8 8 2 5ZM2 8L8 11 14 8M2 11L8 14 14 11" fill="none" stroke="#000" stroke-width="1.4" stroke-linejoin="round"/></svg>"##;
/// A folded workspace with no agent: the sheets, the top one filled.
const LAYERS_FILLED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M8 2L14 5 8 8 2 5Z" stroke="#000" stroke-width="1.4" stroke-linejoin="round"/><path d="M2 8L8 11 14 8M2 11L8 14 14 11" fill="none" stroke="#000" stroke-width="1.4" stroke-linejoin="round"/></svg>"##;
/// Into a workspace: the sheets and a plus.
const LAYERS_PLUS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M7.5 1.5L13.5 4.5 7.5 7.5 1.5 4.5ZM1.5 8L7.5 11M1.5 11.25L7.5 14.25M12.5 9.25v5.5M9.75 12h5.5" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const DOTS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="3.5" cy="8" r="1.3"/><circle cx="8" cy="8" r="1.3"/><circle cx="12.5" cy="8" r="1.3"/></svg>"##;
const PENCIL: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M3 13h2.5L13 5.5 10.5 3 3 10.5z" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// What floats over the list, one at a time.
pub(super) enum Floating {
    /// A group's button: the workspaces to add it or move it to.
    Group(GroupMenu),
    /// A card's `Workspace` cell: the workspaces for its agent's repository, by the agent's name.
    Agent(String),
    /// A workspace's `⋯`; `removing` once `Remove workspace` was clicked, for the click that
    /// confirms it.
    Workspace { name: String, removing: bool },
    /// A workspace's name as a field.
    Rename {
        name: String,
        input: Entity<TextInput>,
        _changed: Subscription,
    },
    /// `New workspace`, or `Add to <name>`.
    Editor(Editor),
    /// Another machine's capsule in the switcher, right-clicked (P5-123), by the machine's name.
    Machine(String),
    /// A sandboxed card's Sandbox cell: what fence's wall stopped (P5-120), by the agent's name.
    Denials(String),
    /// A controller card's Delegates cell: the sandbox for the agents it starts (P5-141), by
    /// the controller's name and the run it was opened for; out of date once that run is gone.
    Delegates {
        name: String,
        instance: Option<String>,
    },
}

impl Floating {
    /// Which button a menu hangs from, to tell a press on it from one elsewhere.
    fn key(&self) -> String {
        match self {
            Floating::Group(menu) => group_key(menu.workspace.as_deref(), &menu.group),
            Floating::Agent(name) => agent_key(name),
            Floating::Workspace { name, .. } => workspace_key(name),
            Floating::Machine(name) => format!("machine {name}"),
            Floating::Denials(name) => format!("denials {name}"),
            Floating::Delegates { name, .. } => format!("delegates {name}"),
            Floating::Rename { .. } | Floating::Editor(_) => String::new(),
        }
    }
}

fn group_key(workspace: Option<&str>, group: &str) -> String {
    format!("group {} {group}", workspace.unwrap_or_default())
}

fn agent_key(name: &str) -> String {
    format!("agent {name}")
}

fn workspace_key(name: &str) -> String {
    format!("workspace {name}")
}

/// A group's menu: what it would move, as its heading has it.
#[derive(Clone)]
pub(super) struct GroupMenu {
    workspace: Option<String>,
    group: String,
    /// The name as the heading shows it.
    title: String,
    repos: Vec<PathBuf>,
}

/// The panel that picks a workspace's repositories.
pub(super) struct Editor {
    /// The workspace whose repositories are picked; `None` for a new one.
    workspace: Option<String>,
    /// The new one's name.
    name: Option<Entity<TextInput>>,
    picks: Picks,
    /// Folders chosen, by their repositories: listed, ticked or not, with those agents work in and
    /// the workspace's own.
    extra: Vec<PathBuf>,
    /// Why the folder chosen was not added.
    note: Option<SharedString>,
    _changed: Option<Subscription>,
}

/// A press on a button that opens a menu.
type OnPress = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// A card's `Workspace` cell: where its agent is, the press that opens its menu, and the menu
/// while it is open. No press for an agent that goes by its name's prefix: the cell only says.
pub(super) struct WorkspaceCell {
    pub value: String,
    pub open: bool,
    pub on_press: Option<OnPress>,
    pub menu: Option<AnyElement>,
}

fn input_colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
        faded: theme.is_faded(),
    }
}

/// An icon drawn as a mask, `size` points square.
fn mask(data: &'static [u8], size: Pixels, color: Hsla) -> gpui::Svg {
    svg()
        .data(data)
        .flex_shrink_0()
        .size(size)
        .text_color(color)
}

/// What a workspace holds, faint after its name in a menu, or on its line among the idle ones.
fn holds(repos: usize) -> String {
    match repos {
        0 => "Empty".to_owned(),
        1 => "1 repo".to_owned(),
        repos => format!("{repos} repos"),
    }
}

impl Sidebar {
    // What is kept.

    /// Reads the workspaces again: another window may have changed them.
    pub(super) fn reload_workspaces(&mut self) {
        if let Some(path) = &self.workspaces_path {
            self.listing.panel.workspaces = Workspaces::load(path);
            self.follow_editor();
        }
    }

    /// Changes the workspaces, as they are kept now, and keeps them as [`Panel::edit_workspaces`]
    /// says: when that changed anything, or when an older paddock's names gave way with none
    /// still waiting for Git (P5-105).
    fn edit_workspaces(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Workspaces)) {
        let path = self.workspaces_path.clone();
        if let Err(error) = self.listing.panel.edit_workspaces(path.as_deref(), change) {
            self.note(format!("Workspaces not saved: {error}"), true, cx);
        }
        self.follow_editor();
        cx.notify();
    }

    /// The names an older paddock recorded give way to the repositories their running agents
    /// work in (P5-105), kept once Git has told of every one of them.
    pub(super) fn adopt_named(&mut self, cx: &mut Context<Self>) {
        if self.listing.panel.workspaces.named() {
            self.edit_workspaces(cx, |_| {});
        }
    }

    /// The open panel of a workspace's repositories follows it: those it gained meanwhile come
    /// ticked, so `Done` does not take them out (P5-105).
    fn follow_editor(&mut self) {
        let Some(Floating::Editor(editor)) = &mut self.floating else {
            return;
        };
        let workspaces = &self.listing.panel.workspaces;
        if let Some(workspace) = editor.workspace.as_deref().and_then(|w| workspaces.get(w)) {
            editor.picks.follow(workspace);
        }
    }

    /// `workspace`'s `New Agent…` (P5-93): the panel, starting in `repo`, else in its first
    /// repository, else where it would anyway; the repository of the agent it starts joins the
    /// workspace when in none (P5-105).
    fn new_agent_in(
        &mut self,
        workspace: &str,
        repo: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_floating(window, cx);
        self.reload_workspaces();
        let start_in = repo.or_else(|| {
            let workspace = self.listing.panel.workspaces.get(workspace)?;
            workspace.first_repo().map(Path::to_path_buf)
        });
        cx.emit(SidebarEvent::NewAgentIn {
            workspace: workspace.to_owned(),
            start_in: start_in.map(|repo| repo.display().to_string()),
        });
    }

    /// An agent was started in `cwd` from `workspace`'s `New Agent…` (P5-93): once Git, off the UI
    /// thread, has said which repository that is, the whole repository joins the workspace when
    /// it is in none (P5-105).
    pub fn join_started(&mut self, workspace: &str, cwd: &str, cx: &mut Context<Self>) {
        let (workspace, dir) = (workspace.to_owned(), cwd.to_owned());
        cx.spawn(async move |this, cx| {
            let repo = cx
                .background_executor()
                .spawn(
                    async move { activity::main_repository("git", &dir, &AtomicBool::new(false)) },
                )
                .await;
            let _ = this.update(cx, |sidebar, cx| {
                sidebar.edit_workspaces(cx, |workspaces| {
                    workspaces.join(&workspace, repo.as_deref());
                });
            });
        })
        .detach();
    }

    // What floats.

    /// Opens `floating` in place of any other. It takes the keys, for Esc and Return, and gives
    /// them back as it closes.
    pub(super) fn float(
        &mut self,
        floating: Floating,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.floating.is_none() {
            self.floating_back = window.focused(cx);
        }
        self.reload_workspaces();
        let focus = match &floating {
            Floating::Rename { input, .. } => input.read(cx).focus_handle(cx),
            Floating::Editor(Editor {
                name: Some(name), ..
            }) => name.read(cx).focus_handle(cx),
            _ => self.floating_focus.clone(),
        };
        window.focus(&focus, cx);
        self.floating = Some(floating);
        cx.notify();
    }

    pub(super) fn close_floating(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.floating.take().is_none() {
            return;
        }
        let back = self.floating_back.take();
        if self.floating_focus.contains_focused(window, cx) {
            match back {
                Some(back) => window.focus(&back, cx),
                None => window.blur(cx),
            }
        }
        cx.notify();
    }

    /// The keys went elsewhere: what floats goes. A panel being filled in stays while the window
    /// is at the back, as it is behind the system's folder chooser.
    pub(super) fn floating_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let stays =
            matches!(self.floating, Some(Floating::Editor(_))) && !window.is_window_active();
        if !stays && self.floating.take().is_some() {
            self.floating_back = None;
            cx.notify();
        }
    }

    /// A press outside what floats closes it; one on a menu's own button is remembered, so the
    /// button does not open it again.
    fn press_out(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(floating) = &self.floating {
            self.dismissed = Some((event.position, floating.key()));
        }
        self.close_floating(window, cx);
    }

    /// A press on a menu's button: opens it, unless this press has just closed it.
    pub(super) fn press_menu(
        &mut self,
        floating: Floating,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dismissed.take() != Some((event.position, floating.key())) {
            self.float(floating, window, cx);
        }
    }

    /// The root of something floating: it has the keys, Esc closes it and so does a press
    /// outside, and nothing under it feels the pointer.
    pub(super) fn floated(
        &self,
        panel: Div,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        panel
            .id(id)
            .track_focus(&self.floating_focus)
            .key_context(menu::DIALOG)
            .occlude()
            .on_mouse_down_out(cx.listener(Self::press_out))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_floating(window, cx)),
            )
    }

    /// `menu` hung under the right end of the row it is added to, or (`left`) under the left
    /// end, inside the window.
    pub(super) fn hung(menu: impl IntoElement, left: bool) -> Div {
        let hung = anchored().snap_to_window_with_margin(px(8.0));
        let hung = if left {
            hung
        } else {
            hung.anchor(Anchor::TopRight)
        };
        div()
            .absolute()
            .top(relative(1.0))
            .map(|spot| if left { spot.left_0() } else { spot.right_0() })
            .child(deferred(hung.child(menu)))
    }

    // The tray.

    /// The mark led by the cards of the agents `stack` names, front first (P5-91): as a split
    /// tab's mark, each the kind's icon in its colour, no status dot. A folded workspace's title
    /// wears it, and a folded group's heading (P5-128).
    pub(super) fn stack_mark(&self, stack: &[String], ui: &UiFont) -> AnyElement {
        let agents = &self.listing.panel.agents;
        let faces: Vec<_> = stack
            .iter()
            .map(|name| {
                let kind = agents
                    .iter()
                    .find(|a| a.name == *name)
                    .and_then(|a| a.kind.as_deref())
                    .unwrap_or_default();
                tab_stack::Face::of_kind(&self.theme, kind)
            })
            .collect();
        let inks = tab_stack::Inks::of(&self.theme);
        tab_stack::stack(&faces, inks, ui.scale(1.0)).into_any_element()
    }

    /// A workspace: its title and under it `inside`, its groups' lines, in a tray sunk in the
    /// column; folded, the title alone with the bar of all its agents, on the top card of a pile
    /// and led by its agents' cards stacked (P5-91). A click on the title folds it or opens it.
    /// `first` when nothing is above it in the list.
    pub(super) fn tray(
        &self,
        tray: card::Tray,
        inside: Vec<AnyElement>,
        first: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let (text, dim, dimmer, accent) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_accent),
        );
        let card::Tray {
            name,
            count,
            bar,
            folded,
            showing,
            stack,
            opened,
        } = tray;
        let hover: SharedString = format!("workspace-{name}").into();
        let menu_open = self.workspace_menu_open(&name);
        let renaming = self.renaming(&name);
        // Opened with no agent, it goes back to the list's foot rather than folding (P5-134).
        let fold = {
            let name = name.clone();
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                if opened {
                    this.lower_workspace(&name, cx);
                } else {
                    this.fold_workspace(&name, cx);
                }
            })
        };
        let label = match renaming {
            Some(input) => self.rename_field(&name, input, &ui, cx).into_any_element(),
            None => div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(ui.px(NAME))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .child(name.clone())
                .into_any_element(),
        };
        // Folded, its agents' cards stacked as a split tab's, with no status dot (the bar says);
        // with none, the sheets with the top one filled. Open, the sheets.
        let mark = if !folded {
            mask(LAYERS, ui.px(14.0), accent).into_any_element()
        } else if stack.is_empty() {
            mask(LAYERS_FILLED, ui.px(14.0), accent).into_any_element()
        } else {
            self.stack_mark(&stack, &ui)
        };
        // Open, the arrow a step brighter, as the well is a step darker.
        let chevron = if folded {
            dim
        } else {
            dim.blend(text.opacity(0.5))
        };
        let title = div()
            .id(ElementId::Name(hover.clone()))
            .group(hover.clone())
            .relative()
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .h(ui.px(if folded { FOLDED } else { TITLE }))
            .px(px(2.0))
            .cursor_pointer()
            .on_click(fold)
            .child(mask(
                if folded { CHEVRON_RIGHT } else { CHEVRON_DOWN },
                ui.px(10.0),
                chevron,
            ))
            .child(mark)
            .child(label)
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(COUNT))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child(count.to_string()),
            )
            // Hollow, as on a folded group: the agent the active pane shows is folded away.
            .when(folded && showing, |title| {
                title.child(
                    div()
                        .flex_shrink_0()
                        .size(ui.px(HERE))
                        .rounded_full()
                        .border(ui.px(HERE_RING))
                        .border_color(accent),
                )
            })
            .child(div().flex_1())
            .child(self.options_button(&name, &hover, menu_open, &ui, false, cx))
            // Folded, how all its agents are doing; open, its groups' bars say.
            .when(folded, |title| title.child(bar_of(&self.theme, &ui, bar)))
            .when(menu_open, |title| {
                title.child(Self::hung(self.workspace_menu(&name, &ui, cx), false))
            });
        // A little off what is above it; closer to the top for the first, as a heading is.
        let above = px(if first { 8.0 } else { 4.0 });
        if folded {
            return self.pile(title, stack.len(), menu_open, above);
        }
        let base = self.grounds().base;
        let (face, edge) = if menu_open {
            (well(&self.theme, base, true), WELL_EDGE_LIT)
        } else {
            (well(&self.theme, base, false), WELL_EDGE)
        };
        let lit = well(&self.theme, base, true);
        div()
            .flex()
            .flex_col()
            .gap(px(CARD_GAP))
            .mt(above)
            .px(px(TRAY_X))
            .pt(px(TRAY_TOP))
            .pb(px(TRAY_X))
            .rounded(px(TRAY_RADIUS))
            .border_1()
            .border_color(text.opacity(edge))
            .bg(face)
            .hover(move |tray| tray.bg(lit).border_color(text.opacity(WELL_EDGE_LIT)))
            .child(title)
            .children(inside)
            .into_any_element()
    }

    /// A folded workspace's `title` on the top card of a pile, whole colours that hide what is
    /// under them; under it the feet of one card more for each of its agents after the first,
    /// two at most, each further in and nearer the column's colour. The feet are in its height.
    fn pile(
        &self,
        title: gpui::Stateful<Div>,
        agents: usize,
        lit: bool,
        above: Pixels,
    ) -> AnyElement {
        let text = hsla(self.theme.fg(|t| t.agents_text), 1.0);
        let column = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        let face = |share: f32| over(text.opacity(share), column);
        let depth = agents.saturating_sub(1).min(PILE_FACE.len() - 1);
        let (lit_face, lit_edge) = (face(PILE_FACE_LIT), text.opacity(PILE_EDGE_LIT));
        let (top_face, top_edge) = if lit {
            (lit_face, lit_edge)
        } else {
            (face(PILE_FACE[0]), text.opacity(PILE_EDGE[0]))
        };
        // From the bottom card up, so each lies over the one under it.
        let under = (1..=depth).rev().map(|card| {
            let inset = px(PILE_INSET * card as f32);
            div()
                .absolute()
                .left(inset)
                .right(inset)
                .bottom(px(PILE_STEP * (depth - card) as f32))
                .h(px(PILE_UNDER))
                .rounded_b(px(PILE_RADIUS))
                .border_1()
                .border_color(text.opacity(PILE_EDGE[card]))
                .bg(face(PILE_FACE[card]))
        });
        let top = div()
            .relative()
            .px(px(TRAY_X))
            .rounded(px(PILE_RADIUS))
            .border_1()
            .border_color(top_edge)
            .bg(top_face)
            .hover(move |card| card.bg(lit_face).border_color(lit_edge))
            .child(title);
        div()
            .relative()
            .mt(above)
            .pb(px(PILE_STEP * depth as f32))
            .children(under)
            .child(top)
            .into_any_element()
    }

    /// The workspace's `⋯` menu is open.
    fn workspace_menu_open(&self, name: &str) -> bool {
        matches!(
            &self.floating,
            Some(Floating::Workspace { name: open, .. }) if open == name
        )
    }

    /// The field the workspace's name is in while it is renamed.
    fn renaming(&self, name: &str) -> Option<Entity<TextInput>> {
        match &self.floating {
            Some(Floating::Rename {
                name: renamed,
                input,
                ..
            }) if renamed == name => Some(input.clone()),
            _ => None,
        }
    }

    /// A workspace's `⋯` (`Workspace options`), shown while the pointer is over what `hover`
    /// names, or lit while its menu is open: on its title, and on its line among the idle ones.
    /// `compact` keeps no room for it: the line's repositories run to the edge until the pointer
    /// is over the line, which gives them the room back (P5-164).
    fn options_button(
        &self,
        name: &str,
        hover: &SharedString,
        menu_open: bool,
        ui: &UiFont,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let text = hsla(self.theme.fg(|t| t.agents_text), 1.0);
        let options = {
            let name = name.to_owned();
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                let menu = Floating::Workspace {
                    name: name.clone(),
                    removing: false,
                };
                this.press_menu(menu, event, window, cx);
            })
        };
        let tip = Tip {
            text: "Workspace options".into(),
            size: ui.px(NOTE_SIZE),
            color: text,
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: hsla(self.theme.fg(|t| t.agents_rule), 1.0),
        };
        let lit = text.opacity(0.12);
        let hover = hover.clone();
        let width = ui.px(22.0);
        div()
            .id("workspace-options")
            .flex_shrink_0()
            .h(width)
            .when(compact, |button| button.w(px(0.0)).overflow_hidden())
            .when(!compact, |button| button.w(width))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(lit))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .on_mouse_down(MouseButton::Left, options)
            .map(|button| {
                if menu_open {
                    button.w(width).bg(lit)
                } else if compact {
                    button.group_hover(hover, move |style| style.w(width))
                } else {
                    button
                        .opacity(0.0)
                        .group_hover(hover, |style| style.opacity(1.0))
                }
            })
            .child(mask(DOTS, ui.px(14.0), text))
    }

    // The workspaces with no agent (P5-134).

    /// A workspace with no agent opened from the list's foot: up, and open.
    fn raise_workspace(&mut self, name: &str, cx: &mut Context<Self>) {
        self.listing.panel.raise(name);
        self.folds_changed(cx);
    }

    /// An opened workspace with no agent back down to the list's foot, its folds as they were.
    fn lower_workspace(&mut self, name: &str, cx: &mut Context<Self>) {
        self.listing.panel.lower(name);
        cx.notify();
    }

    /// The shelf at the list's foot (P5-134), a tray of its own since P5-164: a faint tray whose
    /// title looks as a workspace's, its hollow sheets dim, `Idle` and how many, and under it
    /// `inside`, a line for each workspace with no agent. A click on the title folds it to the
    /// title alone, still in the tray.
    pub(super) fn idle_shelf(
        &self,
        count: usize,
        folded: bool,
        inside: Vec<AnyElement>,
        first: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let (text, dim, dimmer) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
        );
        let title = div()
            .id("idle-shelf")
            .flex()
            .items_center()
            .gap(ui.px(7.0))
            .h(ui.px(TITLE))
            .px(px(2.0))
            .rounded(px(7.0))
            .cursor_pointer()
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.fold_idle(cx)))
            .child(mask(
                if folded { CHEVRON_RIGHT } else { CHEVRON_DOWN },
                ui.px(10.0),
                dimmer,
            ))
            .child(mask(LAYERS, ui.px(14.0), dimmer))
            .child(
                div()
                    .text_size(ui.px(NAME))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dim)
                    .child("Idle"),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(COUNT))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child(count.to_string()),
            );
        // As far above as a workspace's tray is from what is above it; a little off the top for
        // the first, as a heading is.
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .mt(px(if first { 8.0 } else { 4.0 }))
            .px(px(TRAY_X))
            .pt(px(TRAY_TOP))
            .pb(px(TRAY_X))
            .rounded(px(TRAY_RADIUS))
            .border_1()
            .border_color(text.opacity(SHELF_EDGE))
            .bg(shelf(&self.theme, self.grounds().base))
            .child(title)
            .children(inside)
            .into_any_element()
    }

    /// A workspace with no agent on the shelf: its name as a repository's heading, at its left,
    /// and at the right its repositories by name, faint, or `Empty` (P5-164); lit under the
    /// pointer, its name and repositories each a step brighter and its `⋯` (the title's menu,
    /// `Rename` in this line) where the repositories give it room. It keeps no room for the `⋯`
    /// otherwise, so they run to the edge. A click brings the workspace up, open.
    pub(super) fn idle_workspace(
        &self,
        name: &str,
        repos: &[String],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let (text, dim, dimmer) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
        );
        let lit = text.opacity(0.06);
        let hover: SharedString = format!("idle-{name}").into();
        let menu_open = self.workspace_menu_open(name);
        let label = match self.renaming(name) {
            Some(input) => self.rename_field(name, input, &ui, cx).into_any_element(),
            None => div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(ui.px(GROUP_NAME_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(dim)
                .group_hover(hover.clone(), move |style| style.text_color(text))
                .child(name.to_owned())
                .into_any_element(),
        };
        let raise = {
            let name = name.to_owned();
            cx.listener(move |this, _: &ClickEvent, _, cx| this.raise_workspace(&name, cx))
        };
        let holds = if repos.is_empty() {
            "Empty".to_owned()
        } else {
            repos.join(", ")
        };
        let holds = div()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_right()
            .text_size(ui.px(COUNT))
            .text_color(dimmer)
            .group_hover(hover.clone(), move |style| style.text_color(dim))
            .child(holds);
        div()
            .id(ElementId::Name(hover.clone()))
            .group(hover.clone())
            .relative()
            .flex()
            .items_center()
            .h(ui.px(IDLE_ROW))
            .px(px(4.0))
            .rounded(px(7.0))
            .cursor_pointer()
            .hover(move |style| style.bg(lit))
            .when(menu_open, |row| row.bg(lit))
            .on_click(raise)
            // The gap to the repositories, which keeps none for the `⋯` before it is shown.
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .mr(ui.px(10.0))
                    .child(label),
            )
            .child(holds)
            .child(self.options_button(name, &hover, menu_open, &ui, true, cx))
            .when(menu_open, |row| {
                row.child(Self::hung(self.workspace_menu(name, &ui, cx), false))
            })
            .into_any_element()
    }

    /// The repositories of `workspace` with no agent, in one faint line after its groups:
    /// an arrow, their names, `No agents` (P5-134). Opened out, the line says `No agents` and
    /// how many instead, and its repositories follow a line each, their names alone (P5-164).
    /// A click opens them out, or puts them back.
    pub(super) fn vacancies(
        &self,
        workspace: &str,
        repos: &[String],
        open: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let dimmer = fg(|t| t.agents_dimmer);
        let lit = fg(|t| t.agents_text).opacity(0.06);
        let on_click = {
            let workspace = workspace.to_owned();
            cx.listener(move |this, _: &ClickEvent, _, cx| this.fold_vacant(&workspace, cx))
        };
        let row = div()
            .id(SharedString::from(format!("vacancies-{workspace}")))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .px(px(4.0))
            .py(px(2.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .opacity(VACANCIES)
            .hover(move |style| style.bg(lit).opacity(1.0))
            .on_click(on_click)
            .text_size(ui.px(GROUP_NAME_SIZE))
            .child(mask(
                if open { CHEVRON_DOWN } else { CHEVRON_RIGHT },
                ui.px(CHEVRON),
                dimmer,
            ));
        let row = if open {
            row.child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child("No agents"),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .text_size(ui.px(GROUP_COUNT_SIZE))
                    .text_color(dimmer.opacity(0.6))
                    .child(repos.len().to_string()),
            )
        } else {
            row.child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child(repos.join(", ")),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .text_size(ui.px(GROUP_COUNT_SIZE))
                    .text_color(dimmer.opacity(0.6))
                    .child("No agents"),
            )
        };
        row.into_any_element()
    }

    /// A whole repository of `workspace` with no agent now, or (`None`) a workspace with nothing
    /// to list: a faint line, and a way to start an agent there (P5-93): for the repository a `+`
    /// while the pointer is on the line, for the empty workspace a `New Agent` button. `grouped`,
    /// it is one of those under the workspace's opened composite line (P5-164): its name alone,
    /// set in under that line's arrow, without `No agents`.
    pub(super) fn vacant(
        &self,
        workspace: &str,
        repo: Option<String>,
        grouped: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let dimmer = fg(|t| t.agents_dimmer);
        let accent = fg(|t| t.agents_accent);
        let lit = accent.opacity(0.18);
        let hover: SharedString =
            format!("vacant-{workspace}-{}", repo.as_deref().unwrap_or_default()).into();
        let path = repo.as_ref().and_then(|repo| {
            let members = &self.listing.panel.workspaces.get(workspace)?.repos;
            members
                .iter()
                .find(|path| kanban_repos::name(path) == *repo)
                .cloned()
        });
        let on_click = {
            let workspace = workspace.to_owned();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.new_agent_in(&workspace, path.clone(), window, cx)
            })
        };
        let button = div()
            .id(SharedString::from(format!("{hover}-new-agent")))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .h(ui.px(20.0))
            .rounded(ui.px(5.0))
            .cursor_pointer()
            .hover(move |style| style.bg(lit))
            .on_click(on_click);
        let button = match &repo {
            Some(repo) => {
                let tip = Tip {
                    text: format!("New Agent in {repo}").into(),
                    size: ui.px(NOTE_SIZE),
                    color: fg(|t| t.agents_text),
                    background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
                    border: fg(|t| t.agents_rule),
                };
                button
                    .w(ui.px(20.0))
                    .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                    .opacity(0.0)
                    .group_hover(hover.clone(), |style| style.opacity(1.0))
                    .child(footer_icon::icon(
                        Icon::Plus,
                        accent,
                        ui.scale(12.0 / footer_icon::SIZE),
                    ))
            }
            None => button
                .gap(ui.px(4.0))
                .px(ui.px(6.0))
                .text_color(accent)
                .child(footer_icon::icon(
                    Icon::NewAgent,
                    accent,
                    ui.scale(12.0 / footer_icon::SIZE),
                ))
                .child("New Agent"),
        };
        div()
            .group(hover)
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .when(grouped, |line| {
                // In under the opened line's arrow: its left padding, the arrow, and the gap, as
                // the line sizes them.
                line.pl(px(4.0) + ui.px(CHEVRON + 6.0)).pr(px(4.0))
            })
            .when(!grouped, |line| line.px(px(4.0)))
            .py(px(2.0))
            .text_size(ui.px(GROUP_NAME_SIZE))
            .children(repo.map(|repo| {
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child(repo)
            }))
            .when(!grouped, |line| {
                line.child(
                    div()
                        .text_size(ui.px(GROUP_COUNT_SIZE))
                        .text_color(dimmer.opacity(0.6))
                        .child("No agents"),
                )
            })
            .child(div().flex_1())
            // No taller than the line's words: the button lies over it, not in it.
            .child(div().h(px(0.0)).flex().items_center().child(button))
            .into_any_element()
    }

    /// The title as a field: Return keeps the name when it will do, Esc leaves it as it was.
    fn rename_field(
        &self,
        name: &str,
        input: Entity<TextInput>,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let typed = input.read(cx).text().to_owned();
        let taken = self.listing.panel.workspaces.name(&typed, Some(name))
            == Err(workspaces::NameError::Taken);
        let edge = if taken {
            fg(|t| t.agents_red)
        } else {
            fg(|t| t.agents_accent).opacity(0.6)
        };
        let keep = {
            let name = name.to_owned();
            cx.listener(move |this, _: &menu::OpenSelected, window, cx| {
                this.rename(&name, window, cx)
            })
        };
        div()
            .id("workspace-rename")
            .track_focus(&self.floating_focus)
            .key_context(menu::DIALOG)
            .flex_1()
            .min_w(px(0.0))
            .h(ui.px(24.0))
            .px(ui.px(7.0))
            .flex()
            .items_center()
            .rounded(ui.px(6.0))
            .border_1()
            .border_color(edge)
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(NAME))
            .cursor_text()
            // A click in the field is not one on the title.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(Self::press_out))
            .on_action(
                cx.listener(|this, _: &menu::Cancel, window, cx| this.close_floating(window, cx)),
            )
            .on_action(keep)
            .child(div().flex_1().min_w(px(0.0)).child(input))
    }

    // What the menus do.

    fn start_rename(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let colors = input_colors(&self.theme);
        let input = cx.new(|cx| TextInput::new(name.to_owned(), "Name", colors, cx));
        let changed = cx.subscribe(&input, |_, _, _: &Changed, cx| cx.notify());
        let rename = Floating::Rename {
            name: name.to_owned(),
            input,
            _changed: changed,
        };
        self.float(rename, window, cx);
    }

    /// Return in the title's field: the new name, when it will do; its members and what is
    /// folded of it go with it.
    fn rename(&mut self, from: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Floating::Rename { input, .. }) = &self.floating else {
            return;
        };
        let typed = input.read(cx).text().to_owned();
        let mut kept = None;
        self.edit_workspaces(cx, |all| kept = all.rename(from, &typed).ok());
        let Some(name) = kept else {
            return;
        };
        self.listing.panel.rename_workspace(from, &name);
        self.folds_changed(cx);
        self.close_floating(window, cx);
    }

    /// `Remove workspace`, confirmed: the grouping goes, its members back to the list below.
    fn remove_workspace(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_workspaces(cx, |all| all.remove(name));
        self.listing.panel.forget_workspace(name);
        self.folds_changed(cx);
        self.close_floating(window, cx);
    }

    /// A group into `to`: its whole repositories.
    fn move_group(
        &mut self,
        menu: &GroupMenu,
        to: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_workspaces(cx, |all| {
            for repo in &menu.repos {
                all.add_repo(to, repo);
            }
        });
        self.close_floating(window, cx);
    }

    /// A group out of its workspace: its whole repositories.
    fn remove_group(&mut self, menu: &GroupMenu, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_workspaces(cx, |all| {
            for repo in &menu.repos {
                all.remove_repo(repo);
            }
        });
        self.close_floating(window, cx);
    }

    /// A card's agent's whole repository into `to`, or (`None`) into none (P5-105).
    fn place_repo(
        &mut self,
        repo: &Path,
        to: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_workspaces(cx, |all| match to {
            Some(to) => all.add_repo(to, repo),
            None => all.remove_repo(repo),
        });
        self.close_floating(window, cx);
    }

    // The menus.

    /// A menu's row for a workspace: the sheets, its name, and `trailing` at the right.
    fn workspace_row(
        &self,
        ui: &UiFont,
        id: impl Into<ElementId>,
        name: &str,
        trailing: impl IntoElement,
    ) -> gpui::Stateful<Div> {
        let accent = hsla(self.theme.fg(|t| t.agents_accent), 1.0);
        let lead = mask(LAYERS, ui.px(footer_icon::SIZE), accent);
        popover::lead_row(&self.theme, ui, id, lead, false)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(name.to_owned()),
            )
            .child(trailing)
    }

    /// The menu of a group's button. Outside any workspace: the workspaces to add its whole
    /// repository to, and a new one. In one: the others to move it to, and out of this one.
    fn group_menu(
        &self,
        menu: &GroupMenu,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let faint = hsla(theme.fg(|t| t.agents_dimmer), 1.0);
        let others: Vec<_> = (self.listing.panel.workspaces.list().iter())
            .filter(|workspace| Some(&workspace.name) != menu.workspace.as_ref())
            .collect();
        let mut panel = self
            .floated(popover::panel(theme, ui), "group-workspaces", cx)
            .w(ui.px(GROUP_MENU))
            .mt(ui.px(4.0));
        if !others.is_empty() {
            panel = panel.child(popover::heading(
                theme,
                ui,
                match &menu.workspace {
                    Some(_) => "Move to".to_owned(),
                    None => format!("Add {} to", menu.title),
                },
            ));
        }
        for (index, workspace) in others.iter().enumerate() {
            let on_click = {
                let (menu, to) = (menu.clone(), workspace.name.clone());
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.move_group(&menu, &to, window, cx)
                })
            };
            let holds = div()
                .flex_shrink_0()
                .text_size(ui.px(COUNT))
                .text_color(faint)
                .child(holds(workspace.repos.len()));
            panel = panel.child(
                self.workspace_row(ui, ("to-workspace", index), &workspace.name, holds)
                    .on_click(on_click),
            );
        }
        if !others.is_empty() {
            panel = panel.child(popover::rule(theme, ui));
        }
        match &menu.workspace {
            Some(workspace) => {
                let on_click = {
                    let menu = menu.clone();
                    cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.remove_group(&menu, window, cx)
                    })
                };
                panel.child(
                    popover::row(
                        theme,
                        ui,
                        "from-workspace",
                        Icon::Close,
                        format!("Remove from {workspace}"),
                        popover::Tone::Plain,
                    )
                    .on_click(on_click),
                )
            }
            None => {
                let picks = Picks {
                    repos: menu.repos.clone(),
                    ..Picks::default()
                };
                panel.child(self.new_row(picks, ui, cx))
            }
        }
    }

    /// `New workspace…` at a menu's foot, opening the panel with `picks` ticked.
    fn new_row(&self, picks: Picks, ui: &UiFont, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        popover::row(
            &self.theme,
            ui,
            "new-workspace",
            Icon::Plus,
            "New workspace…",
            popover::Tone::Plain,
        )
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.new_workspace(picks.clone(), window, cx)
        }))
    }

    /// The menu of a card's `Workspace` cell: the workspaces, the agent's ticked, picking one
    /// moves its whole repository there; `None` while it is in one, which takes the repository
    /// out; and a new one, with the repository (P5-105).
    fn agent_menu(
        &self,
        card: &Card,
        repo: &Path,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let mut panel = self
            .floated(popover::panel(theme, ui), "agent-workspaces", cx)
            .w(ui.px(AGENT_MENU))
            .mt(ui.px(4.0));
        let workspaces = self.listing.panel.workspaces.list();
        for (index, workspace) in workspaces.iter().enumerate() {
            let on = card.workspace.as_ref() == Some(&workspace.name);
            let on_click = {
                let (repo, to) = (repo.to_owned(), workspace.name.clone());
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if on {
                        this.close_floating(window, cx);
                    } else {
                        this.place_repo(&repo, Some(&to), window, cx);
                    }
                })
            };
            let tick = popover::tick(theme, ui, on);
            panel = panel.child(
                self.workspace_row(ui, ("agent-workspace", index), &workspace.name, tick)
                    .on_click(on_click),
            );
        }
        if card.workspace.is_some() {
            let repo = repo.to_owned();
            let lead = div().size(ui.px(footer_icon::SIZE));
            panel = panel.child(
                popover::lead_row(theme, ui, "agent-no-workspace", lead, false)
                    .child(div().flex_1().child("None"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.place_repo(&repo, None, window, cx)
                    })),
            );
        }
        if !workspaces.is_empty() {
            panel = panel.child(popover::rule(theme, ui));
        }
        let picks = Picks {
            repos: vec![repo.to_owned()],
            ..Picks::default()
        };
        panel.child(self.new_row(picks, ui, cx))
    }

    /// The menu of a workspace's `⋯`: renaming it, picking its members, and removing it, which
    /// asks once more in the menu.
    fn workspace_menu(
        &self,
        name: &str,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let removing = matches!(
            &self.floating,
            Some(Floating::Workspace { removing: true, .. })
        );
        let new_agent = {
            let name = name.to_owned();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.new_agent_in(&name, None, window, cx)
            })
        };
        let rename = {
            let name = name.to_owned();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.start_rename(&name, window, cx)
            })
        };
        let add = {
            let name = name.to_owned();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.edit_members(&name, window, cx)
            })
        };
        let remove = {
            let name = name.to_owned();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                if removing {
                    this.remove_workspace(&name, window, cx);
                } else {
                    this.floating = Some(Floating::Workspace {
                        name: name.clone(),
                        removing: true,
                    });
                    cx.notify();
                }
            })
        };
        let red = fg(|t| t.agents_red);
        let lead = |icon: AnyElement| {
            div()
                .flex_shrink_0()
                .w(ui.px(footer_icon::SIZE))
                .flex()
                .justify_center()
                .child(icon)
        };
        let pencil = mask(PENCIL, ui.px(footer_icon::SIZE), fg(|t| t.agents_dim));
        self.floated(popover::panel(theme, ui), "workspace-menu", cx)
            .w(ui.px(WORKSPACE_MENU))
            .mt(ui.px(4.0))
            .child(
                popover::row(
                    theme,
                    ui,
                    "workspace-new-agent",
                    Icon::NewAgent,
                    "New Agent…",
                    popover::Tone::Plain,
                )
                .on_click(new_agent),
            )
            .child(
                popover::lead_row(theme, ui, "workspace-rename", pencil, false)
                    .child(div().flex_1().child("Rename"))
                    .on_click(rename),
            )
            .child(
                popover::row(
                    theme,
                    ui,
                    "workspace-add",
                    Icon::Plus,
                    "Add repositories…",
                    popover::Tone::Plain,
                )
                .on_click(add),
            )
            .child(popover::rule(theme, ui))
            .child(
                div()
                    .id("workspace-remove")
                    .flex()
                    .items_start()
                    .gap(ui.px(10.0))
                    .px(ui.px(9.0))
                    .py(ui.px(6.0))
                    .rounded(ui.px(6.0))
                    .cursor_pointer()
                    .map(|row| {
                        let tint = red.opacity(0.16);
                        if removing {
                            row.bg(tint)
                        } else {
                            row.hover(move |style| style.bg(tint))
                        }
                    })
                    .on_click(remove)
                    .child(
                        lead(footer_icon::icon(Icon::Close, red, ui.scale(1.0)).into_any_element())
                            .mt(ui.px(1.0)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(div().when(removing, |label| label.text_color(red)).child(
                                if removing {
                                    "Click again to remove"
                                } else {
                                    "Remove workspace"
                                },
                            ))
                            .child(
                                div()
                                    .whitespace_normal()
                                    .text_size(ui.px(COUNT))
                                    .line_height(relative(1.35))
                                    .text_color(fg(|t| t.agents_dimmer))
                                    .child(
                                        "Agents keep running. Its repositories go back to the \
                                         list.",
                                    ),
                            ),
                    ),
            )
    }

    // What a heading and a card add for workspaces.

    /// The button on a group's heading, before its bar, with its menu while open: shown while
    /// the pointer is over the heading `hover` names, its room always kept. `None` for a group
    /// with no repository to move: one that goes by its agents' prefix alone.
    pub(super) fn group_button(
        &self,
        heading: &card::Heading,
        hover: &SharedString,
        cx: &mut Context<Self>,
    ) -> Option<(AnyElement, Option<AnyElement>)> {
        if heading.repos.is_empty() {
            return None;
        }
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let menu = GroupMenu {
            workspace: heading.workspace.clone(),
            group: heading.group.clone(),
            title: heading.title.trim_end_matches('/').to_owned(),
            repos: heading.repos.clone(),
        };
        let key = group_key(menu.workspace.as_deref(), &menu.group);
        let open =
            matches!(&self.floating, Some(floating @ Floating::Group(_)) if floating.key() == key);
        let tip = Tip {
            text: if menu.workspace.is_some() {
                "Move or remove".into()
            } else {
                "Add to workspace".into()
            },
            size: ui.px(NOTE_SIZE),
            color: fg(|t| t.agents_text),
            background: hsla(self.theme.bg(|t| t.agents_bg), 1.0),
            border: fg(|t| t.agents_rule),
        };
        let accent = fg(|t| t.agents_accent);
        let lit = accent.opacity(0.18);
        let hung =
            open.then(|| Self::hung(self.group_menu(&menu, &ui, cx), false).into_any_element());
        let press = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.press_menu(Floating::Group(menu.clone()), event, window, cx);
        });
        // No taller than the heading's words: the button lies over its line, not in it.
        let button = div()
            .flex_shrink_0()
            .h(px(0.0))
            .flex()
            .items_center()
            .child(
                div()
                    .id("group-workspace")
                    .flex_shrink_0()
                    .size(ui.px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui.px(5.0))
                    .cursor_pointer()
                    .hover(move |style| style.bg(lit))
                    .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                    .on_mouse_down(MouseButton::Left, press)
                    .map(|button| {
                        if open {
                            button.bg(lit)
                        } else {
                            button
                                .opacity(0.0)
                                .group_hover(hover.clone(), |style| style.opacity(1.0))
                        }
                    })
                    .child(mask(LAYERS_PLUS, ui.px(13.0), accent)),
            )
            .into_any_element();
        Some((button, hung))
    }

    /// The `Workspace` cell of an open card: `value`, where its agent is, cut to the cell; and
    /// the menu while it is open. One whose repository is not known only says (P5-105).
    pub(super) fn workspace_cell(
        &self,
        card: &Card,
        value: String,
        cx: &mut Context<Self>,
    ) -> WorkspaceCell {
        let Some(repo) = &card.repo else {
            return WorkspaceCell {
                value,
                open: false,
                on_press: None,
                menu: None,
            };
        };
        let ui = UiFont::get(cx);
        let open = matches!(&self.floating, Some(Floating::Agent(name)) if *name == card.name);
        let name = card.name.clone();
        let on_press: OnPress = Box::new(cx.listener(
            move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.press_menu(Floating::Agent(name.clone()), event, window, cx);
            },
        ));
        WorkspaceCell {
            value,
            open,
            on_press: Some(on_press),
            menu: open
                .then(|| Self::hung(self.agent_menu(card, repo, &ui, cx), true).into_any_element()),
        }
    }

    // The panel.

    /// Opens `New workspace` with `picks` ticked: from a group's menu, a card's, the View menu
    /// or the command palette.
    pub fn new_workspace(&mut self, picks: Picks, window: &mut Window, cx: &mut Context<Self>) {
        let colors = input_colors(&self.theme);
        let name = cx.new(|cx| TextInput::new("", "Workspace name", colors, cx));
        let changed = cx.subscribe(&name, |_, _, _: &Changed, cx| cx.notify());
        let editor = Editor {
            workspace: None,
            name: Some(name),
            picks,
            extra: Vec::new(),
            note: None,
            _changed: Some(changed),
        };
        self.float(Floating::Editor(editor), window, cx);
    }

    /// `Add repositories…`: the same panel for a workspace there is, its repositories ticked.
    fn edit_members(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.reload_workspaces();
        let Some(workspace) = self.listing.panel.workspaces.get(name) else {
            return self.close_floating(window, cx);
        };
        let editor = Editor {
            workspace: Some(name.to_owned()),
            name: None,
            picks: Picks::of(workspace),
            extra: Vec::new(),
            note: None,
            _changed: None,
        };
        self.float(Floating::Editor(editor), window, cx);
    }

    fn editor_mut(&mut self) -> Option<&mut Editor> {
        match &mut self.floating {
            Some(Floating::Editor(editor)) => Some(editor),
            _ => None,
        }
    }

    /// `Create`, `Done` or Return: the new workspace with what is ticked, when its name will do;
    /// or the workspace's repositories as ticked since the panel opened, those it gained meanwhile
    /// staying.
    fn confirm_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Floating::Editor(editor)) = &self.floating else {
            return;
        };
        let picks = editor.picks.clone();
        let (workspace, typed) = (
            editor.workspace.clone(),
            editor
                .name
                .as_ref()
                .map(|name| name.read(cx).text().to_owned()),
        );
        // A workspace another window removed meanwhile has no members to set; a name that will
        // not do keeps the panel open.
        let mut done = workspace.is_some();
        let mut made = None;
        self.edit_workspaces(cx, |all| {
            let name = match (workspace, typed) {
                (Some(name), _) => Some(name),
                (None, typed) => {
                    made = all.create(&typed.unwrap_or_default()).ok();
                    made.clone()
                }
            };
            if let Some(name) = name {
                all.set_picked(&name, &picks);
                done = true;
            }
        });
        // A new one stays up and open while it has no agent, rather than at the list's foot
        // (P5-134).
        if let Some(name) = made {
            self.raise_workspace(&name, cx);
        }
        if done {
            self.close_floating(window, cx);
        }
    }

    /// `Choose folder…`: the system's folder chooser. The repository the folder is in gets a row,
    /// ticked; a folder in none does not, and the panel says so.
    fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(folder) = paths.into_iter().next() else {
                return;
            };
            let dir = folder.to_string_lossy().into_owned();
            let repo = cx
                .background_executor()
                .spawn(
                    async move { activity::main_repository("git", &dir, &AtomicBool::new(false)) },
                )
                .await;
            let _ = this.update(cx, |sidebar, cx| {
                let Some(editor) = sidebar.editor_mut() else {
                    return;
                };
                match repo {
                    Some(repo) => {
                        editor.note = None;
                        if !editor.picks.repo(&repo) {
                            editor.picks.repos.push(repo.clone());
                        }
                        if !editor.extra.contains(&repo) {
                            editor.extra.push(repo);
                        }
                    }
                    None => {
                        let name = kanban_repos::name(&folder);
                        editor.note = Some(format!("{name} isn't in a Git repository").into());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The panel while it is open, hung at the top of the list, drawn over everything.
    pub(super) fn editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(Floating::Editor(editor)) = &self.floating else {
            return None;
        };
        let ui = UiFont::get(cx);
        let panel = self.editor_panel(editor, &ui, window, cx);
        Some(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(PAD))
                .child(deferred(
                    anchored().snap_to_window_with_margin(px(8.0)).child(panel),
                ))
                .into_any_element(),
        )
    }

    fn editor_panel(
        &self,
        editor: &Editor,
        ui: &UiFont,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let (text, dim, dimmer, accent) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_accent),
        );
        let ground = hsla(theme.bg(|t| t.agents_bg), 1.0);
        let all = &self.listing.panel.workspaces;
        // The name: what it would be kept as, or why it will not do.
        let name = editor.name.as_ref().map(|input| {
            let typed = input.read(cx).text().to_owned();
            (input.clone(), all.name(&typed, None))
        });
        let ready = name.as_ref().is_none_or(|(_, name)| name.is_ok());

        let tick = |on: bool| {
            div()
                .flex_shrink_0()
                .size(ui.px(BOX))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(3.0))
                .map(|tick| {
                    if on {
                        tick.bg(accent).child(footer_icon::icon(
                            Icon::Check,
                            ground,
                            ui.scale(BOX / footer_icon::SIZE),
                        ))
                    } else {
                        tick.border_1().border_color(dimmer)
                    }
                })
        };
        let faint = |words: String| {
            div()
                .flex_shrink_0()
                .text_size(ui.px(COUNT))
                .text_color(dimmer)
                .child(words)
        };
        let label = |words: String, on: bool| {
            div()
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .overflow_hidden()
                .text_ellipsis()
                .when(on, |label| label.font_weight(FontWeight::SEMIBOLD))
                .child(words)
        };
        let row = |id: ElementId, lit: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(9.0))
                .h(ui.px(ROW))
                .px(ui.px(8.0))
                .rounded(ui.px(7.0))
                .cursor_pointer()
                .when(lit, |row| row.bg(accent.opacity(0.08)))
        };

        let mut list = div()
            .id("workspace-members")
            .flex()
            .flex_col()
            .gap(px(2.0))
            .max_h((window.viewport_size().height - ui.px(PANEL_REST)).max(ui.px(3.0 * ROW)))
            .overflow_y_scroll();
        // A row for each repository, by its main worktree (P5-105): those agents work in, then the
        // workspace's own and the folders chosen. Two of the same name are told apart by where
        // they lie, though the list's groups have them as one (P5-80).
        let mut rows: Vec<(PathBuf, usize)> = (self.listing.panel.repositories().into_iter())
            .map(|listed| (listed.repo, listed.agents))
            .collect();
        for repo in editor.picks.base.iter().chain(&editor.extra) {
            if !rows.iter().any(|(listed, _)| listed == repo) {
                rows.push((repo.clone(), 0));
            }
        }
        let names: Vec<String> = rows
            .iter()
            .map(|(repo, _)| kanban_repos::name(repo))
            .collect();
        for (index, ((repo, agents), called)) in rows.iter().zip(&names).enumerate() {
            let on = editor.picks.repo(repo);
            let elsewhere = all
                .of_repo(repo)
                .filter(|name| Some(*name) != editor.workspace.as_deref());
            let count = match elsewhere {
                Some(other) if !on => format!("in {other}"),
                _ => workspaces::agents(*agents),
            };
            let twin = names.iter().filter(|other| *other == called).count() > 1;
            let place = twin.then(|| {
                let parent = repo.parent().unwrap_or(repo).display().to_string();
                let parent = card::tilde(&parent, self.home.as_deref());
                let parent = card::shorten(&parent, &|path| path.chars().count() <= TWIN_PATH);
                label(parent, false)
                    .text_size(ui.px(COUNT))
                    .text_color(dimmer)
            });
            let on_click = {
                let repo = repo.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(editor) = this.editor_mut() {
                        editor.picks.toggle_repo(&repo);
                        cx.notify();
                    }
                })
            };
            list = list.child(
                row(("workspace-repo", index).into(), on)
                    .on_click(on_click)
                    .child(tick(on))
                    .child(label(called.clone(), on))
                    .children(place)
                    .child(div().flex_1())
                    .child(faint(count)),
            );
        }

        let choose = div()
            .id("workspace-choose")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(9.0))
            .h(ui.px(34.0))
            .px(ui.px(8.0))
            .rounded(ui.px(7.0))
            .text_color(dim)
            .cursor_pointer()
            .hover(move |style| style.bg(text.opacity(0.06)))
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.choose_folder(window, cx)),
            )
            .child(footer_icon::icon(
                Icon::Folder,
                dim,
                ui.scale(13.0 / footer_icon::SIZE),
            ))
            .child("Choose folder…");
        let small = || {
            div()
                .px(ui.px(8.0))
                .whitespace_normal()
                .text_size(ui.px(COUNT))
                .line_height(relative(1.4))
        };
        let button = |id: &'static str, words: &'static str| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(ui.px(30.0))
                .px(ui.px(14.0))
                .flex()
                .items_center()
                .rounded(ui.px(8.0))
                .text_size(ui.px(12.5))
                .child(words)
        };
        let confirm = button(
            "workspace-confirm",
            if editor.workspace.is_some() {
                "Done"
            } else {
                "Create"
            },
        )
        .bg(accent)
        .text_color(ground)
        .font_weight(FontWeight::SEMIBOLD)
        .map(|button| {
            if ready {
                button
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.confirm_editor(window, cx)
                    }))
            } else {
                button.opacity(0.4)
            }
        });
        let title = match &editor.workspace {
            Some(name) => format!("Add to {name}"),
            None => "New workspace".to_owned(),
        };
        let summary = editor.picks.summary();
        self.floated(popover::panel(theme, ui), "workspace-editor", cx)
            .w(ui.px(PANEL))
            .p(ui.px(PANEL_PAD))
            .gap(ui.px(12.0))
            .rounded(ui.px(12.0))
            .on_action(cx.listener(|this, _: &menu::OpenSelected, window, cx| {
                this.confirm_editor(window, cx)
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(8.0))
                    .child(mask(LAYERS, ui.px(15.0), accent))
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_size(ui.px(14.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    ),
            )
            .children(name.map(|(input, name)| {
                let problem = name.err();
                div()
                    .flex()
                    .flex_col()
                    .gap(ui.px(5.0))
                    .child(div().text_size(ui.px(COUNT)).text_color(dim).child("Name"))
                    .child(
                        div()
                            .h(ui.px(30.0))
                            .px(ui.px(9.0))
                            .flex()
                            .items_center()
                            .rounded(ui.px(7.0))
                            .border_1()
                            .border_color(accent.opacity(0.6))
                            .bg(ground)
                            .child(div().flex_1().min_w(px(0.0)).child(input)),
                    )
                    .children(problem.map(|problem| {
                        small().px(px(0.0)).child(problem.text()).text_color(
                            if problem == workspaces::NameError::Taken {
                                fg(|t| t.agents_red)
                            } else {
                                dimmer
                            },
                        )
                    }))
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(ui.px(6.0))
                            .pb(ui.px(3.0))
                            .text_size(ui.px(COUNT))
                            .child(div().text_color(dim).child("Repositories"))
                            .child(div().flex_1())
                            .child(div().text_color(dimmer).child(summary)),
                    )
                    .child(list)
                    .child(choose)
                    .children(
                        editor
                            .note
                            .clone()
                            .map(|note| small().text_color(fg(|t| t.agents_yellow)).child(note)),
                    )
                    .child(small().pt(ui.px(4.0)).text_color(dimmer).child(
                        "A ticked repository brings every agent in it, and any started there \
                         later.",
                    )),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(ui.px(8.0))
                    .pt(ui.px(2.0))
                    .child(
                        button("workspace-cancel", "Cancel")
                            .bg(text.opacity(0.08))
                            .cursor_pointer()
                            .hover(move |style| style.bg(text.opacity(0.12)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.close_floating(window, cx)
                            })),
                    )
                    .child(confirm),
            )
    }
}

/// An open workspace's face, resting or `lit`, over the column's `base` (P5-91): a step darker,
/// on a light sidebar too, so it reads as a well.
pub(super) fn well(theme: &Theme, base: Hsla, lit: bool) -> Hsla {
    let dark = hsla(theme.bg(|t| t.agents_bg), 1.0).l < 0.5;
    let [on_dark, on_light] = if lit { WELL_LIT } else { WELL };
    over(
        gpui::black().opacity(if dark { on_dark } else { on_light }),
        base,
    )
}

/// The idle shelf's face over the column's `base` (P5-164): fainter than an open workspace's, so
/// it reads as laid aside rather than in use, on a light sidebar too.
fn shelf(theme: &Theme, base: Hsla) -> Hsla {
    let dark = hsla(theme.bg(|t| t.agents_bg), 1.0).l < 0.5;
    let [on_dark, on_light] = SHELF;
    over(
        gpui::black().opacity(if dark { on_dark } else { on_light }),
        base,
    )
}

/// How agents are doing, a short stroke each: a group's bar, or a folded workspace's.
pub(super) fn bar_of(theme: &Theme, ui: &UiFont, bar: Vec<Status>) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .gap(px(2.0))
        .w(ui.px((BAR_SHARE * bar.len() as f32).min(BAR_MAX)))
        .children(bar.into_iter().map(|status| {
            div()
                .flex_1()
                .h(px(3.0))
                .rounded(px(2.0))
                .bg(hsla(theme.fg(card::look(status).color), 1.0))
        }))
}

/// The workspace and the group before a tile's name in its hover note: `Agent tools › paddock/`.
pub(super) fn tip_group(card: &Card) -> Option<String> {
    let group = (!card.group.is_empty()).then(|| format!("{}/", card.group));
    match &card.workspace {
        Some(workspace) => Some(format!("{workspace} › {}", group.unwrap_or_default())),
        None => group,
    }
}
