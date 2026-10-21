//! Settings › Chat (DESIGN §13 P5-147): ranch's chat bot pony, view only. On top, pony's
//! version, whether it runs (since when, its pid) and whether its background service is
//! installed; then each chat app it is set up for, as a card: whether it is connected and for
//! how long, why not in pony's words, the workspace and bot, a warning when Slack sees another
//! connection, and the channels each bound to a repo, with what is wrong with one. Read when the
//! page is shown, every few seconds while it is, and when the window comes back to the front;
//! never written.
use super::*;
use crate::{
    fence::Cancel,
    pony::{self, Chat, Found, Status, Tone},
};
use std::sync::atomic::Ordering;

/// How often the page reads pony again while it is shown.
const EVERY: Duration = Duration::from_secs(3);

const VIEW_ONLY: &str = "View only. pony is set up in ~/.pony, not here.";

/// The page's state in the Settings window.
#[derive(Default)]
pub(super) struct ChatPage {
    /// What was last found: none before the first look.
    found: Option<Found>,
    /// Stops a look still running once it is not wanted.
    reading: Option<Cancel>,
    /// The page's timer is running.
    ticking: bool,
}

impl SettingsView {
    /// Looks for pony and reads its status, off the UI thread, unless a look is already running;
    /// what was found stays shown meanwhile.
    pub(super) fn refresh_chat(&mut self, cx: &mut Context<Self>) {
        if self.chat.reading.is_some() {
            return;
        }
        let reading = Cancel::default();
        let flag = reading.flag();
        let stopped = flag.clone();
        self.chat.reading = Some(reading);
        let task = cx.background_spawn(async move { pony::look(&flag) });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                // Stopped for another page: its answer is not wanted.
                if stopped.load(Ordering::Relaxed) {
                    return;
                }
                this.chat.reading = None;
                this.chat.found = Some(found);
                cx.notify();
            });
        })
        .detach();
        self.tick_chat(cx);
    }

    /// Another page: a look still running stops.
    pub(super) fn leave_chat(&mut self) {
        self.chat.reading = None;
    }

    /// Reads pony again every few seconds while the page is shown.
    fn tick_chat(&mut self, cx: &mut Context<Self>) {
        if self.chat.ticking {
            return;
        }
        self.chat.ticking = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(EVERY).await;
                let on = this.update(cx, |this, cx| {
                    let on = this.page == Page::Chat;
                    if on {
                        this.refresh_chat(cx);
                    } else {
                        this.chat.ticking = false;
                    }
                    on
                });
                if !matches!(on, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// A state's colour: green, blue, red, or the quiet grey.
    fn tone(&self, tone: Tone) -> Hsla {
        match tone {
            Tone::Good => self.fg(|t| t.agents_green),
            Tone::Busy => self.fg(|t| t.agents_blue),
            Tone::Bad => self.fg(|t| t.agents_red),
            Tone::Quiet => self.fg(|t| t.agents_dim),
        }
    }

    /// A rounded pill with a dot: `words` in the state's colour.
    fn chat_pill(&self, words: String, tone: Tone, ui: &UiFont) -> Div {
        let color = self.tone(tone);
        let (ground, dot) = match tone {
            Tone::Quiet => (self.faint(0.06), self.fg(|t| t.agents_dimmer)),
            _ => (color.opacity(0.12), color),
        };
        div()
            .flex_shrink_0()
            .h(ui.px(22.0))
            .px(px(9.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded_full()
            .bg(ground)
            .text_size(ui.px(12.0))
            .text_color(color)
            .child(div().size(px(6.0)).rounded_full().bg(dot))
            .child(words)
    }

    /// A small tag in the monospace face: a code as pony gives it.
    fn chat_code(&self, code: String, color: Hsla, ground: Hsla, ui: &UiFont) -> Div {
        div()
            .flex_shrink_0()
            .h(ui.px(20.0))
            .px(px(7.0))
            .flex()
            .items_center()
            .rounded(px(5.0))
            .bg(ground)
            .font_family(MONO)
            .text_size(ui.px(11.5))
            .text_color(color)
            .child(code)
    }

    /// A yellow line with the warning sign.
    fn chat_warning(&self, words: String, ui: &UiFont) -> Div {
        let yellow = self.fg(|t| t.agents_yellow);
        div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .text_size(ui.px(12.5))
            .line_height(relative(1.55))
            .text_color(yellow)
            .child(div().mt(px(2.0)).child(footer_icon::icon(
                Icon::Warning,
                yellow,
                ui.scale(12.0) / footer_icon::SIZE,
            )))
            .child(div().flex_1().min_w(px(0.0)).child(words))
    }

    /// The page.
    pub(super) fn chat_page(&self, ui: &UiFont) -> Vec<AnyElement> {
        let (dim, dimmer, red) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_red),
        );
        let line = |words: String, color: Hsla| {
            div()
                .px(px(4.0))
                .text_size(ui.px(12.5))
                .line_height(relative(1.6))
                .text_color(color)
                .child(words)
        };
        let (version, status) = match &self.chat.found {
            None => return vec![line("Looking for pony…".into(), dim).into_any_element()],
            Some(Found::Missing) => {
                return vec![
                    line(
                        "pony isn't installed, or isn't a version paddock reads (contract 1). \
                         pony is ranch's chat bot and comes with ranch."
                            .into(),
                        dim,
                    )
                    .into_any_element(),
                ];
            }
            Some(Found::Here(version, status)) => (version, status),
        };
        let mut top = div().flex().flex_wrap().items_center().gap(px(10.0));
        let mut details = Vec::new();
        match status {
            Ok(status) => {
                let (state, tone) = status.state();
                top = top.child(self.chat_pill(
                    format!("pony {} {state}", version.version),
                    tone,
                    ui,
                ));
                if !status.missing() {
                    if status.running {
                        details
                            .extend(status.started.map(|at| format!("since {}", pony::when(at))));
                        details.extend(status.pid.map(|pid| format!("pid {pid}")));
                    }
                    details.push(status.service_words());
                }
            }
            Err(_) => {
                top =
                    top.child(self.chat_pill(format!("pony {}", version.version), Tone::Quiet, ui));
            }
        }
        let top = top
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(details.join(" · ")),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(VIEW_ONLY),
            );
        let mut items = vec![top.into_any_element()];
        let status = match status {
            Ok(status) => status,
            Err(refusal) => {
                items.push(
                    line(format!("Couldn't read pony's status: {refusal}"), red).into_any_element(),
                );
                return items;
            }
        };
        if status.missing() {
            items.push(
                line(
                    "Not set up on this Mac. pony runs only on the one machine it's set up on, \
                     so nothing here talks to chat."
                        .into(),
                    dim,
                )
                .into_any_element(),
            );
            return items;
        }
        items.push(
            line(
                "pony is ranch's chat bot: @ it in a bound channel to talk to that repo's agents. \
                 Only bound repos are ever sent to chat."
                    .into(),
                dim,
            )
            .max_w(ui.px(680.0))
            .into_any_element(),
        );
        items.extend(self.chat_problems(status, ui));
        if status.chats.is_empty() {
            items.push(
                line(
                    "pony is set up here, but no chat app is configured in it.".into(),
                    dim,
                )
                .into_any_element(),
            );
            return items;
        }
        for (index, chat) in status.chats.iter().enumerate() {
            items.push(self.chat_card(index, chat, ui).into_any_element());
        }
        items.push(
            div()
                .px(px(4.0))
                .text_size(ui.px(12.0))
                .line_height(relative(1.6))
                .text_color(dimmer)
                .child(
                    "Repos with no channel send nothing to chat. A channel shows its ID until \
                     pony has connected and looked it up.",
                )
                .into_any_element(),
        );
        items
    }

    /// What stops pony from working as set up: a config it can't use, and why it isn't running.
    fn chat_problems(&self, status: &Status, ui: &UiFont) -> Vec<AnyElement> {
        let (dim, red, code) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_red),
            self.fg(|t| t.agents_branch),
        );
        let mut items = Vec::new();
        if status.config == "invalid" {
            let mut block = div()
                .px(px(14.0))
                .py(px(12.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .rounded(px(10.0))
                .bg(red.opacity(0.07))
                .child(
                    div()
                        .text_size(ui.px(12.5))
                        .text_color(red)
                        .child("The config can't be used"),
                )
                .children(status.config_message.clone().map(|message| {
                    div()
                        .font_family(MONO)
                        .text_size(ui.px(11.5))
                        .line_height(relative(1.55))
                        .text_color(code)
                        .child(message)
                }));
            if status.running {
                block = block.child(
                    div()
                        .text_size(ui.px(12.0))
                        .text_color(dim)
                        .child("pony keeps running with the last good config."),
                );
            }
            items.push(block.into_any_element());
        } else if status.config != "ok" {
            // A word paddock doesn't know: shown with what pony says about it.
            let words = match &status.config_message {
                Some(message) => format!("Config {}: {message}", status.config),
                None => format!("Config {}", status.config),
            };
            items.push(self.chat_warning(words, ui).px(px(4.0)).into_any_element());
        }
        if status.running {
            return items;
        }
        let line = |color: Hsla| {
            div()
                .px(px(4.0))
                .text_size(ui.px(12.5))
                .line_height(relative(1.6))
                .text_color(color)
        };
        let exit = status.last_exit.as_ref();
        let stopped = match (
            exit.and_then(|exit| exit.at),
            exit.and_then(|exit| exit.message.clone().or_else(|| exit.code.clone())),
        ) {
            (Some(at), Some(why)) => format!("Stopped at {}: {why}", pony::when(at)),
            (None, Some(why)) => format!("Stopped: {why}"),
            (Some(at), None) => format!("Stopped at {}.", pony::when(at)),
            (None, None) => "Not running.".to_owned(),
        };
        items.push(
            line(if exit.is_some() { red } else { dim })
                .child(stopped)
                .into_any_element(),
        );
        let mono = |words: &'static str| div().font_family(MONO).text_color(code).child(words);
        match status.service.as_str() {
            "not_installed" => items.push(
                line(dim)
                    .flex()
                    .flex_wrap()
                    .gap_x(px(4.0))
                    .child(
                        "The background service isn't installed, so pony won't start at login. \
                         To install it, run",
                    )
                    .child(mono("pony service install --dry-run"))
                    .child("in a terminal to see what it does, then")
                    .child(mono("pony service install"))
                    .into_any_element(),
            ),
            "unsupported" => items.push(
                line(dim)
                    .child(
                        "This system has no background service for pony, so it won't start at \
                         login.",
                    )
                    .into_any_element(),
            ),
            _ => {}
        }
        items
    }

    /// One chat app: its connection, workspace and bot, and the channels it is bound to.
    fn chat_card(&self, index: usize, chat: &Chat, ui: &UiFont) -> Stateful<Div> {
        let (text, dim, dimmer, code) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_branch),
        );
        let (state, tone) = chat.state();
        let failed = chat.connection == "failed";
        let mut head = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(ui.px(14.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text)
                    .child(chat.title()),
            )
            .child(self.chat_pill(state, tone, ui));
        if failed && let Some(reason) = &chat.reason {
            head = head.child(self.chat_code(reason.clone(), code, self.faint(0.06), ui));
        }
        if let Some(since) = chat.since {
            let mut words = format!("for {}", crate::card::how_long(pony::now() - since));
            if failed {
                words.push_str(" · retrying");
            }
            head = head.child(div().text_size(ui.px(12.0)).text_color(dimmer).child(words));
        }
        head = head.child(div().flex_1());
        let mut who = Vec::new();
        if let Some(workspace) = &chat.workspace {
            who.push(("Workspace", workspace.clone()));
        }
        if let Some(bot) = &chat.bot {
            who.push(("Bot", format!("@{bot}")));
        }
        if !who.is_empty() {
            let mut line = div()
                .flex()
                .flex_wrap()
                .gap(px(4.0))
                .text_size(ui.px(12.5))
                .text_color(dim);
            for (index, (label, value)) in who.into_iter().enumerate() {
                if index > 0 {
                    line = line.child("·");
                }
                line = line.child(label).child(div().text_color(text).child(value));
            }
            head = head.child(line);
        }
        let mut card = div()
            .id(ElementId::NamedInteger("chat-app".into(), index as u64))
            .px(px(18.0))
            .py(px(16.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .rounded(px(12.0))
            .bg(self.card_ground())
            .child(head);
        if failed && let Some(message) = &chat.message {
            card = card.child(
                div()
                    .mt(px(-6.0))
                    .text_size(ui.px(12.5))
                    .line_height(relative(1.6))
                    .text_color(dim)
                    .child(message.clone()),
            );
        }
        if let Some(warning) = chat.others() {
            card = card.child(self.chat_warning(warning, ui));
        }
        card.child(self.chat_channels(chat, ui))
    }

    /// `Channels · N`, each `#name` (or its ID) → repo and where `@bot` goes, or what is wrong.
    fn chat_channels(&self, chat: &Chat, ui: &UiFont) -> Div {
        let (text, dim, dimmer, yellow, code) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_branch),
        );
        let mut list = div().flex().flex_col().gap(px(2.0)).child(
            div()
                .mb(px(6.0))
                .text_size(ui.px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(dimmer)
                .child(format!("CHANNELS · {}", chat.channels.len())),
        );
        if chat.channels.is_empty() {
            return list.child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child("No channel is bound."),
            );
        }
        let stripe = self.faint(0.03);
        for (index, channel) in chat.channels.iter().enumerate() {
            let name = match &channel.name {
                Some(name) => div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(ui.px(13.0))
                    .text_color(text)
                    .child(div().text_color(dimmer).child("#"))
                    .child(name.clone())
                    .children(
                        (channel.private == Some(true))
                            .then(|| footer_icon::lock(false, dimmer, None, ui.scale(11.0))),
                    ),
                None => div()
                    .font_family(MONO)
                    .text_size(ui.px(12.5))
                    .text_color(dim)
                    .child(channel.shown()),
            };
            let note = match channel.trouble(chat.bot.as_deref()) {
                Some((words, true)) => self
                    .chat_warning(words, ui)
                    .text_size(ui.px(12.0))
                    .into_any_element(),
                Some((word, false)) => self
                    .chat_code(word, yellow, yellow.opacity(0.1), ui)
                    .into_any_element(),
                None => div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(format!(
                        "@{} goes to {}/main",
                        chat.bot_name(),
                        channel.repo
                    ))
                    .into_any_element(),
            };
            let row = div()
                .min_h(ui.px(34.0))
                .px(px(10.0))
                .py(px(4.0))
                .flex()
                .items_center()
                .rounded(px(8.0))
                .child(
                    div()
                        .w(ui.px(220.0))
                        .flex_shrink_0()
                        .overflow_hidden()
                        .child(name),
                )
                .child(
                    div()
                        .w(ui.px(28.0))
                        .flex_shrink_0()
                        .text_size(ui.px(13.0))
                        .text_color(dimmer)
                        .child("→"),
                )
                .child(
                    div()
                        .w(ui.px(180.0))
                        .flex_shrink_0()
                        .overflow_hidden()
                        .font_family(MONO)
                        .text_size(ui.px(12.5))
                        .text_color(code)
                        .child(channel.repo.clone()),
                )
                .child(div().flex_1().min_w(px(0.0)).child(note));
            list = list.child(if index % 2 == 0 { row.bg(stripe) } else { row });
        }
        list
    }
}
