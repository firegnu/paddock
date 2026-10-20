//! Text selected in a terminal, sent to an agent (DESIGN §13 P5-148c): the small panel the
//! selection's bar opens, hanging from where the bar was. The agents to choose from (the pane's own
//! first, marked `this pane`, then the sidebar's order), a note, what will go, and Send; the text
//! goes with the pane's machine's `corral send` in the background, as the Inbox's replies do. What
//! goes, in what order and what a refusal leaves are `send_to.rs`'s.
use super::*;
use crate::{
    corral::Client,
    send_to::{self, Target},
    view::SendSelection,
};
use gpui::{Subscription, WeakEntity};
use std::sync::atomic::AtomicBool;

/// The panel's width, at the base interface size, and the most its preview is tall.
const WIDTH: f32 = 400.0;
const PREVIEW: f32 = 150.0;

pub(super) struct SendPanel {
    state: send_to::Panel,
    /// When it opened: which panel a late answer from corral is for.
    opened: Instant,
    /// The pane the selection came from, the agent it shows, and its machine's sidebar and link.
    pane: WeakEntity<TerminalView>,
    own: Option<String>,
    sidebar: Entity<Sidebar>,
    remote: Option<Arc<remote::Link>>,
    /// The selection's bar in the window, which the panel hangs from.
    at: Bounds<Pixels>,
    input: Entity<TextInput>,
    list: ScrollHandle,
    _typing: Subscription,
}

impl SendPanel {
    /// Whom it can go to, as the sidebar lists its machine's agents now.
    fn targets(&self, cx: &gpui::App) -> Vec<Target> {
        let panel = self.sidebar.read(cx).panel();
        let listed: Vec<send_to::Listed> = panel
            .ordered(now())
            .into_iter()
            .map(|agent| send_to::Listed {
                name: &agent.name,
                kind: agent.kind.as_deref(),
                status: panel.status(agent, now()),
            })
            .collect();
        send_to::targets(&listed, self.own.as_deref())
    }
}

impl PaddockWindow {
    /// The selection's bar in `pane` asked to send what is selected: the panel opens, the note's
    /// field taking the keys.
    pub(super) fn open_send(
        &mut self,
        pane: &Entity<TerminalView>,
        event: &SendSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self
            .panes
            .iter()
            .find_map(|(id, view)| (view == pane).then_some(*id));
        let own = match id.map(|id| self.workspace.shown(id)) {
            Some(Shown::Agent(name)) => Some(name.clone()),
            _ => None,
        };
        let sidebar = self.sidebar.clone();
        let kind = own.as_deref().and_then(|name| {
            let agents = sidebar.read(cx).agents();
            agents.into_iter().find(|a| a.name == name)?.kind
        });
        let source = send_to::source(&event.pane, kind.as_deref(), self.home().as_deref());
        let colors = text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        };
        let input = cx.new(|cx| TextInput::new("", "Add a note (optional)", colors, cx));
        let typing = cx.subscribe(&input, |this, input, _: &Changed, cx| {
            let note = input.read(cx).text().to_owned();
            // What corral said to the last try no longer stands, as in the Inbox.
            if let Some(panel) = &mut this.send {
                panel.state.note = note;
                panel.state.refused = None;
            }
            cx.notify();
        });
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.let_go(Some(Popup::Send));
        self.palette = None;
        self.chooser = Chooser::default();
        self.send = Some(SendPanel {
            state: send_to::Panel::new(event.text.clone(), source),
            opened: Instant::now(),
            pane: pane.downgrade(),
            own,
            sidebar,
            remote: self.machines.link(self.seats.here()).cloned(),
            at: event.at,
            input,
            list: ScrollHandle::new(),
            _typing: typing,
        });
        self.popup = Some(Popup::Send);
        cx.notify();
    }

    /// ↑↓ in the panel: the agent before or after.
    pub(super) fn step_send(&mut self, step: isize, cx: &mut Context<Self>) {
        let Some(panel) = &mut self.send else {
            return;
        };
        let targets = panel.targets(cx);
        panel.state.step(step, &targets);
        if let Some(index) = panel.state.chosen(&targets) {
            // After the heading.
            panel.list.scroll_to_item(index + 1);
        }
        cx.notify();
    }

    /// Send, or ↩: to the agent chosen, in the background. Taken, the panel closes and the pane
    /// says so; refused, the panel stays as it was with why under it, in the Inbox's words. Closed
    /// meanwhile, the pane says how it went.
    pub(super) fn send_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let program = self.template.corral.clone();
        let Some(panel) = &mut self.send else {
            return;
        };
        let targets = panel.targets(cx);
        let Some((to, words)) = panel.state.start(&targets) else {
            return;
        };
        let client = Client {
            program,
            remote: panel.remote.clone(),
        };
        let (opened, pane, sidebar) = (panel.opened, panel.pane.clone(), panel.sidebar.clone());
        cx.spawn_in(window, async move |this, cx| {
            let name = to.clone();
            let result = cx
                .background_spawn(
                    async move { client.send(&name, &words, &AtomicBool::new(false)) },
                )
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                sidebar.read(cx).refresh();
                let said = match &result {
                    Ok(()) => Ok(format!("Sent to {to}")),
                    Err(refused) => Err(format!(
                        "Couldn't send to {to}: {}",
                        crate::inbox::refusal(refused)
                    )),
                };
                let open = this.popup == Some(Popup::Send);
                if let Some(panel) = this.send.as_mut().filter(|p| open && p.opened == opened) {
                    if !panel.state.finish(result) {
                        cx.notify();
                        return;
                    }
                    this.close_popup(window, cx);
                }
                let _ = pane.update(cx, |pane, cx| match said {
                    Ok(text) => pane.tell(text, cx),
                    Err(text) => pane.say(text, cx),
                });
            });
        })
        .detach();
        cx.notify();
    }

    /// The panel, hanging from where the selection's bar was: `Send to`, the agents, the note,
    /// what will go, and Cancel and Send. Esc or a click outside closes it.
    pub(super) fn send_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let panel = self.send.as_ref()?;
        let ui = UiFont::get(cx);
        let s = ui.scale(1.0);
        let theme = &*self.theme;
        let placed = popover::hang(
            panel.at,
            ui.scale(WIDTH),
            Hang::BelowLeft,
            window.viewport_size(),
            s,
        );
        let targets = panel.targets(cx);
        let chosen = panel.state.chosen(&targets);
        let agents = panel.sidebar.read(cx).agents();
        let now = now();
        let dimmer = self.fg(|t| t.agents_dimmer);
        let accent = self.fg(|t| t.agents_accent);
        let sending = panel.state.sending.is_some();
        let mut list = div()
            .id("send-list")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .max_h(ui.px(5.5 * 30.0 + 24.0))
            .overflow_y_scroll()
            .track_scroll(&panel.list)
            .child(popover::heading(theme, &ui, "Send to"));
        for (index, target) in targets.iter().enumerate() {
            let lit = Some(index) == chosen;
            let color = self.fg(dot(&Shown::Agent(target.name.clone()), &agents, now));
            let (ground, hovered) = if lit {
                (popover::lit(theme), None)
            } else {
                let ground = popover::ground(theme);
                let hovered = ground.blend(popover::lit(theme).opacity(0.6));
                (ground, Some((popover::ROW_GROUP.into(), hovered)))
            };
            let mark = self.agent_mark(target.kind.as_deref(), color, ground, hovered, &ui);
            let name = target.name.clone();
            list = list.child(
                popover::lead_row(theme, &ui, ("send-row", index), mark, lit)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(grouped(
                                agent_name(&target.name, &self.workspace.agents()),
                                dimmer,
                            )),
                    )
                    .children(target.kind.clone().map(|kind| {
                        div()
                            .flex_shrink_0()
                            .text_size(ui.px(12.0))
                            .text_color(dimmer)
                            .child(kind)
                    }))
                    .when(target.own, |row| {
                        row.child(
                            div()
                                .flex_shrink_0()
                                .px(ui.px(6.0))
                                .rounded(ui.px(4.0))
                                .bg(accent.opacity(0.16))
                                .text_size(ui.px(11.0))
                                .text_color(accent)
                                .child("this pane"),
                        )
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(panel) = &mut this.send {
                            panel.state.choose(&name);
                        }
                        cx.notify();
                    })),
            );
        }
        if targets.is_empty() {
            list = list.child(
                div()
                    .px(ui.px(9.0))
                    .py(ui.px(5.0))
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_dim))
                    .child("No agents to send to."),
            );
        }
        let note = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(ui.px(34.0))
            .mx(ui.px(4.0))
            .mt(ui.px(4.0))
            .px(ui.px(8.0))
            .rounded(ui.px(6.0))
            .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
            .border_1()
            .border_color(popover::edge_rule(theme))
            .cursor_text()
            .child(div().flex_1().min_w(px(0.0)).child(panel.input.clone()));
        let preview = div()
            .id("send-preview")
            .flex_shrink_0()
            .mx(ui.px(4.0))
            .mt(ui.px(8.0))
            .px(ui.px(8.0))
            .py(ui.px(6.0))
            .max_h(ui.px(PREVIEW))
            .overflow_y_scroll()
            .overflow_x_hidden()
            .rounded(ui.px(6.0))
            .border_1()
            .border_color(popover::edge_rule(theme))
            .font_family(self.template.font_family.clone())
            .text_size(ui.px(11.5))
            .text_color(self.fg(|t| t.agents_dim))
            .child(panel.state.message());
        let why = panel.state.refused.clone().map(|why| {
            div()
                .flex_shrink_0()
                .mx(ui.px(8.0))
                .mt(ui.px(8.0))
                .whitespace_normal()
                .text_size(ui.px(12.0))
                .text_color(self.fg(|t| t.agents_yellow))
                .child(why)
        });
        let button = |id: &'static str| {
            div()
                .id(id)
                .flex_shrink_0()
                .px(ui.px(12.0))
                .py(ui.px(4.0))
                .rounded(ui.px(6.0))
                .text_size(ui.px(12.5))
        };
        let lit = popover::lit(theme);
        let cancel = button("send-cancel")
            .cursor_pointer()
            .text_color(self.fg(|t| t.agents_text))
            .hover(move |style| style.bg(lit))
            .child("Cancel")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_popup(window, cx)));
        let ready = !targets.is_empty() && !sending;
        let send = button("send-send")
            .bg(accent)
            .text_color(popover::ground(theme))
            .font_weight(FontWeight::MEDIUM)
            .child(if sending { "Sending…" } else { "Send" })
            .map(|send| {
                if ready {
                    send.cursor_pointer()
                        .hover(|style| style.opacity(0.9))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.send_selection(window, cx)
                        }))
                } else {
                    send.opacity(0.5)
                }
            });
        let foot =
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .mt(ui.px(8.0))
                .px(ui.px(4.0))
                .pt(ui.px(8.0))
                .pb(ui.px(2.0))
                .border_t_1()
                .border_color(popover::edge_rule(theme))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap(ui.px(12.0))
                        .text_size(ui.px(11.5))
                        .text_color(dimmer)
                        .children([("↑↓", "choose"), ("↩", "send"), ("esc", "cancel")].map(
                            |(keys, what)| div().flex().gap(ui.px(4.0)).child(keys).child(what),
                        )),
                )
                .child(cancel)
                .child(send);
        let panel = popover::panel(theme, &ui)
            .id("send")
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .max_h(px(placed.max_height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(list)
            .child(popover::rule(theme, &ui))
            .child(note)
            .child(preview)
            .children(why)
            .child(foot)
            .child(cover());
        // A click outside closes it; nothing is dimmed, as for a menu.
        Some(
            div()
                .id("send-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| this.close_popup(window, cx)),
                )
                .child(panel),
        )
    }
}
