//! The Inbox in the window (DESIGN §13 P5-87): the bell's panel as it is drawn and what is done
//! there. The list on the left, the selected agent's last reply on the right, and under it the
//! reply box, whose text goes to the agent with `corral send` in the background. Whom the list
//! has, what a row says and whether a reply can be typed are `inbox.rs`'s.
use super::*;
use crate::{
    corral::{Client, Reply},
    inbox::{self, Group},
    markdown, markdown_view,
};
use gpui::Subscription;
use std::{collections::HashSet, sync::atomic::AtomicBool};

/// Sizes, in points at the base interface size.
const WIDTH: f32 = 860.0;
const HEIGHT: f32 = 560.0;
const LIST: f32 = 290.0;
/// The least the reply is read in beside the list; in less, the list goes above it.
const REPLY_MIN: f32 = 320.0;
/// Of the panel's height, the most the list takes when it is above the reply.
const LIST_ABOVE: f32 = 0.4;
/// The panel's corners, and the least it is as tall (P5-103).
const RADIUS: f32 = 14.0;
const LEAST: f32 = 300.0;
/// The agent's tile over its reply, the room before it and after it; the reply starts in line with
/// the name after it.
const AVATAR: f32 = 32.0;
const SIDE: f32 = 20.0;
const GAP: f32 = 11.0;
/// The reply box: its least height, its text's size and line height, and how many lines it grows
/// to before it scrolls.
const BOX_MIN: f32 = 44.0;
const BOX_TEXT: f32 = 13.0;
const BOX_LINE: f32 = 1.5;
const BOX_LINES: f32 = 6.0;
const BOX_PAD: f32 = 8.0;

/// What a row's hover brightens: this name on each row.
const ROW: &str = "inbox-row";

/// An agent the Inbox keeps things for: its machine (`None` for this Mac) and its name.
type Who = (Option<String>, String);

/// The agent of `row`, by its machine.
fn who(row: &inbox::Row) -> Option<Who> {
    row.agent.clone().map(|name| (row.machine.clone(), name))
}

/// The Inbox's part of the window. All of it is kept only while paddock runs.
#[derive(Default)]
pub(super) struct Inbox {
    /// While the panel is open.
    open: Option<Open>,
    /// What was typed and not sent, by agent.
    drafts: HashMap<Who, String>,
    /// What was sent, by agent, until its next reply is read.
    sent: HashMap<Who, inbox::Sent>,
    /// Why corral did not take the last reply typed, by agent.
    refused: HashMap<Who, String>,
    /// The agents a reply is on its way to.
    sending: HashSet<Who>,
}

struct Open {
    list: inbox::Open,
    /// The reply box.
    input: Entity<TextInput>,
    /// The row on show, by its agent, whose draft the box holds; `None` before the first.
    on: Option<Option<Who>>,
    /// Has the keys while no reply can be typed, so that they never reach a terminal.
    focus: FocusHandle,
    rows: ScrollHandle,
    reply: ScrollHandle,
    typed: ScrollHandle,
    _typing: Subscription,
}

impl Inbox {
    /// The box changed: its text is the draft of the agent on show, and what corral said to the
    /// last try no longer stands.
    fn typed(&mut self, text: String, at_end: bool) {
        let Some(open) = &self.open else {
            return;
        };
        let Some(name) = open.on.clone().flatten() else {
            return;
        };
        if at_end {
            open.typed.scroll_to_bottom();
        }
        self.refused.remove(&name);
        if text.is_empty() {
            self.drafts.remove(&name);
        } else {
            self.drafts.insert(name, text);
        }
    }
}

/// The child of the list that shows row `index`: after its group's heading and those before.
fn list_child(rows: &[inbox::Row], index: usize) -> usize {
    let mut groups: Vec<Group> = rows.iter().take(index + 1).map(|row| row.group).collect();
    groups.dedup();
    index + groups.len()
}

/// A list row as drawn.
struct RowFacts {
    short: String,
    project: String,
    kind: Option<String>,
    dot: Hsla,
    age: Option<String>,
    line: inbox::Line,
    /// Read or answered since the panel opened: still in its group, fainter.
    quiet: bool,
}

/// What the right side shows of the selected row.
enum Showing {
    /// No rows.
    Nothing,
    /// A failed corral read: whose, and why.
    ReadFailed(String, String),
    /// An agent corral no longer lists.
    Gone(String),
    Agent(Box<Reading>),
}

/// The selected agent as its reply, its banner and its reply box go by it.
struct Reading {
    name: String,
    kind: Option<String>,
    status: Status,
    /// Since it came to this state.
    age: Option<String>,
    /// Its reply was new as the panel opened or came since.
    new: bool,
    /// Why it is in error.
    problem: Option<String>,
    reply: Option<Reply>,
    sent: Option<inbox::Sent>,
    sending: bool,
    refused: Option<String>,
    /// What the box holds.
    draft: String,
}

impl PaddockWindow {
    /// The machine named `machine` (`None` for this Mac): its place among the seats.
    fn machine_seat(&self, machine: Option<&str>) -> Option<usize> {
        match machine {
            None => Some(0),
            Some(name) => self
                .machines
                .iter()
                .position(|link| link.name == name)
                .map(|index| index + 1),
        }
    }

    /// The sidebar of the machine named `machine` (`None` for this Mac).
    fn machine_sidebar(&self, machine: Option<&str>) -> Option<&Entity<Sidebar>> {
        self.machine_seat(machine).map(|seat| self.parts(seat).0)
    }

    /// Every machine's agents for the Inbox, this Mac's first (DESIGN §13 P5-90).
    fn inbox_sources<'a>(&'a self, cx: &'a gpui::App) -> Vec<inbox::Source<'a>> {
        (0..self.seats.len())
            .map(|seat| {
                let sidebar = self.parts(seat).0.read(cx);
                inbox::Source {
                    machine: seat
                        .checked_sub(1)
                        .map(|index| self.machines[index].name.as_str()),
                    panel: sidebar.panel(),
                    error: sidebar.read_error(),
                }
            })
            .collect()
    }

    /// ⇧⌘A and the bell: the Inbox, sorted as things stand and its first row on show; again,
    /// closed.
    pub(super) fn toggle_inbox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Attention) {
            self.close_popup(window, cx);
            return;
        }
        let colors = text_input::Colors {
            text: self.fg(|t| t.agents_text),
            placeholder: self.fg(|t| t.agents_dimmer),
            cursor: self.fg(|t| t.focus),
            selection: hsla(self.theme.fg(|t| t.focus), 0.3),
        };
        let input = cx.new(|cx| TextInput::new("", "", colors, cx).wrapping(true).sending());
        let typing = cx.subscribe(&input, |this, input, _: &Changed, cx| {
            let input = input.read(cx);
            let (text, at_end) = (input.text().to_owned(), input.at_end());
            this.inbox.typed(text, at_end);
            cx.notify();
        });
        let list = inbox::Open::of(&self.inbox_sources(cx), now());
        self.palette = None;
        self.chooser = Chooser::default();
        self.inbox.open = Some(Open {
            list,
            input,
            on: None,
            focus: cx.focus_handle(),
            rows: ScrollHandle::new(),
            reply: ScrollHandle::new(),
            typed: ScrollHandle::new(),
            _typing: typing,
        });
        self.popup = Some(Popup::Attention);
        self.show_inbox_row(window, cx);
    }

    /// Another panel took the Inbox's place: it is forgotten, and the keys it held go back to the
    /// active pane.
    pub(super) fn leave_inbox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Attention) {
            return;
        }
        let Some(open) = self.inbox.open.take() else {
            return;
        };
        let field = open.input.read(cx).focus_handle(cx);
        if open.focus.is_focused(window) || field.is_focused(window) {
            self.focus_active(window, cx);
        }
    }

    /// Closed by `close_popup`, which gives the keys back itself.
    pub(super) fn forget_inbox(&mut self) {
        self.inbox.open = None;
    }

    /// The agents were read again: what was sent goes once its answer is read, the list takes
    /// what is new without moving a row, and the row on show is kept up.
    pub(super) fn sync_inbox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = now();
        let mut answered = Vec::new();
        let mut sent = std::mem::take(&mut self.inbox.sent);
        sent.retain(|who, sent| {
            let (machine, name) = who;
            let shown = self
                .machine_sidebar(machine.as_deref())
                .is_some_and(|sidebar| {
                    let panel = sidebar.read(cx).panel();
                    let agent = panel.agents.iter().find(|a| &a.name == name);
                    agent.is_some_and(|a| sent.shown(panel, a, now))
                });
            if !shown {
                answered.push(who.clone());
            }
            shown
        });
        self.inbox.sent = sent;
        let Some(mut open) = self.inbox.open.take() else {
            return;
        };
        // The answer to what was sent to the agent on show is read from its top.
        let on = open.on.clone().flatten();
        if on.is_some_and(|who| answered.contains(&who)) {
            open.reply.set_offset(point(px(0.0), px(0.0)));
        }
        open.list.refresh_of(&self.inbox_sources(cx), now);
        self.inbox.open = Some(open);
        self.show_inbox_row(window, cx);
    }

    /// ↑↓ and ⌃Tab, ⌃⇧Tab: the row after or before.
    pub(super) fn step_inbox(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = &mut self.inbox.open {
            open.list.step(by);
        }
        self.show_inbox_row(window, cx);
    }

    fn select_inbox(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = &mut self.inbox.open {
            open.list.select(index);
        }
        self.show_inbox_row(window, cx);
    }

    /// ↩: the reply sent; for an agent showing a prompt or in error, its pane.
    pub(super) fn act_inbox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.inbox_agent(cx) {
            Some((_, _, Status::Waiting | Status::Error)) => self.open_inbox_pane(window, cx),
            Some(_) => self.send_inbox(window, cx),
            None => {}
        }
    }

    /// The agent of the selected row, by its machine, and its status as the Inbox goes by it;
    /// `None` for a failed read's row and for an agent its corral no longer lists.
    fn inbox_agent(&self, cx: &gpui::App) -> Option<(Who, Agent, Status)> {
        let who = who(self.inbox.open.as_ref()?.list.selected()?)?;
        let panel = self.machine_sidebar(who.0.as_deref())?.read(cx).panel();
        let agent = panel.agents.iter().find(|a| a.name == who.1)?;
        let status = inbox::status(panel, agent, self.inbox.sent.get(&who), now());
        Some((who, agent.clone(), status))
    }

    /// Whether a reply can be typed for the selected row now.
    fn inbox_typing(&self, cx: &gpui::App) -> bool {
        self.inbox_agent(cx).is_some_and(|(who, _, status)| {
            inbox::composer(Some(status)).is_ok() && !self.inbox.sending.contains(&who)
        })
    }

    /// Shows the selected row: its draft in the box and its reply from the top when the row is
    /// another than before, and its reply read. The keys go to the box when a reply can be typed
    /// and stay with the panel when not.
    fn show_inbox_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup != Some(Popup::Attention) {
            return;
        }
        let Inbox { open, drafts, .. } = &mut self.inbox;
        let Some(open) = open else {
            return;
        };
        let row = open.list.selected().cloned();
        let agent = row.as_ref().and_then(who);
        if open.on.as_ref() != Some(&agent) {
            let draft = agent.as_ref().and_then(|who| drafts.get(who)).cloned();
            let placeholder = agent.as_ref().map(|(_, name)| format!("Reply to {name}…"));
            open.input.update(cx, |input, cx| {
                input.set_placeholder(placeholder.unwrap_or_default(), cx);
                input.set_text(draft.unwrap_or_default(), cx);
            });
            open.reply.set_offset(point(px(0.0), px(0.0)));
            open.rows
                .scroll_to_item(list_child(open.list.rows(), open.list.index()));
            open.on = Some(agent);
        }
        let (field, panel) = (open.input.read(cx).focus_handle(cx), open.focus.clone());
        let focus = if self.inbox_typing(cx) { field } else { panel };
        if !focus.is_focused(window) {
            window.focus(&focus, cx);
        }
        if let Some(row) = &row
            && let Some(sidebar) = self.machine_sidebar(row.machine.as_deref()).cloned()
        {
            sidebar.update(cx, |sidebar, cx| sidebar.read(row, cx));
        }
        cx.notify();
    }

    /// `Open pane`: the Inbox closes, the agent's machine comes on show and its pane to the
    /// front, as a click on its card brings it.
    fn open_inbox_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(((machine, name), _, _)) = self.inbox_agent(cx) else {
            return;
        };
        let Some(seat) = self.machine_seat(machine.as_deref()) else {
            return;
        };
        self.popup = None;
        self.inbox.open = None;
        self.switch(seat, window, cx);
        let metadata = self.sidebar.read(cx).metadata(&name);
        self.show_agent(&name, metadata, window, cx);
    }

    /// Sends what the box holds to the selected agent with its machine's `corral send`, in the
    /// background: only to an agent that idles, nothing blank, and the text as typed. Taken, the
    /// box empties and the text stays on show under the agent's reply; refused, it stays in the
    /// box with the reason under it. Either way its corral is read again at once.
    fn send_inbox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.inbox_typing(cx) {
            return;
        }
        let Some((who, agent, _)) = self.inbox_agent(cx) else {
            return;
        };
        let Some(open) = &self.inbox.open else {
            return;
        };
        let text = open.input.read(cx).text().to_owned();
        if text.trim().is_empty() {
            return;
        }
        let Some(seat) = self.machine_seat(who.0.as_deref()) else {
            return;
        };
        let instance = agent.instance;
        self.inbox.sending.insert(who.clone());
        self.inbox.refused.remove(&who);
        let client = Client {
            program: self.template.corral.clone(),
            remote: seat
                .checked_sub(1)
                .map(|index| self.machines[index].clone()),
        };
        let sidebar = self.parts(seat).0.clone();
        let at = now();
        cx.spawn_in(window, async move |this, cx| {
            let (to, words) = (who.1.clone(), text.clone());
            let result = cx
                .background_spawn(async move { client.send(&to, &words, &AtomicBool::new(false)) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.inbox.sending.remove(&who);
                match result {
                    Ok(()) => {
                        this.inbox.drafts.remove(&who);
                        if let Some(open) = &this.inbox.open
                            && open.on.as_ref() == Some(&Some(who.clone()))
                        {
                            open.input.update(cx, |input, cx| input.set_text("", cx));
                            // What was sent is on show, under the reply.
                            open.reply.scroll_to_bottom();
                        }
                        let sent = inbox::Sent { text, at, instance };
                        this.inbox.sent.insert(who, sent);
                    }
                    Err(refused) => {
                        this.inbox.refused.insert(who, inbox::refusal(&refused));
                    }
                }
                sidebar.read(cx).refresh();
                this.sync_inbox(window, cx);
            });
        })
        .detach();
        // The box is not typed in while its text is on the way.
        self.show_inbox_row(window, cx);
    }

    /// The colour of a row's line in `tone`.
    fn inbox_ink(&self, tone: inbox::Tone) -> Hsla {
        match tone {
            inbox::Tone::Waiting => self.fg(|t| t.agents_yellow),
            inbox::Tone::Problem => self.fg(|t| t.agents_red),
            inbox::Tone::Working => self.fg(|t| t.agents_blue),
            inbox::Tone::New => self.fg(|t| t.agents_text),
            inbox::Tone::Quiet => self.fg(|t| t.agents_dim),
        }
    }

    /// The panel's ground, a shade lighter than the panels it shares a look with so that it
    /// stands off the window (P5-103), and the list's, a shade darker than that.
    fn inbox_grounds(&self) -> (Hsla, Hsla) {
        let text = self.fg(|t| t.agents_text);
        let panel = popover::ground(&self.theme).blend(text.opacity(0.03));
        let shade = hsla(self.theme.bg(|t| t.agents_bg), 1.0);
        (panel, panel.blend(shade.opacity(0.5)))
    }

    /// The Inbox, hanging from the bell: the list, and beside it the selected row's reply and the
    /// reply box; as tall as they are, up to its height. In a window too narrow for both the list
    /// goes above. Esc or a click outside closes it.
    pub(super) fn inbox_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let open = self.inbox.open.as_ref()?;
        let ui = UiFont::get(cx);
        let theme = &*self.theme;
        let placed = self.placed(Popup::Attention, WIDTH, window, cx);
        let above = placed.width < ui.scale(LIST + REPLY_MIN);
        let height = ui.scale(HEIGHT).min(placed.max_height);
        let text = self.fg(|t| t.agents_text);
        let (ground, list_ground) = self.inbox_grounds();
        // Inside the edge, so the list's ground keeps to the corners.
        let inner = ui.px(RADIUS - 1.0);
        let list = self
            .inbox_list(open, &ui, cx)
            .flex_shrink_0()
            .bg(list_ground)
            .map(|list| {
                if above {
                    list.max_h(px(height * LIST_ABOVE)).rounded_t(inner)
                } else {
                    list.w(ui.px(LIST)).rounded_l(inner)
                }
            });
        let reply = self.inbox_reply(open, window, &ui, cx);
        let body = div()
            .flex_grow(1.0)
            .flex_shrink(1.0)
            .min_h(px(0.0))
            .flex()
            .map(|body| if above { body.flex_col() } else { body })
            .child(list)
            .child(reply);
        let panel = popover::panel(theme, &ui)
            .p(px(0.0))
            .rounded(ui.px(RADIUS))
            .border_color(text.opacity(0.10))
            .bg(ground)
            .shadow(vec![
                BoxShadow {
                    color: gpui::black().opacity(0.6),
                    offset: point(px(0.0), ui.px(24.0)),
                    blur_radius: ui.px(64.0),
                    spread_radius: px(0.0),
                    inset: false,
                },
                BoxShadow {
                    color: gpui::black().opacity(0.3),
                    offset: point(px(0.0), ui.px(2.0)),
                    blur_radius: ui.px(8.0),
                    spread_radius: px(0.0),
                    inset: false,
                },
            ])
            .id("inbox")
            .track_focus(&open.focus)
            .absolute()
            .left(px(placed.left))
            .w(px(placed.width))
            .min_h(px(ui.scale(LEAST).min(height)))
            .max_h(px(height))
            .map(|panel| match (placed.top, placed.bottom) {
                (Some(top), _) => panel.top(px(top)),
                (None, bottom) => panel.bottom(px(bottom.unwrap_or_default())),
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // A click on the panel itself takes the keys from the box: back they go.
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.show_inbox_row(window, cx)),
            )
            .child(body)
            .child(cover());
        // A click outside closes it; nothing is dimmed, as for a menu.
        Some(
            div()
                .id("inbox-backdrop")
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

    /// The list: `Inbox`, how many rows ring the bell and its keys, then the rows under their
    /// groups' names, each with its icon and status, name, project and age, and its line under
    /// them; along its foot the keys that move and close.
    fn inbox_list(&self, open: &Open, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let now = now();
        let text = self.fg(|t| t.agents_text);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let accent = self.fg(|t| t.agents_accent);
        let amber = self.fg(|t| t.agents_yellow);
        let (_, ground) = self.inbox_grounds();
        let lit = ground.blend(accent.opacity(0.11));
        let hover = ground.blend(text.opacity(0.04));
        let (count, facts): (usize, Vec<RowFacts>) = {
            let sources = self.inbox_sources(cx);
            // What rings, by machine.
            let ringing: Vec<(Option<&str>, Vec<crate::attention::Item>)> = sources
                .iter()
                .map(|s| (s.machine, inbox::ringing(s.panel, s.error, now)))
                .collect();
            let facts = open.list.rows().iter().filter_map(|row| {
                let source = sources
                    .iter()
                    .find(|s| s.machine == row.machine.as_deref())?;
                let (panel, error) = (source.panel, source.error);
                let name = row.agent.as_ref();
                let agent = name.and_then(|name| panel.agents.iter().find(|a| &a.name == name));
                let sent = who(row).and_then(|who| self.inbox.sent.get(&who));
                let label = row.agent.as_deref().unwrap_or("corral");
                let dot: Pick = match (agent, name) {
                    (Some(a), _) => card::look(inbox::status(panel, a, sent, now)).color,
                    (None, Some(_)) => |t| t.agents_faint,
                    (None, None) => |t| t.agents_red,
                };
                let group = agent.map(|a| panel.group(a)).unwrap_or_default();
                let rings = ringing
                    .iter()
                    .find(|(machine, _)| *machine == row.machine.as_deref())
                    .is_some_and(|(_, items)| items.iter().any(|item| item.agent == row.agent));
                Some(RowFacts {
                    short: label.rsplit('/').next().unwrap_or(label).to_owned(),
                    // Another machine's row says which (DESIGN §13 P5-90).
                    project: match &row.machine {
                        Some(machine) if group.is_empty() => machine.clone(),
                        Some(machine) => format!("{machine} · {group}"),
                        None => group.to_owned(),
                    },
                    kind: agent.and_then(|a| a.kind.clone()),
                    dot: self.fg(dot),
                    // Since the agent came to this state, when corral says.
                    age: agent
                        .and_then(|a| a.state_started)
                        .map(|since| card::short_time(Some(now - since))),
                    line: inbox::line(panel, row, error, sent, now),
                    quiet: row.group != Group::Read && !rings,
                })
            });
            let count = ringing.iter().map(|(_, items)| items.len()).sum();
            (count, facts.collect())
        };
        let cap = |key: String| {
            div()
                .flex_shrink_0()
                .h(ui.px(18.0))
                .min_w(ui.px(18.0))
                .px(ui.px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(5.0))
                .bg(text.opacity(0.06))
                .text_size(ui.px(11.0))
                .text_color(dim)
                .child(key)
        };
        let header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .pl(ui.px(16.0))
            .pr(ui.px(14.0))
            .pt(ui.px(14.0))
            .pb(ui.px(8.0))
            .child(
                div()
                    .text_size(ui.px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text)
                    .child("Inbox"),
            )
            .when(count > 0, |header| {
                header.child(
                    div()
                        .flex_shrink_0()
                        .h(ui.px(18.0))
                        .min_w(ui.px(18.0))
                        .px(ui.px(5.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(accent.opacity(0.18))
                        .text_size(ui.px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(accent)
                        .child(count.to_string()),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .gap(ui.px(3.0))
                    .children(menu::keys(&menu::ShowAttention).into_iter().map(cap)),
            );
        let mut list = div()
            .id("inbox-rows")
            .track_scroll(&open.rows)
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .flex_shrink(1.0)
            .min_h(px(0.0))
            .gap(ui.px(2.0))
            .px(ui.px(8.0))
            .overflow_y_scroll();
        if facts.is_empty() {
            list = list.child(
                div()
                    .px(ui.px(8.0))
                    .py(ui.px(6.0))
                    .text_size(ui.px(12.5))
                    .text_color(dim)
                    .child("No controllers, and nothing needs you."),
            );
        }
        let selected = open.list.index();
        let mut group = None;
        for (index, (row, facts)) in open.list.rows().iter().zip(facts).enumerate() {
            if group != Some(row.group) {
                let ink = match row.group {
                    Group::Needs => amber,
                    Group::New => accent,
                    Group::Read => dimmer,
                };
                list = list.child(
                    div()
                        .flex_shrink_0()
                        .px(ui.px(8.0))
                        .pt(ui.px(if group.is_none() { 4.0 } else { 12.0 }))
                        .pb(ui.px(4.0))
                        .text_size(ui.px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink)
                        .child(row.group.title()),
                );
                group = Some(row.group);
            }
            // Its kind and status before its name, ringed in the row's ground.
            let (under, hovered) = if index == selected {
                (lit, None)
            } else {
                (ground, Some((SharedString::from(ROW), hover)))
            };
            let dot = if facts.quiet {
                facts.dot.opacity(0.45)
            } else {
                facts.dot
            };
            let unread = row.group == Group::New && !facts.quiet;
            let badge = self.agent_mark(facts.kind.as_deref(), dot, under, hovered, ui);
            let first = div()
                .flex()
                .items_baseline()
                .gap(ui.px(6.0))
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if facts.quiet { dim } else { text })
                        .child(facts.short),
                )
                .when(!facts.project.is_empty(), |line| {
                    line.child(
                        div()
                            .flex_shrink_0()
                            .text_size(ui.px(12.0))
                            .text_color(dimmer)
                            .child(facts.project),
                    )
                })
                .child(div().flex_1())
                .children(facts.age.map(|age| {
                    div()
                        .flex_shrink_0()
                        .text_size(ui.px(11.5))
                        .text_color(dimmer)
                        .child(age)
                }))
                .when(unread, |line| {
                    line.child(
                        div()
                            .flex_shrink_0()
                            .self_center()
                            .size(ui.px(7.0))
                            .rounded_full()
                            .bg(accent),
                    )
                });
            let ink = self.inbox_ink(facts.line.tone);
            let said = facts.line.sent.is_some();
            let second = div()
                .flex()
                .gap(ui.px(5.0))
                .text_size(ui.px(12.5))
                .when(facts.quiet, |line| line.opacity(0.7))
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(ink)
                        .map(|line| {
                            if said {
                                line.flex_shrink_0()
                            } else {
                                line.flex_1()
                            }
                        })
                        .child(facts.line.text),
                )
                .children(facts.line.sent.map(|sent| {
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(dim)
                        .child(format!("· You: {sent}"))
                }));
            let item = div()
                .id(("inbox-row", index))
                .group(ROW)
                .flex_shrink_0()
                .flex()
                .items_start()
                .gap(ui.px(10.0))
                .p(ui.px(10.0))
                .rounded(ui.px(10.0))
                .cursor_pointer()
                .map(|item| {
                    if index == selected {
                        item.bg(lit)
                    } else {
                        item.hover(move |style| style.bg(hover))
                    }
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.select_inbox(index, window, cx)
                }))
                .child(div().flex_shrink_0().mt(ui.px(2.0)).child(badge))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(3.0))
                        .child(first)
                        .child(second),
                );
            list = list.child(item);
        }
        let keys = div()
            .flex_shrink_0()
            .flex()
            .gap(ui.px(14.0))
            .pl(ui.px(16.0))
            .pt(ui.px(10.0))
            .pb(ui.px(12.0))
            .whitespace_nowrap()
            .text_size(ui.px(11.0))
            .text_color(dimmer)
            .child("⌃Tab next")
            .child("Esc close");
        div()
            .flex()
            .flex_col()
            .min_h(px(0.0))
            .child(header)
            .child(list)
            .child(keys)
    }

    /// What the right side shows, gathered from the sidebar's listing.
    fn inbox_showing(&self, open: &Open, cx: &gpui::App) -> Showing {
        let Some(row) = open.list.selected() else {
            return Showing::Nothing;
        };
        let now = now();
        let Some(sidebar) = self.machine_sidebar(row.machine.as_deref()) else {
            return Showing::Nothing;
        };
        let sidebar = sidebar.read(cx);
        let panel = sidebar.panel();
        // Another machine's say which (DESIGN §13 P5-90).
        let titled = |name: &str| match &row.machine {
            Some(machine) => format!("{name} · {machine}"),
            None => name.to_owned(),
        };
        let Some(who) = who(row) else {
            let error = sidebar.read_error().unwrap_or("Read failed");
            return Showing::ReadFailed(titled("corral"), error.to_owned());
        };
        let name = &who.1;
        let Some(agent) = panel.agents.iter().find(|a| &a.name == name) else {
            return Showing::Gone(titled(name));
        };
        let sent = self.inbox.sent.get(&who);
        Showing::Agent(Box::new(Reading {
            name: titled(name),
            kind: agent.kind.clone(),
            status: inbox::status(panel, agent, sent, now),
            age: agent
                .state_started
                .map(|since| card::short_time(Some(now - since))),
            new: row.group == Group::New,
            problem: crate::attention::problem(agent),
            reply: panel.said(agent, now).map(|said| said.reply.clone()),
            sent: sent.cloned(),
            sending: self.inbox.sending.contains(&who),
            refused: self.inbox.refused.get(&who).cloned(),
            draft: open.input.read(cx).text().to_owned(),
        }))
    }

    /// The right side: the agent's icon and name, how it is and since when under the name, and
    /// `Open pane`; its last reply as Markdown, in line with the name, under a banner when it
    /// shows a prompt or is in error, and over what was just sent to it; then the reply box, or
    /// why there is none, in one rounded well with the send button.
    fn inbox_reply(
        &self,
        open: &Open,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = &*self.theme;
        let now = now();
        let (ground, _) = self.inbox_grounds();
        let text = self.fg(|t| t.agents_text);
        let branch = self.fg(|t| t.agents_branch);
        let dim = self.fg(|t| t.agents_dim);
        let dimmer = self.fg(|t| t.agents_dimmer);
        let accent = self.fg(|t| t.agents_accent);
        let amber = self.fg(|t| t.agents_yellow);
        let red = self.fg(|t| t.agents_red);
        let shade = hsla(theme.bg(|t| t.agents_bg), 1.0);
        let showing = self.inbox_showing(open, cx);
        let column = div()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .whitespace_normal();
        // The agent's icon on a faint tile; a dot of `dot` for one without an icon.
        let avatar = |kind: Option<&str>, dot: Hsla| {
            let icon = kind.and_then(kind_icon::of).map(|icon| {
                icon.render(
                    ui.px(20.0),
                    self.fg(card::brand(kind.unwrap_or_default()).color),
                )
            });
            div()
                .flex_shrink_0()
                .size(ui.px(AVATAR))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.px(9.0))
                .bg(text.opacity(0.06))
                .child(icon.unwrap_or_else(|| plain_dot(dot, ui)))
        };
        // Under the name: the parts given, each in its colour, with dots between.
        let about = |parts: Vec<(String, Hsla)>| {
            div()
                .flex()
                .gap(ui.px(5.0))
                .text_size(ui.px(12.0))
                .children(
                    parts
                        .into_iter()
                        .enumerate()
                        .flat_map(|(at, (words, ink))| {
                            let dot = (at > 0).then(|| div().text_color(dimmer).child("·"));
                            dot.into_iter()
                                .chain(Some(div().text_color(ink).child(words)))
                        }),
                )
        };
        let head = |name: String, avatar: Div, about: Option<Div>| {
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(GAP))
                .pl(ui.px(SIDE))
                .pr(ui.px(14.0))
                .pt(ui.px(14.0))
                .pb(ui.px(10.0))
                .whitespace_nowrap()
                .child(avatar)
                .child(
                    div()
                        .flex_shrink(1.0)
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(2.0))
                        .child(
                            div()
                                .overflow_hidden()
                                .text_ellipsis()
                                .text_size(ui.px(14.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .children(about),
                )
        };
        let says = |words: String, ink: Hsla| {
            div()
                .flex_1()
                .min_h(px(0.0))
                .pl(ui.px(SIDE + AVATAR + GAP))
                .pr(ui.px(28.0))
                .py(ui.px(6.0))
                .text_color(ink)
                .child(words)
        };
        // The rounded well the reply box sits in, and under it why the last reply was refused.
        let well = |lit: bool| {
            div()
                .flex()
                .items_end()
                .gap(ui.px(10.0))
                .pl(ui.px(14.0))
                .pr(ui.px(8.0))
                .py(ui.px(8.0))
                .rounded(ui.px(14.0))
                .border_1()
                .border_color(if lit {
                    accent.opacity(0.5)
                } else {
                    text.opacity(0.08)
                })
                .bg(shade.opacity(0.6))
        };
        let foot = || {
            div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap(ui.px(6.0))
                .px(ui.px(14.0))
                .pb(ui.px(14.0))
        };
        // Where a reply is typed when it can be: why not, in the well.
        let closed = |why: &'static str| {
            well(false).child(
                div()
                    .flex_1()
                    .min_h(ui.px(BOX_MIN))
                    .flex()
                    .items_center()
                    .text_size(ui.px(12.5))
                    .text_color(dim)
                    .child(why),
            )
        };
        let reading = match showing {
            Showing::Nothing => {
                return column.child(
                    div()
                        .px(ui.px(SIDE))
                        .py(ui.px(16.0))
                        .text_color(dim)
                        .child("Nothing to read."),
                );
            }
            Showing::ReadFailed(title, error) => {
                return column
                    .child(head(
                        title,
                        avatar(None, red),
                        Some(about(vec![("Error".into(), red)])),
                    ))
                    .child(says(error, red));
            }
            Showing::Gone(name) => {
                return column
                    .child(head(name, avatar(None, dimmer), None))
                    .child(div().flex_grow(1.0))
                    .child(foot().child(closed(inbox::composer(None).err().unwrap_or_default())));
            }
            Showing::Agent(reading) => *reading,
        };

        let mut facts = Vec::new();
        if let Some((label, tone)) = inbox::pill(reading.status, reading.new) {
            let ink = match tone {
                inbox::Tone::New => accent,
                tone => self.inbox_ink(tone),
            };
            facts.push((label.to_owned(), ink));
        }
        if let Some(age) = &reading.age {
            facts.push((format!("{age} ago"), dimmer));
        }
        let open_pane =
            cx.listener(|this, _: &ClickEvent, window, cx| this.open_inbox_pane(window, cx));
        let header = head(
            reading.name.clone(),
            avatar(
                reading.kind.as_deref(),
                self.fg(card::look(reading.status).color),
            ),
            (!facts.is_empty()).then(|| about(facts)),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("inbox-open")
                .flex_shrink_0()
                .h(ui.px(28.0))
                .px(ui.px(10.0))
                .flex()
                .items_center()
                .gap(ui.px(6.0))
                .rounded(ui.px(8.0))
                .bg(text.opacity(0.06))
                .text_size(ui.px(12.0))
                .text_color(branch)
                .cursor_pointer()
                .hover(move |style| style.bg(text.opacity(0.10)).text_color(text))
                .on_click(open_pane)
                .child("Open pane")
                .child(footer_icon::icon(
                    Icon::External,
                    dim,
                    ui.scale(11.0 / footer_icon::SIZE),
                )),
        );

        // Over the reply of an agent that cannot be answered here: why, and the way to its pane.
        let banner = match reading.status {
            Status::Waiting => Some((
                amber,
                Icon::NewShell,
                "It is showing a prompt in its pane".to_owned(),
                Some("Prompts can only be answered there."),
            )),
            Status::Error => Some((
                red,
                Icon::Warning,
                reading.problem.unwrap_or_else(|| "It is in error".into()),
                None,
            )),
            _ => None,
        };
        let flagged = banner.is_some();
        let banner = banner.map(|(ink, icon, title, detail)| {
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(10.0))
                .px(ui.px(12.0))
                .py(ui.px(10.0))
                .rounded(ui.px(10.0))
                .bg(ink.opacity(0.10))
                .child(
                    div()
                        .flex_shrink_0()
                        .child(footer_icon::icon(icon, ink, ui.scale(1.0))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(2.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ink)
                                .child(title),
                        )
                        .children(detail.map(|detail| {
                            div().text_size(ui.px(12.0)).text_color(dim).child(detail)
                        })),
                )
                .child(
                    div()
                        .id("inbox-open-prompt")
                        .flex_shrink_0()
                        .h(ui.px(26.0))
                        .px(ui.px(10.0))
                        .flex()
                        .items_center()
                        .rounded(ui.px(7.0))
                        .bg(ink)
                        .text_color(ground)
                        .text_size(ui.px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .hover(|style| style.opacity(0.9))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.open_inbox_pane(window, cx)
                        }))
                        .child("Open pane ↩"),
                )
        });
        let note = |words: String| {
            div()
                .flex_shrink_0()
                .text_size(ui.px(12.0))
                .text_color(dim)
                .child(words)
        };
        let answered = reading.sent.is_some();
        let said = match &reading.reply {
            Some(reply) => {
                let colors = markdown_view::Colors::of(theme);
                let mono = mono_font(&self.template);
                let blocks = markdown::blocks(&reply.text);
                let since = reply
                    .at
                    .map(|at| format!(" · {} ago", card::short_time(Some(now - at))));
                Some(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .gap(ui.px(8.0))
                        .when(flagged, |said| {
                            said.child(note(format!("Last reply{}", since.unwrap_or_default())))
                        })
                        .child(
                            markdown_view::blocks(&blocks, &mono, colors, ui)
                                // Answered already: it stays, fainter, until the next one.
                                .when(answered, |blocks| blocks.opacity(0.55)),
                        ),
                )
            }
            // Under a banner there is nothing to add.
            None if flagged => None,
            None => Some(note("No reply to show yet.".into())),
        };
        let sent = reading.sent.map(|sent| {
            let ago = now - sent.at;
            let when = if ago < 60.0 {
                "just now".to_owned()
            } else {
                format!("{} ago", card::short_time(Some(ago)))
            };
            div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap(ui.px(8.0))
                .child(
                    div()
                        .ml(ui.px(48.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(4.0))
                        .px(ui.px(12.0))
                        .py(ui.px(9.0))
                        .rounded(ui.px(10.0))
                        .bg(accent.opacity(0.1))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(ui.px(5.0))
                                .text_size(ui.px(11.5))
                                .text_color(accent)
                                .child(footer_icon::icon(Icon::Check, accent, ui.scale(1.0)))
                                .child(format!("You · sent {when}")),
                        )
                        .child(div().line_height(relative(1.55)).child(sent.text)),
                )
                .child(note(
                    "Its next reply will show up here when the turn ends.".into(),
                ))
        });
        let body = div()
            .id("inbox-reply")
            .track_scroll(&open.reply)
            .flex_grow(1.0)
            .flex_shrink(1.0)
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui.px(12.0))
            .pl(ui.px(SIDE + AVATAR + GAP))
            .pr(ui.px(28.0))
            .pt(ui.px(2.0))
            .pb(ui.px(16.0))
            .children(banner)
            .children(said)
            .children(sent);

        let composer = inbox::composer(Some(reading.status));
        let typing = composer.is_ok() && !reading.sending;
        let field = open.input.read(cx).focus_handle(cx);
        let frame = || {
            div()
                .id("inbox-box")
                .track_scroll(&open.typed)
                .flex_1()
                .min_w(px(0.0))
                .min_h(ui.px(BOX_MIN))
                .max_h(ui.px(BOX_TEXT * BOX_LINE * BOX_LINES + 2.0 * BOX_PAD))
                .overflow_y_scroll()
                .py(ui.px(BOX_PAD))
                .text_size(ui.px(BOX_TEXT))
                .line_height(relative(BOX_LINE))
        };
        let ready = typing && !reading.draft.trim().is_empty();
        let button = div()
            .id("inbox-send")
            .flex_shrink_0()
            .size(ui.px(30.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .map(|button| {
                if ready {
                    button
                        .bg(accent)
                        .cursor_pointer()
                        .hover(|style| style.opacity(0.9))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.send_inbox(window, cx)
                        }))
                } else {
                    button
                        .bg(text.opacity(0.08))
                        .when(reading.sending, |button| button.opacity(0.5))
                }
            })
            .child(send_arrow(
                if ready { ground } else { dimmer },
                ui.scale(14.0),
            ));
        let reply_box = match composer {
            Err(why) => closed(why),
            // On its way: as typed, and not to be changed.
            Ok(()) if reading.sending => well(false)
                .child(frame().text_color(dim).child(reading.draft.clone()))
                .child(button),
            Ok(()) => well(field.is_focused(window))
                .child(frame().cursor_text().child(open.input.clone()))
                .child(
                    div()
                        .flex_shrink_0()
                        .pb(ui.px(7.0))
                        .whitespace_nowrap()
                        .text_size(ui.px(11.0))
                        .text_color(dimmer)
                        .child("⇧↩ new line"),
                )
                .child(button),
        };
        let foot = foot().child(reply_box).children(
            reading
                .refused
                .map(|why| div().text_size(ui.px(12.0)).text_color(amber).child(why)),
        );
        column.child(header).child(body).child(foot)
    }
}

/// The send button's arrow, `size` points square, in `color`.
fn send_arrow(color: Hsla, size: f32) -> impl IntoElement {
    gpui::canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let unit = size / 24.0;
            let at = |x: f32, y: f32| bounds.origin + point(px(x * unit), px(y * unit));
            let mut path = gpui::PathBuilder::stroke(px(2.4 * unit));
            path.move_to(at(12.0, 19.0));
            path.line_to(at(12.0, 5.0));
            path.move_to(at(5.0, 12.0));
            path.line_to(at(12.0, 5.0));
            path.line_to(at(19.0, 12.0));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .flex_shrink_0()
    .size(px(size))
}
