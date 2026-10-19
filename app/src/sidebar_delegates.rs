//! A controller's Delegates setting on its card (DESIGN §13 P5-141): while collapsed and set, a
//! small arrow-into-lock mark after the name; open, a Delegates cell whose menu picks `Not
//! sandboxed` or one of fence's profiles for the agents it starts from now on. A pick asks
//! `corral child-sandbox` off the UI thread; what is saved, being saved or refused is
//! `delegates.rs`'s.
use super::*;
use crate::{
    delegates::{self, Delegates},
    fence::{self, Profiles, Refusal},
    popover,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// A press on the cell's button, which opens the menu.
type OnPress = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// The menu's width, at the base interface size.
const MENU: f32 = 360.0;
/// The collapsed card's mark: the arrow, the gap after it and the lock.
const ARROW: f32 = 11.0;
const ARROW_GAP: f32 = 1.0;
/// The room the menu leaves above and below it in the window.
const MARGIN: f32 = 8.0;

/// Into the lock: down, then right.
const INTO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 4v7a4 4 0 0 0 4 4h9M14 11l4 4-4 4" fill="none" stroke="#000" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// How wide the collapsed card's mark is, for the room the name keeps.
pub(super) fn mark_width(ui: &UiFont) -> f32 {
    ui.scale(ARROW + ARROW_GAP + LOCK)
}

/// The collapsed card's mark, in the accent (an agent's own sandbox is a pill in its own colour): an arrow into a
/// lock, its hover text saying what it means.
pub(super) fn mark(theme: &Theme, ui: &UiFont, tip: Tip) -> impl IntoElement {
    let accent = hsla(theme.fg(|t| t.agents_accent), 1.0);
    div()
        .id("delegates-mark")
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(ui.px(ARROW_GAP))
        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        .child(
            svg()
                .data(INTO)
                .flex_shrink_0()
                .size(ui.px(ARROW))
                .text_color(accent),
        )
        .child(footer_icon::lock(
            false,
            accent,
            Some(accent.opacity(0.2)),
            ui.scale(LOCK),
        ))
}

/// The open card's Delegates cell: the setting, corral's refusal of the last pick, the press that
/// opens the menu (none while a pick is being saved), and the menu while it is open.
pub(super) struct DelegatesCell {
    pub delegates: Delegates,
    pub open: bool,
    pub on_press: Option<OnPress>,
    pub menu: Option<AnyElement>,
}

/// The cell across both columns: `DELEGATES` over a button with the lock and the setting; red,
/// with corral's words under it, when corral turned the last pick down.
pub(super) fn cell(theme: &Theme, ui: &UiFont, cell: DelegatesCell) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let (text, dim, dimmer, accent, red) = (
        fg(|t| t.agents_text),
        fg(|t| t.agents_dim),
        fg(|t| t.agents_dimmer),
        fg(|t| t.agents_accent),
        fg(|t| t.agents_red),
    );
    let DelegatesCell {
        delegates,
        open,
        on_press,
        menu,
    } = cell;
    let refusal = delegates.refusal();
    let (ground, lit) = match refusal {
        Some(_) => (red.opacity(0.14), red.opacity(0.2)),
        None => (text.opacity(0.12), text.opacity(0.18)),
    };
    let lock = match (&delegates.profile, delegates.saving) {
        (Some(_), false) => {
            footer_icon::lock(false, accent, Some(accent.opacity(0.2)), ui.scale(LOCK))
        }
        _ => footer_icon::lock(true, dimmer, None, ui.scale(LOCK)),
    };
    // `In sandbox` and, quieter, `· dev`.
    let words = delegates.value();
    let (main, rest) = match words.split_once(" · ") {
        Some((main, rest)) => (main.to_owned(), Some(format!("· {rest}"))),
        None => (words, None),
    };
    let saving = delegates.saving;
    let button = div()
        .id("card-delegates")
        .h(ui.px(24.0))
        .px(ui.px(CHIP_X))
        .flex()
        .items_center()
        .gap(ui.px(MARK_GAP))
        .rounded(ui.px(7.0))
        .bg(if open { lit } else { ground })
        .whitespace_nowrap()
        .overflow_hidden()
        .text_size(ui.px(SECOND_SIZE))
        .child(lock)
        .child(
            div()
                .text_color(if saving { dim } else { text })
                .child(main),
        )
        .children(rest.map(|rest| div().text_color(dim).child(rest)))
        .when(!saving, |button| {
            button.child(
                svg()
                    .data(CHEVRON_DOWN)
                    .flex_shrink_0()
                    .size(ui.px(CHEVRON))
                    .text_color(dim),
            )
        })
        .map(|button| match on_press {
            Some(on_press) => button
                .cursor_pointer()
                .hover(move |style| style.bg(lit))
                .on_mouse_down(MouseButton::Left, on_press)
                // The press opened the menu: its click is not one on the card.
                .on_click(|_, _, cx| cx.stop_propagation()),
            None => button.on_click(|_, _, cx| cx.stop_propagation()),
        });
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .whitespace_nowrap()
                .text_size(ui.px(CELL_LABEL_SIZE))
                .text_color(if refusal.is_some() {
                    red
                } else {
                    dimmer.opacity(0.8)
                })
                .child("DELEGATES"),
        )
        .child(div().relative().flex().child(button).children(menu))
        .children(refusal.map(|refusal| {
            div()
                .min_w(px(0.0))
                .whitespace_normal()
                .text_size(ui.px(NOTE_SIZE))
                .line_height(relative(1.45))
                .text_color(red)
                .child(refusal)
        }))
}

impl Sidebar {
    /// The open card's Delegates cell, for a controller on this Mac whose corral says it.
    pub(super) fn delegates_cell(
        &self,
        card: &Card,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<DelegatesCell> {
        if self.remote.is_some() {
            return None;
        }
        let delegates = card.delegates.clone()?;
        let ui = UiFont::get(cx);
        let open = matches!(
            &self.floating,
            Some(Floating::Delegates { name, instance })
                if *name == card.name && *instance == card.instance
        );
        let on_press = (!delegates.saving).then(|| -> OnPress {
            let (name, instance) = (card.name.clone(), card.instance.clone());
            Box::new(
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let menu = Floating::Delegates {
                        name: name.clone(),
                        instance: instance.clone(),
                    };
                    let mine = |this: &Self| {
                        matches!(&this.floating, Some(Floating::Delegates { name: open, .. }) if *open == name)
                    };
                    let was = mine(this);
                    this.press_menu(menu, event, window, cx);
                    if !was && mine(this) {
                        this.read_profiles(cx);
                    }
                }),
            )
        });
        Some(DelegatesCell {
            open,
            on_press,
            menu: open.then(|| {
                Self::hung(self.delegates_menu(card, &delegates, window, &ui, cx), true)
                    .into_any_element()
            }),
            delegates,
        })
    }

    /// A Delegates menu is open.
    fn delegates_open(&self) -> bool {
        matches!(self.floating, Some(Floating::Delegates { .. }))
    }

    /// Before drawing: a Delegates menu opened for a run no longer listed under its name closes,
    /// and with no Delegates menu open a read of fence's profiles still running is stopped.
    pub(super) fn tidy_delegates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Floating::Delegates { name, instance }) = &self.floating {
            let listed = self
                .listing
                .panel
                .agents
                .iter()
                .any(|a| a.name == *name && a.instance == *instance);
            if !listed {
                self.close_floating(window, cx);
            }
        }
        if !self.delegates_open() {
            self.finding_profiles = None;
        }
    }

    /// Reads fence's profiles for the menu just opened, off the UI thread, the last ones read
    /// put away meanwhile; a read still running from before is stopped, its answer not taken.
    fn read_profiles(&mut self, cx: &mut Context<Self>) {
        self.profiles = None;
        let finding = fence::Cancel::default();
        let flag = finding.flag();
        let stopped = flag.clone();
        self.finding_profiles = Some(finding);
        let task = cx.background_spawn(async move { fence::profiles(fence::PROGRAM, &flag) });
        cx.spawn(async move |this, cx| {
            let profiles = task.await;
            if stopped.load(Ordering::Relaxed) {
                return;
            }
            let _ = this.update(cx, |this, cx| {
                this.finding_profiles = None;
                this.profiles = Some(profiles);
                cx.notify();
            });
        })
        .detach();
    }

    /// The menu of `card`'s Delegates cell: `Not sandboxed`, then fence's profiles as New Agent
    /// lists them (name, where it comes from, its description; one fence can't read marked), the
    /// one set ticked; under them, from when a pick counts.
    fn delegates_menu(
        &self,
        card: &Card,
        delegates: &Delegates,
        window: &Window,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let (text, dim, dimmer, accent, red) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_accent),
            fg(|t| t.agents_red),
        );
        let lit = popover::lit(theme);
        let row = |id: ElementId, lead: AnyElement, title: Div, notes: Vec<Div>, on: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .flex()
                .items_start()
                .gap(ui.px(10.0))
                .px(ui.px(10.0))
                .py(ui.px(8.0))
                .rounded(ui.px(8.0))
                .cursor_pointer()
                .map(|row| {
                    if on {
                        row.bg(lit)
                    } else {
                        row.hover(move |style| style.bg(lit.opacity(0.7)))
                    }
                })
                .child(div().flex_shrink_0().mt(ui.px(1.0)).child(lead))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(ui.px(3.0))
                        .child(title)
                        .children(notes),
                )
                .child(div().mt(ui.px(2.0)).child(popover::tick(theme, ui, on)))
        };
        let note = |words: String, color: Hsla| {
            div()
                .text_size(ui.px(12.0))
                .line_height(relative(1.5))
                .text_color(color)
                .child(words)
        };
        let menu = self
            .floated(popover::panel(theme, ui), "delegates-menu", cx)
            .w(ui.px(MENU))
            .mt(ui.px(4.0))
            // Many profiles, or long descriptions: it scrolls rather than run out of the window.
            .max_h(window.viewport_size().height - ui.px(2.0 * MARGIN))
            .overflow_y_scroll()
            .whitespace_normal()
            .child(popover::heading(
                theme,
                ui,
                format!("Agents {} starts", card.name),
            ));
        let pick = |profile: Option<String>| {
            let (name, instance) = (card.name.clone(), card.instance.clone());
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.pick_delegates(&name, instance.as_deref(), profile.clone(), window, cx)
            })
        };
        let chosen = delegates.profile.as_deref();
        let mut menu = menu.child(
            row(
                "delegates-none".into(),
                footer_icon::lock(true, dimmer, None, ui.scale(16.0)).into_any_element(),
                div()
                    .text_size(ui.px(13.0))
                    .text_color(text)
                    .child("Not sandboxed"),
                vec![note("As today.".into(), dimmer)],
                chosen.is_none(),
            )
            .on_click(pick(None)),
        );
        match &self.profiles {
            Some(Ok(profiles)) => {
                for (index, profile) in profiles.profiles.iter().enumerate() {
                    let on = chosen == Some(profile.name.as_str());
                    let mut notes = Vec::new();
                    if let Some(description) = profile.description.clone().filter(|d| !d.is_empty())
                    {
                        notes.push(note(description, dim));
                    }
                    if let Some(problem) = profile.problem() {
                        notes.push(note(format!("Can't be read: {problem}"), red));
                    }
                    let mut title = div()
                        .flex()
                        .items_center()
                        .gap(ui.px(6.0))
                        .child(
                            div()
                                .text_size(ui.px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(text)
                                .child(profile.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(ui.px(11.0))
                                .text_color(dimmer)
                                .child(profile.origin()),
                        );
                    if profile.overrides_builtin {
                        title = title.child(
                            div()
                                .text_size(ui.px(11.0))
                                .text_color(dimmer)
                                .child("· replaces the built-in one"),
                        );
                    }
                    let (stroke, fill) = if on {
                        (accent, Some(accent.opacity(0.22)))
                    } else {
                        (dim, None)
                    };
                    menu = menu.child(
                        row(
                            ElementId::NamedInteger("delegates-profile".into(), index as u64),
                            footer_icon::lock(false, stroke, fill, ui.scale(16.0))
                                .into_any_element(),
                            title,
                            notes,
                            on,
                        )
                        .on_click(pick(Some(profile.name.clone()))),
                    );
                }
            }
            Some(Err(refusal)) => {
                menu = menu.child(
                    note(format!("Can't read fence's profiles: {refusal}"), red)
                        .px(ui.px(10.0))
                        .py(ui.px(6.0)),
                );
            }
            None => {
                menu = menu.child(
                    note("Reading fence's profiles…".into(), dimmer)
                        .px(ui.px(10.0))
                        .py(ui.px(6.0)),
                );
            }
        }
        menu.child(
            div()
                .mx(ui.px(4.0))
                .mt(ui.px(4.0))
                .px(ui.px(8.0))
                .pt(ui.px(9.0))
                .pb(ui.px(6.0))
                .border_t_1()
                .border_color(popover::edge_rule(theme))
                .child(note(
                    format!(
                        "From its next start on. Agents already running stay as they are; {} \
                         itself stays outside.",
                        card.name
                    ),
                    dimmer,
                )),
        )
    }

    /// `profile` (none: `Not sandboxed`) picked for the agents `name` starts: corral is asked off
    /// the UI thread, the cell saying `Saving…` meanwhile; its answer shows on the card at once,
    /// and the next listing is read straight after to confirm it. The one set already is left be.
    fn pick_delegates(
        &mut self,
        name: &str,
        instance: Option<&str>,
        profile: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_floating(window, cx);
        let panel = &mut self.listing.panel;
        let Some(pick) =
            delegates::pick(&panel.agents, &mut panel.delegates, name, instance, profile)
        else {
            return;
        };
        let corral = Client::local(self.corral.clone());
        let asked = pick.clone();
        let task = cx.background_spawn(async move { asked.send(&corral, &AtomicBool::new(false)) });
        cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                let panel = &mut this.listing.panel;
                panel.delegates.answer(
                    &pick.name,
                    pick.instance.as_deref(),
                    answer,
                    &mut panel.agents,
                    Instant::now(),
                );
                this.poller.refresh();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

/// The profiles last read for the menu: none before a read, or fence's refusal.
pub(super) type Read = Option<Result<Profiles, Refusal>>;
