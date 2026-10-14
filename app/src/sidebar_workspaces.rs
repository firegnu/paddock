//! Workspaces in the sidebar (DESIGN §13 P5-89): the tray a workspace's groups lie in, with its
//! title; the three menus (a group's button, a card's `Workspace` cell, a workspace's `⋯`); the
//! panel that names a new workspace and picks its members, or picks those of one there is; and
//! the title as a field while it is renamed. Who is in which workspace is `workspaces.rs`'s, and
//! the list's lines are `card.rs`'s.
use super::*;
use crate::{
    activity, kanban_repos, menu, popover,
    text_input::{self, Changed, TextInput},
    workspaces::{self, Picks, Workspaces},
};
use gpui::{Anchor, Focusable, PathPromptOptions, Subscription, anchored, deferred};
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

// Sizes, in points at the base interface size.
/// The tray: its corners, the room it keeps at its sides and foot, and above its title.
const TRAY_RADIUS: f32 = 14.0;
pub(super) const TRAY_X: f32 = 6.0;
const TRAY_TOP: f32 = 2.0;
/// The title's height in an open tray, and a folded tray's.
const TITLE: f32 = 30.0;
const FOLDED: f32 = 34.0;
/// The tray's face and edge, as shares of the text colour; lit while its menu is open.
const FACE: f32 = 0.035;
const EDGE: f32 = 0.09;
const FACE_LIT: f32 = 0.05;
const EDGE_LIT: f32 = 0.13;
const NAME: f32 = 13.0;
const COUNT: f32 = 11.0;
/// The menus, and the panel.
const GROUP_MENU: f32 = 232.0;
const AGENT_MENU: f32 = 216.0;
const WORKSPACE_MENU: f32 = 236.0;
const PANEL: f32 = 320.0;
const PANEL_PAD: f32 = 14.0;
/// A row of the panel's list, and an agent's row under its group.
const ROW: f32 = 32.0;
const AGENT_ROW: f32 = 28.0;
const BOX: f32 = 13.0;
/// The window's height the panel leaves to everything but its list.
const PANEL_REST: f32 = 340.0;

/// A workspace: sheets lying on one another.
const LAYERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M8 2L14 5 8 8 2 5ZM2 8L8 11 14 8M2 11L8 14 14 11" fill="none" stroke="#000" stroke-width="1.4" stroke-linejoin="round"/></svg>"##;
/// Into a workspace: the sheets and a plus.
const LAYERS_PLUS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M8 3L13 5.5 8 8 3 5.5ZM3 8.5L8 11 10 10M12.5 10v4M10.5 12h4" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const DOTS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="3.5" cy="8" r="1.3"/><circle cx="8" cy="8" r="1.3"/><circle cx="12.5" cy="8" r="1.3"/></svg>"##;
const PENCIL: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M3 13h2.5L13 5.5 10.5 3 3 10.5z" fill="none" stroke="#000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// What floats over the list, one at a time.
pub(super) enum Floating {
    /// A group's button: the workspaces to add it or move it to.
    Group(GroupMenu),
    /// A card's `Workspace` cell: the workspaces for its agent, by name.
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
}

impl Floating {
    /// Which button a menu hangs from, to tell a press on it from one elsewhere.
    fn key(&self) -> String {
        match self {
            Floating::Group(menu) => group_key(menu.workspace.as_deref(), &menu.group),
            Floating::Agent(name) => agent_key(name),
            Floating::Workspace { name, .. } => workspace_key(name),
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
    singles: Vec<String>,
}

/// The panel that picks a workspace's members.
pub(super) struct Editor {
    /// The workspace whose members are picked; `None` for a new one.
    workspace: Option<String>,
    /// The new one's name.
    name: Option<Entity<TextInput>>,
    picks: Picks,
    /// The groups whose agents are listed.
    open: Vec<String>,
    /// Repositories no group stands for: folders chosen, and members with no agent now.
    extra: Vec<PathBuf>,
    /// The workspace's single agents that were not running as it opened.
    absent: Vec<String>,
    /// Why the folder chosen was not added.
    note: Option<SharedString>,
    _changed: Option<Subscription>,
}

/// A press on a button that opens a menu.
type OnPress = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// A card's `Workspace` cell: where its agent is, the press that opens its menu, and the menu
/// while it is open.
pub(super) struct WorkspaceCell {
    pub value: String,
    pub open: bool,
    pub on_press: OnPress,
    pub menu: Option<AnyElement>,
}

fn input_colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
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

/// What a workspace holds, faint after its name in a menu.
fn holds(workspace: &workspaces::Workspace) -> String {
    match (workspace.repos.len(), workspace.agents.len()) {
        (0, 0) => "Empty".to_owned(),
        (0, agents) => workspaces::agents(agents),
        (1, _) => "1 repo".to_owned(),
        (repos, _) => format!("{repos} repos"),
    }
}

impl Sidebar {
    // What is kept.

    /// Reads the workspaces again: another window may have changed them.
    pub(super) fn reload_workspaces(&mut self) {
        if let Some(path) = &self.workspaces_path {
            self.listing.panel.workspaces = Workspaces::load(path);
        }
    }

    /// Changes the workspaces, as they are kept now, and keeps them.
    fn edit_workspaces(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Workspaces)) {
        self.reload_workspaces();
        change(&mut self.listing.panel.workspaces);
        if let Some(path) = &self.workspaces_path
            && let Err(error) = self.listing.panel.workspaces.save(path)
        {
            self.note(format!("Workspaces not saved: {error}"), true, cx);
        }
        cx.notify();
    }

    // What floats.

    /// Opens `floating` in place of any other. It takes the keys, for Esc and Return, and gives
    /// them back as it closes.
    fn float(&mut self, floating: Floating, window: &mut Window, cx: &mut Context<Self>) {
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

    fn close_floating(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    fn press_menu(
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
    fn floated(&self, panel: Div, id: &'static str, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
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
    fn hung(menu: impl IntoElement, left: bool) -> Div {
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

    /// A workspace: its title and under it `inside`, its groups' lines, in a tray; folded, the
    /// title alone with the bar of all its agents. A click on the title folds it or opens it.
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
        } = tray;
        let hover: SharedString = format!("workspace-{name}").into();
        let menu_open = matches!(
            &self.floating,
            Some(Floating::Workspace { name: open, .. }) if *open == name
        );
        let renaming = match &self.floating {
            Some(Floating::Rename {
                name: renamed,
                input,
                ..
            }) if *renamed == name => Some(input.clone()),
            _ => None,
        };
        let fold = {
            let name = name.clone();
            cx.listener(move |this, _: &ClickEvent, _, cx| this.fold_workspace(&name, cx))
        };
        let options = {
            let name = name.clone();
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
            border: fg(|t| t.agents_rule),
        };
        let lit = text.opacity(0.12);
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
                dim,
            ))
            .child(mask(LAYERS, ui.px(14.0), accent))
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
            .child(
                div()
                    .id("workspace-options")
                    .flex_shrink_0()
                    .size(ui.px(22.0))
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
                            button.bg(lit)
                        } else {
                            button
                                .opacity(0.0)
                                .group_hover(hover.clone(), |style| style.opacity(1.0))
                        }
                    })
                    .child(mask(DOTS, ui.px(14.0), text)),
            )
            // Folded, how all its agents are doing; open, its groups' bars say.
            .when(folded, |title| title.child(bar_of(&self.theme, &ui, bar)))
            .when(menu_open, |title| {
                title.child(Self::hung(self.workspace_menu(&name, &ui, cx), false))
            });
        let (face, edge) = if menu_open {
            (FACE_LIT, EDGE_LIT)
        } else {
            (FACE, EDGE)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(CARD_GAP))
            // A little off what is above it; closer to the top for the first, as a heading is.
            .mt(px(if first { 8.0 } else { 4.0 }))
            .px(px(TRAY_X))
            .when(!folded, |tray| tray.pt(px(TRAY_TOP)).pb(px(TRAY_X)))
            .rounded(px(TRAY_RADIUS))
            .border_1()
            .border_color(text.opacity(edge))
            .bg(text.opacity(face))
            .hover(move |tray| {
                tray.bg(text.opacity(FACE_LIT))
                    .border_color(text.opacity(EDGE_LIT))
            })
            .child(title)
            .children(inside)
            .into_any_element()
    }

    /// A whole repository of a workspace with no agent now, or (`None`) a workspace with nothing
    /// to list: a faint line.
    pub(super) fn vacant(&self, repo: Option<String>, cx: &mut Context<Self>) -> AnyElement {
        let ui = UiFont::get(cx);
        let dimmer = hsla(self.theme.fg(|t| t.agents_dimmer), 1.0);
        div()
            .flex()
            .items_baseline()
            .gap(ui.px(6.0))
            .px(px(4.0))
            .py(px(2.0))
            .text_size(ui.px(LABEL_SIZE))
            .children(repo.map(|repo| {
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(dimmer)
                    .child(repo)
            }))
            .child(div().text_color(dimmer.opacity(0.6)).child("No agents"))
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
        self.listing.panel.folded.rename_workspace(from, &name);
        self.folds_changed(cx);
        self.close_floating(window, cx);
    }

    /// `Remove workspace`, confirmed: the grouping goes, its members back to the list below.
    fn remove_workspace(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_workspaces(cx, |all| all.remove(name));
        self.listing.panel.folded.forget_workspace(name);
        self.folds_changed(cx);
        self.close_floating(window, cx);
    }

    /// A group into `to`: its whole repositories, and its agents recorded singly.
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
            for agent in &menu.singles {
                all.add_agent(to, agent);
            }
        });
        self.close_floating(window, cx);
    }

    /// A group out of its workspace: what came whole goes whole, what came singly goes singly.
    fn remove_group(&mut self, menu: &GroupMenu, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_workspaces(cx, |all| {
            for repo in &menu.repos {
                all.remove_repo(repo);
            }
            for agent in &menu.singles {
                all.remove_agent(agent);
            }
        });
        self.close_floating(window, cx);
    }

    /// An agent recorded by itself in `to`, or (`None`) in none.
    fn place_agent(
        &mut self,
        agent: &str,
        to: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_workspaces(cx, |all| match to {
            Some(to) => all.add_agent(to, agent),
            None => all.remove_agent(agent),
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
                .child(holds(workspace));
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
                    agents: Vec::new(),
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
    /// records the agent in it by itself; `None` for one recorded so, which takes it out; and a
    /// new one.
    fn agent_menu(&self, card: &Card, ui: &UiFont, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let mut panel = self
            .floated(popover::panel(theme, ui), "agent-workspaces", cx)
            .w(ui.px(AGENT_MENU))
            .mt(ui.px(4.0));
        let workspaces = self.listing.panel.workspaces.list();
        for (index, workspace) in workspaces.iter().enumerate() {
            let on = card.workspace.as_ref() == Some(&workspace.name);
            let on_click = {
                let (agent, to) = (card.name.clone(), workspace.name.clone());
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if on {
                        this.close_floating(window, cx);
                    } else {
                        this.place_agent(&agent, Some(&to), window, cx);
                    }
                })
            };
            let tick = popover::tick(theme, ui, on);
            panel = panel.child(
                self.workspace_row(ui, ("agent-workspace", index), &workspace.name, tick)
                    .on_click(on_click),
            );
        }
        if card.alone {
            let agent = card.name.clone();
            let lead = div().size(ui.px(footer_icon::SIZE));
            panel = panel.child(
                popover::lead_row(theme, ui, "agent-no-workspace", lead, false)
                    .child(div().flex_1().child("None"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.place_agent(&agent, None, window, cx)
                    })),
            );
        }
        if !workspaces.is_empty() {
            panel = panel.child(popover::rule(theme, ui));
        }
        let picks = Picks {
            repos: Vec::new(),
            agents: vec![card.name.clone()],
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
    /// with nothing to move: one that goes by its agents' prefix alone.
    pub(super) fn group_button(
        &self,
        heading: &card::Heading,
        hover: &SharedString,
        cx: &mut Context<Self>,
    ) -> Option<(AnyElement, Option<AnyElement>)> {
        if heading.repos.is_empty() && heading.singles.is_empty() {
            return None;
        }
        let ui = UiFont::get(cx);
        let fg = |pick: Pick| hsla(self.theme.fg(pick), 1.0);
        let menu = GroupMenu {
            workspace: heading.workspace.clone(),
            group: heading.group.clone(),
            title: heading.title.trim_end_matches('/').to_owned(),
            repos: heading.repos.clone(),
            singles: heading.singles.clone(),
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
    /// the menu while it is open.
    pub(super) fn workspace_cell(
        &self,
        card: &Card,
        value: String,
        cx: &mut Context<Self>,
    ) -> WorkspaceCell {
        let ui = UiFont::get(cx);
        let open = matches!(&self.floating, Some(Floating::Agent(name)) if *name == card.name);
        let name = card.name.clone();
        WorkspaceCell {
            value,
            open,
            on_press: Box::new(
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.press_menu(Floating::Agent(name.clone()), event, window, cx);
                }),
            ),
            menu: open.then(|| Self::hung(self.agent_menu(card, &ui, cx), true).into_any_element()),
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
            open: Vec::new(),
            extra: Vec::new(),
            absent: Vec::new(),
            note: None,
            _changed: Some(changed),
        };
        self.open_editor(editor, window, cx);
    }

    /// `Add repositories…`: the same panel for a workspace there is, its members ticked.
    fn edit_members(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.reload_workspaces();
        let panel = &self.listing.panel;
        let Some(workspace) = panel.workspaces.get(name) else {
            return self.close_floating(window, cx);
        };
        let running = |agent: &&String| panel.agents.iter().any(|a| a.name == **agent);
        let editor = Editor {
            workspace: Some(name.to_owned()),
            name: None,
            picks: Picks::of(workspace),
            open: Vec::new(),
            extra: workspace.repos.clone(),
            absent: (workspace.agents.iter())
                .filter(|agent| !running(agent))
                .cloned()
                .collect(),
            note: None,
            _changed: None,
        };
        self.open_editor(editor, window, cx);
    }

    /// The panel opens with the agents listed under each group only some of whose are ticked.
    fn open_editor(&mut self, mut editor: Editor, window: &mut Window, cx: &mut Context<Self>) {
        let panel = &self.listing.panel;
        let taken = panel.workspaces.taken(editor.workspace.as_deref());
        editor.open = (panel.groups().into_iter())
            .filter(|group| !editor.picks.whole(group) && editor.picks.ticked(group, &taken) > 0)
            .map(|group| group.name)
            .collect();
        self.float(Floating::Editor(editor), window, cx);
    }

    fn editor_mut(&mut self) -> Option<&mut Editor> {
        match &mut self.floating {
            Some(Floating::Editor(editor)) => Some(editor),
            _ => None,
        }
    }

    /// `Create`, `Done` or Return: the new workspace with what is ticked, when its name will do;
    /// or the workspace's members as ticked.
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
        self.edit_workspaces(cx, |all| {
            let name = match (workspace, typed) {
                (Some(name), _) => Some(name),
                (None, typed) => all.create(&typed.unwrap_or_default()).ok(),
            };
            if let Some(name) = name {
                all.set_members(&name, &picks.repos, &picks.agents);
                done = true;
            }
        });
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
        let taken = all.taken(editor.workspace.as_deref());
        let groups = self.listing.panel.groups();
        // The name: what it would be kept as, or why it will not do.
        let name = editor.name.as_ref().map(|input| {
            let typed = input.read(cx).text().to_owned();
            (input.clone(), all.name(&typed, None))
        });
        let ready = name.as_ref().is_none_or(|(_, name)| name.is_ok());

        let tick = |on: Option<bool>| {
            // Ticked, only some of it ticked (`None`), or not.
            let mark = match on {
                Some(true) => Some(
                    footer_icon::icon(Icon::Check, ground, ui.scale(BOX / footer_icon::SIZE))
                        .into_any_element(),
                ),
                None => Some(
                    div()
                        .w(ui.px(7.0))
                        .h(ui.px(2.0))
                        .rounded(px(1.0))
                        .bg(ground)
                        .into_any_element(),
                ),
                Some(false) => None,
            };
            div()
                .flex_shrink_0()
                .size(ui.px(BOX))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(3.0))
                .map(|tick| match mark {
                    Some(mark) => tick.bg(accent).child(mark),
                    None => tick.border_1().border_color(dimmer),
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
                .pl(ui.px(8.0))
                .pr(ui.px(4.0))
                .rounded(ui.px(7.0))
                .cursor_pointer()
                .when(lit, |row| row.bg(accent.opacity(0.08)))
        };
        // The kind's colour beside each agent, as its avatar has it.
        let kinds: HashMap<String, Hsla> = (self.listing)
            .unfolded(None, &[], None, now())
            .into_iter()
            .filter_map(|line| match line {
                Line::Agent(card) => Some((card.name.clone(), kind_color(theme, &card))),
                _ => None,
            })
            .collect();

        let mut list = div()
            .id("workspace-members")
            .flex()
            .flex_col()
            .gap(px(2.0))
            .max_h((window.viewport_size().height - ui.px(PANEL_REST)).max(ui.px(3.0 * ROW)))
            .overflow_y_scroll();
        for (index, group) in groups.iter().enumerate() {
            let whole = editor.picks.whole(group);
            let ticked = editor.picks.ticked(group, &taken);
            let open = editor.open.contains(&group.name);
            let elsewhere = (group.repos.iter())
                .find_map(|repo| all.of_repo(repo))
                .filter(|name| Some(*name) != editor.workspace.as_deref());
            let count = match (elsewhere, whole, ticked) {
                (Some(other), false, 0) => format!("in {other}"),
                (_, false, ticked) if ticked > 0 => format!("{ticked} of {}", group.agents.len()),
                _ => workspaces::agents(group.agents.len()),
            };
            let title = if group.name.is_empty() {
                "agents".to_owned()
            } else {
                group.name.clone()
            };
            let toggle_open = || {
                let name = group.name.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    if let Some(editor) = this.editor_mut() {
                        match editor.open.iter().position(|open| *open == name) {
                            Some(at) => {
                                editor.open.remove(at);
                            }
                            None => editor.open.push(name.clone()),
                        }
                        cx.notify();
                    }
                })
            };
            // A group that goes by its agents' prefix has no repository to tick: its row only
            // opens it.
            let head = if group.repos.is_empty() {
                row(("workspace-group", index).into(), false)
                    .child(div().flex_shrink_0().size(ui.px(BOX)))
                    .on_click(toggle_open())
            } else {
                let on_click = {
                    let (group, taken) = (group.clone(), taken.clone());
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(editor) = this.editor_mut() {
                            editor.picks.toggle_whole(&group, &taken);
                            cx.notify();
                        }
                    })
                };
                row(("workspace-group", index).into(), whole && !open)
                    .child(tick(if whole {
                        Some(true)
                    } else if ticked > 0 {
                        None
                    } else {
                        Some(false)
                    }))
                    .on_click(on_click)
            };
            let head = head
                .child(label(title, whole || ticked > 0))
                .child(div().flex_1())
                .child(faint(count))
                .child(
                    div()
                        .id("workspace-group-open")
                        .flex_shrink_0()
                        .size(ui.px(22.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(ui.px(5.0))
                        .hover(move |style| style.bg(text.opacity(0.08)))
                        .on_click(toggle_open())
                        .child(mask(
                            if open { CHEVRON_DOWN } else { CHEVRON_RIGHT },
                            ui.px(10.0),
                            dim,
                        )),
                );
            if !open {
                list = list.child(head);
                continue;
            }
            let mut section = div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .rounded(ui.px(7.0))
                .bg(text.opacity(0.045))
                .pb(ui.px(4.0))
                .child(head);
            for (at, agent) in group.agents.iter().enumerate() {
                let on = editor.picks.agent(group, agent, &taken);
                let on_click = {
                    let (group, agent, taken) = (group.clone(), agent.clone(), taken.clone());
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(editor) = this.editor_mut() {
                            editor.picks.toggle_agent(&group, &agent, &taken);
                            cx.notify();
                        }
                    })
                };
                let short = agent
                    .strip_prefix(crate::agents::group(agent))
                    .unwrap_or(agent)
                    .to_owned();
                let other = all
                    .of_agent(agent)
                    .filter(|name| Some(*name) != editor.workspace.as_deref());
                section = section.child(
                    div()
                        .id(("workspace-agent", index * 1000 + at))
                        .flex()
                        .items_center()
                        .gap(ui.px(9.0))
                        .h(ui.px(AGENT_ROW))
                        .pl(ui.px(30.0))
                        .pr(ui.px(8.0))
                        .cursor_pointer()
                        .on_click(on_click)
                        .child(tick(Some(on)))
                        .child(
                            div()
                                .flex_shrink_0()
                                .size(ui.px(14.0))
                                .rounded(ui.px(4.0))
                                .bg(kinds.get(agent).copied().unwrap_or(dim)),
                        )
                        .child(
                            label(short, false)
                                .text_size(ui.px(12.5))
                                .when(!on, |label| label.text_color(dim)),
                        )
                        .child(div().flex_1())
                        .children(other.map(|other| faint(format!("in {other}")))),
                );
            }
            list = list.child(section);
        }
        // Repositories no group stands for: folders chosen, and members with no agent now.
        let extra = (editor.extra.iter())
            .filter(|repo| !groups.iter().any(|group| group.repos.contains(repo)));
        for (index, repo) in extra.enumerate() {
            let on = editor.picks.repo(repo);
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
                    .pr(ui.px(8.0))
                    .on_click(on_click)
                    .child(tick(Some(on)))
                    .child(label(kanban_repos::name(repo), on))
                    .child(div().flex_1())
                    .child(faint(workspaces::agents(0))),
            );
        }
        // Its single agents that are not running: unticked, they are forgotten.
        if !editor.absent.is_empty() {
            list = list.child(
                div()
                    .flex_shrink_0()
                    .px(ui.px(8.0))
                    .pt(ui.px(8.0))
                    .pb(ui.px(2.0))
                    .text_size(ui.px(COUNT))
                    .text_color(dim)
                    .child("Not running"),
            );
        }
        for (index, agent) in editor.absent.iter().enumerate() {
            let on = editor.picks.agents.contains(agent);
            let on_click = {
                let agent = agent.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(editor) = this.editor_mut() {
                        match editor
                            .picks
                            .agents
                            .iter()
                            .position(|picked| *picked == agent)
                        {
                            Some(at) => {
                                editor.picks.agents.remove(at);
                            }
                            None => editor.picks.agents.push(agent.clone()),
                        }
                        cx.notify();
                    }
                })
            };
            list = list.child(
                row(("workspace-absent", index).into(), false)
                    .h(ui.px(AGENT_ROW))
                    .pr(ui.px(8.0))
                    .on_click(on_click)
                    .child(tick(Some(on)))
                    .child(label(agent.clone(), false).text_size(ui.px(12.5))),
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
                            .child(div().text_color(dim).child("Repositories and agents"))
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
                         later. Tick single agents to add only those.",
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
